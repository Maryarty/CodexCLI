//! The skeleton's own screen tests: the empty chat, a prompt through the
//! scripted backend, the debug menu, the request dialog, stale events. The
//! chat screen here is a placeholder, so its goldens are rtwk's own
//! rendering (the parity pack records it as a delta against codex-tui).

use codex_tui_core::app::{AppEv, Command};
use codex_tui_core::fixtures::builders::{
    agent_completed, agent_delta, server_request_resolved, tool_input_request, turn_completed,
};
use codex_tui_core::ids::{ConnEpoch, ThreadId};
use codex_tui_core::screens::{Kind, ScreenCommand, chat};
use codex_tui_rtwk::app::debug;
use codex_tui_rtwk::services::backend::fake::FAKE_THREAD;

use crate::common::{
    assert_snapshot, assert_snapshot_where, start, start_debug, start_quittable, start_sized,
};

const DIRS: [(&str, &str, Kind); 21] = [
    ("chat", "S10", Kind::Home),
    ("cells", "S11", Kind::Part),
    ("composer", "S10", Kind::Part),
    ("popovers", "S12", Kind::Part),
    ("transcript", "S10", Kind::Part),
    ("select", "S14", Kind::Sheet),
    ("approvals", "S13", Kind::Sheet),
    ("settings", "S15", Kind::Sheet),
    ("backtrack", "S17", Kind::Part),
    ("resume", "S18", Kind::Route),
    ("status", "S19", Kind::Sheet),
    ("usage", "S20", Kind::Route),
    ("agents", "S21", Kind::Route),
    ("dialogs", "S23", Kind::Dialog),
    ("pets", "S24", Kind::Sheet),
    ("voice", "S25", Kind::Sheet),
    ("onboarding", "S01-S04", Kind::Route),
    ("startup", "S05", Kind::Dialog),
    ("import", "S06", Kind::Route),
    ("feedback", "S23", Kind::Dialog),
    ("providers", "providers", Kind::Sheet),
];

#[test]
fn screen_map_registers_the_directories() {
    let dirs = debug::dirs();
    let seen: Vec<_> = dirs
        .iter()
        .map(|dir| (dir.name, dir.id, dir.kind))
        .collect();
    assert_eq!(seen, DIRS.to_vec());
}

#[test]
fn empty_chat_screen() {
    let mut h = start();
    h.wait_idle();
    assert!(h.t.screen().contains("Ask Codex to do anything"));
    assert_snapshot(&h.t, "chat", "empty");
}

#[test]
fn prompt_streams_the_answer_as_plain_text() {
    let mut h = start();
    h.wait_idle();
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
    h.wait_idle();
    assert!(h.t.screen().contains("› hi"), "{}", h.t.screen());
    let commands = h.fake.commands();
    assert!(
        matches!(commands.as_slice(),
            [Command::Startup { .. }, Command::Screen { .. }, Command::Screen { .. }, Command::StartThread { .. }, Command::Popovers { ask, .. }, Command::StartTurn { thread, message, .. }]
                if *ask == codex_tui_core::screens::popovers::Ask::Append("hi".into())
                    && *thread == ThreadId::new(FAKE_THREAD) && message.text == "hi"),
        "{commands:?}"
    );
    // The live completion carries no server timing, so the `Worked for <time> • <clock>`
    // row (oracle `FinalMessageSeparator::from_turn_once`) is drawn from the app clock: the
    // golden leaves that row out, the clock time is not repeatable.
    assert!(h.t.screen().contains("Worked for"), "{}", h.t.screen());
    assert_snapshot_where(&h.t, "chat", "answer", |line| !line.contains("Worked for"));
}

