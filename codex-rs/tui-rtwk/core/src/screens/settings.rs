//! `settings` (S15, settings dialogs): the core half of the screen's directory. The model
//! part, its events and intents, its RPCs and their protocol wiring live here,
//! with no rtwk; the view half is `src/sheets/settings/` in the UI crate. Nothing
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
pub const KIND: Kind = Kind::Sheet;

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

/// The configuration panel (`config_panel/`): the rows of the catalogue. The
/// group shown, the highlighted row and the search text are the view's
/// (`TabController`, `SelectionController`, `TextEditingController`); what
/// they leave visible is a function of them (`visible`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigPanel {
    pub rows: Vec<Row>,
}

impl ConfigPanel {
    pub fn new(rows: Vec<Row>) -> Self {
        Self { rows }
    }

    /// The row editing `id`.
    pub fn row(&self, id: SettingId) -> Option<&Row> {
        self.rows.iter().find(|row| row.id == id)
    }

    /// The rows of `section`, or, while `query` has text, every row that
    /// matches it; as indices into `rows`.
    pub fn visible(&self, section: Section, query: &str) -> Vec<usize> {
        let query = query.trim().to_lowercase();
        self.rows
            .iter()
            .enumerate()
            .filter(|(_, row)| {
                if query.is_empty() {
                    row.section == section
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
    /// The names among `themes` that are the user's own `.tmTheme` files
    /// (shown with "(custom)").
    pub custom: Vec<String>,
    /// The theme to restore when the picker is cancelled; the row the cursor
    /// starts on.
    pub original: Option<String>,
    /// The directory of the custom themes as the subtitle names it
    /// (`~/.codex/themes`); none when it is not below the home directory.
    pub themes_dir: Option<String>,
}

impl ThemePicker {
    /// The themes a search leaves, as indices into `themes`: those whose
    /// name holds the text typed, in any case (`SelectionItem::search_value`).
    pub fn visible(&self, query: &str) -> Vec<usize> {
        let query = query.to_lowercase();
        self.themes
            .iter()
            .enumerate()
            .filter(|(_, name)| name.to_lowercase().contains(&query))
            .map(|(index, _)| index)
            .collect()
    }

    /// The theme is one of the user's own files.
    pub fn is_custom(&self, name: &str) -> bool {
        self.custom.iter().any(|custom| custom == name)
    }

    /// The row the cursor starts on: the theme in force.
    pub fn initial(&self) -> usize {
        self.original
            .as_ref()
            .and_then(|name| self.themes.iter().position(|theme| theme == name))
            .unwrap_or(0)
    }
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

/// Which surface a setup configures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetupKind {
    /// `bottom_pane/status_line_setup.rs`.
    StatusLine,
    /// `bottom_pane/title_setup.rs`: the terminal title.
    Title,
}

/// The status-line and terminal-title setups: `MultiSelectPicker` with the
/// items of the surface. The highlighted row and the search text are the
/// view's; the order and the checks are the model's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Setup {
    pub kind: SetupKind,
    pub items: Vec<SetupItem>,
    /// Runtime values of the items, by id.
    pub values: BTreeMap<String, String>,
}

/// The id of the title's activity item, the one that needs spaces around it.
pub const ACTIVITY: &str = "activity";

/// What the preview of a title with `activity` starts with
/// (`ACTION_REQUIRED_PREVIEW_PREFIX`).
pub const ACTION_REQUIRED_PREFIX: &str = "[ ! ] Action Required";

impl Setup {
    /// The rows a search leaves, as indices into `items`: a fuzzy match on the
    /// id, best score first, ties by name (`MultiSelectPicker::apply_filter`);
    /// every row in order while the search is empty.
    pub fn visible(&self, query: &str) -> Vec<usize> {
        let query = query.trim();
        if query.is_empty() {
            return (0..self.items.len()).collect();
        }
        let mut found: Vec<(usize, i32)> = self
            .items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| {
                crate::commands::fuzzy_match(&item.id, query).map(|(_, score)| (index, score))
            })
            .collect();
        found.sort_by(|a, b| {
            a.1.cmp(&b.1)
                .then_with(|| self.items[a.0].id.cmp(&self.items[b.0].id))
        });
        found.into_iter().map(|(index, _)| index).collect()
    }

    /// The item `id`, as an index into `items`.
    pub fn position(&self, id: &str) -> Option<usize> {
        self.items.iter().position(|item| item.id == id)
    }

    /// The line the enabled items make, in order: the status line joins the
    /// values with ` · ` (`status_line_for_items`); the title with ` | `, and
    /// leads with the action-required prefix when it holds the activity item
    /// (`preview_line_for_title_items`).
    pub fn preview(&self) -> String {
        let values = self
            .items
            .iter()
            .filter(|item| item.enabled && item.id != USE_THEME_COLORS && item.id != ACTIVITY)
            .map(|item| {
                self.values
                    .get(&item.id)
                    .unwrap_or(&item.placeholder)
                    .as_str()
            })
            .filter(|value| !value.is_empty());
        match self.kind {
            SetupKind::StatusLine => values.collect::<Vec<_>>().join(" · "),
            SetupKind::Title => {
                let activity = self
                    .items
                    .iter()
                    .any(|item| item.enabled && item.id == ACTIVITY);
                let mut parts: Vec<&str> = Vec::new();
                if activity {
                    parts.push(ACTION_REQUIRED_PREFIX);
                }
                parts.extend(values);
                parts.join(" | ")
            }
        }
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

impl ActionItem {
    /// No shortcut is bound to the action.
    pub fn is_unbound(&self) -> bool {
        self.binding == "unbound"
    }
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
}

impl KeymapPicker {
    /// The rows of `tab` a search leaves, as indices into its items: those
    /// whose search text holds every word typed (the oracle ranks by fuzzy
    /// match; here a plain match in the catalogue's order).
    pub fn visible(&self, tab: usize, query: &str) -> Vec<usize> {
        let query = query.trim().to_lowercase();
        let words: Vec<&str> = query.split_whitespace().collect();
        self.tabs[tab]
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                let hay = item.search.to_lowercase();
                words.iter().all(|word| hay.contains(word))
            })
            .map(|(index, _)| index)
            .collect()
    }
}

/// The settings dialog open now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Open {
    Config(ConfigPanel),
    Theme(ThemePicker),
    Setup(Setup),
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
    /// `StatusLineSetup` / `TerminalTitleSetup`: the enabled items in order.
    SetupSaved {
        kind: SetupKind,
        items: Vec<String>,
        use_theme_colors: bool,
    },
    SetupCancelled(SetupKind),
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

/// The intents of this screen; a variant of the root `ScreenIntent`. The
/// cursor, the group tab and the search text are the view's controllers; an
/// intent names what it acts on by id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// Enter on a row of the panel: flips a toggle, opens the editor of the
    /// row, sends the test notification.
    Activate(SettingId),
    /// The cursor of the theme picker is on this theme: it is shown.
    Preview(String),
    /// Enter in the theme picker.
    Pick(String),
    /// Space in a setup: flips the item.
    Toggle(String),
    /// Left / Right in a setup: moves the item one step among the orderable
    /// ones.
    Move { id: String, delta: i32 },
    /// Enter in a setup.
    Save,
    /// Enter in the keymap picker.
    KeymapPick(String),
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

pub fn apply_intent(model: &mut Model, intent: Intent, fx: &mut AppFx) {
    let _ = fx;
    let Some(open) = &mut model.open else {
        return;
    };
    match (open, intent) {
        (Open::Config(panel), Intent::Activate(id)) => {
            let Some(row) = panel.rows.iter_mut().find(|row| row.id == id) else {
                return;
            };
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
        (Open::Theme(picker), Intent::Preview(theme)) => {
            if picker.themes.contains(&theme) {
                model.syntax_theme = Some(theme);
            }
        }
        (Open::Theme(picker), Intent::Pick(theme)) => {
            if picker.themes.contains(&theme) {
                model.syntax_theme = Some(theme.clone());
                model.edits.push(Edit::ThemeSelected(theme));
                model.open = None;
            }
        }
        (Open::Setup(setup), Intent::Toggle(id)) => {
            if let Some(item) = setup.items.iter_mut().find(|item| item.id == id) {
                item.enabled = !item.enabled;
            }
        }
        (Open::Setup(setup), Intent::Move { id, delta }) => {
            let Some(at) = setup.position(&id) else {
                return;
            };
            let to = match at.checked_add_signed(delta.signum() as isize) {
                Some(to) if to < setup.items.len() => to,
                _ => return,
            };
            // A row that is not orderable neither moves nor is crossed.
            if setup.items[at].orderable && setup.items[to].orderable {
                setup.items.swap(at, to);
            }
        }
        (Open::Setup(setup), Intent::Save) => {
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
            model.edits.push(Edit::SetupSaved {
                kind: setup.kind,
                items,
                use_theme_colors,
            });
            model.open = None;
        }
        (Open::Keymap(picker), Intent::KeymapPick(name)) => {
            if picker
                .tabs
                .iter()
                .any(|tab| tab.items.iter().any(|item| item.name == name))
            {
                model.edits.push(Edit::KeymapPicked(name));
            }
        }
        (Open::Theme(picker), Intent::Close) => {
            model.syntax_theme = picker.original.clone();
            model.open = None;
        }
        (Open::Setup(setup), Intent::Close) => {
            model.edits.push(Edit::SetupCancelled(setup.kind));
            model.open = None;
        }
        (_, Intent::Close) => model.open = None,
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

/// One item of a setup's catalogue (`StatusLineItem` / `TerminalTitleItem`
/// with its description and preview placeholder).
#[derive(Clone, Debug, serde::Deserialize)]
struct CatalogueItem {
    id: String,
    description: String,
    placeholder: String,
}

/// The seed forms of this screen's events (`kind`): `config_panel` with its
/// `rows`; `theme_picker` with `themes` and the `current` one;
/// `status_line` with `use_theme_colors`, the `selected` ids in order, the
/// `catalogue` and the runtime `values`; `title` the same without the colour
/// preference; `keymap` with its `tabs`.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum SeedEv {
    ConfigPanel {
        rows: Vec<Row>,
    },
    ThemePicker {
        themes: Vec<String>,
        #[serde(default)]
        custom: Vec<String>,
        #[serde(default)]
        current: Option<String>,
        #[serde(default)]
        themes_dir: Option<String>,
    },
    StatusLine {
        use_theme_colors: bool,
        selected: Vec<String>,
        catalogue: Vec<CatalogueItem>,
        #[serde(default)]
        values: BTreeMap<String, String>,
    },
    Title {
        selected: Vec<String>,
        catalogue: Vec<CatalogueItem>,
        #[serde(default)]
        values: BTreeMap<String, String>,
    },
    Keymap {
        tabs: Vec<KeymapTab>,
    },
}

/// `StatusLineSetupView::new` / `TerminalTitleSetupView::new`: the selected
/// items first, in order and enabled, then the rest disabled; the status
/// line's "use theme colors" leads.
fn setup(
    kind: SetupKind,
    use_theme_colors: Option<bool>,
    selected: &[String],
    catalogue: Vec<CatalogueItem>,
    values: BTreeMap<String, String>,
) -> Setup {
    let mut items = Vec::new();
    if let Some(enabled) = use_theme_colors {
        items.push(SetupItem {
            id: USE_THEME_COLORS.to_string(),
            description: String::new(),
            enabled,
            orderable: false,
            section_break_after: true,
            placeholder: String::new(),
        });
    }
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
    Setup {
        kind,
        items,
        values,
    }
}

/// A script's `screen` step for this directory: the dialog `value` opens.
pub fn seed(value: serde_json::Value) -> Result<Ev, crate::screens::SeedError> {
    let seed: SeedEv = serde_json::from_value(value)
        .map_err(|err| crate::screens::SeedError::Bad(err.to_string()))?;
    Ok(Ev::Opened(match seed {
        SeedEv::ConfigPanel { rows } => Open::Config(ConfigPanel::new(rows)),
        SeedEv::ThemePicker {
            themes,
            custom,
            current,
            themes_dir,
        } => Open::Theme(ThemePicker {
            themes,
            custom,
            original: current,
            themes_dir,
        }),
        SeedEv::StatusLine {
            use_theme_colors,
            selected,
            catalogue,
            values,
        } => Open::Setup(setup(
            SetupKind::StatusLine,
            Some(use_theme_colors),
            &selected,
            catalogue,
            values,
        )),
        SeedEv::Title {
            selected,
            catalogue,
            values,
        } => Open::Setup(setup(
            SetupKind::Title,
            None,
            &selected,
            catalogue,
            values,
        )),
        SeedEv::Keymap { tabs } => {
            if tabs.is_empty() {
                return Err(crate::screens::SeedError::Bad("no tabs".into()));
            }
            Open::Keymap(KeymapPicker { tabs })
        }
    }))
}

