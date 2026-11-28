//! The reasoning-effort ignition (oracle `bottom_pane/effort_ignition.rs`,
//! `effort_ignition_styles.rs`, `effort_status_line.rs`): a change of the
//! effort to Max or Ultra plays a one-shot animation over the status line (the
//! old line scrolls out, the tier's letters assemble, hold and fade, the new
//! line fades in) and over the composer (a band of colour sweeps its rows), and
//! the prompt keeps the tier's glyph and accent while the tier lasts.
//!
//! Flutter writes each as an `AnimationController` its `State` starts when the
//! value it watches changes, an `AnimatedBuilder` over it, and the phases as
//! `Interval`s of one run. So here: `Watch` is the part of a `State` that
//! decides, from what the thread says its effort is, whether a change starts a
//! run (a baseline - the first value seen, or the thread's own start - never
//! does); `StatusTransition`, `Aura` and `Prompt` are the three widgets with a
//! clock of their own; the frames are pure functions of the elapsed time, so
//! the oracle's snapshots check them without a screen.

use std::cell::{Cell, RefCell};
use std::time::Duration;

use rtwk::animation::{AnimatedBuilder, AnimationController, AnimationStatus};
use rtwk::layout::{CellPainter, CustomPaint, SizedBox};
use rtwk::layout::{EdgeInsets, Padding};
use rtwk::provider::select;
use rtwk::style::{Attrs, Span, Style};
use rtwk::text::{TextOverflow, segmenter_of};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, Handler, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};
use rtwk_core::canvas::Canvas;
use rtwk_core::style::ColorMode;

use codex_tui_core::screens::ScreenStates;
use codex_tui_core::threads::ThreadModel;

use crate::app::theme::effort::{
    Colors, Grade, band_style, blend, channels, glyph_style, is_light, sample_model_style,
    spark_style,
};
use crate::app::theme::{Rgb, Tokens};
use crate::pages::session::actions::prompt;

use super::effort_frames::{Ink, TRANSITION, transition_spans};
use super::strings::{self, sample};

// ---- what the thread says

/// What the thread says of its effort: the effective effort name, whether the
/// thread has told its own settings yet, and whether motion is on.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Signal {
    pub effort: Option<String>,
    pub known: bool,
    pub animations: bool,
}

/// The effort the status line and the prompt show: the thread's, else the
/// config's (`effective_reasoning_effort`).
pub(crate) fn signal<S>(cx: &mut Cx<'_, S>) -> Signal {
    let (effort, known) = select::<ThreadModel, _, _>(cx, |thread| {
        let part = &thread.screen_slots().chat;
        (
            part.thread_reasoning()
                .map(|effort| effort.map(str::to_owned)),
            part.cwd.is_some(),
        )
    });
    let (configured, animations) =
        select::<ScreenStates, (Option<String>, bool), _>(cx, |states| {
            let facts = &states.chat.status;
            (facts.effort.clone(), facts.animations)
        });
    Signal {
        effort: effort.unwrap_or(configured),
        known,
        animations,
    }
}

/// What a `Watch` makes of the signal it is given.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Seen {
    /// The tier did not change.
    Same,
    /// A baseline, a lowering, or motion off: nothing plays, what played stops
    /// (`set_active_reasoning_effort_baseline`, a change to a tier-less effort).
    Rest,
    /// A genuine change to a tier: play it (`set_active_reasoning_effort`).
    Start(Grade),
}

/// The decision of `set_active_reasoning_effort`: kept in a `State`, asked at
/// each build. The first value seen is the baseline, and so is the first one
/// after the thread told its settings (a restored thread plays nothing).
#[derive(Default)]
pub(crate) struct Watch {
    seen: Cell<bool>,
    known: Cell<bool>,
    tier: Cell<Option<Grade>>,
}

impl Watch {
    pub(crate) fn observe(&self, signal: &Signal) -> Seen {
        let tier = Grade::from_effort(signal.effort.as_deref());
        let baseline = !self.seen.get() || (signal.known && !self.known.get());
        self.seen.set(true);
        self.known.set(signal.known);
        let before = self.tier.replace(tier);
        if baseline {
            return Seen::Rest;
        }
        if before == tier {
            return Seen::Same;
        }
        match tier {
            Some(tier) if signal.animations => Seen::Start(tier),
            _ => Seen::Rest,
        }
    }
}

/// The status line while a transition plays: one row (`status_surface_height`
/// caps the line at one row), starting its clock when it is first built (the
/// oracle starts it at the first rendered frame), and telling `on_done` after
/// its last frame so the line wraps again.
#[derive(Clone, PartialEq)]
pub struct StatusTransition {
    pub tier: Grade,
    pub previous: Vec<Span>,
    pub current: Vec<Span>,
    pub on_done: Option<Handler<()>>,
}

impl_widget!(stateful eq StatusTransition);

