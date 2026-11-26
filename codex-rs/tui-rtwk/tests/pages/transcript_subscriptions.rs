// test root: pages
//! The widget invalidation boundary of the live chat's transcript.
#![allow(clippy::expect_used)]

use codex_tui_core::app::AppEv;
use codex_tui_core::fixtures::builders::{
    agent_completed, agent_delta, command_completed, command_output, command_started,
    turn_completed,
};
use codex_tui_core::ids::{ItemId, TurnId};
use codex_tui_core::threads::ItemKey;
use codex_tui_rtwk::services::backend::fake::FAKE_THREAD;
use rtwk::scroll::ListView;
use rtwk::{AnyWidget, Eid, Key};

use crate::common::{Harness, start_driven};

fn list(h: &Harness) -> AnyWidget {
    let tree = h.t.tree().expect("mounted app");
    h.t.app_elements()
        .into_iter()
        .map(|element| tree.widget(element))
        .find(|widget| widget.is::<ListView>())
        .expect("transcript list")
        .clone()
}

fn row(h: &Harness, turn: &str, item: &str) -> (Eid, AnyWidget) {
    let key = Key::value(ItemKey {
        turn: TurnId::new(turn),
        item: ItemId::new(item),
    });
    let tree = h.t.tree().expect("mounted app");
    let element =
        h.t.app_elements()
            .into_iter()
            .find(|element| tree.widget(*element).key() == Some(&key))
            .expect("mounted transcript row");
    let child = tree.children(element)[0];
    (element, tree.widget(child).clone())
}

fn height(h: &Harness, element: Eid) -> i32 {
    let tree = h.t.tree().expect("mounted app");
    let render = tree.render_object_of(element).expect("row render object");
    tree.render_tree().size(render).h
}

fn chat_with_older_answer() -> Harness {
    let mut h = start_driven(false, 80, 30);
    h.wait_idle();
    h.fake.script_turn(vec![
        agent_completed(FAKE_THREAD, "turn-1", "msg-1", "older answer"),
        turn_completed(FAKE_THREAD, "turn-1"),
    ]);
    h.t.type_text("first prompt");
    h.press("enter");
    h.wait_for("the older answer", |s| s.contains("• older answer"));
    h.wait_idle();
    h
}

#[test]
fn live_deltas_update_and_resize_their_row_without_rebuilding_the_list_or_other_rows() {
    let mut h = chat_with_older_answer();
    h.fake
        .script_turn(vec![agent_delta(FAKE_THREAD, "turn-2", "msg-2", "first")]);
    h.t.type_text("next prompt");
    h.press("enter");
    h.wait_for("the new live row", |screen| screen.contains("• first"));

    let before_list = list(&h);
    let (history_id, before_history) = row(&h, "turn-1", "msg-1");
    let (live_id, before_live) = row(&h, "turn-2", "msg-2");
    let before_height = height(&h, live_id);
    let epoch = h.fake.epoch().expect("attached");
    // Both notifications reach the backend before the next frame.
    for delta in [" second", " third"] {
        h.fake.inject(
            epoch,
            AppEv::Backend(agent_delta(FAKE_THREAD, "turn-2", "msg-2", delta)),
        );
    }
    h.wait_for("both deltas to paint", |screen| {
        screen.contains("• first second third")
    });

    let (new_history_id, history) = row(&h, "turn-1", "msg-1");
    let (new_live_id, live) = row(&h, "turn-2", "msg-2");
    assert!(AnyWidget::identical(&before_list, &list(&h)));
    assert_eq!(history_id, new_history_id);
    assert!(AnyWidget::identical(&before_history, &history));
    assert_eq!(live_id, new_live_id);
    assert!(!AnyWidget::identical(&before_live, &live));
    assert_eq!(height(&h, live_id), before_height);

    h.fake.inject(
        epoch,
        AppEv::Backend(agent_delta(
            FAKE_THREAD,
            "turn-2",
            "msg-2",
            "\n\n## Later heading\n",
        )),
    );
    h.wait_for("the whole answer after it grows", |screen| {
        screen.contains("• first second third") && screen.contains("## Later heading")
    });
    assert!(height(&h, live_id) > before_height);
    assert!(AnyWidget::identical(&before_list, &list(&h)));
    assert!(AnyWidget::identical(
        &before_history,
        &row(&h, "turn-1", "msg-1").1
    ));
    assert_eq!(live_id, row(&h, "turn-2", "msg-2").0);

    h.fake
        .inject(epoch, AppEv::Backend(turn_completed(FAKE_THREAD, "turn-2")));
    h.wait_idle();
    assert!(h.t.screen().contains("• first second third"));
    assert_eq!(live_id, row(&h, "turn-2", "msg-2").0);
}

