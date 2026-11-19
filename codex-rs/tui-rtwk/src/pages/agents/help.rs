//! The shortcut help in place of the list and the details (oracle:
//! `shortcut_help.rs` `group_lines`, `center_help_lines`): three titled
//! groups of `key  action` rows, flowed into three, two or one column by the
//! width, each group's keys aligned in a `Grid`.

use crate::widgets::measure::Measure;
use rtwk::AnyWidget;
use rtwk::layout::{
    Column, CrossAxisAlignment, Grid, GridPlacement, GridTrack, MainAxisSize, Row, SizedBox,
};
use rtwk::style::Style;
use rtwk::text::Text;

/// A titled group of `(key, action)` entries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Group {
    pub title: String,
    pub entries: Vec<(String, String)>,
}

/// The styles of a help row.
pub struct Ink {
    pub title: Style,
    pub key: Style,
    pub plain: Style,
    pub dim: Style,
}

const GAP: i32 = 4;

fn cells(seg: Measure, text: &str) -> usize {
    seg.width(text)
}

impl Group {
    fn rows(&self) -> usize {
        1 + self.entries.len()
    }

    /// The widest of its title and its rows (the key column and two blanks
    /// before the action).
    fn width(&self, seg: Measure) -> usize {
        let key = self
            .entries
            .iter()
            .map(|(key, _)| cells(seg, key))
            .max()
            .unwrap_or(0);
        let rows = self
            .entries
            .iter()
            .map(|(_, action)| key + 2 + cells(seg, action));
        rows.chain([cells(seg, &self.title)]).max().unwrap_or(0)
    }

    /// The widest key (the keys of the rows cut off count too).
    fn keys(&self, seg: Measure) -> i32 {
        let widest = self.entries.iter().map(|(key, _)| cells(seg, key)).max();
        i32::try_from(widest.unwrap_or(0)).unwrap_or(0)
    }

    /// The group's title and its first `rows - 1` entries.
    fn grid(&self, seg: Measure, rows: usize, ink: &Ink) -> AnyWidget {
        let mut grid = Grid::new()
            .overflow_intended(true)
            .columns([GridTrack::fixed(self.keys(seg)), GridTrack::content()])
            .column_gap(2)
            .child(
                GridPlacement::new(Text::line(self.title.clone()).style(ink.title.clone()))
                    .span(1, 2),
            );
        for (key, action) in self.entries.iter().take(rows.saturating_sub(1)) {
            grid = grid
                .child(Text::line(key.clone()).style(ink.key.clone()))
                .child(Text::line(action.clone()).style(ink.plain.clone()));
        }
        grid.into()
    }
}

/// Groups stacked with a blank row between, cut to `keep` rows.
fn stacked(seg: Measure, groups: &[&Group], keep: usize, ink: &Ink) -> AnyWidget {
    let mut column = Column::new()
        .overflow_intended(true)
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start);
    let mut left = keep;
    for (at, group) in groups.iter().enumerate() {
        if at > 0 {
            if left <= 1 {
                break;
            }
            column = column.child(SizedBox::fixed(1, 1));
            left -= 1;
        }
        let rows = group.rows().min(left);
        column = column.child(group.grid(seg, rows, ink));
        left -= rows;
        if left == 0 {
            break;
        }
    }
    column.into()
}

/// The help under its `title`, in `width` cells and `fits` rows: the columns
/// the width holds, and the resize note (`resize`) when the rows do not fit.
pub fn help(
    seg: Measure,
    title: &str,
    groups: &[Group; 3],
    (width, fits): (usize, usize),
    resize: &str,
    ink: &Ink,
) -> AnyWidget {
    let [a, b, c] = groups;
    let (wa, wb, wc) = (a.width(seg), b.width(seg), c.width(seg));
    let (columns, rows): (Vec<Vec<&Group>>, usize) = if wa + wb + wc + 2 * GAP as usize <= width {
        (
            vec![vec![a], vec![b], vec![c]],
            a.rows().max(b.rows()).max(c.rows()),
        )
    } else if wa + GAP as usize + wb.max(wc) <= width {
        (
            vec![vec![a], vec![b, c]],
            a.rows().max(b.rows() + 1 + c.rows()),
        )
    } else {
        (vec![vec![a, b, c]], a.rows() + b.rows() + c.rows() + 2)
    };
    let cut = rows + 2 > fits;
    let keep = if cut { fits.saturating_sub(3) } else { rows };
    let mut row = Row::new()
        .overflow_intended(true)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .spacing(GAP);
    for column in &columns {
        row = row.child(stacked(seg, column, keep, ink));
    }
    let mut page = Column::new()
        .overflow_intended(true)
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .child(Text::line(title.to_owned()).style(ink.title.clone()))
        .child(SizedBox::fixed(1, 1))
        .child(row);
    if cut {
        page = page.child(Text::line(resize.to_owned()).style(ink.dim.clone()));
    }
    page.into()
}