#[derive(Default)]
pub struct StatusTransitionState {
    clock: Option<AnimationController>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum TransitionMsg {
    Done,
}

impl StatefulWidget for StatusTransition {
    type State = StatusTransitionState;

    fn create_state(&self) -> StatusTransitionState {
        StatusTransitionState::default()
    }
}

impl State for StatusTransitionState {
    type Widget = StatusTransition;
    type Msg = TransitionMsg;

    fn init_state(&mut self, _w: &StatusTransition, cx: &mut Cx<'_, Self>) {
        let clock = AnimationController::new(cx).with_duration(TRANSITION);
        clock.add_status_listener(cx.map(|status: AnimationStatus| {
            let _ = status;
            TransitionMsg::Done
        }));
        clock.forward(Some(0.0), cx);
        self.clock = Some(clock);
    }

    fn update(
        &mut self,
        TransitionMsg::Done: TransitionMsg,
        w: &StatusTransition,
        cx: &mut Cx<'_, Self>,
    ) -> Rebuild {
        if self
            .clock
            .as_ref()
            .is_some_and(AnimationController::is_completed)
            && let Some(done) = &w.on_done
        {
            cx.emit(done, ());
        }
        Rebuild::No
    }

    fn dispose(&mut self) {
        if let Some(clock) = self.clock.take() {
            clock.dispose();
        }
    }

    fn build(&self, w: &StatusTransition, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let Some(clock) = self.clock.clone() else {
            return AnyWidget::default();
        };
        let own = strings::of(cx);
        let colors = Colors::of(cx);
        let (tier, previous, current) = (w.tier, w.previous.clone(), w.current.clone());
        AnimatedBuilder::new(clock, move |clock, _| {
            frame_row(
                own,
                colors,
                tier,
                previous.clone(),
                current.clone(),
                TRANSITION.mul_f64(clock.value()),
            )
        })
        .into()
    }
}

/// The status line `elapsed` into the run (`transition_line_at`): one row, as wide as the room.
fn frame_row(
    own: &'static strings::Strings,
    colors: Colors,
    tier: Grade,
    previous: Vec<Span>,
    current: Vec<Span>,
    elapsed: Duration,
) -> AnyWidget {
    LayoutBuilder::new(move |cx, constraints| {
        let seg = segmenter_of(cx);
        let width = usize::try_from(constraints.max_width).unwrap_or(0);
        let spans = transition_spans(
            &Ink {
                grade: tier,
                label: match tier {
                    Grade::Max => own.effort_max,
                    Grade::Ultra => own.effort_ultra,
                },
                colors,
                measure: &|text: &str| seg.width(text),
            },
            &previous,
            &current,
            elapsed,
            width,
        );
        if spans.is_empty() {
            return SizedBox::shrink().into();
        }
        Text::rich(spans)
            .max_lines(Some(1))
            .overflow(TextOverflow::Clip)
            .into()
    })
    .into()
}

/// The spans of one status line.
type Strip = Vec<Span>;

/// The status line's leaf: the configured items wrapped over `rows`, or the
/// transition of a genuine change to Max or Ultra (one row, whatever `rows`
/// says: the oracle's `status_surface_height` caps the line while it plays, and
/// it wraps again after the last frame).
#[derive(Clone, PartialEq)]
pub struct StatusLine {
    pub spans: Vec<Span>,
    pub rows: usize,
}

impl_widget!(stateful eq StatusLine);

#[derive(Default)]
pub struct StatusLineState {
    watch: Watch,
    last: RefCell<Strip>,
    play: RefCell<Option<(Grade, Strip)>>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum LineMsg {
    Done,
}

impl StatefulWidget for StatusLine {
    type State = StatusLineState;

    fn create_state(&self) -> StatusLineState {
        StatusLineState::default()
    }
}

impl State for StatusLineState {
    type Widget = StatusLine;
    type Msg = LineMsg;

    fn update(
        &mut self,
        LineMsg::Done: LineMsg,
        _w: &StatusLine,
        _cx: &mut Cx<'_, Self>,
    ) -> Rebuild {
        *self.play.borrow_mut() = None;
        Rebuild::Yes
    }

