#![allow(clippy::expect_used, clippy::unwrap_used)]

//! The automatic model switch of a usage banner (oracle `backend_banner_fallback.rs`,
//! `backend_banners.rs`): an account read that blocks the model in use moves the thread to the
//! banner's first usable fallback (asking the catalog first when the app has none), Luna Reserve
//! is entered and left the same way, and each answer settles the note and the banner's notice.

use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, apply_app};
use codex_tui_core::ids::{BackendId, ThreadId, Ticket};
use codex_tui_core::protocol;
use codex_tui_core::rpc::Reply;
use codex_tui_core::screens::agents::Ev as AgentsEv;
use codex_tui_core::screens::chat::{AutomaticSwitch, SwitchReason};
use codex_tui_core::screens::select::{self, Ev as SelectEv};
use codex_tui_core::screens::{ScreenCommand, ScreenEv, ScreenReply, chat};
use codex_tui_core::threads::{Item, ModeNote, NoteKey};
use pretty_assertions::assert_eq;
use serde_json::json;

const T: &str = "thread-1";

fn run(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    app.assert_consistent();
    fx
}

/// The app with its thread started and a ChatGPT login read.
fn ready() -> CodexApp {
    let mut app = CodexApp::new();
    let fx = run(&mut app, AppEv::Connected);
    let ticket = fx
        .commands()
        .iter()
        .find_map(|command| match command {
            Command::StartThread { ticket, .. } => Some(*ticket),
            _ => None,
        })
        .expect("the thread starts on connect");
    run(
        &mut app,
        AppEv::Reply(ticket, Reply::thread_started(ThreadId::new(T))),
    );
    run(
        &mut app,
        AppEv::Screen(ScreenEv::Agents(AgentsEv::Account {
            plan: None,
            backend_auth: true,
        })),
    );
    app
}

/// The config's model and effort in use; with `usage` the account read follows (the status
/// facts' `account_usage`, `requires_openai_auth`).
fn model_facts(app: &mut CodexApp, model: &str, effort: Option<&str>, usage: bool) -> AppFx {
    run(
        app,
        AppEv::Screen(ScreenEv::Chat(chat::Ev::Status(Box::new(
            chat::StatusFacts {
                model: Some(model.to_owned()),
                effort: effort.map(str::to_owned),
                account_usage: usage,
                ..chat::StatusFacts::default()
            },
        )))),
    )
}

fn preset(model: &str, visible: bool) -> serde_json::Value {
    json!({
        "model": model, "name": model, "show_in_picker": visible,
        "default_effort": "medium",
        "efforts": [{"effort": "low"}, {"effort": "medium"}],
    })
}

fn catalog(app: &mut CodexApp, models: &[(&str, bool)]) {
    let presets = models
        .iter()
        .map(|(model, visible)| preset(model, *visible))
        .collect::<Vec<_>>();
    let presets = serde_json::from_value(json!(presets)).expect("presets");
    run(
        app,
        AppEv::Screen(ScreenEv::Select(SelectEv::Loaded(presets))),
    );
}

fn blocked_banner() -> serde_json::Value {
    json!({
        "banner_type": "selected_model_limit", "model_slug": "test-model-a",
        "blocked_model_slug": "test-model-a",
        "fallback_model_slugs": ["hidden-model", "test-model-c", "test-model-b"],
        "title": "Selected model usage exhausted",
        "description": "Ask your owner for credits or switch to another model.",
        "ctas": [], "presentation": "inline",
    })
}

fn reserve_banner() -> serde_json::Value {
    json!({
        "banner_type": "luna_reserve", "title": "Usage limit reached",
        "description": "Continue with Luna Reserve.", "ctas": [], "presentation": "inline",
    })
}

/// The account read in `fx` answered with `response`.
fn read_answered(app: &mut CodexApp, fx: &AppFx, response: serde_json::Value) -> AppFx {
    let read = fx
        .commands()
        .iter()
        .find(|command| matches!(command, Command::ReadRateLimits { .. }))
        .unwrap_or_else(|| panic!("an account read in {:?}", fx.commands()));
    let (ticket, reply) = protocol::reply(read, Ok(response)).expect("a reply");
    run(app, AppEv::Reply(ticket, reply))
}

fn banner_read(banner: serde_json::Value) -> serde_json::Value {
    json!({"accountId": "workspace-a", "rateLimits": {}, "rateLimitUpsell": banner})
}

/// The one `model/list` in `fx`: the ticket to answer it.
fn models_ask(fx: &AppFx) -> Ticket {
    let [
        Command::Screen {
            ticket,
            command: ScreenCommand::Select(select::Command::Models { .. }),
            ..
        },
    ] = fx.commands()
    else {
        panic!("expected one model/list, got {:?}", fx.commands());
    };
    *ticket
}

