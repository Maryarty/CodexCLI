//! The transcript's selection area: rtwk's `SelectionArea` owns the gestures (a drag, a double
//! and a triple click, the highlight, the clear key) and tells what a release copied; this hands
//! the text to the app as the feature's `Copy` intent (Flutter: a `SelectableRegion` with an
//! `onSelectionChanged` of the page's, here the one place that names the intent).

use rtwk::gestures::SelectionArea;
use rtwk::provider::read;
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::app::effects::{CopyDestination, CopyFormat};
use codex_tui_core::features::FeatureIntent;
use codex_tui_core::features::selection::Intent;

use crate::app::view::Hs;

/// What the reader selects in: `child` (the transcript's list), with the copy of a selection
/// asked of the app.
#[derive(Clone)]
pub(super) struct SelectionHost {
    pub child: AnyWidget,
}

pub(super) enum Msg {
    /// A selection was copied (a release, the copy key): the text of it.
    Copied(String),
}

impl StatefulWidget for SelectionHost {
    type State = HostState;

    fn create_state(&self) -> HostState {
        HostState
    }
}

impl_widget!(stateful SelectionHost);

pub(super) struct HostState;

impl State for HostState {
    type Widget = SelectionHost;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, _: &SelectionHost, cx: &mut Cx<'_, Self>) -> Rebuild {
        let Msg::Copied(text) = msg;
        let handlers = read::<Hs, _>(cx);
        cx.emit(
            &handlers.feature,
            FeatureIntent::from(Intent::Copy {
                text,
                format: CopyFormat::PlainText,
                destination: CopyDestination::Clipboard,
            }),
        );
        Rebuild::No
    }

    fn build(&self, widget: &SelectionHost, cx: &mut Cx<'_, Self>) -> AnyWidget {
        SelectionArea::new(widget.child.clone())
            .on_copy(cx.map(Msg::Copied))
            .into()
    }
}
