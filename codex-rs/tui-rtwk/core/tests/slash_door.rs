#![allow(clippy::expect_used, clippy::unwrap_used)]

//! The slash door (ADR-012, `Feature::RUNS` / `Feature::slash`): a command a feature runs reaches
//! its owner with the text after its name, through the one `SessionIntent::Feature`; the hub
//! names no slice's verb. The features' own behaviour is their tests'; these are the routing:
//! `/rename <name>`, `/model <name>`, `/recap`, `/mcp verbose`, a command nobody owns, and the
//! rule that every command the dispatcher hands to a feature has one.

use codex_app_server_protocol::RequestId;
use codex_tui_core::app::effects::Ask;
use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Intent, apply_app};
use codex_tui_core::commands::{
    CommandFlags, DispatchCx, Notice, Outcome, SessionIntent, SlashCommand, dispatch,
};
use codex_tui_core::features::mcp_inventory;
use codex_tui_core::features::settings::{Note as SettingsNote, args::Refusal};
use codex_tui_core::features::{self, FeatureCommand, FeatureNote};
use codex_tui_core::fixtures::script::build;
use codex_tui_core::ids::ThreadId;
use codex_tui_core::screens::{ScreenIntent, chat};
use codex_tui_core::threads::{Item, Note, NoteKey};
use pretty_assertions::assert_eq;
use serde_json::{Value, json};

const T: &str = "thread-1";

/// A session with one exchange in thread-1.
fn app() -> CodexApp {
    let user = json!({"step": "notify", "message": {"method": "item/completed", "params": {
        "threadId": T, "turnId": "turn-1", "completedAtMs": 0, "startedAtMs": 0,
        "item": {"type": "userMessage", "id": "u0", "clientId": null,
            "content": [{"type": "text", "text": "hello", "text_elements": []}]}}}});
    let said = json!({"step": "notify", "message": {"method": "item/completed", "params": {
        "threadId": T, "turnId": "turn-1", "completedAtMs": 0, "startedAtMs": 0,
        "item": {"type": "agentMessage", "id": "a0", "text": "hi there", "phase": null}}}});
    let script = [
        r#"{"step":"seed","seed":"session","thread":"thread-1"}"#.to_owned(),
        user.to_string(),
        said.to_string(),
    ];
    build(&script.join("\n")).expect("plays").app
}

fn run(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    app.assert_consistent();
    fx
}

/// What the composer's page says for a line the dispatcher turned into `Feature { args }`.
fn slash(app: &mut CodexApp, command: SlashCommand, args: &str) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Chat(chat::Intent::Session {
            thread: Some(ThreadId::new(T)),
            command,
            intent: SessionIntent::Feature {
                args: args.to_owned(),
            },
        }))),
    )
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

#[test]
fn rename_with_a_name_is_the_rename_slices_and_asks_the_server() {
    let mut app = app();
    let fx = slash(&mut app, SlashCommand::Rename, "Fix the build");
    let named = fx.commands().iter().any(|command| {
        matches!(
            command,
            Command::Feature {
                thread,
                command: FeatureCommand::Rename(features::rename::Command::SetName(name)),
                ..
            } if thread.as_str() == T && name == "Fix the build"
        )
    });
    assert!(named, "{:?}", fx.commands());
}

#[test]
fn model_with_a_name_is_checked_by_the_settings_slice() {
    let mut app = app();
    slash(&mut app, SlashCommand::Model, "gpt-nowhere");
    let refused: Vec<_> = notes(&app)
        .into_iter()
        .filter_map(|note| match note.key {
            NoteKey::Feature(FeatureNote::Settings(SettingsNote::Refused(refusal))) => {
                Some(refusal)
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        refused,
        [Refusal::ModelUnavailable("gpt-nowhere".to_owned())]
    );
}

#[test]
fn recap_is_routed_to_the_recap_slice_and_asks_the_host_for_a_structured_turn() {
    let mut app = app();
    let fx = slash(&mut app, SlashCommand::Recap, "");
    let asked = fx.commands().iter().any(|command| {
        matches!(
            command,
            Command::Effect {
                ask: Ask::StructuredTurn { thread, .. },
                ..
            } if thread.as_str() == T
        )
    });
    assert!(asked, "{:?}", fx.commands());
    assert!(
        notes(&app).iter().all(|note| !matches!(
            note.key,
            NoteKey::Command(Notice::NotAvailable(SlashCommand::Recap))
        )),
        "/recap is not a stub any more"
    );
}

#[test]
fn mcp_verbose_asks_for_the_full_inventory() {
    let mut app = app();
    let fx = slash(&mut app, SlashCommand::Mcp, "verbose");
    let details: Vec<Value> =
        fx.commands()
            .iter()
            .filter_map(|command| match command {
                Command::Feature {
                    command:
                        command @ FeatureCommand::McpInventory(mcp_inventory::Command::List { .. }),
                    ..
                } => {
                    let request = features::wire(command, &ThreadId::new(T), RequestId::Integer(1));
                    Some(
                        serde_json::to_value(request).expect("serializes")["params"]["detail"]
                            .clone(),
                    )
                }
                _ => None,
            })
            .collect();
    assert_eq!(details, [json!("full")]);
    // anything else after `/mcp` is not the feature's verb: nothing is asked
    let mut app = self::app();
    let fx = slash(&mut app, SlashCommand::Mcp, "login github");
    assert!(
        fx.commands()
            .iter()
            .all(|command| !matches!(command, Command::Feature { .. }))
    );
}

#[test]
fn a_command_no_feature_runs_says_it_is_not_available() {
    assert!(!features::runs_of(SlashCommand::Pwd));
    let mut app = app();
    slash(&mut app, SlashCommand::Pwd, "");
    let said = notes(&app)
        .into_iter()
        .filter(|note| {
            matches!(
                note.key,
                NoteKey::Command(Notice::NotAvailable(SlashCommand::Pwd))
            )
        })
        .count();
    assert_eq!(said, 1, "{:?}", notes(&app));
}

/// The dispatcher names no slice's command: whatever it hands over as `Feature { .. }` must
/// have an owner, bare and with arguments, or the command is answered "not available" at run
/// time and nothing but this test notices.
#[test]
fn every_command_the_dispatcher_hands_to_a_feature_has_an_owner() {
    let cx = DispatchCx {
        flags: CommandFlags {
            collaboration_modes_enabled: true,
            connectors_enabled: true,
            plugins_command_enabled: true,
            token_activity_command_enabled: true,
            service_tier_commands_enabled: true,
            daybreak_command_description: Some(""),
            goal_command_enabled: true,
            voice_command_enabled: true,
            worktrees_enabled: true,
            allow_elevate_sandbox: true,
            side_conversation_active: false,
        },
        thread_started: true,
        has_backend_auth: true,
        ..DispatchCx::default()
    };
    let mut unowned = Vec::new();
    for command in SlashCommand::ALL {
        for line in [
            format!("/{}", command.command()),
            format!("/{} verbose", command.command()),
        ] {
            if let Outcome::Session {
                intent: SessionIntent::Feature { .. },
                ..
            } = dispatch(&line, &cx)
                && !features::runs_of(command)
            {
                unowned.push(line);
            }
        }
    }
    assert_eq!(unowned, Vec::<String>::new());
}
