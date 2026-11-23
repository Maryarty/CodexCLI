//! The settings dialogs (S15; oracle `chatwidget/status_surfaces.rs`, `theme_picker.rs`,
//! `config_panel`, `memories_settings.rs`, `app/config_persistence.rs`, `app/event_dispatch.rs`)
//! as an ADR-013 feature: `/theme`, `/memories`, `/statusline`, `/title`, `/skills`, `/config`,
//! `/experimental`, `/keymap`. Core keeps what IO needs: the facts the host read of the config
//! and the disk (`Fact::Facts`), the catalogues the dialogs are offered (`Offer`), the highlight
//! theme in force and the log of the requests the dialogs left (`Edit`); a request that is a
//! write of the config file goes to the host as `Ask::ConfigWrite` through `AppCx::effect`, and
//! its answer is what the session follows (`effect_done`). What a dialog draws and what is not
//! saved yet (cursor, checks, order, page, preview) is the sheet's own draft, in
//! `src/features/settings/`.
//!
//! The dialog is opened by kind (`Intent::Open`): the app builds its catalogue from the facts
//! (the theme picker from the theme files and the config's theme, the status line and the title
//! from the chat's status facts, the memories from the memory tool's switches). The catalogues
//! the server gives (skills, experimental features, hooks) and the host's (config panel, keymap)
//! arrive as facts. The oracle's facts after a write (the status line items, the theme) are the
//! host's: it reloads the config once the file has the edit and sends them again.

mod catalog;
pub mod hooks;
pub mod items;
pub mod persist;
mod seed;

use std::collections::BTreeMap;

use codex_app_server_protocol::{ClientRequest, RequestId};

use super::{AppCx, Never};
use crate::app::effects::{Ask, Done};
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::screens::SeedError;
use crate::threads::Tone;

pub use catalog::*;
pub use persist::Setting;

/// The screen map's id of the dialogs (S15), and what they are: sheets over the session page.
pub const ID: &str = "S15";
pub const KIND: crate::screens::Kind = crate::screens::Kind::Sheet;

/// The marker of the feature (`feature_list!`).
pub struct Settings;

/// What a slash command or a sheet asks of the feature: verbs of the domain. The cursor, the
/// group tab, the search text, the checks and their order are the sheets' own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// The dialog of this kind is opened: its catalogue is built from the facts the app holds
    /// (a kind the server or the host offers has none to build and keeps what it was given).
    Open(Which),
    /// The syntax theme in force (the picker's preview, and the restore on cancel: none is the
    /// config's own).
    SyntaxTheme(Option<String>),
    /// What a dialog asks of the app (`AppEvent`s of the oracle): recorded, and written to the
    /// config file when it is a write.
    Request(Edit),
}

/// What the host tells the feature unasked, and what a script seeds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fact {
    /// A dialog is offered a catalogue.
    Offered(Offer),
    /// The config's values and the disk's files the dialogs open from.
    Facts(Facts),
    /// The server's catalogue of the experimental features arrived; none: the request failed.
    /// The host leaves out what the dialog hides (the stages other than beta, the retired
    /// features).
    FeaturesDiscovered {
        generation: u64,
        features: Option<Vec<catalog::Feature>>,
    },
    /// The write of the experimental features was answered.
    FeaturesSaved {
        generation: u64,
        result: Result<Written, String>,
    },
}

impl Fact {
    /// The config's values the dialogs open from, as the host read them: the theme in force,
    /// the `.tmTheme` files of the themes directory (file stems) with that directory as the
    /// subtitle names it, and the memory tool's two switches when it is on.
    pub fn facts(
        syntax_theme: Option<String>,
        custom_themes: Vec<String>,
        themes_dir: Option<String>,
        memories: Option<(bool, bool)>,
    ) -> Fact {
        Fact::Facts(Facts {
            syntax_theme,
            custom_themes,
            themes_dir,
            memories: memories.map(|(use_memories, generate_memories)| MemoryFacts {
                use_memories,
                generate_memories,
            }),
        })
    }
}

/// What a write of the config file was for, given back with the host's answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ctx {
    Write(persist::Subject),
}

