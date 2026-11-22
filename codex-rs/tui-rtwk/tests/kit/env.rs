//! Env seeds: what a test sets in the world around the app, before it starts.
//!
//! The oracle's tests change the terminal (`set_default_colors`, the palette
//! a probe found), the process (`FORCE_COLOR`, the highlight theme), and the
//! disk and cwd of the session. A kit screen gets each of them from one call
//! on the builder (`Open`), through what `rtwk_test` offers (`Options`: the
//! terminal's default colours and colour depth) and the app's own events:
//!
//! - `terminal_colors(fg, bg)`: the terminal answers the colour queries with
//!   these defaults (`0xRRGGBB`); without it no palette is known, as in a
//!   terminal that never answers (the Tester's default).
//! - `light()` / `dark()`: the same with the oracle's two typical terminals.
//! - `palette(&[(index, rgb)])`: the terminal's ANSI palette entries.
//! - `color_depth(depth)`: the colours the terminal shows (`FORCE_COLOR=0`
//!   is `ColorDepth::None`, `1` is `Ansi16`, `2` is `Ansi256`, `3` is
//!   `TrueColor`); the app's colours come down to them as they are written.
//! - `syntax_theme(name)`: the config's `[tui] theme` (the highlight theme in
//!   force), delivered as the host does, the settings screen's facts.
//! - `env(name, value)`: a variable of the app's environment (empty by
//!   default: what the shell exports changes nothing).
//! - `foreign_writer(thread)`: the chat shows `thread`, which another process
//!   writes (the server refuses the resume: "already has an active writer"),
//!   so the view is read-only.
//!
//! Files of the session: `kit::image_file(name)` writes a tiny PNG, `Open::cwd`
//! (markdown seeds) names the session's directory.

use codex_tui_core::app::{AppEv, Intent};
use codex_tui_core::ids::ThreadId;
use codex_tui_core::rpc::ServerError;
use codex_tui_core::screens::ScreenEv;
use codex_tui_core::screens::agents::{Ev as AgentsEv, Page};
use codex_tui_core::screens::settings::{Ev as SettingsEv, Facts};
use rtwk_core::style::ColorMode;
use rtwk_test::Options;

use super::seed::Open;
use crate::common::Harness;

/// The colours a terminal shows (`FORCE_COLOR` of the oracle's tests).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorDepth {
    /// No colours, only attributes (`FORCE_COLOR=0`).
    None,
    /// The 16 ANSI colours (`FORCE_COLOR=1`).
    Ansi16,
    /// The 256 palette entries (`FORCE_COLOR=2`).
    Ansi256,
    /// 24-bit colour (`FORCE_COLOR=3`, the default).
    TrueColor,
}

impl ColorDepth {
    fn mode(self) -> ColorMode {
        match self {
            ColorDepth::None => ColorMode::None,
            ColorDepth::Ansi16 => ColorMode::Ansi16,
            ColorDepth::Ansi256 => ColorMode::Ansi256,
            ColorDepth::TrueColor => ColorMode::TrueColor,
        }
    }
}

/// The seeds of the world around the app (`Open::env` and the rest).
#[derive(Clone, Default)]
pub(super) struct Env {
    foreground: Option<u32>,
    background: Option<u32>,
    palette: Vec<(u8, u32)>,
    depth: Option<ColorDepth>,
    vars: Vec<(&'static str, &'static str)>,
    syntax_theme: Option<&'static str>,
    foreign_writer: Option<&'static str>,
    replay: Option<Vec<serde_json::Value>>,
}

impl Env {
    /// The terminal as `rtwk_test` makes it for these seeds.
    pub(super) fn options(&self, width: u16, height: u16) -> Options {
        let mut options = Options::new().size(width, height);
        if let Some(bg) = self.background {
            options = options.background_rgb(bg);
        }
        for (name, value) in &self.vars {
            options = options.env(name, value);
        }
        let (fg, palette, depth) = (self.foreground, self.palette.clone(), self.depth);
        options.terminal(move |terminal| {
            terminal.foreground_rgb = fg;
            terminal.palette = palette;
            if let Some(depth) = depth {
                terminal.color_mode = depth.mode();
            }
        })
    }

    /// An app event is to be delivered (the app has to be connected first).
    pub(super) fn needs_app(&self) -> bool {
        self.syntax_theme.is_some() || self.foreign_writer.is_some() || self.replay.is_some()
    }

    /// Delivers what is an app event rather than a terminal's property, once
    /// the app is connected.
    pub(super) fn deliver(&self, h: &mut Harness) {
        if let Some(name) = self.syntax_theme {
            let facts = Facts {
                syntax_theme: Some(name.to_string()),
                ..Facts::default()
            };
            h.connect();
            h.inject([AppEv::Screen(ScreenEv::Settings(SettingsEv::Facts(facts)))]);
        }
        if let Some(thread) = self.foreign_writer {
            // the server refuses to resume the thread: another process writes it
            h.fake.respond(|request| {
                (request["method"] == "thread/resume").then(|| {
                    Err(ServerError {
                        code: None,
                        message: "thread already has an active writer".to_string(),
                    })
                })
            });
            h.connect();
            h.quiesce();
            // the chat can only open a thread it knows: the list has it
            let listed = Page {
                threads: vec![listed_thread(thread)],
                unread: Vec::new(),
                next_cursor: None,
                append: false,
                pins: None,
            };
            h.inject([
                AppEv::Screen(ScreenEv::Agents(AgentsEv::Listed(listed))),
                AppEv::Intent(Intent::OpenThread(ThreadId::new(thread))),
            ]);
            h.advance(200);
        }
        if let Some(turns) = &self.replay {
            // the server lists the thread and answers the resume with its
            // recorded turns: the chat replays them as history
            let turns = turns.clone();
            h.fake.respond(move |request| {
                (request["method"] == "thread/resume")
                    .then(|| Ok(resume_response(REPLAYED, &turns)))
            });
            h.connect();
            h.quiesce();
            let listed = Page {
                threads: vec![listed_thread(REPLAYED)],
                unread: Vec::new(),
                next_cursor: None,
                append: false,
                pins: None,
            };
            h.inject([
                AppEv::Screen(ScreenEv::Agents(AgentsEv::Listed(listed))),
                AppEv::Intent(Intent::OpenThread(ThreadId::new(REPLAYED))),
            ]);
            h.advance(200);
        }
    }
}

