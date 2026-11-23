//! The result of the last copy in the gap above the composer (oracle `composer_gap.rs`
//! `show_copy_feedback`, `CopyFeedback::line`): right aligned, one cell from the edge, in the
//! longest of its three spellings that fits, the accent or red for a failure, for five
//! seconds (a copy under way stays until it ends).
//!
//! Flutter's `SnackBar` with a duration: a timer of the state hides it, and a new answer (its
//! `version`) starts it again; `fallback` is what the gap shows without it.

use std::time::Duration;

use rtwk::binding::{AppBinding, Timer};
use rtwk::layout::{Align, Alignment, Column, EdgeInsets, MainAxisSize, Padding, SizedBox};
use rtwk::text::{Text, segmenter_of};
use rtwk::{AnyWidget, Cx, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};

use super::copy::Feedback;
use super::strings;
use crate::app::theme;
use codex_tui_core::app::effects::CopyStatus;

/// How long the result of a copy stays in the gap (`composer_gap.rs`): a copy under way stays
/// until it ends.
const FEEDBACK_LIFE: Duration = Duration::from_secs(5);

/// The result of the last copy, right aligned in the gap one cell from the edge, in the
/// longest of its three spellings that fits (`first_fitting_line`), the accent or red for a
/// failure. Flutter's `SnackBar` with a duration: a timer of the state hides it, and a new
/// answer (its `version`) starts it again. `fallback` is what the gap shows without it.
#[derive(Clone, PartialEq)]
pub(super) struct CopyFeedback {
    pub feedback: Feedback,
    pub lead: bool,
    pub fallback: AnyWidget,
}

#[derive(Default)]
pub(super) struct CopyFeedbackState {
    /// The answer the timer was started for, and the one it hid.
    timed: Option<u64>,
    hidden: Option<u64>,
    timer: Option<Timer>,
}

impl CopyFeedbackState {
    fn arm(&mut self, w: &CopyFeedback, cx: &mut Cx<'_, Self>) {
        if self.timed == Some(w.feedback.version) {
            return;
        }
        self.timed = Some(w.feedback.version);
        if let Some(timer) = self.timer.take() {
            timer.cancel();
        }
        if matches!(w.feedback.result, Ok(CopyStatus::Pending(_))) {
            return;
        }
        if let Some(binding) = AppBinding::maybe_of(cx) {
            self.timer = Some(binding.start_timer(FEEDBACK_LIFE, cx.send(())));
        }
    }
}

impl StatefulWidget for CopyFeedback {
    type State = CopyFeedbackState;

    fn create_state(&self) -> CopyFeedbackState {
        CopyFeedbackState::default()
    }
}

impl_widget!(stateful eq CopyFeedback);

impl State for CopyFeedbackState {
    type Widget = CopyFeedback;
    type Msg = ();

    fn init_state(&mut self, w: &CopyFeedback, cx: &mut Cx<'_, Self>) {
        self.arm(w, cx);
    }

    fn did_update_widget(&mut self, _: &CopyFeedback, w: &CopyFeedback, cx: &mut Cx<'_, Self>) {
        self.arm(w, cx);
    }

    fn dispose(&mut self) {
        if let Some(timer) = self.timer.take() {
            timer.cancel();
        }
    }

    fn update(&mut self, (): (), _: &CopyFeedback, _: &mut Cx<'_, Self>) -> Rebuild {
        self.hidden = self.timed;
        Rebuild::Yes
    }

    fn build(&self, w: &CopyFeedback, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let tokens = theme::Tokens::of(cx);
        if self.hidden == Some(w.feedback.version) {
            return w.fallback.clone();
        }
        let labels = strings::copy_feedback(w.feedback.result, w.feedback.characters);
        let style = if w.feedback.result.is_err() {
            tokens.error()
        } else {
            tokens.accent()
        };
        let line = LayoutBuilder::new(move |cx, constraints| {
            let segmenter = segmenter_of(cx);
            // the oracle fits the line to the width less the cell it leaves free
            let room = constraints.max_width.saturating_sub(1);
            let label = labels
                .iter()
                .find(|label| i32::try_from(segmenter.width(label)).unwrap_or(i32::MAX) <= room)
                .or(labels.last())
                .cloned()
                .unwrap_or_default();
            Align::new(
                Alignment::CENTER_RIGHT,
                Padding::new(
                    EdgeInsets::only(0, 0, 1, 0),
                    Text::line(label).style(style.clone()),
                ),
            )
            .into()
        });
        let row: AnyWidget = SizedBox::new(None, Some(1), Some(line.into())).into();
        if w.lead {
            Column::new()
                .main_axis_size(MainAxisSize::Min)
                .child(SizedBox::new(None, Some(1), None))
                .child(row)
                .into()
        } else {
            row
        }
    }
}
