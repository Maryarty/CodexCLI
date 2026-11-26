//! The routes (oracle `clipboard_copy.rs` and `routing_tests.rs` tests), tmux's
//! client choice (`tmux_tests.rs`) and the worker's budget rules
//! (`worker_tests.rs`: `expired_setup_...`, `delivery_claim_...`, backlog).

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc;
use std::time::Duration;
use std::time::Instant;

use pretty_assertions::assert_eq;

use super::*;

type Calls = Arc<Mutex<Vec<&'static str>>>;
type Native = Box<dyn FnMut(&str, Option<&str>) -> Result<Option<Lease>, String> + Send>;
type Text = Box<dyn FnMut(&str) -> Result<(), String> + Send>;

/// The machine, scripted: every route logs its name and answers as set.
struct Fake {
    calls: Calls,
    native: Native,
    primary: Text,
    windows: Text,
    tmux: Text,
}

impl Fake {
    fn new() -> (Self, Calls) {
        let calls = Calls::default();
        let fake = Self {
            calls: Arc::clone(&calls),
            native: Box::new(|_, _| Ok(Some(Lease::new(())))),
            primary: Box::new(|_| Err("X11 primary selection unavailable".to_owned())),
            windows: Box::new(|_| panic!("the Windows clipboard was not to be written")),
            tmux: Box::new(|_| panic!("tmux was not to be asked")),
        };
        (fake, calls)
    }

    fn with_native(
        mut self,
        f: impl FnMut(&str, Option<&str>) -> Result<Option<Lease>, String> + Send + 'static,
    ) -> Self {
        self.native = Box::new(f);
        self
    }

    fn with_windows(mut self, f: impl FnMut(&str) -> Result<(), String> + Send + 'static) -> Self {
        self.windows = Box::new(f);
        self
    }

    fn with_tmux(mut self, f: impl FnMut(&str) -> Result<(), String> + Send + 'static) -> Self {
        self.tmux = Box::new(f);
        self
    }
}

impl Routes for Fake {
    fn native(&mut self, text: &str, html: Option<&str>) -> Result<Option<Lease>, String> {
        self.calls.lock().unwrap().push("native");
        (self.native)(text, html)
    }

    fn windows(&mut self, text: &str, _deadline: Instant) -> Result<(), String> {
        self.calls.lock().unwrap().push("windows");
        (self.windows)(text)
    }

    fn tmux(&mut self, text: &str) -> Result<(), String> {
        self.calls.lock().unwrap().push("tmux");
        (self.tmux)(text)
    }

    fn primary(
        &mut self,
        text: &str,
        begin_delivery: &dyn Fn() -> Result<(), String>,
    ) -> Result<Outcome, String> {
        begin_delivery()?;
        self.calls.lock().unwrap().push("primary");
        (self.primary)(text)?;
        Ok(Outcome::Copied(None))
    }
}

fn local() -> Environment {
    Environment::default()
}

fn remote_wsl() -> Environment {
    Environment {
        ssh: true,
        wsl: true,
        ..local()
    }
}

fn local_tmux() -> Environment {
    Environment {
        tmux: true,
        ..local()
    }
}

/// `route` with a terminal request that logs and answers as `osc`.
fn run(
    text: &str,
    format: &Format,
    environment: Environment,
    mut fake: Fake,
    calls: &Calls,
    osc: Result<(), &'static str>,
) -> Result<Outcome, String> {
    let log = Arc::clone(calls);
    route(
        text,
        format,
        environment,
        &mut fake,
        &mut |_| {
            log.lock().unwrap().push("osc52");
            osc.map_err(str::to_owned)
        },
        Instant::now() + SETUP_TIMEOUT,
    )
}

fn copied(outcome: &Result<Outcome, String>) -> bool {
    matches!(outcome, Ok(Outcome::Copied(_)))
}

fn calls_of(calls: &Calls) -> Vec<&'static str> {
    calls.lock().unwrap().clone()
}

