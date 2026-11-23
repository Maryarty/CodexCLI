//! The composer's sparkle (oracle `bottom_pane/chat_composer/sparkle.rs` and
//! `sparkle_field.rs`): braille dots that twinkle in the free cells of an
//! empty composer on a fresh Astra thread, for the first fifteen seconds that
//! it is shown, the last of them fading.
//!
//! Flutter writes this as a foreground `CustomPaint` over the composer, a
//! `Ticker` that asks for the next frame while the dots are due, and a
//! controller (`SparkleController`, one per thread in `ThreadUi`) the keys
//! and the page talk to. `Sparkle` is the widget: it wraps the page's
//! composer box, so the dots reach the padding rows and the right margin the
//! way the oracle's whole-area pass does. It reads what is on the surface
//! when it paints (only a blank, unstyled cell with a true-colour background
//! gets a dot) and puts the dots over it, so what is written there never
//! needs to know of them. The frames are a pure function of the elapsed time
//! (`stars`), checked by the unit tests below against the oracle's hash.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use rtwk::binding::{AppBinding, Timer};
use rtwk::focus::FocusNode;
use rtwk::foundation::{Listenable, VoidCallback};
use rtwk::layout::{CellPainter, CustomPaint};
use rtwk::text::segmenter_of;
use rtwk::{AnyWidget, Cx, Handler, Rebuild, State, StatefulWidget, impl_widget};
use rtwk_core::canvas::Canvas;
use rtwk_core::style::{Attrs, ColorMode};
use rtwk_core::surface::BLANK;

use crate::app::theme::Rgb;
use crate::app::theme::effort::{Colors, blend, channels, dot_style};
use crate::widgets::composer_sparkle::{Draft, FRAME_TICK, IDLE_FADE, IDLE_TIMEOUT, Phase};
use crate::widgets::thread_ui::ThreadUi;

/// The glyphs of the dots (`DOTS`): one or two of the eight braille points.
pub const DOTS: [char; 8] = ['⠁', '⠂', '⠄', '⠈', '⠐', '⠠', '⡀', '⢀'];

/// The dots of the frame `elapsed` into the show, at `visibility`, in a box of
/// `width` by `height` cells: `(x, y, glyph, brightness)` of each, cell by
/// cell (`render_stars` without the surface). The hash is the oracle's, so the
/// same cells twinkle in the same phase.
pub fn stars(
    width: i32,
    height: i32,
    elapsed: Duration,
    visibility: f32,
) -> Vec<(i32, i32, char, f32)> {
    let time = elapsed.as_secs_f32();
    let mut out = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let mut hash = u64::try_from(y).unwrap_or(0) * 65537 + u64::try_from(x).unwrap_or(0);
            hash = (hash ^ (hash >> 16)).wrapping_mul(0x045d_9f3b);
            hash = (hash ^ (hash >> 16)).wrapping_mul(0x045d_9f3b);
            hash ^= hash >> 16;
            if hash % 5 != 0 {
                continue;
            }
            let phase =
                (time / (4.0 + (hash % 31) as f32 / 10.0) + (hash % 997) as f32 / 997.0).fract();
            let brightness = (phase * std::f32::consts::PI).sin().powi(12) * 0.55 * visibility;
            if brightness < 0.04 {
                continue;
            }
            out.push((x, y, DOTS[(hash / 161 % 8) as usize], brightness));
        }
    }
    out
}

/// How visible the dots are `elapsed` into the show: whole, then fading over
/// the last `IDLE_FADE`.
pub fn visibility(elapsed: Duration) -> f32 {
    let fade_start = IDLE_TIMEOUT - IDLE_FADE;
    if elapsed > fade_start {
        (IDLE_TIMEOUT.saturating_sub(elapsed)).as_secs_f32() / IDLE_FADE.as_secs_f32()
    } else {
        1.0
    }
}

/// The dots the show has `elapsed` into it, in a box of `width` by `height`
/// cells whose field starts at column 2 of row 1 with a placeholder of
/// `protected` columns (never a dot's): the field stops moving when the fade
/// begins, and the fade only takes dots away.
pub fn frame(
    width: i32,
    height: i32,
    elapsed: Duration,
    protected: i32,
) -> Vec<(i32, i32, char, f32)> {
    let still = elapsed.min(IDLE_TIMEOUT - IDLE_FADE);
    let protected = protected.min(width - 3);
    stars(width, height, still, visibility(elapsed))
        .into_iter()
        .filter(|&(x, y, _, _)| !(y == 1 && (2..2 + protected).contains(&x)))
        .collect()
}

