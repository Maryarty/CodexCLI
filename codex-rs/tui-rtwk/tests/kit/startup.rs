//! The session's start, as a test drives it: the live chat opened with `Open::held_start` has no
//! thread yet (the oracle's chat widget before `thread_id` is set); `start_thread` is the server's
//! answer to its `thread/start`, the moment the session is configured.

use codex_tui_core::app::{AppEv, Command};
use codex_tui_core::ids::ThreadId;
use codex_tui_core::rpc::Reply;
use codex_tui_core::screens::{ScreenEv, chat};

use super::Screen;

impl Screen {
    /// The server answers the held `thread/start`: the session starts, what waited for it goes
    /// out (`handle_thread_session`, `maybe_send_next_queued_input`).
    pub fn start_thread(&mut self) -> &mut Screen {
        let ticket = self
            .h
            .fake
            .commands()
            .into_iter()
            .find_map(|c| match c {
                Command::StartThread { ticket, .. } => Some(ticket),
                _ => None,
            })
            .expect("the app asked for a thread: open the screen with `held_start()`");
        let epoch = self.h.fake.epoch().expect("attached");
        self.h.fake.inject(
            epoch,
            AppEv::Reply(ticket, Reply::thread_started(ThreadId::new("thread-1"))),
        );
        self.h.advance(100);
        self
    }
}

impl Screen {
    /// The host has read the config: its directory is the one the startup header names while the
    /// session is still starting (`StartupDraftPump::apply_settings`, `StatusFacts::cwd`).
    pub fn configured_cwd(&mut self, cwd: &str) -> &mut Screen {
        self.h
            .inject([AppEv::Screen(ScreenEv::Chat(chat::Ev::Status(Box::new(
                chat::StatusFacts {
                    cwd: Some(cwd.to_string()),
                    ..chat::StatusFacts::default()
                },
            ))))]);
        self.h.advance(100);
        self
    }
}
