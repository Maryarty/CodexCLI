//! States the oracle sets by a widget's own setter, reached here by the route the app takes: a
//! composer whose input is off, an image of a size, a thread with no model, the composer alone.

use codex_tui_core::app::{AppEv, Intent};
use codex_tui_core::features::windows_sandbox::{Config, Fact, Host, Intent as Sandbox, Mode};
use codex_tui_core::features::{FeatureFact, FeatureIntent};
use codex_tui_core::fixtures::script::Answer;

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

impl Open {
    /// The session's thread has no model (the oracle's `set_model("")`): the server's
    /// `thread/start` answer names an empty one.
    pub fn unavailable_model(self) -> Open {
        self.thread_start(serde_json::json!({
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
            "model": "", "modelProvider": "openai", "serviceTier": null,
            "cwd": "/tmp/project", "instructionSources": [],
            "approvalPolicy": "on-request", "approvalsReviewer": "user",
            "sandbox": {"type": "dangerFullAccess"}, "reasoningEffort": null
        }))
    }

    /// The composer rendered alone in the area (the oracle's `ChatComposer` tests draw it into a
    /// bare buffer: the prompt at the top, the footer on the last row), as the `composer`
    /// directory's `alone` entry does; `vim_mode()` and `keymap(..)` before it set what the
    /// composer is in.
    pub fn composer_alone(self) -> Open {
        self.entry("composer", "alone")
    }
}
