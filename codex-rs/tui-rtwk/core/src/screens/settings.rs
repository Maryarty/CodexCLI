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

use std::collections::{BTreeMap, BTreeSet};

pub mod hooks;
pub mod items;
pub mod persist;

use hooks::{Hook, HookIssue, Hooks};

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
    /// What the row shows when it is not the id (a skill's display name).
    pub label: String,
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
    /// `bottom_pane/skills_toggle_view.rs`: Enable/Disable Skills, a plain
    /// checklist with a search (no order, no preview); each toggle is saved
    /// as it is made.
    Skills,
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

impl SetupItem {
    /// The name the row shows.
    pub fn name(&self) -> &str {
        if self.label.is_empty() {
            &self.id
        } else {
            &self.label
        }
    }
}

impl Setup {
    /// The rows a search leaves, as indices into `items`: a fuzzy match on the
    /// id, best score first, ties by name (`MultiSelectPicker::apply_filter`);
    /// every row in order while the search is empty. The skills match the
    /// display name first, then the skill's own name (`match_skill`), ties by
    /// display name.
    pub fn visible(&self, query: &str) -> Vec<usize> {
        let query = query.trim();
        if query.is_empty() {
            return (0..self.items.len()).collect();
        }
        let skills = self.kind == SetupKind::Skills;
        let mut found: Vec<(usize, i32)> = self
            .items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| {
                let item_match = if skills {
                    crate::commands::fuzzy_match(item.name(), query).or_else(|| {
                        (item.name() != item.id)
                            .then(|| crate::commands::fuzzy_match(&item.id, query))
                            .flatten()
                    })
                } else {
                    crate::commands::fuzzy_match(&item.id, query)
                };
                item_match.map(|(_, score)| (index, score))
            })
            .collect();
        found.sort_by(|a, b| {
            a.1.cmp(&b.1).then_with(|| {
                if skills {
                    self.items[a.0].name().cmp(self.items[b.0].name())
                } else {
                    self.items[a.0].id.cmp(&self.items[b.0].id)
                }
            })
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
        if self.kind == SetupKind::Skills {
            return String::new();
        }
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
            SetupKind::StatusLine | SetupKind::Skills => values.collect::<Vec<_>>().join(" · "),
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
/// One experimental feature of the dialog (`ExperimentalFeatureItem`).
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct Feature {
    pub key: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub enabled: bool,
    /// Read-only features cannot be flipped (`item.writable`).
    #[serde(default = "writable")]
    pub writable: bool,
}

fn writable() -> bool {
    true
}

/// The line under the subtitle of the experimental features: the state of
/// the catalogue and of the write (`discovery_status`). The words are the
/// view's (ADR-010); only what the server said is text here.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum FeaturesStatus {
    #[default]
    Quiet,
    /// The server's catalogue was asked for.
    Loading,
    /// The write was sent.
    Saving,
    /// The catalogue came with nothing in it.
    NoExperiments,
    /// The catalogue request failed.
    Unavailable,
    /// What the write answered: a warning or an error of the server.
    Message(String),
}

/// `ExperimentalFeaturesView`: the features as the config reads them, and where discovery and
/// the write stand. What the user flipped before saving is the sheet's draft (the keys), read
/// through `enabled` and `updates`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Experimental {
    pub features: Vec<Feature>,
    /// The keys a write was sent for and did not confirm: Enter sends them
    /// again (`unconfirmed`).
    pub unconfirmed: Vec<String>,
    pub status: FeaturesStatus,
    /// The catalogue is on its way (`catalog_rx.is_some()`).
    pub loading: bool,
    /// A write is on its way (`write_rx.is_some()`).
    pub saving: bool,
    /// How many writes the server confirmed: a draft that saw fewer starts again from the
    /// features as they now are.
    pub confirmed: u32,
    /// Which open of the dialog the answers belong to; an answer of
    /// another one is stale and dropped.
    pub generation: u64,
}

impl Experimental {
    pub fn new(features: Vec<Feature>, loading: bool, generation: u64) -> Self {
        Self {
            features,
            unconfirmed: Vec::new(),
            status: if loading {
                FeaturesStatus::Loading
            } else {
                FeaturesStatus::Quiet
            },
            loading,
            saving: false,
            confirmed: 0,
            generation,
        }
    }