/// The one thread settings update in `fx`: its ticket, thread and patch.
fn settings_ask(fx: &AppFx) -> (Ticket, String, codex_tui_core::threads::SettingsPatch) {
    let [
        Command::Screen {
            ticket,
            command: ScreenCommand::Select(select::Command::Settings { thread, patch }),
            ..
        },
    ] = fx.commands()
    else {
        panic!("expected one settings update, got {:?}", fx.commands());
    };
    (*ticket, thread.clone(), patch.clone())
}

fn accepted(app: &mut CodexApp, ticket: Ticket) -> AppFx {
    run(
        app,
        AppEv::Reply(
            ticket,
            Reply::screen(ScreenReply::Select(select::Reply::Updated)),
        ),
    )
}

fn switches(app: &CodexApp) -> Vec<AutomaticSwitch> {
    app.active_thread()
        .transcript()
        .iter()
        .filter_map(|(_, item)| match item {
            Item::Note(note) => match &note.key {
                NoteKey::Mode(ModeNote::AutomaticSwitch(switch)) => Some(switch.clone()),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

fn backend(app: &CodexApp) -> BackendId {
    *app.session()
        .backend_banners
        .keys()
        .next()
        .expect("a banner")
}

/// Oracle `backend_banner_fallback_candidates_and_notice_follow_selected_model`: the catalog is
/// asked when the app has none; the first fallback the picker lists is chosen (the hidden one
/// is skipped, the order of the banner decides between the others) with the effort of the
/// thread when the model supports it; the accepted update says so, and the banner stays for
/// the replacement.
#[test]
fn a_blocked_model_banner_switches_to_its_first_listed_fallback() {
    let mut app = ready();
    let fx = model_facts(&mut app, "test-model-a", Some("low"), true);
    let fx = read_answered(&mut app, &fx, banner_read(blocked_banner()));
    let list = models_ask(&fx);
    let fx = run(
        &mut app,
        AppEv::Reply(
            list,
            Reply::screen(ScreenReply::Select(select::Reply::Models(
                serde_json::from_value(json!([
                    preset("hidden-model", false),
                    preset("test-model-b", true),
                    preset("test-model-c", true),
                ]))
                .expect("presets"),
            ))),
        ),
    );
    let (ticket, thread, patch) = settings_ask(&fx);
    assert_eq!(thread, T);
    assert_eq!(patch.model.as_deref(), Some("test-model-c"));
    assert_eq!(
        patch.effort.as_ref().map(|effort| effort.as_str()),
        Some("low")
    );
    // nothing is said before the server accepts
    assert!(switches(&app).is_empty());
    accepted(&mut app, ticket);
    assert_eq!(
        switches(&app),
        [AutomaticSwitch {
            model: "test-model-c".into(),
            effort: Some("low".into()),
            reason: SwitchReason::UsageLimit,
        }]
    );
    // the server's `thread/settings/updated` brings the model; the banner follows it
    model_facts(&mut app, "test-model-c", Some("low"), true);
    let occurrence = &app.session().backend_banners[&backend(&app)];
    assert!(occurrence.is_visible(Some("test-model-c")));
    assert!(!occurrence.is_visible(Some("test-model-a")));
    assert!(!occurrence.is_visible(Some("unrelated-model")));
}

/// An effort the replacement does not support gives way to its own default.
#[test]
fn an_unsupported_effort_falls_back_to_the_models_default() {
    let mut app = ready();
    catalog(&mut app, &[("test-model-c", true)]);
    let fx = model_facts(&mut app, "test-model-a", Some("high"), true);
    let fx = read_answered(&mut app, &fx, banner_read(blocked_banner()));
    let (_, _, patch) = settings_ask(&fx);
    assert_eq!(
        patch.effort.as_ref().map(|effort| effort.as_str()),
        Some("medium")
    );
}

/// A retained ChatGPT login must not change a task whose server does not require auth, and a
/// thread not on the blocked model has nothing to switch.
#[test]
fn nothing_switches_without_the_account_the_banner_is_about() {
    let mut app = ready();
    catalog(&mut app, &[("test-model-c", true)]);
    // no `requires_openai_auth` (the facts carry no account usage): no read, no switch
    let fx = model_facts(&mut app, "test-model-a", None, false);
    assert!(fx.commands().is_empty());

    let mut app = ready();
    catalog(&mut app, &[("test-model-c", true)]);
    let fx = model_facts(&mut app, "test-model-b", None, true);
    let fx = read_answered(&mut app, &fx, banner_read(blocked_banner()));
    assert!(fx.commands().is_empty(), "{:?}", fx.commands());

    // an account that is not a ChatGPT login
    let mut app = ready();
    run(
        &mut app,
        AppEv::Screen(ScreenEv::Agents(AgentsEv::Account {
            plan: None,
            backend_auth: false,
        })),
    );
    catalog(&mut app, &[("test-model-c", true)]);
    let fx = model_facts(&mut app, "test-model-a", None, true);
    let fx = read_answered(&mut app, &fx, banner_read(blocked_banner()));
    assert!(fx.commands().is_empty(), "{:?}", fx.commands());
}

/// A candidate the picker hides or the catalog lacks is not a fallback: with none left the
/// thread stays where it is.
#[test]
fn a_banner_without_a_usable_fallback_changes_nothing() {
    let mut app = ready();
    catalog(&mut app, &[("hidden-model", false), ("other-model", true)]);
    let fx = model_facts(&mut app, "test-model-a", None, true);
    let fx = read_answered(&mut app, &fx, banner_read(blocked_banner()));
    assert!(fx.commands().is_empty(), "{:?}", fx.commands());
}

/// Older servers can decline the update: no note, and the existing recovery actions stay.
#[test]
fn a_declined_update_leaves_the_banner_and_says_nothing() {
    let mut app = ready();
    catalog(&mut app, &[("test-model-c", true)]);
    let fx = model_facts(&mut app, "test-model-a", None, true);
    let fx = read_answered(&mut app, &fx, banner_read(blocked_banner()));
    let (ticket, _, _) = settings_ask(&fx);
    run(&mut app, AppEv::Reply(ticket, Reply::failed("unsupported")));
    assert!(switches(&app).is_empty());
    assert!(app.session().backend_banners[&backend(&app)].is_visible(Some("test-model-c")));
}

/// One switch of a thread at a time: a second read before the answer asks nothing.
#[test]
fn a_second_read_waits_for_the_switch_in_flight() {
    let mut app = ready();
    catalog(&mut app, &[("test-model-c", true)]);
    let fx = model_facts(&mut app, "test-model-a", None, true);
    let fx = read_answered(&mut app, &fx, banner_read(blocked_banner()));
    let (ticket, ..) = settings_ask(&fx);
    let fx = model_facts(&mut app, "test-model-a", None, true);
    let fx = read_answered(&mut app, &fx, banner_read(blocked_banner()));
    assert!(fx.commands().is_empty(), "{:?}", fx.commands());
    accepted(&mut app, ticket);
    assert_eq!(switches(&app).len(), 1);
}

/// Luna Reserve is entered from the reserve banner (hidden from the picker on purpose) and left
/// again, to the model it replaced with the effort it had, once a full read says ordinary usage
/// is available.
#[test]
fn luna_reserve_is_entered_and_left_with_the_saved_return() {
    let mut app = ready();
    catalog(&mut app, &[("test-model-a", true), ("gpt-reserve", false)]);
    let fx = model_facts(&mut app, "test-model-a", Some("low"), true);
    let fx = read_answered(&mut app, &fx, banner_read(reserve_banner()));
    let (ticket, _, patch) = settings_ask(&fx);
    assert_eq!(patch.model.as_deref(), Some("gpt-reserve"));
    accepted(&mut app, ticket);
    assert_eq!(
        switches(&app),
        [AutomaticSwitch {
            model: "gpt-reserve".into(),
            effort: Some("low".into()),
            reason: SwitchReason::UsageLimit,
        }]
    );
    // the thread is on Reserve now; the next read says ordinary usage is back
    let fx = model_facts(&mut app, "gpt-reserve", Some("medium"), true);
    let fx = read_answered(
        &mut app,
        &fx,
        json!({"accountId": "workspace-a", "rateLimits": {}, "ordinaryUsageAllowed": true}),
    );
    let (ticket, _, patch) = settings_ask(&fx);
    assert_eq!(patch.model.as_deref(), Some("test-model-a"));
    assert_eq!(
        patch.effort.as_ref().map(|effort| effort.as_str()),
        Some("low")
    );
    accepted(&mut app, ticket);
    assert_eq!(
        switches(&app).last().map(|switch| switch.reason),
        Some(SwitchReason::UsageRecovered)
    );
    // the return is spent: a later recovered read asks nothing
    let fx = model_facts(&mut app, "gpt-reserve", Some("low"), true);
    let fx = read_answered(
        &mut app,
        &fx,
        json!({"accountId": "workspace-a", "rateLimits": {}, "ordinaryUsageAllowed": true}),
    );
    assert!(fx.commands().is_empty(), "{:?}", fx.commands());
}

/// A return saved for another account is not followed.
#[test]
fn a_return_of_another_account_is_dropped() {
    let mut app = ready();
    catalog(&mut app, &[("test-model-a", true), ("gpt-reserve", false)]);
    let fx = model_facts(&mut app, "test-model-a", None, true);
    let fx = read_answered(&mut app, &fx, banner_read(reserve_banner()));
    let (ticket, ..) = settings_ask(&fx);
    accepted(&mut app, ticket);
    let fx = model_facts(&mut app, "gpt-reserve", None, true);
    let fx = read_answered(
        &mut app,
        &fx,
        json!({"accountId": "workspace-b", "rateLimits": {}, "ordinaryUsageAllowed": true}),
    );
    assert!(fx.commands().is_empty(), "{:?}", fx.commands());
}
