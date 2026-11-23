//! The connection fact: which kind of server the session talks to, and
//! whether its workspace is another machine's (oracle `AppServerTarget`,
//! `uses_remote_workspace`).
//!
//! The commands that read or write local files ask it: `/cd` is refused for
//! a remote workspace, `/export` writes under the launch directory instead
//! of the session's, the sign-in's browser callback and `/app` stay local.
//! The host builds the fact once per connection from its flags
//! (`host/src/target.rs`) and the backend kind.

/// What the session is connected to, per target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Connection {
    /// The in-process app server.
    Embedded,
    /// The local background daemon: another process, this machine's files.
    LocalDaemon,
    /// A server at an endpoint (`--remote`): another machine's files.
    Remote,
    /// Codex Cloud tasks: another machine's files.
    Cloud,
}

impl Connection {
    /// `uses_remote_workspace`: the workspace the session sees is not this
    /// machine's.
    pub fn uses_remote_workspace(self) -> bool {
        matches!(self, Self::Remote | Self::Cloud)
    }
}
