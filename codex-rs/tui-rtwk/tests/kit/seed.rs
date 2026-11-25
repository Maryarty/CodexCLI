//! Seeds: the state a screen test starts from, in three kinds, each one call
//! on the builder (`Open`).
//!
//! - `entry(dir, name)`: a screen's debug-menu entry, the screen over the
//!   fixture script its directory registered (the way the debug menu and the
//!   `screen` scenario step open it). The entry exists when the slice
//!   registered it.
//! - `script(jsonl)`: the live chat, fed the script's steps as a server would
//!   (`notify`, `request`, and the directories' `screen` events), then its
//!   `key` and `resize` steps. A `seed` step is skipped (the live app is
//!   already in its session).
//! - `live()`: the live chat as the session left it, nothing fed.
//!
//! Options on any seed: `early()` stops at the first frame (a loading state
//! before the data arrives), `keymap(..)` remaps keys. After opening,
//! `Screen::feed(jsonl)` applies more script steps to the open screen
//! (history prepended, a tail absorbed, a result arriving). An `entry` screen
//! plays its fixture in a model of its own: `feed` is for the live app.

use codex_tui_core::app::{AppEv, BackendEv};
use codex_tui_core::fixtures::script::{self, Step};
use codex_tui_core::screens::seed_ev;
use codex_tui_rtwk::app::UiApp;

use super::Screen;
use super::env::Env;
use super::verdict::{Owner, missing};
use crate::common::{Harness, ms};