/// The frame as a painter: the dots over the blank cells of the surface the
/// box's own paint left, but for the placeholder's (protected) and the cursor's.
#[derive(Clone, Debug, PartialEq)]
struct StarPaint {
    elapsed: Duration,
    foreground: Rgb,
    /// The columns the placeholder takes, in the text area.
    protected: i32,
}

impl CellPainter for StarPaint {
    fn paint(&self, canvas: &mut Canvas<'_>, width: i32, height: i32) {
        // `render_sparkle_at`: an area under three rows holds no field
        if width < 4 || height < 3 || canvas.styles().color_mode() != ColorMode::TrueColor {
            return;
        }
        let (ox, oy) = (canvas.origin_x(), canvas.origin_y());
        for (x, y, glyph, brightness) in frame(width, height, self.elapsed, self.protected) {
            let (Ok(ax), Ok(ay)) = (usize::try_from(x + ox), usize::try_from(y + oy)) else {
                continue;
            };
            let surface = canvas.surface();
            if ax >= surface.width() || ay >= surface.height() || surface.cell(ax, ay) != BLANK {
                continue;
            }
            let style = surface.style(ax, ay);
            let styles = canvas.styles();
            let Some(background) = channels(Some(styles.bg_of(style))) else {
                continue;
            };
            if styles.attrs_of(style) != Attrs::NONE {
                continue;
            }
            let pen = dot_style(blend(self.foreground, background, brightness))
                .pen_in(canvas.styles_mut());
            canvas.fill_char(x, y, 1, 1, pen, glyph);
        }
    }

    fn should_repaint(&self, old: &dyn CellPainter) -> bool {
        old.downcast_ref::<Self>().is_none_or(|old| old != self)
    }
}

/// The composer's sparkle: always wraps `child` (so the draft keeps its element
/// and focus) and paints the dots over it while they are due.
#[derive(Clone, PartialEq)]
pub struct Sparkle {
    pub ui: Rc<ThreadUi>,
    /// The composer's focus: the dots show only where typing would land.
    pub focus: FocusNode,
    /// The thread is offered the sparkle (`ThreadPart::sparkle_eligible`).
    pub eligible: bool,
    /// Something the page shows hides the dots: the shortcut overlay, a
    /// running turn, the history search.
    pub hidden: bool,
    /// The placeholder: its cells are the field's, never a dot's.
    pub placeholder: &'static str,
    pub child: AnyWidget,
}

impl_widget!(stateful eq Sparkle);

pub enum Msg {
    /// The draft, the focus or the phase moved: ask again.
    Poke,
    /// The next frame of the dots is due.
    Tick,
}

#[derive(Default)]
pub struct SparkleState {
    timer: RefCell<Option<Timer>>,
    listening: Option<Listening>,
}

/// The controllers whose changes the widget asks again for.
struct Listening {
    ui: Rc<ThreadUi>,
    focus: FocusNode,
    poke: VoidCallback,
    text: VoidCallback,
}

impl SparkleState {
    fn listen(&mut self, w: &Sparkle, cx: &mut Cx<'_, Self>) {
        self.unlisten();
        let tell = cx.local_sender();
        let poke: VoidCallback = {
            let tell = tell.clone();
            Rc::new(move || tell(Msg::Poke))
        };
        // every change of the composer's text, whoever made it: a key, a
        // paste, history, a clear (`note_sparkle_replaced_text`); the
        // history search's own previews are not the draft
        let text: VoidCallback = {
            let (ui, tell) = (w.ui.clone(), tell);
            Rc::new(move || {
                let text = ui.composer.text();
                ui.sparkle.text_changed(&text, ui.draft.searching());
                tell(Msg::Poke);
            })
        };
        w.ui.sparkle.listenable().add_listener(poke.clone());
        w.ui.composer.listenable().add_listener(text.clone());
        w.focus.add_listener(poke.clone());
        self.listening = Some(Listening {
            ui: w.ui.clone(),
            focus: w.focus.clone(),
            poke,
            text,
        });
    }