#[test]
fn debug_menu_lists_the_directories() {
    // Every directory and entry, wide enough for the longest description: more rows than the oracle's 24 lines,
    // which this menu (not an oracle screen) need not fit. Tall, so that the
    // entries a slice adds still show: a heading and a blank row per
    // directory, a row per entry, and room for the frame.
    let entries: usize = debug::dirs()
        .iter()
        .map(|d| (d.debug_entries)().len())
        .sum();
    let rows = u16::try_from(entries + 2 * debug::dirs().len() + 40).unwrap_or(u16::MAX);
    let mut h = start_sized(true, 160, rows);
    h.t.pump_for(crate::common::ms(100));
    let screen = h.t.screen();
    for (name, id, _) in DIRS {
        assert!(screen.contains(name), "`{name}` missing:\n{screen}");
        assert!(screen.contains(id), "`{id}` missing:\n{screen}");
    }
    // Every entry of every directory is listed, under its own heading ...
    for dir in debug::dirs() {
        for entry in (dir.debug_entries)() {
            let row = format!("{} - {}", entry.name, entry.description);
            assert!(screen.contains(&row), "`{row}` missing:\n{screen}");
        }
    }
    // ... and the golden holds the heading rows only, so a slice that adds an
    // entry does not edit it.
    let entry_rows: Vec<String> = debug::dirs()
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
    h.wait_idle();
    h.press("ctrl+c");
    assert!(!h.t.is_running());
}

#[test]
fn an_event_of_an_older_connection_is_dropped() {
    let mut h = start();
    h.wait_idle();
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
    h.wait_idle();
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
    h.move_cursor("down", pets);
    let screen = h.t.screen();
    // The cursor is on `pets`; the first rows scrolled off a 12-line screen.
    assert!(screen.contains("pets"), "{screen}");
    assert!(!screen.contains(" chat "), "scrolled off: {screen}");
    h.press("enter");
    // The sheet fades in.
    h.t.pump_for(crate::common::ms(400));
    assert!(
        h.t.screen().contains("pets (S24): empty slot"),
        "{}",
        h.t.screen()
    );
}

#[test]
fn f12_opens_the_debug_menu_from_chat() {
    let mut h = start();
    h.wait_idle();
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
    h.wait_idle();
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
    h.wait_idle();
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
    h.wait_idle();
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
    assert!(screen.contains("Working"), "{screen}");
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
    h.wait_idle();
}

fn turn_texts(h: &crate::common::Harness) -> Vec<String> {
    h.fake
        .commands()
        .into_iter()
        .filter_map(|command| match command {
            Command::StartTurn { message, .. } => Some(message.text),
            _ => None,
        })
        .collect()
}

fn steer_texts(h: &crate::common::Harness) -> Vec<String> {
    steer_texts_of(&h.fake.commands())
}

fn steer_texts_of(commands: &[Command]) -> Vec<String> {
    commands
        .iter()
        .filter_map(|command| match command {
            Command::Screen {
                command: ScreenCommand::Chat(chat::Command::Steer { message, .. }),
                ..
            } => Some(message.text.clone()),
            _ => None,
        })
        .collect()
}

/// R10: the model decides. A message sent with Enter while a turn runs steers
/// it (`turn/steer`, the oracle's `submit_user_message_as_plain_user_turn`):
/// the composer clears and no second turn starts.
#[test]
fn a_message_typed_while_a_turn_runs_is_steered() {
    let mut h = start();
    h.wait_idle();
    h.fake
        .script_turn(vec![agent_delta(FAKE_THREAD, "turn-1", "msg-1", "so far")]);
    h.t.type_text("one");
    h.press("enter");
    h.wait_for("the turn to run", |s| s.contains("• so far"));
    h.t.type_text("two");
    h.press("enter");
    let fake = h.fake.clone();
    h.wait_for("the steer request", |_| {
        !steer_texts_of(&fake.commands()).is_empty()
    });
    let screen = h.t.screen();
    assert!(!screen.contains("› two"), "{screen}");
    h.settle();
    assert_eq!(steer_texts(&h), vec!["two".to_string()]);
    assert_eq!(turn_texts(&h), vec!["one".to_string()], "no second turn");
}

