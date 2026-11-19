//! The skeleton's own screen tests: the empty chat, a prompt through the
//! scripted backend, the debug menu, the request dialog, stale events. The
//! chat screen here is a placeholder, so its goldens are rtwk's own
//! rendering (the parity pack records it as a delta against codex-tui).

use codex_tui_core::app::{AppEv, Command};
use codex_tui_core::fixtures::builders::{
    agent_completed, agent_delta, server_request_resolved, tool_input_request, turn_completed,
};
use codex_tui_core::ids::{ConnEpoch, ThreadId};
use codex_tui_core::screens::Kind;
use codex_tui_rtwk::screens;
use codex_tui_rtwk::shell::fake::FAKE_THREAD;

use crate::common::{
    assert_snapshot, assert_snapshot_where, start, start_debug, start_quittable, start_sized,
};

const DIRS: [(&str, &str, Kind); 21] = [
    ("chat", "S10", Kind::Home),
    ("cells", "S11", Kind::Part),
    ("composer", "S10", Kind::Part),
    ("popovers", "S12", Kind::Part),
    ("transcript", "S10", Kind::Part),
    ("select", "S14", Kind::Dialog),
    ("approvals", "S13", Kind::Dialog),
    ("settings", "S15", Kind::Dialog),
    ("backtrack", "S17", Kind::Part),
    ("resume", "S18", Kind::Route),
    ("status", "S19", Kind::Dialog),
    ("usage", "S20", Kind::Route),
    ("agents", "S21", Kind::Route),
    ("sidebar", "S22", Kind::Part),
    ("dialogs", "S23", Kind::Dialog),
    ("pets", "S24", Kind::Part),
    ("voice", "S25", Kind::Part),
    ("ui_tree", "S26", Kind::Part),
    ("onboarding", "S02-S04", Kind::Route),
    ("startup", "S05", Kind::Dialog),
    ("import", "S06", Kind::Route),
];

#[test]
fn screen_map_registers_the_22_directories() {
    let dirs = screens::dirs();
    let seen: Vec<_> = dirs
        .iter()
        .map(|dir| (dir.name, dir.id, dir.kind))
        .collect();
    assert_eq!(seen, DIRS.to_vec());
}

#[test]
fn empty_chat_screen() {
    let mut h = start();
    h.wait_for("ready", |s| s.contains("ready"));
    assert!(h.t.screen().contains("Ask Codex to do anything"));
    assert_snapshot(&h.t, "chat", "empty");
}

#[test]
fn prompt_streams_the_answer_as_plain_text() {
    let mut h = start();
    h.wait_for("ready", |s| s.contains("ready"));
    let epoch_thread = FAKE_THREAD;
    h.fake.script_turn(vec![
        agent_delta(epoch_thread, "turn-1", "msg-1", "Hello, "),
        agent_delta(epoch_thread, "turn-1", "msg-1", "world"),
        agent_completed(epoch_thread, "turn-1", "msg-1", "Hello, world"),
        turn_completed(epoch_thread, "turn-1"),
    ]);
    h.t.type_text("hi");
    h.press("enter");
    h.wait_for("the answer", |s| s.contains("• Hello, world"));
    h.wait_for("the turn to end", |s| s.contains("ready"));
    assert!(h.t.screen().contains("› hi"), "{}", h.t.screen());
    let commands = h.fake.commands();
    assert!(
        matches!(commands.as_slice(),
            [Command::StartThread { .. }, Command::StartTurn { thread, text, .. }]
                if *thread == ThreadId::new(FAKE_THREAD) && text == "hi"),
        "{commands:?}"
    );
    assert_snapshot(&h.t, "chat", "answer");
}