    fn build(&self, w: &StatusLine, cx: &mut Cx<'_, Self>) -> AnyWidget {
        match self.watch.observe(&signal(cx)) {
            Seen::Same => {}
            Seen::Rest => *self.play.borrow_mut() = None,
            Seen::Start(tier) => {
                *self.play.borrow_mut() = Some((tier, self.last.borrow().clone()));
            }
        }
        *self.last.borrow_mut() = w.spans.clone();
        match self.play.borrow().as_ref() {
            Some((tier, previous)) => StatusTransition {
                tier: *tier,
                previous: previous.clone(),
                current: w.spans.clone(),
                on_done: Some(cx.map(|(): ()| LineMsg::Done)),
            }
            .into(),
            None => Text::rich(w.spans.clone())
                .max_lines(Some(w.rows))
                .overflow(TextOverflow::Ellipsis)
                .into(),
        }
    }
}

// ---- the ignition over the composer

/// A run of the ignition: the tier and the look it plays.
type Flight = (Grade, Look);

const WAVE_HALF_WIDTH: f32 = 9.0;
const PULSE_HALF_WIDTH: f32 = 4.5;
const SPARK_START: Duration = Duration::from_millis(900);
const SPARK_FRAME: Duration = Duration::from_millis(100);
const SPARK_GLYPHS: [char; 3] = ['\u{b7}', '\u{2726}', '\u{2727}'];
const CHARGE: Duration = Duration::from_millis(150);

/// `IgnitionStyle`: the three looks of the band.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Look {
    Wave,
    Aurora,
    Pulse,
}

impl Look {
    pub(crate) const ALL: [Look; 3] = [Look::Wave, Look::Aurora, Look::Pulse];

    /// `IgnitionStyle::random`: any look but the last one shown. The pick
    /// is the process's own randomness (`RandomState`), not a clock.
    pub(crate) fn random(previous: Option<Look>) -> Look {
        use std::hash::{BuildHasher, Hasher};
        let mut pick = std::collections::hash_map::RandomState::new()
            .build_hasher()
            .finish();
        loop {
            let look = Look::ALL[(pick % 3) as usize];
            if Some(look) != previous {
                return look;
            }
            pick = std::collections::hash_map::RandomState::new()
                .build_hasher()
                .finish();
        }
    }

    /// `IgnitionStyle::total_duration`.
    pub(crate) fn total(self, tier: Grade) -> Duration {
        let (max, ultra) = match self {
            Look::Wave => (1000, 1300),
            Look::Aurora => (1300, 1600),
            Look::Pulse => (900, 1250),
        };
        Duration::from_millis(match tier {
            Grade::Max => max,
            Grade::Ultra => ultra,
        })
    }
}

/// A band: `(launch_or_speed, travel_or_phase, strength_or_hue)`.
type Band = (f32, f32, f32);

fn bands(look: Look, tier: Grade) -> &'static [Band] {
    match (look, tier) {
        (Look::Wave, Grade::Max) => &[(0.10, 0.75, 1.0)],
        (Look::Wave, Grade::Ultra) => &[(0.10, 0.70, 1.0), (0.35, 0.55, 1.0)],
        (Look::Aurora, Grade::Max) => &[(0.35, 0.15, 0.0), (-0.50, 0.60, 1.0)],
        (Look::Aurora, Grade::Ultra) => &[(0.35, 0.15, 0.0), (-0.50, 0.60, 1.0), (0.75, 0.35, 2.0)],
        (Look::Pulse, Grade::Max) => &[(0.10, 0.60, 1.0)],
        (Look::Pulse, Grade::Ultra) => &[(0.10, 0.55, 0.8), (0.45, 0.55, 1.1)],
    }
}

fn crest(distance: f32) -> f32 {
    if distance >= 1.0 {
        0.0
    } else {
        0.5 * (1.0 + (std::f32::consts::PI * distance).cos())
    }
}

fn ease_in_out(progress: f32) -> f32 {
    let progress = progress.clamp(0.0, 1.0);
    if progress < 0.5 {
        4.0 * progress * progress * progress
    } else {
        let inverse = -2.0 * progress + 2.0;
        1.0 - inverse * inverse * inverse / 2.0
    }
}

pub(crate) fn envelope(elapsed: f32, total: f32, fade_in: f32, fade_out: f32) -> f32 {
    if elapsed <= 0.0 || elapsed >= total {
        return 0.0;
    }
    (elapsed / fade_in.max(f32::EPSILON))
        .min((total - elapsed) / fade_out.max(f32::EPSILON))
        .clamp(0.0, 1.0)
}

fn band_sample(look: Look, band: &Band, elapsed: f32, column: u16, width: u16) -> (usize, f32) {
    let column = f32::from(column);
    let width = f32::from(width);
    let (first, second, third) = *band;
    match look {
        Look::Wave => {
            let progress = (elapsed - first) / second;
            if !(0.0..=1.0).contains(&progress) {
                return (0, 0.0);
            }
            let center = ease_in_out(progress) * (width + 2.0 * WAVE_HALF_WIDTH) - WAVE_HALF_WIDTH;
            (0, crest((column - center).abs() / WAVE_HALF_WIDTH))
        }
        Look::Aurora => {
            let center =
                (0.5 + 0.38 * (std::f32::consts::TAU * (first * elapsed + second)).sin()) * width;
            let half_width = (width * 0.22).max(4.0);
            (third as usize, crest((column - center).abs() / half_width))
        }
        Look::Pulse => {
            let progress = (elapsed - first) / second;
            if !(0.0..=1.0).contains(&progress) {
                return (0, 0.0);
            }
            let inverse = 1.0 - progress;
            let radius =
                (1.0 - inverse * inverse * inverse) * (width / 2.0 + 2.0 * PULSE_HALF_WIDTH);
            let distance = (column - width / 2.0).abs();
            (
                0,
                crest((distance - radius).abs() / PULSE_HALF_WIDTH)
                    * third
                    * (1.0 - 0.6 * progress),
            )
        }
    }
}

