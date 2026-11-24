// test root: pages
//! Proofs that upstream `openai/codex` TUI issues, which the port's design should have
//! fixed, are fixed: each test names its issue and fails if the port still has the bug.
//! (The resume snapshot of #43952 is a core test, `core/tests/issue_proofs.rs`.)

#![allow(clippy::expect_used)]

use codex_tui_core::app::{AppEv, Command, Intent};
use codex_tui_core::fixtures::builders::{agent_completed, agent_delta};
use codex_tui_core::ids::ThreadId;
use codex_tui_core::screens::ScreenEv;
use codex_tui_core::screens::agents::{Ev as AgentsEv, Page};
use codex_tui_rtwk::services::backend::fake::FAKE_THREAD;
use rtwk_test::Options;
use serde_json::{Value, json};

use crate::common::{Harness, start_driven_with, start_quittable};
use crate::kit::{Owner, Screen};

/// The column of the first `needle` of row `y`, in cells (the rows here are single width).
fn column_of(s: &Screen, y: usize, needle: &str) -> u16 {
    let row = &s.rows()[y];
    let at = row.find(needle).expect("the text is in the row");
    u16::try_from(row[..at].chars().count()).expect("a column")
}

/// openai/codex#12882 (the prompt does not support Shift+Arrow selection): Shift+Left selects
/// the characters before the cursor, drawn reversed, and a typed character replaces them.
#[test]
fn issue_12882_shift_left_selects_in_the_composer() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 12).live().open();
    s.type_text("hello");
    let y = s
        .rows()
        .iter()
        .position(|row| row.contains("› hello"))
        .expect("the composer");
    let x = column_of(&s, y, "hello");
    let y = u16::try_from(y).expect("a row");
    assert!(
        !s.reversed(x + 3, y) && !s.reversed(x + 4, y),
        "{}",
        s.text()
    );

    s.keys("shift+left shift+left");
    assert!(s.reversed(x + 3, y), "`l` is selected:\n{}", s.text());
    assert!(s.reversed(x + 4, y), "`o` is selected:\n{}", s.text());
    assert!(!s.reversed(x + 2, y), "`l` before it is not:\n{}", s.text());
    s.assert_composer("hello");

    s.type_text("X");
    s.assert_composer("helX");
}

/// The bytes of Ctrl+C and Ctrl+D as a kitty-protocol terminal sends them
/// (`CSI key:shifted:base-layout ; mods u`): on a Latin layout the key is `c` / `d`; on a
/// Russian layout the same physical keys give `с` (U+0441) / `в` (U+0432) and the terminal adds
/// the base-layout key, which is what the shortcut is matched by.
const CTRL_C: [(&str, &[u8]); 2] = [("latin", b"\x1b[99;5u"), ("cyrillic", b"\x1b[1089::99;5u")];
const CTRL_D: [(&str, &[u8]); 2] = [
    ("latin", b"\x1b[100;5u"),
    ("cyrillic", b"\x1b[1074::100;5u"),
];

/// A quittable chat with the session ready and nothing running.
fn idle_chat() -> Harness {
    let mut h = start_quittable(false);
    h.wait_idle();
    h
}

/// openai/codex#35599 (Ctrl+C and Ctrl+D are not recognized with a Cyrillic layout under kitty
/// keyboard enhancement): Ctrl+D quits an idle chat, on either layout.
#[test]
fn issue_35599_ctrl_d_quits_with_a_cyrillic_layout() {
    let stayed: Vec<&str> = CTRL_D
        .into_iter()
        .filter(|(_, bytes)| {
            let mut h = idle_chat();
            h.t.input(bytes);
            h.advance(200);
            h.t.is_running()
        })
        .map(|(layout, _)| layout)
        .collect();
    assert_eq!(
        stayed,
        Vec::<&str>::new(),
        "ctrl+d did not quit on these layouts"
    );
}

/// openai/codex#35599: Ctrl+C quits an idle chat, on either layout.
#[test]
fn issue_35599_ctrl_c_quits_an_idle_chat_with_a_cyrillic_layout() {
    let stayed: Vec<&str> = CTRL_C
        .into_iter()
        .filter(|(_, bytes)| {
            let mut h = idle_chat();
            h.t.input(bytes);
            h.advance(200);
            h.t.is_running()
        })
        .map(|(layout, _)| layout)
        .collect();
    assert_eq!(
        stayed,
        Vec::<&str>::new(),
        "ctrl+c did not quit on these layouts"
    );
}

