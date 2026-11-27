//! Open side tabs share their parent and keep their own drafts and transcript writers.

use std::sync::atomic::{AtomicUsize, Ordering};

use codex_tui_core::app::{AppEv, Command, Ingress, LocalSideStorage};
use codex_tui_core::fixtures::builders::{agent_completed, turn_completed, turn_started};
use codex_tui_core::ids::ThreadId;
use codex_tui_core::keymap::{Keymap, KeymapSpec, actions::GLOBAL_SIDEBAR_UP};
use codex_tui_core::threads::TurnState;
use codex_tui_rtwk::app::UiApp;
use codex_tui_rtwk::services::backend::fake::FAKE_THREAD;
use rtwk_core::input::{Mods, MouseKind};

use crate::common::Harness;
use crate::side_pane::configured_workspace;
use crate::side_return::{SIDE, forked};

const SECOND: &str = "side-2";
const THIRD: &str = "side-3";

fn workspace(attached: bool) -> Harness {
    let mut h = configured_workspace(true, true, None);
    if attached {
        let epoch = h
            .fake
            .epoch()
            .unwrap_or_else(|| panic!("workspace transport"));
        h.fake.inject_ingress(Ingress::local_side_storage(
            epoch,
            epoch,
            LocalSideStorage::Embedded,
        ));
        h.advance(100);
    }
    let forks = AtomicUsize::new(0);
    h.fake
        .respond(move |request| match request["method"].as_str()? {
            "thread/read" if attached && request["params"]["threadId"] == FAKE_THREAD => {
                let mut thread = forked()["thread"].clone();
                thread["id"] = FAKE_THREAD.into();
                thread["sessionId"] = FAKE_THREAD.into();
                thread["ephemeral"] = false.into();
                thread["historyMode"] = "paginated".into();
                thread["path"] = "/tmp/main.jsonl".into();
                Some(Ok(serde_json::json!({"thread": thread})))
            }
            "thread/fork" => {
                let (id, name) = match forks.fetch_add(1, Ordering::Relaxed) {
                    0 => (SIDE, ""),
                    1 => (SECOND, "Second side"),
                    2 => (THIRD, "Third side"),
                    _ => panic!("unexpected fourth fork"),
                };
                let mut reply = forked();
                let thread = &mut reply["thread"];
                thread["id"] = id.into();
                thread["sessionId"] = id.into();
                thread["preview"] = "".into();
                thread["name"] = name.into();
                thread["forkedFromId"] = FAKE_THREAD.into();
                if attached {
                    assert_eq!(request["params"]["attachToSource"], true);
                    thread["attachedToThreadId"] = FAKE_THREAD.into();
                    thread["ephemeral"] = false.into();
                    thread["historyMode"] = "paginated".into();
                    thread["path"] = format!("/tmp/{id}.jsonl").into();
                }
                Some(Ok(reply))
            }
            "thread/inject_items" | "thread/unsubscribe" | "turn/interrupt" => {
                Some(Ok(serde_json::json!({})))
            }
            _ => None,
        });
    h
}

fn open_workspace(attached: bool) -> Harness {
    let mut h = workspace(attached);
    send_line(&mut h, "/side first-side");
    assert_active(&h, SIDE);
    h
}

fn send_line(h: &mut Harness, line: &str) {
    h.t.type_text(line);
    h.press("enter");
    h.advance(300);
    h.quiesce();
}

fn assert_active(h: &Harness, expected: &str) {
    let active =
        h.t.read_app(|app: &UiApp| app.core.session().active.clone());
    assert_eq!(
        active.as_ref().map(ThreadId::as_str),
        Some(expected),
        "wire requests: {:?}\n{}",
        h.fake.requests(),
        h.t.screen()
    );
}

fn completed_answer(h: &mut Harness, thread: &str, answer: &str) {
    let turn = format!("{thread}-{answer}");
    h.inject([
        AppEv::Backend(turn_started(thread, &turn)),
        AppEv::Backend(agent_completed(thread, &turn, "answer", answer)),
        AppEv::Backend(turn_completed(thread, &turn)),
    ]);
    h.advance(150);
}

fn finish_opening_turn(h: &mut Harness, thread: &str, answer: &str) {
    let key = ThreadId::new(thread);
    let turn =
        h.t.read_app(move |app: &UiApp| match app.core.thread(&key)?.turn() {
            TurnState::Running(turn) => Some(turn.turn.clone()),
            TurnState::Idle => None,
        })
        .unwrap_or_else(|| panic!("no opening turn for {thread}"));
    h.inject([
        AppEv::Backend(agent_completed(thread, turn.as_str(), "answer", answer)),
        AppEv::Backend(turn_completed(thread, turn.as_str())),
    ]);
    h.advance(150);
}

