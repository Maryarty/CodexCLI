//! The status row between the transcript and the composer: what the thread's
//! run shows (`StatusSlot`), and the running turn's native spinner and elapsed
//! time (`Activity`, mounted only while the turn runs).

use std::time::Duration;

use rtwk::animation::{AnimatedBuilder, AnimationController, Spinner};
use rtwk::binding::AppBinding;
use rtwk::layout::{Column, CrossAxisAlignment, Expanded, MainAxisSize, Row, SizedBox};
use rtwk::provider::{select, watch};
use rtwk::style::Span;
use rtwk::text::TextOverflow;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, Key, Rebuild, State, StatefulWidget, StatelessWidget, impl_widget};

use codex_tui_core::app::{ConnectionState, Session, SessionSlice};
use codex_tui_core::ids::ThreadId;
use codex_tui_core::screens::{ScreenStates, chat};
use codex_tui_core::threads::{Failure, ThreadModel, TurnState};

use crate::app::theme;
use crate::l10n::strings as l10n;

use super::actions::failure_text;

/// What a thread's run looks like to the status row and to the status line:
/// the part of the thread model they read (`select` keeps the widget off every
/// streamed delta).
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Run {
    pub id: ThreadId,
    turn: TurnState,
    pub can_interrupt: bool,
    pub read_only: bool,
    transcript_empty: bool,
    /// The thread's last failure, or the one the chat screen keeps.
    failure: Option<Failure>,
    chat: chat::ThreadPart,
}

impl Run {
    pub(super) fn of(thread: &ThreadModel) -> Run {
        let screen_error = thread.screen_slots().chat.error.clone().map(Failure::from);
        Run {
            id: thread.id().clone(),
            turn: thread.turn().clone(),
            can_interrupt: thread.can_interrupt(),
            read_only: thread.read_only(),
            transcript_empty: thread.transcript().is_empty(),
            failure: thread.last_error().cloned().or(screen_error),
            chat: thread.screen_slots().chat.clone(),
        }
    }

    pub(super) fn running(&self) -> bool {
        matches!(self.turn, TurnState::Running(_))
    }

    pub(super) fn compacting(&self) -> bool {
        self.chat.is_compacting()
    }

    /// The session's failure first, then the thread's last one.
    pub(super) fn failure<'a>(&'a self, session: &'a SessionSlice) -> Option<&'a Failure> {
        match &session.status {
            Session::Failed(failure) => Some(failure),
            _ => self.failure.as_ref(),
        }
    }

    /// The word of the status line's run-state item.
    pub(super) fn word(&self, session: &SessionSlice, reconnecting: bool) -> &'static str {
        if self.compacting() {
            "Compacting"
        } else if self.running() {
            match self.chat.activity {
                chat::ActivityPhase::Working => "Working",
                chat::ActivityPhase::Thinking => "Thinking",
            }
        } else if reconnecting {
            "Reconnecting"
        } else if self.failure(session).is_some() {
            "Error"
        } else {
            "Ready"
        }
    }
}

/// The status row: the connection lost, the session's or the thread's failure,
/// the session starting, or the running turn's `Activity`; nothing when idle
/// (and a one-row gap above it once the transcript has rows).
#[derive(Clone, Debug, PartialEq)]
pub(super) struct StatusSlot;

impl_widget!(stateless eq StatusSlot);

impl StatelessWidget for StatusSlot {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let tokens = theme::Tokens::of(cx);
        let strings = l10n::of(cx);
        let session = watch::<SessionSlice, _>(cx);
        let states = watch::<ScreenStates, _>(cx);
        let run = select::<ThreadModel, Run, _>(cx, Run::of);
        if run.read_only {
            return SizedBox::shrink().into();
        }
        let backend = states.agents.owner_of(run.id.as_str());
        let reconnecting =
            session.connections.get(&backend) == Some(&ConnectionState::Reconnecting);
        let facts = &states.chat.status;
        let running = match &run.turn {
            TurnState::Running(running) => Some(running),
            TurnState::Idle => None,
        };
        let compacting = run.compacting();
        let status: AnyWidget = if reconnecting {
            Text::new(strings.chat.reconnecting)
                .style(tokens.dim())
                .into()
        } else if let Some(failure) = run.failure(&session) {
            Text::new(failure_text(strings, failure))
                .style(tokens.error())
                .into()
        } else if session.status == Session::Starting {
            Text::new(strings.chat.starting).style(tokens.dim()).into()
        } else if running.is_some() || compacting {
            let offset_seconds = running
                .and_then(|running| running.started_at)
                .filter(|started| *started > 0)
                .map(|started| {
                    u64::try_from(states.agents.now.saturating_sub(started)).unwrap_or(0)
                })
                .unwrap_or(0);
            AnyWidget::from(Activity {
                show_interrupt: run.can_interrupt,
                spinner: facts.animations && facts.progress_effect,
                shimmer: facts.animations && facts.shimmer_effect,
                header: if compacting {
                    strings.chat.compacting
                } else if run.chat.activity == chat::ActivityPhase::Thinking {
                    strings.chat.thinking
                } else {
                    strings.chat.working
                },
                step: run.chat.step.clone(),
                offset_seconds,
            })
            .with_key(Key::value((
                run.id.clone(),
                running.map(|running| running.turn.clone()),
            )))
        } else {
            return SizedBox::shrink().into();
        };
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Start);
        if !run.transcript_empty {
            column = column.child(SizedBox::new(None, Some(1), None));
        }
        column.child(status).into()
    }
}

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
