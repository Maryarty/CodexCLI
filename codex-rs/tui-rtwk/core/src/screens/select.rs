//! `select` (S14, generic selection dialog): the core half of the screen's directory. The model
//! part, its events and intents, its RPCs and their protocol wiring live here,
//! with no rtwk; the view half is `src/sheets/select/` in the UI crate. Nothing
//! outside these two directories (bar their test files) needs an edit when it
//! grows.

use std::collections::{BTreeMap, VecDeque};

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;

use crate::app::AppFx;
use crate::ids::{BackendId, ThreadId, Ticket};
use crate::rpc::RpcError;
use crate::screens::Kind;
use crate::threads::ThreadFx;
use crate::threads::queue::QueueOperation;

pub mod agent_picker;
pub mod apps;
pub mod collaboration;
pub mod models;
pub mod permission_args;
pub mod permission_shortcuts;
pub mod permissions;
pub mod persist;
pub mod quick_dials;
pub mod reasoning;
pub mod review;
pub mod worktree;
use models::{Answer, Effort, GaveUp, PlanSource};
pub use models::{Catalog, Current, ModelMenu, Preset, Provider, Selection};
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

/// A sheet the screen wants shown, once (ADR-013 Ж): the decision is the domain's (a thread is
/// asked to be archived: the question comes first), how it looks and where it opens is the
/// presenter's. The presenter shows it and says so (`Intent::Shown`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Show {
    /// Numbers the shows in the order they were made; what a presenter says it showed.
    pub seq: u64,
    pub opening: Opening,
    /// Only the queued model picker carries a continuation back to the thread owner.
    pub queued_model: Option<QueuedModelShow>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QueuedModelShow {
    Requested(QueueOperation),
    Withdrawn(QueueOperation),
}

impl QueuedModelShow {
    pub fn operation(&self) -> &QueueOperation {
        match self {
            Self::Requested(operation) | Self::Withdrawn(operation) => operation,
        }
    }
}

/// What a show opens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Opening {
    /// A confirmation: No (row 0), then Yes (row 1); the answer Yes is `Intent::Confirmed`.
    Confirm(Confirm),
    /// `/review`: the presets, and over them the lists of the host (`review.rs`); the pages and
    /// what Esc does on each are the sheet's.
    Review,
    /// The worktree pickers (`worktree.rs`): the sheet opens on this page, the checkout choice of
    /// `/new` and `/fork` or `/worktree`'s; the browser, a worktree's actions and the delete
    /// confirmation follow as the sheet's own pages.
    Worktree(worktree::Menu),
    /// `/apps`: the directory of apps (`Model::apps`: loading, the failure with its Retry row, or
    /// the list) and the link dialog of the app picked from it; the sheet reads the directory as
    /// a fact and its pages are its own.
    Apps,
    /// The presets sheet: the page it opens on (the slot picker, the sources or the editor,
    /// `quick_dials::Menu`); the pages over it (`quick_dials::Menu::choose`, `submit`) are routes
    /// of the sheet's own navigator, and what Esc does on each is the sheet's. The model keeps no
    /// stack of them. (`/presets` itself says its overlay is not built until it lands.)
    Quick(Box<quick_dials::Menu>),
    /// `/permissions`, its inline pick and its shortcut (`permissions.rs`): the menu the sheet
    /// opens on; the page it is on after a pick is the sheet's.
    Permissions(PermissionsMenu),
    /// `/model`: the page it opens on (`Loading models…` while the catalog is empty, else the
    /// quick menu); the pages over it and what Esc does on each are the sheet's
    /// (`models.rs`: the pages are `ModelMenu`s and `ModelMenu::choose` leads from one to the
    /// next).
    Models(ModelMenu),
}

/// The shows nobody has shown yet.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Shows {
    queue: VecDeque<Show>,
    seq: u64,
}

impl Shows {
    fn push(&mut self, opening: Opening) {
        self.push_owned(opening, None);
    }

    fn push_owned(&mut self, opening: Opening, operation: Option<QueueOperation>) {
        self.seq += 1;
        self.queue.push_back(Show {
            seq: self.seq,
            opening,
            queued_model: operation.map(QueuedModelShow::Requested),
        });
    }

