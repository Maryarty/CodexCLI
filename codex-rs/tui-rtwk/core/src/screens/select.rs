//! `select` (S14, generic selection dialog): the core half of the screen's directory. The model
//! part, its events and intents, its RPCs and their protocol wiring live here,
//! with no rtwk; the view half is `src/sheets/select/` in the UI crate. Nothing
//! outside these two directories (bar their test files) needs an edit when it
//! grows.

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;

use crate::app::AppFx;
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::screens::Kind;
use crate::threads::ThreadFx;

pub mod apps;
pub mod collaboration;
pub mod layout;
pub mod list;
pub mod models;
pub mod permission_args;
pub mod permission_shortcuts;
pub mod permissions;
pub mod persist;
pub mod quick_dials;
pub mod reasoning;
pub mod review;
pub mod worktree;
pub use list::List;
pub use models::{Catalog, Current, ModelMenu, Preset, Provider, Selection};
use models::{Effort, PlanSource, Step};
pub use permissions::{PermissionsMenu, Profiles};

pub const NAME: &str = "select";
pub const ID: &str = "S14";
pub const KIND: Kind = Kind::Sheet;

/// A question with two answers, No first: the oracle's confirmations of
/// `/archive` and `/delete` (`SelectionViewParams::confirmation`). The words
/// are the view's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Confirm {
    /// Archive the session; `running` says a turn is in progress, which the
    /// archive stops.
    Archive { running: bool },
    /// Delete the session for good.
    Delete,
}

impl Confirm {
    /// The ids of its two rows (`Intent::Choose`); No is the first row.
    pub const NO: &'static str = "no";
    pub const YES: &'static str = "yes";
}

/// A menu the generic selection dialog shows; one variant per
/// parameterisation, its rows data (the words are the view's).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Menu {
    /// A menu of the `/model` family (`models.rs`): its rows come from the
    /// catalog and the thread's settings as they were when it opened.
    Model(Box<ModelMenu>),
    /// A menu of the `/permissions` family (`permissions.rs`): the presets,
    /// the server's profiles, the full-access confirmation.
    Permissions(Box<PermissionsMenu>),
    /// `/apps`: the directory of apps, its rows `Model::apps` (loading, the
    /// failure with its Retry row, or the list).
    Apps,
    /// A confirmation: No (row 0), then Yes (row 1).
    Confirm(Confirm),
    /// A menu of `/review` (`review.rs`): the presets, the base branches, the commits and the
    /// instructions prompt; the lists come from the host.
    Review(Box<review::Menu>),
    /// A menu of the worktree pickers (`worktree.rs`): the checkout choice of `/new` and `/fork`,
    /// `/worktree`, the browser, a worktree's actions and the delete confirmation.
    Worktree(Box<worktree::Menu>),
    /// A menu of `/presets` (`quick_dials.rs`): the slot picker, the sources among the current
    /// settings, the editor and the confirmation of a change.
    Quick(Box<quick_dials::Menu>),
    /// A menu of a feature slice (ADR-012); the feature turns it into rows and settles its
    /// picks (`Feature::choose`).
    Feature(crate::features::FeatureMenu),
    /// The generic form: every option of the oracle's `SelectionViewParams`
    /// (search, tabs, disabled rows, side preview, footers) as data. A named
    /// parameterisation is a variant above whose view turns it into a `List`.
    List(Box<List>),
}

/// What the user picked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Picked {
    /// A model and effort of the `/model` family, and where it applies.
    Selection(Selection),
    /// A permission mode or profile of the `/permissions` family, and how it
    /// applies.
    Permissions(permissions::Selection),
    /// The service tier the model's tier command switched to (`default` is the one that
    /// leaves the tier).
    Tier(String),
    /// A step of a reasoning shortcut: the thread's effort for its next turns, nothing saved;
    /// in Plan mode (`plan`) the Plan override only.
    Effort { model: String, effort: Effort, plan: bool },
    /// The answer of a confirmation was Yes (No picks nothing).
    Confirmed(Confirm),
    /// An item of a generic list, by id.
    Item(String),
    /// The secondary action of an item of a generic list, by id.
    ItemSecondary(String),
    /// A multi-select list (`List::multi`) was confirmed: the ids of the
    /// items whose toggle is on, in the order of the list.
    Items(Vec<String>),
    /// An app of the directory (`Model::apps`), by id: its link dialog opens (`Apps::link`) or
    /// it says it has no link.
    App(String),
    /// A review target the `/review` menus settled on (the instructions typed, a branch, a
    /// commit, or the uncommitted changes): the app starts the review on the active thread.
    Review(crate::threads::ReviewTarget),
    /// What the worktree menus settled on; the app acts.
    Worktree(worktree::Pick),
    /// A saved preset's input, to be submitted as the user typed it (`RecallQuickDial`): the
    /// session's composer takes it.
    Recall(String),
    /// What a feature slice's menu settled on (`Feature::picked`).
    Feature(crate::features::FeaturePick),
}

/// A typed menu as rows: what the view turns into its list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Page {
    Model(models::Page),
    Permissions(permissions::Page),
    Apps(apps::Page),
}

/// What the picker says in the history (`add_info_message`/`add_error_message`):
/// data, the cell is the history's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Note {
    /// `No additional models are available right now.`
    NoAdditionalModels,
    /// `Luna model settings are unavailable; please try /model again in a moment.`
    ReserveUnavailable,
    /// `Could not load models. Try /model again.`
    ModelsNotLoaded,
    /// `Model changed to {model} {effort}` (`for this session only` when the
    /// pick was the `s` key), after the pick was applied.
    ModelChanged {
        model: String,
        effort: Option<Effort>,
        session: bool,
    },
    /// The Ultra concurrency warning of a pick
    /// (`ultra_reasoning_concurrency_warning`): the threads configured.
    UltraConcurrency(usize),
    /// `Permissions updated to {label}`.
    PermissionsUpdated(permissions::Label),
    /// `Service tier set to {tier}`, after the default was saved.
    ServiceTierSet(String),
    /// `Reasoning shortcuts are disabled until startup completes.`
    ReasoningDisabled,
    /// `Reasoning shortcuts are unavailable for {model}.`
    ReasoningUnavailable(String),
    /// `Reasoning is already at the lowest level ({effort}).` / `...highest...`
    ReasoningBound { direction: reasoning::Direction, effort: Effort },
    /// `Ultra is available under /model -> ... -> More reasoning...`: a raise stops before Ultra.
    UltraAvailable(String),
    /// `Permission selection requested: {label}`: the server took a profile
    /// pick of a thread.
    PermissionSelectionRequested(permissions::Label),
    /// `Failed to save default model: {error}`, or `Failed to set permission
    /// profile`: what was being stored and the server's text.
    SaveFailed { setting: Setting, error: String },
    /// `Saved {setting}, but a higher-priority configuration layer overrides
    /// the saved value.`
    Overridden(Setting),
    /// `Apps are disabled.` and the hint `Enable the apps feature to use $ or
    /// /apps.`
    AppsDisabled,
    /// `No apps available.`
    NoApps,
    /// The directory failed before: the error as an error cell.
    AppsFailed(String),
    /// `/model` is blocked (`model_picker_unavailable`): `The model list may be out of date.
    /// Restart the app server and TUI to refresh /model.` after a provider write, or `Saving the
    /// default provider. Try /model after the save finishes.` while it runs.
    ModelPickerUnavailable(Provider),
    /// `Managed worktrees require a local Git repository.` (an error).
    WorktreesUnavailable,
    /// `Cannot list managed worktrees: {error}` (an error).
    WorktreeListFailed(String),
    /// `Removed worktree at {root}. Thread history was kept.`
    WorktreeRemoved { root: String },
    /// `Could not remove worktree at {root}: {error}` (an error).
    WorktreeRemoveFailed { root: String, error: String },
    /// `{status} · App link unavailable`: an app without an install URL was
    /// picked.
    AppLinkUnavailable(apps::Status),
    /// `Presets are unavailable in this conversation.` (an error).
    PresetsUnavailable,
    /// `Presets are available after startup completes.`
    PresetsStarting,
    /// `Images cannot be copied into a preset. Starting with empty text.`
    PresetImages,
    /// `Linked mentions cannot be copied into a preset. Starting with empty text.`
    PresetMentions,
    /// The input is not a preset: the reason as an error.
    PresetInvalid(quick_dials::Invalid),
    /// `Preset {slot}: {reason}`: a saved preset that does not validate (an error).
    PresetBroken { slot: u8, reason: quick_dials::Invalid },
    /// `Preset {slot} is defined outside your user config. Edit its source configuration
    /// instead.` (an error).
    PresetOutside(u8),
    /// `Failed to update preset {slot}: {error}` (an error).
    PresetFailed { slot: u8, error: String },
    /// `Saved preset {slot}.`
    PresetSaved(u8),
    /// `Deleted preset {slot}.`
    PresetDeleted(u8),
    /// The arguments of `/permissions <profile> ..` do not parse (an error).
    PermissionArgs(permission_args::Error),
    /// The profile `/permissions` named cannot be picked (an error).
    PermissionRefused(permission_args::Refusal),
    /// The catalog an inline `/permissions <profile>` waited for could not be read (an error): the
    /// failure's own words.
    PermissionsFailed(permissions::Failure),
    /// `No other permission modes are available.` (the shortcut found no other mode).
    NoOtherPermissionModes,
}

