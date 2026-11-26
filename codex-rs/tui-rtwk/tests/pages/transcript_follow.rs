// test root: pages
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

/// One finished turn whose answer is the rows `row 00` .. `row {n-1}`. It is a restored
/// completion (no `turn/started` first): a live one the server did not time would add the
/// `Worked for` row from the status timer (oracle `FinalMessageSeparator::from_turn_once`).
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
        "{}\n{}\n",
        r#"{"step": "seed", "seed": "session", "thread": "thread-1"}"#,
        format_args!(
            r#"{{"step": "notify", "message": {{"method": "turn/completed", "params": {{"threadId": "thread-1", "turn": {}}}}}}}"#,
            turn("completed", &item, "summary")
        ),
    )
}

/// The oracle's `new_output` fixture: a later turn answering "new output".
const NEW_OUTPUT: &str = include_str!("../oracle/fixtures/transcript_view_1/new_output.jsonl");

/// The kit's script is `&'static`: built once for the binary.
fn script() -> &'static str {
    static SCRIPT: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    SCRIPT.get_or_init(|| rows(40))
}

fn chat(width: u16, height: u16) -> Screen {
    let mut s = Screen::build(Owner::ChatLive)
        .size(width, height)
        .script(script())
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
    s.assert_absent("edit previous message");
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
fn the_empty_composer_adds_its_shortcuts_key_to_the_footer_while_it_fits() {
    // `render_transcript_footer`: ` · ? shortcuts` after the hint of a
    // non-interactive transcript footer, only while the composer is empty and
    // the line holds it (the footer's row is two cells narrower than the window)
    let mut s = chat(80, 12);
    s.keys("pageup");
    s.assert_contains("enter/esc latest · ? shortcuts");
    s.type_text("d");
    s.assert_absent("shortcuts");
    let mut narrow = chat(31, 12);
    narrow.keys("pageup");
    // 16 cells of hint, 14 of shortcuts: 30 does not fit in 29
    narrow.assert_contains("enter/esc latest");
    narrow.assert_absent("shortcuts");
    let mut wide = chat(32, 12);
    wide.keys("pageup");
    wide.assert_contains("enter/esc latest · ? shortcuts");
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

const MOUSE_SELECTION: &str =
    include_str!("../oracle/fixtures/transcript_view_4/sl_command_c.jsonl");

fn selected_tail() -> Screen {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 18)
        .script(MOUSE_SELECTION)
        .open();
    s.selection_copy_policy(false, false);
    s.double_click_text("selected");
    s.assert_text_style("selected", "reversed");
    s
}

#[test]
fn clearing_a_tail_selection_stays_paused_until_the_reader_clicks_to_follow() {
    let mut s = selected_tail();
    s.keys("esc");
    s.assert_text_style("selected", "!reversed");
    s.assert_absent("⌃c copy");
    s.live_tail("first unseen response");
    s.assert_absent("first unseen response");
    s.assert_contains("New activity");
    s.click_text("Back to bottom");
    s.assert_contains("first unseen response");
    s.assert_absent("New activity");
    s.live_tail(" visible after following");
    s.assert_contains("visible after following");
}

#[test]
fn scrolling_down_with_a_cleared_range_rejoins_the_live_tail() {
    for wheel in [false, true] {
        let mut s = selected_tail();
        s.live_tail("first unseen response");
        s.keys("esc");
        s.assert_absent("first unseen response");
        s.assert_contains("New activity");
        if wheel {
            s.wheel_down(4, s.row_of("selected") as u16, 1);
        } else {
            s.keys("pagedown");
        }
        s.assert_contains("first unseen response");
        s.assert_absent("New activity");
    }
}

#[test]
fn end_with_a_tail_selection_releases_the_range_and_follows() {
    let mut s = selected_tail();
    s.live_tail("first unseen response");
    s.assert_absent("first unseen response");
    s.keys("ctrl+end");
    s.assert_contains("first unseen response");
    s.assert_absent("New activity");
    s.assert_text_style("selected", "!reversed");
    s.assert_copies_asked(&[]);
}

#[test]
fn an_accepted_submit_releases_the_selected_reading_position() {
    let mut s = selected_tail();
    s.live_tail("first unseen response").commit_live_tail();
    s.type_text("actually send this");
    s.assert_composer("actually send this");
    s.assert_absent("first unseen response");
    s.assert_contains("New activity");
    s.keys("enter");
    s.assert_sole_turn(crate::kit::Turn::text("actually send this"));
    s.assert_contains("first unseen response");
    s.assert_absent("New activity");
    s.assert_copies_asked(&[]);
}

#[test]
fn a_resume_that_replaces_the_selected_tail_releases_it_even_with_the_same_first_item() {
    use crate::kit::agent_turn;

    let old = agent_turn("history", &["kept first reply", "selected retired reply"]);
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 18)
        .replay(vec![old])
        .open();
    s.selection_copy_policy(false, false);
    s.double_click_text("selected");
    s.assert_text_style("selected", "reversed");

    let mut repaired = agent_turn("history", &["kept first reply", "replacement reply"]);
    repaired["items"][1]["id"] = serde_json::json!("replacement-id");
    s.stream_lost("The writer must reconnect.");
    s.restore_writer_with_history(vec![repaired]);
    s.assert_requests_of("thread/resume", 2);
    s.assert_contains("kept first reply");
    s.assert_contains("replacement reply");
    s.assert_absent("selected retired reply");
    s.assert_absent("⌃c copy");
    s.keys("ctrl+insert");
    s.assert_copies_asked(&[]);
}