    /// The show `seq` was shown (or is given up): it is gone from the queue.
    fn shown(&mut self, seq: u64) {
        self.queue.retain(|show| show.seq != seq);
    }

    /// The shows of the `/model` family not shown yet are given up (a provider write came first).
    fn forget_models(&mut self) {
        self.queue.retain_mut(|show| {
            if !matches!(show.opening, Opening::Models(_)) {
                return true;
            }
            if let Some(QueuedModelShow::Requested(operation)) = &show.queued_model {
                show.queued_model = Some(QueuedModelShow::Withdrawn(operation.clone()));
            }
            // The presenter must settle an owned show it has not already mounted. If it
            // is mounted, the picker sees the provider change and its own close settles.
            show.queued_model.is_some()
        });
    }

    /// The shows still to show, oldest first.
    pub fn pending(&self) -> impl Iterator<Item = &Show> {
        self.queue.iter()
    }
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
    Effort {
        model: String,
        effort: Effort,
        plan: bool,
    },
    /// The answer of a confirmation was Yes (No picks nothing).
    Confirmed(Confirm),
    /// A review target the `/review` menus settled on (the instructions typed, a branch, a
    /// commit, or the uncommitted changes): the app starts the review on the active thread.
    Review(crate::threads::ReviewTarget),
    /// What the worktree menus settled on; the app acts.
    Worktree(worktree::Pick),
    /// A saved preset's input, to be submitted as the user typed it (`RecallQuickDial`): the
    /// session's composer takes it.
    Recall(String),
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
    ReasoningBound {
        direction: reasoning::Direction,
        effort: Effort,
    },
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
    /// A line of the worktree pickers and of the conversation started in a worktree.
    Worktree(worktree::WorktreeLine),
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
    PresetBroken {
        slot: u8,
        reason: quick_dials::Invalid,
    },
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
            | Note::PresetsUnavailable
            | Note::PresetInvalid(_)
            | Note::PresetBroken { .. }
            | Note::PresetOutside(_)
            | Note::PresetFailed { .. }
            | Note::PermissionArgs(_)
            | Note::PermissionRefused(_) => Tone::Error,
            Note::Worktree(line) if line.error() => Tone::Error,
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
    /// The models the pickers list.
    pub catalog: Catalog,
    /// The current conversation's visit to the live model picker.
    pub model_visit: u64,
    pub(crate) plan_defaults: BTreeMap<BackendId, Effort>,
    /// The newest acknowledged Plan config write on each backend.
    pub(crate) last_plan_write: BTreeMap<BackendId, Ticket>,
    /// The permission profiles the pickers list.
    pub profiles: Profiles,
    /// The apps `/apps` lists.
    pub apps: apps::Apps,
    /// The `/review` menus' working directory and the git list asked for.
    pub review: review::Review,
    /// The scope and the requests of the worktree menus.
    pub worktree: worktree::Worktree,
    /// The sheets asked for and not shown yet.
    pub shows: Shows,
    /// The presets saved in the config and the writes asked for them.
    pub quick: quick_dials::Quick,
    /// Picks made; the effect that stores them is the settings slice's.
    pub picked: Vec<Picked>,
    /// Notes for the history; whoever shows them takes them.
    pub notes: Vec<Note>,
    /// An inline `/permissions <profile>` pick failed or was given up: what waits in the thread's
    /// queue stays unsent (`retain_input_after_failed_permission_selection`). The app takes it.
    pub hold_queue: bool,
}

