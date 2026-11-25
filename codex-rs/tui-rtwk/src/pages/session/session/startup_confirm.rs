//! The startup draft's confirmed submission (oracle `startup_draft_input.rs`, `chatwidget/
//! startup_submission.rs`): Enter on the draft typed while the session starts confirms it. The
//! text stays in the composer, the footer says `Waiting for startup · esc cancel`, an edit or Esc
//! cancels, and the draft is submitted once when the session is ready. The draft is the page's
//! (ADR-003): nothing reaches the thread until then.
//!
//! No request and no question exists before the first thread, so the oracle's other protected
//! views (`startup_submission_has_protected_input`) reduce to a route over the page: the
//! submission waits for the page to be in front again (`Edge::Front`).

use std::rc::Rc;

use rtwk::navigation::Route;
use rtwk::provider::read;
use rtwk::{Cx, Rebuild};

use codex_tui_core::app::{Session, SessionSlice};
use codex_tui_core::threads::ThreadModel;
use codex_tui_core::threads::startup;

use crate::app::view::Hs;
use crate::pages::session::actions::run_line;
use crate::pages::session::composer::queue::Delivery;
use crate::pages::session::composer::submission;

use super::{Chat, ChatState};

/// The draft the user confirmed: what it would submit and whether it is in shell mode.
pub(super) struct Confirmed {
    text: Rc<str>,
    shell: bool,
}

impl Confirmed {
    /// The draft is still the one confirmed (a cursor move leaves it so, an edit does not).
    fn holds(&self, w: &Chat) -> bool {
        *submission(&w.ui) == *self.text && w.ui.draft.shell() == self.shell
    }
}

impl ChatState {
    /// Enter on the draft while the session starts: confirmed when it is text for the model
    /// (`is_startup_submit_key`); a command is run at once and takes the other path.
    pub(super) fn confirm_startup(
        &mut self,
        text: &str,
        w: &Chat,
        cx: &mut Cx<'_, ChatState>,
    ) -> bool {
        if read::<SessionSlice, _>(cx).status != Session::Starting || !startup::confirms(text) {
            return false;
        }
        self.startup_confirm = Some(Confirmed {
            text: Rc::from(text),
            shell: w.ui.draft.shell(),
        });
        true
    }

    /// The footer says the confirmed draft waits (only while the session starts).
    pub(super) fn startup_waiting(&self, session: &SessionSlice) -> bool {
        self.startup_confirm.is_some() && session.status == Session::Starting
    }

    /// An edit of the draft cancels the confirmation: only the intent goes, the text stays.
    pub(super) fn startup_edited(&mut self, w: &Chat) {
        if self.startup_confirm.as_ref().is_some_and(|c| !c.holds(w)) {
            self.startup_confirm = None;
        }
    }

    /// The session may be ready (`Edge::Ready`, or the page in front again): the confirmed
    /// draft is submitted once, through the line a typed Enter takes. A draft that changed in
    /// the meantime is dropped silently; one in shell mode stays for a second Enter (the startup
    /// editor cannot show shell mode, `prepare_startup_submission`).
    pub(super) fn startup_ready(&mut self, w: &Chat, cx: &mut Cx<'_, ChatState>) -> Rebuild {
        if self.startup_confirm.is_none() {
            return Rebuild::No;
        }
        let session = read::<SessionSlice, _>(cx);
        if matches!(session.status, Session::Failed(_)) {
            self.startup_confirm = None;
            return Rebuild::Yes;
        }
        if !startup::confirm_ready(&session, &read::<ThreadModel, _>(cx))
            || !Route::is_current_of(cx)
        {
            return Rebuild::No;
        }
        let Some(confirmed) = self.startup_confirm.take() else {
            return Rebuild::No;
        };
        if confirmed.holds(w) && !confirmed.shell {
            let hs = read::<Hs, _>(cx);
            run_line(&confirmed.text, w, cx, &hs, Delivery::Submit);
            self.sync_draft(w);
        }
        Rebuild::Yes
    }
}
