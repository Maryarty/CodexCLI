//! The words of `/goal` (oracle `goal_display.rs`, `goal_menu.rs`, `thread_goal_actions.rs`):
//! its transcript notes and the rows of its two confirmations. The wording is the oracle's
//! English until the catalog has these entries.

use codex_tui_core::features::goal::{
    Action, CommandsHint, Failure, Menu, Note, Prompt, UsageHint,
};
use codex_tui_core::format::{format_goal_elapsed_seconds, format_tokens_compact};
use crate::widgets::picker::list::{Columns, Item, List};
use codex_tui_core::threads::{Goal, GoalStatus};
use rtwk_core::unicode::grapheme_boundaries;

use crate::features::PromptWords;

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

/// The words of one of the goal's notes.
pub fn note(note: &Note) -> String {
    match note {
        Note::Usage(hint) => {
            let hint = match hint {
                UsageHint::Example => "Example: /goal improve benchmark coverage",
                UsageHint::NoGoal => "No goal is currently set.",
                UsageHint::StartToChange => "The session must start before you can change a goal.",
                UsageHint::StartToSet => "The session must start before you can set a goal.",
                UsageHint::CreateFirst => "Create a goal before editing it.",
            };
            format!("{USAGE}\n{hint}")
        }
        Note::Summary(goal) => {
            let mut lines = vec![
                "Goal".to_owned(),
                format!("Status: {}", label(goal.status)),
                format!("Objective: {}", goal.objective),
                format!(
                    "Time used: {}",
                    format_goal_elapsed_seconds(goal.time_used_seconds)
                ),
                format!("Tokens used: {}", format_tokens_compact(goal.tokens_used)),
            ];
            if let Some(budget) = goal.token_budget {
                lines.push(format!("Token budget: {}", format_tokens_compact(budget)));
            }
            lines.push(String::new());
            lines.push(
                match CommandsHint::of(goal.status) {
                    CommandsHint::Pause => "Commands: /goal edit, /goal pause, /goal clear",
                    CommandsHint::Resume => "Commands: /goal edit, /goal resume, /goal clear",
                    CommandsHint::Finished => "Commands: /goal edit, /goal clear",
                }
                .to_owned(),
            );
            lines.join("\n")
        }
        Note::Status(goal) => format!("Goal {}\n{}", label(goal.status), usage_summary(goal)),
        Note::Cleared => "Goal cleared".to_owned(),
        Note::NothingToClear => {
            "No goal to clear\nThis thread does not currently have a goal.".to_owned()
        }
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

/// The edit prompt (oracle `show_goal_edit_prompt`).
pub fn prompt(prompt: &Prompt) -> PromptWords {
    match prompt {
        Prompt::Edit { .. } => PromptWords {
            title: "Edit goal".into(),
            placeholder: "Type a goal objective and press Enter".into(),
        },
    }
}

/// The list of one of the two confirmations (`show_resume_paused_goal_prompt`, the replace
/// confirmation of `thread_goal_actions.rs`), shaped as `SelectionViewParams::confirmation()`.
pub fn menu(menu: &Menu) -> List {
    let (title, subtitle, rows) = match menu {
        Menu::Resume { objective, .. } => (
            "Resume paused goal?",
            format!("Goal: {objective}"),
            [
                (
                    "Resume goal",
                    "Mark it active and continue when idle",
                ),
                (
                    "Leave paused",
                    "Keep it paused; use /goal resume later",
                ),
            ],
        ),
        Menu::Replace { objective, .. } => (
            "Replace goal?",
            format!("New objective: {}", cut(objective, SUBTITLE_GRAPHEMES)),
            [
                (
                    "Replace current goal",
                    "Set the new objective and start it now",
                ),
                ("Cancel", "Keep the current goal"),
            ],
        ),
    };
    let items = menu
        .rows()
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
