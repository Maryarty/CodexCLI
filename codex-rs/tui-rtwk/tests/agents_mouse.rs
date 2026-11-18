//! Mouse tests of S21, the agents command center (ADR-011): a click selects
//! a row, a click on the selected row opens it, the wheel scrolls the list by
//! item, the filter tabs are clickable. The oracle has no mouse; these hold
//! the port's ADR-011 contract on the oracle's own fixtures.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use codex_app_server_protocol::Thread;
use codex_tui_core::app::{AppEv, Command};
use codex_tui_core::fixtures::builders::{agent_completed, agent_delta, turn_completed};
use codex_tui_core::ids::Ticket;
use codex_tui_core::rpc::Reply as AppReply;
use codex_tui_core::screens::agents::{Command as AgentsCommand, Page, Reply as AgentsReply};
use codex_tui_core::screens::{ScreenCommand, ScreenReply};
use codex_tui_rtwk::shell::fake::FAKE_THREAD;
use common::{Harness, start_sized};
use rtwk_core::input::{Mods, MouseKind};

fn open(entry: &str, width: u16, height: u16) -> Harness {
    let mut h = start_sized(true, width, height);
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
    h.wait_for("ready", |s| s.contains("ready"));
    h.fake.script_turn(vec![
        agent_delta(FAKE_THREAD, "turn-1", "msg-1", "Hello"),
        agent_completed(FAKE_THREAD, "turn-1", "msg-1", "Hello"),
        turn_completed(FAKE_THREAD, "turn-1"),
    ]);
    h.t.type_text("hi");
    h.press("enter");
    h.wait_for("the answer", |s| s.contains("• Hello"));
    h.wait_for("the turn to end", |s| s.contains("ready"));
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
                    command: ScreenCommand::Agents(AgentsCommand::Resume { .. }),
                    ..
                }
            )
        })
        .collect()
}

/// The threads of the `overview` fixture, as the protocol has them.
fn fixture_threads() -> Vec<Thread> {
    let script = include_str!("../src/screens/agents/fixtures/overview.jsonl");
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
