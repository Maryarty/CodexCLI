//! The 20-cell bar of a limit (oracle `render_limit_progress_bar`): `[`, whole
//! `█` cells for the share left, `░` for the rest, `]`. A `CustomPaint` with a
//! `CellPainter` (G40: no gauge draws whole cells).

use rtwk::AnyWidget;
use rtwk::layout::{CellPainter, CustomPaint};
use rtwk_core::canvas::Canvas;
use rtwk_core::style::PenId;

/// The cells of the bar between its brackets.
pub(super) const BAR_CELLS: i32 = 20;

/// The bar alone: `[`, whole `█` cells for the share left, `░` for the rest, `]`.
pub(super) fn bar(percent_left: i64) -> AnyWidget {
    CustomPaint::new()
        .painter(Bar {
            filled: filled_cells(percent_left),
        })
        .size(BAR_CELLS + 2, 1)
        .into()
}

pub(super) fn text(percent_left: i64) -> String {
    codex_tui_core::format::text_progress_bar(
        usize::try_from(filled_cells(percent_left)).unwrap_or_default(),
        usize::try_from(BAR_CELLS).unwrap_or_default(),
    )
}

fn filled_cells(percent_left: i64) -> i32 {
    ((percent_left.clamp(0, 100) * i64::from(BAR_CELLS)) as f64 / 100.0).round() as i32
}

#[derive(Debug)]
struct Bar {
    filled: i32,
}

impl CellPainter for Bar {
    fn paint(&self, canvas: &mut Canvas<'_>, width: i32, _height: i32) {
        let inner = (width - 2).max(0);
        let filled = self.filled.min(inner);
        canvas.fill_char(0, 0, 1, 1, PenId(0), '[');
        canvas.fill_char(1, 0, filled, 1, PenId(0), '█');
        canvas.fill_char(1 + filled, 0, inner - filled, 1, PenId(0), '░');
        canvas.fill_char(1 + inner, 0, 1, 1, PenId(0), ']');
    }

    fn should_repaint(&self, old: &dyn CellPainter) -> bool {
        old.downcast_ref::<Bar>()
            .is_none_or(|old| old.filled != self.filled)
    }
}
