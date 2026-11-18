//! Screen tests of S21, the agents command center. Oracle:
//! `codex-rs/tui/src/app/agents_overview_tests.rs` (the snapshots under
//! `snapshots/codex_tui__app__agents_overview__tests__*.snap`).

#[allow(clippy::duplicate_mod)] // one helper, a module in each slice file
#[path = "oracle_snap/mod.rs"]
mod oracle_snap;

use crate::common::{Harness, start_sized};

fn snap(name: &str) -> String {
    format!("snapshots/codex_tui__app__agents_overview__tests__{name}.snap")
}

fn open(entry: &str, width: u16, height: u16) -> Harness {
    let mut h = start_sized(true, width, height);
    h.open_debug_row("agents", Some(entry));
    h
}

/// The oracle snapshot's rows and the height they were taken at (blank rows
/// to the footer included: the footer is its last row).
fn oracle(name: &str) -> (Vec<String>, u16) {
    let rows = oracle_snap::rows(&snap(name));
    let height = rows.len() as u16;
    (rows, height)
}

#[test]
fn shared_overview_shows_only_root_sessions() {
    let (expected, height) = oracle("agents_overview");
    let h = open("overview", 96, height);
    oracle_snap::assert_same("agents_overview", &oracle_snap::screen(&h.t), &expected);
}

#[test]
fn agents_overview_details_show_available_attention_without_expanding_rows() {
    let (expected, height) = oracle("agents_overview_attention");
    let h = open("attention", 96, height);
    oracle_snap::assert_same(
        "agents_overview_attention",
        &oracle_snap::screen(&h.t),
        &expected,
    );
}

#[test]
fn empty_command_center_can_open_resume_picker() {
    let (expected, height) = oracle("agents_overview_empty_narrow");
    let h = open("empty_narrow", 48, height);
    oracle_snap::assert_same(
        "agents_overview_empty_narrow",
        &oracle_snap::screen(&h.t),
        &expected,
    );
}

/// Down moves the highlight and the details follow it.
#[test]
fn down_moves_the_highlight_and_the_details() {
    let (_, height) = oracle("agents_overview");
    let mut h = open("overview", 96, height);
    h.press("down");
    let screen = h.t.screen();
    assert!(screen.contains("› ● Repair authentication"), "{screen}");
    assert!(screen.contains("Task details"), "{screen}");
    let details: Vec<&str> = screen
        .lines()
        .filter_map(|l| l.split_once("│ ").map(|(_, r)| r.trim_end()))
        .collect();
    assert!(details.contains(&"Repair authentication"), "{details:?}");
    assert!(details.contains(&"● Working"), "{details:?}");
}
