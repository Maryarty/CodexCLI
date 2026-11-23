//! A `request_user_input` exchange that ended (oracle
//! history_cell/request_user_input.rs `RequestUserInputResultCell`): the
//! header `• Questions n/m answered` (and ` (interrupted)` in the accent),
//! each question after a plain `  • ` (` (unanswered)` dim at its end when it
//! has no answer), the answers under it (`answer: ` and `note: ` dim labels,
//! the text in the accent, a secret one as bullets), and for an interrupted
//! exchange `  ↳ interrupted with N unanswered`.
//!
//! Flutter: a `Column` of a `Text.rich` header and, per row, a `Row` of the
//! hanging prefix and an `Expanded` `Text`.

use rtwk::AnyWidget;
use rtwk::layout::{Column, CrossAxisAlignment, MainAxisSize};
use rtwk::style::{Attrs, Span, Style};
use rtwk::widgets::Text;

use codex_tui_core::screens::cells::AnsweredQuestion;

use crate::app::theme::Tokens;

use super::strings::Strings;
use super::text::Part;
use super::tool_rows::glyph_row;

fn answered(questions: &[AnsweredQuestion]) -> usize {
    questions
        .iter()
        .filter(|question| question.answered())
        .count()
}

/// One row under the header: its prefix (the wrapped lines keep the column
/// the prefix's width gives them, the width of the oracle's wrapped-line
/// prefix in every row), its text and the style of the text.
struct Row {
    first: &'static str,
    prefix_style: Style,
    text: String,
    /// The dim words after the text (` (unanswered)`).
    suffix: Option<&'static str>,
    text_style: Style,
}

fn header_text(questions: &[AnsweredQuestion], interrupted: bool, strings: &Strings) -> String {
    let mut text = [strings.bullet, strings.questions_title].concat();
    text.push_str(&strings.fill(
        strings.questions_count,
        &[
            &answered(questions).to_string(),
            &questions.len().to_string(),
        ],
    ));
    if interrupted {
        text.push_str(strings.questions_interrupted);
    }
    text
}

fn rows(
    questions: &[AnsweredQuestion],
    interrupted: bool,
    tokens: &Tokens,
    strings: &Strings,
) -> Vec<Row> {
    let dim = Style::new().attrs(Attrs::DIM);
    // the oracle's `accent_color()` (blue on the terminal's own background), not the theme's accent
    let accent = tokens.accent_on(&Style::new());
    let mut rows = Vec::new();
    for question in questions {
        rows.push(Row {
            first: strings.question_first,
            // oracle `wrap_with_prefix(.., "  • ".into(), ..)`: the question's bullet is plain
            prefix_style: Style::new(),
            text: question.question.clone(),
            suffix: (!question.answered()).then_some(strings.unanswered),
            text_style: Style::new(),
        });
        if !question.answered() {
            continue;
        }
        let answer = |text: String, note: bool| Row {
            first: if note {
                strings.note_first
            } else {
                strings.answer_first
            },
            prefix_style: dim.clone(),
            text,
            suffix: None,
            text_style: accent.clone(),
        };
        if question.secret {
            rows.push(answer(strings.secret_answer.to_owned(), false));
            continue;
        }
        let (options, note) = question.split();
        for option in options {
            rows.push(answer(option.to_owned(), false));
        }
        if let Some(note) = note {
            // a note of a question without options reads as the answer
            rows.push(answer(note.to_owned(), question.has_options));
        }
    }
    let unanswered = questions.len() - answered(questions);
    if interrupted && unanswered > 0 {
        rows.push(Row {
            first: strings.questions_summary_mark,
            prefix_style: accent.clone(),
            text: strings.fill(strings.questions_summary, &[&unanswered.to_string()]),
            suffix: None,
            text_style: accent,
        });
    }
    rows
}

/// The texts of the cell: the header, then each row's prefix (a gutter) and
/// its text.
pub fn texts(questions: &[AnsweredQuestion], interrupted: bool, strings: &Strings) -> Vec<Part> {
    let tokens = Tokens::new(std::rc::Rc::new(rtwk::prelude::ThemeData::DARK));
    let mut parts = vec![Part::text(header_text(questions, interrupted, strings))];
    for row in rows(questions, interrupted, &tokens, strings) {
        parts.push(Part::gutter(row.first));
        parts.push(Part::text(
            [row.text.as_str(), row.suffix.unwrap_or_default()].concat(),
        ));
    }
    parts
}

/// The cell.
pub fn cell(
    questions: &[AnsweredQuestion],
    interrupted: bool,
    tokens: &Tokens,
    strings: &Strings,
) -> AnyWidget {
    // The oracle's `.dim()` is the modifier alone (the terminal's own foreground, dimmed), and
    // only the mark of the bullet is dim: its blank is plain.
    let dim = Style::new().attrs(Attrs::DIM);
    let (mark, gap) = strings.bullet.split_at(strings.bullet.trim_end().len());
    let mut header = vec![
        Span::styled(mark, dim.clone()),
        Span::new(gap),
        Span::styled(strings.questions_title, tokens.bold()),
        Span::styled(
            strings.fill(
                strings.questions_count,
                &[
                    &answered(questions).to_string(),
                    &questions.len().to_string(),
                ],
            ),
            dim.clone(),
        ),
    ];
    if interrupted {
        header.push(Span::styled(
            strings.questions_interrupted,
            tokens.accent_on(&Style::new()),
        ));
    }
    let mut column = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(Text::rich(header));
    for row in rows(questions, interrupted, tokens, strings) {
        let mut spans = vec![Span::styled(row.text, row.text_style)];
        if let Some(suffix) = row.suffix {
            spans.push(Span::styled(suffix, dim.clone()));
        }
        column = column.child(glyph_row(
            row.first,
            Text::rich(spans).into(),
            row.prefix_style,
        ));
    }
    column.into()
}
