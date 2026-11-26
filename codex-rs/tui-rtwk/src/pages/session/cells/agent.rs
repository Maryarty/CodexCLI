// cell: Item::Agent { .. }
//! The assistant's message cell (oracle history_cell/messages.rs
//! `AgentMessageCell`): the dim bullet and the Markdown of the message, one
//! column free at the right. The Markdown is `citation::view` (directives,
//! stream preview) in the transcript's theme.
//!
//! Flutter: `Row[SelectionContainer.disabled(Text("• ")), Expanded(MarkdownBody)]` with a right
//! padding: the bullet is a gutter, not part of what a selection copies.

use rtwk::AnyWidget;
use rtwk::gestures::ExcludeSelection;
use rtwk::layout::{Column, CrossAxisAlignment, EdgeInsets, Expanded, Padding, Row, SizedBox};
use rtwk::provider::Provider;
use rtwk::style::Style;
use rtwk::text::DefaultTextStyle;
use rtwk::widgets::{Builder, Text};
use rtwk_rich::markdown::MarkdownTheme;

use crate::pages::session::transcript::copy::Fill;

use super::citation;
use super::dispatch::{CellEnv, Place, below};
use super::strings::Strings;
use codex_tui_core::app::facts::{Rendering, TuiSettings};
use codex_tui_core::question_reply::QuestionText;
use codex_tui_core::threads::Item;

/// Split the server's visible Markdown at titles corroborated by the question metadata.
/// When the metadata diverges, keep the remaining visible text as the last section's body.
pub(crate) fn question_sections<'a>(
    source: &'a str,
    questions: &[QuestionText],
) -> Vec<(&'a str, &'a str)> {
    let mut rest = source;
    let mut titles = Vec::new();
    'questions: for question in questions {
        rest = rest.trim_start_matches('\n');
        let title = question.title.replace("\r\n", "\n");
        let title = title.trim_matches('\n');
        let Some(after) = rest.strip_prefix(title) else {
            break;
        };
        let start = source.len() - rest.len();
        titles.push(start..start + title.len());
        rest = after;
        for option in &question.options {
            let option = option.replace("\r\n", "\n");
            let Some(after) = rest
                .trim_start_matches('\n')
                .strip_prefix("- ")
                .and_then(|text| text.strip_prefix(&option))
            else {
                break 'questions;
            };
            rest = after;
        }
    }
    titles
        .iter()
        .enumerate()
        .map(|(at, title)| {
            let end = titles.get(at + 1).map_or(source.len(), |next| next.start);
            (
                &source[title.clone()],
                source[title.end..end].trim_matches('\n'),
            )
        })
        .collect()
}

/// What a message is drawn with.
pub struct Look<'a> {
    /// The mark in front of the first row (`• `) and its style.
    pub bullet: &'a str,
    pub bullet_style: Style,
    pub markdown: &'a MarkdownTheme,
    pub cells: &'static Strings,
}

/// The cell of an assistant message; `live` while it streams, `cwd` the
/// session's directory (file directives read against it).
pub fn message(source: &str, live: bool, cwd: Option<&str>, look: &Look<'_>) -> AnyWidget {
    message_filled(source, live, cwd, look, None)
}

/// `message` with the top-level block `fill` of its Markdown filled: every cell of the rows of
/// the copy mode's target block is reversed, the blank rows inside it and the gutter too (the
/// oracle's highlight).
///
/// Flutter: a `Column` of three `MarkdownBody`s (the blocks before, the block, the blocks after)
/// with the one blank row the blocks of a document have between them, the middle one in a
/// `DecoratedBox` of the full width. The text model does not change: the same nodes, the same
/// separators.
pub fn message_filled(
    source: &str,
    live: bool,
    cwd: Option<&str>,
    look: &Look<'_>,
    fill: Option<usize>,
) -> AnyWidget {
    match fill {
        Some(block) => citation::cited_filled(
            source,
            live,
            cwd,
            look.cells,
            look.markdown.clone(),
            citation::Filled {
                block,
                bullet: look.bullet.to_owned(),
                bullet_style: look.bullet_style.clone(),
            },
        ),
        None => plain(source, live, cwd, look),
    }
}

