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
/// screen is 84 wide: the list starts two columns in, at screen row 6).
#[test]
fn mixed_backend_resume_table_snapshot() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(84, 22)
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
        .size(84, 22)
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
    // `!fg=` / `!bg=`: the colour is not that one (the `!` was ignored before)
    g.assert_style(0, 0, "!fg=Red");
    g.assert_style(0, 0, "!bg=Rgb(1,2,3)");
    g.assert_style(2, 0, "!fg=Cyan");
    assert!(fails(|| g.assert_style(0, 0, "!fg=Cyan")));
    assert!(fails(|| g.assert_style(2, 0, "!bg=Rgb(1,2,3)")));
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

/// Gap 9: a keymap seed hands the app the bindings (`AppEv::Keymap`, as the
/// host does once it has read the config) before the screen opens, and the
/// screen reads them: here the agents command center, over a debug entry.
/// Without the seed `f6` is no key of the page and `v` moves nothing; with it
/// `f6` opens the search line and `v` steps the cursor. The oracle's own tests
/// set the keys the same way (the fields of its `RuntimeKeymap`).
#[test]
fn a_keymap_seed_remaps_the_keys_of_a_debug_entry() {
    let mut plain = Screen::build(Owner::DialogsPane)
        .size(100, 24)
        .entry("agents", "overview")
        .open();
    let before = plain.shot();
    plain.keys("v f6");
    plain.assert_same_rows(&before);

    let mut seeded = Screen::build(Owner::DialogsPane)
        .size(100, 24)
        .entry("agents", "overview")
        .keymap(&[("agents.search", "f6"), ("list.move_down", "v")])
        .open();
    let before = seeded.shot();
    seeded.keys("v");
    seeded.assert_rows_differ(&before);
    seeded.keys("f6");
    seeded.assert_contains("Search \u{203a}");
}

/// The seed writes the pairs the way the oracle's tests name them: an empty
/// key unbinds the action (the hint of it goes), a chord is a prefix and a
/// completion. The live chat takes the same seed.
#[test]
fn a_keymap_seed_unbinds_and_chords_what_the_page_shows() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(100, 24)
        .entry("agents", "overview")
        .keymap(&[("list.accept", ""), ("agents.search", "ctrl+x s")])
        .open();
    s.assert_absent("enter open");
    s.keys("ctrl+x s");
    s.assert_contains("Search \u{203a}");
    drop(
        Screen::build(Owner::ChatLive)
            .size(80, 24)
            .keymap(&[("app.open_transcript", "ctrl+k")])
            .live()
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

/// A debug entry's screen plays its own fixture; feeding it would silently do
/// nothing, so it says so.
#[test]
#[should_panic(expected = "feed works on a script() or live() screen")]
fn feed_on_an_entry_screen_says_it_cannot() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(84, 22)
        .entry("resume", "table")
        .open();
    s.feed(include_str!("kit/fixtures/typed.jsonl"));
}

/// The help of the agents command center holds the Delete key as the shipped
/// binary prints it (`delete`, crossterm on macOS); the oracle's snapshot
/// holds `backspace` (its `cfg(test)` label), and a longer key column. The
/// kit compares ours as the snapshot has it.
#[test]
fn the_delete_key_label_and_its_column_are_normalized_on_our_rows_only() {
    let oracle: Vec<String> = [
        "  Tasks",
        "  n          New",
        "  a          Archive",
        "  backspace  Delete",
        "",
        "  View",
        "  g  Group",
    ]
    .map(String::from)
    .to_vec();
    let ours: Vec<String> = [
        "  Tasks",
        "  n       New",
        "  a       Archive",
        "  delete  Delete",
        "",
        "  View",
        "  g  Group",
    ]
    .map(String::from)
    .to_vec();
    assert_eq!(kit::normalize_rows(ours.clone(), &oracle), oracle);
    // A real difference stays one: the description, and a row outside the column.
    let mut wrong = ours;
    wrong[2] = "  a       Archived".to_string();
    wrong[6] = "  g   Group".to_string();
    let got = kit::normalize_rows(wrong, &oracle);
    assert_eq!(got[2], "  a          Archived");
    assert_eq!(got[6], "  g   Group");
    // An oracle that holds `delete` itself leaves ours as they are.
    let literal = ["  Tasks".to_string(), "  delete  Delete".to_string()];
    let same = vec!["  delete  Delete".to_string()];
    assert_eq!(kit::normalize_key_names(same.clone(), &literal[1..]), same);
}