fn click_label(h: &mut Harness, label: &str) {
    let (x, y) =
        h.t.lines()
            .iter()
            .enumerate()
            .find_map(|(y, line)| line.find(label).map(|at| (line[..at].chars().count(), y)))
            .unwrap_or_else(|| panic!("missing {label}:\n{}", h.t.screen()));
    h.t.click(x as u16, y as u16);
    h.advance(150);
}

fn submitted(h: &Harness, thread_id: &str, text: &str) -> bool {
    h.fake.commands().iter().any(|command| {
        matches!(command, Command::StartTurn { thread, message, .. }
            if thread.as_str() == thread_id && message.text == text)
    })
}

#[test]
fn native_tabs_select_open_children_by_keyboard_and_mouse_with_independent_drafts() {
    let mut h = open_workspace(false);
    finish_opening_turn(&mut h, SIDE, "FIRST HISTORY");
    h.t.type_text("first-draft");
    h.press("left");
    h.press("f12 h");
    send_line(&mut h, "/side second-side");
    assert_active(&h, SECOND);
    finish_opening_turn(&mut h, SECOND, "SECOND HISTORY");
    h.t.type_text("second-draft");
    h.advance(120);
    let bar = h.t.lines().first().cloned().unwrap_or_default();
    assert!(
        bar.contains("first-side") && bar.contains("Second side"),
        "{bar}"
    );

    h.press("f12 t");
    h.advance(150);
    assert_active(&h, FAKE_THREAD);
    assert!(h.t.screen().contains("New side chat"));
    h.press("down");
    h.press("enter");
    h.advance(150);
    assert_active(&h, SIDE);
    assert!(h.t.screen().contains("FIRST HISTORY") && h.t.screen().contains("first-draft"));
    assert!(!h.t.screen().contains("SECOND HISTORY"));
    assert!(
        !submitted(&h, SIDE, "first-draft"),
        "Enter selects the existing child without submitting its draft"
    );
    h.t.type_text("!");
    h.advance(120);
    assert!(h.t.screen().contains("first-draf!t"), "{}", h.t.screen());

    click_label(&mut h, "Second side");
    assert_active(&h, SECOND);
    assert!(h.t.screen().contains("SECOND HISTORY") && h.t.screen().contains("second-draft"));
    for alias in ["/btw", "/side"] {
        h.press("f12 h");
        send_line(&mut h, alias);
        assert_active(&h, SECOND);
        assert!(h.t.screen().contains("second-draft"));
    }
    h.press("f12 u");
    assert_active(&h, SIDE);
    assert!(h.t.screen().contains("first-draf!t"));
    h.press("f12 h");
    send_line(&mut h, "/btw one more question");
    assert_active(&h, THIRD);
    assert!(submitted(&h, THIRD, "one more question"));
    click_label(&mut h, "Second side");
    assert_active(&h, SECOND);
    assert!(h.t.screen().contains("second-draft"));
    assert_eq!(
        h.fake
            .requests()
            .iter()
            .filter(|r| r["method"] == "thread/fork")
            .count(),
        3
    );
}

#[test]
fn side_tabs_restore_the_readers_position_and_enable_the_default_shift_tab_chord() {
    let default = Keymap::resolve_for_workspace(&KeymapSpec::default(), true)
        .unwrap_or_else(|error| panic!("workspace defaults: {error}"));
    assert!(
        default
            .specs(GLOBAL_SIDEBAR_UP)
            .iter()
            .any(|spec| spec == "leader shift-tab")
    );

    let mut h = open_workspace(false);
    for i in 0..18 {
        completed_answer(&mut h, SIDE, &format!("first reader line {i:02}"));
    }
    assert!(h.t.screen().contains("first reader line 17"));
    for _ in 0..24 {
        h.t.mouse(MouseKind::WheelUp, 105, 9, 0, Mods::NONE);
        h.advance(80);
    }
    assert!(h.t.screen().contains("first reader line 00"));
    h.press("f12 h");
    send_line(&mut h, "/side second-side");
    assert_active(&h, SECOND);
    click_label(&mut h, "first-side");
    assert_active(&h, SIDE);
    assert!(
        h.t.screen().contains("first reader line 00"),
        "{}",
        h.t.screen()
    );
    assert!(!h.t.screen().contains("first reader line 17"));
}

