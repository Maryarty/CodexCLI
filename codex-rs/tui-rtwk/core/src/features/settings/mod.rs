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

pub mod args;
mod catalog;
pub mod hooks;
pub mod items;
pub mod persist;
mod seed;
pub mod wire;

use std::collections::BTreeMap;

use codex_app_server_protocol::{ClientRequest, RequestId};

use super::AppCx;
use crate::app::effects::{Ask, Done};
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::screens::SeedError;
use crate::threads::Tone;

pub use catalog::*;
pub use persist::Setting;
pub use wire::{Command, Reply, Skill};

/// What the server said, as the notes quote it.
fn text(error: &RpcError) -> String {
    match error {
        RpcError::Remote(message) | RpcError::Malformed(message) | RpcError::Prepare(message) => {
            message.clone()
        }
        RpcError::Mismatch => String::from("unexpected answer"),
        RpcError::Provider(error) => format!("{error:?}"),
    }
}

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
    /// The arguments of a command were refused: the note says why.
    Refused(args::Refusal),
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
    /// The experimental features were written; the listing that follows is read back.
    FeaturesWrite {
        generation: u64,
        updates: Vec<(String, bool)>,
    },
    /// A page of the experimental features: what the pages before it gave and the cursors
    /// they were read with.
    FeaturesPage {
        generation: u64,
        after: After,
        found: Vec<catalog::Feature>,
        cursors: Vec<String>,
    },
    Skills,
    SkillWrite {
        path: String,
        enabled: bool,
    },
    Hooks,
    HookWrite {
        key: String,
        enabled: bool,
    },
    HookTrust {
        many: bool,
    },
}

/// What a listing of the experimental features is for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum After {
    /// The dialog was opened: the features are its catalogue.
    Open,
    /// A write was answered: the values it set, as the server reads them again.
    Readback(Vec<(String, bool)>),
}

/// What the feature says in the history: a write the config file refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Note {
    /// `Failed to save {setting}: {error}`.
    SaveFailed { setting: Setting, error: String },
    /// `Failed to update skill config for {path}: {error}`.
    SkillWriteFailed { path: String, error: String },
    /// `Failed to load skills: {error}` (the listing failed).
    SkillsFailed { error: String },
    /// `Failed to update hook config: {error}`.
    HookWriteFailed { error: String },
    /// `Failed to trust hook: {error}` / `Failed to trust hooks: {error}`.
    HookTrustFailed { many: bool, error: String },
    /// `Failed to load hooks: {error}`.
    HooksFailed { error: String },
    /// `/experimental` before the session has a thread to list the features for.
    ExperimentalUnavailable,
    /// `Saved TUI mode: {mode}. Restart Codex to apply; launch overrides still apply.`
    TuiModeSaved { fullscreen: bool },
    /// The arguments of `/title`, `/statusline` or `/model` do not pass.
    Refused(args::Refusal),
}

impl Note {
    /// Whether the oracle says it with `add_info_message`.
    pub fn is_info(&self) -> bool {
        matches!(
            self,
            Note::ExperimentalUnavailable | Note::TuiModeSaved { .. }
        )
    }
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
    /// The last open of the experimental dialog: what an answer is checked against.
    pub generation: u64,
    /// The hook switches on their way, by key: what was asked for while one was in flight
    /// (`pending_hook_enabled_writes`: only the last value is sent, and after the answer).
    pub hook_writes: BTreeMap<String, Option<bool>>,
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
    fn open(&mut self, which: Which, cx: &mut AppCx<'_>) {
        let offer = match which {
            Which::Theme => Offer::Theme(self.facts.theme_picker()),
            Which::Memories => match self.facts.memories {
                Some(facts) => Offer::Memories(Memories {
                    use_memories: facts.use_memories,
                    generate_memories: facts.generate_memories,
                }),
                None => return,
            },
            Which::StatusLine => Offer::Setup(items::status_line(&cx.view().screens().chat.status)),
            Which::Title => Offer::Setup(items::title(&cx.view().screens().chat.status)),
            Which::Experimental => return self.open_features(cx),
            Which::Skills => {
                cx.ask(
                    None,
                    Command::Skills {
                        cwd: cx.view().working_directory(None),
                    },
                    Ctx::Skills,
                );
                return;
            }
            Which::Hooks => {
                cx.ask(
                    None,
                    Command::Hooks {
                        cwd: cx.view().working_directory(None),
                    },
                    Ctx::Hooks,
                );
                return;
            }
            // The host's catalogues: they arrive as facts.
            Which::Config | Which::Keymap => return,
        };
        self.offer(offer);
    }

