//! The empty conversation blossom. Geometry, lighting, and timing come from
//! the oracle's pure artwork routines in `tui/src/empty_state_animation/`.

mod geometry;
mod lighting;
mod paths;
mod policy;
mod renderer;
mod sequence;

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use crate::app::theme::{BlossomPalette, Tokens};
use rtwk::animation::{AnimatedBuilder, AnimationController};
use rtwk::gestures::GestureDetector;
use rtwk::layout::{CellPainter, Center, CustomPaint};
use rtwk::{AnyWidget, Cx, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};
use rtwk_core::canvas::Canvas;

use self::lighting::Lighting;
use self::renderer::{MAX_COLUMNS, Renderer};
use self::sequence::{
    LOOP_SECONDS, SPIN_DURATION, STATIC_FADE, STATIC_OPACITY, progress, static_opacity,
};

const FRAME_MS: u64 = 50;
const SETTLED_PHASE: f64 = 0.5;

pub(super) use self::policy::startup_only;

pub(super) fn welcome() -> AnyWidget {
    LayoutBuilder::new(|_, constraints| {
        let width = (constraints.max_width - 4).clamp(0, i32::from(MAX_COLUMNS));
        let height = width * 21 / i32::from(MAX_COLUMNS);
        if height < 14 || height > constraints.max_height {
            return AnyWidget::default();
        }
        Center::new(Blossom {
            width,
            height,
            replay: None,
            faded: false,
        })
        .into()
    })
    .into()
}

/// The blossom on a fixed stage of `width` by `height` cells that plays as it
/// is shown and again each time `replay` changes (the onboarding welcome:
/// `welcome.rs`, its 60 by 21 stage and the replay key). `faded` is the
/// oracle's `Presentation::Faded` (a text entry is open): the spin stops
/// where it is and the mark fades to the static opacity; back to animated,
/// it resumes.
pub(crate) fn stage(width: i32, height: i32, replay: u32, faded: bool) -> AnyWidget {
    Blossom {
        width,
        height,
        replay: Some(replay),
        faded,
    }
    .into()
}

#[derive(Clone, PartialEq)]
struct Blossom {
    width: i32,
    height: i32,
    /// Plays at once and on each change; none plays on a tap only.
    replay: Option<u32>,
    /// `Presentation::Faded`: stopped and faded to the static opacity.
    faded: bool,
}

#[derive(Default)]
struct BlossomState {
    animation: Option<AnimationController>,
    /// How far the mark is faded to the static opacity, 0 to 1 over
    /// `STATIC_FADE` (oracle `fade_elapsed`).
    fade: Option<AnimationController>,
    /// The spin was stopped by the fade where it stood; it resumes with it.
    paused: bool,
    renderer: Rc<RefCell<Renderer>>,
}

impl StatefulWidget for Blossom {
    type State = BlossomState;

    fn create_state(&self) -> BlossomState {
        BlossomState::default()
    }
}

impl State for BlossomState {
    type Widget = Blossom;
    type Msg = ();

    fn init_state(&mut self, w: &Blossom, cx: &mut Cx<'_, Self>) {
        let animation = AnimationController::new(cx).with_duration(SPIN_DURATION + STATIC_FADE);
        let fade = AnimationController::new(cx)
            .with_duration(STATIC_FADE)
            .with_value(if w.faded { 1.0 } else { 0.0 });
        // Shown faded from the start, the spin never runs (`static_mark`).
        if w.faded {
            self.paused = true;
        } else if w.replay.is_some() {
            animation.forward(Some(0.0), cx);
        }
        self.animation = Some(animation);
        self.fade = Some(fade);
    }

    fn did_update_widget(&mut self, old: &Blossom, w: &Blossom, cx: &mut Cx<'_, Self>) {
        let (Some(animation), Some(fade)) = (&self.animation, &self.fade) else {
            return;
        };
        if w.faded != old.faded {
            if w.faded {
                // A spin under way stops where it is and fades; a finished
                // one stays as it ended (the oracle's `finished` opacity).
                if animation.is_animating() {
                    animation.stop();
                    self.paused = true;
                    fade.forward(None, cx);
                }
            } else {
                fade.reverse(None, cx);
                if self.paused {
                    self.paused = false;
                    animation.forward(Some(animation.value()), cx);
                }
            }
        }
        if w.replay.is_some() && w.replay != old.replay && !w.faded {
            self.paused = false;
            animation.forward(Some(0.0), cx);
        }
    }

