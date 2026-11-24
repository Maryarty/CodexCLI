// cell: Item::Tool(_)
//! What the tool cells share: the parts a call contributes to its row (the
//! bullet, the shared disclosure and the expanded flag belong to
//! `transcript_activity.rs`'s `ToolRow`, Flutter's `ExpansionTile` state) and
//! the dim `└` rows (oracle `exec_cell/render.rs` `prefix_lines`, `mcp.rs`).

use rtwk::AnyWidget;
use rtwk::layout::{CrossAxisAlignment, Expanded, Row};
use rtwk::style::{Attrs, Span, Style};
use rtwk::text::TextOverflow;
use rtwk::widgets::Text;
use rtwk_rich::highlight::SyntaxTheme;

use crate::app::theme::Tokens;
use crate::pages::session::strings::{LINE_BREAK, MORE};
use crate::widgets::dots::dots_end;

use super::dispatch::{CellEnv, Place, below};
use super::strings::Strings;
use codex_tui_core::threads::Item;

/// What a cell draws with: the tables, the theme, and whether the rows are
/// the expanded ones (the transcript's detailed mode or a tap).
pub struct Env<'a> {
    pub strings: &'static Strings,
    pub tokens: &'a Tokens,
    pub syntax: &'a SyntaxTheme,
    pub detailed: bool,
    pub expanded: bool,
}

/// A call as the row draws it.
pub enum Parts {
    /// A cell with its own layout (no bullet of the row's), with the label
    /// of the control under it, if any.
    Plain(AnyWidget, Option<String>),
    Rows(Rows),
}

pub struct Rows {
    /// The bullet's style.
    pub mark: Style,
    pub header: AnyWidget,
    /// The preview rows, or the details when expanded.
    pub body: Vec<AnyWidget>,
    /// The control's label; `None` draws no control.
    pub disclosure: Option<String>,
}

/// The cells of the bullet and the blank after it (the row's gutter).
pub const BULLET_COLUMNS: i32 = 2;

/// A row under a header: `└ ` first, a blank after (oracle `"  └ "` and
/// `"    "`, two cells of them are the bullet's column).
pub fn branch_row(index: usize, body: AnyWidget, marker: Style) -> AnyWidget {
    glyph_row(if index == 0 { "└ " } else { "  " }, body, marker)
}

/// A row after `glyph` (`├ `, `└ ` or blanks).
pub fn glyph_row(glyph: &'static str, body: AnyWidget, marker: Style) -> AnyWidget {
    Row::new()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .child(Text::line(glyph).style(marker))
        .child(Expanded::new(1, body))
        .into()
}

/// One dim row of plain text, cut by `…` at the room's end (oracle
/// `clipped_line` over `detail.dim()`; the cut keeps the blanks before its
/// `…`, `dots_end`).
pub fn result_row(index: usize, text: impl Into<String>, tokens: &Tokens) -> AnyWidget {
    let dim = tokens.dim().attrs(Attrs::DIM);
    branch_row(index, dots_end(text.into(), dim.clone()), dim)
}

/// A wrapped dim row of plain text (the expanded forms).
pub fn detail_row(index: usize, text: impl Into<String>, tokens: &Tokens) -> AnyWidget {
    let dim = tokens.dim().attrs(Attrs::DIM);
    let body: AnyWidget = Text::rich(vec![Span::styled(text.into(), dim.clone())]).into();
    branch_row(index, body, dim)
}

/// The text of a row with its own SGR colours, every span dimmed (oracle
/// `dimmed_output_line`): `wrap` lets it take more rows, else one clipped.
pub fn ansi_row(index: usize, line: &str, tokens: &Tokens, wrap: bool, clip: usize) -> AnyWidget {
    let dim = tokens.dim().attrs(Attrs::DIM);
    let line = if !wrap && line.len() > clip {
        let prefix = &line[..line.floor_char_boundary(clip)];
        format!("{prefix}{MORE}")
    } else {
        line.to_owned()
    };
    let mut spans: Vec<Span> = Vec::new();
    for (index, parsed) in rtwk::ansi::parse_ansi(&line).into_iter().enumerate() {
        if index != 0 {
            spans.push(Span::new(LINE_BREAK));
        }
        spans.extend(parsed.into_iter().map(|mut span| {
            span.style.attrs |= Attrs::DIM;
            span
        }));
    }
    let body = Text::rich(spans);
    branch_row(
        index,
        if wrap {
            body.into()
        } else {
            body.max_lines(Some(1))
                .overflow(TextOverflow::Ellipsis)
                .into()
        },
        dim,
    )
}

/// The lines of a source text (oracle `raw_lines_from_source`): split at
/// `\n`, a final newline does not start another line.
pub fn source_lines(text: &str) -> Vec<&str> {
    if text.is_empty() {
        return Vec::new();
    }
    let mut parts: Vec<&str> = text.split('\n').collect();
    if text.ends_with('\n') {
        parts.pop();
    }
    parts
}

/// A tool call: the rows of the run it starts (an exploration, a computer call row shares its run).
/// The cell of `item` when it is this file's (`cell_list!`).
pub fn view(item: &Item, env: &CellEnv<'_>, place: &Place) -> AnyWidget {
    let Item::Tool(tool) = item else {
        return AnyWidget::default();
    };
    below(
        place.separator,
        crate::pages::session::transcript_activity::ToolRow {
            first: tool.clone(),
            rest: place.rest.clone(),
            detailed: env.detailed,
        }
        .into(),
    )
}
