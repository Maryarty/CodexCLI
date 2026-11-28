// cell: Item::WebSocketTiming(_)
//! One Responses WebSocket timing sample as it arrives during the turn.

use codex_tui_core::screens::cells::RuntimeMetrics;
use codex_tui_core::threads::Item;
use rtwk::AnyWidget;
use rtwk::style::{Color, Span, Style};
use rtwk::widgets::Text;

use super::dispatch::{CellEnv, Place, below};
use super::separator;
use super::strings::Strings;

pub(super) fn line(metrics: &RuntimeMetrics, strings: &Strings) -> Option<String> {
    separator::metrics_label(metrics, strings)
        .map(|label| strings.label(strings.websocket_timing, &label))
}

pub fn view(item: &Item, env: &CellEnv<'_>, place: &Place) -> AnyWidget {
    let Item::WebSocketTiming(metrics) = item else {
        return AnyWidget::default();
    };
    let Some(line) = line(metrics, env.cells) else {
        return AnyWidget::default();
    };
    below(
        place.separator,
        Text::rich(vec![
            Span::styled(env.cells.bullet, env.tokens.dim()),
            Span::styled(line, Style::new().fg(Color::BRIGHT_BLACK)),
        ])
        .into(),
    )
}
