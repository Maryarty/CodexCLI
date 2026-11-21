//! Moving an empty conversation to the next backend with the leader chord
//! `Ctrl+X b` (oracle `change_host` -> `CycleNewConversationBackend`,
//! `app/tests/key_chords.rs::empty_thread_backend_chord_preserves_draft_until_first_submission`,
//! codex-tui 0.170.0) on two scripted servers: the control above the
//! composer names the displayed backend and the key, the chord starts the
//! conversation on the other backend (the RPC goes there), the draft goes
//! with it, and the control ends with the first submission.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::disallowed_methods)]

use std::time::Duration;

use codex_tui_core::app::Command;
use codex_tui_core::ids::BackendId;
use codex_tui_rtwk::app::UiApp;
use codex_tui_rtwk::app::run::{localized, root};
use codex_tui_rtwk::l10n::Lang;
use codex_tui_rtwk::services::agents::workspace::Facts;
use codex_tui_rtwk::services::backend::fake::FakeServer;
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

/// The line of the screen that starts with `name`, trimmed (the oracle's
/// `screen.lines().find(|line| line.trim_start().starts_with("Cloud"))`).
fn control(t: &Tester, name: &str) -> Option<String> {
    t.screen()
        .lines()
        .find(|line| line.trim_start().starts_with(name))
        .map(|line| line.trim().to_owned())
}

/// How many conversations the server was asked to start.
fn started(server: &codex_tui_rtwk::services::backend::fake::FakeHandle) -> usize {
    server
        .commands()
        .iter()
        .filter(|command| matches!(command, Command::StartThread { .. }))
        .count()
}

#[test]
fn empty_thread_backend_chord_preserves_draft_until_first_submission() {
    let guard = rtwk_tokio::enter(runtime());
    let (first, local_fake) = FakeServer::new();
    let (second, cloud_fake) = FakeServer::new();
    cloud_fake.announce(Facts {
        cloud: true,
        worktrees: false,
    });
    let backend = Backend::Several {
        first: Box::new(Backend::Fake(first)),
        others: vec![(CLOUD, Backend::Fake(second))],
    };
    let app = UiApp::new(backend);
    let mut t = Tester::with(Options::new().size(100, 30));
    t.pump_widget(localized(
        root(codex_tui_rtwk::app::run::app_view(app)),
        Lang::En,
    ));
    settle(&mut t);
    assert_eq!(
        control(&t, "Local").as_deref(),
        Some("Local  \u{2303}x b to cycle"),
        "{}",
        t.screen()
    );
    assert_eq!(started(&cloud_fake), 0);

    t.type_text("first draft");
    // the chars arrived as one burst: the composer holds them until the
    // typing stops (the page's chord does not flush them)
    t.pump_for(Duration::from_millis(50));
    // `Ctrl+X` alone does nothing; `b` completes the chord.
    t.press("ctrl+x");
    t.press("b");
    settle(&mut t);
    assert_eq!(
        control(&t, "Cloud").as_deref(),
        Some("Cloud  \u{2303}x b to cycle"),
        "{}",
        t.screen()
    );
    assert_eq!(started(&cloud_fake), 1, "the cloud started it");
    assert!(t.screen().contains("first draft"), "{}", t.screen());
    assert!(control(&t, "Local").is_none());

    // Round trip back to the local backend, the draft still with it.
    t.press("ctrl+x");
    t.press("b");
    settle(&mut t);
    assert!(control(&t, "Local").is_some(), "{}", t.screen());
    assert!(t.screen().contains("first draft"), "{}", t.screen());
    assert_eq!(started(&local_fake), 2);

    // A chord with another second key is not the backend chord.
    t.press("ctrl+x");
    t.press("q");
    settle(&mut t);
    assert!(control(&t, "Local").is_some());
    assert_eq!(started(&cloud_fake), 1);

    // The first submission ends the offer.
    t.press("enter");
    settle(&mut t);
    assert!(control(&t, "Local").is_none(), "{}", t.screen());
    assert!(control(&t, "Cloud").is_none());
    drop(guard);
}