/// The same on a real screen: the help of the agents command center at the
/// oracle's width, compared with the oracle's snapshot through the kit.
#[test]
fn the_agents_help_equals_the_oracle_with_the_delete_label() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(60, 24)
        .entry("agents", "navigation")
        .open();
    s.keys("?");
    s.assert_snap("command_center__live_center_help_two_columns");
}

// ---- settle: the kit waits for the backend, never for the clock ----------

/// The session is started and the script's events are applied when `open`
/// returns: no `wait_for` between them and the first assertion.
#[test]
fn a_script_screen_shows_its_events_when_open_returns() {
    let s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!("kit/fixtures/answered.jsonl"))
        .open();
    s.assert_contains("• Hello there");
    s.assert_contains("› Ask Codex to do anything");
}

/// The events of a script reach a session that is already started: the first
/// race of the oracle suite was a script fed while the app still said
/// `starting…`, its events dropped for a thread that did not exist yet.
#[test]
fn a_script_is_fed_to_a_started_session() {
    let s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "oracle/fixtures/renderer_internals/crlf_code.jsonl"
        ))
        .open();
    s.assert_in_order(&["fn main() {", "println!(\"hi\");", "}"]);
}

/// A key that makes the app ask the backend shows the answer when the key
/// returns: Enter submits, the fake starts the turn and echoes the message.
#[test]
fn a_key_that_asks_the_backend_shows_the_answer_when_it_returns() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.assert_contains("› Ask Codex to do anything");
    s.type_text("hello").keys("enter");
    // the echoed message and the started turn, the composer empty again
    s.assert_contains("› hello");
    s.assert_contains("Working");
    s.assert_contains("› Ask Codex to do anything");
}

/// The result does not depend on how fast the machine is: the same screen
/// opened again and again is the same rows.
#[test]
fn opening_a_script_screen_gives_the_same_rows_every_time() {
    let rows = |_: usize| {
        Screen::build(Owner::ChatLive)
            .size(80, 24)
            .script(include_str!("kit/fixtures/answered.jsonl"))
            .open()
            .rows()
    };
    let first = rows(0);
    for n in 1..30 {
        assert_eq!(rows(n), first, "open number {n}");
    }
}

/// The composer's text is read from its rows: the placeholder is "", a draft
/// is its text, a pasted two-line draft is two lines, and a user message in
/// the transcript (`› hi`) is not the composer.
#[test]
fn the_composer_text_is_read_from_its_rows() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!("kit/fixtures/answered.jsonl"))
        .open();
    s.wait_for("• Hello there");
    s.assert_contains("› hi");
    s.assert_composer("");
    s.type_text("draft one");
    s.assert_composer("draft one");
    s.keys(
        "backspace backspace backspace backspace backspace backspace backspace backspace backspace",
    );
    s.assert_composer("");
    s.paste("first\nsecond");
    assert_eq!(s.composer_text(), "first\nsecond");
    s.type_text("/");
    s.assert_composer("first\nsecond/");
}

/// A composer text that differs fails, and a typed text that equals the
/// placeholder is not mistaken for it.
#[test]
fn a_different_composer_text_fails() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.type_text("abc");
    assert!(fails(|| s.assert_composer("abd")));
    assert!(fails(|| s.assert_composer("")));
    s.keys("backspace backspace backspace");
    s.type_text("Ask Codex to do anything");
    s.assert_composer("Ask Codex to do anything");
}

// ---- kit-caps: input a plain xterm cannot send ----

