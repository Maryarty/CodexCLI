// cell: Item::User(_)
//! The user's message cell (oracle history_cell/messages.rs
//! `UserHistoryCell`): the prompt mark and the text on the prompt surface, a
//! label for each attached image above it, one blank row of the surface
//! above and below. Typed text is Markdown (links keep their destination);
//! text with inline elements (placeholders, mentions) is drawn as written
//! with the elements in the accent.
//!
//! Flutter: a `DecoratedBox` (the surface) over `Row[prompt mark,
//! Expanded(body)]`; the body is a `MarkdownBody`, or a `Text.rich` whose
//! spans carry the element style.

use codex_tui_core::threads::UserPrompt;
use rtwk::AnyWidget;
use rtwk::gestures::ExcludeSelection;
use rtwk::layout::{
    BoxWidget, Column, CrossAxisAlignment, EdgeInsets, Expanded, MainAxisSize, Padding, Row,
    SizedBox,
};
use rtwk::style::{Attrs, Span, Style};
use rtwk::text::DefaultTextStyle;
use rtwk::widgets::Text;
use rtwk_rich::markdown::{MarkdownTheme, MdBlock, MdInline, parse_markdown};

use crate::app::theme::Tokens;
use crate::pages::session::strings::Strings as Chat;

use crate::pages::session::transcript::copy::Fill;

use super::dispatch::{CellEnv, Place, below};
use super::strings::Strings;
use super::text::{Gap, Part, Texts};
use codex_tui_core::threads::Item;

/// What a prompt is drawn with: the surface, the colours, the tables.
pub struct Look<'a> {
    pub surface: Style,
    pub tokens: &'a Tokens,
    pub chat: &'a Chat,
    pub cells: &'a Strings,
    pub markdown: &'a MarkdownTheme,
    /// The marked prompt of a browsing sheet: its body reversed, the mark as it is
    /// (oracle `TranscriptView::set_highlight`).
    pub highlighted: bool,
}

/// The texts of the cell: a label for each attached image, then the prompt
/// mark (a gutter) and the body. A body the Markdown view lays out is the
/// gap: rtwk gives no text of a document that is not mounted.
pub fn texts(prompt: &UserPrompt, chat: &Chat, cells: &Strings) -> Texts {
    let shown = prompt.shown();
    let text = shown.text.trim_end();
    let mut parts: Vec<Part> = prompt
        .images
        .iter()
        .map(|number| Part::text(cells.label(cells.image_label, &number.to_string())))
        .collect();
    if !text.is_empty() {
        if prompt.elements.is_empty() && has_markup(text) {
            return Texts::Gap(Gap::Markdown);
        }
        // the blank row under the labels
        if !parts.is_empty() {
            parts.push(Part::gap());
        }
        parts.push(Part::gutter(chat.prompt.trim_end()));
        parts.push(Part::text(text));
    }
    Texts::Exact(parts)
}

/// The cell of `prompt`; `joins_previous` when it follows another prompt
/// with no turn between (no blank row above).
pub fn panel(prompt: &UserPrompt, look: &Look<'_>, joins_previous: bool) -> BoxWidget {
    let Look {
        surface,
        tokens,
        chat,
        cells,
        markdown,
        highlighted,
    } = look;
    let shown = prompt.shown();
    let text = shown.text.trim_end();
    let mut content = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch);
    // labels and inline elements are the accent that reads on the painted surface
    // (oracle `user_message_element_style`)
    let element = tokens.accent_on(surface);
    if !prompt.images.is_empty() {
        let labels = prompt.images.iter().map(|number| {
            Text::new(cells.label(cells.image_label, &number.to_string())).style(element.clone())
        });
        content = content.child(Padding::new(
            EdgeInsets::only(2, 0, 1, i32::from(!text.is_empty())),
            Column::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .children(labels.map(Into::into)),
        ));
    }
    if !text.is_empty() {
        let body: AnyWidget = if prompt.elements.is_empty() && has_markup(text) {
            crate::widgets::markdown::source(text, (*markdown).clone()).into()
        } else if shown.elements.is_empty() {
            Text::new(text.to_owned()).into()
        } else {
            let mut spans = Vec::with_capacity(shown.elements.len() * 2 + 1);
            let mut cursor = 0;
            for range in shown.elements.iter() {
                let Some(piece) = text.get(range.clone()) else {
                    break;
                };
                spans.push(Span::new(text[cursor..range.start].to_owned()));
                spans.push(Span::styled(piece.to_owned(), element.clone()));
                cursor = range.end;
            }
            spans.push(Span::new(text[cursor..].to_owned()));
            Text::rich(spans).into()
        };
        let body: AnyWidget = if *highlighted {
            DefaultTextStyle::new(Style::new().attrs(Attrs::REVERSE), body).into()
        } else {
            body
        };
        content = content.child(
            Row::new()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                // narrower than the mark, the mark is cut (oracle: the prefix is drawn whole)
                .overflow_intended(true)
                .spacing(1)
                .child(ExcludeSelection::new(
                    Text::line(chat.prompt.trim_end()).style(tokens.history_prompt_mark()),
                ))
                // one column of the text stays free (oracle: wrap width is width - 3)
                .child(Expanded::new(
                    1,
                    Padding::new(EdgeInsets::only(0, 0, 1, 0), body),
                )),
        );
    }
    BoxWidget::new()
        .style(surface.clone())
        .padding(EdgeInsets::only(0, i32::from(!joins_previous), 0, 1))
        .child(content)
}

