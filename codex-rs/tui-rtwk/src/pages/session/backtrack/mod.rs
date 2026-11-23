//! `backtrack`: the view half of the slot of S17 (backtrack (Esc Esc)), the `mock-chat` task's to fill. It
//! holds the empty form the skeleton registers: a screen widget, its opener
//! and the debug-menu entries. Its model part, events, intents and RPCs are
//! core's `codex_tui_core::screens::backtrack`. Nothing outside these two
//! directories (bar their test files) needs an edit when it grows.

use std::time::Duration;

use rtwk::binding::{AppBinding, Timer};
use rtwk::layout::SizedBox;
use rtwk::provider::{read, select};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::ids::ThreadId;
use codex_tui_core::screens::backtrack::{
    CONFIRM_SECS, EditError, ID, NAME, Selection, ThreadIntent,
};
use codex_tui_core::screens::{ScreenStates, ScreenThreadIntent};
use codex_tui_core::threads::{ItemKey, ThreadModel};

use crate::app::debug::DebugEntry;
use crate::app::debug::DebugOpen;
use crate::app::view::Hs;
use crate::pages::session::cells::error::ErrorCell;

pub mod strings;

/// What the debug menu lists for this directory: nothing yet. A slice adds
/// one entry per state worth opening (a screen on made-up data).
pub fn debug_entries() -> Vec<DebugEntry> {
    Vec::new()
}

/// The part as a widget, placed by the chat screen's subtree.
pub fn part() -> AnyWidget {
    Screen.into()
}

/// Opens the empty form of this slot (debug menu), as a page.
pub fn open_empty() -> DebugOpen {
    DebugOpen::Route(|_| Screen.into())
}

/// The empty screen widget of this slot.
#[derive(Clone, Debug, PartialEq)]
pub struct Screen;

/// Widget state: ids, drafts and controllers only (R14).
#[derive(Debug)]
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

    fn build(&self, _: &Screen, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let shared = crate::l10n::strings::of(cx);
        Text::new(shared.empty_slot(NAME, ID).to_string()).into()
    }
}

/// The intent of the prompt the reader chose (Enter while browsing): the edit, or the
/// refusal of a side conversation (the chat's flag, which the thread slot does not see).
/// None when the prompt is no longer in the transcript.
pub fn edit_intent<S>(
    cx: &mut Cx<'_, S>,
    thread: &ThreadModel,
    chosen: &Selection,
) -> Option<ScreenThreadIntent> {
    let side = read::<ScreenStates, _>(cx)
        .chat
        .flags
        .side_conversation_active;
    let intent = if side {
        ThreadIntent::Unavailable
    } else {
        ThreadIntent::EditPrompt(chosen.edit(thread.transcript())?)
    };
    Some(ScreenThreadIntent::Backtrack(intent))
}

/// The edit of a past prompt as the chat sees it: the error line of an edit that failed
/// or was not confirmed, and the 10 s timer of the wait for the server's
/// `ThreadReverted` (Flutter: a `Timer` in a `State`; core has no clock, it names the
/// ticket to wait on and takes the expiry back as an intent).
#[derive(Clone, Debug, PartialEq)]
pub struct EditWatch;

/// Widget state: the timer and the prompt whose edit it waits for (R14).
#[derive(Default)]
pub struct EditWatchState {
    waiting: Option<(ThreadId, ItemKey, Timer)>,
}

/// The thread and the prompt whose edit waits on the server's notice.
fn waiting_for(cx: &mut Cx<'_, EditWatchState>) -> (ThreadId, Option<ItemKey>) {
    select::<ThreadModel, (ThreadId, Option<ItemKey>), _>(cx, |thread| {
        (
            thread.id().clone(),
            thread.screen_slots().backtrack.awaiting(),
        )
    })
}

impl EditWatchState {
    fn watch(&mut self, cx: &mut Cx<'_, Self>) {
        let now = waiting_for(cx);
        let wanted = now.1.map(|key| (now.0, key));
        let armed = self
            .waiting
            .as_ref()
            .map(|(thread, key, _)| (thread.clone(), key.clone()));
        if armed == wanted {
            return;
        }
        if let Some((_, _, timer)) = self.waiting.take() {
            timer.cancel();
        }
        if let Some((thread, key)) = wanted
            && let Some(binding) = AppBinding::maybe_of(cx)
        {
            let timer = binding.start_timer(
                Duration::from_secs(CONFIRM_SECS),
                cx.map(move |(): ()| Msg::Expired),
            );
            self.waiting = Some((thread, key, timer));
        }
    }
}

/// The timer ran out.
#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Expired,
}

impl StatefulWidget for EditWatch {
    type State = EditWatchState;

    fn create_state(&self) -> EditWatchState {
        EditWatchState::default()
    }
}

impl_widget!(stateful eq EditWatch);

impl State for EditWatchState {
    type Widget = EditWatch;
    type Msg = Msg;

    fn did_change_dependencies(&mut self, _: &EditWatch, cx: &mut Cx<'_, Self>) {
        self.watch(cx);
    }

    fn dispose(&mut self) {
        if let Some((_, _, timer)) = self.waiting.take() {
            timer.cancel();
        }
    }

    fn update(&mut self, Msg::Expired: Msg, _: &EditWatch, cx: &mut Cx<'_, Self>) -> Rebuild {
        if let Some((thread, key, _)) = self.waiting.take() {
            let hs = read::<Hs, _>(cx);
            let intent = ScreenThreadIntent::Backtrack(ThreadIntent::Expired(key));
            cx.emit(&hs.thread_screen, (thread, intent));
        }
        Rebuild::No
    }

    fn build(&self, _: &EditWatch, cx: &mut Cx<'_, Self>) -> AnyWidget {
        // each build selects anew: the prompt's key stays a dependency of `did_change_dependencies`
        waiting_for(cx);
        let error = select::<ThreadModel, Option<EditError>, _>(cx, |thread| {
            thread.screen_slots().backtrack.error.clone()
        });
        let Some(error) = error else {
            return SizedBox::shrink().into();
        };
        let table = strings::of(cx);
        let message = match &error {
            EditError::Failed(message) => table.failed_with(message),
            EditError::NotConfirmed => table.not_confirmed.to_string(),
            EditError::Side => table.side.to_string(),
        };
        ErrorCell { message }.into()
    }
}
