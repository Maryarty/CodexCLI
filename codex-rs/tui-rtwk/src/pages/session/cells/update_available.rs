//! A newer release exists (oracle history_cell/notices.rs
//! `UpdateAvailableHistoryCell`): a rounded dim border around the title and
//! versions, the command that updates this install (or where to read how),
//! a blank row, and the release notes' address. The box is as wide as its
//! words, at most the width.
//!
//! Flutter: a `Container(decoration: BoxDecoration(border:
//! Border.all(borderRadius)))` over a `Column` of `Text`s.

use rtwk::AnyWidget;
use rtwk::layout::{
    Align, Alignment, Border, BorderType, BoxWidget, Column, CrossAxisAlignment, EdgeInsets,
    IntrinsicWidth, MainAxisSize, SizedBox,
};
use rtwk::style::{Attrs, Span};
use rtwk::widgets::Text;

use crate::app::theme::Tokens;

use super::strings::Strings;
use super::text::Part;

/// The instruction row: run the command, or see the install options.
fn instruction(command: Option<&str>, strings: &Strings) -> [String; 3] {
    match command {
        Some(command) => [
            strings.update_run.0.to_owned(),
            command.to_owned(),
            strings.update_run.1.to_owned(),
        ],
        None => [
            strings.update_see.0.to_owned(),
            strings.update_install_url.to_owned(),
            strings.update_see.1.to_owned(),
        ],
    }
}

/// The texts of the cell: the title row, the instruction, a blank row, the
/// notes' label and address (the border is no text).
pub fn texts(current: &str, latest: &str, command: Option<&str>, strings: &Strings) -> Vec<Part> {
    vec![
        Part::text(
            [
                strings.update_mark,
                strings.update_title,
                &strings.fill(strings.update_versions, &[current, latest]),
            ]
            .concat(),
        ),
        Part::text(instruction(command, strings).concat()),
        Part::gap(),
        Part::text(strings.update_notes),
        Part::text(strings.update_notes_url),
    ]
}

/// The cell.
pub fn cell(
    current: &str,
    latest: &str,
    command: Option<&str>,
    tokens: &Tokens,
    strings: &Strings,
) -> AnyWidget {
    let accent = tokens.accent();
    let address = |url: &str| {
        Span::styled(
            url.to_owned(),
            accent.clone().attrs(Attrs::UNDERLINE).link(url),
        )
    };
    let [head, middle, tail] = instruction(command, strings);
    let middle = if command.is_some() {
        Span::styled(middle, accent.clone())
    } else {
        address(&middle)
    };
    let title = accent.clone().attrs(Attrs::BOLD);
    let card = BoxWidget::new()
        .border(Border::all(tokens.dim().attrs(Attrs::DIM)).kind(BorderType::Rounded))
        .padding(EdgeInsets::only(1, 0, 1, 0))
        .child(
            Column::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .child(Text::rich(vec![
                    Span::styled(strings.update_mark, title.clone()),
                    Span::styled(strings.update_title, title),
                    Span::styled(
                        strings.fill(strings.update_versions, &[current, latest]),
                        tokens.bold(),
                    ),
                ]))
                .child(Text::rich(vec![Span::new(head), middle, Span::new(tail)]))
                .child(SizedBox::new(None, Some(1), None))
                .child(Text::new(strings.update_notes))
                .child(Text::rich(vec![address(strings.update_notes_url)])),
        );
    // as wide as its longest row, at most the width: Flutter's
    // `IntrinsicWidth` (oracle `with_border_with_inner_width`)
    Align::new(Alignment::TOP_LEFT, IntrinsicWidth::new(card))
        .width_factor(1)
        .height_factor(1)
        .into()
}
