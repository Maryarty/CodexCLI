//! Clipboard copy: the routes and the worker that runs them (oracle
//! `clipboard_copy.rs`, `clipboard_copy/{worker,tmux,primary}.rs`).
//!
//! In Flutter this is `Clipboard.setData` over a platform channel: one call,
//! one answer, the machine behind it. Here the machine is the host's
//! (`codex-rtwk`, `host/src/clipboard.rs`: arboard, PowerShell on WSL, tmux,
//! X11 PRIMARY) behind the [`Routes`] trait; this module owns what the oracle
//! decides around it and what a test can pin without a machine:
//!
//! * the order of the routes ([`route`]): native first (the Windows clipboard
//!   on WSL, unless X11 is forwarded), then the terminal where a client may
//!   attach later (tmux, Herdr, ssh), the terminal only when native failed
//!   elsewhere; a terminal send is a request, never a confirmation;
//! * the terminal's payload cap and tmux's client choice ([`tmux_target`]);
//! * the worker ([`Clipboard`]): one thread owns the blocking calls and the
//!   native lease, a request while one runs is [`Status::Busy`], setup has a
//!   five second budget after which the copy is abandoned and never delivered
//!   late, and a delivery that began cannot be abandoned.
//!
//! The terminal route (OSC 52) is not written here: like the oracle, whose UI
//! writes it, the sequence goes to the owner of the terminal ([`Done::terminal`]),
//! so it never interleaves with a frame. rtwk builds the sequence
//! (`rtwk::editing::copy_to_clipboard`, wrapped for tmux or GNU screen).

use std::any::Any;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::PoisonError;
use std::sync::mpsc;
use std::time::Duration;
use std::time::Instant;

use codex_tui_core::app::effects::PrimaryOwner;

mod html;
mod machine;
pub mod read;
mod tmux;
mod wsl;

pub use html::render_markdown;
pub use machine::Machine;
pub use machine::environment;
pub use machine::is_wsl_session;

/// Raw bytes tmux and OSC 52 take at most (oracle `OSC52_MAX_RAW_BYTES`):
/// larger payloads are refused before encoding.
pub const OSC52_MAX_RAW_BYTES: usize = 100_000;

/// The budget for setup, and for a Windows clipboard write.
pub const SETUP_TIMEOUT: Duration = Duration::from_secs(5);

/// What a copy abandoned during setup answers.
pub const SETUP_TIMEOUT_MESSAGE: &str = "clipboard setup timed out; copy abandoned";

/// What an empty selection answers, without touching a clipboard.
pub const EMPTY_MESSAGE: &str = "nothing to copy: the selected content is empty";

/// How a copy is offered besides its text (oracle `CopyFormat`: the HTML of
/// a Markdown copy goes to the native clipboard; terminal and WSL routes keep
/// the text). The HTML comes from the owner of the source (`markdown_copy`),
/// rendered once.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Format {
    Plain,
    Html(Arc<str>),
}

impl Format {
    fn html(&self) -> Option<&str> {
        match self {
            Self::Plain => None,
            Self::Html(html) => Some(html),
        }
    }
}

/// Where a mouse selection's copy goes (oracle `CopyDestination`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Destination {
    Clipboard,
    /// The X11 PRIMARY selection only.
    Primary(PrimaryOwner),
    /// The clipboard, and PRIMARY with the plain text.
    ClipboardAndPrimary(Arc<str>, PrimaryOwner),
}

impl Destination {
    fn primary<'a>(&'a self, text: &'a Arc<str>) -> Option<(&'a Arc<str>, &'a PrimaryOwner)> {
        match self {
            Self::Clipboard => None,
            Self::Primary(owner) => Some((text, owner)),
            Self::ClipboardAndPrimary(plain, owner) => Some((plain, owner)),
        }
    }
}

/// What keeps a native clipboard alive (oracle `ClipboardLease`): on X11 and
/// some Wayland compositors the process that wrote the clipboard serves it, so
/// dropping the handle empties it. The host puts the platform handle in; the
/// worker keeps it for the life of the app.
pub struct Lease {
    _owner: Box<dyn Any + Send>,
}

impl Lease {
    pub fn new(owner: impl Any + Send) -> Self {
        Self {
            _owner: Box::new(owner),
        }
    }
}