/// What the feature says in the history: a write the config file refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Note {
    /// `Failed to save {setting}: {error}`.
    SaveFailed { setting: Setting, error: String },
}

/// The app-scoped state of the feature, changed only by the steps of `Feature`. The dialogs are
/// the sheets' (`src/features/settings/`): what is open, the cursor, what is checked, moved or
/// typed before it is saved are their drafts. The state keeps what the session reads or writes
/// out (ADR-013): the facts the dialogs open from, the catalogues they are offered, the
/// highlight theme in force, the requests the dialogs left.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    pub facts: Facts,
    /// What the dialogs are offered, by kind (the server's and the host's catalogues).
    pub offers: BTreeMap<Which, Offer>,
    /// The syntax theme in force (the oracle: `highlight::current_syntax_theme`): the picker
    /// previews by changing it.
    pub syntax_theme: Option<String>,
    /// The requests the dialogs left, in order.
    pub edits: Vec<Edit>,
}

impl State {
    /// The catalogue of the experimental features, as the server gave it.
    fn experimental_mut(&mut self) -> Option<&mut Experimental> {
        match self.offers.get_mut(&Which::Experimental) {
            Some(Offer::Experimental(dialog)) => Some(dialog),
            _ => None,
        }
    }

    /// A catalogue is offered; the theme picker's offer is the theme in force.
    fn offer(&mut self, offer: Offer) {
        if let Offer::Theme(picker) = &offer {
            self.syntax_theme = picker.original.clone();
        }
        self.offers.insert(offer.which(), offer);
    }

    /// The facts changed: the theme in force follows the config, but not while a picker
    /// previews another one.
    fn facts_changed(&mut self, facts: Facts) {
        if self.syntax_theme == self.facts.syntax_theme {
            self.syntax_theme = facts.syntax_theme.clone();
        }
        self.facts = facts;
    }

    /// The catalogue of the dialog of kind `which`, built from what the app holds.
    fn open(&mut self, which: Which, cx: &AppCx<'_>) {
        let offer = match which {
            Which::Theme => Offer::Theme(self.facts.theme_picker()),
            Which::Memories => match self.facts.memories {
                Some(facts) => Offer::Memories(Memories {
                    use_memories: facts.use_memories,
                    generate_memories: facts.generate_memories,
                }),
                None => return,
            },
            Which::StatusLine => {
                Offer::Setup(items::status_line(&cx.view().screens().chat.status))
            }
            Which::Title => Offer::Setup(items::title(&cx.view().screens().chat.status)),
            Which::Config | Which::Skills | Which::Keymap | Which::Experimental | Which::Hooks => {
                return;
            }
        };
        self.offer(offer);
    }

    /// A request of a dialog: the state it changes at once, then the write it stands for.
    fn request(&mut self, edit: Edit, cx: &mut AppCx<'_>) {
        match &edit {
            Edit::ThemeSelected(theme) => self.syntax_theme = Some(theme.clone()),
            // A write is on its way: a failed answer can follow a committed write, so these
            // keys stay dirty and putting a value back still sends a correction.
            Edit::FeaturesSave(updates) => {
                if let Some(dialog) = self.experimental_mut() {
                    dialog.saving = true;
                    dialog.unconfirmed = updates.iter().map(|(key, _)| key.clone()).collect();
                    dialog.status = FeaturesStatus::Saving;
                }
            }
            _ => {}
        }
        if let Some(write) = persist::write_of(&edit) {
            cx.effect(
                Ask::ConfigWrite { edits: write.edits },
                Ctx::Write(write.subject),
            );
        }
        self.edits.push(edit);
    }

    /// The config file has what a dialog saved.
    fn saved(&mut self, subject: persist::Subject) {
        match subject {
            persist::Subject::Theme(name) => self.facts.syntax_theme = Some(name),
            persist::Subject::Memories(memories) => self.facts.memories = Some(memories),
            // The chat's status facts hold these: the host sends them again.
            persist::Subject::StatusLine { .. } | persist::Subject::Title(_) => {}
        }
    }

