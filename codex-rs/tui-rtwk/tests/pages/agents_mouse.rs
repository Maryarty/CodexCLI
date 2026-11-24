// test root: pages
//! Mouse tests of S21, the agents command center (ADR-011): a click selects
//! a row, a click on the selected row opens it, the wheel scrolls the list by
//! item, the filter tabs are clickable. The oracle has no mouse; these hold
//! the port's ADR-011 contract on the oracle's own fixtures.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::common;

use codex_app_server_protocol::Thread;
use codex_tui_core::app::{AppEv, Command};
use codex_tui_core::fixtures::builders::{agent_completed, agent_delta, turn_completed};
use codex_tui_core::ids::Ticket;
use codex_tui_core::rpc::Reply as AppReply;
use codex_tui_core::screens::agents::{Command as AgentsCommand, Ev, Page, Reply as AgentsReply};
use codex_tui_core::screens::{ScreenCommand, ScreenEv, ScreenReply};
use codex_tui_core::threads::ThreadAsk;
use codex_tui_rtwk::services::backend::fake::FAKE_THREAD;
use common::{Harness, start_driven};
use rtwk_core::input::{Mods, MouseKind};
use serde_json::json;

/// On a driven harness: these tests settle after every step, and a settle on
/// the real clock sleeps 100 ms (the ten tests were 8.7 s; driven, 1.7 s).
fn open(entry: &str, width: u16, height: u16) -> Harness {
    let mut h = start_driven(true, width, height);
    h.open_debug_row("agents", Some(entry));
    h
}

/// The row (y) of the screen whose text holds `needle`, and the column of it.
fn find(h: &Harness, needle: &str) -> (u16, u16) {
    let screen = h.t.screen();
    for (y, line) in screen.lines().enumerate() {
        if let Some(byte) = line.find(needle) {
            let x = line[..byte].chars().count();
            return (x as u16, y as u16);
        }
    }
    panic!("`{needle}` is not on the screen:\n{screen}");
}

fn highlighted(h: &Harness) -> String {
    h.t.screen()
        .lines()
        .find(|line| line.contains('›'))
        .unwrap_or_default()
        .to_owned()
}

#[test]
fn a_click_selects_the_row_under_it() {
    let mut h = open("overview", 96, 25);
    assert!(
        highlighted(&h).contains("Build the dashboard"),
        "{}",
        h.t.screen()
    );
    let (x, y) = find(&h, "Repair authentication");
    h.t.click(x, y);
    h.settle();
    assert!(
        highlighted(&h).contains("Repair authentication"),
        "{}",
        h.t.screen()
    );
}

/// The live path end to end on the fake backend: `/agents` asks for the list,
/// the answer fills the center, a click selects, a click on the selected row
/// asks the backend to resume it.
#[test]
fn a_click_on_the_selected_row_opens_it() {
    let mut h = common::start();
    h.wait_idle();
    h.fake.script_turn(vec![
        agent_delta(FAKE_THREAD, "turn-1", "msg-1", "Hello"),
        agent_completed(FAKE_THREAD, "turn-1", "msg-1", "Hello"),
        turn_completed(FAKE_THREAD, "turn-1"),
    ]);
    h.t.type_text("hi");
    h.press("enter");
    h.wait_for("the answer", |s| s.contains("• Hello"));
    h.wait_idle();
    h.t.type_text("/agents");
    h.press("enter");
    h.t.pump_for(common::ms(800));
    h.wait_for("the command center", |s| s.contains("Agent command center"));
    h.settle();
    let ticket = list_ticket(&h);
    let epoch = h.fake.epoch().expect("connected");
    let threads: Vec<Thread> = fixture_threads();
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
    h.wait_for("the rows", |s| s.contains("Repair authentication"));
    let (x, y) = find(&h, "Repair authentication");
    h.t.click(x, y);
    h.settle();
    assert!(
        highlighted(&h).contains("Repair authentication"),
        "{}",
        h.t.screen()
    );
    assert!(resumes(&h).is_empty(), "a first click only selects");
    h.t.click(x, y);
    h.settle();
    assert_eq!(resumes(&h).len(), 1, "{:?}", h.fake.commands());
}

fn list_ticket(h: &Harness) -> Ticket {
    h.fake
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
        .expect("the list was asked for")
}

fn resumes(h: &Harness) -> Vec<Command> {
    h.fake
        .commands()
        .into_iter()
        .filter(|command| {
            matches!(
                command,
                Command::Screen {
                    command: ScreenCommand::Agents(AgentsCommand::Thread(ThreadAsk::Resume { .. })),
                    ..
                }
            )
        })
        .collect()
}

