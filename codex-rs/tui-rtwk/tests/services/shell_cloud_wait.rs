//! The empty conversation moves to a Cloud backend with `Ctrl+X b` while the draft stays
//! editable (oracle `app/tests/new_session_tests.rs::cloud_switch_wait_*`, codex-tui 0.170.0,
//! `app/cloud_switch_wait.rs`) on two scripted servers whose Cloud holds its start: nothing is
//! submitted while it connects, the footer says `Connecting to Cloud…`, a refused start says
//! `Cloud conversation did not open` and `draft kept` for the frame that heard it, and the
//! draft's Vim Normal mode goes with it into the Cloud conversation.
//!
//! Port form of the oracle's steps: the oracle drives crossterm events through
//! `wait_for_cloud_switch` (Shift+Down, `TuiEvent::Draw` timing, the 70 ms paste settle, the
//! legacy `Alt+x` Escape of a terminal without enhanced keys, the right-click clipboard paste);
//! rtwk delivers keys and pastes itself and has no legacy-escape recovery, so those steps have
//! no counterpart here (the legacy-escape half of
//! `cloud_switch_wait_preserves_legacy_vim_escape_and_search_acceptance` is n/a: no crossterm
//! event stream, no `enhanced_keys_supported`).

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::disallowed_methods)]

use std::time::Duration;

use codex_tui_core::app::facts::TuiSettings;
use codex_tui_core::app::{AppEv, Command};
use codex_tui_core::ids::{BackendId, ThreadId, Ticket};
use codex_tui_core::rpc::Reply;
use codex_tui_rtwk::app::UiApp;
use codex_tui_rtwk::app::run::{localized, root};
use codex_tui_rtwk::l10n::Lang;
use codex_tui_rtwk::services::agents::workspace::Facts;
use codex_tui_rtwk::services::backend::fake::{FakeHandle, FakeServer};
use codex_tui_rtwk::services::shell::Backend;
use rtwk_test::{Options, Tester};

const CLOUD: BackendId = BackendId(1);

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

