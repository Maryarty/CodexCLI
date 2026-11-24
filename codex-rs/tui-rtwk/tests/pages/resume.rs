//! Screen tests of S18, the resume picker. The oracle's snapshots are
//! fragments of the screen (the list, the footer, the search line, each at its
//! own inset), so each test compares the rows of its region of our screen.
//! Oracle: `codex-rs/tui/src/resume_picker.rs` (`render_list`, `search_line`,
//! `render_picker_footer`) and `resume_picker/layout.rs` (the chrome).

#[allow(clippy::duplicate_mod)] // one helper, a module in each slice file
#[path = "../oracle_snap/mod.rs"]
mod oracle_snap;

use codex_tui_core::app::Now;
use codex_tui_rtwk::services::shell::FixedClock;

use crate::common::{Harness, start_sized};

const SNAPS: &str = "snapshots/codex_tui__resume_picker__tests__";

fn snap(name: &str) -> String {
    format!("{SNAPS}{name}.snap")
}

fn open(entry: &str, width: u16, height: u16) -> Harness {
    let mut h = start_sized(true, width, height);
    h.open_debug_row("resume", Some(entry));
    h
}

/// The oracle's fragment shifted right by the region's inset.
fn inset(rows: Vec<String>, by: usize) -> Vec<String> {
    rows.into_iter()
        .map(|row| {
            if row.is_empty() {
                row
            } else {
                format!("{}{row}", " ".repeat(by))
            }
        })
        .collect()
}

// The chrome of `layout::areas`: header, toolbar, search, list, footer (4 rows)
// with up to three one-row gaps while the list keeps six rows.
const LIST_AT_22: usize = 6;
// The oracle renders the list into an 80-cell area; our list is the screen
// less its two-cell inset on each side (`PICKER_LIST_HORIZONTAL_INSET` is 4),
// so the screen is 84 cells wide.
const WIDE_FOR_80_LIST: u16 = 84;
const LIST_AT_13: usize = 3;

#[test]
fn mixed_backend_resume_table_snapshot() {
    let mut h = open("table", WIDE_FOR_80_LIST, 22);
    // The oracle's test selects the second row (`state.selected = 1`).
    h.press("down");
    oracle_snap::assert_fragment(
        "resume_picker_table",
        &oracle_snap::screen(&h.t),
        LIST_AT_22,
        &inset(oracle_snap::rows(&snap("resume_picker_table")), 2),
    );
}

// `session_list_more_indicators_snapshot` is in the oracle shard
// (`tests/oracle/resume_2.rs`, G42 not a gap: `ListController::jump_to_index`
// by the oracle's window rule), at 82 cells for the oracle's 80-cell list.

#[test]
fn resume_search_error_snapshot() {
    let h = open("search_error", 80, 13);
    oracle_snap::assert_fragment(
        "resume_picker_search_error",
        &oracle_snap::screen(&h.t),
        2,
        &inset(oracle_snap::rows(&snap("resume_picker_search_error")), 1),
    );
}

#[test]
fn hint_line_snapshot_uses_distributed_wide_footer() {
    let h = open("footer_wide", 220, 22);
    let screen = oracle_snap::screen(&h.t);
    oracle_snap::assert_fragment(
        "resume_picker_footer_wide",
        &screen,
        18,
        &oracle_snap::rows(&snap("resume_picker_footer_wide")),
    );
}

/// `render_empty_state_line` for the first load.
#[test]
fn loading_sessions_line() {
    let h = open("loading", 80, 13);
    let screen = oracle_snap::screen(&h.t);
    assert_eq!(screen[LIST_AT_13], "  Loading sessions…", "{screen:?}");
}

#[test]
fn no_sessions_yet_line() {
    let h = open("empty", 80, 13);
    let screen = oracle_snap::screen(&h.t);
    assert_eq!(screen[LIST_AT_13], "  No sessions yet", "{screen:?}");
}

