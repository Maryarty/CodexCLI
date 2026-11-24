//! Opens the screen a command or a key leads to, from the chat: a route
//! pushed over it, a dialog shown over it, or a part of the chat's own
//! subtree on its own page. Each screen's opener is its directory's; the
//! chat only picks which (the trigger table of `parity/chat-nav.md`).

use std::cell::Cell;

use rtwk::Cx;
use rtwk::navigation::{Navigator, PageRoute};

use codex_tui_core::commands::Target;

use crate::pages::session::{backtrack, transcript};
use crate::pages::startup::import;
use crate::pages::{agents, resume, usage};
use crate::sheets::{approvals, dialogs, pets, voice};

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

/// Opens `target` over the chat of `cx`: the registry of the targets, one line each, an opener
/// that is the owning slice's (`ui_registry` rules: no arm of this match has a body). Only the
/// pages are routes (`Opens::Page`: Agents, Resume, Usage, the import step of Startup); a sheet
/// or dialog is shown over the page that asks, a mode belongs to the session page. The chat
/// stays below, with its transcript and its composer's text; Esc returns to it. A feature's own
/// sheet is opened by its UI half with typed arguments (`features::open`), the select and
/// settings sheets by the presenter for the show the model made.
pub fn open<S: 'static>(cx: &mut Cx<'_, S>, target: Target) {
    match target {
        Target::Resume => push(cx, resume::route()),
        Target::Agents => push(cx, agents::route()),
        Target::Import => push(cx, import::route()),
        Target::Usage => push(cx, usage::route()),
        Target::Dialogs => drop(dialogs::open(cx)),
        Target::Approvals => drop(approvals::open(cx)),
        Target::Pets => drop(pets::open(cx)),
        Target::Voice => drop(voice::open(cx)),
        Target::AgentPicker => agents::open_picker(cx),
        Target::Pager => transcript::pager::open(cx),
        Target::Backtrack => push(cx, PageRoute::new(|_| backtrack::part())),
        Target::Select | Target::Settings | Target::Feature(_) => {}
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