    /// The value of a feature with the keys the user flipped since the config was read.
    pub fn enabled(&self, feature: &Feature, flipped: &BTreeSet<String>) -> bool {
        feature.enabled != flipped.contains(&feature.key)
    }

    /// `save_changes`: what a write would send, in the dialog's order.
    pub fn updates(&self, flipped: &BTreeSet<String>) -> Vec<(String, bool)> {
        self.features
            .iter()
            .filter(|feature| {
                feature.writable
                    && (flipped.contains(&feature.key) || self.unconfirmed.contains(&feature.key))
            })
            .map(|feature| (feature.key.clone(), self.enabled(feature, flipped)))
            .collect()
    }
}

/// What a write answered (`FeatureWriteResult`): the features as the config
/// reads them again, and a warning that keeps the dialog open.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Written {
    pub features: Vec<(String, bool)>,
    pub warning: Option<String>,
}

/// `MemoriesSettingsView`: the two switches as the config has them. The reset confirmation,
/// the cursor of either list and the switches as flipped before Save are the sheet's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Memories {
    pub use_memories: bool,
    pub generate_memories: bool,
}

/// Which settings dialog a catalogue is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Which {
    Config,
    Theme,
    StatusLine,
    Title,
    Skills,
    Keymap,
    Experimental,
    Memories,
    Hooks,
}

/// What a dialog is offered: the catalogue it lists, as the server and the host gave it (a
/// fact). What is open, the cursor, the checks before Save and the order of a setup are the
/// sheet's own draft (`src/sheets/settings/draft.rs`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Offer {
    Config(ConfigPanel),
    Theme(ThemePicker),
    Setup(Setup),
    Keymap(KeymapPicker),
    Experimental(Experimental),
    Memories(Memories),
    Hooks(Hooks),
}

impl Offer {
    /// The dialog the catalogue is for.
    pub fn which(&self) -> Which {
        match self {
            Offer::Config(_) => Which::Config,
            Offer::Theme(_) => Which::Theme,
            Offer::Setup(setup) => match setup.kind {
                SetupKind::StatusLine => Which::StatusLine,
                SetupKind::Title => Which::Title,
                SetupKind::Skills => Which::Skills,
            },
            Offer::Keymap(_) => Which::Keymap,
            Offer::Experimental(_) => Which::Experimental,
            Offer::Memories(_) => Which::Memories,
            Offer::Hooks(_) => Which::Hooks,
        }
    }
}

/// What the dialogs open from, read of the config and the disk by the host
/// and sent as `Ev::Facts` (the model reads no files: ADR-003).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Facts {
    /// `[tui] theme` of the config: the syntax theme in force.
    pub syntax_theme: Option<String>,
    /// The `.tmTheme` files of `{codex_home}/themes` (file stems).
    pub custom_themes: Vec<String>,
    /// That directory as the subtitle names it (`~/.codex/themes`); none when
    /// it is not below the home directory.
    pub themes_dir: Option<String>,
    /// The memory tool is on (`Feature::MemoryTool`) with these switches
    /// (`config.memories`); none: it is off and `/memories` offers to turn it
    /// on (that prompt is not built).
    pub memories: Option<MemoryFacts>,
}

/// `config.memories`: the two switches of the memories dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct MemoryFacts {
    pub use_memories: bool,
    pub generate_memories: bool,
}

