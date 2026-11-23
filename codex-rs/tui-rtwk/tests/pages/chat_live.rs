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
use rtwk_core::style::Attrs;
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
    // the title shows a frame before the rows below it: wait for the last of them
    h.wait_for("the banner", |s| s.contains("View usage"));
    h
}

/// The cell of the first row that shows `needle`, if the screen shows it.
fn find_cell(h: &Harness, needle: &str) -> Option<(u16, u16)> {
    for (y, line) in h.t.lines().iter().enumerate() {
        if let Some(byte) = line.find(needle) {
            return Some((line[..byte].chars().count() as u16, y as u16));
        }
    }
    None
}

/// The cell of the first row that shows `needle`.
fn cell_of(h: &Harness, needle: &str) -> (u16, u16) {
    find_cell(h, needle)
        .unwrap_or_else(|| panic!("`{needle}` is not on the screen:\n{}", h.t.screen()))
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
    h.wait_until("the usage page to open", |h| !opened_urls(h).is_empty());
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
    h.wait_until("the usage page to open", |h| !opened_urls(h).is_empty());
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

/// Tab queues, Enter steers (the person, 2026-11-22). The preview lists every
/// queued message under its header (oracle `pending_input_preview.rs`).
#[test]
fn messages_queued_during_a_turn_show_above_the_composer_under_one_header() {
    let mut h = running();
    for text in ["one", "two", "three", "four", "five"] {
        h.t.type_text(text);
        h.wait_for("the queue hint", |s| s.contains("tab to queue message"));
        h.press("tab");
        h.wait_for("the message queued", |s| s.contains(&format!("↳ {text}")));
    }
    let screen = h.t.screen();
    assert!(screen.contains("• Queued follow-up inputs"), "{screen}");
    for text in ["one", "two", "three", "four", "five"] {
        assert!(screen.contains(&format!("↳ {text}")), "{screen}");
    }
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
        h.wait_for("the message queued", |s| s.contains(&format!("↳ {text}")));
    }
    h.press("alt+up");
    h.wait_for("the message back in the composer", |s| {
        !s.contains("↳ two") && s.contains("› two")
    });
    let screen = h.t.screen();
    assert!(screen.contains("↳ one"), "{screen}");
}

/// An edge that lands in the flush of a rebuild of the page (the turn's next delta arrives with
/// the key) is not lost to the rebuild (`widgets/directives.rs`: the listener is a dependent of
/// its own) and is carried out once: the composer holds the message once, not twice.
#[test]
fn an_edge_in_the_flush_of_a_rebuild_is_carried_out_once() {
    let mut h = running();
    h.t.type_text("one");
    h.wait_for("the queue hint", |s| s.contains("tab to queue message"));
    h.press("tab");
    h.wait_for("the message queued", |s| s.contains("↳ one"));
    let epoch = h.fake.epoch().expect("attached");
    h.fake.inject(
        epoch,
        AppEv::Backend(agent_delta(FAKE_THREAD, "turn-1", "msg-1", " and more")),
    );
    h.press("alt+up");
    h.wait_for("the message back in the composer", |s| {
        !s.contains("↳ one") && s.contains("› one")
    });
    h.advance(300);
    let screen = h.t.screen();
    assert_eq!(screen.matches("› one").count(), 1, "{screen}");
    assert!(!screen.contains("› oneone"), "{screen}");
}

/// A chat on `model` whose account read carried the Luna Reserve banner; `draft` is typed first.
fn with_reserve(model: &str, draft: &str) -> Harness {
    let mut h = start();
    h.wait_idle();
    h.t.type_text(draft);
    // the draft is in the composer before the selector takes the pane
    if !draft.is_empty() {
        h.wait_for("the draft", |s| s.contains(draft));
    }
    h.fake
        .respond(move |request| match request["method"].as_str() {
            Some("account/rateLimits/read") => Some(Ok(json!({
                "accountId": "account-preview",
                "rateLimits": {},
                "rateLimitUpsell": {
                    "banner_type": "luna_reserve",
                    "presentation": "dismissible",
                    "title": "You\u{2019}re now using Luna, a faster model for simpler tasks.",
                    "description": "Add credits or upgrade to continue using the most advanced models.",
                    "ctas": [{"action": "add_credits", "label": "Add credits"},
                             {"action": "open_pricing_dialog", "label": "Upgrade"}],
                },
            }))),
            _ => None,
        });
    let epoch = h.fake.epoch().expect("attached");
    h.fake.inject(
        epoch,
        AppEv::Screen(ScreenEv::Chat(chat::Ev::Status(Box::new(
            chat::StatusFacts {
                model: Some(model.to_owned()),
                account_usage: true,
                ..chat::StatusFacts::default()
            },
        )))),
    );
    h.wait_for("the selector", |s| s.contains("Add credits"));
    h
}