    fn unlisten(&mut self) {
        if let Some(listening) = self.listening.take() {
            listening
                .ui
                .sparkle
                .listenable()
                .remove_listener(&listening.poke);
            listening
                .ui
                .composer
                .listenable()
                .remove_listener(&listening.text);
            listening.focus.remove_listener(&listening.poke);
        }
    }

    fn cancel_timer(&self) {
        if let Some(timer) = self.timer.borrow_mut().take() {
            timer.cancel();
        }
    }

    /// The frame is due `after` from now, unless one is already asked for.
    fn schedule(&self, after: Duration, due: Handler<()>, binding: &AppBinding) {
        let mut timer = self.timer.borrow_mut();
        if timer.as_ref().is_some_and(Timer::is_active) {
            return;
        }
        *timer = Some(binding.start_timer(after, due));
    }
}

impl StatefulWidget for Sparkle {
    type State = SparkleState;

    fn create_state(&self) -> SparkleState {
        SparkleState::default()
    }
}

impl State for SparkleState {
    type Widget = Sparkle;
    type Msg = Msg;

    fn init_state(&mut self, w: &Sparkle, cx: &mut Cx<'_, Self>) {
        self.listen(w, cx);
    }

    fn did_update_widget(&mut self, old: &Sparkle, w: &Sparkle, cx: &mut Cx<'_, Self>) {
        if !Rc::ptr_eq(&old.ui, &w.ui) || old.focus != w.focus {
            self.listen(w, cx);
        }
    }

    fn dispose(&mut self) {
        self.unlisten();
        self.cancel_timer();
    }

    fn update(&mut self, msg: Msg, _w: &Sparkle, _cx: &mut Cx<'_, Self>) -> Rebuild {
        if matches!(msg, Msg::Tick) {
            // the timer that brought it is spent
            self.timer.borrow_mut().take();
        }
        Rebuild::Yes
    }

    fn build(&self, w: &Sparkle, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let mut paint = CustomPaint::new();
        if let Some(star) = self.verdict(w, cx) {
            paint = paint.foreground_painter(star);
        }
        paint.child(w.child.clone()).into()
    }
}

