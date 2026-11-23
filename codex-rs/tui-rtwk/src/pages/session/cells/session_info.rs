//! The startup card under the session header (oracle history_cell/session.rs
//! `new_session_info`): the first session's help (the getting-started line
//! and five commands, the command plain and its words dim), else the tip
//! (`Tip:` in Markdown, two columns in) and, when the server used another
//! model, the `model changed:` block. The parts are one blank row apart; the
//! transcript's separator row (under the session header) stands where the oracle's
//! composite puts its blank after the header.
//!
//! Flutter: a `Column` of `Text`s, `Padding(left: 2)` around a
//! `MarkdownBody` for the tip.

use rtwk::AnyWidget;
use rtwk::layout::{Column, CrossAxisAlignment, EdgeInsets, MainAxisSize, Padding, SizedBox};
use rtwk::style::{Attrs, Span};
use rtwk::widgets::Text;
use rtwk_rich::markdown::MarkdownTheme;

use codex_tui_core::screens::cells::SessionInfo;

use crate::app::theme::Tokens;

use super::strings::Strings;
use super::text::Part;

/// The columns the tip is in (oracle `TooltipHistoryCell::indent`).
const TIP_INDENT: i32 = 2;

/// Whether the card holds Markdown (its tip), whose text rtwk gives no
/// function of the source (`Gap::Markdown`).
pub fn has_markdown(info: &SessionInfo) -> bool {
    info.tip.is_some() && !info.help
}

/// What the card shows: the help of the first session, else the tip and
/// the model note.
fn parts_of(info: &SessionInfo) -> (bool, Option<&str>, Option<&(String, String)>) {
    if info.help {
        (true, None, None)
    } else {
        (false, info.tip.as_deref(), info.model_changed.as_ref())
    }
}

/// The texts of the card, the tip's (Markdown) excepted.
pub fn texts(info: &SessionInfo, strings: &Strings) -> Vec<Part> {
    let (help, _, changed) = parts_of(info);
    let mut parts = Vec::new();
    if help {
        parts.push(Part::text(strings.help_intro));
        parts.push(Part::gap());
        parts.extend(
            strings
                .help_commands
                .iter()
                .map(|(command, words)| Part::text([strings.help_indent, command, words].concat())),
        );
    }
    if let Some((requested, used)) = changed {
        parts.push(Part::text(strings.model_changed));
        parts.push(Part::text(
            strings.fill(strings.model_requested, &[requested]),
        ));
        parts.push(Part::text(strings.fill(strings.model_used, &[used])));
    }
    parts
}

/// The card.
pub fn cell(
    info: &SessionInfo,
    markdown: MarkdownTheme,
    tokens: &Tokens,
    strings: &Strings,
) -> AnyWidget {
    let (help, tip, changed) = parts_of(info);
    let dim = tokens.dim().attrs(Attrs::DIM);
    let blank = || SizedBox::new(None, Some(1), None);
    let mut column = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch);
    let mut first = true;
    let mut part = |column: Column, widget: AnyWidget| {
        let column = if first { column } else { column.child(blank()) };
        first = false;
        column.child(widget)
    };
    if help {
        let mut lines = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(Text::new(strings.help_intro).style(dim.clone()))
            .child(blank());
        for (command, words) in strings.help_commands {
            lines = lines.child(Text::rich(vec![
                Span::new([strings.help_indent, command].concat()),
                Span::styled(words, dim.clone()),
            ]));
        }
        column = part(column, lines.into());
    }
    if let Some(tip) = tip {
        column = part(
            column,
            Padding::new(
                EdgeInsets::only(TIP_INDENT, 0, 0, 0),
                crate::widgets::markdown::themed(
                    crate::widgets::markdown::view(&strings.fill(strings.tip_label, &[tip])),
                    markdown,
                ),
            )
            .into(),
        );
    }
    if let Some((requested, used)) = changed {
        column = part(
            column,
            Column::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .child(Text::new(strings.model_changed).style(tokens.accent().attrs(Attrs::BOLD)))
                .child(Text::new(
                    strings.fill(strings.model_requested, &[requested]),
                ))
                .child(Text::new(strings.fill(strings.model_used, &[used])))
                .into(),
        );
    }
    column.into()
}