    /// `/experimental`: the dialog opens loading and the first page is asked for the active
    /// thread (the server computes the enablement from that thread's config); with none the
    /// dialog is not offered.
    fn open_features(&mut self, cx: &mut AppCx<'_>) {
        let Some(thread) = cx.view().active().cloned() else {
            self.offers.remove(&Which::Experimental);
            cx.note(None, Note::ExperimentalUnavailable);
            return;
        };
        self.generation += 1;
        let generation = self.generation;
        self.offer(Offer::Experimental(Experimental::new(
            Vec::new(),
            true,
            generation,
        )));
        cx.ask(
            Some(&thread),
            Command::Features { cursor: None },
            Ctx::FeaturesPage {
                generation,
                after: After::Open,
                found: Vec::new(),
                cursors: Vec::new(),
            },
        );
    }

    /// A page of the experimental features arrived: the next one is asked for, or the listing
    /// is over (a cursor that repeats, or more pages than `MAX_PAGES`, is a failed listing).
    fn features_page(&mut self, mut ctx: Ctx, result: Result<Reply, RpcError>, cx: &mut AppCx<'_>) {
        let Ctx::FeaturesPage {
            generation,
            after,
            found,
            cursors,
        } = &mut ctx
        else {
            return;
        };
        let (generation, after) = (*generation, after.clone());
        let page = match result {
            Ok(Reply::Features { features, next }) => Ok((features, next)),
            Ok(_) => Err("unexpected answer".to_owned()),
            Err(error) => Err(text(&error)),
        };
        let outcome = page.and_then(|(features, next)| {
            for feature in features {
                if !found.iter().any(|known| known.key == feature.key) {
                    found.push(feature);
                }
            }
            match next {
                None => Ok(None),
                Some(cursor)
                    if cursors.contains(&cursor)
                        || cursors.len() >= usize::from(wire::MAX_PAGES) =>
                {
                    Err(format!("repeated cursor {cursor}"))
                }
                Some(cursor) => Ok(Some(cursor)),
            }
        });
        match outcome {
            Ok(Some(cursor)) => {
                cursors.push(cursor.clone());
                let thread = cx.view().active().cloned();
                cx.ask(
                    thread.as_ref(),
                    Command::Features {
                        cursor: Some(cursor),
                    },
                    ctx,
                );
            }
            Ok(None) => {
                let Ctx::FeaturesPage { found, .. } = ctx else {
                    return;
                };
                match after {
                    After::Open => self.features_discovered(generation, Some(found)),
                    After::Readback(updates) => {
                        let features = updates
                            .iter()
                            .filter_map(|(key, _)| {
                                found
                                    .iter()
                                    .find(|feature| &feature.key == key)
                                    .map(|feature| (key.clone(), feature.enabled))
                            })
                            .collect();
                        self.features_saved(
                            generation,
                            Ok(Written {
                                features,
                                warning: None,
                            }),
                        );
                    }
                }
            }
            Err(error) => match after {
                After::Open => self.features_discovered(generation, None),
                After::Readback(_) => self.features_saved(generation, Err(error)),
            },
        }
    }

    /// The skills the server listed are the checklist of `/skills`.
    fn skills_listed(&mut self, skills: Vec<Skill>) {
        let items = skills
            .into_iter()
            .map(|skill| SetupItem {
                id: skill.path,
                label: skill.display_name,
                description: skill.description,
                enabled: skill.enabled,
                orderable: false,
                section_break_after: false,
                placeholder: skill.name,
            })
            .collect();
        self.offer(Offer::Setup(Setup {
            kind: SetupKind::Skills,
            items,
            values: BTreeMap::new(),
        }));
    }