impl Model {
    /// `/apps` (`add_connectors_output`): the directory is asked for again,
    /// and what it held before decides what the user sees: the sheet, or a note.
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
            _ => self.shows.push(Opening::Apps),
        }
    }

    /// An app of the directory was picked and has no page to open (`App::link` is none): the
    /// note says why. The sheet opens the link dialog of an app that has one.
    fn app_unlinked(&mut self, id: &str) {
        if let Some(app) = self.apps.app(id) {
            let status = app.status();
            self.notes.push(Note::AppLinkUnavailable(status));
        }
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

    /// `/presets`: the sheet of the presets on the page asked for, or a preset recalled.
    fn open_quick(&mut self, gate: quick_dials::Gate, open: quick_dials::Open) {
        use quick_dials::{Menu as Quick, Open, PickerMode};
        if !self.quick_available(gate) {
            return;
        }
        let page = match open {
            // `/presets` (`open_quick_dial_menu`): the shortcut menu is the composer's own drawn
            // state; the model only decided it may open (the gate's notes are said above).
            Open::Menu => return,
            Open::Picker(mode) => Quick::Slots { mode, dial: None },
            Open::Save(dial) => {
                if let Err(reason) = dial.validate() {
                    self.notes.push(Note::PresetInvalid(reason));
                    return;
                }
                Quick::Slots {
                    mode: PickerMode::Save,
                    dial: Some(dial),
                }
            }
            Open::Editor {
                target,
                dial,
                composer,
            } => {
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
                Quick::Editor { target, dial }
            }
            Open::Prefills(settings) => {
                Quick::Sources(quick_dials::sources(&settings, &self.catalog))
            }
            Open::Recall(slot) => {
                let Some(saved) = self.quick.get(slot) else {
                    return;
                };
                match saved.dial.validate() {
                    Ok(()) => self.picked.push(Picked::Recall(saved.dial.input.clone())),
                    Err(reason) => self.notes.push(Note::PresetBroken { slot, reason }),
                }
                return;
            }
        };
        self.shows.push(Opening::Quick(Box::new(page)));
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

    /// `restore_quick_dial_editor`: what was being saved is not lost; the sheet that asked for the
    /// write is gone, a new one opens on the editor.
    fn restore_editor(&mut self, slot: u8, dial: Option<quick_dials::Dial>) {
        if let Some(dial) = dial {
            self.shows
                .push(Opening::Quick(Box::new(quick_dials::Menu::Editor {
                    target: quick_dials::Target::Slot(slot),
                    dial,
                })));
        }
    }

    /// `dismiss_stale_model_picker`: the request the pickers waited for is dropped, and the sheet
    /// of the family (which reads `Catalog::provider`) closes itself.
    fn dismiss_stale_models(&mut self) {
        self.catalog.pending = None;
        self.shows.forget_models();
    }

    /// Whether the app directory knows the app `id` as installed (the setup of a plugin's apps
    /// reads it).
    pub fn app_installed(&self, id: &str) -> bool {
        self.apps
            .ready()
            .is_some_and(|apps| apps.iter().any(|app| app.id == id && app.installed))
    }

    /// An inline pick is given up (`recover_inline_permission_profile_selection`): what waits in
    /// the queue stays unsent. False when none was pending.
    fn abandon_inline(&mut self) -> bool {
        let pending = self.profiles.inline.take().is_some();
        self.hold_queue |= pending;
        pending
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
        self.profiles.inline = Some(Inline::Discovering {
            args,
            current: current.clone(),
        });
        if !self.apply_inline() {
            self.open_permissions(current);
        }
    }

    /// The inline pick waiting for the catalog, applied now that it is held
    /// (`apply_pending_permission_profile`): the profile is picked, or asks for the confirmation
    /// (a sheet of its own), or is refused with a note. False when nothing waits or the catalog is
    /// not held.
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
        match permission_args::resolve(&args, &discovery, current.guardian) {
            Ok(selection) => match permissions::settle(&current, selection) {
                permissions::Chosen::Menu(menu) => self.shows.push(Opening::Permissions(menu)),
                permissions::Chosen::Done(selection) => self.set_permissions(selection),
                permissions::Chosen::Retry => {}
            },
            Err(refusal) => {
                self.notes.push(Note::PermissionRefused(refusal));
                self.abandon_inline();
            }
        }
        true
    }

    /// The pick of the permission menus is final: `complete_inline_permission_profile_selection`
    /// settles an inline one, the app applies it.
    fn set_permissions(&mut self, selection: permissions::Selection) {
        self.profiles.inline = None;
        self.picked.push(Picked::Permissions(selection));
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

    /// The catalog is wanted (`request_permission_profiles`), with the model catalog the cyber
    /// warning of the confirmation reads (a request in flight serves as well).
    fn want_profiles(&mut self) {
        if self.catalog.presets.is_empty() && self.catalog.pending.is_none() {
            self.catalog.request();
        }
        self.profiles.want();
    }

    /// `open_permissions_popup`: a sheet of the legacy list of presets, or (with a thread, the
    /// explicit profile mode or a custom active profile) of the server's catalog: the menu on the
    /// one held, or `Loading…` and a request while there is none.
    fn open_permissions(&mut self, current: permissions::Current) {
        let menu = permissions::PermissionsMenu::start(current);
        self.want_profiles();
        self.shows.push(Opening::Permissions(menu));
    }

    /// The answer of the `permissionProfile/list` request (`on_permission_profiles_loaded`): the
    /// catalog the menus read, or why there is none. An inline pick takes the catalog first.
    fn discovered(&mut self, result: Result<permissions::Discovery, permissions::Failure>) {
        match result {
            Ok(discovery) => {
                self.profiles.found = Some(discovery);
                self.profiles.failure = None;
                self.apply_inline();
            }
            Err(failure) => self.profiles.failure = Some(failure),
        }
    }

    /// `open_model_popup_with_presets`: the quick menu, or a note when the
    /// catalog has nothing to offer.
    fn open_models(&mut self, current: Current, operation: Option<QueueOperation>) {
        if let Some(why) = self.catalog.unavailable() {
            self.notes.push(Note::ModelPickerUnavailable(why));
            return;
        }
        let request = self.catalog.request();
        let menu = if self.catalog.presets.is_empty() {
            ModelMenu::Loading { current, request }
        } else {
            let menu = ModelMenu::Models(current);
            if menu.is_empty(&self.catalog) {
                self.notes.push(if menu.is_reserve() {
                    Note::ReserveUnavailable
                } else {
                    Note::NoAdditionalModels
                });
                return;
            }
            menu
        };
        self.shows.push_owned(Opening::Models(menu), operation);
    }

    /// A catalog arrived (the startup one, or the answer of a request): a
    /// catalog that differs and is not empty replaces the held one. The pickers open on it show
    /// it without being reopened (they read it), and one left with no choice gives itself up
    /// (`refresh_open_model_picker`: `Intent::GiveUp`).
    fn accept(&mut self, presets: Vec<Preset>) {
        if presets.is_empty() || self.catalog.presets == presets {
            return;
        }
        self.catalog.presets = presets;
    }
}

/// The events of this screen (backend results, tickets); a variant of the
/// root `ScreenEv`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ev {
    /// A sheet is asked for (a seed of the debug entries; the intents that ask for one make
    /// the show themselves).
    Showing(Opening),
    /// The catalog the app started with.
    Loaded(Vec<Preset>),
    /// The default provider's write state changed (`provider_write`): a write in flight or
    /// committed closes the sheet of the family and drops the answer in flight
    /// (`dismiss_stale_model_picker`), and blocks `/model` until the restart.
    Provider(Provider),
    /// The permission catalog the app holds; the menus open on it show it in
    /// place.
    Listed(permissions::Discovery),
    /// A sheet is open as if its opener had run, and the catalog could not be read when `failure`
    /// is told (a seed of the debug entries: the only producer).
    Seeded {
        opening: Opening,
        failure: Option<permissions::Failure>,
    },
    /// The answer of the `permissionProfile/list` request `request`
    /// (`Profiles::pending`); the profiles of every page and the
    /// requirements, or why they could not be read.
    Discovered {
        request: u64,
        result: Result<permissions::Discovery, permissions::Failure>,
    },
    /// The account or the scope changed (`invalidate_permission_discovery`):
    /// the answer in flight is no longer wanted and the menus that waited on
    /// it or showed it go (`Profiles::scope` tells them).
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
}

