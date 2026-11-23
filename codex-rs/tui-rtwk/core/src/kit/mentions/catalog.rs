//! What can be mentioned and the candidates of the unified popup built of it
//! (`mentions_v2/search_catalog.rs`, carried over [`Mention`] and
//! [`TaskMention`]). The backend supplies skills, plugins and apps; the task
//! search supplies tasks; scripts seed both.

use std::collections::HashSet;

use crate::data::mention::{Candidate, Mention, MentionKind, MentionType, Selection, uri};
use crate::data::task::{MAX_TASK_TITLE_CHARS, TaskMention};
use crate::kit::mentions::codec::{is_common_env_var, is_mention_name_char};

/// The unified popup's candidates: skills (bar those a listed plugin owns),
/// plugins, tasks.
pub fn build_search_catalog(mentions: &[Mention], tasks: &[TaskMention]) -> Vec<Candidate> {
    let plugin_ids: HashSet<&str> = mentions
        .iter()
        .filter(|m| m.kind == MentionKind::Plugin)
        .map(Mention::config_name)
        .collect();
    let mut candidates = Vec::new();
    candidates.extend(
        mentions
            .iter()
            .filter(|m| m.kind == MentionKind::Skill)
            .filter(|skill| {
                !skill
                    .plugin_id
                    .as_deref()
                    .is_some_and(|plugin_id| plugin_ids.contains(plugin_id))
            })
            .map(skill_candidate),
    );
    candidates.extend(
        mentions
            .iter()
            .filter(|m| m.kind == MentionKind::Plugin)
            .map(plugin_candidate),
    );
    candidates.extend(tasks.iter().map(task_candidate));
    candidates
}

fn task_candidate(task: &TaskMention) -> Candidate {
    let title = task.title.split_whitespace().collect::<Vec<_>>().join(" ");
    let title: String = title.chars().take(MAX_TASK_TITLE_CHARS).collect();
    Candidate {
        display_name: title.clone(),
        description: (!task.cwd.is_empty()).then(|| task.cwd.clone()),
        capabilities: Vec::new(),
        search_terms: vec![
            title.clone(),
            task.cwd.chars().take(MAX_TASK_TITLE_CHARS).collect(),
            task.snippet.chars().take(MAX_TASK_TITLE_CHARS).collect(),
        ],
        mention_type: MentionType::Task,
        selection: Selection::Tool {
            insert_text: format!("@{title}"),
            path: Some(uri("thread", task.thread_id.as_str())),
        },
    }
}

fn skill_candidate(skill: &Mention) -> Candidate {
    let display_name = skill.name.clone();
    let skill_name = skill.insert.clone();
    let search_terms = if display_name == skill_name {
        vec![skill_name.clone()]
    } else {
        vec![skill_name.clone(), display_name.clone()]
    };
    let description = skill.description.trim();
    Candidate {
        display_name,
        description: (!description.is_empty()).then(|| description.to_string()),
        capabilities: Vec::new(),
        search_terms,
        mention_type: MentionType::Skill,
        selection: Selection::Tool {
            insert_text: format!("${skill_name}"),
            path: skill.path.clone(),
        },
    }
}

fn plugin_candidate(plugin: &Mention) -> Candidate {
    let config_name = plugin.config_name();
    let (plugin_name, marketplace_name) = config_name.split_once('@').unwrap_or((config_name, ""));
    let mention_name = plugin_mention_name(plugin_name, &plugin.name);
    let mut search_terms = vec![plugin_name.to_string(), config_name.to_string()];
    if plugin.name != plugin_name {
        search_terms.push(plugin.name.clone());
    }
    if !marketplace_name.is_empty() {
        search_terms.push(marketplace_name.to_string());
    }
    let own_description = !plugin.description.is_empty();
    Candidate {
        display_name: plugin.name.clone(),
        description: own_description.then(|| plugin.description.clone()),
        capabilities: if own_description {
            Vec::new()
        } else {
            plugin.capabilities.clone()
        },
        search_terms,
        mention_type: MentionType::Plugin,
        selection: Selection::Tool {
            insert_text: format!("@{mention_name}"),
            path: Some(uri("plugin", config_name)),
        },
    }
}

fn plugin_mention_name(plugin_name: &str, display_name: &str) -> String {
    let plugin_segments = split_plugin_name_segments(plugin_name);
    let display_segments = split_display_name_segments(display_name);

    if plugin_segments.len() == display_segments.len()
        && plugin_segments.iter().zip(&display_segments).all(
            |((plugin_segment, _), display_segment)| {
                plugin_segment.eq_ignore_ascii_case(display_segment.as_str())
            },
        )
    {
        let mut result = String::new();
        for ((_, separator), display_segment) in plugin_segments.into_iter().zip(display_segments) {
            result.push_str(display_segment.as_str());
            if let Some(separator) = separator {
                result.push(separator);
            }
        }
        return result;
    }

    // Generated app IDs have no useful words to title-case; use a bounded mention label instead.
    if let Some(app_id) = plugin_name.strip_prefix("app-")
        && app_id.len() == 32
        && app_id
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
        && (1..=64).contains(&display_name.len())
        && display_name.bytes().all(is_mention_name_char)
        && !is_common_env_var(display_name)
    {
        return display_name.to_owned();
    }

    title_case_plugin_name(plugin_name)
}

