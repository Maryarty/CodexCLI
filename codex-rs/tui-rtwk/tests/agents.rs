//! Screen tests of S21, the agents command center. Oracle:
//! `codex-rs/tui/src/app/agents_overview_tests.rs` (the snapshots under
//! `snapshots/codex_tui__app__agents_overview__tests__*.snap`).

#[allow(clippy::duplicate_mod)] // one helper, a module in each slice file
#[path = "oracle_snap/mod.rs"]
mod oracle_snap;

use crate::common::{Harness, start_sized};

/// Who makes a pending test pass (tm aliases of the slices).
#[derive(Clone, Copy)]
enum Owner {
    Keymap,
    CoreSeed,
}

impl Owner {
    fn slice(self) -> &'static str {
        match self {
            Owner::Keymap => "keymap",
            Owner::CoreSeed => "agents-live-shell",
        }
    }
}

/// The screen, the state or the seed the test needs does not exist yet: the
/// marker fails the test with the reason and the owner; under
/// `KIT_PROBE_PENDING=1` the body below it runs (a stale marker shows).
fn pending(owner: Owner, reason: &str) {
    let reason = reason.split_whitespace().collect::<Vec<_>>().join(" ");
    if std::env::var_os("KIT_PROBE_PENDING").is_some_and(|v| v != "0") {
        println!("PROBE[{}]: {reason}", owner.slice());
        return;
    }
    panic!("PENDING[{}]: {reason}", owner.slice());
}

fn snap(name: &str) -> String {
    format!("snapshots/codex_tui__app__agents_overview__tests__{name}.snap")
}

fn app_snap(name: &str) -> String {
    format!("app/snapshots/codex_tui__app__agents_overview__tests__{name}.snap")
}

/// The oracle's rows of an `app/snapshots` snapshot.
fn app_oracle(name: &str) -> Vec<String> {
    oracle_snap::rows(&app_snap(name))
}

/// The screen compared with an `app/snapshots` snapshot (recorded on macOS:
/// the footer reads `tab/⇧tab`, as the view writes it there).
fn assert_app(h: &Harness, name: &str) {
    let actual: Vec<String> = oracle_snap::screen(&h.t).iter().map(|row| as_snapshot_cells(row)).collect();
    oracle_snap::assert_same(name, &actual, &app_oracle(name));
}