/// `spark_frame`: the glyph of the Ultra wave's spark, after it lands.
pub(crate) fn spark_frame(elapsed: Duration, start: Duration) -> Option<char> {
    let frame = elapsed.checked_sub(start)?.as_millis() / SPARK_FRAME.as_millis();
    SPARK_GLYPHS.get(usize::try_from(frame).ok()?).copied()
}

/// `paint_bands`: the hue and strength each of `width` columns is tinted with.
pub(crate) fn tints(
    tier: Grade,
    look: Look,
    elapsed: Duration,
    hues: [Rgb; 3],
    width: u16,
) -> Vec<Option<(Rgb, f32)>> {
    let elapsed = elapsed.as_secs_f32();
    let total = look.total(tier).as_secs_f32();
    let fade = if look == Look::Aurora {
        envelope(elapsed, total, 0.25, 0.40)
    } else {
        1.0
    };
    (0..width)
        .map(|column| {
            let mut weights = [0.0_f32; 3];
            for band in bands(look, tier) {
                let (hue, strength) = band_sample(look, band, elapsed, column, width);
                weights[hue] = if look == Look::Aurora {
                    weights[hue] + strength
                } else {
                    weights[hue].max(strength)
                };
            }
            let weight = weights.iter().sum::<f32>();
            if weight <= 0.01 {
                return None;
            }
            let mut sum = [0.0_f32; 3];
            for (weight, (red, green, blue)) in weights.into_iter().zip(hues) {
                sum[0] += weight * f32::from(red);
                sum[1] += weight * f32::from(green);
                sum[2] += weight * f32::from(blue);
            }
            let hue = (
                (sum[0] / weight) as u8,
                (sum[1] / weight) as u8,
                (sum[2] / weight) as u8,
            );
            let alpha = if look == Look::Aurora {
                (weight * 0.40).min(0.50) * fade
            } else {
                weight * 0.55
            };
            Some((hue, alpha))
        })
        .collect()
}

/// The cells the composer's surface has beyond the box the aura wraps: the
/// padding `session_page` gives the composer (left, top, right, bottom).
const SURFACE: (i32, i32, i32, i32) = (0, 1, 1, 1);

/// The band over the composer's surface, painted under its text (the oracle
/// blends backgrounds underneath the draft and lets glyphs land only in empty
/// cells): the columns' tint, and the Ultra wave's spark.
#[derive(PartialEq)]
struct IgnitionPaint {
    tier: Grade,
    look: Look,
    elapsed: Duration,
    /// The surface colour the tint blends over (`user_message_bg_rgb`).
    band: Rgb,
    light: bool,
    /// The surface's cells beyond the box painted in (left, top, right, bottom).
    surface: (i32, i32, i32, i32),
}

impl std::fmt::Debug for IgnitionPaint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IgnitionPaint")
            .field("tier", &self.tier)
            .field("look", &self.look)
            .field("elapsed", &self.elapsed)
            .finish_non_exhaustive()
    }
}

impl CellPainter for IgnitionPaint {
    fn paint(&self, canvas: &mut Canvas<'_>, width: i32, height: i32) {
        // `effort_animation_enabled`: no ignition on 16 colours or none
        if !matches!(
            canvas.styles_mut().color_mode(),
            ColorMode::TrueColor | ColorMode::Ansi256
        ) {
            return;
        }
        let (left, top, right, bottom) = self.surface;
        let (area_w, area_h) = (width + left + right, height + top + bottom);
        let (Ok(columns), true) = (u16::try_from(area_w), area_h > 0) else {
            return;
        };
        let hues = self.tier.hues(self.light);
        for (column, tint) in tints(self.tier, self.look, self.elapsed, hues, columns)
            .into_iter()
            .enumerate()
        {
            let Some((hue, alpha)) = tint else { continue };
            if alpha < 0.02 {
                continue;
            }
            let color = blend(hue, self.band, alpha.clamp(0.0, 0.6));
            let pen = band_style(color).pen_in(canvas.styles_mut());
            canvas.fill(column as i32 - left, -top, 1, area_h, pen);
        }
        if self.look == Look::Wave
            && self.tier == Grade::Ultra
            && let Some(glyph) = spark_frame(self.elapsed, SPARK_START)
        {
            let color = blend(hues[0], self.band, 1.0);
            let pen = spark_style(color).pen_in(canvas.styles_mut());
            canvas.fill_char((area_w - 2).max(0) - left, -top, 1, 1, pen, glyph);
        }
    }

