#![allow(clippy::expect_used, clippy::unwrap_used)]

//! The server feeds of the settings dialogs (oracle `experimental_features.rs`,
//! `hooks_rpc.rs`, `chatwidget/skills.rs`, `app/tui_mode_picker.rs`): what is asked of the
//! server when a dialog opens or a switch is flipped, and what the dialogs are offered from the
//! answers. Cursor, checks and pages are the sheets' own.

use codex_tui_core::app::effects::{Ask, Done};
use codex_tui_core::commands::SessionIntent;
use codex_tui_core::app::{AppEv, CodexApp, Command, Ingress, Intent};
use codex_tui_core::features::settings::{
    Command as SettingsCommand, Edit, Intent as SettingsIntent, Note, Offer, SetupKind, Which, Wish,
};
use codex_tui_core::features::{self, FeatureCommand, FeatureIntent};
use codex_tui_core::fixtures::script::build;
use codex_tui_core::ids::Ticket;
use codex_tui_core::rpc::Reply;
use codex_tui_core::screens::{ScreenIntent, chat};
use pretty_assertions::assert_eq;
use serde_json::{Value, json};

const SESSION: &str = r#"{"step":"seed","seed":"session","thread":"thread-1"}"#;

fn app() -> CodexApp {
    build(SESSION).expect("session").app
}

fn run(app: &mut CodexApp, ev: AppEv) -> Vec<Command> {
    let epoch = app.epoch();
    let fx = app.ingest(Ingress::new(epoch, ev));
    app.assert_consistent();
    fx.map(|fx| fx.commands().to_vec()).unwrap_or_default()
}

fn open(app: &mut CodexApp, which: Which) -> Vec<Command> {
    let intent = SettingsIntent::Open(which);
    run(
        app,
        AppEv::Intent(Intent::Feature(FeatureIntent::from(intent))),
    )
}

fn request(app: &mut CodexApp, edit: Edit) -> Vec<Command> {
    let intent = SettingsIntent::Request(edit);
    run(
        app,
        AppEv::Intent(Intent::Feature(FeatureIntent::from(intent))),
    )
}

/// The settings requests of a step, with their tickets.
fn asked(commands: &[Command]) -> Vec<(Ticket, SettingsCommand)> {
    commands
        .iter()
        .filter_map(|command| match command {
            Command::Feature {
                ticket,
                command: FeatureCommand::Settings(command),
                ..
            } => Some((*ticket, command.clone())),
            _ => None,
        })
        .collect()
}

fn answer(app: &mut CodexApp, asked: &(Ticket, SettingsCommand), result: Value) -> Vec<Command> {
    let command = FeatureCommand::Settings(asked.1.clone());
    let reply = features::decode(&command, result).expect("decodes");
    run(app, AppEv::Reply(asked.0, Reply::feature(reply)))
}

fn offer(app: &CodexApp, which: Which) -> Option<Offer> {
    app.features().settings.offers.get(&which).cloned()
}

fn feature(name: &str, stage: &str, enabled: bool) -> Value {
    json!({
        "name": name,
        "stage": stage,
        "displayName": (stage == "beta").then(|| format!("{name} name")),
        "description": (stage == "beta").then(|| format!("{name} text")),
        "announcement": null,
        "enabled": enabled,
        "defaultEnabled": false,
    })
}

#[test]
fn experimental_pages_are_read_to_the_end_and_only_beta_is_offered() {
    let mut app = app();
    let first = asked(&open(&mut app, Which::Experimental));
    let [first] = first.as_slice() else {
        panic!("one request: {first:?}")
    };
    assert_eq!(first.1, SettingsCommand::Features { cursor: None });
    let Some(Offer::Experimental(loading)) = offer(&app, Which::Experimental) else {
        panic!("offered while loading")
    };
    assert!(loading.loading);

    let second = asked(&answer(
        &mut app,
        first,
        json!({"data": [feature("a", "beta", true), feature("x", "stable", true)], "nextCursor": "c1"}),
    ));
    let [second] = second.as_slice() else {
        panic!("the next page is asked for: {second:?}")
    };
    assert_eq!(
        second.1,
        SettingsCommand::Features {
            cursor: Some("c1".into())
        }
    );
    // the same key again is dropped (`dedup`)
    answer(
        &mut app,
        second,
        json!({"data": [feature("a", "beta", true), feature("b", "beta", false)], "nextCursor": null}),
    );
    let Some(Offer::Experimental(done)) = offer(&app, Which::Experimental) else {
        panic!("offered")
    };
    assert!(!done.loading);
    let keys: Vec<_> = done.features.iter().map(|f| f.key.as_str()).collect();
    assert_eq!(keys, ["a", "b"]);
}

