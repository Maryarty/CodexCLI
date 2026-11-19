//! Screen tests of S18, the resume picker. The oracle's snapshots are
//! fragments of the screen (the list, the footer, the search line, each at its
//! own inset), so each test compares the rows of its region of our screen.
//! Oracle: `codex-rs/tui/src/resume_picker.rs` (`render_list`, `search_line`,
//! `render_picker_footer`) and `resume_picker/layout.rs` (the chrome).

#[allow(clippy::duplicate_mod)] // one helper, a module in each slice file
#[path = "oracle_snap/mod.rs"]
mod oracle_snap;

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
// less its two-cell inset, so the screen is 82 cells wide.
const WIDE_FOR_80_LIST: u16 = 82;
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

// `session_list_more_indicators_snapshot` is pending on G42 in the oracle
// shard (`tests/oracle/resume_2.rs`): the row scroll of rtwk's list cuts the
// third row, and the oracle's rows cannot be had.

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

/// The probe of G41 (`rtwk-gaps.md`): the routes rtwk offers for a chrome row
/// that takes the first spelling that fits. The oracle's compact toolbar at
/// 40 cells is `Filter: All   Status: Active   Sort: Updated`-short on one
/// row; `Wrap` over the full strip breaks it into rows instead, and
/// `KeyHintsBar::from_pairs` reads its keys as a `KeyBindings` does (`ctrl+c`) and
/// panics on the oracle's `⌃c` (its doc), and has no wide/compact labels.
#[test]
fn g39_probe_wrap_is_not_first_fit() {
    use rtwk::layout::Wrap;
    use rtwk::text::Text;
    let strip = [
        "Filter: ",
        " All ",
        "   ",
        "Status: ",
        " Active ",
        " Archived ",
        "   ",
        "Sort: ",
        " Updated ",
        " Created ",
    ];
    let mut wrap = Wrap::new().spacing(0).run_spacing(0);
    for part in strip {
        wrap = wrap.child(Text::new(part));
    }
    let mut t = rtwk_test::Tester::with(rtwk_test::Options::new().size(40, 4));
    t.pump_widget(wrap);
    let rows = t
        .screen()
        .lines()
        .filter(|row| !row.trim().is_empty())
        .count();
    assert!(
        rows > 1,
        "Wrap puts the strip on {rows} rows; the oracle keeps one"
    );
}
