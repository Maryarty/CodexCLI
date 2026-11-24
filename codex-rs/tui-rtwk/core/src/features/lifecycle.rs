//! The doors of a thread's life that a feature has (ADR-012, "lifecycle doors"; ADR-003): a feature
//! opens a thread, spawns one (a fork, a fresh start), drops one, sends a message in one or gives
//! text back to a composer. The app owns every `ThreadModel`; a feature says what it wants through
//! `AppCx` (`open_thread`, `spawn_thread`, `discard`, `submit`, `give_back`) and the app runs it through
//! its own steps (`app/lifecycle.rs`), as `thread_routing.rs` and `session_lifecycle.rs` of the
//! oracle do. What the lifecycle RPCs are stays in `threads.rs` (`ThreadAsk`), the feature never
//! builds one.
//!
//! The answers come back to the feature's `thread_opened` (`open_thread`, `spawn_thread`), under the
//! `ctx` the feature gave.

use super::FeatureCtx;
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::threads::UserMessage;

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
    /// Drop `thread`: its turn is interrupted, the server stops holding it and the app forgets it.
    /// Never the thread in use.
    Discard { thread: ThreadId },
    /// The user sent `message` in `thread`: the thread's own input pipeline takes it (a running
    /// turn is steered, a busy thread queues it).
    Submit {
        thread: ThreadId,
        message: UserMessage,
    },
    /// Text that could not go is given back to `thread`'s composer.
    GiveBack { thread: ThreadId, text: String },
}