/// openai/codex#35599: Ctrl+C interrupts a running turn and keeps the app, on either layout.
#[test]
fn issue_35599_ctrl_c_interrupts_a_running_turn_with_a_cyrillic_layout() {
    for (layout, bytes) in CTRL_C {
        let mut h = idle_chat();
        h.fake
            .script_turn(vec![agent_delta(FAKE_THREAD, "turn-1", "msg-1", "so far")]);
        h.t.type_text("go");
        h.press("enter");
        h.wait_for("the turn to run", |s| s.contains("esc to interrupt"));
        h.t.input(bytes);
        h.applied();
        h.advance(200);
        let interrupts = h
            .fake
            .commands()
            .iter()
            .filter(|command| matches!(command, Command::InterruptTurn { .. }))
            .count();
        assert_eq!(interrupts, 1, "ctrl+c on a {layout} layout");
        assert!(h.t.is_running(), "{layout}: the interrupt keeps the app");
    }
}

/// The background of the composer's first text cell, as `(red, text row)`.
fn composer_background(h: &Harness) -> (u8, usize) {
    let screen = h.t.screen();
    let y = screen
        .lines()
        .position(|row| row.contains("› "))
        .expect("the composer");
    (h.t.cell(3, y).bg.r(), y)
}

/// openai/codex#45163 and #38575 (the palette is read once at startup; the composer is left
/// unreadable after the terminal's light/dark change): after the terminal reports a new colour
/// scheme (mode 2031, `CSI ? 997 ; 2 n`) and its new default background (the answer to the
/// colours the app asks for again), the composer's surface is the light terminal's, not the
/// startup one's.
#[test]
fn issue_45163_a_color_scheme_event_after_startup_rebuilds_the_theme() {
    let options = Options::new().size(80, 12).background_rgb(0x1e_1e_1e);
    let mut h = start_driven_with(false, options);
    h.connect();
    h.quiesce();
    let (dark, _) = composer_background(&h);
    assert!(
        dark < 0x80,
        "a dark terminal shades toward white: {dark:#x}"
    );

    h.t.input(b"\x1b[?997;2n");
    h.t.input(b"\x1b]11;rgb:ffff/ffff/ffff\x1b\\");
    h.advance(300);
    let (light, _) = composer_background(&h);
    assert!(
        light > 0xc0,
        "the composer kept the startup palette: {light:#x}\n{}",
        h.t.screen()
    );

    h.t.input(b"\x1b[?997;1n");
    h.t.input(b"\x1b]11;rgb:1e1e/1e1e/1e1e\x1b\\");
    h.advance(300);
    assert!(composer_background(&h).0 < 0x80, "and back to dark");
}

const OTHER: &str = "other-1";

/// The server's thread `id`, idle, in `/work`.
fn listed_thread(id: &str) -> Value {
    json!({
        "id": id, "environments": null, "extra": null, "sessionId": id,
        "forkedFromId": null, "attachedToThreadId": null, "parentThreadId": null,
        "preview": id, "ephemeral": false, "section": null, "sectionEnteredAt": null,
        "projectId": null, "historyMode": "legacy", "modelProvider": "openai",
        "model": null, "reasoningEffort": null, "createdAt": 1_700_000_000,
        "updatedAt": 1_700_000_100, "recencyAt": 1_700_000_100, "status": {"type": "idle"},
        "path": null, "cwd": "/work", "cliVersion": "0.0.0", "originator": null,
        "source": "cli", "canAcceptDirectInput": true, "threadSource": null,
        "agentNickname": null, "agentRole": null, "gitInfo": null, "name": id,
        "daybreakEnabled": null, "turns": []
    })
}

