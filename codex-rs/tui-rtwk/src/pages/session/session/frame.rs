//! What one build of the chat page reads before it draws, shared by its sections (`reader`,
//! `slot`, `bottom`, `title`, `input`): the providers' values and what follows from them for the
//! whole build. Reading them here, in the order the build always did, keeps what the page
//! depends on in one place.

use std::rc::Rc;

use rtwk::Cx;
use rtwk::provider::{read, select, watch};

use codex_protocol::account::PlanType;
use codex_tui_core::app::{ConnectionState, SessionSlice};
use codex_tui_core::features::FeatureStates;
use codex_tui_core::keymap::Keymap;
use codex_tui_core::kit::history::SearchStatus;
use codex_tui_core::screens::ScreenStates;
use codex_tui_core::screens::chat;
use codex_tui_core::screens::status::{Account, Figure};
use codex_tui_core::threads::ThreadModel;
use codex_tui_core::threads::hooks::RunningHook;

use crate::app::theme::Tokens;
use crate::l10n::strings::{self as l10n, Shared};
use crate::pages::session::activity::Run;
use crate::pages::session::live::provider_error;
use crate::pages::session::slice::{self, Drawn};
use crate::pages::session::status_surfaces::StatusContext;
use crate::pages::session::strings::{self as chat_strings, Strings};

use super::msg::ChatMsg;
use super::{Chat, ChatState};

/// The values of one build: what the sections draw from.
pub(super) struct Frame {
    pub tokens: Tokens,
    pub thread: Rc<ThreadModel>,
    pub session: Rc<SessionSlice>,
    pub strings: &'static Shared,
    pub chat_strings: &'static Strings,
    pub keymap: Rc<Keymap>,
    pub drawn: Drawn,
    /// Reverse history search (Ctrl+R): its query and where it stands.
    pub searching: Option<(String, SearchStatus)>,
    pub run: Run,
    pub reconnecting: bool,
    /// The page waits on a fork it asked for.
    pub forking: bool,
    /// The server refused the chat for its model provider: a notice stands where the composer
    /// was, and the keys are the notice's.
    pub provider: Option<provider_error::Lock>,
    pub can_continue: bool,
    /// The Windows sandbox setup runs: the composer takes no input and the status row says so
    /// (`show_windows_sandbox_setup_status`).
    pub setting_up: bool,
    /// What the backend estimates the thread cost (`estimated_thread_usage`).
    pub cost: Option<Figure>,
    /// The plan of the account the server is signed in to.
    pub plan: Option<PlanType>,
}

impl Frame {
    pub(super) fn read(state: &ChatState, w: &Chat, cx: &mut Cx<'_, ChatState>) -> Frame {
        let tokens = Tokens::of(cx);
        let thread = read::<ThreadModel, _>(cx);
        let session = watch::<SessionSlice, _>(cx);
        let strings = l10n::of(cx);
        let keymap = crate::keymap::keymap(cx);
        // what the page's shape follows (each build selects anew)
        thread_shape(cx);
        let mut drawn = slice::drawn(cx, thread.id());
        if !w.presentation.focused() {
            let id = thread.id().clone();
            drawn.flags.side_conversation_active =
                select::<FeatureStates, _, _>(cx, move |features| features.side.is_side(&id));
            drawn.facts.side = None;
        }
        let searching = w.ui.draft.search_state();
        // the page follows the runs under way, not the rest of the thread
        let runs = select::<ThreadModel, Vec<RunningHook>, _>(cx, |thread| {
            thread.running_hooks().to_vec()
        });
        let hook = state.hooks.shown(&runs, cx, ChatMsg::HookRevealed);
        let run = Run::of(&thread)
            .asking(super::questions::count(cx, thread.id()))
            .hooked(hook);
        let reconnecting =
            session.connections.get(&drawn.owner) == Some(&ConnectionState::Reconnecting);
        let chat_strings = chat_strings::of(cx);
        let forking = state.forking_of(&thread);
        let provider = thread
            .provider_error()
            .and_then(|error| provider_error::Lock::of(&serde_json::to_value(error).ok()?));
        let can_continue = thread.screen_slots().chat.path.is_some();
        let setting_up =
            select::<FeatureStates, bool, _>(cx, |features| features.windows_sandbox.setting_up());
        let id = thread.id().clone();
        let cost = select::<FeatureStates, Option<Figure>, _>(cx, move |features| {
            features.thread_usage.figure_of(&id)
        });
        let plan =
            select::<ScreenStates, Option<PlanType>, _>(cx, |states| match states.status.account {
                Some(Account::ChatGpt { plan, .. }) => plan,
                Some(Account::ApiKey) | None => None,
            });
        Frame {
            tokens,
            thread,
            session,
            strings,
            chat_strings,
            keymap,
            drawn,
            searching,
            run,
            reconnecting,
            forking,
            provider,
            can_continue,
            setting_up,
            cost,
            plan,
        }
    }

    /// The status line's and the terminal title's inputs.
    pub(super) fn context(&self) -> StatusContext<'_> {
        let slots = self.thread.screen_slots();
        StatusContext {
            thread_id: (!self.thread.id().as_str().is_empty()).then(|| self.thread.id()),
            thread: Some(&slots.chat),
            daybreak_enabled: !self.drawn.flags.side_conversation_active
                && codex_tui_core::features::daybreak_of(self.thread.view()).unwrap_or(false),
            usage: Some(&slots.status),
            rate_limits: self.session.rate_limits.get(&self.drawn.owner),
            run_state: Some(self.run.word(&self.session, self.reconnecting)),
            strings: self.strings,
            catalog: &self.drawn.catalog,
            chat: &self.drawn.facts,
            raw_output: self.drawn.raw_output,
            usage_pending: chat::facts::usage_pending(slots),
            cost: self.cost.as_ref(),
            plan: self.plan,
        }
    }
}

impl ChatState {
    /// What the build tells the cells the key handler and the sparkle read between builds: the
    /// keymap, whether the backend control's chord is live (it is while the control shows), and
    /// what the sparkle needs to tell a command being typed from a message.
    pub(super) fn tell(&self, w: &Chat, frame: &Frame) {
        *self.keymap.borrow_mut() = frame.keymap.clone();
        let drawn = &frame.drawn;
        self.control.set(
            frame
                .session
                .cycle_label(&frame.thread, drawn.offer)
                .is_some()
                && !w.ui.draft.searching(),
        );
        let slots = frame.thread.screen_slots();
        let tiers = chat::facts::tier_commands(
            &drawn.status,
            slots.chat.model.as_deref(),
            &drawn.catalog,
            &drawn.tiers,
        );
        let mut command_flags = drawn.flags;
        command_flags.service_tier_commands_enabled = !tiers.is_empty();
        w.ui.sparkle.know(command_flags, tiers, self.draft_shell);
    }
}

/// The parts of the thread the page's shape follows: a change of one rebuilds it.
fn thread_shape(cx: &mut Cx<'_, ChatState>) {
    select::<ThreadModel, _, _>(cx, |thread| {
        (
            thread.id().clone(),
            thread.turn().clone(),
            thread.can_interrupt(),
            codex_tui_core::threads::composer::composer_state(thread, false),
            thread.transcript().is_empty(),
            thread.queue().len(),
            thread.pending_steers().len(),
            thread.rejected_steers().len(),
            thread.last_error().cloned(),
            thread.screen_slots().chat.clone(),
            thread.screen_slots().status.clone(),
            codex_tui_core::features::daybreak_of(thread.view()),
        )
    });
}