/// A key with no bytes of its own on an xterm reaches the app as the kitty
/// protocol sends it: Shift+Enter in the composer is a new line (the pty kit
/// could not send it), and SUPER+space is a key the composer does not type.
#[test]
fn keys_without_bytes_of_their_own_are_sent_as_kitty_keys() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.type_text("a");
    s.keys("shift+enter");
    s.type_text("b");
    let rows = s.rows();
    let a = rows.iter().position(|r| r.contains("› a")).expect("row a");
    assert!(rows[a + 1].trim_start().starts_with('b'), "{rows:?}");
    s.keys("super+space");
    s.type_text("c");
    s.assert_contains("bc");
}

/// Held and released keys are the kitty protocol's repeat and release events,
/// with the modifiers the key names.
#[test]
fn key_events_are_the_kitty_bytes_of_a_key() {
    use kit::event_bytes_of;
    assert_eq!(event_bytes_of("k", false), "\x1b[107;1:2u");
    assert_eq!(event_bytes_of("super+c", true), "\x1b[99;9:3u");
    assert_eq!(event_bytes_of("alt+shift+left", true), "\x1b[1;4:3D");
    assert_eq!(event_bytes_of("enter", true), "\x1b[13;1:3u");
    assert_eq!(event_bytes_of("delete", true), "\x1b[3;1:3~");
}

/// The released key of a typed char is no input: the draft is as typed (a
/// release the app took for a press would type the char twice).
#[test]
fn a_key_release_is_not_a_press() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.type_text("x");
    s.key_release("x");
    s.key_repeat("y");
    s.key_release("y");
    s.assert_contains("› xy");
    s.assert_absent("xx");
    s.assert_absent("yy");
}

/// The mouse of the oracle's events: the right button, modifiers, a triple
/// click and a bare pointer move are delivered (the effect is the screens').
#[test]
fn the_kit_sends_the_rest_of_the_mouse() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(82, 22)
        .entry("resume", "table")
        .open();
    s.pointer_move(10, 8);
    s.right_click(10, 8);
    s.shift_click(10, 8);
    s.ctrl_click(10, 8);
    s.triple_click(10, 8);
    s.right_down(10, 8).right_up(10, 8);
    s.assert_contains("Fix resume picker timestamps");
}

// ---- kit-caps: what the app asked the backend to do ----

/// The sent turns are readable: the typed text, literal, once; the probe
/// bites on a different text.
#[test]
fn the_probe_reads_the_turns_the_app_started() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "oracle/fixtures/chat_commands_2/session.jsonl"
        ))
        .open();
    s.assert_no_turn();
    s.type_text("**send once**");
    s.keys("enter");
    s.assert_turn_texts(&["**send once**"]);
    s.assert_interrupts(0);
    assert_eq!(s.turn_texts(), vec!["**send once**".to_string()]);
}

#[test]
#[should_panic(expected = "the turns the app started differ")]
fn the_probe_bites_on_a_text_that_was_not_sent() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "oracle/fixtures/chat_commands_2/session.jsonl"
        ))
        .open();
    s.type_text("sent");
    s.keys("enter");
    s.assert_turn_texts(&["not sent"]);
}

/// A key that interrupts a running turn is one interrupt request, counted.
#[test]
fn the_probe_counts_interrupt_requests() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "oracle/fixtures/chat_commands_2/running.jsonl"
        ))
        .open();
    s.assert_interrupts(0);
    s.keys("esc");
    s.assert_interrupts(1);
}

/// A snapshot the oracle joined of several renders under bare labels
/// (`before scroll:` / `after scroll:`) reads as two frames.
#[test]
fn a_snapshot_with_bare_labels_reads_as_frames() {
    assert_eq!(
        Screen::snap_frame_names("list_selection_col_width_mode_auto_all_rows_scroll"),
        vec!["before scroll:".to_string(), "after scroll:".to_string()]
    );
}

// ---- kit-caps: the terminal and the app's environment ----

/// A reply with a quote, code and a link: the cells the theme colours.
const REPLY: &str = "> quoted\n\n`code` and [link](https://example.com)";

