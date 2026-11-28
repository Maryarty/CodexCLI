// test root: pages
//! The resume-time Daybreak message belongs to the session being opened.

use codex_app_server_protocol::AuthMode;
use codex_tui_core::app::facts::{DaybreakConfig, TuiSettings};
use codex_tui_core::app::{AppEv, BackendEv, Intent};
use codex_tui_core::ids::ThreadId;
use codex_tui_core::screens::onboarding::session::Start;
use codex_tui_core::screens::startup::Action;
use codex_tui_core::screens::{ScreenEv, agents, status};
use codex_tui_rtwk::app::UiApp;
use rtwk_test::Options;
use serde_json::{Value, json};

const NOTICE: &str = "Daybreak is on for new turns.";
const HISTORY: &str = "saved history shown after its opening context";
const ENABLED: DaybreakConfig = DaybreakConfig {
    enabled: true,
    api_key: false,
};

fn settings(daybreak: DaybreakConfig) -> AppEv {
    AppEv::Tui(TuiSettings {
        daybreak,
        ..TuiSettings::default()
    })
}

fn reply(id: &str, enabled: bool) -> Value {
    json!({
        "thread": {
            "id": id, "sessionId": id, "preview": id, "ephemeral": false,
            "historyMode": "legacy", "modelProvider": "openai",
            "createdAt": 1_700_000_000, "updatedAt": 1_700_000_100,
            "status": {"type": "idle"}, "cwd": "/work", "cliVersion": "0.0.0",
            "source": "cli", "canAcceptDirectInput": true, "name": id,
            "daybreakEnabled": enabled,
            "turns": [crate::kit::agent_turn("history", &[HISTORY])],
        },
        "model": "gpt-5", "modelProvider": "openai", "cwd": "/work", "instructionSources": [],
        "approvalPolicy": "on-request", "approvalsReviewer": "user",
        "sandbox": {"type": "dangerFullAccess"}, "reasoningEffort": null,
    })
}

fn assert_notice(screen: &str, enabled: bool) {
    assert!(screen.contains(HISTORY), "{screen}");
    assert_eq!(
        screen.matches(NOTICE).count(),
        usize::from(enabled),
        "{screen}"
    );
    if let Some(notice) = screen.find(NOTICE) {
        assert!(
            screen
                .find("OpenAI Codex")
                .is_some_and(|header| header < notice),
            "{screen}"
        );
        assert!(
            screen.find(HISTORY).is_some_and(|history| notice < history),
            "{screen}"
        );
    }
}

fn metadata(id: &str, enabled: bool) -> AppEv {
    AppEv::Backend(BackendEv::Notification(
        serde_json::from_value(json!({
            "method": "thread/started", "params": { "thread": reply(id, enabled)["thread"] },
        }))
        .unwrap_or_else(|error| panic!("invalid thread metadata fixture: {error}")),
    ))
}

fn account_updated(auth_mode: AuthMode) -> AppEv {
    AppEv::Backend(BackendEv::Notification(
        serde_json::from_value(json!({
            "method": "account/updated", "params": { "authMode": auth_mode },
        }))
        .unwrap_or_else(|error| panic!("invalid account update fixture: {error}")),
    ))
}

#[test]
fn launch_resume_uses_the_saved_daybreak_preference_and_effective_feature_flags() {
    let cases = [
        (true, DaybreakConfig::default(), false, false),
        (true, ENABLED, true, false),
        (true, ENABLED, false, true),
        (
            true,
            DaybreakConfig {
                api_key: true,
                ..ENABLED
            },
            true,
            true,
        ),
        (false, ENABLED, false, false),
    ];
    for (saved, config, api_key, show_notice) in cases {
        let id = "launch-resume";
        let mut h = crate::common::start_driven_as(false, Options::new().size(80, 24), |app| {
            app.with_awaited_facts(true).with_start_session(Start {
                action: Action::Resume,
                id: id.into(),
            })
        });
        h.fake.respond(move |request| {
            if request["params"]["threadId"] != id {
                return None;
            }
            match request["method"].as_str()? {
                "thread/read" => Some(Ok(json!({ "thread": reply(id, saved)["thread"] }))),
                "thread/resume" => Some(Ok(reply(id, saved))),
                _ => None,
            }
        });
        h.connect();
        h.inject([
            settings(config),
            AppEv::Screen(ScreenEv::Status(status::Ev::Account(
                api_key.then_some(status::Account::ApiKey),
            ))),
        ]);
        h.finish_startup_without_prompts();
        h.advance(800);
        assert!(h.t.screen().contains("Use session directory"));
        h.press("enter");
        h.advance(800);
        assert_notice(&h.t.screen(), show_notice);
    }
}

#[test]
fn opening_saved_threads_keeps_the_notice_with_the_original_resume() {
    let mut h = crate::common::start_driven(false, 80, 24);
    h.connect();
    h.inject([settings(ENABLED)]);
    h.fake.respond(|request| {
        if request["method"] != "thread/resume" {
            return None;
        }
        let id = request["params"]["threadId"].as_str()?;
        Some(Ok(reply(id, id != "daybreak-off")))
    });
    let listed = ["daybreak-on", "daybreak-off", "blocked-api-key", "chatgpt"]
        .into_iter()
        .map(|id| {
            serde_json::from_value(reply(id, id != "daybreak-off")["thread"].clone())
                .unwrap_or_else(|error| panic!("invalid listed thread fixture: {error}"))
        })
        .collect();
    h.inject([AppEv::Screen(ScreenEv::Agents(agents::Ev::Listed(
        agents::Page {
            threads: listed,
            unread: Vec::new(),
            next_cursor: None,
            append: false,
            pins: None,
        },
    )))]);
    for (id, account, enabled) in [
        ("daybreak-on", None, true),
        ("blocked-api-key", Some(AuthMode::ApiKey), false),
        ("chatgpt", Some(AuthMode::Chatgpt), true),
        ("daybreak-off", None, false),
        ("blocked-api-key", None, false),
        ("daybreak-on", None, true),
    ] {
        if let Some(account) = account {
            let blocked = account == AuthMode::ApiKey;
            h.inject([account_updated(account)]);
            h.advance(800);
            // The notice records what was true when its session opened.
            if blocked {
                assert_notice(&h.t.screen(), true);
            }
        }
        h.inject([AppEv::Intent(Intent::OpenThread(ThreadId::new(id)))]);
        h.advance(800);
        assert_notice(&h.t.screen(), enabled);
    }
    // Refreshing current metadata can turn the option off without removing
    // the first resume's historical context or making a second notice.
    h.inject([
        metadata("daybreak-on", false),
        metadata("daybreak-on", false),
    ]);
    h.advance(800);
    assert_eq!(
        h.t.read_app(|app: &UiApp| {
            codex_tui_core::features::daybreak_of(app.core.active_thread().view())
        }),
        Some(false),
    );
    assert_notice(&h.t.screen(), true);
}

#[test]
fn a_new_thread_with_daybreak_does_not_show_the_resume_notice() {
    let mut h = crate::common::start_driven(false, 80, 24);
    h.fake.respond(|request| {
        (request["method"] == "thread/start").then(|| Ok(reply("fresh-thread", true)))
    });
    h.connect();
    h.inject([settings(ENABLED)]);
    h.advance(800);
    assert_eq!(
        h.t.read_app(|app: &UiApp| {
            codex_tui_core::features::daybreak_of(app.core.active_thread().view())
        }),
        Some(true),
    );
    assert!(!h.t.screen().contains(NOTICE), "{}", h.t.screen());
}