#[test]
fn a_repeated_cursor_fails_the_listing() {
    let mut app = app();
    let first = asked(&open(&mut app, Which::Experimental)).remove(0);
    let second = asked(&answer(
        &mut app,
        &first,
        json!({"data": [], "nextCursor": "c"}),
    ))
    .remove(0);
    answer(&mut app, &second, json!({"data": [], "nextCursor": "c"}));
    let Some(Offer::Experimental(dialog)) = offer(&app, Which::Experimental) else {
        panic!("offered")
    };
    assert!(!dialog.loading);
    assert_eq!(dialog.features.len(), 0);
}

#[test]
fn saved_features_are_written_to_the_config_and_read_back() {
    let mut app = app();
    let first = asked(&open(&mut app, Which::Experimental)).remove(0);
    answer(
        &mut app,
        &first,
        json!({"data": [feature("a", "beta", false)], "nextCursor": null}),
    );
    let commands = request(&mut app, Edit::FeaturesSave(vec![("a".into(), true)]));
    let (request_id, edits) = commands
        .iter()
        .find_map(|command| match command {
            Command::Effect {
                request,
                ask: Ask::ConfigWrite { edits },
                ..
            } => Some((*request, edits.clone())),
            _ => None,
        })
        .expect("a config write");
    assert_eq!(edits[0].key_path, "features.a");
    let after = run(
        &mut app,
        AppEv::Effect {
            request: request_id,
            done: Done::Config(Ok(())),
        },
    );
    let readback = asked(&after).remove(0);
    answer(
        &mut app,
        &readback,
        json!({"data": [feature("a", "beta", true)], "nextCursor": null}),
    );
    let Some(Offer::Experimental(dialog)) = offer(&app, Which::Experimental) else {
        panic!("offered")
    };
    assert!(dialog.features[0].enabled);
    assert_eq!(dialog.confirmed, 1);
}

#[test]
fn skills_are_listed_and_a_switch_follows_the_server() {
    let mut app = app();
    let list = asked(&open(&mut app, Which::Skills)).remove(0);
    answer(
        &mut app,
        &list,
        json!({"data": [{"cwd": "", "errors": [], "skills": [{
            "name": "deploy", "description": "Deploys", "path": "/s/deploy/SKILL.md",
            "scope": "user", "enabled": true, "pluginId": null
        }]}]}),
    );
    let Some(Offer::Setup(setup)) = offer(&app, Which::Skills) else {
        panic!("offered")
    };
    assert_eq!(setup.kind, SetupKind::Skills);
    assert_eq!(setup.items[0].id, "/s/deploy/SKILL.md");
    let write = asked(&request(
        &mut app,
        Edit::SkillToggled("/s/deploy/SKILL.md".into(), false),
    ))
    .remove(0);
    answer(&mut app, &write, json!({"effectiveEnabled": false}));
    let Some(Offer::Setup(setup)) = offer(&app, Which::Skills) else {
        panic!("offered")
    };
    assert!(!setup.items[0].enabled);
}

#[test]
fn hook_switches_send_one_write_at_a_time_and_the_last_value_after_it() {
    let mut app = app();
    let first = asked(&request(
        &mut app,
        Edit::HookEnabled {
            key: "k".into(),
            enabled: false,
        },
    ));
    assert_eq!(first.len(), 1);
    // flipped back and forth while the write is on its way: nothing is sent yet
    assert_eq!(
        asked(&request(
            &mut app,
            Edit::HookEnabled {
                key: "k".into(),
                enabled: true
            }
        ))
        .len(),
        0
    );
    let next = asked(&answer(&mut app, &first[0], json!({"status": "ok"})));
    assert_eq!(next.len(), 1);
    assert_eq!(
        next[0].1,
        SettingsCommand::HooksWrite {
            value: json!({"k": {"enabled": true}})
        }
    );
    assert!(asked(&answer(&mut app, &next[0], json!({}))).is_empty());
    assert!(app.features().settings.hook_writes.is_empty());
}

