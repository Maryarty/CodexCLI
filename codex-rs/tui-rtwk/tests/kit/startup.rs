//! The session's start, as a test drives it: the live chat opened with `Open::held_start` has no
//! thread yet (the oracle's chat widget before `thread_id` is set); `start_thread` is the server's
//! answer to its `thread/start`, the moment the session is configured.

use codex_tui_core::app::{AppEv, Command};
use codex_tui_core::ids::ThreadId;
use codex_tui_core::rpc::Reply;
use codex_tui_core::screens::{ScreenCommand, ScreenEv, chat};

use super::Screen;

impl Screen {
    /// The server answers the held `thread/start`: the session starts, what waited for it goes
    /// out (`handle_thread_session`, `maybe_send_next_queued_input`).
    pub fn start_thread(&mut self) -> &mut Screen {
        let Command::StartThread { ticket, .. } = self.thread_start_request() else {
            unreachable!("expected thread/start");
        };
        let epoch = self.h.fake.epoch().expect("attached");
        self.h.fake.inject(
            epoch,
            AppEv::Reply(ticket, Reply::thread_started(ThreadId::new("thread-1"))),
        );
        self.h.advance(100);
        self
    }

    /// The first-launch onboarding (`Open::first_login`) is done: the host answers the startup
    /// facts, the sign-in and trust questions have nothing to ask, and the launch starts the
    /// session (its `thread/start` is held by `held_start`).
    pub fn finish_onboarding(&mut self) -> &mut Screen {
        self.h.finish_startup_without_prompts();
        self.h.advance(100);
        self
    }

    /// The server answers the held `thread/start` with its full JSON response.
    pub fn start_thread_with(&mut self, response: serde_json::Value) -> &mut Screen {
        self.answer_request(&self.thread_start_request(), response);
        self
    }

    fn thread_start_request(&self) -> Command {
        self.h
            .fake
            .commands()
            .into_iter()
            .find(|command| matches!(command, Command::StartThread { .. }))
            .expect("the app asked for a thread: open the screen with `held_start()`")
    }

    /// The thread's settings name `model` (`thread/settings/updated`): the empty name is the
    /// oracle's `set_model("")`, a thread with no model to send with.
    pub fn set_thread_model(&mut self, model: &str) -> &mut Screen {
        let step = serde_json::json!({"step": "notify", "message": {
            "method": "thread/settings/updated",
            "params": {"threadId": "thread-1", "threadSettings": {
                "cwd": "/tmp/project", "approvalPolicy": "on-request", "approvalsReviewer": "user",
                "sandboxPolicy": {"type": "dangerFullAccess"}, "activePermissionProfile": null,
                "model": model, "modelProvider": "openai", "serviceTier": null, "effort": null,
                "summary": null, "personality": null,
                "collaborationMode": {"mode": "default", "settings": {
                    "model": model, "reasoning_effort": null, "developer_instructions": null}}
            }}
        }});
        self.feed(&step.to_string())
    }

    /// The `!cmd` lines the app asked the screen to run as the user's own shell commands
    /// (`submit_shell_command`), oldest first, without the `!`.
    pub fn shell_commands(&self) -> Vec<String> {
        self.h
            .fake
            .commands()
            .into_iter()
            .filter_map(|c| match c {
                Command::Screen {
                    command: ScreenCommand::Chat(chat::Command::Shell(command)),
                    ..
                } => Some(command),
                _ => None,
            })
            .collect()
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
