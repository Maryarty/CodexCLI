//! The words of `/daemon` (oracle `app/daemon_menu.rs`, `event_dispatch.rs` `RunDaemonRestart`):
//! its transcript notes. The wording is the oracle's English until the catalog has these entries.

use codex_tui_core::features::daemon::Note;

/// The words of one of the feature's notes.
pub fn note(note: &Note) -> String {
    match note {
        Note::RestartFailed(error) => format!("Could not restart the background server: {error}"),
    }
}

use crate::widgets::picker::list::{Columns, Item, List, Role, Span};
use codex_tui_core::features::daemon::{
    Cli, Guidance, Menu, Page, Reason, Row, RowState, Source, Status, Verb,
};

/// The ids of the rows (what a pick returns).
pub const INSTALL_STABLE: &str = "daemon.stable";
pub const USE_THIS_CLI: &str = "daemon.this-cli";
pub const RESTART: &str = "daemon.restart";
pub const CANCEL: &str = "daemon.cancel";
pub const CONFIRM: &str = "daemon.confirm";

/// What picking row `id` of `menu` says: `Some(None)` for a row that only closes, `None` for an
/// id the page does not have.
pub fn verb_of(menu: Menu, id: &str) -> Option<Option<Verb>> {
    Some(match (menu, id) {
        (Menu::Main, INSTALL_STABLE) => Some(super::sheet::row_verb(Row::Update(Source::PublicStable))),
        (Menu::Main, USE_THIS_CLI) => Some(super::sheet::row_verb(Row::Update(Source::ThisCli))),
        (Menu::Main, RESTART) => Some(super::sheet::row_verb(Row::Restart)),
        (Menu::ConfirmRestart | Menu::ConfirmUpdate(_), CANCEL) => None,
        (Menu::ConfirmRestart, CONFIRM) => Some(Verb::Restart),
        (Menu::ConfirmUpdate(source), CONFIRM) => Some(Verb::Update(source)),
        _ => return None,
    })
}

fn line(text: &str, role: Role) -> Vec<Span> {
    vec![Span {
        text: text.to_owned(),
        role,
        link: None,
    }]
}

fn status(status: &Status) -> String {
    match status {
        Status::Running(version) => format!("Running daemon: {version}"),
        Status::NotConnected => "Not connected to the local background server.".to_owned(),
    }
}

fn guidance(guidance: Guidance) -> &'static str {
    match guidance {
        Guidance::Remote => {
            "Manage this server on its host. Local daemon updates are unavailable for remote connections."
        }
        Guidance::NoCli => "Run the Codex CLI to manage the daemon from this menu.",
    }
}

fn reason(reason: Reason) -> &'static str {
    match reason {
        Reason::NoPackage => "This CLI has no local package to copy",
        Reason::Unverified => {
            "Codex hasn't verified it manages this server. Restart it where it was started."
        }
    }
}

fn row(state: RowState) -> Item {
    let (id, name) = match state.row {
        Row::Update(Source::PublicStable) => (INSTALL_STABLE, "Install latest public stable"),
        Row::Update(Source::ThisCli) => (USE_THIS_CLI, "Use this CLI build"),
        Row::Restart => (RESTART, "Restart background server"),
    };
    Item {
        id: id.to_owned(),
        name: name.to_owned(),
        disabled: state.disabled,
        disabled_reason: state.reason.map(|why| reason(why).to_owned()),
        ..Item::default()
    }
}

/// The explanation of an update (`confirm_daemon_update`), one line per paragraph line.
fn explanation(source: Source, cli: Option<&Cli>) -> Vec<String> {
    let first = match (source, cli) {
        (Source::PublicStable, _) => "Install the latest public stable release. Restore production updates; keep your automatic-update setting.".to_owned(),
        (Source::ThisCli, cli) => format!(
            "Use this CLI package v{} from {}. Copy the complete package and pin it against automatic updates.",
            cli.map_or("", |cli| cli.version.as_str()),
            cli.map_or("", |cli| cli.path.as_str()),
        ),
    };
    vec![
        first,
        "This may restart the daemon and interrupt active or queued work.".to_owned(),
        "Codex exits to update in this terminal. Relaunch it afterward.".to_owned(),
    ]
}

fn item(id: &str, name: &str) -> Item {
    Item {
        id: id.to_owned(),
        name: name.to_owned(),
        ..Item::default()
    }
}

/// The list of one page (`SelectionViewParams::picker()` / `confirmation()`).
pub fn menu(menu: Menu, page: &Page, cli: Option<&Cli>) -> List {
    let (header, items) = match menu {
        Menu::Main => {
            let mut header = vec![
                line("Daemon", Role::Strong),
                line(&status(&page.status), Role::Dim),
            ];
            if let Some(why) = page.guidance {
                header.push(line(guidance(why), Role::Dim));
            }
            (header, page.rows.iter().copied().map(row).collect())
        }
        Menu::ConfirmRestart => (
            vec![
                line("Restart background server?", Role::Strong),
                line(
                    "Active or queued work in other chats may be interrupted.",
                    Role::Plain,
                ),
                line("Codex will reconnect automatically.", Role::Plain),
            ],
            vec![item(CANCEL, "Cancel"), item(CONFIRM, "Restart")],
        ),
        Menu::ConfirmUpdate(source) => {
            let mut header = vec![line("Update daemon and exit Codex?", Role::Strong)];
            header.extend(
                explanation(source, cli)
                    .iter()
                    .map(|text| line(text, Role::Plain)),
            );
            (
                header,
                vec![item(CANCEL, "Cancel"), item(CONFIRM, "Update and exit")],
            )
        }
    };
    List {
        header,
        standard_hint: true,
        items,
        columns: Columns::AutoAllRows,
        hide_description_below: Some(24),
        panel: true,
        ..List::default()
    }
}