impl Note {
    /// The oracle says each as information or as an error.
    pub fn tone(&self) -> crate::threads::Tone {
        use crate::threads::Tone;
        match self {
            Note::ModelsNotLoaded
            | Note::SaveFailed { .. }
            | Note::AppsFailed(_)
            | Note::WorktreesUnavailable
            | Note::WorktreeListFailed(_)
            | Note::WorktreeRemoveFailed { .. }
            | Note::PresetsUnavailable
            | Note::PresetInvalid(_)
            | Note::PresetBroken { .. }
            | Note::PresetOutside(_)
            | Note::PresetFailed { .. }
            | Note::PermissionArgs(_)
            | Note::PermissionRefused(_)
            | Note::PermissionsFailed(_) => Tone::Error,
            _ => Tone::Info,
        }
    }

    /// Whether the main transcript shows the line: the two warnings
    /// (`add_warning_message`: `visible_in_transcript: false`) are for the
    /// detailed transcript only.
    pub fn in_transcript(&self) -> bool {
        !matches!(self, Note::UltraConcurrency(_) | Note::Overridden(_))
    }
}

/// What a failed or overridden write was for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Setting {
    /// The default model and reasoning effort.
    Model,
    /// The Plan mode's reasoning effort.
    PlanEffort,
    /// The thread's permissions.
    Permissions,
    /// The default service tier.
    ServiceTier,
    /// The switch of an app (the link dialog), by id.
    App(String),
}

/// The app-scoped model state of this screen, changed only by `apply`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Model {
    /// The menus open, the one on top is shown; a menu below stays until the
    /// ones above it are closed or one of them is picked from.
    pub stack: Vec<Menu>,
    /// The models the pickers list.
    pub catalog: Catalog,
    /// The permission profiles the pickers list.
    pub profiles: Profiles,
    /// The apps `/apps` lists.
    pub apps: apps::Apps,
    /// The `/review` menus' working directory and the git list asked for.
    pub review: review::Review,
    /// The scope and the requests of the worktree menus.
    pub worktree: worktree::Worktree,
    /// The presets saved in the config and the writes asked for them.
    pub quick: quick_dials::Quick,
    /// Picks made; the effect that stores them is the settings slice's.
    pub picked: Vec<Picked>,
    /// Notes for the history; whoever shows them takes them.
    pub notes: Vec<Note>,
    /// An inline `/permissions <profile>` pick failed or was given up: what waits in the thread's
    /// queue stays unsent (`retain_input_after_failed_permission_selection`). The app takes it.
    pub hold_queue: bool,
    /// The id of the row that opened the menu above the one at this depth of
    /// the stack: Esc shows that menu again with the cursor on it (the
    /// oracle's parent view keeps its highlight).
    kept: std::collections::BTreeMap<usize, String>,
    /// How many intents were applied: a sheet opened with the intent that asks for its menu
    /// compares it to tell a menu that has not come yet from one the intent left a note for
    /// (`/model` blocked by a provider switch, or over a catalog with nothing to pick).
    applied: u64,
}

impl Model {
    /// How many intents the model has applied.
    pub fn applied(&self) -> u64 {
        self.applied
    }

    /// The menu shown now.
    pub fn menu(&self) -> Option<&Menu> {
        self.stack.last()
    }

    /// The row the shown menu opens its cursor on: the one that opened the menu above it, shown
    /// again by Esc (the oracle's parent view keeps its highlight).
    pub fn cursor_row(&self) -> Option<&str> {
        self.kept
            .get(&self.stack.len().checked_sub(1)?)
            .map(String::as_str)
    }

    /// The rows of the menu shown now, if it is one of the typed families.
    pub fn page(&self) -> Option<Page> {
        match self.menu()? {
            Menu::Model(menu) => {
                let mut page = menu.page(&self.catalog);
                let kept = self.kept.get(&(self.stack.len() - 1));
                if let Some(at) = kept.and_then(|id| page.rows.iter().position(|row| &row.id == id))
                {
                    page.initial = Some(at);
                }
                Some(Page::Model(page))
            }
            Menu::Permissions(menu) => Some(Page::Permissions(menu.page(&self.profiles))),
            Menu::Apps => Some(Page::Apps(self.apps.page())),
            Menu::Confirm(_)
            | Menu::List(_)
            | Menu::Review(_)
            | Menu::Worktree(_)
            | Menu::Quick(_)
            | Menu::Feature(_) => None,
        }
    }

    /// `/apps` (`add_connectors_output`): the directory is asked for again,
    /// and what it held before decides what the user sees.
    fn open_apps(&mut self, enabled: bool) {
        if !enabled {
            self.notes.push(Note::AppsDisabled);
            return;
        }
        let held = self.apps.cache.clone();
        self.apps.open();
        match held {
            apps::Cache::Ready(apps) if apps.is_empty() => self.notes.push(Note::NoApps),
            apps::Cache::Failed(error) => self.notes.push(Note::AppsFailed(error)),
            _ => {
                self.apps.link = None;
                self.stack = vec![Menu::Apps];
            }
        }
    }