#[test]
fn ssh_on_wsl_with_forwarded_x11_uses_native_clipboard() {
    let (fake, calls) = Fake::new();
    let environment = Environment {
        forwarded_x11: true,
        tmux: true,
        ..remote_wsl()
    };

    let result = run(
        "hello",
        &Format::Plain,
        environment,
        fake.with_tmux(|_| Ok(())),
        &calls,
        Ok(()),
    );

    assert!(matches!(result, Ok(Outcome::Copied(Some(_)))));
    assert_eq!(calls_of(&calls), ["native", "tmux"]);
}

#[test]
fn ssh_on_wsl_attempts_windows_and_osc52() {
    let (fake, calls) = Fake::new();

    let result = run(
        "**hello**",
        &Format::Html("<p>hello</p>".into()),
        remote_wsl(),
        fake.with_windows(|_| Ok(())),
        &calls,
        Ok(()),
    );

    assert!(matches!(result, Ok(Outcome::Copied(None))));
    assert_eq!(calls_of(&calls), ["windows", "osc52"]);
}

#[test]
fn ssh_reports_failure_when_all_backends_fail() {
    let (fake, calls) = Fake::new();
    let fake = fake
        .with_native(|_, _| Err("native unavailable".to_owned()))
        .with_windows(|_| Err("powershell unavailable".to_owned()));

    let result = run(
        "hello",
        &Format::Plain,
        remote_wsl(),
        fake,
        &calls,
        Err("blocked"),
    );

    assert_eq!(
        result.err().as_deref(),
        Some(
            "Windows clipboard: powershell unavailable; native clipboard: native unavailable; terminal clipboard: blocked"
        )
    );
    assert_eq!(calls_of(&calls), ["windows", "native", "osc52"]);
}

#[test]
fn ssh_on_wsl_inside_tmux_attempts_windows_and_tmux() {
    let (fake, calls) = Fake::new();
    let environment = Environment {
        tmux: true,
        ..remote_wsl()
    };

    let result = run(
        "**hello**",
        &Format::Plain,
        environment,
        fake.with_windows(|_| Ok(())).with_tmux(|_| Ok(())),
        &calls,
        Ok(()),
    );

    assert!(matches!(result, Ok(Outcome::Copied(None))));
    assert_eq!(calls_of(&calls), ["windows", "tmux"]);
}

#[test]
fn ssh_inside_tmux_falls_back_to_osc52_when_tmux_copy_fails() {
    let (fake, calls) = Fake::new();
    let environment = Environment {
        tmux: true,
        ..remote_wsl()
    };

    let result = run(
        "hello",
        &Format::Plain,
        environment,
        fake.with_windows(|_| Ok(()))
            .with_tmux(|_| Err("tmux unavailable".to_owned())),
        &calls,
        Ok(()),
    );

    assert!(matches!(result, Ok(Outcome::Copied(None))));
    assert_eq!(calls_of(&calls), ["windows", "tmux", "osc52"]);
}

#[test]
fn ssh_inside_tmux_reports_tmux_and_osc52_errors_when_both_fail() {
    let (fake, calls) = Fake::new();
    let environment = Environment {
        tmux: true,
        ..remote_wsl()
    };
    let fake = fake
        .with_native(|_, _| Err("native unavailable".to_owned()))
        .with_windows(|_| Err("powershell unavailable".to_owned()))
        .with_tmux(|_| Err("tmux unavailable".to_owned()));

    let result = run(
        "hello",
        &Format::Plain,
        environment,
        fake,
        &calls,
        Err("osc blocked"),
    );

    assert_eq!(
        result.err().as_deref(),
        Some(
            "Windows clipboard: powershell unavailable; native clipboard: native unavailable; terminal clipboard: tmux clipboard: tmux unavailable; OSC 52 fallback: osc blocked"
        )
    );
}

