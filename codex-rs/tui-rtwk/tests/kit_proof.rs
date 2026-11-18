//! The proof of the screen-test kit (`tests/kit/`): three oracle tests of
//! landed screens ported with it, and the kit's own tests. Every test here is
//! green; the pending, todo and n/a markers are shown by tests that expect
//! them.

mod common;
mod kit;

use kit::{Owner, Screen};

// ---- three oracle tests, ported (recipe: docs/rtwk-port/refs/screen-tests.md) ----

/// Oracle: `resume_picker.rs` `resume_search_error_snapshot`. Seed: the
/// resume screen's entry for a failed listing; the snapshot is the search-line
/// region (screen row 2, one column in) of an 80 x 13 screen.
#[test]
fn resume_search_error_snapshot() {
    Screen::build(Owner::RoutesLists)
        .size(80, 13)
        .entry("resume", "search_error")
        .open()
        .assert_snap_region("resume_picker_search_error", 2, 1);
}

/// Oracle: `resume_picker.rs` `mixed_backend_resume_table_snapshot`: the oracle
/// selects the second row; its snapshot is the list region of the screen (our
/// screen is 82 wide: the list starts two columns in, at screen row 6).
#[test]
fn mixed_backend_resume_table_snapshot() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(82, 22)
        .entry("resume", "table")
        .open();
    s.keys("down");
    s.assert_snap_region("resume_picker_table", 6, 2);
}

/// Oracle: `resume_picker.rs` `hint_line_snapshot_uses_distributed_wide_footer`:
/// the footer region, at screen row 18.
#[test]
fn hint_line_snapshot_uses_distributed_wide_footer() {
    Screen::build(Owner::RoutesLists)
        .size(220, 22)
        .entry("resume", "footer_wide")
        .open()
        .assert_snap_region("resume_picker_footer_wide", 18, 0);
}

// ---- the verdict markers ----

#[test]
#[should_panic(expected = "PENDING[dialogs-pane]: the bottom pane does not draw this yet")]
fn pending_fails_with_its_owner_and_reason() {
    kit::pending(
        Owner::DialogsPane,
        "the bottom pane does not draw  this yet",
    );
    unreachable!("the body of a pending test is not reached outside a probe");
}

#[test]
#[should_panic(expected = "PENDING[ui-pane]: no screen directory `no_such_dir`")]
fn a_seed_that_does_not_exist_is_pending_not_a_crash() {
    drop(
        Screen::build(Owner::UiPane)
            .entry("no_such_dir", "x")
            .open(),
    );
}

#[test]
#[should_panic(expected = "PENDING[ui-pane]: no debug entry `status/no_such_entry`")]
fn a_missing_entry_is_pending_with_its_name() {
    drop(
        Screen::build(Owner::UiPane)
            .entry("status", "no_such_entry")
            .open(),
    );
}

#[test]
#[should_panic(expected = "TODO[port]")]
fn a_stub_is_todo() {
    kit::todo();
}

#[test]
fn na_passes_and_says_whose() {
    kit::na(Owner::SessionCore, "needs the real App to resume a thread");
}

// ---- the comparison ----

#[test]
#[should_panic(expected = "ours (right) differs from the oracle (left)")]
fn a_different_row_fails_and_is_never_loosened() {
    // The status card is not the resume table.
    Screen::build(Owner::DialogsPane)
        .size(100, 24)
        .entry("status", "reasoning_details")
        .open()
        .assert_snap("resume_picker_table");
}

#[test]
fn labels_of_keys_are_normalized_on_our_rows_only() {
    let none: Vec<String> = Vec::new();
    assert_eq!(
        kit::normalize_labels("shift+tab to cycle", &none),
        "⇧tab to cycle"
    );
    assert_eq!(kit::normalize_labels("ctrl + c quit", &none), "⌃c quit");
    assert_eq!(kit::normalize_labels("alt+x ", &none), "⌥x ");
    assert_eq!(kit::normalize_labels("shift-tab", &none), "shift-tab");
    assert_eq!(kit::normalize_labels("alternate", &none), "alternate");
    // The oracle's own text holds the word form: ours is left alone.
    let oracle = vec!["  ctrl + c to quit".to_string()];
    assert_eq!(
        kit::normalize_labels("ctrl + c quit", &oracle),
        "ctrl + c quit"
    );
}

