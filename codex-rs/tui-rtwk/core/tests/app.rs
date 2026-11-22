//! When the chat thread starts (oracle: after the startup screens, `App::run`
//! creates the session once onboarding, trust and the launch prompts are
//! done). A host that answers the startup facts has the app wait for
//! `Ev::Launched`; without one the thread starts on connect.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use codex_tui_core::app::{
    AppEv, AppFx, CodexApp, Command, FirstThread, Intent, Session, apply_app,
};
use codex_tui_core::ids::ThreadId;
use codex_tui_core::rpc::Reply;
use codex_tui_core::screens::onboarding::{
    Ask, Ev, Facts, Intent as StartupIntent, TrustAsk, TrustDecision,
};
use codex_tui_core::screens::startup::{self, Answer, MigrationOutcome, MigrationPrompt, Prompt};
use codex_tui_core::screens::{ScreenEv, ScreenIntent, agents};
use codex_tui_core::threads::UserMessage;

fn run(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    app.assert_consistent();
    fx
}

fn starts(fx: &AppFx) -> usize {
    fx.commands()
        .iter()
        .filter(|c| matches!(c, Command::StartThread { .. }))
        .count()
}

fn launch_request(fx: &AppFx) -> Option<u64> {
    fx.commands().iter().find_map(|c| match c {
        Command::Startup {
            ask: Ask::Launch { request },
            ..
        } => Some(*request),
        _ => None,
    })
}

/// The launch ask the host gets once the steps are over: after the gateway's
/// device code (none here) is asked for.
fn launch_ask(app: &mut CodexApp, fx: &AppFx) -> u64 {
    if let Some(request) = launch_request(fx) {
        return request;
    }
    let device = fx
        .commands()
        .iter()
        .find_map(|c| match c {
            Command::Startup {
                ask: Ask::GatewayDevice { request },
                ..
            } => Some(*request),
            _ => None,
        })
        .expect("the device code is asked before the launch");
    let fx = run(
        app,
        AppEv::Screen(ScreenEv::Onboarding(Ev::GatewayDevice {
            request: device,
            code: None,
        })),
    );
    assert_eq!(starts(&fx), 0);
    launch_request(&fx).expect("then the launch")
}

fn facts(app: &mut CodexApp, facts: Facts) -> AppFx {
    run(
        app,
        AppEv::Screen(ScreenEv::Onboarding(Ev::Facts { request: 1, facts })),
    )
}

fn trust_facts() -> Facts {
    Facts {
        trust: Some(TrustAsk {
            cwd: "/work".into(),
            target: "/work".into(),
            ..TrustAsk::default()
        }),
        ..Facts::default()
    }
}

fn launched(app: &mut CodexApp, request: u64) -> AppFx {
    launched_with(app, request, Vec::new())
}

fn launched_with(app: &mut CodexApp, request: u64, prompts: Vec<Prompt>) -> AppFx {
    run(
        app,
        AppEv::Screen(ScreenEv::Onboarding(Ev::Launched { request, prompts })),
    )
}

/// What the startup page does with the first due prompt: puts it on screen,
/// then tells the onboarding model it was asked.
fn ask_next_prompt(app: &mut CodexApp, prompt: Prompt) -> AppFx {
    let fx = run(
        app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Startup(startup::Intent::Ask(
            prompt,
        )))),
    );
    assert_eq!(starts(&fx), 0, "an open prompt holds the thread");
    intent(app, StartupIntent::PromptAsked)
}

fn answer(app: &mut CodexApp, answer: Answer) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Startup(
            startup::Intent::Answer(answer),
        ))),
    )
}

