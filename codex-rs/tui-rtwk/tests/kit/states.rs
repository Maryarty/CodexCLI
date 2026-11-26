//! States the oracle sets by a widget's own setter, reached here by the route the app takes: a
//! composer whose input is off, an image of a size, a thread with no model, the composer alone.

use codex_tui_core::app::{AppEv, BackendEv, Command, Intent};
use codex_tui_core::features::windows_sandbox::{Config, Fact, Host, Intent as Sandbox, Mode};
use codex_tui_core::features::{FeatureFact, FeatureIntent};
use codex_tui_core::fixtures::builders::turn_started;
use codex_tui_core::fixtures::script::Answer;
use codex_tui_core::ids::ThreadId;
use codex_tui_core::rpc::{Reply, ServerError};
use codex_tui_core::screens::{ScreenCommand, ScreenEv, agents, chat};
use codex_tui_rtwk::app::UiApp;

use super::Screen;
use super::env::{listed_thread, replay_turn, resume_response};
use super::seed::{Open, answer};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SubmissionReceipt {
    Legacy,
    Unrelated,
    Exact,
}

impl Screen {
    /// The composer's input is off, as the oracle's `set_input_enabled(false, ..)` leaves it: the
    /// Windows sandbox setup runs and has no end yet (`Input disabled until setup completes.`,
    /// the draft stays, no key is taken). The live chat only.
    pub fn input_disabled(&mut self) -> &mut Screen {
        assert!(
            !self.entry,
            "the setup runs in the live app, not in an entry's model"
        );
        answer(
            &mut self.h,
            "windowsSandbox/setupStart".to_string(),
            Answer::Result(serde_json::json!({"started": true})),
        );
        let epoch = self.h.fake.epoch().expect("attached");
        let fact = Fact::Environment {
            config: Config {
                mxc: false,
                mode: None,
                requirements: Some(None),
            },
            host: Host::Local,
            local_server: true,
            elevated_ready: false,
        };
        self.h
            .fake
            .inject(epoch, AppEv::Feature(FeatureFact::from(fact)));
        let begin = FeatureIntent::from(Sandbox::Begin(Mode::Elevated));
        self.h
            .fake
            .inject(epoch, AppEv::Intent(Intent::Feature(begin)));
        self.h.advance(200);
        self
    }

    /// Attaches a PNG of `width` x `height` pixels at the caret (`attach_image` with the size a
    /// thumbnail is laid out for): the file is a solid image `name` in a directory of this test
    /// process. Returns its path.
    pub fn attach_image_sized(
        &mut self,
        name: &str,
        width: u32,
        height: u32,
    ) -> std::path::PathBuf {
        let path = super::image_file_sized(name, width, height);
        self.paste(&path.to_string_lossy());
        self.keys("backspace");
        path
    }
}

impl Screen {
    /// The thread the chat shows.
    fn active_thread(&self) -> ThreadId {
        self.h
            .t
            .read_app(|app: &UiApp| app.core.session().active.clone())
            .expect("the session has a thread")
    }

    /// The event stream of the thread is lost while the connection stands (a Cloud backend's
    /// `ThreadStreamDisconnected`, the oracle's reconnect tests): what the user sends waits in
    /// the queue, `error` is the server's words. The live chat only.
    pub fn stream_lost(&mut self, error: &str) -> &mut Screen {
        assert!(!self.entry, "a stream belongs to the live app");
        let thread = self.active_thread();
        let notification = serde_json::from_value(serde_json::json!({
            "error": {"message": error, "codexErrorInfo": null, "additionalDetails": null},
            "willRetry": false,
            "threadId": thread.as_str(),
            "turnId": "",
        }))
        .expect("an error notification");
        self.h.inject([AppEv::Backend(BackendEv::StreamLost {
            thread,
            error: Box::new(notification),
        })]);
        self.h.advance(100);
        self
    }

    /// Pause the live connection like the remote host does before reconnecting.
    pub fn reconnecting(&mut self) -> &mut Screen {
        self.h
            .inject([AppEv::Screen(ScreenEv::Agents(agents::Ev::Reconnecting))]);
        self.h.advance(100);
        self
    }

    /// The displayed thread loses its stream and the subsequent resume hits an active writer.
    pub fn becomes_foreign_writer(&mut self) -> &mut Screen {
        self.reopen_with_refusal("thread already has an active writer")
    }