/// The oracle's selector takes the bottom pane (`show_selection_view`): no composer, no footer;
/// Esc gives the draft back, and the composer takes keys again.
#[test]
fn the_reserve_selector_replaces_the_composer_and_gives_the_draft_back() {
    let mut h = with_reserve("gpt-reserve", "saved draft");
    let screen = h.t.screen();
    assert!(screen.contains("3. Continue with Luna Reserve"), "{screen}");
    assert!(screen.contains("esc to continue working"), "{screen}");
    assert!(!screen.contains("saved draft"), "{screen}");
    h.press("esc");
    h.wait_for("the draft back", |s| s.contains("saved draft"));
    let screen = h.t.screen();
    assert!(!screen.contains("You\u{2019}re now using Luna"), "{screen}");
    assert!(screen.contains("saved draft"), "{screen}");
    h.t.type_text("!");
    h.wait_for("the key in the draft", |s| s.contains("saved draft!"));
    assert!(h.t.screen().contains("saved draft!"), "{}", h.t.screen());
}

/// ADR-011: a tap on a purchase row picks it and closes the selector; a tap on the local row closes
/// it with no page.
#[test]
fn a_tap_on_a_reserve_row_picks_it() {
    let mut h = with_reserve("gpt-reserve", "saved draft");
    let (x, y) = cell_of(&h, "Upgrade");
    h.t.click(x, y);
    h.wait_until("the page and the draft", |h| {
        !opened_urls(h).is_empty() && h.t.screen().contains("saved draft")
    });
    assert_eq!(
        opened_urls(&h),
        ["https://chatgpt.com/?cta_tab=personal&highlight_plan=plus#pricing".to_owned()]
    );
    assert!(!h.t.screen().contains("Luna, a faster"), "{}", h.t.screen());
    assert!(h.t.screen().contains("saved draft"), "{}", h.t.screen());
    let mut h = with_reserve("gpt-reserve", "saved draft");
    let (x, y) = cell_of(&h, "Continue with Luna Reserve");
    h.t.click(x, y);
    h.wait_for("the draft back", |s| s.contains("saved draft"));
    h.settle();
    assert!(opened_urls(&h).is_empty());
    assert!(h.t.screen().contains("saved draft"), "{}", h.t.screen());
}

/// Another model than the reserve (the switch is pending): the usage-limit copy, the purchase rows
/// only, and the list's own footer.
#[test]
fn the_selector_before_the_switch_has_no_local_row() {
    let h = with_reserve("gpt-5", "");
    let screen = h.t.screen();
    assert!(screen.contains("Usage limit reached"), "{screen}");
    assert!(screen.contains("2. Upgrade"), "{screen}");
    assert!(!screen.contains("Continue with Luna Reserve"), "{screen}");
    assert!(!screen.contains("esc to continue working"), "{screen}");
}

// ---- the detailed transcript and Esc Esc (Modes: `detailed`, `primed`) ----

/// A chat with one finished exchange: the prompt `go` and the answer `so far`.
fn answered() -> Harness {
    let mut h = running();
    let epoch = h.fake.epoch().expect("attached");
    h.fake
        .inject(epoch, AppEv::Backend(turn_completed(FAKE_THREAD, "turn-1")));
    h.wait_for("the turn to end", |s| !s.contains("esc to interrupt"));
    h
}

/// The first cell of `needle` is drawn reversed (the marked prompt); a prompt
/// the screen does not show (the mention popup grows over its rows) is not.
fn reversed(h: &Harness, needle: &str) -> bool {
    let Some((x, y)) = find_cell(h, needle) else {
        return false;
    };
    h.t.cell(usize::from(x), usize::from(y))
        .attrs
        .intersects(Attrs::REVERSE)
}

