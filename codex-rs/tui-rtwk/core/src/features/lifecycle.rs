//! The doors of a thread's life that a feature has (ADR-012, "lifecycle doors"; ADR-003): a feature
//! opens a thread, spawns one (a fork, a fresh start), drops one, sends a message in one or gives
//! text back to a composer. The app owns every `ThreadModel`; a feature says what it wants through
//! `AppCx` (`open_thread`, `spawn_thread`, `discard`, `submit`, `give_back`) and the app runs it through
//! its own steps (`app/lifecycle.rs`), as `thread_routing.rs` and `session_lifecycle.rs` of the
//! oracle do. What the lifecycle RPCs are stays in `threads.rs` (`ThreadAsk`), the feature never
//! builds one.
//!
//! The answers come back to the feature's `thread_opened` (`open_thread`, `spawn_thread`) and
//! `structured_done` (`structured_turn`), under the `ctx` the feature gave.

use super::FeatureCtx;
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::threads::UserMessage;
use crate::threads::structured::StructuredTurn;

/// A thread a feature asks the app to make (`AppCx::spawn`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThreadSpawn {
    /// A new thread (`thread/start`) on the backend of the thread in use.
    Start,
    /// A copy of `parent` (`thread/fork`). `ephemeral` holds the developer instructions of an
    /// ephemeral fork: the server does not keep it and it starts empty on the screen, carrying
    /// them (a side conversation); `None` is a fork that keeps the parent's history on screen.
    Fork {
        parent: ThreadId,
        ephemeral: Option<String>,
    },
    /// Hold a side fork offscreen until the feature confirms its context is ready.
    SideFork {
        parent: ThreadId,
        temporary_policy: String,
        attached_policy: String,
    },
}

/// Why a thread a feature asked for did not come (`Feature::thread_opened`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The app was already making or opening a conversation (one at a time: `/new`, `/fork`,
    /// `/side` and `/resume` share it) or shows an error that holds it: nothing was asked of the
    /// server.
    Busy,
    /// The server could not read, resume or make the thread.
    Failed(RpcError),
}

/// How the app was connected to its server, told to every feature (`Feature::on_connection`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Connection {
    /// A connection is up (the first, or a reconnect).
    Up,
    /// The backend stopped: what waited for it will not come.
    Lost,
    /// The host is going to quit: what a feature holds on the server is let go.
    ShuttingDown,
}

/// What a feature asked of a thread's life, in order (`Out::Lifecycle`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Lifecycle {
    /// Show `thread`, resuming it when this process does not hold it; answered by
    /// `thread_opened` (`Err` when the server has no such thread).
    Open { thread: ThreadId, ctx: FeatureCtx },
    /// Make a thread; answered by `thread_opened` once it is open here (before it is shown).
    Spawn { spawn: ThreadSpawn, ctx: FeatureCtx },
    /// An isolated request to the model in an ephemeral thread of its own, answered by
    /// `structured_done` with the last message of its one turn.
    Structured {
        turn: Box<StructuredTurn>,
        ctx: FeatureCtx,
    },
    /// Drop `thread`: its turn is interrupted, the server stops holding it and the app forgets it.
    /// Never the thread in use.
    Discard {
        thread: ThreadId,
        rollback: Option<FeatureCtx>,
    },
    /// Make a fork available after the feature's prerequisite RPC has succeeded.
    FinishPreparation { thread: ThreadId },
    /// The user sent `message` in `thread`: the thread's own input pipeline takes it (a running
    /// turn is steered, a busy thread queues it).
    Submit {
        thread: ThreadId,
        message: UserMessage,
    },
    /// Text that could not go is given back to `thread`'s composer.
    GiveBack { thread: ThreadId, text: String },
    /// `thread` leaves no notice in its transcript when a turn is interrupted (`quiet`), or
    /// does again: the flag is the thread's (a side conversation sets it on its own thread).
    QuietInterrupts { thread: ThreadId, quiet: bool },
}