/// Down moves the selection marker one row (three screen rows) and up moves
/// it back (oracle: `PickerState::move_selection`).
#[test]
fn down_and_up_move_the_marker() {
    let mut h = open("table", WIDE_FOR_80_LIST, 22);
    let first = oracle_snap::screen(&h.t);
    assert!(first[LIST_AT_22].starts_with("  › "), "{first:?}");
    h.press("down");
    let second = oracle_snap::screen(&h.t);
    assert!(second[LIST_AT_22 + 3].starts_with("  › "), "{second:?}");
    assert!(!second[LIST_AT_22].contains('›'), "{second:?}");
    h.press("up");
    assert!(oracle_snap::screen(&h.t)[LIST_AT_22].starts_with("  › "));
}

/// ADR-011 (beyond the oracle): a click on a row puts the cursor there, a
/// click on the row the cursor is on opens it, the wheel scrolls the list.
#[test]
fn a_click_selects_the_row_and_a_second_click_opens_it() {
    let mut h = open("table", WIDE_FOR_80_LIST, 22);
    let before = oracle_snap::screen(&h.t);
    assert!(before[LIST_AT_22].starts_with("  › "), "{before:?}");
    let y = u16::try_from(LIST_AT_22 + 3).unwrap();
    h.t.click(8, y);
    h.settle();
    let after = oracle_snap::screen(&h.t);
    assert!(after[LIST_AT_22 + 3].starts_with("  › "), "{after:?}");
    assert!(!after[LIST_AT_22].contains('›'), "{after:?}");
    h.t.click(8, y);
    h.settle();
    h.t.pump_for(crate::common::ms(800));
    let gone = h.t.screen();
    assert!(
        !gone.contains("Resume a previous session"),
        "a click on the selected row returns its choice:\n{gone}"
    );
}

#[test]
fn the_wheel_scrolls_the_resume_list() {
    let mut h = open("table", WIDE_FOR_80_LIST, 12);
    let y = u16::try_from(LIST_AT_22).unwrap();
    assert!(!h.t.screen().contains("↑ more"), "{}", h.t.screen());
    h.t.mouse(
        rtwk_core::input::MouseKind::WheelDown,
        8,
        y,
        0,
        rtwk_core::input::Mods::NONE,
    );
    h.settle();
    assert!(h.t.screen().contains("↑ more"), "{}", h.t.screen());
}

// ---- the oracle's joined snapshots of `resume_picker_color_tests.rs`: one screen per action and
// size under a header line (`Resume 118x30`), which the kit's snapshot sections do not name; the
// frames are split here (the oracle shard tests of the same names are pending on that).

const COLOR_SNAPS: &str = "snapshots/codex_tui__resume_picker__color_tests__";

/// The frames of a joined snapshot: the header line (`Resume 118x30`, or with the focused
/// control after it) and the rows under it up to the next header, trailing blank rows cut.
fn joined(name: &str) -> Vec<(String, Vec<String>)> {
    let size_of = |row: &str| -> bool {
        let mut words = row.split(' ');
        let (action, size) = (words.next(), words.next());
        matches!(action, Some("Resume" | "Fork"))
            && size
                .and_then(|size| size.split_once('x'))
                .is_some_and(|(w, h)| w.parse::<u16>().is_ok() && h.parse::<u16>().is_ok())
    };
    let mut frames: Vec<(String, Vec<String>)> = Vec::new();
    for row in oracle_snap::rows(&format!("{COLOR_SNAPS}{name}.snap")) {
        if size_of(&row) {
            frames.push((row, Vec::new()));
        } else if let Some((_, rows)) = frames.last_mut() {
            rows.push(row);
        }
    }
    for (_, rows) in &mut frames {
        while rows.last().is_some_and(String::is_empty) {
            rows.pop();
        }
    }
    frames
}