/// What a route did (oracle `CopyOutcome`).
pub enum Outcome {
    /// A native write: confirmed. A new lease replaces the old one.
    Copied(Option<Lease>),
    /// An unacknowledged request to the user's terminal.
    Requested,
}

/// The delivery status of a copy (oracle `CopyStatus`, less `Pending`: a copy
/// in flight is the asker's own state).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Confirmed,
    Unconfirmed,
    /// Another copy still runs: this one was not started.
    Busy,
}

impl Outcome {
    /// Replace native ownership only when a route supplies a new lease.
    pub fn store(self, lease: &mut Option<Lease>) -> Status {
        match self {
            Self::Copied(next) => {
                if let Some(next) = next {
                    *lease = Some(next);
                }
                Status::Confirmed
            }
            Self::Requested => Status::Unconfirmed,
        }
    }
}

/// What the routes depend on in the environment (oracle `CopyEnvironment`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Environment {
    pub herdr: bool,
    pub ssh: bool,
    pub forwarded_x11: bool,
    pub wsl: bool,
    pub tmux: bool,
}

impl Environment {
    /// Reads the environment through `get` (the process's variables in the
    /// app, a table in a test); `wsl` is the host's probe.
    pub fn from_lookup(get: impl Fn(&str) -> Option<String>, wsl: bool) -> Self {
        let has = |name: &str| get(name).is_some();
        let ssh = has("SSH_TTY") || has("SSH_CONNECTION");
        Self {
            herdr: get("HERDR_ENV").is_some_and(|value| value == "1"),
            ssh,
            forwarded_x11: ssh && get("DISPLAY").is_some_and(|display| !display.is_empty()),
            wsl,
            tmux: has("TMUX") || has("TMUX_PANE"),
        }
    }

    /// Whether X11 PRIMARY may be written (oracle `primary::available`): a
    /// local X11 session only, never over ssh or tmux, on Wayland, WSL or in
    /// a VS Code terminal.
    pub fn primary_available(
        self,
        get: impl Fn(&str) -> Option<String>,
        linux: bool,
        vscode: bool,
    ) -> bool {
        linux
            && get("DISPLAY").is_some()
            && get("WAYLAND_DISPLAY").is_none()
            && !self.ssh
            && !self.tmux
            && !self.wsl
            && !vscode
    }
}

/// The machine behind a copy: what the host implements over the platform.
/// Every call may block for a long time; the worker thread makes them.
pub trait Routes: Send + 'static {
    /// Setup that may stall (opening the native clipboard where WSL does not
    /// stand in the way): done before the copy is committed to, so an
    /// abandoned copy stops here and writes nothing.
    fn prepare(&mut self, _environment: Environment) -> Result<(), String> {
        Ok(())
    }

    /// Write the native clipboard, with the HTML flavour where there is one.
    fn native(&mut self, text: &str, html: Option<&str>) -> Result<Option<Lease>, String>;

    /// Write the Windows clipboard from WSL, within `deadline`.
    fn windows(&mut self, text: &str, deadline: Instant) -> Result<(), String>;

    /// Hand the text to the attached tmux client (`load-buffer -w`).
    fn tmux(&mut self, text: &str) -> Result<(), String>;

    /// Write X11 PRIMARY. `begin_delivery` is asked after setup and before the
    /// write; an `Err` abandons the write.
    fn primary(
        &mut self,
        text: &str,
        begin_delivery: &dyn Fn() -> Result<(), String>,
    ) -> Result<Outcome, String>;
}

