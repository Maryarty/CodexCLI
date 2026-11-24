// test root: pages
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

/// The oracle draws the selection REVERSED over the cells' own style (`rendered_selection`).
#[test]
fn the_selection_is_drawn_reversed_and_only_while_it_lasts() {
    let mut s = chat();
    let (x, y) = at(&s, "row 03");
    assert!(!s.reversed(x, y), "{}", s.text());
    select(&mut s, "row 03");
    for dx in 0..6 {
        assert!(s.reversed(x + dx, y), "column {dx}:\n{}", s.text());
    }
    assert!(!s.reversed(x + 6, y), "one past the selection");
    assert!(!s.reversed(x, y - 1), "the row above");
    s.keys("esc");
    assert!(!s.reversed(x, y), "{}", s.text());
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

/// The text of the copies asked so far, oldest first.
fn texts(s: &Screen) -> Vec<String> {
    copies(s).into_iter().map(|copy| copy.1).collect()
}

#[test]
fn a_right_click_copies_what_is_selected_and_lets_it_go_once_delivered() {
    let mut s = chat();
    select(&mut s, "row 03");
    assert_eq!(texts(&s), ["row 03"]);
    let (x, y) = at(&s, "row 01");
    s.right_click(x, y);
    assert_eq!(texts(&s), ["row 03", "row 03"], "{}", s.text());
    let (command, ..) = copies(&s).remove(1);
    s.answer_effect(&command, Done::Copy(Ok(CopyStatus::Confirmed)));
    // the selection is gone: a second right click has nothing to copy
    s.right_click(x, y);
    assert_eq!(copies(&s).len(), 2, "{}", s.text());
}

#[test]
fn a_right_click_with_nothing_selected_copies_nothing() {
    let mut s = chat();
    let (x, y) = at(&s, "row 01");
    s.right_click(x, y);
    assert_eq!(copies(&s).len(), 0);
}

#[test]
fn a_copy_key_copies_the_selection_in_place_of_what_it_does_otherwise() {
    for key in ["ctrl+c", "super+c", "ctrl+shift+c"] {
        let mut s = chat();
        select(&mut s, "row 03");
        s.keys(key);
        assert_eq!(texts(&s), ["row 03", "row 03"], "{key}: {}", s.text());
        // asked as an explicit copy: delivered, the selection goes
        let (command, ..) = copies(&s).remove(1);
        s.answer_effect(&command, Done::Copy(Ok(CopyStatus::Confirmed)));
        s.keys("super+c");
        assert_eq!(copies(&s).len(), 2, "{key}: {}", s.text());
    }
}

#[test]
fn enter_on_a_selection_copies_it() {
    let mut s = chat();
    select(&mut s, "row 04");
    s.keys("enter");
    assert_eq!(texts(&s), ["row 04", "row 04"], "{}", s.text());
}

#[test]
fn enter_with_nothing_selected_copies_nothing() {
    let mut s = chat();
    s.keys("enter");
    assert_eq!(copies(&s).len(), 0);
}

#[test]
fn escape_lets_the_selection_go_before_anything_else() {
    let mut s = chat();
    select(&mut s, "row 03");
    s.keys("esc");
    let (x, y) = at(&s, "row 01");
    s.right_click(x, y);
    assert_eq!(copies(&s).len(), 1, "{}", s.text());
}

#[test]
fn the_footer_hints_at_the_keys_of_a_selection_while_there_is_one() {
    let mut s = chat();
    s.assert_absent("copy & follow");
    select(&mut s, "row 03");
    s.assert_contains("⌃c copy · enter copy & follow · esc clear");
    s.keys("esc");
    s.assert_absent("copy & follow");
}

#[test]
fn a_narrow_footer_takes_the_shorter_hint() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(24, 16)
        .script(script())
        .open();
    s.wait_for("row 05");
    select(&mut s, "row 03");
    s.assert_absent("copy & follow");
    s.assert_contains("enter copy+↓ · esc");
}

/// A transcript longer than the window, so the reader can be away from its end.
fn long_chat() -> Screen {
    let long = rows(40);
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 16)
        .script(Box::leak(long.into_boxed_str()))
        .open();
    s.wait_for("row 39");
    s
}

#[test]
fn enter_copies_and_goes_back_to_the_end_once_it_is_delivered() {
    let mut s = long_chat();
    select(&mut s, "row 38");
    s.keys("pageup");
    s.assert_absent("row 39");
    s.keys("enter");
    let (command, ..) = copies(&s).remove(1);
    // not before the host has delivered it
    s.assert_absent("row 39");
    s.answer_effect(&command, Done::Copy(Ok(CopyStatus::Confirmed)));
    s.assert_contains("row 39");
}