    /// A row of the apps menu: `Retry` reloads, an app opens its link dialog (or says it has
    /// no link); with the dialog open, its actions (`apps::LINK_OPEN`, `apps::LINK_INSTALLED`).
    fn choose_app(&mut self, id: &str) {
        if self.apps.link.is_some() {
            // `complete_external_flow_and_close`: the page was visited, the directory is read
            // again and the dialog closes. Opening the page is the app's
            // (`AppEvent::OpenUrlInBrowser`): the dialog stays.
            if id == apps::LINK_INSTALLED {
                self.apps.link = None;
                self.stack.pop();
                self.apps.refresh(true);
            }
            return;
        }
        if id == apps::RETRY {
            if matches!(self.apps.cache, apps::Cache::Failed(_)) {
                self.apps.refresh(true);
            }
            return;
        }
        let Some(app) = self
            .apps
            .ready()
            .and_then(|apps| apps.iter().find(|app| app.id == id))
        else {
            return;
        };
        let status = app.status();
        let link = app.link();
        self.picked.push(Picked::App(id.to_string()));
        match link {
            Some(link) => self.apps.link = Some(link),
            None => {
                self.notes.push(Note::AppLinkUnavailable(status));
                self.stack.pop();
            }
        }
    }

    /// What a pick of the row `id` did to the stack; a menu pushed over this
    /// one remembers the row that opened it.
    fn step_from(&mut self, id: String, step: Step) {
        if matches!(step, Step::Push(_)) {
            self.kept.insert(self.stack.len() - 1, id);
        }
        self.step(step);
    }

    /// `quick_dials_available`: whether presets can be used here; a note says why not.
    fn quick_available(&mut self, gate: quick_dials::Gate) -> bool {
        match gate {
            quick_dials::Gate::Ready => true,
            quick_dials::Gate::Blocked => {
                self.notes.push(Note::PresetsUnavailable);
                false
            }
            quick_dials::Gate::Starting => {
                self.notes.push(Note::PresetsStarting);
                false
            }
        }
    }

    /// `/presets`: a menu of the presets, or a preset recalled.
    fn open_quick(&mut self, gate: quick_dials::Gate, open: quick_dials::Open) {
        use quick_dials::{Menu as Quick, Open, PickerMode};
        if !self.quick_available(gate) {
            return;
        }
        match open {
            Open::Picker(mode) => {
                self.stack = vec![Menu::Quick(Box::new(Quick::Slots { mode, dial: None }))];
            }
            Open::Save(dial) => {
                if let Err(reason) = dial.validate() {
                    self.notes.push(Note::PresetInvalid(reason));
                    return;
                }
                self.stack = vec![Menu::Quick(Box::new(Quick::Slots {
                    mode: PickerMode::Save,
                    dial: Some(dial),
                }))];
            }
            Open::Editor { target, dial, composer } => {
                let dial = dial.unwrap_or_else(|| {
                    // `composer_quick_dial_text`: images and linked mentions cannot be saved.
                    let input = if composer.images {
                        self.notes.push(Note::PresetImages);
                        String::new()
                    } else if composer.mentions {
                        self.notes.push(Note::PresetMentions);
                        String::new()
                    } else {
                        composer.text
                    };
                    quick_dials::Dial::new(input)
                });
                self.stack = vec![Menu::Quick(Box::new(Quick::Editor { target, dial }))];
            }
            Open::Prefills(settings) => {
                let rows = quick_dials::sources(&settings, &self.catalog);
                self.stack = vec![Menu::Quick(Box::new(Quick::Sources(rows)))];
            }
            Open::Recall(slot) => {
                let Some(saved) = self.quick.get(slot) else { return };
                match saved.dial.validate() {
                    Ok(()) => self.picked.push(Picked::Recall(saved.dial.input.clone())),
                    Err(reason) => self.notes.push(Note::PresetBroken { slot, reason }),
                }
            }
        }
    }

    /// `confirm_quick_dial`: the question of replacing, deleting or saving; nothing to ask when
    /// there is nothing to delete.
    fn quick_confirm(&mut self, slot: u8, new: Option<quick_dials::Dial>) {
        if let Some(Err(reason)) = new.as_ref().map(quick_dials::Dial::validate) {
            self.notes.push(Note::PresetInvalid(reason));
            return;
        }
        let old = self.quick.get(slot).map(|saved| saved.dial.clone());
        if old.is_none() && new.is_none() {
            return;
        }
        self.stack.push(Menu::Quick(Box::new(quick_dials::Menu::Confirm { slot, old, new })));
    }

    /// `save_quick_dial`: a refused write says why and shows the editor again; an allowed one is
    /// left for the app to ask of the config.
    fn quick_write(&mut self, slot: u8, dial: Option<quick_dials::Dial>) {
        match self.quick.check(slot, dial.as_ref()) {
            Ok(()) => self.quick.want(quick_dials::Write { slot, dial }),
            Err(refused) => {
                self.notes.push(match refused {
                    quick_dials::Refused::Outside(slot) => Note::PresetOutside(slot),
                    quick_dials::Refused::Invalid(reason) => Note::PresetInvalid(reason),
                });
                self.restore_editor(slot, dial);
            }
        }
    }

    /// `restore_quick_dial_editor`: what was being saved is not lost.
    fn restore_editor(&mut self, slot: u8, dial: Option<quick_dials::Dial>) {
        if let Some(dial) = dial {
            self.stack = vec![Menu::Quick(Box::new(quick_dials::Menu::Editor {
                target: quick_dials::Target::Slot(slot),
                dial,
            }))];
        }
    }

    /// `dismiss_stale_model_picker`: the model menus go, with the request they waited for.
    fn dismiss_stale_models(&mut self) {
        self.catalog.pending = None;
        self.stack.retain(|menu| !matches!(menu, Menu::Model(_)));
    }

    /// Forgets the rows kept for menus that are gone.
    fn tidy(&mut self) {
        let depth = self.stack.len();
        self.kept.retain(|at, _| *at < depth);
        if !self.stack.iter().any(|menu| matches!(menu, Menu::Review(_))) {
            self.review.forget();
        }
        if !self.stack.iter().any(|menu| matches!(menu, Menu::Worktree(_))) {
            self.worktree.forget();
        }
    }

    /// Whether the app directory knows the app `id` as installed (the setup of a plugin's apps
    /// reads it).
    pub fn app_installed(&self, id: &str) -> bool {
        self.apps
            .ready()
            .is_some_and(|apps| apps.iter().any(|app| app.id == id && app.installed))
    }

    fn step(&mut self, step: Step) {
        match step {
            Step::None => {}
            Step::Push(menu) => self.stack.push(Menu::Model(Box::new(menu))),
            Step::Replace(menu) => {
                self.stack.pop();
                self.stack.push(Menu::Model(Box::new(menu)));
            }
            Step::Done(selection) => {
                self.stack.clear();
                self.picked.push(Picked::Selection(selection));
            }
            Step::Prompt(menu) => {
                self.stack.clear();
                self.stack.push(Menu::Model(Box::new(menu)));
            }
        }
    }

    fn permissions_step(&mut self, step: permissions::Step) {
        use permissions::Step;
        match step {
            Step::None => {}
            Step::Replace(mut menu) => {
                // The catalog is the app's cache of `model/list`
                // (`model_catalog.try_list_models()`): unknown stays plain.
                if let permissions::PermissionsMenu::FullAccess { current, .. } = &mut menu {
                    let cyber = current
                        .model
                        .as_deref()
                        .is_some_and(|model| self.catalog.is_cyber(model));
                    current.cyber |= cyber;
                }
                self.stack.pop();
                self.stack.push(Menu::Permissions(Box::new(menu)));
            }
            Step::Done(selection) => {
                self.stack.clear();
                // `complete_inline_permission_profile_selection`: the pick is the settled one.
                self.profiles.inline = None;
                self.picked.push(Picked::Permissions(selection));
            }
            Step::Reopen(current) | Step::Retry(current) => self.open_permissions(current),
        }
    }

