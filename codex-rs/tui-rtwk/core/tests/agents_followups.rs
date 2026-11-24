//! The follow-ups of the command center's first landing, the pure half:
//! the per-thread colour slot (oracle: `thread_color.rs`
//! `color_for_id`, `thread_color_tests.rs`), the colour switch
//! (`tui.status_line_use_colors`), and the account facts the usage fetch
//! needs (oracle: `app_server_session/account.rs`, `AccountUpdated` in
//! `app_server_events.rs`).

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::collections::HashSet;

use codex_app_server_protocol::AccountUpdatedNotification;
use codex_app_server_protocol::AuthMode;
use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::ServerNotification;
use codex_tui_core::app::{AppEv, AppFx, BackendEv, CodexApp, Command, Intent, apply_app};
use codex_tui_core::rpc::ServerError;
use codex_tui_core::screens::ScreenEv;
use codex_tui_core::screens::agents::{Ev, account_ev, account_request};
use codex_tui_core::thread_color::slot;
use pretty_assertions::assert_eq;
use serde_json::json;

/// The id the oracle's tests build with `ThreadId::from_u128`: a UUID text.
fn id(value: u128) -> String {
    let hex = format!("{value:032x}");
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

/// `assignments_reach_all_fourteen_default_accents` (thread_color_tests.rs:8):
/// over 512 ids every one of the fourteen slots is used, and none beyond.
#[test]
fn assignments_reach_all_fourteen_default_accents() {
    let slots: HashSet<usize> = (0..512)
        .map(|n| slot(&id(n), 14).expect("a slot"))
        .collect();
    assert_eq!(slots, (0..14).collect::<HashSet<_>>());
}

/// The slots the oracle's FNV-1a assigns (computed once with the oracle's
/// `color_for_id` over the default accents; id 42 is the snapshot's
/// `Rgb(242, 205, 205)`, the tenth of the fourteen).
#[test]
fn the_slot_is_the_oracles_fnv_1a_of_the_full_id() {
    let expected = [
        (0, 7),
        (1, 4),
        (2, 1),
        (3, 12),
        (5, 2),
        (42, 10),
        (43, 5),
        (255, 7),
        (511, 10),
    ];
    for (value, want) in expected {
        assert_eq!(slot(&id(value), 14), Some(want), "id {value}");
    }
}

/// It depends on the id and the count only: the same id, the same slot; the
/// slot is below the count; no colours, no slot (oracle: `Color::Reset`).
#[test]
fn the_slot_depends_on_the_id_and_the_count_only() {
    assert_eq!(slot(&id(42), 14), slot(&id(42), 14));
    for n in 1..20 {
        assert!(slot(&id(7), n).expect("a slot") < n);
    }
    assert_eq!(slot(&id(1), 0), None);
}

fn run(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    fx
}

/// Thread colours are on until the setting says otherwise (the config's
/// `status_line_use_colors` defaults to true).
#[test]
fn thread_colours_are_on_until_the_setting_turns_them_off() {
    let mut app = CodexApp::new();
    assert!(app.screens().agents.thread_colors());
    run(
        &mut app,
        AppEv::Screen(ScreenEv::Agents(Ev::ThreadColors(false))),
    );
    assert!(!app.screens().agents.thread_colors());
    run(
        &mut app,
        AppEv::Screen(ScreenEv::Agents(Ev::ThreadColors(true))),
    );
    assert!(app.screens().agents.thread_colors());
}

/// A script seeds the switch.
#[test]
fn the_switch_is_seeded_by_a_script() {
    let ev =
        codex_tui_core::screens::agents::seed(json!({"thread_colors": false})).expect("a seed");
    assert_eq!(ev, Ev::ThreadColors(false));
}

/// The bootstrap's `account/read` (`refresh_token: false`).
#[test]
fn the_account_request_does_not_refresh_the_token() {
    let request = account_request(RequestId::Integer(3));
    let json = serde_json::to_value(&request).unwrap();
    assert_eq!(json["method"], "account/read");
    // `false` is not serialized (the protocol skips it).
    assert!(json["params"].get("refreshToken").is_none(), "{json}");
    assert!(matches!(&request, ClientRequest::GetAccount { params, .. } if !params.refresh_token));
}

/// `SessionAccount::from`: a ChatGPT account has a plan and Codex backend
/// auth (`init.has_codex_backend_auth = has_chatgpt_account`); an API key or
/// none has neither.
#[test]
fn the_account_reply_gives_the_plan_and_the_backend_auth() {
    let chatgpt = account_ev(json!({
        "account": {"type": "chatgpt", "email": "a@b.c", "planType": "business"},
        "requiresOpenaiAuth": true
    }))
    .expect("an account");
    assert_eq!(
        chatgpt,
        Ev::Account {
            plan: Some(serde_json::from_value(json!("business")).unwrap()),
            backend_auth: true
        }
    );
    for body in [
        json!({"account": {"type": "apiKey"}, "requiresOpenaiAuth": false}),
        json!({"account": null, "requiresOpenaiAuth": true}),
    ] {
        assert_eq!(
            account_ev(body).expect("an account"),
            Ev::Account {
                plan: None,
                backend_auth: false
            }
        );
    }
    assert!(account_ev(json!("nonsense")).is_err());
}

/// `AccountUpdated` (app_server_events.rs:403): the usage the old account
/// had is dropped, and the plan and Codex backend auth are the new ones
/// (`has_codex_backend_auth` for ChatGPT-like auth modes only).
#[test]
fn an_account_update_changes_the_plan_and_drops_the_usage() {
    use codex_tui_core::fixtures::script::build;
    use codex_tui_core::rpc::Reply;
    use codex_tui_core::screens::agents::{self, UsageOutcome};
    use codex_tui_core::screens::{ScreenIntent, ScreenReply};

    let mut app = build(include_str!(
        "../../src/pages/agents/fixtures/overview.jsonl"
    ))
    .expect("plays")
    .app;
    run(
        &mut app,
        AppEv::Screen(ScreenEv::Agents(Ev::Account {
            plan: Some(serde_json::from_value(json!("business")).unwrap()),
            backend_auth: true,
        })),
    );
    let thread = app.screens().agents.center(&app.agents_threads()).rows[0]
        .id()
        .to_owned();
    let fx = run(
        &mut app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Agents(
            agents::Intent::NeedUsage(Some(thread)),
        ))),
    );
    let [Command::Screen { ticket, .. }] = fx.commands() else {
        panic!("expected a usage read");
    };
    run(
        &mut app,
        AppEv::Reply(
            *ticket,
            Reply::screen(ScreenReply::Agents(agents::Reply::Usage(
                UsageOutcome::Disabled,
            ))),
        ),
    );
    assert!(app.screens().agents.usage_disabled);
    let update = |mode, plan: Option<&str>| {
        AppEv::Backend(BackendEv::Notification(Box::new(
            ServerNotification::AccountUpdated(AccountUpdatedNotification {
                auth_mode: mode,
                plan_type: plan.map(|p| serde_json::from_value(json!(p)).unwrap()),
            }),
        )))
    };
    run(&mut app, update(Some(AuthMode::Chatgpt), Some("pro")));
    let agents = &app.screens().agents;
    assert!(agents.backend_auth);
    assert_eq!(
        agents.plan,
        Some(serde_json::from_value(json!("pro")).unwrap())
    );
    assert!(!agents.usage_disabled, "usage is asked again");
    run(&mut app, update(Some(AuthMode::ApiKey), None));
    assert!(!app.screens().agents.backend_auth);
    assert_eq!(app.screens().agents.plan, None);
}

