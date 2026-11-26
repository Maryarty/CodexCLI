//! The folder trust asked over the chat (`/cd` into a folder whose trust is undecided): the
//! question comes before the new thread starts, trusting writes the key and then starts the
//! session there, keeping the current directory starts nothing. Oracle:
//! `app/working_directory.rs:262-292`, `onboarding/directory_trust.rs`.

use codex_app_server_protocol::ServerNotification;
use codex_tui_core::app::{
    AppEv, AppFx, BackendEv, CodexApp, Command, FirstThread, Intent, apply_app,
};
use codex_tui_core::commands::{SessionIntent, SlashCommand};
use codex_tui_core::ids::{ThreadId, Ticket};
use codex_tui_core::rpc::Reply;
use codex_tui_core::screens::onboarding::consent::{Checked, Target};
use codex_tui_core::screens::onboarding::{
    Ask, Ev, Intent as StartupIntent, TrustAsk, TrustCancel, TrustDecision, consent,
};
use codex_tui_core::screens::{ScreenCommand, ScreenEv, ScreenIntent, agents, chat};
use codex_tui_core::threads::{CdRefusal, Item, Note, NoteKey, ThreadAsk, Tone};
use pretty_assertions::assert_eq;

const T: &str = "thread-1";

fn run(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    fx
}

/// The app with its thread started, the host answering the folder trust.
fn ready() -> CodexApp {
    let mut app = CodexApp::new().with_folder_consent(true);
    let fx = run(&mut app, AppEv::Connected);
    let ticket: Ticket = fx
        .commands()
        .iter()
        .find_map(|c| match c {
            Command::StartThread { ticket, .. } => Some(*ticket),
            _ => None,
        })
        .expect("a StartThread");
    run(
        &mut app,
        AppEv::Reply(ticket, Reply::thread_started(ThreadId::new(T))),
    );
    app
}

/// `thread/started` of the thread, with a rollout path (history can be forked).
fn started(cwd: &str, path: Option<&str>) -> AppEv {
    let thread = saved_thread(T, cwd, path);
    let notification =
        ServerNotification::ThreadStarted(codex_app_server_protocol::ThreadStartedNotification {
            thread,
        });
    AppEv::Backend(BackendEv::Notification(Box::new(notification)))
}

fn saved_thread(id: &str, cwd: &str, path: Option<&str>) -> codex_app_server_protocol::Thread {
    serde_json::from_value(serde_json::json!({
            "id": id, "environments": null, "extra": null, "sessionId": id,
            "forkedFromId": null, "attachedToThreadId": null, "parentThreadId": null,
            "preview": "", "ephemeral": false, "section": null, "sectionEnteredAt": null,
            "projectId": null, "historyMode": "legacy", "modelProvider": "openai",
            "model": null, "reasoningEffort": null, "createdAt": 0, "updatedAt": 0,
            "recencyAt": 0, "status": {"type": "idle"}, "path": path, "cwd": cwd,
            "cliVersion": "0.0.0", "originator": null, "source": "cli",
            "canAcceptDirectInput": true, "threadSource": null,
            "agentNickname": null, "agentRole": null, "gitInfo": null, "name": null,
            "daybreakEnabled": null, "turns": []
    }))
    .expect("a thread")
}

fn notified(app: &mut CodexApp, notification: serde_json::Value) {
    let notification: ServerNotification =
        serde_json::from_value(notification).expect("notification");
    run(
        app,
        AppEv::Backend(BackendEv::Notification(Box::new(notification))),
    );
}

/// One whole turn: the agent said something and the turn ended.
fn a_turn_with_history(app: &mut CodexApp) {
    notified(
        app,
        serde_json::json!({"method": "turn/started", "params": {"threadId": T, "turn": {
            "id": "turn-1", "items": [], "itemsView": "full", "status": "inProgress",
            "error": null}}}),
    );
    notified(
        app,
        serde_json::json!({"method": "item/completed", "params": {"threadId": T,
            "turnId": "turn-1", "completedAtMs": 0, "startedAtMs": 0,
            "item": {"type": "agentMessage", "id": "a1", "text": "done", "phase": null}}}),
    );
    notified(
        app,
        serde_json::json!({"method": "turn/completed", "params": {"threadId": T, "turn": {
            "id": "turn-1", "items": [], "itemsView": "notLoaded", "status": "completed",
            "error": null}}}),
    );
}