#[test]
fn local_uses_native_clipboard_first_and_asks_no_terminal() {
    let (fake, calls) = Fake::new();

    let result = run(
        "hello",
        &Format::Plain,
        local(),
        fake,
        &calls,
        Err("never asked"),
    );

    assert!(matches!(result, Ok(Outcome::Copied(Some(_)))));
    assert_eq!(calls_of(&calls), ["native"]);
}

#[test]
fn local_copy_offers_html_only_when_the_format_has_it() {
    let html = "<p><strong>hello</strong></p>\n";
    for (format, expected) in [
        (Format::Plain, None),
        (Format::Html(html.into()), Some(html)),
    ] {
        let (fake, calls) = Fake::new();
        let seen = Arc::new(Mutex::new(None));
        let record = Arc::clone(&seen);
        let fake = fake.with_native(move |text, html| {
            *record.lock().unwrap() = Some((text.to_owned(), html.map(str::to_owned)));
            Ok(None)
        });

        let result = run("**hello**", &format, local(), fake, &calls, Ok(()));

        assert!(matches!(result, Ok(Outcome::Copied(None))));
        assert_eq!(
            seen.lock().unwrap().clone(),
            Some(("**hello**".to_owned(), expected.map(str::to_owned)))
        );
    }
}

#[test]
fn local_non_wsl_falls_back_to_osc52_when_native_fails() {
    let (fake, calls) = Fake::new();
    let fake = fake.with_native(|_, _| Err("native unavailable".to_owned()));

    let result = run("hello", &Format::Plain, local(), fake, &calls, Ok(()));

    assert!(matches!(result, Ok(Outcome::Requested)));
    assert_eq!(calls_of(&calls), ["native", "osc52"]);
}

#[test]
fn local_tmux_fallback_prefers_tmux_when_native_fails() {
    let (fake, calls) = Fake::new();
    let fake = fake
        .with_native(|_, _| Err("native unavailable".to_owned()))
        .with_tmux(|_| Ok(()));

    let result = run("hello", &Format::Plain, local_tmux(), fake, &calls, Ok(()));

    assert!(matches!(result, Ok(Outcome::Requested)));
    assert_eq!(calls_of(&calls), ["native", "tmux"]);
}

#[test]
fn local_wsl_uses_windows_and_skips_osc52_on_success() {
    let (fake, calls) = Fake::new();
    let environment = Environment {
        wsl: true,
        ..local()
    };

    let result = run(
        "hello",
        &Format::Plain,
        environment,
        fake.with_windows(|_| Ok(())),
        &calls,
        Err("never asked"),
    );

    assert!(matches!(result, Ok(Outcome::Copied(None))));
    assert_eq!(calls_of(&calls), ["windows"]);
}

#[test]
fn local_wsl_falls_back_to_osc52_when_powershell_fails() {
    let (fake, calls) = Fake::new();
    let environment = Environment {
        wsl: true,
        ..local()
    };
    let fake = fake
        .with_native(|_, _| Err("native unavailable".to_owned()))
        .with_windows(|_| Err("powershell unavailable".to_owned()));

    let result = run("hello", &Format::Plain, environment, fake, &calls, Ok(()));

    assert!(matches!(result, Ok(Outcome::Requested)));
    assert_eq!(calls_of(&calls), ["windows", "native", "osc52"]);
}

#[test]
fn local_reports_both_errors_when_native_and_osc52_fail() {
    let (fake, calls) = Fake::new();
    let fake = fake.with_native(|_, _| Err("native unavailable".to_owned()));

    let result = run(
        "hello",
        &Format::Plain,
        local(),
        fake,
        &calls,
        Err("osc blocked"),
    );

    assert_eq!(
        result.err().as_deref(),
        Some("native clipboard: native unavailable; terminal clipboard: osc blocked")
    );
}

#[test]
fn local_wsl_reports_windows_and_osc52_errors_when_all_fail() {
    let (fake, calls) = Fake::new();
    let environment = Environment {
        wsl: true,
        ..local()
    };
    let fake = fake
        .with_native(|_, _| Err("native unavailable".to_owned()))
        .with_windows(|_| Err("powershell unavailable".to_owned()));

    let result = run(
        "hello",
        &Format::Plain,
        environment,
        fake,
        &calls,
        Err("osc blocked"),
    );

    assert_eq!(
        result.err().as_deref(),
        Some(
            "Windows clipboard: powershell unavailable; native clipboard: native unavailable; terminal clipboard: osc blocked"
        )
    );
}

