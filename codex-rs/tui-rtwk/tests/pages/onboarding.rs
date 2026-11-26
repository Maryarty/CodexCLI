//! Screen tests of the startup page's `onboarding` directory (S01-S04): its
//! oracle tests are the `onboarding` shard (`tests/oracle/onboarding.rs`);
//! these hold what the shard cannot name: the directory's debug entries.

#[test]
fn onboarding_lists_the_startup_page_states() {
    let dirs = codex_tui_rtwk::app::debug::dirs();
    let dir = dirs
        .iter()
        .find(|d| d.name == "onboarding")
        .expect("the onboarding directory");
    let names: Vec<&str> = (dir.debug_entries)().iter().map(|e| e.name).collect();
    for want in [
        "update_prompt",
        "update_prompt_windows",
        "welcome",
        "trust_git_repo",
        "trust_restricted",
        "flow_update_trust",
    ] {
        assert!(names.contains(&want), "no entry `{want}` in {names:?}");
    }
}

use codex_tui_core::app::{AppEv, Command};
use codex_tui_core::screens::ScreenEv;
use codex_tui_core::screens::onboarding::{
    Ask, Ev, Facts, TrustAsk, TrustCancel, UpdateOffer, Welcome,
};
use codex_tui_core::screens::startup::{Provider, Status};

fn asks(h: &crate::common::Harness) -> Vec<Ask> {
    h.fake
        .commands()
        .into_iter()
        .filter_map(|command| match command {
            Command::Effect {
                ask: codex_tui_core::app::effects::Ask::Startup { ask, .. },
                ..
            } => Some(ask),
            _ => None,
        })
        .collect()
}

fn trust_facts() -> Facts {
    Facts {
        welcome: Some(Welcome { animations: false }),
        trust: Some(TrustAsk {
            cwd: "/workspace/project".into(),
            target: "/workspace/project".into(),
            cancel: TrustCancel::AgentsOverview,
            ..TrustAsk::default()
        }),
        ..Facts::default()
    }
}

/// The launch: the app asks the host for the startup facts once connected,
/// drops an answer to an ask it no longer waits for, and pushes the startup
/// page when the answer awaited has a step due (lib.rs: the trust check
/// runs before the chat). Trusting writes through the host under a new
/// number, and the page leaves to the chat when the write is confirmed.
#[test]
fn the_startup_page_opens_on_the_launch_facts_and_writes_the_trust() {
    let mut h = crate::common::start();
    h.settle();
    assert_eq!(asks(&h), [Ask::Facts { request: 1 }]);
    let epoch = h.fake.epoch().expect("attached");
    let facts = |request| {
        AppEv::Screen(ScreenEv::Onboarding(Ev::Facts {
            request,
            facts: trust_facts(),
        }))
    };
    h.fake.inject(epoch, facts(7));
    h.settle();
    assert!(
        !h.t.screen().contains("Folder access"),
        "a stale answer opens nothing:\n{}",
        h.t.screen()
    );
    h.fake.inject(epoch, facts(1));
    h.wait_for("the trust prompt", |s| s.contains("Folder access"));
    h.press("enter");
    h.settle();
    let asks = asks(&h);
    let [
        Ask::Facts { request: facts },
        Ask::WriteTrust { request, target },
    ] = asks.as_slice()
    else {
        panic!("launch facts and one trust write: {asks:?}");
    };
    assert_eq!(target, "/workspace/project");
    assert_ne!(request, facts);
    assert!(
        h.t.screen().contains("Folder access"),
        "the write is awaited"
    );
    h.fake.inject(
        epoch,
        AppEv::Screen(ScreenEv::Onboarding(Ev::TrustSaved { request: *request })),
    );
    h.wait_for("the chat", |s| !s.contains("Folder access"));
}

