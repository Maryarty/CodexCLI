#![allow(clippy::expect_used, clippy::unwrap_used)]

//! The security-setup reminder (oracle `security_setup.rs`, `chatwidget/security_setup.rs`): the
//! notice the host fetched and verified is a fact, a notice of another identity is another
//! occurrence (a dismissal is the banner's, per occurrence), a stale one hides, an invalid one is
//! dropped, the banner's action opens its page in the browser and a page that did not open is the
//! oracle's error. The words and the banner are the view's.

use codex_tui_core::app::effects::{Ask, Done};
use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Intent, apply_app};
use codex_tui_core::features::security_setup::{Action, Fact, Identity, Intent as Open, Notice};
use codex_tui_core::features::{FeatureFact, FeatureIntent};
use pretty_assertions::assert_eq;

fn run(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    app.assert_consistent();
    fx
}

fn identity(user: &str) -> Identity {
    Identity {
        account: "acct".into(),
        user: user.into(),
    }
}

fn notice(title: &str, url: &str) -> Notice {
    Notice {
        title: title.into(),
        description: "Protect your account.".into(),
        action: Action {
            label: "Set up".into(),
            url: url.into(),
        },
    }
}

fn fact(app: &mut CodexApp, fact: Fact) {
    run(app, AppEv::Feature(FeatureFact::from(fact)));
}

fn shown(app: &CodexApp) -> Option<(String, u64)> {
    let state = &*app.features().security_setup;
    state
        .notice()
        .map(|notice| (notice.title.clone(), state.occurrence()))
}

#[test]
fn a_notice_is_held_and_the_same_identity_is_the_same_occurrence() {
    let mut app = CodexApp::new();
    assert_eq!(shown(&app), None);
    fact(
        &mut app,
        Fact::Loaded {
            identity: identity("u1"),
            notice: notice("Add a key", "https://chatgpt.com/a"),
        },
    );
    assert_eq!(shown(&app), Some(("Add a key".into(), 1)));
    fact(
        &mut app,
        Fact::Loaded {
            identity: identity("u1"),
            notice: notice("Add two keys", "https://chatgpt.com/a"),
        },
    );
    assert_eq!(shown(&app), Some(("Add two keys".into(), 1)));
    fact(
        &mut app,
        Fact::Loaded {
            identity: identity("u2"),
            notice: notice("Add a key", "https://chatgpt.com/a"),
        },
    );
    assert_eq!(shown(&app), Some(("Add a key".into(), 2)));
}

#[test]
fn an_invalidated_notice_hides_and_an_invalid_one_is_dropped() {
    let mut app = CodexApp::new();
    fact(
        &mut app,
        Fact::Loaded {
            identity: identity("u1"),
            notice: notice("Add a key", "https://chatgpt.com/a"),
        },
    );
    fact(&mut app, Fact::Invalidated);
    assert_eq!(shown(&app), None);
    // the occurrence is kept until another identity is verified
    fact(
        &mut app,
        Fact::Loaded {
            identity: identity("u1"),
            notice: notice("Add a key", "https://chatgpt.com/a"),
        },
    );
    assert_eq!(shown(&app), Some(("Add a key".into(), 1)));
    let mut app = CodexApp::new();
    fact(
        &mut app,
        Fact::Loaded {
            identity: identity("u1"),
            notice: notice("  ", "https://chatgpt.com/a"),
        },
    );
    fact(
        &mut app,
        Fact::Loaded {
            identity: identity("u1"),
            notice: notice("a\nb", "https://chatgpt.com/a"),
        },
    );
    assert_eq!(shown(&app), None);
}

#[test]
fn the_banner_action_opens_its_page_and_a_failure_is_an_error_line() {
    let mut app = CodexApp::new();
    // no notice: nothing is opened
    let fx = run(
        &mut app,
        AppEv::Intent(Intent::Feature(FeatureIntent::from(Open::Open))),
    );
    assert!(fx.commands().is_empty());
    fact(
        &mut app,
        Fact::Loaded {
            identity: identity("u1"),
            notice: notice("Add a key", "https://chatgpt.com/a"),
        },
    );
    let fx = run(
        &mut app,
        AppEv::Intent(Intent::Feature(FeatureIntent::from(Open::Open))),
    );
    let [Command::Effect { request, ask, .. }] = fx.commands() else {
        panic!("expected one ask, got {:?}", fx.commands());
    };
    assert_eq!(ask, &Ask::open_url("https://chatgpt.com/a".into()));
    run(
        &mut app,
        AppEv::Effect {
            request: *request,
            done: Done::Opened(Err("no browser".into())),
        },
    );
    let said = app.screens().chat.notes.len() + app.active_thread().transcript().iter().count();
    assert!(said > 0, "the failure is said");
}
