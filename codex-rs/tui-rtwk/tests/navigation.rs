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

const NOT_YET: &str = "Not implemented yet: expanded transcript";

/// A key or command whose action is not built yet opens a small popup, never
/// a silent no-op (the person, 2026-11-19): Ctrl+T and `/diff` on the session
/// page until chat-live builds the expanded transcript. Esc and Enter close.
#[test]
fn ctrl_t_and_diff_open_the_not_implemented_popup() {
    for trigger in ["ctrl+t", "/diff"] {
        let mut h = start();
        crate::chat::chat_with_history(&mut h);
        let before = h.t.screen();
        if trigger.starts_with('/') {
            h.t.type_text(trigger);
            h.press("enter");
        } else {
            h.press(trigger);
        }
        h.t.pump_for(crate::common::ms(800));
        assert!(
            h.t.screen().contains(NOT_YET),
            "{trigger}:\n{}",
            h.t.screen()
        );
        for close in ["escape", "enter"] {
            h.press(close);
            h.t.pump_for(crate::common::ms(600));
            assert!(
                !h.t.screen().contains(NOT_YET),
                "{trigger} {close}:\n{}",
                h.t.screen()
            );
            if close == "escape" {
                // Again: the same trigger reopens it.
                if trigger.starts_with('/') {
                    h.t.type_text(trigger);
                    h.press("enter");
                } else {
                    h.press(trigger);
                }
                h.t.pump_for(crate::common::ms(800));
                assert!(h.t.screen().contains(NOT_YET));
            }
        }
        assert_eq!(h.t.screen(), before, "back on the session page");
    }
}

/// The session page's bottom menu: the oracle's idle footer
/// (`footer_shortcuts_default`): the shortcuts hint at the left.
#[test]
fn the_session_page_footer_is_the_oracle_idle_footer() {
    let mut h = start();
    h.wait_for("ready", |s| s.contains("ready"));
    let screen = h.t.screen();
    let last = screen.lines().last().unwrap_or_default().to_string();
    assert!(last.starts_with("  ? for shortcuts"), "{screen}");
    assert!(!screen.contains("F12 screens"), "{screen}");
}

// ---------------------------------------------------------------- the edges
// between the five pages (architecture §1, Amendment 2026-11-19b): the
// Session is the Navigator's home, every other page is pushed over it and
// pops back to it; "replace" (a page ends and the session shows) is that pop.

fn open_over_the_session(command: &str) -> Harness {
    let mut h = start();
    crate::chat::chat_with_history(&mut h);
    h.t.type_text(command);
    h.press("enter");
    h.t.pump_for(crate::common::ms(800));
    h
}

fn assert_session_back(h: &Harness) {
    let screen = h.t.screen();
    assert!(screen.contains("› hi"), "transcript lost:\n{screen}");
    assert!(screen.contains("• Hello"), "transcript lost:\n{screen}");
    assert!(screen.contains("ready"), "no session footer:\n{screen}");
}

#[test]
fn session_to_agents_and_back_keeps_the_session() {
    use codex_tui_rtwk::screens::agents;
    use rtwk_test::find;

    let mut h = open_over_the_session("/agents");
    let page = find::by_type::<agents::Screen>();
    assert_eq!(page.count(&h.t), 1, "{}", h.t.screen());
    h.press("down");
    h.press("escape");
    h.t.pump_for(crate::common::ms(600));
    assert_eq!(page.count(&h.t), 0, "{}", h.t.screen());
    assert_session_back(&h);
    // Again: the page is there again and Esc still returns.
    h.t.type_text("/agents");
    h.press("enter");
    h.t.pump_for(crate::common::ms(800));
    assert_eq!(page.count(&h.t), 1, "{}", h.t.screen());
    h.press("escape");
    h.t.pump_for(crate::common::ms(600));
    assert_session_back(&h);
}

