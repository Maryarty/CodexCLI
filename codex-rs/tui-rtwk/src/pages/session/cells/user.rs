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
use rtwk::layout::{
    BoxWidget, Column, CrossAxisAlignment, EdgeInsets, Expanded, MainAxisSize, Padding, Row,
};
use rtwk::style::{Attrs, Span, Style};
use rtwk::text::DefaultTextStyle;
use rtwk::widgets::Text;
use rtwk_rich::markdown::{MarkdownTheme, MdBlock, MdInline, parse_markdown};

use crate::app::theme::Tokens;
use crate::pages::session::strings::Strings as Chat;

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
            crate::widgets::markdown::themed(
                crate::widgets::markdown::view(text),
                (*markdown).clone(),
            )
            .into()
        } else if shown.elements.is_empty() {
            Text::new(text.to_owned()).into()
        } else {
            let mut spans = Vec::with_capacity(shown.elements.len() * 2 + 1);
            let mut cursor = 0;
            for range in shown.elements {
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
                .child(Text::line(chat.prompt.trim_end()).style(tokens.history_prompt_mark()))
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

/// A prompt: the panel on the prompt surface; its blank row belongs to the transcript, outside the surface, and a prompt that joins the one above has none.
/// The cell of `item` when it is this file's (`cell_list!`).
pub fn view(item: &Item, env: &CellEnv<'_>, place: &Place) -> AnyWidget {
    let Item::User(prompt) = item else {
        return AnyWidget::default();
    };
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
    below(place.separator && !place.joins_prompt, panel.into())
}
