//! The words the review tells from core's typed data: group and item labels,
//! the lines under an item, the sources, the errors. Oracle:
//! `external_agent_config_migration/model.rs` (`*_label`, `*_detail`,
//! `display_description`, `plugin_detail_lines`).

use codex_tui_core::screens::import::{Failure, Group, GroupKind, ItemKind, Places, Sel, Source};
use rtwk::l10n::NumSyms;

use super::strings::Strings;

pub fn source(strings: &Strings, source: Source) -> &'static str {
    match source {
        Source::Claude => strings.source_claude,
        Source::Cursor => strings.source_cursor,
    }
}

pub fn group(strings: &Strings, group: &Group, num: &NumSyms) -> String {
    match group.kind {
        GroupKind::Tools => strings.tools_and_setup.to_owned(),
        GroupKind::Projects { count: 1 } => strings.current_project.to_owned(),
        GroupKind::Projects { count } => strings.projects(count, num),
        GroupKind::Chats { count } => strings.chat_sessions(count, num),
    }
}

pub fn group_description(strings: &Strings, group: &Group) -> &'static str {
    match group.kind {
        GroupKind::Tools => strings.tools_description,
        GroupKind::Projects { .. } => strings.project_description,
        GroupKind::Chats { .. } => strings.chat_description,
    }
}

/// The label of an item in the checklist.
pub fn item(strings: &Strings, kind: ItemKind) -> &'static str {
    match kind {
        ItemKind::Instructions => strings.label_instructions,
        ItemKind::Settings => strings.label_settings,
        ItemKind::Skills => strings.label_skills,
        ItemKind::Plugins => strings.label_plugins,
        ItemKind::McpServers => strings.label_mcp_servers,
        ItemKind::Agents => strings.label_agents,
        ItemKind::Hooks => strings.label_hooks,
        ItemKind::Commands => strings.label_commands,
        ItemKind::Memory => strings.label_memory,
        ItemKind::Sessions => strings.label_sessions,
    }
}

/// The label of a kind in the counts (`Settings 1, Memory 1`).
pub fn kind(strings: &Strings, kind: ItemKind) -> &'static str {
    match kind {
        ItemKind::Sessions => strings.type_sessions,
        other => item(strings, other),
    }
}

/// `Settings 1, Memory 1`: the objects of the chosen items by kind, in the
/// order the kinds first appear.
pub fn count_summary<'a>(strings: &Strings, items: impl IntoIterator<Item = &'a Sel>) -> String {
    let mut counts: Vec<(ItemKind, usize)> = Vec::new();
    for sel in items {
        match counts.iter_mut().find(|(at, _)| *at == sel.kind()) {
            Some((_, total)) => *total += sel.count(),
            None => counts.push((sel.kind(), sel.count())),
        }
    }
    counts
        .into_iter()
        .map(|(at, count)| format!("{} {count}", kind(strings, at)))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The line under an item's description (plugins have their own lines).
pub fn detail(strings: &Strings, sel: &Sel, num: &NumSyms) -> Option<String> {
    if !sel.has_details() {
        return None;
    }
    let noun = match sel.kind() {
        ItemKind::Plugins | ItemKind::Instructions | ItemKind::Settings => return None,
        ItemKind::Skills => strings.noun_skill,
        ItemKind::McpServers => strings.noun_mcp_server,
        ItemKind::Agents => strings.noun_agent,
        ItemKind::Hooks => strings.noun_hook,
        ItemKind::Commands => strings.noun_command,
        ItemKind::Memory => strings.noun_memory,
        ItemKind::Sessions => strings.noun_session,
    };
    let names: Vec<&str> = sel.names().into_iter().take(4).collect();
    Some(strings.counted(noun, sel.listed(), &names, num))
}

/// `• acme-tools: deployer, formatter, +1 more`: three marketplaces, two
/// plugins each, then how many marketplaces are hidden.
pub fn plugin_lines(strings: &Strings, sel: &Sel, num: &NumSyms) -> Vec<String> {
    let markets = sel.marketplaces();
    let mut lines: Vec<String> = markets
        .iter()
        .take(3)
        .map(|market| {
            let mut names: Vec<String> = market.plugins.iter().take(2).cloned().collect();
            let hidden = market.plugins.len().saturating_sub(names.len());
            if hidden > 0 {
                names.push(strings.more_plugins(hidden, num));
            }
            format!("• {}: {}", market.name, names.join(", "))
        })
        .collect();
    let hidden = markets.len().saturating_sub(lines.len());
    if hidden > 0 {
        lines.push(format!("• {}", strings.more_marketplaces(hidden, num)));
    }
    lines
}

/// The description of an item as the checklist shows it: the app server's
/// `Migrate ` becomes `Import `, and the paths in it are told from the
/// item's project directory.
pub fn description(strings: &Strings, sel: &Sel, places: &Places, num: &NumSyms) -> String {
    let text = sel
        .description()
        .strip_prefix(strings.migrate_prefix)
        .map_or_else(
            || sel.description().to_owned(),
            |rest| format!("{}{rest}", strings.import_prefix),
        );
    if sel.cwd().is_none() {
        return text;
    }
    let shown = |raw: &str| {
        sel.shown_path(places, raw)
            .unwrap_or_else(|| raw.to_owned())
    };
    let two = |prefix: &str, separator: &str| -> Option<String> {
        let rest = text.strip_prefix(prefix)?;
        let (left, right) = rest.split_once(separator)?;
        Some(format!(
            "{prefix}{}{separator}{}",
            shown(left),
            shown(right)
        ))
    };
    if let Some(done) = two(strings.import_prefix, strings.into) {
        return done;
    }
    if let Some(done) = two(strings.skills_from, strings.to) {
        return done;
    }
    if let Some(done) = two(strings.import_prefix, strings.to) {
        return done;
    }
    if let Some(source) = text.strip_prefix(strings.plugins_from) {
        let head = format!("{}{}", strings.plugins_from, shown(source));
        if sel.has_details() {
            let plugins = sel
                .marketplaces()
                .iter()
                .map(|market| market.plugins.len())
                .sum();
            let summary = strings.plugin_summary(sel.marketplaces().len(), plugins, num);
            return format!("{head} {summary}");
        }
        return head;
    }
    text
}

/// What a flow that ended with an error says.
pub fn failure(strings: &Strings, failure: &Failure, num: &NumSyms) -> String {
    match failure {
        Failure::InProgress => strings.in_progress.to_owned(),
        Failure::SourceGone => strings.source_gone.to_owned(),
        Failure::Detect(errors) => {
            let told: Vec<String> = errors
                .iter()
                .map(|(from, error)| strings.source_error(source(strings, *from), error, num))
                .collect();
            strings.detect_failed(&told.join("; "), num)
        }
    }
}
