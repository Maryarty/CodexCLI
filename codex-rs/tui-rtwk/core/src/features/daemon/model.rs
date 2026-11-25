//! The model of the `/daemon` menu (oracle `app/daemon_menu.rs`): what the host tells of the
//! connection (`Facts`), the page the menu derives from it, and the work in flight (`State`).
//! The rows are derived from the facts by `Facts::page` (never stored), so a sheet and a test
//! read the same enabled flags and reasons. A verb (`Verb`) changes the state and says what
//! follows (`Step`): a page to show, or an ask of the host.

use crate::app::effects::DaemonAsk;

/// Which server the app is connected to (`AppServerTarget`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Target {
    /// The in-process server.
    #[default]
    Embedded,
    /// The local background server.
    LocalDaemon,
    /// A server on another host.
    Remote,
    /// The cloud.
    Cloud,
}

/// The connection to a server other than the in-process one (`RemoteConnectionStatus`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Connection {
    /// `v0.153.0` as the server reports it.
    pub version: String,
    pub local_daemon: bool,
    /// The server is a daemon this CLI manages (proved by the version probe, never assumed
    /// from a socket alone).
    pub managed: bool,
}

/// The CLI that runs the app, when it can manage the daemon (`daemon_cli_executable`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cli {
    /// Where the executable is (the confirmation says it).
    pub path: String,
    /// Its install has a package layout the daemon can be pointed at.
    pub has_package: bool,
    /// The version of that package (the manifest's, else the build's).
    pub version: String,
}

/// What the host tells about the server: sent when a connection is made or lost.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Facts {
    pub target: Target,
    pub connection: Option<Connection>,
    pub cli: Option<Cli>,
    /// The connection was lost and the app is reconnecting (`reconnect.offline`).
    pub offline: bool,
}

/// Where an update comes from (`DaemonUpdateSource`): the host runs it after the exit, so the
/// type is the app's (`effects`), the feature's name for it.
pub use crate::app::effects::DaemonSource as Source;

/// The page the sheet opens on, or goes to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Menu {
    /// The menu: status, guidance, the updates and the restart.
    Main,
    /// `Restart background server?`: Cancel, then Restart.
    ConfirmRestart,
    /// `Update daemon and exit Codex?`: Cancel, then Update and exit.
    ConfirmUpdate(Source),
}

/// What the header says about the server.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    /// `Running daemon: {version}`.
    Running(String),
    /// `Not connected to the local background server.`
    NotConnected,
}

/// Why the rows are all disabled (the third header line).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Guidance {
    /// `Manage this server on its host. Local daemon updates are unavailable for remote
    /// connections.`
    Remote,
    /// `Run the Codex CLI to manage the daemon from this menu.`
    NoCli,
}

/// Why one row is disabled, where it says so.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    /// `This CLI has no local package to copy`.
    NoPackage,
    /// `Codex hasn't verified it manages this server. Restart it where it was started.`
    Unverified,
}

/// One row of the main page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Row {
    Update(Source),
    Restart,
}

/// A row with whether it can be picked and why not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowState {
    pub row: Row,
    pub disabled: bool,
    pub reason: Option<Reason>,
}

/// The main page as the oracle builds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Page {
    pub status: Status,
    pub guidance: Option<Guidance>,
    pub rows: Vec<RowState>,
}

impl Facts {
    fn managed(&self) -> bool {
        self.connection.as_ref().is_some_and(|c| c.managed)
    }

    /// Updates cannot run: another host's server, or no CLI to run them with.
    fn guidance(&self) -> Option<Guidance> {
        if matches!(self.target, Target::Remote | Target::Cloud) {
            Some(Guidance::Remote)
        } else if self.cli.is_none() {
            Some(Guidance::NoCli)
        } else {
            None
        }
    }

