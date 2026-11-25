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
    /// Drawn from the first row and the first column, with no inset and no list (only Esc and
    /// the cancel key answer): the waiting view of a user verification.
    pub flat: bool,
}

fn piece(text: &str, tone: Tone) -> Piece {
    Piece {
        text: text.to_string(),
        tone,
    }
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
        // The oracle draws the reason as one span: its line breaks have no width, the sentences
        // of a multi-line reason run together (`clipped_exec_approval_opens_the_complete_command`).
        header.push(labelled(s.reason, &reason.replace('\n', ""), Tone::Italic));
        header.push(Line::Blank);
    }
}

/// The plain rows of an approval; None for a prompt with a view of its own (the form of an
/// elicitation, the questions of a `request_user_input`).
pub fn body(prompt: &Prompt, s: &Strings) -> Option<Body> {
    let mut body = build(prompt, s)?;
    // The oracle leaves two blank rows between the header and the options (`header_gap`): the
    // last `GAP` lines of the header, which a dialog short of room gives up before it cuts the
    // header (`clip.rs`). The waiting view has no list under its header, so no gap.
    if !body.flat {
        body.header.extend(std::iter::repeat_n(Line::Blank, GAP));
    }
    Some(body)
}

/// The blank rows the oracle's list view puts between the header and the options.
pub const GAP: usize = 2;

fn build(prompt: &Prompt, s: &Strings) -> Option<Body> {
    Some(match prompt {
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
                title: Some(match (&exec.input, &exec.network_host) {
                    // `request.command.get(2)`: the terminal the input goes to.
                    (Some(_), _) => match &exec.process {
                        Some(process) => s.send_input_title(process).to_string(),
                        None => s.send_input_existing.to_string(),
                    },
                    (None, Some(host)) => s.network_title(host).to_string(),
                    (None, None) => s.run_command.to_string(),
                }),
                header,
                options,
                hint: approval_hint(s, exec.thread_label.is_some()),
                open_thread: exec.thread_label.is_some(),
                flat: false,
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
                flat: false,
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
                flat: false,
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
                flat: false,
            }
        }
        // The link to open is the dialog of `select::app_link`, as the questions are their own.
        Prompt::UserInput(_) | Prompt::AppLink(_) => return None,
        Prompt::UserVerification(verification) => {
            // The oracle's own view (`bottom_pane/user_verification.rs`): the title in bold,
            // the thread, the server and the description; no thread key, no digits-only list.
            let mut details = Vec::new();
            if let Some(thread) = &verification.thread_label {
                details.push(labelled(s.thread, thread, Tone::Bold));
                details.push(Line::Blank);
            }
            details.push(labelled(s.server, &verification.server, Tone::Bold));
            details.push(Line::Blank);
            details.push(plain(verification.description.clone()));
            let options = prompt
                .decisions()
                .iter()
                .filter_map(|decision| approval_option(decision, s, Flavour::Verification))
                .collect();
            if verification.waiting {
                let mut header = vec![
                    Line::Pieces(vec![piece(s.verify_waiting, Tone::Bold)]),
                    Line::Blank,
                ];
                header.extend(details);
                header.push(Line::Blank);
                header.push(Line::Pieces(vec![
                    piece(s.hint_press, Tone::Dim),
                    piece(s.key_esc, Tone::Bold),
                    piece(s.hint_cancel_request, Tone::Dim),
                ]));
                Body {
                    title: None,
                    header,
                    options,
                    hint: Vec::new(),
                    open_thread: false,
                    flat: true,
                }
            } else {
                Body {
                    title: Some(verification.title.clone()),
                    header: details,
                    options,
                    hint: approval_hint(s, false),
                    open_thread: false,
                    flat: false,
                }
            }
        }
    })
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
    Verification,
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
        Decision::Verify => (s.verify_accept.to_string(), Some('y')),
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
        Decision::Cancel if matches!(flavour, Flavour::Verification) => {
            (s.elicit_cancel.to_string(), Some('c'))
        }
        Decision::Cancel if matches!(flavour, Flavour::Elicitation) => {
            (s.elicit_cancel.to_string(), None)
        }
        // The default `approval.decline` is esc and `n`: the list shows the first.
        Decision::Cancel => (s.cancel.to_string(), Some('n')),
        Decision::GrantForTurn => (s.grant_turn.to_string(), Some('y')),
        Decision::GrantForTurnStrict => (s.grant_turn_strict.to_string(), Some('r')),
        Decision::GrantForSession => (s.grant_session.to_string(), Some('a')),
        Decision::DenyPermissions => (s.deny_permissions.to_string(), Some('d')),
    };
    // The user-verification list shows the cancel option's own key (`c`), not `esc`.
    let escape = matches!(decision, Decision::Cancel) && !matches!(flavour, Flavour::Verification);
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