/// A model request may belong to one issued command of a thread's queue.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelOpen {
    pub operation: Option<QueueOperation>,
    pub current: Current,
}

/// A result returned when a model sheet closes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelResult {
    Picked(Selection),
    GaveUp(GaveUp),
}

/// The sheet's result or the reason a page left while another page remains.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelCompletion {
    Inline {
        selection: Selection,
        scope: Option<models::Scope>,
    },
    Queued {
        operation: QueueOperation,
        result: ModelResult,
    },
    QueuedPageGaveUp {
        operation: QueueOperation,
        why: GaveUp,
    },
}

pub(crate) fn model_picker_note(why: GaveUp) -> Note {
    match why {
        GaveUp::NotLoaded => Note::ModelsNotLoaded,
        GaveUp::Empty { reserve: true } => Note::ReserveUnavailable,
        GaveUp::Empty { reserve: false } => Note::NoAdditionalModels,
    }
}

/// A queued model pick may have a config write in addition to its thread update.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QueuedModelWrite {
    Applying(QueueOperation),
    SavingDefault(QueueOperation),
}

impl QueuedModelWrite {
    pub(crate) fn operation(&self) -> &QueueOperation {
        match self {
            Self::Applying(operation) | Self::SavingDefault(operation) => operation,
        }
    }
}

