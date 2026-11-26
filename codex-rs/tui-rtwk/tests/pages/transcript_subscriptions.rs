// test root: pages
//! The widget invalidation boundary of the live chat's transcript.
#![allow(clippy::expect_used)]

use codex_tui_core::app::AppEv;
use codex_tui_core::fixtures::builders::{agent_completed, agent_delta, turn_completed};
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

#[test]
fn live_deltas_update_and_resize_their_row_without_rebuilding_the_list_or_other_rows() {
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
