#![allow(clippy::expect_used, clippy::unwrap_used)]

//! The doors of the W2 round-1 "needs first" task (doors-r1) that sit in the core: one test per
//! door, each the smallest use a package makes of it.

use std::rc::Rc;

use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Intent, apply_app};
use codex_tui_core::features::windows_sandbox::{Config, Fact, Host, Intent as Sandbox, Mode, Reply};
use codex_tui_core::features::{FeatureCommand, FeatureFact, FeatureIntent, FeatureReply};
use codex_tui_core::ids::{ThreadId, Ticket};
use codex_tui_core::rpc::Reply as Answer;
use codex_tui_core::screens::agents::CenterThreads;
use codex_tui_core::screens::select::Note;
use codex_tui_core::screens::select::permissions::{Label, Preset, Reviewer, Selection};
use codex_tui_core::threads::{NoteKey, ThreadPreviews};

#[test]
fn the_center_names_the_live_thread() {
    assert_eq!(CenterThreads::default().live(), None);
    let id = ThreadId::new("thread-1");
    let threads = CenterThreads::new(Some(id.clone()), ThreadPreviews::default(), Rc::default());
    assert_eq!(threads.live(), Some(&id));
    assert!(threads.preview("thread-1").is_none());
}

fn run(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    app.assert_consistent();
    fx
}

fn sandbox_ready(app: &mut CodexApp, allowed: Option<Vec<Mode>>) {
    let fact = Fact::Environment {
        config: Config {
            mxc: false,
            mode: None,
            requirements: Some(allowed),
        },
        host: Host::Local,
        local_server: true,
        elevated_ready: false,
    };
    run(app, AppEv::Feature(FeatureFact::from(fact)));
}

fn begin_then(app: &mut CodexApp, mode: Mode, then: Selection) -> Option<Ticket> {
    let intent = Sandbox::BeginThen { mode, then };
    let fx = run(
        app,
        AppEv::Intent(Intent::Feature(FeatureIntent::from(intent))),
    );
    fx.commands().iter().find_map(|command| match command {
        Command::Feature {
            ticket,
            command: FeatureCommand::WindowsSandbox(_),
            ..
        } => Some(*ticket),
        _ => None,
    })
}

fn completed(app: &mut CodexApp, mode: Mode, success: bool) {
    let fact = Fact::Completed {
        mode,
        success,
        error: None,
    };
    run(app, AppEv::Feature(FeatureFact::from(fact)));
}

fn started(app: &mut CodexApp, ticket: Ticket) {
    let reply = Answer::feature(FeatureReply::WindowsSandbox(Reply::Started(true)));
    run(app, AppEv::Reply(ticket, reply));
}

fn applied(app: &CodexApp) -> Vec<NoteKey> {
    app.screens()
        .chat
        .notes
        .iter()
        .map(|note| note.key.clone())
        .collect()
}

/// `/permissions` -> `Ask for approval` with no sandbox -> the Windows prompt -> the pick is
/// applied as the sheet's own pick would be, when the setup ends well and not before.
#[test]
fn a_permissions_pick_waits_for_the_windows_sandbox_setup() {
    let pick = Selection::Preset {
        preset: Preset::Auto,
        reviewer: Reviewer::User,
    };
    let said = NoteKey::Select(Note::PermissionsUpdated(Label::Mode {
        preset: Preset::Auto,
        reviewer: Reviewer::User,
    }));
    let mut app = CodexApp::new();
    sandbox_ready(&mut app, None);
    let ticket = begin_then(&mut app, Mode::Elevated, pick.clone()).expect("the setup starts");
    started(&mut app, ticket);
    assert_eq!(applied(&app), []);
    assert_eq!(app.features().windows_sandbox.then(), Some(&pick));
    completed(&mut app, Mode::Elevated, true);
    assert_eq!(applied(&app), [said]);
    assert_eq!(app.features().windows_sandbox.then(), None);

    // A setup that is not allowed is not started and applies nothing.
    let mut app = CodexApp::new();
    sandbox_ready(&mut app, Some(vec![Mode::Elevated]));
    assert_eq!(begin_then(&mut app, Mode::Unelevated, pick.clone()), None);
    assert_eq!(app.features().windows_sandbox.then(), None);

    // An elevated failure is the fallback prompt, which keeps the pick for its own choice.
    let mut app = CodexApp::new();
    sandbox_ready(&mut app, None);
    let ticket = begin_then(&mut app, Mode::Elevated, pick.clone()).expect("the setup starts");
    started(&mut app, ticket);
    completed(&mut app, Mode::Elevated, false);
    assert_eq!(applied(&app), []);
    assert_eq!(app.features().windows_sandbox.then(), Some(&pick));
}
