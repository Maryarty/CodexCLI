//! Probe of rtwk gap G46 (the DIM attribute on blank cells). The oracle
//! (codex-rs/tui `external_agent_config_migration/mod.rs`
//! `action_rows_keep_single_line_allocation_and_focus_style`) styles a whole
//! row with `buf.set_style(area, Style::default().dim())` and asserts that the
//! row's cell 2, a blank, is DIM. In rtwk a blank cell of a dim `Text` (and of
//! a `BoxWidget` filled with the dim style) reads back without DIM, the glyph
//! cells have it on the terminal. The painted surface keeps DIM on the blank
//! (the last probe): the presenter leaves out an attribute that does not show
//! on a blank. So the oracle test
//! `import::action_rows_keep_single_line_allocation_and_focus_style` reads the
//! surface (`Screen::assert_painted`) and G46 is closed as not a gap.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_methods)]

use rtwk::layout::BoxWidget;
use rtwk::style::{Attrs, Style};
use rtwk::widgets::Text;
use rtwk_test::{Options, Tester};

fn dim_of(t: &Tester, x: usize) -> bool {
    t.cell(x, 0).attrs.intersects(Attrs::DIM)
}

#[test]
fn probe_g46_a_blank_cell_of_a_dim_text_is_not_dim() {
    let dim = Style::new().attrs(Attrs::DIM);
    let mut t = Tester::with(Options::new().size(12, 1));
    t.pump_widget(Text::new("  1. a").style(dim));
    assert!(dim_of(&t, 5), "the glyph `a` is dim");
    assert!(
        !dim_of(&t, 0),
        "G46: the blank in the first cell is not dim"
    );
}

#[test]
fn probe_g46_a_blank_cell_under_a_dim_box_is_not_dim() {
    let dim = Style::new().attrs(Attrs::DIM);
    let mut t = Tester::with(Options::new().size(12, 1));
    t.pump_widget(
        BoxWidget::new()
            .style(dim.clone())
            .child(Text::new("  1. a").style(dim)),
    );
    assert!(dim_of(&t, 5), "the glyph `a` is dim");
    assert!(
        !dim_of(&t, 0),
        "G46: the blank in the first cell is not dim"
    );
    assert!(!dim_of(&t, 11), "G46: the blank after the text is not dim");
}

#[test]
fn probe_g46_the_painted_surface_keeps_dim_on_the_blank() {
    let dim = Style::new().attrs(Attrs::DIM);
    let mut t = Tester::with(Options::new().size(12, 1));
    t.pump_widget(Text::new("  1. a").style(dim));
    let frame = t.frame_loop();
    let painted = |x| {
        frame
            .styles()
            .attrs_of(frame.surface().style(x, 0))
            .intersects(Attrs::DIM)
    };
    assert!(painted(5), "the glyph `a` is dim");
    assert!(
        painted(0),
        "the blank in the first cell is dim on the surface"
    );
}