/// The bundled syntax themes in kebab-case, alphabetical
/// (`render/highlight.rs`: `BUILTIN_THEME_NAMES`).
pub const BUILTIN_THEMES: [&str; 32] = [
    "1337",
    "ansi",
    "base16",
    "base16-256",
    "base16-eighties-dark",
    "base16-mocha-dark",
    "base16-ocean-dark",
    "base16-ocean-light",
    "catppuccin-frappe",
    "catppuccin-latte",
    "catppuccin-macchiato",
    "catppuccin-mocha",
    "coldark-cold",
    "coldark-dark",
    "dark-neon",
    "dracula",
    "github",
    "gruvbox-dark",
    "gruvbox-light",
    "inspired-github",
    "monokai-extended",
    "monokai-extended-bright",
    "monokai-extended-light",
    "monokai-extended-origin",
    "nord",
    "one-half-dark",
    "one-half-light",
    "solarized-dark",
    "solarized-light",
    "sublime-snazzy",
    "two-dark",
    "zenburn",
];

/// The dialog a slash command opens, built from the facts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dialog {
    /// `/theme`.
    Theme,
    /// `/memories`.
    Memories,
    /// `/statusline`: the app builds it from the chat's status facts (`app/settings.rs`).
    StatusLine,
    /// `/title`: likewise.
    Title,
}

impl Facts {
    /// The picker `/theme` opens: the bundled names and the custom files,
    /// sorted together case-insensitively (`list_available_themes`).
    fn theme_picker(&self) -> ThemePicker {
        let mut themes: Vec<String> = BUILTIN_THEMES.iter().map(ToString::to_string).collect();
        let mut custom = Vec::new();
        for name in &self.custom_themes {
            if !themes.contains(name) {
                themes.push(name.clone());
                custom.push(name.clone());
            }
        }
        themes.sort_by_cached_key(|name| (name.to_ascii_lowercase(), name.clone()));
        ThemePicker {
            themes,
            custom,
            original: self.syntax_theme.clone(),
            themes_dir: self.themes_dir.clone(),
        }
    }
}

/// What a settings dialog asked the app to do (the oracle's `AppEvent`s): a request record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Edit {
    /// A toggle row was flipped (`PersistConfig`).
    Toggled(SettingId, bool),
    TestNotification,
    /// Enter on a row that opens an editor of its own.
    Editor(SettingId),
    /// `SyntaxThemeSelected`.
    ThemeSelected(String),
    /// A skill was switched (`AppEvent::SetSkillEnabled`): its id, its value.
    SkillToggled(String, bool),
    /// The skills toggle was closed (`AppEvent::ManageSkillsClosed`: the
    /// skills are read again).
    SkillsClosed,
    /// `StatusLineSetup` / `TerminalTitleSetup`: the enabled items in order.
    SetupSaved {
        kind: SetupKind,
        items: Vec<String>,
        use_theme_colors: bool,
    },
    SetupCancelled(SetupKind),
    /// `AppEvent::SaveExperimentalFeatures`: the keys and values to write.
    FeaturesSave(Vec<(String, bool)>),
    /// An action was picked in the keymap picker.
    KeymapPicked(String),
    /// Enter on a switch of the memories dialog (`UpdateMemorySettings`).
    MemoriesSaved {
        use_memories: bool,
        generate_memories: bool,
    },
    /// Enter on "Reset all memories" in its confirmation (`ResetMemories`).
    MemoriesReset,
    /// A hook was switched (`AppEvent::SetHookEnabled`): its key, its value.
    HookEnabled {
        key: String,
        enabled: bool,
    },
    /// A hook was trusted (`AppEvent::TrustHook`): its key and the hash
    /// vouched for.
    HookTrusted {
        key: String,
        current_hash: String,
    },
    /// Every hook waiting for a review was trusted (`AppEvent::TrustHooks`):
    /// the keys and hashes.
    HooksTrusted(Vec<(String, String)>),
}

