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

    /// The group's name as the oracle's search sees it (`ConfigSection::title`).
    fn search_title(self) -> &'static str {
        match self {
            Section::Appearance => "Appearance",
            Section::Notifications => "Notifications",
            Section::Input => "Input",
            Section::Sessions => "Sessions",
            Section::TextSelection => "Text selection",
        }
    }
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

impl Value {
    /// The value column as the oracle's search sees it
    /// (`ConfigSettingValue::label`); what the view shows is its own table.
    fn search_label(&self) -> String {
        match self {
            Value::Toggle(true) => "[x] On".to_string(),
            Value::Toggle(false) => "[ ] Off".to_string(),
            Value::Choice(label) => format!("{label} ›"),
            Value::Configure => "Configure ›".to_string(),
            Value::Test => "Send in 5s".to_string(),
            Value::Unavailable => "Unavailable".to_string(),
        }
    }
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
        let query = self.query.trim().to_lowercase();
        self.rows
            .iter()
            .enumerate()
            .filter(|(_, row)| {
                if query.is_empty() {
                    row.section == self.section()
                } else {
                    // `config_panel/render.rs`: the row's `search_value`.
                    format!(
                        "{} {} {} {} {}",
                        row.section.search_title(),
                        row.label,
                        row.value.search_label(),
                        row.keywords,
                        row.description
                    )
                    .to_lowercase()
                    .contains(&query)
                }
            })
            .map(|(index, _)| index)
            .collect()
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
        let query: Vec<char> = self.query.trim().to_lowercase().chars().collect();
        self.items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                // `multi_select_picker.rs`: a fuzzy match on the name.
                let name = item.id.replace('_', " ").to_lowercase();
                let mut rest = query.iter().peekable();
                for c in name.chars() {
                    if rest.peek() == Some(&&c) {
                        rest.next();
                    }
                }
                rest.peek().is_none()
            })
            .map(|(index, _)| index)
            .collect()
    }

    /// The status line the enabled items make, in order (`status_line_for_items`).
    pub fn preview(&self) -> String {
        self.items
            .iter()
            .filter(|item| item.enabled && item.id != USE_THEME_COLORS)
            .map(|item| {
                self.values
                    .get(&item.id)
                    .unwrap_or(&item.placeholder)
                    .as_str()
            })
            .collect::<Vec<_>>()
            .join(" · ")
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

impl KeymapPicker {
    /// The actions of the tab shown.
    pub fn items(&self) -> &[ActionItem] {
        &self.tabs[self.tab].items
    }
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
        Ev::Opened(open) => {
            if let Open::Theme(picker) = &open {
                model.syntax_theme = picker.original.clone();
            }
            model.open = Some(open);
        }
    }
}

/// `index + delta` on a ring of `len` rows.
fn wrap(index: usize, delta: i32, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    (index as i64 + i64::from(delta)).rem_euclid(len as i64) as usize
}

