#![allow(clippy::expect_used, clippy::unwrap_used)]

//! Tests of copying what the reader selects in the transcript (S10; oracle
//! `transcript_view/selection.rs` `copy_selected_text_with`, `composer_gap.rs`
//! `show_copy_feedback`): a drag selects, the release asks the host's clipboard for the text
//! (`Ask::Copy`), and its answer is the line in the gap above the composer: `Copied 6 chars to
//! host clipboard`, `Copying…` while a copy is under way, `Copy failed · try again`, gone after
//! five seconds.
//!
//! The transcript is one agent answer of numbered rows joined by hard breaks.

use codex_tui_core::app::Command;
use codex_tui_core::app::effects::{Ask, CopyDestination, CopyFormat, CopyStatus, Done};

use crate::kit::{Owner, Screen};

/// One finished turn whose answer is the rows `row 00` .. `row {n-1}` (the same as the
/// follow control's tests, which says why it is a restored completion).
fn rows(n: usize) -> String {
    let text = (0..n)
        .map(|i| format!("row {i:02}"))
        .collect::<Vec<_>>()
        .join("  \\n");
    let item = format!(
        r#"{{"type": "agentMessage", "moderation": null, "id": "msg-1", "text": "{text}", "phase": "final_answer", "memoryCitation": null, "delivery": null, "questions": null, "startedAtMs": null, "completedAtMs": null}}"#
    );
    format!(
        "{}\n{}\n",
        r#"{"step": "seed", "seed": "session", "thread": "thread-1"}"#,
        format_args!(
            r#"{{"step": "notify", "message": {{"method": "turn/completed", "params": {{"threadId": "thread-1", "turn": {{"id": "turn-1", "rootTurnId": null, "items": [{item}], "itemsView": "summary", "status": "completed", "error": null, "startedAt": null, "completedAt": null, "durationMs": null}}}}}}}}"#
        ),
    )
}

fn script() -> &'static str {
    static SCRIPT: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    SCRIPT.get_or_init(|| rows(6))
}

fn chat() -> Screen {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 16)
        .script(script())
        .open();
    s.wait_for("row 05");
    s
}

/// Where `text` starts on the screen: its column and row.
fn at(s: &Screen, text: &str) -> (u16, u16) {
    for (y, row) in s.rows().iter().enumerate() {
        if let Some(byte) = row.find(text) {
            let x = row[..byte].chars().count();
            return (
                u16::try_from(x).expect("a column"),
                u16::try_from(y).expect("a row"),
            );
        }
    }
    panic!("no `{text}` on the screen:\n{}", s.text());
}

/// The copies the app has asked of the host, oldest first.
fn copies(s: &Screen) -> Vec<(Command, String, CopyFormat, CopyDestination)> {
    s.commands()
        .into_iter()
        .filter_map(|command| match &command {
            Command::Effect {
                ask:
                    Ask::Copy {
                        text,
                        format,
                        destination,
                    },
                ..
            } => Some((
                command.clone(),
                text.clone(),
                format.clone(),
                destination.clone(),
            )),
            _ => None,
        })
        .collect()
}

/// Drags over `text`, from its first cell to its last (the cell the pointer is released in is
/// selected too).
fn select(s: &mut Screen, text: &str) {
    let (x, y) = at(s, text);
    let width = u16::try_from(text.chars().count()).expect("a width");
    s.drag(x, y, x + width - 1, y);
}

#[test]
fn a_drag_asks_the_host_to_copy_the_selected_text() {
    let mut s = chat();
    select(&mut s, "row 03");
    let copies = copies(&s);
    assert_eq!(copies.len(), 1, "{}", s.text());
    assert_eq!(copies[0].1, "row 03");
    assert_eq!(copies[0].2, CopyFormat::PlainText);
    assert_eq!(copies[0].3, CopyDestination::Clipboard);
}

#[test]
fn nothing_is_shown_before_the_host_answers() {
    let mut s = chat();
    select(&mut s, "row 03");
    s.assert_absent("Copied");
    s.assert_absent("Copying");
}

#[test]
fn a_confirmed_copy_says_how_much_went_to_the_host_clipboard() {
    let mut s = chat();
    select(&mut s, "row 03");
    let (command, ..) = copies(&s).remove(0);
    s.answer_effect(&command, Done::Copy(Ok(CopyStatus::Confirmed)));
    s.assert_contains("Copied 6 chars to host clipboard");
    // right aligned, one cell from the edge, in the row above the composer
    let (x, y) = at(&s, "Copied 6 chars to host clipboard");
    assert_eq!(x + 32 + 1, 80, "{}", s.text());
    let prompt = s
        .rows()
        .iter()
        .position(|row| row.contains("› "))
        .expect("the composer");
    assert_eq!(usize::from(y) + 2, prompt, "{}", s.text());
}

#[test]
fn the_feedback_is_gone_after_five_seconds() {
    let mut s = chat();
    select(&mut s, "row 03");
    let (command, ..) = copies(&s).remove(0);
    s.answer_effect(&command, Done::Copy(Ok(CopyStatus::Confirmed)));
    s.wait(4_000);
    s.assert_contains("Copied 6 chars");
    s.wait(1_500);
    s.assert_absent("Copied");
}

#[test]
fn a_copy_under_way_stays_until_it_ends() {
    let mut s = chat();
    select(&mut s, "row 03");
    let (command, ..) = copies(&s).remove(0);
    s.answer_effect(&command, Done::Copy(Ok(CopyStatus::Pending(1))));
    s.assert_contains("Copying 6 chars…");
    s.wait(9_000);
    s.assert_contains("Copying 6 chars…");
    s.answer_effect(&command, Done::Copy(Ok(CopyStatus::Confirmed)));
    s.assert_contains("Copied 6 chars to host clipboard");
    s.wait(5_500);
    s.assert_absent("Copied");
}

#[test]
fn a_failed_copy_says_to_try_again() {
    let mut s = chat();
    select(&mut s, "row 03");
    let (command, ..) = copies(&s).remove(0);
    s.answer_effect(&command, Done::Copy(Err("no clipboard".into())));
    s.assert_contains("Copy failed · try again");
}

#[test]
fn a_narrow_window_gets_the_shorter_spelling() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(24, 16)
        .script(script())
        .open();
    s.wait_for("row 05");
    select(&mut s, "row 03");
    let (command, ..) = copies(&s).remove(0);
    s.answer_effect(&command, Done::Copy(Ok(CopyStatus::Confirmed)));
    s.assert_contains("Copied 6 chars");
    s.assert_absent("host clipboard");
}

#[test]
fn a_busy_answer_does_not_replace_the_copy_under_way() {
    let mut s = chat();
    select(&mut s, "row 03");
    let (first, ..) = copies(&s).remove(0);
    s.answer_effect(&first, Done::Copy(Ok(CopyStatus::Pending(1))));
    select(&mut s, "row 04");
    let (second, ..) = copies(&s).remove(1);
    s.answer_effect(&second, Done::Copy(Ok(CopyStatus::Busy)));
    s.assert_contains("Copying 6 chars…");
    s.assert_absent("already in progress");
}