fn cd(app: &mut CodexApp, path: &str) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Chat(chat::Intent::Session {
            thread: Some(ThreadId::new(T)),
            command: SlashCommand::Cd,
            intent: SessionIntent::Cd {
                path: path.to_string(),
            },
        }))),
    )
}

fn choose(app: &mut CodexApp, intent: StartupIntent) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Onboarding(intent))),
    )
}

fn answer(app: &mut CodexApp, ev: Ev) -> AppFx {
    run(app, AppEv::Screen(ScreenEv::Onboarding(ev)))
}

fn asks(fx: &AppFx) -> Vec<ThreadAsk> {
    fx.commands()
        .iter()
        .filter_map(|c| match c {
            Command::Screen {
                command: ScreenCommand::Agents(agents::Command::Thread(ask)),
                ..
            } => Some(ask.clone()),
            _ => None,
        })
        .collect()
}

/// The trust reads asked: the request number, the folder and the cancel choice.
fn checks(fx: &AppFx) -> Vec<(u64, Target, TrustCancel)> {
    fx.commands()
        .iter()
        .filter_map(|c| match c {
            Command::Effect {
                ask:
                    codex_tui_core::app::effects::Ask::Startup {
                        ask:
                            Ask::CheckConsent {
                                request,
                                target,
                                cancel,
                            },
                        ..
                    },
                ..
            } => Some((*request, target.clone(), *cancel)),
            _ => None,
        })
        .collect()
}

/// The trust writes asked: the request number and the folder to trust.
fn writes(fx: &AppFx) -> Vec<(u64, String)> {
    fx.commands()
        .iter()
        .filter_map(|c| match c {
            Command::Effect {
                ask:
                    codex_tui_core::app::effects::Ask::Startup {
                        ask: Ask::WriteTrust { request, target },
                        ..
                    },
                ..
            } => Some((*request, target.clone())),
            _ => None,
        })
        .collect()
}

/// The one trust write asked.
fn write_of(fx: &AppFx) -> (u64, String) {
    let mut all = writes(fx);
    assert_eq!(all.len(), 1, "one write: {:?}", fx.commands());
    all.remove(0)
}

/// The one trust read asked: the request number.
fn check_of(fx: &AppFx) -> u64 {
    let all = checks(fx);
    assert_eq!(all.len(), 1, "one check: {:?}", fx.commands());
    all[0].0
}

fn question(cwd: &str) -> TrustAsk {
    TrustAsk {
        cwd: cwd.into(),
        target: cwd.into(),
        cancel: TrustCancel::CurrentTask,
        ..TrustAsk::default()
    }
}

fn notes(app: &CodexApp) -> Vec<Note> {
    app.active_thread()
        .transcript()
        .iter()
        .filter_map(|(_, item)| match item {
            Item::Note(note) => Some(note.clone()),
            _ => None,
        })
        .collect()
}

/// `/cd /tmp` asked and answered with a question: the app waits on it.
fn asked(app: &mut CodexApp, path: &str) -> u64 {
    let fx = cd(app, path);
    let request = check_of(&fx);
    let fx = answer(
        app,
        Ev::Consent {
            request,
            ask: Some(question(path)),
        },
    );
    assert_eq!(asks(&fx), []);
    request
}

fn open(app: &CodexApp) -> Option<TrustAsk> {
    consent::open(&app.screens().onboarding).cloned()
}

#[test]
fn cd_asks_the_folder_trust_before_it_starts_the_thread() {
    let mut app = ready();
    let fx = cd(&mut app, "/tmp");
    assert_eq!(
        checks(&fx)
            .into_iter()
            .map(|(_, cwd, cancel)| (cwd, cancel))
            .collect::<Vec<_>>(),
        [(
            Target::Folder(Some("/tmp".to_string())),
            TrustCancel::CurrentTask
        )]
    );
    assert_eq!(asks(&fx), [], "nothing starts before the answer");
    assert_eq!(open(&app), None, "the question is read first");
}

