//! JSONL event scripts: a screen's mock state as the events that make it.
//!
//! A script is a text of JSON lines, one step each; blank lines and lines
//! starting with `#` are skipped. Steps (`"step"` names the kind):
//!
//! - `seed`: model state the oracle's tests start from, made by `apply`
//!   (`{"step":"seed","seed":"session","thread":"thread-1"}`).
//! - `screen`: an event of one screen directory, read by that directory's own
//!   `seed` (`core/src/screens/<dir>.rs`), so a slice never edits this file.
//! - `fact`: a fact the host tells a feature unasked, read by that feature's own `SEED`
//!   (`core/src/features/<f>/`, `Feature::SEED`): `{"step":"fact","feature":"settings",
//!   "fact":{"kind":"theme_picker",..}}`. It is `AppEv::Feature`, the path the host's facts take.
//! - `note`: a line a feature says in a transcript, read by that feature's own `SEED_NOTE`
//!   (`Feature::SEED_NOTE`): `{"step":"note","feature":"goal","note":{..}}`, and `"thread"` for a
//!   thread other than the one in use. It is the chat's own `Intent::Note`.
//! - `presets`: the presets of the config, `AppEv::Presets` (`{"step":"presets","slots":[{"slot":1,
//!   "input":"/status","label":"Status","origin":"user"}]}`; `origin` `user`, `shadowing` or
//!   `outside`).
//! - `clock`: the time now (`{"step":"clock","secs":968250720}`, or `"millis"`, and an
//!   optional `"utc_offset"` in seconds east of UTC): the app is told it as the shell tells it
//!   (`AppEv::Clock`), so a script says what time it is and nothing waits for a real second. The
//!   old per-screen forms (`{"step":"screen","dir":"agents"|"resume","ev":{"clock":secs}}`) are
//!   read as this step.
//! - `notify`: a server notification, the protocol's own JSON in `message`.
//! - `request`: a server request, the protocol's own JSON in `message`.
//! - `lagged`: the app server's event stream dropped `skipped` events
//!   (`{"step":"lagged","skipped":3}`; oracle: the `Lagged` event).
//! - `respond`: what the backend answers a client request of that method
//!   with (`{"step":"respond","method":"account/rateLimits/read","result":{..}}`,
//!   or `"error":{"message":..}` for a refusal, as `reply` has it);
//!   the model has no backend, so `play` skips it and a tester's fake backend
//!   answers with it.
//! - `reply`: what the backend answers the NEXT request of a method with, one
//!   line per answer, in the order the requests come (`{"step":"reply",
//!   "method":"thread/turns/list","result":{..}}`, or `"error":{"message":..}`
//!   for a refusal). A reply is not an event of the script's timeline, so it is
//!   not a [`Step`]: `parse` collects the lines into [`Script::replies`] and a
//!   tester's fake backend takes from the queue as the requests arrive.
//! - `entry`: one entry of the transcript, in the shorthand the transcript tests write instead of
//!   the notifications that make it: `{"step":"entry","kind":"agent","text":"top"}`. `kind` is
//!   `agent`, `user`, `reasoning`, `plan` (the proposed plan, `text`), `plan_update` (`steps`:
//!   `[["Write it","completed"]]`, `explanation`) or `exec` (`command`, `output`, `exit`, or
//!   `running`). The entry is a turn of its own (`turn/started`, the item, `turn/completed`;
//!   `turn` names it, else `entry-<line>`); `"end":false` leaves the turn open, so an `agent`
//!   entry is the live tail, and `"start":false` adds to the open turn `turn` names. `item` names
//!   the item, so a second `agent` entry of it adds to its text. `text` is the text as the model
//!   said it, a tab and Markdown included. `"replace":true` makes an `agent` entry of a named
//!   `item` the item's whole text instead of more of it. `thread` is `thread-1` unless the step says. A recap
//!   is not a shape: it is a feature's note (`Screen::recap` of the test kit).
//! - `commit`: the end of an open turn (`{"step":"commit","turn":"turn-2"}`): the live tail is
//!   committed to the transcript (`turn/completed`).
//! - `key` and `resize`: inputs for a tester; the model has no use for them,
//!   `play` hands them back in order.
//!
//! The steps change the model only through `CodexApp::ingest`, the path the
//! live app takes: a script cannot build a state the app cannot reach.

