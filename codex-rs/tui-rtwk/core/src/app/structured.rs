//! The app's half of `AppCx::structured_turn` (ADR-012 "structured turn", ADR-003; oracle
//! `temporary_structured_request.rs`): a feature asks for an isolated request to the model and
//! the app runs it as the thread RPCs of `threads::structured`, one after the other, in an
//! ephemeral thread that never becomes a `ThreadModel`: `config/read` (the MCP servers to
//! disable), `thread/start`, `turn/start` with the output schema, the thread's last assistant
//! message from the server's stream, `thread/unsubscribe`. The feature is told the message, or
//! why there is none (`Feature::structured_done`), whatever happened.
//!
//! A run is a request record by its own key (the run's number); the ticket of the RPC in flight
//! says which run it answers (`AppCtx::Structured`). What the stream said of the thread before
//! `turn/start` answered with the turn's id is kept and read once the id is known. A phase that
//! takes longer than 30 seconds (the oracle's `STRUCTURED_TURN_TIMEOUT`) ends the run, by the
//! clock the shell tells (`wake_at`, `expire`).

use std::collections::BTreeMap;
use std::rc::Rc;

use codex_app_server_protocol::{ServerNotification, ThreadItemPayload, TurnStatus};

use super::{AppCtx, AppFx, CodexApp, Command, Now, features};
use crate::features::FeatureCtx;
use crate::ids::{BackendId, ThreadId};
use crate::rpc::{Answer, RpcError};
use crate::screens::{ScreenCommand, ScreenReply, agents};
use crate::threads::structured::{
    RESPONSE_MAX_BYTES, StructuredTurn, TIMEOUT_MILLIS, Temporary, Why,
};
use crate::threads::{ThreadAsk, TurnEnd};

/// The structured turns in flight, by number.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Runs {
    next: u64,
    runs: BTreeMap<u64, Run>,
}

/// One run: what was asked, who asked, and where it is.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Run {
    ctx: FeatureCtx,
    turn: StructuredTurn,
    /// The backend of the thread the request is about: every RPC of the run goes there.
    backend: BackendId,
    phase: Phase,
    /// What the stream said of the temporary thread (`observe`), until the run ends.
    seen: Vec<Seen>,
    /// When the phase in progress has taken too long; none while the shell has told no clock.
    deadline: Option<Now>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Phase {
    /// `config/read` is out.
    Config,
    /// `thread/start` is out.
    Starting,
    /// `turn/start` is out on `thread`.
    Beginning { thread: ThreadId },
    /// The turn `turn` of `thread` runs: its end comes in the stream.
    Waiting { thread: ThreadId, turn: String },
}

impl Phase {
    fn thread(&self) -> Option<&ThreadId> {
        match self {
            Phase::Config | Phase::Starting => None,
            Phase::Beginning { thread } | Phase::Waiting { thread, .. } => Some(thread),
        }
    }
}

/// What the stream said of the temporary thread.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Seen {
    /// An assistant message of the turn `turn` was completed.
    Message { turn: String, text: String },
    /// The turn `turn` ended.
    Completed { turn: String, end: TurnEnd },
}

impl Runs {
    /// A run is in flight (one of them is waiting for the stream or for an RPC).
    pub(super) fn is_empty(&self) -> bool {
        self.runs.is_empty()
    }

    /// The runs that have an RPC in flight (every phase but the wait for the stream), by number.
    pub(super) fn asking(&self) -> impl Iterator<Item = u64> + '_ {
        self.runs
            .iter()
            .filter(|(_, run)| !matches!(run.phase, Phase::Waiting { .. }))
            .map(|(id, _)| *id)
    }

    /// The moment the nearest run takes too long.
    pub(super) fn wake_at(&self) -> Option<Now> {
        self.runs
            .values()
            .filter_map(|run| run.deadline)
            .reduce(Now::min)
    }
}

/// A feature asked for a structured turn: the first RPC goes out.
pub(super) fn begin(app: &mut CodexApp, turn: StructuredTurn, ctx: FeatureCtx, fx: &mut AppFx) {
    let backend = app.screens.agents.owner_of(turn.source.as_str());
    let cwd = directory(app, &turn.source);
    let id = app.structured.next;
    app.structured.next += 1;
    app.structured.runs.insert(
        id,
        Run {
            ctx,
            turn,
            backend,
            phase: Phase::Config,
            seen: Vec::new(),
            deadline: deadline(app.now),
        },
    );
    ask(app, id, ThreadAsk::McpServers { cwd }, fx);
}

/// The directory of the thread the request is about, as the server told it.
fn directory(app: &CodexApp, source: &ThreadId) -> Option<String> {
    app.threads
        .get(source)
        .and_then(|model| model.screen_slots().chat.cwd.clone())
}

