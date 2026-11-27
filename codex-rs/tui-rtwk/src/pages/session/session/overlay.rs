//! What lies over the page: a window of three rows or fewer shows nothing, the running state
//! included (`ui_snapshots_small_heights_idle`). The page keeps its layout and clips, and a
//! blank sheet laid over it covers it independently of the page's layout.

use rtwk::binding::{AppBinding, ScreenMode};
use rtwk::gestures::IgnorePointer;
use rtwk::layout::{
    BoxWidget, Column, CrossAxisAlignment, Expanded, Flexible, MainAxisSize, SizedBox, Stack,
    StackFit,
};
use rtwk::style::Style;
use rtwk::{AnyWidget, Cx, LayoutBuilder, StatelessWidget, impl_widget};

use codex_tui_core::screens::pets::Motion;

use crate::pages::session::composer::images;
use crate::sheets::pets::ambient;

use super::chord_guide::ChordGuide;
use super::frame::Frame;
use super::{Chat, ChatState};

/// The window height under which the chat draws nothing.
const HIDDEN_BELOW_ROWS: i32 = 4;

/// The transcript above the bottom pane, the guide of a held chord at the bottom right, and the
/// blank sheet over all. The live inline region fits its rows after settled output leaves it.
pub(super) fn page(
    transcript: Stack,
    body: Column,
    inline_draft: bool,
    guide: ChordGuide,
    pet: ambient::Pet,
) -> Stack {
    let page = ChatColumn {
        transcript: ambient::reserve(transcript),
        body: Bottom { child: body.into() },
        inline_draft,
    };
    let blank = LayoutBuilder::new(|_, constraints| {
        if constraints.max_height < HIDDEN_BELOW_ROWS {
            BoxWidget::new().style(Style::new()).into()
        } else {
            SizedBox::shrink().into()
        }
    });
    Stack::new().fit(StackFit::Expand).child(ambient::cover(
        pet,
        Stack::new()
            .fit(StackFit::Expand)
            .child(images::page_room(page.into()))
            .child(guide)
            .child(blank)
            .into(),
    ))
}

#[derive(Clone, PartialEq)]
struct ChatColumn {
    transcript: AnyWidget,
    body: Bottom,
    inline_draft: bool,
}

impl_widget!(stateless eq ChatColumn);

impl StatelessWidget for ChatColumn {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let inline =
            AppBinding::maybe_of(cx).is_some_and(|binding| binding.mode() == ScreenMode::Inline);
        let transcript = self.transcript.clone();
        let transcript = if self.inline_draft {
            transcript
        } else if inline {
            Flexible::new(1, transcript).into()
        } else {
            Expanded::new(1, transcript).into()
        };
        let main_axis = if inline {
            MainAxisSize::Min
        } else {
            MainAxisSize::Max
        };
        Column::new()
            .main_axis_size(main_axis)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .overflow_intended(true)
            .child(transcript)
            .child(self.body.clone())
            .into()
    }
}

pub(super) fn pet(state: &ChatState, w: &Chat, frame: &Frame) -> ambient::Pet {
    let motion = match () {
        () if frame.run.asking > 0 => Motion::Waiting,
        () if frame.run.running() => Motion::Running,
        () if frame.run.failure(&frame.session).is_some() => Motion::Failed,
        () => Motion::Idle,
    };
    ambient::Pet {
        thread: frame.thread.id().clone(),
        motion,
        hidden: state.shortcuts
            || state.banner.quick_menu.get()
            || frame.searching.is_some()
            || state.editing(frame.run.asking),
        ui: w.ui.clone(),
    }
}

/// The patch prompt leaves the transcript's pointer input available. The composer
/// beneath the prompt keeps its widgets and draft while its pointer input is off.
#[derive(Clone, PartialEq)]
struct Bottom {
    child: AnyWidget,
}

impl_widget!(stateless eq Bottom);

impl StatelessWidget for Bottom {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        IgnorePointer::new()
            .ignoring(crate::sheets::approvals::patch_route_active(cx))
            .child(self.child.clone())
            .into()
    }
}
