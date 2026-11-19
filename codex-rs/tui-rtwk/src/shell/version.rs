//! The words the command center shows when the server it talks to is older
//! than, or differs from, this client. Oracle: `update_versions.rs`
//! (`server_version_notice_kind`) and `app/server_version_notice.rs`
//! (`update_server_version_overview_notice`).

/// How the server differs from the client.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Older,
    Different,
}

/// Stable clients compare release precedence. Prerelease clients only order
/// versions within the same release line; local builds and other release
/// lines compare identity (oracle: `server_version_notice_kind`).
pub fn kind(client: &str, server: &str) -> Option<Kind> {
    let client = semver::Version::parse(client).ok()?;
    let server = semver::Version::parse(server).ok()?;
    let client_release = (client.major, client.minor, client.patch);
    let server_release = (server.major, server.minor, server.patch);
    let client_is_local = client_release == (0, 0, 0) || !client.build.is_empty();
    if client_is_local || (!client.pre.is_empty() && client_release != server_release) {
        return (client != server).then_some(Kind::Different);
    }
    (server.build.is_empty() && server_release != (0, 0, 0) && client > server)
        .then_some(Kind::Older)
}

/// The notice for `server` (a version without the leading `v`), or none.
/// `daemon`: the server is the local daemon, whose notice points at
/// `/daemon`.
pub fn notice(client: &str, server: &str, daemon: bool) -> Option<String> {
    let comparison = match kind(client, server)? {
        Kind::Older => "<",
        Kind::Different => "\u{2260}",
    };
    let guidance = if daemon { " \u{b7} /daemon" } else { "" };
    Some(format!(
        "Service v{server} {comparison} Codex CLI v{client}{guidance}"
    ))
}
