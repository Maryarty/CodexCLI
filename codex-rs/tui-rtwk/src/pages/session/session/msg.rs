//! The messages of the chat page's state (`ChatState::update`): what its keys, its footer's hints,
//! its listener's edges and its composer's notifications say to it.

use std::rc::Rc;

use crate::pages::session::banners::BannerId;
use crate::pages::session::keys::Action;
use crate::pages::session::live::browse::Browse;
use crate::pages::session::transcript::selection::Key as SelectionKey;
use crate::widgets::directives::Edge;

#[derive(Clone, Debug, PartialEq)]
pub enum ChatMsg {
    /// A row of the banner shown was chosen (its number key).
    BannerPick(BannerId, usize),
    /// Esc dismissed the banner shown.
    BannerDismiss(BannerId),
    /// `change_host`: the empty conversation moves to the next backend.
    Cycle,
    Submit(Rc<str>),
    QueueDraft,
    /// The edit-queued key: the newest queued message goes back to the composer (over questions
    /// that wait, it opens their editor).
    EditQueued,
    /// What the user decided in the editor of the model's questions.
    Question(crate::sheets::approvals::questions::Outcome),
    /// The reasoning shortcuts: a step down or up (`handle_reasoning_shortcut`).
    Reasoning(codex_tui_core::screens::select::reasoning::Direction),
    CtrlC,
    /// A key of the shortcut menu of `/presets` (`Key::Open`: the command ran).
    Presets(crate::sheets::select::hud::Step),
    RetryWriter,
    /// The provider-error notice's keys: `c` continues in a new chat, `n` starts
    /// a fresh one, Esc (and the third hint) opens the chats to resume.
    ProviderContinue,
    ProviderNew,
    ProviderOthers,
    ForkWriter,
    QuitReadOnly,
    CtrlD,
    /// The hidden key: the debug menu over the chat.
    DebugMenu,
    /// A key action of the chat (`keys::defaults`).
    Action(Action),
    /// PageUp (-1) and PageDown (1): the transcript a page.
    Page(i8),
    /// Ctrl+Home, Alt+<: the transcript's first row; Ctrl+End, Alt+>: its last (`JumpTarget`).
    Beginning,
    Latest,
    /// A key of the mode that browses the prompts.
    Browse(Browse),
    /// A key that is the transcript selection's (copy, copy and follow, clear).
    Selection(SelectionKey),
    /// A suggestion was put in: a command is run, the others are text.
    Picked(Rc<str>),
    Escape,
    /// `?`: the shortcut overlay (the footer's hint).
    Shortcuts,
    /// `←` on an empty composer: the agents page (the footer's hint).
    Agents,
    /// The composer's text changed (the field's own notification).
    Changed(Rc<str>),
    /// One of the model's one-shots the listener told (`widgets/directives.rs`).
    Edge(Edge),
    /// The reveal delay of a hook run passed: the status row may name it now.
    HookRevealed,
}