/// The threads of the `overview` fixture, as the protocol has them.
fn fixture_threads() -> Vec<Thread> {
    let script = include_str!("../../src/pages/agents/fixtures/overview.jsonl");
    script
        .lines()
        .filter(|line| line.contains("\"listed\""))
        .find_map(|line| {
            let step: serde_json::Value = serde_json::from_str(line).ok()?;
            let threads = step["ev"]["listed"]["threads"].clone();
            serde_json::from_value(threads).ok()
        })
        .expect("the fixture lists threads")
}

#[test]
fn a_click_on_a_filter_tab_filters() {
    let mut h = open("overview", 96, 25);
    let (x, y) = find(&h, "Needs you");
    h.t.click(x, y);
    let screen = h.t.screen();
    assert!(screen.contains("Needs you 1"), "{screen}");
    assert!(screen.contains("1 of 3"), "{screen}");
    assert!(!screen.contains("Review yesterday"), "{screen}");
}

#[test]
fn the_wheel_scrolls_the_list_by_item() {
    let mut h = open("overview", 96, 8);
    let (x, y) = find(&h, "Tasks");
    h.t.mouse(MouseKind::WheelDown, x, y + 2, 0, Mods::NONE);
    let screen = h.t.screen();
    assert!(
        screen.contains("↑"),
        "the list moved one item or more:\n{screen}"
    );
}

/// VS1 review D2: two steps that arrive in one terminal read move two rows
/// (the step starts from the list's cursor, which moves at once, not from the
/// model, which changes at the flush).
#[test]
fn two_steps_in_one_read_move_two_rows() {
    let mut slow = open("overview", 96, 25);
    slow.press("j");
    slow.press("j");
    slow.settle();
    let expected = highlighted(&slow);
    let mut fast = open("overview", 96, 25);
    fast.t.type_text("jj");
    fast.settle();
    assert_eq!(highlighted(&fast), expected, "{}", fast.t.screen());
    assert!(!expected.contains("Build the dashboard"));
}

/// VS1 review D3: a click in the same flush as a step is judged against the
/// list's cursor. Down moves to the second row; a click on the first row (the
/// one that was highlighted) selects it, and does not open the second.
#[test]
fn a_click_after_a_step_in_one_flush_selects_the_clicked_row() {
    let mut h = open("overview", 96, 25);
    let (x, y) = find(&h, "Build the dashboard");
    h.t.input(b"\x1b[B");
    h.t.click(x, y);
    h.settle();
    assert!(
        highlighted(&h).contains("Build the dashboard"),
        "{}",
        h.t.screen()
    );
    assert!(h.t.screen().contains("Agent command center"));
}

// ---- the agents-canonical fixes ----

fn highlighted_task(h: &Harness) -> String {
    h.t.screen()
        .lines()
        .find(|line| line.trim_start().starts_with("\u{203a} "))
        .and_then(|line| line.split('\u{2502}').next())
        .unwrap_or_default()
        .trim_end()
        .to_owned()
}

/// Fix 1: `ExcludeFocus` only removes focus; the list itself must refuse the
/// click (`SelectionList::enabled(false)`) while a name is being edited.
#[test]
fn a_click_while_renaming_leaves_the_highlight_where_it_was() {
    let mut h = open("overview", 96, 25);
    let before = highlighted_task(&h);
    assert!(before.contains("Build the dashboard"), "{}", h.t.screen());
    h.press("r");
    h.settle();
    let (x, y) = find(&h, "Repair authentication");
    h.t.click(x, y);
    h.settle();
    assert_eq!(highlighted_task(&h), before, "{}", h.t.screen());
    h.press("esc");
    h.settle();
    let (x, y) = find(&h, "Repair authentication");
    h.t.click(x, y);
    h.settle();
    assert!(
        highlighted_task(&h).contains("Repair authentication"),
        "{}",
        h.t.screen()
    );
}

fn usage_asks(h: &Harness) -> Vec<(Ticket, String)> {
    h.fake
        .commands()
        .into_iter()
        .filter_map(|command| match command {
            Command::Screen {
                ticket,
                command: ScreenCommand::Agents(AgentsCommand::Usage { thread }),
                ..
            } => Some((ticket, thread)),
            _ => None,
        })
        .collect()
}

