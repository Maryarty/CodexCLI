//! The session heading scrolls with the transcript.

use codex_tui_core::screens::chat::StatusFacts;
use codex_tui_core::screens::status::Profile;
use rtwk::AnyWidget;
use rtwk::layout::{Column, CrossAxisAlignment, EdgeInsets, MainAxisSize, Padding};
use rtwk::style::{Attrs, Span};
use rtwk::text::TextOverflow;
use rtwk::widgets::Text;

use crate::app::theme::Tokens;
use crate::services::cli_help;

use super::strings::Strings;

pub(super) fn header(
    cwd: &str,
    facts: &StatusFacts,
    tokens: &Tokens,
    strings: &Strings,
) -> AnyWidget {
    let version = facts
        .version
        .as_deref()
        .filter(|version| *version != "0.0.0")
        .or_else(|| cli_help::VERSION.split_whitespace().last());
    let dim = tokens.dim().attrs(Attrs::DIM);
    let mut title = vec![
        Span::styled(strings.header_prompt, tokens.accent()),
        Span::styled(strings.header_title, tokens.bold()),
    ];
    if let Some(version) = version {
        title.push(Span::styled(
            strings.transcript_label(strings.header_version, version),
            dim.clone(),
        ));
    }
    let mut column = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(Padding::new(
            EdgeInsets::only(2, 1, 0, 0),
            Text::rich(title)
                .max_lines(1)
                .overflow(TextOverflow::Ellipsis),
        ))
        .child(Padding::new(
            EdgeInsets::only(5, 0, 0, 0),
            Text::line(super::status_surfaces::display_dir(
                cwd,
                facts.home_directory.as_deref(),
            ))
            .overflow(TextOverflow::EllipsisMiddle)
            .style(dim.clone()),
        ));
    if facts.profile == Profile::Full && facts.approval_policy.as_deref() == Some("never") {
        column = column.child(Padding::new(
            EdgeInsets::only(2, 0, 0, 0),
            Text::rich([
                Span::styled(strings.header_permissions, dim),
                Span::styled(strings.header_yolo, tokens.accent().attrs(Attrs::BOLD)),
            ])
            .max_lines(1)
            .overflow(TextOverflow::Ellipsis),
        ));
    }
    column.into()
}
