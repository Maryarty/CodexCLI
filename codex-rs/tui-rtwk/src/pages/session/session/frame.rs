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

use crate::app::theme::Tokens;
use crate::l10n::strings::{self as l10n, Shared};
use crate::pages::session::activity::Run;
use crate::pages::session::live::provider_error;
use crate::pages::session::slice::{self, Drawn};
use crate::pages::session::status_surfaces::StatusContext;
use crate::pages::session::strings::{self as chat_strings, Strings};

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
        let drawn = slice::drawn(cx, thread.id());
        let searching = w.ui.draft.search_state();
        let run = Run::of(&thread);
        let reconnecting =
            session.connections.get(&drawn.owner) == Some(&ConnectionState::Reconnecting);
        let chat_strings = chat_strings::of(cx);
        let forking = state.forking_of(&thread);
        let provider = thread
            .provider_error()
            .and_then(|error| provider_error::Lock::of(&serde_json::to_value(error).ok()?));
        let can_continue = thread.screen_slots().chat.path.is_some();
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
            cost,
            plan,
        }
    }

    /// The status line's and the terminal title's inputs.
    pub(super) fn context(&self) -> StatusContext<'_> {
        let slots = self.thread.screen_slots();
        StatusContext {
            thread_id: self.session.active.as_ref(),
            thread: Some(&slots.chat),
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
            thread.read_only(),
            thread.transcript().is_empty(),
            thread.queue().len(),
            thread.pending_steers().len(),
            thread.rejected_steers().len(),
            thread.last_error().cloned(),
            thread.screen_slots().chat.clone(),
            thread.screen_slots().status.clone(),
        )
    });
}