fn saved_thread(id: &str) -> codex_app_server_protocol::Thread {
    serde_json::from_value(serde_json::json!({
        "id": id, "environments": null, "extra": null, "sessionId": id,
        "forkedFromId": null, "attachedToThreadId": null, "parentThreadId": null,
        "preview": id, "ephemeral": false, "section": null, "sectionEnteredAt": null,
        "projectId": null, "historyMode": "legacy", "modelProvider": "openai",
        "model": null, "reasoningEffort": null, "createdAt": 1_700_000_000,
        "updatedAt": 1_700_000_000, "recencyAt": 1_700_000_000,
        "status": {"type": "idle"}, "path": null,
        "cwd": "/tmp/project", "cliVersion": "0.0.0", "originator": null, "source": "cli",
        "canAcceptDirectInput": true, "threadSource": null,
        "agentNickname": null, "agentRole": null, "gitInfo": null, "name": id,
        "daybreakEnabled": null, "turns": []
    }))
    .expect("a thread")
}

fn migration() -> Prompt {
    Prompt::Migration(MigrationPrompt {
        current_model: "gpt-old".into(),
        target_model: "gpt-new".into(),
        target_display_name: "GPT New".into(),
        target_description: None,
        copy: None,
        link: None,
        markdown: None,
        can_opt_out: true,
        effort: None,
    })
}

/// The model the thread start carries, when it goes out.
fn start_model(fx: &AppFx) -> Option<Option<String>> {
    fx.commands().iter().find_map(|c| match c {
        Command::StartThread { model, .. } => Some(model.clone()),
        _ => None,
    })
}

fn intent(app: &mut CodexApp, intent: StartupIntent) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Onboarding(intent))),
    )
}

fn page_left(app: &mut CodexApp) -> AppFx {
    intent(app, StartupIntent::StartPageLeft)
}

fn gated() -> CodexApp {
    CodexApp::new().with_first_thread(FirstThread::AfterLaunch)
}

#[test]
fn without_a_startup_host_the_thread_starts_on_connect() {
    let mut app = CodexApp::new();
    let fx = run(&mut app, AppEv::Connected);
    assert_eq!(starts(&fx), 1);
}

#[test]
fn a_startup_screen_holds_the_thread_until_the_launch_is_known() {
    let mut app = gated();
    let fx = run(&mut app, AppEv::Connected);
    assert_eq!(starts(&fx), 0, "{:?}", fx.commands());
    assert_eq!(app.session().status, Session::Starting);
    let fx = facts(&mut app, trust_facts());
    assert_eq!(starts(&fx), 0);
    intent(&mut app, StartupIntent::Trust);
    let fx = run(
        &mut app,
        AppEv::Screen(ScreenEv::Onboarding(Ev::TrustSaved { request: 2 })),
    );
    assert_eq!(starts(&fx), 0, "the prompts are still to be read");
    let request = launch_ask(&mut app, &fx);
    let fx = launched(&mut app, request);
    assert_eq!(starts(&fx), 1, "{:?}", fx.commands());
}

#[test]
fn with_no_step_to_show_the_thread_starts_when_the_launch_is_known() {
    let mut app = gated();
    run(&mut app, AppEv::Connected);
    let fx = facts(&mut app, Facts::default());
    assert_eq!(starts(&fx), 0);
    let request = launch_ask(&mut app, &fx);
    assert_eq!(starts(&launched(&mut app, request)), 1);
}

#[test]
fn a_stale_launch_answer_starts_nothing() {
    let mut app = gated();
    run(&mut app, AppEv::Connected);
    let fx = facts(&mut app, Facts::default());
    let request = launch_ask(&mut app, &fx);
    assert_eq!(starts(&launched(&mut app, request + 7)), 0);
}

#[test]
fn a_cancelled_trust_never_opens_a_thread() {
    let mut app = gated();
    run(&mut app, AppEv::Connected);
    facts(&mut app, trust_facts());
    let fx = intent(&mut app, StartupIntent::CancelTrust);
    assert_eq!(starts(&fx), 0);
    assert_eq!(
        app.screens().onboarding.trust.as_ref().unwrap().decision,
        Some(TrustDecision::Cancelled)
    );
    assert!(launch_request(&fx).is_none());
}

