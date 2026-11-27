//! The workspace keeps each retained conversation's model and draft under its own pane.

use codex_tui_core::app::facts::TuiSettings;
use codex_tui_core::app::{AppEv, Command};
use codex_tui_core::fixtures::builders::{
    agent_completed, agent_delta, turn_completed, turn_started,
};
use codex_tui_core::keymap::{Keymap, KeymapSpec};
use codex_tui_rtwk::services::backend::fake::FAKE_THREAD;

use crate::common::{Harness, start_driven};
use crate::side_return::{SIDE, forked};

fn workspace(enabled: bool, owned: bool) -> Harness {
    let mut h = start_driven(false, 120, 24);
    h.connect();
    h.quiesce();
    let spec = KeymapSpec::default()
        .with("global", "focus_main_composer", &["f12 h"])
        .with("global", "focus_sidebar_content", &["f12 s"])
        .with("global", "hide_pane", &["f12 x"])
        .with("global", "close_sidebar_tab", &["f12 c"])
        .with("global", "toggle_pane_zoom", &["f12 z"])
        .with("global", "move_divider_left", &["f12 j"])
        .with("global", "resize_panes", &["f12 r"]);
    let keymap = Keymap::resolve_for_workspace(&spec, enabled && owned)
        .unwrap_or_else(|error| panic!("workspace keymap: {error}"));
    h.inject([AppEv::Keymap(Box::new(keymap))]);
    h.inject([AppEv::Tui(TuiSettings {
        transcript_owned: owned,
        workspace_enabled: enabled,
        ..TuiSettings::default()
    })]);
    h.fake.respond(|request| match request["method"].as_str()? {
        "thread/fork" => Some(Ok(forked())),
        "thread/inject_items" | "thread/unsubscribe" | "turn/interrupt" => {
            Some(Ok(serde_json::json!({})))
        }
        _ => None,
    });
    h.t.type_text("/side");
    h.press("enter");
    h.advance(300);
    h.quiesce();
    h
}

fn column(h: &Harness, text: &str) -> usize {
    h.t.lines()
        .iter()
        .find_map(|row| row.find(text).map(|at| row[..at].chars().count()))
        .unwrap_or_else(|| panic!("missing {text:?}:\n{}", h.t.screen()))
}

fn divider(h: &Harness) -> usize {
    h.t.lines()
        .iter()
        .find(|row| row.contains("Side chat"))
        .and_then(|row| row.chars().position(|ch| ch == '│'))
        .unwrap_or_else(|| panic!("missing side pane divider:\n{}", h.t.screen()))
}

#[test]
fn docked_streams_and_submissions_stay_with_their_thread() {
    let mut h = workspace(true, true);
    let split = divider(&h);
    assert_eq!(split, 79);
    assert!(h.t.cursor_visible());
    assert!(h.t.cursor().0 > split, "the child has input focus");

    // The notifications use the same item and turn strings, but their
    // owning thread differs. Updating one must leave the other on screen.
    h.inject([
        AppEv::Backend(turn_started(FAKE_THREAD, "same-turn")),
        AppEv::Backend(turn_started(SIDE, "same-turn")),
        AppEv::Backend(agent_delta(
            FAKE_THREAD,
            "same-turn",
            "same-item",
            "PARENT-LIVE",
        )),
        AppEv::Backend(agent_delta(SIDE, "same-turn", "same-item", "CHILD-LIVE")),
    ]);
    h.advance(150);
    assert!(column(&h, "PARENT-LIVE") < split);
    assert!(column(&h, "CHILD-LIVE") > split);
    h.inject([
        AppEv::Backend(agent_completed(
            FAKE_THREAD,
            "same-turn",
            "same-item",
            "PARENT-DONE",
        )),
        AppEv::Backend(agent_completed(
            SIDE,
            "same-turn",
            "same-item",
            "CHILD-DONE",
        )),
        AppEv::Backend(turn_completed(FAKE_THREAD, "same-turn")),
        AppEv::Backend(turn_completed(SIDE, "same-turn")),
    ]);
    h.advance(150);
    assert!(column(&h, "PARENT-DONE") < split);
    assert!(column(&h, "CHILD-DONE") > split);

    h.t.type_text("child-draft");
    h.press("left");
    h.press("f12 h");
    assert!(h.t.cursor_visible());
    assert!(h.t.cursor().0 < split, "the parent has input focus");
    h.t.type_text("parent-input");
    h.press("enter");
    h.advance(120);
    assert!(h.fake.commands().iter().any(|command| {
        matches!(command, Command::StartTurn { thread, message, .. }
            if thread.as_str() == FAKE_THREAD && message.text == "parent-input")
    }));

    h.t.click((split + 5) as u16, 15);
    h.advance(120);
    assert!(h.t.cursor_visible());
    assert!(
        h.t.cursor().0 > split,
        "clicking the peer returns its focus"
    );
    assert!(column(&h, "child-draft") > split);
    h.t.type_text("!");
    h.press("enter");
    h.advance(120);
    assert!(h.fake.commands().iter().any(|command| {
        matches!(command, Command::StartTurn { thread, message, .. }
            if thread.as_str() == SIDE && message.text == "child-draf!t")
    }));
}