/// The server's answer to `thread/resume` of `OTHER`: one finished turn of the rows
/// `other 00` .. `other 39`.
fn resume_of_other() -> Value {
    let items: Vec<Value> = (0..40)
        .map(|i| {
            json!({"type": "agentMessage", "moderation": null, "id": format!("o-{i}"),
                "text": format!("other {i:02}"), "phase": "final_answer",
                "memoryCitation": null, "delivery": null, "questions": null,
                "startedAtMs": null, "completedAtMs": null})
        })
        .collect();
    let mut thread = listed_thread(OTHER);
    thread["turns"] = json!([{
        "id": "turn-o", "rootTurnId": null, "items": items, "itemsView": "full",
        "status": "completed", "error": null, "startedAt": 1_700_000_000,
        "completedAt": 1_700_000_010, "durationMs": 10_000
    }]);
    json!({
        "thread": thread, "model": "gpt-5", "modelProvider": "openai", "serviceTier": null,
        "cwd": "/work", "instructionSources": [],
        "approvalPolicy": "on-request", "approvalsReviewer": "user",
        "sandbox": {"type": "dangerFullAccess"}, "reasoningEffort": null
    })
}

fn open_thread(h: &mut Harness, id: &str) {
    h.inject([AppEv::Intent(Intent::OpenThread(ThreadId::new(id)))]);
    h.advance(300);
}

/// The chat with the thread `FAKE_THREAD` holding the rows `row 00` .. `row 39`, scrolled up two
/// pages from its end, and what the screen shows then.
fn scrolled_up() -> (Harness, String) {
    let mut h = start_driven_with(false, Options::new().size(80, 12));
    h.connect();
    h.quiesce();
    h.fake
        .respond(|request| (request["method"] == "thread/resume").then(|| Ok(resume_of_other())));
    let rows = (0..40).map(|i| {
        AppEv::Backend(agent_completed(
            FAKE_THREAD,
            "turn-1",
            &format!("msg-{i}"),
            &format!("row {i:02}"),
        ))
    });
    h.inject(rows);
    h.advance(300);
    assert!(h.t.screen().contains("row 39"), "{}", h.t.screen());
    h.press("pageup");
    h.press("pageup");
    let scrolled = h.t.screen();
    assert!(!scrolled.contains("row 39"), "{scrolled}");
    assert!(scrolled.contains("Back to bottom"), "{scrolled}");
    (h, scrolled)
}

/// `OTHER` is opened from the thread list (resumed from the server).
fn open_other(h: &mut Harness) {
    h.inject([AppEv::Screen(ScreenEv::Agents(AgentsEv::Listed(Page {
        threads: vec![serde_json::from_value(listed_thread(OTHER)).expect("a thread")],
        unread: Vec::new(),
        next_cursor: None,
        append: false,
        pins: None,
    })))]);
    open_thread(h, OTHER);
    h.advance(300);
}

/// The first `row ..` line of `screen`.
fn first_row(screen: &str) -> Option<String> {
    screen
        .lines()
        .find(|line| line.contains("row "))
        .map(str::to_owned)
}

/// openai/codex#57499 (returning from /side resets the main conversation's scroll position):
/// every thread owns its reader, so a thread left scrolled up is as it was when the person
/// comes back. (`/side` is not in the port yet, `'/side' is not available yet.`; the switch of
/// the active thread is the one `OpenThread` makes, and /side's return is a switch back.)
#[test]
fn issue_57499_a_thread_left_scrolled_up_is_as_it_was_on_return() {
    let (mut h, scrolled) = scrolled_up();
    open_other(&mut h);
    assert!(h.t.screen().contains("other "), "{}", h.t.screen());
    open_thread(&mut h, FAKE_THREAD);
    let back = h.t.screen();
    assert!(!back.contains("row 39"), "{back}");
    assert!(back.contains("Back to bottom"), "{back}");
    assert_eq!(
        first_row(&back),
        first_row(&scrolled),
        "{back}\n--\n{scrolled}"
    );
}

/// The other half of #57499: the thread switched to has its own reader, so it opens at its own
/// end and does not take the offset of the thread that was scrolled.
#[test]
fn issue_57499_a_thread_opened_after_another_was_scrolled_starts_at_its_own_end() {
    let (mut h, _) = scrolled_up();
    open_other(&mut h);
    let other = h.t.screen();
    assert!(other.contains("other 39"), "{other}");
    assert!(!other.contains("Back to bottom"), "{other}");
}