    /// A skill was switched on the server: the checklist the dialogs are offered follows.
    fn skill_written(&mut self, path: &str, enabled: bool) {
        if let Some(Offer::Setup(setup)) = self.offers.get_mut(&Which::Skills)
            && let Some(item) = setup.items.iter_mut().find(|item| item.id == path)
        {
            item.enabled = enabled;
        }
    }

    /// `set_hook_enabled`: one write per hook at a time; what was asked while it was on its
    /// way is the next one.
    fn hook_enabled(&mut self, key: String, enabled: bool, cx: &mut AppCx<'_>) {
        if let Some(queued) = self.hook_writes.get_mut(&key) {
            *queued = Some(enabled);
            return;
        }
        self.hook_writes.insert(key.clone(), None);
        Self::send_hook_enabled(key, enabled, cx);
    }

    fn send_hook_enabled(key: String, enabled: bool, cx: &mut AppCx<'_>) {
        let value = serde_json::json!({ key.clone(): { "enabled": enabled } });
        cx.ask(
            None,
            Command::HooksWrite { value },
            Ctx::HookWrite { key, enabled },
        );
    }

    /// `HookEnabledSet`: a value queued behind the answer is sent when it differs from what was
    /// written or the write failed; else the answer is the end of it (and an error is said).
    fn hook_enabled_set(
        &mut self,
        key: String,
        enabled: bool,
        result: Result<(), String>,
        cx: &mut AppCx<'_>,
    ) {
        let queued = self.hook_writes.get_mut(&key).and_then(Option::take);
        match queued {
            Some(queued) if result.is_err() || queued != enabled => {
                Self::send_hook_enabled(key, queued, cx);
            }
            _ => {
                self.hook_writes.remove(&key);
                if let Err(error) = result {
                    cx.note(None, Note::HookWriteFailed { error });
                }
            }
        }
    }

    fn hook_trust(&mut self, updates: Vec<(String, String)>, many: bool, cx: &mut AppCx<'_>) {
        let value = serde_json::Value::Object(
            updates
                .into_iter()
                .map(|(key, hash)| (key, serde_json::json!({ "trusted_hash": hash })))
                .collect(),
        );
        cx.ask(None, Command::HooksWrite { value }, Ctx::HookTrust { many });
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
                let edits = updates
                    .iter()
                    .map(|(key, enabled)| crate::app::effects::ConfigEdit {
                        key_path: format!("features.{key}"),
                        value: serde_json::json!(enabled),
                    })
                    .collect();
                cx.effect(
                    Ask::ConfigWrite { edits },
                    Ctx::FeaturesWrite {
                        generation: self.generation,
                        updates: updates.clone(),
                    },
                );
            }
            Edit::SkillToggled(path, enabled) => cx.ask(
                None,
                Command::SkillWrite {
                    path: path.clone(),
                    enabled: *enabled,
                },
                Ctx::SkillWrite {
                    path: path.clone(),
                    enabled: *enabled,
                },
            ),
            Edit::SkillsClosed => cx.ask(
                None,
                Command::Skills {
                    cwd: cx.view().working_directory(None),
                },
                Ctx::Skills,
            ),
            Edit::HookEnabled { key, enabled } => self.hook_enabled(key.clone(), *enabled, cx),
            Edit::HookTrusted { key, current_hash } => {
                self.hook_trust(vec![(key.clone(), current_hash.clone())], false, cx);
            }
            Edit::HooksTrusted(updates) => self.hook_trust(updates.clone(), true, cx),
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
            persist::Subject::TuiMode(_) | persist::Subject::Keymap => {}
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
    type Target = super::Never<Self>;
    type Intent = Intent;
    type ThreadIntent = super::Never<Self>;
    type Command = Command;
    type Reply = Reply;
    type Ctx = Ctx;
    type Note = Note;
    type Fact = Fact;
    type Wish = super::Never<Self>;
    type Interest = super::Never<Self>;

