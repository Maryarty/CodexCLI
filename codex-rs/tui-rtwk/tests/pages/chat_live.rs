//! Page tests of the chat's live turn (the status row, the queued messages and the
//! backend banner above the composer): the keys that interrupt a turn, and the mouse on
//! the banner (ADR-011; the oracle's banner has no mouse, so its shards say nothing).
//! The rows themselves are the oracle's: `tests/oracle/chat_live_*.rs`.

#![allow(clippy::expect_used)]

use codex_tui_core::app::{AppEv, Command};
use codex_tui_core::fixtures::builders::{agent_delta, turn_completed};
use codex_tui_core::screens::{ScreenEv, chat};
use codex_tui_rtwk::services::backend::fake::FAKE_THREAD;
use rtwk_core::input::{Mods, MouseKind};
use serde_json::json;

use crate::common::{Harness, start, start_quittable};

/// A chat whose first turn is running: its answer began and does not end.
fn running() -> Harness {
    let mut h = start();
    h.wait_idle();
    h.fake
        .script_turn(vec![agent_delta(FAKE_THREAD, "turn-1", "msg-1", "so far")]);
    h.t.type_text("go");
    h.press("enter");
    h.wait_for("the turn to run", |s| s.contains("esc to interrupt"));
    h
}

fn interrupts(h: &Harness) -> usize {
    h.fake
        .commands()
        .iter()
        .filter(|command| matches!(command, Command::InterruptTurn { .. }))
        .count()
}

#[test]
fn the_status_row_runs_with_the_turn_and_leaves_with_it() {
    let mut h = running();
    h.t.pump_for(crate::common::ms(1100));
    let screen = h.t.screen();
    assert!(screen.contains("Working"), "{screen}");
    assert!(screen.contains("1s"), "{screen}");
    let epoch = h.fake.epoch().expect("attached");
    h.fake
        .inject(epoch, AppEv::Backend(turn_completed(FAKE_THREAD, "turn-1")));
    h.wait_for("the row to leave", |s| !s.contains("esc to interrupt"));
    assert!(!h.t.screen().contains("Working"), "{}", h.t.screen());
}

/// A chat whose account read carried a backend banner (`rate_limit_upsell`).
fn with_banner(presentation: &str) -> Harness {
    let mut h = start();
    h.wait_idle();
    let presentation = presentation.to_owned();
    h.fake
        .respond(move |request| match request["method"].as_str() {
            Some("account/rateLimits/read") => Some(Ok(json!({
                "rateLimits": {},
                "rateLimitUpsell": {
                    "banner_type": "credits",
                    "title": "Usage limit reached",
                    "description": "Add credits to keep working.",
                    "ctas": [{"action": "view_usage", "label": "View usage"}],
                    "presentation": presentation,
                },
            }))),
            _ => None,
        });
    let epoch = h.fake.epoch().expect("attached");
    h.fake.inject(
        epoch,
        AppEv::Screen(ScreenEv::Chat(chat::Ev::Status(Box::new(
            chat::StatusFacts {
                account_usage: true,
                ..chat::StatusFacts::default()
            },
        )))),
    );
    h.wait_for("the banner", |s| s.contains("Usage limit reached"));
    h
}

/// The cell of the first row that shows `needle`.
fn cell_of(h: &Harness, needle: &str) -> (u16, u16) {
    for (y, line) in h.t.lines().iter().enumerate() {
        if let Some(byte) = line.find(needle) {
            return (line[..byte].chars().count() as u16, y as u16);
        }
    }
    panic!("`{needle}` is not on the screen:\n{}", h.t.screen());
}

fn opened_urls(h: &Harness) -> Vec<String> {
    h.fake
        .commands()
        .into_iter()
        .filter_map(|command| match command {
            Command::OpenUrl { url, .. } => Some(url),
            _ => None,
        })
        .collect()
}

/// `inline_banner_snapshot` and `backend_banner_personal_limit`: a blank row, the bold
/// title and the description, the numbered rows with the cursor on the first, and the hint
/// under them (`esc to dismiss` only for a dismissible banner).
#[test]
fn the_banner_is_the_oracles_numbered_rows_and_hint() {
    let h = with_banner("dismissible");
    let screen = h.t.screen();
    assert!(screen.contains("Usage limit reached"), "{screen}");
    assert!(screen.contains("Add credits to keep working."), "{screen}");
    let lines = h.t.lines();
    let row = lines
        .iter()
        .position(|line| line.starts_with("\u{203a} 1. View usage"))
        .unwrap_or_else(|| panic!("the cursor row is not on the screen:\n{screen}"));
    assert_eq!(lines[row - 1].trim(), "", "a blank row above the rows");
    assert!(lines[row - 2].contains("Add credits"), "{screen}");
    assert_eq!(lines[row + 1].trim(), "", "a blank row under the rows");
    assert!(
        lines[row + 2].starts_with("  Press a number to choose \u{b7} esc to dismiss"),
        "{screen}"
    );
    let inline = with_banner("inline");
    assert!(
        inline
            .t
            .lines()
            .iter()
            .any(|line| line.trim_end() == "  Press a number to choose"),
        "{}",
        inline.t.screen()
    );
}

/// `handle_inline_banner_key`: a digit chooses the row, and choosing does not dismiss.
#[test]
fn a_number_key_chooses_the_row_and_the_banner_stays() {
    let mut h = with_banner("dismissible");
    h.press("1");
    h.settle();
    assert_eq!(
        opened_urls(&h),
        ["https://chatgpt.com/settings/usage".to_owned()]
    );
    assert!(
        h.t.screen().contains("Usage limit reached"),
        "{}",
        h.t.screen()
    );
    // a number with no row of its own is the composer's
    h.press("2");
    h.settle();
    assert_eq!(opened_urls(&h).len(), 1);
    assert!(h.t.screen().contains('2'), "{}", h.t.screen());
}