/// Oracle `trust_directory.rs` `handle_key_event`: Up goes to the first
/// choice and Down to the last, and neither wraps. The cursor is the shared
/// `Options` controller's, moved by `set`, not by its wrapping `step`.
#[test]
fn the_trust_cursor_stays_at_the_ends() {
    let mut h = crate::common::start();
    h.settle();
    let epoch = h.fake.epoch().expect("attached");
    h.fake.inject(
        epoch,
        AppEv::Screen(ScreenEv::Onboarding(Ev::Facts {
            request: 1,
            facts: trust_facts(),
        })),
    );
    h.wait_for("the trust prompt", |s| s.contains("Folder access"));
    h.press("up");
    h.settle();
    assert!(h.t.screen().contains("› 1. Trust and continue"));
    h.press("down");
    h.press("down");
    h.settle();
    assert!(h.t.screen().contains("› 2. Back to Agent Command Center"));
    h.press("down");
    h.settle();
    assert!(
        h.t.screen().contains("› 2. Back to Agent Command Center"),
        "Down on the last choice stays:\n{}",
        h.t.screen()
    );
    h.press("up");
    h.press("up");
    h.settle();
    assert!(h.t.screen().contains("› 1. Trust and continue"));
}

fn update_facts() -> Facts {
    Facts {
        update: Some(UpdateOffer {
            current: "1.0.0".into(),
            latest: "9.9.9".into(),
            command: "npm install -g @openai/codex".into(),
            choice: None,
        }),
        ..Facts::default()
    }
}

/// R-F1: choosing "Update now" asks the host to store the update; the page
/// stays until the host acknowledges, then the app quits (the oracle
/// returns `RunUpdate` and exits). Quitting first would end the app before
/// the host held the update.
#[test]
fn update_now_quits_the_app_only_after_the_host_stored_the_update() {
    let mut h = crate::common::start_quittable(false);
    h.wait_idle();
    let epoch = h.fake.epoch().expect("attached");
    h.fake.inject(
        epoch,
        AppEv::Screen(ScreenEv::Onboarding(Ev::Facts {
            request: 1,
            facts: update_facts(),
        })),
    );
    h.wait_for("the update prompt", |s| s.contains("Update available"));
    h.press("1");
    h.settle();
    let asks = asks(&h);
    let [Ask::Facts { request: facts }, Ask::RunUpdate { request }] = asks.as_slice() else {
        panic!("launch facts and one stored update: {asks:?}");
    };
    assert_ne!(request, facts);
    assert!(h.t.is_running(), "waits for the host");
    assert!(h.t.screen().contains("Update available"));
    h.fake.inject(
        epoch,
        AppEv::Screen(ScreenEv::Onboarding(Ev::UpdateQueued { request: *request })),
    );
    h.settle();
    assert!(!h.t.is_running(), "quits once the update is stored");
}

/// `--oss` with both or no local server (oracle `oss_selection.rs`): the host asks the provider
/// before the config exists, the panel shows over the startup page, and a row answers the host
/// (the launch task takes it).
#[test]
fn the_provider_panel_shows_before_connect_and_a_row_answers() {
    let mut h = crate::common::start();
    h.settle();
    let epoch = h.fake.epoch().expect("attached");
    h.fake.inject(
        epoch,
        AppEv::Screen(ScreenEv::Onboarding(Ev::OssPick {
            lmstudio: Status::NotRunning,
            ollama: Status::NotRunning,
        })),
    );
    h.wait_for("the provider panel", |s| {
        s.contains("Select an open-source provider")
    });
    assert!(h.t.screen().contains("LM Studio"), "{}", h.t.screen());
    assert!(h.t.screen().contains("Ollama"), "{}", h.t.screen());
    h.press("down");
    h.press("enter");
    h.settle();
    assert_eq!(
        asks(&h).last(),
        Some(&Ask::OssChoice {
            provider: Some(Provider::Ollama)
        })
    );
    assert!(
        !h.t.screen().contains("Select an open-source provider"),
        "{}",
        h.t.screen()
    );
}

