//! One newtype per id kind (R13). Ids that are only unique per scope carry
//! the scope in a composite key (`RequestKey`).

use std::fmt;

macro_rules! string_id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(pub String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

string_id!(
    /// A conversation thread of the app-server.
    ThreadId
);
impl std::borrow::Borrow<str> for ThreadId {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

string_id!(
    /// A working directory as the app-server and a feature's data name it (the key a feature's
    /// per-directory answers are cached under, ADR-013 З).
    Cwd
);

string_id!(
    /// A turn inside a thread.
    TurnId
);
string_id!(
    /// An item inside a turn. Unique only per turn.
    ItemId
);
string_id!(
    /// A server request id (approval, elicitation, user input), the JSON-RPC
    /// id rendered as JSON. Unique only per connection.
    ServerRequestId
);

/// The generation of the backend connection. Every event carries the epoch
/// it was attached under; the ingress router drops older ones (R1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ConnEpoch(pub u64);

impl ConnEpoch {
    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

/// One backend of the command center (oracle: `backend_id::BackendId`): the
/// local server or the cloud. Rows, pins and discovery are kept per backend;
/// `BackendId::default()` is the one the first connection is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BackendId(pub u32);

/// The id of one of our own RPCs; its reply carries it back (R1, R3). Not
/// forgeable: only the app's `TicketMint` makes one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Ticket(u64);

impl Ticket {
    pub(crate) fn new(n: u64) -> Self {
        Self(n)
    }

    /// The number the ticket was minted with (a domain counter for `ThreadModel::submit_seq`).
    pub(crate) fn seq(self) -> u64 {
        self.0
    }
}

/// The one source of tickets, kept by the app.
#[derive(Clone, Debug, Default)]
pub struct TicketMint {
    sequence: u64,
    submission_namespace: String,
}

impl TicketMint {
    pub(crate) fn next(&mut self) -> Ticket {
        self.sequence += 1;
        Ticket::new(self.sequence)
    }

    pub(crate) fn set_submission_namespace(&mut self, namespace: String) {
        self.submission_namespace = namespace;
    }

    pub(crate) fn submission_id(&self, ticket: Ticket) -> String {
        format!("{}:{}", self.submission_namespace, ticket.seq())
    }
}

/// A request the user answers: scoped by connection, thread and request id
/// (R3).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RequestKey {
    pub backend: BackendId,
    pub epoch: ConnEpoch,
    pub thread: ThreadId,
    pub id: ServerRequestId,
}