    fn should_repaint(&self, old: &dyn CellPainter) -> bool {
        old.downcast_ref::<Self>().is_none_or(|old| old != self)
    }
}

/// The composer's aura: always wraps `child` (so the draft keeps its element
/// and focus), paints the ignition band under it while a run plays.
#[derive(Clone, PartialEq)]
pub struct Aura {
    pub child: AnyWidget,
}

impl_widget!(stateful eq Aura);

#[derive(Default)]
pub struct AuraState {
    clock: Option<AnimationController>,
    watch: Watch,
    play: Cell<Option<Flight>>,
    last: Cell<Option<Look>>,
}

impl StatefulWidget for Aura {
    type State = AuraState;

    fn create_state(&self) -> AuraState {
        AuraState::default()
    }
}

impl State for AuraState {
    type Widget = Aura;
    type Msg = ();

    fn init_state(&mut self, _w: &Aura, cx: &mut Cx<'_, Self>) {
        self.clock = Some(AnimationController::new(cx).with_duration(Duration::from_secs(2)));
    }

    fn dispose(&mut self) {
        if let Some(clock) = self.clock.take() {
            clock.dispose();
        }
    }

    fn build(&self, w: &Aura, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let Some(clock) = self.clock.clone() else {
            return w.child.clone();
        };
        let signal = signal(cx);
        match self.watch.observe(&signal) {
            Seen::Same => {}
            Seen::Rest => {
                self.play.set(None);
                clock.stop();
            }
            Seen::Start(tier) => {
                let look = Look::random(self.last.get());
                self.last.set(Some(look));
                self.play.set(Some((tier, look)));
                clock.set_duration(Some(look.total(tier)));
                clock.forward(Some(0.0), cx);
            }
        }
        // the band blends over the surface; a terminal with no known
        // background plays nothing (`default_bg()` is none: cancelled)
        let tokens = Tokens::of(cx);
        let band = channels(tokens.prompt_surface(cx, false).bg);
        let play = self.play.get();
        let child = w.child.clone();
        AnimatedBuilder::new(clock, move |clock, child| {
            let painter = match (play, band, clock.duration()) {
                (Some((tier, look)), Some(band), Some(duration)) => {
                    let elapsed = duration.mul_f64(clock.value());
                    (elapsed < look.total(tier)).then_some(IgnitionPaint {
                        tier,
                        look,
                        elapsed,
                        band,
                        light: is_light(band),
                        surface: SURFACE,
                    })
                }
                _ => None,
            };
            let mut paint = CustomPaint::new();
            if let Some(painter) = painter {
                paint = paint.painter(painter);
            }
            match child {
                Some(child) => paint.child(child.clone()).into(),
                None => paint.into(),
            }
        })
        .child(child)
        .into()
    }
}

// ---- the prompt

/// The tier's glyph in its accent, one cell: painted rather than written as
/// text, because the colour depends on the colours the terminal shows (the
/// accent as it is on a true-colour or 256-colour terminal, the tier's own
/// yellow or magenta on 16 colours, none on a terminal without colour).
#[derive(Debug, PartialEq)]
struct GlyphPaint {
    tier: Grade,
    glyph: char,
    charge: f32,
    colors: Colors,
}

impl CellPainter for GlyphPaint {
    fn paint(&self, canvas: &mut Canvas<'_>, width: i32, height: i32) {
        if width < 1 || height < 1 {
            return;
        }
        let mode = canvas.styles_mut().color_mode();
        let style = glyph_style(self.tier, self.charge, self.colors, mode);
        let pen = style.pen_in(canvas.styles_mut());
        canvas.fill_char(0, 0, 1, 1, pen, self.glyph);
    }

    fn should_repaint(&self, old: &dyn CellPainter) -> bool {
        old.downcast_ref::<Self>().is_none_or(|old| old != self)
    }
}

/// The prompt cell of `tier` at `charge`, and the one blank after it.
fn tier_prompt(tier: Grade, glyph: &str, charge: f32, colors: Colors) -> AnyWidget {
    Padding::new(
        EdgeInsets::only(0, 0, 1, 0),
        CustomPaint::new().size(1, 1).painter(GlyphPaint {
            tier,
            glyph: glyph.chars().next().unwrap_or(' '),
            charge,
            colors,
        }),
    )
    .into()
}

/// The composer's prompt glyph (`tier.prompt(charge)`): the tier's glyph in its
/// accent while Max or Ultra is the effort, else `label` in `style`.
#[derive(Clone, PartialEq)]
pub struct Prompt {
    pub label: &'static str,
    pub style: Style,
}

impl_widget!(stateful eq Prompt);

