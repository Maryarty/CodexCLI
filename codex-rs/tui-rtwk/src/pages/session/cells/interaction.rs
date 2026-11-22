//! A write to a background terminal (oracle history_cell/exec.rs
//! `UnifiedExecInteractionCell::transcript_hyperlink_lines`): the detailed
//! transcript's header `↳ Interacted with background terminal · <command>`
//! (a dim mark, the title bold, the command dim) and the input under a dim
//! `└` branch, every line of it kept; a wait with nothing sent is the one bold
//! row `• Waited for background terminal`.
//!
//! Flutter: a `Column` of a `Text.rich` and one `Row` of the branch and an
//! `Expanded` `Text` per input line.

use rtwk::AnyWidget;
use rtwk::layout::{Column, CrossAxisAlignment, MainAxisSize};
use rtwk::style::{Attrs, Span};
use rtwk::widgets::Text;

use crate::app::theme::Tokens;

use super::strings::Strings;
use super::text::Part;
use super::tool_rows::glyph_row;

/// The command, when there is one to name.
fn named(command: Option<&str>) -> Option<&str> {
    command.filter(|command| !command.is_empty())
}

/// The header row's text.
fn header(command: Option<&str>, waited: bool, strings: &Strings) -> String {
    let mut text = if waited {
        strings.interaction_waited.to_owned()
    } else {
        [strings.interaction_mark, strings.interaction_title].concat()
    };
    if let Some(command) = named(command) {
        text.push_str(strings.interaction_separator);
        text.push_str(command);
    }
    text
}

/// The glyph of the input row `index`.
fn glyph(index: usize, strings: &Strings) -> &'static str {
    if index == 0 {
        strings.interaction_first
    } else {
        strings.interaction_rest
    }
}

/// The texts of the cell: the header, then each input line after its
/// branch (a gutter).
pub fn texts(command: Option<&str>, stdin: &str, strings: &Strings) -> Vec<Part> {
    let mut parts = vec![Part::text(header(command, stdin.is_empty(), strings))];
    for (index, line) in stdin.lines().enumerate() {
        parts.push(Part::gutter(glyph(index, strings)));
        parts.push(Part::text(line));
    }
    parts
}

/// The cell.
pub fn cell(command: Option<&str>, stdin: &str, tokens: &Tokens, strings: &Strings) -> AnyWidget {
    let dim = tokens.dim().attrs(Attrs::DIM);
    let mut spans = if stdin.is_empty() {
        vec![Span::styled(strings.interaction_waited, tokens.bold())]
    } else {
        vec![
            Span::styled(strings.interaction_mark, dim.clone()),
            Span::styled(strings.interaction_title, tokens.bold()),
        ]
    };
    if let Some(command) = named(command) {
        spans.push(Span::styled(strings.interaction_separator, dim.clone()));
        spans.push(Span::styled(command.to_owned(), dim.clone()));
    }
    let mut column = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(Text::rich(spans));
    for (index, line) in stdin.lines().enumerate() {
        column = column.child(glyph_row(
            glyph(index, strings),
            Text::new(line.to_owned()).into(),
            dim.clone(),
        ));
    }
    column.into()
}
