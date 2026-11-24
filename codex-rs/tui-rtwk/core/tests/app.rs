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
use codex_tui_core::screens::onboarding::session::Start;
use codex_tui_core::screens::onboarding::{
    Ask, Ev, Facts, Intent as StartupIntent, TrustAsk, TrustDecision,
};
use codex_tui_core::screens::startup::cwd::{ResumeMode, Setting};
use codex_tui_core::screens::startup::{
    self, Action, Answer, CwdSelection, HistoryLine, MigrationOutcome, MigrationPrompt, Prompt,
};
use codex_tui_core::screens::{ScreenCommand, ScreenEv, ScreenIntent, ScreenReply, agents};
use codex_tui_core::threads::{ThreadAsk, UserMessage};

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

// `codex resume <id>` and `codex fork <id>` (oracle: `run_ratatui_app`,
// `lookup_session_target_with_app_server` and the cwd prompt).

fn named(action: Action) -> CodexApp {
    gated().with_start_session(Start {
        action,
        id: "saved-1".into(),
    })
}

/// The thread RPCs of the effects, in order.
fn thread_asks(fx: &AppFx) -> Vec<(codex_tui_core::ids::Ticket, ThreadAsk)> {
    fx.commands()
        .iter()
        .filter_map(|c| match c {
            Command::Screen {
                ticket,
                command: ScreenCommand::Agents(agents::Command::Thread(ask)),
                ..
            } => Some((*ticket, ask.clone())),
            _ => None,
        })
        .collect()
}

fn session_facts(configured: Option<ResumeMode>) -> Facts {
    Facts {
        resume: Setting {
            current: "/work/now".into(),
            configured,
            ..Setting::default()
        },
        ..Facts::default()
    }
}

/// Connected, the session read answered with `cwd` as its directory.
fn read_session(app: &mut CodexApp, cwd: &str) {
    let fx = run(app, AppEv::Connected);
    assert_eq!(starts(&fx), 0, "{:?}", fx.commands());
    let asks = thread_asks(&fx);
    let [(ticket, ThreadAsk::Read { thread })] = asks.as_slice() else {
        panic!("the session is read at connect: {asks:?}");
    };
    assert_eq!(thread, "saved-1");
    let mut saved = saved_thread("saved-1");
    saved.cwd = cwd.try_into().expect("an absolute path");
    let fx = run(
        app,
        AppEv::Reply(
            *ticket,
            Reply::screen(ScreenReply::Agents(agents::Reply::Read {
                thread: Box::new(saved),
                unread: false,
            })),
        ),
    );
    assert!(launch_request(&fx).is_none(), "the facts are not known yet");
}

fn session_launch(app: &mut CodexApp, facts_in: Facts) -> (AppFx, u64) {
    let fx = facts(app, facts_in);
    let request = launch_ask(app, &fx);
    (fx, request)
}

#[test]
fn a_named_session_opens_instead_of_a_fresh_thread() {
    let mut app = named(Action::Resume);
    read_session(&mut app, "/tmp/project");
    let (_, request) = session_launch(&mut app, session_facts(Some(ResumeMode::Session)));
    let fx = launched(&mut app, request);
    assert_eq!(starts(&fx), 0, "{:?}", fx.commands());
    let asks = thread_asks(&fx);
    let [(ticket, ThreadAsk::ResumeAt { thread, cwd })] = asks.as_slice() else {
        panic!("the session is resumed: {asks:?}");
    };
    assert_eq!(
        (thread.as_str(), cwd.as_deref()),
        ("saved-1", Some("/tmp/project"))
    );
    assert_eq!(app.session().status, Session::Starting);
    run(
        &mut app,
        AppEv::Reply(
            *ticket,
            Reply::screen(ScreenReply::Agents(agents::Reply::Resumed(Box::new(
                saved_thread("saved-1"),
            )))),
        ),
    );
    assert_eq!(app.session().active, Some(ThreadId::new("saved-1")));
    assert_eq!(app.session().status, Session::Ready);
}

#[test]
fn a_forked_session_opens_the_copy() {
    let mut app = named(Action::Fork);
    read_session(&mut app, "/tmp/project");
    let (_, request) = session_launch(&mut app, session_facts(Some(ResumeMode::Current)));
    let fx = launched(&mut app, request);
    let asks = thread_asks(&fx);
    let [(ticket, ThreadAsk::ForkAt { thread, cwd, .. })] = asks.as_slice() else {
        panic!("the session is forked: {asks:?}");
    };
    assert_eq!(
        (thread.as_str(), cwd.as_deref()),
        ("saved-1", Some("/work/now"))
    );
    run(
        &mut app,
        AppEv::Reply(
            *ticket,
            Reply::screen(ScreenReply::Agents(agents::Reply::Forked(Box::new(
                saved_thread("copy-1"),
            )))),
        ),
    );
    assert_eq!(app.session().active, Some(ThreadId::new("copy-1")));
}

