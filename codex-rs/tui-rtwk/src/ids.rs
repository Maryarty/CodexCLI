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

/// The id of one of our own RPCs; its reply carries it back (R1, R3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Ticket(pub u64);

/// A request the user answers: scoped by connection, thread and request id
/// (R3).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RequestKey {
    pub epoch: ConnEpoch,
    pub thread: ThreadId,
    pub id: ServerRequestId,
}