    /// An inline pick is given up (`recover_inline_permission_profile_selection`): what waits in
    /// the queue stays unsent. False when none was pending.
    fn abandon_inline(&mut self) -> bool {
        let pending = self.profiles.inline.take().is_some();
        self.hold_queue |= pending;
        pending
    }

    /// The menus were left (Esc) with an inline pick still waiting on them: it is given up
    /// (`cancel_pending_permission_profile_selection`).
    fn inline_unattended(&mut self) {
        let shown = self
            .stack
            .iter()
            .any(|menu| matches!(menu, Menu::Permissions(_)));
        if !shown {
            self.abandon_inline();
        }
    }

    /// `/permissions <profile> ..` (`select_permission_profile`): the arguments are checked, and
    /// the profile is picked at once when the catalog is held, else after it is read (the
    /// `Loading` menu shows meanwhile).
    fn permissions_args(&mut self, line: &str, mut current: permissions::Current) {
        use permission_args::Inline;
        let args = match permission_args::parse(line) {
            Ok(args) => args,
            Err(error) => {
                self.notes.push(Note::PermissionArgs(error));
                self.hold_queue = true;
                return;
            }
        };
        current.discover = true;
        self.profiles.inline = Some(Inline::Discovering { args, current: current.clone() });
        if !self.apply_inline() {
            self.open_permissions(current);
        }
    }

    /// The inline pick waiting for the catalog, applied now that it is held
    /// (`apply_pending_permission_profile`): the profile becomes the pick of the menus, or is
    /// refused with a note. False when nothing waits or the catalog is not held.
    fn apply_inline(&mut self) -> bool {
        use permission_args::Inline;
        let Some(discovery) = self.profiles.found.clone() else {
            return false;
        };
        if !matches!(self.profiles.inline, Some(Inline::Discovering { .. })) {
            return false;
        }
        let Some(Inline::Discovering { args, current }) =
            self.profiles.inline.replace(Inline::Applying)
        else {
            return false;
        };
        self.stack.clear();
        match permission_args::resolve(&args, &discovery, current.guardian) {
            Ok(selection) => self.permissions_step(permissions::settle(&current, selection)),
            Err(refusal) => {
                self.notes.push(Note::PermissionRefused(refusal));
                self.abandon_inline();
            }
        }
        true
    }

    /// The permission shortcut keys (`handle_permission_shortcut`): the pick of the next or
    /// previous built-in mode over the catalog held, or the catalog asked for (and its menu shown)
    /// when there is none. A pick still on its way ignores the key.
    fn permission_shortcut(&mut self, forward: bool, mut current: permissions::Current) {
        if self.profiles.shortcut {
            return;
        }
        let Some(discovery) = &self.profiles.found else {
            current.discover = true;
            self.open_permissions(current);
            return;
        };
        match permission_shortcuts::next(&current, discovery, forward) {
            Some(selection) => {
                self.profiles.shortcut = true;
                self.picked.push(Picked::Permissions(selection));
            }
            None => self.notes.push(Note::NoOtherPermissionModes),
        }
    }

    /// `open_permissions_popup`: the legacy list of presets, or (with a
    /// thread, the explicit profile mode or a custom active profile) the
    /// server's catalog: the menu on the one held, or `Loading…` and a
    /// request while there is none (`request_permission_profiles`).
    fn open_permissions(&mut self, current: permissions::Current) {
        use permissions::PermissionsMenu as Perms;
        self.stack.clear();
        // The cyber warning of full access reads the model catalog: have it
        // before the confirmation (a request in flight serves as well).
        if self.catalog.presets.is_empty() && self.catalog.pending.is_none() {
            self.catalog.request();
        }
        let menu = if !current.discover {
            Perms::Presets(current)
        } else if self.profiles.found.is_some() {
            Perms::Profiles(current)
        } else {
            // A request already out serves this menu too.
            if self.profiles.pending.is_none() {
                self.profiles.request();
            }
            Perms::Loading(current)
        };
        self.stack.push(Menu::Permissions(Box::new(menu)));
    }

    /// The answer of the `permissionProfile/list` request: the menu waiting on
    /// it (the one on top) shows the catalog, or why there is none
    /// (`on_permission_profiles_loaded`).
    fn discovered(&mut self, result: Result<permissions::Discovery, permissions::Failure>) {
        use permissions::PermissionsMenu as Perms;
        let loading = |menu: &Menu| {
            matches!(menu, Menu::Permissions(menu) if matches!(**menu, Perms::Loading(_)))
        };
        if let Ok(discovery) = &result {
            self.profiles.found = Some(discovery.clone());
        }
        if self.stack.last().is_some_and(loading) {
            let Some(Menu::Permissions(menu)) = self.stack.pop() else {
                return;
            };
            let current = menu.current().clone();
            match result {
                // An inline pick takes the catalog first; else the menu shows it.
                Ok(_) => {
                    if !self.apply_inline() {
                        self.stack
                            .push(Menu::Permissions(Box::new(Perms::Profiles(current))));
                    }
                }
                Err(failure) => self
                    .stack
                    .push(Menu::Permissions(Box::new(Perms::Failed { current, failure }))),
            }
            return;
        }
        // The user left the menu, or another one covers it: the catalog is held, and a menu below
        // another is not shown over it (`dismiss_view_by_id`).
        let covered = self.stack.iter().any(loading);
        self.stack.retain(|menu| !loading(menu));
        if covered && result.is_ok() {
            return;
        }
        if let Err(failure) = result
            && matches!(self.profiles.inline, Some(permission_args::Inline::Discovering { .. }))
        {
            self.notes.push(Note::PermissionsFailed(failure));
        }
        if matches!(self.profiles.inline, Some(permission_args::Inline::Discovering { .. })) {
            self.abandon_inline();
        }
    }

    /// `open_model_popup_with_presets`: the quick menu, or a note when the
    /// catalog has nothing to offer.
    fn open_models(&mut self, current: Current) {
        let menu = ModelMenu::Models(current);
        if menu.is_empty(&self.catalog) {
            self.notes.push(if menu.is_reserve() { Note::ReserveUnavailable } else { Note::NoAdditionalModels });
        } else {
            self.stack.push(Menu::Model(Box::new(menu)));
        }
    }

    /// A catalog arrived (the startup one, or the answer of a request): a
    /// catalog that differs and is not empty replaces the held one, and the
    /// pickers open on it show it without being reopened; one left with no
    /// choice is dismissed (`refresh_open_model_picker`).
    fn accept(&mut self, presets: Vec<Preset>) {
        if presets.is_empty() || self.catalog.presets == presets {
            return;
        }
        self.catalog.presets = presets;
        let catalog = &self.catalog;
        let mut dismissed = Vec::new();
        self.stack.retain(|menu| match menu {
            Menu::Model(menu) if menu.is_empty(catalog) => {
                dismissed.push(if menu.is_reserve() { Note::ReserveUnavailable } else { Note::NoAdditionalModels });
                false
            }
            _ => true,
        });
        self.notes.extend(dismissed);
    }
}