#[derive(Clone, Copy)]
pub(super) enum Seed {
    Live,
    Entry {
        dir: &'static str,
        entry: &'static str,
    },
    /// A screen directory's own row of the debug menu: its live opening, no fixture.
    Dir(&'static str),
    Script(&'static str),
}

/// A screen test's start: who owns the screen, the terminal size, the seed.
pub struct Open {
    pub(super) owner: Owner,
    pub(super) width: u16,
    pub(super) height: u16,
    pub(super) seed: Seed,
    pub(super) early: bool,
    pub(super) first_frame: bool,
    pub(super) held_start: bool,
    /// The fake mints `fake-turn-{n}` for the turns the app starts (`Open::fake_turn_ids`).
    pub(super) fake_turns: bool,
    /// The chat starts inside a `/side` conversation (`Open::side`).
    pub(super) side: bool,
    pub(super) gutter: usize,
    pub(super) keymap: Option<&'static [(&'static str, &'static str)]>,
    pub(super) env: Env,
    /// The remote image URLs the chat's draft starts with (`Open::remote_images`).
    pub(super) remote_images: Vec<String>,
    /// The text the chat's draft starts with (`Open::draft`).
    pub(super) draft: Option<String>,
    /// What the usage page's host answers (`Open::usage_accounts`, `usage_report`).
    pub(super) usage: super::usage::Host,
}

impl Screen {
    /// Starts describing a screen test; `owner` is the slice that builds what
    /// the test opens (it names the pending when the seed does not exist yet).
    pub fn build(owner: Owner) -> Open {
        Open {
            owner,
            width: crate::common::WIDTH,
            height: crate::common::HEIGHT,
            seed: Seed::Live,
            early: false,
            first_frame: false,
            held_start: false,
            fake_turns: false,
            side: false,
            gutter: 0,
            keymap: None,
            env: Env::default(),
            remote_images: Vec::new(),
            draft: None,
            usage: super::usage::Host::default(),
        }
    }
}

impl Open {
    /// The terminal's height, its width the default one.
    pub fn height(mut self, height: u16) -> Open {
        self.height = height;
        self
    }

    /// The terminal size of the oracle test (columns, rows).
    pub fn size(mut self, width: u16, height: u16) -> Open {
        self.width = width;
        self.height = height;
        self
    }

    /// A screen directory's debug entry.
    pub fn entry(mut self, dir: &'static str, entry: &'static str) -> Open {
        self.seed = Seed::Entry { dir, entry };
        self
    }

    /// A screen directory's own row of the debug menu (no fixture entry): the page as the app
    /// opens it, on the live model, asking its host (the fake backend). The usage page asks for
    /// its account and reports; `usage_accounts` / `usage_report` say what the host answers.
    pub fn dir(mut self, dir: &'static str) -> Open {
        self.seed = Seed::Dir(dir);
        self
    }

    /// The live chat, fed a JSONL script (`include_str!` of a file under
    /// `tests/oracle/fixtures/`).
    pub fn script(mut self, jsonl: &'static str) -> Open {
        self.seed = Seed::Script(jsonl);
        self
    }

    /// The live chat as it starts.
    pub fn live(mut self) -> Open {
        self.seed = Seed::Live;
        self
    }

    /// Stops at the screen's first frame: the state before the data arrives
    /// (`Loading sessions…` before the list). `Screen::wait_for(..)` runs on.
    pub fn early(mut self) -> Open {
        self.early = true;
        self
    }

    /// An entry's very first frame, at zero elapsed time: the Enter that
    /// opens it runs one frame, without the 100 ms the kit lets pass after a
    /// key (an animation's frame 0, the oracle's first draw).
    pub fn first_frame(mut self) -> Open {
        self.first_frame = true;
        self
    }

    /// The turns the app starts get the ids `fake-turn-1`.. (the fake's own), not `turn-1`..,
    /// which a script's events use for turns of their own: a turn the test starts by keys is then
    /// told apart from the script's.
    pub fn fake_turn_ids(mut self) -> Open {
        self.fake_turns = true;
        self
    }

    /// The live chat before its session has started: the backend leaves the
    /// thread's start unanswered (the oracle's chat widget with no thread id).
    pub fn held_start(mut self) -> Open {
        self.held_start = true;
        self
    }

    /// The live chat inside a side conversation (`/side`, the fork answered): its footer says
    /// `to switch`, Ctrl+C and Ctrl+D over an empty composer return to the parent
    /// (`Screen::side_closed`). For `live()` and `script(..)`.
    pub fn side(mut self) -> Open {
        self.side = true;
        self
    }

    /// The user's key bindings: `(action, key)` pairs as the keymap config
    /// names them, `&[("list.move_down", "z")]`.
    pub fn keymap(mut self, bindings: &'static [(&'static str, &'static str)]) -> Open {
        self.keymap = Some(bindings);
        self
    }

    /// The chat's draft starts with `urls` as remote images above the text (the oracle's
    /// `set_remote_image_urls`): the rows `[Image #1]`.. over the prompt, the local images
    /// numbered after them. For the live chat (`live()`, `script(..)`).
    pub fn remote_images(mut self, urls: &[&str]) -> Open {
        self.remote_images = urls.iter().map(|url| (*url).to_string()).collect();
        self
    }

    /// The chat's draft starts with `text`, the caret at its end: what a script of the oracle's
    /// tests sets with `set_text_content` before the screen is looked at. For the live chat
    /// (`live()`, `script(..)`); the images and the text of a restored message are
    /// `remote_images`' and the keys'.
    pub fn draft(mut self, text: &str) -> Open {
        self.draft = Some(text.to_string());
        self
    }

    pub fn open(self) -> Screen {
        let gutter = self.gutter;
        let draft = self.draft.clone();
        let remote = self.remote_images.clone();
        let side = self.side;
        let mut screen = match self.seed {
            Seed::Live => Screen::from(self.live_harness()),
            Seed::Entry { dir, entry } => self.open_entry(dir, Some(entry)),
            Seed::Dir(dir) => self.open_entry(dir, None),
            Seed::Script(text) => self.open_script(text),
        };
        screen.gutter = gutter;
        if side {
            assert!(!screen.entry, "a side conversation is of the live chat");
            super::side::enter(&mut screen.h);
        }
        if !remote.is_empty() {
            screen.set_remote_images(remote);
        }
        if let Some(text) = draft {
            screen.set_draft(&text);
        }
        screen
    }

    /// Hands the app the user's bindings, as the host does once it has read
    /// the config (`AppEv::Keymap`): a debug entry's screen reads the keymap
    /// the live app provides above it.
    fn set_keymap(&self, h: &mut Harness) {
        let Some(bindings) = self.keymap else { return };
        h.set_keymap(bindings);
    }

    /// The app on a terminal made as the env seeds say.
    fn driven(&self, debug: bool) -> Harness {
        let h = crate::common::start_driven_with(debug, self.env.options(self.width, self.height));
        if self.fake_turns {
            h.fake.mint_turns_as("fake-turn-");
        }
        self.usage.tell(&h.fake);
        h
    }

    fn live_harness(&self) -> Harness {
        let mut h = self.driven(false);
        if self.held_start {
            h.fake.hold_thread_start();
        }
        if let Some(response) = &self.env.thread_start {
            answer(
                &mut h,
                "thread/start".to_string(),
                script::Answer::Result(response.clone()),
            );
        }
        // The server's mode catalog (`collaborationMode/list`, read at connect): the Default and
        // Plan presets with no model or effort of their own, so `/plan` and Shift+Tab have a Plan
        // to switch to (the oracle's chat widget enables the collaboration modes the same way).
        answer(
            &mut h,
            "collaborationMode/list".to_string(),
            script::Answer::Result(serde_json::json!({"data": [
                {"name": "Plan", "mode": "plan", "model": null, "reasoning_effort": null},
                {"name": "Default", "mode": "default", "model": null, "reasoning_effort": null},
            ]})),
        );
        h.connect();
        self.env.deliver(&mut h);
        self.set_keymap(&mut h);
        // The session is started (`ready`) when `open` returns.
        h.quiesce();
        h
    }

    fn open_entry(self, dir: &str, entry: Option<&str>) -> Screen {
        let dirs = codex_tui_rtwk::app::debug::dirs();
        let Some(d) = dirs.iter().find(|d| d.name == dir) else {
            missing(self.owner, &format!("no screen directory `{dir}`"));
        };
        if let Some(entry) = entry
            && !(d.debug_entries)().iter().any(|e| e.name == entry)
        {
            missing(
                self.owner,
                &format!("no debug entry `{dir}/{entry}`: the slice has not registered this state"),
            );
        }
        let mut h = self.driven(true);
        if self.keymap.is_some() || self.env.needs_app() {
            h.connect();
            self.env.deliver(&mut h);
            self.set_keymap(&mut h);
        }
        if self.env.inline.is_some() {
            for _ in 0..crate::common::debug_row(dir, entry) {
                h.t.press("down");
            }
            h.t.press("enter");
            h.t.pump_for(ms(800));
        } else if self.first_frame {
            for _ in 0..crate::common::debug_row(dir, entry) {
                h.press("down");
            }
            h.t.press("enter");
            h.t.pump();
        } else {
            h.open_debug_row(dir, entry);
        }
        if self.env.inline.is_some() || self.first_frame {
        } else if self.early {
            h.t.pump_for(ms(20));
        } else {
            h.advance(800);
        }
        Screen {
            entry: true,
            ..Screen::from(h)
        }
    }

    fn open_script(self, text: &str) -> Screen {
        let mut h = self.live_harness();
        feed(&mut h, text);
        Screen::from(h)
    }
}

impl Screen {
    /// The remote images of the chat's draft are `urls` now (`Open::remote_images`, the oracle's
    /// `set_remote_image_urls`): UI state of the open thread, so the seed reaches it as a
    /// handler would, then the frame shows the rows.
    pub fn remote_images(&mut self, urls: &[&str]) -> &mut Screen {
        self.set_remote_images(urls.iter().map(|url| (*url).to_string()).collect());
        self
    }

    /// The host answers the clipboard-image ask (`Ask::ClipboardImage`, asked by Ctrl+V or Alt+V)
    /// with the PNG file at `path`: the composer attaches it where the caret is. The system
    /// clipboard itself is the host's, so a test hands over the answer.
    pub fn clipboard_image(&mut self, path: &std::path::Path) -> &mut Screen {
        self.answer_clipboard_image(path);
        self.h.advance(100);
        self
    }

    /// `key` (Ctrl+V or Alt+V) pressed and the host's answer with the PNG at `path` at once, a
    /// millisecond apart and with nothing waited after: what the paste detector holds is still
    /// held (the oracle's `handle_key_event` then `attach_image`, no time between).
    pub fn paste_clipboard_image_at_once(
        &mut self,
        key: &str,
        path: &std::path::Path,
    ) -> &mut Screen {
        self.press_key(key);
        self.h.t.pump_for(crate::common::ms(1));
        self.answer_clipboard_image(path);
        self.h.t.pump_for(crate::common::ms(1));
        self
    }

    fn answer_clipboard_image(&mut self, path: &std::path::Path) {
        use codex_tui_core::app::AppEv;
        use codex_tui_core::screens::{ScreenEv, popovers};
        let epoch = self.h.fake.epoch().expect("attached");
        self.h.fake.inject(
            epoch,
            AppEv::Screen(ScreenEv::Popovers(popovers::Ev::ImagePasted(
                path.to_path_buf(),
            ))),
        );
    }

    /// The chat's draft is `text` now, the caret at its end (`Open::draft`): UI state of the open
    /// thread, set as a handler would (what was held, attached or typed goes), then the frame
    /// shows it.
    pub fn draft(&mut self, text: &str) -> &mut Screen {
        self.set_draft(text);
        self
    }

    fn set_draft(&mut self, text: &str) {
        assert!(
            !self.entry,
            "a draft seeds the live chat's composer: a debug entry has none"
        );
        let text = text.to_string();
        self.h.t.update_app(move |app: &mut UiApp, _| {
            let ui = app.ui.thread_ui.get(app.core.session().active.as_ref());
            ui.draft.clear();
            ui.composer.reset("");
            ui.composer.insert(&text);
            ui.draft.sync(&ui.composer);
        });
        self.h.advance(100);
    }

    fn set_remote_images(&mut self, urls: Vec<String>) {
        assert!(
            !self.entry,
            "remote images seed the live chat's draft: a debug entry has none"
        );
        self.h.t.update_app(move |app: &mut UiApp, _| {
            let ui = app.ui.thread_ui.get(app.core.session().active.as_ref());
            ui.draft
                .set_remote_images(&ui.composer, urls, |n| format!("[Image #{n}]"));
        });
        self.h.advance(100);
    }

    /// Applies more steps of a JSONL script to the open screen, as a server
    /// and the user would deliver them: `notify`, `request`, a directory's
    /// `screen` events, then its `key` and `resize` steps.
    pub fn feed(&mut self, jsonl: &str) -> &mut Screen {
        assert!(
            !self.entry,
            "feed works on a script() or live() screen: a debug entry's screen plays its own fixture, the app's events do not reach it"
        );
        feed(&mut self.h, jsonl);
        self
    }

    /// The server refuses the request `command` made, with `message` (the oracle's
    /// `Err("app/list failed: 403 Forbidden")` of a load): the app's answer of a refusal, for a
    /// request that was left unanswered (a `respond` step with an `error` answers each one the
    /// moment it is made).
    pub fn answer_error(
        &mut self,
        command: &codex_tui_core::app::Command,
        message: &str,
    ) -> &mut Screen {
        let (ticket, reply) = codex_tui_core::protocol::reply(command, Err(message.to_string()))
            .expect("a command with a reply");
        let epoch = self.h.fake.epoch().expect("attached");
        self.h.fake.inject(epoch, AppEv::Reply(ticket, reply));
        self.h.advance(100);
        self
    }
}

fn feed(h: &mut Harness, text: &str) {
    let script = match script::parse(text) {
        Ok(script) => script,
        Err(err) => panic!("script line {}: {}", err.line, err.message),
    };
    h.connect();
    // The session has to be there before the script's events: an event for
    // a thread the app has not started yet is dropped.
    h.quiesce();
    let Some(epoch) = h.fake.epoch() else {
        panic!("the app never connected");
    };
    let mut inputs = Vec::new();
    for (line, step) in script.steps {
        match step {
            Step::Seed(_) => {}
            Step::Screen { dir, ev } => match seed_ev(&dir, ev) {
                Ok(ev) => h.fake.inject(epoch, AppEv::Screen(ev)),
                Err(err) => panic!("script line {line}: screen `{dir}`: {err:?}"),
            },
            step @ Step::Clock { .. } => {
                let now = step.now().expect("`parse` checked the step");
                h.fake.inject(epoch, AppEv::Clock(now));
            }
            step @ (Step::Note { .. } | Step::Presets { .. }) => {
                match step
                    .event()
                    .expect("a `note` or `presets` step is an event")
                {
                    Ok(ev) => h.fake.inject(epoch, ev),
                    Err(err) => panic!("script line {line}: {err}"),
                }
            }
            Step::Fact { feature, fact } => {
                match codex_tui_core::features::seed_fact(&feature, fact) {
                    Ok(fact) => h.fake.inject(epoch, AppEv::Feature(fact)),
                    Err(err) => panic!("script line {line}: fact `{feature}`: {err:?}"),
                }
            }
            Step::Notify { message } => {
                h.fake
                    .inject(epoch, AppEv::Backend(BackendEv::Notification(message)));
            }
            Step::Request { message } => {
                h.fake
                    .inject(epoch, AppEv::Backend(BackendEv::Request(message)));
            }
            Step::Lagged { skipped } => {
                h.fake
                    .inject(epoch, AppEv::Backend(BackendEv::Lagged(skipped)));
            }
            step @ Step::Respond { .. } => {
                let (method, result) = step.respond().expect("`parse` checked the answer");
                answer(h, method.to_string(), result);
            }
            Step::Key { keys } => inputs.push(script::Input::Key(keys)),
            Step::Resize { width, height } => {
                inputs.push(script::Input::Resize { width, height });
            }
        }
    }
    h.advance(200);
    h.play_inputs(&inputs);
}

thread_local! {
    /// What the fake backend answers a client request with, by method (the
    /// `respond` steps of this test's scripts).
    static ANSWERS: std::sync::Arc<std::sync::Mutex<std::collections::BTreeMap<String, script::Answer>>> =
        std::sync::Arc::default();
}

/// `{"step":"respond"}`: the backend answers requests of `method` with the step's `result` (or
/// refuses them with its `error`) from now on (the responder reads this test's table, later
/// steps replace an entry).
pub(super) fn answer(h: &mut Harness, method: String, result: script::Answer) {
    let answers = ANSWERS.with(std::sync::Arc::clone);
    answers.lock().expect("answers").insert(method, result);
    h.fake.respond(move |request| {
        let method = request["method"].as_str()?;
        let answer = answers.lock().ok()?.get(method).cloned()?;
        Some(match answer {
            script::Answer::Result(result) => Ok(result),
            script::Answer::Error {
                code,
                message,
                data,
            } => Err(codex_tui_core::rpc::ServerError {
                code,
                message,
                data,
            }),
        })
    });
}

impl Screen {
    /// From now on the server answers requests of `method` with `result` (the script's `respond`
    /// step, for a test that changes the answer between two keys).
    pub fn server_answers(&mut self, method: &str, result: serde_json::Value) -> &mut Screen {
        answer(
            &mut self.h,
            method.to_string(),
            script::Answer::Result(result),
        );
        self
    }

    /// From now on the server refuses requests of `method` with the JSON-RPC error `code` and
    /// `message`.
    pub fn server_refuses(
        &mut self,
        method: &str,
        code: Option<i64>,
        message: &str,
    ) -> &mut Screen {
        answer(
            &mut self.h,
            method.to_string(),
            script::Answer::Error {
                code,
                message: message.to_string(),
                data: None,
            },
        );
        self
    }
}