fn plain(source: &str, live: bool, cwd: Option<&str>, look: &Look<'_>) -> AnyWidget {
    let markdown = citation::cited(source, live, cwd, look.cells, look.markdown.clone());
    Row::new()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .spacing(1)
        // narrower than the bullet, the bullet is cut (the oracle draws the prefix alone)
        .overflow_intended(true)
        .child(ExcludeSelection::new(
            Text::new(look.bullet.to_owned()).style(look.bullet_style.clone()),
        ))
        // one column of the text stays free (oracle `right_reserve`)
        .child(Expanded::new(
            1,
            Padding::new(EdgeInsets::only(0, 0, 1, 0), markdown),
        ))
        .into()
}

/// An assistant message: it keeps one column free at the right.
/// The cell of `item` when it is this file's (`cell_list!`).
pub fn view(item: &Item, env: &CellEnv<'_>, place: &Place) -> AnyWidget {
    let Item::Agent {
        text, questions, ..
    } = item
    else {
        return AnyWidget::default();
    };
    let sections = question_sections(text, questions.as_deref().unwrap_or_default());
    if !sections.is_empty() {
        return below(place.separator, questions_view(sections, env, place.fill));
    }
    below(
        place.separator,
        message_filled(
            text,
            place.streaming,
            env.cwd,
            &Look {
                bullet: env.bullet,
                bullet_style: env.bullet_style.clone(),
                markdown: env.markdown,
                cells: env.cells,
            },
            match place.fill {
                Some(Fill::Block(block)) => Some(block),
                _ => None,
            },
        ),
    )
}

fn questions_view(sections: Vec<(&str, &str)>, env: &CellEnv<'_>, fill: Option<Fill>) -> AnyWidget {
    let sections = sections
        .into_iter()
        .map(|(heading, body)| (heading.to_owned(), body.to_owned()))
        .collect::<Vec<_>>();
    let fill = match fill {
        Some(Fill::Block(block)) => Some(block),
        _ => None,
    };
    let cwd = env.cwd.map(str::to_owned);
    let cells = env.cells;
    let accent = env.tokens.accent();
    let theme = env.markdown.clone();
    Builder::new(move |cx| {
        let rendering = Provider::<TuiSettings>::maybe_of(cx, true)
            .map_or_else(Rendering::default, |settings| settings.rendering);
        let mut remaining = fill;
        let mut segment = |source: &str, heading: bool| {
            let bullet = if heading { "?" } else { " " };
            let look = Look {
                bullet,
                bullet_style: accent.clone(),
                markdown: &theme,
                cells,
            };
            let local_fill = remaining.and_then(|offset| {
                let len =
                    citation::blocks_of(source, false, cwd.as_deref(), cells, rendering).len();
                remaining = offset.checked_sub(len);
                (offset < len).then_some(offset)
            });
            let markdown = if let Some(block) = local_fill {
                message_filled(source, false, cwd.as_deref(), &look, Some(block))
            } else if heading {
                message(source, false, cwd.as_deref(), &look)
            } else {
                Padding::new(
                    EdgeInsets::only(2, 0, 1, 0),
                    citation::cited(source, false, cwd.as_deref(), cells, theme.clone()),
                )
                .into()
            };
            if heading {
                DefaultTextStyle::new(accent.clone(), markdown).into()
            } else {
                markdown
            }
        };
        let mut column = Column::new().cross_axis_alignment(CrossAxisAlignment::Start);
        for (at, (heading, body)) in sections.iter().enumerate() {
            if at > 0 {
                column = column.child(SizedBox::new(None, Some(1), None));
            }
            column = column.child(segment(heading, true));
            if !body.is_empty() {
                column = column
                    .child(SizedBox::new(None, Some(1), None))
                    .child(segment(body, false));
            }
        }
        column.into()
    })
    .into()
}