#[test]
fn snapshots_are_found_by_test_name_or_file_stem() {
    let by_name = kit::snap_size("resume_picker_table");
    let by_stem = kit::snap_size("codex_tui__resume_picker__tests__resume_picker_table");
    assert_eq!(by_name, by_stem);
    assert!(by_name.0 > 40 && by_name.1 > 3, "{by_name:?}");
}

// ---- the seeds and inputs ----

/// A script feeds the live chat as a server would; the chat shows it.
#[test]
fn a_script_seeds_the_live_chat() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!("kit/fixtures/answered.jsonl"))
        .open();
    s.wait_for("• Hello there");
    s.assert_contains("› hi");
    s.assert_absent("never shown");
}

/// Keys reach the screen: typing fills the composer.
#[test]
fn keys_and_text_reach_the_live_chat() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.type_text("draft");
    s.assert_contains("draft");
}

/// The mouse reaches the screen: a click on a row text and a wheel turn are
/// delivered without a crash; the effects are the slices' (their tests).
#[test]
fn mouse_helpers_deliver() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(82, 22)
        .entry("resume", "table")
        .open();
    s.click_text("Explain the codebase");
    s.double_click(3, 8);
    s.wheel_down(10, 8, 2);
    s.wheel_up(10, 8, 2);
    s.assert_contains("Fix resume picker timestamps");
}

// ---- gaps closed by screen-test-kit-2 ----
//
// The comparison engine is shown on grids fed with the oracle's own text (a
// `rtwk_test::Screen` taking bytes), so a test can change one cell and watch
// the compare fail: every feature here has a passing case and a biting one.

use kit::{Grid, Want};

/// A terminal of `width` x `height` cells that has been fed `bytes`.
fn grid_of(width: usize, height: usize, bytes: &str) -> Grid {
    let mut vt = rtwk_test::Screen::new(width, height);
    vt.feed(bytes.as_bytes());
    Grid::of(&vt)
}

/// `rows` written from the top left, one per line.
fn put(rows: &[&str]) -> String {
    format!("\x1b[H{}", rows.join("\r\n"))
}

fn want<'a>(name: &'a str, frame: &'a str) -> Want<'a> {
    Want {
        name,
        frame,
        at: None,
    }
}

/// Whether `f` panics (a bite: the compare must fail on a changed screen).
fn fails(f: impl FnOnce()) -> bool {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).is_err()
}

/// Gap 1: every header kind the oracle's joined snapshots use, and the lookup
/// by the tail of a file name that has a test-module name before it (gap 6).
#[test]
fn frames_of_every_header_kind_are_found() {
    // `---- x ----`
    assert_eq!(
        kit::snap_frames("resume_picker_db_fallback_transition"),
        ["loading fallback", "repaired results"]
    );
    // `--- x ---`, file `..__transcript_tests__<name>`
    assert_eq!(
        kit::snap_frames("transcript_overlay_paginated_history_states")[0],
        "loading"
    );
    // `N columns:`, a header twice is `#2`
    assert_eq!(
        kit::snap_frames("hook_status_reflows_with_background_activity"),
        ["124 columns", "123 columns", "24 columns", "124 columns#2"]
    );
    // `N columns: text` on one line is a frame of one row
    assert_eq!(
        kit::snap_rows("usage_notice_widths", "98 columns"),
        ["⚠ 5h limit: 8% left · resets at 1:00 PM · /status"]
    );
    // `width=N, ..`
    assert!(
        kit::snap_frames("narrow_long_range_charts_never_truncate_dates")[0].starts_with("width=")
    );
    // a label above a `Buffer {` dump
    assert_eq!(
        kit::snap_frames("activity_focus_keeps_controls_without_passive_hints")[0],
        "collapsed"
    );
}