    fn dispose(&mut self) {
        if let Some(animation) = &self.animation {
            animation.dispose();
        }
        if let Some(fade) = &self.fade {
            fade.dispose();
        }
    }

    fn update(&mut self, (): (), _: &Blossom, cx: &mut Cx<'_, Self>) -> Rebuild {
        if let Some(animation) = &self.animation
            && !animation.is_animating()
            && !self.paused
        {
            animation.forward(Some(0.0), cx);
        }
        Rebuild::No
    }

    fn build(&self, w: &Blossom, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let (Some(animation), Some(fade)) = (self.animation.clone(), self.fade.clone()) else {
            return AnyWidget::default();
        };
        let palette = Tokens::of(cx).blossom_palette();
        let renderer = self.renderer.clone();
        let (width, height) = (w.width, w.height);
        let fresh = w.replay.is_some();
        let paused = self.paused;
        let paint = AnimatedBuilder::new(fade, move |fade, _| {
            let faded =
                (fade.value() * STATIC_FADE.as_secs_f64() * 1000.0).round() as u64 / FRAME_MS;
            let renderer = renderer.clone();
            AnimatedBuilder::new(animation.clone(), move |animation, _| {
                let time = animation.value() * (SPIN_DURATION + STATIC_FADE).as_secs_f64();
                let millis = (time * 1000.0).round() as u64;
                let frame = if fresh || animation.is_animating() || paused {
                    millis / FRAME_MS
                } else {
                    0
                };
                CustomPaint::new()
                    .size(width, height)
                    .painter(BlossomPaint {
                        frame,
                        faded,
                        fresh,
                        palette,
                        renderer: renderer.clone(),
                    })
                    .into()
            })
            .into()
        });
        GestureDetector::new()
            .child(paint)
            .on_tap(cx.map(|_| ()))
            .into()
    }
}

impl_widget!(stateful eq Blossom);

struct BlossomPaint {
    frame: u64,
    /// The fade to the static opacity, in frames (`FRAME_MS`) of its
    /// `STATIC_FADE`.
    faded: u64,
    /// The welcome's fresh start (oracle `start_fresh`): the spin from the
    /// intact mark at full opacity, not the replay from the settled blossom.
    fresh: bool,
    palette: BlossomPalette,
    renderer: Rc<RefCell<Renderer>>,
}

impl std::fmt::Debug for BlossomPaint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BlossomPaint")
            .field("frame", &self.frame)
            .finish()
    }
}

impl CellPainter for BlossomPaint {
    fn paint(&self, canvas: &mut Canvas<'_>, width: i32, height: i32) {
        if width == 0 || height == 0 {
            return;
        }
        let elapsed = Duration::from_millis(self.frame * FRAME_MS);
        let faded = Duration::from_millis(self.faded * FRAME_MS);
        let (phase, opacity) = if self.fresh {
            (
                elapsed.min(SPIN_DURATION).as_secs_f64() / LOOP_SECONDS,
                static_opacity(faded, 1.0),
            )
        } else if elapsed >= SPIN_DURATION {
            (SETTLED_PHASE, static_opacity(elapsed - SPIN_DURATION, 1.0))
        } else {
            (
                SETTLED_PHASE + elapsed.as_secs_f64() / LOOP_SECONDS,
                STATIC_OPACITY + (1.0 - STATIC_OPACITY) * progress(elapsed, STATIC_FADE) as f32,
            )
        };
        let lighting = Lighting::terminal(self.palette.foreground, self.palette.background);
        let mut renderer = self.renderer.borrow_mut();
        let cells = renderer.frame(width as u16, height as u16, phase, &lighting);
        for (index, cell) in cells.iter().enumerate().filter(|(_, cell)| cell.dots != 0) {
            let [_, r, g, b] = cell.rgb.to_be_bytes();
            let style = self.palette.style((r, g, b), f64::from(opacity));
            let pen = style.pen_in(canvas.styles_mut());
            let glyph = char::from_u32(0x2800 + u32::from(cell.dots)).unwrap_or(' ');
            let index = index as i32;
            canvas.fill_char(index % width, index / width, 1, 1, pen, glyph);
        }
    }

    fn should_repaint(&self, old: &dyn CellPainter) -> bool {
        old.downcast_ref::<Self>().is_none_or(|old| {
            old.frame != self.frame || old.faded != self.faded || old.palette != self.palette
        })
    }
}