// The composer's asks before the first thread (BeforeThread) wait in order
// and go out when the held thread starts.
#[test]
fn asks_made_during_the_startup_steps_drain_in_order_when_the_thread_starts() {
    let mut app = gated();
    run(&mut app, AppEv::Connected);
    let fx = facts(&mut app, Facts::default());
    let request = launch_ask(&mut app, &fx);
    for text in ["first", "second"] {
        let fx = run(
            &mut app,
            AppEv::Intent(Intent::Submit {
                thread: None,
                message: UserMessage::text(text),
            }),
        );
        assert!(fx.commands().is_empty());
    }
    let fx = launched(&mut app, request);
    let ticket = fx
        .commands()
        .iter()
        .find_map(|c| match c {
            Command::StartThread { ticket, .. } => Some(*ticket),
            _ => None,
        })
        .expect("the thread starts");
    let fx = run(
        &mut app,
        AppEv::Reply(ticket, Reply::thread_started(ThreadId::new("t-1"))),
    );
    let sent: Vec<String> = fx
        .commands()
        .iter()
        .filter_map(|c| match c {
            Command::StartTurn { message, .. } => Some(message.text.clone()),
            _ => None,
        })
        .collect();
    // The first message starts the turn; the second waits behind it in the
    // thread's queue (oracle: a message typed while a turn runs is queued).
    assert_eq!(sent, ["first"]);
    let queued: Vec<&str> = app
        .thread(&ThreadId::new("t-1"))
        .expect("the thread is open")
        .queue()
        .iter()
        .map(|m| m.text.as_str())
        .collect();
    assert_eq!(queued, ["second"]);
}

// A new connection after the launch is known has nothing left to wait for.
#[test]
fn a_reconnect_after_the_launch_starts_the_thread_at_once() {
    let mut app = gated();
    run(&mut app, AppEv::Connected);
    let fx = facts(&mut app, Facts::default());
    let request = launch_ask(&mut app, &fx);
    launched(&mut app, request);
    run(&mut app, AppEv::Disconnected);
    let fx = run(&mut app, AppEv::Connected);
    assert_eq!(starts(&fx), 1, "{:?}", fx.commands());
}

// The oracle starts the session after the launch prompts are answered
// (`App::run`, before `spawn_startup_thread_start`): known is not enough.
#[test]
fn an_open_startup_prompt_holds_the_thread_until_it_is_answered() {
    let mut app = gated();
    run(&mut app, AppEv::Connected);
    let fx = facts(&mut app, Facts::default());
    let request = launch_ask(&mut app, &fx);
    let fx = launched_with(&mut app, request, vec![Prompt::GovCloud]);
    assert_eq!(starts(&fx), 0, "the prompt is still to be asked");
    let fx = ask_next_prompt(&mut app, Prompt::GovCloud);
    assert_eq!(starts(&fx), 0, "the prompt is on screen");
    let fx = answer(&mut app, Answer::GovCloud);
    assert_eq!(starts(&fx), 1, "{:?}", fx.commands());
    assert_eq!(start_model(&fx), Some(None));
}

// A prompt that arrives with the launch event itself is on screen at once
// (the page asks it after the event), so the thread waits in that arm too.
#[test]
fn a_prompt_in_the_launch_event_holds_the_thread_in_the_event_arm() {
    let mut app = gated();
    run(&mut app, AppEv::Connected);
    let fx = facts(&mut app, Facts::default());
    let request = launch_ask(&mut app, &fx);
    let fx = launched_with(&mut app, request, vec![migration()]);
    assert_eq!(starts(&fx), 0, "{:?}", fx.commands());
    assert!(app.screens().onboarding.due());
}

#[test]
fn an_accepted_migration_starts_the_thread_on_the_new_model() {
    let mut app = gated();
    run(&mut app, AppEv::Connected);
    let fx = facts(&mut app, Facts::default());
    let request = launch_ask(&mut app, &fx);
    launched_with(&mut app, request, vec![migration()]);
    ask_next_prompt(&mut app, migration());
    let fx = answer(&mut app, Answer::Migration(MigrationOutcome::Accepted));
    assert_eq!(start_model(&fx), Some(Some("gpt-new".to_string())));
}

