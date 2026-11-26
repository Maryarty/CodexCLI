// test root: pages
//! The folder-trust prompt cancelled at launch opens the command center in place of the folder
//! (oracle `lib.rs`: `consent.should_exit` -> `SessionSelection::AgentsOverview`). Opening a task
//! from it leaves the chat with the focus on its composer, so that the keys typed next land in the
//! draft (the same shape as `agents_focus` and `resume_focus`: the chat takes the focus as it
//! mounts, after the center has popped).

use crate::common::{self, Harness};
use codex_tui_core::app::{AppEv, Command};
use codex_tui_core::ids::ThreadId;
use codex_tui_core::screens::ScreenEv;
use codex_tui_core::screens::onboarding::consent::{Checked, Target};
use codex_tui_core::screens::onboarding::{Ask, Ev, Facts, TrustAsk, TrustCancel, Welcome};
use codex_tui_rtwk::app::UiApp;

const ID: &str = "00000000-0000-0000-0000-000000000001";
const SAVED_CWD: &str = "/tmp/codex";

fn thread_json(id: &str, preview: &str, now: u64) -> serde_json::Value {
    serde_json::json!({
        "id": id, "environments": null, "extra": null, "sessionId": id,
        "forkedFromId": null, "attachedToThreadId": null, "parentThreadId": null,
        "preview": preview, "ephemeral": false, "section": null, "sectionEnteredAt": null,
        "projectId": null, "historyMode": "legacy", "modelProvider": "openai",
        "model": null, "reasoningEffort": null, "createdAt": now - 7200,
        "updatedAt": now - 3600, "recencyAt": now - 3600, "status": {"type": "idle"},
        "path": null, "cwd": SAVED_CWD, "cliVersion": "0.0.0", "originator": null,
        "source": "cli", "canAcceptDirectInput": true, "threadSource": null,
        "agentNickname": null, "agentRole": null,
        "gitInfo": {"sha": null, "branch": "main", "originUrl": null},
        "name": "Selected task", "daybreakEnabled": null, "turns": []
    })
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

/// A server that lists one task and resumes it, the app started as the binary starts it (the home
/// waits for the host's facts) with `make` the command line's page, and the folder-trust prompt
/// on screen.
fn trust_prompt(
    make: impl FnOnce(codex_tui_rtwk::app::UiApp) -> codex_tui_rtwk::app::UiApp,
) -> Harness {
    let now: u64 = 1_777_399_200;
    let mut h = common::start_app(|app| make(app.with_awaited_facts(true)));
    h.fake
        .respond(move |request| match request["method"].as_str()? {
            "thread/list" => Some(Ok(if request["params"]["sectionId"].is_null() {
                serde_json::json!({
                    "data": [thread_json(ID, "Selected task", now)],
                    "nextCursor": null, "backwardsCursor": null,
                })
            } else {
                serde_json::json!({"data": [], "nextCursor": null, "backwardsCursor": null})
            })),
            "thread/resume" => Some(Ok(serde_json::json!({
                "thread": thread_json(ID, "Selected task", now),
                "model": "gpt-5", "modelProvider": "openai", "serviceTier": null,
                "cwd": SAVED_CWD, "instructionSources": [],
                "approvalPolicy": "on-request", "approvalsReviewer": "user",
                "sandbox": {"type": "dangerFullAccess"}, "reasoningEffort": null
            }))),
            _ => None,
        });
    h.inject([AppEv::Screen(ScreenEv::Onboarding(Ev::Facts {
        request: 1,
        facts: trust_facts(),
    }))]);
    h.wait_until("the trust prompt", |h| h.t.screen().contains("trust"));
    h.settle();
    h
}

/// The prompt cancelled: the page is left for the center at once (the oracle opens the overview
/// before its launch prompts are asked).
fn cancelled_into_the_center(h: &mut Harness) {
    h.press("esc");
    h.wait_until("the task", |h| h.t.screen().contains("Selected task"));
    h.settle();
    h.t.pump_for(common::ms(400));
}

/// What opening a task leaves: the center is gone and what is typed next is in the composer.
fn typed_into_the_composer(h: &mut Harness) {
    h.wait_until("the chat", |h| {
        h.t.screen().contains("Ask Codex to do anything")
    });
    h.settle();
    h.t.pump_for(common::ms(400));
    let screen = h.t.screen();
    assert!(
        !screen.contains("Agent command center"),
        "the center is gone: {screen}"
    );
    h.t.type_text("hi");
    h.t.pump_for(common::ms(100));
    let screen = h.t.screen();
    assert!(
        screen.contains("› hi"),
        "the composer shows what was typed: {screen}"
    );
}

#[test]
fn a_task_opened_from_the_center_the_cancelled_trust_prompt_opened_has_the_composer_focused() {
    let mut h = trust_prompt(|app| app);
    cancelled_into_the_center(&mut h);
    h.press("enter");
    typed_into_the_composer(&mut h);
}

#[test]
fn the_same_with_agents_on_the_command_line() {
    let mut h = trust_prompt(|app| app.with_start_agents(true));
    cancelled_into_the_center(&mut h);
    h.press("enter");
    typed_into_the_composer(&mut h);
}

#[test]
fn the_same_with_resume_on_the_command_line() {
    // the center replaces the resume picker: no picker is left to mount once it pops
    let mut h = trust_prompt(|app| app.with_start_resume(true));
    cancelled_into_the_center(&mut h);
    h.press("enter");
    typed_into_the_composer(&mut h);
}

fn startup_asks(h: &Harness) -> Vec<Ask> {
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

fn task_checks(h: &Harness) -> Vec<(u64, ThreadId, TrustCancel)> {
    startup_asks(h)
        .into_iter()
        .filter_map(|ask| match ask {
            Ask::CheckConsent {
                request,
                target: Target::Task(thread),
                cancel,
            } => Some((request, thread, cancel)),
            _ => None,
        })
        .collect()
}

fn trust_writes(h: &Harness) -> Vec<(u64, String)> {
    startup_asks(h)
        .into_iter()
        .filter_map(|ask| match ask {
            Ask::WriteTrust { request, target } => Some((request, target)),
            _ => None,
        })
        .collect()
}

fn resumes(h: &Harness) -> Vec<serde_json::Value> {
    h.fake
        .requests()
        .into_iter()
        .filter(|request| request["method"] == "thread/resume")
        .collect()
}

fn center_after_launch_trust() -> Harness {
    let mut h = trust_prompt(|app| app.with_folder_consent(true));
    h.wait_for("the launch folder's trust prompt", |screen| {
        screen.contains("Folder access") && screen.contains("/workspace/project")
    });
    cancelled_into_the_center(&mut h);
    h
}

fn ask_about_saved_task(h: &mut Harness, attempt: usize, restricted: bool) {
    h.press("enter");
    h.wait_until("the saved task's folder check", |h| {
        task_checks(h).len() >= attempt
    });
    let checks = task_checks(h);
    assert_eq!(checks.len(), attempt);
    let (request, thread, cancel) = &checks[attempt - 1];
    assert_eq!(thread.as_str(), ID);
    assert_eq!(*cancel, TrustCancel::AgentsOverview);
    assert!(resumes(h).is_empty(), "the task waits for its folder check");

    // The local-daemon host returns the saved task's checked cwd with its question.
    h.inject([AppEv::Screen(ScreenEv::Onboarding(Ev::ConsentAt {
        request: *request,
        ask: Some(TrustAsk {
            cwd: SAVED_CWD.into(),
            target: SAVED_CWD.into(),
            restricted,
            existing_task: true,
            cancel: *cancel,
            ..TrustAsk::default()
        }),
        checked: Checked::Task(SAVED_CWD.into()),
    }))]);
    let affirmative = if restricted {
        "1. Open existing task"
    } else {
        "1. Trust and continue"
    };
    h.wait_for("the existing task's trust prompt", |screen| {
        screen.contains("Folder access")
            && screen.contains(SAVED_CWD)
            && screen.contains(affirmative)
            && screen.contains("2. Back to Agent Command Center")
    });
    h.settle();
    assert!(resumes(h).is_empty(), "the task waits for the trust choice");
}

fn cancel_saved_task(h: &mut Harness) {
    h.press("down");
    h.wait_for("the selected Back choice", |screen| {
        screen.contains("› 2. Back to Agent Command Center")
    });
    h.press("enter");
    h.wait_for("the saved task back in the command center", |screen| {
        screen.contains("Agent command center")
            && screen.contains("Selected task")
            && !screen.contains("Folder access")
    });
    h.settle();
    assert!(trust_writes(h).is_empty());
    assert!(resumes(h).is_empty(), "Back must not resume the task");
}

#[test]
fn a_restricted_existing_task_can_go_back_to_the_center_after_launch_cancel() {
    let mut h = center_after_launch_trust();
    ask_about_saved_task(&mut h, 1, true);
    cancel_saved_task(&mut h);
}

#[test]
fn accepting_saved_task_trust_after_cancel_writes_once_then_resumes_with_composer_focus() {
    let mut h = center_after_launch_trust();
    ask_about_saved_task(&mut h, 1, false);
    cancel_saved_task(&mut h);
    ask_about_saved_task(&mut h, 2, false);
    h.press("enter");
    h.wait_until("the saved folder's trust write", |h| {
        !trust_writes(h).is_empty()
    });
    h.press("enter");
    h.settle();
    let writes = trust_writes(&h);
    let [(write, folder)] = writes.as_slice() else {
        panic!("expected one trust write, got {writes:?}");
    };
    assert_eq!(folder, SAVED_CWD);
    assert!(resumes(&h).is_empty(), "the task waits for the trust write");
    assert!(!startup_asks(&h).iter().any(|ask| matches!(
        ask,
        Ask::CheckConsent {
            target: Target::TaskRecheck { .. },
            ..
        }
    )));
    h.inject([AppEv::Screen(ScreenEv::Onboarding(Ev::TrustSaved {
        request: *write,
    }))]);

    h.wait_until("the saved task's daemon folder recheck", |h| {
        startup_asks(h).iter().any(|ask| {
            matches!(
                ask,
                Ask::CheckConsent {
                    target: Target::TaskRecheck { .. },
                    ..
                }
            )
        })
    });
    let checks: Vec<_> = startup_asks(&h)
        .into_iter()
        .filter_map(|ask| match ask {
            Ask::CheckConsent {
                request,
                target: Target::TaskRecheck { thread, checked },
                cancel,
            } => Some((request, thread, checked, cancel)),
            _ => None,
        })
        .collect();
    let [(request, thread, checked, cancel)] = checks.as_slice() else {
        panic!("expected one folder recheck, got {checks:?}");
    };
    assert_eq!(thread.as_str(), ID);
    assert_eq!(checked, &[SAVED_CWD]);
    assert_eq!(*cancel, TrustCancel::AgentsOverview);
    assert!(
        resumes(&h).is_empty(),
        "the task waits for the daemon recheck"
    );
    h.inject([AppEv::Screen(ScreenEv::Onboarding(Ev::ConsentAt {
        request: *request,
        ask: None,
        checked: Checked::ResumeIn(SAVED_CWD.into()),
    }))]);

    h.wait_until("the saved task active in the chat", |h| {
        h.t.read_app(|app: &UiApp| {
            app.core.session().active.as_ref().map(ThreadId::as_str) == Some(ID)
        })
    });
    typed_into_the_composer(&mut h);
    let asked = resumes(&h);
    let [resume] = asked.as_slice() else {
        panic!("expected one resume, got {asked:?}");
    };
    assert_eq!(resume["params"]["threadId"], ID);
    assert_eq!(resume["params"]["cwd"], SAVED_CWD);
    assert_eq!(trust_writes(&h), writes);
}
