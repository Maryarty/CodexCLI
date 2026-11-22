//! Tests of the transcript's reader and its follow control (S10; oracle
//! `transcript_view/follow_control.rs`, `composer_gap.rs`, `footer.rs`,
//! `input.rs`, `app/owned_transcript.rs`): the row above the composer, the
//! "back to bottom" control that appears there when the reader is away from
//! the end and the new output it has not seen, the page keys, Esc and Enter
//! that return to the latest, and the footer's hints for them.
//!
//! The transcript is one agent answer of numbered rows joined by hard breaks:
//! the oracle's plain rows (a soft break renders as a blank, G63).

use crate::kit::{Owner, Screen};

/// One finished turn whose answer is the rows `row 00` .. `row {n-1}`.
fn rows(n: usize) -> String {
    let text = (0..n)
        .map(|i| format!("row {i:02}"))
        .collect::<Vec<_>>()
        .join("  \\n");
    let item = format!(
        r#"{{"type": "agentMessage", "moderation": null, "id": "msg-1", "text": "{text}", "phase": "final_answer", "memoryCitation": null, "delivery": null, "questions": null, "startedAtMs": null, "completedAtMs": null}}"#
    );
    let turn = |status: &str, items: &str, view: &str| {
        format!(
            r#"{{"id": "turn-1", "rootTurnId": null, "items": [{items}], "itemsView": "{view}", "status": "{status}", "error": null, "startedAt": null, "completedAt": null, "durationMs": null}}"#
        )
    };
    format!(
        "{}\n{}\n{}\n",
        r#"{"step": "seed", "seed": "session", "thread": "thread-1"}"#,
        format_args!(
            r#"{{"step": "notify", "message": {{"method": "turn/started", "params": {{"threadId": "thread-1", "turn": {}}}}}}}"#,
            turn("inProgress", "", "full")
        ),
        format_args!(
            r#"{{"step": "notify", "message": {{"method": "turn/completed", "params": {{"threadId": "thread-1", "turn": {}}}}}}}"#,
            turn("completed", &item, "summary")
        ),
    )
}

/// The oracle's `new_output` fixture: a later turn answering "new output".
const NEW_OUTPUT: &str = include_str!("../oracle/fixtures/transcript_view_1/new_output.jsonl");

fn chat(width: u16, height: u16) -> Screen {
    // the kit's script is `&'static`: built once for the binary
    static SCRIPT: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    let script = SCRIPT.get_or_init(|| rows(40));
    let mut s = Screen::build(Owner::ChatLive)
        .size(width, height)
        .script(script)
        .open();
    s.wait_for("row 39");
    s
}

/// The row of the first occurrence of `text`.
fn row_of(s: &Screen, text: &str) -> Option<usize> {
    s.rows().iter().position(|row| row.contains(text))
}

#[test]
fn the_control_is_hidden_while_the_last_row_is_visible() {
    let s = chat(80, 12);
    s.assert_absent("Back to bottom");
    s.assert_absent("↓");
    s.assert_absent("latest");
}

#[test]
fn a_scrolled_reader_shows_the_control_in_the_row_above_the_composer() {
    let mut s = chat(80, 12);
    s.keys("pageup");
    s.assert_contains("↓ Back to bottom · esc");
    let control = row_of(&s, "↓ Back to bottom").expect("the control");
    let prompt = row_of(&s, "› ").expect("the composer");
    // the gap row, then the composer's own blank row, then the prompt
    assert_eq!(control + 2, prompt, "{}", s.text());
    // centred: ` ↓ Back to bottom · esc ` is 24 cells wide, in 80
    s.assert_text_at(29, control, "↓ Back to bottom · esc");
}

#[test]
fn the_row_above_the_composer_is_reserved_at_the_tail() {
    let s = chat(80, 12);
    let last = row_of(&s, "row 39").expect("the last row");
    let prompt = row_of(&s, "› ").expect("the composer");
    // the gap row and the composer's blank row: two rows between them
    assert_eq!(last + 3, prompt, "{}", s.text());
}

#[test]
fn the_label_follows_the_width() {
    for (width, label) in [
        (80, "↓ Back to bottom · esc"),
        (20, "↓ Back to bottom"),
        (17, "↓ Bottom · esc"),
        (12, "↓ Bottom"),
        (9, "↓"),
    ] {
        let mut s = chat(width, 12);
        s.keys("pageup");
        let row = row_of(&s, "↓").unwrap_or_else(|| panic!("no control at {width}:\n{}", s.text()));
        assert_eq!(s.rows()[row].trim(), label, "at {width}:\n{}", s.text());
    }
}

#[test]
fn new_activity_widens_the_label() {
    for (width, label) in [
        (80, "New activity · ↓ Back to bottom · esc"),
        (30, "New activity · ↓ Bottom"),
        (17, "New · ↓ Bottom"),
        (12, "↓ Bottom"),
        (9, "↓"),
    ] {
        let mut s = chat(width, 12);
        s.keys("pageup");
        s.feed(NEW_OUTPUT);
        let row = row_of(&s, "↓").unwrap_or_else(|| panic!("no control at {width}:\n{}", s.text()));
        assert_eq!(s.rows()[row].trim(), label, "at {width}:\n{}", s.text());
    }
}

