//! The words of the Windows sandbox notes (oracle `platform_actions.rs`).

use codex_tui_core::features::windows_sandbox::Note;

pub fn note(note: &Note) -> String {
    match note {
        Note::Disallowed => "That Windows sandbox option is disallowed by requirements.".into(),
        Note::StillRunning => "Windows sandbox setup is still running. Restart Codex to retry.".into(),
        Note::UpdateServer => "Update the local app server to set up the Windows sandbox.".into(),
        Note::Failed => "Windows sandbox setup failed.".into(),
        Note::NotStarted => "Windows sandbox setup did not start.".into(),
        Note::SetupFailed(error) => format!("Windows sandbox setup failed: {error}"),
        Note::Mixed => "Windows sandbox setup is unavailable when local and remote executors are configured together.".into(),
    }
}

use crate::widgets::picker::list::{Columns, Item, List, Role, Span};
use codex_tui_core::features::windows_sandbox::{Choice, Page, Prompt};

const ELEVATED: &str = "windows-sandbox.elevated";
const LEGACY: &str = "windows-sandbox.legacy";
const QUIT: &str = "windows-sandbox.quit";
const LEARN_MORE: &str = "Learn more <https://developers.openai.com/codex/windows>";

/// The id of a row.
pub fn id_of(choice: Choice) -> &'static str {
    match choice {
        Choice::Elevated => ELEVATED,
        Choice::Legacy => LEGACY,
        Choice::Quit => QUIT,
    }
}

/// The choice a row's id names.
pub fn choice_of(id: &str) -> Option<Choice> {
    [Choice::Elevated, Choice::Legacy, Choice::Quit]
        .into_iter()
        .find(|choice| id_of(*choice) == id)
}

fn name(prompt: Prompt, choice: Choice) -> &'static str {
    match (prompt, choice) {
        (Prompt::Enable, Choice::Elevated) => {
            "Set up default sandbox (requires Administrator permissions)"
        }
        (Prompt::Enable, Choice::Legacy) => "Use non-admin sandbox (higher risk if prompt injected)",
        (Prompt::Fallback, Choice::Elevated) => "Try setting up admin sandbox again",
        (Prompt::Fallback, Choice::Legacy) => "Use Codex with non-admin sandbox",
        (_, Choice::Quit) => "Quit",
    }
}

fn paragraph(text: &str, role: Role) -> Vec<Span> {
    vec![Span {
        text: text.to_owned(),
        role,
    }]
}

/// The header paragraphs of a prompt.
fn header(page: &Page) -> Vec<Vec<Span>> {
    let plain = |text: &str| paragraph(text, Role::Plain);
    match (page.prompt, page.required_by_organization) {
        (Prompt::Enable, false) => vec![plain(&format!(
            "Set up the Codex agent sandbox to protect your files and control network access. {LEARN_MORE}"
        ))],
        (Prompt::Enable, true) => vec![
            plain("Your organization requires the default Codex agent sandbox to continue. Set it up to protect your files and control network access."),
            plain(LEARN_MORE),
        ],
        (Prompt::Fallback, required) => vec![
            paragraph(
                "Couldn't set up your sandbox with Administrator permissions",
                Role::Strong,
            ),
            plain(""),
            plain(if required {
                "Your organization requires the default sandbox before Codex can continue."
            } else {
                "You can still use Codex in a non-admin sandbox. It carries greater risk if prompt injected."
            }),
            plain(LEARN_MORE),
        ],
    }
}

/// One row. Its digit picks it, except the row that needs an explicit confirmation
/// (`require_explicit_confirmation`, `Choice::explicit_confirmation`): a number a fragmented
/// terminal response types must not choose the non-admin sandbox, so that row is numbered in its
/// words only and Enter on it confirms (the oracle's digit moves the cursor to it: the picker has
/// no such row, parity/popups.md Needs).
fn row(prompt: Prompt, choice: Choice, index: usize) -> Item {
    let number = index + 1;
    let item = Item {
        id: id_of(choice).to_owned(),
        name: name(prompt, choice).to_owned(),
        ..Item::default()
    };
    match (choice.explicit_confirmation(), char::from_digit(number as u32, 10)) {
        (false, Some(digit)) => Item {
            shortcut: Some(digit),
            ..item
        },
        _ => Item {
            prefix_spans: vec![Span {
                text: format!("{number}. "),
                role: Role::Plain,
            }],
            ..item
        },
    }
}

/// The list of a prompt (`SelectionViewParams::picker()` with the standard hint).
pub fn menu(page: &Page) -> List {
    List {
        header: header(page),
        standard_hint: true,
        items: page
            .choices
            .iter()
            .enumerate()
            .map(|(index, choice)| row(page.prompt, *choice, index))
            .collect(),
        numbered: false,
        columns: Columns::AutoAllRows,
        hide_description_below: Some(24),
        panel: true,
        ..List::default()
    }
}