    /// A stream that cannot be reattached leaves the cached thread unavailable.
    pub fn becomes_unavailable(&mut self) -> &mut Screen {
        self.h.inject([AppEv::Screen(ScreenEv::Status(
            codex_tui_core::screens::status::Ev::Connection(Some(
                codex_tui_core::screens::status::Remote {
                    address: "ws://devbox.example.com:4500/".into(),
                    version: "unknown".into(),
                    local_daemon: false,
                },
            )),
        ))]);
        self.refuse_resume("conversation no longer available".into());
        self.reconnecting();
        self.h
            .inject([AppEv::Screen(ScreenEv::Agents(agents::Ev::Reconnected))]);
        self.h.advance(100);
        self
    }

    /// The backend now attaches the displayed thread again.
    pub fn restore_writer(&mut self) -> &mut Screen {
        let thread = self.active_thread();
        self.h.fake.respond(|request| {
            let id = request["params"]["threadId"].as_str()?;
            (request["method"] == "thread/resume").then(|| Ok(resume_response(id, &[], false)))
        });
        self.h.inject([AppEv::Intent(Intent::OpenThread(thread))]);
        self.h.advance(100);
        self
    }

    fn reopen_with_refusal(&mut self, error: impl Into<ServerError>) -> &mut Screen {
        let thread = self.active_thread();
        self.refuse_resume(error.into());
        self.stream_lost("Cloud stream ended. Reopen the conversation to reconnect.");
        self.h.inject([AppEv::Intent(Intent::OpenThread(thread))]);
        self.h.advance(100);
        self
    }

    fn refuse_resume(&mut self, error: ServerError) {
        let original = self.active_thread();
        self.h.fake.respond(move |request| {
            if request["method"] != "thread/resume" {
                return None;
            }
            let id = request["params"]["threadId"].as_str()?;
            Some(if id == original.as_str() {
                Err(error.clone())
            } else {
                Ok(resume_response(id, &[], false))
            })
        });
    }

    pub fn provider_refusal(&mut self, error: &str, remote: bool) -> &mut Screen {
        let mut facts = self
            .h
            .t
            .read_app(|app: &UiApp| (*app.core.screens().chat.status).clone());
        facts.provider_names.extend([
            chat::ModelProviderName {
                id: "custom-gateway".to_owned(),
                name: "Team Gateway".to_owned(),
            },
            chat::ModelProviderName {
                id: "old-provider".to_owned(),
                name: "  ".to_owned(),
            },
        ]);
        self.h.inject([
            AppEv::Screen(ScreenEv::Chat(chat::Ev::Status(Box::new(facts)))),
            AppEv::Screen(ScreenEv::Status(
                codex_tui_core::screens::status::Ev::Connection(remote.then(|| {
                    codex_tui_core::screens::status::Remote {
                        address: "ws://devbox.example.com:4500/".into(),
                        version: "unknown".into(),
                        local_daemon: false,
                    }
                })),
            )),
        ]);
        self.reopen_with_refusal(ServerError {
            code: Some(-32000),
            message: "provider refused resume".to_owned(),
            data: Some(serde_json::from_str(error).expect("typed provider error")),
        })
    }

    /// Open another task through the same intent a row in the command center uses.
    pub fn visit_thread(&mut self, id: &str) -> &mut Screen {
        self.show_task_list(&[id]);
        self.h
            .inject([AppEv::Intent(Intent::OpenThread(ThreadId::new(id)))]);
        self.h.advance(100);
        self
    }

    /// Supply the normal resume response for a listed thread.
    pub fn server_resumes(&mut self, id: &str) -> &mut Screen {
        self.server_answers("thread/resume", resume_response(id, &[], false))
    }

    pub fn assert_active_thread(&self, id: &str) {
        assert_eq!(self.active_thread().as_str(), id);
    }

    pub fn assert_started_turn_on(&self, id: &str) {
        assert!(self.commands().iter().any(|command| matches!(
            command,
            Command::StartTurn { thread, .. } if thread.as_str() == id
        )));
    }

