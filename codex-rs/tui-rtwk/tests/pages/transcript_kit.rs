// test root: pages
//! The doors of `d-transcript-kit`: one test per door, each the smallest use a transcript feature
//! makes of it. The features that use them have their own tests.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_methods)]

use rtwk_core::input::{Mods, MouseKind};

use crate::kit::{Owner, Screen};

const TOP_BOTTOM: &str = include_str!("../oracle/fixtures/transcript_view_4/sm_entry_1.jsonl");

const SESSION: &str = r#"{"step":"seed","seed":"session","thread":"thread-1"}"#;

fn entries(lines: &[&str]) -> &'static str {
    let mut text = String::from(SESSION);
    for line in lines {
        text.push('\n');
        text.push_str(line);
    }
    Box::leak(text.into_boxed_str())
}

/// The oracle's select-mode tests drive the view at 32x6 with no bottom pane (`render` of
/// `select_mode_tests.rs`); the full chat leaves its list one row there.
#[test]
fn the_view_alone_gives_the_list_every_row_of_a_32x6_terminal() {
    let two_rows = entries(&[
        r#"{"step":"entry","kind":"agent","turn":"t","text":"top","end":false}"#,
        r#"{"step":"entry","kind":"agent","turn":"t","text":"bottom","start":false}"#,
    ]);
    let full = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(two_rows)
        .open();
    full.assert_absent("top");
    let view = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(two_rows)
        .view_only()
        .open();
    view.assert_row(1, "• top");
    view.assert_row(3, "• bottom");
    view.assert_contains("Worked for");
    // no composer, no footer: the transcript is the whole screen
    view.assert_absent("Ask Codex");
    view.assert_absent("? for shortcuts");
}

/// The oracle's `sm_entry_*` cells are one entry per turn; the port closes each turn with its
/// "Worked for" row, so two of them do not fit 6 rows: the newest is at the bottom, no bottom pane
/// under it, the older one is a scroll away.
#[test]
fn the_view_alone_scrolls_to_the_older_entry_of_the_oracles_cells() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(TOP_BOTTOM)
        .view_only()
        .open();
    s.assert_contains("• bottom");
    s.assert_absent("Ask Codex");
    s.wheel_up(0, 0, 10);
    s.assert_contains("• top");
}

/// The shapes the transcript features read: each is an `entry` step, drawn by the transcript sheet
/// as the chat's own cell (the tab and the styled spans are the entry features' to draw).
#[test]
fn every_entry_shape_has_a_script_step() {
    let text = entries(&[
        r#"{"step":"entry","kind":"user","text":"the prompt"}"#,
        r#"{"step":"entry","kind":"agent","text":"a\tb **bold**"}"#,
        r#"{"step":"entry","kind":"reasoning","text":"the reasoning"}"#,
        r#"{"step":"entry","kind":"plan","text":"the plan"}"#,
        r#"{"step":"entry","kind":"plan_update","steps":[["Write it","completed"],["Test it","inProgress"]]}"#,
        r#"{"step":"entry","kind":"exec","command":"printf visible","output":"visible\n","exit":0}"#,
        r#"{"step":"entry","kind":"exec","command":"sleep 9","running":true,"end":false}"#,
    ]);
    let mut s = Screen::build(Owner::ChatLive)
        .size(60, 90)
        .script(text)
        .view_only()
        .open();
    s.recap("The recap.", Some("Run it."));
    for shown in [
        "› the prompt",
        "• a b bold",
        "• the reasoning",
        "• Proposed Plan",
        "  the plan",
        "• Updated Plan",
        "  └ ✔ Write it",
        "    □ Test it",
        "$ printf visible",
        "↳ Recap: The recap.",
        "Next: Run it.",
        "$ sleep 9",
    ] {
        s.assert_contains(shown);
    }
}

/// The oracle's live cell: the answer streams (hidden from history until its turn ends), then
/// commits. The transcript shows it all along and the commit adds the turn's "Worked for" row.
#[test]
fn the_live_tail_grows_and_a_commit_ends_its_turn() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(40, 8)
        .script(SESSION)
        .view_only()
        .open();
    s.live_tail("live needle");
    s.assert_contains("• live needle");
    s.assert_absent("Worked for");
    s.live_tail(" again needle");
    s.assert_contains("• live needle again needle");
    s.assert_absent("Worked for");
    s.commit_live_tail();
    s.assert_contains("• live needle again needle");
    s.assert_contains("Worked for");
}

