//! What the model knows of the items the app server detected, as typed data:
//! the kind of an item, the objects it stands for, the names in it, the groups
//! of the summary. No word of the screen is here: the labels, the nouns and
//! the lines made of them are the view's (`src/pages/startup/import`).
//! Oracle: `external_agent_config_migration/model.rs`.

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use codex_app_server_protocol::ExternalAgentConfigMigrationItem as Item;
use codex_app_server_protocol::ExternalAgentConfigMigrationItemType as Proto;

use super::Sel;

/// What an item migrates (the protocol's item type, as the screen's own).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Instructions,
    Settings,
    Skills,
    Plugins,
    McpServers,
    Agents,
    Hooks,
    Commands,
    Memory,
    Sessions,
}

impl Kind {
    pub(super) fn of(proto: Proto) -> Kind {
        match proto {
            Proto::AgentsMd => Kind::Instructions,
            Proto::Config => Kind::Settings,
            Proto::Skills => Kind::Skills,
            Proto::Plugins => Kind::Plugins,
            Proto::McpServerConfig => Kind::McpServers,
            Proto::Subagents => Kind::Agents,
            Proto::Hooks => Kind::Hooks,
            Proto::Commands => Kind::Commands,
            Proto::Memory => Kind::Memory,
            Proto::Sessions => Kind::Sessions,
        }
    }
}

/// How a group of the summary is told.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupKind {
    /// Home settings: items with no project directory.
    Tools,
    /// Items of project directories (`count` of them; one is the current project).
    Projects { count: usize },
    /// Chat sessions (`count` of them over all items).
    Chats { count: usize },
}

/// A group of the summary: what it is and the items (by index) it holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Group {
    pub kind: GroupKind,
    pub items: Vec<usize>,
}

/// The plugins of one marketplace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Marketplace<'a> {
    pub name: &'a str,
    pub plugins: &'a [String],
}

impl Sel {
    pub fn kind(&self) -> Kind {
        Kind::of(self.item.item_type)
    }

    /// The description the app server gave.
    pub fn description(&self) -> &str {
        &self.item.description
    }

    /// The project directory of the item (`None`: home).
    pub fn cwd(&self) -> Option<String> {
        self.item.cwd.as_deref().map(|dir| dir.display().to_string())
    }

    /// Whether the app server listed what is in the item.
    pub fn has_details(&self) -> bool {
        self.item.details.is_some()
    }

    /// `raw`, a path of the item's description, as the oracle's
    /// `display_path_for` shows it from the item's project directory (`None`:
    /// the item has none).
    pub fn shown_path(&self, places: &Places, raw: &str) -> Option<String> {
        let cwd = self.item.cwd.as_deref()?;
        Some(places.display(Path::new(raw), cwd))
    }

    /// The objects the item stands for. Types with no details are one file or
    /// directory each (memory: none).
    pub fn count(&self) -> usize {
        count(&self.item)
    }

    /// The names of the objects, in the order the app server listed them.
    pub fn names(&self) -> Vec<&str> {
        let Some(details) = self.item.details.as_ref() else {
            return Vec::new();
        };
        match self.item.item_type {
            Proto::Plugins => details
                .plugins
                .iter()
                .flat_map(|group| group.plugin_names.iter().map(String::as_str))
                .collect(),
            Proto::Skills => details.skills.iter().map(|skill| skill.name.as_str()).collect(),
            Proto::McpServerConfig => details.mcp_servers.iter().map(|s| s.name.as_str()).collect(),
            Proto::Subagents => details.subagents.iter().map(|a| a.name.as_str()).collect(),
            Proto::Hooks => details.hooks.iter().map(|h| h.name.as_str()).collect(),
            Proto::Commands => details.commands.iter().map(|c| c.name.as_str()).collect(),
            Proto::Memory => details.memory.iter().map(String::as_str).collect(),
            Proto::Sessions => details.sessions.iter().filter_map(|s| s.title.as_deref()).collect(),
            Proto::AgentsMd | Proto::Config => Vec::new(),
        }
    }

    /// How many entries the item's details list (`names` skips the sessions
    /// with no title).
    pub fn listed(&self) -> usize {
        let Some(details) = self.item.details.as_ref() else {
            return 0;
        };
        match self.item.item_type {
            Proto::Plugins => details.plugins.len(),
            Proto::Skills => details.skills.len(),
            Proto::McpServerConfig => details.mcp_servers.len(),
            Proto::Subagents => details.subagents.len(),
            Proto::Hooks => details.hooks.len(),
            Proto::Commands => details.commands.len(),
            Proto::Memory => details.memory.len(),
            Proto::Sessions => details.sessions.len(),
            Proto::AgentsMd | Proto::Config => 0,
        }
    }

