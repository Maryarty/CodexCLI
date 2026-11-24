//! The keys of a chat that is locked: a thread another process writes (retry, fork, quit) and a
//! chat the server refused for its model provider (continue in a new chat, start a fresh one,
//! open the chats to resume).

use rtwk::Cx;
use rtwk::provider::read;

use codex_tui_core::commands::{SessionIntent, SlashCommand, Target};
use codex_tui_core::screens::{ScreenIntent, chat};
use codex_tui_core::threads::ThreadModel;

use crate::app::view::Hs;
use crate::pages::session::open::open;
use crate::pages::session::strings as chat_strings;
use crate::widgets::thread_ui::Modes;

use super::{ChatState, Forking};

/// The writer's thread, opened again (`RetryWriter`).
pub(super) fn retry(cx: &mut Cx<'_, ChatState>, hs: &Hs) {
    let thread = read::<ThreadModel, _>(cx);
    if thread.read_only() {
        cx.emit(&hs.open_thread, thread.id().clone());
    }
}

/// `ContinueProviderChat`: only the locked chat on screen, and only one that was saved
/// (`rollout_path`).
pub(super) fn provider_continue(cx: &mut Cx<'_, ChatState>, hs: &Hs) {
    let thread = read::<ThreadModel, _>(cx);
    let slots = thread.screen_slots();
    if thread.provider_error().is_some()
        && let Some(path) = &slots.chat.path
    {
        let message =
            chat_strings::of(cx).provider_continue_message(slots.chat.name.as_deref(), path);
        cx.emit(
            &hs.screen,
            ScreenIntent::from(chat::Intent::Session {
                thread: Some(thread.id().clone()),
                command: SlashCommand::New,
                intent: SessionIntent::ContinueChat { message },
            }),
        );
    }
}

/// The locked chat's third key: a fresh chat.
pub(super) fn provider_new(cx: &mut Cx<'_, ChatState>, hs: &Hs) {
    let thread = read::<ThreadModel, _>(cx);
    if thread.provider_error().is_some() {
        cx.emit(
            &hs.screen,
            ScreenIntent::from(chat::Intent::Session {
                thread: Some(thread.id().clone()),
                command: SlashCommand::New,
                intent: SessionIntent::New { name: None },
            }),
        );
    }
}

/// The locked chat's Esc (and the third hint) opens the chats to resume; true when it did.
pub(super) fn provider_others(modes: &Modes, cx: &mut Cx<'_, ChatState>) -> bool {
    if read::<ThreadModel, _>(cx).provider_error().is_some() {
        modes.close();
        open(cx, Target::Resume);
        return true;
    }
    false
}

impl ChatState {
    /// The writer's thread forked into a copy; true when the page now waits on the fork.
    pub(super) fn fork_writer(&mut self, cx: &mut Cx<'_, ChatState>, hs: &Hs) -> bool {
        let thread = read::<ThreadModel, _>(cx);
        if thread.read_only() {
            let intent = codex_tui_core::screens::agents::Intent::Fork(thread.id().to_string());
            cx.emit(&hs.screen, ScreenIntent::from(intent));
            self.forking = Some(Forking {
                thread: thread.id().clone(),
            });
            return true;
        }
        false
    }
}
