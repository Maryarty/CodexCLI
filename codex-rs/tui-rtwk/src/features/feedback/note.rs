//! The note step (oracle `bottom_pane/feedback_note_view.rs`): the title, the disclosure of
//! the audience, a text field behind the accent gutter and the keys. The disclosure scrolls
//! when the pane is short, so the field and the keys stay visible. In Flutter it is a
//! `Column` of a clipped `ListView`, a `TextField` and a `KeyHintsBar` on a `Material` surface;
//! the rows of the disclosure are measured here (`LayoutBuilder`, the oracle's `word_wrap_lines`
//! with the gutter as the indent) because how many there are decides how much room the field
//! gets.

use std::cell::Cell;
use std::rc::Rc;

use rtwk::LayoutBuilder;
use rtwk::editing::{TextArea, TextEditingController};
use rtwk::focus::{Focus, FocusNode, KeyPress};
use rtwk::layout::{BoxWidget, Column, CrossAxisAlignment, Expanded, MainAxisSize, Row, SizedBox};
use rtwk::overlay::Dialog;
use rtwk::prelude::{KeyCode, Mods};
use rtwk::style::{Attrs, Span};
use rtwk::text::{segmenter_of, Text};
use rtwk::{AnyWidget, Cx};

use codex_tui_core::features::feedback::{Audience, Destination};
use codex_tui_core::keymap::fixed;
use codex_tui_core::screens::dialogs::FeedbackCategory;

use super::sheet::{Msg, SheetState};
use super::strings::{self, Piece};
use crate::app::theme::Tokens;
use crate::keymap::bind_fixed;
use crate::widgets::measure;
use crate::widgets::page_keys::{Variant, footer_fit};

/// What the note step draws from.
pub(super) struct NoteView {
    pub category: FeedbackCategory,
    /// Who the report is for; none when uploads are disabled.
    pub audience: Option<Audience>,
    pub can_save: bool,
    pub text: TextEditingController,
    pub node: FocusNode,
    pub scroll: i32,
    /// What the last layout left for the sheet to read: the rows of a page of the disclosure,
    /// and the first row of the last page.
    pub page: Rc<Cell<i32>>,
    pub most: Rc<Cell<i32>>,
}

const GUTTER: &str = "\u{258c} ";

