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
pub mod capture;
mod catalog;
pub mod hooks;
pub mod items;
pub mod keymap;
mod keymap_actions;
pub mod persist;
pub mod resume;
pub mod rpcs;
mod seed;

use std::collections::BTreeMap;

use codex_app_server_protocol::{ClientRequest, RequestId};

use super::{AppCx, PublicVerb, Refusal};
use crate::app::effects::{Ask, Done};
use crate::commands::{
    CommandRule, DispatchCx, Need, Notice, Outcome, SlashCommand, Target, UsageKey, open,
};
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::screens::SeedError;
use crate::threads::Tone;
use crate::threads::queue::QueueOperation;

pub use catalog::*;
pub use persist::Setting;
pub use rpcs::{Command, Reply, Skill};

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
    /// The status editor opened while the thread drains this queued command.
    OpenQueuedStatusLine(QueueOperation),
    /// `/keymap debug`: the keymap editor opens on the keypress inspector.
    Inspect,
    /// The syntax theme in force (the picker's preview, and the restore on cancel: none is the
    /// config's own).
    SyntaxTheme(Option<String>),
    /// What a dialog asks of the app (`AppEvent`s of the oracle): recorded, and written to the
    /// config file when it is a write.
    Request(Edit),
    /// A shortcut was captured for an action of the keymap picker: the key as the config
    /// writes it, and what it is for. The feature applies it to the config the host read,
    /// checks that the keymap still resolves, and writes it (`Edit::KeymapBound`'s write).
    KeymapCaptured {
        context: String,
        action: String,
        key: String,
        intent: capture::Intent,
    },
    /// The custom binding of an action of the keymap picker is removed: the default comes back
    /// (`KeymapCleared`).
    KeymapCleared { context: String, action: String },
}

/// What a command with arguments asks of the page (ADR-013: a one-shot signal the surface's
/// presenter shows once).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Wish {
    /// The dialog of this kind has its catalogue now (`Intent::Open` built it, or the server's
    /// answer did): the presenter shows its sheet. The keymap editor opens on its inspector
    /// when `inspect` says so.
    Show {
        which: Which,
        inspect: bool,
        queued: bool,
    },
    /// The captured key cannot be bound: the keymap with it does not resolve (the conflict
    /// sheet: pick another key, or cancel).
    KeymapConflict {
        context: String,
        action: String,
        key: String,
        intent: capture::Intent,
        error: String,
    },
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
            ..Facts::default()
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
    /// The listing a closed skills toggle reads back (the dialog is not shown again).
    SkillsRefresh,
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
    /// A preference of the configuration panel was written: its row follows once the file has
    /// it.
    Preference(Edit),
    /// A captured shortcut was written: what the session says once the file has it.
    KeymapWrite {
        saved: Note,
    },
    /// A page of the listing `/resume <label>` looks the label up in.
    Resume {
        lookup: resume::Lookup,
    },
    /// The thread `/resume <id or label>` asked the app to open (`AppCx::open_thread`, the one
    /// door of ADR-003): the app reads the id it does not know, so a session the server has not
    /// got is answered here.
    Opened {
        name: String,
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
    /// What a visit of `/skills` changed: `Enabled {enabled} skill(s)` and/or `disabled
    /// {disabled} skill(s)`.
    SkillsChanged { enabled: usize, disabled: usize },
    /// `/skills` > "Enable/Disable Skills" with no skill to toggle.
    NoSkills,
    /// `{label} setting saved on the server for new threads. ...` (a feature was turned on).
    FeatureEnabled(Enable),
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
    /// `No saved chat found matching '{name}'.`
    NoSavedChat { name: String },
    /// `Multiple sessions match '{name}' (including {first} and {second}); use a session UUID to
    /// disambiguate.`
    AmbiguousSession {
        name: String,
        first: String,
        second: String,
    },
    /// `Cannot verify a unique session label across server pages; matching session UUID: {id}.
    /// Use it only if this is the session you want.`
    UnverifiedSession { id: String },
    /// `Failed to list sessions while resolving session label: {error}`.
    ResumeFailed { error: String },
    /// What a captured shortcut did (`Remapped`, `Added`, `Replaced`, `No change`).
    KeymapEdited {
        context: String,
        action: String,
        key: String,
        said: capture::Said,
    },
    /// `Removed custom shortcut for `{context}.{action}`.`
    KeymapRemoved { context: String, action: String },
    /// `Unknown keymap action `{context}.{action}`. Reopen /keymap and choose an action.`
    KeymapUnknown { context: String, action: String },
    /// `` `{context}.{action}` no longer uses `{old_key}`. Reopen /keymap and choose a binding
    /// again.``
    KeymapStale {
        context: String,
        action: String,
        old_key: String,
    },
}