impl SparkleState {
    /// What the frame shows (`render_sparkle_at`): the dots of the time since
    /// they were first due, or nothing; the next frame is asked for while
    /// they are due.
    fn verdict(&self, w: &Sparkle, cx: &mut Cx<'_, Self>) -> Option<StarPaint> {
        let sparkle = &w.ui.sparkle;
        let composer = &w.ui.composer;
        let draft = &w.ui.draft;
        sparkle.offer(w.eligible);
        let text = composer.text();
        let empty = text.is_empty() && !draft.shell();
        sparkle.settle(&text, empty, draft.searching());
        let binding = AppBinding::maybe_of(cx)?;
        let now = binding.clock().now();
        let (since, elapsed) = match sparkle.phase() {
            Phase::Visible(since) => (Some(since), now.saturating_sub(since)),
            Phase::Waiting => (None, Duration::ZERO),
            Phase::Unarmed | Phase::Finished => {
                self.cancel_timer();
                return None;
            }
        };
        if elapsed >= IDLE_TIMEOUT {
            self.cancel_timer();
            sparkle.stop_visible();
            return None;
        }
        let colors = Colors::of(cx);
        if !w.focus.has_focus()
            || w.hidden
            || !empty
            || sparkle.draft() == Draft::Command
            || draft.in_burst()
        {
            self.cancel_timer();
            return None;
        }
        // a terminal that told no foreground colour shows nothing
        let foreground = colors.fg?;
        if since.is_none() {
            sparkle.begin(now);
        }
        let due = cx.map(|()| Msg::Tick);
        self.schedule(FRAME_TICK.min(IDLE_TIMEOUT - elapsed), due, &binding);
        Some(StarPaint {
            elapsed,
            foreground,
            protected: i32::try_from(segmenter_of(cx).width(w.placeholder)).unwrap_or(i32::MAX),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The cells the oracle's `sparkle_field` snapshot has dotted at time 0
    /// are a pure function of the hash: none of them in the cell 0,0.
    #[test]
    fn the_same_cells_twinkle_at_the_same_phase_every_run() {
        let a = stars(80, 4, Duration::ZERO, 1.0);
        let b = stars(80, 4, Duration::ZERO, 1.0);
        assert_eq!(a, b);
        assert!(!a.is_empty());
        assert!(a.iter().all(|(_, _, glyph, _)| DOTS.contains(glyph)));
    }

    #[test]
    fn a_fade_adds_no_dot_and_the_deadline_leaves_none() {
        let whole = stars(80, 4, Duration::from_secs(14), 1.0);
        let faded = stars(80, 4, Duration::from_secs(14), 0.5);
        assert!(faded.len() <= whole.len());
        for (x, y, _, _) in &faded {
            assert!(whole.iter().any(|(wx, wy, _, _)| (wx, wy) == (x, y)));
        }
        assert!(stars(80, 4, Duration::from_secs(14), 0.0).is_empty());
        assert_eq!(visibility(Duration::from_secs(14)), 1.0);
        assert_eq!(visibility(Duration::from_millis(14_500)), 0.5);
        assert_eq!(visibility(IDLE_TIMEOUT), 0.0);
    }

    /// The oracle's own snapshots, at its own instants: what each of their
    /// rows holds where the box has three rows (`sparkle_tests.rs` draws
    /// them at `now`, 14 s, 14.5 s and 15 s with no tick between).
    const FIELD: &str = include_str!(
        "../../../../../tui/src/bottom_pane/chat_composer/snapshots/codex_tui__bottom_pane__chat_composer__sparkle__tests__sparkle_matches_the_original_starfield_and_protects_the_placeholder_and_cursor.snap"
    );
    const FADE: &str = include_str!(
        "../../../../../tui/src/bottom_pane/chat_composer/snapshots/codex_tui__bottom_pane__chat_composer__sparkle__tests__sparkle_fades_without_adding_dots_and_finishes_by_fifteen_seconds.snap"
    );

    fn oracle_rows(text: &str, label: &str) -> Vec<Vec<char>> {
        let body = text.splitn(3, "---\n").nth(2).expect("snapshot body");
        let body = body.split_once("---\n").map_or(body, |(_, rest)| rest);
        let lines: Vec<&str> = body.lines().collect();
        let at = lines
            .iter()
            .position(|line| *line == label)
            .unwrap_or_else(|| panic!("no row `{label}`"));
        lines[at + 1..at + 4]
            .iter()
            .map(|line| line.chars().collect())
            .collect()
    }

    /// The dots of `frame(width, 3, elapsed, protected)` are the dots of the oracle's
    /// rows, and no other blank cell of those rows holds one.
    fn assert_matches(rows: &[Vec<char>], width: i32, elapsed: Duration, protected: i32) {
        let dots = frame(width, 3, elapsed, protected);
        for (y, row) in rows.iter().enumerate() {
            for x in 0..width {
                let cell = row.get(usize::try_from(x).unwrap()).copied().unwrap_or(' ');
                let ours = dots
                    .iter()
                    .find(|d| (d.0, d.1) == (x, i32::try_from(y).unwrap()))
                    .map(|d| d.2);
                if DOTS.contains(&cell) {
                    assert_eq!(ours, Some(cell), "dot at column {x}, row {y}, {elapsed:?}");
                } else if cell == ' ' {
                    assert_eq!(ours, None, "no dot at column {x}, row {y}, {elapsed:?}");
                }
            }
        }
    }

    #[test]
    fn the_starfield_is_the_oracles_at_its_first_instant() {
        assert_matches(
            &oracle_rows(FIELD, "80 columns, cursor Some((2, 1))"),
            80,
            Duration::ZERO,
            24,
        );
        assert_matches(
            &oracle_rows(FIELD, "40 columns, cursor Some((2, 1))"),
            40,
            Duration::ZERO,
            24,
        );
    }

    #[test]
    fn the_fade_is_the_oracles_at_fourteen_and_a_half_seconds() {
        assert_matches(
            &oracle_rows(FADE, "before fade:"),
            80,
            Duration::from_secs(14),
            24,
        );
        assert_matches(
            &oracle_rows(FADE, "fading:"),
            80,
            Duration::from_millis(14_500),
            24,
        );
        assert_matches(&oracle_rows(FADE, "finished:"), 80, IDLE_TIMEOUT, 24);
    }
}