pub(super) fn build(view: NoteView, cx: &mut Cx<'_, SheetState>) -> AnyWidget {
    let tokens = Tokens::of(cx);
    let surface = tokens.prompt_surface(cx, false);
    let dim = tokens.dim();
    let bold = tokens.bold();
    let accent = tokens.accent();
    let (title, placeholder) = strings::title_and_placeholder(view.category);
    let url = match view.audience {
        Some(Audience::OpenAiEmployee) => strings::EMPLOYEE_FAQ_URL,
        _ => strings::POLICY_URL,
    };
    let link = accent.clone().attrs(Attrs::UNDERLINE).link(url);
    let send = view.audience.is_some();
    let on_submitted = if send {
        cx.map(|_| Msg::Submit(Destination::OpenAi))
    } else {
        cx.map(|_| Msg::Typed)
    };
    let on_changed = cx.map(|_| Msg::Typed);
    let paragraphs = strings::disclosure(view.audience, view.can_save);
    let NoteView {
        audience,
        can_save,
        text,
        node,
        scroll,
        page,
        most,
        ..
    } = view;
    let body = LayoutBuilder::new(move |cx, constraints| {
        let seg = segmenter_of(cx);
        let width = constraints.max_width.max(1);
        let inner = usize::try_from(width - 2).unwrap_or(1).max(1);
        let typed = text.value().text();
        let intro = |with_title: bool| -> Vec<Vec<Span>> {
            let mut rows: Vec<Vec<Span>> = Vec::new();
            let mut gutter_row = |spans: Vec<Span>| {
                let mut row = vec![Span::styled(GUTTER, accent.clone())];
                row.extend(spans);
                rows.push(row);
            };
            if with_title {
                gutter_row(vec![Span::styled(title, bold.clone())]);
                gutter_row(Vec::new());
            }
            for paragraph in &paragraphs {
                let whole: String = paragraph.iter().map(|(words, _)| *words).collect();
                let mut at = 0;
                for row in measure::rows(seg, &whole, inner) {
                    let start = whole[at..].find(&row).map_or(at, |found| at + found);
                    let end = start + row.len();
                    at = end;
                    let mut spans = Vec::new();
                    let mut from = 0;
                    for (words, piece) in paragraph {
                        let (lo, hi) = (from.max(start), (from + words.len()).min(end));
                        from += words.len();
                        if lo >= hi {
                            continue;
                        }
                        let style = match piece {
                            Piece::Dim => dim.clone(),
                            Piece::Bold => bold.clone(),
                            Piece::Link => link.clone(),
                        };
                        spans.push(Span::styled(whole[lo..hi].to_owned(), style));
                    }
                    gutter_row(spans);
                }
            }
            rows
        };
        let full = intro(true);
        let text_rows = measure::rows(seg, &typed, inner).len().clamp(1, 8);
        let input_wanted = (text_rows + 1).min(9);
        let room = usize::try_from(constraints.max_height).unwrap_or(usize::MAX);
        let height = (full.len() + input_wanted + 2).min(room);
        let footer = if height > 4 { 2 } else { 1 };
        let input_height = input_wanted
            .min(height.saturating_sub(footer + 1).max(2))
            .min(height);
        let intro_height = full.len().min(height.saturating_sub(input_height + footer));
        let lines = if height == 4 { intro(false) } else { full };
        let most_scroll = lines.len().saturating_sub(intro_height);
        page.set(i32::try_from(intro_height.max(1)).unwrap_or(1));
        most.set(i32::try_from(most_scroll).unwrap_or(0));
        let first = usize::try_from(scroll).unwrap_or(0).min(most_scroll);
        let size = |rows: usize| i32::try_from(rows).unwrap_or(i32::MAX);
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch);
        let mut shown = Column::new().cross_axis_alignment(CrossAxisAlignment::Stretch);
        for row in lines.into_iter().skip(first).take(intro_height) {
            shown = shown.child(Text::rich(row));
        }
        column = column.child(SizedBox::new(None, Some(size(intro_height)), Some(shown.into())));
        // The input: the gutter on every row, a blank first row, then the field.
        let mut bars = Column::new();
        for _ in 0..input_height {
            bars = bars.child(Text::line(GUTTER).style(accent.clone()));
        }
        let field = TextArea::new()
            .controller(text.clone())
            .placeholder(placeholder)
            .placeholder_style(dim.clone())
            .min_lines(1)
            .max_lines(input_height.saturating_sub(1).max(1))
            .focus_node(node.clone())
            .autofocus(true)
            .on_changed(on_changed.clone())
            .on_submitted(on_submitted.clone());
        let input = Row::new()
            .child(SizedBox::new(Some(2), None, Some(bars.into())))
            .child(Expanded::new(
                1,
                Column::new()
                    .cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .child(SizedBox::new(None, Some(1), None))
                    .child(Expanded::new(1, field)),
            ));
        column = column.child(SizedBox::new(None, Some(size(input_height)), Some(input.into())));
        if height > 4 {
            let scrolls = if most_scroll > 0 {
                let (key, words) = strings::SCROLL_HINT;
                Text::rich(vec![
                    Span::styled(key, dim.clone()),
                    Span::styled(format!(" {words}"), dim.clone()),
                ])
            } else {
                Text::new(String::new())
            };
            column = column.child(SizedBox::new(None, Some(1), Some(scrolls.into())));
        }
        let variant = |hints: &strings::Hints| {
            let mut entries = Vec::new();
            if can_save {
                entries.push(("\u{2303}s".to_owned(), hints.save.to_owned()));
            }
            if audience.is_some() {
                entries.push(("enter".to_owned(), hints.send.to_owned()));
            }
            entries.push(("esc".to_owned(), hints.cancel.to_owned()));
            Variant {
                min_width: 0,
                entries,
            }
        };
        column = column.child(footer_fit(
            vec![variant(&strings::HINTS_LONG), variant(&strings::HINTS_SHORT)],
            dim.clone(),
            dim.clone(),
            " \u{b7} ",
            0,
        ));
        column.into()
    });
    let panel = BoxWidget::new().style(surface).child(body);
    let mut focus = Focus::new(Dialog::bare(panel)).can_request_focus(false);
    focus = bind_fixed(focus, fixed::DISMISS, cx.send(Msg::Close));
    focus = focus
        .bind(KeyPress::code(KeyCode::PageUp), cx.send(Msg::Scroll(-1)))
        .bind(KeyPress::code(KeyCode::PageDown), cx.send(Msg::Scroll(1)));
    if can_save {
        focus = focus.bind(
            KeyPress::char('s').with_mods(Mods::CTRL),
            cx.send(Msg::Submit(Destination::LocalZip)),
        );
    }
    focus.into()
}