/// The routes in order (oracle `copy_to_clipboard_with`). `terminal` is the
/// OSC 52 request: it checks the payload and records the text for the owner
/// of the terminal.
pub fn route(
    text: &str,
    format: &Format,
    environment: Environment,
    routes: &mut dyn Routes,
    terminal: &mut dyn FnMut(&str) -> Result<(), String>,
    deadline: Instant,
) -> Result<Outcome, String> {
    if text.is_empty() {
        return Err(EMPTY_MESSAGE.to_owned());
    }
    let mut terminal_copy = |routes: &mut dyn Routes| {
        if environment.tmux {
            let raw_bytes = text.len();
            let result = if raw_bytes > OSC52_MAX_RAW_BYTES {
                Err(format!(
                    "tmux clipboard payload too large ({raw_bytes} bytes; max {OSC52_MAX_RAW_BYTES})"
                ))
            } else {
                routes.tmux(text)
            };
            result.or_else(|tmux_error| {
                terminal(text).map_err(|osc_error| {
                    format!("tmux clipboard: {tmux_error}; OSC 52 fallback: {osc_error}")
                })
            })
        } else {
            terminal(text)
        }
    };
    let native_result = {
        let native = |routes: &mut dyn Routes| {
            routes
                .native(text, format.html())
                .map_err(|error| format!("native clipboard: {error}"))
        };
        if environment.wsl && !environment.forwarded_x11 {
            match routes.windows(text, deadline) {
                Ok(()) => Ok(None),
                Err(windows_error) => native(routes).map_err(|native_error| {
                    format!("Windows clipboard: {windows_error}; {native_error}")
                }),
            }
        } else {
            native(routes)
        }
    };
    // Natively first: an X11 SelectionClear from a terminal write can
    // otherwise race with the native owner and clear the new data. Persistent
    // multiplexer sessions may gain remote clients after the app started, so
    // forward even when no ssh variables were inherited or native succeeded.
    let terminal_result =
        (environment.tmux || environment.herdr || environment.ssh).then(|| terminal_copy(routes));
    match native_result {
        Ok(lease) => Ok(Outcome::Copied(lease)),
        Err(native_error) => terminal_result
            .unwrap_or_else(|| terminal_copy(routes))
            .map(|()| Outcome::Requested)
            .map_err(|terminal_error| {
                format!("{native_error}; terminal clipboard: {terminal_error}")
            }),
    }
}

/// The terminal route's check (oracle `osc52_sequence`): the payload is
/// refused above [`OSC52_MAX_RAW_BYTES`].
pub fn osc52_check(text: &str) -> Result<(), String> {
    let raw_bytes = text.len();
    if raw_bytes > OSC52_MAX_RAW_BYTES {
        return Err(format!(
            "OSC 52 payload too large ({raw_bytes} bytes; max {OSC52_MAX_RAW_BYTES})"
        ));
    }
    Ok(())
}

/// The tmux client a copy is forwarded to (oracle `tmux::clipboard_target`):
/// the most recently active client of the current pane's session, when tmux
/// forwards clipboards at all and that client has the `Ms` capability. The
/// three queries are the host's (`tmux show-options -gv set-clipboard`,
/// `list-clients -F "#{client_activity} #{client_name}"`, `show-messages -T`).
pub fn tmux_target(
    set_clipboard: impl FnOnce() -> Result<String, String>,
    list_clients: impl FnOnce() -> Result<String, String>,
    client_info: impl FnOnce(&str) -> Result<String, String>,
) -> Result<String, String> {
    let set_clipboard = set_clipboard()?;
    if set_clipboard.trim() == "off" {
        return Err("tmux clipboard forwarding is disabled".to_owned());
    }
    // list-clients is scoped to the current pane's session: an empty list
    // cannot select another session.
    let clients = list_clients()?;
    let client = clients
        .lines()
        .filter_map(|line| {
            let (activity, client) = line.split_once(' ')?;
            Some((activity.parse::<u64>().ok()?, client))
        })
        .filter(|(_, client)| !client.is_empty())
        .max_by_key(|(activity, _)| *activity)
        .map(|(_, client)| client.to_owned())
        .ok_or_else(|| "tmux clipboard forwarding is unavailable: no attached client".to_owned())?;
    let info = client_info(&client)?;
    if !info.lines().any(|line| {
        line.split_once("Ms: (string) ")
            .is_some_and(|(_, sequence)| !sequence.trim().is_empty())
    }) {
        return Err("tmux clipboard forwarding is unavailable: missing Ms capability".to_owned());
    }
    Ok(client)
}

/// How far a copy has come: the asker's deadline and the worker's decision to
/// deliver share one short critical section (oracle `CopySetup`).
enum Phase {
    Pending(Instant),
    Delivering,
    Abandoned,
}

struct Setup {
    phase: Mutex<Phase>,
}