/// Gap 1 and 2: a frame is compared with the screen, whole or as a region at a
/// row and column.
#[test]
fn a_frame_is_compared_as_a_whole_or_as_a_region() {
    let g = grid_of(40, 3, &put(&["⚠ 5h 8% left"]));
    kit::assert_snap_on(&g, &want("usage_notice_widths", "37 columns"));
    // the same rows one column in and one row down
    let g = grid_of(40, 4, "\x1b[2;2H⚠ 5h 8% left");
    kit::assert_snap_on(
        &g,
        &Want {
            name: "usage_notice_widths",
            frame: "37 columns",
            at: Some((1, 1)),
        },
    );
    // bite: another frame's text
    let wrong = grid_of(40, 3, &put(&["⚠ 5h limit: 8% left"]));
    assert!(fails(|| kit::assert_snap_on(
        &wrong,
        &want("usage_notice_widths", "37 columns")
    )));
    // bite: a snapshot of several frames asked for whole
    let g = grid_of(40, 3, &put(&["x"]));
    assert!(fails(|| kit::assert_snap_on(
        &g,
        &want("usage_notice_widths", "")
    )));
}

/// Gap 8: a `Buffer` dump gives rows, styles and cursor; the styles are
/// compared cell by cell.
#[test]
fn a_buffer_dump_is_compared_with_its_styles() {
    // `collapsed`: row 0 plain, row 1 DIM, row 2 plain, 32 cells wide
    let rows =
        "Activity summary                \r\n\x1b[2m    + Show details              \x1b[0m\r\n";
    let g = grid_of(32, 3, &format!("\x1b[H{rows}"));
    kit::assert_snap_on(
        &g,
        &want(
            "activity_focus_keeps_controls_without_passive_hints",
            "collapsed",
        ),
    );
    // bite: row 1 not dim
    let plain = grid_of(32, 3, &put(&["Activity summary", "    + Show details"]));
    let outcome = std::panic::catch_unwind(|| {
        kit::assert_snap_on(
            &plain,
            &want(
                "activity_focus_keeps_controls_without_passive_hints",
                "collapsed",
            ),
        );
    });
    let message = format!(
        "{:?}",
        outcome
            .err()
            .and_then(|e| e.downcast_ref::<String>().cloned())
    );
    assert!(message.contains("cell styles differ"), "{message}");
    // bite: the rows differ
    let other = grid_of(32, 3, &put(&["Activity", "    + Show details"]));
    assert!(fails(|| kit::assert_snap_on(
        &other,
        &want(
            "activity_focus_keeps_controls_without_passive_hints",
            "collapsed"
        ),
    )));
}

/// Gap 8: the `Cursor:` line of a snapshot is compared with the screen's cursor.
#[test]
fn the_cursor_line_of_a_snapshot_is_compared() {
    let name = "history_search_pasted_separators";
    let rows = kit::snap_rows(name, "");
    let refs: Vec<&str> = rows.iter().map(String::as_str).collect();
    // the oracle: `Cursor: Some((31, 4))`
    let g = grid_of(70, 5, &format!("{}\x1b[5;32H\x1b[?25h", put(&refs)));
    kit::assert_snap_on(&g, &want(name, ""));
    // bite: the cursor one column left; the cursor hidden
    let left = grid_of(70, 5, &format!("{}\x1b[5;31H\x1b[?25h", put(&refs)));
    assert!(fails(|| kit::assert_snap_on(&left, &want(name, ""))));
    let hidden = grid_of(70, 5, &format!("{}\x1b[5;32H\x1b[?25l", put(&refs)));
    assert!(fails(|| kit::assert_snap_on(&hidden, &want(name, ""))));
}