/// The oracle's snapshot writer prints the empty second cell of a wide
/// character as a space (`日 本 語`); the screen's text has none. A
/// comparison-only normalization of our rows to the snapshot's form.
fn as_snapshot_cells(row: &str) -> String {
    let wide = |c: char| ('\u{2e80}'..='\u{ffff}').contains(&c) && c != '\u{2022}';
    let mut out = String::new();
    for c in row.chars() {
        out.push(c);
        if wide(c) {
            out.push(' ');
        }
    }
    out.trim_end().to_owned()
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

/// The footer's `⇧tab` (one column on darwin) written as the Linux snapshot
/// writes it, `shift+tab` (six columns). The footer is right-aligned, so the
/// five columns the label gains come out of the padding before it. A
/// comparison-only normalization: the expectation text is the snapshot's.
fn shift_label_as_linux(row: &str) -> String {
    let Some(at) = row.find("tab/⇧tab") else {
        return row.to_owned();
    };
    let head = &row[..at];
    let body = head.trim_end_matches(' ');
    let pad = (head.chars().count() - body.chars().count())
        .saturating_sub("shift+".chars().count() - "⇧".chars().count());
    format!(
        "{body}{}tab/shift+tab{}",
        " ".repeat(pad),
        &row[at + "tab/⇧tab".len()..]
    )
}

/// `daemon_startup__agents_overview_before_folder_consent` (cli snapshot): the
/// first rows are the empty center at 120 columns (the consent dialog below
/// them belongs to the daemon startup flow, not to this screen). The snapshot
/// was recorded on Linux, where the tab key reads `shift+tab`; on darwin the
/// view writes the `⇧` glyph, normalized in the actual rows only.
#[test]
fn agents_overview_before_folder_consent_empty_center_rows() {
    let rel =
        "../../cli/tests/snapshots/daemon_startup__agents_overview_before_folder_consent.snap";
    let expected = oracle_snap::rows(rel);
    let h = open("empty_narrow", 120, 58);
    let actual: Vec<String> = oracle_snap::screen(&h.t)
        .into_iter()
        .map(|row| shift_label_as_linux(&row))
        .collect();
    oracle_snap::assert_fragment(
        "agents_overview_before_folder_consent",
        &actual,
        0,
        &expected[..4],
    );
}

// The oracle's screen tests that need a state the debug rows do not give yet.
// Each is a todo! with its reason; the debug entries are registered in
// `src/screens/agents/mod.rs` (agents-live's file), so the fixture scripts for
// them are the handover, not this slice.

/// `overview_worktree_creation_busy_state` (agents_overview_tests.rs:25).
#[test]
fn overview_worktree_creation_busy_state() {
    todo!(
        "agents-live: needs a debug row with a worktree being created (the busy notice); not registered (mod.rs:36)"
    )
}

/// `overview_thread_colors_match_footer_and_respect_color_suppression` (agents_overview_tests.rs:51).
#[test]
fn overview_thread_colors_match_footer_and_respect_color_suppression() {
    todo!(
        "agents-live: needs a debug row with threads of each status and color suppression (mod.rs:36)"
    )
}

/// `older_server_notice_is_visible_in_agents_overview` (agents_overview_tests.rs:100).
#[test]
fn older_server_notice_is_visible_in_agents_overview() {
    todo!(
        "agents-live: needs a debug row with the older-server notice (Notice::ServerVersion); not registered (mod.rs:36)"
    )
}

/// `older_server_notice_truncates_in_narrow_overview` (agents_overview_tests.rs:127, snap older_server_narrow_overview).
#[test]
fn older_server_notice_truncates_in_narrow_overview() {
    todo!(
        "agents-live: needs the older-server notice at a narrow width (snap older_server_narrow_overview); not registered (mod.rs:36)"
    )
}

/// `agents_overview_details_render_markdown` (agents_overview_tests.rs:1138).
#[test]
fn agents_overview_details_render_markdown() {
    todo!(
        "agents-live: needs a debug row whose details carry markdown (rtwk-rich markdown in details); not registered (mod.rs:36)"
    )
}

/// `agents_overview_preview_links_survive_wrapping_and_clipping` (agents_overview_tests.rs:1221).
#[test]
fn agents_overview_preview_links_survive_wrapping_and_clipping() {
    todo!(
        "agents-live: needs a debug row with preview links at a clipping width; not registered (mod.rs:36)"
    )
}

/// `agents_overview_markdown_preview_preserves_layout_and_bounds` (agents_overview_tests.rs:1271).
#[test]
fn agents_overview_markdown_preview_preserves_layout_and_bounds() {
    let text = format!("a\r\n\t\u{1b}{}", "界".repeat(600));
    assert_eq!(
        codex_tui_core::screens::agents::preview_markdown(&text),
        format!("a\n\t{}", "界".repeat(509))
    );
}

/// `filtered_dashboard_actions_use_configured_shortcuts` (agents_overview_tests.rs:1753).
/// Its filter and keys go through the rtwk list (screen level); its F-keys need
/// the configured keymap, and `Fork` has no intent yet.
#[test]
fn filtered_dashboard_actions_use_configured_shortcuts() {
    pending(
        Owner::Keymap,
        "the configured keymap (`agents.search/stop/resume`, `list.cancel/move_down`) does not exist in tui-rtwk; the screen's keys are the defaults (mod.rs `intent_of`)",
    );
}

/// `command_center_cursor_tracks_fixed_footer` (agents_overview_tests.rs:2472).
#[test]
fn command_center_cursor_tracks_fixed_footer() {
    todo!(
        "agents-live: needs the rtwk list's cursor with the fixed footer at screen level; no debug row for a long list (mod.rs:36)"
    )
}

/// `archive_confirmation_number_keys_act_immediately` (agents_overview_actions_tests.rs:185),
/// with the confirmation snapshot archive_task_confirmation: the dialog is
/// asked with `a`; `1` (Cancel) closes it with no action, `2` closes it and
/// starts the archive (the center then holds back its keys: the removing
/// notice).
#[test]
fn archive_confirmation_number_keys_act_immediately() {
    let mut h = open("confirm", 80, 24);
    h.press("a");
    h.settle();
    assert_app(&h, "actions__archive_task_confirmation");
    h.press("1");
    h.settle();
    let screen = h.t.screen();
    assert!(!screen.contains("Archive \u{201c}"), "{screen}");
    assert!(!screen.contains("Archiving"), "{screen}");
    h.press("a");
    h.settle();
    h.press("2");
    h.settle();
    let screen = h.t.screen();
    assert!(!screen.contains("Archive \u{201c}"), "{screen}");
    assert!(screen.contains("Archiving"), "{screen}");
}

/// The delete confirmation (delete_task_confirmation) of
/// `lifecycle_removes_background_and_current_tasks_without_losing_the_dashboard`;
/// its row waits for Enter (`require_explicit_confirmation`): the digit only
/// moves the cursor to it. Its failure (delete_task_failure) is a backend
/// answer of the live server.
#[test]
fn delete_task_confirmation_and_failure_snapshots() {
    let mut h = open("confirm", 80, 24);
    h.press("backspace");
    h.settle();
    assert_app(&h, "actions__delete_task_confirmation");
    h.press("2");
    h.settle();
    let screen = h.t.screen();
    assert!(screen.contains("\u{203a} 2. Permanently delete task and child agents"), "{screen}");
    h.press("enter");
    h.settle();
    let screen = h.t.screen();
    assert!(!screen.contains("Permanently delete \u{201c}"), "{screen}");
    assert!(screen.contains("Deleting"), "{screen}");
}

/// `modal_consumes_input_and_finishes_after_input_closes` (agents_overview_action_progress_tests.rs:17),
/// snapshots archive_progress and delete_progress.
#[test]
fn modal_consumes_input_and_finishes_after_input_closes() {
    todo!(
        "agents-live: needs a debug row with an action in progress (archive_progress, delete_progress); not registered (mod.rs:36)"
    )
}

/// `live_center_columns` (agent_center_tests.rs:294).
#[test]
fn live_center_columns() {
    todo!(
        "agents-live: needs the live-center fixture (columns) through a debug row; not registered (mod.rs:36)"
    )
}

/// `live_center_pages_visible_rows_without_wrapping` (agent_center_tests.rs:387).
#[test]
fn live_center_pages_visible_rows_without_wrapping() {
    todo!(
        "agents-live: needs the rtwk list's paging at screen level and a long fixture; not registered (mod.rs:36)"
    )
}

/// `live_center_metadata_clips_at_grapheme_boundaries` (agent_center_tests.rs:449).
#[test]
fn live_center_metadata_clips_at_grapheme_boundaries() {
    let mut h = open("clip", 40, 18);
    let input = format!("{}日本語 e\u{301} 👩\u{200d}💻", "界".repeat(100_000));
    for (key, label) in [("r", "Rename › "), ("/", "Search › ")] {
        h.press(key);
        h.t.paste(&input);
        h.settle();
        let rendered = h.t.screen();
        let prompt = rendered.lines().find(|line| line.contains(label)).unwrap();
        assert!(prompt.contains("e\u{301}"), "{rendered}");
        assert!(prompt.trim_end().ends_with("👩\u{200d}💻"), "{rendered}");
        h.t.assert_cursor(36, 3);
        if key == "r" {
            // The oracle snapshots the 40-column render, then asks for the
            // cursor of a 24-column area.
            assert_app(&h, "command_center__live_center_narrow_rename");
            h.t.resize(24, 18);
            h.settle();
            let (x, _) = h.t.cursor();
            assert!(x < 24, "cursor at {x}");
            h.t.resize(40, 18);
            h.settle();
        }
        h.press("esc");
    }
}

/// `live_center_navigation_and_complete_hints` (agent_center_tests.rs:588).
/// Navigation: screen level through the rtwk list (scope change, orchestrator).
#[test]
fn live_center_navigation_and_complete_hints() {
    let mut h = open("navigation", 40, 12);
    h.press("tab");
    assert!(h.t.screen().contains("› ! Needs input"), "{}", h.t.screen());
    h.press("shift+tab");
    h.press("shift+tab");
    assert!(
        h.t.screen().contains("› ○ Unloaded task"),
        "{}",
        h.t.screen()
    );
    assert_app(&h, "command_center__live_center_status_filter");
    h.press("tab");
    h.press("?");
    for (name, width, height) in [
        ("live_center_help_three_columns", 100, 24),
        ("live_center_help_two_columns", 60, 24),
        ("live_center_help_one_column", 40, 32),
        ("live_center_navigation_and_complete_hints", 40, 24),
    ] {
        h.t.resize(width, height);
        h.settle();
        assert_app(&h, &format!("command_center__{name}"));
    }
    h.press("esc");
    assert!(h.t.screen().contains("Tasks"), "{}", h.t.screen());
    h.press("esc");
    assert!(
        !h.t.screen().contains("Agent command center"),
        "{}",
        h.t.screen()
    );
}

/// `live_center_fixed_shortcuts_yield_to_configured_actions` (agent_center_tests.rs:649).
#[test]
fn live_center_fixed_shortcuts_yield_to_configured_actions() {
    pending(
        Owner::Keymap,
        "the configured keymap (`agents.*` overriding the fixed shortcuts) does not exist in tui-rtwk",
    );
}

/// `live_center_hints_follow_configured_bindings` (agent_center_tests.rs:738).
#[test]
fn live_center_hints_follow_configured_bindings() {
    pending(
        Owner::Keymap,
        "the footer hints and help are the oracle's default keys; a configured keymap does not exist in tui-rtwk",
    );
}

/// `live_center_search_row_appears_only_while_editing` (agent_center_tests.rs:779).
/// Editing: screen level through the rtwk TextField (scope change, orchestrator).
#[test]
fn live_center_search_row_appears_only_while_editing() {
    let mut h = open("search", 88, 16);
    // The column headings carry one style.
    let row = 3;
    let screen = h.t.lines()[row].clone();
    let at = |label: &str| screen[..screen.find(label).unwrap()].chars().count();
    let styles =
        ["Tasks", "Status", "Updated"].map(|label| rtwk_test::style_of(&h.t.cell(at(label), row)));
    assert!(styles.iter().all(|style| *style == styles[0]), "{styles:?}");
    let idle = oracle_snap::screen(&h.t);
    for key in ["/", "f3"] {
        h.press(key);
        h.t.paste("find");
        h.settle();
        assert_app(&h, "command_center__live_center_search_active");
        h.press("esc");
        assert_eq!(oracle_snap::screen(&h.t), idle, "after {key} and esc");
    }
}
