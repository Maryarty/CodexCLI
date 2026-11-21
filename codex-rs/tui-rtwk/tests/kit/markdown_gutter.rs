//! The agent's answer in the chat against the oracle's Markdown renderer.
//!
//! The oracle's Markdown tests render a source at a width into bare lines. The
//! chat draws the same Markdown inside the agent's cell: a two-column gutter
//! (`• ` on the first row, blanks below) and one free column at the right
//! (messages.rs `right_reserve`). `Open::markdown(width, height)` opens the
//! chat at the terminal that gives the renderer exactly `width` columns and
//! reads the screen without the gutter, so every assertion and snapshot
//! speaks of the oracle's rows. Nothing else of the screen changes.

use super::grid::Grid;
use super::seed::Open;

/// The agent cell's bullet column (history_cell/messages.rs).
pub const GUTTER: u16 = 2;
/// The column the cell keeps free at its right (messages.rs `right_reserve`).
pub const RESERVE: u16 = 1;

impl Open {
    /// The oracle rendered Markdown at `width` columns; the terminal is as
    /// wide as the agent's cell needs for that, `height` rows high.
    pub fn markdown(mut self, width: u16, height: u16) -> Open {
        self.width = width + GUTTER + RESERVE;
        self.height = height;
        self.gutter = usize::from(GUTTER);
        self
    }
}

impl Grid {
    /// The grid without its `left` first columns.
    pub(super) fn without_left(&self, left: usize) -> Grid {
        let width = self.width.saturating_sub(left);
        let mut cells = Vec::with_capacity(width * self.height);
        for y in 0..self.height {
            for x in left..self.width {
                cells.push(self.cell(x, y).clone());
            }
        }
        Grid {
            width,
            height: self.height,
            cells,
            cursor: self
                .cursor
                .and_then(|(x, y)| x.checked_sub(left).map(|x| (x, y))),
        }
    }
}