#[test]
fn local_tmux_preserves_native_copy_when_terminal_silently_rejects_it() {
    // Like default xterm: the tmux command succeeds without delivery.
    let (fake, calls) = Fake::new();

    let result = run(
        "**hello**",
        &Format::Plain,
        local_tmux(),
        fake.with_tmux(|_| Ok(())),
        &calls,
        Ok(()),
    );

    assert!(matches!(result, Ok(Outcome::Copied(Some(_)))));
    assert_eq!(calls_of(&calls), ["native", "tmux"]);
}

#[test]
fn copy_uses_osc52_when_tmux_fails_even_if_native_copy_succeeds() {
    let (fake, calls) = Fake::new();

    let result = run(
        "hello",
        &Format::Plain,
        local_tmux(),
        fake.with_tmux(|_| Err("tmux unavailable".to_owned())),
        &calls,
        Ok(()),
    );

    assert!(matches!(result, Ok(Outcome::Copied(Some(_)))));
    assert_eq!(calls_of(&calls), ["native", "tmux", "osc52"]);
}

#[test]
fn herdr_forwards_to_osc52_even_when_native_copy_succeeds() {
    let (fake, calls) = Fake::new();
    let environment = Environment {
        herdr: true,
        ..local()
    };

    let result = run("hello", &Format::Plain, environment, fake, &calls, Ok(()));

    assert!(copied(&result));
    assert_eq!(calls_of(&calls), ["native", "osc52"]);
}

#[test]
fn oversized_tmux_copy_preserves_native_copy() {
    for (size, expected) in [
        (OSC52_MAX_RAW_BYTES, vec!["native", "tmux"]),
        (OSC52_MAX_RAW_BYTES + 1, vec!["native", "osc52"]),
    ] {
        let (fake, calls) = Fake::new();
        let text = "x".repeat(size);

        let result = run(
            &text,
            &Format::Plain,
            local_tmux(),
            fake.with_tmux(|_| Ok(())),
            &calls,
            Err("payload too large"),
        );

        assert!(copied(&result));
        assert_eq!(calls_of(&calls), expected);
    }
}

#[test]
fn empty_copy_reports_failure_without_touching_clipboards() {
    let (fake, calls) = Fake::new();

    let result = run(
        "",
        &Format::Plain,
        local_tmux(),
        fake,
        &calls,
        Err("never asked"),
    );

    assert_eq!(
        result.err().as_deref(),
        Some("nothing to copy: the selected content is empty")
    );
    assert_eq!(calls_of(&calls), Vec::<&str>::new());
}

#[test]
fn confirmed_copy_without_new_lease_preserves_native_ownership() {
    let mut lease = Some(Lease::new(()));

    assert_eq!(Outcome::Copied(None).store(&mut lease), Status::Confirmed);
    assert!(lease.is_some());
    assert_eq!(Outcome::Requested.store(&mut lease), Status::Unconfirmed);
    assert!(lease.is_some());
}

#[test]
fn the_terminal_payload_is_refused_above_the_cap_before_it_is_sent() {
    assert_eq!(osc52_check(&"x".repeat(OSC52_MAX_RAW_BYTES)), Ok(()));
    assert_eq!(
        osc52_check(&"x".repeat(OSC52_MAX_RAW_BYTES + 1)),
        Err("OSC 52 payload too large (100001 bytes; max 100000)".to_owned())
    );
}

fn vars<'a>(set: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
    let table: HashMap<&str, &str> = set.iter().copied().collect();
    move |name| table.get(name).map(|value| (*value).to_owned())
}