#[test]
fn a_rejected_migration_starts_the_thread_on_the_current_model() {
    let mut app = gated();
    run(&mut app, AppEv::Connected);
    let fx = facts(&mut app, Facts::default());
    let request = launch_ask(&mut app, &fx);
    launched_with(&mut app, request, vec![migration()]);
    ask_next_prompt(&mut app, migration());
    let fx = answer(&mut app, Answer::Migration(MigrationOutcome::Rejected));
    assert_eq!(start_model(&fx), Some(None));
}

// The ask for the facts or the launch is lost with a remote connection
// (`reconnect.rs`: Reconnecting, then Reconnected, no new `Connected`): the
// startup steps ask again and the held thread goes out.
#[test]
fn a_remote_reconnect_before_the_launch_asks_again_and_starts() {
    let mut app = gated();
    run(&mut app, AppEv::Connected);
    let fx = facts(&mut app, Facts::default());
    let lost = launch_ask(&mut app, &fx);
    run(
        &mut app,
        AppEv::Screen(ScreenEv::Agents(agents::Ev::Reconnecting)),
    );
    let fx = run(
        &mut app,
        AppEv::Screen(ScreenEv::Agents(agents::Ev::Reconnected)),
    );
    let again = launch_request(&fx).expect("the launch is asked again");
    assert_ne!(again, lost);
    assert_eq!(
        starts(&launched(&mut app, lost)),
        0,
        "the old answer is stale"
    );
    assert_eq!(starts(&launched(&mut app, again)), 1);
}

#[test]
fn a_remote_reconnect_before_the_facts_asks_for_them_again() {
    let mut app = gated();
    let fx = run(&mut app, AppEv::Connected);
    let asked = |fx: &AppFx| {
        fx.commands()
            .iter()
            .filter(|c| {
                matches!(
                    c,
                    Command::Startup {
                        ask: Ask::Facts { .. },
                        ..
                    }
                )
            })
            .count()
    };
    assert_eq!(asked(&fx), 1);
    run(
        &mut app,
        AppEv::Screen(ScreenEv::Agents(agents::Ev::Reconnecting)),
    );
    let fx = run(
        &mut app,
        AppEv::Screen(ScreenEv::Agents(agents::Ev::Reconnected)),
    );
    assert_eq!(asked(&fx), 1, "{:?}", fx.commands());
}

// `codex agents` and `codex resume` open a page instead of a fresh thread
// (oracle: `SessionSelection::Resume`/overview start no session).
#[test]
fn a_page_start_opens_no_fresh_thread_until_asked() {
    let mut app = CodexApp::new().with_first_thread(FirstThread::Never);
    let fx = run(&mut app, AppEv::Connected);
    assert_eq!(starts(&fx), 0);
    assert_eq!(app.session().status, Session::Ready);
    let fx = page_left(&mut app);
    assert_eq!(starts(&fx), 1);
    let fx = page_left(&mut app);
    assert_eq!(starts(&fx), 0, "one start is on its way");
}

// The picker's pick opens the thread through the app; leaving the picker
// after it starts no second, fresh one.
#[test]
fn leaving_the_picker_after_a_pick_starts_no_fresh_thread() {
    let mut app = CodexApp::new().with_first_thread(FirstThread::Never);
    run(&mut app, AppEv::Connected);
    run(
        &mut app,
        AppEv::Screen(ScreenEv::Agents(agents::Ev::Listed(agents::Page {
            threads: vec![saved_thread("saved-1")],
            unread: Vec::new(),
            next_cursor: None,
            append: false,
            pins: None,
        }))),
    );
    let fx = run(
        &mut app,
        AppEv::Intent(Intent::OpenThread(ThreadId::new("saved-1"))),
    );
    assert_eq!(starts(&fx), 0);
    assert!(!fx.commands().is_empty(), "the resume is asked");
    assert_eq!(starts(&page_left(&mut app)), 0);
}

#[test]
fn a_fresh_thread_is_not_asked_for_when_the_thread_starts_on_its_own() {
    let mut app = CodexApp::new();
    run(&mut app, AppEv::Connected);
    let fx = page_left(&mut app);
    assert_eq!(starts(&fx), 0);
}