    fn intent(state: &mut State, intent: Intent, cx: &mut AppCx<'_>) {
        match intent {
            Intent::Open(which) => state.open(which, cx),
            Intent::SyntaxTheme(theme) => {
                state.syntax_theme = theme.or_else(|| state.facts.syntax_theme.clone());
            }
            Intent::Request(edit) => state.request(edit, cx),
            Intent::Refused(refusal) => cx.note(None, Note::Refused(refusal)),
        }
    }

    /// The config file's answer to a write of a dialog: the session follows what it saved, or
    /// stays as it was and says why.
    fn effect_done(state: &mut State, ctx: Ctx, done: Done, cx: &mut AppCx<'_>) {
        if let (
            Ctx::FeaturesWrite {
                generation,
                updates,
            },
            Done::Config(result),
        ) = (&ctx, &done)
        {
            match result {
                Ok(()) => {
                    // The write is read back: the config may hold another value than the one
                    // asked for.
                    let thread = cx.view().active().cloned();
                    cx.ask(
                        thread.as_ref(),
                        Command::Features { cursor: None },
                        Ctx::FeaturesPage {
                            generation: *generation,
                            after: After::Readback(updates.clone()),
                            found: Vec::new(),
                            cursors: Vec::new(),
                        },
                    );
                }
                Err(error) => state.features_saved(*generation, Err(error.clone())),
            }
            return;
        }
        let (Ctx::Write(subject), Done::Config(result)) = (ctx, done) else {
            return;
        };
        match result {
            Ok(()) => {
                if let persist::Subject::TuiMode(fullscreen) = &subject {
                    cx.note(
                        None,
                        Note::TuiModeSaved {
                            fullscreen: *fullscreen,
                        },
                    );
                }
                state.saved(subject);
            }
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

    fn answered(state: &mut State, ctx: Ctx, result: Result<Reply, RpcError>, cx: &mut AppCx<'_>) {
        match ctx {
            Ctx::FeaturesPage { .. } => state.features_page(ctx, result, cx),
            Ctx::Skills => match result {
                Ok(Reply::Skills(skills)) => state.skills_listed(skills),
                Ok(_) => {}
                Err(error) => cx.note(
                    None,
                    Note::SkillsFailed {
                        error: text(&error),
                    },
                ),
            },
            Ctx::SkillWrite { path, enabled } => match result {
                Ok(_) => state.skill_written(&path, enabled),
                Err(error) => cx.note(
                    None,
                    Note::SkillWriteFailed {
                        path,
                        error: text(&error),
                    },
                ),
            },
            Ctx::Hooks => match result {
                Ok(Reply::Hooks(hooks)) => state.offer(Offer::Hooks(hooks)),
                Ok(_) => {}
                Err(error) => cx.note(
                    None,
                    Note::HooksFailed {
                        error: text(&error),
                    },
                ),
            },
            Ctx::HookWrite { key, enabled } => {
                state.hook_enabled_set(
                    key,
                    enabled,
                    result.map(|_| ()).map_err(|error| text(&error)),
                    cx,
                );
            }
            Ctx::HookTrust { many } => {
                if let Err(error) = result {
                    cx.note(
                        None,
                        Note::HookTrustFailed {
                            many,
                            error: text(&error),
                        },
                    );
                }
            }
            Ctx::Write(_) | Ctx::FeaturesWrite { .. } => {}
        }
    }

    fn note_tone(note: &Note) -> Tone {
        if note.is_info() {
            Tone::Info
        } else {
            Tone::Error
        }
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

    fn wire(command: &Command, thread: &ThreadId, id: RequestId) -> ClientRequest {
        wire::request(command, thread, id)
    }

    fn decode(command: &Command, result: serde_json::Value) -> Result<Reply, RpcError> {
        wire::decode(command, result).map_err(RpcError::Malformed)
    }
}
