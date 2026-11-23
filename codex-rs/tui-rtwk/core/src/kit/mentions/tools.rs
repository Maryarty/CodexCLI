//! The tools a submitted message names, from the mentions its draft bound.

use std::collections::HashSet;

use super::session::ComposerBinding;
use crate::data::mention::{Mention, MentionKind, ToolMention};

/// The tools a submitted message names, from the mentions its draft bound (oracle
/// `input_submission.rs`): the skills first (a binding's path is the skill's file, or
/// `skill://` and that), then the plugins (`plugin://<config name>`), then the apps
/// (`app://<id>`), each once, in the order bound, and only those the catalog lists. The
/// turn carries each beside the text (`UserInput::Skill` / `UserInput::Mention`).
pub fn tool_mentions(bindings: &[ComposerBinding], catalog: &[Mention]) -> Vec<ToolMention> {
    let mut out = Vec::new();
    let mut seen: HashSet<&str> = HashSet::new();
    for binding in bindings {
        let path = binding.path.strip_prefix("skill://").unwrap_or(&binding.path);
        if path.contains("://") {
            continue;
        }
        let skill = catalog.iter().find(|m| {
            m.kind == MentionKind::Skill
                && m.path.as_deref().map(|p| p.strip_prefix("skill://").unwrap_or(p)) == Some(path)
        });
        if let Some(skill) = skill
            && seen.insert(path)
        {
            out.push(ToolMention::Skill {
                name: skill.insert.clone(),
                path: path.to_string(),
            });
        }
    }
    for (scheme, kind) in [("plugin://", MentionKind::Plugin), ("app://", MentionKind::App)] {
        let mut taken: HashSet<&str> = HashSet::new();
        for binding in bindings {
            let Some(id) = binding.path.strip_prefix(scheme).filter(|id| !id.is_empty()) else {
                continue;
            };
            if !taken.insert(id) {
                continue;
            }
            let found = catalog.iter().find(|m| {
                m.kind == kind
                    && match kind {
                        MentionKind::Plugin => m.config_name() == id,
                        _ => m.path.as_deref() == Some(binding.path.as_str()),
                    }
            });
            if let Some(found) = found {
                out.push(ToolMention::Mention {
                    name: found.name.clone(),
                    path: binding.path.clone(),
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn bound(sigil: char, mention: &str, path: &str) -> ComposerBinding {
        ComposerBinding {
            sigil,
            mention: mention.into(),
            path: path.into(),
        }
    }

    fn tools_catalog() -> Vec<Mention> {
        let mut deploy = Mention::new(MentionKind::Skill, "Deploy it", "", "deploy");
        deploy.path = Some("/skills/deploy/SKILL.md".into());
        let mut figma = Mention::new(MentionKind::Plugin, "Figma", "", "figma");
        figma.path = Some("plugin://figma@openai".into());
        let mut slack = Mention::new(MentionKind::App, "Slack", "", "slack");
        slack.path = Some("app://slack".into());
        vec![deploy, figma, slack]
    }

    #[test]
    fn bound_mentions_become_the_tools_the_turn_carries() {
        // oracle input_submission.rs: skills first, then plugins, then apps, each once
        let bindings = [
            bound('$', "slack", "app://slack"),
            bound('$', "figma", "plugin://figma@openai"),
            bound('$', "deploy", "skill:///skills/deploy/SKILL.md"),
            bound('$', "deploy", "/skills/deploy/SKILL.md"),
            bound('$', "gone", "app://not-listed"),
            bound('@', "task", "thread://t1"),
        ];
        assert_eq!(
            tool_mentions(&bindings, &tools_catalog()),
            vec![
                ToolMention::Skill {
                    name: "deploy".into(),
                    path: "/skills/deploy/SKILL.md".into()
                },
                ToolMention::Mention {
                    name: "Figma".into(),
                    path: "plugin://figma@openai".into()
                },
                ToolMention::Mention {
                    name: "Slack".into(),
                    path: "app://slack".into()
                },
            ]
        );
    }

    #[test]
    fn no_bindings_name_no_tools() {
        assert_eq!(tool_mentions(&[], &tools_catalog()), vec![]);
    }
}