/// The oracle ends the launch on Ctrl-C with its error; the page asks the host and waits for
/// that error (`StartFailed`) instead of ending silently.
#[test]
fn ctrl_c_on_the_provider_panel_ends_only_on_the_host_error() {
    let mut h = crate::common::start_quittable(false);
    h.settle();
    let epoch = h.fake.epoch().expect("attached");
    h.fake.inject(
        epoch,
        AppEv::Screen(ScreenEv::Onboarding(Ev::OssPick {
            lmstudio: Status::Running,
            ollama: Status::Running,
        })),
    );
    h.wait_for("the provider panel", |s| {
        s.contains("Select an open-source provider")
    });
    h.press("ctrl+c");
    h.settle();
    assert_eq!(asks(&h).last(), Some(&Ask::OssChoice { provider: None }));
    assert!(h.t.is_running(), "the host's error ends the app");
    h.fake.inject(
        epoch,
        AppEv::StartFailed("OSS provider selection was cancelled by user".into()),
    );
    h.settle();
    assert!(!h.t.is_running());
}

/// The page is held from the pick until the facts: the steps they bring (here the folder trust)
/// open after the panel instead of the page leaving with the last prompt.
#[test]
fn the_startup_steps_still_open_after_the_provider_pick() {
    let mut h = crate::common::start();
    h.settle();
    let epoch = h.fake.epoch().expect("attached");
    h.fake.inject(
        epoch,
        AppEv::Screen(ScreenEv::Onboarding(Ev::OssPick {
            lmstudio: Status::NotRunning,
            ollama: Status::NotRunning,
        })),
    );
    h.wait_for("the provider panel", |s| {
        s.contains("Select an open-source provider")
    });
    h.press("enter");
    h.settle();
    h.fake.inject(
        epoch,
        AppEv::Screen(ScreenEv::Onboarding(Ev::Facts {
            request: 1,
            facts: trust_facts(),
        })),
    );
    h.wait_for("the trust prompt", |s| s.contains("Folder access"));
}

#[test]
fn only_an_accepted_startup_failure_reaches_the_process_exit_slot() {
    use codex_tui_core::app::{Ingress, Session};
    use codex_tui_core::threads::Failure;
    use codex_tui_rtwk::app::UiApp;
    use std::sync::{Arc, Mutex};

    for stale in [true, false] {
        let slot = Arc::new(Mutex::new(None));
        let mut h = crate::common::start_app(|app| {
            app.with_awaited_facts(true)
                .with_startup_failure_slot(slot.clone())
        });
        h.settle();
        let facts = h
            .fake
            .commands()
            .into_iter()
            .find(|command| {
                matches!(
                    command,
                    Command::Effect {
                        ask: codex_tui_core::app::effects::Ask::Startup {
                            ask: Ask::Facts { .. },
                            ..
                        },
                        ..
                    }
                )
            })
            .expect("the launch facts command");
        let epoch = h.fake.epoch().expect("attached");
        let message = "account/read failed";
        if stale {
            let Command::Effect {
                ask:
                    codex_tui_core::app::effects::Ask::Startup {
                        ask: Ask::Facts { request },
                        ..
                    },
                ..
            } = facts
            else {
                panic!("launch facts");
            };
            h.inject([AppEv::Screen(ScreenEv::Onboarding(Ev::Facts {
                request,
                facts: trust_facts(),
            }))]);
        }
        h.fake.inject_ingress(Ingress::host_failure(
            epoch,
            Some(epoch),
            facts.host_ticket().expect("host ticket"),
            message.into(),
        ));
        h.settle();
        assert_eq!(
            slot.lock().expect("launch failure slot").as_deref(),
            (!stale).then_some(message)
        );
        let failed = h.t.read_app(|app: &UiApp| {
            matches!(
                &app.core.session().status, Session::Failed(Failure::Message(why)) if why == message
            )
        });
        assert_eq!(failed, !stale);
    }
}