    /// The write was refused: the session stays as it was. `restore_runtime_theme_from_config`:
    /// the theme in force is the config's again.
    fn refused(&mut self, subject: &persist::Subject) {
        if let persist::Subject::Theme(_) = subject {
            self.syntax_theme = self.facts.syntax_theme.clone();
        }
    }

    fn features_discovered(&mut self, generation: u64, features: Option<Vec<catalog::Feature>>) {
        let Some(dialog) = self.experimental_mut() else {
            return;
        };
        if dialog.generation != generation || !dialog.loading {
            return;
        }
        dialog.loading = false;
        match features {
            Some(features) => {
                dialog.features.extend(features);
                dialog.status = if dialog.features.is_empty() {
                    FeaturesStatus::NoExperiments
                } else {
                    FeaturesStatus::Quiet
                };
            }
            None => dialog.status = FeaturesStatus::Unavailable,
        }
    }

    fn features_saved(&mut self, generation: u64, result: Result<Written, String>) {
        let Some(dialog) = self.experimental_mut() else {
            return;
        };
        if dialog.generation != generation || !dialog.saving {
            return;
        }
        dialog.saving = false;
        match result {
            Ok(written) => {
                for feature in &mut dialog.features {
                    if let Some((_, enabled)) =
                        written.features.iter().find(|(key, _)| *key == feature.key)
                    {
                        feature.enabled = *enabled;
                    }
                }
                dialog.unconfirmed.clear();
                dialog.confirmed += 1;
                dialog.status = match written.warning {
                    Some(warning) => FeaturesStatus::Message(warning),
                    None => FeaturesStatus::Quiet,
                };
            }
            Err(error) => dialog.status = FeaturesStatus::Message(error),
        }
    }
}

impl super::Feature for Settings {
    const NAME: &'static str = "settings";
    const SEED: Option<fn(serde_json::Value) -> Result<Fact, SeedError>> = Some(seed::seed);

    type State = State;
    type ThreadPart = ();
    type Target = Never<Self>;
    type Intent = Intent;
    type ThreadIntent = Never<Self>;
    type Command = Never<Self>;
    type Reply = Never<Self>;
    type Ctx = Ctx;
    type Note = Note;
    type Fact = Fact;
    type Wish = Never<Self>;
    type Interest = Never<Self>;

    fn intent(state: &mut State, intent: Intent, cx: &mut AppCx<'_>) {
        match intent {
            Intent::Open(which) => state.open(which, cx),
            Intent::SyntaxTheme(theme) => {
                state.syntax_theme = theme.or_else(|| state.facts.syntax_theme.clone());
            }
            Intent::Request(edit) => state.request(edit, cx),
        }
    }

    /// The config file's answer to a write of a dialog: the session follows what it saved, or
    /// stays as it was and says why.
    fn effect_done(state: &mut State, ctx: Ctx, done: Done, cx: &mut AppCx<'_>) {
        let (Ctx::Write(subject), Done::Config(result)) = (ctx, done) else {
            return;
        };
        match result {
            Ok(()) => state.saved(subject),
            Err(error) => {
                state.refused(&subject);
                cx.note(
                    None,
                    Note::SaveFailed {
                        setting: subject.setting(),
                        error,
                    },
                );
            }
        }
    }

    fn note_tone(_note: &Note) -> Tone {
        Tone::Error
    }

    fn on_fact(state: &mut State, fact: Fact, _cx: &mut AppCx<'_>) {
        match fact {
            Fact::Offered(offer) => state.offer(offer),
            Fact::Facts(facts) => state.facts_changed(facts),
            Fact::FeaturesDiscovered {
                generation,
                features,
            } => state.features_discovered(generation, features),
            Fact::FeaturesSaved { generation, result } => state.features_saved(generation, result),
        }
    }

    fn wire(command: &Never<Self>, _thread: &ThreadId, _id: RequestId) -> ClientRequest {
        command.absurd()
    }

    fn decode(command: &Never<Self>, _result: serde_json::Value) -> Result<Never<Self>, RpcError> {
        command.absurd()
    }
}