/// Enter on a task of the command center resumes it; the answer (here the
/// shell's `Started`, which bumps the same counter) ends the page and the
/// session shows.
#[test]
fn agents_to_session_when_a_task_opens() {
    use codex_app_server_protocol::Thread;
    use codex_tui_core::app::AppEv;
    use codex_tui_core::screens::ScreenEv;
    use codex_tui_core::screens::agents::Ev;
    use codex_tui_rtwk::screens::agents;
    use rtwk_test::find;

    let mut h = open_over_the_session("/agents");
    let page = find::by_type::<agents::Screen>();
    assert_eq!(page.count(&h.t), 1, "{}", h.t.screen());
    let thread: Thread = serde_json::from_value(serde_json::json!({
        "id": "t-2", "environments": null, "extra": null, "sessionId": "t-2",
        "forkedFromId": null, "attachedToThreadId": null, "parentThreadId": null,
        "preview": "t-2", "ephemeral": false, "section": null, "sectionEnteredAt": null,
        "projectId": null, "historyMode": "legacy", "modelProvider": "openai",
        "model": null, "reasoningEffort": null, "createdAt": 1_700_000_000,
        "updatedAt": 1_700_000_000, "recencyAt": 1_700_000_000, "status": {"type": "idle"},
        "path": null, "cwd": "/tmp", "cliVersion": "0.0.0", "originator": null,
        "source": "cli", "canAcceptDirectInput": true, "threadSource": null,
        "agentNickname": null, "agentRole": null, "gitInfo": null, "name": "t-2",
        "daybreakEnabled": null, "turns": []
    }))
    .unwrap();
    let epoch = h.fake.epoch().expect("connected");
    h.fake.inject(
        epoch,
        AppEv::Screen(ScreenEv::Agents(Ev::Started(Box::new(thread)))),
    );
    h.settle();
    h.t.pump_for(crate::common::ms(600));
    assert_eq!(page.count(&h.t), 0, "{}", h.t.screen());
    assert!(h.t.screen().contains("ready"), "{}", h.t.screen());
}

#[test]
fn session_to_resume_and_back() {
    use codex_tui_rtwk::screens::resume;
    use rtwk_test::find;

    let mut h = open_over_the_session("/resume");
    let page = find::by_type::<resume::Screen>();
    assert_eq!(page.count(&h.t), 1, "{}", h.t.screen());
    h.press("escape");
    h.t.pump_for(crate::common::ms(600));
    assert_eq!(page.count(&h.t), 0, "{}", h.t.screen());
    assert_session_back(&h);
}

/// Enter on a row of the picker ends the page (the session replaces it): the
/// picker returns its choice.
#[test]
fn resume_picks_a_row_and_returns_to_the_session() {
    use crate::common::start_sized;
    use codex_tui_rtwk::screens::resume;
    use rtwk_test::find;

    let mut h = start_sized(true, 80, 24);
    h.open_debug_row("resume", Some("table"));
    let page = find::by_type::<resume::Screen>();
    assert_eq!(page.count(&h.t), 1, "{}", h.t.screen());
    h.press("enter");
    h.t.pump_for(crate::common::ms(600));
    assert_eq!(page.count(&h.t), 0, "{}", h.t.screen());
}

#[test]
fn session_to_usage_and_back() {
    use codex_tui_rtwk::screens::usage;
    use rtwk_test::find;

    let mut h = open_over_the_session("/usage");
    let page = find::by_type::<usage::Screen>();
    assert_eq!(page.count(&h.t), 1, "{}", h.t.screen());
    h.press("escape");
    h.t.pump_for(crate::common::ms(600));
    assert_eq!(page.count(&h.t), 0, "{}", h.t.screen());
    assert_session_back(&h);
}

/// The command line's entries: `codex-rtwk` starts on the session,
/// `codex-rtwk agents` on the command center, `codex-rtwk resume` on the
/// picker; the entry page has no page to return to (agents) or returns to
/// the session (resume).
#[test]
fn the_cli_entries_start_at_the_right_page() {
    use codex_tui_rtwk::Cli;
    use codex_tui_rtwk::screens::{agents, resume};
    use rtwk_test::find;

    let args = |words: &[&str]| Cli::parse(words.iter().map(|w| (*w).to_string())).unwrap();
    assert!(!args(&[]).agents && !args(&[]).resume);
    assert!(args(&["agents"]).agents);
    assert!(args(&["resume"]).resume);

    let agents_page = find::by_type::<agents::Screen>();
    let resume_page = find::by_type::<resume::Screen>();

    let mut h = crate::common::start_app(|app| app);
    h.t.pump_for(crate::common::ms(400));
    assert_eq!(agents_page.count(&h.t), 0);
    assert_eq!(resume_page.count(&h.t), 0);

    let mut h = crate::common::start_app(|app| app.with_start_agents(true));
    h.t.pump_for(crate::common::ms(400));
    assert_eq!(agents_page.count(&h.t), 1, "{}", h.t.screen());
    assert_eq!(resume_page.count(&h.t), 0);

    let mut h = crate::common::start_app(|app| app.with_start_resume(true));
    h.t.pump_for(crate::common::ms(400));
    assert_eq!(resume_page.count(&h.t), 1, "{}", h.t.screen());
    assert_eq!(agents_page.count(&h.t), 0);
    h.press("escape");
    h.t.pump_for(crate::common::ms(600));
    assert_eq!(resume_page.count(&h.t), 0, "{}", h.t.screen());
}

