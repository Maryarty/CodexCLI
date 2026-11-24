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
//! - `legacy_text()`: a terminal without grapheme clustering (`TextMode::Legacy`).
//! - `syntax_theme(name)`: the config's `[tui] theme` (the highlight theme in
//!   force), delivered as the host does, the settings screen's facts.
//! - `env(name, value)`: a variable of the app's environment (empty by
//!   default: what the shell exports changes nothing).
//! - `foreign_writer(thread)`: the chat shows `thread`, which another process
//!   writes (the server refuses the resume: "already has an active writer"),
//!   so the view is read-only; `fork_refused()` also has the server refuse the
//!   fork (otherwise it stays in progress).
//! - `update_available(latest, action)` / `server_notice(message, offer_update)`: the
//!   notices the host announces to the first conversation (`AppEv::Announce`): the
//!   update banner (the running version is the source build's `0.0.0`) and the
//!   other-version notice of the connected service.
//! - `thread_start(response)`: the server's `thread/start` answer of the session (the
//!   instruction sources, model and cwd the thread starts with).
//! - `daybreak(models)`: what the host knows of the account's cyber access (an eligible
//!   account and the catalog's `(model, standard, daybreak)` programs) that words a refusal.
//! - `raw_reasoning()`: the config's `show_raw_agent_reasoning` is on (a reasoning block also
//!   shows the model's raw reasoning).
//! - `tui(settings)`: the `[tui]` settings the host tells the app (`vim_mode_default`).
//! - `startup(first_run, tooltip)` / `tooltips_off()`: how the run started as the host tells
//!   the app (the user's first run, the model-availability notice) and `tui.show_tooltips`
//!   off; the conversation then opens with the oracle's session card. Without `startup` no
//!   conversation has a card.
//!
//! Files of the session: `kit::image_file(name)` writes a tiny PNG, `Open::cwd`
//! (markdown seeds) names the session's directory.

use codex_tui_core::app::facts::TuiSettings;
use codex_tui_core::app::{AppEv, Intent};
use codex_tui_core::features::settings::{Fact as SettingsFact, Facts};
use codex_tui_core::ids::ThreadId;
use codex_tui_core::rpc::ServerError;
use codex_tui_core::screens::ScreenEv;
use codex_tui_core::screens::agents::{Ev as AgentsEv, Page};
use codex_tui_core::screens::cells::{Ev as CellsEv, Startup};
use codex_tui_core::threads::Notice;
use codex_tui_core::threads::daybreak::{Access, ModelAccess, Programs};
use codex_tui_rtwk::services::startup::UpdateAction;
use rtwk::binding::ScreenMode;
use rtwk_core::style::ColorMode;
use rtwk_core::unicode::TextMode;
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

/// How the CLI was installed (what its update command runs; `Open::update_available`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Install {
    Npm,
    Bun,
    VitePlus,
    Pnpm,
    Brew,
    StandaloneUnix,
    StandaloneWindows,
}