#[test]
fn a_held_tail_belongs_to_its_thread_when_switching_away_and_back() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 18)
        .script(MOUSE_SELECTION)
        .side()
        .open();
    s.selection_copy_policy(false, false);
    s.keys("ctrl+/");
    s.double_click_text("selected");
    s.assert_text_style("selected", "reversed");
    s.live_tail("main unseen response");
    s.assert_contains("New activity");

    s.keys("ctrl+/");
    s.live_tail("independent side response");
    s.assert_contains("independent side response");
    s.assert_absent("main unseen response");
    s.assert_absent("New activity");
    s.assert_absent("selected café");

    s.keys("ctrl+/");
    s.assert_contains("selected café");
    s.assert_contains("New activity");
    s.assert_absent("main unseen response");
    s.assert_absent("independent side response");
    s.keys("ctrl+end");
    s.assert_contains("main unseen response");
    s.assert_absent("New activity");
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
fn changing_wheel_speed_keeps_the_paused_viewport() {
    let mut s = chat(80, 12);
    s.type_text("draft stays");
    s.keys("pageup pageup");
    s.assert_contains("Back to bottom");
    s.assert_absent("row 39");
    let paused = s.shot();
    for speed in [Some(0.5), Some(1.5), None] {
        s.mouse_scroll_speed(speed);
        s.assert_same_rows(&paused);
        s.assert_composer("draft stays");
    }
    s.keys("esc");
    s.assert_contains("row 39");
    s.assert_absent("Back to bottom");
    s.assert_composer("draft stays");
}

#[test]
fn wheel_step_clamps_finite_config_values_to_native_range() {
    use crate::common::{ms, start_driven_with};
    use codex_tui_core::app::{AppEv, facts::TuiSettings};
    use rtwk::scroll::{ScrollBehavior, ScrollConfiguration};
    use rtwk_core::input::{MouseEvent, MouseKind};
    use rtwk_core::scroll::ScrollInput;
    use rtwk_test::{Options, find};

    let mut h = start_driven_with(false, Options::new().size(80, 12));
    h.connect();
    h.advance(100);
    let configuration = find::by_type::<ScrollConfiguration>();
    for (requested, step) in [
        (1e-100, f32::from_bits(1)),
        (1.5, 1.5),
        (1e100, 2_147_483_520.0),
    ] {
        h.inject([AppEv::Tui(TuiSettings {
            wheel_rows_per_event: Some(requested),
            ..TuiSettings::default()
        })]);
        h.advance(100);
        h.t.widget(&configuration, |scope: &ScrollConfiguration| {
            assert_eq!(scope.behavior, ScrollBehavior::fixed(step));
        });
    }

    let behavior =
        h.t.widget(&configuration, |scope: &ScrollConfiguration| scope.behavior);
    let mut wheel = ScrollInput::new(behavior);
    let event = MouseEvent::new(MouseKind::WheelDown, 12, 3);
    for time in [0, 10, 20] {
        assert!(wheel.event(ms(time), &event));
        assert_eq!(wheel.take().map(|delta| delta.rows), Some(2_147_483_520));
    }
}

