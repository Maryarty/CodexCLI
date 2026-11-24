#![allow(clippy::expect_used, clippy::unwrap_used)]

//! Time as a declared wake (ADR-012): the shell tells the app the time (`AppEv::Clock`), what
//! wants to be woken says when, and the app gives the nearest moment back (`AppFx::wake`) for
//! the shell's one timer. Every test here says the time itself, with a moment it makes up: none
//! waits for a second, and none reads the real clock.

use std::time::Duration;

use codex_app_server_protocol::Thread;
use codex_tui_core::app::{AppEv, AppFx, CodexApp, Now, apply_app};
use codex_tui_core::fixtures::script::build;
use codex_tui_core::screens::ScreenEv;
use codex_tui_core::screens::agents::{Ev, Page};
use pretty_assertions::assert_eq;
use serde_json::json;

fn tell(app: &mut CodexApp, now: Now) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, AppEv::Clock(now), &mut fx);
    app.assert_consistent();
    fx
}

/// A task the command center lists.
fn task() -> Thread {
    serde_json::from_value(json!({
        "id": "task-1", "environments": null, "extra": null, "sessionId": "task-1",
        "forkedFromId": null, "attachedToThreadId": null, "parentThreadId": null,
        "preview": "Review parser", "ephemeral": false, "section": null,
        "sectionEnteredAt": null, "projectId": null, "historyMode": "legacy",
        "modelProvider": "openai", "model": null, "reasoningEffort": null,
        "createdAt": 900, "updatedAt": 900, "recencyAt": 900, "status": {"type": "idle"},
        "path": null, "cwd": "/tmp/project", "cliVersion": "0.0.0", "originator": null,
        "source": "cli", "canAcceptDirectInput": true, "threadSource": null,
        "agentNickname": null, "agentRole": null, "gitInfo": null, "name": "Review parser",
        "daybreakEnabled": null, "turns": []
    }))
    .expect("a thread")
}

/// The center is handed `tasks` as its list.
fn list(app: &mut CodexApp, tasks: Vec<Thread>) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(
        app,
        AppEv::Screen(ScreenEv::Agents(Ev::Listed(Page {
            threads: tasks,
            unread: vec![],
            next_cursor: None,
            append: false,
            pins: None,
        }))),
        &mut fx,
    );
    fx
}

/// The command center lists one task: its age is worded in minutes, so the center wants the
/// clock at every minute.
fn app_listing_a_task() -> CodexApp {
    let mut app = CodexApp::new();
    list(&mut app, vec![task()]);
    app
}

#[test]
fn an_app_nothing_waits_on_asks_for_no_timer() {
    let mut app = CodexApp::new();
    let fx = tell(&mut app, Now::at_secs(1_000));
    assert_eq!(app.now(), Now::at_secs(1_000));
    assert_eq!(
        fx.wake(),
        None,
        "no feature, thread or screen declared a wake"
    );
}

/// The center's wake is the start of the next minute: the clock is told then, the ages the
/// screens word are moved on, and the next wake is a minute after.
#[test]
fn the_center_asks_for_the_next_minute_and_the_clock_ages_it() {
    let mut app = app_listing_a_task();
    let fx = tell(&mut app, Now::at_secs(1_000));
    let wake = fx.wake().expect("the center lists a task");
    assert_eq!(
        wake.at(),
        Now::at_secs(1_020),
        "16 min 40 s: the minute is at 17 min"
    );
    assert_eq!(wake.after(), Duration::from_secs(20));
    assert_eq!(
        app.screens().agents.now,
        1_000,
        "the screens read the app's clock"
    );

    let fx = tell(&mut app, Now::at_secs(1_020));
    assert_eq!(app.screens().agents.now, 1_020);
    assert_eq!(fx.wake().expect("still listing").at(), Now::at_secs(1_080));
}

/// A wake that would come in less than a second is asked a second later: a moment the clock
/// has reached and a feature still asks for must never make the shell spin.
#[test]
fn a_wake_is_never_less_than_a_second_away() {
    let mut app = app_listing_a_task();
    let fx = tell(&mut app, Now::at_millis(1_019_500));
    let wake = fx.wake().expect("the center lists a task");
    assert_eq!(wake.at(), Now::at_millis(1_020_500));
    assert_eq!(wake.after(), Duration::from_secs(1));
}

/// The wake is read at the end of every step, whatever the event: a task listed after the clock
/// was told is asked for with the clock the app has.
#[test]
fn the_wake_follows_the_model_not_the_event_that_changed_it() {
    let mut app = CodexApp::new();
    tell(&mut app, Now::at_secs(1_000));
    assert_eq!(
        list(&mut app, vec![]).wake(),
        None,
        "an empty center ages nothing"
    );
    let fx = list(&mut app, vec![task()]);
    assert_eq!(
        fx.wake().expect("a task is listed").at(),
        Now::at_secs(1_020),
        "an event that is not the clock still carries the wake"
    );
}

#[test]
fn a_script_says_what_time_it_is() {
    let played = build(
        r#"{"step":"clock","secs":968250720,"utc_offset":3600}
{"step":"clock","millis":968250720500}"#,
    )
    .expect("plays")
    .app;
    assert_eq!(played.now(), Now::at_millis(968_250_720_500));
    let zoned = build(r#"{"step":"clock","secs":968250720,"utc_offset":3600}"#)
        .expect("plays")
        .app;
    assert_eq!(zoned.now(), Now::at_secs(968_250_720).in_zone(3600));
}

#[test]
fn the_screens_old_clock_step_is_the_clock_step() {
    let app = build(r#"{"step":"screen","dir":"agents","ev":{"clock":999873120}}"#)
        .expect("plays")
        .app;
    assert_eq!(app.now(), Now::at_secs(999_873_120));
    assert_eq!(app.screens().agents.now, 999_873_120);
}

#[test]
fn a_clock_step_must_name_its_time() {
    let neither = build(r#"{"step":"clock"}"#);
    assert!(
        neither
            .err()
            .is_some_and(|err| err.message.contains("secs"))
    );
}