#[derive(Default)]
pub struct PromptState {
    clock: Option<AnimationController>,
    watch: Watch,
    charging: Cell<bool>,
}

impl StatefulWidget for Prompt {
    type State = PromptState;

    fn create_state(&self) -> PromptState {
        PromptState::default()
    }
}

impl State for PromptState {
    type Widget = Prompt;
    type Msg = ();

    fn init_state(&mut self, _w: &Prompt, cx: &mut Cx<'_, Self>) {
        self.clock = Some(AnimationController::new(cx).with_duration(CHARGE));
    }

    fn dispose(&mut self) {
        if let Some(clock) = self.clock.take() {
            clock.dispose();
        }
    }

    fn build(&self, w: &Prompt, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let Some(clock) = self.clock.clone() else {
            return prompt(w.label, w.style.clone());
        };
        let signal = signal(cx);
        let tier = Grade::from_effort(signal.effort.as_deref());
        match self.watch.observe(&signal) {
            Seen::Same => {}
            Seen::Rest => self.charging.set(false),
            Seen::Start(_) => {
                self.charging.set(true);
                clock.forward(Some(0.0), cx);
            }
        }
        let own = strings::of(cx);
        let colors = Colors::of(cx);
        let (label, style) = (w.label, w.style.clone());
        let charging = self.charging.get();
        AnimatedBuilder::new(clock, move |clock, _| {
            let Some(tier) = tier else {
                return prompt(label, style.clone());
            };
            let charge = if charging { clock.value() as f32 } else { 1.0 };
            let glyph = match tier {
                Grade::Max => own.prompt_max,
                Grade::Ultra => own.prompt_ultra,
            };
            tier_prompt(tier, glyph, charge, colors)
        })
        .into()
    }
}

// ---- the debug entries

/// The status line of the oracle's snapshots: `model` in cyan, the branch dim.
fn sample_line(model: &str) -> Strip {
    vec![
        Span::styled(model.to_string(), sample_model_style()),
        Span::styled(sample::BRANCH, Style::new().attrs(Attrs::DIM)),
    ]
}

pub(crate) use entries::debug_entries;

mod entries {
    use crate::app::debug::{DebugEntry, EntryOpen};

    use super::*;

    const IDLE: &str = include_str!("fixtures/footer_idle.jsonl");

    /// The status line of the oracle's snapshots at `effort`.
    fn line(effort: &str) -> Vec<Span> {
        sample_line(&format!("{} {effort}", sample::MODEL))
    }

    fn transition(tier: Grade, previous: Vec<Span>, current: Vec<Span>) -> AnyWidget {
        StatusTransition {
            tier,
            previous,
            current,
            on_done: None,
        }
        .into()
    }

    /// The prompt at the oracle's one cell: the glyph of `tier`, charged.
    fn prompt_of(tier: Grade) -> AnyWidget {
        // the real `Prompt` reads the thread; this draws what it draws once
        // charged, from the same function
        super::entry_prompt(tier)
    }

    pub(crate) fn debug_entries() -> Vec<DebugEntry> {
        vec![
            DebugEntry {
                name: "effort_prompt_max",
                description: "the Max prompt glyph at full charge",
                script: IDLE,
                open: EntryOpen::Route(|_| prompt_of(Grade::Max)),
            },
            DebugEntry {
                name: "effort_prompt_ultra",
                description: "the Ultra prompt glyph at full charge",
                script: IDLE,
                open: EntryOpen::Route(|_| prompt_of(Grade::Ultra)),
            },
            DebugEntry {
                name: "effort_ignition_ultra",
                description: "the Ultra wave over a composer holding a draft",
                script: IDLE,
                open: EntryOpen::Route(|_| super::entry_ignition(Grade::Ultra, Look::Wave)),
            },
            DebugEntry {
                name: "effort_ignition_ultra_aurora",
                description: "the Ultra aurora over a composer holding a draft",
                script: IDLE,
                open: EntryOpen::Route(|_| super::entry_ignition(Grade::Ultra, Look::Aurora)),
            },
            DebugEntry {
                name: "effort_ignition_ultra_pulse",
                description: "the Ultra pulse over a composer holding a draft",
                script: IDLE,
                open: EntryOpen::Route(|_| super::entry_ignition(Grade::Ultra, Look::Pulse)),
            },
            DebugEntry {
                name: "effort_ignition_max_wave",
                description: "the Max wave over a composer holding a draft",
                script: IDLE,
                open: EntryOpen::Route(|_| super::entry_ignition(Grade::Max, Look::Wave)),
            },
            DebugEntry {
                name: "effort_transition_high_to_ultra",
                description: "the status line going from high to ultra",
                script: IDLE,
                open: EntryOpen::Route(|_| transition(Grade::Ultra, line("high"), line("ultra"))),
            },
            DebugEntry {
                name: "effort_transition_links",
                description: "the status line's links following the visible text",
                script: IDLE,
                open: EntryOpen::Route(|_| {
                    let link = |text: &str, url: &str| {
                        Span::styled(text.to_string(), Style::new().link(url.to_string()))
                    };
                    transition(
                        Grade::Max,
                        vec![
                            Span::new(sample::LINK_A),
                            link("PR42\u{754c}", "https://example.com/pr/42"),
                        ],
                        vec![
                            Span::new(sample::LINK_B),
                            link("PR43", "https://example.com/pr/43"),
                        ],
                    )
                }),
            },
            DebugEntry {
                name: "effort_status_line_frames",
                description: "the status line going from high to max and to ultra, twelve frames each",
                script: IDLE,
                open: EntryOpen::Route(|_| super::entry_transition_frames()),
            },
            DebugEntry {
                name: "effort_ignition_gallery",
                description: "every ignition look at both tiers, seven times each, over the draft",
                script: IDLE,
                open: EntryOpen::Route(|_| super::entry_ignition_gallery()),
            },
        ]
    }
}

