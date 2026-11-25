#![allow(clippy::expect_used, clippy::unwrap_used)]

//! The countdown of the questions that wait (`features::async_questions`): a question that
//! arrives while the editor is closed expires thirty seconds later and the summary counts the
//! last twenty down; the user at the editor stops it, a message of the user's own drops the
//! questions. The clock is the script's: no test waits.

use codex_tui_core::app::{AppEv, AppFx, CodexApp, Intent, Now, apply_app};
use codex_tui_core::fixtures::script::build;
use codex_tui_core::ids::ThreadId;
use codex_tui_core::question_reply::Intent as Questions;

const BASE: i64 = 1_000_000_000_000;

/// The session of `q_plain`, the clock at `BASE`, then the message that asks.
fn asked() -> (CodexApp, ThreadId) {
    let script = include_str!("../../tests/oracle/fixtures/chat_approvals_2/q_plain.jsonl");
    let lines: Vec<&str> = script.lines().collect();
    let text = format!(
        "{}\n{}\n{{\"step\":\"clock\",\"millis\":{BASE}}}\n{}\n",
        lines[0], lines[1], lines[2]
    );
    let app = build(&text).expect("a script").app;
    let thread = app.session().active.clone().expect("a thread");
    (app, thread)
}

fn tell(app: &mut CodexApp, millis: i64) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, AppEv::Clock(Now::at_millis(BASE + millis)), &mut fx);
    fx
}

fn say(app: &mut CodexApp, intent: Questions) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, AppEv::Intent(Intent::Feature(intent.into())), &mut fx);
    fx
}

fn left(app: &CodexApp, thread: &ThreadId) -> Option<u64> {
    app.features().async_questions.countdown(thread)
}

#[test]
fn a_question_that_arrives_closed_counts_down_in_its_last_twenty_seconds() {
    let (mut app, thread) = asked();
    assert_eq!(app.features().async_questions.of(&thread).len(), 1);
    assert_eq!(left(&app, &thread), None, "thirty seconds are not shown");
    // the wake is the start of the last twenty seconds
    let fx = tell(&mut app, 0);
    assert_eq!(
        fx.wake().expect("it counts").at(),
        Now::at_millis(BASE + 10_000)
    );
    tell(&mut app, 10_000);
    assert_eq!(left(&app, &thread), Some(20));
    let fx = tell(&mut app, 15_000);
    assert_eq!(left(&app, &thread), Some(15));
    assert_eq!(
        fx.wake().expect("each second").at(),
        Now::at_millis(BASE + 15_000 + 1_000)
    );
    tell(&mut app, 29_500);
    assert_eq!(left(&app, &thread), Some(1), "rounded up");
    let fx = tell(&mut app, 30_000);
    assert_eq!(left(&app, &thread), None, "it has run out");
    assert_eq!(fx.wake(), None, "and nothing waits for the clock");
    assert_eq!(
        app.features().async_questions.of(&thread).len(),
        1,
        "the question itself stays"
    );
}

#[test]
fn the_user_at_the_editor_stops_the_countdown() {
    let (mut app, thread) = asked();
    tell(&mut app, 12_000);
    assert_eq!(left(&app, &thread), Some(18));
    let fx = say(
        &mut app,
        Questions::Snooze {
            thread: thread.clone(),
        },
    );
    assert_eq!(left(&app, &thread), None);
    assert_eq!(fx.wake(), None, "no wake stands for a snoozed question");
    tell(&mut app, 20_000);
    assert_eq!(left(&app, &thread), None, "it stays stopped");
}

#[test]
fn a_message_of_the_users_own_overtakes_the_questions() {
    let (mut app, thread) = asked();
    say(
        &mut app,
        Questions::Superseded {
            thread: thread.clone(),
        },
    );
    assert!(app.features().async_questions.of(&thread).is_empty());
    assert_eq!(left(&app, &thread), None);
}