/// Tab queues, Enter steers (the person, 2026-11-22).
#[test]
fn queued_messages_are_sent_in_order_one_per_turn_end() {
    let mut h = start();
    h.wait_idle();
    h.fake
        .script_turn(vec![agent_delta(FAKE_THREAD, "turn-1", "msg-1", "so far")]);
    h.fake
        .script_turn(vec![agent_delta(FAKE_THREAD, "turn-2", "msg-2", "second")]);
    h.t.type_text("one");
    h.press("enter");
    h.wait_for("the turn to run", |s| s.contains("• so far"));
    for text in ["two", "three"] {
        h.t.type_text(text);
        h.press("tab");
        h.wait_for("queued", |s| s.contains(&format!("↳ {text}")));
    }
    let epoch = h.fake.epoch().expect("attached");
    h.fake
        .inject(epoch, AppEv::Backend(turn_completed(FAKE_THREAD, "turn-1")));
    h.wait_for("the second turn", |s| s.contains("• second"));
    let screen = h.t.screen();
    assert!(!screen.contains("↳ two"), "{screen}");
    assert!(screen.contains("↳ three"), "{screen}");
    assert_eq!(turn_texts(&h), ["one", "two"].map(String::from).to_vec());
    h.fake
        .inject(epoch, AppEv::Backend(turn_completed(FAKE_THREAD, "turn-2")));
    h.wait_for("the third turn", |s| !s.contains("↳ three"));
    h.settle();
    assert_eq!(
        turn_texts(&h),
        ["one", "two", "three"].map(String::from).to_vec()
    );
}

/// Tab queues, Enter steers (the person, 2026-11-22).
#[test]
fn a_queue_survives_a_second_submit() {
    let mut h = start();
    h.wait_idle();
    h.fake
        .script_turn(vec![agent_delta(FAKE_THREAD, "turn-1", "msg-1", "so far")]);
    h.t.type_text("one");
    h.press("enter");
    h.wait_for("the turn to run", |s| s.contains("• so far"));
    for text in ["two", "three"] {
        h.t.type_text(text);
        h.press("tab");
        h.wait_for("queued", |s| s.contains(&format!("↳ {text}")));
    }
    let screen = h.t.screen();
    assert!(screen.contains("↳ two"), "{screen}");
    assert!(screen.contains("↳ three"), "{screen}");
}

#[test]
fn escape_interrupts_the_visible_turn_while_its_answer_streams() {
    let mut h = start();
    h.wait_idle();
    h.fake
        .script_turn(vec![agent_delta(FAKE_THREAD, "turn-1", "msg-1", "partial")]);
    h.t.type_text("hi");
    h.press("enter");
    h.wait_for("the streamed text", |s| s.contains("• partial"));
    h.press("esc");
    h.settle();
    assert!(h.fake.commands().iter().any(|command| {
        matches!(command, Command::InterruptTurn { thread, turn, .. }
            if thread.as_str() == FAKE_THREAD && turn.as_str() == "turn-1")
    }));
}

/// Tab queues, Enter steers (the person, 2026-11-22).
#[test]
fn running_chat_shows_elapsed_and_tab_queue_and_ctrl_c_interrupts() {
    let mut h = start_quittable(false);
    h.wait_idle();
    h.fake
        .script_turn(vec![agent_delta(FAKE_THREAD, "turn-1", "msg-1", "so far")]);
    h.t.type_text("one");
    h.press("enter");
    h.wait_for("running indicator", |s| s.contains("esc to interrupt"));
    h.t.pump_for(crate::common::ms(1100));
    assert!(h.t.screen().contains("1s"), "{}", h.t.screen());

    h.t.type_text("two");
    h.wait_for("queue hint", |s| s.contains("tab to queue message"));
    h.press("tab");
    h.wait_for("queued message", |s| s.contains("↳ two"));
    h.press("ctrl+c");
    let fake = h.fake.clone();
    h.wait_for("interrupt request", |_| {
        fake.commands().iter().any(|command| {
        matches!(command, Command::InterruptTurn { turn, .. } if turn.as_str() == "turn-1")
    })
    });
    assert!(h.t.is_running());
}

#[test]
fn streaming_markdown_fits_inside_the_transcript_viewport() {
    let mut h = start_sized(false, 48, 12);
    h.wait_idle();
    let body = "# Start\n\n".to_owned()
        + &(1..=28)
            .map(|n| format!("paragraph {n}\n\n"))
            .collect::<String>();
    h.fake
        .script_turn(vec![agent_delta(FAKE_THREAD, "turn-1", "msg-1", &body)]);
    h.t.type_text("make a list");
    h.press("enter");
    h.wait_for("last paragraph", |s| s.contains("paragraph 28"));
    assert!(h.t.screen().contains("Ask Codex to do anything"));

    // Scroll the transcript backwards and keep reading the same area as more
    // Markdown arrives. The composer is outside that viewport.
    for _ in 0..4 {
        h.t.input(b"\x1b[<64;12;2M");
        h.t.pump_for(crate::common::ms(40));
    }
    let screen = h.t.screen();
    assert!(!screen.contains("paragraph 28"), "{screen}");
    assert!(screen.contains("Ask Codex to do anything"), "{screen}");
    let epoch = h.fake.epoch().expect("attached");
    h.fake.inject(
        epoch,
        AppEv::Backend(agent_delta(FAKE_THREAD, "turn-1", "msg-1", "paragraph 29")),
    );
    h.settle();
    assert!(!h.t.screen().contains("paragraph 29"), "{}", h.t.screen());
}

