//! The words of `/goal` (oracle `goal_display.rs`, `goal_menu.rs`, `thread_goal_actions.rs`):
//! its transcript notes and the rows of its two confirmations. The wording is the oracle's
//! English until the catalog has these entries.

use crate::widgets::picker::list::{Columns, Item, List};
use codex_tui_core::features::goal::{Action, CommandsHint, Failure, Note, UsageHint};
use codex_tui_core::format::{format_goal_elapsed_seconds, format_tokens_compact};
use codex_tui_core::threads::{Goal, GoalStatus};
use rtwk_core::unicode::grapheme_boundaries;

use super::Question;

const USAGE: &str = "Usage: /goal [<objective>|clear|edit|pause|resume]";

/// The most graphemes of the new objective the `Replace goal?` subtitle shows.
const SUBTITLE_GRAPHEMES: usize = 200;

fn label(status: GoalStatus) -> &'static str {
    match status {
        GoalStatus::Active => "active",
        GoalStatus::Paused => "paused",
        GoalStatus::Blocked => "stalled",
        GoalStatus::UsageLimited => "usage limited",
        GoalStatus::BudgetLimited => "limited by budget",
        GoalStatus::Complete => "complete",
    }
}

fn usage_summary(goal: &Goal) -> String {
    let mut parts = vec![format!("Objective: {}", goal.objective)];
    if goal.time_used_seconds > 0 {
        parts.push(format!(
            "Time: {}.",
            format_goal_elapsed_seconds(goal.time_used_seconds)
        ));
    }
    if let Some(budget) = goal.token_budget {
        parts.push(format!(
            "Tokens: {}/{}.",
            format_tokens_compact(goal.tokens_used),
            format_tokens_compact(budget)
        ));
    }
    parts.join(" ")
}