use codex_app_server_protocol::ServerNotification;
use codex_app_server_protocol::ServerRequest;
use serde::Deserialize;
use std::collections::{BTreeMap, VecDeque};

use crate::app::AppEv;
use crate::app::AppFx;
use crate::app::BackendEv;
use crate::app::CodexApp;
use crate::app::Command;
use crate::app::Ingress;
use crate::app::Intent;
use crate::app::Now;
use crate::app::Session;
use crate::fixtures::builders;
use crate::ids::ThreadId;
use crate::rpc::Reply;
use crate::screens;
use crate::screens::ScreenIntent;
use crate::screens::chat;
use crate::screens::select::quick_dials::{Dial, Origin, Slot};
use crate::threads::{Note, NoteKey};

/// One line of a script.
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "step", rename_all = "snake_case")]
pub enum Step {
    Seed(Seed),
    /// An event of a screen directory, read by that directory's own `seed`
    /// (`{"step":"screen","dir":"resume","ev":{..}}`).
    Screen {
        dir: String,
        ev: serde_json::Value,
    },
    /// A fact of a feature, read by the feature's own `SEED`
    /// (`{"step":"fact","feature":"settings","fact":{..}}`).
    Fact {
        feature: String,
        fact: serde_json::Value,
    },
    /// The time now: the app is told it (`AppEv::Clock`). One of `secs` and `millis`.
    Clock {
        #[serde(default)]
        secs: Option<i64>,
        #[serde(default)]
        millis: Option<i64>,
        #[serde(default)]
        utc_offset: Option<i32>,
    },
    /// A line a feature says in a transcript (`{"step":"note","feature":"goal","note":{..}}`),
    /// read by the feature's own `SEED_NOTE`; `thread` none is the thread in use.
    Note {
        feature: String,
        #[serde(default)]
        thread: Option<String>,
        note: serde_json::Value,
    },
    /// The presets of the config (`AppEv::Presets`: `{"step":"presets","slots":[{"slot":1,
    /// "input":"/status","label":"Status","origin":"user"}]}`); a step replaces the set.
    Presets {
        slots: Vec<PresetLine>,
    },
    /// An entry of the transcript (see the module docs).
    Entry(Entry),
    /// The end of an open turn: its live tail is committed (`turn/completed`).
    Commit {
        #[serde(default)]
        thread: Option<String>,
        turn: String,
    },
    Notify {
        message: Box<ServerNotification>,
    },
    Request {
        message: Box<ServerRequest>,
    },
    Lagged {
        skipped: usize,
    },
    /// What the backend answers requests of `method` with: exactly one of `result` and `error`
    /// (checked by `parse`; `answer` reads it).
    Respond {
        method: String,
        #[serde(default)]
        result: Option<serde_json::Value>,
        #[serde(default)]
        error: Option<ErrorBody>,
    },
    Key {
        keys: String,
    },
    Resize {
        width: u16,
        height: u16,
    },
}

/// A saved preset of a `presets` step.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PresetLine {
    pub slot: u8,
    pub input: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub origin: PresetOrigin,
}

/// Which configuration layers hold a preset (`quick_dials::Origin`).
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PresetOrigin {
    /// Only the user config has it.
    #[default]
    User,
    /// The user config has it, and so does a layer below.
    Shadowing,
    /// A layer above the user config has it.
    Outside,
}

impl PresetLine {
    fn slot(&self) -> Slot {
        Slot {
            slot: self.slot,
            dial: Dial {
                input: self.input.clone(),
                label: self.label.clone(),
            },
            origin: match self.origin {
                PresetOrigin::User => Origin::User,
                PresetOrigin::Shadowing => Origin::Shadowing,
                PresetOrigin::Outside => Origin::Outside,
            },
        }
    }
}

