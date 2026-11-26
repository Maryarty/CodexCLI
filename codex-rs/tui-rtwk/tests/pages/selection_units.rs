// test root: pages
//! Mouse selection of painted transcript text and aligned text in rtwk.

use rtwk::gestures::{SelectionArea, SelectionAreaController};
use rtwk::layout::{EdgeInsets, Padding, SizedBox};
use rtwk::style::{Attrs, Style};
use rtwk::text::{Text, TextAlign};
use rtwk_core::input::{Mods, MouseKind};
use rtwk_test::{Options, Tester};

use crate::kit::{Owner, Screen};

#[test]
fn returning_a_drag_to_its_origin_leaves_no_selection_to_copy() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(36, 6)
        .script(include_str!(
            "../oracle/fixtures/transcript_view_2/link.jsonl"
        ))
        .view_only()
        .open();
    s.selection_copy_policy(true, false);
    let y = u16::try_from(s.row_of("example.com")).expect("screen row");
    s.mouse_down(2, y);
    s.mouse_drag(5, y);
    s.assert_text_style("exa", "reversed");
    s.mouse_drag(2, y);
    s.mouse_up(2, y);
    s.assert_text_style("exa", "!reversed");
    s.assert_copies_asked(&[]);
    s.assert_opened_urls(&[]);
}

#[test]
fn returning_to_the_same_cell_after_scrolling_keeps_the_drag_selection() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(20, 8)
        .script(include_str!(
            "../oracle/fixtures/transcript_view_5/rows40.jsonl"
        ))
        .view_only()
        .open();
    s.selection_copy_policy(true, false);
    s.wheel_up(0, 0, 12);
    let before = s.shot();
    s.mouse_down(2, 3);
    s.mouse_drag(5, 3);
    s.wheel_down(5, 3, 1);
    s.assert_rows_differ(&before);
    s.mouse_drag(2, 3);
    s.assert_style(2, 3, "reversed");
    s.mouse_up(2, 3);
    s.assert_style(2, 3, "reversed");
    s.assert_copies_asked(&["row 20\nr"]);
}

#[test]
fn a_right_aligned_paragraph_selects_the_letters_before_its_reserved_margin() {
    let selection = SelectionAreaController::new();
    let area = SelectionArea::new(Padding::new(
        EdgeInsets::only(0, 0, 2, 0),
        SizedBox::new(
            Some(6),
            Some(1),
            Some(Text::new("ab").text_align(TextAlign::Right).into()),
        ),
    ))
    .controller(selection.clone())
    .style(Style::new().attrs(Attrs::REVERSE).pen())
    .copy_on_release(false);
    let mut t = Tester::with(Options::new().size(8, 1));
    t.pump_widget(area);
    assert_eq!(t.screen().trim_end(), "    ab");
    t.mouse(MouseKind::Down, 4, 0, 0, Mods::NONE);
    t.mouse(MouseKind::Drag, 6, 0, 0, Mods::NONE);
    t.mouse(MouseKind::Up, 6, 0, 0, Mods::NONE);
    t.pump();

    assert_eq!(selection.text(), "ab");
    for col in 0..8 {
        assert_eq!(
            t.cell(col, 0).attrs.intersects(Attrs::REVERSE),
            (4..6).contains(&col),
            "column {col}",
        );
    }
}
