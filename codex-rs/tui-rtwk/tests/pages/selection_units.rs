// test root: pages
//! Mouse selection of painted transcript text and aligned text in rtwk.

use codex_tui_core::app::Command;
use codex_tui_core::app::effects::{Ask, CopyDestination};
use rtwk::gestures::{SelectionArea, SelectionAreaController};
use rtwk::layout::{EdgeInsets, Padding, SizedBox};
use rtwk::style::{Attrs, Style};
use rtwk::text::{Text, TextAlign};
use rtwk_core::input::{Mods, MouseKind};
use rtwk_test::{Options, Tester};

use crate::kit::{Owner, Screen};

const ALPHA: &str = include_str!("../oracle/fixtures/transcript_view_2/alpha.jsonl");

#[test]
fn x11_word_and_line_selections_each_publish_once_on_release() {
    for auto_copy in [false, true] {
        let mut s = Screen::build(Owner::ChatLive)
            .size(24, 5)
            .script(ALPHA)
            .view_only()
            .open();
        s.selection_copy_policy(auto_copy, false);
        s.primary_selection();
        let line = "alpha beta gamma";
        let y = u16::try_from(s.row_of(line)).expect("screen row");
        s.click(9, y);
        s.mouse_down(9, y);
        s.assert_copies_asked(&[]);
        s.mouse_up(9, y);
        s.assert_copies_asked(&["beta"]);
        s.assert_primary_copies_asked(&[("beta", auto_copy)]);
        s.mouse_down(9, y);
        s.assert_copies_asked(&["beta"]);
        s.mouse_up(9, y);
        s.assert_copies_asked(&["beta", line]);
        s.assert_primary_copies_asked(&[("beta", auto_copy), (line, auto_copy)]);
        s.assert_text_style(line, "reversed");
    }
}

#[test]
fn x11_release_only_replaces_its_own_native_clipboard_write() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(24, 5)
        .script(ALPHA)
        .view_only()
        .open();
    s.selection_copy_policy(false, false);
    s.primary_selection();
    let y = u16::try_from(s.row_of("alpha beta gamma")).expect("screen row");
    s.mouse_down(2, y);
    s.mouse_drag(6, y);
    assert_eq!(
        s.release_with_native_copies(6, y, &["earlier copy", "alpha"]),
        ["earlier copy", "alpha"]
    );
    s.assert_copies_asked(&["alpha"]);
    s.assert_primary_copies_asked(&[("alpha", false)]);
}

#[test]
fn a_keyboard_range_replacing_a_drag_is_not_published_to_primary() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(24, 5)
        .script(ALPHA)
        .view_only()
        .open();
    s.selection_copy_policy(false, false);
    s.primary_selection();
    let y = u16::try_from(s.row_of("alpha beta gamma")).expect("screen row");
    s.mouse_down(8, y);
    s.mouse_drag(11, y);
    s.assert_text_style("beta", "reversed");
    s.keys("shift+left");
    s.assert_text_style("bet", "reversed");
    s.mouse_up(11, y);
    s.assert_copies_asked(&[]);

    s.wait(350);
    s.drag(8, y, 11, y);
    s.assert_primary_copies_asked(&[("beta", false)]);
    let owner = s
        .commands()
        .into_iter()
        .find_map(|command| match command {
            Command::Effect {
                ask:
                    Ask::Copy {
                        destination: CopyDestination::Primary(owner),
                        ..
                    },
                ..
            } => Some(owner),
            _ => None,
        })
        .expect("owner of the completed mouse selection");
    assert!(owner.is_current());
    s.keys("shift+left");
    s.assert_text_style("bet", "reversed");
    assert!(!owner.is_current());
}

#[test]
fn a_host_without_x11_primary_keeps_its_clipboard_policy() {
    for auto_copy in [false, true] {
        let mut s = Screen::build(Owner::ChatLive)
            .size(24, 5)
            .script(ALPHA)
            .view_only()
            .open();
        s.selection_copy_policy(auto_copy, false);
        let y = u16::try_from(s.row_of("alpha beta gamma")).expect("screen row");
        s.drag(2, y, 6, y);
        s.assert_copies_asked(if auto_copy { &["alpha"] } else { &[] });
        s.assert_primary_copies_asked(&[]);
        s.assert_text_style("alpha", "reversed");
    }
}

#[test]
fn primary_without_clipboard_copy_keeps_the_selection_and_stays_quiet() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(60, 18)
        .script(ALPHA)
        .open();
    s.selection_copy_policy(false, true);
    s.primary_selection();
    let y = u16::try_from(s.row_of("alpha beta gamma")).expect("screen row");
    s.drag(2, y, 6, y);
    s.assert_copies_asked(&["alpha"]);
    s.assert_primary_copies_asked(&[("alpha", false)]);
    let before = s.shot();
    s.answer_copy(Ok(()));
    s.assert_same_rows(&before);
    s.assert_text_style("alpha", "reversed");
    s.wait(350);
    s.drag(8, y, 11, y);
    s.assert_copies_asked(&["alpha", "beta"]);
    s.assert_primary_copies_asked(&[("alpha", false), ("beta", false)]);
    let before = s.shot();
    s.answer_copy(Err("PRIMARY unavailable"));
    s.assert_same_rows(&before);
    s.assert_text_style("beta", "reversed");
}

#[test]
fn x11_primary_does_not_claim_link_clicks_or_empty_drags() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(36, 6)
        .script(include_str!(
            "../oracle/fixtures/transcript_view_2/link.jsonl"
        ))
        .view_only()
        .open();
    s.selection_copy_policy(false, false);
    s.primary_selection();
    let y = u16::try_from(s.row_of("example.com")).expect("screen row");
    s.click(2, y);
    s.assert_opened_urls(&["https://example.com/docs"]);
    s.assert_copies_asked(&[]);
    s.wait(350);
    s.mouse_down(2, y);
    s.mouse_drag(4, y);
    s.mouse_drag(2, y);
    s.mouse_up(2, y);
    s.assert_opened_urls(&["https://example.com/docs"]);
    s.assert_copies_asked(&[]);
}

#[test]
fn a_dialog_opened_during_an_x11_drag_owns_the_release() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(72, 20)
        .script(ALPHA)
        .open();
    s.selection_copy_policy(true, false);
    s.primary_selection();
    let y = u16::try_from(s.row_of("alpha beta gamma")).expect("screen row");
    s.mouse_down(2, y);
    s.mouse_drag(6, y);
    s.assert_text_style("alpha", "reversed");
    s.type_text("/permissions").keys("enter");
    s.assert_contains("Update Model Permissions");
    s.mouse_up(6, y);
    s.assert_copies_asked(&[]);
    s.keys("escape").wait(350);
    let y = u16::try_from(s.row_of("alpha beta gamma")).expect("screen row");
    s.drag(2, y, 6, y);
    s.assert_copies_asked(&["alpha"]);
    s.assert_primary_copies_asked(&[("alpha", true)]);
}

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