/// Ctrl+T shows the details in the chat's own transcript (`set_presentation`): no
/// page, no pager keys, the composer under it keeps what is typed and pasted.
#[test]
fn ctrl_t_leaves_the_composer_live() {
    let mut h = answered();
    h.press("ctrl+t");
    h.t.type_text("zz");
    h.wait_for("the composer", |s| s.contains("zz"));
    // `q` is text here, not the pager's close
    h.t.type_text("q");
    h.wait_for("the composer", |s| s.contains("zzq"));
    h.press("ctrl+t");
    h.t.type_text("!");
    h.wait_for("the composer", |s| s.contains("zzq!"));
}

/// The first Esc says what the second does; any other key takes it back.
#[test]
fn the_first_esc_primes_and_any_other_key_takes_it_back() {
    let mut h = answered();
    h.press("escape");
    h.wait_for("the hint", |s| {
        s.contains("esc again to edit previous message")
    });
    h.t.type_text("x");
    h.wait_for("the hint gone", |s| {
        !s.contains("esc again to edit previous message")
    });
    // the pair is over: the next Esc primes again
    h.press("backspace");
    h.press("escape");
    h.wait_for("the hint", |s| {
        s.contains("esc again to edit previous message")
    });
}

/// The second Esc marks the newest prompt in the transcript; Esc ends the
/// browsing and the composer is the chat's again.
#[test]
fn esc_esc_marks_the_newest_prompt_and_esc_ends_the_browsing() {
    let mut h = answered();
    h.press("escape");
    h.press("escape");
    h.wait_for("the mark", |s| s.contains("go") && s.contains("so far"));
    assert!(reversed(&h, "go"), "{}", h.t.screen());
    assert!(
        !h.t.screen().contains("esc again to edit previous message"),
        "{}",
        h.t.screen()
    );
    h.press("escape");
    h.t.pump_for(crate::common::ms(100));
    assert!(!reversed(&h, "go"), "{}", h.t.screen());
    // the arrow keys are the composer's again (`q` is text)
    h.t.type_text("qr");
    h.wait_for("the composer", |s| s.contains("qr"));
}

/// While prompts are browsed another key ends the browsing and is the
/// composer's (`app.rs`: leave browsing before unhandled editing input).
#[test]
fn a_key_that_is_text_ends_the_browsing_and_types() {
    let mut h = answered();
    h.press("escape");
    h.press("escape");
    h.wait_until("the mark", |h| reversed(h, "go"));
    h.t.type_text("@");
    h.wait_until("the mark gone", |h| !reversed(h, "go"));
    h.wait_for("the composer", |s| s.contains('@'));
}

/// Enter in the browsing is the prompt chosen: the browsing is over.
#[test]
fn enter_while_browsing_chooses_the_prompt() {
    let mut h = answered();
    h.press("escape");
    h.press("escape");
    h.wait_until("the mark", |h| reversed(h, "go"));
    h.press("enter");
    h.wait_until("the mark gone", |h| !reversed(h, "go"));
    h.t.type_text("abc");
    h.wait_for("the composer", |s| s.contains("abc"));
}

/// `q` in the browsing is the pager's close (`navigate_pager`): the transcript
/// is the chat's again.
#[test]
fn q_while_browsing_closes_it() {
    let mut h = answered();
    h.press("escape");
    h.press("escape");
    h.wait_until("the mark", |h| reversed(h, "go"));
    h.press("q");
    h.wait_until("the mark gone", |h| !reversed(h, "go"));
    h.t.type_text("abc");
    h.wait_for("the composer", |s| s.contains("abc"));
}

/// Esc on the detailed transcript with an empty composer begins the browsing
/// (`begin_overlay_backtrack_preview`); Ctrl+T in it only changes the details.
#[test]
fn esc_on_the_detailed_transcript_browses_and_ctrl_t_keeps_the_mark() {
    let mut h = answered();
    h.press("ctrl+t");
    h.press("escape");
    h.wait_until("the mark", |h| reversed(h, "go"));
    h.press("ctrl+t");
    h.t.pump_for(crate::common::ms(100));
    assert!(reversed(&h, "go"), "{}", h.t.screen());
    h.press("escape");
    h.wait_until("the mark gone", |h| !reversed(h, "go"));
}