#[test]
fn debug_menu_lists_the_directories() {
    // 22 directories and every entry: more rows than the oracle's 24 lines,
    // which this menu (not an oracle screen) need not fit. Tall, so that the
    // entries a slice adds still show.
    let mut h = start_sized(true, 80, 120);
    h.t.pump_for(crate::common::ms(100));
    let screen = h.t.screen();
    for (name, id, _) in DIRS {
        assert!(screen.contains(name), "`{name}` missing:\n{screen}");
        assert!(screen.contains(id), "`{id}` missing:\n{screen}");
    }
    // Every entry of every directory is listed, under its own heading ...
    for dir in screens::dirs() {
        for entry in (dir.debug_entries)() {
            let row = format!("{} - {}", entry.name, entry.description);
            assert!(screen.contains(&row), "`{row}` missing:\n{screen}");
        }
    }
    // ... and the golden holds the heading rows only, so a slice that adds an
    // entry does not edit it.
    let entry_rows: Vec<String> = screens::dirs()
        .iter()
        .flat_map(|dir| (dir.debug_entries)())
        .map(|entry| format!("{} - {}", entry.name, entry.description))
        .collect();
    assert_snapshot_where(&h.t, "debug", "menu", |line| {
        !entry_rows.iter().any(|row| row == line.trim_start())
    });
}

#[test]
fn debug_menu_opens_a_slot() {
    let mut h = start_debug();
    h.open_debug_row("cells", None);
    assert!(
        h.t.screen().contains("cells (S11): empty slot"),
        "{}",
        h.t.screen()
    );
}

/// ADR-002 binding condition 3: the Navigator is keyed, so its stack stands
/// when the view runs again with new provider values.
#[test]
fn the_navigator_stack_survives_a_view_run() {
    let mut h = start_debug();
    h.open_debug_row("cells", None);
    assert!(h.t.screen().contains("cells (S11): empty slot"));
    let epoch = h.fake.epoch().expect("attached");
    let fake = h.fake.clone();
    h.wait_for("a started thread", |_| {
        fake.commands()
            .iter()
            .any(|c| matches!(c, Command::StartThread { .. }))
    });
    h.fake.inject(
        epoch,
        AppEv::Backend(agent_completed(FAKE_THREAD, "turn-9", "msg-9", "late")),
    );
    for _ in 0..10 {
        h.t.pump_for(crate::common::ms(50));
        std::thread::sleep(crate::common::ms(10));
    }
    assert!(
        h.t.screen().contains("cells (S11): empty slot"),
        "{}",
        h.t.screen()
    );
}

#[test]
fn debug_menu_q_quits() {
    let mut h = start_quittable(true);
    h.press("q");
    assert!(!h.t.is_running());
}

#[test]
fn ctrl_c_quits() {
    let mut h = start_quittable(false);
    h.wait_for("ready", |s| s.contains("ready"));
    h.press("ctrl+c");
    assert!(!h.t.is_running());
}

#[test]
fn an_event_of_an_older_connection_is_dropped() {
    let mut h = start();
    h.wait_for("ready", |s| s.contains("ready"));
    let epoch = h.fake.epoch().expect("attached");
    assert!(epoch.0 >= 1, "the first connection is epoch 1");
    let stale = ConnEpoch(epoch.0 - 1);
    let item = || {
        AppEv::Backend(agent_completed(
            FAKE_THREAD,
            "turn-9",
            "msg-9",
            "from the past",
        ))
    };
    h.fake.inject(stale, item());
    h.t.pump_for(crate::common::ms(100));
    std::thread::sleep(crate::common::ms(50));
    h.t.pump_for(crate::common::ms(100));
    assert!(!h.t.screen().contains("from the past"));
    h.fake.inject(epoch, item());
    h.wait_for("the current event", |s| s.contains("• from the past"));
}

#[test]
fn a_server_request_opens_the_dismiss_dialog_and_is_rejected_once() {
    let mut h = start();
    h.wait_for("ready", |s| s.contains("ready"));
    let epoch = h.fake.epoch().expect("attached");
    h.fake.inject(
        epoch,
        AppEv::Backend(tool_input_request(FAKE_THREAD, "turn-1", 7)),
    );
    h.wait_for("the dialog", |s| s.contains("needs an answer"));
    h.press("enter");
    h.wait_for("the dialog to close", |s| !s.contains("needs an answer"));
    h.settle();
    let rejected: Vec<_> = h
        .fake
        .commands()
        .into_iter()
        .filter(|command| matches!(command, Command::RejectRequest { .. }))
        .collect();
    assert_eq!(rejected.len(), 1, "{rejected:?}");
}

