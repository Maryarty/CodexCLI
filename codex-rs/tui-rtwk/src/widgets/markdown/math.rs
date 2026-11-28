//! The text of a formula as the Markdown view draws it, for the readers that have no mounted
//! view: the copy text of a transcript item scrolled out of the list (`transcript::markdown_text`).
//!
//! rtwk-rich draws a `MdInline::Math` node through rtwk-math (`MarkdownView`, feature `math`)
//! and keeps the functions that choose the form to itself; this is the same choice written from
//! the same crate and the same options, so the model of the text equals what the mounted view
//! holds (`tests/pages/markdown_text.rs` mounts a corpus of formulas against it):
//!
//! - an inline formula (`$x$`, and `$$x$$` in a heading, a table cell, emphasis or a link) is
//!   one row of text, or its source with its delimiters where rtwk-math cannot draw it in one
//!   row;
//! - a display formula (`$$x$$` between the text of its paragraph) is a block of its own: the
//!   rows of the formula, or its one row, or `$$x$$`. The view picks the form from the width of
//!   the block; a reader that knows the width says it, one that does not (the text of an item
//!   that is not mounted: `G-new-markdown-text`) takes the widest, the rows at 256 columns at
//!   most, which a block of that width shows too.
//!
//! Flutter: the `MarkdownElementBuilder` of the math node that `flutter_markdown` would call for
//! the text of a node it has not laid out.

use rtwk_math::{MathBox, MathOptions, MathStyle};
use rtwk_rich::markdown::MdInline;

/// The columns a display formula may take (`MathOptions::display`).
const DISPLAY_COLUMNS: usize = 256;
/// The columns the one-row form may take (`MathOptions::inline`).
const FLAT_COLUMNS: usize = 1024;

/// A formula that flows in a paragraph: its one row, or `$source$` (`$$source$$` for a display
/// formula that has no block of its own) where it cannot be one row.
pub fn inline(source: &str, display: bool) -> String {
    let drawn = rtwk_math::render(source, &MathOptions::inline())
        .ok()
        .and_then(|drawn| one_row(&drawn));
    drawn.unwrap_or_else(|| delimited(source, display))
}

/// A display formula that has a block of its own, in a block `width` columns wide (none: as wide
/// as a formula may be): the rows of the formula (each as wide as the formula, a `Text` of the
/// view) where they fit, else its one row where that fits, else `$$source$$`.
pub fn display(source: &str, width: Option<usize>) -> String {
    let Ok(nodes) = rtwk_math::parse(source) else {
        return delimited(source, true);
    };
    let width = width.unwrap_or(usize::MAX);
    let options = |style: MathStyle, columns: usize| {
        MathOptions::display()
            .style(style)
            .max_columns(width.min(columns))
    };
    if let Ok(rows) = rtwk_math::layout(&nodes, &options(MathStyle::Display, DISPLAY_COLUMNS))
        && rows.width() > 0
        && rows.width() <= width
    {
        return rows.rows().join("\n");
    }
    let flat = options(MathStyle::Inline, FLAT_COLUMNS).max_rows(1);
    rtwk_math::layout(&nodes, &flat)
        .ok()
        .filter(|flat| flat.width() <= width)
        .and_then(|flat| one_row(&flat))
        .unwrap_or_else(|| delimited(source, true))
}

/// `source` with the dollars it was written between.
fn delimited(source: &str, display: bool) -> String {
    let dollars = if display { "$$" } else { "$" };
    format!("{dollars}{source}{dollars}")
}

/// The one row of `drawn`, trailing spaces cut; none for a box of no text or of more rows.
fn one_row(drawn: &MathBox) -> Option<String> {
    if drawn.rows().len() != 1 {
        return None;
    }
    let text = drawn.to_text();
    (!text.is_empty()).then_some(text)
}

/// A paragraph's piece: text of its own, or a display formula that has a block to itself.
pub enum Piece<'a> {
    Text(Vec<MdInline>),
    Display(&'a str),
}

/// Where the paragraph `inlines` is cut so that each display formula (`$$..$$`) between its text
/// has a block of its own, as the view cuts it (rtwk-rich `math::pieces`): the text before it,
/// the formula, the text after it, the line breaks and white space at a cut taken off the
/// text's edges. None where the paragraph has no such formula: it is not cut. (A display formula
/// in emphasis or a link is not cut out of it, nor one in a heading or a table cell: they are
/// one row where they stand.)
pub fn pieces(inlines: &[MdInline]) -> Option<Vec<Piece<'_>>> {
    if !inlines
        .iter()
        .any(|inline| matches!(inline, MdInline::Math { display: true, .. }))
    {
        return None;
    }
    let mut out = Vec::new();
    let mut from = 0;
    let text = |from: usize, to: usize, out: &mut Vec<Piece<'_>>| {
        let piece = trimmed(&inlines[from..to]);
        if !piece.is_empty() {
            out.push(Piece::Text(piece));
        }
    };
    for (at, inline) in inlines.iter().enumerate() {
        if let MdInline::Math {
            source,
            display: true,
        } = inline
        {
            text(from, at, &mut out);
            out.push(Piece::Display(source));
            from = at + 1;
        }
    }
    text(from, inlines.len(), &mut out);
    Some(out)
}

/// `piece` without the line breaks and the white space at its ends.
fn trimmed(piece: &[MdInline]) -> Vec<MdInline> {
    let mut out = piece.to_vec();
    while let Some(first) = out.first() {
        match first {
            MdInline::SoftBreak | MdInline::HardBreak => {
                out.remove(0);
            }
            MdInline::Text(text) if text.trim_start().is_empty() => {
                out.remove(0);
            }
            MdInline::Text(text) if text.len() != text.trim_start().len() => {
                out[0] = MdInline::text(text.trim_start());
                break;
            }
            _ => break,
        }
    }
    while let Some(last) = out.last() {
        match last {
            MdInline::SoftBreak | MdInline::HardBreak => {
                out.pop();
            }
            MdInline::Text(text) if text.trim_end().is_empty() => {
                out.pop();
            }
            MdInline::Text(text) if text.len() != text.trim_end().len() => {
                let at = out.len() - 1;
                out[at] = MdInline::text(text.trim_end());
                break;
            }
            _ => break,
        }
    }
    out
}