/// The id of the thread `Open::replay` opens.
pub const REPLAYED: &str = "replayed-1";

impl Open {
    /// The terminal's default colours (`0xRRGGBB`), as its answer to the
    /// colour queries gives them: the oracle's `set_default_colors`.
    pub fn terminal_colors(mut self, foreground: u32, background: u32) -> Open {
        self.env.foreground = Some(foreground);
        self.env.background = Some(background);
        self
    }

    /// A light terminal as the oracle's light-palette tests set one:
    /// foreground `555753`, background `ffffff`.
    pub fn light(self) -> Open {
        self.terminal_colors(0x55_57_53, 0xff_ff_ff)
    }

    /// A dark terminal: foreground `eeeeee`, background `1e1e1e`.
    pub fn dark(self) -> Open {
        self.terminal_colors(0xee_ee_ee, 0x1e_1e_1e)
    }

    /// The terminal's ANSI palette entries: `(index, 0xRRGGBB)`.
    pub fn palette(mut self, entries: &[(u8, u32)]) -> Open {
        self.env.palette = entries.to_vec();
        self
    }

    /// The colours the terminal shows (`FORCE_COLOR`).
    pub fn color_depth(mut self, depth: ColorDepth) -> Open {
        self.env.depth = Some(depth);
        self
    }

    /// The syntax theme of the config (`[tui] theme`), by name.
    pub fn syntax_theme(mut self, name: &'static str) -> Open {
        self.env.syntax_theme = Some(name);
        self
    }

    /// The chat shows `thread`, written by another process: the server
    /// refuses the resume (the oracle's external-writer view), so the view is
    /// read-only.
    pub fn foreign_writer(mut self, thread: &'static str) -> Open {
        self.env.foreign_writer = Some(thread);
        self
    }

    /// The chat shows a resumed thread whose recorded `turns` (the server's
    /// `Turn` objects: [`replay_turn`]) it replays as history, the oracle's
    /// `replay_thread_turns` / `initial_messages`.
    pub fn replay(mut self, turns: Vec<serde_json::Value>) -> Open {
        self.env.replay = Some(turns);
        self
    }

    /// A variable of the app's environment.
    pub fn env(mut self, name: &'static str, value: &'static str) -> Open {
        self.env.vars.push((name, value));
        self
    }
}

/// One recorded turn of the server (`Turn`): completed, with `items` (the
/// server's `ThreadItem` objects, `{"type": "userMessage", ..}`).
pub fn replay_turn(id: &str, items: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "id": id, "rootTurnId": null, "items": items, "itemsView": "full",
        "status": "completed", "error": null, "startedAt": 1_700_000_000,
        "completedAt": 1_700_000_010, "durationMs": 10_000
    })
}

/// The server's answer to `thread/resume` for `id`: the thread with `turns`.
fn resume_response(id: &str, turns: &[serde_json::Value]) -> serde_json::Value {
    let mut thread = serde_json::to_value(listed_thread(id)).expect("a thread");
    thread["turns"] = serde_json::Value::Array(turns.to_vec());
    serde_json::json!({
        "thread": thread, "model": "gpt-5", "modelProvider": "openai", "serviceTier": null,
        "cwd": "/work", "instructionSources": [],
        "approvalPolicy": "on-request", "approvalsReviewer": "user",
        "sandbox": {"type": "dangerFullAccess"}, "reasoningEffort": null
    })
}

/// A listed thread of the server (`thread/list`), idle, in `/work`.
fn listed_thread(id: &str) -> codex_app_server_protocol::Thread {
    let json = serde_json::json!({
        "id": id, "environments": null, "extra": null, "sessionId": id,
        "forkedFromId": null, "attachedToThreadId": null, "parentThreadId": null,
        "preview": id, "ephemeral": false, "section": null, "sectionEnteredAt": null,
        "projectId": null, "historyMode": "legacy", "modelProvider": "openai",
        "model": null, "reasoningEffort": null, "createdAt": 1_700_000_000,
        "updatedAt": 1_700_000_100, "recencyAt": 1_700_000_100, "status": {"type": "idle"},
        "path": null, "cwd": "/work", "cliVersion": "0.0.0", "originator": null,
        "source": "cli", "canAcceptDirectInput": true, "threadSource": null,
        "agentNickname": null, "agentRole": null, "gitInfo": null, "name": id,
        "daybreakEnabled": null, "turns": []
    });
    serde_json::from_value(json).expect("a listed thread")
}