#[test]
fn empty_blossom_is_visible_only_without_a_draft() {
    use codex_tui_core::screens::{ScreenEv, chat};

    let mut h = start_sized(false, 80, 30);
    h.wait_idle();
    let epoch = h.fake.epoch().expect("attached");
    h.fake.inject(
        epoch,
        AppEv::Screen(ScreenEv::Chat(chat::Ev::Status(Box::new(
            chat::StatusFacts {
                empty_state: true,
                ..chat::StatusFacts::default()
            },
        )))),
    );
    let blossom = |s: &str| s.chars().any(|ch| ('⠁'..='⣿').contains(&ch));
    h.wait_for("blossom", blossom);
    h.t.type_text("draft");
    h.wait_for("draft without blossom", |s| {
        s.contains("draft") && !blossom(s)
    });
}

#[test]
fn running_shell_preview_keeps_its_output_when_completion_omits_it() {
    use codex_tui_core::app::BackendEv;
    use codex_tui_core::fixtures::builders::turn_started;
    use serde_json::json;

    let mut h = start_sized(false, 56, 24);
    h.wait_idle();
    let epoch = h.fake.epoch().expect("attached");
    h.fake
        .inject(epoch, AppEv::Backend(turn_started(FAKE_THREAD, "turn-1")));
    let command = |status: &str| {
        json!({
            "type": "commandExecution", "id": "cmd-1", "command": "/bin/zsh -lc 'printf test'",
            "cwd": "/tmp", "source": "agent", "status": status, "commandActions": [],
        })
    };
    let notify = |method, params| {
        BackendEv::Notification(Box::new(
            serde_json::from_value(json!({"method": method, "params": params}))
                .expect("notification"),
        ))
    };
    let context = |item| {
        json!({
            "item": item, "threadId": FAKE_THREAD, "turnId": "turn-1",
            "startedAtMs": 1000, "completedAtMs": 2000,
        })
    };
    h.fake.inject(
        epoch,
        AppEv::Backend(notify("item/started", context(command("inProgress")))),
    );
    h.fake.inject(epoch, AppEv::Backend(notify("item/commandExecution/outputDelta", json!({
        "threadId": FAKE_THREAD, "turnId": "turn-1", "itemId": "cmd-1", "delta": "tool output"
    }))));
    h.wait_for("running command with output", |s| {
        s.contains("Running printf test") && s.contains("└ tool output")
    });
    h.fake.inject(
        epoch,
        AppEv::Backend(notify("item/completed", context(command("completed")))),
    );
    h.wait_for("completed command with preserved output", |s| {
        s.contains("Ran printf test") && s.contains("└ tool output")
    });
    h.t.type_text("pending draft");
    h.press("ctrl+t");
    h.wait_for("full command and intact draft", |s| {
        s.contains("$ printf test") && s.contains("tool output") && s.contains("pending draft")
    });
    h.press("ctrl+t");
    h.wait_for("compact command and intact draft", |s| {
        s.contains("Ran printf test") && s.contains("pending draft")
    });
}

