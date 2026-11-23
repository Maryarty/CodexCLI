use codex_tui_rtwk::widgets::tab_strip::*;
use rtwk::AnyWidget;
use rtwk::Handler;
use rtwk::LayoutBuilder;
use rtwk::gestures::GestureDetector;
use rtwk::gestures::PointerEvent;
use rtwk::layout::Expanded;
use rtwk::layout::Row;
use rtwk::layout::SizedBox;
use rtwk::style::Style;
use rtwk::text::Text;
use rtwk::text::segmenter_of;
use rtwk_core::unicode::TextSegmenter;
use std::rc::Rc;

#[test]
fn the_strip_scrolls_around_the_active_tab() {
    let labels: Vec<String> = ["All 0", "Needs you 0", "Working 0", "Ready 0", "Inactive 0"]
        .iter()
        .map(ToString::to_string)
        .collect();
    let narrow = strip(TextSegmenter::default(), &labels, 0, 44);
    assert_eq!((narrow.start, narrow.end, narrow.right), (0, 3, true));
    let wide = strip(TextSegmenter::default(), &labels, 0, 76);
    assert_eq!((wide.end, wide.right), (5, false));
}