#[test]
fn the_environment_reads_ssh_tmux_herdr_and_forwarded_x11() {
    let get = vars(&[
        ("SSH_TTY", "/dev/pts/1"),
        ("DISPLAY", ":10"),
        ("TMUX_PANE", "%1"),
        ("HERDR_ENV", "1"),
    ]);
    assert_eq!(
        Environment::from_lookup(get, true),
        Environment {
            herdr: true,
            ssh: true,
            forwarded_x11: true,
            wsl: true,
            tmux: true,
        }
    );
    // DISPLAY without ssh is a local display, not a forwarded one; HERDR_ENV must be "1".
    assert_eq!(
        Environment::from_lookup(vars(&[("DISPLAY", ":0"), ("HERDR_ENV", "0")]), false),
        Environment::default()
    );
    let empty = vars(&[("SSH_CONNECTION", "x"), ("DISPLAY", "")]);
    assert!(!Environment::from_lookup(empty, false).forwarded_x11);
}

#[test]
fn primary_is_for_a_local_x11_session_only() {
    let x11 = || vars(&[("DISPLAY", ":0")]);
    let local = Environment::default();
    assert!(local.primary_available(x11(), true, false));
    assert!(!local.primary_available(x11(), false, false));
    assert!(!local.primary_available(x11(), true, true));
    let wayland = vars(&[("DISPLAY", ":0"), ("WAYLAND_DISPLAY", "w")]);
    assert!(!local.primary_available(wayland, true, false));
    assert!(!local.primary_available(vars(&[]), true, false));
    for forwarded in [
        Environment { ssh: true, ..local },
        Environment {
            tmux: true,
            ..local
        },
        Environment { wsl: true, ..local },
    ] {
        assert!(!forwarded.primary_available(x11(), true, false));
    }
}

#[test]
fn tmux_forwards_to_the_most_recent_client_with_the_ms_capability() {
    let result = tmux_target(
        || Ok("external\n".to_owned()),
        || Ok("42 /dev/pts/7\n1 /dev/pts/8\n".to_owned()),
        |client| {
            assert_eq!(client, "/dev/pts/7");
            Ok("193: Ms: (string) \\033]52;%p1%s;%p2%s\\a\n".to_owned())
        },
    );

    assert_eq!(result, Ok("/dev/pts/7".to_owned()));
}

#[test]
fn tmux_with_forwarding_off_asks_nothing_more() {
    let result = tmux_target(
        || Ok("off\n".to_owned()),
        || panic!("client should not be queried when forwarding is disabled"),
        |_| panic!("tmux info should not be queried when forwarding is disabled"),
    );

    assert_eq!(
        result,
        Err("tmux clipboard forwarding is disabled".to_owned())
    );
}

#[test]
fn tmux_needs_a_nonempty_ms_capability() {
    for info in ["", "193: Ms: [missing]\n", "193: Ms: (string) \n"] {
        let result = tmux_target(
            || Ok("external\n".to_owned()),
            || Ok("42 /dev/pts/7\n".to_owned()),
            |_| Ok(info.to_owned()),
        );

        assert_eq!(
            result,
            Err("tmux clipboard forwarding is unavailable: missing Ms capability".to_owned())
        );
    }
}

#[test]
fn tmux_needs_an_attached_client() {
    let result = tmux_target(
        || Ok("external\n".to_owned()),
        || Ok("\n".to_owned()),
        |_| panic!("capabilities require an attached client"),
    );

    assert_eq!(
        result,
        Err("tmux clipboard forwarding is unavailable: no attached client".to_owned())
    );
}

/// A native route that waits for the test: `prepare` blocks on `release`
/// (setup that may stall), `native` reports each write.
struct Stalled {
    started: mpsc::Sender<()>,
    release: mpsc::Receiver<()>,
    writes: mpsc::Sender<String>,
}

impl Routes for Stalled {
    fn prepare(&mut self, _environment: Environment) -> Result<(), String> {
        self.started.send(()).unwrap();
        self.release.recv().unwrap();
        Ok(())
    }

