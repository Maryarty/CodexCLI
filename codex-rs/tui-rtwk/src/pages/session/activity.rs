//! The status row between the transcript and the composer: what the thread's
//! run shows (`StatusSlot`), and the running turn's native spinner and elapsed
//! time (`Activity`, mounted only while the turn runs).

use std::time::Duration;

use rtwk::animation::{AnimatedBuilder, AnimationController};
use rtwk::binding::AppBinding;
use rtwk::layout::{Column, CrossAxisAlignment, Expanded, MainAxisSize, Row, SizedBox};
use rtwk::provider::{select, watch};
use rtwk::style::{Attrs, Span};
use rtwk::text::{Text, TextOverflow, TextWrap, segmenter_of};
use rtwk::{
    AnyWidget, Cx, Key, LayoutBuilder, Rebuild, State, StatefulWidget, StatelessWidget, impl_widget,
};

use codex_tui_core::app::{ConnectionState, Session, SessionSlice};
use codex_tui_core::ids::ThreadId;
use codex_tui_core::keymap::actions::CHAT_INTERRUPT_TURN;
use codex_tui_core::screens::{ScreenStates, chat};
use codex_tui_core::threads::{Failure, ThreadModel, TurnState};

use crate::app::theme;
use crate::l10n::strings as l10n;

use super::actions::failure_text;
use super::status::{self, Pending, PendingInput, TerminalsLine};

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
    /// Background terminals still running (`ThreadModel::terminals`).
    terminals: usize,
    /// What the running hooks say (`ThreadModel::hook_status`).
    hook: Option<String>,
    /// The user's input that waits: steers, refused steers, the queue.
    pending: bool,
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
            terminals: thread.terminals().len(),
            hook: thread.hook_status(),
            pending: !Pending::of(thread).is_empty(),
        }
    }

    pub(super) fn running(&self) -> bool {
        matches!(self.turn, TurnState::Running(_))
    }

    /// The hook line, the background terminals' line or the pending preview
    /// shows above the composer.
    pub(super) fn has_activity_rows(&self) -> bool {
        self.pending || self.terminals > 0 || self.hook.is_some()
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

/// The status area above the composer (the oracle's bottom pane flex): the
/// connection lost, the session's or the thread's failure, the session
/// starting, or the running turn's `Activity`; else the hook line and the
/// background terminals' line; a blank row, then the pending preview, or a
/// blank row alone after the activity; nothing when none of them is there (and
/// a one-row gap above the status row once the transcript has rows).
#[derive(Clone, Debug, PartialEq)]
pub(super) struct StatusSlot;

impl_widget!(stateless eq StatusSlot);

impl StatelessWidget for StatusSlot {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let tokens = theme::Tokens::of(cx);
        let strings = l10n::of(cx);
        let session_strings = super::strings::of(cx);
        let status_strings = status::strings::of(cx);
        let session = watch::<SessionSlice, _>(cx);
        let run = select::<ThreadModel, Run, _>(cx, Run::of);
        if run.read_only {
            return SizedBox::shrink().into();
        }
        let owner = run.id.clone();
        let (backend, now, facts) = select::<ScreenStates, _, _>(cx, move |states| {
            (
                states.agents.owner_of(owner.as_str()),
                states.agents.now,
                states.chat.status.clone(),
            )
        });
        let keymap = crate::keymap::keymap(cx);
        let interrupt = run
            .can_interrupt
            .then(|| crate::keymap::label(&keymap, CHAT_INTERRUPT_TURN))
            .flatten();
        let reconnecting =
            session.connections.get(&backend) == Some(&ConnectionState::Reconnecting);
        let facts = &facts;
        let running = match &run.turn {
            TurnState::Running(running) => Some(running),
            TurnState::Idle => None,
        };
        let compacting = run.compacting();
        let mut working = false;
        let status: Option<AnyWidget> = if reconnecting {
            Some(
                Text::new(strings.chat.reconnecting)
                    .style(tokens.dim())
                    .into(),
            )
        } else if let Some(failure) = run.failure(&session) {
            Some(
                Text::new(failure_text(strings, failure))
                    .style(tokens.error())
                    .into(),
            )
        } else if session.status == Session::Starting {
            Some(Text::new(strings.chat.starting).style(tokens.dim()).into())
        } else if running.is_some() || compacting {
            working = true;
            let offset_seconds = running
                .and_then(|running| running.started_at)
                .filter(|started| *started > 0)
                .map(|started| u64::try_from(now.saturating_sub(started)).unwrap_or(0))
                .unwrap_or(0);
            Some(
                AnyWidget::from(Activity {
                    interrupt,
                    spinner: effects(facts).0,
                    shimmer: effects(facts).1,
                    header: run
                        .chat
                        .header()
                        .map(|header| session_strings.status_header(&header))
                        .unwrap_or_else(|| {
                            if compacting {
                                strings.chat.compacting
                            } else if run.chat.activity == chat::ActivityPhase::Thinking {
                                strings.chat.thinking
                            } else {
                                strings.chat.working
                            }
                            .to_string()
                        }),
                    details: run.chat.details().cloned().or_else(|| {
                        compacting.then(|| {
                            chat::StatusDetails::new(
                                status_strings.compaction_details.to_string(),
                                chat::STATUS_DETAILS_MAX_LINES,
                            )
                        })
                    }),
                    // the summary rides after the elapsed label while the row is there
                    inline: (run.terminals > 0).then(|| status_strings.terminals(run.terminals)),
                    hook: run.hook.clone(),
                    offset_seconds,
                })
                .with_key(Key::value((
                    run.id.clone(),
                    running.map(|running| running.turn.clone()),
                ))),
            )
        } else {
            None
        };
        // the hook line and the terminals' line stand in for a missing activity row
        let hook_line = run.hook.as_deref().filter(|_| !working);
        let terminals_line = run.terminals > 0 && !working;
        if status.is_none() && hook_line.is_none() && !terminals_line && !run.pending {
            return SizedBox::shrink().into();
        }
        let blank = || -> AnyWidget { SizedBox::new(None, Some(1), None).into() };
        // a window under four rows clips this column under the blank sheet
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .overflow_intended(true)
            .cross_axis_alignment(CrossAxisAlignment::Start);
        if let Some(status) = status {
            if !run.transcript_empty {
                column = column.child(blank());
            }
            column = column.child(status);
        }
        if let Some(message) = hook_line {
            column = column.child(HookLine {
                message: message.to_string(),
            });
        }
        if terminals_line {
            column = column.child(TerminalsLine);
        }
        let above = working || hook_line.is_some() || terminals_line;
        if above {
            column = column.child(blank());
        }
        if run.pending {
            column = column.child(PendingInput);
        }
        column.into()
    }
}

