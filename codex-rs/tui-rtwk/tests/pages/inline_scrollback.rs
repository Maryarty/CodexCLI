// test root: pages
//! A normal inline chat with a real terminal history sink and a deterministic app-server.

use codex_tui_core::app::effects::Ask;
use codex_tui_core::app::{AppEv, BackendEv, Intent};
use codex_tui_core::fixtures::builders::{
    agent_completed, agent_delta, turn_completed, turn_failed, turn_started,
};
use codex_tui_core::ids::ThreadId;
use codex_tui_core::screens::{ScreenEv, ScreenIntent, agents, chat};
use codex_tui_core::threads::NoteKey;
use codex_tui_rtwk::services::backend::fake::FAKE_THREAD;
use rtwk::binding::ScreenMode;
use rtwk_test::Options;
use serde_json::{Value, json};

use crate::common::{Harness, start_driven_mode};
use crate::kit::{self, Screen};

fn inline_chat() -> Harness {
    let mut history = vec!["S27 shell sentinel".to_owned()];
    history.extend((0..24).map(|row| format!("shell history {row}")));
    let mut h = start_driven_mode(
        false,
        Options::new()
            .mode(ScreenMode::Inline)
            .history(history.iter().map(String::as_str))
            .size(68, 16)
            .fail_on_overflow(false),
        true,
        |app| app,
    );
    h.connect();
    h.quiesce();
    h
}

fn all_rows(h: &Harness) -> String {
    format!("{}\n{}", h.t.scrollback().join("\n"), h.t.screen())
}

fn long_answer(first: &str) -> String {
    std::iter::once(first.to_owned())
        .chain((0..32).map(|row| format!("completed response paragraph {row:02}")))
        .collect::<Vec<_>>()
        .join("\n\n")
}

#[test]
fn only_settled_rows_leave_the_live_view_and_survive_resize_once() {
    let mut h = inline_chat();
    h.t.type_text("S27 retained draft");
    h.inject([
        AppEv::Backend(turn_started(FAKE_THREAD, "first")),
        AppEv::Backend(agent_delta(
            FAKE_THREAD,
            "first",
            "answer",
            "a mutable preview",
        )),
    ]);
    h.advance(800);
    assert!(h.t.screen().contains("a mutable preview"));
    assert!(!h.t.scrollback().join("\n").contains("a mutable preview"));

    let text = long_answer("S27 settled answer");
    let end = || {
        [
            AppEv::Backend(agent_completed(FAKE_THREAD, "first", "answer", &text)),
            AppEv::Backend(turn_completed(FAKE_THREAD, "first")),
        ]
    };
    h.inject(end());
    h.advance(1200);
    assert!(h.t.scrollback().join("\n").contains("S27 settled answer"));
    assert!(
        h.t.frame_loop().height() < 10,
        "the idle region fits the composer"
    );
    h.inject(end());
    h.inject([
        AppEv::Backend(turn_started(FAKE_THREAD, "second")),
        AppEv::Backend(agent_delta(
            FAKE_THREAD,
            "second",
            "answer",
            "still streaming",
        )),
    ]);
    h.advance(800);
    h.t.resize(58, 17);
    h.advance(500);
    let rows = all_rows(&h);
    assert_eq!(rows.matches("S27 settled answer").count(), 1);
    assert_eq!(rows.matches("S27 shell sentinel").count(), 1);
    assert!(h.t.screen().contains("S27 retained draft"));
    assert!(h.t.screen().contains("still streaming"));
    assert!(!h.t.scrollback().join("\n").contains("still streaming"));

    h.press("ctrl+t");
    h.press("ctrl+home");
    h.wait_for("the detailed reader still has committed rows", |screen| {
        screen.contains("S27 settled answer")
    });
    h.press("ctrl+t");
    assert!(h.t.screen().contains("S27 retained draft"));
    assert_eq!(
        h.t.scrollback()
            .join("\n")
            .matches("S27 settled answer")
            .count(),
        1
    );
}

#[test]
fn a_standalone_note_does_not_block_a_finalized_failed_turn() {
    let mut h = inline_chat();
    h.inject([AppEv::Intent(Intent::Screen(ScreenIntent::Chat(
        chat::Intent::Note {
            thread: Some(ThreadId::new(FAKE_THREAD)),
            note: NoteKey::ServerDone("standalone startup note".to_owned()).into(),
        },
    )))]);
    h.inject([
        AppEv::Backend(turn_started(FAKE_THREAD, "failed")),
        AppEv::Backend(agent_delta(
            FAKE_THREAD,
            "failed",
            "answer",
            &long_answer("S27 finalized failure"),
        )),
        AppEv::Backend(turn_failed(FAKE_THREAD, "failed")),
    ]);
    h.advance(1200);
    let history = h.t.scrollback().join("\n");
    assert_eq!(
        history.matches("S27 finalized failure").count(),
        1,
        "{history}"
    );
    assert_eq!(history.matches("S27 shell sentinel").count(), 1);
}

