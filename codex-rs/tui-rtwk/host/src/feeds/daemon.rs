//! The `/daemon` slice's host half (oracle `app/daemon_menu.rs`, `lib.rs`
//! `daemon_cli_executable`, `remote_connection_status_value`): which server the app is
//! connected to, the CLI that can manage it, and the connection once it is up. The menu's rows
//! follow these facts (`daemon::Facts::page`). In Flutter this is the repository the menu's
//! provider reads.
//!
//! `Connection::managed` is never told: `/daemon` probes it when it opens
//! (`Verb::Open`, `Ask::BackgroundServer(Verify)`) and only then enables the restart. `offline`
//! (the connection is lost and the app reconnects) is not told either: no feed reports a loss.

use std::path::Path;

use codex_install_context::InstallContext;
use codex_tui_core::features::daemon::{Cli, Connection, Facts, Target as Server};

use super::{Event, Feed, FeedCx};
use crate::startup::Target;

/// The facts at start (the CLI, no connection yet) and each time a connection is up.
pub(crate) fn feed(event: &Event<'_>, cx: &FeedCx<'_>, out: &mut Vec<Feed>) {
    if let Some(facts) = facts(event, cx.target, cli) {
        out.push(Feed::fact(facts));
    }
}

/// What `event` tells of the server; `cli` reads the launching executable, only when it is asked.
fn facts(event: &Event<'_>, target: Target, cli: impl FnOnce() -> Option<Cli>) -> Option<Facts> {
    let connection = match event {
        Event::Startup => None,
        Event::Connection {
            server_version,
            local_daemon,
        } => Some(Connection {
            // `remote_connection_status_value`: `v{version}`, or `unknown`.
            version: server_version.map_or_else(|| "unknown".to_owned(), |v| format!("v{v}")),
            local_daemon: *local_daemon,
            managed: false,
        }),
        _ => return None,
    };
    Some(Facts {
        target: match target {
            Target::Embedded => Server::Embedded,
            Target::LocalDaemon => Server::LocalDaemon,
            Target::Remote => Server::Remote,
        },
        connection,
        cli: cli(),
        offline: false,
    })
}

/// The CLI that runs the app, when it can manage the daemon (`daemon_cli_executable`: the
/// launching executable, canonical; none when it cannot be located).
fn cli() -> Option<Cli> {
    let path = std::env::current_exe().ok()?.canonicalize().ok()?;
    let context = InstallContext::from_exe(
        cfg!(target_os = "macos"),
        Some(path.as_path()),
        /*method_override*/ None,
    );
    // The version `Use this CLI build` copies: the package's, else the build's.
    let version = InstallContext::current().package_manifest().map_or_else(
        || env!("CARGO_PKG_VERSION").to_owned(),
        |m| m.version.to_string(),
    );
    Some(cli_of(&path, &context, version))
}

/// The CLI at `path` installed as `context` says.
fn cli_of(path: &Path, context: &InstallContext, version: String) -> Cli {
    Cli {
        path: path.display().to_string(),
        has_package: context.package_layout.is_some(),
        version,
    }
}

#[cfg(test)]
mod tests {
    use codex_install_context::{CodexPackageLayout, InstallMethod};
    use codex_tui_core::features::daemon::Status;
    use codex_utils_absolute_path::AbsolutePathBuf;

    use super::*;

    fn some_cli() -> Option<Cli> {
        Some(Cli {
            path: "/usr/bin/codex".to_owned(),
            has_package: true,
            version: "0.153.0".to_owned(),
        })
    }

    #[test]
    fn the_daemon_facts_follow_the_connection() {
        // At start: the CLI is known, no connection yet.
        let start = facts(&Event::Startup, Target::LocalDaemon, some_cli).expect("start facts");
        assert!(start.cli.is_some());
        assert_eq!(start.connection, None);
        assert_eq!(start.page(false).status, Status::NotConnected);
        // The daemon is up: the page says which version runs; managed waits for the probe.
        let up = facts(
            &Event::Connection {
                server_version: Some("0.153.0"),
                local_daemon: true,
            },
            Target::LocalDaemon,
            some_cli,
        )
        .expect("connection facts");
        assert_eq!(
            up.page(false).status,
            Status::Running("v0.153.0".to_owned())
        );
        assert_eq!(
            up.connection,
            Some(Connection {
                version: "v0.153.0".to_owned(),
                local_daemon: true,
                managed: false,
            })
        );
        // A server that does not say its version.
        let unknown = facts(
            &Event::Connection {
                server_version: None,
                local_daemon: true,
            },
            Target::LocalDaemon,
            some_cli,
        )
        .expect("connection facts");
        assert_eq!(
            unknown.page(false).status,
            Status::Running("unknown".to_owned())
        );
        // The other events tell nothing, and do not read the executable.
        assert!(facts(&Event::Reconnected, Target::LocalDaemon, || panic!("read")).is_none());
        assert!(facts(&Event::ConfigWritten, Target::Embedded, || panic!("read")).is_none());
    }

    #[test]
    fn the_cli_facts_say_whether_the_package_is_there() {
        let root = std::env::temp_dir().join("native-release");
        let dir = |name: &str| AbsolutePathBuf::from_absolute_path(root.join(name)).expect("path");
        let bare = InstallContext {
            method: InstallMethod::Other,
            package_layout: None,
        };
        let packaged = InstallContext {
            method: InstallMethod::Other,
            package_layout: Some(CodexPackageLayout {
                package_dir: dir("package"),
                bin_dir: dir("package/bin"),
                resources_dir: None,
                path_dir: None,
            }),
        };
        let at = Path::new("/usr/bin/codex");
        assert!(!cli_of(at, &bare, "1.0.0".to_owned()).has_package);
        let cli = cli_of(at, &packaged, "1.0.0".to_owned());
        assert!(cli.has_package);
        assert_eq!(
            (cli.path.as_str(), cli.version.as_str()),
            ("/usr/bin/codex", "1.0.0")
        );
    }
}