/// Hook activity with no turn row to ride on: `• ` and the message, dim,
/// clipped with an ellipsis (`bottom_pane/hook_status.rs`).
#[derive(Clone, Debug, PartialEq)]
struct HookLine {
    message: String,
}

impl_widget!(stateless eq HookLine);

impl StatelessWidget for HookLine {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let strings = status::strings::of(cx);
        let mut message = self.message.chars();
        let message: String = message
            .next()
            .map(|first| first.to_uppercase().chain(message).collect())
            .unwrap_or_default();
        Text::line(format!("{}{message}", strings.bullet))
            .overflow(TextOverflow::Ellipsis)
            .style(status::dim())
            .into()
    }
}

/// The progress bullet and the shimmer of the header: each on its own switch,
/// both under the master switch of the animations (`StatusIndicatorWidget`).
pub(super) fn effects(facts: &chat::StatusFacts) -> (bool, bool) {
    (
        facts.animations && facts.progress_effect,
        facts.animations && facts.shimmer_effect,
    )
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Activity {
    /// The key shown after the elapsed label (`esc to interrupt`), none while the turn cannot be interrupted.
    pub interrupt: Option<String>,
    pub spinner: bool,
    pub shimmer: bool,
    pub header: String,
    pub details: Option<chat::StatusDetails>,
    /// The summary after the elapsed label (`N background terminals running ...`).
    pub inline: Option<String>,
    /// What the running hooks say: after the label when it fits, else a row of its own.
    pub hook: Option<String>,
    pub offset_seconds: u64,
}

#[derive(Default)]
pub(super) struct ActivityState {
    elapsed_seconds: u64,
    started: Option<Duration>,
    initial_offset: u64,
    timer: Option<rtwk::binding::Timer>,
    /// The header's sweep (`summary_shimmer`: one second of wave every four), on a terminal whose colours are known.
    shimmer: Option<AnimationController>,
    /// The bullet's pulse (`shimmer_spans`, two seconds a round), on such a terminal.
    pulse: Option<AnimationController>,
    /// The bullet's blink on a terminal without them (`animated_activity_indicator`: 600 ms on, 600 ms off).
    blink: Option<AnimationController>,
}

impl ActivityState {
    /// A clock that runs round `period` over and over.
    fn spin(period: Duration, cx: &mut Cx<'_, Self>) -> AnimationController {
        let controller = AnimationController::new(cx).with_duration(period);
        AnimationController::repeat(&controller, false, None, cx);
        controller
    }

    /// The clocks the effects need: the header's sweep and the bullet's pulse
    /// where the terminal's colours are known, else the bullet's blink.
    fn configure_clocks(&mut self, w: &Activity, cx: &mut Cx<'_, Self>) {
        let colours = theme::terminal_colors(cx).is_some();
        if !(w.shimmer && colours) {
            self.shimmer.take().inspect(AnimationController::dispose);
        } else if self.shimmer.is_none() {
            self.shimmer = Some(Self::spin(Duration::from_secs(4), cx));
        }
        if !(w.spinner && colours) {
            self.pulse.take().inspect(AnimationController::dispose);
        } else if self.pulse.is_none() {
            self.pulse = Some(Self::spin(Duration::from_secs(2), cx));
        }
        if !(w.spinner && !colours) {
            self.blink.take().inspect(AnimationController::dispose);
        } else if self.blink.is_none() {
            self.blink = Some(Self::spin(Duration::from_millis(1200), cx));
        }
    }

    /// The header: plain; the sweep of `summary_shimmer` while the shimmer is
    /// on (each letter a blend of the terminal's colours, the band brightest),
    /// dim and still where the colours are unknown.
    fn header(&self, w: &Activity, cx: &mut Cx<'_, Self>) -> AnyWidget {
        if !w.shimmer {
            return Text::line(w.header.clone()).into();
        }
        let (Some(animation), Some((fg, bg))) = (&self.shimmer, theme::terminal_colors(cx)) else {
            return Text::line(w.header.clone()).style(status::dim()).into();
        };
        let seg = segmenter_of(cx);
        // each letter with the column its middle is at
        let mut letters: Vec<(String, f64)> = Vec::new();
        let mut column = 0.0;
        for letter in w.header.chars() {
            let width = seg.width(&letter.to_string()) as f64;
            if width == 0.0
                && let Some((last, _)) = letters.last_mut()
            {
                last.push(letter);
                continue;
            }
            letters.push((letter.to_string(), column + width / 2.0));
            column += width;
        }
        let extent = column;
        AnimatedBuilder::new(animation.clone(), move |animation, _| {
            const START_DELAY: f64 = 0.6;
            const SWEEP: f64 = 1.0;
            const INTERVAL: f64 = 4.0;
            let half = (extent * 0.1).max(3.0);
            let sweep = (animation.value() * INTERVAL - START_DELAY)
                .rem_euclid(INTERVAL)
                .min(SWEEP);
            let position = sweep / SWEEP * (extent + 2.0 * half) - half;
            let spans: Vec<_> = letters
                .iter()
                .map(|(letter, center)| {
                    let distance = ((center - position).abs() / half).min(1.0);
                    let intensity = 0.5 * (1.0 + (std::f64::consts::PI * distance).cos());
                    let alpha = 0.5 + 0.5 * intensity;
                    Span::styled(letter.clone(), theme::blended(fg, bg, alpha))
                })
                .collect();
            Text::rich_line(spans).into()
        })
        .into()
    }

    /// The activity bullet before the header: the pulse (bold, toward the
    /// background), or the blink; none while the progress effect is off.
    fn bullet(&self, w: &Activity, cx: &mut Cx<'_, Self>) -> Option<AnyWidget> {
        if !w.spinner {
            return None;
        }
        if let Some(pulse) = &self.pulse {
            let colours = theme::terminal_colors(cx)?;
            return Some(
                AnimatedBuilder::new(pulse.clone(), move |animation, _| {
                    // `shimmer_spans("•")`: the band's middle ten letters in, 21 steps a round
                    let position = (animation.value() * 21.0).floor();
                    let distance = (10.0 - position).abs();
                    let t = if distance <= 5.0 {
                        0.5 * (1.0 + (std::f64::consts::PI * distance / 5.0).cos())
                    } else {
                        0.0
                    };
                    let (fg, bg) = colours;
                    Text::line("\u{2022}")
                        .style(theme::blended(bg, fg, t * 0.9).attrs(Attrs::BOLD))
                        .into()
                })
                .into(),
            );
        }
        let blink = self.blink.as_ref()?;
        let dim = status::dim();
        Some(
            AnimatedBuilder::new(blink.clone(), move |animation, _| {
                if animation.value() < 0.5 {
                    Text::line("\u{2022}").into()
                } else {
                    Text::line("\u{25e6}").style(dim.clone()).into()
                }
            })
            .into(),
        )
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
        if old.shimmer != w.shimmer || old.spinner != w.spinner {
            self.configure_clocks(w, cx);
        }
    }

    fn did_change_dependencies(&mut self, w: &Activity, cx: &mut Cx<'_, Self>) {
        self.configure_clocks(w, cx);
    }

    fn dispose(&mut self) {
        if let Some(animation) = self.shimmer.take() {
            animation.dispose();
        }
        for controller in [self.blink.take(), self.pulse.take()].into_iter().flatten() {
            controller.dispose();
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
        let strings = status::strings::of(cx);
        let elapsed = compact_elapsed(
            self.elapsed_seconds
                .saturating_add(self.initial_offset)
                .max(w.offset_seconds),
        );
        let dim = status::dim();
        // `(0s • esc to interrupt)`, or `(0s)` while the turn cannot be interrupted
        let mut label = Vec::new();
        match &w.interrupt {
            Some(key) => {
                label.push(Span::styled(
                    format!("({elapsed}{}", strings.elapsed_sep),
                    dim.clone(),
                ));
                // the key in the terminal's own foreground, bold (`key_hint`)
                let key_style = match theme::terminal_colors(cx) {
                    Some((fg, _)) => theme::terminal_foreground(fg).attrs(Attrs::BOLD),
                    None => status::bold(),
                };
                label.push(Span::styled(key.clone(), key_style));
                label.push(Span::styled(strings.to_interrupt, dim.clone()));
            }
            None => label.push(Span::styled(
                format!("{}{elapsed}{}", strings.elapsed_open, strings.elapsed_close),
                dim.clone(),
            )),
        }
        if let Some(inline) = &w.inline {
            label.push(Span::styled(strings.inline_sep, dim.clone()));
            label.push(Span::styled(inline.clone(), dim.clone()));
        }
        let header = self.header(w, cx);
        let bullet = self.bullet(w, cx);
        let (spinner, header_text, hook, details) = (
            bullet.is_some(),
            w.header.clone(),
            w.hook.clone(),
            w.details.clone(),
        );
        LayoutBuilder::new(move |cx, constraints| {
            let seg = segmenter_of(cx);
            let width = usize::try_from(constraints.max_width).unwrap_or(0);
            let mut label = label.clone();
            // The hook text rides after the label while the whole row fits, else it
            // moves under it (`StatusIndicator::lines`).
            let mut hook_row = None;
            if let Some(hook) = &hook {
                let indicator = if spinner { 2 } else { 0 };
                let used = indicator
                    + seg.width(&header_text)
                    + 1
                    + label
                        .iter()
                        .map(|span| seg.width(&span.text))
                        .sum::<usize>();
                if used + seg.width(strings.inline_sep) + seg.width(hook) <= width {
                    label.push(Span::styled(strings.inline_sep, dim.clone()));
                    label.push(Span::styled(hook.clone(), dim.clone()));
                } else {
                    hook_row = Some(
                        Text::rich_line(vec![
                            Span::styled(strings.details_prefix, dim.clone()),
                            Span::styled(hook.clone(), dim.clone()),
                        ])
                        .overflow(TextOverflow::Ellipsis),
                    );
                }
            }
            let mut line = Row::new().spacing(1);
            if let Some(bullet) = &bullet {
                line = line.child(bullet.clone());
            }
            let line = line.child(header.clone()).child(Expanded::new(
                1,
                Text::rich_line(label).overflow(TextOverflow::Ellipsis),
            ));
            let mut body = Column::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .child(line);
            if let Some(row) = hook_row {
                body = body.child(row);
            }
            if let Some(details) = &details {
                body = body.child(details_rows(strings, details));
            }
            body.into()
        })
        .into()
    }
}

/// The lines under the header: `  └ ` before the first row, four columns of
/// indent under it, at most `max_lines` rows, the last ending in `…` when
/// more were cut (`wrapped_details_lines`).
fn details_rows(strings: &status::strings::Strings, details: &chat::StatusDetails) -> AnyWidget {
    let hang = i32::try_from(strings.item_hang.len()).unwrap_or(4);
    let mark = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .child(Text::line(strings.details_prefix.trim_end()).style(status::dim()));
    Row::new()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .child(SizedBox::new(Some(hang), None, Some(mark.into())))
        .child(Expanded::new(
            1,
            Text::new(details.text.clone())
                .wrap(TextWrap::Word)
                .max_lines(Some(details.max_lines))
                .overflow(TextOverflow::Ellipsis)
                .style(status::dim()),
        ))
        .into()
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