/// Gap 7: a wide glyph is one char in the oracle's Debug rows (`Hidden by
/// multi-width symbols`) and a glyph and a space in its text rows; one compare
/// takes either.
#[test]
fn wide_glyphs_match_either_convention_of_the_oracle() {
    // text rows: `printf '診 断 '` is the oracle's way to write 診断
    let name = "question_multiline";
    let rows: Vec<String> = kit::snap_rows(name, "")
        .iter()
        .map(|r| r.replace("診 断 ", "診断"))
        .collect();
    let refs: Vec<&str> = rows.iter().map(String::as_str).collect();
    let g = grid_of(80, rows.len(), &put(&refs));
    kit::assert_snap_on(&g, &want(name, ""));
    let mut broken = rows.clone();
    broken[0].push('x');
    let refs: Vec<&str> = broken.iter().map(String::as_str).collect();
    let g = grid_of(80, rows.len(), &put(&refs));
    assert!(fails(|| kit::assert_snap_on(&g, &want(name, ""))));
    // Debug rows: the note after the quotes is not part of the row
    let name = "hook_status_reflows_without_background_activity";
    let frames = kit::snap_frames(name);
    assert!(frames.len() >= 3, "{frames:?}");
    let narrow = kit::snap_rows(name, &frames[2]);
    assert!(narrow[1].starts_with("  └ checking 日本語"), "{narrow:?}");
    assert!(!narrow[1].contains("Hidden"), "{narrow:?}");
    // ... and a screen with the glyphs (two cells each) equals them
    let refs: Vec<&str> = narrow.iter().map(String::as_str).collect();
    let g = grid_of(24, 3, &put(&refs));
    kit::assert_snap_on(&g, &want(name, &frames[2]));
    let cut: Vec<&str> = refs.iter().map(|r| r.trim_end_matches('…')).collect();
    let g = grid_of(24, 3, &put(&cut));
    assert!(fails(|| kit::assert_snap_on(&g, &want(name, &frames[2]))));
}

/// Gap 8: the style spec is written as the oracle's assertion reads.
#[test]
fn a_style_spec_reads_like_the_oracles_assertion() {
    // A bold cyan, B dim, C reversed on an rgb background, D plain
    let g = grid_of(
        8,
        1,
        "\x1b[H\x1b[1;36mA\x1b[0m\x1b[2mB\x1b[0m\x1b[7;48;2;1;2;3mC\x1b[0mD",
    );
    g.assert_style(0, 0, "fg=Cyan bold");
    g.assert_style(0, 0, "fg=Cyan bold !dim bg=Reset");
    g.assert_style(1, 0, "dim !bold");
    g.assert_style(2, 0, "reversed bg=Rgb(1,2,3)");
    g.assert_style(2, 0, "bg=Rgb(1, 2, 3)");
    g.assert_style(3, 0, "plain fg=Reset");
    assert!(fails(|| g.assert_style(0, 0, "fg=Red")));
    assert!(fails(|| g.assert_style(0, 0, "dim")));
    assert!(fails(|| g.assert_style(1, 0, "!dim")));
    assert!(fails(|| g.assert_style(3, 0, "bold")));
    assert!(fails(|| g.assert_style(0, 0, "plain")));
    assert!(fails(|| g.assert_style(0, 0, "bld"))); // a typo is not a pass
    g.assert_text_style("AB", "!reversed");
    assert!(fails(|| g.assert_text_style("AB", "bold")));
    assert_eq!(kit::color_name(rtwk_core::style::Color::YELLOW), "Yellow");
    assert_eq!(kit::modifier_name(rtwk_core::style::Attrs::NONE), "NONE");
}

