//! The transcript's selection area (oracle `transcript_view/selection.rs`, `input.rs`): rtwk's
//! `SelectionArea` owns the gestures (a drag, a double and a triple click, the highlight, the
//! clear key) and tells what a release copied; the host around it turns what the reader asks to
//! copy into the app's clipboard ask (`Hs.copy`: Flutter's `SelectableRegion` with an
//! `onSelectionChanged` of the page's), copies with a right click (`input.rs` `Down(Right)`),
//! and lets the selection go once a copy it asked for is confirmed
//! (`tui.copy_on_select_clear_selection`; `finish_copy`).
//!
//! The selection is drawn state: it lives in the area and its controller, which the thread's
//! `ThreadUi` keeps so that the page's keys can read it too (`key`).

use rtwk::focus::KeyPress;
use rtwk::gestures::{Listener, PointerEvent, SelectionArea, SelectionAreaController};
use rtwk::prelude::{KeyCode, Mods};
use rtwk::provider::{read, watch};
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::app::effects::CopyStatus;
use codex_tui_core::app::facts::TuiSettings;

use super::copy::{CopyJob, Shown};
use crate::app::view::Hs;
use crate::widgets::thread_ui::Follow;

/// The button a right click is in a `PointerEvent`.
const RIGHT_BUTTON: u8 = 2;

/// What the reader selects in: `child` (the transcript's list), served by `controller`; `follow`
/// is the list's reader, whom a copy by Enter takes back to the last row.
#[derive(Clone)]
pub(in crate::pages::session) struct SelectionHost {
    pub child: AnyWidget,
    pub controller: SelectionAreaController,
    pub follow: Follow,
}

/// What a key does while something is selected (oracle `handle_selection_key`): the keys are
/// the selection's before they are the composer's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::pages::session) enum Key {
    /// Ctrl+C, Cmd+C, Ctrl+Shift+C and Ctrl+Insert (`text_selection::is_copy_key`).
    Copy,
    /// Enter: copy, and back to the last row once it is delivered.
    CopyAndFollow,
    /// Esc: let the selection go.
    Clear,
}

/// The selection's meaning of `key`, if it has one.
pub(in crate::pages::session) fn key(key: &KeyPress) -> Option<Key> {
    let letter = |c: &str| {
        key.code == KeyCode::Char && key.text().is_some_and(|t| t.eq_ignore_ascii_case(c))
    };
    if (letter("c")
        && (key.mods == Mods::CTRL
            || key.mods == Mods::SUPER
            || key.mods == (Mods::CTRL | Mods::SHIFT)))
        || (key.code == KeyCode::Insert && key.mods == Mods::CTRL)
    {
        return Some(Key::Copy);
    }
    match key.code {
        KeyCode::Enter if key.mods.is_empty() => Some(Key::CopyAndFollow),
        KeyCode::Escape => Some(Key::Clear),
        _ => None,
    }
}

pub(in crate::pages::session) enum Msg {
    /// A release copied what its press selected: the text of it.
    Copied(String),
    /// A press of the mouse over the transcript; the right button copies what is selected.
    Press(PointerEvent),
}

#[derive(Default)]
pub(in crate::pages::session) struct HostState {
    /// The host has looked at the feedback once: what stood then is older than the host.
    mounted: bool,
    /// The newest answer of a copy this host has taken in: an older one is none of its business.
    answered: Option<u64>,
}

impl StatefulWidget for SelectionHost {
    type State = HostState;

    fn create_state(&self) -> HostState {
        HostState::default()
    }
}

impl_widget!(stateful SelectionHost);

impl State for HostState {
    type Widget = SelectionHost;
    type Msg = Msg;

    fn did_change_dependencies(&mut self, w: &SelectionHost, cx: &mut Cx<'_, Self>) {
        let shown = watch::<Shown, _>(cx);
        let first = !std::mem::replace(&mut self.mounted, true);
        let Some(feedback) = &shown.0 else {
            return;
        };
        if self.answered == Some(feedback.version) {
            return;
        }
        self.answered = Some(feedback.version);
        if first || feedback.result != Ok(CopyStatus::Confirmed) {
            return;
        }
        // the answer a copy was waiting for: its selection goes, and only that one
        if let Some(text) = &feedback.clear_after
            && w.controller.text() == *text
        {
            w.controller.clear();
        }
        if feedback.follow {
            w.follow.jump_to_latest();
        }
    }

    fn update(&mut self, msg: Msg, w: &SelectionHost, cx: &mut Cx<'_, Self>) -> Rebuild {
        let (text, explicit) = match msg {
            Msg::Copied(text) => (text, false),
            Msg::Press(press) if press.button == RIGHT_BUTTON => (w.controller.text(), true),
            Msg::Press(_) => return Rebuild::No,
        };
        if text.is_empty() {
            return Rebuild::No;
        }
        let settings = read::<TuiSettings, _>(cx);
        let handlers = read::<Hs, _>(cx);
        cx.emit(&handlers.copy, CopyJob::selected(text, explicit, &settings));
        Rebuild::No
    }

    fn build(&self, w: &SelectionHost, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let settings = watch::<TuiSettings, _>(cx);
        let area = SelectionArea::new(w.child.clone())
            .controller(w.controller.clone())
            .copy_on_release(settings.copy_on_select)
            .on_copy(cx.map(Msg::Copied));
        Listener::new()
            .on_pointer_down(cx.map(Msg::Press))
            .child(area)
            .into()
    }
}