/// The oracle sets `TranscriptHistoryState::Partial` by hand and joins a page by hand: here the
/// server's first page has a cursor behind it, the reader goes to the top, and the older page
/// the app asked for joins above, under a live answer that keeps streaming.
#[test]
fn an_older_page_joins_above_a_partial_history_and_the_live_tail() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(30, 6)
        .replay(Vec::new())
        .view_only()
        .open();
    s.partial_history(3);
    s.assert_contains("row 02");
    s.live_tail("latest answer");
    s.assert_contains("• latest answer");
    s.join_older_page(&["older a", "older b"], false);
    assert_eq!(s.page_requests(), [None, Some("cursor-1".to_string())]);
    assert_eq!(s.pages_waiting(), 0);
    // the reader keeps the row it was on; the joined rows are above it
    s.assert_contains("row 00");
    s.assert_absent("older a");
    s.wheel_up(0, 0, 100);
    s.assert_contains("older a");
    s.live_tail(" and more");
    s.wheel_down(0, 0, 100);
    s.assert_contains("• latest answer and more");
}

fn find_in_the_reader(overlay: bool, accepted: bool) -> Screen {
    let open = Screen::build(Owner::ChatLive)
        .size(80, 18)
        .script(include_str!(
            "../oracle/fixtures/transcript_view_2/sel_hint.jsonl"
        ));
    let mut s = if overlay { open.view_overlay() } else { open }.open();
    s.selection_copy_policy(false, false);
    s.keys("f3");
    s.type_text("selection");
    s.assert_find_query("selection");
    s.assert_text_style("selection", "reversed");
    s.assert_absent("⌃c copy");
    if accepted {
        s.keys("enter");
        s.assert_contains("Find ·");
    }
    s
}

fn find_with_range_in_the_reader(overlay: bool, accepted: bool) -> Screen {
    let mut s = find_in_the_reader(overlay, accepted);
    s.double_click_text("é");
    s.assert_text_style("é", "reversed");
    s.assert_contains("⌃c copy");
    s.assert_copies_asked(&[]);
    s
}

#[test]
fn a_manual_range_copies_while_the_find_query_is_being_edited() {
    for overlay in [false, true] {
        let mut s = find_with_range_in_the_reader(overlay, false);
        s.assert_contains("Find: selection");
        s.assert_absent("enter accept");
        s.keys("ctrl+c");
        s.assert_copies_asked(&["é"]);
        s.assert_find_query("selection");
        s.answer_copy(Ok(()));
        s.assert_text_style("é", "!reversed");
        s.assert_contains("enter accept");
        s.assert_absent("⌃c copy");
        s.type_text("x");
        s.assert_find_query("selectionx");
        s.assert_contains("Find: selectionx");
        s.keys("ctrl+c");
        s.assert_absent("Find:");
        s.assert_copies_asked(&["é"]);
    }
}

#[test]
fn a_manual_range_copies_while_reading_an_accepted_find_match() {
    for overlay in [false, true] {
        let mut s = find_with_range_in_the_reader(overlay, true);
        s.assert_absent("Find ·");
        s.keys("ctrl+c");
        s.assert_copies_asked(&["é"]);
        s.answer_copy(Ok(()));
        s.assert_find_query("selection");
        s.assert_contains("Find ·");
        s.assert_text_style("é", "!reversed");
        s.assert_absent("⌃c copy");
    }
}

#[test]
fn middle_click_does_not_make_a_find_match_copyable() {
    for overlay in [false, true] {
        let mut s = find_in_the_reader(overlay, true);
        let y = u16::try_from(s.row_of("é selection")).expect("a transcript row");
        s.mouse_event(MouseKind::Down, 1, 2, y, Mods::NONE);
        s.mouse_event(MouseKind::Up, 1, 2, y, Mods::NONE);
        s.assert_contains("Find ·");
        s.assert_absent("⌃c copy");
        s.keys("ctrl+c");
        s.assert_copies_asked(&[]);
        s.assert_absent("Find ·");
    }
}

#[test]
fn a_startup_tip_opens_its_web_link_from_both_cells_of_each_glyph() {
    const DOCS: &str = "https://example.com/docs";
    let mut s = Screen::build(Owner::ChatLive)
        .size(72, 24)
        .live()
        .startup(false, Some("[文档](https://example.com/docs)"))
        .open();
    let row = s.row_of("Tip: 文档");
    s.assert_row_starts_with(row, "  Tip: 文档");
    s.assert_span_linked("Tip:", 5, 4, DOCS, true);
    let y = u16::try_from(row).expect("a tip row");
    let mut opened = Vec::new();
    for x in 7..11 {
        s.mouse_down(x, y);
        s.assert_opened_urls(&opened);
        s.mouse_up(x, y);
        opened.push(DOCS);
        s.assert_opened_urls(&opened);
    }
}