/// A fork the server refuses is the failure of the thread it copied: the signal is per thread
/// (ADR-013 O4), so the page that waits on thread A does not stop waiting for a failed fork of B.
#[test]
fn a_failed_fork_of_one_thread_leaves_the_forking_of_another_alone() {
    use codex_tui_core::fixtures::script::build;
    use codex_tui_core::ids::ThreadId;
    use codex_tui_core::rpc::Reply;
    use codex_tui_core::screens::{ScreenIntent, agents};

    let mut app = build(include_str!(
        "../../src/pages/agents/fixtures/overview.jsonl"
    ))
    .expect("plays")
    .app;
    let rows = app.screens().agents.center(&app.agents_threads()).rows;
    let a = rows[0].id().to_owned();
    let b = rows[1].id().to_owned();
    assert!(a != b);
    let fork = |app: &mut CodexApp, thread: &str| {
        let fx = run(
            app,
            AppEv::Intent(Intent::Screen(ScreenIntent::Agents(agents::Intent::Fork(
                thread.to_owned(),
            )))),
        );
        let [Command::Screen { ticket, .. }] = fx.commands() else {
            panic!("expected a fork of {thread}");
        };
        *ticket
    };
    let _of_a = fork(&mut app, &a);
    let of_b = fork(&mut app, &b);
    run(&mut app, AppEv::Reply(of_b, Reply::failed("refused")));
    let failed = &app.session().fork_failed;
    assert_eq!(failed.get(&ThreadId::new(&b)), Some(&1));
    assert_eq!(failed.get(&ThreadId::new(&a)), None, "A's fork still waits");
}

/// A reply that is a failure is an error, never an account.
#[test]
fn a_server_error_has_its_code_and_prints_its_message() {
    let error = ServerError {
        code: Some(-32602),
        data: None,
        message: "invalid params".to_owned(),
    };
    assert_eq!(error.to_string(), "invalid params");
    assert_eq!(ServerError::from("down").code, None);
}

/// VS1 review P1/P6: an event of another screen, or a notification the center
/// does not take, copies none of the center's model: the new `ScreenStates`
/// holds the same one, so a reader that selects it is not woken and the
/// provider's comparison is a pointer.
#[test]
fn another_screens_events_leave_the_centers_model_shared() {
    use codex_tui_core::screens::agents::Model;
    use codex_tui_core::screens::{Shared, usage};
    let mut app = CodexApp::new();
    let before = app.screens().clone();
    run(
        &mut app,
        AppEv::Screen(ScreenEv::Usage(usage::Ev::Invalidated { session: 0 })),
    );
    assert!(
        !std::rc::Rc::ptr_eq(&before, app.screens()),
        "the event changed the screens"
    );
    let same: fn(&Shared<Model>, &Shared<Model>) -> bool = Shared::ptr_eq;
    assert!(same(&before.agents, &app.screens().agents));
    // A notification no screen takes changes nothing at all.
    let before = app.screens().clone();
    run(
        &mut app,
        AppEv::Backend(BackendEv::Notification(Box::new(
            ServerNotification::DeprecationNotice(
                codex_app_server_protocol::DeprecationNoticeNotification {
                    summary: "old".into(),
                    details: None,
                },
            ),
        ))),
    );
    assert!(same(&before.agents, &app.screens().agents));
}
