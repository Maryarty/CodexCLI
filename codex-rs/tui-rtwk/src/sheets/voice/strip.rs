//! The listening dashboard above the composer (oracle bottom_pane/voice_strip.rs).

use codex_tui_core::keymap::actions::VOICE_TOGGLE_VOICE_MUTE;
use codex_tui_core::screens::{ScreenStates, voice::Display};
use codex_tui_core::threads::ThreadModel;
use rtwk::layout::{
    BoxWidget, Column, CrossAxisAlignment, EdgeInsets, Expanded, MainAxisSize, Row, SizedBox,
};
use rtwk::provider::select;
use rtwk::style::{Span, Style};
use rtwk::text::{TextOverflow, segmenter_of};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, LayoutBuilder, StatelessWidget, impl_widget};

use crate::app::theme::{Tokens, voice};

use super::strings::{self, Strings};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Strip;

impl_widget!(stateless eq Strip);

impl StatelessWidget for Strip {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let Some(display) =
            select::<ThreadModel, _, _>(cx, |model| model.screen_slots().voice.display())
        else {
            return SizedBox::shrink().into();
        };
        let animations =
            select::<ScreenStates, bool, _>(cx, |states| states.chat.status.animations);
        let key = crate::keymap::label(&crate::keymap::keymap(cx), VOICE_TOGGLE_VOICE_MUTE);
        let own = strings::of(cx);
        let tokens = Tokens::of(cx);
        LayoutBuilder::new(move |cx, constraints| {
            let seg = segmenter_of(cx);
            let available =
                usize::try_from(constraints.max_width.saturating_sub(2)).unwrap_or_default();
            let mut status = status_spans(display, own, &tokens);
            let mut controls = control_spans(display, own, &tokens, key.as_deref());
            let width = |spans: &[Span]| {
                spans
                    .iter()
                    .map(|span| seg.width(&span.text))
                    .sum::<usize>()
            };
            let fits =
                |status: &[Span], controls: &[Span]| width(status) + width(controls) < available;
            if !fits(&status, &controls) {
                status.pop();
            }
            if !fits(&status, &controls) {
                controls = vec![Span::styled(own.stop_command, tokens.dim())];
            }
            if !fits(&status, &controls) {
                status = vec![Span::styled(own.voice, tokens.dim())];
            }
            let row = Row::new()
                .child(Expanded::new(
                    1,
                    Text::rich_line(status).overflow(TextOverflow::Clip),
                ))
                .child(Text::rich_line(controls).overflow(TextOverflow::Clip));
            let mut pane = Column::new()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .main_axis_size(MainAxisSize::Min)
                .child(row);
            if animations {
                let width = available.saturating_sub(14).saturating_div(2).min(6);
                let mut spans = vec![Span::styled(own.microphone, tokens.dim())];
                meter(
                    &mut spans,
                    display.microphone,
                    width,
                    &voice::mic(),
                    display.muted,
                );
                spans.push(Span::styled(own.speaker, tokens.dim()));
                meter(&mut spans, display.speaker, width, &voice::speaker(), false);
                pane = pane.child(Text::rich_line(spans).overflow(TextOverflow::Clip));
            }
            BoxWidget::new()
                .padding(EdgeInsets::only(1, 1, 1, 0))
                .child(pane)
                .into()
        })
        .into()
    }
}

pub(crate) fn status_spans(display: Display, strings: &Strings, tokens: &Tokens) -> Vec<Span> {
    let (mark, style, activity) = if display.connecting {
        (
            strings.quiet_marker,
            voice::connecting(),
            strings.connecting,
        )
    } else if display.muted {
        (strings.quiet_marker, voice::quiet(), strings.muted)
    } else {
        (strings.active_marker, voice::recording(), strings.listening)
    };
    vec![
        Span::styled(strings.voice_prefix, tokens.dim()),
        Span::styled(mark, style),
        Span::styled(activity, tokens.dim()),
    ]
}

fn control_spans(
    display: Display,
    strings: &Strings,
    tokens: &Tokens,
    key: Option<&str>,
) -> Vec<Span> {
    if display.connecting {
        return vec![Span::styled(strings.stop_command, tokens.dim())];
    }
    let mut spans = Vec::new();
    if let Some(key) = key {
        let action = if display.muted {
            strings.unmute
        } else {
            strings.mute
        };
        spans.push(Span::styled(key.to_owned(), tokens.bold()));
        spans.push(Span::styled(action, tokens.dim()));
    } else {
        spans.push(Span::styled(strings.mute_command, tokens.dim()));
    }
    spans.push(Span::styled(strings.control_gap, tokens.dim()));
    spans.push(Span::styled(strings.stop_command, tokens.dim()));
    spans
}

fn meter(spans: &mut Vec<Span>, samples: [u8; 6], width: usize, active: &Style, muted: bool) {
    const BARS: [&str; 8] = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];
    for sample in &samples[samples.len() - width..] {
        let level = if muted {
            0
        } else {
            (usize::from(*sample) * 7).div_ceil(255)
        };
        let style = if level == 0 {
            voice::quiet()
        } else {
            active.clone()
        };
        spans.push(Span::styled(BARS[level], style));
    }
}