/// Fix 2 (oracle: the repaint after the move asks): an arrow move asks for
/// the new task's usage in the same flush, not at the next model change.
#[test]
fn an_arrow_move_asks_for_the_new_tasks_usage() {
    let mut h = common::start();
    h.wait_idle();
    h.fake.script_turn(vec![
        agent_delta(FAKE_THREAD, "turn-1", "msg-1", "Hello"),
        agent_completed(FAKE_THREAD, "turn-1", "msg-1", "Hello"),
        turn_completed(FAKE_THREAD, "turn-1"),
    ]);
    h.t.type_text("hi");
    h.press("enter");
    h.wait_for("the answer", |s| s.contains("\u{2022} Hello"));
    h.wait_idle();
    h.t.type_text("/agents");
    h.press("enter");
    h.t.pump_for(common::ms(800));
    h.wait_for("the command center", |s| s.contains("Agent command center"));
    h.settle();
    let epoch = h.fake.epoch().expect("connected");
    h.fake.inject(
        epoch,
        AppEv::Screen(ScreenEv::Agents(Ev::Account {
            plan: Some(serde_json::from_value(json!("business")).unwrap()),
            backend_auth: true,
        })),
    );
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
    let script = include_str!("../../src/pages/agents/fixtures/overview.jsonl");
    let threads: Vec<Thread> = script
        .lines()
        .filter(|line| line.contains("\"listed\""))
        .find_map(|line| {
            let step: serde_json::Value = serde_json::from_str(line).ok()?;
            serde_json::from_value(step["ev"]["listed"]["threads"].clone()).ok()
        })
        .expect("threads");
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
    h.wait_for("the rows", |s| s.contains("Repair authentication"));
    h.settle();
    let first = usage_asks(&h);
    assert!(
        !first.is_empty(),
        "the highlighted task's usage is asked for"
    );
    let asked = first.len();
    // The fetch ends (a refused one counts), or no other is due.
    h.fake
        .inject(epoch, AppEv::Reply(first[0].0, AppReply::failed("down")));
    h.settle();
    h.press("down");
    h.settle();
    let after = usage_asks(&h);
    assert!(
        after.len() > asked && after.last().map(|a| &a.1) != first.last().map(|a| &a.1),
        "an arrow move asks for the new task: {first:?} then {after:?}"
    );
}

/// Fix 3: every mode's keys sit on the widget that shows it and the focus
/// follows the mode with no request from the screen: search, rename and the
/// help each close back to a list that takes its keys.
#[test]
fn modes_hand_the_focus_back_without_a_request() {
    let mut h = open("overview", 96, 25);
    for (open_key, shown) in [
        ("/", "Search \u{203a}"),
        ("r", "Rename \u{203a}"),
        ("?", "Task shortcuts"),
    ] {
        h.press(open_key);
        h.settle();
        assert!(h.t.screen().contains(shown), "{open_key}\n{}", h.t.screen());
        h.press("esc");
        h.settle();
        assert!(
            !h.t.screen().contains(shown),
            "{open_key}\n{}",
            h.t.screen()
        );
        h.press("j");
        h.settle();
        assert!(
            highlighted_task(&h).contains("Repair authentication")
                || highlighted_task(&h).contains("Review"),
            "the list has the keys after {open_key}\n{}",
            h.t.screen()
        );
        h.press("k");
        h.settle();
        assert!(
            highlighted_task(&h).contains("Build the dashboard"),
            "{open_key}"
        );
    }
}

/// Fix 3: the empty list's line is a focus stop with the keys too: help
/// opens and closes over it, and a tab with tasks gives the list its keys.
#[test]
fn the_empty_list_and_the_list_each_hold_the_keys() {
    let mut h = open("overview", 96, 25);
    for _ in 0..3 {
        h.press("tab");
    }
    h.settle();
    assert!(
        !h.t.screen().contains("Build the dashboard"),
        "{}",
        h.t.screen()
    );
    h.press("?");
    h.settle();
    assert!(h.t.screen().contains("Task shortcuts"), "{}", h.t.screen());
    h.press("esc");
    h.settle();
    h.press("tab");
    h.press("tab");
    h.settle();
    assert!(
        h.t.screen().contains("Build the dashboard"),
        "{}",
        h.t.screen()
    );
    // The highlight kept the last task (the one the filter before showed);
    // `k` moves it up, `j` would wrap to the first (oracle `move_selection`).
    h.press("k");
    h.settle();
    assert!(
        highlighted_task(&h).contains("Repair authentication"),
        "the list has the keys again\n{}",
        h.t.screen()
    );
}