/// The intents of this screen; a variant of the root `ScreenIntent`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// The answer of the plan-implementation question (`chat::plan_mode::Answer`): the question
    /// is a list of the sheet's own (`sheets/select/generic.rs`), only its answer is the domain's.
    PlanAnswer(crate::screens::chat::plan_mode::Answer),
    /// `/model`: the quick menu on the catalog held (`Loading models…` while
    /// it is empty), and the catalog is asked for again: the request is
    /// `Catalog::pending`.
    OpenModels(Box<ModelOpen>),
    /// A model sheet settled on a choice or an owned picker reported its result.
    Settle(ModelCompletion),
    /// An inline model sheet gave up because its models failed to load or there were no choices.
    GiveUp(GaveUp),
    /// The catalog is asked for, with no menu: the app does it when it connects.
    LoadModels,
    /// `/permissions`: the menu for what the thread has now; the catalog is
    /// asked for when it has none (`Profiles::pending` is the request).
    OpenPermissions(permissions::Current),
    /// `/permissions <profile> ..`: the profile is picked without a menu once the catalog is
    /// read (`select_permission_profile`); `current` is what the thread has now.
    PermissionsArgs {
        args: String,
        current: permissions::Current,
    },
    /// The next or previous built-in mode (`handle_permission_shortcut`): the pick of the cycle,
    /// or the catalog asked for when none is held.
    PermissionShortcut {
        forward: bool,
        current: permissions::Current,
    },
    /// The permission menus settled on a pick (the sheet's, or the confirmation's Continue): the
    /// app applies it and the inline pick that waited is the settled one.
    SetPermissions(permissions::Selection),
    /// A page of the permission menus waits for the catalog (`Retry`, or a menu opened again): it
    /// is asked for unless it is held or on its way.
    LoadProfiles,
    /// The permission sheet was left without a pick: an inline pick still waiting on it is given
    /// up (`cancel_pending_permission_profile_selection`).
    AbandonPermissions,
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
    /// The review a page of `/review` settled on (a preset, a branch, a commit, the instructions
    /// typed): the app starts it on the active thread.
    Review(crate::threads::ReviewTarget),
    /// A page of `/review` asks for a list of the host (the base branches, the commit log).
    ReviewList(review::Git),
    /// What a page of the worktree sheet settled on or asked for (`worktree::Verb`).
    Worktree(worktree::Verb),
    /// The app directory is read again (`apps.refresh(true)`): a surface that waited for an app
    /// to be installed asks for it.
    RefreshApps,
    /// `/presets`: a menu of the presets, or a preset recalled; `gate` says whether presets can
    /// be used where the user is.
    Quick {
        gate: quick_dials::Gate,
        open: quick_dials::Open,
    },
    /// What a page of `/presets` asks of the model (`quick_dials::Ask`): a write of a slot, or
    /// the note that the text typed is not a preset.
    Preset(quick_dials::Ask),
    /// The link dialog of an app switched it on or off (`SetAppEnabled`): the dialog shows the
    /// switch at once, the app writes the config and, once it is saved, tells the directory
    /// (`Ev::AppSwitched`).
    SetApp { id: String, enabled: bool },
    /// The link dialog of the app `id` opens its page in the browser (`AppEvent::OpenUrlInBrowser`):
    /// the app does it.
    OpenAppPage(String),
    /// The app `id` was picked from the directory and has no page to open: the note says why.
    AppUnlinked(String),
    /// A confirmation is asked for (`/archive`, `/delete`): a sheet shows it.
    Confirm(Confirm),
    /// The answer Yes of a confirmation (No is the sheet's own: it closes).
    Confirmed(Confirm),
    /// The sheet of show `seq` is shown (the presenter says so).
    Shown(u64),
    /// `/fast`, `/ultrafast`...: the tier `id` is switched on, or off when it is `current`
    /// (`toggle_service_tier_from_ui`).
    ServiceTier { id: String, current: Option<String> },
    /// A reasoning shortcut on the thread `current` describes: `plan` is the thread in Plan
    /// mode (the step is its override only), `started` the session being configured.
    Reasoning {
        direction: reasoning::Direction,
        current: Current,
        plan: bool,
        started: bool,
    },
}