    pub fn assert_thread_model_patch(&self, id: &str, model: &str) {
        let requests = self.request_params("thread/settings/update");
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0]["threadId"], id);
        assert_eq!(requests[0]["model"], model);
    }

    pub fn show_task_list(&mut self, threads: &[&str]) -> &mut Screen {
        self.h
            .inject([AppEv::Screen(ScreenEv::Agents(agents::Ev::Listed(
                agents::Page {
                    threads: threads.iter().map(|id| listed_thread(id)).collect(),
                    unread: Vec::new(),
                    next_cursor: None,
                    append: false,
                    pins: None,
                },
            )))]);
        self.h.advance(100);
        self
    }

    pub fn assert_thread_unavailable(&self) {
        use codex_tui_core::threads::composer::{ComposerState, composer_state};
        let state = self
            .h
            .t
            .read_app(|app: &UiApp| composer_state(&app.core.active_thread(), false));
        assert_eq!(state, ComposerState::Unavailable);
    }

    /// The large paste is still represented by a single placeholder in the editor.
    pub fn assert_saved_paste(&self) {
        let draft = self.h.t.read_app(|app: &UiApp| {
            app.ui
                .thread_ui
                .get(app.core.session().active.as_ref())
                .composer
                .text()
                .to_string()
        });
        assert!(
            draft.starts_with("[Pasted Content"),
            "uncompressed draft of {} bytes",
            draft.len()
        );
    }

    /// The thread's composer controller retains the draft even when the editor is hidden.
    pub fn assert_saved_draft(&self, expected: &str) {
        let draft = self.h.t.read_app(|app: &UiApp| {
            app.ui
                .thread_ui
                .get(app.core.session().active.as_ref())
                .composer
                .text()
                .to_string()
        });
        assert_eq!(draft, expected);
    }

    /// The quit request made it through the live widget to the terminal frame loop.
    pub fn assert_quitting(&self, quitting: bool) {
        assert_eq!(!self.h.t.is_running(), quitting);
    }

    /// The event stream of the thread is back (`ThreadStreamReconnected`).
    pub fn stream_restored(&mut self) -> &mut Screen {
        assert!(!self.entry, "a stream belongs to the live app");
        let thread = self.active_thread();
        self.h
            .inject([AppEv::Backend(BackendEv::StreamRestored(thread))]);
        self.h.advance(100);
        self
    }

    /// Prepare a start or steer whose answer the server has yet to deliver.
    pub fn hold_reconnect_submission(&mut self, pending_start: bool) -> &mut Screen {
        if pending_start {
            self.h.fake.hold_turn_start();
        } else {
            let thread = self.active_thread();
            self.h
                .inject([AppEv::Backend(turn_started(thread.as_str(), "turn-1"))]);
            self.h.advance(100);
        }
        self
    }

    /// Disconnect and reattach the visible thread, with a recovered prompt that carries
    /// the specified ID. The recovered text matches the submitted text in every case.
    pub fn recover_submission(&mut self, receipt: SubmissionReceipt, text: &str) -> &mut Screen {
        let thread = self.active_thread();
        let client_id = self
            .h
            .fake
            .commands()
            .into_iter()
            .rev()
            .find_map(|command| match command {
                Command::StartTurn { client_id, .. }
                | Command::Screen {
                    command: ScreenCommand::Chat(chat::Command::Steer { client_id, .. }),
                    ..
                } => Some(client_id),
                _ => None,
            })
            .expect("a submission in flight");
        let recovered_id = match receipt {
            SubmissionReceipt::Legacy => None,
            SubmissionReceipt::Unrelated => Some("unrelated-submission".to_owned()),
            SubmissionReceipt::Exact => Some(client_id),
        };
        let turn = replay_turn(
            "turn-1",
            serde_json::json!([{
                "type": "userMessage",
                "id": "recovered",
                "clientId": recovered_id,
                "content": [{"type": "text", "text": text, "textElements": []}]
            }]),
        );
        self.h
            .fake
            .respond(move |request| match request["method"].as_str() {
                Some("thread/resume") => Some(Ok(resume_response(thread.as_str(), &[], false))),
                _ => None,
            });
        self.h.inject([
            AppEv::Screen(ScreenEv::Agents(agents::Ev::Reconnecting)),
            AppEv::Screen(ScreenEv::Agents(agents::Ev::Reconnected)),
        ]);
        self.history_page(vec![turn], None)
    }

    pub fn assert_reconnect_queue(&self, expected: &[&str]) {
        let queued = self.h.t.read_app(|app: &UiApp| {
            app.core
                .active_thread()
                .queue()
                .iter()
                .map(|entry| entry.text.clone())
                .collect::<Vec<_>>()
        });
        assert_eq!(queued, expected, "the recovered input queue");
    }

    /// The thread is controlled by its parent (the oracle's `set_parent_owned_thread`): the server
    /// says it takes no direct input (`canAcceptDirectInput: false` in its `thread/started`). The
    /// live chat only.
    pub fn parent_owned(&mut self) -> &mut Screen {
        assert!(!self.entry, "a thread belongs to the live app");
        let thread = self.active_thread();
        let notification = serde_json::json!({
            "step": "notify",
            "message": {"method": "thread/started", "params": {"thread": {
                "id": thread.as_str(), "environments": null, "extra": null,
                "sessionId": thread.as_str(), "forkedFromId": null, "attachedToThreadId": null,
                "parentThreadId": null, "preview": "", "ephemeral": false, "section": null,
                "sectionEnteredAt": null, "projectId": null, "historyMode": "legacy",
                "modelProvider": "openai", "model": null, "reasoningEffort": null,
                "createdAt": 0, "updatedAt": 0, "recencyAt": 0, "status": {"type": "idle"},
                "path": null, "cwd": "/tmp/project", "cliVersion": "0.0.0", "originator": null,
                "source": "cli", "canAcceptDirectInput": false, "threadSource": null,
                "agentNickname": null, "agentRole": null, "gitInfo": null, "name": null,
                "daybreakEnabled": null, "turns": []
            }}}
        });
        self.feed(&notification.to_string());
        self.h.advance(100);
        self
    }

    /// The images of what the user sends are still being prepared: the turn's start is held in
    /// flight (the host prepares them before it sends), so the composer is already free and the
    /// message is not on the server. `fail_image_preparation` ends the wait.
    pub fn hold_image_preparation(&mut self) -> &mut Screen {
        self.h.fake.hold_turn_start();
        self
    }

    /// The preparation of the images of the turn in flight ends in `message` (`Failed to
    /// prepare image: ..`): the message never left, so it goes back whole to the composer.
    pub fn fail_image_preparation(&mut self, message: &str) -> &mut Screen {
        let ticket = self
            .h
            .fake
            .commands()
            .iter()
            .rev()
            .find_map(|command| match command {
                Command::StartTurn { ticket, .. } => Some(*ticket),
                _ => None,
            })
            .expect("a turn is in flight");
        let epoch = self.h.fake.epoch().expect("attached");
        self.h
            .fake
            .inject(epoch, AppEv::Reply(ticket, Reply::prepare_failed(message)));
        self.h.advance(100);
        self
    }
}