fn styles_of(s: &Screen) -> Vec<(String, String)> {
    let g = s.grid();
    ["quoted", "code", "link"]
        .into_iter()
        .map(|text| {
            let (x, _, y) = g
                .locate(text)
                .unwrap_or_else(|| panic!("no `{text}`:\n{}", g.text()));
            let st = g.style(x, y);
            (st.fg, st.bg)
        })
        .collect()
}

/// The terminal's default colours reach the app: a light and a dark
/// terminal paint the same reply differently.
#[test]
fn the_terminal_colours_are_the_apps_theme() {
    let light = Screen::build(Owner::ChatLive)
        .size(60, 12)
        .light()
        .agent_says(REPLY)
        .open();
    let dark = Screen::build(Owner::ChatLive)
        .size(60, 12)
        .dark()
        .agent_says(REPLY)
        .open();
    let (l, d) = (styles_of(&light), styles_of(&dark));
    assert_ne!(l, d, "a light and a dark terminal paint alike: {l:?}");
}

/// The colour depth is the app's: a terminal that shows no colour gets none.
#[test]
fn the_colour_depth_is_the_apps() {
    let full = Screen::build(Owner::ChatLive)
        .size(60, 12)
        .dark()
        .agent_says(REPLY)
        .open();
    let none = Screen::build(Owner::ChatLive)
        .size(60, 12)
        .dark()
        .color_depth(kit::ColorDepth::None)
        .agent_says(REPLY)
        .open();
    let (f, n) = (styles_of(&full), styles_of(&none));
    assert_ne!(f, n, "the depth changed no colour: {f:?}");
}

/// The syntax theme of the config reaches the app as the host's fact: the
/// theme picker opens on it.
#[test]
fn the_syntax_theme_is_the_apps_fact() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .syntax_theme("ansi")
        .live()
        .open();
    s.type_text("/theme").keys("enter");
    s.assert_contains("ansi");
    let with = s.rows();
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.type_text("/theme").keys("enter");
    assert_ne!(
        with,
        s.rows(),
        "the theme fact changed nothing the picker shows"
    );
}

/// A thread another process writes opens as the server answers: the app asks
/// to resume the thread and the server refuses (the seed's answer), which the
/// chat takes as a foreign writer; a chat without the seed asks nothing.
#[test]
fn a_foreign_writer_is_a_refused_resume() {
    let plain = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    let foreign = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .foreign_writer("thread-foreign")
        .live()
        .open();
    assert_eq!(plain.requests_of("thread/resume"), 0);
    assert_eq!(foreign.requests_of("thread/resume"), 1);
}

#[test]
fn a_replay_is_a_resumed_thread_with_recorded_turns() {
    let turns = vec![kit::replay_turn(
        "turn-1",
        serde_json::json!([
            {"type": "userMessage", "id": "u1", "content": [{"type": "text", "text": "hello", "text_elements": []}]},
            {"type": "agentMessage", "id": "a1", "text": "hi there", "phase": null, "memoryCitation": null}
        ]),
    )];
    let s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .replay(turns)
        .live()
        .open();
    assert_eq!(s.requests_of("thread/resume"), 1);
    let asked = s.request_params("thread/resume");
    assert_eq!(asked[0]["threadId"], kit::REPLAYED);
}

// ---- the delta marker (a small visual difference, never a functional one) ----
//
// `kit::delta` softens the kit's comparisons of the screen with the oracle's rows and cell styles,
// per thread (a thread is a test), and only while the rows still say the same words.

const COLLAPSED: &str = "activity_focus_keeps_controls_without_passive_hints";

/// Message of the panic of `f` ("" when it does not panic).
fn panic_of(f: impl FnOnce()) -> String {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(()) => String::new(),
        Err(e) => e
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| e.downcast_ref::<&str>().map(|s| (*s).to_string()))
            .unwrap_or_default(),
    }
}

