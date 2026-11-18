//! Shared helpers of the screen tests: the app on an in-memory terminal over
//! the scripted backend, waiting for the backend thread in real time while
//! rtwk's clock stays fake, and golden snapshots.

#![allow(dead_code)]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::disallowed_methods)]

use std::path::PathBuf;
use std::time::Duration;

use codex_tui_core::fixtures::script::Input;
use codex_tui_rtwk::app::UiApp;
use codex_tui_rtwk::l10n::Lang;
use codex_tui_rtwk::run::{app_view, localized, root};
use codex_tui_rtwk::screens;
use codex_tui_rtwk::shell::Backend;
use codex_tui_rtwk::shell::fake::{FakeHandle, FakeServer};
use rtwk_test::{Options, Tester};

/// The size of the oracle's snapshots.
pub const WIDTH: u16 = 80;
pub const HEIGHT: u16 = 24;

pub fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

/// The tokio runtime the shell's tasks run on, as `main` provides one: shared
/// by the tests of a binary, entered on each test's (UI) thread.
fn runtime() -> &'static tokio::runtime::Runtime {
    static RUNTIME: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_time()
            .build()
            .expect("runtime")
    })
}

/// The app under test: a terminal and the backend's handle.
pub struct Harness {
    pub t: Tester,
    pub fake: FakeHandle,
    /// The runtime of a driven harness (`start_driven`): the test thread runs
    /// its tasks, so nothing happens behind the test's back.
    driver: Option<&'static tokio::runtime::Runtime>,
    /// Dropped last: the runtime stays entered while the app lives.
    _runtime: tokio::runtime::EnterGuard<'static>,
}

pub fn start() -> Harness {
    start_with(false)
}

pub fn start_debug() -> Harness {
    start_with(true)
}

/// The app on `Tester::pump_app`, which gives it the frame loop's handle:
/// `AppCx::quit` ends the loop only there (the overlay root of `start` has
/// none), as in `run_app_with`.
pub fn start_quittable(debug: bool) -> Harness {
    let guard = rtwk_tokio::enter(runtime());
    let (server, fake) = FakeServer::new();
    let app = UiApp::new(Backend::Fake(server)).with_debug_screens(debug);
    let mut t = Tester::with(Options::new().size(WIDTH, HEIGHT));
    t.pump_app(app_view(app));
    t.pump_for(ms(200));
    Harness {
        t,
        fake,
        driver: None,
        _runtime: guard,
    }
}

/// The app under `lang` (the locale `--locale` or the environment gave).
pub fn start_lang(lang: Lang) -> Harness {
    let guard = rtwk_tokio::enter(runtime());
    let (server, fake) = FakeServer::new();
    let app = UiApp::new(Backend::Fake(server)).with_lang(lang);
    let mut t = Tester::with(Options::new().size(WIDTH, HEIGHT));
    t.pump_widget(localized(root(app_view(app)), lang));
    t.pump_for(ms(200));
    Harness {
        t,
        fake,
        driver: None,
        _runtime: guard,
    }
}

/// As `start_debug`/`start` at another terminal size.
pub fn start_sized(debug: bool, width: u16, height: u16) -> Harness {
    start_at(debug, width, height)
}

fn start_with(debug: bool) -> Harness {
    start_at(debug, WIDTH, HEIGHT)
}

fn start_at(debug: bool, width: u16, height: u16) -> Harness {
    let guard = rtwk_tokio::enter(runtime());
    let (server, fake) = FakeServer::new();
    let app = UiApp::new(Backend::Fake(server)).with_debug_screens(debug);
    let mut t = Tester::with(Options::new().size(width, height));
    // The app under the overlay `run_app` puts above it.
    t.pump_widget(localized(root(app_view(app)), Lang::En));
    t.pump_for(ms(200));
    Harness {
        t,
        fake,
        driver: None,
        _runtime: guard,
    }
}

