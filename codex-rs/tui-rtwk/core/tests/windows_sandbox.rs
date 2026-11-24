#![allow(clippy::expect_used, clippy::unwrap_used)]

//! The Windows sandbox prompts and `/setup-default-sandbox` (oracle
//! `chatwidget/windows_sandbox_prompts.rs`, `app/platform_actions.rs`, `windows_sandbox.rs`): the
//! degraded sandbox is a fact the permissions page reads, the verb is a real one (no
//! "not available"), the prompts' choices follow the managed requirements, a setup runs one at a
//! time, an elevated failure is the fallback prompt, a non-admin failure is an error line. The
//! words and the sheets are the view's.

use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Intent, apply_app};
use codex_tui_core::commands::SlashCommand;
use codex_tui_core::features::windows_sandbox::{
    Choice, Config, Fact, Host, Intent as Sandbox, Mode, Note as SandboxNote, Prompt, Reply, Wanted,
};
use codex_tui_core::features::{
    FeatureCommand, FeatureFact, FeatureIntent, FeatureNote, FeatureReply, target_of,
};
use codex_tui_core::rpc::Reply as Answer;
use codex_tui_core::threads::NoteKey;
use pretty_assertions::assert_eq;

fn run(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    app.assert_consistent();
    fx
}

fn config(mode: Option<Mode>, allowed: Option<Vec<Mode>>) -> Config {
    Config {
        mxc: false,
        mode,
        requirements: Some(allowed),
    }
}

fn environment(app: &mut CodexApp, config: Config, host: Host) {
    let fact = Fact::Environment {
        config,
        host,
        local_server: host == Host::Local,
        elevated_ready: false,
    };
    run(app, AppEv::Feature(FeatureFact::from(fact)));
}

fn verb(app: &mut CodexApp, intent: Sandbox) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Feature(FeatureIntent::from(intent))),
    )
}

fn elevate_command(app: &mut CodexApp) -> AppFx {
    // `/setup-default-sandbox` is the feature's own command (`Target::Feature`): its view's
    // `open` says this verb.
    assert!(target_of(SlashCommand::ElevateSandbox).is_some());
    verb(app, Sandbox::Elevate)
}

fn starts(fx: &AppFx) -> Vec<(codex_tui_core::ids::Ticket, Mode)> {
    fx.commands()
        .iter()
        .filter_map(|c| match c {
            Command::Feature {
                ticket,
                command: FeatureCommand::WindowsSandbox(Wanted::Start { mode, .. }),
                ..
            } => Some((*ticket, *mode)),
            _ => None,
        })
        .collect()
}

fn notes(app: &CodexApp) -> Vec<SandboxNote> {
    app.screens()
        .chat
        .notes
        .iter()
        .filter_map(|note| match &note.key {
            NoteKey::Feature(FeatureNote::WindowsSandbox(note)) => Some(note.clone()),
            _ => None,
        })
        .collect()
}

fn state(app: &CodexApp) -> &codex_tui_core::features::windows_sandbox::State {
    &app.features().windows_sandbox
}

#[test]
fn the_non_admin_sandbox_is_degraded_and_hints_at_the_default_one() {
    let mut app = CodexApp::new();
    environment(&mut app, config(Some(Mode::Unelevated), None), Host::Local);
    assert!(state(&app).degraded());
    assert!(state(&app).elevate_hint());
    // a requirement that does not allow the admin sandbox takes the hint away, a remote host the label
    environment(
        &mut app,
        config(Some(Mode::Unelevated), Some(vec![Mode::Unelevated])),
        Host::Local,
    );
    assert!(state(&app).degraded());
    assert!(!state(&app).elevate_hint());
    environment(&mut app, config(Some(Mode::Unelevated), None), Host::Remote);
    assert!(!state(&app).degraded());
    environment(&mut app, config(Some(Mode::Elevated), None), Host::Local);
    assert!(!state(&app).degraded());
}

#[test]
fn setup_default_sandbox_starts_the_elevated_setup_only_while_degraded() {
    let mut app = CodexApp::new();
    environment(&mut app, config(Some(Mode::Elevated), None), Host::Local);
    assert_eq!(starts(&elevate_command(&mut app)), []);
    environment(&mut app, config(Some(Mode::Unelevated), None), Host::Local);
    let fx = elevate_command(&mut app);
    let [(_, mode)] = starts(&fx).as_slice().try_into().expect("one start");
    assert_eq!(mode, Mode::Elevated);
    assert!(state(&app).setting_up());
    assert!(state(&app).blocks_thread_switch());
    assert_eq!(notes(&app), []);
}