/// A glyph, an indent and a colour are small: the rows that differ only there pass as a delta,
/// each differing comparison is a hit and a matching one is none.
#[test]
fn delta_passes_a_difference_of_glyph_indent_and_colour() {
    kit::delta(Owner::UiPane, "glyph, indent and dim");
    // the glyph: `!` for the oracle's `⚠`
    let glyph = grid_of(40, 3, &put(&["! 5h 8% left"]));
    kit::assert_snap_on(&glyph, &want("usage_notice_widths", "37 columns"));
    assert_eq!(kit::delta_hits(), 1);
    // the indent: one column in
    let indent = grid_of(40, 3, &put(&[" ⚠ 5h 8% left"]));
    kit::assert_snap_on(&indent, &want("usage_notice_widths", "37 columns"));
    assert_eq!(kit::delta_hits(), 2);
    // the colour: row 1 not dim, the rows as the oracle's
    let plain = grid_of(32, 3, &put(&["Activity summary", "    + Show details"]));
    kit::assert_snap_on(&plain, &want(COLLAPSED, "collapsed"));
    assert_eq!(kit::delta_hits(), 3);
    // a screen that equals the oracle's is no hit
    let same = grid_of(40, 3, &put(&["⚠ 5h 8% left"]));
    kit::assert_snap_on(&same, &want("usage_notice_widths", "37 columns"));
    assert_eq!(kit::delta_hits(), 3);
}

/// Other words are not small: the row that says more, less or else fails the test whatever
/// the marker says, and the failure names the rule.
#[test]
fn delta_refuses_rows_that_say_other_things() {
    kit::delta(Owner::UiPane, "a difference that is not small");
    let more = grid_of(40, 3, &put(&["⚠ 5h limit: 8% left"]));
    let message =
        panic_of(|| kit::assert_snap_on(&more, &want("usage_notice_widths", "37 columns")));
    assert!(message.contains("kit::delta refused"), "{message}");
    // a missing row
    let missing = grid_of(32, 3, &put(&["Activity summary"]));
    let message = panic_of(|| kit::assert_snap_on(&missing, &want(COLLAPSED, "collapsed")));
    assert!(message.contains("kit::delta refused"), "{message}");
    assert_eq!(kit::delta_hits(), 0);
}

/// The row comparisons that are not a snapshot (`assert_rows_are`) are softened alike, with the
/// same refusal.
#[test]
fn delta_covers_the_literal_rows_of_the_oracle_too() {
    kit::delta(Owner::UiPane, "a glyph");
    let ours = vec!["* one".to_string(), "  two".to_string()];
    kit::assert_rows_are(&ours, &["• one", "two"]);
    assert_eq!(kit::delta_hits(), 1);
    assert!(panic_of(|| kit::assert_rows_are(&ours, &["• one", "three"])).contains("refused"));
}

/// The cursor stays strict under a delta: a cursor elsewhere is not a visual delta.
#[test]
fn delta_keeps_the_cursor_strict() {
    kit::delta(Owner::UiPane, "the rows differ by a glyph");
    let name = "history_search_pasted_separators";
    let rows = kit::snap_rows(name, "");
    let refs: Vec<&str> = rows.iter().map(String::as_str).collect();
    let left = grid_of(70, 5, &format!("{}\x1b[5;31H\x1b[?25h", put(&refs)));
    assert!(fails(|| kit::assert_snap_on(&left, &want(name, ""))));
}

/// A delta says in words what differs.
#[test]
#[should_panic(expected = "needs `what`")]
fn delta_needs_what() {
    kit::delta(Owner::UiPane, "  ");
}

/// The marker of one test does not soften another thread's comparison.
#[test]
fn without_delta_the_comparison_still_bites() {
    let glyph = grid_of(40, 3, &put(&["! 5h 8% left"]));
    assert!(fails(|| kit::assert_snap_on(
        &glyph,
        &want("usage_notice_widths", "37 columns")
    )));
    assert_eq!(kit::delta_hits(), 0);
}

/// `delta` softens the comparison of the screen only: a text check of the same test still fails.
#[test]
#[should_panic(expected = "the screen has no `a row the status card never draws`")]
fn delta_never_softens_a_functional_check() {
    kit::delta(Owner::UiPane, "a glyph");
    let g = grid_of(40, 3, &put(&["! 5h 8% left"]));
    kit::assert_snap_on(&g, &want("usage_notice_widths", "37 columns"));
    g.assert_contains("a row the status card never draws");
}
