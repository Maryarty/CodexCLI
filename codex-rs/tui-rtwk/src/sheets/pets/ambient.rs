//! The chat owns the ambient pet's visibility. rtwk owns all frame decoding and playback.
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use codex_tui_core::features::FeatureStates;
use codex_tui_core::ids::ThreadId;
use codex_tui_core::screens::pets::{Ambient, Anchor, Clip, Motion};
use rtwk::animation::TickerMode;
use rtwk::binding::{AppBinding, InputListener, ScreenMode, Timer};
use rtwk::foundation::ListenableBuilder;
use rtwk::gestures::IgnorePointer;
use rtwk::graphics::GraphicsScope;
use rtwk::impl_widget;
use rtwk::layout::{EdgeInsets, Offstage, Padding, Positioned, RepaintBoundary, Stack, StackFit};
use rtwk::navigation::Route;
use rtwk::provider::{Provider, select};
use rtwk::{AnyWidget, Cx, Key, LayoutBuilder, Rebuild, State, StatefulWidget, StatelessWidget};
use rtwk_core::graphics::GraphicsProtocol;
use rtwk_core::input::InputEvent;
use rtwk_rich::image::{Image, ImageFit};

use crate::features::{PetIntent, verb};
use crate::widgets::thread_ui::ThreadUi;

const ROWS: i32 = 5;
const BOTTOM_GAP: i32 = 1;
const WRAP_GAP: i32 = 2;

#[derive(Clone)]
pub(crate) struct Pet {
    pub thread: ThreadId,
    pub hidden: bool,
    pub motion: Motion,
    pub ui: Rc<ThreadUi>,
}

impl PartialEq for Pet {
    fn eq(&self, other: &Self) -> bool {
        self.thread == other.thread
            && self.hidden == other.hidden
            && self.motion == other.motion
            && Rc::ptr_eq(&self.ui, &other.ui)
    }
}

pub(crate) fn cover(pet: Pet, child: AnyWidget) -> AnyWidget {
    Cover { pet, child }.into()
}

#[derive(Clone, PartialEq)]
struct Cover {
    pet: Pet,
    child: AnyWidget,
}

struct CoverState {
    focused: bool,
}

impl StatefulWidget for Cover {
    type State = CoverState;

    fn create_state(&self) -> CoverState {
        CoverState { focused: true }
    }
}

impl_widget!(stateful eq Cover);

impl State for CoverState {
    type Widget = Cover;
    type Msg = bool;

    fn init_state(&mut self, _: &Cover, cx: &mut Cx<'_, Self>) {
        if supported(cx) {
            verb(cx, PetIntent::FetchAmbient);
        }
    }

    fn update(&mut self, focused: bool, _: &Cover, _: &mut Cx<'_, Self>) -> Rebuild {
        if std::mem::replace(&mut self.focused, focused) == focused {
            Rebuild::No
        } else {
            Rebuild::Yes
        }
    }

    fn build(&self, widget: &Cover, cx: &mut Cx<'_, Self>) -> AnyWidget {
        if !supported(cx) {
            return widget.child.clone();
        }
        let ambient =
            select::<FeatureStates, _, _>(cx, |features| features.pets.ambient().cloned());
        let inline =
            AppBinding::maybe_of(cx).is_some_and(|binding| binding.mode() == ScreenMode::Inline);
        let visible = self.focused && Route::is_current_of(cx) && TickerMode::of(cx);
        let Cover { pet, child } = widget.clone();
        let mentions = pet.ui.draft.mentions();
        let layout = LayoutBuilder::new(move |_, room| {
            let cols = ambient.as_deref().map_or(0, columns);
            let reserve = if cols == 0 {
                0
            } else {
                (cols + WRAP_GAP).min((room.max_width - 1).max(0))
            };
            let child = Provider::value(Reserve(reserve))
                .update_should_notify(|old, new| old != new)
                .child(child.clone());
            let mut stack = Stack::new().fit(StackFit::Passthrough).child(child);
            if let Some(ambient) = ambient.as_ref() {
                let fits = room.max_width >= cols && room.max_height >= ROWS + BOTTOM_GAP;
                let visible = visible && !pet.hidden && fits;
                let pet = pet.clone();
                let anchor = ambient.anchor;
                let ambient = ambient.clone();
                let mentions = mentions.clone();
                let player = ListenableBuilder::new(mentions.listenable(), move |_, _| {
                    AnyWidget::from(Player {
                        ambient: ambient.clone(),
                        motion: pet.motion,
                        visible: visible && !mentions.is_open(),
                    })
                    .with_key(Key::value(pet.thread.clone()))
                });
                let position = Positioned::new(IgnorePointer::new().child(player))
                    .right(0)
                    .width(cols)
                    .height(ROWS);
                stack = stack.child(match anchor {
                    // Inline gives the chat a maximum available height. The native canvas
                    // still clips the image if the actual inline surface ends earlier.
                    Anchor::ScreenBottom if inline => {
                        position.top((room.max_height - ROWS - BOTTOM_GAP).max(0))
                    }
                    Anchor::Composer | Anchor::ScreenBottom => position.bottom(BOTTOM_GAP),
                });
            }
            stack.into()
        });
        let tell = cx.local_sender();
        InputListener::new(layout, move |event| {
            if let InputEvent::Focus(focused) = event {
                tell(*focused);
            }
            false
        })
        .into()
    }
}

