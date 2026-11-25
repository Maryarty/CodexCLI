#![allow(clippy::expect_used, clippy::unwrap_used)]

//! The slash validation gate of `chatwidget/slash_dispatch.rs:150-167`: which commands a running
//! turn, a start in flight, a review and a queued follow-up refuse.

use codex_tui_core::commands::*;
use pretty_assertions::assert_eq;

fn refused(command: SlashCommand) -> Outcome {
    Outcome::Notice(Notice::DisabledDuringTask(command))
}

fn idle() -> DispatchCx {
    DispatchCx {
        thread_started: true,
        ..Default::default()
    }
}

#[test]
fn fork_and_review_are_refused_in_review_mode() {
    let cx = DispatchCx {
        review_mode: true,
        ..idle()
    };
    for (line, command) in [
        ("/fork", SlashCommand::Fork),
        ("/fork named", SlashCommand::Fork),
        ("/review", SlashCommand::Review),
        ("/review changes", SlashCommand::Review),
    ] {
        assert_eq!(dispatch(line, &cx), refused(command), "{line}");
    }
    assert!(!matches!(dispatch("/fork", &idle()), Outcome::Notice(_)));
    assert!(!matches!(dispatch("/review", &idle()), Outcome::Notice(_)));
}

#[test]
fn review_is_refused_with_a_queued_follow_up_but_fork_is_not() {
    let cx = DispatchCx {
        queued_follow_up: true,
        ..idle()
    };
    assert_eq!(dispatch("/review", &cx), refused(SlashCommand::Review));
    assert_eq!(
        dispatch("/review changes", &cx),
        refused(SlashCommand::Review)
    );
    assert!(!matches!(dispatch("/fork", &cx), Outcome::Notice(_)));
}

#[test]
fn resume_ignores_review_mode_but_not_a_running_turn() {
    // `/resume` is available during a task: only a pending start or a running turn refuses it.
    let review = DispatchCx {
        review_mode: true,
        ..idle()
    };
    assert!(!matches!(dispatch("/resume", &review), Outcome::Notice(_)));
    let busy = DispatchCx {
        task_running: true,
        ..idle()
    };
    assert_eq!(dispatch("/resume", &busy), refused(SlashCommand::Resume));
    // `/cd` is not available during a task: the review refuses it as well (the first clause).
    assert_eq!(dispatch("/cd /tmp", &review), refused(SlashCommand::Cd));
}

#[test]
fn a_started_turn_refuses_what_it_refused_before() {
    let cx = DispatchCx {
        task_running: true,
        ..idle()
    };
    for (line, command) in [
        ("/fork", SlashCommand::Fork),
        ("/review", SlashCommand::Review),
        ("/clear", SlashCommand::Clear),
        ("/new", SlashCommand::New),
    ] {
        assert_eq!(dispatch(line, &cx), refused(command), "{line}");
    }
    // what a task allows stays open
    assert!(!matches!(dispatch("/status", &cx), Outcome::Notice(_)));
}
