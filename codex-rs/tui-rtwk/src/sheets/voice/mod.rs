//! Voice settings are pages of a sheet over the session that opened it.

use rtwk::navigation::Navigator;
use rtwk::overlay::{Dialog, DialogResult, close_dialog, show_dialog};
use rtwk::provider::{read, select};
use rtwk::{AnyWidget, Cx, State, StatefulWidget, impl_widget};

use codex_tui_core::ids::ThreadId;
use codex_tui_core::threads::ThreadModel;

use crate::app::debug::{DebugEntry, DebugOpen};
use crate::widgets::sheet::bottom;

mod settings;
pub mod strings;
pub(crate) mod strip;

pub fn debug_entries() -> Vec<DebugEntry> {
    Vec::new()
}

pub fn open<S>(cx: &mut Cx<'_, S>) -> DialogResult<()> {
    let thread = read::<ThreadModel, _>(cx).id().clone();
    show_dialog(
        cx,
        move |_| {
            Dialog::bare(Screen {
                thread: Some(thread.clone()),
            })
            .into()
        },
        crate::sheets::sheet_options(),
    )
}

pub fn open_empty() -> DebugOpen {
    DebugOpen::Dialog(|_| Dialog::bare(Screen { thread: None }).into())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Screen {
    thread: Option<ThreadId>,
}

pub struct ScreenState;

impl StatefulWidget for Screen {
    type State = ScreenState;

    fn create_state(&self) -> ScreenState {
        ScreenState
    }
}

impl_widget!(stateful eq Screen);

impl State for ScreenState {
    type Widget = Screen;
    type Msg = ();

    fn did_change_dependencies(&mut self, w: &Screen, cx: &mut Cx<'_, Self>) {
        if let Some(thread) = &w.thread
            && select::<ThreadModel, _, _>(cx, |active| active.id().clone()) != *thread
        {
            close_dialog::<_, ()>(cx, None);
        }
    }

    fn build(&self, w: &Screen, _: &mut Cx<'_, Self>) -> AnyWidget {
        Navigator::new()
            .home(bottom(settings::Page::home(w.thread.clone())))
            .back_key(None)
            .into()
    }
}