    /// The marketplaces of a plugins item, in order.
    pub fn marketplaces(&self) -> Vec<Marketplace<'_>> {
        self.item.details.as_ref().map_or_else(Vec::new, |details| {
            details
                .plugins
                .iter()
                .map(|group| Marketplace {
                    name: &group.marketplace_name,
                    plugins: &group.plugin_names,
                })
                .collect()
        })
    }
}

fn count(item: &Item) -> usize {
    let details = item.details.as_ref();
    match item.item_type {
        Proto::Plugins => {
            details.map_or(1, |d| d.plugins.iter().map(|g| g.plugin_names.len()).sum())
        }
        Proto::McpServerConfig => details.map_or(1, |d| d.mcp_servers.len()),
        Proto::Subagents => details.map_or(1, |d| d.subagents.len()),
        Proto::Hooks => details.map_or(1, |d| d.hooks.len()),
        Proto::Commands => details.map_or(1, |d| d.commands.len()),
        Proto::Memory => details.map_or(0, |d| d.memory.len()),
        Proto::Sessions => details.map_or(1, |d| d.sessions.len()),
        Proto::Skills => details.map_or(1, |d| d.skills.len()),
        Proto::AgentsMd | Proto::Config => 1,
    }
}

/// The groups of the summary of `items`.
pub fn groups(items: &[Item]) -> Vec<Group> {
    let indices = |keep: &dyn Fn(&Item) -> bool| -> Vec<usize> {
        items
            .iter()
            .enumerate()
            .filter_map(|(at, item)| keep(item).then_some(at))
            .collect()
    };
    let tools = indices(&|item| item.cwd.is_none() && item.item_type != Proto::Sessions);
    let projects = indices(&|item| item.cwd.is_some() && item.item_type != Proto::Sessions);
    let chats = indices(&|item| item.item_type == Proto::Sessions);

    let mut groups = Vec::new();
    if !tools.is_empty() {
        groups.push(Group { kind: GroupKind::Tools, items: tools });
    }
    if !projects.is_empty() {
        let count = projects
            .iter()
            .filter_map(|at| items[*at].cwd.as_deref())
            .collect::<BTreeSet<_>>()
            .len();
        groups.push(Group { kind: GroupKind::Projects { count }, items: projects });
    }
    if !chats.is_empty() {
        let count = chats
            .iter()
            .filter_map(|at| items[*at].details.as_ref())
            .map(|details| details.sessions.len())
            .sum::<usize>();
        groups.push(Group { kind: GroupKind::Chats { count }, items: chats });
    }
    groups
}

/// Where paths are told from: the home directory and the repository roots of
/// the project directories, read by the shell (the model does no IO).
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(default)]
pub struct Places {
    pub home: Option<String>,
    /// `(project directory, root of its repository)`.
    pub repos: Vec<(String, String)>,
}

impl Places {
    /// A path as the oracle's `display_path_for` shows it: relative to the
    /// project directory, or to it through its repository, or from `~`.
    pub fn display(&self, path: &Path, cwd: &Path) -> String {
        if path.is_relative() {
            return path.display().to_string();
        }
        if let Ok(rest) = path.strip_prefix(cwd) {
            return rest.display().to_string();
        }
        let repo = self
            .repos
            .iter()
            .find(|(dir, _)| Path::new(dir) == cwd)
            .map(|(_, root)| PathBuf::from(root));
        if repo.is_some_and(|root| path.starts_with(root)) {
            return diff(path, cwd).unwrap_or_else(|| path.to_path_buf()).display().to_string();
        }
        let home = self.home.as_deref().map(Path::new);
        match home.and_then(|home| path.strip_prefix(home).ok()) {
            Some(rest) => Path::new("~").join(rest).display().to_string(),
            None => path.display().to_string(),
        }
    }
}

/// `path` relative to `base` (both absolute), with `..` where needed.
fn diff(path: &Path, base: &Path) -> Option<PathBuf> {
    let path: Vec<Component<'_>> = path.components().collect();
    let base: Vec<Component<'_>> = base.components().collect();
    let common = path.iter().zip(&base).take_while(|(a, b)| a == b).count();
    if common == 0 {
        return None;
    }
    let ups = base.len() - common;
    let mut out = PathBuf::new();
    for _ in 0..ups {
        out.push("..");
    }
    out.extend(path[common..].iter().map(|c| c.as_os_str()));
    Some(out)
}