fn resume(id: &str, turns: &[Value]) -> Value {
    json!({
        "thread": {
            "id": id, "environments": null, "extra": null, "sessionId": id,
            "preview": id, "ephemeral": false, "historyMode": "legacy", "modelProvider": "openai",
            "createdAt": 1_700_000_000, "updatedAt": 1_700_000_100, "status": {"type": "idle"},
            "cwd": "/work", "cliVersion": "0.0.0", "source": "cli", "canAcceptDirectInput": true,
            "name": id, "turns": turns,
        },
        "model": "gpt-5", "modelProvider": "openai", "cwd": "/work", "instructionSources": [],
        "approvalPolicy": "on-request", "approvalsReviewer": "user",
        "sandbox": {"type": "dangerFullAccess"}, "reasoningEffort": null,
    })
}

#[test]
fn switching_threads_keeps_queued_history_separate_and_preserves_the_draft() {
    let mut h = inline_chat();
    let first = long_answer("settled thread A");
    let first_for_resume = first.clone();
    h.fake.respond(move |request| {
        let id = request["params"]["threadId"].as_str()?;
        (request["method"] == "thread/resume").then(|| {
            let turns = (id == FAKE_THREAD)
                .then(|| kit::agent_turn("same-turn", &[&first_for_resume]))
                .into_iter()
                .collect::<Vec<_>>();
            Ok(resume(id, &turns))
        })
    });
    let Ok(second_thread) = serde_json::from_value(resume("second-thread", &[])["thread"].clone())
    else {
        panic!("the second session should be a valid thread");
    };
    h.inject([AppEv::Screen(ScreenEv::Agents(agents::Ev::Listed(
        agents::Page {
            threads: vec![second_thread],
            unread: Vec::new(),
            next_cursor: None,
            append: false,
            pins: None,
        },
    )))]);
    h.t.type_text("keep the A draft");
    h.inject([
        AppEv::Backend(turn_started(FAKE_THREAD, "same-turn")),
        AppEv::Backend(agent_completed(
            FAKE_THREAD,
            "same-turn",
            "same-turn-0",
            &first,
        )),
        AppEv::Backend(turn_completed(FAKE_THREAD, "same-turn")),
        AppEv::Intent(Intent::OpenThread(ThreadId::new("second-thread"))),
    ]);
    h.advance(600);
    let second = long_answer("settled thread B");
    h.inject([
        AppEv::Backend(turn_started("second-thread", "same-turn")),
        AppEv::Backend(agent_completed(
            "second-thread",
            "same-turn",
            "same-turn-0",
            &second,
        )),
        AppEv::Backend(turn_completed("second-thread", "same-turn")),
    ]);
    h.advance(600);
    h.inject([AppEv::Intent(Intent::OpenThread(ThreadId::new(
        FAKE_THREAD,
    )))]);
    h.advance(1000);
    let rows = all_rows(&h);
    assert_eq!(rows.matches("settled thread A").count(), 1, "{rows}");
    assert_eq!(rows.matches("settled thread B").count(), 1, "{rows}");
    assert!(h.t.screen().contains("keep the A draft"));
}

#[test]
fn finalized_voice_and_the_following_idle_note_do_not_block_the_next_turn() {
    let mut s = Screen::from(inline_chat());
    s.voice_available().keys("f8");
    let session = s.connect_voice();
    s.voice_caption(false, &session, "voice", "assistant", "voice preview");
    assert!(!s.scrollback().join("\n").contains("voice preview"));
    s.voice_caption(
        true,
        &session,
        "voice",
        "assistant",
        "finalized voice caption",
    );
    s.type_text("/pwd").keys("enter");
    s.live_tail(&long_answer("normal turn after voice"))
        .commit_live_tail();
    s.wait(1200);
    let history = s.scrollback().join("\n");
    for text in [
        "finalized voice caption",
        "Current working directory:",
        "normal turn after voice",
    ] {
        assert_eq!(history.matches(text).count(), 1, "{history}");
    }
}

#[test]
fn an_artifact_with_an_outstanding_snapshot_stays_in_the_live_view() {
    let mut h = inline_chat();
    let Ok(viewed) = serde_json::from_value(json!({
        "method": "item/completed", "params": {
            "threadId": FAKE_THREAD, "turnId": "view", "completedAtMs": 0,
            "item": {"type": "imageView", "id": "artifact", "path": "/tmp/s27-sample.png"},
        },
    })) else {
        panic!("known artifact notification");
    };
    h.inject([
        AppEv::Backend(turn_started(FAKE_THREAD, "view")),
        AppEv::Backend(BackendEv::Notification(Box::new(viewed))),
        AppEv::Backend(turn_completed(FAKE_THREAD, "view")),
        AppEv::Backend(turn_started(FAKE_THREAD, "later")),
        AppEv::Backend(agent_completed(
            FAKE_THREAD,
            "later",
            "answer",
            "later text remains visible",
        )),
        AppEv::Backend(turn_completed(FAKE_THREAD, "later")),
    ]);
    h.advance(800);
    assert!(h.t.screen().contains("s27-sample.png"));
    assert!(h.t.screen().contains("later text remains visible"));
    assert!(!h.t.scrollback().join("\n").contains("s27-sample.png"));
    assert!(h.fake.commands().iter().any(|cmd| {
        matches!(cmd.host_ask(), Some(Ask::ImageSnapshot { path, .. }) if path == "/tmp/s27-sample.png")
    }));
}