#[test]
fn narrowing_zoom_hide_and_close_preserve_the_retained_pair() {
    let mut h = workspace(true, true);
    h.t.type_text("retained child draft");
    h.advance(120);
    h.press("f12 r");
    assert!(h.t.screen().contains("Resize ·"));
    h.t.click(5, 10);
    h.advance(100);
    h.press("shift+left");
    assert_eq!(divider(&h), 69);
    h.press("h");
    h.t.paste("ignored paste");
    h.press("enter");
    assert_eq!(divider(&h), 68);
    assert!(!h.t.screen().contains("Resize ·"));
    assert!(h.t.screen().contains("retained child draft"));
    assert!(!h.t.screen().contains("ignored paste"));
    assert!(
        !h.fake
            .commands()
            .iter()
            .any(|command| matches!(command, Command::StartTurn { .. }))
    );
    h.press("f12 j");
    let moved = divider(&h);
    assert_eq!(moved, 66);

    h.t.resize(99, 24);
    h.advance(120);
    assert!(!h.t.screen().contains("Side chat"));
    assert_eq!(column(&h, "retained child draft"), 2);
    h.t.resize(100, 24);
    h.advance(120);
    assert_eq!(divider(&h), 63);
    h.t.resize(120, 24);
    h.advance(120);
    assert_eq!(divider(&h), moved);

    h.press("f12 z");
    assert!(!h.t.screen().contains("Side chat"));
    assert_eq!(column(&h, "retained child draft"), 2);
    h.press("f12 z");
    assert_eq!(divider(&h), moved);
    h.press("f12 x");
    assert!(!h.t.screen().contains("retained child draft"));
    assert!(!h.t.screen().contains("Side chat"));
    assert!(
        !h.fake
            .requests()
            .iter()
            .any(|request| request["method"] == "thread/unsubscribe")
    );
    h.press("f12 s");
    assert!(column(&h, "retained child draft") > moved);

    h.press("ctrl+u");
    h.press("f12 c");
    h.advance(300);
    assert!(!h.t.screen().contains("Side chat"));
    assert!(h.fake.requests().iter().any(|request| {
        request["method"] == "thread/unsubscribe" && request["params"]["threadId"] == SIDE
    }));
    h.inject([AppEv::Backend(agent_delta(
        SIDE,
        "late",
        "item",
        "CLOSED-SIDE",
    ))]);
    assert!(!h.t.screen().contains("CLOSED-SIDE"));
}

#[test]
fn workspace_and_owned_transcript_are_both_required_for_docking() {
    for (enabled, owned) in [(false, true), (true, false)] {
        let h = workspace(enabled, owned);
        assert!(h.t.screen().contains("to switch"), "{}", h.t.screen());
        assert!(!h.t.screen().contains("Side chat"), "{}", h.t.screen());
    }
}

#[test]
fn configured_nudge_changes_two_percent_and_only_from_the_child() {
    let mut h = workspace(true, true);
    h.t.resize(200, 24);
    h.advance(120);
    assert_eq!(divider(&h), 133);
    h.press("f12 h");
    h.press("f12 j");
    assert_eq!(divider(&h), 133);
    h.press("f12 s");
    h.press("f12 j");
    assert_eq!(divider(&h), 129);
}

#[test]
fn closing_a_side_chat_asks_before_losing_either_draft_or_temporary_history() {
    for history in [false, true] {
        let mut h = workspace(true, true);
        let loss = if history {
            h.inject([
                AppEv::Backend(turn_started(SIDE, "retained")),
                AppEv::Backend(agent_completed(
                    SIDE,
                    "retained",
                    "answer",
                    "temporary answer",
                )),
                AppEv::Backend(turn_completed(SIDE, "retained")),
            ]);
            "lose this temporary chat’s history"
        } else {
            h.t.type_text("unsent child draft");
            "discard unsent input"
        };
        h.press("f12 c");
        assert!(h.t.screen().contains(loss), "{}", h.t.screen());
        h.press("esc");
        let kept = if history {
            "temporary answer"
        } else {
            "unsent child draft"
        };
        assert!(h.t.screen().contains(kept), "{}", h.t.screen());
        assert!(
            !h.fake
                .requests()
                .iter()
                .any(|request| request["method"] == "thread/unsubscribe")
        );
        h.press("f12 c");
        assert!(h.t.screen().contains(loss));
        h.press("tab");
        h.press("enter");
        h.advance(200);
        assert!(!h.t.screen().contains(kept));
        assert!(h.fake.requests().iter().any(|request| {
            request["method"] == "thread/unsubscribe" && request["params"]["threadId"] == SIDE
        }));
    }
}
