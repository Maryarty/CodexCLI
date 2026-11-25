//! The pieces of a shortcut block (oracle `bottom_pane/shortcut_hud.rs`, `ShortcutHud`): a row
//! of a key, a separator and a label, and the rows down one or two columns. The block of
//! `/presets` (`sheets/select/quick/hud.rs`) and the guide of a held chord
//! (`pages/session/session/chord_guide.rs`) are built of them. In Flutter this is a `Row` of a
//! `Text.rich` and an `Expanded` label, and a `Row` of `Expanded` columns.

use rtwk::AnyWidget;
use rtwk::layout::{Column, CrossAxisAlignment, Expanded, MainAxisSize, Row, SizedBox};
use rtwk::style::{Span, Style};
use rtwk::text::TextWrap;
use rtwk::widgets::Text;

use super::dots::dots_end;

/// The narrowest a column of entries is (`MIN_COLUMN_WIDTH`).
const MIN_COLUMN_WIDTH: i32 = 32;
/// The gap between two columns (`COLUMN_GAP`).
const COLUMN_GAP: i32 = 2;
/// Up to this many entries stay in one column (`SINGLE_COLUMN_ENTRY_LIMIT`).
const SINGLE_COLUMN_ENTRIES: usize = 5;

/// `key` `separator` `label` as one row (`ShortcutHud::item`): the key in `accent`, the separator
/// in `dim`, the label cut with `…` when the room is short. The separator ends in a space that is
/// the label's gap: a word-wrapped text would let it hang, so the key is not wrapped.
pub fn item(
    key: String,
    label: String,
    separator: &'static str,
    accent: &Style,
    dim: &Style,
) -> AnyWidget {
    Row::new()
        .child(
            Text::rich(vec![
                Span::styled(key, accent.clone()),
                Span::styled(separator, dim.clone()),
            ])
            .wrap(TextWrap::None),
        )
        .child(Expanded::new(1, dots_end(label, Style::default())))
        .into()
}

/// How many columns `width` holds for `count` entries (`column_count`: one up to
/// `SINGLE_COLUMN_ENTRIES`, two above).
pub fn column_count(width: i32, count: usize) -> usize {
    let limit = if count > SINGLE_COLUMN_ENTRIES { 2 } else { 1 };
    let fits = (width + COLUMN_GAP) / (MIN_COLUMN_WIDTH + COLUMN_GAP);
    usize::try_from(fits).unwrap_or(1).clamp(1, limit)
}

/// The entries down `columns` columns of equal width (`grid_lines`).
pub fn grid(entries: &[AnyWidget], columns: usize) -> AnyWidget {
    let rows = entries.len().div_ceil(columns).max(1);
    let mut row = Row::new().cross_axis_alignment(CrossAxisAlignment::Start);
    for (index, chunk) in entries.chunks(rows).enumerate() {
        if index > 0 {
            row = row.child(SizedBox::new(Some(COLUMN_GAP), None, None));
        }
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch);
        for entry in chunk {
            column = column.child(entry.clone());
        }
        row = row.child(Expanded::new(1, column));
    }
    row.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_narrow_room_or_few_entries_keep_one_column() {
        assert_eq!(column_count(96, 6), 2);
        assert_eq!(column_count(96, 5), 1);
        assert_eq!(column_count(44, 6), 1);
        assert_eq!(column_count(96, 0), 1);
    }
}
