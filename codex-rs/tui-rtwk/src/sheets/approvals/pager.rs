//! The full-screen read of a request (`ctrl+a`, the action `approval.open_fullscreen`): the oracle
//! opens its static pager over the request's own text, so a header the dialog clips can be read to
//! its end (`FullScreenApprovalRequest`, `FullScreenUserVerificationRequest`). The document is made
//! from the prompt the model holds; the pager is the transcript's (`StaticPager`), shown over the
//! dialog, and `q` returns to it.
//!
//! The patch request has no document: its changes are the history cell's (`cells/patch.rs`), the
//! prompt keeps only the paths, and the oracle pages the diff summary of the changes.

pub mod strings;

use codex_tui_core::screens::approvals::{Prompt, UserVerificationPrompt};
use codex_tui_core::screens::transcript::PagerDoc;

use super::present::{self, Line, Piece, Tone};
use super::strings::Strings;

/// The document of the pager over `prompt`: its title and the request's text, as the oracle pages
/// it; none for a request with nothing to page.
pub fn doc(prompt: &Prompt, s: &Strings, words: &strings::Strings) -> Option<PagerDoc> {
    let (title, mut lines) = match prompt {
        // the command alone
        Prompt::Exec(exec) => (words.exec, vec![Line::Command(exec.command.clone())]),
        Prompt::Permissions(_) => (words.permissions, present::body(prompt, s)?.header),
        Prompt::Elicitation(elicitation) if elicitation.form.is_none() => {
            (words.elicitation, present::body(prompt, s)?.header)
        }
        // in either state: the title, then the thread, the server and the description
        Prompt::UserVerification(verification) => {
            (words.verification, verification_lines(verification, s))
        }
        Prompt::Patch(_) | Prompt::Elicitation(_) | Prompt::UserInput(_) | Prompt::AppLink(_) => {
            return None;
        }
    };
    // the blank rows the dialog keeps above its list end no document
    while lines.last() == Some(&Line::Blank) {
        lines.pop();
    }
    Some(PagerDoc {
        title: title.to_owned(),
        text: ansi(&lines),
        action: None,
    })
}

/// `prompt_header`: the title in bold, a blank row, then `request_details`.
fn verification_lines(verification: &UserVerificationPrompt, s: &Strings) -> Vec<Line> {
    let labelled = |label: &str, value: &str| {
        Line::Pieces(vec![
            Piece {
                text: label.to_owned(),
                tone: Tone::Plain,
            },
            Piece {
                text: value.to_owned(),
                tone: Tone::Bold,
            },
        ])
    };
    let mut lines = vec![
        Line::Pieces(vec![Piece {
            text: verification.title.clone(),
            tone: Tone::Bold,
        }]),
        Line::Blank,
    ];
    if let Some(thread) = &verification.thread_label {
        lines.push(labelled(s.thread, thread));
        lines.push(Line::Blank);
    }
    lines.push(labelled(s.server, &verification.server));
    lines.push(Line::Blank);
    lines.push(Line::Pieces(vec![Piece {
        text: verification.description.clone(),
        tone: Tone::Plain,
    }]));
    lines
}

/// The lines as the ANSI text a pager reads. What a server wrote has its control characters
/// dropped first: an escape in it would style the pager, not the text.
fn ansi(lines: &[Line]) -> String {
    let clean = |text: &str| -> String {
        text.chars()
            .filter(|c| *c == '\n' || !c.is_control())
            .collect()
    };
    lines
        .iter()
        .map(|line| match line {
            Line::Blank => String::new(),
            Line::Command(command) => clean(command),
            Line::Pieces(pieces) => pieces
                .iter()
                .map(|piece| {
                    let sgr = match piece.tone {
                        Tone::Plain => return clean(&piece.text),
                        Tone::Bold => "1",
                        Tone::Dim => "2",
                        Tone::Italic => "3",
                        Tone::Accent => "36",
                    };
                    format!("\u{1b}[{sgr}m{}\u{1b}[0m", clean(&piece.text))
                })
                .collect(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}