/// The banner keeps its keys from a draft: a digit with text in the composer is text.
#[test]
fn a_number_with_a_draft_is_text() {
    let mut h = with_banner("dismissible");
    h.t.type_text("a");
    h.press("1");
    h.settle();
    assert!(opened_urls(&h).is_empty());
    assert!(h.t.screen().contains("a1"), "{}", h.t.screen());
}

#[test]
fn esc_dismisses_a_dismissible_banner_only() {
    let mut h = with_banner("dismissible");
    h.press("esc");
    h.settle();
    let screen = h.t.screen();
    assert!(!screen.contains("Usage limit reached"), "{screen}");
    let mut h = with_banner("inline");
    h.press("esc");
    h.settle();
    assert!(
        h.t.screen().contains("Usage limit reached"),
        "{}",
        h.t.screen()
    );
}

/// ADR-011: a tap on a row chooses it (the cursor is on the first already); the oracle's
/// banner has no mouse.
#[test]
fn a_tap_on_a_row_chooses_it() {
    let mut h = with_banner("dismissible");
    let (x, y) = cell_of(&h, "View usage");
    h.t.click(x, y);
    h.settle();
    assert_eq!(
        opened_urls(&h),
        ["https://chatgpt.com/settings/usage".to_owned()]
    );
    assert!(
        h.t.screen().contains("Usage limit reached"),
        "{}",
        h.t.screen()
    );
}

#[test]
fn a_tap_on_the_title_leaves_the_banner_alone() {
    let mut h = with_banner("inline");
    let (x, y) = cell_of(&h, "Usage limit reached");
    h.t.click(x, y);
    h.settle();
    assert!(
        h.t.screen().contains("Usage limit reached"),
        "{}",
        h.t.screen()
    );
    assert!(opened_urls(&h).is_empty());
}

#[test]
fn the_wheel_over_the_banner_leaves_it_alone() {
    let mut h = with_banner("dismissible");
    let (x, y) = cell_of(&h, "Add credits");
    h.t.mouse(MouseKind::WheelDown, x, y, 0, Mods::NONE);
    h.t.mouse(MouseKind::WheelUp, x, y, 0, Mods::NONE);
    h.settle();
    assert!(
        h.t.screen().contains("Usage limit reached"),
        "{}",
        h.t.screen()
    );
    assert!(opened_urls(&h).is_empty());
}

#[test]
fn esc_with_no_turn_interrupts_nothing() {
    let mut h = start();
    h.wait_idle();
    h.press("esc");
    h.settle();
    assert_eq!(interrupts(&h), 0);
}

#[test]
fn esc_interrupts_a_running_turn_once() {
    let mut h = running();
    h.press("esc");
    let fake = h.fake.clone();
    h.wait_for("the interrupt", |_| {
        fake.commands()
            .iter()
            .any(|command| matches!(command, Command::InterruptTurn { .. }))
    });
    assert_eq!(interrupts(&h), 1);
}

#[test]
fn ctrl_c_interrupts_a_running_turn_and_keeps_the_app() {
    let mut h = start_quittable(false);
    h.wait_idle();
    h.fake
        .script_turn(vec![agent_delta(FAKE_THREAD, "turn-1", "msg-1", "so far")]);
    h.t.type_text("go");
    h.press("enter");
    h.wait_for("the turn to run", |s| s.contains("esc to interrupt"));
    h.press("ctrl+c");
    let fake = h.fake.clone();
    h.wait_for("the interrupt", |_| {
        fake.commands()
            .iter()
            .any(|command| matches!(command, Command::InterruptTurn { .. }))
    });
    assert!(h.t.is_running());
}

/// Tab queues, Enter steers (the person, 2026-11-22).
#[test]
fn messages_queued_during_a_turn_show_above_the_composer_with_the_count_of_the_rest() {
    let mut h = running();
    for (text, shown) in [
        ("one", "(queued) one"),
        ("two", "(queued) two"),
        ("three", "(queued) three"),
        ("four", "… and 1 more"),
        ("five", "… and 2 more"),
    ] {
        h.t.type_text(text);
        h.wait_for("the queue hint", |s| s.contains("tab to queue message"));
        h.press("tab");
        h.wait_for("the message queued", |s| s.contains(shown));
    }
    let screen = h.t.screen();
    for text in ["one", "two", "three"] {
        assert!(screen.contains(&format!("(queued) {text}")), "{screen}");
    }
    assert!(!screen.contains("(queued) four"), "{screen}");
    assert!(screen.contains("… and 2 more"), "{screen}");
}

/// The edit-queued key (`alt-up` by default) takes the newest queued message
/// back into the composer (oracle `pop_latest_queued_composer_state`).
#[test]
fn the_edit_queued_key_puts_the_newest_queued_message_back_in_the_composer() {
    let mut h = running();
    for text in ["one", "two"] {
        h.t.type_text(text);
        h.wait_for("the queue hint", |s| s.contains("tab to queue message"));
        h.press("tab");
        h.wait_for("the message queued", |s| {
            s.contains(&format!("(queued) {text}"))
        });
    }
    h.press("alt+up");
    h.wait_for("the message back in the composer", |s| {
        !s.contains("(queued) two") && s.contains("› two")
    });
    let screen = h.t.screen();
    assert!(screen.contains("(queued) one"), "{screen}");
}