#[test]
fn close_uses_storage_and_leaves_the_previous_child_alive() {
    for attached in [false, true] {
        let mut h = open_workspace(attached);
        finish_opening_turn(&mut h, SIDE, "FIRST HISTORY");
        h.press("f12 h");
        send_line(&mut h, "/side second-side");
        finish_opening_turn(&mut h, SECOND, "SECOND HISTORY");
        if attached {
            close_tab(&mut h, "Second side", 0);
        } else {
            h.press("f12 c");
            assert!(h.t.screen().contains("lose this temporary chat’s history"));
            h.press("right");
            h.press("left");
            h.press("enter");
            assert_active(&h, SECOND);
            h.press("f12 c");
            h.press("right");
            h.press("enter");
        }
        h.advance(250);
        assert!(!h.t.screen().contains("Close side chat?"));
        assert_active(&h, SIDE);
        assert!(h.t.screen().contains("FIRST HISTORY"));
        assert!(!h.t.screen().contains("SECOND HISTORY"));
        let requests = h.fake.requests();
        assert!(
            requests
                .iter()
                .any(|r| r["method"] == "thread/unsubscribe" && r["params"]["threadId"] == SECOND)
        );
        assert!(
            !requests
                .iter()
                .any(|r| r["method"] == "thread/unsubscribe" && r["params"]["threadId"] == SIDE)
        );
        assert!(!requests.iter().any(|r| r["method"] == "thread/archive"));
    }
}

fn close_tab(h: &mut Harness, label: &str, button: u8) {
    let bar = h.t.lines().first().cloned().unwrap_or_default();
    let index = bar
        .find(label)
        .unwrap_or_else(|| panic!("missing {label}: {bar}"));
    let x = if button == 1 {
        bar[..index].chars().count()
    } else {
        let close = bar[index..]
            .find('×')
            .unwrap_or_else(|| panic!("missing close: {bar}"));
        bar[..index + close].chars().count()
    };
    let x = u16::try_from(x).expect("visible tab column");
    h.t.mouse(MouseKind::Down, x, 0, button, Mods::NONE);
    h.t.mouse(MouseKind::Up, x, 0, button, Mods::NONE);
    h.advance(150);
}

#[test]
fn the_empty_workspace_chooser_creates_and_plus_can_open_a_fresh_child() {
    let mut h = workspace(false);
    assert_active(&h, FAKE_THREAD);
    h.press("f12 t");
    h.advance(150);
    assert!(h.t.screen().contains("New side chat"), "{}", h.t.screen());
    h.press("enter");
    h.advance(300);
    assert_active(&h, SIDE);
    assert!(!submitted(&h, SIDE, "New side chat"));
    click_label(&mut h, "+");
    assert_active(&h, FAKE_THREAD);
    assert!(h.t.screen().contains("New side chat"), "{}", h.t.screen());
    h.press("enter");
    h.advance(300);
    assert_active(&h, SECOND);
    let forks = h
        .fake
        .requests()
        .into_iter()
        .filter(|r| r["method"] == "thread/fork")
        .count();
    assert_eq!(forks, 2);
}

#[test]
fn closing_an_inactive_attached_child_uses_its_own_loss_and_keeps_the_composer() {
    for button in [0, 1] {
        let mut h = open_workspace(true);
        finish_opening_turn(&mut h, SIDE, "FIRST HISTORY");
        h.t.type_text("first parked input");
        h.press("f12 h");
        send_line(&mut h, "/side second-side");
        finish_opening_turn(&mut h, SECOND, "SECOND HISTORY");
        h.t.type_text("second active input");
        h.advance(120);

        close_tab(&mut h, "first-side", button);
        let dialog = h.t.screen();
        assert!(
            dialog.contains("Close side chat?") && dialog.contains("first-side"),
            "{dialog}"
        );
        assert!(dialog.contains("discard unsent input"), "{dialog}");
        h.press("esc");
        assert_active(&h, SECOND);
        assert!(h.t.screen().contains("second active input"));
        assert!(
            !h.fake
                .requests()
                .iter()
                .any(|r| r["method"] == "thread/unsubscribe")
        );
        close_tab(&mut h, "first-side", button);
        h.press("right");
        h.press("enter");
        h.advance(250);
        assert_active(&h, SECOND);
        assert!(!h.t.screen().contains("first-side"));
        h.t.type_text("!");
        h.press("enter");
        h.advance(250);
        assert!(submitted(&h, SECOND, "second active input!"));
        assert!(!submitted(&h, SIDE, "first parked input"));
        assert!(
            h.fake
                .requests()
                .iter()
                .any(|r| r["method"] == "thread/unsubscribe" && r["params"]["threadId"] == SIDE)
        );
    }
}