/// A phase that starts at `now` is late at this moment; none until the shell has told the time.
fn deadline(now: Now) -> Option<Now> {
    (now != Now::default()).then(|| now.plus_millis(TIMEOUT_MILLIS))
}

/// One RPC of the run.
fn ask(app: &mut CodexApp, id: u64, ask: ThreadAsk, fx: &mut AppFx) {
    let Some(run) = app.structured.runs.get(&id) else {
        return;
    };
    let backend = run.backend;
    let ticket = app.mint.next();
    app.outgoing
        .insert(ticket, AppCtx::Structured { run: id, backend });
    fx.commands.push(Command::Screen {
        ticket,
        thread: ThreadId::default(),
        command: ScreenCommand::Agents(agents::Command::Thread(ask)),
    });
}

/// An RPC nobody waits for (`thread/unsubscribe`, `turn/interrupt`): the answer is dropped.
fn tell(app: &mut CodexApp, ask: ThreadAsk, fx: &mut AppFx) {
    let ticket = app.mint.next();
    fx.commands.push(Command::Screen {
        ticket,
        thread: ThreadId::default(),
        command: ScreenCommand::Agents(agents::Command::Thread(ask)),
    });
}

/// The answer of the run's RPC in flight.
pub(super) fn answered(app: &mut CodexApp, id: u64, answer: Answer, fx: &mut AppFx) {
    let Some(phase) = app.structured.runs.get(&id).map(|run| run.phase.clone()) else {
        return;
    };
    let reply = match answer {
        Answer::Screen(ScreenReply::Agents(reply)) => reply,
        Answer::Failed(error) => return finish(app, id, Err(Why::Refused(error)), fx),
        _ => return finish(app, id, Err(Why::Refused(RpcError::Mismatch)), fx),
    };
    match (phase, reply) {
        (Phase::Config, agents::Reply::McpServers(servers)) => start(app, id, servers, fx),
        (Phase::Starting, agents::Reply::Temporary { thread, as_asked }) => {
            started(app, id, thread, as_asked, fx);
        }
        (Phase::Beginning { thread }, agents::Reply::StructuredTurn { turn }) => {
            if let Some(run) = app.structured.runs.get_mut(&id) {
                run.phase = Phase::Waiting { thread, turn };
                run.deadline = deadline(app.now);
            }
            settle(app, id, fx);
        }
        _ => finish(app, id, Err(Why::Refused(RpcError::Mismatch)), fx),
    }
}

/// The MCP servers are known: the ephemeral thread starts with every one of them disabled.
fn start(
    app: &mut CodexApp,
    id: u64,
    servers: serde_json::Map<String, serde_json::Value>,
    fx: &mut AppFx,
) {
    let Some(run) = app.structured.runs.get(&id) else {
        return;
    };
    let source = app.threads.get(&run.turn.source);
    let slots = source.map(|model| &model.screen_slots().chat);
    let status = &app.screens.chat.status;
    let temporary = Temporary {
        kind: run.turn.kind,
        model: run
            .turn
            .model
            .clone()
            .or_else(|| slots.and_then(|chat| chat.model.clone()))
            .or_else(|| status.model.clone()),
        provider: slots
            .and_then(|chat| chat.provider.clone())
            .or_else(|| status.provider.clone()),
        cwd: directory(app, &run.turn.source),
        profile: status.permission_profile.clone(),
        mcp_servers: servers,
    };
    if let Some(run) = app.structured.runs.get_mut(&id) {
        run.phase = Phase::Starting;
        run.deadline = deadline(app.now);
    }
    ask(app, id, ThreadAsk::StartTemporary(Box::new(temporary)), fx);
}

/// The ephemeral thread started: it is checked for the permissions that were asked for, and its
/// one turn starts.
fn started(app: &mut CodexApp, id: u64, thread: String, as_asked: bool, fx: &mut AppFx) {
    let Some(run) = app.structured.runs.get(&id) else {
        return;
    };
    if !as_asked {
        tell(app, ThreadAsk::Unsubscribe { thread }, fx);
        return finish(app, id, Err(Why::NotReadOnly), fx);
    }
    // The feature may have gone stale while the thread started: the turn never starts.
    if !features::structured_wanted(app, &run.ctx) {
        tell(app, ThreadAsk::Unsubscribe { thread }, fx);
        return finish(app, id, Err(Why::Unwanted), fx);
    }
    let backend = run.backend;
    let turn = ThreadAsk::StructuredTurn {
        thread: thread.clone(),
        kind: run.turn.kind,
        prompt: run.turn.prompt.clone(),
        schema: run.turn.schema.clone(),
        effort: run.turn.effort.clone(),
    };
    // The thread belongs to the backend that made it: what is said of it afterwards goes there.
    Rc::make_mut(&mut app.screens)
        .agents
        .observe(backend, &thread);
    if let Some(run) = app.structured.runs.get_mut(&id) {
        run.phase = Phase::Beginning {
            thread: ThreadId::new(thread),
        };
        run.deadline = deadline(app.now);
    }
    ask(app, id, turn, fx);
}

