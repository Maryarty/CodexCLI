//! The session heading scrolls with the transcript.

use codex_tui_core::screens::chat::{SessionHeader, StatusFacts};
use codex_tui_core::screens::status::Profile;
use rtwk::AnyWidget;
use rtwk::layout::{Column, CrossAxisAlignment, EdgeInsets, MainAxisSize, Padding};
use rtwk::style::{Attrs, Span, Style};
use rtwk::text::{TextOverflow, segmenter_of};
use rtwk::widgets::Text;
use rtwk::LayoutBuilder;

use crate::app::theme::Tokens;
use crate::pages::agents::path::center_truncate_path;
use crate::services::cli_help;

use super::strings::Strings;

/// The chat is the startup draft (`startup_draft_renderable`): the session has not started, so
/// the heading stands over the composer with no transcript, no status row and no agents hint.
pub(super) fn startup_draft(session: &codex_tui_core::app::SessionSlice) -> bool {
    session.status == codex_tui_core::app::Session::Starting
}

/// The directory the startup heading names: the configured one once the host has read the config,
/// `loading` before (`startup_session_header`).
pub(super) fn startup_dir<'a>(thread_cwd: Option<&'a str>, facts: &'a StatusFacts) -> &'a str {
    thread_cwd
        .or(facts.cwd.as_deref())
        .filter(|cwd| !cwd.is_empty())
        .unwrap_or("loading")
}

pub(super) fn header(
    cwd: &str,
    facts: &StatusFacts,
    tokens: &Tokens,
    strings: &Strings,
) -> AnyWidget {
    let dim = tokens.dim().attrs(Attrs::DIM);
    let mut title = vec![
        Span::styled(strings.header_prompt, tokens.accent()),
        Span::styled(strings.header_title, tokens.bold()),
    ];
    if let Some(version) = version(facts) {
        title.push(Span::styled(
            strings.transcript_label(strings.header_version, version),
            dim.clone(),
        ));
    }
    let mut column = Column::new()
        .main_axis_size(MainAxisSize::Min)
        // the startup draft puts it where the first layout pass gives it one row
        .overflow_intended(true)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(Padding::new(
            EdgeInsets::only(2, 1, 0, 0),
            Text::rich(title)
                .max_lines(1)
                .overflow(TextOverflow::Ellipsis),
        ))
        .child(Padding::new(
            EdgeInsets::only(5, 0, 0, 0),
            directory(
                super::status_surfaces::display_dir(cwd, facts.home_directory.as_deref()),
                dim.clone(),
            ),
        ));
    if yolo(facts) {
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

/// The directory in the room the heading leaves it: cut by whole path segments (oracle
/// `format_directory`, `center_truncate_path`), measured at layout.
fn directory(text: String, style: Style) -> AnyWidget {
    LayoutBuilder::new(move |cx, constraints| {
        let room = usize::try_from(constraints.max_width).unwrap_or(0);
        Text::line(center_truncate_path(segmenter_of(cx), &text, room))
            .overflow(TextOverflow::Ellipsis)
            .style(style.clone())
            .into()
    })
    .into()
}

/// The transcript's raw session heading (oracle `SessionHeaderHistoryCell::raw_lines`).
pub(super) fn raw_header(
    session: &SessionHeader,
    facts: &StatusFacts,
    display_name: Option<&str>,
    strings: &Strings,
) -> AnyWidget {
    let version = version(facts)
        .map(|version| strings.transcript_label(strings.header_version, version))
        .unwrap_or_default();
    let mut column = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(Text::line([strings.header_title, &version].concat()));
    if let Some(model) = session.model.as_deref().filter(|model| !model.is_empty()) {
        let label = display_name
            .filter(|name| !name.is_empty())
            .unwrap_or(model);
        let mut line = [strings.header_model, label].concat();
        if let Some(effort) = &session.effort {
            line.push(' ');
            line.push_str(effort);
        }
        column = column.child(Text::line(line));
    }
    let cwd = super::status_surfaces::display_dir(&session.cwd, facts.home_directory.as_deref());
    column = column.child(Text::line([strings.header_directory, &cwd].concat()));
    if yolo(facts) {
        column = column.child(Text::line(
            [strings.header_permissions, strings.header_yolo].concat(),
        ));
    }
    column.into()
}

fn version(facts: &StatusFacts) -> Option<&str> {
    facts
        .version
        .as_deref()
        .filter(|version| *version != "0.0.0")
        .or_else(|| cli_help::VERSION.split_whitespace().last())
}

fn yolo(facts: &StatusFacts) -> bool {
    facts.profile == Profile::Full && facts.approval_policy.as_deref() == Some("never")
}
