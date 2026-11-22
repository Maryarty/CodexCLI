//! What the unified `@` popup lists (`mentions_v2/candidate.rs`, carried):
//! the candidates of the catalog, the rows a query leaves of them and what
//! picking a row puts in the draft. Looks (tag colours) are the view's.

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

    /// The tag column's text, unpadded.
    pub fn label(self) -> &'static str {
        match self {
            Self::Plugin => "Plugin",
            Self::Skill => "Skill",
            Self::Task => "Task",
            Self::File => "File",
            Self::Directory => "Dir",
        }
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

impl Candidate {
    pub fn to_row(&self, match_indices: Option<Vec<usize>>, score: i32) -> Row {
        Row {
            display_name: self.display_name.clone(),
            description: self.description.clone(),
            capabilities: self.capabilities.clone(),
            mention_type: self.mention_type,
            selection: self.selection.clone(),
            match_indices,
            score,
        }
    }
}