#[test]
fn a_click_on_the_control_returns_to_the_latest_and_keeps_the_draft() {
    let mut s = chat(80, 12);
    s.type_text("draft stays");
    s.keys("pageup");
    s.assert_contains("↓ Back to bottom");
    s.click_text("↓");
    s.assert_absent("Back to bottom");
    s.assert_contains("row 39");
    s.assert_composer("draft stays");
}

#[test]
fn a_click_where_the_control_is_hidden_reaches_nothing() {
    let mut s = chat(80, 12);
    let gap = row_of(&s, "row 39").expect("the last row") + 1;
    s.click(40, u16::try_from(gap).expect("row"));
    s.assert_contains("row 39");
    s.assert_absent("Back to bottom");
}

#[test]
fn page_up_and_page_down_scroll_by_a_page_less_one_row() {
    let mut s = chat(80, 12);
    let at_end = s.shot();
    s.keys("pageup");
    s.assert_rows_differ(&at_end);
    // the transcript above the gap and the composer is 8 rows: a page is 7
    assert!(row_of(&s, "row 39").is_none(), "{}", s.text());
    assert!(row_of(&s, "row 32").is_some(), "{}", s.text());
    s.keys("pagedown");
    s.assert_same_rows(&at_end);
}

#[test]
fn escape_returns_to_the_latest_and_keeps_the_draft() {
    let mut s = chat(80, 12);
    s.type_text("draft preserved");
    s.keys("pageup");
    s.assert_contains("↓ Back to bottom");
    s.keys("esc");
    s.assert_absent("Back to bottom");
    s.assert_contains("row 39");
    s.assert_composer("draft preserved");
    // the Escape was the transcript's: no backtrack priming
    s.assert_absent("edit the previous message");
}

#[test]
fn enter_on_an_empty_composer_returns_to_the_latest() {
    let mut s = chat(80, 12);
    s.keys("pageup");
    s.keys("enter");
    s.assert_contains("row 39");
    s.assert_absent("Back to bottom");
}

#[test]
fn the_footer_names_the_keys_that_return() {
    let mut s = chat(80, 12);
    s.assert_absent("latest");
    s.keys("pageup");
    s.assert_contains("enter/esc latest");
    s.type_text("draft");
    s.assert_contains("esc latest");
    s.assert_absent("enter/esc latest");
}

#[test]
fn new_output_while_away_is_named_in_the_footer_until_the_reader_returns() {
    let mut s = chat(80, 12);
    s.keys("pageup");
    s.type_text("d");
    s.feed(NEW_OUTPUT);
    s.assert_contains("New activity · esc latest");
    s.keys("esc");
    s.assert_absent("New activity");
    s.assert_contains("new output");
}

#[test]
fn new_output_at_the_tail_is_not_new_activity() {
    let mut s = chat(80, 12);
    s.feed(NEW_OUTPUT);
    s.assert_contains("new output");
    s.assert_absent("New activity");
    s.assert_absent("latest");
}

#[test]
fn the_wheel_reaching_the_end_hides_the_control() {
    let mut s = chat(80, 12);
    s.wheel_up(10, 3, 3);
    s.assert_contains("↓ Back to bottom");
    s.wheel_down(10, 3, 30);
    s.assert_absent("Back to bottom");
    s.assert_contains("row 39");
}

#[test]
fn escape_on_the_empty_composer_returns_and_primes_the_backtrack() {
    let mut s = chat(80, 12);
    s.keys("pageup");
    s.assert_contains("↓ Back to bottom");
    s.keys("esc");
    s.assert_absent("Back to bottom");
    s.assert_contains("row 39");
    s.assert_contains("edit the previous message");
}

#[test]
fn the_page_keys_are_the_transcripts_with_a_multiline_draft() {
    let mut s = chat(80, 12);
    s.type_text("first");
    s.keys("ctrl+j");
    s.type_text("second");
    s.keys("pageup");
    s.assert_contains("↓ Back to bottom");
    // the caret stays where it was: the keys were not the field's
    s.assert_contains("› second");
    s.keys("pagedown");
    s.assert_absent("Back to bottom");
}

#[test]
fn the_page_keys_stay_the_popups_while_the_text_opens_it() {
    let mut s = chat(80, 12);
    s.type_text("/");
    s.keys("pageup");
    s.assert_absent("Back to bottom");
}

#[test]
fn a_press_on_the_control_released_elsewhere_returns_nothing() {
    let mut s = chat(80, 12);
    s.keys("pageup");
    let control = row_of(&s, "↓ Back to bottom").expect("the control");
    let y = u16::try_from(control).expect("row");
    s.drag(40, y, 40, 2);
    s.assert_contains("↓ Back to bottom");
    s.assert_absent("row 39");
}

#[test]
fn the_control_is_reversed_and_bold_under_the_pointer() {
    let mut s = chat(80, 12);
    s.keys("pageup");
    let control = row_of(&s, "↓ Back to bottom").expect("the control");
    let y = u16::try_from(control).expect("row");
    s.assert_style(40, control, "!reversed !bold");
    s.mouse_drag(40, y);
    s.assert_style(40, control, "reversed bold");
    s.mouse_drag(40, 2);
    s.assert_style(40, control, "!reversed !bold");
}