/// The events of this screen (backend results, tickets); a variant of the
/// root `ScreenEv`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ev {
    /// A menu was opened: the only one on the stack.
    Opened(Menu),
    /// The catalog the app started with.
    Loaded(Vec<Preset>),
    /// The default provider's write state changed (`provider_write`): a write in flight or
    /// committed dismisses the model menus and drops the answer in flight
    /// (`dismiss_stale_model_picker`), and blocks `/model` until the restart.
    Provider(Provider),
    /// The permission catalog the app holds; menus open on it show it in
    /// place.
    Listed(permissions::Discovery),
    /// The answer of the `permissionProfile/list` request `request`
    /// (`Profiles::pending`); the profiles of every page and the
    /// requirements, or why they could not be read.
    Discovered {
        request: u64,
        result: Result<permissions::Discovery, permissions::Failure>,
    },
    /// The account or the scope changed (`invalidate_permission_discovery`):
    /// the answer in flight is no longer wanted and the menus that waited on
    /// it or showed it are dismissed.
    ScopeChanged,
    /// The answer of the `model/list` request `request` (`Catalog::pending`).
    Fetched {
        request: u64,
        result: Result<Vec<Preset>, String>,
    },
    /// The answer of the `app/list` request `request` (`Apps::pending`).
    AppsLoaded {
        request: u64,
        result: Result<Vec<apps::App>, String>,
    },
    /// The host's list for the `/review` menus (`Review::pending` names the request): the child
    /// menu opens over the presets, unless they are gone.
    Reviewed {
        request: u64,
        listing: review::Listing,
    },
    /// The host's worktree list (`Worktree::pending` names the request): the browser replaces the
    /// loading row, or the error is said; a stale answer is dropped.
    Worktrees {
        request: u64,
        result: Result<Vec<worktree::Entry>, String>,
    },
    /// The host's answer to a removal asked for.
    WorktreeRemoved {
        request: u64,
        result: Result<(), String>,
    },
    /// The presets of the config, with the layers each comes from (told when the config is read).
    Dials(Vec<quick_dials::Slot>),
    /// The config took the write of a preset, or refused it.
    PresetWritten {
        slot: u8,
        dial: Option<quick_dials::Dial>,
        result: Result<(), String>,
    },
    /// The app server says the directory changed
    /// (`refresh_connector_directory_after_notification`).
    AppsNotified(Vec<apps::App>),
    /// An app was switched on or off (`update_connector_enabled`).
    AppSwitched { id: String, enabled: bool },
    /// The link dialog of an app is shown over the directory (a seed of the debug entries).
    AppLinkOpened(apps::Link),
}

/// The intents of this screen; a variant of the root `ScreenIntent`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// A screen asks for this menu (a generic list: its picks come back as
    /// `Picked::Item(id)` in `Model::picked`).
    Open(Menu),
    /// `/model`: the quick menu on the catalog held (`Loading models…` while
    /// it is empty), and the catalog is asked for again: the request is
    /// `Catalog::pending`.
    OpenModels(Current),
    /// The catalog is asked for, with no menu: the app does it when it connects.
    LoadModels,
    /// `/permissions`: the menu for what the thread has now; the catalog is
    /// asked for when it has none (`Profiles::pending` is the request).
    OpenPermissions(permissions::Current),
    /// `/permissions <profile> ..`: the profile is picked without a menu once the catalog is
    /// read (`select_permission_profile`); `current` is what the thread has now.
    PermissionsArgs { args: String, current: permissions::Current },
    /// The next or previous built-in mode (`handle_permission_shortcut`): the pick of the cycle,
    /// or the catalog asked for when none is held.
    PermissionShortcut { forward: bool, current: permissions::Current },
    /// `/apps` (`enabled`: the apps feature is on): the directory, asked for
    /// again (`Apps::pending` is the request).
    OpenApps { enabled: bool },
    /// `/review`: the presets menu; the git lists are asked in `cwd`.
    OpenReview { cwd: String },
    /// `/new` or `/fork` (`mode`) in `cwd`: where it runs is asked when a local repository
    /// (`available`) offers a new checkout, else it is the current one at once.
    OpenCheckout {
        mode: worktree::Conversation,
        name: Option<String>,
        available: bool,
        cwd: String,
        thread: Option<ThreadId>,
    },
    /// `/worktree`: continue, start or browse; an error line where no local repository
    /// (`available`) is.
    OpenWorktrees {
        available: bool,
        cwd: String,
        thread: Option<ThreadId>,
    },
    /// The text typed at `Custom review instructions`.
    Instructions(String),
    /// The app directory is read again (`apps.refresh(true)`): a surface that waited for an app
    /// to be installed asks for it.
    RefreshApps,
    /// `/presets`: a menu of the presets, or a preset recalled; `gate` says whether presets can
    /// be used where the user is.
    Quick {
        gate: quick_dials::Gate,
        open: quick_dials::Open,
    },
    /// The text submitted at the preset editor.
    Draft(String),
    /// The link dialog of an app switched it on or off (`SetAppEnabled`): the dialog shows the
    /// switch at once, the app writes the config and, once it is saved, tells the directory
    /// (`Ev::AppSwitched`).
    SetApp { id: String, enabled: bool },
    /// The item with this id (`List::id_of`, or a row of `Page`) was
    /// accepted.
    Choose(String),
    /// The secondary action of the item with this id (`s for secondary`).
    Secondary(String),
    /// The toggle of the item with this id was flipped.
    Toggle(String),
    /// The orderable item with this id moves one place (`true`: down).
    Reorder(String, bool),
    /// A multi-select list (`List::multi`) was confirmed (Enter): the ids of
    /// the toggled-on items are picked together and the menu closes.
    Confirm,
    /// The menu shown was closed without a pick; the one below shows again.
    Close,
    /// `/fast`, `/ultrafast`...: the tier `id` is switched on, or off when it is `current`
    /// (`toggle_service_tier_from_ui`).
    ServiceTier { id: String, current: Option<String> },
    /// A reasoning shortcut on the thread `current` describes: `plan` is the thread in Plan
    /// mode (the step is its override only), `started` the session being configured.
    Reasoning { direction: reasoning::Direction, current: Current, plan: bool, started: bool },
}

pub fn apply(model: &mut Model, ev: Ev, fx: &mut AppFx) {
    apply_ev(model, ev, fx);
    model.tidy();
}

