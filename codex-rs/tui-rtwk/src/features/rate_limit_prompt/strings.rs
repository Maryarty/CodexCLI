//! The words of the limit prompts (oracle `chatwidget/rate_limits.rs`
//! `open_rate_limit_switch_prompt`, `open_workspace_owner_nudge_prompt`,
//! `finish_add_credits_nudge_email_request`). The wording is the oracle's English until the
//! catalog has these entries.

use crate::widgets::picker::list::{Columns, Item, List};
use codex_tui_core::features::rate_limit_prompt::{Candidate, Note};
use codex_tui_core::screens::chat::{Nudge, NudgeOutcome};

/// The rows of the switch prompt and of the nudge.
pub const SWITCH: &str = "rate_limit.switch";
pub const KEEP: &str = "rate_limit.keep";
pub const NEVER: &str = "rate_limit.never";
pub const YES: &str = "rate_limit.yes";
pub const NO: &str = "rate_limit.no";

/// `Approaching rate limits`: the lower-cost model offered, kept, or never offered again.
pub fn switch_menu(candidate: &Candidate) -> List {
    let model = &candidate.model;
    let description = if candidate.description.is_empty() {
        "Uses fewer credits for upcoming turns".to_owned()
    } else {
        candidate.description.clone()
    };
    let row = |id: &str, name: String, description: Option<String>| Item {
        id: id.to_owned(),
        name,
        description,
        ..Item::default()
    };
    List {
        title: Some("Approaching rate limits".to_owned()),
        subtitle: Some(format!("Switch to {model} for lower credit usage?")),
        standard_hint: true,
        items: vec![
            row(SWITCH, format!("Switch to {model}"), Some(description)),
            row(KEEP, "Keep current model".to_owned(), None),
            row(
                NEVER,
                "Keep current model (never show again)".to_owned(),
                Some("Hide future rate limit reminders about switching models".to_owned()),
            ),
        ],
        columns: Columns::AutoVisible,
        panel: true,
        ..List::default()
    }
}

/// The question to the workspace owner's member: `Yes (y)`, `No (default) (n)` with the cursor on
/// the second.
pub fn nudge_menu(nudge: Nudge) -> List {
    let (title, prompt) = match nudge {
        Nudge::Credits => (
            "You've reached your workspace credit limit",
            "Your workspace is out of credits. Ask your workspace owner to add more. Notify owner?",
        ),
        Nudge::UsageLimit => (
            "Usage limit reached",
            "Request a limit increase from your owner to continue using codex. Request increase?",
        ),
    };
    let row = |id: &str, name: &str, shortcut: char| Item {
        id: id.to_owned(),
        name: name.to_owned(),
        shortcut: Some(shortcut),
        ..Item::default()
    };
    List {
        title: Some(title.to_owned()),
        subtitle: Some(prompt.to_owned()),
        standard_hint: true,
        items: vec![row(YES, "Yes", 'y'), row(NO, "No (default)", 'n')],
        initial: Some(1),
        columns: Columns::AutoAllRows,
        panel: true,
        ..List::default()
    }
}

/// The words of the transcript note the e-mail's answer leaves.
pub fn note(note: &Note) -> String {
    let Note::Nudged { nudge, outcome } = note;
    match (nudge, outcome) {
        (Nudge::Credits, NudgeOutcome::Sent) => "Workspace owner notified.",
        (Nudge::Credits, NudgeOutcome::Cooldown) => "Workspace owner was already notified recently.",
        (Nudge::Credits, NudgeOutcome::Failed) => {
            "Could not notify your workspace owner. Please try again."
        }
        (Nudge::UsageLimit, NudgeOutcome::Sent) => "Limit increase requested.",
        (Nudge::UsageLimit, NudgeOutcome::Cooldown) => {
            "A limit increase was already requested recently."
        }
        (Nudge::UsageLimit, NudgeOutcome::Failed) => {
            "Could not request a limit increase. Please try again."
        }
    }
    .to_owned()
}