/// Oracle: `compact_picker_keeps_metadata_and_labeled_primary_actions`, the screen of each action
/// and size equals the snapshot's frame; the progress mark is the only `%` on it.
#[test]
fn compact_picker_keeps_metadata_and_labeled_primary_actions() {
    let frames = joined("compact_picker_keeps_metadata_and_labeled_primary_actions");
    assert_eq!(frames.len(), 4, "{frames:?}");
    for (header, expected) in frames {
        let mut words = header.split(' ');
        let (action, size) = (words.next().unwrap(), words.next().unwrap());
        let (w, h) = size.split_once('x').unwrap();
        let entry = if action == "Fork" {
            "compact_six_rows_fork"
        } else {
            "compact_six_rows"
        };
        let h = open(entry, w.parse().unwrap(), h.parse().unwrap());
        let screen = oracle_snap::screen(&h.t);
        assert_eq!(
            screen
                .iter()
                .map(|row| row.matches('%').count())
                .sum::<usize>(),
            1,
            "{header}: {screen:?}"
        );
        oracle_snap::assert_same(&header, &screen, &expected);
    }
}

/// Oracle: `narrow_toolbar_keeps_keyboard_focused_control_visible`: each control focused in turn
/// (tab) stays on screen in its narrow spelling at 38x12 and 24x9, in the resume picker (on the
/// archived status) and the fork picker.
#[test]
fn narrow_toolbar_keeps_keyboard_focused_control_visible() {
    let frames = joined("narrow_toolbar_keeps_keyboard_focused_control_visible");
    assert_eq!(frames.len(), 10, "{frames:?}");
    for (action, entry) in [
        ("Resume", "toolbar_focus_narrow"),
        ("Fork", "toolbar_focus_narrow_fork"),
    ] {
        for (width, height) in [(38u16, 12u16), (24, 9)] {
            let mut h = open(entry, width, height);
            if action == "Resume" {
                // The oracle's picker starts on the archived status with the first control
                // focused: the status moves to Archived under the focus, which goes round.
                h.press("tab");
                h.press("right");
                h.press("tab");
                h.press("tab");
            }
            let prefix = format!("{action} {width}x{height} ");
            for (header, expected) in frames
                .iter()
                .filter(|(header, _)| header.starts_with(&prefix))
            {
                let label = &header[prefix.len()..];
                let screen = oracle_snap::screen(&h.t);
                let row = screen
                    .iter()
                    .find(|row| row.contains(label))
                    .unwrap_or_else(|| panic!("{header}: no row has `{label}`: {screen:?}"));
                oracle_snap::assert_same(header, std::slice::from_ref(row), expected);
                h.press("tab");
            }
        }
    }
}

// ---- the live picker: what an entry screen has no server for.

fn thread_json(id: &str, preview: &str, created: u64, updated: u64) -> serde_json::Value {
    serde_json::json!({
        "id": id, "environments": null, "extra": null, "sessionId": id,
        "forkedFromId": null, "attachedToThreadId": null, "parentThreadId": null,
        "preview": preview, "ephemeral": false, "section": null, "sectionEnteredAt": null,
        "projectId": null, "historyMode": "legacy", "modelProvider": "openai",
        "model": null, "reasoningEffort": null, "createdAt": created,
        "updatedAt": updated, "recencyAt": updated, "status": {"type": "idle"},
        "path": null, "cwd": "/tmp/codex", "cliVersion": "0.0.0", "originator": null,
        "source": "cli", "canAcceptDirectInput": true, "threadSource": null,
        "agentNickname": null, "agentRole": null,
        "gitInfo": {"sha": null, "branch": "main", "originUrl": null},
        "name": null, "daybreakEnabled": null, "turns": []
    })
}