#[test]
fn the_mermaid_code_fallback_copies_the_complete_fenced_source() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(50, 15)
        .owned_transcript()
        .script(include_str!(
            "../oracle/fixtures/transcript_view_2/mermaid.jsonl"
        ))
        .open();
    s.assert_in_order(&["graph LR", "A --> B"]);
    s.type_text("/copy").keys("enter");
    s.keys("up");
    s.assert_contains("Copy code block");
    s.keys("enter");
    s.assert_copies_asked(&["graph LR\nA --> B\n"]);
}

const PATCH: &str = include_str!("../oracle/fixtures/transcript_view_2/patch_disclosure.jsonl");
const APPROVAL: &str =
    include_str!("../oracle/fixtures/transcript_view_2/patch_disclosure_approval.jsonl");
const RESOLVED: &str =
    include_str!("../oracle/fixtures/transcript_view_2/patch_disclosure_resolved.jsonl");
const PATCH_PROMPT: &str = "Would you like to make the following edits?";

#[test]
fn resolving_a_patch_approval_keeps_the_disclosed_manual_viewport() {
    for scroll_before_click in [false, true] {
        let mut s = Screen::build(Owner::ChatLive).size(48, 29).live().open();
        s.mouse_scroll_speed(Some(1.0));
        s.feed(PATCH);
        s.assert_in_order(&["approval-patch.txt", "+ Show details", "tail output 10"]);
        s.assert_absent("Back to bottom");
        s.assert_absent("patch line five");
        if scroll_before_click {
            s.wheel_up(1, 1, 1);
        }
        s.click_text("+ Show details");
        s.assert_in_order(&["approval-patch.txt", "patch line five", "Hide details"]);
        if !scroll_before_click {
            s.wheel_up(1, 1, 1);
        }
        s.assert_contains("Back to bottom");
        let end = s.row_of("Hide details") + 1;
        let before = s.shot();
        s.feed(APPROVAL);
        s.assert_contains(PATCH_PROMPT);
        s.feed(RESOLVED);
        s.assert_absent(PATCH_PROMPT);
        s.assert_no_resolved_requests();
        s.assert_contains("Back to bottom");
        s.assert_same_rows_in(&before, 0..end);
    }
}

#[test]
fn patch_disclosure_keeps_the_active_approval_and_its_keyboard_focus() {
    use codex_tui_core::app::Command;

    let mut s = Screen::build(Owner::ChatLive).size(48, 29).live().open();
    s.feed(PATCH);
    s.type_text("carried draft");
    s.feed(APPROVAL).wait_for(PATCH_PROMPT);

    let second = APPROVAL.replace("\"id\":73", "\"id\":74").replace(
        "Inspect the patch before applying it",
        "Second patch confirmation",
    );
    s.feed(&second);
    s.assert_contains(PATCH_PROMPT);
    s.assert_absent("Second patch confirmation");

    s.click_text("+ Show details");
    s.assert_in_order(&["approval-patch.txt", "patch line five", "Hide details"]);
    s.assert_contains(PATCH_PROMPT);
    s.assert_no_resolved_requests();

    s.keys("down");
    s.assert_contains("› 2. Yes, and don't ask again");
    s.keys("up y").wait_for("Second patch confirmation");
    let answers: Vec<_> = s
        .commands()
        .into_iter()
        .filter_map(|command| match command {
            Command::ResolveRequest { key, .. } => Some(key),
            _ => None,
        })
        .collect();
    assert_eq!(answers.len(), 1);
    assert_eq!(answers[0].thread.as_str(), "thread-1");
    assert_eq!(answers[0].id.as_str(), "73");
    s.assert_contains(PATCH_PROMPT);

    s.feed(RESOLVED);
    s.assert_contains("Second patch confirmation");
    let second_resolved = RESOLVED.replace("\"requestId\":73", "\"requestId\":74");
    s.feed(&second_resolved);
    s.assert_absent(PATCH_PROMPT);
    assert_eq!(s.resolved_requests().len(), 1);
    s.type_text(" after");
    s.assert_contains("carried draft after");
}