#[test]
fn trusting_writes_the_key_and_then_starts_the_thread() {
    let mut app = ready();
    let request = asked(&mut app, "/tmp");
    assert_eq!(open(&app).map(|ask| ask.cwd), Some("/tmp".to_string()));
    let fx = choose(&mut app, StartupIntent::Trust);
    let (write, target) = write_of(&fx);
    assert_ne!(write, request);
    assert_eq!(target, "/tmp");
    assert_eq!(asks(&fx), [], "the thread waits for the write");
    assert_eq!(
        open(&app).and_then(|ask| ask.decision),
        Some(TrustDecision::Saving)
    );
    let fx = answer(&mut app, Ev::TrustSaved { request: write });
    assert_eq!(
        asks(&fx),
        [ThreadAsk::Start {
            cwd: Some("/tmp".to_string())
        }]
    );
    assert_eq!(open(&app), None);
    let fx = answer(&mut app, Ev::TrustSaved { request: write });
    assert_eq!(asks(&fx), [], "a late answer starts nothing more");
}

#[test]
fn trusting_a_conversation_with_history_forks_it_into_the_folder() {
    let mut app = ready();
    run(&mut app, started("/tmp/project", Some("/tmp/r.jsonl")));
    a_turn_with_history(&mut app);
    asked(&mut app, "/tmp");
    let fx = choose(&mut app, StartupIntent::Trust);
    let (write, _) = write_of(&fx);
    let fx = answer(&mut app, Ev::TrustSaved { request: write });
    assert_eq!(
        asks(&fx),
        [ThreadAsk::ForkAt {
            thread: T.to_string(),
            cwd: Some("/tmp".to_string()),
            temporary: None,
        }]
    );
}

#[test]
fn keeping_the_current_directory_starts_nothing() {
    let mut app = ready();
    asked(&mut app, "/tmp");
    let fx = choose(&mut app, StartupIntent::CancelTrust);
    assert_eq!(asks(&fx), []);
    assert_eq!(open(&app), None);
    assert_eq!(notes(&app), [], "the chat goes on as before");
    let fx = cd(&mut app, "/tmp");
    assert_eq!(checks(&fx).len(), 1, "the next /cd asks again");
}

#[test]
fn a_folder_that_needs_no_question_goes_straight_on() {
    let mut app = ready();
    let fx = cd(&mut app, "/tmp");
    let request = check_of(&fx);
    let fx = answer(&mut app, Ev::Consent { request, ask: None });
    assert_eq!(
        asks(&fx),
        [ThreadAsk::Start {
            cwd: Some("/tmp".to_string())
        }]
    );
    assert_eq!(open(&app), None);
}

#[test]
fn a_second_cd_while_the_question_is_open_asks_nothing() {
    let mut app = ready();
    let fx = cd(&mut app, "/tmp");
    let request = check_of(&fx);
    let fx = cd(&mut app, "/var");
    assert_eq!(
        (checks(&fx), asks(&fx)),
        (vec![], vec![]),
        "while it is read"
    );
    answer(
        &mut app,
        Ev::Consent {
            request,
            ask: Some(question("/tmp")),
        },
    );
    let fx = cd(&mut app, "/var");
    assert_eq!(
        (checks(&fx), asks(&fx)),
        (vec![], vec![]),
        "while it is open"
    );
    let fx = choose(&mut app, StartupIntent::Trust);
    let (write, _) = write_of(&fx);
    let fx = answer(&mut app, Ev::TrustSaved { request: write });
    assert_eq!(
        asks(&fx),
        [ThreadAsk::Start {
            cwd: Some("/tmp".to_string())
        }],
        "one thread start, for the first folder"
    );
}

#[test]
fn a_connection_lost_with_the_question_open_drops_the_cd_and_late_answers() {
    // Lost while the folder is read.
    let mut app = ready();
    let fx = cd(&mut app, "/tmp");
    let request = check_of(&fx);
    run(&mut app, AppEv::Disconnected);
    let fx = answer(&mut app, Ev::Consent { request, ask: None });
    assert_eq!(asks(&fx), [], "a late read starts nothing");
    assert_eq!(open(&app), None);

    // Lost while the question is on screen.
    let mut app = ready();
    let request = asked(&mut app, "/tmp");
    run(&mut app, AppEv::Disconnected);
    assert_eq!(open(&app), None, "the page leaves");
    let fx = answer(&mut app, Ev::Consent { request, ask: None });
    assert_eq!(asks(&fx), []);

    // Lost while the write is out.
    let mut app = ready();
    asked(&mut app, "/tmp");
    let fx = choose(&mut app, StartupIntent::Trust);
    let (write, _) = write_of(&fx);
    run(&mut app, AppEv::Disconnected);
    assert_eq!(open(&app), None);
    let fx = answer(&mut app, Ev::TrustSaved { request: write });
    assert_eq!(asks(&fx), [], "a late write starts nothing");
}