#[test]
fn the_tui_mode_is_written_to_the_config() {
    let mut app = app();
    let commands = request(&mut app, Edit::TuiMode(true));
    let edits = commands
        .iter()
        .find_map(|command| match command {
            Command::Effect {
                ask: Ask::ConfigWrite { edits },
                ..
            } => Some(edits.clone()),
            _ => None,
        })
        .expect("a config write");
    assert_eq!(edits[0].key_path, "tui.fullscreen_transcript");
    assert_eq!(edits[0].value, json!(true));
}

#[test]
fn the_keymap_binding_is_written_under_its_context_and_action() {
    let mut app = app();
    let commands = request(
        &mut app,
        Edit::KeymapBound {
            context: "composer".into(),
            action: "submit".into(),
            keys: vec!["ctrl-j".into()],
        },
    );
    let edits = commands
        .iter()
        .find_map(|command| match command {
            Command::Effect {
                ask: Ask::ConfigWrite { edits },
                ..
            } => Some(edits.clone()),
            _ => None,
        })
        .expect("a config write");
    assert_eq!(edits[0].key_path, "tui.keymap.composer.submit");
    assert_eq!(edits[0].value, json!(["ctrl-j"]));
}

// ---- `/resume <name or id>`, the skills summary, the shortcut capture

fn slash_resume(app: &mut CodexApp, name: &str) -> Vec<Command> {
    run(
        app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Chat(chat::Intent::Session {
            thread: Some(codex_tui_core::ids::ThreadId::new("thread-1")),
            command: codex_tui_core::commands::SlashCommand::Resume,
            intent: SessionIntent::Feature {
                args: name.to_owned(),
            },
        }))),
    )
}

fn thread_row(id: &str, name: Option<&str>, preview: &str) -> Value {
    json!({
        "id": id, "sessionId": id, "forkedFromId": null, "preview": preview, "ephemeral": false,
        "modelProvider": "openai", "createdAt": 1, "updatedAt": 1, "status": {"type": "notLoaded"},
        "path": null, "cwd": "/w", "cliVersion": "1", "source": "cli", "agentNickname": null,
        "agentRole": null, "gitInfo": null, "name": name, "turns": [],
    })
}

fn opened(app: &CodexApp) -> Vec<String> {
    app.wishes()
        .pending()
        .filter_map(|wish| match &wish.wish {
            features::FeatureWish::Settings(Wish::OpenThread(id)) => Some(id.to_string()),
            _ => None,
        })
        .collect()
}