impl Intent {
    pub fn open_models(current: Current) -> Self {
        Self::OpenModels(Box::new(ModelOpen {
            operation: None,
            current,
        }))
    }

    pub fn settle(selection: Selection) -> Self {
        Self::settle_model(selection, None)
    }

    pub fn settle_model(selection: Selection, scope: Option<models::Scope>) -> Self {
        Self::Settle(ModelCompletion::Inline { selection, scope })
    }
}

pub fn apply(model: &mut Model, ev: Ev, fx: &mut AppFx) {
    apply_ev(model, ev, fx);
}

fn apply_ev(model: &mut Model, ev: Ev, fx: &mut AppFx) {
    let _ = fx;
    match ev {
        Ev::Showing(opening) => model.shows.push(opening),
        Ev::Loaded(presets) => model.accept(presets),
        // The list stays as a fact under its request (`Worktree::listing`): the loading page
        // replaces itself with the browser, or closes; a failure is said unless it was given up.
        Ev::Worktrees { request, result } => {
            if let Some(error) = model.worktree.answered(request, result) {
                model
                    .notes
                    .push(Note::Worktree(worktree::WorktreeLine::ListFailed(error)));
            }
        }
        Ev::WorktreeRemoved { request, result } => {
            if let Some(root) = model.worktree.removed(request) {
                model.notes.push(match result {
                    Ok(()) => Note::Worktree(worktree::WorktreeLine::Removed { root }),
                    Err(error) => {
                        Note::Worktree(worktree::WorktreeLine::RemoveFailed { root, error })
                    }
                });
            }
        }
        Ev::Dials(slots) => model.quick.set(slots),
        Ev::PresetWritten { slot, dial, result } => match result {
            Ok(()) => {
                model.quick.written(quick_dials::Write {
                    slot,
                    dial: dial.clone(),
                });
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
        // The list stays as a fact under its request: the sheet over the presets pushes the page
        // of it (once per request) if it is still open.
        Ev::Reviewed { request, listing } => model.review.answered(request, listing),
        Ev::Listed(discovery) => {
            model.profiles.found = Some(discovery);
            model.profiles.failure = None;
        }
        Ev::Seeded { opening, failure } => {
            model.profiles.failure = failure;
            model.shows.push(opening);
        }
        Ev::Discovered { request, result } => {
            if model.profiles.pending != Some(request) {
                return;
            }
            model.profiles.pending = None;
            model.discovered(result);
        }
        Ev::ScopeChanged => {
            model.profiles.pending = None;
            model.profiles.scope += 1;
            // `invalidate_permission_discovery`: an inline pick waiting on the old scope is given up.
            model.abandon_inline();
            model.apps.invalidate();
            model.worktree.rescope();
        }
        Ev::AppsLoaded { request, result } => {
            model.apps.loaded(request, result);
        }
        Ev::AppsNotified(apps) => {
            model.apps.notified(apps);
        }
        Ev::AppSwitched { id, enabled } => model.apps.switched(&id, enabled),
        Ev::Fetched { request, result } => {
            // Another request, or none: the answer is not for the pickers.
            if model.catalog.pending != Some(request) {
                return;
            }
            model.catalog.pending = None;
            // The sheet that waited on `Loading models…` reads the answer under its request and
            // goes on (the quick menu, or the note of what it could not do).
            match result {
                Ok(presets) => {
                    model.catalog.answered = Some((request, Answer::Listed));
                    model.accept(presets);
                }
                Err(_) => model.catalog.answered = Some((request, Answer::Failed)),
            }
        }
    }
}

pub fn apply_intent(model: &mut Model, intent: Intent, fx: &mut AppFx) {
    let _ = fx;
    match intent {
        Intent::Confirm(confirm) => model.shows.push(Opening::Confirm(confirm)),
        Intent::Confirmed(confirm) => model.picked.push(Picked::Confirmed(confirm)),
        Intent::Shown(seq) => model.shows.shown(seq),
        Intent::OpenModels(request) => {
            let ModelOpen { operation, current } = *request;
            model.open_models(current, operation);
        }
        Intent::Settle(ModelCompletion::Inline { selection, .. }) => {
            model.picked.push(Picked::Selection(selection))
        }
        Intent::GiveUp(why) => model.notes.push(model_picker_note(why)),
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
        Intent::SetPermissions(selection) => model.set_permissions(selection),
        Intent::LoadProfiles => model.want_profiles(),
        Intent::AbandonPermissions => {
            model.abandon_inline();
        }
        Intent::ServiceTier { id, current } => {
            let next = if current.as_deref() == Some(id.as_str()) {
                codex_protocol::config_types::SERVICE_TIER_DEFAULT_REQUEST_VALUE.to_string()
            } else {
                id
            };
            model.picked.push(Picked::Tier(next));
        }
        Intent::Reasoning {
            direction,
            current,
            plan,
            started,
        } => match reasoning::step(&model.catalog, &current, direction, started) {
            reasoning::Outcome::Set(effort) => {
                model.picked.push(Picked::Effort {
                    model: current.model,
                    effort,
                    plan,
                });
            }
            reasoning::Outcome::Say(note) => model.notes.push(note),
        },
        Intent::OpenApps { enabled } => model.open_apps(enabled),
        // `/review`: the presets, over the session's working directory.
        Intent::OpenReview { cwd } => {
            model.review.open(cwd);
            model.shows.push(Opening::Review);
        }
        // `/new` and `/fork` (`show_session_checkout_picker`): the choice of checkout, or none
        // where there is no local repository to make one in.
        Intent::OpenCheckout {
            mode,
            name,
            available,
            cwd,
            thread,
        } => {
            model.worktree.open(cwd, thread);
            if available {
                let menu = worktree::Menu::Checkout { mode, name };
                model.shows.push(Opening::Worktree(menu));
            } else {
                model
                    .picked
                    .push(Picked::Worktree(worktree::Pick::Current { mode, name }));
            }
        }
        // `/worktree` (`show_managed_worktree_picker`).
        Intent::OpenWorktrees {
            available,
            cwd,
            thread,
        } => {
            if available {
                model.worktree.open(cwd, thread);
                model
                    .shows
                    .push(Opening::Worktree(worktree::Menu::Worktrees));
            } else {
                model
                    .notes
                    .push(Note::Worktree(worktree::WorktreeLine::Unavailable));
            }
        }
        // What a page of `/review` settled on (`Picked::Review`), and the list a page asks of the
        // host (the answer is `Ev::Reviewed`).
        Intent::Review(target) => model.picked.push(Picked::Review(target)),
        Intent::ReviewList(git) => model.review.want(git),
        // What a page of the worktree sheet said (`worktree::Verb`).
        Intent::Worktree(verb) => {
            if let Some(pick) = model.worktree.say(verb) {
                model.picked.push(Picked::Worktree(pick));
            }
        }
        Intent::RefreshApps => {
            model.apps.refresh(true);
        }
        Intent::Quick { gate, open } => model.open_quick(gate, open),
        // What a page of the presets asked: the write (the sheet closed), or why the text is no preset.
        Intent::Preset(ask) => match ask {
            quick_dials::Ask::Write(quick_dials::Write { slot, dial }) => {
                model.quick_write(slot, dial);
            }
            quick_dials::Ask::Invalid(reason) => model.notes.push(Note::PresetInvalid(reason)),
        },
        // The app writes the config (`app/select.rs`); the directory follows the saved write. The
        // dialog of the app shows its switch at once, the screen keeps no copy of it.
        Intent::SetApp { .. } => {}
        // The page of the app is opened by the app (`app/select.rs`); the screen keeps no record.
        Intent::OpenAppPage(_) => {}
        Intent::AppUnlinked(id) => model.app_unlinked(&id),
        // The answer is acted on by the app (`app/select.rs`); the screen keeps no record of it.
        Intent::PlanAnswer(_) | Intent::Settle(_) => {}
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
        operation: Option<QueueOperation>,
    },
    /// A pick's write: the pick whose line follows a saved write.
    Write {
        setting: Setting,
        then: Option<Note>,
        queued: Option<QueuedModelWrite>,
    },
    PlanWrite(Box<collaboration::PlanWrite>),
    /// The write of an app's switch: the directory takes it once it is saved.
    App {
        id: String,
        enabled: bool,
    },
    /// The write of a preset: the slot follows once it is saved.
    Preset {
        slot: u8,
        dial: Option<quick_dials::Dial>,
    },
    /// `model/list` for an automatic model switch of a usage banner that finds no catalog
    /// (`app/banners.rs`).
    Fallback {
        backend: crate::ids::BackendId,
    },
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
/// "menu":"profiles","current":{..}}`, `{"kind":"dials","slots":[{"slot":1,"input":"..","label":".."}]}` (the presets the config holds).
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
    /// The presets of the user config (`Ev::Dials`).
    Dials {
        slots: Vec<SeedSlot>,
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

/// A preset the user config holds, as a script tells it.
#[derive(Clone, Debug, serde::Deserialize)]
struct SeedSlot {
    slot: u8,
    input: String,
    #[serde(default)]
    label: Option<String>,
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
    let model = |menu| Ev::Showing(Opening::Models(menu));
    Ok(match seed {
        SeedEv::Catalog { presets } => Ev::Loaded(presets),
        SeedEv::Provider { state } => Ev::Provider(state),
        SeedEv::Loading { current } => model(ModelMenu::Loading {
            current,
            request: 0,
        }),
        SeedEv::Models { current } => model(ModelMenu::Models(current)),
        SeedEv::AllModels { current } => model(ModelMenu::AllModels(current)),
        SeedEv::Reasoning { preset, current } => model(ModelMenu::Reasoning {
            preset,
            current: Box::new(current),
        }),
        SeedEv::Advanced { preset, current } => model(ModelMenu::Advanced {
            preset,
            current: Box::new(current),
        }),
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
            scope: None,
        }),
        SeedEv::Dials { slots } => Ev::Dials(
            slots
                .into_iter()
                .map(|seed| quick_dials::Slot {
                    slot: seed.slot,
                    dial: quick_dials::Dial {
                        input: seed.input,
                        label: seed.label,
                    },
                    origin: quick_dials::Origin::User,
                })
                .collect(),
        ),
        SeedEv::Profiles { discovery } => Ev::Listed(discovery),
        SeedEv::Permissions {
            menu,
            current,
            failure,
        } => {
            use permissions::PermissionsMenu as Perms;
            // The catalog menus differ by what the model holds: a read that failed is a fact.
            let (menu, failure) = match menu {
                SeedPermissions::Presets => (Perms::Presets(current), None),
                SeedPermissions::Loading => (Perms::Catalog(current), None),
                SeedPermissions::Profiles => (Perms::Catalog(current), None),
                SeedPermissions::Failed => (
                    Perms::Catalog(current),
                    Some(permissions::Failure::Other(failure.unwrap_or_default())),
                ),
                SeedPermissions::FullAccess => {
                    let pick = permissions::Selection::Preset {
                        preset: permissions::Preset::FullAccess,
                        reviewer: permissions::Reviewer::User,
                    };
                    (Perms::FullAccess { current, pick }, None)
                }
            };
            Ev::Seeded {
                opening: Opening::Permissions(menu),
                failure,
            }
        }
    })
}