fn settle(t: &mut Tester) {
    for _ in 0..30 {
        t.pump_for(Duration::from_millis(20));
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// What the app is started with.
#[derive(Clone, Copy, Default)]
struct Setup {
    /// The config's `tui.vim_mode_default` is on, told before the first conversation starts.
    vim: bool,
    /// On `pump_app`, which gives the app the frame loop's handle: only there does a quit end
    /// the loop.
    quittable: bool,
}

/// An app on two scripted servers whose Cloud leaves its start unanswered, settled on the empty
/// conversation, with `draft` typed (and the typing burst flushed).
fn with_wait(draft: &str, setup: Setup, test: impl FnOnce(&mut Tester, &FakeHandle, &FakeHandle)) {
    let vim = setup.vim;
    let _guard = rtwk_tokio::enter(runtime());
    let (first, local) = FakeServer::new();
    let (second, cloud) = FakeServer::new();
    cloud.announce(Facts {
        cloud: true,
        worktrees: false,
    });
    cloud.hold_thread_start();
    if vim {
        local.hold_thread_start();
    }
    let backend = Backend::Several {
        first: Box::new(Backend::Fake(first)),
        others: vec![(CLOUD, Backend::Fake(second))],
    };
    let app = UiApp::new(backend);
    let mut t = Tester::with(Options::new().size(100, 30).kitty_keyboard(true));
    if setup.quittable {
        t.pump_app(codex_tui_rtwk::app::run::app_view(app));
    } else {
        t.pump_widget(localized(
            root(codex_tui_rtwk::app::run::app_view(app)),
            Lang::En,
        ));
    }
    settle(&mut t);
    if vim {
        local.inject(
            local.epoch().unwrap(),
            AppEv::Tui(TuiSettings {
                vim_mode_default: true,
                ..TuiSettings::default()
            }),
        );
        t.pump_for(Duration::from_millis(50));
        let ticket = starts(&local)[0];
        local.inject(
            local.epoch().unwrap(),
            AppEv::Reply(ticket, Reply::thread_started(ThreadId::new("local-1"))),
        );
        settle(&mut t);
    }
    if !draft.is_empty() {
        t.type_text(draft);
        t.pump_for(Duration::from_millis(50));
    }
    test(&mut t, &local, &cloud);
}

/// The chord that moves the conversation to the next backend.
fn cycle(t: &mut Tester) {
    t.press("ctrl+x");
    t.press("b");
}

/// How many conversations `server` was asked to start, and the ticket of the first.
fn starts(server: &FakeHandle) -> Vec<Ticket> {
    server
        .commands()
        .iter()
        .filter_map(|command| match command {
            Command::StartThread { ticket, .. } => Some(*ticket),
            _ => None,
        })
        .collect()
}

/// A turn or a command that needs a thread reached `server`.
fn turns(server: &FakeHandle) -> usize {
    server
        .commands()
        .iter()
        .filter(|command| matches!(command, Command::StartTurn { .. }))
        .count()
}

/// Cloud answers its held start.
fn answer(cloud: &FakeHandle, reply: Reply) {
    let ticket = starts(cloud)[0];
    cloud.inject(cloud.epoch().unwrap(), AppEv::Reply(ticket, reply));
}

/// The first screen line that holds `needle`, trimmed.
fn line(t: &Tester, needle: &str) -> Option<String> {
    t.screen()
        .lines()
        .find(|row| row.contains(needle))
        .map(|row| row.trim().to_owned())
}

/// `cloud_switch_wait_displays_pending_submission_hint`: `Ctrl+X b` on the empty conversation
/// leaves the footer saying `Connecting to Cloud…` until Cloud answers.
#[test]
fn cloud_wait_displays_pending_submission_hint() {
    with_wait("", Setup::default(), |t, _, cloud| {
        assert!(line(t, "Connecting to Cloud").is_none(), "{}", t.screen());
        cycle(t);
        settle(t);
        assert_eq!(starts(cloud).len(), 1);
        assert_eq!(
            line(t, "Connecting to Cloud").as_deref(),
            Some("Connecting to Cloud…"),
            "{}",
            t.screen()
        );
        answer(cloud, Reply::thread_started(ThreadId::new("cloud-1")));
        settle(t);
        assert!(line(t, "Connecting to Cloud").is_none(), "{}", t.screen());
        assert!(line(t, "Cloud  ").is_some(), "{}", t.screen());
    });
}

/// `cloud_switch_wait_keeps_draft_editable_without_submitting_to_local`: while Cloud connects
/// the draft is edited (a paste, a newline, typed keys) and Enter, Tab and the chord submit,
/// queue and start nothing; a refused start says `Cloud conversation did not open draft kept`
/// and keeps the draft, which is the user's again.
#[test]
fn cloud_wait_keeps_draft_editable_without_submitting_to_local() {
    with_wait(
        "/review first draft",
        Setup::default(),
        |t, local, cloud| {
            let before = turns(local);
            cycle(t);
            settle(t);
            assert_eq!(starts(cloud).len(), 1);
            t.press("enter");
            t.press("tab");
            // the chord during the wait starts nothing and types nothing
            cycle(t);
            t.paste(" more");
            t.press("shift+enter");
            t.type_text("aa");
            t.pump_for(Duration::from_millis(50));
            settle(t);
            assert_eq!(starts(cloud).len(), 1, "a second chord starts nothing");
            assert!(
                t.screen().contains("/review first draft more"),
                "{}",
                t.screen()
            );
            assert!(
                line(t, "aa").is_some_and(|row| row.ends_with("aa")),
                "{}",
                t.screen()
            );
            assert_eq!(turns(local), before);
            assert_eq!(turns(cloud), 0);

            answer(cloud, Reply::failed("Cloud unavailable"));
            // the frame that heard the refusal says it (the oracle's failure frame)
            t.pump_for(Duration::from_millis(20));
            assert!(
                line(t, "Cloud conversation did not open")
                    .is_some_and(|row| row.contains("Cloud conversation did not open draft kept")),
                "{}",
                t.screen()
            );
            assert!(line(t, "Connecting to Cloud").is_none(), "{}", t.screen());
            // the draft is kept and the user's again: it takes keys, and Enter is no longer swallowed
            t.type_text("cc");
            t.pump_for(Duration::from_millis(50));
            assert!(line(t, "aacc").is_some(), "{}", t.screen());
            // the next apply (the wake the app declared) takes the hint away
            std::thread::sleep(Duration::from_millis(1200));
            settle(t);
            assert!(
                line(t, "Cloud conversation did not open").is_none(),
                "{}",
                t.screen()
            );
            assert!(
                t.screen().contains("/review first draft more"),
                "{}",
                t.screen()
            );
        },
    );
}

/// `cloud_switch_wait_routes_ctrl_d_to_the_draft_or_queues_shutdown`: Ctrl+D with text in the
/// draft deletes forward and ends nothing; on the empty draft it quits, and the held start is
/// never adopted (the app has ended).
#[test]
fn cloud_wait_routes_ctrl_d_to_the_draft_or_quits() {
    with_wait(
        "",
        Setup {
            quittable: true,
            ..Setup::default()
        },
        |t, _, cloud| {
            cycle(t);
            settle(t);
            assert_eq!(starts(cloud).len(), 1);
            t.type_text("a");
            // buffered typing counts as draft content
            t.press("ctrl+d");
            assert!(t.is_running(), "ctrl+d with typing held ends nothing");
            t.pump_for(Duration::from_millis(50));
            t.press("home");
            // the draft's only character is deleted forward
            t.press("ctrl+d");
            t.pump_for(Duration::from_millis(50));
            assert!(t.is_running(), "{}", t.screen());
            assert!(!t.screen().contains("› a"), "{}", t.screen());
            // the draft is empty now: quit
            t.press("ctrl+d");
            t.pump_for(Duration::from_millis(200));
            assert!(!t.is_running());
        },
    );
}

/// `cloud_switch_wait_preserves_legacy_vim_escape_and_search_acceptance` (its Vim half): the
/// draft's Normal mode, made while Cloud connected, goes with it into the Cloud conversation,
/// where `x` deletes under the caret instead of typing an `x` (the new draft's start in Insert
/// mode would have undone the Escape).
#[test]
fn cloud_wait_keeps_vim_normal_mode_into_the_cloud_conversation() {
    with_wait(
        "",
        Setup {
            vim: true,
            ..Setup::default()
        },
        |t, _, cloud| {
            // Vim on and a new draft in Insert mode (`tui.vim_mode_default`)
            t.type_text("alpha beta");
            t.pump_for(Duration::from_millis(50));
            cycle(t);
            settle(t);
            assert_eq!(starts(cloud).len(), 1);
            // the Escape of the wait is accepted by the draft
            t.press("escape");
            settle(t);
            assert!(t.screen().contains("Vim: Normal"), "{}", t.screen());
            answer(cloud, Reply::thread_started(ThreadId::new("cloud-1")));
            settle(t);
            assert!(line(t, "Cloud  ").is_some(), "{}", t.screen());
            assert!(t.screen().contains("Vim: Normal"), "{}", t.screen());
            t.press("x");
            settle(t);
            assert!(
                line(t, "alpha bet").is_some_and(|row| row.contains("› alpha bet")),
                "{}",
                t.screen()
            );
            assert!(!t.screen().contains("alpha betx"), "{}", t.screen());
        },
    );
}