impl Note {
    /// Whether the oracle says it with `add_info_message`.
    pub fn is_info(&self) -> bool {
        matches!(
            self,
            Note::ExperimentalUnavailable
                | Note::NoSkills
                | Note::TuiModeSaved { .. }
                | Note::KeymapEdited { .. }
                | Note::KeymapRemoved { .. }
                | Note::SkillsChanged { .. }
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
    /// The last open of the experimental dialog: what an answer is checked against.
    pub generation: u64,
    /// The hook switches on their way, by key: what was asked for while one was in flight
    /// (`pending_hook_enabled_writes`: only the last value is sent, and after the answer).
    pub hook_writes: BTreeMap<String, Option<bool>>,
    /// The skills' switches when `/skills` was opened, by path: what its summary compares to.
    pub skills_opened: BTreeMap<String, bool>,
}

impl State {
    /// The catalogue of the experimental features, as the server gave it.
    fn experimental_mut(&mut self) -> Option<&mut Experimental> {
        match self.offers.get_mut(&Which::Experimental) {
            Some(Offer::Experimental(dialog)) => Some(dialog),
            _ => None,
        }
    }

    fn config_panel_mut(&mut self) -> Option<&mut ConfigPanel> {
        match self.offers.get_mut(&Which::Config) {
            Some(Offer::Config(panel)) => Some(panel),
            _ => None,
        }
    }

    /// The file has a preference of the panel (`save_config_preference`): its row shows it, and
    /// the panel says what the oracle says (`Saved · applies to new composers`), or why not.
    fn preference_saved(&mut self, edit: &Edit, result: Result<(), String>) {
        let Some(panel) = self.config_panel_mut() else {
            return;
        };
        let error = match result {
            Ok(()) => {
                panel.notice = None;
                match edit {
                    Edit::Toggled(SettingId::Animations, on) => panel.effects.animations = *on,
                    Edit::Toggled(SettingId::ComposerStars, on) => {
                        panel.effects.composer_stars = *on;
                    }
                    Edit::Toggled(id, on) => {
                        if let Some(row) = panel.rows.iter_mut().find(|row| row.id == *id) {
                            row.value = Value::Toggle(*on);
                        }
                        if *id == SettingId::StartInVimMode {
                            panel.notice = Some(PanelNotice::AppliesToNewComposers);
                        }
                    }
                    Edit::Chosen(id, choice) => {
                        if let Some(row) = panel.rows.iter_mut().find(|row| row.id == *id) {
                            row.value = Value::Choice(choice_label(*id, choice.as_deref()));
                        }
                    }
                    _ => {}
                }
                return;
            }
            Err(error) => error,
        };
        panel.notice = Some(PanelNotice::CouldNotSave(error));
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
        // A dialog that is offered follows the config (the oracle refreshes the panel after a
        // save).
        if self.offers.contains_key(&Which::Config)
            && let Some(panel) = self.facts.config_panel()
        {
            self.offers.insert(Which::Config, Offer::Config(panel));
        }
        if self.offers.contains_key(&Which::Keymap)
            && let Some(picker) = self.facts.keymap_picker()
        {
            self.offers.insert(Which::Keymap, Offer::Keymap(picker));
        }
    }

    /// The catalogue of the dialog of kind `which`, built from what the app holds.
    fn open(&mut self, which: Which, inspect: bool, cx: &mut AppCx<'_>) {
        let offer = match which {
            Which::SkillsMenu => Offer::SkillsMenu,
            Which::EnableMemories => Offer::Enable(Enable::Memories),
            Which::EnableSubagents => Offer::Enable(Enable::Subagents),
            Which::Theme => Offer::Theme(self.facts.theme_picker()),
            Which::Memories => match self.facts.memories {
                Some(facts) => Offer::Memories(Memories {
                    use_memories: facts.use_memories,
                    generate_memories: facts.generate_memories,
                }),
                // The memory tool is off: the prompt that offers to turn it on.
                None => Offer::Enable(Enable::Memories),
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
            // The host's catalogues, as the facts have them (none yet: what was offered stays).
            Which::Config => match self.facts.config_panel() {
                Some(panel) => Offer::Config(panel),
                None => return self.show_offered(which, inspect, cx),
            },
            Which::Keymap => match self.facts.keymap_picker() {
                Some(picker) => Offer::Keymap(picker),
                None => return self.show_offered(which, inspect, cx),
            },
        };
        let shown = offer.which();
        self.offer(offer);
        self.show(shown, inspect, cx);
    }

    /// The host has no catalogue in its facts (yet): what it offered stays, and shows.
    fn show_offered(&mut self, which: Which, inspect: bool, cx: &mut AppCx<'_>) {
        if self.offers.contains_key(&which) {
            self.show(which, inspect, cx);
        }
    }

    /// The dialog of kind `which` has its catalogue: its sheet is wished for (ADR-013), once.
    fn show(&mut self, which: Which, inspect: bool, cx: &mut AppCx<'_>) {
        let scope = cx.view().active().cloned().unwrap_or_default();
        cx.wish(
            scope,
            Wish::Show {
                which,
                inspect,
                queued: false,
            },
        );
    }

    fn open_queued_status_line(&mut self, operation: QueueOperation, cx: &mut AppCx<'_>) {
        let which = Which::StatusLine;
        let wish = Wish::Show {
            which,
            inspect: false,
            queued: true,
        };
        if cx.wish_for_queued_operation(operation, wish) {
            self.offer(Offer::Setup(items::status_line(
                &cx.view().screens().chat.status,
            )));
        }
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
        self.show(Which::Experimental, false, cx);
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
                        || cursors.len() >= usize::from(rpcs::MAX_PAGES) =>
                {
                    Err(cursor)
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
    fn skills_listed(&mut self, skills: Vec<Skill>, show: Option<&mut AppCx<'_>>) {
        self.skills_opened = skills
            .iter()
            .map(|skill| (skill.path.clone(), skill.enabled))
            .collect();
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
        if let Some(cx) = show {
            if self.skills_opened.is_empty() {
                self.offers.remove(&Which::Skills);
                cx.note(None, Note::NoSkills);
            } else {
                self.show(Which::Skills, false, cx);
            }
        }
    }

    /// `handle_manage_skills_closed`: how many skills the visit switched on and off (a skill
    /// put back as it was counts for nothing); none, no line.
    fn skills_summary(&self, cx: &mut AppCx<'_>) {
        let Some(Offer::Setup(setup)) = self.offers.get(&Which::Skills) else {
            return;
        };
        let (mut enabled, mut disabled) = (0, 0);
        for item in &setup.items {
            match self.skills_opened.get(&item.id) {
                Some(was) if *was != item.enabled => {
                    if item.enabled {
                        enabled += 1;
                    } else {
                        disabled += 1;
                    }
                }
                _ => {}
            }
        }
        if enabled + disabled > 0 {
            cx.note(None, Note::SkillsChanged { enabled, disabled });
        }
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

    /// A slash command with arguments.
    fn slash(
        &mut self,
        command: SlashCommand,
        args: &str,
        thread: Option<ThreadId>,
        cx: &mut AppCx<'_>,
    ) {
        let verb = match command {
            SlashCommand::Hooks => return self.open(Which::Hooks, false, cx),
            SlashCommand::Resume => return Self::resume(args.trim(), cx),
            SlashCommand::Title => args::title_with(args, cx.view().facts().rate_limits())
                .map(PublicVerb::StatusOverride),
            SlashCommand::Statusline => {
                args::status_line_with(args, cx.view().facts().rate_limits())
                    .map(PublicVerb::StatusOverride)
            }
            SlashCommand::Model => {
                let screens = cx.view().screens();
                args::model(
                    args,
                    thread.is_some(),
                    &crate::screens::select::models::Current::from_facts(&screens.chat.status),
                    &screens.select.catalog.presets,
                )
                .map(|pick| PublicVerb::PickModel(pick.selection))
            }
            _ => return,
        };
        match verb {
            Ok(verb) => cx.verb(verb),
            Err(refusal) => cx.note(None, Note::Refused(refusal)),
        }
    }

    /// `/resume <name or id>`: an id is opened (the app reads it first when it does not know it),
    /// a label is looked for in the listing.
    fn resume(name: &str, cx: &mut AppCx<'_>) {
        if name.is_empty() {
            return;
        }
        if resume::is_session_id(name) {
            let opened = Ctx::Opened {
                name: name.to_owned(),
            };
            cx.open_thread(&ThreadId::new(name), opened);
        } else {
            cx.ask(
                None,
                Command::Sessions { cursor: None },
                Ctx::Resume {
                    lookup: resume::Lookup::new(name),
                },
            );
        }
    }

    /// A page of the listing arrived: the next one is asked for, or the lookup ends.
    fn resume_page(
        mut lookup: resume::Lookup,
        result: Result<Reply, RpcError>,
        cx: &mut AppCx<'_>,
    ) {
        let step = match result {
            Ok(Reply::Sessions { rows, next }) => lookup.page(&rows, next),
            Ok(_) => return,
            Err(error) => {
                let error = text(&error);
                return cx.note(None, Note::ResumeFailed { error });
            }
        };
        match step {
            resume::Step::Next(cursor) => cx.ask(
                None,
                Command::Sessions {
                    cursor: Some(cursor),
                },
                Ctx::Resume { lookup },
            ),
            resume::Step::Done(verdict) => Self::resumed(&lookup.name, verdict, cx),
        }
    }

    /// How the lookup ended: the thread to open, or the line saying why not.
    fn resumed(name: &str, verdict: resume::Verdict, cx: &mut AppCx<'_>) {
        match verdict {
            resume::Verdict::Found(id) => {
                let opened = Ctx::Opened {
                    name: name.to_owned(),
                };
                cx.open_thread(&ThreadId::new(id), opened);
            }
            resume::Verdict::Missing => cx.note(
                None,
                Note::NoSavedChat {
                    name: name.to_owned(),
                },
            ),
            resume::Verdict::Ambiguous { first, second } => cx.note(
                None,
                Note::AmbiguousSession {
                    name: name.to_owned(),
                    first,
                    second,
                },
            ),
            resume::Verdict::Paginated(id) => cx.note(None, Note::UnverifiedSession { id }),
        }
    }

    /// A key was captured for `context.action`: the edit it makes of the config, checked
    /// against the keymap it would give, then written; a refusal is the line or the conflict
    /// sheet saying why.
    fn keymap_captured(
        &mut self,
        context: String,
        action: String,
        key: String,
        intent: capture::Intent,
        cx: &mut AppCx<'_>,
    ) {
        let said = |said| Note::KeymapEdited {
            context: context.clone(),
            action: action.clone(),
            key: key.clone(),
            said,
        };
        match capture::captured(&self.facts.keymap_spec, &context, &action, &key, &intent) {
            Ok(capture::Outcome::Unchanged) => cx.note(None, said(capture::Said::Unchanged)),
            Ok(capture::Outcome::Updated {
                spec: _,
                keys,
                said: done,
            }) => {
                let edit = Edit::KeymapBound {
                    context: context.clone(),
                    action: action.clone(),
                    keys,
                };
                if let Some(write) = persist::write_of(&edit) {
                    cx.effect(
                        Ask::ConfigWrite { edits: write.edits },
                        Ctx::KeymapWrite { saved: said(done) },
                    );
                }
            }
            Err(capture::Failure::UnknownAction) => {
                cx.note(None, Note::KeymapUnknown { context, action });
            }
            Err(capture::Failure::NoLongerUses { old_key }) => cx.note(
                None,
                Note::KeymapStale {
                    context,
                    action,
                    old_key,
                },
            ),
            Err(capture::Failure::Conflict(error)) => {
                let scope = cx.view().active().cloned().unwrap_or_default();
                cx.wish(
                    scope,
                    Wish::KeymapConflict {
                        context,
                        action,
                        key,
                        intent,
                        error,
                    },
                );
            }
        }
    }

    /// The custom binding of `context.action` is removed from the config (`apply_keymap_clear`).
    fn keymap_cleared(&mut self, context: String, action: String, cx: &mut AppCx<'_>) {
        if keymap::action(&context, &action).is_none() {
            cx.note(None, Note::KeymapUnknown { context, action });
            return;
        }
        let edit = Edit::KeymapCleared {
            context: context.clone(),
            action: action.clone(),
        };
        if let Some(write) = persist::write_of(&edit) {
            cx.effect(
                Ask::ConfigWrite { edits: write.edits },
                Ctx::KeymapWrite {
                    saved: Note::KeymapRemoved { context, action },
                },
            );
        }
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
                        key_path: ["features.", key.as_str()].concat(),
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
            Edit::SkillsClosed => {
                self.skills_summary(cx);
                cx.ask(
                    None,
                    Command::Skills {
                        cwd: cx.view().working_directory(None),
                    },
                    Ctx::SkillsRefresh,
                );
            }
            Edit::FeatureEnabled(_) => {}
            Edit::Toggled(..) | Edit::Chosen(..) => {
                if let Some(write) = persist::preference_write(&edit) {
                    if let Some(panel) = self.config_panel_mut() {
                        panel.notice = None;
                    }
                    cx.effect(
                        Ask::ConfigWrite { edits: vec![write] },
                        Ctx::Preference(edit.clone()),
                    );
                }
            }
            Edit::TestNotification => {
                if let Some(panel) = self.config_panel_mut() {
                    panel.notice = Some(if panel.notice == Some(PanelNotice::TestScheduled) {
                        PanelNotice::TestAlreadyScheduled
                    } else {
                        PanelNotice::TestScheduled
                    });
                }
            }
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
    }

    /// The config file has what a dialog saved.
    fn saved(&mut self, subject: persist::Subject) {
        match subject {
            persist::Subject::Theme(name) => self.facts.syntax_theme = Some(name),
            persist::Subject::Memories(memories) => self.facts.memories = Some(memories),
            persist::Subject::TuiMode(_)
            | persist::Subject::Enable(_)
            | persist::Subject::Keymap
            | persist::Subject::Pet(_)
            | persist::Subject::RateLimitPrompt => {}
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

fn keymap_arguments(command: SlashCommand, args: &str, _: &DispatchCx) -> Outcome {
    if args.eq_ignore_ascii_case("debug") {
        open(Target::Settings, command, "debug")
    } else {
        Outcome::Notice(Notice::Usage(UsageKey::Keymap))
    }
}

impl super::Feature for Settings {
    const NAME: &'static str = "settings";
    const SEED: Option<fn(serde_json::Value) -> Result<Fact, SeedError>> = Some(seed::seed);
    const RUNS: &'static [SlashCommand] = &[
        SlashCommand::Hooks,
        SlashCommand::Title,
        SlashCommand::Statusline,
        SlashCommand::Model,
        SlashCommand::Resume,
    ];
    const COMMAND_RULES: &'static [CommandRule] = &[
        CommandRule::new(SlashCommand::Skills).opens(Target::Settings),
        CommandRule::new(SlashCommand::Config).opens(Target::Settings),
        CommandRule::new(SlashCommand::Theme).opens(Target::Settings),
        CommandRule::new(SlashCommand::Title).opens(Target::Settings),
        CommandRule::new(SlashCommand::Statusline).opens(Target::Settings),
        CommandRule::new(SlashCommand::Experimental).opens(Target::Settings),
        CommandRule::new(SlashCommand::Memories).opens(Target::Settings),
        CommandRule::new(SlashCommand::Keymap)
            .opens(Target::Settings)
            .arguments(keymap_arguments),
        CommandRule::new(SlashCommand::Tui).opens(Target::Settings),
        CommandRule::new(SlashCommand::Model)
            .requiring(Need::ThreadWhenBare(|_| Notice::ModelStartup))
            .opens(Target::Select),
        CommandRule::new(SlashCommand::Resume)
            .requiring(Need::NoRunningTask)
            .opens(Target::Resume),
    ];

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
    type Wish = Wish;
    type Interest = super::Never<Self>;
    type Event = super::Never<Self>;

    /// `/hooks`, `/title`, `/statusline`, `/model` and `/resume` with what followed the name: the
    /// arguments are checked here; what they ask is a wish for the page's presenter (the chat's
    /// status override, a model pick for this session), what they cannot be is a note.
    fn slash(state: &mut State, slash: super::Slash, cx: &mut AppCx<'_>) {
        state.slash(slash.command, &slash.args, slash.source, cx);
    }

    fn intent(state: &mut State, intent: Intent, cx: &mut AppCx<'_>) {
        match intent {
            Intent::Open(which) => state.open(which, false, cx),
            Intent::OpenQueuedStatusLine(operation) => state.open_queued_status_line(operation, cx),
            Intent::Inspect => state.open(Which::Keymap, true, cx),
            Intent::SyntaxTheme(theme) => {
                state.syntax_theme = theme.or_else(|| state.facts.syntax_theme.clone());
            }
            Intent::Request(edit) => state.request(edit, cx),
            Intent::KeymapCaptured {
                context,
                action,
                key,
                intent,
            } => state.keymap_captured(context, action, key, intent, cx),
            Intent::KeymapCleared { context, action } => state.keymap_cleared(context, action, cx),
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
        if let (Ctx::KeymapWrite { saved }, Done::Config(result)) = (&ctx, &done) {
            match result {
                Ok(()) => cx.note(None, saved.clone()),
                Err(error) => cx.note(
                    None,
                    Note::SaveFailed {
                        setting: Setting::Keymap,
                        error: error.clone(),
                    },
                ),
            }
            return;
        }
        if let (Ctx::Preference(edit), Done::Config(result)) = (&ctx, &done) {
            state.preference_saved(edit, result.clone());
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
                if let persist::Subject::Enable(feature) = &subject {
                    cx.note(None, Note::FeatureEnabled(*feature));
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
            Ctx::Skills | Ctx::SkillsRefresh => match result {
                Ok(Reply::Skills(skills)) => {
                    let show = (ctx == Ctx::Skills).then_some(&mut *cx);
                    state.skills_listed(skills, show)
                }
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
                Ok(Reply::Hooks(hooks)) => {
                    state.offer(Offer::Hooks(hooks));
                    state.show(Which::Hooks, false, cx);
                }
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
            Ctx::Resume { lookup } => State::resume_page(lookup, result, cx),
            Ctx::Opened { .. } => {}
            Ctx::Write(_)
            | Ctx::Preference(_)
            | Ctx::FeaturesWrite { .. }
            | Ctx::KeymapWrite { .. } => {}
        }
    }

    /// `/resume <id or label>` asked the app to open a thread: a session the server does not read
    /// is no saved chat. Another open that holds the app says nothing (the oracle's one resume
    /// in flight).
    fn thread_opened(
        _state: &mut State,
        ctx: Ctx,
        result: Result<ThreadId, Refusal>,
        cx: &mut AppCx<'_>,
    ) {
        if let (Ctx::Opened { name }, Err(Refusal::Failed(_))) = (ctx, result) {
            cx.note(None, Note::NoSavedChat { name });
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
        command.request(thread, id)
    }

    fn decode(command: &Command, result: serde_json::Value) -> Result<Reply, RpcError> {
        command.reply(result)
    }
}
