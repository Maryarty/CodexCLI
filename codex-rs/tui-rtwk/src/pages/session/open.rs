//! Opens the screen a command or a key leads to, from the chat: a route
//! pushed over it, a dialog shown over it, or a part of the chat's own
//! subtree on its own page. Each screen's opener is its directory's; the
//! chat only picks which (the trigger table of `parity/chat-nav.md`).

use std::cell::Cell;

use rtwk::Cx;
use rtwk::navigation::{Navigator, PageRoute};

use codex_tui_core::commands::{Opens, Target};

use crate::pages::session::backtrack;
use crate::pages::startup::import;
use crate::pages::{agents, resume, usage};
use crate::sheets::{approvals, dialogs, pets, select, settings, voice};

thread_local! {
    /// The target the chat opened last on this thread: the identity a test
    /// checks, whatever the target's screen looks like (stub or real).
    static LAST: Cell<Option<Target>> = const { Cell::new(None) };
}

/// The target the chat opened last on this thread, if any (for tests).
#[doc(hidden)]
pub fn last_opened() -> Option<Target> {
    LAST.with(Cell::get)
}

/// Forgets the last opened target (for tests).
#[doc(hidden)]
pub fn reset_last_opened() {
    LAST.with(|last| last.set(None));
}

/// Records that a sheet of `target` was opened by something other than `open` (a feature's own
/// sheet): the identity a test reads.
pub fn note_opened(target: Target) {
    LAST.with(|last| last.set(Some(target)));
}

/// Opens `target` over the chat of `cx`. Only the pages are routes
/// (`Opens::Page`: Agents, Resume, Usage, the import step of Startup); a sheet
/// or dialog is shown over the page that asks, a mode belongs to the session
/// page. The chat stays below, with its transcript and its composer's text;
/// Esc returns to it.
pub fn open<S>(cx: &mut Cx<'_, S>, target: Target) {
    match target.opens() {
        Opens::Page => match target {
            Target::Resume => push(cx, resume::route()),
            Target::Agents => push(cx, agents::route()),
            Target::Import => push(cx, import::route()),
            _ => push(cx, usage::route()),
        },
        Opens::Sheet | Opens::Dialog => match target {
            Target::Select => drop(select::open(cx)),
            Target::Settings => drop(settings::open(cx)),
            Target::Dialogs => drop(dialogs::open(cx)),
            // The picker of this session's subagents is not built: the sheet
            // says so (oracle: `AppEvent::OpenAgentPicker`).
            Target::AgentPicker => {
                let action = crate::l10n::strings::of(cx).pages.action_subagents;
                drop(crate::widgets::page_keys::not_yet(cx, action));
            }
            Target::Approvals => drop(approvals::open(cx)),
            Target::Pets => drop(pets::open(cx)),
            // Opened by its own UI half with typed arguments (`features::open`).
            Target::Feature(_) => {}
            _ => drop(voice::open(cx)),
        },
        Opens::Mode => match target {
            // The pager is the expanded mode of the session page's transcript
            // (chat-live); until that lands `/diff` and Ctrl+T say so in a popup.
            Target::Pager => {
                let action = crate::l10n::strings::of(cx).pages.action_transcript;
                drop(crate::widgets::page_keys::not_yet(cx, action));
            }
            // Backtrack becomes a mode of the expanded transcript with chat-live.
            _ => push(cx, PageRoute::new(|_| backtrack::part())),
        },
    }
    // Recorded once the directory's opener has run: the identity a test
    // reads is what was opened, not what was asked for.
    LAST.with(|last| last.set(Some(target)));
}

/// F2 and `/warnings`: the warnings viewer over the chat (the viewer reads the warnings the
/// session retains itself; nothing is asked of the model).
pub fn open_warnings<S>(cx: &mut Cx<'_, S>) {
    drop(dialogs::open_warnings(cx));
    LAST.with(|last| last.set(Some(Target::Dialogs)));
}

fn push<S>(cx: &mut Cx<'_, S>, route: PageRoute<()>) {
    drop(Navigator::of(cx).push::<()>(route));
}
