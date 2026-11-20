//! The terminal caption, with an rtwk animation scoped to the activity field.

use rtwk::animation::{AnimatedBuilder, AnimationController, SpinnerFrames};
use rtwk::binding::TerminalTitle;
use rtwk::layout::SizedBox;
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

#[derive(Clone, PartialEq)]
pub(super) struct WindowTitle {
    pub before: Vec<String>,
    pub after: Vec<String>,
}

#[derive(Default)]
pub(super) struct WindowTitleState {
    animation: Option<AnimationController>,
}

impl StatefulWidget for WindowTitle {
    type State = WindowTitleState;

    fn create_state(&self) -> Self::State {
        WindowTitleState::default()
    }
}

impl State for WindowTitleState {
    type Widget = WindowTitle;
    type Msg = ();

    fn init_state(&mut self, _: &WindowTitle, cx: &mut Cx<'_, Self>) {
        let frames = SpinnerFrames::DOTS;
        let period = frames.interval * frames.frames.len() as u32;
        let animation = AnimationController::new(cx).with_duration(period);
        AnimationController::repeat(&animation, false, None, cx);
        self.animation = Some(animation);
    }

    fn dispose(&mut self) {
        if let Some(animation) = self.animation.take() {
            animation.dispose();
        }
    }

    fn update(&mut self, (): (), _: &WindowTitle, _: &mut Cx<'_, Self>) -> Rebuild {
        Rebuild::No
    }

    fn build(&self, w: &WindowTitle, _: &mut Cx<'_, Self>) -> AnyWidget {
        let Some(animation) = &self.animation else {
            return empty().into();
        };
        let frames = SpinnerFrames::DOTS.frames;
        let before = w.before.join(" · ");
        let after = w.after.join(" · ");
        AnimatedBuilder::new(animation.clone(), move |animation, _| {
            let index = (animation.value() * frames.len() as f64) as usize % frames.len();
            let parts = [before.as_str(), &frames[index], after.as_str()];
            let caption = parts
                .into_iter()
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
                .join(" · ");
            TerminalTitle::new(caption, empty()).into()
        })
        .into()
    }
}

impl_widget!(stateful eq WindowTitle);

pub(super) fn empty() -> SizedBox {
    SizedBox::new(Some(0), Some(0), None)
}
