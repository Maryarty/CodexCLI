//! What a prompt looks like: the oracle's `build_options` / `build_header`
//! (`bottom_pane/approval_overlay.rs`), the elicitation approval form and the
//! user-input options form, as plain rows. A pure function of the prompt and
//! the strings; the widget in `dialog.rs` only draws it.

use codex_tui_core::screens::approvals::{Decision, Prompt, Rule};

use super::strings::Strings;

/// One option row: its label, the shortcut key (shown and bound) and the
/// description column some forms have.
pub struct Choice {
    pub label: String,
    pub key: Option<char>,
    /// The shortcut shown after the label: the key, or `esc`.
    pub key_label: Option<String>,
    pub description: Option<String>,
}

pub struct Body {
    /// The question the oracle prints in bold above the header; the dialog's
    /// title. None for the forms that have a progress row instead.
    pub title: Option<String>,
    pub header: Vec<String>,
    pub options: Vec<Choice>,
    pub hint: String,
    /// Whether the rows are `1. label (key)` (an approval) or `1. label  description` (a form).
    pub form: bool,
}

pub fn body(prompt: &Prompt, s: &Strings) -> Body {
    let mut body = build(prompt, s);
    if !body.form {
        // The oracle leaves two blank rows between the header and the options.
        body.header.extend([String::new(), String::new()]);
    }
    body
}

fn build(prompt: &Prompt, s: &Strings) -> Body {
    match prompt {
        Prompt::Exec(exec) => {
            let mut header = Vec::new();
            if let Some(environment) = &exec.environment {
                header.push(format!("{}{environment}", s.environment));
                header.push(String::new());
            }
            if let Some(reason) = &exec.reason {
                header.push(format!("{}{reason}", s.reason));
                header.push(String::new());
            }
            if exec.network_host.is_none() {
                header.push(format!("$ {}", exec.command));
            }
            let network = exec.network_host.is_some();
            let options = exec
                .decisions
                .iter()
                .map(|decision| approval_option(decision, s, network, false))
                .collect();
            Body {
                title: Some(match &exec.network_host {
                    Some(host) => s.network_title(host).to_string(),
                    None => s.run_command.to_string(),
                }),
                header,
                options,
                hint: s.confirm_hint.to_string(),
                form: false,
            }
        }
        Prompt::Patch(patch) => Body {
            title: Some(s.make_edits.to_string()),
            header: vec![
                format!(
                    "{}{}",
                    s.description,
                    patch
                        .reason
                        .as_deref()
                        .filter(|reason| !reason.is_empty())
                        .unwrap_or(s.default_description)
                ),
                // The app-server request carries no file changes: the
                // oracle's header prints `unavailable` for it.
                format!("{}{}", s.destination, s.unavailable),
            ],
            options: prompt
                .decisions()
                .iter()
                .map(|decision| approval_option(decision, s, false, true))
                .collect(),
            hint: s.confirm_hint.to_string(),
            form: false,
        },
        Prompt::Permissions(permissions) => {
            let mut header = Vec::new();
            if let Some(environment) = &permissions.environment {
                header.push(format!("{}{environment}", s.environment));
                header.push(String::new());
            }
            if let Some(reason) = &permissions.reason {
                header.push(format!("{}{reason}", s.reason));
                header.push(String::new());
            }
            if !permissions.rule.is_empty() {
                header.push(format!("{}{}", s.permission_rule, rule_line(&permissions.rule, s)));
            }
            Body {
                title: Some(s.grant_permissions.to_string()),
                header,
                options: prompt
                    .decisions()
                    .iter()
                    .map(|decision| approval_option(decision, s, false, false))
                    .collect(),
                hint: s.confirm_hint.to_string(),
                form: false,
            }
        }
        Prompt::Elicitation(elicitation) => Body {
            title: None,
            header: vec![s.field_progress.to_string(), elicitation.message.clone()],
            options: vec![
                Choice {
                    label: s.elicit_allow.to_string(),
                    key: None,
                    key_label: None,
                    description: Some(s.elicit_allow_description.to_string()),
                },
                Choice {
                    label: s.elicit_cancel.to_string(),
                    key: None,
                    key_label: None,
                    description: Some(s.elicit_cancel_description.to_string()),
                },
            ],
            hint: s.elicit_hint.to_string(),
            form: true,
        },
        Prompt::UserInput(input) => {
            let question = input.questions.first();
            let mut options: Vec<Choice> = question
                .map(|q| {
                    q.options
                        .iter()
                        .map(|option| Choice {
                            label: option.label.clone(),
                            key: None,
                            key_label: None,
                            description: Some(option.description.clone()),
                        })
                        .collect()
                })
                .unwrap_or_default();
            if question.is_some_and(|q| q.is_other) {
                options.push(Choice {
                    label: s.none_of_the_above.to_string(),
                    key: None,
                    key_label: None,
                    description: Some(s.none_description.to_string()),
                });
            }
            Body {
                title: None,
                header: vec![
                    s.question_progress(1, input.questions.len(), input.questions.len())
                        .to_string(),
                    question.map(|q| q.text.clone()).unwrap_or_default(),
                    String::new(),
                ],
                options,
                hint: s.user_input_hint.to_string(),
                form: true,
            }
        }
    }
}

fn rule_line(rule: &[Rule], s: &Strings) -> String {
    let paths = |paths: &[String]| {
        paths
            .iter()
            .map(|path| format!("`{path}`"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    rule.iter()
        .map(|part| match part {
            Rule::Network => s.rule_network.to_string(),
            Rule::Read(list) => format!("{} {}", s.rule_read, paths(list)),
            Rule::Write(list) => format!("{} {}", s.rule_write, paths(list)),
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// The label and shortcut of a decision (`exec_options`, `patch_options`,
/// `permissions_options`; keys `y p a d r`, `esc` for the cancel).
fn approval_option(decision: &Decision, s: &Strings, network: bool, patch: bool) -> Choice {
    let (label, key) = match decision {
        Decision::Accept if network => (s.accept_network.to_string(), Some('y')),
        Decision::Accept => (s.accept.to_string(), Some('y')),
        Decision::AcceptWithPrefix(prefix) => (s.accept_prefix(prefix).to_string(), Some('p')),
        Decision::AcceptForSession if network => (s.accept_session_network.to_string(), Some('a')),
        Decision::AcceptForSession if patch => (s.patch_session.to_string(), Some('a')),
        Decision::AcceptForSession => (s.accept_session.to_string(), Some('a')),
        Decision::AllowHost => (s.allow_host.to_string(), Some('p')),
        Decision::Decline => (s.decline.to_string(), Some('d')),
        Decision::Cancel => (s.cancel.to_string(), None),
        Decision::GrantForTurn => (s.grant_turn.to_string(), Some('y')),
        Decision::GrantForTurnStrict => (s.grant_turn_strict.to_string(), Some('r')),
        Decision::GrantForSession => (s.grant_session.to_string(), Some('a')),
        Decision::DenyPermissions => (s.deny_permissions.to_string(), Some('d')),
        Decision::Choose(_) => (String::new(), None),
    };
    let escape = matches!(decision, Decision::Cancel);
    Choice {
        label,
        key,
        key_label: if escape {
            Some(s.key_esc.to_string())
        } else {
            key.map(String::from)
        },
        description: None,
    }
}
