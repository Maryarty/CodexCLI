//! What the status-line and title setups preview: the live values of the items (the oracle's
//! `StatusSurfacePreviewData`, read from the chat as it stands) and the line the checked items
//! make, drawn as the footer draws it.

use rtwk::style::{Color, Span};
use rtwk::text::{Text, TextOverflow};
use rtwk::{AnyWidget, Cx};

use codex_tui_core::app::SessionSlice;
use codex_tui_core::features::settings::{ACTIVITY, Setup, SetupKind, USE_THEME_COLORS};

use super::strings::RATE_LIMIT_COPY;
use crate::app::theme::Tokens;
use crate::widgets::status_line::{status_spans, thread_color};

/// The setup with the live values of its items: what the offer names stays (a script's values),
/// the chat's live value fills the rest, and an item whose placeholder is left out reads empty.
pub(super) fn live<S: 'static>(cx: &mut Cx<'_, S>, mut setup: Setup) -> Setup {
    if setup.kind == SetupKind::Skills {
        return setup;
    }
    let ids: Vec<&str> = setup
        .items
        .iter()
        .map(|item| item.id.as_str())
        .filter(|id| *id != USE_THEME_COLORS && *id != ACTIVITY)
        .collect();
    let values = crate::pages::session::status::setup_values(
        cx,
        setup.kind == SetupKind::Title,
        &ids,
    );
    // the chat's value wins over the offer's (the offer's came from the config alone)
    setup.values.extend(values);
    for item in &mut setup.items {
        if !matches!(item.id.as_str(), "five-hour-limit" | "weekly-limit") {
            continue;
        }
        let Some(value) = setup.values.get(&item.id).map(|value| value.trim_start()) else {
            continue;
        };
        if let Some((_, name, description)) = RATE_LIMIT_COPY
            .iter()
            .find(|(prefix, ..)| value.starts_with(prefix))
        {
            item.label = (*name).to_owned();
            item.description = (*description).to_owned();
        }
    }
    setup
}

/// The colour of the active thread, which its name takes in the preview.
pub(super) fn thread<S: 'static>(cx: &mut Cx<'_, S>) -> Option<Color> {
    let session = rtwk::provider::read::<SessionSlice, _>(cx);
    thread_color(cx, session.active.as_ref().map(|id| id.as_str()))
}

/// The preview row under the list: the status line as the footer draws it (the accent of each
/// item with the colours on), the title as text; none while nothing is checked.
pub(super) fn row(
    setup: &Setup,
    tokens: &Tokens,
    thread: Option<Color>,
    indent: &str,
) -> Option<AnyWidget> {
    if setup.kind != SetupKind::StatusLine {
        let text = setup.preview();
        return (!text.is_empty()).then(|| Text::line(format!("{indent}{text}")).into());
    }
    let colors = setup
        .items
        .iter()
        .any(|item| item.id == USE_THEME_COLORS && item.enabled);
    let items = setup
        .items
        .iter()
        .filter(|item| item.enabled && item.id != USE_THEME_COLORS)
        .map(|item| {
            let value = setup.values.get(&item.id).unwrap_or(&item.placeholder);
            (item.id.as_str(), value.clone())
        })
        .filter(|(_, value)| !value.is_empty());
    let mut spans = status_spans(items, colors, tokens, thread, |_| None);
    if spans.is_empty() {
        return None;
    }
    spans.insert(0, Span::new(indent.to_string()));
    Some(
        Text::rich(spans)
            .max_lines(Some(1))
            .overflow(TextOverflow::Ellipsis)
            .into(),
    )
}