/// A PNG `name` of `width` x `height` pixels in a directory of this test process: a real image,
/// a decoder reads its size and pixels. Returns its path.
pub fn image_file_sized(name: &str, width: u32, height: u32) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("codex-kit-images-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("image dir");
    let path = dir.join(name);
    image::RgbaImage::from_pixel(width, height, image::Rgba([40, 120, 200, 255]))
        .save_with_format(&path, image::ImageFormat::Png)
        .expect("write the image");
    path
}

/// The server's `thread/start` answer: the thread `model` runs on (`""`: none).
fn thread_start_answer(model: &str) -> serde_json::Value {
    serde_json::json!({
        "thread": {
            "id": "thread-1", "environments": null, "extra": null, "sessionId": "thread-1",
            "forkedFromId": null, "attachedToThreadId": null, "parentThreadId": null,
            "preview": "", "ephemeral": false, "section": null, "sectionEnteredAt": null,
            "projectId": null, "historyMode": "legacy", "modelProvider": "openai",
            "model": null, "reasoningEffort": null, "createdAt": 0, "updatedAt": 0,
            "recencyAt": 0, "status": {"type": "idle"}, "path": null, "cwd": "/tmp/project",
            "cliVersion": "0.0.0", "originator": null, "source": "cli",
            "canAcceptDirectInput": true, "threadSource": null, "agentNickname": null,
            "agentRole": null, "gitInfo": null, "name": null, "daybreakEnabled": null,
            "turns": []
        },
        "model": model, "modelProvider": "openai", "serviceTier": null,
        "cwd": "/tmp/project", "instructionSources": [],
        "approvalPolicy": "on-request", "approvalsReviewer": "user",
        "sandbox": {"type": "dangerFullAccess"}, "reasoningEffort": null
    })
}

impl Open {
    /// The session's thread has no model (the oracle's `set_model("")`): the server's
    /// `thread/start` answer names an empty one.
    pub fn unavailable_model(self) -> Open {
        self.thread_start(thread_start_answer(""))
    }

    /// The composer rendered alone in the area (the oracle's `ChatComposer` tests draw it into a
    /// bare buffer: the prompt at the top, the footer on the last row), as the `composer`
    /// directory's `alone` entry does; `vim_mode()` and `keymap(..)` before it set what the
    /// composer is in.
    pub fn composer_alone(self) -> Open {
        self.entry("composer", "alone")
    }
}