fn supported<S>(cx: &mut Cx<'_, S>) -> bool {
    GraphicsScope::of(cx).is_some_and(|caps| caps.protocol != GraphicsProtocol::None)
}

fn columns(ambient: &Ambient) -> i32 {
    let aspect = f64::from(ambient.size.height) / f64::from(ambient.size.width) * 0.52;
    (f64::from(ROWS) / aspect).round().max(1.0) as i32
}

#[derive(Clone, Copy, PartialEq)]
struct Reserve(i32);

pub(crate) fn reserve(child: impl Into<AnyWidget>) -> AnyWidget {
    Wrap {
        child: child.into(),
    }
    .into()
}

#[derive(Clone, PartialEq)]
struct Wrap {
    child: AnyWidget,
}

impl_widget!(stateless eq Wrap);

impl StatelessWidget for Wrap {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let right = Provider::<Reserve>::maybe_of(cx, true).map_or(0, |reserve| reserve.0);
        Padding::new(EdgeInsets::only(0, right, 0, 0), self.child.clone()).into()
    }
}

#[derive(Clone, PartialEq)]
struct Player {
    ambient: Arc<Ambient>,
    motion: Motion,
    visible: bool,
}

#[derive(Default)]
struct Playback {
    motion: Motion,
    timer: Option<Timer>,
    epoch: u64,
    started: Option<Duration>,
    remaining: Option<Duration>,
}

impl StatefulWidget for Player {
    type State = Playback;

    fn create_state(&self) -> Playback {
        Playback::default()
    }
}

impl_widget!(stateful eq Player);

impl Playback {
    fn play(&mut self, widget: &Player, motion: Motion, cx: &mut Cx<'_, Self>) {
        self.cancel();
        self.motion = motion;
        let Clip { until_idle, .. } = widget.ambient.clip(motion);
        self.remaining = if motion == Motion::Idle {
            None
        } else {
            *until_idle
        };
        if widget.visible {
            self.resume(cx);
        }
    }

    fn resume(&mut self, cx: &mut Cx<'_, Self>) {
        if let Some((delay, binding)) = self.remaining.zip(AppBinding::maybe_of(cx)) {
            self.started = Some(binding.clock().now());
            self.timer = Some(binding.start_timer(delay, cx.send(self.epoch)));
        }
    }

    fn pause(&mut self, cx: &Cx<'_, Self>) {
        if let Some((started, binding)) = self.started.zip(AppBinding::maybe_of(cx)) {
            let elapsed = binding.clock().now().saturating_sub(started);
            self.remaining = self.remaining.map(|delay| delay.saturating_sub(elapsed));
        }
        self.cancel();
    }

    fn cancel(&mut self) {
        if let Some(timer) = self.timer.take() {
            timer.cancel();
        }
        self.started = None;
        self.epoch += 1;
    }
}

impl State for Playback {
    type Widget = Player;
    type Msg = u64;

    fn init_state(&mut self, widget: &Player, cx: &mut Cx<'_, Self>) {
        self.play(widget, widget.motion, cx);
    }

    fn did_update_widget(&mut self, old: &Player, widget: &Player, cx: &mut Cx<'_, Self>) {
        if !Arc::ptr_eq(&old.ambient, &widget.ambient) {
            self.play(widget, widget.motion, cx);
        } else if old.motion != widget.motion {
            let motion = match (old.motion, widget.motion) {
                (Motion::Running, Motion::Idle) => Motion::Review,
                (_, motion) => motion,
            };
            self.play(widget, motion, cx);
        } else if old.visible != widget.visible {
            if widget.visible {
                self.resume(cx);
            } else {
                self.pause(cx);
            }
        }
    }

    fn dispose(&mut self) {
        self.cancel();
    }

    fn update(&mut self, epoch: u64, _: &Player, _: &mut Cx<'_, Self>) -> Rebuild {
        if epoch != self.epoch {
            return Rebuild::No;
        }
        self.cancel();
        self.remaining = None;
        self.motion = Motion::Idle;
        Rebuild::Yes
    }

    fn build(&self, widget: &Player, _: &mut Cx<'_, Self>) -> AnyWidget {
        let clip = widget.ambient.clip(self.motion);
        Offstage::new(
            !widget.visible,
            TickerMode::new(
                widget.visible,
                RepaintBoundary::new(Image::memory(clip.data.clone()).fit(ImageFit::Fill)),
            ),
        )
        .into()
    }
}