fn yes() -> bool {
    true
}

/// An `entry` step: one entry of the transcript in a turn.
#[derive(Clone, Debug, Deserialize)]
pub struct Entry {
    /// The thread; none is `thread-1`, the one scripts seed.
    #[serde(default)]
    pub thread: Option<String>,
    /// The turn the entry is in; none is a turn of its own, named by the line.
    #[serde(default)]
    pub turn: Option<String>,
    /// The item's id; none is named by the line. A second `agent` entry with the same `item` in
    /// the same turn is more of its text (a delta), so a live answer grows by steps.
    #[serde(default)]
    pub item: Option<String>,
    /// The turn begins with this entry (`turn/started`).
    #[serde(default = "yes")]
    pub start: bool,
    /// The turn ends with this entry (`turn/completed`): false leaves the live tail open.
    #[serde(default = "yes")]
    pub end: bool,
    /// An `agent` entry of an item that has text says the text is now this one, in place of the
    /// text it had (the item completes with it, `item/completed`) instead of adding to it; an
    /// `exec` entry of a running command says so of its output.
    #[serde(default)]
    pub replace: bool,
    #[serde(flatten)]
    pub shape: Shape,
}

/// What an entry is.
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Shape {
    /// An agent message (`item/agentMessage/delta`: it is live until its turn ends).
    Agent { text: String },
    /// The user's prompt.
    User { text: String },
    /// The model's reasoning (its summary, Markdown).
    Reasoning { text: String },
    /// A proposed plan (Markdown).
    Plan { text: String },
    /// The `update_plan` report: `[step, status]` pairs (`pending`, `inProgress`, `completed`).
    PlanUpdate {
        steps: Vec<(String, String)>,
        #[serde(default)]
        explanation: Option<String>,
    },
    /// A shell command: running (with the `output` it has streamed so far), or finished with
    /// `output` and `exit`.
    Exec {
        command: String,
        #[serde(default)]
        output: String,
        #[serde(default)]
        exit: i32,
        #[serde(default)]
        running: bool,
    },
}

impl Entry {
    /// The events that make the entry, in order; `line` names the turn and the item when the step
    /// does not.
    fn events(&self, line: usize) -> Result<Vec<AppEv>, String> {
        let thread = self.thread.as_deref().unwrap_or(DEFAULT_THREAD);
        // ids, not prose: a turn and an item the step did not name are named by its line
        let at = line.to_string();
        let turn = (self.turn.clone()).unwrap_or_else(|| ["entry-", at.as_str()].concat());
        let item = (self.item.clone()).unwrap_or_else(|| ["item-", at.as_str()].concat());
        let backend = |ev: BackendEv| AppEv::Backend(ev);
        let mut events = Vec::new();
        if self.start {
            events.push(backend(builders::turn_started(thread, &turn)));
        }
        match &self.shape {
            Shape::Agent { text } if self.replace => {
                events.push(backend(builders::agent_completed(
                    thread, &turn, &item, text,
                )));
            }
            Shape::Agent { text } => {
                events.push(backend(builders::agent_delta(thread, &turn, &item, text)));
            }
            Shape::User { text } => {
                events.push(backend(builders::user_item(thread, &turn, &item, text)));
            }
            Shape::Reasoning { text } => events.push(backend(builders::reasoning_completed(
                thread, &turn, &item, text,
            )?)),
            Shape::Plan { text } => events.push(backend(builders::plan_completed(
                thread, &turn, &item, text,
            )?)),
            Shape::PlanUpdate { steps, explanation } => {
                let steps: Vec<_> = steps
                    .iter()
                    .map(|(step, status)| (step.as_str(), status.as_str()))
                    .collect();
                events.push(backend(builders::plan_updated(
                    thread,
                    &turn,
                    explanation.as_deref(),
                    &steps,
                )?));
            }
            Shape::Exec {
                command,
                output,
                exit,
                running,
            } => {
                if self.replace {
                    // the running command's output is now `output`, in place of what it had
                    events.push(backend(builders::command_progress(
                        thread, &turn, &item, command, output,
                    )?));
                } else {
                    events.push(backend(builders::command_started(
                        thread, &turn, &item, command,
                    )?));
                    if *running && !output.is_empty() {
                        events.push(backend(builders::command_output(
                            thread, &turn, &item, output,
                        )?));
                    }
                }
                if !running {
                    events.push(backend(builders::command_completed(
                        thread, &turn, &item, command, output, *exit,
                    )?));
                }
            }
        }
        if self.end {
            events.push(backend(builders::turn_completed(thread, &turn)));
        }
        Ok(events)
    }
}

