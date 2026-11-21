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
use codex_tui_core::screens::onboarding::{Ask, Ev, Facts, TrustAsk, TrustCancel, Welcome};

fn asks(h: &crate::common::Harness) -> Vec<Ask> {
    h.fake
        .commands()
        .into_iter()
        .filter_map(|command| match command {
            Command::Startup { ask, .. } => Some(ask),
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
    assert_eq!(
        asks(&h),
        [
            Ask::Facts { request: 1 },
            Ask::WriteTrust {
                request: 2,
                target: "/workspace/project".into(),
            },
        ]
    );
    assert!(
        h.t.screen().contains("Folder access"),
        "the write is awaited"
    );
    h.fake.inject(
        epoch,
        AppEv::Screen(ScreenEv::Onboarding(Ev::TrustSaved { request: 2 })),
    );
    h.wait_for("the chat", |s| !s.contains("Folder access"));
}