fn settings_notes(app: &CodexApp) -> Vec<Note> {
    use codex_tui_core::threads::{Item, NoteKey};
    app.active_thread()
        .transcript()
        .iter()
        .filter_map(|(_, item)| match item {
            Item::Note(note) => match &note.key {
                NoteKey::Feature(features::FeatureNote::Settings(note)) => Some(note.clone()),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

#[test]
fn resume_by_name_pages_through_the_listing_and_opens_the_one_session() {
    let mut app = app();
    let first = asked(&slash_resume(&mut app, "work")).remove(0);
    assert_eq!(first.1, SettingsCommand::Sessions { cursor: None });
    let second = asked(&answer(
        &mut app,
        &first,
        json!({"data": [thread_row("a", Some("play"), "x")], "nextCursor": "2024-01-01T00-00-00|11111111-1111-1111-1111-111111111111"}),
    ))
    .remove(0);
    assert!(matches!(
        second.1,
        SettingsCommand::Sessions { cursor: Some(_) }
    ));
    answer(
        &mut app,
        &second,
        json!({"data": [thread_row("b", None, "  work ")], "nextCursor": null}),
    );
    assert_eq!(opened(&app), ["b"]);
}

#[test]
fn resume_by_name_refuses_two_sessions_with_one_label_and_none() {
    let mut app = app();
    let first = asked(&slash_resume(&mut app, "dup")).remove(0);
    answer(
        &mut app,
        &first,
        json!({"data": [thread_row("a", Some("dup"), ""), thread_row("b", Some("dup"), "")], "nextCursor": null}),
    );
    assert_eq!(
        settings_notes(&app),
        [Note::AmbiguousSession {
            name: "dup".into(),
            first: "a".into(),
            second: "b".into()
        }]
    );
    let none = asked(&slash_resume(&mut app, "nothing")).remove(0);
    answer(&mut app, &none, json!({"data": [], "nextCursor": null}));
    assert!(settings_notes(&app).contains(&Note::NoSavedChat {
        name: "nothing".into()
    }));
    assert!(opened(&app).is_empty());
}

#[test]
fn a_label_found_across_a_cursor_that_pins_no_session_is_not_verified() {
    let mut app = app();
    let first = asked(&slash_resume(&mut app, "x")).remove(0);
    let second = asked(&answer(
        &mut app,
        &first,
        json!({"data": [thread_row("a", Some("x"), "")], "nextCursor": "opaque"}),
    ))
    .remove(0);
    answer(&mut app, &second, json!({"data": [], "nextCursor": null}));
    assert_eq!(
        settings_notes(&app),
        [Note::UnverifiedSession { id: "a".into() }]
    );
}

#[test]
fn resume_by_id_reads_the_session() {
    let mut app = app();
    let id = "11111111-1111-1111-1111-111111111111";
    let read = asked(&slash_resume(&mut app, id)).remove(0);
    assert_eq!(read.1, SettingsCommand::Session { id: id.into() });
    answer(
        &mut app,
        &read,
        json!({"thread": thread_row(id, None, "p")}),
    );
    assert_eq!(opened(&app), [id]);
}

#[test]
fn closing_skills_says_how_many_were_switched() {
    let mut app = app();
    let list = asked(&open(&mut app, Which::Skills)).remove(0);
    let skill = |name: &str, enabled: bool| {
        json!({"name": name, "description": "d", "path": format!("/s/{name}/SKILL.md"),
               "scope": "user", "enabled": enabled, "pluginId": null})
    };
    answer(
        &mut app,
        &list,
        json!({"data": [{"cwd": "", "errors": [], "skills": [skill("a", true), skill("b", false), skill("c", true)]}]}),
    );
    for (path, enabled) in [("/s/a/SKILL.md", false), ("/s/b/SKILL.md", true)] {
        let write = asked(&request(&mut app, Edit::SkillToggled(path.into(), enabled))).remove(0);
        answer(&mut app, &write, json!({"effectiveEnabled": enabled}));
    }
    request(&mut app, Edit::SkillsClosed);
    assert_eq!(
        settings_notes(&app),
        [Note::SkillsChanged {
            enabled: 1,
            disabled: 1
        }]
    );
}

fn capture(
    app: &mut CodexApp,
    key: &str,
    intent: codex_tui_core::features::settings::capture::Intent,
) -> Vec<Command> {
    let intent = SettingsIntent::KeymapCaptured {
        context: "composer".into(),
        action: "submit".into(),
        key: key.into(),
        intent,
    };
    run(
        app,
        AppEv::Intent(Intent::Feature(FeatureIntent::from(intent))),
    )
}

#[test]
fn a_captured_key_is_written_and_said_once_the_file_has_it() {
    use codex_tui_core::features::settings::capture::{Intent as Capture, Said};
    let mut app = app();
    // (`ctrl-j` is the composer's newline: a conflict, which the keymap refuses)
    let commands = capture(&mut app, "ctrl-alt-s", Capture::ReplaceAll);
    let (request_id, edits) = commands
        .iter()
        .find_map(|command| match command {
            Command::Effect {
                request,
                ask: Ask::ConfigWrite { edits },
                ..
            } => Some((*request, edits.clone())),
            _ => None,
        })
        .expect("a config write");
    assert_eq!(edits[0].key_path, "tui.keymap.composer.submit");
    assert_eq!(edits[0].value, json!(["ctrl-alt-s"]));
    run(
        &mut app,
        AppEv::Effect {
            request: request_id,
            done: Done::Config(Ok(())),
        },
    );
    assert_eq!(
        settings_notes(&app),
        [Note::KeymapEdited {
            context: "composer".into(),
            action: "submit".into(),
            key: "ctrl-alt-s".into(),
            said: Said::Remapped
        }]
    );
}

#[test]
fn a_captured_key_that_means_two_things_is_a_conflict_for_the_surface() {
    use codex_tui_core::features::settings::capture::Intent as Capture;
    let mut app = app();
    // `ctrl-t` opens the transcript (global): the composer cannot take it too.
    let commands = capture(&mut app, "ctrl-t", Capture::ReplaceAll);
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, Command::Effect { .. })),
        "nothing is written"
    );
    let conflict = app.wishes().pending().find_map(|wish| match &wish.wish {
        features::FeatureWish::Settings(Wish::KeymapConflict { key, .. }) => Some(key.clone()),
        _ => None,
    });
    assert_eq!(conflict.as_deref(), Some("ctrl-t"));
}
