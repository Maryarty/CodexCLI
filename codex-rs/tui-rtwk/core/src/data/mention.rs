//! What can be mentioned and what the unified `@` popup lists (`mentions_v2/candidate.rs`,
//! carried): the skills, apps and plugins of the catalog, the candidates built of it, the rows a
//! query leaves of them, what picking a row puts in the draft and the tools a message names.
//! Plain values (`core::data`); looks (tag colours) are the view's.

/// What a mention is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MentionKind {
    Skill,
    App,
    Plugin,
}

/// A skill, app or plugin that can be mentioned.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mention {
    pub kind: MentionKind,
    /// The display name.
    pub name: String,
    /// Empty: none (a plugin then says what it carries).
    pub description: String,
    /// What follows the sigil when it is picked: a skill's name, an app's
    /// slug (a plugin's config name when `path` is absent).
    pub insert: String,
    /// The binding: a skill's path, `plugin://<config name>`, `app://<id>`.
    pub path: Option<String>,
    /// The plugin a skill belongs to; the skill hides behind the plugin's
    /// own row.
    pub plugin_id: Option<String>,
    /// What a plugin carries.
    pub capabilities: Vec<Capability>,
}

impl Mention {
    pub fn new(kind: MentionKind, name: &str, description: &str, insert: &str) -> Self {
        Self {
            kind,
            name: name.to_string(),
            description: description.to_string(),
            insert: insert.to_string(),
            path: None,
            plugin_id: None,
            capabilities: Vec::new(),
        }
    }

    /// A plugin's config name (`name@marketplace`).
    pub fn config_name(&self) -> &str {
        self.path
            .as_deref()
            .and_then(|path| path.strip_prefix("plugin://"))
            .unwrap_or(&self.insert)
    }
}

/// A skill, plugin or app the message names: what the turn carries beside the text so the
/// model gets the tool itself (`UserInput::Skill` / `UserInput::Mention`). The composer
/// resolves its bound mentions against its catalog (`mentions::tool_mentions`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolMention {
    /// A skill: its name and the path of its file.
    Skill { name: String, path: String },
    /// A plugin or an app: its display name and its `plugin://` / `app://` path.
    Mention { name: String, path: String },
}

/// What a row is; its order is the order of the sections.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MentionType {
    Plugin,
    Skill,
    Task,
    File,
    Directory,
}

impl MentionType {
    pub fn is_filesystem(self) -> bool {
        matches!(self, Self::File | Self::Directory)
    }
}

/// What a plugin carries: the words that say so are the view's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Capability {
    Skills,
    McpServers(usize),
    Apps(usize),
}

/// The capabilities of a plugin as the backend lists them, the empty ones left out.
pub fn plugin_capabilities(has_skills: bool, mcp_servers: usize, apps: usize) -> Vec<Capability> {
    let mut out = Vec::new();
    if has_skills {
        out.push(Capability::Skills);
    }
    if mcp_servers > 0 {
        out.push(Capability::McpServers(mcp_servers));
    }
    if apps > 0 {
        out.push(Capability::Apps(apps));
    }
    out
}

/// A link of `scheme` and `rest` (`skill://path`, `plugin://name`, `thread://id`).
pub fn uri(scheme: &str, rest: &str) -> String {
    [scheme, "://", rest].concat()
}

/// What picking a row puts in the draft.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Selection {
    /// A path, inserted as plain text (an image path becomes an image).
    File(String),
    /// A skill, plugin or task: its text (sigil included) and the binding.
    Tool {
        insert_text: String,
        path: Option<String>,
    },
}

/// One entry of the catalog, before a query.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub display_name: String,
    pub description: Option<String>,
    /// What a plugin without a description of its own says it carries.
    pub capabilities: Vec<Capability>,
    pub search_terms: Vec<String>,
    pub mention_type: MentionType,
    pub selection: Selection,
}

/// One row of the popup: a candidate (or a file match) a query kept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub display_name: String,
    pub description: Option<String>,
    pub capabilities: Vec<Capability>,
    pub mention_type: MentionType,
    pub selection: Selection,
    /// The positions of the display name the query matched (to highlight).
    pub match_indices: Option<Vec<usize>>,
    pub score: i32,
}
