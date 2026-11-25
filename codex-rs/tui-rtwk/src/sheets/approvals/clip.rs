//! A header cut to the room the rest of the dialog leaves (the oracle's `list_selection_view`
//! render and `picker_areas`): a header taller than that room shows its first rows and a dim
//! `[… N lines] ⌃a view all` row (the key the user bound to `approval.open_fullscreen`) in place of its last one, N being the rows of the whole header.
//!
//! Flutter writes it as a `Column` whose header is a `Flexible` child under a `ClipRect`, and the
//! elision shown when the text says it overflows. Here the room is measured (`LayoutBuilder` +
//! `segmenter_of(cx)` + `widgets/measure.rs`, the rows `Text` breaks the lines into) and the dialog
//! picks WHICH composition to build: the whole column, or the header in a box of the room it has
//! (`SizedOverflowBox` cuts what sticks out) above the elision row. The blank row before the hint
//! gives way first: a header that fits only without it is not clipped, the oracle's order.

use rtwk::layout::{
    Alignment, Column, CrossAxisAlignment, EdgeInsets, MainAxisSize, OverflowBox, Padding,
    SizedOverflowBox,
};
use rtwk::style::Style;
use rtwk::text::{Text, segmenter_of};
use rtwk::{AnyWidget, LayoutBuilder};

use super::strings::Strings;
use crate::widgets::measure;

/// One row group of the header: its widget (already inset) and the text the measure reads.
#[derive(Clone)]
pub struct Block {
    pub widget: AnyWidget,
    pub text: String,
}

/// The pane of a request dialog: a blank row, the header, the options, a blank row, the hint.
#[derive(Clone)]
pub struct Pane {
    pub header: Vec<Block>,
    /// The blank rows between the header and the options when there is room for them.
    pub gap: usize,
    pub options: AnyWidget,
    /// What each option row says (the measure of the rows the list takes).
    pub option_texts: Vec<String>,
    pub hint: AnyWidget,
    pub hint_text: String,
    pub dim: Style,
    pub strings: &'static Strings,
    /// The label of the key bound to `approval.open_fullscreen` (`⌃a` by default); none when the
    /// user unbound it: the row then says only how many lines were cut.
    pub view_all: Option<String>,
}

/// The two cells the dialog is inset on each side.
const INSET: i32 = 2;
/// The cursor mark and the blank that keeps its place (`› `).
const CURSOR: i32 = 2;

impl Pane {
    pub fn view(self) -> AnyWidget {
        LayoutBuilder::new(move |cx, constraints| {
            let seg = segmenter_of(cx);
            let width = constraints.max_width;
            let text_width = usize::try_from((width - 2 * INSET).max(1)).unwrap_or(1);
            let rows = |text: &str, width: usize| measure::rows(seg, text, width).len().max(1);
            let header_rows: usize = self
                .header
                .iter()
                .map(|block| rows(&block.text, text_width))
                .sum();
            let option_width = usize::try_from((width - CURSOR).max(1)).unwrap_or(1);
            let options_rows: usize = self
                .option_texts
                .iter()
                .map(|text| rows(text, option_width))
                .sum();
            let hint_rows = rows(&self.hint_text, text_width);
            // The blank row on top, the options and the hint stay; what is left is the header's
            // (`picker_areas`: the header first, then the gap under it, then the blank row
            // before the hint).
            let fixed = 1 + options_rows + hint_rows;
            let height = usize::try_from(constraints.max_height.max(0)).unwrap_or(usize::MAX);
            let room = height.saturating_sub(fixed);
            let mut pane = Column::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .child(Text::new(String::new()));
            if header_rows <= room {
                let spare = room - header_rows;
                let gap = spare.min(self.gap);
                let before_hint = (spare - gap).min(1);
                for block in &self.header {
                    pane = pane.child(block.widget.clone());
                }
                for _ in 0..gap {
                    pane = pane.child(Text::new(String::new()));
                }
                pane = pane.child(self.options.clone());
                for _ in 0..before_hint {
                    pane = pane.child(Text::new(String::new()));
                }
                return pane.child(self.hint.clone()).into();
            }
            // Cut: the room less the elision row, at least the elision row itself.
            let shown = room.saturating_sub(1);
            let mut header = Column::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Stretch);
            for block in &self.header {
                header = header.child(block.widget.clone());
            }
            // The header is laid out at the height it wants and the box shows its top.
            let whole = OverflowBox::new(header)
                .max_height(i32::try_from(header_rows).unwrap_or(i32::MAX))
                .alignment(Alignment::TOP_LEFT);
            let cut = SizedOverflowBox::new(width, i32::try_from(shown).unwrap_or(i32::MAX), whole)
                .alignment(Alignment::TOP_LEFT);
            pane.child(cut)
                .child(self.elision(header_rows))
                .child(self.options.clone())
                .child(self.hint.clone())
                .into()
        })
        .into()
    }

    /// `[… N lines] ⌃a view all`, dim, as the oracle's footer of a clipped header (its key the
    /// bound one, none when unbound).
    fn elision(&self, rows: usize) -> AnyWidget {
        let s = self.strings;
        let line = match &self.view_all {
            Some(key) => format!("{} {key}{}", s.header_cut(rows), s.view_all),
            None => s.header_cut(rows).to_string(),
        };
        Padding::new(
            EdgeInsets::symmetric(INSET, 0),
            Text::new(line).style(self.dim.clone()),
        )
        .into()
    }
}
