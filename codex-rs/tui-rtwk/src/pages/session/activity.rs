//! A running turn's native spinner and elapsed time, mounted only while the turn runs.

use std::time::Duration;

use rtwk::animation::{AnimatedBuilder, AnimationController, Spinner};
use rtwk::binding::AppBinding;
use rtwk::layout::{Column, CrossAxisAlignment, Expanded, MainAxisSize, Row};
use rtwk::style::Span;
use rtwk::text::TextOverflow;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use crate::app::theme;
use crate::l10n::strings as l10n;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Activity {
    pub show_interrupt: bool,
    pub spinner: bool,
    pub shimmer: bool,
    pub header: &'static str,
    pub step: Option<String>,
    pub offset_seconds: u64,
}

#[derive(Default)]
pub(super) struct ActivityState {
    elapsed_seconds: u64,
    started: Option<Duration>,
    initial_offset: u64,
    timer: Option<rtwk::binding::Timer>,
    shimmer: Option<AnimationController>,
}

impl ActivityState {
    fn configure_shimmer(&mut self, enabled: bool, cx: &mut Cx<'_, Self>) {
        if enabled && theme::Tokens::of(cx).shimmer_colors().is_some() {
            if self.shimmer.is_none() {
                let controller =
                    AnimationController::new(cx).with_duration(Duration::from_millis(1500));
                AnimationController::repeat(&controller, false, None, cx);
                self.shimmer = Some(controller);
            }
        } else if let Some(controller) = self.shimmer.take() {
            controller.dispose();
        }
    }

    fn header(&self, w: &Activity, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let tokens = theme::Tokens::of(cx);
        if let Some(animation) = &self.shimmer
            && let Some((base, highlight)) = tokens.shimmer_colors()
        {
            let letters: Vec<String> = w.header.chars().map(|letter| letter.to_string()).collect();
            let extent = letters.len() as f64;
            let style = tokens.accent();
            return AnimatedBuilder::new(animation.clone(), move |animation, _| {
                let center = (extent + 8.0) * animation.value() - 4.0;
                let spans: Vec<_> = letters
                    .iter()
                    .enumerate()
                    .map(|(index, letter)| {
                        let strength = (1.0 - (index as f64 - center).abs() / 4.0).max(0.0);
                        Span::styled(
                            letter.clone(),
                            theme::shimmer_style(&style, base, highlight, strength),
                        )
                    })
                    .collect();
                Text::rich(spans).into()
            })
            .into();
        }
        Text::line(w.header).style(tokens.accent()).into()
    }
}

impl StatefulWidget for Activity {
    type State = ActivityState;

    fn create_state(&self) -> ActivityState {
        ActivityState::default()
    }
}

impl State for ActivityState {
    type Widget = Activity;
    type Msg = ();

    fn init_state(&mut self, w: &Activity, cx: &mut Cx<'_, Self>) {
        self.initial_offset = w.offset_seconds;
        if let Some(binding) = AppBinding::maybe_of(cx) {
            self.started = binding.ticker().map(|ticker| ticker.now());
            self.timer = Some(binding.start_periodic(Duration::from_secs(1), cx.send(())));
        }
    }

    fn did_update_widget(&mut self, old: &Activity, w: &Activity, cx: &mut Cx<'_, Self>) {
        if old.shimmer != w.shimmer {
            self.configure_shimmer(w.shimmer, cx);
        }
    }

    fn did_change_dependencies(&mut self, w: &Activity, cx: &mut Cx<'_, Self>) {
        self.configure_shimmer(w.shimmer, cx);
    }

    fn dispose(&mut self) {
        if let Some(animation) = self.shimmer.take() {
            animation.dispose();
        }
        if let Some(timer) = self.timer.take() {
            timer.cancel();
        }
    }

    fn update(&mut self, (): (), _w: &Activity, cx: &mut Cx<'_, Self>) -> Rebuild {
        self.elapsed_seconds = self
            .started
            .zip(AppBinding::maybe_of(cx).and_then(|binding| binding.ticker()))
            .map_or(
                self.elapsed_seconds.saturating_add(1),
                |(started, ticker)| ticker.now().saturating_sub(started).as_secs(),
            );
        Rebuild::Yes
    }

    fn build(&self, w: &Activity, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let tokens = theme::Tokens::of(cx);
        let strings = l10n::of(cx);
        let elapsed = compact_elapsed(
            self.elapsed_seconds
                .saturating_add(self.initial_offset)
                .max(w.offset_seconds),
        );
        let label = if w.show_interrupt {
            strings.interrupt_elapsed(&elapsed).to_string()
        } else {
            format!("({elapsed})")
        };
        let mut header = Row::new().spacing(1);
        if w.spinner {
            header = header.child(Spinner::new().style(tokens.accent()));
        } else {
            header = header.child(Text::line("•").style(tokens.dim()));
        }
        let header = header.child(self.header(w, cx)).child(Expanded::new(
            1,
            Text::line(label)
                .overflow(TextOverflow::Ellipsis)
                .style(tokens.dim()),
        ));
        let mut body = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(header);
        if let Some(step) = &w.step {
            body = body.child(
                Text::new(step.clone())
                    .max_lines(Some(2))
                    .overflow(TextOverflow::Ellipsis)
                    .style(tokens.dim()),
            );
        }
        body.into()
    }
}

impl_widget!(stateful eq Activity);

// Oracle: status_indicator_widget.rs::fmt_elapsed_compact.
pub(super) fn compact_elapsed(seconds: u64) -> String {
    match seconds {
        0..=59 => format!("{seconds}s"),
        60..=3599 => format!("{}m {:02}s", seconds / 60, seconds % 60),
        _ => format!(
            "{}h {:02}m {:02}s",
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60
        ),
    }
}
