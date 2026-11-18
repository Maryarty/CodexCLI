//! `settings` (S15, settings dialogs): the core half of the screen's directory. The model
//! part, its events and intents, its RPCs and their protocol wiring live here,
//! with no rtwk; the view half is `src/screens/settings/` in the UI crate. Nothing
//! outside these two directories (bar their test files) needs an edit when it
//! grows.

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;

use crate::app::AppFx;
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::screens::Kind;
use crate::threads::ThreadFx;

pub const NAME: &str = "settings";
pub const ID: &str = "S15";
pub const KIND: Kind = Kind::Dialog;

use std::collections::BTreeMap;

/// The groups of the configuration panel (`config_panel/mod.rs`:
/// `ConfigSection`), in the oracle's order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Section {
    Appearance,
    Notifications,
    Input,
    Sessions,
    TextSelection,
}

impl Section {
    pub const ALL: [Section; 5] = [
        Section::Appearance,
        Section::Notifications,
        Section::Input,
        Section::Sessions,
        Section::TextSelection,
    ];
}

/// What a row of the panel edits (`ConfigToggle` / `ConfigEditor`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SettingId {
    Theme,
    Effects,
    Tips,
    StatusLineColors,
    StatusLineDisplay,
    TerminalTitleDisplay,
    Notifications,
    NotifyWhen,
    TestNotification,
    KeyboardShortcuts,
    StartInVimMode,
    PasteDetection,
    MouseScrollSpeed,
    AutomaticRecaps,
    SessionPickerLayout,
    ResumeDirectory,
    ModelProviders,
    CollapseTurnDetails,
    CopyOnSelect,
    DeselectAfterCopy,
    RightClickPaste,
}

/// The value column of a row (`ConfigSettingValue`).
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Value {
    /// `[x] On` / `[ ] Off`: Enter flips it.
    Toggle(bool),
    /// `<label> ›`: Enter opens its editor.
    Choice(String),
    /// `Configure ›`.
    Configure,
    /// `Send in 5s`.
    Test,
    /// `Unavailable`.
    Unavailable,
}

/// One setting of the panel; the texts are the catalogue's (the oracle builds
/// them from its config).
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct Row {
    pub id: SettingId,
    pub section: Section,
    pub label: String,
    pub description: String,
    #[serde(default)]
    pub keywords: String,
    pub value: Value,
}

/// The configuration panel (`config_panel/`): one group at a time, or every
/// group while a search is typed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigPanel {
    pub rows: Vec<Row>,
    /// The group shown, an index into `Section::ALL`.
    pub tab: usize,
    /// The highlighted row, an index into `visible()`.
    pub cursor: usize,
    pub query: String,
}

impl ConfigPanel {
    pub fn new(rows: Vec<Row>) -> Self {
        Self {
            rows,
            tab: 0,
            cursor: 0,
            query: String::new(),
        }
    }

    /// The group shown.
    pub fn section(&self) -> Section {
        Section::ALL[self.tab]
    }

    /// The rows shown, as indices into `rows`.
    pub fn visible(&self) -> Vec<usize> {
        let _ = self;
        Vec::new()
    }
}

/// The syntax theme picker (`theme_picker.rs`): the names to pick from; the
/// theme in force while it is open is `Model::syntax_theme`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThemePicker {
    pub themes: Vec<String>,
    pub cursor: usize,
    /// The theme to restore when the picker is cancelled.
    pub original: Option<String>,
}

/// The id of the status-line setup's first row, "Use theme colors"
/// (`STATUS_LINE_USE_THEME_COLORS_ITEM_ID`).
pub const USE_THEME_COLORS: &str = "use_theme_colors";

/// One row of the status-line setup (`MultiSelectItem`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetupItem {
    pub id: String,
    pub description: String,
    pub enabled: bool,
    /// May be moved with the arrow keys.
    pub orderable: bool,
    pub section_break_after: bool,
    /// What the preview shows when the runtime has no value for the item.
    pub placeholder: String,
}

/// The status-line setup (`bottom_pane/status_line_setup.rs`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatusLineSetup {
    pub items: Vec<SetupItem>,
    /// Runtime values of the items, by id.
    pub values: BTreeMap<String, String>,
    pub cursor: usize,
    pub query: String,
}

impl StatusLineSetup {
    /// The rows shown, as indices into `items`.
    pub fn visible(&self) -> Vec<usize> {
        (0..self.items.len()).collect()
    }

