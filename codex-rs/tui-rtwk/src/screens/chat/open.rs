//! Opens the screen a command or a key leads to, from the chat: a route
//! pushed over it, a dialog shown over it, or a part of the chat's own
//! subtree on its own page. Each screen's opener is its directory's; the
//! chat only picks which (the trigger table of `parity/chat-nav.md`).

use rtwk::Cx;
use rtwk::navigation::{Navigator, PageRoute};

use codex_tui_rtwk_core::commands::Target;

use crate::screens::{
    agents, approvals, backtrack, dialogs, import, pager, pets, resume, select, settings, status,
    usage, voice,
};

/// Opens `target` over the chat of `cx`. The chat stays below, with its
/// transcript and its composer's text; Esc returns to it.
pub fn open<S>(cx: &mut Cx<'_, S>, target: Target) {
    match target {
        Target::Resume => push(cx, resume::route()),
        Target::Agents => push(cx, agents::route()),
        Target::Import => push(cx, import::route()),
        Target::Usage => push(cx, usage::route()),
        Target::Pager => push(cx, pager::route()),
        Target::Status => drop(status::open(cx)),
        Target::Select => drop(select::open(cx)),
        Target::Settings => drop(settings::open(cx)),
        Target::Dialogs => drop(dialogs::open(cx)),
        Target::Approvals => drop(approvals::open(cx)),
        Target::Pets => push(cx, PageRoute::new(|_| pets::part())),
        Target::Voice => push(cx, PageRoute::new(|_| voice::part())),
        Target::Backtrack => push(cx, PageRoute::new(|_| backtrack::part())),
    }
}

fn push<S>(cx: &mut Cx<'_, S>, route: PageRoute<()>) {
    drop(Navigator::of(cx).push::<()>(route));
}
