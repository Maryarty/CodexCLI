//! Opens the screen a command or a key leads to, from the chat: a route
//! pushed over it, a dialog shown over it, or a part of the chat's own
//! subtree on its own page. Each screen's opener is its directory's; the
//! chat only picks which (the trigger table of `parity/chat-nav.md`).

use std::cell::Cell;

use rtwk::Cx;
use rtwk::navigation::{Navigator, PageRoute};

use codex_tui_core::commands::Target;

use crate::screens::{
    agents, approvals, backtrack, dialogs, import, pets, resume, select, settings, status,
    usage, voice,
};

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

/// Opens `target` over the chat of `cx`. The chat stays below, with its
/// transcript and its composer's text; Esc returns to it.
pub fn open<S>(cx: &mut Cx<'_, S>, target: Target) {
    match target {
        Target::Resume => push(cx, resume::route()),
        Target::Agents => push(cx, agents::route()),
        Target::Import => push(cx, import::route()),
        Target::Usage => push(cx, usage::route()),
        // The pager is the expanded mode of the session page's transcript (chat-live);
        // until that lands `/diff` and Ctrl+T open nothing.
        Target::Pager => {}
        Target::Status => drop(status::open(cx)),
        Target::Select => drop(select::open(cx)),
        Target::Settings => drop(settings::open(cx)),
        Target::Dialogs => drop(dialogs::open(cx)),
        Target::Approvals => drop(approvals::open(cx)),
        Target::Pets => drop(pets::open(cx)),
        Target::Voice => drop(voice::open(cx)),
        Target::Backtrack => push(cx, PageRoute::new(|_| backtrack::part())),
    }
    // Recorded once the directory's opener has run: the identity a test
    // reads is what was opened, not what was asked for.
    LAST.with(|last| last.set(Some(target)));
}

fn push<S>(cx: &mut Cx<'_, S>, route: PageRoute<()>) {
    drop(Navigator::of(cx).push::<()>(route));
}
