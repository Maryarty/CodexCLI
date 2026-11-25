//! States the oracle sets by a widget's own setter, reached here by the route the app takes: a
//! composer whose input is off, an image of a size, a thread with no model, the composer alone.

use codex_tui_core::app::{AppEv, BackendEv, Command, Intent};
use codex_tui_core::features::windows_sandbox::{Config, Fact, Host, Intent as Sandbox, Mode};
use codex_tui_core::features::{FeatureFact, FeatureIntent};
use codex_tui_core::fixtures::script::Answer;
use codex_tui_core::ids::ThreadId;
use codex_tui_core::rpc::Reply;
use codex_tui_rtwk::app::UiApp;

use super::Screen;
use super::seed::{Open, answer};

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

    /// The event stream of the thread is back (`ThreadStreamReconnected`).
    pub fn stream_restored(&mut self) -> &mut Screen {
        assert!(!self.entry, "a stream belongs to the live app");
        let thread = self.active_thread();
        self.h
            .inject([AppEv::Backend(BackendEv::StreamRestored(thread))]);
        self.h.advance(100);
        self
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
