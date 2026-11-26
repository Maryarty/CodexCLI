//! Shared helpers of the screen tests: the app on an in-memory terminal over
//! the scripted backend, waiting for the backend thread in real time while
//! rtwk's clock stays fake, and golden snapshots.

#![allow(dead_code)]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::disallowed_methods)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use codex_tui_core::app::AppEv;
use codex_tui_core::app::Command;
use codex_tui_core::app::Now;
use codex_tui_core::fixtures::script::Input;
use codex_tui_core::keymap::{Keymap, KeymapSpec};
use codex_tui_core::screens::ScreenEv;
use codex_tui_core::screens::onboarding::{Ask, Ev, Facts};
use codex_tui_rtwk::app::UiApp;
use codex_tui_rtwk::app::debug;
use codex_tui_rtwk::app::run::{app_view, localized, root};
use codex_tui_rtwk::l10n::Lang;
use codex_tui_rtwk::services::backend::fake::{FakeHandle, FakeServer};
use codex_tui_rtwk::services::shell::Backend;
use rtwk_test::{Options, Tester};

pub mod wait;

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
    /// On `pump_app` (`start_quittable`): the app can end (`AppCx::quit`),
    /// and then nothing is mounted to wait on.
    quittable: bool,
    /// Dropped last: the runtime stays entered while the app lives.
    _runtime: tokio::runtime::EnterGuard<'static>,
}

pub fn start() -> Harness {
    start_with(false)
}

pub fn start_debug() -> Harness {
    start_with(true)
}