/// The thread a script's `entry` and `commit` steps are for when they name none.
const DEFAULT_THREAD: &str = "thread-1";

impl Step {
    /// The events an `entry` or `commit` step is, in order; none for any other step. `line` is the
    /// step's line in its script. The error is the shape's refusal of its JSON.
    pub fn events(&self, line: usize) -> Option<Result<Vec<AppEv>, String>> {
        match self {
            Step::Entry(entry) => Some(entry.events(line)),
            Step::Commit { thread, turn } => {
                let thread = thread.as_deref().unwrap_or(DEFAULT_THREAD);
                Some(Ok(vec![AppEv::Backend(builders::turn_completed(
                    thread, turn,
                ))]))
            }
            _ => None,
        }
    }

    /// The event a `note` or `presets` step is, for the router (`ingest`) of a script or the
    /// events a test injects; none for any other step. The error is the feature's own refusal of
    /// the note's JSON (`SeedError`).
    pub fn event(&self) -> Option<Result<AppEv, String>> {
        match self {
            Step::Presets { slots } => Some(Ok(AppEv::Presets(
                slots.iter().map(PresetLine::slot).collect(),
            ))),
            Step::Note {
                feature,
                thread,
                note,
            } => Some(
                crate::features::seed_note(feature, note.clone())
                    .map(|note| {
                        AppEv::Intent(Intent::Screen(ScreenIntent::Chat(chat::Intent::Note {
                            thread: thread.as_deref().map(ThreadId::new),
                            note: Note::from(NoteKey::Feature(note)),
                        })))
                    })
                    .map_err(|err| format!("{feature}: {err:?}")),
            ),
            _ => None,
        }
    }

    /// The moment of a `clock` step; none for any other step or a step that names no time.
    pub fn now(&self) -> Option<Now> {
        let Step::Clock {
            secs,
            millis,
            utc_offset,
        } = self
        else {
            return None;
        };
        let now = match (secs, millis) {
            (Some(secs), None) => Now::at_secs(*secs),
            (None, Some(millis)) => Now::at_millis(*millis),
            _ => return None,
        };
        Some(now.in_zone(utc_offset.unwrap_or(0)))
    }
}

/// The clock events screens used to take (`{"step":"screen","dir":"agents"|"resume","ev":
/// {"clock":secs}}`) as the `clock` step: the screens no longer have an event for the time.
fn legacy_clock(value: &serde_json::Value) -> Option<Step> {
    if value.get("step")?.as_str()? != "screen" {
        return None;
    }
    if !matches!(value.get("dir")?.as_str()?, "agents" | "resume") {
        return None;
    }
    let secs = value.get("ev")?.get("clock")?.as_i64()?;
    Some(Step::Clock {
        secs: Some(secs),
        millis: None,
        utc_offset: None,
    })
}

/// Model state a script starts from.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(tag = "seed", rename_all = "snake_case")]
pub enum Seed {
    /// The session is ready with `thread` active: `Connected` and the reply
    /// to its `StartThread`, applied.
    Session { thread: String },
}

/// How the backend answers one request of a `reply` step.
#[derive(Clone, Debug, PartialEq)]
pub enum Answer {
    /// The request's `result`.
    Result(serde_json::Value),
    /// A refusal: the JSON-RPC error's code (if the script gave one) and message.
    Error {
        code: Option<i64>,
        message: String,
        /// The error's `data` (a provider failure carries one).
        data: Option<serde_json::Value>,
    },
}