#[test]
fn a_failed_write_keeps_the_question_with_its_error() {
    let mut app = ready();
    asked(&mut app, "/tmp");
    let fx = choose(&mut app, StartupIntent::Trust);
    let (write, _) = write_of(&fx);
    let fx = answer(
        &mut app,
        Ev::TrustFailed {
            request: write,
            message: "disk full".into(),
        },
    );
    assert_eq!(asks(&fx), [], "the cd waits");
    let ask = open(&app).expect("the question stays");
    assert_eq!(ask.error.as_deref(), Some("disk full"));
    assert_eq!(ask.decision, None);
    let fx = choose(&mut app, StartupIntent::CancelTrust);
    assert_eq!(asks(&fx), []);
    assert_eq!(open(&app), None, "Esc drops it");
}

#[test]
fn a_failed_read_says_so_and_starts_nothing() {
    let mut app = ready();
    let fx = cd(&mut app, "/tmp");
    let request = check_of(&fx);
    let fx = answer(
        &mut app,
        Ev::ConsentFailed {
            request,
            message: "config/read failed".into(),
        },
    );
    assert_eq!(asks(&fx), []);
    assert_eq!(open(&app), None);
    let key = NoteKey::CdRefused(CdRefusal::TrustCheck("config/read failed".into()));
    assert_eq!(notes(&app), [Note::from(key)]);
    assert_eq!(notes(&app)[0].tone, Tone::Error);
}

#[test]
fn a_host_that_does_not_ask_starts_the_session_at_once() {
    let mut app = CodexApp::new();
    let fx = run(&mut app, AppEv::Connected);
    let ticket = fx
        .commands()
        .iter()
        .find_map(|c| match c {
            Command::StartThread { ticket, .. } => Some(*ticket),
            _ => None,
        })
        .expect("a StartThread");
    run(
        &mut app,
        AppEv::Reply(ticket, Reply::thread_started(ThreadId::new(T))),
    );
    let fx = cd(&mut app, "/tmp");
    assert_eq!(checks(&fx), []);
    assert_eq!(asks(&fx).len(), 1);
}

fn center(app: &mut CodexApp, intent: agents::Intent) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Agents(intent))),
    )
}

fn list_saved_task(app: &mut CodexApp) -> ThreadId {
    let id = "saved-task";
    run(
        app,
        AppEv::Screen(ScreenEv::Agents(agents::Ev::Listed(agents::Page {
            threads: vec![saved_thread(id, "/tmp/saved-task", None)],
            unread: Vec::new(),
            next_cursor: None,
            append: false,
            pins: None,
        }))),
    );
    ThreadId::new(id)
}

fn start_page() -> CodexApp {
    let mut app = CodexApp::new()
        .with_first_thread(FirstThread::Never)
        .with_folder_consent(true);
    run(&mut app, AppEv::Connected);
    app
}

#[test]
fn opening_a_saved_task_waits_for_the_folder_check() {
    let mut app = start_page();
    let id = list_saved_task(&mut app);
    let pick = AppEv::Intent(Intent::OpenThread(id.clone()));
    let fx = run(&mut app, pick);
    assert_eq!(asks(&fx), []);
    let request = check_of(&fx);
    assert_eq!(
        checks(&fx),
        [(
            request,
            Target::Task(id.clone()),
            TrustCancel::AgentsOverview
        )]
    );
    assert!(app.screens().agents.opening.is_none());

    let duplicate = run(&mut app, AppEv::Intent(Intent::OpenThread(id.clone())));
    assert_eq!(checks(&duplicate), []);
    let left = choose(&mut app, StartupIntent::StartPageLeft);
    assert!(
        left.commands().is_empty(),
        "leaving the picker must not start a second session"
    );
    let fx = answer(&mut app, Ev::Consent { request, ask: None });
    assert_eq!(
        asks(&fx),
        [ThreadAsk::Resume {
            thread: id.to_string()
        }]
    );
}

