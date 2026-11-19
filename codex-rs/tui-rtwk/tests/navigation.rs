//! Navigation of the skeleton by hand (the person's first try: "chat at
//! start, F12, and you can never get back"): Escape pops the route in front
//! everywhere one is pushed, a dialog closes on Escape, `q` in the pushed
//! menu is back and quits only when the menu is the home (`--debug-screens`),
//! and the chat says which keys do what.

use codex_tui_core::fixtures::builders::{agent_completed, agent_delta, turn_completed};
use codex_tui_rtwk::shell::fake::FAKE_THREAD;

use crate::common::{Harness, start, start_debug, start_quittable};

const MENU_TITLE: &str = "codex-rtwk debug screens";

/// A chat with one answered prompt and a draft in the composer.
fn chat_with_history(h: &mut Harness) {
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
    h.t.type_text("a draft");
    h.t.pump_for(crate::common::ms(100));
}

fn assert_chat_intact(h: &Harness) {
    let screen = h.t.screen();
    assert!(
        !screen.contains(MENU_TITLE),
        "the menu is still up:\n{screen}"
    );
    assert!(screen.contains("› hi"), "transcript lost:\n{screen}");
    assert!(screen.contains("• Hello"), "transcript lost:\n{screen}");
    assert!(screen.contains("a draft"), "composer text lost:\n{screen}");
}

#[test]
fn chat_f12_escape_returns_to_the_chat_with_its_state() {
    let mut h = start();
    chat_with_history(&mut h);
    h.press("f12");
    let menu = h.t.screen();
    assert!(menu.contains(MENU_TITLE), "{menu}");
    assert!(!menu.contains("› hi"), "the menu is over the chat:\n{menu}");
    h.press("escape");
    assert_chat_intact(&h);
}

#[test]
fn a_nested_route_pops_to_the_menu_and_then_to_the_chat() {
    let mut h = start();
    chat_with_history(&mut h);
    h.press("f12");
    h.open_debug_row("cells", None);
    assert!(
        h.t.screen().contains("cells (S11): empty slot"),
        "{}",
        h.t.screen()
    );
    h.press("escape");
    let menu = h.t.screen();
    assert!(menu.contains(MENU_TITLE), "{menu}");
    assert!(!menu.contains("empty slot"), "{menu}");
    h.press("escape");
    assert_chat_intact(&h);
}

#[test]
fn a_dialog_closes_on_escape() {
    let mut h = start_debug();
    h.open_debug_row("select", Some("model_picker"));
    // The dialog fades in.
    h.t.pump_for(crate::common::ms(800));
    assert!(
        h.t.screen().contains("Select Model and Effort"),
        "{}",
        h.t.screen()
    );
    h.press("escape");
    // The dialog fades out before its route is gone.
    h.t.pump_for(crate::common::ms(600));
    let screen = h.t.screen();
    assert!(!screen.contains("Select Model and Effort"), "{screen}");
    assert!(screen.contains(MENU_TITLE), "{screen}");
}

#[test]
fn q_in_the_pushed_menu_returns_to_the_chat() {
    let mut h = start_quittable(false);
    chat_with_history(&mut h);
    h.press("f12");
    assert!(h.t.screen().contains(MENU_TITLE));
    h.press("q");
    assert!(h.t.is_running(), "q in a pushed menu is back, not quit");
    assert_chat_intact(&h);
}

#[test]
fn q_in_the_menu_as_home_quits() {
    let mut h = start_quittable(true);
    h.press("q");
    assert!(!h.t.is_running());
}

#[test]
fn the_chat_lists_its_keys() {
    let mut h = start();
    h.wait_for("ready", |s| s.contains("ready"));
    let screen = h.t.screen();
    assert!(screen.contains("F12 screens"), "{screen}");
    assert!(screen.contains("Ctrl+C quit"), "{screen}");
}

#[test]
fn the_menu_says_how_to_go_back_and_the_home_menu_says_q_quits() {
    let mut h = start();
    h.wait_for("ready", |s| s.contains("ready"));
    h.press("f12");
    let pushed = h.t.screen();
    assert!(pushed.contains("esc/q back"), "{pushed}");
    assert!(!pushed.contains("q quits"), "{pushed}");
    let home = start_debug();
    let screen = home.t.screen();
    assert!(screen.contains("q quits"), "{screen}");
    assert!(!screen.contains("esc/q back"), "{screen}");
}

/// Pets and voice are modal sheets over the session page (architecture §1,
/// amendment 2026-11-19b), not pages: the transcript stays on the screen
/// while they are up, and Escape returns to exactly the chat before.
#[test]
fn pets_is_a_sheet_over_the_session_page() {
    let mut h = start();
    crate::chat::chat_with_history(&mut h);
    let before = h.t.screen();
    h.t.type_text("/pets");
    h.press("enter");
    h.t.pump_for(crate::common::ms(800));
    let open = h.t.screen();
    assert!(
        open.contains("• Hello"),
        "the sheet covers the session page:\n{open}"
    );
    assert!(open.contains("empty slot"), "the sheet is not up:\n{open}");
    h.press("escape");
    h.t.pump_for(crate::common::ms(600));
    assert!(h.t.screen().contains("• Hello"));
    assert_eq!(
        h.t.screen().lines().count(),
        before.lines().count(),
        "back on the session page"
    );
}

/// Import is a step of the startup page (architecture §1, amendment
/// 2026-11-19b: one StartupPage with steps, a step controller), not a page of
/// its own: `/import` pushes the StartupPage on the import step, and Escape
/// returns to the chat.
#[test]
fn import_is_a_step_of_the_startup_page() {
    use codex_tui_rtwk::screens::onboarding::{StartupPage, Step};
    use rtwk_test::find;

    let mut h = start();
    crate::chat::chat_with_history(&mut h);
    h.t.type_text("/import");
    h.press("enter");
    h.t.pump_for(crate::common::ms(800));
    let page = find::by_type::<StartupPage>();
    assert_eq!(page.count(&h.t), 1, "{}", h.t.screen());
    assert!(
        h.t.screen().contains("import (S06): empty slot"),
        "{}",
        h.t.screen()
    );
    assert_eq!(Step::Import.index(), 3);
    h.press("escape");
    h.t.pump_for(crate::common::ms(600));
    assert_eq!(page.count(&h.t), 0);
    assert!(h.t.screen().contains("• Hello"));
}