    /// The main page (`open_daemon_menu`); `restarting` is a restart in flight.
    pub fn page(&self, restarting: bool) -> Page {
        let status = match (&self.connection, self.target) {
            (Some(connection), Target::LocalDaemon) => Status::Running(connection.version.clone()),
            _ => Status::NotConnected,
        };
        let guidance = self.guidance();
        let has_package = self.cli.as_ref().is_some_and(|cli| cli.has_package);
        let mut rows: Vec<RowState> = [Source::PublicStable, Source::ThisCli]
            .into_iter()
            .map(|source| RowState {
                row: Row::Update(source),
                disabled: guidance.is_some(),
                reason: (guidance.is_none() && source == Source::ThisCli && !has_package)
                    .then_some(Reason::NoPackage),
            })
            .collect();
        rows.push(RowState {
            row: Row::Restart,
            disabled: !self.managed() || self.offline || restarting,
            reason: self
                .connection
                .as_ref()
                .filter(|c| c.local_daemon && !c.managed)
                .map(|_| Reason::Unverified),
        });
        Page {
            status,
            guidance,
            rows,
        }
    }
}

/// What the sheet asks of the model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verb {
    /// `/daemon`: the connection is probed when a local server is not yet known to be managed,
    /// then the menu opens.
    Open,
    /// The Restart row: the confirmation opens (nothing where the restart is not allowed).
    ConfirmRestart,
    /// `Restart` of the confirmation: the host restarts the server.
    Restart,
    /// An update row: the confirmation opens (nothing where updates are not allowed).
    ConfirmUpdate(Source),
    /// `Update and exit`: Codex exits and the update runs in this terminal.
    Update(Source),
}

/// What a verb leaves to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// Show this page of the menu.
    Show(Menu),
    /// Ask the host (the probe or the restart).
    Ask(DaemonAsk),
}

/// The model of the menu: the host's facts and the work in flight.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    pub facts: Facts,
    /// A restart is running (`daemon_restart.is_some()`).
    pub restarting: bool,
    /// The probe is out; the menu opens when it answers.
    verifying: bool,
    /// `RunDaemonUpdate` is out: the host keeps this update and the app ends when it answers.
    update: Option<Source>,
}

impl State {
    /// The page of the main menu now.
    pub fn page(&self) -> Page {
        self.facts.page(self.restarting)
    }

    /// The host told the server's state (a connection made or lost). A reconnect to the local
    /// daemon keeps what the probe proved of it (`reconnect.rs`: `is_managed_daemon` carries over).
    pub fn told(&mut self, mut facts: Facts) {
        let proved = self.facts.managed();
        if let Some(connection) = facts.connection.as_mut() {
            connection.managed |= connection.local_daemon && proved;
        }
        self.facts = facts;
    }

    /// A verb of the sheet; what follows, if anything.
    pub fn verb(&mut self, verb: Verb) -> Option<Step> {
        match verb {
            Verb::Open => {
                let unknown = self
                    .facts
                    .connection
                    .as_ref()
                    .is_some_and(|c| c.local_daemon && !c.managed);
                if !self.facts.offline && unknown && !self.verifying {
                    self.verifying = true;
                    return Some(Step::Ask(DaemonAsk::Verify));
                }
                (!self.verifying).then_some(Step::Show(Menu::Main))
            }
            Verb::ConfirmRestart => (self.facts.managed() && !self.restarting)
                .then_some(Step::Show(Menu::ConfirmRestart)),
            Verb::Restart => {
                if self.facts.managed() && !self.restarting {
                    self.restarting = true;
                    return Some(Step::Ask(DaemonAsk::Restart));
                }
                None
            }
            Verb::ConfirmUpdate(source) => (self.facts.cli.is_some()
                && !matches!(self.facts.target, Target::Remote | Target::Cloud))
            .then_some(Step::Show(Menu::ConfirmUpdate(source))),
            Verb::Update(source) => {
                if self.update.is_some() {
                    return None;
                }
                self.update = Some(source);
                Some(Step::Ask(DaemonAsk::Update(source)))
            }
        }
    }

    /// The probe answered: the connection is a managed daemon or not; the menu opens.
    pub fn verified(&mut self, managed: bool) -> Option<Menu> {
        if !self.verifying {
            return None;
        }
        self.verifying = false;
        if let Some(connection) = self.facts.connection.as_mut() {
            connection.managed = managed;
        }
        Some(Menu::Main)
    }

    /// The restart ended: nothing, or why it could not (`Could not restart the background
    /// server: {error}`, an error line of the conversation).
    pub fn restarted(&mut self, result: Result<(), String>) -> Option<String> {
        self.restarting = false;
        result.err()
    }
}