/// 12 lines show 10 rows of the menu; the cursor must scroll the list.
#[test]
fn debug_menu_scrolls_to_the_cursor() {
    let mut h = start_sized(true, 80, 12);
    assert!(h.t.screen().contains("chat"));
    let pets = crate::common::debug_row("pets", None);
    for _ in 0..pets {
        h.press("down");
    }
    let screen = h.t.screen();
    // The cursor is on `pets`; the first rows scrolled off a 12-line screen.
    assert!(screen.contains("pets"), "{screen}");
    assert!(!screen.contains(" chat "), "scrolled off: {screen}");
    h.press("enter");
    assert!(
        h.t.screen().contains("pets (S24): empty slot"),
        "{}",
        h.t.screen()
    );
}

#[test]
fn f12_opens_the_debug_menu_from_chat() {
    let mut h = start();
    h.wait_for("ready", |s| s.contains("ready"));
    h.press("f12");
    assert!(
        h.t.screen().contains("codex-rtwk debug screens"),
        "{}",
        h.t.screen()
    );
    h.press("escape");
    assert!(h.t.screen().contains("Ask Codex to do anything"));
}

/// ADR-002: the presenter wraps the home, whichever home it is.
#[test]
fn a_request_opens_the_dialog_under_debug_screens_too() {
    let mut h = start_debug();
    let fake = h.fake.clone();
    h.wait_for("a connection", |_| fake.epoch().is_some());
    let epoch = h.fake.epoch().expect("attached");
    h.fake.inject(
        epoch,
        AppEv::Backend(tool_input_request(FAKE_THREAD, "turn-1", 3)),
    );
    h.wait_for("the dialog", |s| s.contains("needs an answer"));
}

#[test]
fn a_second_request_waits_for_the_first_dialog() {
    let mut h = start();
    h.wait_for("ready", |s| s.contains("ready"));
    let epoch = h.fake.epoch().expect("attached");
    for id in [7, 8] {
        h.fake.inject(
            epoch,
            AppEv::Backend(tool_input_request(FAKE_THREAD, "turn-1", id)),
        );
    }
    h.wait_for("the first dialog", |s| s.contains("7 needs an answer"));
    assert!(!h.t.screen().contains("8 needs an answer"));
    h.press("enter");
    h.wait_for("the second dialog", |s| s.contains("8 needs an answer"));
    h.press("enter");
    h.wait_for("no dialog", |s| !s.contains("needs an answer"));
    h.settle();
    assert_eq!(rejected(&h), 2);
}

/// `serverRequest/resolved`: the server answered (or gave up) first, so the
/// dialog closes and nothing is rejected on top.
#[test]
fn server_request_resolved_closes_the_dialog_without_a_reject() {
    let mut h = start();
    h.wait_for("ready", |s| s.contains("ready"));
    let epoch = h.fake.epoch().expect("attached");
    h.fake.inject(
        epoch,
        AppEv::Backend(tool_input_request(FAKE_THREAD, "turn-1", 7)),
    );
    h.wait_for("the dialog", |s| s.contains("needs an answer"));
    h.fake.inject(
        epoch,
        AppEv::Backend(server_request_resolved(FAKE_THREAD, 7)),
    );
    h.wait_for("the dialog to close", |s| !s.contains("needs an answer"));
    h.settle();
    assert_eq!(rejected(&h), 0);
}

fn rejected(h: &crate::common::Harness) -> usize {
    h.fake
        .commands()
        .iter()
        .filter(|command| matches!(command, Command::RejectRequest { .. }))
        .count()
}