    fn native(&mut self, text: &str, _html: Option<&str>) -> Result<Option<Lease>, String> {
        self.writes.send(text.to_owned()).unwrap();
        Ok(None)
    }

    fn windows(&mut self, _text: &str, _deadline: Instant) -> Result<(), String> {
        unreachable!("not WSL")
    }

    fn tmux(&mut self, _text: &str) -> Result<(), String> {
        unreachable!("not tmux")
    }

    fn primary(
        &mut self,
        _text: &str,
        _begin_delivery: &dyn Fn() -> Result<(), String>,
    ) -> Result<Outcome, String> {
        unreachable!("not PRIMARY")
    }
}

struct Rig {
    clipboard: Clipboard,
    started: mpsc::Receiver<()>,
    release: mpsc::Sender<()>,
    written: mpsc::Receiver<String>,
}

fn stalled() -> Rig {
    let (started, started_rx) = mpsc::channel();
    let (release, released) = mpsc::channel();
    let (writes, written) = mpsc::channel();
    let clipboard = Clipboard::with_timeout(
        Stalled {
            started,
            release: released,
            writes,
        },
        Environment::default(),
        Duration::from_millis(80),
    );
    Rig {
        clipboard,
        started: started_rx,
        release,
        written,
    }
}

async fn copy_plain(clipboard: &Clipboard, text: &str) -> Done {
    clipboard
        .copy(text.into(), Format::Plain, Destination::Clipboard)
        .await
}

#[tokio::test]
async fn primary_only_and_combined_copies_write_their_requested_destinations_once() {
    let (mut fake, calls) = Fake::new();
    fake.primary = Box::new(|text| {
        assert_eq!(text, "visit the café");
        Ok(())
    });
    let fake = fake.with_native(|text, _| {
        assert_eq!(text, "clipboard text");
        Ok(None)
    });
    let clipboard = Clipboard::new(fake, local());
    let scope = Arc::new(());
    let owner = PrimaryOwner::new(&scope);

    let primary = clipboard
        .copy(
            "visit the café".into(),
            Format::Plain,
            Destination::Primary(owner.clone()),
        )
        .await;
    assert!(primary.result.is_ok());
    assert_eq!(primary.terminal, None);
    assert_eq!(*calls.lock().unwrap(), ["primary"]);

    let both = clipboard
        .copy(
            "clipboard text".into(),
            Format::Plain,
            Destination::ClipboardAndPrimary("visit the café".into(), owner),
        )
        .await;
    assert_eq!(both.result, Ok(Status::Confirmed));
    assert_eq!(both.terminal, None);
    assert_eq!(*calls.lock().unwrap(), ["primary", "primary", "native"]);
}