// ---------------------------------------------------------------- the footers

/// The usage page's footer rows are the oracle's (`hints.rs` `footer_lines`,
/// pinned by the summary snapshots of tests/usage.rs); every key the footer
/// shows is bound: `esc` closes, `tab` and the digits pick the report, and
/// `z`, `?` and `R` (not built yet) open the not-implemented popup.
#[test]
fn usage_footer_keys_are_bound() {
    use codex_tui_rtwk::screens::usage;
    use rtwk_test::find;

    let mut h = open_over_the_session("/usage");
    let screen = h.t.screen();
    assert!(
        screen.contains("tab/1\u{2013}6 report · z dashboard · ? help · R refresh"),
        "{screen}"
    );
    assert!(screen.contains("esc back"), "{screen}");
    for (key, action) in [("z", "dashboard"), ("?", "help"), ("R", "refresh")] {
        h.press(key);
        h.t.pump_for(crate::common::ms(600));
        let screen = h.t.screen();
        assert!(
            screen.contains(&format!("Not implemented yet: {action}")),
            "{key}:\n{screen}"
        );
        h.press("enter");
        h.t.pump_for(crate::common::ms(600));
        assert!(
            !h.t.screen().contains("Not implemented yet"),
            "{key}: the popup stays"
        );
    }
    h.press("tab");
    h.t.pump_for(crate::common::ms(300));
    assert!(!h.t.screen().contains("Not implemented yet"));
    assert_eq!(find::by_type::<usage::Screen>().count(&h.t), 1);
}

/// The resume picker's footer keys not built yet open the popup (the rows
/// themselves are the oracle's, tests/resume.rs); `esc` returns to the session.
#[test]
fn resume_footer_keys_are_bound() {
    let mut h = open_over_the_session("/resume");
    for (key, action) in [
        ("tab", "focus sort/filter"),
        ("right", "change option"),
        ("ctrl+o", "dense view"),
        ("ctrl+t", "transcript"),
        ("ctrl+e", "expand"),
    ] {
        h.press(key);
        h.t.pump_for(crate::common::ms(600));
        let screen = h.t.screen();
        assert!(
            screen.contains(&format!("Not implemented yet: {action}")),
            "{key}:\n{screen}"
        );
        h.press("enter");
        h.t.pump_for(crate::common::ms(600));
    }
    h.press("escape");
    h.t.pump_for(crate::common::ms(600));
    assert_session_back(&h);
}

/// `?` on an empty composer opens the shortcuts (not built yet: the popup);
/// with text in it the key passes through and types the character. Decided at
/// the field by `EditingKeys`, as the oracle's footer shortcut is.
#[test]
fn question_mark_is_the_shortcut_on_an_empty_composer_only() {
    let mut h = start();
    h.wait_for("ready", |s| s.contains("ready"));
    h.t.type_text("?");
    h.t.pump_for(crate::common::ms(600));
    let screen = h.t.screen();
    assert!(
        screen.contains("Not implemented yet: shortcut overlay"),
        "{screen}"
    );
    h.press("enter");
    h.t.pump_for(crate::common::ms(600));
    assert!(!h.t.screen().contains("Not implemented yet"));

    h.t.type_text("a?");
    h.t.pump_for(crate::common::ms(300));
    let screen = h.t.screen();
    assert!(!screen.contains("Not implemented yet"), "{screen}");
    assert!(screen.contains("a?"), "{screen}");
}
