#![allow(clippy::expect_used, clippy::unwrap_used)]

//! The doors of the W2 round-1 "needs first" task (doors-r1) that sit in the core: one test per
//! door, each the smallest use a package makes of it.

use std::rc::Rc;

use codex_tui_core::app::{AppEv, AppFx, BackendEv, CodexApp, Command, Intent, apply_app};
use codex_tui_core::features::windows_sandbox::{
    Config, Fact, Host, Intent as Sandbox, Mode, Reply,
};
use codex_tui_core::features::{FeatureCommand, FeatureFact, FeatureIntent, FeatureReply};
use codex_tui_core::ids::{BackendId, ThreadId, Ticket};
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

/// The kit's `presets` and `note` steps are events the host or a feature makes (`AppEv::Presets`,
/// the chat's `Intent::Note`), read from a script line: a refusal names the line.
#[test]
fn a_script_seeds_the_presets_and_refuses_a_note_no_feature_seeds() {
    use codex_tui_core::fixtures::script;
    use codex_tui_core::screens::select::quick_dials::{Dial, Origin, Slot};

    let parsed = script::parse(
        r#"{"step":"presets","slots":[{"slot":2,"input":"/status","label":"Status","origin":"shadowing"},{"slot":5,"input":"hello"}]}"#,
    )
    .expect("a presets line");
    let (_, step) = &parsed.steps[0];
    let Some(Ok(AppEv::Presets(slots))) = step.event() else {
        panic!("a `presets` step is `AppEv::Presets`");
    };
    assert_eq!(
        slots,
        [
            Slot {
                slot: 2,
                dial: Dial {
                    input: "/status".into(),
                    label: Some("Status".into())
                },
                origin: Origin::Shadowing,
            },
            Slot {
                slot: 5,
                dial: Dial::new("hello"),
                origin: Origin::User,
            },
        ]
    );
    // The set goes through the router like the host's.
    let played = script::build(r#"{"step":"presets","slots":[{"slot":1,"input":"/status"}]}"#)
        .expect("the presets are told to the app");
    played.app.assert_consistent();

    // A feature that seeds no note, and a name no feature has, are refused with their line.
    for (feature, want) in [("settings", "NoSeeds"), ("nothing", "UnknownDir")] {
        let line = format!(
            "{}\n{}",
            r#"{"step":"seed","seed":"session","thread":"thread-1"}"#,
            serde_json::json!({"step": "note", "feature": feature, "note": {}})
        );
        let Err(err) = script::build(&line) else {
            panic!("`{feature}` seeds no note");
        };
        assert_eq!(err.line, 2);
        assert!(err.message.contains(want), "{}", err.message);
    }
}

/// The account's plan reaches the limits (`update_account_state` sets `plan_type` before any
/// snapshot): a rolling update that names no plan is judged by it, so a Plus account's notice
/// starts at 50% of the five-hour window and a Pro account's at 75%.
#[test]
fn the_account_plan_judges_a_snapshot_that_names_none() {
    fn notify(app: &mut CodexApp, message: serde_json::Value) {
        let message = serde_json::from_value(message).expect("a notification");
        run(
            app,
            AppEv::Backend(BackendEv::Notification(Box::new(message))),
        );
    }
    fn account(app: &mut CodexApp, plan: &str) {
        let params = serde_json::json!({"authMode": "chatgpt", "planType": plan});
        notify(
            app,
            serde_json::json!({"method": "account/updated", "params": params}),
        );
    }
    fn rolling(app: &mut CodexApp, percent: i32) {
        let window = serde_json::json!({"usedPercent": percent, "windowDurationMins": 300, "resetsAt": 2_000_000_000});
        let rate_limits = serde_json::json!({"limitId": "codex", "primary": window});
        notify(
            app,
            serde_json::json!({"method": "account/rateLimits/updated", "params": {"rateLimits": rate_limits}}),
        );
    }
    fn notice(app: &CodexApp) -> Option<i32> {
        let store = app.session().rate_snapshots.get(&BackendId::default())?;
        store.limits().notice().map(|(used, _)| used.percent)
    }

    let mut app = CodexApp::new();
    account(&mut app, "plus");
    rolling(&mut app, 55);
    assert_eq!(notice(&app), Some(55));

    let mut app = CodexApp::new();
    account(&mut app, "pro");
    rolling(&mut app, 55);
    assert_eq!(notice(&app), None);

    // the new account's plan replaces the old one's, and what was read goes with the old account
    let mut app = CodexApp::new();
    account(&mut app, "plus");
    rolling(&mut app, 55);
    account(&mut app, "pro");
    assert_eq!(notice(&app), None);
    rolling(&mut app, 60);
    assert_eq!(notice(&app), None);
    rolling(&mut app, 80);
    assert_eq!(notice(&app), Some(80));
}

/// The rate limit prompt's "Don't remind me again" is a settings write of
/// `notice.hide_rate_limit_model_nudge`; the host reads the config again and the setting is in
/// `TuiSettings`; a refusal of the file is said in the history with the oracle's words.
#[test]
fn the_rate_limit_prompt_is_hidden_by_a_persisted_notice_setting() {
    use codex_tui_core::app::Ingress;
    use codex_tui_core::app::effects::{Ask, Done};
    use codex_tui_core::app::facts::TuiSettings;
    use codex_tui_core::features::FeatureNote;
    use codex_tui_core::features::settings::{
        Edit, Intent as SettingsIntent, Note as SettingsNote, Setting,
    };
    use codex_tui_core::fixtures::script::build;
    use codex_tui_core::threads::{Item, Note};

    fn write_of(app: &mut CodexApp) -> (u64, Vec<(String, serde_json::Value)>) {
        let intent = SettingsIntent::Request(Edit::RateLimitPromptHidden);
        let fx = run(
            app,
            AppEv::Intent(Intent::Feature(FeatureIntent::from(intent))),
        );
        fx.commands()
            .iter()
            .find_map(|command| match command {
                Command::Effect {
                    request,
                    ask: Ask::ConfigWrite { edits },
                    ..
                } => Some((
                    *request,
                    edits
                        .iter()
                        .map(|edit| (edit.key_path.clone(), edit.value.clone()))
                        .collect(),
                )),
                _ => None,
            })
            .expect("the setting is a write of the config")
    }
    fn answered(app: &mut CodexApp, request: u64, result: Result<(), String>) {
        let epoch = app.epoch();
        let done = Done::Config(result);
        app.ingest(Ingress::new(epoch, AppEv::Effect { request, done }));
        app.assert_consistent();
    }
    fn said(app: &CodexApp) -> Vec<SettingsNote> {
        app.active_thread()
            .transcript()
            .iter()
            .filter_map(|(_, item)| match item {
                Item::Note(Note {
                    key: NoteKey::Feature(FeatureNote::Settings(note)),
                    ..
                }) => Some(note.clone()),
                _ => None,
            })
            .collect()
    }

    let mut app = build(r#"{"step":"seed","seed":"session","thread":"thread-1"}"#)
        .expect("plays")
        .app;
    assert!(!app.tui().hide_rate_limit_model_nudge);
    let (request, edits) = write_of(&mut app);
    assert_eq!(
        edits,
        [("notice.hide_rate_limit_model_nudge".to_owned(), true.into())]
    );
    answered(&mut app, request, Ok(()));
    assert_eq!(said(&app), []);
    // The host's read of the config after the write.
    let read = TuiSettings {
        hide_rate_limit_model_nudge: true,
        ..TuiSettings::default()
    };
    run(&mut app, AppEv::Tui(read));
    assert!(app.tui().hide_rate_limit_model_nudge);

    let (request, _) = write_of(&mut app);
    answered(&mut app, request, Err("read-only".into()));
    assert_eq!(
        said(&app),
        [SettingsNote::SaveFailed {
            setting: Setting::RateLimitReminder,
            error: "read-only".into()
        }]
    );
}
