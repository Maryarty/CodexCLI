use codex_tui_rtwk::widgets::session_list::*;
use rtwk::AnyWidget;
use rtwk::Handler;
use rtwk::NotificationListener;
use rtwk::controls::Pressable;
use rtwk::controls::SelectionController;
use rtwk::focus::FocusNode;
use rtwk::foundation::ListenableBuilder;
use rtwk::foundation::ValueNotifier;
use rtwk::gestures::GestureDetector;
use rtwk::gestures::PointerEvent;
use rtwk::scroll::ListController;
use rtwk::scroll::ListView;
use rtwk::scroll::ScrollNotification;
use std::rc::Rc;

fn all(_: usize) -> bool {
    true
}

#[test]
fn a_step_goes_round_the_ends() {
    assert_eq!(target(3, Some(2), &all, Move::Step(true), 1), Some(0));
    assert_eq!(target(3, Some(0), &all, Move::Step(false), 1), Some(2));
    assert_eq!(target(3, None, &all, Move::Step(true), 1), Some(0));
}

#[test]
fn a_page_stops_at_the_ends_and_skips_rows_without_a_cursor() {
    let not_one = |at: usize| at != 5;
    assert_eq!(target(30, Some(0), &not_one, Move::Page(true), 5), Some(6));
    assert_eq!(target(30, Some(29), &all, Move::Page(true), 5), Some(29));
    assert_eq!(target(30, Some(3), &all, Move::Page(false), 5), Some(0));
    assert_eq!(target(30, Some(9), &all, Move::Edge(true), 5), Some(29));
}