#[test]
fn a_resize_mid_stream_keeps_the_answer() {
    let mut h = start();
    h.wait_for("ready", |s| s.contains("ready"));
    h.fake.script_turn(vec![
        agent_delta(FAKE_THREAD, "turn-1", "msg-1", "Hello, "),
        agent_delta(FAKE_THREAD, "turn-1", "msg-1", "world"),
    ]);
    h.t.type_text("hi");
    h.press("enter");
    h.wait_for("the streamed text", |s| s.contains("• Hello, world"));
    h.t.resize(30, 10);
    h.t.pump_for(crate::common::ms(100));
    let screen = h.t.screen();
    assert!(screen.contains("• Hello, world"), "{screen}");
    assert!(screen.contains("working"), "{screen}");
    let epoch = h.fake.epoch().expect("attached");
    h.fake.inject(
        epoch,
        AppEv::Backend(agent_completed(
            FAKE_THREAD,
            "turn-1",
            "msg-1",
            "Hello, world",
        )),
    );
    h.fake
        .inject(epoch, AppEv::Backend(turn_completed(FAKE_THREAD, "turn-1")));
    h.wait_for("the turn to end", |s| s.contains("ready"));
}

fn turn_texts(h: &crate::common::Harness) -> Vec<String> {
    h.fake
        .commands()
        .into_iter()
        .filter_map(|command| match command {
            Command::StartTurn { text, .. } => Some(text),
            _ => None,
        })
        .collect()
}

/// R10: the model decides. A message sent while a turn runs is queued: the
/// composer clears and the chat shows it as a dim line above the composer.
#[test]
fn a_message_typed_while_a_turn_runs_is_queued_and_shown() {
    let mut h = start();
    h.wait_for("ready", |s| s.contains("ready"));
    h.fake
        .script_turn(vec![agent_delta(FAKE_THREAD, "turn-1", "msg-1", "so far")]);
    h.t.type_text("one");
    h.press("enter");
    h.wait_for("the turn to run", |s| s.contains("• so far"));
    h.t.type_text("two");
    h.press("enter");
    h.wait_for("the queued line", |s| s.contains("(queued) two"));
    let screen = h.t.screen();
    assert!(!screen.contains("› two"), "{screen}");
    h.settle();
    assert_eq!(turn_texts(&h), vec!["one".to_string()], "not sent yet");
}

#[test]
fn queued_messages_are_sent_in_order_one_per_turn_end() {
    let mut h = start();
    h.wait_for("ready", |s| s.contains("ready"));
    h.fake
        .script_turn(vec![agent_delta(FAKE_THREAD, "turn-1", "msg-1", "so far")]);
    h.fake
        .script_turn(vec![agent_delta(FAKE_THREAD, "turn-2", "msg-2", "second")]);
    h.t.type_text("one");
    h.press("enter");
    h.wait_for("the turn to run", |s| s.contains("• so far"));
    for text in ["two", "three"] {
        h.t.type_text(text);
        h.press("enter");
        h.wait_for("queued", |s| s.contains(&format!("(queued) {text}")));
    }
    let epoch = h.fake.epoch().expect("attached");
    h.fake
        .inject(epoch, AppEv::Backend(turn_completed(FAKE_THREAD, "turn-1")));
    h.wait_for("the second turn", |s| s.contains("• second"));
    let screen = h.t.screen();
    assert!(!screen.contains("(queued) two"), "{screen}");
    assert!(screen.contains("(queued) three"), "{screen}");
    assert_eq!(turn_texts(&h), ["one", "two"].map(String::from).to_vec());
    h.fake
        .inject(epoch, AppEv::Backend(turn_completed(FAKE_THREAD, "turn-2")));
    h.wait_for("the third turn", |s| !s.contains("(queued) three"));
    h.settle();
    assert_eq!(
        turn_texts(&h),
        ["one", "two", "three"].map(String::from).to_vec()
    );
}

#[test]
fn a_queue_survives_a_second_submit() {
    let mut h = start();
    h.wait_for("ready", |s| s.contains("ready"));
    h.fake
        .script_turn(vec![agent_delta(FAKE_THREAD, "turn-1", "msg-1", "so far")]);
    h.t.type_text("one");
    h.press("enter");
    h.wait_for("the turn to run", |s| s.contains("• so far"));
    for text in ["two", "three"] {
        h.t.type_text(text);
        h.press("enter");
        h.wait_for("queued", |s| s.contains(&format!("(queued) {text}")));
    }
    let screen = h.t.screen();
    assert!(screen.contains("(queued) two"), "{screen}");
    assert!(screen.contains("(queued) three"), "{screen}");
}
