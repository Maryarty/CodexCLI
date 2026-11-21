//! The empty conversation blossom. Geometry, lighting, and timing come from
//! the oracle's pure artwork routines in `tui/src/empty_state_animation/`.

mod geometry;
mod lighting;
mod paths;
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
        })
        .into()
    })
    .into()
}

/// The blossom on a fixed stage of `width` by `height` cells that plays as it
/// is shown and again each time `replay` changes (the onboarding welcome:
/// `welcome.rs`, its 60 by 21 stage and the replay key).
pub(crate) fn stage(width: i32, height: i32, replay: u32) -> AnyWidget {
    Blossom {
        width,
        height,
        replay: Some(replay),
    }
    .into()
}

#[derive(Clone, PartialEq)]
struct Blossom {
    width: i32,
    height: i32,
    /// Plays at once and on each change; none plays on a tap only.
    replay: Option<u32>,
}

#[derive(Default)]
struct BlossomState {
    animation: Option<AnimationController>,
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
        if w.replay.is_some() {
            animation.forward(Some(0.0), cx);
        }
        self.animation = Some(animation);
    }

    fn did_update_widget(&mut self, old: &Blossom, w: &Blossom, cx: &mut Cx<'_, Self>) {
        if w.replay.is_some()
            && w.replay != old.replay
            && let Some(animation) = &self.animation
        {
            animation.forward(Some(0.0), cx);
        }
    }

    fn dispose(&mut self) {
        if let Some(animation) = &self.animation {
            animation.dispose();
        }
    }

    fn update(&mut self, (): (), _: &Blossom, cx: &mut Cx<'_, Self>) -> Rebuild {
        if let Some(animation) = &self.animation
            && !animation.is_animating()
        {
            animation.forward(Some(0.0), cx);
        }
        Rebuild::No
    }

    fn build(&self, w: &Blossom, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let Some(animation) = self.animation.clone() else {
            return AnyWidget::default();
        };
        let palette = Tokens::of(cx).blossom_palette();
        let renderer = self.renderer.clone();
        let (width, height) = (w.width, w.height);
        let fresh = w.replay.is_some();
        let paint = AnimatedBuilder::new(animation, move |animation, _| {
            let time = animation.value() * (SPIN_DURATION + STATIC_FADE).as_secs_f64();
            let millis = (time * 1000.0).round() as u64;
            let frame = if fresh || animation.is_animating() {
                millis / FRAME_MS
            } else {
                0
            };
            CustomPaint::new()
                .size(width, height)
                .painter(BlossomPaint {
                    frame,
                    fresh,
                    palette,
                    renderer: renderer.clone(),
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
        let (phase, opacity) = if self.fresh {
            (elapsed.min(SPIN_DURATION).as_secs_f64() / LOOP_SECONDS, 1.0)
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
        old.downcast_ref::<Self>()
            .is_none_or(|old| old.frame != self.frame || old.palette != self.palette)
    }
}