/// The prompt of `tier` as the oracle's test draws it: one glyph, charged.
fn entry_prompt(tier: Grade) -> AnyWidget {
    // an entry has no thread to read: the accent from the terminal's colours
    #[derive(Clone, PartialEq)]
    struct ChargedPrompt(Grade);
    impl_widget!(stateless eq ChargedPrompt);
    impl rtwk::StatelessWidget for ChargedPrompt {
        fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
            let own = strings::of(cx);
            let glyph = match self.0 {
                Grade::Max => own.prompt_max,
                Grade::Ultra => own.prompt_ultra,
            };
            CustomPaint::new()
                .size(1, 1)
                .painter(GlyphPaint {
                    tier: self.0,
                    glyph: glyph.chars().next().unwrap_or(' '),
                    charge: 1.0,
                    colors: Colors::of(cx),
                })
                .into()
        }
    }
    ChargedPrompt(tier).into()
}

/// The ignition over a three-row composer holding the oracle's draft.
fn entry_ignition(tier: Grade, look: Look) -> AnyWidget {
    #[derive(Clone, PartialEq)]
    struct Demo(Grade, Look);
    impl_widget!(stateful eq Demo);
    #[derive(Default)]
    struct DemoState {
        clock: Option<AnimationController>,
    }
    impl StatefulWidget for Demo {
        type State = DemoState;
        fn create_state(&self) -> DemoState {
            DemoState::default()
        }
    }
    impl State for DemoState {
        type Widget = Demo;
        type Msg = ();
        fn init_state(&mut self, w: &Demo, cx: &mut Cx<'_, Self>) {
            let clock = AnimationController::new(cx).with_duration(w.1.total(w.0));
            clock.forward(Some(0.0), cx);
            self.clock = Some(clock);
        }
        fn dispose(&mut self) {
            if let Some(clock) = self.clock.take() {
                clock.dispose();
            }
        }
        fn build(&self, w: &Demo, cx: &mut Cx<'_, Self>) -> AnyWidget {
            let Some(clock) = self.clock.clone() else {
                return AnyWidget::default();
            };
            let band = channels(Tokens::of(cx).prompt_surface(cx, false).bg);
            let (tier, look) = (w.0, w.1);
            AnimatedBuilder::new(clock, move |clock, _| {
                ignition_block(band, tier, look, look.total(tier).mul_f64(clock.value()))
            })
            .into()
        }
    }
    Demo(tier, look).into()
}

/// The three-row composer holding the oracle's draft, `elapsed` into the run of `look`: the
/// band's tint under it while the run plays.
fn ignition_block(band: Option<Rgb>, tier: Grade, look: Look, elapsed: Duration) -> AnyWidget {
    let mut paint = CustomPaint::new();
    if let Some(band) = band
        && elapsed < look.total(tier)
    {
        paint = paint.painter(IgnitionPaint {
            tier,
            look,
            elapsed,
            band,
            light: is_light(band),
            surface: (0, 0, 0, 0),
        });
    }
    paint
        .child(
            rtwk::layout::Column::new()
                .cross_axis_alignment(rtwk::layout::CrossAxisAlignment::Stretch)
                .child(Text::line(""))
                .child(Text::line(sample::DRAFT))
                .child(Text::line("")),
        )
        .into()
}