#[test]
fn a_cancelled_task_open_ignores_a_late_answer_and_can_be_retried() {
    let mut app = start_page();
    let id = list_saved_task(&mut app);
    let fx = run(&mut app, AppEv::Intent(Intent::OpenThread(id.clone())));
    let request = check_of(&fx);
    answer(
        &mut app,
        Ev::Consent {
            request,
            ask: Some(TrustAsk {
                existing_task: true,
                cancel: TrustCancel::AgentsOverview,
                ..question("/tmp/saved-task")
            }),
        },
    );
    assert_eq!(asks(&choose(&mut app, StartupIntent::CancelTrust)), []);
    assert_eq!(open(&app), None);
    let late = answer(&mut app, Ev::Consent { request, ask: None });
    assert_eq!(asks(&late), []);
    let retry = run(&mut app, AppEv::Intent(Intent::OpenThread(id)));
    assert_ne!(check_of(&retry), request);
}

fn daemon_question(app: &mut CodexApp, request: u64, cwd: &str) {
    let effects = answer(
        app,
        Ev::ConsentAt {
            request,
            ask: Some(TrustAsk {
                existing_task: true,
                cancel: TrustCancel::AgentsOverview,
                ..question(cwd)
            }),
            checked: Checked::Task(cwd.into()),
        },
    );
    assert_eq!(asks(&effects), []);
}

fn approve(app: &mut CodexApp, cwd: &str) -> AppFx {
    let write = choose(app, StartupIntent::Trust);
    let (request, target) = write_of(&write);
    assert_eq!(target, cwd);
    assert_eq!(asks(&write), []);
    answer(app, Ev::TrustSaved { request })
}

#[test]
fn daemon_task_rechecks_the_same_target_and_consents_to_each_changed_cwd() {
    let mut app = ready();
    let id = list_saved_task(&mut app);
    let first = check_of(&run(
        &mut app,
        AppEv::Intent(Intent::OpenThread(id.clone())),
    ));
    daemon_question(&mut app, first, "/work/a");
    let number = consent::open_number(&app.screens().onboarding);
    let effects = approve(&mut app, "/work/a");
    let second = check_of(&effects);
    assert_ne!(second, first);
    assert_eq!(asks(&effects), []);
    assert_eq!(
        checks(&effects),
        [(
            second,
            Target::TaskRecheck {
                thread: id.clone(),
                checked: vec!["/work/a".into()]
            },
            TrustCancel::AgentsOverview
        )]
    );
    assert!(
        effects
            .commands()
            .iter()
            .all(|command| app.backend_of(command) == Default::default())
    );
    assert_eq!(
        asks(&answer(
            &mut app,
            Ev::Consent {
                request: first,
                ask: None
            }
        )),
        []
    );

    daemon_question(&mut app, second, "/work/b");
    assert_eq!(consent::open_number(&app.screens().onboarding), number);
    let effects = approve(&mut app, "/work/b");
    let third = check_of(&effects);
    assert_eq!(
        checks(&effects),
        [(
            third,
            Target::TaskRecheck {
                thread: id.clone(),
                checked: vec!["/work/a".into(), "/work/b".into()]
            },
            TrustCancel::AgentsOverview
        )]
    );
    assert_eq!(asks(&effects), []);
    let verified = Ev::ConsentAt {
        request: third,
        ask: None,
        checked: Checked::ResumeIn("/work/b".into()),
    };
    let effects = answer(&mut app, verified);
    assert_eq!(
        asks(&effects),
        [ThreadAsk::ResumeAt {
            thread: id.to_string(),
            cwd: Some("/work/b".into())
        }]
    );
    assert_eq!(
        asks(&answer(
            &mut app,
            Ev::Consent {
                request: third,
                ask: None
            }
        )),
        []
    );

    let ticket = effects
        .commands()
        .iter()
        .find_map(|command| match command {
            Command::Screen {
                ticket,
                command: ScreenCommand::Agents(agents::Command::Thread(ThreadAsk::ResumeAt { .. })),
                ..
            } => Some(*ticket),
            _ => None,
        })
        .expect("the resume ticket");
    let active = app.session().active.clone();
    center(&mut app, agents::Intent::CancelOpen);
    run(
        &mut app,
        AppEv::Reply(
            ticket,
            Reply::screen(codex_tui_core::screens::ScreenReply::Agents(
                agents::Reply::Resumed(Box::new(saved_thread(id.as_str(), "/work/b", None))),
            )),
        ),
    );
    assert_eq!(
        app.session().active,
        active,
        "a cancelled resume leaves navigation alone"
    );
    assert!(
        app.thread(&id).is_some_and(|held| held.view().has_writer()),
        "the successful reply still attaches"
    );
}