#[tokio::test]
async fn a_busy_clipboard_publishes_only_the_latest_owned_primary() {
    for keep_latest in [true, false] {
        let (started, active) = tokio::sync::oneshot::channel();
        let mut started = Some(started);
        let (release, blocked) = mpsc::channel();
        let (mut fake, calls) = Fake::new();
        fake.primary = Box::new(|text| {
            assert_eq!(text, "latest PRIMARY");
            Ok(())
        });
        let fake = fake.with_native(move |text, _| {
            assert_eq!(text, "first clipboard");
            started.take().unwrap().send(()).unwrap();
            blocked.recv().unwrap();
            Ok(None)
        });
        let clipboard = Clipboard::new(fake, local());
        let running = tokio::spawn({
            let clipboard = clipboard.clone();
            async move { copy_plain(&clipboard, "first clipboard").await }
        });
        tokio::time::timeout(Duration::from_secs(1), active)
            .await
            .unwrap()
            .unwrap();

        let older = Arc::new(());
        let latest = Arc::new(());
        let last = clipboard
            .copy(
                "old PRIMARY".into(),
                Format::Plain,
                Destination::Primary(PrimaryOwner::new(&older)),
            )
            .await;
        assert_eq!(last.result, Ok(Status::Busy));
        let last = clipboard
            .copy(
                "second clipboard".into(),
                Format::Plain,
                Destination::ClipboardAndPrimary(
                    "latest PRIMARY".into(),
                    PrimaryOwner::new(&latest),
                ),
            )
            .await;
        assert_eq!(last.result, Ok(Status::Busy));
        let expired = PrimaryOwner::new(&Arc::new(()));
        let late = clipboard
            .copy(
                "expired PRIMARY".into(),
                Format::Plain,
                Destination::Primary(expired),
            )
            .await;
        assert_eq!(late.result, Ok(Status::Busy));
        let _current_owner = keep_latest.then_some(latest);
        release.send(()).unwrap();
        assert_eq!(running.await.unwrap().result, Ok(Status::Confirmed));
        tokio::time::timeout(Duration::from_secs(1), async {
            while clipboard.is_busy() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let expected: &[&str] = if keep_latest {
            &["native", "primary"]
        } else {
            &["native"]
        };
        assert_eq!(calls.lock().unwrap().as_slice(), expected);
    }
}

#[tokio::test]
async fn a_copy_that_finishes_in_time_is_confirmed_and_the_worker_is_free_again() {
    let rig = stalled();
    rig.release.send(()).unwrap();
    rig.release.send(()).unwrap();

    let done = copy_plain(&rig.clipboard, "first").await;

    assert_eq!(
        done,
        Done {
            result: Ok(Status::Confirmed),
            terminal: None
        }
    );
    assert_eq!(
        rig.written.recv_timeout(Duration::from_secs(5)).unwrap(),
        "first"
    );
    assert!(rig.started.try_recv().is_ok());
    assert!(!rig.clipboard.is_busy());
    assert_eq!(
        copy_plain(&rig.clipboard, "second").await.result,
        Ok(Status::Confirmed)
    );
}

#[tokio::test]
async fn an_empty_copy_fails_without_starting_the_worker() {
    let rig = stalled();

    let done = copy_plain(&rig.clipboard, "").await;

    assert_eq!(done.result, Err(EMPTY_MESSAGE.to_owned()));
    assert!(rig.started.try_recv().is_err());
}

#[tokio::test]
async fn an_expired_setup_is_never_delivered_and_the_worker_stays_busy_until_it_returns() {
    let rig = stalled();

    let done = copy_plain(&rig.clipboard, "abandoned").await;

    assert_eq!(done.result, Err(SETUP_TIMEOUT_MESSAGE.to_owned()));
    rig.started.recv_timeout(Duration::from_secs(5)).unwrap();
    // A timeout completes the request, not the blocked worker: no backlog.
    assert!(rig.clipboard.is_busy());
    assert_eq!(
        copy_plain(&rig.clipboard, "retry").await.result,
        Ok(Status::Busy)
    );

    rig.release.send(()).unwrap();
    for _ in 0..500 {
        if !rig.clipboard.is_busy() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(!rig.clipboard.is_busy());
    assert!(rig.written.try_recv().is_err());

    // The next copy runs as if nothing had happened (and here stalls again).
    let done = copy_plain(&rig.clipboard, "fresh").await;
    assert_eq!(done.result, Err(SETUP_TIMEOUT_MESSAGE.to_owned()));
    rig.release.send(()).unwrap();
}

#[test]
fn a_delivery_that_began_cannot_be_abandoned() {
    let setup = Setup::new(Instant::now() + SETUP_TIMEOUT);

    assert_eq!(setup.begin_delivery(), Ok(()));
    assert!(!setup.timed_out(Instant::now() + SETUP_TIMEOUT));
    // Fallback checks cannot revoke a delivery that already began.
    assert_eq!(setup.begin_delivery(), Ok(()));
}

#[test]
fn a_setup_past_its_deadline_refuses_to_begin_delivery() {
    let setup = Setup::new(Instant::now());

    assert_eq!(
        setup.begin_delivery(),
        Err(SETUP_TIMEOUT_MESSAGE.to_owned())
    );
    assert!(setup.timed_out(Instant::now()));
}