#[test]
fn only_new_f4_eligibility_rebuilds_the_list_during_command_output() {
    const TURN: &str = "turn-2";
    const ITEM: &str = "call-1";
    const COMMAND: &str = "echo build";
    let notify = |delta: &str| {
        AppEv::Backend(command_output(FAKE_THREAD, TURN, ITEM, delta).expect("output delta"))
    };

    let mut h = chat_with_older_answer();
    h.fake.script_turn(vec![
        command_started(FAKE_THREAD, TURN, ITEM, COMMAND).expect("command start"),
    ]);
    h.t.type_text("run tests");
    h.press("enter");
    h.wait_for("the running command", |screen| screen.contains(COMMAND));

    let epoch = h.fake.epoch().expect("attached");
    let before_output = list(&h);
    h.fake.inject(epoch, notify("one\r\ntwo\r\nthree\r"));
    h.wait_for("the third line", |screen| screen.contains("three"));
    assert!(AnyWidget::identical(&before_output, &list(&h)));
    h.press("f4");
    assert!(!h.t.screen().contains("enter expand"));

    let before_threshold = list(&h);
    h.fake.inject(epoch, notify("\n"));
    h.advance(20);
    assert!(AnyWidget::identical(&before_threshold, &list(&h)));
    assert!(!h.t.screen().contains("+ 1 line"));

    // A whitespace-only fourth line counts, even with a split CRLF.
    h.fake.inject(epoch, notify(" \r"));
    h.wait_for("the new disclosure", |screen| screen.contains("+ 1 line"));
    assert!(!AnyWidget::identical(&before_threshold, &list(&h)));
    h.press("f4");
    assert!(h.t.screen().contains("enter expand"));
    h.press("enter");
    assert!(h.t.screen().contains("− Hide details"));
    h.press("enter");
    assert!(h.t.screen().contains("+ 1 line"));
    h.press("esc");

    let stable_list = list(&h);
    let (history_id, old_history) = row(&h, "turn-1", "msg-1");
    let (tool_id, mut old_tool) = row(&h, TURN, ITEM);
    for n in 1..=32 {
        let line = format!("stdout update {n:02}");
        let delta = format!("\n{line}\r");
        h.fake.inject(epoch, notify(&delta));
        h.wait_for("the latest command output", |screen| screen.contains(&line));
        assert!(AnyWidget::identical(&stable_list, &list(&h)));
        let (next_history_id, history) = row(&h, "turn-1", "msg-1");
        assert_eq!(history_id, next_history_id);
        assert!(AnyWidget::identical(&old_history, &history));
        let (next_tool_id, tool) = row(&h, TURN, ITEM);
        assert_eq!(tool_id, next_tool_id);
        assert!(!AnyWidget::identical(&old_tool, &tool));
        old_tool = tool;
    }

    assert!(h.t.screen().contains("+ 33 lines (⌃t to expand)"));

    h.fake.inject(
        epoch,
        AppEv::Backend(
            command_completed(FAKE_THREAD, TURN, ITEM, COMMAND, "finished", 7)
                .expect("command completion"),
        ),
    );
    h.wait_for("the completed tool's output and status", |screen| {
        screen.contains("finished") && screen.contains("Exited (status 7)")
    });
    assert!(!h.t.screen().contains("stdout update 32"));
    assert_eq!(tool_id, row(&h, TURN, ITEM).0);
    h.press("f4");
    assert!(!h.t.screen().contains("enter expand"));
}