fn apply_ev(model: &mut Model, ev: Ev, fx: &mut AppFx) {
    let _ = fx;
    match ev {
        Ev::Opened(menu) => model.stack = vec![menu],
        Ev::Loaded(presets) => model.accept(presets),
        Ev::Worktrees { request, result } => {
            if !model.worktree.answered(request) {
                return;
            }
            let loading = matches!(
                model.stack.last(),
                Some(Menu::Worktree(menu)) if **menu == worktree::Menu::Loading
            );
            if !loading {
                return;
            }
            match result {
                Ok(entries) => {
                    model.stack = vec![Menu::Worktree(Box::new(worktree::Menu::Browser(entries)))];
                }
                Err(error) => {
                    model.stack.clear();
                    model.notes.push(Note::WorktreeListFailed(error));
                }
            }
        }
        Ev::WorktreeRemoved { request, result } => {
            if let Some(root) = model.worktree.removed(request) {
                model.notes.push(match result {
                    Ok(()) => Note::WorktreeRemoved { root },
                    Err(error) => Note::WorktreeRemoveFailed { root, error },
                });
            }
        }
        Ev::Dials(slots) => model.quick.set(slots),
        Ev::PresetWritten { slot, dial, result } => match result {
            Ok(()) => {
                model.quick.written(quick_dials::Write { slot, dial: dial.clone() });
                model.notes.push(if dial.is_some() {
                    Note::PresetSaved(slot)
                } else {
                    Note::PresetDeleted(slot)
                });
            }
            Err(error) => {
                model.notes.push(Note::PresetFailed { slot, error });
                model.restore_editor(slot, dial);
            }
        },
        Ev::Provider(provider) => {
            model.catalog.provider = provider;
            if provider != Provider::Settled {
                model.dismiss_stale_models();
            }
        }
        Ev::Reviewed { request, listing } => {
            let over_presets = matches!(
                model.stack.last(),
                Some(Menu::Review(menu)) if **menu == review::Menu::Presets
            );
            if model.review.answered(request) && over_presets {
                model.kept.insert(model.stack.len() - 1, listing.kind().row().to_string());
                model.stack.push(Menu::Review(Box::new(listing.menu())));
            }
        }
        Ev::Listed(discovery) => model.profiles.found = Some(discovery),
        Ev::Discovered { request, result } => {
            if model.profiles.pending != Some(request) {
                return;
            }
            model.profiles.pending = None;
            model.discovered(result);
        }
        Ev::ScopeChanged => {
            use permissions::PermissionsMenu as Perms;
            model.profiles.pending = None;
            // `invalidate_permission_discovery`: an inline pick waiting on the old scope is given up.
            model.abandon_inline();
            model.apps.invalidate();
            model.worktree.forget();
            model.stack.retain(|menu| match menu {
                Menu::Apps | Menu::Worktree(_) => false,
                Menu::Permissions(menu) => !matches!(
                    **menu,
                    Perms::Loading(_) | Perms::Profiles(_) | Perms::Failed { .. }
                ),
                _ => true,
            });
        }
        Ev::AppsLoaded { request, result } => {
            model.apps.loaded(request, result);
        }
        Ev::AppsNotified(apps) => {
            model.apps.notified(apps);
        }
        Ev::AppSwitched { id, enabled } => model.apps.switched(&id, enabled),
        Ev::AppLinkOpened(link) => {
            model.apps.link = Some(link);
            model.stack = vec![Menu::Apps];
        }
        Ev::Fetched { request, result } => {
            // Another request, or none: the answer is not for the pickers.
            if model.catalog.pending != Some(request) {
                return;
            }
            model.catalog.pending = None;
            let loading = match model.stack.last() {
                Some(Menu::Model(menu)) => match &**menu {
                    ModelMenu::Loading(current) => Some(current.clone()),
                    _ => None,
                },
                _ => None,
            };
            if let Some(current) = loading {
                model.stack.pop();
                match result {
                    Ok(presets) => {
                        if !presets.is_empty() {
                            model.catalog.presets = presets;
                        }
                        model.open_models(current);
                    }
                    Err(_) => model.notes.push(Note::ModelsNotLoaded),
                }
            } else if let Ok(presets) = result {
                model.accept(presets);
            }
        }
    }
}

pub fn apply_intent(model: &mut Model, intent: Intent, fx: &mut AppFx) {
    model.applied += 1;
    apply_choice(model, intent, fx);
    model.tidy();
}

fn apply_choice(model: &mut Model, intent: Intent, fx: &mut AppFx) {
    let _ = fx;
    match intent {
        Intent::Open(menu) => model.stack = vec![menu],
        Intent::OpenModels(current) => {
            if let Some(why) = model.catalog.unavailable() {
                model.notes.push(Note::ModelPickerUnavailable(why));
                return;
            }
            model.stack.clear();
            model.catalog.request();
            if model.catalog.presets.is_empty() {
                model
                    .stack
                    .push(Menu::Model(Box::new(ModelMenu::Loading(current))));
            } else {
                model.open_models(current);
            }
        }
        Intent::LoadModels => {
            if model.catalog.unavailable().is_none() {
                model.catalog.request();
            }
        }
        Intent::OpenPermissions(current) => model.open_permissions(current),
        Intent::PermissionsArgs { args, current } => model.permissions_args(&args, current),
        Intent::PermissionShortcut { forward, current } => {
            model.permission_shortcut(forward, current);
        }
        Intent::ServiceTier { id, current } => {
            let next = if current.as_deref() == Some(id.as_str()) {
                codex_protocol::config_types::SERVICE_TIER_DEFAULT_REQUEST_VALUE.to_string()
            } else {
                id
            };
            model.picked.push(Picked::Tier(next));
        }
        Intent::Reasoning { direction, current, plan, started } => {
            match reasoning::step(&model.catalog, &current, direction, started) {
                reasoning::Outcome::Set(effort) => {
                    model.picked.push(Picked::Effort { model: current.model, effort, plan });
                }
                reasoning::Outcome::Say(note) => model.notes.push(note),
            }
        }
        Intent::OpenApps { enabled } => model.open_apps(enabled),
        // `/review`: the presets, over the session's working directory.
        Intent::OpenReview { cwd } => {
            model.review.open(cwd);
            model.stack = vec![Menu::Review(Box::new(review::Menu::Presets))];
        }
        // `/new` and `/fork` (`show_session_checkout_picker`): the choice of checkout, or none
        // where there is no local repository to make one in.
        Intent::OpenCheckout { mode, name, available, cwd, thread } => {
            model.worktree.open(cwd, thread);
            if available {
                model.stack = vec![Menu::Worktree(Box::new(worktree::Menu::Checkout { mode, name }))];
            } else {
                model.stack.clear();
                model.picked.push(Picked::Worktree(worktree::Pick::Current { mode, name }));
            }
        }
        // `/worktree` (`show_managed_worktree_picker`).
        Intent::OpenWorktrees { available, cwd, thread } => {
            if available {
                model.worktree.open(cwd, thread);
                model.stack = vec![Menu::Worktree(Box::new(worktree::Menu::Worktrees))];
            } else {
                model.notes.push(Note::WorktreesUnavailable);
            }
        }
        // The text of `Custom review instructions` (an empty one is not an answer).
        Intent::Instructions(text) => {
            let text = text.trim();
            if matches!(model.stack.last(), Some(Menu::Review(menu)) if **menu == review::Menu::Instructions)
                && !text.is_empty()
            {
                model.stack.clear();
                model.picked.push(Picked::Review(crate::threads::ReviewTarget::Custom {
                    instructions: text.to_string(),
                }));
            }
        }
        Intent::RefreshApps => {
            model.apps.refresh(true);
        }
        Intent::Quick { gate, open } => model.open_quick(gate, open),
        // The text submitted at the preset editor: the slot picker, or the confirmation.
        Intent::Draft(text) => {
            use quick_dials::{Dial, Menu as Quick, Target};
            let Some(Menu::Quick(menu)) = model.stack.last() else { return };
            let Quick::Editor { target, dial } = &**menu else { return };
            let (target, dial) = (*target, Dial { input: text, label: dial.label.clone() });
            model.stack.pop();
            match target {
                Target::PickSlot => model.open_quick(
                    quick_dials::Gate::Ready,
                    quick_dials::Open::Save(dial),
                ),
                Target::Slot(slot) => model.quick_confirm(slot, Some(dial)),
            }
        }
        // The app writes the config (`app/select.rs`); the directory follows the saved write.
        Intent::SetApp { id, enabled } => {
            if let Some(link) = &mut model.apps.link
                && link.id == id
            {
                link.enabled = enabled;
            }
        }
        Intent::Choose(id) => match model.stack.last_mut() {
            Some(Menu::Model(menu)) => {
                let step = menu.choose(&model.catalog, &id, false);
                model.step_from(id, step);
            }
            Some(Menu::List(list)) => {
                let Some(item) = list.find(&id) else { return };
                if item.is_disabled() {
                    return;
                }
                let keep = item.keep_open;
                if item.checkbox && item.toggle.is_some() {
                    list.flip(&id);
                    return;
                }
                model.picked.push(Picked::Item(id));
                if !keep {
                    model.stack.pop();
                }
            }
            Some(Menu::Permissions(menu)) => {
                let step = menu.choose(&model.profiles, &id);
                model.permissions_step(step);
            }
            Some(Menu::Apps) => model.choose_app(&id),
            Some(Menu::Review(menu)) => {
                use review::Chosen;
                match menu.choose(&id) {
                    Some(Chosen::Ask(git)) => model.review.want(git),
                    Some(Chosen::Prompt) => {
                        model.kept.insert(model.stack.len() - 1, id);
                        model.stack.push(Menu::Review(Box::new(review::Menu::Instructions)));
                    }
                    Some(Chosen::Target(target)) => {
                        model.stack.clear();
                        model.picked.push(Picked::Review(target));
                    }
                    None => {}
                }
            }
            Some(Menu::Quick(menu)) => {
                use quick_dials::Chosen;
                match menu.choose(&id, &model.quick) {
                    Some(Chosen::Push(next)) => {
                        model.kept.insert(model.stack.len() - 1, id);
                        model.stack.push(Menu::Quick(Box::new(next)));
                    }
                    Some(Chosen::Confirm { slot, new }) => {
                        model.kept.insert(model.stack.len() - 1, id);
                        model.quick_confirm(slot, new);
                    }
                    Some(Chosen::Write { slot, dial }) => {
                        model.stack.clear();
                        model.quick_write(slot, dial);
                    }
                    None => {}
                }
            }
            Some(Menu::Worktree(menu)) => {
                use worktree::Chosen;
                match menu.choose(&id) {
                    Some(Chosen::Pick(worktree::Pick::Remove { root })) => {
                        model.stack.clear();
                        if model.worktree.removable(&root) {
                            model.worktree.want(worktree::Want::Remove { root });
                        }
                    }
                    Some(Chosen::Pick(pick)) => {
                        model.stack.clear();
                        model.picked.push(Picked::Worktree(pick));
                    }
                    Some(Chosen::Browse) => {
                        model.stack = vec![Menu::Worktree(Box::new(worktree::Menu::Loading))];
                        model.worktree.want(worktree::Want::List);
                    }
                    Some(Chosen::Actions(entry)) => {
                        let can_delete = model.worktree.can_delete(&entry);
                        model.stack = vec![Menu::Worktree(Box::new(worktree::Menu::Actions {
                            entry,
                            can_delete,
                        }))];
                    }
                    Some(Chosen::Confirm { root }) => {
                        if model.worktree.removable(&root) {
                            model.stack =
                                vec![Menu::Worktree(Box::new(worktree::Menu::Confirm { root }))];
                        }
                    }
                    Some(Chosen::Close) => model.stack.clear(),
                    None => {}
                }
            }
            Some(Menu::Feature(menu)) => {
                use crate::features::Choice;
                match crate::features::choose(menu, &id) {
                    Choice::Stay => {}
                    Choice::Dismiss => {
                        model.stack.pop();
                    }
                    Choice::Pick(pick) => {
                        model.stack.pop();
                        model.picked.push(Picked::Feature(pick));
                    }
                    Choice::Replace(next) => {
                        model.stack.pop();
                        model.stack.push(Menu::Feature(next));
                    }
                }
            }
            Some(Menu::Confirm(confirm)) => {
                let confirm = *confirm;
                if id == Confirm::YES {
                    model.picked.push(Picked::Confirmed(confirm));
                }
                if id == Confirm::YES || id == Confirm::NO {
                    model.stack.pop();
                }
            }
            None => {}
        },
        Intent::Secondary(id) => match model.stack.last() {
            Some(Menu::Model(menu)) => {
                let step = menu.choose(&model.catalog, &id, true);
                model.step_from(id, step);
            }
            Some(Menu::List(list)) if list.find(&id).is_some_and(|item| !item.is_disabled()) => {
                model.picked.push(Picked::ItemSecondary(id));
                model.stack.pop();
            }
            _ => {}
        },
        Intent::Toggle(id) => match model.stack.last_mut() {
            Some(Menu::List(list)) => list.flip(&id),
            _ => {}
        },
        Intent::Reorder(id, down) => {
            if let Some(Menu::List(list)) = model.stack.last_mut() {
                list.reorder(&id, down);
            }
        }
        Intent::Confirm => {
            if let Some(Menu::List(list)) = model.stack.last()
                && list.multi
            {
                model.picked.push(Picked::Items(list.checked()));
                model.stack.pop();
            }
        }
        Intent::Close => {
            model.apps.link = None;
            model.stack.pop();
            model.inline_unattended();
        }
    }
}