pub fn apply_intent(model: &mut Model, intent: Intent, fx: &mut AppFx) {
    let _ = fx;
    let Some(open) = &mut model.open else {
        return;
    };
    match (open, intent) {
        (Open::Config(panel), Intent::Tab(delta)) => {
            panel.tab = wrap(panel.tab, delta, Section::ALL.len());
            panel.cursor = 0;
        }
        (Open::Keymap(picker), Intent::Tab(delta)) => {
            picker.tab = wrap(picker.tab, delta, picker.tabs.len());
            picker.cursor = 0;
        }
        (Open::Config(panel), Intent::Move(delta)) => {
            panel.cursor = wrap(panel.cursor, delta, panel.visible().len());
        }
        (Open::Theme(picker), Intent::Move(delta)) => {
            picker.cursor = wrap(picker.cursor, delta, picker.themes.len());
            if let Some(theme) = picker.themes.get(picker.cursor) {
                model.syntax_theme = Some(theme.clone());
            }
        }
        (Open::StatusLine(setup), Intent::Move(delta)) => {
            setup.cursor = wrap(setup.cursor, delta, setup.visible().len());
        }
        (Open::Keymap(picker), Intent::Move(delta)) => {
            picker.cursor = wrap(picker.cursor, delta, picker.items().len());
        }
        (Open::Config(panel), Intent::Search(query)) => {
            panel.query = query;
            panel.cursor = 0;
        }
        (Open::StatusLine(setup), Intent::Search(query)) => {
            setup.query = query;
            setup.cursor = 0;
        }
        (Open::Config(panel), Intent::Activate) => {
            let Some(row) = panel
                .visible()
                .get(panel.cursor)
                .copied()
                .and_then(|index| panel.rows.get_mut(index))
            else {
                return;
            };
            let id = row.id;
            match &mut row.value {
                Value::Toggle(enabled) => {
                    *enabled = !*enabled;
                    model.edits.push(Edit::Toggled(id, *enabled));
                }
                Value::Choice(_) | Value::Configure => model.edits.push(Edit::Editor(id)),
                Value::Test => model.edits.push(Edit::TestNotification),
                Value::Unavailable => {}
            }
        }
        (Open::Theme(picker), Intent::Activate) => {
            if let Some(theme) = picker.themes.get(picker.cursor).cloned() {
                model.syntax_theme = Some(theme.clone());
                model.edits.push(Edit::ThemeSelected(theme));
                model.open = None;
            }
        }
        (Open::StatusLine(setup), Intent::Activate) => {
            let items = setup
                .items
                .iter()
                .filter(|item| item.enabled && item.id != USE_THEME_COLORS)
                .map(|item| item.id.clone())
                .collect();
            let use_theme_colors = setup
                .items
                .iter()
                .any(|item| item.id == USE_THEME_COLORS && item.enabled);
            model.edits.push(Edit::StatusLineSaved {
                items,
                use_theme_colors,
            });
            model.open = None;
        }
        (Open::Keymap(picker), Intent::Activate) => {
            if let Some(item) = picker.items().get(picker.cursor) {
                model.edits.push(Edit::KeymapPicked(item.name.clone()));
            }
        }
        (Open::StatusLine(setup), Intent::Toggle) => {
            // An item with an action of its own is not a toggle; none here.
            if let Some(item) = setup
                .visible()
                .get(setup.cursor)
                .copied()
                .and_then(|index| setup.items.get_mut(index))
            {
                item.enabled = !item.enabled;
            }
        }
        (Open::StatusLine(setup), Intent::Reorder(delta)) => {
            if !setup.query.trim().is_empty() {
                return;
            }
            let at = setup.cursor;
            let to = match at.checked_add_signed(delta.signum() as isize) {
                Some(to) if to < setup.items.len() => to,
                _ => return,
            };
            if setup.items[at].orderable && setup.items[to].orderable {
                setup.items.swap(at, to);
                setup.cursor = to;
            }
        }
        (Open::Config(panel), Intent::Close) => {
            if panel.query.is_empty() {
                model.open = None;
            } else {
                panel.query.clear();
                panel.cursor = 0;
            }
        }
        (Open::Theme(picker), Intent::Close) => {
            model.syntax_theme = picker.original.clone();
            model.open = None;
        }
        (Open::StatusLine(_), Intent::Close) => {
            model.edits.push(Edit::StatusLineCancelled);
            model.open = None;
        }
        (Open::Keymap(_), Intent::Close) => model.open = None,
        _ => {}
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

/// One action of the status-line catalogue (`StatusLineItem` with its
/// description and preview placeholder).
#[derive(Clone, Debug, serde::Deserialize)]
struct CatalogueItem {
    id: String,
    description: String,
    placeholder: String,
}

/// The seed forms of this screen's events (`kind`): `config_panel` with its
/// `rows`; `theme_picker` with `themes` and the `current` one;
/// `status_line` with `use_theme_colors`, the `selected` ids in order, the
/// `catalogue` and the runtime `values`; `keymap` with its `tabs`.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum SeedEv {
    ConfigPanel {
        rows: Vec<Row>,
    },
    ThemePicker {
        themes: Vec<String>,
        #[serde(default)]
        current: Option<String>,
    },
    StatusLine {
        use_theme_colors: bool,
        selected: Vec<String>,
        catalogue: Vec<CatalogueItem>,
        #[serde(default)]
        values: BTreeMap<String, String>,
    },
    Keymap {
        tabs: Vec<KeymapTab>,
    },
}

/// `StatusLineSetupView::new`: the selected items first, in order and
/// enabled, then the rest disabled; "use theme colors" leads.
fn status_line(
    use_theme_colors: bool,
    selected: &[String],
    catalogue: Vec<CatalogueItem>,
    values: BTreeMap<String, String>,
) -> StatusLineSetup {
    let mut items = vec![SetupItem {
        id: USE_THEME_COLORS.to_string(),
        description: String::new(),
        enabled: use_theme_colors,
        orderable: false,
        section_break_after: true,
        placeholder: String::new(),
    }];
    let item = |entry: &CatalogueItem, enabled: bool| SetupItem {
        id: entry.id.clone(),
        description: entry.description.clone(),
        enabled,
        orderable: true,
        section_break_after: false,
        placeholder: entry.placeholder.clone(),
    };
    let mut used: Vec<&str> = Vec::new();
    for id in selected {
        if used.contains(&id.as_str()) {
            continue;
        }
        if let Some(entry) = catalogue.iter().find(|entry| &entry.id == id) {
            used.push(entry.id.as_str());
            items.push(item(entry, true));
        }
    }
    for entry in &catalogue {
        if !used.contains(&entry.id.as_str()) {
            items.push(item(entry, false));
        }
    }
    StatusLineSetup {
        items,
        values,
        cursor: 0,
        query: String::new(),
    }
}

/// A script's `screen` step for this directory: the dialog `value` opens.
pub fn seed(value: serde_json::Value) -> Result<Ev, crate::screens::SeedError> {
    let seed: SeedEv = serde_json::from_value(value)
        .map_err(|err| crate::screens::SeedError::Bad(err.to_string()))?;
    Ok(Ev::Opened(match seed {
        SeedEv::ConfigPanel { rows } => Open::Config(ConfigPanel::new(rows)),
        SeedEv::ThemePicker { themes, current } => {
            let cursor = current
                .as_ref()
                .and_then(|name| themes.iter().position(|theme| theme == name))
                .unwrap_or(0);
            Open::Theme(ThemePicker {
                themes,
                cursor,
                original: current,
            })
        }
        SeedEv::StatusLine {
            use_theme_colors,
            selected,
            catalogue,
            values,
        } => Open::StatusLine(status_line(use_theme_colors, &selected, catalogue, values)),
        SeedEv::Keymap { tabs } => {
            if tabs.is_empty() {
                return Err(crate::screens::SeedError::Bad("no tabs".into()));
            }
            Open::Keymap(KeymapPicker {
                tabs,
                tab: 0,
                cursor: 0,
            })
        }
    }))
}
