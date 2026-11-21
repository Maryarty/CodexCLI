//! Probe of rtwk gap G46 (the DIM attribute on blank cells). The oracle
//! (codex-rs/tui `external_agent_config_migration/mod.rs`
//! `action_rows_keep_single_line_allocation_and_focus_style`) styles a whole
//! row with `buf.set_style(area, Style::default().dim())` and asserts that the
//! row's cell 2, a blank, is DIM. In rtwk a blank cell of a dim `Text` (and of
//! a `BoxWidget` filled with the dim style) reads back without DIM, the glyph
//! cells have it. These probes pin that behaviour: when rtwk keeps DIM on
//! blanks they fail, and the oracle test `import::action_rows_keep_single_line_allocation_and_focus_style`
//! loses its `pending(G46)`.
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