/// The app-scoped model state of this screen, changed only by `apply`. The dialogs are the
/// sheets' (`src/sheets/settings/`): what is open, the cursor, what is checked, moved or typed
/// before it is saved are their drafts. The model keeps what the session reads or writes out
/// (ADR-013): the facts the dialogs open from, the catalogues they are offered, the highlight
/// theme in force, the requests the dialogs left.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Model {
    pub facts: Facts,
    /// What the dialogs are offered, by dialog (the server's and the host's catalogues).
    pub offers: BTreeMap<Which, Offer>,
    /// The dialog a slash command (or a seed) asked for, until the surface took it
    /// (`Intent::Taken`): a one-shot signal to the surface, not what is open.
    pub requested: Option<Which>,
    /// The syntax theme in force (the oracle: `highlight::current_syntax_theme`):
    /// the picker previews by changing it.
    pub syntax_theme: Option<String>,
    /// The requests the dialogs left, in order; the app takes each once (`take_edits`).
    pub edits: Vec<Edit>,
    /// How many of `edits` the app has taken.
    pub sent: usize,
}

impl Model {
    /// The edits recorded since the last call, in order: the app writes them
    /// (`persist::write_of`) and carries what else they change.
    pub fn take_edits(&mut self) -> Vec<Edit> {
        let fresh = self.edits.get(self.sent..).unwrap_or_default().to_vec();
        self.sent = self.edits.len();
        fresh
    }

    /// The catalogue of the experimental features, as the server gave it.
    fn experimental_mut(&mut self) -> Option<&mut Experimental> {
        match self.offers.get_mut(&Which::Experimental) {
            Some(Offer::Experimental(dialog)) => Some(dialog),
            _ => None,
        }
    }
}

/// The events of this screen (backend results, tickets); a variant of the
/// root `ScreenEv`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ev {
    /// A dialog is offered a catalogue, and asked for.
    Offered(Offer),
    /// The config's values and the disk's files the dialogs open from.
    Facts(Facts),
    /// The server's catalogue of the experimental features arrived; none:
    /// the request failed. The host leaves out what the dialog hides (the
    /// stages other than beta, the retired features).
    FeaturesDiscovered {
        generation: u64,
        features: Option<Vec<Feature>>,
    },
    /// The config file has what a dialog saved: the session follows (`persist::Subject`).
    Saved(persist::Subject),
    /// The write of a dialog was refused: the session stays as it was.
    SaveFailed(persist::Subject),
    /// The write of the experimental features was answered.
    FeaturesSaved {
        generation: u64,
        result: Result<Written, String>,
    },
}

/// The intents of this screen; a variant of the root `ScreenIntent`: verbs of the domain. The
/// cursor, the group tab, the search text, the checks and their order are the sheets' own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// A slash command asks for a dialog, built from the facts.
    Open(Dialog),
    /// The surface took the dialog that was asked for.
    Taken,
    /// The syntax theme in force (the picker's preview, and the restore on cancel: none is the
    /// config's own).
    SyntaxTheme(Option<String>),
    /// What a dialog asks the app to do.
    Request(Edit),
}