/// The `reply` steps of a script, queued by method: the n-th request of a method gets the n-th
/// reply of that method; a request with no reply left is not answered by the script.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Replies {
    queues: BTreeMap<String, VecDeque<Answer>>,
}

impl Replies {
    /// Queues `answer` behind the replies of `method` already there.
    pub fn push(&mut self, method: impl Into<String>, answer: Answer) {
        self.queues
            .entry(method.into())
            .or_default()
            .push_back(answer);
    }

    /// The reply for the next request of `method`, taken off its queue.
    pub fn next(&mut self, method: &str) -> Option<Answer> {
        self.queues.get_mut(method)?.pop_front()
    }

    /// How many replies of `method` are still waiting.
    pub fn waiting(&self, method: &str) -> usize {
        self.queues.get(method).map_or(0, VecDeque::len)
    }

    pub fn is_empty(&self) -> bool {
        self.queues.values().all(VecDeque::is_empty)
    }
}

/// A parsed script.
#[derive(Clone, Debug)]
pub struct Script {
    pub steps: Vec<(usize, Step)>,
    /// The `reply` steps, by method (see the module docs).
    pub replies: Replies,
}

/// A `reply` line: exactly one of `result` and `error` (the `step` key was read by `parse`).
#[derive(Deserialize)]
struct ReplyLine {
    method: String,
    result: Option<serde_json::Value>,
    error: Option<ErrorBody>,
}

/// The JSON-RPC error of a `reply` or `respond` line.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorBody {
    pub code: Option<i64>,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

/// The answer of a line that has exactly one of `result` and `error` (`step`: its name).
fn answer_of(
    step: &str,
    result: Option<serde_json::Value>,
    error: Option<ErrorBody>,
) -> Result<Answer, String> {
    match (result, error) {
        (Some(result), None) => Ok(Answer::Result(result)),
        (None, Some(error)) => Ok(Answer::Error {
            code: error.code,
            message: error.message,
            data: error.data,
        }),
        _ => Err(format!(
            "a `{step}` step has exactly one of `result` and `error`"
        )),
    }
}

fn reply_of(line: ReplyLine) -> Result<(String, Answer), String> {
    let answer = answer_of("reply", line.result, line.error)?;
    Ok((line.method, answer))
}

impl Step {
    /// The method and the answer of a `respond` step; none for another step.
    pub fn respond(&self) -> Option<(&str, Answer)> {
        let Step::Respond {
            method,
            result,
            error,
        } = self
        else {
            return None;
        };
        let answer = answer_of("respond", result.clone(), error.clone()).ok()?;
        Some((method, answer))
    }
}

/// An input for a tester, in the order the script gave it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Input {
    Key(String),
    Resize { width: u16, height: u16 },
}

/// Why a script did not parse or play; `line` counts from 1.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptError {
    pub line: usize,
    pub message: String,
}

/// A script played over a fresh model.
pub struct Played {
    pub app: CodexApp,
    pub inputs: Vec<Input>,
}

/// Parses `text`; the first bad line stops it.
pub fn parse(text: &str) -> Result<Script, ScriptError> {
    let mut steps = Vec::new();
    let mut replies = Replies::default();
    for (index, line) in text.lines().enumerate() {
        let line_no = index + 1;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let bad = |message: String| ScriptError {
            line: line_no,
            message,
        };
        let value =
            serde_json::from_str::<serde_json::Value>(line).map_err(|e| bad(e.to_string()))?;
        if value.get("step").and_then(|step| step.as_str()) == Some("reply") {
            let line =
                serde_json::from_value::<ReplyLine>(value).map_err(|e| bad(e.to_string()))?;
            let (method, answer) = reply_of(line).map_err(bad)?;
            replies.push(method, answer);
            continue;
        }
        let step = match legacy_clock(&value) {
            Some(step) => step,
            None => serde_json::from_value::<Step>(value).map_err(|e| bad(e.to_string()))?,
        };
        if let Step::Clock { .. } = &step
            && step.now().is_none()
        {
            return Err(bad("a `clock` step names `secs` or `millis`".to_owned()));
        }
        if let Step::Respond { result, error, .. } = &step {
            answer_of("respond", result.clone(), error.clone()).map_err(bad)?;
        }
        steps.push((line_no, step));
    }
    Ok(Script { steps, replies })
}