#[test]
fn cancelling_a_task_while_its_cwd_is_read_drops_the_late_answer() {
    let mut app = ready();
    let id = list_saved_task(&mut app);
    let first = check_of(&run(
        &mut app,
        AppEv::Intent(Intent::OpenThread(id.clone())),
    ));
    daemon_question(&mut app, first, "/work/a");
    let reread = check_of(&approve(&mut app, "/work/a"));
    let effects = center(&mut app, agents::Intent::CancelOpen);
    assert_eq!(asks(&effects), []);
    assert!(!consent::pending(&app.screens().onboarding));
    assert_eq!(
        asks(&answer(
            &mut app,
            Ev::Consent {
                request: reread,
                ask: None
            }
        )),
        []
    );
    assert_ne!(
        check_of(&run(&mut app, AppEv::Intent(Intent::OpenThread(id)))),
        reread
    );
}

#[test]
fn a_new_center_task_waits_for_the_trust_write() {
    let mut app = ready();
    let id = list_saved_task(&mut app);
    let fx = center(&mut app, agents::Intent::New(Some(id.to_string())));
    let request = check_of(&fx);
    let cwd = "/tmp/saved-task";
    assert_eq!(
        checks(&fx),
        [(
            request,
            Target::Folder(Some(cwd.into())),
            TrustCancel::AgentsOverview
        )]
    );
    assert_eq!(asks(&fx), []);
    answer(
        &mut app,
        Ev::Consent {
            request,
            ask: Some(TrustAsk {
                cancel: TrustCancel::AgentsOverview,
                ..question(cwd)
            }),
        },
    );
    let fx = choose(&mut app, StartupIntent::Trust);
    let (write, target) = write_of(&fx);
    assert_eq!(target, cwd);
    assert_eq!(asks(&fx), []);
    let fx = answer(&mut app, Ev::TrustSaved { request: write });
    assert_eq!(
        asks(&fx),
        [ThreadAsk::Start {
            cwd: Some(cwd.into())
        }]
    );
}

#[test]
fn fresh_session_waits_for_the_launch_folder_and_cancel_returns_to_the_center() {
    let mut app = start_page();
    let fx = choose(&mut app, StartupIntent::StartPageLeft);
    let request = check_of(&fx);
    assert_eq!(
        checks(&fx),
        [(request, Target::Folder(None), TrustCancel::AgentsOverview)]
    );
    assert!(
        !fx.commands()
            .iter()
            .any(|c| matches!(c, Command::StartThread { .. }))
    );
    answer(
        &mut app,
        Ev::Consent {
            request,
            ask: Some(TrustAsk {
                cancel: TrustCancel::AgentsOverview,
                ..question("/tmp/launch")
            }),
        },
    );
    let fx = choose(&mut app, StartupIntent::CancelTrust);
    assert!(fx.commands().is_empty());
    assert!(consent::return_to_center(&app.screens().onboarding));

    let retry = check_of(&choose(&mut app, StartupIntent::StartPageLeft));
    assert!(!consent::return_to_center(&app.screens().onboarding));
    let fx = answer(
        &mut app,
        Ev::Consent {
            request: retry,
            ask: None,
        },
    );
    assert!(
        fx.commands()
            .iter()
            .any(|c| matches!(c, Command::StartThread { .. }))
    );
}

#[test]
fn a_failed_check_for_a_center_task_reports_the_error_without_starting_it() {
    let mut app = start_page();
    let fx = center(&mut app, agents::Intent::New(None));
    let request = check_of(&fx);
    let fx = answer(
        &mut app,
        Ev::ConsentFailed {
            request,
            message: "config/read failed".into(),
        },
    );
    assert_eq!(asks(&fx), []);
    assert_eq!(
        app.screens().agents.notice,
        Some(agents::Notice::TrustCheckFailed(
            "config/read failed".into()
        ))
    );
}