/// Per-thread state of this screen: a part of every `ThreadModel`, changed
/// only by `apply_thread` and `apply_reply`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ThreadPart {}

/// An event of one thread for this screen; a variant of `ThreadEv::Screen`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThreadEv {}

/// The RPCs of this screen. They are about the app (the catalog of models,
/// the server's permission profiles), not about one thread: the app sends
/// them (`app/select.rs`) as a screen command with no thread, under a ticket
/// of its own, and the shell sends them through `wire`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// `model/list` for the request `Catalog::pending` names.
    Models { request: u64 },
    /// `collaborationMode/list`: the modes `/plan` and Shift+Tab switch between (asked at
    /// connect; the app keeps the answer).
    Collaboration,
    /// `configRequirements/read`, the first half of a permission read.
    Requirements { request: u64 },
    /// One page of `permissionProfile/list` (100 per page, at most 10).
    Profiles {
        request: u64,
        cursor: Option<String>,
        cwd: Option<String>,
    },
    /// `app/list` for the request `Apps::pending` names (`force`: past the
    /// server's caches), for the thread displayed.
    Apps {
        request: u64,
        force: bool,
        thread: Option<String>,
    },
    /// `thread/settings/update` of `thread`: what a pick changes for its
    /// next turns.
    Settings {
        thread: String,
        patch: crate::threads::SettingsPatch,
    },
    /// `config/batchWrite`: the defaults a pick saves.
    Write { edits: Vec<persist::Edit> },
    /// `config/batchWrite` of the `apps.<id>` keys: the app switched on or off.
    SetApp { id: String, enabled: bool },
    /// `config/batchWrite` of `tui.presets.<slot>`: the preset saved, or deleted (`None`).
    Preset {
        slot: u8,
        dial: Option<quick_dials::Dial>,
    },
}

/// The replies of those RPCs, decoded by `decode`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {
    Models(Vec<Preset>),
    /// The collaboration-mode presets of the server.
    Collaboration(Vec<collaboration::Mask>),
    Apps(Vec<apps::App>),
    Requirements(Option<permissions::Requirements>),
    Profiles {
        profiles: Vec<permissions::Profile>,
        next: Option<String>,
    },
    /// The thread's settings were updated.
    Updated,
    /// The config was written; whether a layer above overrides the value.
    Written {
        overridden: bool,
    },
}

/// What an outgoing RPC of this screen was for, kept with its ticket.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ctx {
    Models {
        request: u64,
    },
    /// The read of the collaboration-mode catalog.
    Collaboration,
    Apps {
        request: u64,
    },
    Requirements {
        request: u64,
    },
    /// The pages of the catalog read so far, and the requirements read before
    /// them.
    Profiles {
        request: u64,
        requirements: Option<permissions::Requirements>,
        found: Vec<permissions::Profile>,
        pages: u8,
    },
    /// A pick's settings update: what to tell if it fails, and the line that
    /// follows once the server took it.
    Settings {
        setting: Setting,
        then: Option<Note>,
    },
    /// A pick's write: the pick whose line follows a saved write.
    Write {
        setting: Setting,
        then: Option<Note>,
    },
    /// The write of an app's switch: the directory takes it once it is saved.
    App { id: String, enabled: bool },
    /// The write of a preset: the slot follows once it is saved.
    Preset {
        slot: u8,
        dial: Option<quick_dials::Dial>,
    },
    /// `model/list` for an automatic model switch of a usage banner that finds no catalog
    /// (`app/banners.rs`).
    Fallback { backend: crate::ids::BackendId },
    /// The settings update of such a switch: the model and effort it asked for, and why.
    Switch {
        backend: crate::ids::BackendId,
        thread: crate::ids::ThreadId,
        model: String,
        effort: Effort,
        reason: super::chat::SwitchReason,
    },
}