#[test]
fn the_session_heading_scrolls_with_the_transcript() {
    use codex_tui_core::screens::{ScreenEv, chat};
    let mut h = start_sized(false, 56, 20);
    h.wait_idle();
    let epoch = h.fake.epoch().expect("attached");
    h.fake.inject(
        epoch,
        AppEv::Screen(ScreenEv::Chat(chat::Ev::Status(Box::new(
            chat::StatusFacts {
                cwd: Some("/Users/example/project".into()),
                home_directory: Some("/Users/example".into()),
                version: Some("0.170.0".into()),
                ..chat::StatusFacts::default()
            },
        )))),
    );
    h.wait_for("session heading", |screen| {
        screen.contains("OpenAI Codex (v0.170.0)")
    });
    assert_eq!(h.t.lines()[1].trim_end(), "  >_ OpenAI Codex (v0.170.0)");
    assert_eq!(h.t.lines()[2].trim_end(), "     ~/project");
    for index in 0..12 {
        h.fake.inject(
            epoch,
            AppEv::Backend(agent_completed(
                FAKE_THREAD,
                "turn-1",
                &format!("message-{index}"),
                &format!("Reply {index}"),
            )),
        );
    }
    h.wait_for("latest reply", |screen| screen.contains("Reply 11"));
    assert!(!h.t.screen().contains("OpenAI Codex"), "{}", h.t.screen());
}

#[test]
fn transcript_prompt_surfaces_and_separators_keep_their_rows() {
    use codex_tui_core::fixtures::builders::user_item;

    for width in [56, 148] {
        let mut h = start_sized(false, width, 26);
        h.wait_idle();
        let epoch = h.fake.epoch().expect("attached");
        for item in [
            agent_completed(
                FAKE_THREAD,
                "turn-1",
                "prev",
                "Предыдущий ответ.\n\n| Вариант | Количество |\n| --- | ---: |\n| Пример | 42 |\n\nКонец сравнения.",
            ),
            user_item(FAKE_THREAD, "turn-2", "prompt-1", "Первый запрос\r\n\n  \n"),
            user_item(
                FAKE_THREAD,
                "turn-2",
                "prompt-2",
                "Второй запрос\nПродолжение",
            ),
            agent_completed(FAKE_THREAD, "turn-2", "last", "Следующий ответ."),
        ] {
            h.fake.inject(epoch, AppEv::Backend(item));
        }
        h.wait_for("follow-up response", |screen| {
            screen.contains("Следующий ответ.")
        });
        let lines = h.t.lines();
        let row = |needle: &str| {
            lines
                .iter()
                .position(|line| line.contains(needle))
                .unwrap_or_else(|| panic!("missing {needle} at width {width}\n{}", h.t.screen()))
        };
        assert!(lines[row("Вариант")].contains("Количество"));
        let before = row("Конец сравнения.");
        let first = row("Первый запрос");
        let second = row("Второй запрос");
        let after = row("Следующий ответ.");
        assert_eq!(first, before + 3, "{}", h.t.screen());
        // the chat keeps each prompt's own rows: the pad under the first, the blank row between
        // two cells, the pad over the second (the detailed transcript shares one block)
        assert_eq!(second, first + 4, "{}", h.t.screen());
        assert_eq!(after, second + 4, "{}", h.t.screen());
        assert!(
            lines[second + 1].starts_with("  Продолжение"),
            "{}",
            h.t.screen()
        );

        let background = h.t.cell(0, before).bg;
        let panel = h.t.cell(0, first).bg;
        assert_ne!(panel, background);
        assert_eq!(h.t.cell(0, before + 1).bg, background);
        assert_eq!(h.t.cell(0, after - 1).bg, background);
        assert_eq!(h.t.cell(0, first + 2).bg, background, "{}", h.t.screen());
        for line in (first - 1..after - 1).filter(|line| *line != first + 2) {
            assert_eq!(h.t.cell(0, line).bg, panel, "row {line} at width {width}");
            assert_eq!(
                h.t.cell(usize::from(width - 1), line).bg,
                panel,
                "right edge at row {line}, width {width}"
            );
        }

        // the detailed transcript (Ctrl+T) shares one block between the two prompts (oracle
        // `TranscriptView`): one pad row, no blank row between them
        h.press("ctrl+t");
        h.wait_idle();
        let lines = h.t.lines();
        let row = |needle: &str| {
            lines
                .iter()
                .position(|line| line.contains(needle))
                .unwrap_or_else(|| panic!("missing {needle} at width {width}\n{}", h.t.screen()))
        };
        let (first, second) = (row("Первый запрос"), row("Второй запрос"));
        assert_eq!(second, first + 2, "{}", h.t.screen());
        let panel = h.t.cell(0, first).bg;
        for line in first - 1..=second + 2 {
            assert_eq!(h.t.cell(0, line).bg, panel, "row {line} at width {width}");
        }
    }
}