fn split_plugin_name_segments(plugin_name: &str) -> Vec<(String, Option<char>)> {
    let mut segments = Vec::new();
    let mut current = String::new();
    for ch in plugin_name.chars() {
        if matches!(ch, '-' | '_') {
            if !current.is_empty() {
                segments.push((std::mem::take(&mut current), Some(ch)));
            }
        } else {
            current.push(ch);
        }
    }
    if !current.is_empty() {
        segments.push((current, None));
    }
    segments
}

fn split_display_name_segments(display_name: &str) -> Vec<String> {
    display_name
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|segment| !segment.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn title_case_plugin_name(plugin_name: &str) -> String {
    let mut result = String::with_capacity(plugin_name.len());
    let mut capitalize_next = true;
    for ch in plugin_name.chars() {
        if matches!(ch, '-' | '_') {
            capitalize_next = true;
            result.push(ch);
            continue;
        }
        if capitalize_next && ch.is_ascii_alphabetic() {
            result.push(ch.to_ascii_uppercase());
        } else {
            result.push(ch);
        }
        capitalize_next = false;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::mention::{Capability, plugin_capabilities};
    use crate::ids::ThreadId;
    use pretty_assertions::assert_eq;

    #[test]
    fn plugin_mention_name_uses_display_segments_when_they_match_plugin_name() {
        assert_eq!(
            plugin_mention_name("mcp-search", "MCP Search"),
            "MCP-Search"
        );
        assert_eq!(
            plugin_mention_name("google_calendar", "Google Calendar"),
            "Google_Calendar"
        );
        assert_eq!(
            plugin_mention_name(
                "app-6a6b12e06c5c8191ac5d5252fa5f92c8",
                "app_6a6b12e06c5c8191ac5d5252fa5f92c8"
            ),
            "app-6a6b12e06c5c8191ac5d5252fa5f92c8"
        );
    }

    #[test]
    fn generated_app_mentions_use_bounded_display_names() {
        let plugin_name = "app-6a6b12e06c5c8191ac5d5252fa5f92c8";
        for display_name in ["Postman", "foo-bar_baz", &"a".repeat(64)] {
            assert_eq!(plugin_mention_name(plugin_name, display_name), display_name);
        }
        for display_name in [
            "",
            "Foo Bar",
            "Café",
            "PATH",
            "XDG_CONFIG_HOME",
            &"a".repeat(65),
        ] {
            assert_eq!(
                plugin_mention_name(plugin_name, display_name),
                "App-6a6b12e06c5c8191ac5d5252fa5f92c8"
            );
        }
    }

    #[test]
    fn plugin_mention_name_falls_back_to_title_cased_plugin_name() {
        assert_eq!(plugin_mention_name("sample", "Sample Plugin"), "Sample");
        assert_eq!(
            plugin_mention_name("browser-use", "Browser Use"),
            "Browser-Use"
        );
        for (plugin_name, expected) in [
            ("1vault", "1vault"),
            ("app-short", "App-Short"),
            (
                "app-6a6b12e06c5c8191ac5d5252fa5f92cg",
                "App-6a6b12e06c5c8191ac5d5252fa5f92cg",
            ),
            (
                "app-6A6B12E06C5C8191AC5D5252FA5F92C8",
                "App-6A6B12E06C5C8191AC5D5252FA5F92C8",
            ),
        ] {
            assert_eq!(plugin_mention_name(plugin_name, "Vault"), expected);
        }
    }

    fn plugin(display: &str, config: &str) -> Mention {
        let mut m = Mention::new(MentionKind::Plugin, display, "", config);
        m.path = Some(uri("plugin", config));
        m
    }

    #[test]
    fn the_catalog_hides_a_skill_behind_its_plugin_and_lists_tasks() {
        let mut owned = Mention::new(MentionKind::Skill, "Owned", "d", "owned");
        owned.plugin_id = Some("sample@debug".to_string());
        let free = Mention::new(MentionKind::Skill, "Free", "", "free");
        let task = TaskMention {
            thread_id: ThreadId::new("t1"),
            title: "fix   the\nbug".to_string(),
            cwd: "/repo".to_string(),
            snippet: String::new(),
        };
        let catalog = build_search_catalog(
            &[owned, free, plugin("Sample Plugin", "sample@debug")],
            &[task],
        );
        let names: Vec<_> = catalog.iter().map(|c| c.display_name.as_str()).collect();
        assert_eq!(names, ["Free", "Sample Plugin", "fix the bug"]);
        assert_eq!(catalog[0].description, None);
        assert_eq!(catalog[1].description, None);
        assert_eq!(
            catalog[1].selection,
            Selection::Tool {
                insert_text: "@Sample".to_string(),
                path: Some("plugin://sample@debug".to_string())
            }
        );
        assert_eq!(
            catalog[2].selection,
            Selection::Tool {
                insert_text: "@fix the bug".to_string(),
                path: Some("thread://t1".to_string())
            }
        );
    }

    #[test]
    fn a_plugin_without_a_description_carries_its_capabilities() {
        let mut p = plugin("Sample", "sample@debug");
        p.capabilities = plugin_capabilities(true, 2, 1);
        let catalog = build_search_catalog(&[p], &[]);
        assert_eq!(catalog[0].description, None);
        assert_eq!(
            catalog[0].capabilities,
            [
                Capability::Skills,
                Capability::McpServers(2),
                Capability::Apps(1)
            ]
        );
    }
}