    /// The status line the enabled items make, in order (`status_line_for_items`).
    pub fn preview(&self) -> String {
        let _ = self;
        String::new()
    }
}

/// One action of the keymap picker (`keymap_setup.rs`).
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct ActionItem {
    pub name: String,
    pub binding: String,
    /// What a search matches: context, action and description.
    pub search: String,
}

/// A tab of the keymap picker.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct KeymapTab {
    pub id: String,
    pub label: String,
    /// How many actions of the tab can be picked.
    pub selectable: usize,
    #[serde(default)]
    pub items: Vec<ActionItem>,
}

/// The keymap picker: the actions of the keymap, by tab.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeymapPicker {
    pub tabs: Vec<KeymapTab>,
    pub tab: usize,
    pub cursor: usize,
}

/// The settings dialog open now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Open {
    Config(ConfigPanel),
    Theme(ThemePicker),
    StatusLine(StatusLineSetup),
    Keymap(KeymapPicker),
}

/// What a settings dialog asked the app to do (the oracle's `AppEvent`s).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Edit {
    /// A toggle row was flipped (`PersistConfig`).
    Toggled(SettingId, bool),
    TestNotification,
    /// Enter on a row that opens an editor of its own.
    Editor(SettingId),
    /// `SyntaxThemeSelected`.
    ThemeSelected(String),
    /// `StatusLineSetup`: the enabled items in order.
    StatusLineSaved {
        items: Vec<String>,
        use_theme_colors: bool,
    },
    StatusLineCancelled,
    /// An action was picked in the keymap picker.
    KeymapPicked(String),
}

/// The app-scoped model state of this screen, changed only by `apply`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Model {
    pub open: Option<Open>,
    /// The syntax theme in force (the oracle: `highlight::current_syntax_theme`):
    /// the picker previews by changing it.
    pub syntax_theme: Option<String>,
    pub edits: Vec<Edit>,
}

/// The events of this screen (backend results, tickets); a variant of the
/// root `ScreenEv`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ev {
    /// A settings dialog was opened.
    Opened(Open),
}

/// The intents of this screen; a variant of the root `ScreenIntent`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// Left / Right: the group of the panel, the tab of the keymap picker.
    Tab(i32),
    /// Up / Down: the highlighted row; wraps.
    Move(i32),
    /// The search text changed.
    Search(String),
    /// Enter.
    Activate,
    /// Space in the status-line setup.
    Toggle,
    /// Left / Right in the status-line setup: move the row among the orderable.
    Reorder(i32),
    /// Esc.
    Close,
}

pub fn apply(model: &mut Model, ev: Ev, fx: &mut AppFx) {
    let _ = fx;
    match ev {
        Ev::Opened(open) => model.open = Some(open),
    }
}

pub fn apply_intent(model: &mut Model, intent: Intent, fx: &mut AppFx) {
    let _ = (model, fx);
    match intent {
        Intent::Tab(_)
        | Intent::Move(_)
        | Intent::Search(_)
        | Intent::Activate
        | Intent::Toggle
        | Intent::Reorder(_)
        | Intent::Close => {}
    }
}

/// Per-thread state of this screen: a part of every `ThreadModel`, changed
/// only by `apply_thread` and `apply_reply`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ThreadPart {}

/// An event of one thread for this screen; a variant of `ThreadEv::Screen`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThreadEv {}

/// The RPCs of this screen: requested with `ThreadFx::request`, which stamps
/// the thread and the ticket; the shell sends them through `wire`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {}

/// The replies of those RPCs, decoded by `decode`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {}

/// What an outgoing RPC of this screen was for, kept with its ticket.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ctx {}

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
    let _ = (part, result, fx);
    match ctx {}
}

/// The wire request of `command` for `thread`.
pub fn wire(command: &Command, thread: &ThreadId, id: RequestId) -> ClientRequest {
    let _ = (thread, id);
    match *command {}
}

/// The reply `result` (the JSON result of the request) is for `command`.
pub fn decode(command: &Command, result: serde_json::Value) -> Result<Reply, RpcError> {
    let _ = result;
    match *command {}
}

/// A script's `screen` step for this directory: the dialog `value` opens.
pub fn seed(value: serde_json::Value) -> Result<Ev, crate::screens::SeedError> {
    let _ = value;
    Err(crate::screens::SeedError::NoSeeds)
}