/// How many times a driven runtime gives each ready task a turn per `drive`:
/// deeper than any chain of hand-offs between the app, the fake and the
/// ingress stream.
const DRIVE_TURNS: usize = 16;

/// The most rounds `quiesce` runs before it gives the screen as it is.
/// The slice of fake time between two runs of the driven runtime.
const SLICE_MS: u64 = 20;

const QUIESCE_ROUNDS: usize = 200;

/// As `start_sized`, on a runtime of its own that only the test thread runs
/// (`drive`, `pause`): the backend task, the ingress stream and every timer
/// make progress when the test lets them, never in parallel with its reads.
/// The oracle kit builds its screens on it.
pub fn start_driven(debug: bool, width: u16, height: u16) -> Harness {
    let runtime: &'static tokio::runtime::Runtime = Box::leak(Box::new(
        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .start_paused(true)
            .build()
            .expect("runtime"),
    ));
    let guard = rtwk_tokio::enter(runtime);
    let (server, fake) = FakeServer::new();
    let app = UiApp::new(Backend::Fake(server)).with_debug_screens(debug);
    let mut t = Tester::with(Options::new().size(width, height));
    t.pump_widget(localized(root(app_view(app)), Lang::En));
    let mut h = Harness {
        t,
        fake,
        driver: Some(runtime),
        _runtime: guard,
    };
    h.t.pump_for(ms(200));
    h
}

impl Harness {
    /// Lets the driven runtime run every task that is ready, and the ones
    /// they make ready, `DRIVE_TURNS` times over. A harness on the shared
    /// multi-thread runtime has nothing to drive.
    pub fn drive(&self) {
        self.drive_for(Duration::ZERO);
    }

    /// `drive`, the runtime's clock (paused, so only the test moves it) first
    /// moved on by `d`: the timers of the backend run on the fake clock's
    /// time (the search's debounce).
    fn drive_for(&self, d: Duration) {
        if let Some(runtime) = self.driver {
            runtime.block_on(async {
                if !d.is_zero() {
                    tokio::time::advance(d).await;
                }
                for _ in 0..DRIVE_TURNS {
                    tokio::task::yield_now().await;
                }
            });
        }
    }

    /// Waits `d` of real time on a shared runtime (the backend thread
    /// answers meanwhile); on a driven one nothing waits in real time: the
    /// runtime runs and its clock moves on by `d`.
    pub fn pause(&self, d: Duration) {
        match self.driver {
            Some(_) => self.drive_for(d),
            None => std::thread::sleep(d),
        }
    }

    /// Lets `millis` of the fake clock pass in slices of `SLICE_MS`, the
    /// driven runtime running between them: an answer that arrives in one
    /// slice has the rest of the time to show (a tick that commits a streamed
    /// item, a fade). Then `quiesce`.
    pub fn advance(&mut self, millis: u64) {
        let mut left = millis;
        while left > 0 {
            let slice = left.min(SLICE_MS);
            self.drive_for(ms(slice));
            self.t.pump_for(ms(slice));
            left -= slice;
        }
        self.quiesce();
    }

    /// `advance` on a driven harness, a plain `pump_for` on the others (their
    /// tests have their own waits).
    fn advance_driven(&mut self, millis: u64) {
        if self.driver.is_some() {
            self.advance(millis);
        } else {
            self.t.pump_for(ms(millis));
        }
    }

    /// Runs the app and the backend alternately until a round changes
    /// neither the screen nor what the backend was asked: every event the app
    /// sent has been answered and every answer has been applied. No sleep
    /// and no clock: a round is `drive` and a frame without time passing.
    pub fn quiesce(&mut self) {
        let signature = |h: &Harness| {
            (
                h.fake.commands().len(),
                h.fake.requests().len(),
                h.t.screen(),
            )
        };
        let mut last = signature(self);
        let mut calm = 0;
        for _ in 0..QUIESCE_ROUNDS {
            self.drive();
            self.t.pump();
            let now = signature(self);
            if now == last {
                calm += 1;
                if calm == 2 {
                    return;
                }
            } else {
                calm = 0;
                last = now;
            }
        }
    }