/// Gap 8 (OSC 8, title) and the cursor.
#[test]
fn links_cursor_and_title_are_read() {
    let g = grid_of(
        30,
        2,
        "\x1b[H\x1b]8;;https://example.com/pr/42\x07#42\x1b]8;;\x07 plain\x1b[2;4H\x1b[?25h",
    );
    g.assert_link("#42", "https://example.com/pr/42");
    g.assert_no_link("plain");
    assert!(fails(|| g.assert_link("#42", "https://example.com/other")));
    assert!(fails(|| g.assert_link("plain", "https://example.com/pr/42")));
    assert!(fails(|| g.assert_no_link("#42")));
    g.assert_cursor(3, 1);
    assert!(fails(|| g.assert_cursor(3, 0)));
    assert!(fails(|| g.assert_cursor_hidden()));
    let hidden = grid_of(5, 1, "\x1b[?25l");
    hidden.assert_cursor_hidden();
    // the last title the app wrote, OSC 0 or 2, ended by BEL or ST
    assert_eq!(
        kit::title_in("\x1b]0;first\x07x\x1b]2;second\x1b\\y"),
        Some("second".to_string())
    );
    assert_eq!(kit::title_in("no title"), None);
}

/// Gap 4: the oracle's assertion shapes beyond `contains`.
#[test]
fn row_prefix_suffix_order_and_column_asserts() {
    let g = grid_of(
        40,
        3,
        &put(&[
            "Working (0s • esc to interrupt)",
            "  1 of 3 · ↑ scroll",
            "alpha beta",
        ]),
    );
    g.assert_row_starts_with(0, "Working (0s");
    g.assert_row_ends_with(1, " · ↑ scroll");
    g.assert_has_row("alpha beta");
    g.assert_in_order(&["Working", "1 of 3", "beta"]);
    g.assert_text_at(2, 1, "1 of 3");
    g.assert_row(2, "alpha beta");
    assert!(fails(|| g.assert_row_starts_with(1, "Working")));
    assert!(fails(|| g.assert_row_ends_with(1, "· z scroll")));
    assert!(fails(|| g.assert_has_row("alpha")));
    assert!(fails(|| g.assert_in_order(&["beta", "Working"])));
    assert!(fails(|| g.assert_text_at(3, 1, "1 of 3")));
    assert!(fails(|| g.assert_row(2, "alpha")));
}

/// Gaps 5, 8 and 9 on a live screen: the first frame, a screen against an
/// earlier one, the cursor, held keys and a drag reach the app.
#[test]
fn screen_methods_reach_the_app() {
    // 5: the first frame of an entry
    let s = Screen::build(Owner::RoutesLists)
        .size(82, 12)
        .entry("resume", "loading")
        .early()
        .open();
    s.assert_contains("Loading sessions");

    // 8: the cursor follows the typed draft
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    let before = s.shot();
    s.type_text("draft");
    s.assert_rows_differ(&before);
    let row = s
        .rows()
        .iter()
        .position(|r| r.contains("draft"))
        .expect("the draft row");
    let line = s.rows()[row].clone();
    let col = line[..line.find("draft").expect("the draft column")]
        .chars()
        .count()
        + 5;
    s.assert_cursor(col, row);
    s.assert_text_at(col - 5, row, "draft");
    s.assert_no_link("draft");

    // 9: screen against screen
    let shot = s.shot();
    s.assert_same_rows(&shot);
    s.assert_same_rows_in(&shot, 0..3);
    assert!(fails(|| {
        s.keys("x");
        s.assert_same_rows(&shot);
    }));

    // 9: a held key reaches the composer as a key does; a drag is delivered
    s.key_repeat("y");
    s.assert_contains("draftxy");
    s.drag(2, 3, 12, 5);
    s.mouse_down(2, 3).mouse_drag(8, 3).mouse_up(8, 3);
}

/// Gap 9: a keymap cannot be seeded yet; the test body is written under the
/// kit's own pending.
#[test]
#[should_panic(expected = "PENDING[reuse-carry]: no keymap seed")]
fn a_keymap_seed_is_pending_until_the_port_reads_one() {
    drop(
        Screen::build(Owner::DialogsPane)
            .keymap(&[("list.move_down", "z")])
            .open(),
    );
}

/// Gap 9: `feed` applies more script steps to an open screen.
#[test]
fn feed_applies_more_of_a_script_to_an_open_screen() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!("kit/fixtures/answered.jsonl"))
        .open();
    s.wait_for("• Hello there");
    s.feed(include_str!("kit/fixtures/typed.jsonl"));
    s.assert_contains("fed");
}