impl Setup {
    fn new(deadline: Instant) -> Self {
        Self {
            phase: Mutex::new(Phase::Pending(deadline)),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Phase> {
        self.phase.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Whether the budget is spent before delivery began: from then on the
    /// copy is abandoned and `begin_delivery` refuses.
    fn timed_out(&self, now: Instant) -> bool {
        let mut phase = self.lock();
        if matches!(*phase, Phase::Pending(deadline) if now >= deadline) {
            *phase = Phase::Abandoned;
        }
        matches!(*phase, Phase::Abandoned)
    }

    /// Decide before propagating setup's success or failure: either result
    /// could otherwise start a write long after the user gave up. Once it
    /// returns `Ok`, delivery cannot be abandoned.
    fn begin_delivery(&self) -> Result<(), String> {
        if self.timed_out(Instant::now()) {
            return Err(SETUP_TIMEOUT_MESSAGE.to_owned());
        }
        *self.lock() = Phase::Delivering;
        Ok(())
    }
}

/// The answer to a copy: the status or the failure, and the text the owner of
/// the terminal is to put on the terminal's clipboard (OSC 52) when a route
/// asked for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Done {
    pub result: Result<Status, String>,
    pub terminal: Option<String>,
}

impl Done {
    fn of(result: Result<Status, String>) -> Self {
        Self {
            result,
            terminal: None,
        }
    }
}

struct Job {
    text: Arc<str>,
    format: Format,
    destination: Destination,
    setup: Arc<Setup>,
    reply: tokio::sync::oneshot::Sender<Done>,
}

/// The worker handle (oracle `ClipboardWorker`): clone it, copy from any task.
/// The worker thread starts at the first copy and ends when the last handle
/// drops; a native lease is kept until then.
#[derive(Clone)]
pub struct Clipboard {
    inner: Arc<Inner>,
}

struct Inner {
    environment: Environment,
    timeout: Duration,
    occupancy: Arc<Mutex<Occupancy>>,
    start: Mutex<Start>,
}

#[derive(Default)]
struct Occupancy {
    busy: bool,
    primary: Option<(Arc<str>, PrimaryOwner)>,
}

enum Start {
    Idle(Box<dyn Routes>),
    Running(mpsc::Sender<Job>),
    Failed(String),
}

impl Clipboard {
    pub fn new(routes: impl Routes, environment: Environment) -> Self {
        Self::with_timeout(routes, environment, SETUP_TIMEOUT)
    }

    /// With another setup budget (tests).
    pub fn with_timeout(routes: impl Routes, environment: Environment, timeout: Duration) -> Self {
        Self {
            inner: Arc::new(Inner {
                environment,
                timeout,
                occupancy: Arc::new(Mutex::new(Occupancy::default())),
                start: Mutex::new(Start::Idle(Box::new(routes))),
            }),
        }
    }

    /// Whether a copy still holds the worker.
    pub fn is_busy(&self) -> bool {
        lock_occupancy(&self.inner.occupancy).busy
    }

    /// A busy clipboard copy is refused. Its PRIMARY part, if any, replaces
    /// the pending publication and can run while its selection is current.
    pub async fn copy(&self, text: Arc<str>, format: Format, destination: Destination) -> Done {
        {
            let mut occupancy = lock_occupancy(&self.inner.occupancy);
            if occupancy.busy {
                if !text.is_empty()
                    && let Some((plain, owner)) = destination.primary(&text)
                    && owner.is_current()
                {
                    occupancy.primary = Some((plain.clone(), owner.clone()));
                }
                return Done::of(Ok(Status::Busy));
            }
            if text.is_empty() {
                return Done::of(Err(EMPTY_MESSAGE.to_owned()));
            }
            occupancy.busy = true;
        }
        let sender = match self.sender() {
            Ok(sender) => sender,
            Err(error) => {
                *lock_occupancy(&self.inner.occupancy) = Occupancy::default();
                return Done::of(Err(error));
            }
        };
        let setup = Arc::new(Setup::new(Instant::now() + self.inner.timeout));
        let (reply, mut answer) = tokio::sync::oneshot::channel();
        let job = Job {
            text,
            format,
            destination,
            setup: Arc::clone(&setup),
            reply,
        };
        if sender.send(job).is_err() {
            *lock_occupancy(&self.inner.occupancy) = Occupancy::default();
            return Done::of(Err("clipboard worker stopped".to_owned()));
        }
        let stopped = || Done::of(Err("clipboard worker stopped".to_owned()));
        match tokio::time::timeout(self.inner.timeout, &mut answer).await {
            Ok(Ok(done)) => done,
            Ok(Err(_)) => stopped(),
            Err(_) => {
                // The budget is spent: a copy that has not begun delivering is
                // abandoned (the worker stays busy until its call returns and
                // drops the job); one that began must finish.
                if setup.timed_out(Instant::now()) {
                    Done::of(Err(SETUP_TIMEOUT_MESSAGE.to_owned()))
                } else {
                    answer.await.unwrap_or_else(|_| stopped())
                }
            }
        }
    }

    fn sender(&self) -> Result<mpsc::Sender<Job>, String> {
        let mut start = self
            .inner
            .start
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if let Start::Running(sender) = &*start {
            return Ok(sender.clone());
        }
        if let Start::Failed(error) = &*start {
            return Err(error.clone());
        }
        let Start::Idle(routes) = std::mem::replace(
            &mut *start,
            Start::Failed("clipboard worker stopped".into()),
        ) else {
            return Err("clipboard worker stopped".to_owned());
        };
        let (sender, incoming) = mpsc::channel::<Job>();
        let environment = self.inner.environment;
        let occupancy = Arc::clone(&self.inner.occupancy);
        let timeout = self.inner.timeout;
        let spawned = std::thread::Builder::new()
            .name("clipboard-copy".into())
            .spawn(move || work(routes, environment, &incoming, &occupancy, timeout));
        match spawned {
            Ok(_) => {
                *start = Start::Running(sender.clone());
                Ok(sender)
            }
            Err(error) => {
                let error = format!("could not start clipboard worker: {error}");
                *start = Start::Failed(error.clone());
                Err(error)
            }
        }
    }
}

fn lock_occupancy(occupancy: &Mutex<Occupancy>) -> std::sync::MutexGuard<'_, Occupancy> {
    occupancy.lock().unwrap_or_else(PoisonError::into_inner)
}

fn next_primary(occupancy: &Mutex<Occupancy>) -> Option<(Arc<str>, PrimaryOwner)> {
    let mut occupancy = lock_occupancy(occupancy);
    let pending = occupancy.primary.take();
    occupancy.busy = pending.is_some();
    pending
}

fn publish_primary(
    routes: &mut dyn Routes,
    text: &str,
    owner: &PrimaryOwner,
    setup: &Setup,
    lease: &mut Option<Lease>,
) -> Result<Status, String> {
    let ended = || "selection ended before PRIMARY delivery".to_owned();
    if !owner.is_current() {
        return Err(ended());
    }
    routes
        .primary(text, &|| {
            if !owner.is_current() {
                return Err(ended());
            }
            setup.begin_delivery()
        })
        .map(|outcome| outcome.store(lease))
}

/// The worker thread: one request at a time, the leases kept between them.
fn work(
    mut routes: Box<dyn Routes>,
    environment: Environment,
    incoming: &mpsc::Receiver<Job>,
    occupancy: &Mutex<Occupancy>,
    timeout: Duration,
) {
    let mut lease: Option<Lease> = None;
    // An ordinary copy must not relinquish PRIMARY, nor the reverse.
    let mut primary_lease: Option<Lease> = None;
    while let Ok(job) = incoming.recv() {
        let Job {
            text,
            format,
            destination,
            setup,
            reply,
        } = job;
        let mut terminal_text: Option<String> = None;
        let primary = destination.primary(&text).map(|(plain, owner)| {
            publish_primary(routes.as_mut(), plain, owner, &setup, &mut primary_lease)
        });
        let outcome = match primary {
            Some(result) if matches!(destination, Destination::Primary(_)) => {
                result.map(|_| Outcome::Requested)
            }
            // An unavailable PRIMARY must not break the configured clipboard copy.
            _ => routes
                .prepare(environment)
                .and_then(|()| setup.begin_delivery())
                .and_then(|()| {
                    route(
                        &text,
                        &format,
                        environment,
                        routes.as_mut(),
                        &mut |text| {
                            osc52_check(text)?;
                            terminal_text = Some(text.to_owned());
                            Ok(())
                        },
                        Instant::now() + SETUP_TIMEOUT,
                    )
                }),
        };
        let result = outcome.map(|outcome| outcome.store(&mut lease));
        let mut deferred = next_primary(occupancy);
        // An abandoned or dropped asker has nobody to hear it.
        let _ = reply.send(Done {
            result,
            terminal: terminal_text,
        });
        while let Some((text, owner)) = deferred {
            let setup = Setup::new(Instant::now() + timeout);
            let _ = publish_primary(routes.as_mut(), &text, &owner, &setup, &mut primary_lease);
            deferred = next_primary(occupancy);
        }
    }
}

#[cfg(test)]
mod tests;