/// The live app as `make` builds it from the plain one (a start page of the
/// command line, `with_start_*`), over the scripted backend.
pub fn start_app(make: impl FnOnce(UiApp) -> UiApp) -> Harness {
    let guard = rtwk_tokio::enter(runtime());
    let (server, fake) = FakeServer::new();
    let app = make(UiApp::new(Backend::Fake(server)));
    let mut t = Tester::with(Options::new().size(WIDTH, HEIGHT));
    t.pump_widget(localized(root(app_view(app)), Lang::En));
    t.pump_for(ms(200));
    Harness {
        t,
        fake,
        driver: None,
        quittable: false,
        _runtime: guard,
    }
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
        quittable: true,
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
        quittable: false,
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

/// As `start_sized`, for sizes smaller than the debug menu under the screen
/// can draw: the tester does not fail on the clips of the rows below.
pub fn start_clipping(debug: bool, width: u16, height: u16) -> Harness {
    start_with_options(
        debug,
        Options::new().size(width, height).fail_on_overflow(false),
    )
}

/// `start_clipping` on a driven runtime (`start_driven`): no real pause
/// anywhere, so a test of many keys at many sizes costs its frames only.
pub fn start_driven_clipping(debug: bool, width: u16, height: u16) -> Harness {
    start_driven_with(
        debug,
        Options::new().size(width, height).fail_on_overflow(false),
    )
}

fn start_at(debug: bool, width: u16, height: u16) -> Harness {
    start_with_options(debug, Options::new().size(width, height))
}

fn start_with_options(debug: bool, options: Options) -> Harness {
    let guard = rtwk_tokio::enter(runtime());
    let (server, fake) = FakeServer::new();
    let app = UiApp::new(Backend::Fake(server)).with_debug_screens(debug);
    let mut t = Tester::with(options);
    // The app under the overlay `run_app` puts above it.
    t.pump_widget(localized(root(app_view(app)), Lang::En));
    t.pump_for(ms(200));
    Harness {
        t,
        fake,
        driver: None,
        quittable: false,
        _runtime: guard,
    }
}

/// How many times a driven runtime gives each ready task a turn per `drive`:
/// deeper than any chain of hand-offs between the app, the fake and the
/// ingress stream.
const DRIVE_TURNS: usize = 16;

/// The most rounds `quiesce` runs before it gives the screen as it is.
/// The slice of fake time between two runs of the driven runtime.
/// The pause before a key a test presses: longer than the composer's 8 ms paste interval.
pub const KEY_PACE_MS: u64 = 10;

const SLICE_MS: u64 = 20;

const QUIESCE_ROUNDS: usize = 200;

/// As `start_sized`, on a runtime of its own that only the test thread runs
/// (`drive`, `pause`): the backend task, the ingress stream and every timer
/// make progress when the test lets them, never in parallel with its reads.
/// The oracle kit builds its screens on it.
pub fn start_driven(debug: bool, width: u16, height: u16) -> Harness {
    start_driven_with(debug, Options::new().size(width, height))
}

/// `start_driven` on a terminal made with `options` (its size, its default
/// colours, its colour depth, the environment the app reads): the kit's seeds
/// of the terminal (`kit::env`).
pub fn start_driven_with(debug: bool, options: Options) -> Harness {
    start_driven_as(debug, options, |app| app)
}

/// `start_driven_with` where `configure` shapes the app before it mounts (a prompt of the
/// command line, the startup facts it waits for).
pub fn start_driven_as(
    debug: bool,
    options: Options,
    configure: impl FnOnce(UiApp) -> UiApp,
) -> Harness {
    start_driven_mode(debug, options, false, configure)
}

pub fn start_driven_mode(
    debug: bool,
    options: Options,
    quittable: bool,
    configure: impl FnOnce(UiApp) -> UiApp,
) -> Harness {
    let runtime: &'static tokio::runtime::Runtime = Box::leak(Box::new(
        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .start_paused(true)
            .build()
            .expect("runtime"),
    ));
    let guard = rtwk_tokio::enter(runtime);
    let (server, fake) = FakeServer::new();
    let app = configure(UiApp::new(Backend::Fake(server)).with_debug_screens(debug));
    let mut t = Tester::with(options);
    if quittable {
        t.pump_app(app_view(app));
    } else {
        t.pump_widget(localized(root(app_view(app)), Lang::En));
    }
    let mut h = Harness {
        t,
        fake,
        driver: Some(runtime),
        quittable,
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
        wait::wait_until(self, what, |h| ready(&h.t.screen()));
    }

    /// `wait::wait_until` on this harness: frames until `ready` holds of it
    /// (the screen, a page found in the tree, the facts of the app).
    pub fn wait_until(&mut self, what: &str, ready: impl FnMut(&mut Harness) -> bool) {
        wait::wait_until(self, what, ready);
    }

    /// The latest matching ask actually received by the fake host.
    pub fn startup_request(&mut self, what: &str, read: impl Fn(&Ask) -> Option<u64>) -> u64 {
        let mut request = None;
        self.wait_until(what, |h| {
            request = h
                .fake
                .commands()
                .iter()
                .rev()
                .find_map(|command| match command {
                    Command::Effect {
                        ask: codex_tui_core::app::effects::Ask::Startup { ask, .. },
                        ..
                    } => read(ask),
                    _ => None,
                });
            request.is_some()
        });
        request.expect(what)
    }

    pub fn answer_startup_facts(&mut self, facts: Facts) {
        let request = self.startup_request("the launch facts ask", |ask| match ask {
            Ask::Facts { request } => Some(*request),
            _ => None,
        });
        self.inject([AppEv::Screen(ScreenEv::Onboarding(Ev::Facts {
            request,
            facts,
        }))]);
    }

    /// A launch with no sign-in, trust question or startup prompts.
    pub fn finish_startup_without_prompts(&mut self) {
        self.answer_startup_facts(Facts::default());
        let request = self.startup_request("the gateway device ask", |ask| match ask {
            Ask::GatewayDevice { request, .. } => Some(*request),
            _ => None,
        });
        self.inject([AppEv::Screen(ScreenEv::Onboarding(Ev::GatewayDevice {
            request,
            code: None,
        }))]);
        let request = self.startup_request("the launch ask", |ask| match ask {
            Ask::Launch { request } => Some(*request),
            _ => None,
        });
        self.inject([AppEv::Screen(ScreenEv::Onboarding(Ev::Launched {
            request,
            prompts: Vec::new(),
        }))]);
    }

    /// Waits for a ready session and a settled turn. Idle is silent in the chat frame.
    pub fn wait_idle(&mut self) {
        wait::wait_until(self, "the session idle", |h| {
            h.t.read_app(|app: &UiApp| {
                app.core.session().status == codex_tui_core::app::Session::Ready
                    && !app.core.active_thread().can_interrupt()
            })
        });
    }

    /// Where the import flow is. The import page is up the moment its route
    /// is pushed, but it only starts its flow (`Phase::Detecting`) when its
    /// `Open` has gone through the shell and back: a key sent before that
    /// reaches a page whose flow has not begun.
    pub fn import_phase(&self) -> codex_tui_core::screens::import::Phase {
        self.t
            .read_app(|app: &UiApp| app.core.screens().import.phase)
    }

    /// Delivers `events` as the backend would, then waits until the app has
    /// applied every one of them. The ingress is one queue applied in order:
    /// a clock event sent after them and seen by the app means they are all
    /// in the model. Nothing else is promised about the screen (a dialog
    /// that fades in still needs frames).
    pub fn inject(&mut self, events: impl IntoIterator<Item = AppEv>) {
        // The shell attaches its connection on its own thread: a test that
        // injects at once after the start has to wait for it, not assume it.
        wait::wait_until(self, "the backend's connection", |h| {
            h.fake.epoch().is_some()
        });
        let epoch = self.fake.epoch().expect("connected");
        for ev in events {
            self.fake.inject(epoch, ev);
        }
        // The shell's own first clock event comes from its thread too: let it
        // land, or it could overwrite the mark after it was seen.
        let clock = |h: &mut Harness| h.t.read_app(|app: &UiApp| app.core.now().millis());
        wait::wait_until(self, "the shell's clock", |h| clock(h) != 0);
        let mark = clock(self) + 1;
        self.fake.inject(epoch, AppEv::Clock(Now::at_millis(mark)));
        wait::wait_until(self, "the injected events applied", |h| {
            h.t.read_app(|app: &UiApp| app.core.now().millis()) >= mark
        });
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
        // a person's key comes later than a paste's chars (the composer
        // reads keys under 8 ms apart as a paste): `type_text` then `press("enter")`
        // submits
        self.t.pump_for(ms(KEY_PACE_MS));
        self.t.press(keys);
        self.advance_driven(100);
        self.applied();
    }

    /// Waits until the app has applied what its own handlers sent to the
    /// shell. A key's handler sends an intent into the ingress, and the model
    /// has it at the app's next flush after the shell thread passed it on:
    /// how many frames that takes depends on the machine, and a sheet that a
    /// handler shows at once (a dialog route) is on the screen, empty, before
    /// the model it reads has the menu. The ingress is one queue: a clock
    /// event that the fake sends after the handler's items and the app has
    /// seen means they were all applied. A driven harness has no other thread
    /// (`quiesce` ran them), and a harness not yet connected has sent nothing.
    pub fn applied(&mut self) {
        let ended = |h: &Harness| h.quittable && !h.t.is_running();
        if self.driver.is_some() || ended(self) {
            return;
        }
        let Some(epoch) = self.fake.epoch() else {
            return;
        };
        let clock = |h: &Harness| h.t.read_app(|app: &UiApp| app.core.now().millis());
        // the shell's own first clock event comes from its thread too
        wait::wait_until(self, "the shell's clock", |h| ended(h) || clock(h) != 0);
        if ended(self) {
            return;
        }
        let mark = clock(self) + 1;
        self.fake.inject(epoch, AppEv::Clock(Now::at_millis(mark)));
        // a quit the handler asked for ends the app before the mark lands
        wait::wait_until(self, "the handler's items applied", |h| {
            ended(h) || clock(h) >= mark
        });
    }
}

