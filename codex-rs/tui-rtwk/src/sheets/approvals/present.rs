//! What a prompt looks like: the oracle's `build_options` / `build_header`
//! (`bottom_pane/approval_overlay.rs`), the elicitation approval form and the
//! user-input options form, as plain rows. A pure function of the prompt and
//! the strings; the widget in `dialog.rs` only draws it.

use codex_tui_core::screens::approvals::{Decision, Prompt, Rule};

use super::strings::Strings;

/// One option row: its label, the shortcut key (shown and bound) and the
/// description column some forms have.
pub struct Choice {
    /// What choosing the row answers.
    pub decision: Decision,
    pub label: String,
    pub key: Option<char>,
    /// The shortcut shown after the label: the key, or `esc`.
    pub key_label: Option<String>,
    pub description: Option<String>,
}

/// How a piece of the header is set: the oracle's `bold()`, `italic()` and
/// accent colour on the values of its labelled lines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Plain,
    Dim,
    Bold,
    Italic,
    Accent,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Piece {
    pub text: String,
    pub tone: Tone,
}

/// One line of the header: labelled pieces, a blank row, or the command
/// (`$ ` and the command, highlighted by the dialog).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Line {
    Blank,
    Pieces(Vec<Piece>),
    Command(String),
}

pub struct Body {
    /// The question the oracle prints in bold above the header; the dialog's
    /// title. None for the forms that have a progress row instead.
    pub title: Option<String>,
    pub header: Vec<Line>,
    pub options: Vec<Choice>,
    pub hint: Vec<Piece>,
    /// The key that opens the thread the request came from (`o`), when the
    /// request is from another thread.
    pub open_thread: bool,
    /// Whether the rows are `1. label (key)` (an approval) or `1. label  description` (a form).
    pub form: bool,
}

fn piece(text: &str, tone: Tone) -> Piece {
    Piece {
        text: text.to_string(),
        tone,
    }
}

fn dim(text: &str) -> Vec<Piece> {
    vec![piece(text, Tone::Dim)]
}

/// `Press enter to confirm or esc to cancel`, and ` or o to open thread` for
/// a request of another thread (the oracle's `approval_footer_hint`).
fn approval_hint(s: &Strings, open_thread: bool) -> Vec<Piece> {
    let mut hint = vec![
        piece(s.hint_press, Tone::Dim),
        piece(s.key_enter, Tone::Bold),
        piece(s.hint_confirm, Tone::Dim),
        piece(s.key_esc, Tone::Bold),
        piece(s.hint_cancel, Tone::Dim),
    ];
    if open_thread {
        hint.extend([
            piece(s.hint_or, Tone::Dim),
            piece(s.key_open, Tone::Bold),
            piece(s.hint_open, Tone::Dim),
        ]);
    }
    hint
}

fn plain(text: impl Into<String>) -> Line {
    Line::Pieces(vec![Piece {
        text: text.into(),
        tone: Tone::Plain,
    }])
}

/// `label` plain, then `value` in `tone`: the oracle's `Reason: <italic>`.
fn labelled(label: &str, value: &str, tone: Tone) -> Line {
    Line::Pieces(vec![
        Piece {
            text: label.to_string(),
            tone: Tone::Plain,
        },
        Piece {
            text: value.to_string(),
            tone,
        },
    ])
}

/// The oracle's `Thread:` / `Environment:` / `Reason:` lead of the exec,
/// permissions and elicitation headers, each followed by a blank row.
fn lead(
    header: &mut Vec<Line>,
    s: &Strings,
    thread: Option<&str>,
    environment: Option<&str>,
    reason: Option<&str>,
) {
    if let Some(thread) = thread {
        header.push(labelled(s.thread, thread, Tone::Bold));
        header.push(Line::Blank);
    }
    if let Some(environment) = environment {
        header.push(labelled(s.environment, environment, Tone::Bold));
        header.push(Line::Blank);
    }
    if let Some(reason) = reason {
        header.push(labelled(s.reason, reason, Tone::Italic));
        header.push(Line::Blank);
    }
}