// The directory differs and nothing is remembered: the cwd prompt comes
// first, before the launch's prompts, and the answer picks the directory.
#[test]
fn another_directory_asks_before_the_launch_prompts() {
    for (selection, expected) in [
        (CwdSelection::Session, "/tmp/project"),
        (CwdSelection::Current, "/work/now"),
    ] {
        let mut app = named(Action::Resume);
        read_session(&mut app, "/tmp/project");
        let (_, request) = session_launch(&mut app, session_facts(None));
        let fx = launched_with(&mut app, request, vec![migration()]);
        assert!(thread_asks(&fx).is_empty() && starts(&fx) == 0);
        let first = app.screens().onboarding.prompts[0].clone();
        let Prompt::Cwd(prompt) = &first else {
            panic!("the cwd prompt is first: {first:?}");
        };
        assert_eq!(
            (prompt.current.as_str(), prompt.session.as_str()),
            ("/work/now", "/tmp/project")
        );
        ask_next_prompt(&mut app, first);
        let fx = answer(&mut app, Answer::Cwd(selection));
        assert!(thread_asks(&fx).is_empty(), "the migration prompt is next");
        let second = app.screens().onboarding.prompts[0].clone();
        assert_eq!(second, migration());
        ask_next_prompt(&mut app, second);
        let fx = answer(&mut app, Answer::Migration(MigrationOutcome::Rejected));
        let asks = thread_asks(&fx);
        let [(_, ThreadAsk::ResumeAt { cwd, .. })] = asks.as_slice() else {
            panic!("the session opens once the prompts are over: {asks:?}");
        };
        assert_eq!(cwd.as_deref(), Some(expected));
        assert_eq!(starts(&fx), 0);
    }
}

#[test]
fn leaving_the_cwd_prompt_opens_nothing() {
    let mut app = named(Action::Resume);
    read_session(&mut app, "/tmp/project");
    let (_, request) = session_launch(&mut app, session_facts(None));
    launched(&mut app, request);
    let first = app.screens().onboarding.prompts[0].clone();
    ask_next_prompt(&mut app, first);
    let fx = answer(&mut app, Answer::CwdExit);
    assert!(thread_asks(&fx).is_empty() && starts(&fx) == 0);
}

// "Always use ..." is written to the config; a write that fails leaves a line
// for the history, the choice still opens the session.
#[test]
fn a_remembered_choice_is_saved_and_a_failed_save_leaves_a_line() {
    let mut app = named(Action::Resume);
    read_session(&mut app, "/tmp/project");
    let (_, request) = session_launch(&mut app, session_facts(None));
    launched(&mut app, request);
    let first = app.screens().onboarding.prompts[0].clone();
    ask_next_prompt(&mut app, first);
    let fx = answer(&mut app, Answer::Cwd(CwdSelection::SessionAndRemember));
    let save = fx
        .commands()
        .iter()
        .find_map(|c| match c {
            Command::Startup {
                ask: Ask::SaveResumeCwd { request, mode },
                ..
            } => Some((*request, *mode)),
            _ => None,
        })
        .expect("the choice is saved");
    assert_eq!(save.1, ResumeMode::Session);
    assert!(
        thread_asks(&fx)
            .iter()
            .any(|(_, ask)| matches!(ask, ThreadAsk::ResumeAt { .. }))
    );
    run(
        &mut app,
        AppEv::Screen(ScreenEv::Startup(startup::Ev::CwdSaved {
            request: save.0 + 5,
            failed: true,
        })),
    );
    assert!(app.screens().startup.history.is_empty(), "a stale answer");
    run(
        &mut app,
        AppEv::Screen(ScreenEv::Startup(startup::Ev::CwdSaved {
            request: save.0,
            failed: true,
        })),
    );
    assert_eq!(
        app.screens().startup.history,
        [HistoryLine::CwdPreferenceNotSaved]
    );
}

#[test]
fn an_unknown_id_ends_the_launch_with_the_oracles_message() {
    let mut app = named(Action::Fork);
    let fx = run(&mut app, AppEv::Connected);
    let (ticket, _) = thread_asks(&fx).remove(0);
    run(
        &mut app,
        AppEv::Reply(ticket, Reply::failed("thread/read: no rollout found")),
    );
    let fx = facts(&mut app, session_facts(None));
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
        .expect("the device code is asked first");
    let fx = run(
        &mut app,
        AppEv::Screen(ScreenEv::Onboarding(Ev::GatewayDevice {
            request: device,
            code: None,
        })),
    );
    let message = fx
        .commands()
        .iter()
        .find_map(|c| match c {
            Command::Startup {
                ask: Ask::FailLaunch { message },
                ..
            } => Some(message.clone()),
            _ => None,
        })
        .expect("the launch ends");
    assert_eq!(
        message,
        "No saved session found with ID saved-1. Run `codex fork` without an ID to choose from existing sessions."
    );
    assert!(launch_request(&fx).is_none() && starts(&fx) == 0);
}

#[test]
fn a_refused_resume_fails_the_start() {
    let mut app = named(Action::Resume);
    read_session(&mut app, "/tmp/project");
    let (_, request) = session_launch(&mut app, session_facts(Some(ResumeMode::Session)));
    let fx = launched(&mut app, request);
    let (ticket, _) = thread_asks(&fx).remove(0);
    run(
        &mut app,
        AppEv::Reply(ticket, Reply::failed("thread/resume: broken")),
    );
    assert!(matches!(app.session().status, Session::Failed(_)));
}

// A read lost with its connection is asked again on the next one.
#[test]
fn a_read_lost_with_the_connection_is_asked_again() {
    let mut app = named(Action::Resume);
    let fx = run(&mut app, AppEv::Connected);
    assert_eq!(thread_asks(&fx).len(), 1);
    run(
        &mut app,
        AppEv::Screen(ScreenEv::Agents(agents::Ev::Reconnecting)),
    );
    let fx = run(
        &mut app,
        AppEv::Screen(ScreenEv::Agents(agents::Ev::Reconnected)),
    );
    let asks = thread_asks(&fx);
    assert!(
        matches!(asks.first(), Some((_, ThreadAsk::Read { thread })) if thread == "saved-1"),
        "{asks:?}"
    );
}