    /// Pumps until `ready` holds of the screen; the backend thread answers
    /// in real time, the frames run on the fake clock.
    pub fn wait_for(&mut self, what: &str, ready: impl Fn(&str) -> bool) {
        for _ in 0..400 {
            self.t.pump_for(ms(20));
            if ready(&self.t.screen()) {
                return;
            }
            self.pause(ms(5));
        }
        panic!("never saw {what}; the screen:\n{}", self.t.screen());
    }

    /// Lets the backend thread run what the app sent it: commands travel
    /// to the fake on the runtime thread, so the app's screen can be ahead
    /// of what the fake has recorded.
    pub fn settle(&mut self) {
        for _ in 0..10 {
            self.t.pump_for(ms(20));
            self.pause(ms(10));
        }
    }

    pub fn press(&mut self, keys: &str) {
        self.t.press(keys);
        self.advance_driven(100);
    }
}

/// The index of a debug-menu row: a directory's heading, or one of its
/// entries (the rows run directory by directory, each followed by its entries).
pub fn debug_row(dir: &str, entry: Option<&str>) -> usize {
    let mut index = 0;
    for d in screens::dirs() {
        if d.name == dir {
            let Some(entry) = entry else { return index };
            let at = (d.debug_entries)()
                .iter()
                .position(|e| e.name == entry)
                .unwrap_or_else(|| panic!("`{dir}` has no debug entry `{entry}`"));
            return index + 1 + at;
        }
        index += 1 + (d.debug_entries)().len();
    }
    panic!("`{dir}` is not a registered screen directory");
}

impl Harness {
    /// Opens a row of the debug menu (the menu is the home, nothing above it).
    pub fn open_debug_row(&mut self, dir: &str, entry: Option<&str>) {
        for _ in 0..debug_row(dir, entry) {
            self.press("down");
        }
        self.press("enter");
    }

    /// Plays the `key` and `resize` steps of a script, in order.
    pub fn play_inputs(&mut self, inputs: &[Input]) {
        for input in inputs {
            match input {
                Input::Key(keys) => self.press(keys),
                Input::Resize { width, height } => {
                    self.t.resize(*width, *height);
                    self.advance_driven(100);
                }
            }
        }
    }
}

/// A slot of the screen map: registered once, with the id and kind the
/// map says, and empty until its slice fills it.
pub fn slot(name: &str) {
    let dirs = screens::dirs();
    let dir = dirs
        .iter()
        .find(|dir| dir.name == name)
        .unwrap_or_else(|| panic!("`{name}` is not a registered screen directory"));
    assert!(!dir.id.is_empty());
    let _ = (dir.open_empty)();
}

fn snapshot_path(dir: &str, name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/snapshots")
        .join(dir)
        .join(format!("{name}.snap"))
}

/// Compares the screen with the golden text of `tests/snapshots/<dir>/<name>.snap`
/// (trailing blanks trimmed per line). `UPDATE_SNAPSHOTS=1` writes it.
pub fn assert_snapshot(t: &Tester, dir: &str, name: &str) {
    assert_snapshot_where(t, dir, name, |_| true);
}

/// As `assert_snapshot`, over the screen lines `keep` accepts: a golden that
/// must not change when a slice adds its own rows (the debug menu's entries).
pub fn assert_snapshot_where(t: &Tester, dir: &str, name: &str, keep: impl Fn(&str) -> bool) {
    let actual = t
        .lines()
        .iter()
        .map(|line| line.trim_end().to_string())
        .filter(|line| keep(line))
        .collect::<Vec<_>>()
        .join("\n");
    let path = snapshot_path(dir, name);
    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        std::fs::write(&path, format!("{actual}\n")).expect("write snapshot");
        return;
    }
    let expected = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("missing snapshot {}; the screen:\n{actual}", path.display()));
    pretty_assertions::assert_eq!(expected.trim_end(), actual.trim_end(), "{name}");
}