/// Plays `text` over a fresh, attached model.
pub fn build(text: &str) -> Result<Played, ScriptError> {
    let script = parse(text)?;
    let mut app = CodexApp::new();
    app.attach();
    let inputs = script.play(&mut app)?;
    Ok(Played { app, inputs })
}

impl Script {
    /// Applies the steps to `app` (attached to a connection) through
    /// `ingest`; returns the `key` and `resize` inputs in order.
    pub fn play(&self, app: &mut CodexApp) -> Result<Vec<Input>, ScriptError> {
        let mut inputs = Vec::new();
        for (line, step) in &self.steps {
            let fail = |message: &str| ScriptError {
                line: *line,
                message: message.to_string(),
            };
            match step {
                Step::Seed(Seed::Session { thread }) => {
                    if app.session().status != Session::Starting {
                        return Err(fail("the session is already seeded"));
                    }
                    let fx = ingest(app, AppEv::Connected);
                    let ticket = fx
                        .commands()
                        .iter()
                        .find_map(|command| match command {
                            Command::StartThread { ticket, .. } => Some(*ticket),
                            _ => None,
                        })
                        .ok_or_else(|| fail("`Connected` asked for no thread"))?;
                    let reply = Reply::thread_started(ThreadId::new(thread));
                    ingest(app, AppEv::Reply(ticket, reply));
                }
                Step::Screen { dir, ev } => {
                    let ev = screens::seed_ev(dir, ev.clone()).map_err(|err| ScriptError {
                        line: *line,
                        message: format!("{err:?}"),
                    })?;
                    app.ingest_seeded_screen(ev);
                }
                Step::Clock { .. } => {
                    let now = step
                        .now()
                        .ok_or_else(|| fail("a `clock` step names a time"))?;
                    ingest(app, AppEv::Clock(now));
                }
                Step::Note { .. } | Step::Presets { .. } => {
                    let ev = step
                        .event()
                        .ok_or_else(|| fail("a `note` or `presets` step is an event"))?
                        .map_err(|message| ScriptError {
                            line: *line,
                            message,
                        })?;
                    ingest(app, ev);
                }
                Step::Entry(_) | Step::Commit { .. } => {
                    let events = step
                        .events(*line)
                        .ok_or_else(|| fail("an `entry` or `commit` step is events"))?
                        .map_err(|message| ScriptError {
                            line: *line,
                            message,
                        })?;
                    for ev in events {
                        ingest(app, ev);
                    }
                }
                Step::Fact { feature, fact } => {
                    let fact =
                        crate::features::seed_fact(feature, fact.clone()).map_err(|err| {
                            ScriptError {
                                line: *line,
                                message: format!("{err:?}"),
                            }
                        })?;
                    ingest(app, AppEv::Feature(fact));
                }
                Step::Notify { message } => {
                    let ev = BackendEv::Notification(message.clone());
                    ingest(app, AppEv::Backend(ev));
                }
                Step::Request { message } => {
                    let ev = BackendEv::Request(message.clone());
                    ingest(app, AppEv::Backend(ev));
                }
                Step::Lagged { skipped } => {
                    ingest(app, AppEv::Backend(BackendEv::Lagged(*skipped)));
                }
                Step::Respond { .. } => {}
                Step::Key { keys } => inputs.push(Input::Key(keys.clone())),
                Step::Resize { width, height } => inputs.push(Input::Resize {
                    width: *width,
                    height: *height,
                }),
            }
        }
        Ok(inputs)
    }
}

/// One event through the live app's router, under the current epoch. The
/// effects (RPCs) a script's events cause are returned, not run: a fixture
/// has no backend.
fn ingest(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let epoch = app.epoch();
    app.ingest(Ingress::new(epoch, ev)).unwrap_or_default()
}
