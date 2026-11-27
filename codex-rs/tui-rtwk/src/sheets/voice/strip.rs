//! The listening dashboard above the composer (oracle bottom_pane/voice_strip.rs).

use codex_tui_core::keymap::actions::VOICE_TOGGLE_VOICE_MUTE;
use codex_tui_core::screens::{ScreenStates, voice::Display};
use codex_tui_core::threads::ThreadModel;
use rtwk::layout::{
    BoxWidget, Column, CrossAxisAlignment, EdgeInsets, Expanded, MainAxisSize, Row, SizedBox,
};
use rtwk::provider::select;
use rtwk::style::{Attrs, Color, Span, Style};
use rtwk::text::{TextOverflow, segmenter_of};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, LayoutBuilder, StatelessWidget, impl_widget};

use crate::app::theme::Tokens;

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
                    &display.microphone,
                    width,
                    Color::CYAN,
                    display.muted,
                );
                spans.push(Span::styled(own.speaker, tokens.dim()));
                meter(&mut spans, &display.speaker, width, Color::MAGENTA, false);
                pane = pane.child(Text::rich_line(spans).overflow(TextOverflow::Clip));
            }
            BoxWidget::new()
                .padding(EdgeInsets::only(0, 1, 0, 1))
                .child(pane)
                .into()
        })
        .into()
    }
}

pub(crate) fn status_spans(display: Display, strings: &Strings, tokens: &Tokens) -> Vec<Span> {
    let (mark, color, activity) = if display.connecting {
        ("◌", Color::CYAN, strings.connecting)
    } else if display.muted {
        ("◌", Color::BRIGHT_BLACK, strings.muted)
    } else {
        ("●", Color::RED, strings.listening)
    };
    vec![
        Span::styled(format!("{} ", strings.voice), tokens.dim()),
        Span::styled(mark, Style::new().fg(color).attrs(Attrs::BOLD)),
        Span::styled(format!(" {activity}"), tokens.dim()),
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
        spans.push(Span::styled(format!(" {action}   "), tokens.dim()));
    } else {
        spans.push(Span::styled(
            format!("{}   ", strings.mute_command),
            tokens.dim(),
        ));
    }
    spans.push(Span::styled(strings.stop_command, tokens.dim()));
    spans
}

fn meter(spans: &mut Vec<Span>, samples: &[u8; 6], width: usize, color: Color, muted: bool) {
    const BARS: [&str; 8] = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];
    for sample in &samples[samples.len() - width..] {
        let level = if muted {
            0
        } else {
            (usize::from(*sample) * 7).div_ceil(255)
        };
        let foreground = if level == 0 {
            Color::BRIGHT_BLACK
        } else {
            color
        };
        spans.push(Span::styled(BARS[level], Style::new().fg(foreground)));
    }
}