/// A notification of the server's stream: what it says of the temporary thread of a run is kept,
/// and a turn that ended settles the run.
pub(super) fn observe(app: &mut CodexApp, notification: &ServerNotification, fx: &mut AppFx) {
    if app.structured.is_empty() {
        return;
    }
    let (thread, seen) = match notification {
        ServerNotification::ItemCompleted(done) => match &done.item.payload {
            ThreadItemPayload::AgentMessage { text, .. } => (
                &done.thread_id,
                Seen::Message {
                    turn: done.turn_id.clone(),
                    text: text.clone(),
                },
            ),
            _ => return,
        },
        ServerNotification::TurnCompleted(done) => (
            &done.thread_id,
            Seen::Completed {
                turn: done.turn.id.clone(),
                end: match done.turn.status {
                    TurnStatus::Completed => TurnEnd::Completed,
                    TurnStatus::Interrupted => TurnEnd::Interrupted,
                    TurnStatus::Failed | TurnStatus::InProgress => TurnEnd::Failed,
                },
            },
        ),
        _ => return,
    };
    let Some((id, run)) = app.structured.runs.iter_mut().find(|(_, run)| {
        run.phase
            .thread()
            .is_some_and(|ours| ours.as_str() == thread)
    }) else {
        return;
    };
    run.seen.push(seen);
    let id = *id;
    settle(app, id, fx);
}

/// `collect_structured_response`: the last assistant message of the turn once it completed; a
/// message that is too long, or a turn that did not complete, is the run's end.
fn settle(app: &mut CodexApp, id: u64, fx: &mut AppFx) {
    let Some(run) = app.structured.runs.get(&id) else {
        return;
    };
    let Phase::Waiting { turn, .. } = &run.phase else {
        return;
    };
    let mut response = None;
    let mut result = None;
    for seen in &run.seen {
        match seen {
            Seen::Message { turn: of, text } if of == turn => {
                if text.len() > RESPONSE_MAX_BYTES {
                    result = Some(Err(Why::TooLong));
                    break;
                }
                response = Some(text.clone());
            }
            Seen::Completed { turn: of, end } if of == turn => {
                result = Some(match end {
                    TurnEnd::Completed => response.take().ok_or(Why::Empty),
                    other => Err(Why::Ended(*other)),
                });
                break;
            }
            _ => {}
        }
    }
    if let Some(result) = result {
        finish(app, id, result, fx);
    }
}

/// The run is over: a turn that is still running is stopped when it was the clock that ended
/// the run, the temporary thread is let go, and the feature is told.
fn finish(app: &mut CodexApp, id: u64, result: Result<String, Why>, fx: &mut AppFx) {
    let Some(run) = app.structured.runs.remove(&id) else {
        return;
    };
    if let Phase::Waiting { thread, turn } = &run.phase
        && result == Err(Why::TimedOut)
    {
        let interrupt = ThreadAsk::Interrupt {
            thread: thread.to_string(),
            turn: turn.clone(),
        };
        tell(app, interrupt, fx);
    }
    if let Some(thread) = run.phase.thread().filter(|_| result != Err(Why::Lost)) {
        let unsubscribe = ThreadAsk::Unsubscribe {
            thread: thread.to_string(),
        };
        tell(app, unsubscribe, fx);
    }
    features::structured_done(app, run.ctx, result, fx);
}

/// The clock reached the deadline of a run: it ends, as a phase that took longer than the
/// oracle's 30 seconds does.
pub(super) fn expire(app: &mut CodexApp, now: Now, fx: &mut AppFx) {
    let late: Vec<u64> = app
        .structured
        .runs
        .iter()
        .filter(|(_, run)| run.deadline.is_some_and(|due| now.reached(due)))
        .map(|(id, _)| *id)
        .collect();
    for id in late {
        // a request in flight is let go: its answer finds no run
        app.outgoing
            .retain(|ctx| !matches!(ctx, AppCtx::Structured { run, .. } if *run == id));
        finish(app, id, Err(Why::TimedOut), fx);
    }
}

/// The connection of `backend` was lost: a run that waits for its stream will not hear it end
/// (a request in flight is failed by the connection's own reset).
pub(super) fn lost(app: &mut CodexApp, backend: BackendId, fx: &mut AppFx) {
    let waiting: Vec<u64> = app
        .structured
        .runs
        .iter()
        .filter(|(_, run)| run.backend == backend && matches!(run.phase, Phase::Waiting { .. }))
        .map(|(id, _)| *id)
        .collect();
    for id in waiting {
        finish(app, id, Err(Why::Lost), fx);
    }
}
