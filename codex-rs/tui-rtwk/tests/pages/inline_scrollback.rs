// test root: pages
//! A normal inline chat with a real terminal history sink and a deterministic app-server.

use codex_tui_core::app::effects::{Ask, Done, ImageRead, ImageSnapshot};
use codex_tui_core::app::{AppEv, BackendEv, Command, Intent};
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
use crate::kit::{self, Owner, Screen};

fn inline_chat() -> Harness {
    inline_chat_with(Options::new(), true)
}

fn inline_chat_with(options: Options, quittable: bool) -> Harness {
    let mut history = vec!["S27 shell sentinel".to_owned()];
    history.extend((0..24).map(|row| format!("shell history {row}")));
    let mut h = start_driven_mode(
        false,
        options
            .mode(ScreenMode::Inline)
            .history(history.iter().map(String::as_str))
            .size(68, 16),
        quittable,
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
fn a_replayed_completion_separator_reaches_terminal_history() {
    let first = long_answer("replayed first response");
    let second = long_answer("replayed second response");
    let mut first = kit::agent_turn("first", &[&first]);
    first["durationMs"] = json!(125_000);
    first["completedAt"] = Value::Null;
    let mut s = Screen::build(Owner::ChatLive)
        .size(58, 16)
        .inline(&[])
        .replay(vec![kit::agent_turn("second", &[&second]), first])
        .open();
    s.wait(1200);

    let history = s.scrollback().join("\n");
    let separator = "  Worked for 2m 5s";
    assert_eq!(
        history
            .lines()
            .filter(|row| row.trim_end() == separator)
            .count(),
        1,
        "{history}",
    );
    let first = history
        .find("replayed first response")
        .expect("first response");
    let timing = history.find(separator).expect("complete timing row");
    let second = history
        .find("replayed second response")
        .expect("second response");
    assert!(first < timing && timing < second, "{history}");
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

const ARTIFACT: &str = "/tmp/s27-sample.png";
const SUFFIX: &str = "S27 first response after the image";

fn completed_item(turn: &str, item: Value) -> AppEv {
    let notification = serde_json::from_value(json!({
        "method": "item/completed", "params": {
            "threadId": FAKE_THREAD, "turnId": turn, "completedAtMs": 0,
            "item": item,
        },
    }))
    .expect("known artifact notification");
    AppEv::Backend(BackendEv::Notification(Box::new(notification)))
}

fn finish_artifact_and_suffix(h: &mut Harness, item: Value) {
    h.inject([
        AppEv::Backend(turn_started(FAKE_THREAD, "view")),
        completed_item("view", item),
        AppEv::Backend(turn_completed(FAKE_THREAD, "view")),
        AppEv::Backend(turn_started(FAKE_THREAD, "later")),
        AppEv::Backend(agent_completed(
            FAKE_THREAD,
            "later",
            "answer",
            &long_answer(SUFFIX),
        )),
        AppEv::Backend(turn_completed(FAKE_THREAD, "later")),
    ]);
    h.advance(800);
}

fn viewed_item() -> Value {
    json!({"type": "imageView", "id": "artifact", "path": ARTIFACT})
}

fn image_requests(h: &Harness, path: &str) -> Vec<u64> {
    h.fake
        .commands()
        .into_iter()
        .filter_map(|command| match command {
            Command::Effect {
                request,
                ask: Ask::ImageSnapshot {
                    path: candidate, ..
                },
                ..
            } if candidate == path => Some(request),
            _ => None,
        })
        .collect()
}

fn image_reply(h: &mut Harness, request: u64, read: ImageRead) {
    h.inject([AppEv::Effect {
        request,
        done: Done::ImageSnapshot(read),
    }]);
    h.advance(800);
}

fn assert_artifact_prefix(h: &Harness, artifact: &str) {
    let history = h.t.scrollback().join("\n");
    let image = history
        .find(artifact)
        .unwrap_or_else(|| panic!("missing {artifact}: {history}"));
    let suffix = history.find(SUFFIX).expect("suffix enters native history");
    assert!(image < suffix, "{history}");
    let rows = all_rows(h);
    for text in [artifact, SUFFIX, "S27 shell sentinel"] {
        assert_eq!(rows.matches(text).count(), 1, "{rows}");
    }
    assert_eq!(
        h.t.frame_loop().inline_queued_rows(),
        h.t.frame_loop().inline_presented_rows()
    );
}

#[test]
fn unavailable_offscreen_artifact_releases_the_held_suffix() {
    let mut h = inline_chat();
    h.fake.hold_image_snapshot(ARTIFACT.to_owned());
    let before = h.t.frame_loop().inline_queued_rows();
    finish_artifact_and_suffix(&mut h, viewed_item());
    let screen = h.t.screen();
    assert!(
        screen.contains("completed response paragraph 31"),
        "{screen}"
    );
    assert!(!screen.contains("s27-sample.png"), "{screen}");
    assert_eq!(h.t.frame_loop().inline_queued_rows(), before);

    let wishes = image_requests(&h, ARTIFACT);
    assert_eq!(wishes.len(), 1);
    image_reply(&mut h, wishes[0], ImageRead::Unavailable);
    assert_artifact_prefix(&h, "s27-sample.png");
}

#[test]
fn ready_artifact_keeps_its_authorized_link_through_native_receipt() {
    let mut h = inline_chat();
    h.fake.hold_image_snapshot(ARTIFACT.to_owned());
    finish_artifact_and_suffix(&mut h, viewed_item());
    let wishes = image_requests(&h, ARTIFACT);
    assert_eq!(wishes.len(), 1);

    let url = "file:///tmp/s27-sample.png";
    assert!(!h.t.written().contains(url));
    image_reply(
        &mut h,
        wishes[0],
        ImageRead::Snapshot(ImageSnapshot {
            link: Some(url.to_owned()),
            ..Default::default()
        }),
    );
    assert_artifact_prefix(&h, "s27-sample.png");
    assert!(h.t.written().contains(url));

    h.press("ctrl+t");
    h.press("ctrl+t");
    h.advance(500);
    assert_artifact_prefix(&h, "s27-sample.png");
}

#[test]
fn generated_preview_is_presented_with_its_native_history_row() {
    use rtwk_core::graphics::GraphicsProtocol;

    let mut h = inline_chat_with(
        Options::new()
            .graphics(GraphicsProtocol::Kitty)
            .cell_pixels(8, 16),
        false,
    );
    let file = kit::image_file_sized("s27-native-commit.png", 96, 32);
    let path = file.to_string_lossy().into_owned();
    h.fake.allow_image_snapshot(path.clone());
    finish_artifact_and_suffix(
        &mut h,
        json!({
            "type": "imageGeneration", "id": "artifact", "status": "completed",
            "result": "ok", "revisedPrompt": "S27 generated preview", "savedPath": path,
        }),
    );

    assert_artifact_prefix(&h, "S27 generated preview");
    assert!(h.fake.commands().iter().any(|cmd| {
        matches!(
            cmd.host_ask(),
            Some(Ask::ImageSnapshot {
                preview: Some(_),
                ..
            })
        )
    }));
    assert!(
        h.t.written().contains("\x1b_Ga=T"),
        "native Kitty image was written"
    );
}

#[test]
fn a_read_from_before_a_thread_remount_cannot_release_the_new_prefix() {
    let mut h = inline_chat();
    h.fake.hold_image_snapshot(ARTIFACT.to_owned());
    let replay = [
        kit::replay_turn("view", json!([viewed_item()])),
        kit::replay_turn(
            "later",
            json!([{
                "type": "agentMessage", "id": "answer", "text": long_answer(SUFFIX),
                "phase": "final_answer",
            }]),
        ),
    ];
    h.fake.respond(move |request| {
        let id = request["params"]["threadId"].as_str()?;
        (request["method"] == "thread/resume").then(|| {
            let turns = if id == FAKE_THREAD { &replay[..] } else { &[] };
            Ok(resume(id, turns))
        })
    });
    let second_thread = serde_json::from_value(resume("second-thread", &[])["thread"].clone())
        .expect("valid second thread");
    h.inject([AppEv::Screen(ScreenEv::Agents(agents::Ev::Listed(
        agents::Page {
            threads: vec![second_thread],
            unread: Vec::new(),
            next_cursor: None,
            append: false,
            pins: None,
        },
    )))]);
    finish_artifact_and_suffix(&mut h, viewed_item());
    let asks = image_requests(&h, ARTIFACT);
    assert_eq!(asks.len(), 1);

    for thread in ["second-thread", FAKE_THREAD] {
        h.inject([AppEv::Intent(Intent::OpenThread(ThreadId::new(thread)))]);
        h.advance(600);
    }
    let renewed = image_requests(&h, ARTIFACT);
    assert_eq!(renewed.len(), 2, "new mount acquires its own read");
    assert_ne!(renewed[1], asks[0]);

    let stale_link = "file:///tmp/stale-s27-artifact.png";
    image_reply(
        &mut h,
        asks[0],
        ImageRead::Snapshot(ImageSnapshot {
            link: Some(stale_link.to_owned()),
            ..Default::default()
        }),
    );
    assert!(!h.t.scrollback().join("\n").contains(SUFFIX));
    assert!(!h.t.written().contains(stale_link));
    image_reply(&mut h, renewed[1], ImageRead::Unavailable);
    assert_artifact_prefix(&h, "s27-sample.png");
}
