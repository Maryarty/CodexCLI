//! Screen tests of S21, the agents command center. Oracle:
//! `codex-rs/tui/src/app/agents_overview_tests.rs` (the snapshots under
//! `snapshots/codex_tui__app__agents_overview__tests__*.snap`).

#![allow(clippy::unwrap_used, clippy::expect_used)]

#[allow(clippy::duplicate_mod)] // one helper, a module in each slice file
#[path = "../oracle_snap/mod.rs"]
mod oracle_snap;

use crate::common::{Harness, start_driven, start_driven_clipping, start_sized};
use codex_app_server_protocol::Thread;
use codex_tui_core::app::{AppEv, Command};
use codex_tui_core::rpc::Reply as AppReply;
use codex_tui_core::screens::agents::{Command as AgentsCommand, Page, Reply as AgentsReply};
use codex_tui_core::screens::{ScreenCommand, ScreenReply};

/// The kit's key-label normalization of our rows (platform labels: `delete`
/// shipped for the `backspace` of the oracle's `cfg(test)` snapshots, the
/// modifier symbols), the same one the kit's comparison makes.
#[allow(clippy::duplicate_mod)]
#[path = "../kit/labels.rs"]
mod labels;

/// The screen's rows equal the oracle's: ours go through the kit's label
/// normalization, the oracle's expected rows are compared as they are.
fn assert_rows(name: &str, actual: &[String], expected: &[String]) {
    let actual = labels::normalize(actual.to_vec(), expected);
    let actual: Vec<String> = actual.iter().map(|r| r.trim_end().to_owned()).collect();
    oracle_snap::assert_same(name, &actual, expected);
}
use rtwk_core::style::{Attrs, Color};

/// Who makes a pending test pass (tm aliases of the slices).
#[derive(Clone, Copy)]
enum Owner {
    ReuseCarry,
    /// An open rtwk gap (the reason starts with its `Gxx`).
    Rtwk,
}