#[test]
fn a_setup_runs_one_at_a_time_and_a_disallowed_one_is_not_started() {
    let mut app = CodexApp::new();
    environment(
        &mut app,
        config(None, Some(vec![Mode::Elevated])),
        Host::Local,
    );
    assert_eq!(
        starts(&verb(&mut app, Sandbox::Begin(Mode::Unelevated))),
        []
    );
    assert_eq!(notes(&app), [SandboxNote::Disallowed]);
    let fx = verb(&mut app, Sandbox::Begin(Mode::Elevated));
    assert_eq!(starts(&fx).len(), 1);
    // running: a second one is not started and, once started, says nothing
    assert_eq!(starts(&verb(&mut app, Sandbox::Begin(Mode::Elevated))), []);
    assert_eq!(notes(&app), [SandboxNote::Disallowed]);
}

#[test]
fn the_prompts_offer_what_the_requirements_allow() {
    let mut app = CodexApp::new();
    environment(&mut app, config(None, None), Host::Local);
    let page = state(&app).page(Prompt::Enable);
    assert_eq!(
        page.choices,
        [Choice::Elevated, Choice::Legacy, Choice::Quit]
    );
    assert!(!page.required_by_organization);
    assert!(!page.choice_required);
    assert_eq!(Choice::Legacy.mode(), Some(Mode::Unelevated));
    assert_eq!(Choice::Quit.mode(), None);
    assert!(Choice::Legacy.explicit_confirmation());
    assert!(!Choice::Elevated.explicit_confirmation());
    environment(
        &mut app,
        config(Some(Mode::Elevated), Some(vec![Mode::Elevated])),
        Host::Local,
    );
    let page = state(&app).page(Prompt::Fallback);
    assert_eq!(page.choices, [Choice::Elevated, Choice::Quit]);
    assert!(page.required_by_organization);
    assert!(page.choice_required);
}

#[test]
fn an_elevated_failure_is_the_fallback_prompt_and_a_legacy_one_an_error_line() {
    let mut app = CodexApp::new();
    environment(&mut app, config(None, None), Host::Local);
    let fx = verb(&mut app, Sandbox::Begin(Mode::Elevated));
    let [(ticket, _)] = starts(&fx).as_slice().try_into().expect("one start");
    run(
        &mut app,
        AppEv::Reply(
            ticket,
            Answer::feature(FeatureReply::WindowsSandbox(Reply::Started(true))),
        ),
    );
    assert!(state(&app).setting_up());
    run(
        &mut app,
        AppEv::Feature(FeatureFact::from(Fact::Completed {
            mode: Mode::Elevated,
            success: false,
            error: Some("denied".into()),
        })),
    );
    assert_eq!(state(&app).due(), Some(Prompt::Fallback));
    assert!(!state(&app).setting_up());
    verb(&mut app, Sandbox::Shown(Prompt::Fallback));
    assert_eq!(state(&app).due(), None);
    verb(&mut app, Sandbox::Begin(Mode::Unelevated));
    run(
        &mut app,
        AppEv::Feature(FeatureFact::from(Fact::Completed {
            mode: Mode::Unelevated,
            success: false,
            error: Some("denied".into()),
        })),
    );
    assert_eq!(notes(&app), [SandboxNote::SetupFailed("denied".into())]);
}

#[test]
fn a_finished_setup_is_the_configuration_and_another_clients_end_is_not_ours() {
    let mut app = CodexApp::new();
    environment(&mut app, config(None, None), Host::Local);
    verb(&mut app, Sandbox::Begin(Mode::Unelevated));
    run(
        &mut app,
        AppEv::Feature(FeatureFact::from(Fact::Completed {
            mode: Mode::Elevated,
            success: true,
            error: None,
        })),
    );
    assert!(state(&app).setting_up());
    run(
        &mut app,
        AppEv::Feature(FeatureFact::from(Fact::Completed {
            mode: Mode::Unelevated,
            success: true,
            error: None,
        })),
    );
    assert!(!state(&app).setting_up());
    assert!(state(&app).degraded());
    assert_eq!(state(&app).last(), Some((Mode::Unelevated, true)));
}