/// Oracle: `footer_prioritizes_active_sort_timestamp` (resume_picker.rs): the row's meta line
/// leads with the time of the sort in force. The Updated half is the oracle shard's (an entry);
/// the Created half needs the reload a sort change starts, which only a live picker has a server
/// for: the fake backend answers `thread/list` and the sort control moves to Created.
#[test]
fn the_meta_line_leads_with_the_created_time_once_the_sort_is_created() {
    // The clock is a model input (R11): a fixed time, not the machine's.
    let now: u64 = 1_777_399_200;
    let (created, updated) = (now - 5 * 3600, now - 3 * 3600);
    // The shell tells the app this time at its start (and the link has no other: the one timer
    // asks a minute ahead), so no injected event can lose to the shell's own first clock.
    let fixed = FixedClock::at(Now::at_secs(i64::try_from(now).unwrap_or(0)));
    let mut h = crate::common::start_app(|app| app.with_start_resume(true).with_clock(fixed));
    h.fake.respond(move |request| {
        (request["method"] == "thread/list").then(|| {
            Ok(serde_json::json!({
                "data": [thread_json(
                    "00000000-0000-0000-0000-000000000001",
                    "Selected session",
                    created,
                    updated,
                )],
                "nextCursor": null,
            }))
        })
    });
    // The first listing went out before the fake could answer: a sort change lists again.
    h.press("tab");
    h.press("tab");
    h.press("right");
    h.settle();
    h.t.pump_for(crate::common::ms(400));
    let screen = oracle_snap::screen(&h.t);
    let meta = screen
        .iter()
        .find(|row| row.contains("⌁ "))
        .unwrap_or_else(|| panic!("no meta line: {screen:?}"));
    assert!(meta.trim_start().starts_with("5h ago"), "{screen:?}");
    assert!(!meta.contains("3h ago"), "{screen:?}");
}

// ---- a tap on a toolbar value (ADR-011): the value is picked, as the arrows would.

/// The screen's rows and where `word` stands on the toolbar row (the one with "Sort"): the
/// row and the column of its middle.
fn on_toolbar(h: &Harness, word: &str) -> (u16, u16) {
    let rows = oracle_snap::screen(&h.t);
    let y = rows
        .iter()
        .position(|row| row.contains("Sort"))
        .unwrap_or_else(|| panic!("no toolbar row: {rows:?}"));
    let x = rows[y]
        .find(word)
        .unwrap_or_else(|| panic!("no {word:?} on the toolbar: {}", rows[y]));
    (
        u16::try_from(x + word.len() / 2).unwrap_or(u16::MAX),
        u16::try_from(y).unwrap_or(u16::MAX),
    )
}

#[test]
fn a_tap_on_a_status_value_picks_it() {
    let mut h = open("toolbar_repo_cwd", 102, 24);
    let rows = oracle_snap::screen(&h.t);
    assert!(
        rows.iter().any(|row| row.contains("enter resume")),
        "{rows:?}"
    );
    let (x, y) = on_toolbar(&h, "Archived");
    h.t.click(x, y);
    h.settle();
    let rows = oracle_snap::screen(&h.t);
    // As the right arrow on the status control: archived sessions, the footer offers restore.
    assert!(
        rows.iter().any(|row| row.contains("enter restore")),
        "{rows:?}"
    );
    assert!(
        rows.iter().any(|row| row.contains("Loading sessions")),
        "{rows:?}"
    );
}

/// The style of the toolbar cell where `word` stands.
fn toolbar_style(h: &Harness, word: &str) -> String {
    let (x, y) = on_toolbar(h, word);
    rtwk_test::style_of(&h.t.cell(usize::from(x), usize::from(y)))
}

#[test]
fn a_tap_on_the_sort_value_picks_it_and_the_control_takes_the_focus() {
    let mut h = open("toolbar_repo_cwd", 102, 24);
    // The filter has the focus: its value in force wears the tab style, the
    // sort value in force the plain one, the other one the dim one.
    let focused = toolbar_style(&h, "Cwd");
    let in_force = toolbar_style(&h, "Updated");
    let other = toolbar_style(&h, "Created");
    assert_ne!(focused, in_force);
    assert_ne!(focused, other);
    let (x, y) = on_toolbar(&h, "Created");
    h.t.click(x, y);
    h.settle();
    // The tapped value is the one in force, and its control has the focus now.
    assert_eq!(toolbar_style(&h, "Created"), focused);
    assert_eq!(toolbar_style(&h, "Updated"), other);
    assert_eq!(toolbar_style(&h, "Cwd"), in_force);
    let rows = oracle_snap::screen(&h.t);
    assert!(
        rows.iter().any(|row| row.contains("Loading sessions")),
        "the list restarts under the new sort: {rows:?}"
    );
    // A tap on the value in force changes nothing.
    let (x, y) = on_toolbar(&h, "Created");
    h.t.click(x, y);
    h.settle();
    assert_eq!(toolbar_style(&h, "Created"), focused);
}

