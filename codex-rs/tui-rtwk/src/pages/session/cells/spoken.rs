//! A spoken answer of the assistant (oracle `AgentMarkdownCell::new_spoken`): the Markdown of
//! the message in the frame of an assistant message, and the words of it that read as paths of
//! the workspace (`src/lib.rs:42`) underlined and linked when the host finds the file there
//! (`spoken_artifacts.rs`). The oracle looks at the rendered lines, so the scan reads them too:
//! a word that is already a link stays what it is.
//!
//! Flutter: a `LayoutBuilder` (the wrap is the width's) over the lines of the Markdown, each a
//! `Text.rich`; the paths a `MouseRegion` would want are `TextSpan`s with a `recognizer`, made
//! once the file check `ref.watch` waits for has answered.

use std::sync::Arc;

use rtwk::layout::{Column, CrossAxisAlignment, MainAxisSize, SizedBox};
use rtwk::provider::Provider;
use rtwk::style::Span;
use rtwk::text::segmenter_of;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, LayoutBuilder};
use rtwk_rich::markdown::{
    Glyphs, LinkDestinations, MarkdownTheme, markdown_blocks_lines_with_destinations,
};

use codex_tui_core::app::facts::{Rendering, TuiSettings};
use codex_tui_core::data::links::{LinkDisplay, web_destination};

use super::agent::{Look, framed};
use super::citation;
use super::strings::Strings;
use crate::app::theme::Tokens;
use crate::widgets::measure::Measure;

/// The message `text` of a spoken answer, `cwd` the session's directory (what its paths and
/// file directives are read against).
pub(super) fn message(text: &str, cwd: Option<&str>, look: &Look<'_>) -> AnyWidget {
    let text = text.to_owned();
    let cwd = cwd.map(str::to_owned);
    let theme = look.markdown.clone();
    let cells = look.cells;
    let body = LayoutBuilder::new(move |cx, room| {
        let width = usize::try_from(room.max_width).unwrap_or(0);
        if width == 0 {
            return AnyWidget::default();
        }
        let rendering = Provider::<TuiSettings>::maybe_of(cx, true)
            .map_or_else(Rendering::default, |settings| settings.rendering);
        let destinations = match Tokens::link_display(cx) {
            LinkDisplay::WithDestination => LinkDestinations::Suffix,
            LinkDisplay::LabelOnly => LinkDestinations::Auto,
        };
        let lines = lines(
            segmenter_of(cx),
            &text,
            cwd.as_deref(),
            cells,
            rendering,
            &theme,
            width,
            destinations,
        );
        crate::features::link_spoken(cx, cwd.as_deref(), lines, |lines| {
            stacked(lines.into_iter().map(row).collect())
        })
    });
    framed(body.into(), look)
}

/// The Markdown of `text` as lines of `width`, the destinations of web links written as the
/// terminal needs; only a web address stays a hyperlink (oracle `web_destination`).
#[allow(clippy::too_many_arguments)]
fn lines(
    segmenter: Measure,
    text: &str,
    cwd: Option<&str>,
    cells: &'static Strings,
    rendering: Rendering,
    theme: &MarkdownTheme,
    width: usize,
    destinations: LinkDestinations,
) -> Vec<Vec<Span>> {
    let view = citation::view_with(text, false, cwd, cells, rendering);
    let blocks = view.blocks.clone().unwrap_or_default();
    markdown_blocks_lines_with_destinations(
        &segmenter,
        &blocks,
        &view.definitions,
        width,
        theme,
        &Glyphs::UNICODE,
        true,
        destinations,
    )
    .into_iter()
    .map(|line| {
        line.into_iter()
            .map(|rich| {
                let mut style = rich.style;
                if let Some(link) = style.link.as_deref() {
                    style.link = web_destination(link).map(Arc::from);
                }
                Span::styled(rich.text, style)
            })
            .collect()
    })
    .collect()
}

fn row(spans: Vec<Span>) -> AnyWidget {
    if spans.is_empty() {
        // a blank row between two blocks
        return SizedBox::new(None, Some(1), None).into();
    }
    Text::rich_line(spans).into()
}

fn stacked(rows: Vec<AnyWidget>) -> AnyWidget {
    let mut column = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch);
    for row in rows {
        column = column.child(row);
    }
    column.into()
}