#[test]
fn fractional_wheel_pauses_follow_only_when_it_moves_a_row() {
    let mut s = chat(80, 12);
    s.type_text("draft stays");
    s.mouse_scroll_speed(Some(0.5));
    s.wheel_up(12, 3, 1);
    s.assert_contains("row 39");
    s.assert_absent("Back to bottom");
    s.wheel_up(12, 3, 1);
    s.assert_contains("Back to bottom");
    s.resize(40, 12);
    s.assert_row(0, "  row 31");
    let paused = s.shot();
    s.wheel_down(12, 3, 1);
    s.assert_same_rows(&paused);
    s.wheel_down(12, 3, 1);
    s.assert_row(0, "  row 32");
    s.keys("esc");
    s.assert_contains("row 39");
    s.assert_absent("Back to bottom");
    s.assert_composer("draft stays");
}

#[test]
fn wheel_speed_is_kept_when_switching_between_main_and_side_conversations() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 12)
        .script(script())
        .side()
        .open();
    s.mouse_scroll_speed(Some(1.5));
    let rows = (0..40)
        .map(|i| format!("side row {i:02}"))
        .collect::<Vec<_>>()
        .join("  \n");
    s.live_tail(&rows).commit_live_tail();
    s.assert_contains("side row 39");

    fn first(s: &Screen, prefix: &str) -> usize {
        s.rows()
            .iter()
            .find_map(|line| line.trim().strip_prefix(prefix)?.parse().ok())
            .unwrap_or_else(|| panic!("no {prefix} row:\n{}", s.text()))
    }

    let side = first(&s, "side row ");
    s.wheel_up(12, 3, 1);
    assert_eq!(first(&s, "side row "), side - 1);
    s.keys("esc");
    s.assert_contains("side row 39");
    s.keys("ctrl+/");
    s.assert_absent("side row ");
    s.assert_contains("row 39");
    let main = first(&s, "row ");
    s.wheel_up(12, 3, 1);
    assert_eq!(first(&s, "row "), main - 1);
    s.wheel_up(12, 3, 1);
    assert_eq!(first(&s, "row "), main - 3);
    s.keys("ctrl+/");
    s.assert_contains("side row 39");
    assert_eq!(first(&s, "side row "), side);
    s.wheel_up(12, 3, 1);
    assert_eq!(first(&s, "side row "), side - 1);
}

#[test]
fn escape_on_the_empty_composer_returns_and_primes_the_backtrack() {
    let mut s = chat(80, 12);
    s.keys("pageup");
    s.assert_contains("↓ Back to bottom");
    s.keys("esc");
    s.assert_absent("Back to bottom");
    s.assert_contains("row 39");
    // primed, but the hint is only for a chat with a prompt to edit
    // (`prime_backtrack`: `has_backtrack_target`); this one has no prompt
    s.assert_absent("edit previous message");
}

#[test]
fn the_page_keys_are_the_transcripts_with_a_multiline_draft() {
    let mut s = chat(80, 12);
    s.type_text("first");
    s.keys("ctrl+j");
    s.type_text("second");
    s.keys("pageup");
    s.assert_contains("↓ Back to bottom");
    // the draft is the two lines of `ctrl+j` and the caret stays after the second: the keys were
    // not the field's
    s.assert_contains("› first");
    s.assert_row(9, "  second");
    s.assert_cursor(8, 9);
    s.keys("pagedown");
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

#[test]
fn a_window_with_no_row_for_the_transcript_has_no_reader_away() {
    // the reserved row, the composer and the footer fill the five rows: the
    // transcript has none, and a reader with no viewport is not away (no
    // control, no hint: `area.is_empty()` in the oracle)
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 5)
        .script(script())
        .open();
    s.wait_for("shortcuts");
    s.assert_absent("Back to bottom");
    s.assert_absent("latest");
}

#[test]
fn the_composer_is_the_same_element_while_the_turn_starts_and_the_footer_changes() {
    // sending starts a turn: the working row and the blank rows come above
    // the composer and the footer below it changes kind; a new composer
    // element would lose the messages queued for the old one (a panic in
    // its update: no provider above it)
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    s.type_text("first").keys("enter");
    s.type_text("second").keys("enter");
    s.type_text("third");
    s.assert_composer("third");
}
