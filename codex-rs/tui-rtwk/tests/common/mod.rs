//! Shared helpers of the screen tests: the app on an in-memory terminal over
//! the scripted backend, waiting for the backend thread in real time while
//! rtwk's clock stays fake, and golden snapshots.

#![allow(dead_code)]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::disallowed_methods)]

use std::path::PathBuf;
use std::time::Duration;

use codex_tui_rtwk::app::UiApp;
use codex_tui_rtwk_core::fixtures::script::Input;
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
        _runtime: guard,
    }
}

impl Harness {
    /// Pumps until `ready` holds of the screen; the backend thread answers
    /// in real time, the frames run on the fake clock.
    pub fn wait_for(&mut self, what: &str, ready: impl Fn(&str) -> bool) {
        for _ in 0..400 {
            self.t.pump_for(ms(20));
            if ready(&self.t.screen()) {
                return;
            }
            std::thread::sleep(ms(5));
        }
        panic!("never saw {what}; the screen:\n{}", self.t.screen());
    }

    /// Lets the backend thread run what the app sent it: commands travel
    /// to the fake on the runtime thread, so the app's screen can be ahead
    /// of what the fake has recorded.
    pub fn settle(&mut self) {
        for _ in 0..10 {
            self.t.pump_for(ms(20));
            std::thread::sleep(ms(10));
        }
    }

    pub fn press(&mut self, keys: &str) {
        self.t.press(keys);
        self.t.pump_for(ms(100));
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
                    self.t.pump_for(ms(100));
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
    let actual = t
        .lines()
        .iter()
        .map(|line| line.trim_end().to_string())
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