/// The index of a debug-menu row: a directory's heading, or one of its
/// entries (the rows run directory by directory, each followed by its entries).
pub fn debug_row(dir: &str, entry: Option<&str>) -> usize {
    let mut index = 0;
    for d in debug::dirs() {
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
    /// Runs the app until the backend has attached its connection (the epoch
    /// the app's events are stamped with).
    pub fn connect(&mut self) {
        for _ in 0..200 {
            if self.fake.epoch().is_some() {
                break;
            }
            self.t.pump_for(ms(20));
            self.pause(ms(5));
        }
    }

    /// Hands the app the user's bindings, as the host does once it has read
    /// the config (`AppEv::Keymap`); the app is connected first. A pair names
    /// an action as the oracle's tests set the fields of its `RuntimeKeymap`
    /// (`list.move_down`; `app.` is the config's `global` table and a bare
    /// action is a global one) and a key as the config writes it (`ctrl+x
    /// enter` is `ctrl-x enter`); the pairs of one action add up, an empty key
    /// alone unbinds it. The oracle's tests set the fields without the
    /// config's checks, so the keymap is not checked either.
    pub fn set_keymap(&mut self, bindings: &[(&str, &str)]) {
        self.connect();
        let mut table: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
        for (action, key) in bindings {
            let (context, name) = action.split_once('.').unwrap_or(("global", action));
            let context = if context == "app" { "global" } else { context };
            let keys = table
                .entry((context.to_owned(), name.to_owned()))
                .or_default();
            if !key.is_empty() {
                keys.push(
                    key.split(' ')
                        .map(|k| k.replace('+', "-"))
                        .collect::<Vec<_>>()
                        .join(" "),
                );
            }
        }
        let mut spec = KeymapSpec::default();
        for ((context, name), keys) in &table {
            let keys: Vec<&str> = keys.iter().map(String::as_str).collect();
            spec = spec.with(context, name, &keys);
        }
        self.inject([AppEv::Keymap(Box::new(Keymap::resolve_unchecked(&spec)))]);
    }

    /// `times` presses of a key that moves a widget's own cursor (a list's
    /// `down`): nothing goes through the shell, so the keys go to the terminal
    /// as they are, with the frames between them, and one `applied` after the
    /// run. A `press` each waited for the shell, a real pause per key: the
    /// debug menu's 450 rows were 3 s.
    pub fn move_cursor(&mut self, key: &str, times: usize) {
        for _ in 0..times {
            self.t.press(key);
        }
        self.t.pump_for(ms(100));
        self.applied();
    }

    /// Opens a row of the debug menu (the menu is the home, nothing above it).
    pub fn open_debug_row(&mut self, dir: &str, entry: Option<&str>) {
        // The menu is one list whose cursor the SelectionList owns.
        self.move_cursor("down", debug_row(dir, entry));
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
    let dirs = debug::dirs();
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

impl wait::Pumped for Harness {
    fn tester(&mut self) -> &mut Tester {
        &mut self.t
    }

    fn pause(&mut self) {
        Harness::pause(self, ms(5));
    }
}