pub fn apply(model: &mut Model, ev: Ev, fx: &mut AppFx) {
    let _ = fx;
    match ev {
        Ev::Offered(offer) => {
            if let Offer::Theme(picker) = &offer {
                model.syntax_theme = picker.original.clone();
            }
            let which = offer.which();
            model.offers.insert(which, offer);
            model.requested = Some(which);
        }
        Ev::Facts(facts) => {
            // The syntax theme in force follows the config, but not while a
            // picker previews another one.
            if model.syntax_theme == model.facts.syntax_theme {
                model.syntax_theme = facts.syntax_theme.clone();
            }
            model.facts = facts;
        }
        Ev::Saved(subject) => match subject {
            persist::Subject::Theme(name) => model.facts.syntax_theme = Some(name),
            persist::Subject::Memories(memories) => model.facts.memories = Some(memories),
            // The chat's status facts hold these (the app updates them).
            persist::Subject::StatusLine { .. } | persist::Subject::Title(_) => {}
        },
        // `restore_runtime_theme_from_config`: the theme in force is the config's again.
        Ev::SaveFailed(persist::Subject::Theme(_)) => {
            model.syntax_theme = model.facts.syntax_theme.clone();
        }
        Ev::SaveFailed(_) => {}
        Ev::FeaturesDiscovered {
            generation,
            features,
        } => {
            let Some(dialog) = model.experimental_mut() else {
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
        Ev::FeaturesSaved { generation, result } => {
            let Some(dialog) = model.experimental_mut() else {
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
}

pub fn apply_intent(model: &mut Model, intent: Intent, fx: &mut AppFx) {
    let _ = fx;
    match intent {
        Intent::Open(dialog) => match dialog {
            Dialog::Theme => {
                let picker = model.facts.theme_picker();
                model.syntax_theme = picker.original.clone();
                model.offers.insert(Which::Theme, Offer::Theme(picker));
                model.requested = Some(Which::Theme);
            }
            Dialog::Memories => {
                if let Some(facts) = model.facts.memories {
                    model.offers.insert(
                        Which::Memories,
                        Offer::Memories(Memories {
                            use_memories: facts.use_memories,
                            generate_memories: facts.generate_memories,
                        }),
                    );
                    model.requested = Some(Which::Memories);
                }
            }
            // Built by the app, which holds the status facts, and offered as `Ev::Offered`.
            Dialog::StatusLine | Dialog::Title => {}
        },
        Intent::Taken => model.requested = None,
        Intent::SyntaxTheme(theme) => {
            model.syntax_theme = theme.or_else(|| model.facts.syntax_theme.clone());
        }
        Intent::Request(edit) => {
            match &edit {
                Edit::ThemeSelected(theme) => model.syntax_theme = Some(theme.clone()),
                // A write is on its way: a failed answer can follow a committed write, so
                // these keys stay dirty and putting a value back still sends a correction.
                Edit::FeaturesSave(updates) => {
                    if let Some(dialog) = model.experimental_mut() {
                        dialog.saving = true;
                        dialog.unconfirmed = updates.iter().map(|(key, _)| key.clone()).collect();
                        dialog.status = FeaturesStatus::Saving;
                    }
                }
                _ => {}
            }
            model.edits.push(edit);
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

/// The RPCs of this screen: requested with `ThreadFx::request`, which stamps
/// the thread and the ticket; the shell sends them through `wire`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// `config/batchWrite`: what a dialog saved (`persist::write_of`).
    Write {
        edits: Vec<crate::app::effects::ConfigEdit>,
    },
}

/// The replies of those RPCs, decoded by `decode`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {
    /// The write is in the file; `overridden`: a layer above the user's config overrides it.
    Written { overridden: bool },
}

/// What an outgoing RPC of this screen was for, kept with its ticket. The app owns the
/// answers: no thread asks for them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ctx {
    /// A write of the config file, and the dialog it came from.
    Write(persist::Write),
}

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
    // The replies are the app's.
    let _ = (part, result, fx, ctx);
}

/// The wire request of `command` for `thread`.
pub fn wire(command: &Command, thread: &ThreadId, id: RequestId) -> ClientRequest {
    let _ = thread;
    match command {
        Command::Write { edits } => persist::write_request(edits, id),
    }
}

/// The reply `result` (the JSON result of the request) is for `command`.
pub fn decode(command: &Command, result: serde_json::Value) -> Result<Reply, RpcError> {
    match command {
        Command::Write { .. } => persist::written(result)
            .map(|overridden| Reply::Written { overridden })
            .map_err(RpcError::Malformed),
    }
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
/// preference; `keymap` with its `tabs`; `memories` with its two switches;
/// `facts` with `memories`.
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
    Skills {
        items: Vec<SkillSeed>,
    },
    Experimental {
        features: Vec<Feature>,
        #[serde(default)]
        loading: bool,
        /// Keys whose write did not confirm.
        #[serde(default)]
        unconfirmed: Vec<String>,
        /// What the failed write answered.
        #[serde(default)]
        failed: Option<String>,
        #[serde(default)]
        generation: u64,
    },
    FeaturesDiscovered {
        #[serde(default)]
        generation: u64,
        #[serde(default)]
        features: Option<Vec<Feature>>,
    },
    Memories {
        use_memories: bool,
        generate_memories: bool,
    },
    Hooks {
        hooks: Vec<Hook>,
        #[serde(default)]
        warnings: Vec<String>,
        #[serde(default)]
        errors: Vec<HookIssue>,
    },
    /// The config's values the dialogs open from (`Ev::Facts`).
    Facts {
        #[serde(default)]
        memories: Option<MemoryFacts>,
    },
}

/// One skill of the toggle (`SkillsToggleItem`).
#[derive(Clone, Debug, serde::Deserialize)]
struct SkillSeed {
    /// The skill's own name (`skill_name`).
    id: String,
    /// Its display name.
    name: String,
    description: String,
    enabled: bool,
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
            label: String::new(),
            description: String::new(),
            enabled,
            orderable: false,
            section_break_after: true,
            placeholder: String::new(),
        });
    }
    let item = |entry: &CatalogueItem, enabled: bool| SetupItem {
        id: entry.id.clone(),
        label: String::new(),
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
    Ok(Ev::Offered(match seed {
        SeedEv::FeaturesDiscovered {
            generation,
            features,
        } => {
            return Ok(Ev::FeaturesDiscovered {
                generation,
                features,
            });
        }
        SeedEv::Facts { memories } => {
            return Ok(Ev::Facts(Facts {
                memories,
                ..Facts::default()
            }));
        }
        SeedEv::Hooks {
            hooks,
            warnings,
            errors,
        } => Offer::Hooks(Hooks::new(hooks, warnings, errors)),
        SeedEv::Memories {
            use_memories,
            generate_memories,
        } => Offer::Memories(Memories {
            use_memories,
            generate_memories,
        }),
        SeedEv::Experimental {
            features,
            loading,
            unconfirmed,
            failed,
            generation,
        } => {
            let mut dialog = Experimental::new(features, loading, generation);
            if let Some(error) = failed {
                dialog.status = FeaturesStatus::Message(error);
            }
            // A write that failed: the features show what the user flipped them to, the config
            // has the opposite (the sheet's draft starts with these keys flipped).
            if !unconfirmed.is_empty() {
                for feature in &mut dialog.features {
                    if unconfirmed.contains(&feature.key) {
                        feature.enabled = !feature.enabled;
                    }
                }
                dialog.unconfirmed = unconfirmed;
            }
            Offer::Experimental(dialog)
        }
        SeedEv::ConfigPanel { rows } => Offer::Config(ConfigPanel::new(rows)),
        SeedEv::ThemePicker {
            themes,
            custom,
            current,
            themes_dir,
        } => Offer::Theme(ThemePicker {
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
        } => Offer::Setup(setup(
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
        } => Offer::Setup(setup(SetupKind::Title, None, &selected, catalogue, values)),
        SeedEv::Skills { items } => Offer::Setup(Setup {
            kind: SetupKind::Skills,
            items: items
                .into_iter()
                .map(|skill| SetupItem {
                    id: skill.id,
                    label: skill.name,
                    description: skill.description,
                    enabled: skill.enabled,
                    orderable: false,
                    section_break_after: false,
                    placeholder: String::new(),
                })
                .collect(),
            values: BTreeMap::new(),
        }),
        SeedEv::Keymap { tabs } => {
            if tabs.is_empty() {
                return Err(crate::screens::SeedError::Bad("no tabs".into()));
            }
            Offer::Keymap(KeymapPicker { tabs })
        }
    }))
}