impl Owner {
    fn slice(self) -> &'static str {
        match self {
            Owner::ReuseCarry => "reuse-carry",
            Owner::Rtwk => "rtwk",
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
    let actual: Vec<String> = oracle_snap::screen(&h.t)
        .iter()
        .map(|row| as_snapshot_cells(row))
        .collect();
    assert_rows(name, &actual, &app_oracle(name));
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
    assert_rows("agents_overview", &oracle_snap::screen(&h.t), &expected);
}

#[test]
fn agents_overview_details_show_available_attention_without_expanding_rows() {
    let (expected, height) = oracle("agents_overview_attention");
    let h = open("attention", 96, height);
    assert_rows(
        "agents_overview_attention",
        &oracle_snap::screen(&h.t),
        &expected,
    );
}

#[test]
fn empty_command_center_can_open_resume_picker() {
    let (expected, height) = oracle("agents_overview_empty_narrow");
    let h = open("empty_narrow", 48, height);
    assert_rows(
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

/// An empty filter drops the highlight; coming back to a filter with tasks
/// puts it on the first row (oracle: `reconcile_command_center_selection`
/// sets `selected` to `usize::MAX`; pty scenario `agents`, `filter-all`).
#[test]
fn filter_without_tasks_resets_the_highlight_to_the_first_row() {
    let mut h = open("paging5", 100, 24);
    let marked = |h: &Harness| {
        h.t.screen()
            .lines()
            .find(|l| l.contains("\u{203a}"))
            .map(|l| l.trim().to_owned())
    };
    let first = marked(&h);
    assert!(first.is_some_and(|row| row.starts_with("\u{203a} \u{25cb} Paging task 00")));
    h.press("down");
    assert!(marked(&h).is_some_and(|row| row.contains("Paging task 01")));
    h.press("tab");
    assert!(
        h.t.screen().contains("No matching tasks"),
        "{}",
        h.t.screen()
    );
    h.press("shift+tab");
    let screen = h.t.screen();
    assert!(
        marked(&h).is_some_and(|row| row.starts_with("\u{203a} \u{25cb} Paging task 00")),
        "{screen}"
    );
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

// The oracle's screen tests that need a state the debug rows cannot give yet
// are `pending` with their reason and owner (body written, the marker first);
// the debug entries are in `src/pages/agents/mod.rs`.

/// `overview_worktree_creation_busy_state` (agents_overview_tests.rs:25): with a
/// worktree being created (and an older-server notice below it) the center
/// holds back `n w o r x h a`; its footer says it is busy.
#[test]
fn overview_worktree_creation_busy_state() {
    let mut h = open("busy", 80, 24);
    for key in ["n", "w", "o", "r", "x", "h", "a"] {
        h.press(key);
    }
    assert_app(&h, "overview_worktree_creation_busy_state");
}

/// Where `text` stands on the screen: every `(column, row)` of it.
fn find_all(h: &Harness, text: &str) -> Vec<(usize, usize)> {
    let mut found = Vec::new();
    for (y, row) in h.t.lines().iter().enumerate() {
        for (at, _) in row.match_indices(text) {
            found.push((row[..at].chars().count(), y));
        }
    }
    found
}

/// `overview_thread_colors_match_footer_and_respect_color_suppression`
/// (agents_overview_tests.rs:51): an unselected task's title is drawn in the
/// colour of its thread (id 42: `Rgb(242, 205, 205)` in the snapshot), the
/// selected row's is the selection's (no thread colour), the details title
/// of the selected task is its thread's colour and bold; with the
/// status-line colours off every title is plain.
#[test]
fn overview_thread_colors_match_footer_and_respect_color_suppression() {
    let (rgb, bold) = (Color::rgb(242, 205, 205), Attrs::BOLD);
    let on = open("colors", 100, 30);
    let (x, y) = find_all(&on, "Named task")[0];
    let title = on.t.cell(x, y);
    assert_eq!(title.fg, rgb, "the unselected title");
    assert!(!title.attrs.intersects(bold), "the list title is not bold");
    // The selected row keeps the selection style, whatever its thread.
    let (x, y) = find_all(&on, "\u{25cf} Selected task")[0];
    let selected = on.t.cell(x + 2, y);
    assert_ne!(selected.fg, Color::rgb(137, 180, 250), "{selected:?}");
    assert!(
        !selected.bg.is_default(),
        "the selection's background: {selected:?}"
    );
    // The details heading: the selected thread's colour (id 43), bold.
    let (x, y) = *find_all(&on, "Selected task")
        .iter()
        .find(|(x, _)| *x >= 50)
        .unwrap_or_else(|| panic!("the details title: {}", on.t.screen()));
    let details = on.t.cell(x, y);
    assert_eq!(details.fg, Color::rgb(137, 180, 250), "{details:?}");
    assert!(details.attrs.intersects(bold), "{details:?}");

    let off = open("colors_off", 100, 30);
    let (x, y) = find_all(&off, "Named task")[0];
    assert!(
        off.t.cell(x, y).fg.is_default(),
        "no colour when suppressed"
    );
    let (x, y) = *find_all(&off, "Selected task")
        .iter()
        .find(|(x, _)| *x >= 50)
        .unwrap_or_else(|| panic!("the details title: {}", off.t.screen()));
    let details = off.t.cell(x, y);
    assert!(details.fg.is_default(), "{details:?}");
    assert!(details.attrs.intersects(bold), "{details:?}");
}

/// `older_server_notice_is_visible_in_agents_overview` (agents_overview_tests.rs:100):
/// the footer's last row is the notice.
#[test]
fn older_server_notice_is_visible_in_agents_overview() {
    let h = open("older_server", 80, 24);
    let rows = oracle_snap::screen(&h.t);
    assert_eq!(
        rows.last().map(String::as_str),
        Some(app_oracle("older_server_notice_is_visible_in_agents_overview")[0].as_str())
    );
}

/// `older_server_notice_truncates_in_narrow_overview` (agents_overview_tests.rs:127,
/// snap older_server_narrow_overview): the whole screen at 12 cells.
#[test]
fn older_server_notice_truncates_in_narrow_overview() {
    let height = app_oracle("older_server_narrow_overview").len() as u16;
    let h = open("older_server", 12, height);
    assert_app(&h, "older_server_narrow_overview");
}

/// `agents_overview_details_render_markdown` (agents_overview_tests.rs:1138):
/// the last message and the prompt of the details are markdown, drawn as the
/// oracle's `markdown_render` draws it (`## Findings` kept, `•` items, code
/// lines, a table) in the rows left under the budget of the panel.
#[test]
fn agents_overview_details_render_markdown() {
    pending(
        Owner::ReuseCarry,
        "the oracle's markdown renderer (`markdown_render`, wrapping, code and table layout) is not carried into tui-rtwk's core or rtwk-rich-backed view yet, and the scripts cannot seed a task's last message (wire.rs `seed_ev` has no key for it); the details print the text plain (view.rs `details`)",
    );
    let h = open("overview", 96, 40);
    assert_app(&h, "agents_overview_markdown");
}

/// `agents_overview_preview_links_survive_wrapping_and_clipping` (agents_overview_tests.rs:1221):
/// a link of the last message spans several rows and keeps its destination
/// (OSC 8) when wrapped and when clipped (an ellipsis outside the link).
#[test]
fn agents_overview_preview_links_survive_wrapping_and_clipping() {
    pending(
        Owner::ReuseCarry,
        "needs the markdown renderer with links (OSC 8 cells) and a seed for the last message, see agents_overview_details_render_markdown",
    );
    let h = open("overview", 96, 40);
    let screen = h.t.screen();
    assert!(
        screen.contains("a pull request with a label that wraps"),
        "{screen}"
    );
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

/// The page over a debug entry, under the keymap the oracle's test builds
/// (`TuiKeymap` fields, here as the seed names them).
fn keyed(entry: &str, width: u16, height: u16, keys: &[(&str, &str)]) -> Harness {
    let mut h = start_driven(true, width, height);
    h.set_keymap(keys);
    h.open_debug_row("agents", Some(entry));
    h.advance(800);
    h
}

/// How many commands the app has sent the backend whose debug text holds
/// `what` (a stop asks the thread's active turn, an open resumes it).
fn asked(h: &Harness, what: &str) -> usize {
    h.fake
        .commands()
        .iter()
        .filter(|command| format!("{command:?}").contains(what))
        .count()
}

/// The text of the highlighted row of the list, none while nothing is.
fn highlighted_row(h: &Harness) -> Option<String> {
    h.t.screen()
        .lines()
        .find(|line| line.trim_start().starts_with("\u{203a} "))
        .map(str::to_owned)
}

/// The configured keys of `filtered_dashboard_actions_use_configured_shortcuts`.
const FILTERED_KEYS: [(&str, &str); 5] = [
    ("agents.search", "f6"),
    ("agents.stop", "f10"),
    ("agents.resume", "f8"),
    ("list.cancel", "f7"),
    ("list.move_down", "v"),
];

/// `filtered_dashboard_actions_use_configured_shortcuts` (agents_overview_tests.rs:1753):
/// F6 searches, F8 resumes, F7 cancels and `v` moves down. The configured keys
/// replace the defaults: `j` no longer moves, `k` still moves up. The oracle
/// drives one view and checks the events it sends; the debug entry shows what
/// the keys do to the page, `configured_task_actions_ask_the_backend` the
/// commands they send on the live path (a debug entry runs on its own model).
#[test]
fn filtered_dashboard_actions_use_configured_shortcuts() {
    let keys = FILTERED_KEYS;
    let mut h = keyed("keys_two", 100, 24, &keys);
    for (key, task) in [
        ("v", "Second task"),
        ("k", "First task"),
        ("j", "First task"),
    ] {
        h.press(key);
        let row = highlighted_row(&h).unwrap_or_default();
        assert!(row.contains(task), "after {key}: {row}\n{}", h.t.screen());
    }
    // F6 opens the search line, a paste filters the list to the second task.
    h.press("f6");
    assert!(h.t.screen().contains("Search \u{203a}"), "{}", h.t.screen());
    h.t.paste("Second task");
    h.settle();
    let row = highlighted_row(&h).unwrap_or_default();
    assert!(row.contains("Second task"), "{row}\n{}", h.t.screen());

    // F8 opens the resume picker over the center.
    let mut h = keyed("keys_two", 100, 24, &keys);
    h.press("f8");
    h.wait_for("the resume picker", |s| {
        s.contains("Resume a previous session")
    });

    // F6 and a search nothing matches leave no row highlighted, F7 closes
    // the line and the highlight is back on the first task, online or not.
    for entry in ["keys_two", "paging30_offline"] {
        let mut h = keyed(entry, 100, 24, &keys);
        h.press("f6");
        h.t.paste("no matching task");
        h.settle();
        // No task is highlighted (online, the "Searching history…" row of
        // the model may be, which the oracle's view has no row for).
        let row = highlighted_row(&h);
        assert!(
            row.as_deref()
                .is_none_or(|row| row.contains("Searching history")),
            "{entry}: {}",
            h.t.screen()
        );
        h.press("f7");
        let row = highlighted_row(&h).unwrap_or_default();
        assert!(
            row.contains("First task") || row.contains("Paging task 00"),
            "{entry}: {row}\n{}",
            h.t.screen()
        );
    }
}

/// The live center on the fake backend (`/agents` over a ready session) under
/// `keys`, its list the `keys_two` fixture's: the commands the page sends are
/// the fake's `commands()` (a debug entry runs on its own model and sends none).
fn live_keyed(keys: &[(&str, &str)]) -> Harness {
    let mut h = start_driven(false, 100, 24);
    h.wait_idle();
    h.set_keymap(keys);
    h.t.type_text("/agents");
    h.press("enter");
    h.advance(800);
    h.wait_for("the command center", |s| s.contains("Agent command center"));
    let ticket = h
        .fake
        .commands()
        .iter()
        .find_map(|command| match command {
            Command::Screen {
                ticket,
                command: ScreenCommand::Agents(AgentsCommand::List { .. }),
                ..
            } => Some(*ticket),
            _ => None,
        })
        .expect("the list was asked for");
    let script = include_str!("../../src/pages/agents/fixtures/keys_two.jsonl");
    let threads: Vec<Thread> = script
        .lines()
        .filter(|line| line.contains("\"listed\""))
        .find_map(|line| {
            let step: serde_json::Value = serde_json::from_str(line).ok()?;
            serde_json::from_value(step["ev"]["listed"]["threads"].clone()).ok()
        })
        .expect("the fixture lists threads");
    let epoch = h.fake.epoch().expect("connected");
    h.fake.inject(
        epoch,
        AppEv::Reply(
            ticket,
            AppReply::screen(ScreenReply::Agents(AgentsReply::Listed(Page {
                threads,
                unread: Vec::new(),
                next_cursor: None,
                append: false,
                pins: None,
            }))),
        ),
    );
    h.wait_for("the rows", |s| s.contains("Second task"));
    h
}

/// The second half of `filtered_dashboard_actions_use_configured_shortcuts`
/// (agents_overview_tests.rs:1753): on the live path F10 stops the working task
/// (the idle one asks for nothing), with the search line open too, and Enter
/// on the filtered list resumes it.
#[test]
fn configured_task_actions_ask_the_backend() {
    let mut h = live_keyed(&FILTERED_KEYS);
    // The first task is idle: stopping it asks for nothing.
    h.press("f10");
    assert_eq!(asked(&h, "ActiveTurn"), 0, "{:?}", h.fake.commands());
    // Searching filters to the second task, which is working: F10 stops it
    // from the line, Enter opens it.
    h.press("f6");
    h.t.paste("Second task");
    h.settle();
    h.press("f10");
    assert_eq!(asked(&h, "ActiveTurn"), 1, "{:?}", h.fake.commands());
    assert_eq!(asked(&h, "Resume {"), 0);
    h.press("enter");
    assert_eq!(asked(&h, "Resume {"), 1, "{:?}", h.fake.commands());
}

/// `command_center_cursor_tracks_fixed_footer` (agents_overview_tests.rs:2472):
/// with the search line or the rename line open, the terminal cursor sits on
/// its row at every width, inside the screen.
#[test]
fn command_center_cursor_tracks_fixed_footer() {
    let mut h = open("confirm", 40, 24);
    for (key, label) in [("/", "Search \u{203a}"), ("r", "Rename \u{203a}")] {
        if key == "r" {
            h.press("escape");
        }
        h.press(key);
        for width in [48u16, 96, 120] {
            h.t.resize(width + 1, 24);
            h.t.resize(width, 24);
            h.settle();
            let screen = h.t.screen();
            let prompt_row = screen
                .lines()
                .position(|row| row.contains(label))
                .unwrap_or_else(|| panic!("no prompt row\n{screen}"));
            let (_, y) = h.t.cursor();
            assert!(h.t.cursor_visible(), "{screen}");
            assert_eq!(y, prompt_row, "{label} at {width}\n{screen}");
            assert!(y < 24);
        }
    }
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

/// The confirmation is a dialog of rtwk's overlay: Esc closes only it (the
/// screen stays), and the keys are the list's again at once (the dialog gives
/// the focus back), where the screen once refocused by hand.
#[test]
fn confirmation_dialog_closes_on_escape_and_gives_the_list_its_keys_back() {
    let (_, height) = oracle("agents_overview");
    let mut h = open("overview", 96, height);
    h.press("a");
    h.settle();
    assert!(
        h.t.screen().contains("Archive \u{201c}"),
        "{}",
        h.t.screen()
    );
    h.press("down");
    h.settle();
    assert!(
        !h.t.screen()
            .contains("\u{203a} \u{25cf} Repair authentication"),
        "a key under the dialog moved the list\n{}",
        h.t.screen()
    );
    h.press("escape");
    h.settle();
    let screen = h.t.screen();
    assert!(!screen.contains("Archive \u{201c}"), "{screen}");
    assert!(
        screen.contains("Agent command center"),
        "Esc left the screen\n{screen}"
    );
    h.press("down");
    h.settle();
    let screen = h.t.screen();
    assert!(
        screen.contains("\u{203a} \u{25cf} Repair authentication"),
        "{screen}"
    );
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
    assert!(
        screen.contains("\u{203a} 2. Permanently delete task and child agents"),
        "{screen}"
    );
    h.press("enter");
    h.settle();
    let screen = h.t.screen();
    assert!(!screen.contains("Permanently delete \u{201c}"), "{screen}");
    assert!(screen.contains("Deleting"), "{screen}");
}

// `lifecycle_failure_dialogs` (agents_overview_actions_tests.rs:845, :1007):
// the failure is a backend answer of the live server, so it is proven on the
// live shell, in tests/shell_agents.rs (a debug row has no backend).

/// `modal_consumes_input_and_finishes_after_input_closes` (agents_overview_action_progress_tests.rs:17),
/// snapshots archive_progress and delete_progress: while the removal runs the
/// footer says so, the removed task is gone, `End` still browses and
/// Enter and `n` do nothing. (Esc and the input queue of the oracle's modal
/// are the live shell's: here the model's gate lets Esc leave, see the parity pack.)
#[test]
fn modal_consumes_input_and_finishes_after_input_closes() {
    let age = |row: &str| -> String {
        // `[0-9]+d ago` with its padding is `[age]` in the snapshot.
        let Some(end) = row.find("d ago") else {
            return row.to_owned();
        };
        let head = &row[..end];
        let digits = head.chars().rev().take_while(char::is_ascii_digit).count();
        let head = head[..head.len() - digits].trim_end();
        format!("{head} [age]{}", &row[end + "d ago".len()..])
    };
    for (key, snapshot, notice) in [
        ("a", "archive_progress", "Archiving task"),
        ("backspace", "delete_progress", "Deleting task"),
    ] {
        let mut h = open("progress", 80, 24);
        h.press(key);
        h.settle();
        // Archive takes row 2 of the dialog; the delete waits for Enter.
        h.press("2");
        h.settle();
        if key == "backspace" {
            h.press("enter");
            h.settle();
        }
        h.press("end");
        for held in ["enter", "n"] {
            h.press(held);
        }
        h.settle();
        let screen = h.t.screen();
        assert!(screen.contains(notice), "{screen}");
        let rows: Vec<String> = oracle_snap::screen(&h.t)
            .iter()
            .map(|row| age(row))
            .collect();
        let expected = oracle_snap::rows(&format!(
            "app/snapshots/codex_tui__app__agents_overview_actions__progress_tests__{snapshot}.snap"
        ));
        assert_rows(snapshot, &rows, &expected);
    }
}

/// `live_center_columns` (agent_center_tests.rs:294): the columns at 160
/// cells (snapshot live_center_columns) and the narrow status text at 50. The
/// oracle's third part, the selected row's status glyph styles
/// (`live_center_selected_status_styles`), dumps ratatui `Style` values, which
/// have no counterpart in rtwk cells; see the parity pack.
#[test]
fn live_center_columns() {
    let h = open("columns", 50, 24);
    let screen = h.t.screen();
    assert!(screen.contains("\u{25cf} Error: Failed task"), "{screen}");
    let mut h = open("columns", 160, 22);
    h.settle();
    assert_app(&h, "command_center__live_center_columns");
}

/// The number of the task the highlight is on (`Paging task NN`).
fn highlighted_task(h: &Harness) -> usize {
    let screen = h.t.screen();
    let row = screen
        .lines()
        .find(|line| line.trim_start().starts_with("\u{203a} ") && line.contains("Paging task"))
        .unwrap_or_else(|| panic!("no highlighted task\n{screen}"));
    let at = row.find("Paging task ").unwrap() + "Paging task ".len();
    row[at..at + 2].parse().unwrap()
}

/// The paging flow of `live_center_pages_visible_rows_without_wrapping`
/// (agent_center_tests.rs:387) on `count` tasks, with the search line open
/// when `search`.
fn pages_visible_rows(count: usize, search: bool, loop_key: &str, wraps: bool) {
    let entry = if count == 5 { "paging5" } else { "paging30" };
    paging_flow(entry, count, search, loop_key, wraps);
}

/// The flow on the debug entry `entry` (`count` tasks).
fn paging_flow(entry: &str, count: usize, search: bool, loop_key: &str, wraps: bool) {
    let mut h = open(entry, 110, 18);
    if search {
        h.press("/");
        h.t.paste("Paging");
        h.settle();
    }
    assert_eq!(highlighted_task(&h), 0);
    h.press("pagedown");
    // Eleven visible rows: skip the next heading/gap, or clamp a short list.
    assert_eq!(highlighted_task(&h), (count - 1).min(10));
    let page_down = highlighted_task(&h);
    h.press("pageup");
    assert!(highlighted_task(&h) < page_down);
    h.press("pageup");
    assert_eq!(highlighted_task(&h), 0);
    // Repeated pages eventually reach the last filtered task and stay there.
    for _ in 0..count {
        h.press(loop_key);
    }
    let last = highlighted_task(&h);
    h.press("pagedown");
    assert_eq!((last, highlighted_task(&h)), (count - 1, last));
    if wraps {
        // Down from the last task wraps to the first (`move_selection`).
        h.press("down");
        let screen = h.t.screen();
        assert!(
            screen
                .lines()
                .any(|line| line.trim_start().starts_with("/tmp/project")),
            "{screen}"
        );
    }
}

/// `live_center_pages_visible_rows_without_wrapping` (agent_center_tests.rs:387),
/// the online flow: five tasks, then thirty tasks filtered by a search (the
/// oracle's repeated pages are `ctrl+f`; here PageDown, see the next test).
#[test]
fn live_center_pages_visible_rows_without_wrapping() {
    pages_visible_rows(5, false, "pagedown", false);
    pages_visible_rows(30, false, "pagedown", false);
}

/// The same flow of the oracle's second round while the center is offline
/// (`connection_notice = "Reconnecting\u{2026}"`).
#[test]
fn live_center_pages_visible_rows_while_reconnecting() {
    paging_flow("paging30_offline", 30, true, "pagedown", false);
}

/// What the oracle's flow asks beyond the list rtwk gives: the repeated pages
/// are `ctrl+f` (the list keymap's `page_down` default beside PageDown), and
/// Down from the last task wraps to the first.
#[test]
fn live_center_page_keys_and_wrap_follow_the_list_keymap() {
    pages_visible_rows(5, false, "ctrl+f", true);
    pages_visible_rows(30, true, "ctrl+f", true);
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
/// Its first half: the configured cancel key `q`, a plain character, leaves
/// the search line and the rename line. The oracle asks the keymap before it
/// types the character; rtwk's `TextField` takes a typed character before the
/// `KeyBindings` above it (`is_text`, shortcuts.rs), so the `q` is typed. The
/// second half is `the_page_keys_yield_to_the_action_bound_to_them`.
#[test]
fn live_center_fixed_shortcuts_yield_to_configured_actions() {
    pending(
        Owner::Rtwk,
        "G77: a configured plain-character cancel key cannot leave the search or rename line; Flutter's Shortcuts above a TextField take the character before the field types it, rtwk's KeyBindings above a TextField get only what the field does not take",
    );
    // `list.cancel = 'q'`: the cancel key leaves the search line and the
    // rename line, a plain character too, and the center stays.
    for (editor_key, label) in [("/", "Search \u{203a}"), ("r", "Rename \u{203a}")] {
        let mut h = keyed("search", 100, 24, &[("list.cancel", "q")]);
        h.press(editor_key);
        assert!(h.t.screen().contains(label), "{}", h.t.screen());
        h.press("q");
        assert!(!h.t.screen().contains(label), "{}", h.t.screen());
        assert!(h.t.screen().contains("Agent command center"));
    }
    the_page_keys_yield_to_the_action_bound_to_them();
}

/// The second half of `live_center_fixed_shortcuts_yield_to_configured_actions`:
/// the page's own keys (`?`, Tab) and the list's yield to the action the user
/// bound to them: the key resumes, the page does not use it.
#[test]
fn the_page_keys_yield_to_the_action_bound_to_them() {
    for binding in ["tab", "?", "shift+?", "f9", "enter"] {
        let mut h = keyed(
            "keys_empty",
            100,
            24,
            &[("list.cancel", "f9"), ("agents.resume", binding)],
        );
        let shows = |h: &Harness, text: &str| h.t.screen().contains(text);
        h.press("esc");
        assert!(shows(&h, "Agent command center"), "{binding}");
        if binding.ends_with('?') {
            assert!(!shows(&h, "? help"), "{binding}\n{}", h.t.screen());
        }
        if binding == "enter" {
            assert!(!shows(&h, "enter open"), "{}", h.t.screen());
            h.press("?");
            assert!(!shows(&h, "enter open"), "{}", h.t.screen());
            assert!(shows(&h, "f9 back"), "{}", h.t.screen());
            h.press("esc");
            assert!(shows(&h, "Task shortcuts"), "{}", h.t.screen());
            h.press("f9");
            assert!(!shows(&h, "Task shortcuts"), "{}", h.t.screen());
            h.press("/");
            assert!(!shows(&h, "enter open"), "{}", h.t.screen());
            h.press("f9");
        }
        // The terminal sends `?` with Shift as the character alone.
        h.press(binding.trim_start_matches("shift+"));
        h.wait_for("the resume picker", |s| {
            s.contains("Resume a previous session")
        });
        assert_eq!(asked(&h, "Resume {"), 0, "{binding}");
    }
}

/// `live_center_hints_follow_configured_bindings` (agent_center_tests.rs:738):
/// `k/j` move, no page keys, no archive, `z r` renames and `z p` pins; the
/// footer and the help name what is bound, and only that.
#[test]
fn live_center_hints_follow_configured_bindings() {
    let keys = [
        ("list.move_up", "k"),
        ("list.move_down", "j"),
        ("list.page_up", ""),
        ("agents.archive", ""),
        ("agents.rename", "z r"),
        ("agents.toggle_pin", "z p"),
    ];
    let h = keyed("keys_empty", 100, 12, &keys);
    assert_app(&h, "command_center__live_center_custom_footer");
    let mut h = keyed("keys_task", 100, 24, &keys);
    h.press("?");
    assert_app(&h, "command_center__live_center_custom_help");
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

/// vs1-review D4: the screen draws at any width, with the list, the search
/// line, the help and the confirmation (the old view subtracted past zero
/// below nine cells). One test per width, on a driven harness (no real
/// pause: the debug menu's 450 keys and the settles cost their frames only,
/// 0.2 s a test; on the real clock each was 20 s).
fn the_center_draws_at(width: u16) {
    for height in [3u16, 6, 24] {
        let mut h = start_driven_clipping(true, width, height);
        h.open_debug_row("agents", Some("overview"));
        h.settle();
        h.press("/");
        h.settle();
        h.press("escape");
        h.settle();
        h.press("?");
        h.settle();
        h.press("escape");
        h.settle();
        let mut h = start_driven_clipping(true, width, height);
        h.open_debug_row("agents", Some("confirm"));
        h.press("a");
        h.settle();
        assert!(
            h.t.screen().lines().count() <= usize::from(height),
            "{width}x{height}"
        );
    }
}

macro_rules! the_center_draws_at_widths {
    ($($name:ident: $width:literal,)*) => {
        $(
            #[test]
            fn $name() {
                the_center_draws_at($width);
            }
        )*
    };
}

the_center_draws_at_widths! {
    the_center_draws_at_width_1: 1,
    the_center_draws_at_width_2: 2,
    the_center_draws_at_width_3: 3,
    the_center_draws_at_width_4: 4,
    the_center_draws_at_width_5: 5,
    the_center_draws_at_width_6: 6,
    the_center_draws_at_width_7: 7,
    the_center_draws_at_width_8: 8,
    the_center_draws_at_width_9: 9,
    the_center_draws_at_width_10: 10,
}

/// Item 2 (review point 44): `o` opens the resume picker in the center, empty
/// or not (oracle: `agents_overview_view.rs:766-769`, `OpenResumePicker`), and
/// it does not open the highlighted task.
#[test]
fn o_opens_the_resume_picker() {
    for entry in ["overview", "empty_narrow"] {
        let mut h = open(entry, 80, 24);
        h.press("o");
        h.wait_for("the resume picker", |s| {
            s.contains("Resume a previous session")
        });
        assert!(
            !h.fake
                .commands()
                .iter()
                .any(|command| format!("{command:?}").contains("Resume {")),
            "{entry}: no task was resumed"
        );
    }
}

/// While a worktree is being created the center holds `o` back too (oracle:
/// the gate before `agents_keymap.resume`).
#[test]
fn o_is_held_back_while_busy() {
    let mut h = open("busy", 80, 24);
    h.press("o");
    h.settle();
    assert!(!h.t.screen().contains("Resume a previous session"));
}

/// Item 4: the search key toggles the line (oracle:
/// `agents_overview_view.rs:731-745`): F3 opens it, F3 again closes it.
#[test]
fn f3_toggles_the_search_line() {
    let mut h = open("overview", 80, 24);
    assert!(!h.t.screen().contains("Search \u{203a}"));
    h.press("f3");
    h.settle();
    assert!(h.t.screen().contains("Search \u{203a}"), "{}", h.t.screen());
    h.press("f3");
    h.settle();
    assert!(
        !h.t.screen().contains("Search \u{203a}"),
        "{}",
        h.t.screen()
    );
}