/// With text in the composer Esc is the composer's even on the detailed
/// transcript: nothing is browsed.
#[test]
fn esc_with_a_draft_on_the_detailed_transcript_browses_nothing() {
    let mut h = answered();
    h.press("ctrl+t");
    h.t.type_text("draft");
    h.press("escape");
    h.t.pump_for(crate::common::ms(200));
    assert!(h.t.screen().contains("draft"), "{}", h.t.screen());
    assert!(!reversed(&h, "go"), "{}", h.t.screen());
}

/// With no prompt to edit the second Esc says so and opens nothing.
#[test]
fn esc_esc_with_no_prompt_says_there_is_nothing_to_edit() {
    let mut h = start();
    h.wait_idle();
    h.press("escape");
    h.press("escape");
    h.wait_for("the line", |s| s.contains("No previous message to edit."));
    h.t.type_text("abc");
    h.wait_for("the composer", |s| s.contains("abc"));
}

/// An Esc with a draft is the composer's: nothing is primed.
#[test]
fn esc_with_a_draft_primes_nothing() {
    let mut h = answered();
    h.t.type_text("draft");
    h.press("escape");
    h.press("escape");
    h.t.pump_for(crate::common::ms(200));
    assert!(h.t.screen().contains("draft"), "{}", h.t.screen());
    assert!(
        !h.t.screen().contains("esc again to edit previous message"),
        "{}",
        h.t.screen()
    );
}

/// F3 shows the details the find will search; the composer stays live.
#[test]
fn f3_shows_the_details() {
    let mut h = answered();
    h.press("f3");
    h.t.type_text("abc");
    h.wait_for("the composer", |s| s.contains("abc"));
}

// ---- the plan question and other sheets (`no_modal_or_popup_active`) ----

const PLAN_ITEM: &str = include_str!("../oracle/fixtures/chat_live_4/plan_item.jsonl");
const PLAN_TITLE: &str = "Implement this plan?";

/// The server notifications of the oracle's plan fixture (`chat_live_4/plan_item.jsonl`) that
/// `wanted` takes: the settings that make the mode Plan, and the turn with its plan.
fn plan_notes(h: &mut Harness, wanted: impl Fn(&str) -> bool) {
    use codex_app_server_protocol::ServerNotification;
    let events: Vec<AppEv> = PLAN_ITEM
        .lines()
        .filter(|line| line.starts_with('{') && wanted(line))
        .filter_map(|line| {
            let step: serde_json::Value = serde_json::from_str(line).expect("a json line");
            let message = step.get("message")?.clone();
            let note: ServerNotification = serde_json::from_value(message).expect("a notification");
            Some(AppEv::Backend(
                codex_tui_core::app::BackendEv::Notification(Box::new(note)),
            ))
        })
        .collect();
    h.inject(events);
}

fn plan_turn_ends(h: &mut Harness) {
    plan_notes(h, |line| !line.contains("thread/settings/updated"));
    h.inject([AppEv::Backend(turn_completed(FAKE_THREAD, "turn-1"))]);
    h.settle();
}

/// A Plan-mode turn that ends with a proposed plan while the sheet of `/permissions` is open does not
/// open the question over it: the oracle skips it (`turn_runtime.rs`, `no_modal_or_popup_active`),
/// and does not keep it waiting after the sheet closes.
#[test]
fn a_plan_that_ends_under_another_sheet_asks_nothing() {
    let mut h = start();
    h.wait_idle();
    plan_notes(&mut h, |line| line.contains("thread/settings/updated"));
    // `/permissions` is refused while a turn runs: the sheet opens first, the turn comes under it
    h.t.type_text("/permissions");
    h.press("enter");
    h.wait_for("the permissions sheet", |s| {
        s.contains("Update Model Permissions")
    });
    plan_turn_ends(&mut h);
    let screen = h.t.screen();
    assert!(!screen.contains(PLAN_TITLE), "{screen}");
    assert!(screen.contains("Update Model Permissions"), "{screen}");
    h.press("esc");
    h.settle();
    let screen = h.t.screen();
    assert!(!screen.contains("Update Model Permissions"), "{screen}");
    assert!(!screen.contains(PLAN_TITLE), "{screen}");
}

/// The same turn with no sheet open asks.
#[test]
fn a_plan_that_ends_with_no_sheet_open_asks() {
    let mut h = start();
    h.wait_idle();
    plan_notes(&mut h, |_| true);
    h.inject([AppEv::Backend(turn_completed(FAKE_THREAD, "turn-1"))]);
    h.wait_for("the plan question", |s| s.contains(PLAN_TITLE));
}