#[test]
fn a_tap_on_a_compact_value_moves_to_the_next_one() {
    let mut h = open("toolbar_repo_cwd", 60, 24);
    // The compact strip names the value in force of each control; the
    // filter has the focus.
    let focused = toolbar_style(&h, "Cwd");
    let (x, y) = on_toolbar(&h, "Updated");
    h.t.click(x, y);
    h.settle();
    let rows = oracle_snap::screen(&h.t);
    assert!(
        rows.iter()
            .any(|row| row.contains("Sort") && row.contains("Created")),
        "{rows:?}"
    );
    assert_eq!(
        toolbar_style(&h, "Created"),
        focused,
        "the sort has the focus"
    );
    // The filter's lone value taps to its next value: the repository's checkouts.
    let (x, y) = on_toolbar(&h, "Cwd");
    h.t.click(x, y);
    h.settle();
    let rows = oracle_snap::screen(&h.t);
    assert!(
        rows.iter()
            .any(|row| row.contains("Filter") && row.contains("Repo")),
        "{rows:?}"
    );
    assert_eq!(
        toolbar_style(&h, "Repo"),
        focused,
        "the filter has the focus"
    );
}

/// The list actions of the keymap move the cursor (oracle: `self.keymap.list`):
/// ctrl+n is a default of `move_down`; a plain `j`, though bound to it too, is
/// typed into the search.
#[test]
fn list_actions_of_the_keymap_move_and_plain_characters_type() {
    let mut h = open("table", WIDE_FOR_80_LIST, 22);
    h.press("ctrl+n");
    let moved = oracle_snap::screen(&h.t);
    assert!(moved[LIST_AT_22 + 3].starts_with("  › "), "{moved:?}");
    h.press("j");
    let typed = oracle_snap::screen(&h.t);
    assert!(
        typed
            .iter()
            .any(|row| row.contains("Search") && row.contains('j')),
        "{typed:?}"
    );
}

/// A rebound list action is the picker's key: `move_down` on ctrl+x.
#[test]
fn a_rebound_move_down_moves_the_cursor() {
    let mut h = start_sized(true, WIDE_FOR_80_LIST, 22);
    h.set_keymap(&[("list.move_down", "ctrl+x")]);
    h.open_debug_row("resume", Some("table"));
    h.press("ctrl+x");
    let moved = oracle_snap::screen(&h.t);
    assert!(moved[LIST_AT_22 + 3].starts_with("  › "), "{moved:?}");
    h.press("down");
    let still = oracle_snap::screen(&h.t);
    assert!(still[LIST_AT_22 + 3].starts_with("  › "), "{still:?}");
}

/// Ctrl+E opens the row's details and the same key closes them; ctrl+O makes
/// the rows dense and back (both the page's own state).
#[test]
fn ctrl_e_and_ctrl_o_are_the_pages_own_state() {
    let mut h = open("expanded_session", 100, 24);
    let closed = oracle_snap::screen(&h.t);
    h.press("ctrl+e");
    let open_rows = oracle_snap::screen(&h.t);
    assert_ne!(closed, open_rows, "the details are drawn");
    h.press("ctrl+e");
    assert_eq!(oracle_snap::screen(&h.t), closed, "the details are closed");
    h.press("ctrl+o");
    let dense = oracle_snap::screen(&h.t);
    assert_ne!(dense, closed, "the rows are dense");
    h.press("ctrl+o");
    assert_eq!(oracle_snap::screen(&h.t), closed, "comfortable again");
}