pub fn body(prompt: &Prompt, s: &Strings) -> Body {
    let mut body = build(prompt, s);
    if !body.form {
        // The oracle leaves two blank rows between the header and the options.
        body.header.extend([Line::Blank, Line::Blank]);
    }
    body
}

fn build(prompt: &Prompt, s: &Strings) -> Body {
    match prompt {
        Prompt::Exec(exec) => {
            let mut header = Vec::new();
            lead(
                &mut header,
                s,
                exec.thread_label.as_deref(),
                exec.environment.as_deref(),
                exec.reason.as_deref(),
            );
            if !exec.permission_rule.is_empty() {
                header.push(labelled(
                    s.permission_rule,
                    &rule_line(&exec.permission_rule, s),
                    Tone::Accent,
                ));
                header.push(Line::Blank);
            }
            if let Some(input) = &exec.input {
                header.push(labelled(s.input, &format!("{input:?}"), Tone::Plain));
            } else if exec.network_host.is_none() {
                header.push(Line::Command(exec.command.clone()));
            }
            let network = exec.network_host.is_some();
            let permissions = !exec.permission_rule.is_empty();
            let options = exec
                .decisions
                .iter()
                .filter_map(|decision| {
                    approval_option(
                        decision,
                        s,
                        Flavour::Exec {
                            network,
                            permissions,
                        },
                    )
                })
                .collect();
            Body {
                title: Some(match &exec.network_host {
                    Some(host) => s.network_title(host).to_string(),
                    None => s.run_command.to_string(),
                }),
                header,
                options,
                hint: approval_hint(s, exec.thread_label.is_some()),
                open_thread: exec.thread_label.is_some(),
                form: false,
            }
        }
        Prompt::Patch(patch) => {
            let mut header = Vec::new();
            if let Some(thread) = &patch.thread_label {
                header.push(labelled(s.thread, thread, Tone::Bold));
                header.push(Line::Blank);
            }
            header.push(labelled(
                s.description,
                patch
                    .reason
                    .as_deref()
                    .filter(|reason| !reason.is_empty())
                    .unwrap_or(s.default_description),
                Tone::Italic,
            ));
            // The app-server request carries no file changes: the oracle's
            // header prints `unavailable` for it.
            if patch.destinations.is_empty() {
                header.push(labelled(s.destination, s.unavailable, Tone::Bold));
            }
            for destination in &patch.destinations {
                header.push(labelled(s.destination, destination, Tone::Bold));
            }
            Body {
                title: Some(s.make_edits.to_string()),
                header,
                options: prompt
                    .decisions()
                    .iter()
                    .filter_map(|decision| approval_option(decision, s, Flavour::Patch))
                    .collect(),
                hint: approval_hint(s, patch.thread_label.is_some()),
                open_thread: patch.thread_label.is_some(),
                form: false,
            }
        }
        Prompt::Permissions(permissions) => {
            let mut header = Vec::new();
            lead(
                &mut header,
                s,
                permissions.thread_label.as_deref(),
                permissions.environment.as_deref(),
                permissions.reason.as_deref(),
            );
            if !permissions.rule.is_empty() {
                header.push(labelled(
                    s.permission_rule,
                    &rule_line(&permissions.rule, s),
                    Tone::Accent,
                ));
            }
            Body {
                title: Some(s.grant_permissions.to_string()),
                header,
                options: prompt
                    .decisions()
                    .iter()
                    .filter_map(|decision| approval_option(decision, s, Flavour::Permissions))
                    .collect(),
                hint: approval_hint(s, permissions.thread_label.is_some()),
                open_thread: permissions.thread_label.is_some(),
                form: false,
            }
        }
        Prompt::Elicitation(elicitation) => {
            // Only an elicitation the form cannot take comes here: the oracle's approval
            // overlay (`ApprovalRequest::McpElicitation`).
            let mut header = Vec::new();
            if let Some(thread) = &elicitation.thread_label {
                header.push(labelled(s.thread, thread, Tone::Bold));
                header.push(Line::Blank);
            }
            header.push(labelled(s.server, &elicitation.server, Tone::Bold));
            header.push(Line::Blank);
            header.push(plain(elicitation.message.clone()));
            Body {
                title: Some(s.elicit_title(&elicitation.server).to_string()),
                header,
                options: prompt
                    .decisions()
                    .iter()
                    .filter_map(|decision| approval_option(decision, s, Flavour::Elicitation))
                    .collect(),
                hint: approval_hint(s, elicitation.thread_label.is_some()),
                open_thread: elicitation.thread_label.is_some(),
                form: false,
            }
        }
        Prompt::UserInput(input) => {
            let question = input.questions.first();
            let mut options: Vec<Choice> = question
                .map(|q| {
                    q.options
                        .iter()
                        .enumerate()
                        .map(|(index, option)| Choice {
                            decision: Decision::Choose(index),
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
                    decision: Decision::Choose(options.len()),
                    label: s.none_of_the_above.to_string(),
                    key: None,
                    key_label: None,
                    description: Some(s.none_description.to_string()),
                });
            }
            Body {
                title: None,
                header: vec![
                    plain(
                        s.question_progress(1, input.questions.len(), input.questions.len())
                            .to_string(),
                    ),
                    plain(question.map(|q| q.text.clone()).unwrap_or_default()),
                    Line::Blank,
                ],
                options,
                hint: dim(s.user_input_hint),
                open_thread: false,
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

/// Which request's list a decision is labelled for.
#[derive(Clone, Copy)]
enum Flavour {
    Exec { network: bool, permissions: bool },
    Patch,
    Permissions,
    Elicitation,
}

/// The label and shortcut of a decision (`exec_options`, `patch_options`,
/// `permissions_options`; keys `y p a d r`, `esc` for the cancel). None for
/// an option the oracle hides (a prefix over several lines).
fn approval_option(decision: &Decision, s: &Strings, flavour: Flavour) -> Option<Choice> {
    let network = matches!(flavour, Flavour::Exec { network: true, .. });
    let permissions = matches!(
        flavour,
        Flavour::Exec {
            permissions: true,
            ..
        }
    );
    let patch = matches!(flavour, Flavour::Patch);
    let (label, key) = match decision {
        Decision::Accept if matches!(flavour, Flavour::Elicitation) => {
            (s.elicit_accept.to_string(), Some('y'))
        }
        Decision::Accept if network => (s.accept_network.to_string(), Some('y')),
        Decision::Accept => (s.accept.to_string(), Some('y')),
        Decision::AcceptWithPrefix(prefix) => {
            if prefix.contains(['\n', '\r']) {
                return None;
            }
            (s.accept_prefix(prefix).to_string(), Some('p'))
        }
        Decision::AcceptForSession if network => (s.accept_session_network.to_string(), Some('a')),
        Decision::AcceptForSession if patch => (s.patch_session.to_string(), Some('a')),
        Decision::AcceptForSession if permissions => {
            (s.accept_session_permissions.to_string(), Some('a'))
        }
        Decision::AcceptForSession => (s.accept_session.to_string(), Some('a')),
        Decision::AllowHost => (s.allow_host.to_string(), Some('p')),
        Decision::NetworkRule { allow: true, .. } => (s.allow_host.to_string(), Some('p')),
        Decision::NetworkRule { allow: false, .. } => (s.block_host.to_string(), Some('d')),
        Decision::Decline if matches!(flavour, Flavour::Elicitation) => {
            (s.elicit_decline.to_string(), Some('d'))
        }
        Decision::Decline => (s.decline.to_string(), Some('d')),
        Decision::Cancel if matches!(flavour, Flavour::Elicitation) => {
            (s.elicit_cancel.to_string(), None)
        }
        Decision::Cancel => (s.cancel.to_string(), None),
        Decision::GrantForTurn => (s.grant_turn.to_string(), Some('y')),
        Decision::GrantForTurnStrict => (s.grant_turn_strict.to_string(), Some('r')),
        Decision::GrantForSession => (s.grant_session.to_string(), Some('a')),
        Decision::DenyPermissions => (s.deny_permissions.to_string(), Some('d')),
        Decision::Choose(_) => (String::new(), None),
    };
    let escape = matches!(decision, Decision::Cancel);
    Some(Choice {
        decision: decision.clone(),
        label,
        key,
        key_label: if escape {
            Some(s.key_esc.to_string())
        } else {
            key.map(String::from)
        },
        description: None,
    })
}