/// Whether Markdown changes how `text` reads (links, emphasis, code, lists,
/// anything but plain paragraphs).
fn has_markup(text: &str) -> bool {
    parse_markdown(text)
        .iter()
        .any(|block| match block.as_ref() {
            MdBlock::Paragraph(inlines) => inlines.iter().any(|inline| {
                !matches!(
                    inline,
                    MdInline::Text(_) | MdInline::SoftBreak | MdInline::HardBreak
                )
            }),
            _ => true,
        })
}

/// A prompt with nothing to draw: no text (spaces and control characters aside) and no image.
/// The oracle's cell has no rows then (`transcript_hyperlink_lines`), and the cells around it
/// are laid out as if it were not there.
pub fn is_blank(prompt: &UserPrompt) -> bool {
    prompt.shown().text.trim().is_empty() && prompt.images.is_empty()
}

/// A prompt: the panel on the prompt surface; its blank row belongs to the transcript, outside the surface, and a prompt that joins the one above has none.
/// The cell of `item` when it is this file's (`cell_list!`).
pub fn view(item: &Item, env: &CellEnv<'_>, place: &Place) -> AnyWidget {
    let Item::User(prompt) = item else {
        return AnyWidget::default();
    };
    if is_blank(prompt) {
        return SizedBox::shrink().into();
    }
    let panel = panel(
        prompt,
        &Look {
            surface: env.prompt_surface.clone(),
            tokens: env.tokens,
            chat: env.chat,
            cells: env.cells,
            markdown: env.markdown,
            highlighted: place.highlighted,
        },
        place.joins_prompt,
    );
    // the prompt is the copy target as a whole: the oracle fills every cell of its rows, the
    // padding ones included
    let panel: AnyWidget = match place.fill {
        Some(Fill::Row) => crate::pages::session::transcript::copy::reversed(panel).into(),
        _ => panel.into(),
    };
    below(place.separator && !place.joins_prompt, panel)
}

/// A spoken prompt (a voice caption of the user; oracle `UserMessagePresentation::Spoken`): the
/// mark is the red bold of the voice controls and the text is plain, never Markdown. Its rows
/// carry the surface only under their glyphs (the oracle resolves the line style onto the spans),
/// and the blank rows above and below are unpainted. Speech transcripts can begin with
/// whitespace; the mark already supplies its separator.
///
/// Flutter: a `Column` of a blank row, a `Row[mark, Expanded(Text)]` and a blank row; the mark and
/// the text paint the surface themselves, not a `DecoratedBox` around the cell.
pub fn spoken(text: &str, env: &CellEnv<'_>, place: &Place) -> AnyWidget {
    let text = text.trim().to_owned();
    if text.is_empty() {
        return SizedBox::shrink().into();
    }
    let surface = env.prompt_surface.clone();
    let mark = match surface.bg {
        Some(bg) => crate::app::theme::voice::recording().bg(bg),
        None => crate::app::theme::voice::recording(),
    };
    let panel = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(SizedBox::new(None, Some(1), None))
        .child(
            Row::new()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .overflow_intended(true)
                .child(ExcludeSelection::new(SizedBox::new(
                    Some(2),
                    Some(1),
                    Some(
                        BoxWidget::new()
                            .style(mark.clone())
                            .child(Text::line(env.chat.prompt.trim_end()).style(mark))
                            .into(),
                    ),
                )))
                // one column of the text stays free (oracle: wrap width is width - 3)
                .child(Expanded::new(
                    1,
                    Padding::new(EdgeInsets::only(0, 0, 1, 0), Text::new(text).style(surface)),
                )),
        )
        .child(SizedBox::new(None, Some(1), None));
    below(place.separator, panel.into())
}