impl Install {
    fn action(self) -> UpdateAction {
        match self {
            Install::Npm => UpdateAction::NpmGlobalLatest,
            Install::Bun => UpdateAction::BunGlobalLatest,
            Install::VitePlus => UpdateAction::VitePlusGlobalLatest,
            Install::Pnpm => UpdateAction::PnpmGlobalLatest,
            Install::Brew => UpdateAction::BrewUpgrade,
            Install::StandaloneUnix => UpdateAction::StandaloneUnix,
            Install::StandaloneWindows => UpdateAction::StandaloneWindows,
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
    legacy_text: bool,
    vars: Vec<(&'static str, &'static str)>,
    syntax_theme: Option<&'static str>,
    foreign_writer: Option<&'static str>,
    /// The server refuses the fork of the foreign thread (left unanswered
    /// otherwise: the fork stays in progress).
    fork_refused: bool,
    replay: Option<Vec<serde_json::Value>>,
    /// What the host announces to the first conversation.
    announced: Vec<Notice>,
    /// What the host knows of the account's cyber access.
    daybreak: Option<Access>,
    /// The server's answer to the session's `thread/start` (its instruction sources, settings).
    pub(super) thread_start: Option<serde_json::Value>,
    /// `show_raw_agent_reasoning` is on.
    raw_reasoning: bool,
    /// How the run started, as the host says.
    startup: Option<Startup>,
    /// `tui.show_tooltips` is off.
    tooltips_off: bool,
    /// The `[tui]` settings the host tells the app (`Open::tui`).
    tui: Option<TuiSettings>,
    /// The app runs inline (`--no-alt-screen`) under these shell rows.
    pub(super) inline: Option<Vec<String>>,
}

impl Env {
    /// The terminal as `rtwk_test` makes it for these seeds.
    pub(super) fn options(&self, width: u16, height: u16) -> Options {
        let mut options = Options::new().size(width, height);
        if let Some(history) = &self.inline {
            options = options
                .mode(ScreenMode::Inline)
                .history(history.iter().map(String::as_str));
        }
        if let Some(bg) = self.background {
            options = options.background_rgb(bg);
        }
        for (name, value) in &self.vars {
            options = options.env(name, value);
        }
        let (fg, palette, depth, legacy) = (
            self.foreground,
            self.palette.clone(),
            self.depth,
            self.legacy_text,
        );
        options.terminal(move |terminal| {
            terminal.foreground_rgb = fg;
            terminal.palette = palette;
            if let Some(depth) = depth {
                terminal.color_mode = depth.mode();
            }
            if legacy {
                terminal.text_mode = TextMode::Legacy;
            }
        })
    }

    /// An app event is to be delivered (the app has to be connected first).
    pub(super) fn needs_app(&self) -> bool {
        self.syntax_theme.is_some()
            || self.foreign_writer.is_some()
            || self.replay.is_some()
            || !self.announced.is_empty()
            || self.daybreak.is_some()
            || self.raw_reasoning
            || self.startup.is_some()
            || self.tooltips_off
            || self.tui.is_some()
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
            h.inject([AppEv::Feature(SettingsFact::Facts(facts).into())]);
        }
        if let Some(access) = &self.daybreak {
            h.connect();
            h.inject([AppEv::Daybreak(access.clone())]);
        }
        if self.raw_reasoning {
            h.connect();
            h.inject([AppEv::Screen(ScreenEv::Cells(CellsEv::RawReasoning(true)))]);
        }
        if self.tooltips_off || self.tui.is_some() {
            h.connect();
            let mut tui = self.tui.unwrap_or_default();
            if self.tooltips_off {
                tui.show_tooltips = false;
            }
            h.inject([AppEv::Tui(tui)]);
        }
        if let Some(startup) = &self.startup {
            h.connect();
            h.inject([AppEv::Screen(ScreenEv::Cells(CellsEv::Startup(
                startup.clone(),
            )))]);
        }
        if !self.announced.is_empty() {
            h.connect();
            h.inject(self.announced.iter().cloned().map(AppEv::Announce));
        }
        if let Some(thread) = self.foreign_writer {
            // the server refuses to resume the thread: another process writes it
            let fork_refused = self.fork_refused;
            h.fake.respond(move |request| {
                let refused = match request["method"].as_str() {
                    Some("thread/resume") => "thread already has an active writer",
                    // the fork the notice's `f` asks for ends in an error too
                    Some("thread/fork") if fork_refused => "fork refused",
                    _ => return None,
                };
                Some(Err(ServerError {
                    code: None,
                    data: None,
                    message: refused.to_string(),
                }))
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

    /// A terminal that does not cluster graphemes (`TextMode::Legacy`): a
    /// halfwidth katakana and its sound mark take two cells, as the oracle's
    /// `UnicodeWidthStr` counts them.
    pub fn legacy_text(mut self) -> Open {
        self.env.legacy_text = true;
        self
    }

    /// The app runs inline (`--no-alt-screen`): a region of the normal screen under `shell`, the
    /// rows the shell had printed (what the oracle's `render_inline` draws under).
    pub fn inline(mut self, shell: &[&str]) -> Open {
        self.env.inline = Some(shell.iter().map(|row| (*row).to_owned()).collect());
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

    /// With `foreign_writer`: the server refuses the fork (`f` on the notice)
    /// at once; without it the fork stays unanswered, in progress.
    pub fn fork_refused(mut self) -> Open {
        self.env.fork_refused = true;
        self
    }

    /// The chat shows a resumed thread whose recorded `turns` (the server's
    /// `Turn` objects: [`replay_turn`]) it replays as history, the oracle's
    /// `replay_thread_turns` / `initial_messages`.
    pub fn replay(mut self, turns: Vec<serde_json::Value>) -> Open {
        self.env.replay = Some(turns);
        self
    }

    /// The host announces a newer release `latest` (to run: `action`'s command, none for an
    /// install the CLI does not know) as the first conversation opens.
    pub fn update_available(mut self, latest: &str, action: Option<Install>) -> Open {
        self.env.announced.push(Notice::UpdateAvailable {
            current: "0.0.0".to_owned(),
            latest: latest.to_owned(),
            command: action.map(|install| install.action().command_str()),
        });
        self
    }

    /// The host announces that the connected service runs another version than the CLI
    /// (`message`; `offer_update`: it is the local daemon, `/daemon` manages it).
    pub fn server_notice(mut self, message: &str, offer_update: bool) -> Open {
        self.env.announced.push(Notice::ServerVersion {
            message: message.to_owned(),
            offer_update,
        });
        self
    }

    /// The host reads an eligible account whose catalog lists, per `(model, standard,
    /// daybreak)`, the cyber access programs of each model.
    pub fn daybreak(mut self, models: &[(&str, bool, bool)]) -> Open {
        self.env.daybreak = Some(Access {
            eligible: true,
            models: models
                .iter()
                .map(|&(model, standard, daybreak)| ModelAccess {
                    model: model.to_owned(),
                    programs: Some(Programs { standard, daybreak }),
                })
                .collect(),
        });
        self
    }

    /// The server answers the session's `thread/start` with `response` (a `ThreadStartResponse`:
    /// the instruction sources the thread loaded, its model and cwd) instead of the fake's own.
    pub fn thread_start(mut self, response: serde_json::Value) -> Open {
        self.env.thread_start = Some(response);
        self
    }

    /// The config's `show_raw_agent_reasoning` is on.
    pub fn raw_reasoning(mut self) -> Open {
        self.env.raw_reasoning = true;
        self
    }

    /// How the run started, as the host tells the app: `first_run` (the project's trust was
    /// never set: the card is the getting-started help) and the model-availability notice that
    /// takes the tip's place in the first conversation.
    pub fn startup(mut self, first_run: bool, tooltip: Option<&str>) -> Open {
        self.env.startup = Some(Startup {
            first_run,
            tooltip: tooltip.map(str::to_owned),
        });
        self
    }

    /// `tui.show_tooltips` is off.
    pub fn tooltips_off(mut self) -> Open {
        self.env.tooltips_off = true;
        self
    }

    /// The `[tui]` settings of the config as the host tells the app (`vim_mode_default`, ...).
    pub fn tui(mut self, settings: TuiSettings) -> Open {
        self.env.tui = Some(settings);
        self
    }

    /// A new draft starts in Vim Insert mode (`tui.vim_mode_default`).
    pub fn vim_mode(self) -> Open {
        self.tui(TuiSettings {
            vim_mode_default: true,
            ..TuiSettings::default()
        })
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