/// The pages a catalog may take (`permission_discovery.rs`).
pub const MAX_PAGES: u8 = 10;

pub fn apply_thread(part: &mut ThreadPart, ev: ThreadEv, fx: &mut ThreadFx<'_>) {
    let _ = (part, fx);
    match ev {}
}

pub fn apply_reply(
    part: &mut ThreadPart,
    ctx: Ctx,
    result: Result<Reply, RpcError>,
    fx: &mut ThreadFx<'_>,
) {
    // The replies are the app's (`app/select.rs`): no thread asks for them.
    let _ = (part, ctx, result, fx);
}

/// The wire request of `command`; the thread is none.
pub fn wire(command: &Command, thread: &ThreadId, id: RequestId) -> ClientRequest {
    let _ = thread;
    match command {
        Command::Settings { thread, patch } => crate::threads::ThreadAsk::SettingsUpdate {
            thread: thread.clone(),
            patch: patch.clone(),
        }
        .wire(id),
        Command::Write { edits } => persist::write_request(edits, id),
        Command::SetApp { id: app, enabled } => persist::app_enabled_request(app, *enabled, id),
        Command::Preset { slot, dial } => persist::preset_request(*slot, dial.as_ref(), id),
        Command::Models { .. } => models::list_request(id),
        Command::Collaboration => collaboration::list_request(id),
        Command::Apps { force, thread, .. } => apps::list_request(id, *force, thread.clone()),
        Command::Requirements { .. } => permissions::requirements_request(id),
        Command::Profiles { cursor, cwd, .. } => {
            permissions::profiles_request(id, cursor.clone(), cwd.clone())
        }
    }
}

/// The reply `result` (the JSON result of the request) is for `command`.
pub fn decode(command: &Command, result: serde_json::Value) -> Result<Reply, RpcError> {
    match command {
        Command::Settings { .. } => Ok(Reply::Updated),
        Command::Write { .. } | Command::SetApp { .. } | Command::Preset { .. } => {
            persist::written(result).map(|overridden| Reply::Written { overridden })
        }
        Command::Models { .. } => models::presets(result).map(Reply::Models),
        Command::Collaboration => collaboration::masks(result).map(Reply::Collaboration),
        Command::Apps { .. } => apps::apps(result).map(Reply::Apps),
        Command::Requirements { .. } => permissions::requirements(result).map(|wire| {
            Reply::Requirements(wire.as_ref().map(permissions::Requirements::from_wire))
        }),
        Command::Profiles { .. } => {
            permissions::profiles_page(result).map(|(wire, next)| Reply::Profiles {
                profiles: permissions::profiles_of(wire),
                next,
            })
        }
    }
    .map_err(RpcError::Malformed)
}

/// The seed form of this screen's events: `{"kind":"catalog","presets":[..]}`
/// (the catalog the app started with), `{"kind":"provider","state":"changed"}` (the default
/// provider was written), the menus of the `/model` family
/// (`{"kind":"models","current":{"model":..}}`, `all_models`, `loading`,
/// `reasoning` and `advanced` with a `preset`), `{"kind":"profiles",
/// "discovery":{..}}` (the permission catalog), `{"kind":"permissions",
/// "menu":"profiles","current":{..}}`, `{"kind":"list", ..}` (the fields
/// of `List`) and `{"kind":"app_link","link":{..}}` (the link dialog of an app).
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum SeedEv {
    Catalog {
        presets: Vec<Preset>,
    },
    /// The state of the default provider's write (`Ev::Provider`).
    Provider {
        state: Provider,
    },
    Loading {
        current: Current,
    },
    Models {
        current: Current,
    },
    AllModels {
        current: Current,
    },
    Reasoning {
        preset: Preset,
        current: Current,
    },
    Advanced {
        preset: Preset,
        current: Current,
    },
    PlanScope {
        model: String,
        effort: Option<Effort>,
        source: PlanSource,
        #[serde(default)]
        warning: Option<usize>,
    },
    List(Box<List>),
    /// The link dialog of an app (`/apps`, an app picked).
    AppLink {
        link: apps::Link,
    },
    /// The profiles the app holds (a startup read of the catalog).
    Profiles {
        discovery: permissions::Discovery,
    },
    /// A menu of the `/permissions` family: the legacy presets unless `menu`
    /// says otherwise; `failure` is the text of a read that failed.
    Permissions {
        #[serde(default)]
        menu: SeedPermissions,
        #[serde(default)]
        current: permissions::Current,
        #[serde(default)]
        failure: Option<String>,
    },
}

#[derive(Clone, Copy, Debug, Default, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum SeedPermissions {
    #[default]
    Presets,
    Loading,
    Profiles,
    Failed,
    /// The confirmation `Enable full access?` (the pick is Full Access).
    FullAccess,
}

/// A script's `screen` step for this directory.
pub fn seed(value: serde_json::Value) -> Result<Ev, crate::screens::SeedError> {
    let seed: SeedEv = serde_json::from_value(value)
        .map_err(|err| crate::screens::SeedError::Bad(err.to_string()))?;
    let model = |menu| Ev::Opened(Menu::Model(Box::new(menu)));
    Ok(match seed {
        SeedEv::Catalog { presets } => Ev::Loaded(presets),
        SeedEv::Provider { state } => Ev::Provider(state),
        SeedEv::Loading { current } => model(ModelMenu::Loading(current)),
        SeedEv::Models { current } => model(ModelMenu::Models(current)),
        SeedEv::AllModels { current } => model(ModelMenu::AllModels(current)),
        SeedEv::Reasoning { preset, current } => model(ModelMenu::Reasoning { preset, current }),
        SeedEv::Advanced { preset, current } => model(ModelMenu::Advanced { preset, current }),
        SeedEv::PlanScope {
            model: name,
            effort,
            source,
            warning,
        } => model(ModelMenu::PlanScope {
            model: name,
            effort,
            source,
            warning,
        }),
        SeedEv::List(list) => Ev::Opened(Menu::List(list)),
        SeedEv::AppLink { link } => Ev::AppLinkOpened(link),
        SeedEv::Profiles { discovery } => Ev::Listed(discovery),
        SeedEv::Permissions {
            menu,
            current,
            failure,
        } => {
            use permissions::PermissionsMenu as Perms;
            let menu = match menu {
                SeedPermissions::Presets => Perms::Presets(current),
                SeedPermissions::Loading => Perms::Loading(current),
                SeedPermissions::Profiles => Perms::Profiles(current),
                SeedPermissions::Failed => Perms::Failed {
                    current,
                    failure: permissions::Failure::Other(failure.unwrap_or_default()),
                },
                SeedPermissions::FullAccess => {
                    let pick = permissions::Selection::Preset {
                        preset: permissions::Preset::FullAccess,
                        reviewer: permissions::Reviewer::User,
                    };
                    Perms::FullAccess { current, pick }
                }
            };
            Ev::Opened(Menu::Permissions(Box::new(menu)))
        }
    })
}