/// One row of the bare `/goal` summary (oracle `goal_summary_lines`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Row {
    /// `Goal`, bold.
    Heading(&'static str),
    /// `Status: ` (dim) and its value.
    Field(&'static str, String),
    Blank,
    /// The commands under the summary, dim.
    Hint(&'static str),
}

/// The rows of the summary of `goal`.
pub fn summary(goal: &Goal) -> Vec<Row> {
    let mut rows = vec![
        Row::Heading("Goal"),
        Row::Field("Status: ", label(goal.status).to_owned()),
        Row::Field("Objective: ", goal.objective.clone()),
        Row::Field(
            "Time used: ",
            format_goal_elapsed_seconds(goal.time_used_seconds),
        ),
        Row::Field("Tokens used: ", format_tokens_compact(goal.tokens_used)),
    ];
    if let Some(budget) = goal.token_budget {
        rows.push(Row::Field("Token budget: ", format_tokens_compact(budget)));
    }
    rows.push(Row::Blank);
    rows.push(Row::Hint(match CommandsHint::of(goal.status) {
        CommandsHint::Pause => "Commands: /goal edit, /goal pause, /goal clear",
        CommandsHint::Resume => "Commands: /goal edit, /goal resume, /goal clear",
        CommandsHint::Finished => "Commands: /goal edit, /goal clear",
    }));
    rows
}

/// What stands between a note's message and its hint on the line.
pub const HINT_SEPARATOR: &str = " ";

/// A note's message and the hint the oracle's `add_info_message` puts after it on the same line
/// (`new_info_event`): the hint is None for a note that has none.
pub fn message_and_hint(note: &Note) -> (String, Option<String>) {
    match note {
        Note::Usage(hint) => {
            let hint = match hint {
                UsageHint::Example => "Example: /goal improve benchmark coverage",
                UsageHint::NoGoal => "No goal is currently set.",
                UsageHint::StartToChange => "The session must start before you can change a goal.",
                UsageHint::StartToSet => "The session must start before you can set a goal.",
                UsageHint::CreateFirst => "Create a goal before editing it.",
            };
            (USAGE.to_owned(), Some(hint.to_owned()))
        }
        Note::Status(goal) => (
            format!("Goal {}", label(goal.status)),
            Some(usage_summary(goal)),
        ),
        Note::NothingToClear => (
            "No goal to clear".to_owned(),
            Some("This thread does not currently have a goal.".to_owned()),
        ),
        other => (note_text(other), None),
    }
}

/// The words of one of the goal's notes.
pub fn note(note: &Note) -> String {
    note_text(note)
}

fn note_text(note: &Note) -> String {
    match note {
        Note::Usage(_) | Note::Status(_) | Note::NothingToClear => {
            let (message, hint) = message_and_hint(note);
            format!("{message}{HINT_SEPARATOR}{}", hint.unwrap_or_default())
        }
        Note::Summary(goal) => summary(goal)
            .iter()
            .map(|row| match row {
                Row::Heading(text) => (*text).to_owned(),
                Row::Field(label, value) => format!("{label}{value}"),
                Row::Blank => String::new(),
                Row::Hint(text) => (*text).to_owned(),
            })
            .collect::<Vec<_>>()
            .join("\n"),
        Note::Cleared => "Goal cleared".to_owned(),
        Note::NoGoalToEdit => "No goal is currently set.".to_owned(),
        Note::Failed {
            failure: Failure::Ephemeral,
            ..
        } => "Goals need a saved session. This session is temporary.\nRun `codex` to start a saved session, or `codex resume` / `/resume` to reopen one.".to_owned(),
        Note::Failed {
            action,
            failure: Failure::Other(error),
        } => {
            let action = match action {
                Action::Read => "read",
                Action::Replace => "replace",
                Action::Set => "set",
                Action::Update => "update",
                Action::Clear => "clear",
            };
            format!("Failed to {action} thread goal: {error}")
        }
    }
}

/// `truncate_text`: at most `max` graphemes, the last three dots when it is cut.
fn cut(text: &str, max: usize) -> String {
    let Some((head, _)) = grapheme_boundaries(text).nth(max) else {
        return text.to_owned();
    };
    let keep = grapheme_boundaries(text)
        .nth(max - 3)
        .map_or(head, |(start, _)| start);
    format!("{}...", &text[..keep])
}

/// The edit prompt (oracle `show_goal_edit_prompt`): its title and the placeholder of an empty
/// field.
pub const EDIT_TITLE: &str = "Edit goal";
pub const EDIT_PLACEHOLDER: &str = "Type a goal objective and press Enter";

/// The rows of `Resume paused goal?` and `Replace goal?`, in order.
pub const RESUME: &str = "goal.resume";
pub const LEAVE: &str = "goal.leave";
pub const REPLACE: &str = "goal.replace";
pub const CANCEL: &str = "goal.cancel";

/// The list of one of the two confirmations (`show_resume_paused_goal_prompt`, the replace
/// confirmation of `thread_goal_actions.rs`), shaped as `SelectionViewParams::confirmation()`.
pub fn menu(question: Question, objective: &str) -> List {
    let (title, subtitle, rows, ids) = match question {
        Question::Resume => (
            "Resume paused goal?",
            format!("Goal: {objective}"),
            [
                ("Resume goal", "Mark it active and continue when idle"),
                ("Leave paused", "Keep it paused; use /goal resume later"),
            ],
            [RESUME, LEAVE],
        ),
        Question::Replace => (
            "Replace goal?",
            format!("New objective: {}", cut(objective, SUBTITLE_GRAPHEMES)),
            [
                (
                    "Replace current goal",
                    "Set the new objective and start it now",
                ),
                ("Cancel", "Keep the current goal"),
            ],
            [REPLACE, CANCEL],
        ),
    };
    let items = ids
        .into_iter()
        .zip(rows)
        .map(|(id, (name, description))| Item {
            id: id.to_owned(),
            name: name.to_owned(),
            description: Some(description.to_owned()),
            ..Item::default()
        })
        .collect();
    List {
        title: Some(title.to_owned()),
        subtitle: Some(subtitle),
        standard_hint: true,
        items,
        columns: Columns::AutoAllRows,
        hide_description_below: Some(24),
        panel: true,
        ..List::default()
    }
}