/// The oracle's gallery of the ignition (`effort_ignition_animation_gallery`): every look, at
/// both tiers, at the seven times it prints, one three-row block under the other.
fn entry_ignition_gallery() -> AnyWidget {
    #[derive(Clone, PartialEq)]
    struct Gallery;
    impl_widget!(stateless eq Gallery);
    impl rtwk::StatelessWidget for Gallery {
        fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
            let band = channels(Tokens::of(cx).prompt_surface(cx, false).bg);
            let mut column = rtwk::layout::Column::new()
                .cross_axis_alignment(rtwk::layout::CrossAxisAlignment::Stretch);
            for look in Look::ALL {
                for tier in [Grade::Max, Grade::Ultra] {
                    for millis in IGNITION_TIMES {
                        // a paint fills the room it is given: three rows each
                        column = column.child(SizedBox::new(
                            None,
                            Some(3),
                            Some(ignition_block(
                                band,
                                tier,
                                look,
                                Duration::from_millis(millis),
                            )),
                        ));
                    }
                }
            }
            column.into()
        }
    }
    Gallery.into()
}

/// The elapsed times the oracle's ignition gallery prints.
const IGNITION_TIMES: [u64; 7] = [0, 180, 420, 720, 1100, 1580, 2050];

/// The oracle's frames of the status-line transition (`transition_frames_snapshot`): the line
/// going from `high` to Max, then to Ultra, at twelve times each, one row each.
fn entry_transition_frames() -> AnyWidget {
    #[derive(Clone, PartialEq)]
    struct Frames;
    impl_widget!(stateless eq Frames);
    impl rtwk::StatelessWidget for Frames {
        fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
            let own = strings::of(cx);
            let colors = Colors::of(cx);
            let mut column = rtwk::layout::Column::new()
                .cross_axis_alignment(rtwk::layout::CrossAxisAlignment::Start);
            let from = "high";
            for (tier, to) in [(Grade::Max, "max"), (Grade::Ultra, "ultra")] {
                for millis in TRANSITION_TIMES {
                    column = column.child(frame_row(
                        own,
                        colors,
                        tier,
                        sample_line(&format!("{} {from}", sample::MODEL)),
                        sample_line(&format!("{} {to}", sample::MODEL)),
                        Duration::from_millis(millis),
                    ));
                }
            }
            column.into()
        }
    }
    Frames.into()
}

/// The elapsed times the oracle's transition snapshot prints.
const TRANSITION_TIMES: [u64; 12] = [
    0, 300, 520, 680, 820, 1050, 1320, 1550, 1800, 2020, 2250, 2500,
];

#[cfg(test)]
mod tests {
    use super::*;

    fn signal(effort: Option<&str>, known: bool, animations: bool) -> Signal {
        Signal {
            effort: effort.map(str::to_string),
            known,
            animations,
        }
    }

    // oracle chat_composer_effort_tests.rs `effort_composer_baseline_repeat_and_lowering_do_not_replay`
    #[test]
    fn a_baseline_a_repeat_and_a_lowering_do_not_replay() {
        let watch = Watch::default();
        let ultra = signal(Some("ultra"), true, true);
        assert_eq!(watch.observe(&ultra), Seen::Rest);
        assert_eq!(watch.observe(&ultra), Seen::Same);
        assert_eq!(
            watch.observe(&signal(Some("max"), true, true)),
            Seen::Start(Grade::Max)
        );
        assert_eq!(watch.observe(&ultra), Seen::Start(Grade::Ultra));
        assert_eq!(
            watch.observe(&signal(Some("medium"), true, true)),
            Seen::Rest
        );
    }

    // oracle chat_composer_effort_tests.rs `effort_composer_restored_baseline_and_reduced_motion_do_not_start`
    #[test]
    fn a_restored_thread_and_reduced_motion_start_nothing() {
        let watch = Watch::default();
        // the first values seen before the thread has told its settings, then the thread's own
        assert_eq!(watch.observe(&signal(None, false, true)), Seen::Rest);
        assert_eq!(
            watch.observe(&signal(Some("ultra"), true, true)),
            Seen::Rest,
            "the thread's start is a baseline"
        );
        // reduced motion: the change is kept, nothing plays
        let quiet = Watch::default();
        quiet.observe(&signal(Some("high"), true, false));
        assert_eq!(
            quiet.observe(&signal(Some("ultra"), true, false)),
            Seen::Rest
        );
    }

    #[test]
    fn every_look_tints_some_column_while_it_runs() {
        for tier in [Grade::Max, Grade::Ultra] {
            for look in Look::ALL {
                let hues = tier.hues(false);
                let total = u64::try_from(look.total(tier).as_millis()).unwrap_or(0);
                let tinted = (1..total / 20).any(|step| {
                    tints(tier, look, Duration::from_millis(step * 20), hues, 40)
                        .iter()
                        .any(|tint| tint.is_some_and(|(_, alpha)| alpha >= 0.02))
                });
                assert!(tinted, "{tier:?} {look:?}");
                // and nothing at the end
                assert!(
                    tints(tier, look, look.total(tier), hues, 40)
                        .iter()
                        .all(|tint| tint.is_none_or(|(_, alpha)| alpha < 0.02)),
                    "{tier:?} {look:?} ends clean"
                );
            }
        }
    }
}
