#![allow(clippy::expect_used, clippy::unwrap_used)]

//! `/presets` (oracle `chatwidget/quick_dials.rs`, `app/quick_dials.rs`,
//! `tests/quick_dials_tests.rs`): the current settings as sources, the slot pickers, the
//! editor and the confirmation, and the write of a slot into the config
//! (`tui.presets.<slot>`), which changes the slot only once the config took it.

use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Intent, apply_app};
use codex_tui_core::ids::{ThreadId, Ticket};
use codex_tui_core::rpc::{Reply, RpcError};
use codex_tui_core::screens::chat::StatusFacts;
use codex_tui_core::screens::select::models::{Current, Preset};
use codex_tui_core::screens::select::quick_dials::{
    self as quick, Action, Composer, Dial, Gate, Invalid, Menu as Quick, Open, Origin, PickerMode,
    Settings, Slot, Source, Target, Unavailable,
};
use codex_tui_core::screens::select::{self, Intent as Select, Menu, Note, Picked};
use codex_tui_core::screens::{ScreenCommand, ScreenEv, ScreenIntent, ScreenReply};
use pretty_assertions::assert_eq;
use serde_json::json;

fn run(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    app.assert_consistent();
    fx
}

/// An app with a thread in use: the notes the menus say go into its transcript.
fn ready() -> CodexApp {
    let mut app = CodexApp::new();
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
        AppEv::Reply(ticket, Reply::thread_started(ThreadId::new("thread-1"))),
    );
    app
}

fn select(app: &mut CodexApp, intent: Select) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Select(intent))),
    )
}

fn quick(app: &mut CodexApp, open: Open) -> AppFx {
    select(
        app,
        Select::Quick {
            gate: Gate::Ready,
            open,
        },
    )
}

fn choose(app: &mut CodexApp, id: &str) -> AppFx {
    select(app, Select::Choose(id.to_owned()))
}

fn menu(app: &CodexApp) -> Option<Quick> {
    match app.screens().select.menu() {
        Some(Menu::Quick(menu)) => Some((**menu).clone()),
        _ => None,
    }
}

fn catalog(app: &mut CodexApp) {
    let preset: Preset = serde_json::from_value(json!({
        "model": "gpt-5.5", "name": "GPT 5.5", "default_effort": "medium",
        "efforts": [{"effort": "low"}, {"effort": "medium"}, {"effort": "high"}],
        "service_tiers": [{"id": "priority", "name": "Fast"}],
    }))
    .expect("preset");
    run(
        app,
        AppEv::Screen(ScreenEv::Select(select::Ev::Loaded(vec![preset]))),
    );
}

fn slots(app: &mut CodexApp, slots: Vec<Slot>) {
    run(app, AppEv::Presets(slots));
}

fn saved(slot: u8, input: &str, origin: Origin) -> Slot {
    Slot {
        slot,
        dial: Dial {
            input: input.to_owned(),
            label: Some(String::new()),
        },
        origin,
    }
}

fn facts() -> StatusFacts {
    StatusFacts {
        model: Some("gpt-5.5".into()),
        effort: Some("high".into()),
        approval_policy: Some("on-request".into()),
        permission_profile: Some(":workspace".into()),
        terminal_title: Some(vec![
            "model-name".into(),
            "project".into(),
            "model-name".into(),
        ]),
        items: Some(vec![
            "model-name".into(),
            "project-root".into(),
            "model-name".into(),
        ]),
        colors: false,
        ..StatusFacts::default()
    }
}

fn settings(facts: &StatusFacts) -> Box<Settings> {
    Box::new(Settings::from_facts(facts, Current::from_facts(facts)))
}

fn sources(app: &CodexApp) -> Vec<(Source, Result<String, Unavailable>)> {
    match menu(app) {
        Some(Quick::Sources(rows)) => rows
            .into_iter()
            .map(|row| (row.source, row.dial.map(|dial| dial.input)))
            .collect(),
        other => panic!("expected the sources, got {other:?}"),
    }
}

/// The preset write the menus left for the config, with its ticket.
fn written(fx: &AppFx) -> (Ticket, u8, Option<Dial>) {
    let writes: Vec<_> = fx
        .commands()
        .iter()
        .filter_map(|command| match command {
            Command::Screen {
                ticket,
                command: ScreenCommand::Select(select::Command::Preset { slot, dial }),
                ..
            } => Some((*ticket, *slot, dial.clone())),
            _ => None,
        })
        .collect();
    let [one] = writes.as_slice() else {
        panic!("expected one preset write, got {:?}", fx.commands());
    };
    one.clone()
}

fn took(app: &mut CodexApp, ticket: Ticket) {
    run(
        app,
        AppEv::Reply(
            ticket,
            Reply::screen(ScreenReply::Select(select::Reply::Written {
                overridden: false,
            })),
        ),
    );
}

/// What the menus said, as the transcript of the thread in use has it.
fn said(app: &CodexApp) -> Vec<Note> {
    use codex_tui_core::threads::{Item, NoteKey};
    assert!(
        app.screens().select.notes.is_empty(),
        "the screen's notes are told at once"
    );
    app.active_thread()
        .transcript()
        .iter()
        .filter_map(|(_, item)| match item {
            Item::Note(note) => match &note.key {
                NoteKey::Select(note) => Some(note.clone()),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

#[test]
fn the_current_settings_are_offered_as_model_permissions_title_and_status_layout() {
    let mut app = ready();
    catalog(&mut app);
    let facts = facts();
    quick(&mut app, Open::Prefills(settings(&facts)));
    assert_eq!(
        sources(&app),
        vec![
            (
                Source::Model,
                Ok("/model gpt-5.5 --reasoning=high --speed=standard".to_owned())
            ),
            (
                Source::Permissions,
                Ok(
                    "/permissions :workspace --approval-policy=on-request --reviewer=user"
                        .to_owned()
                )
            ),
            (
                Source::TerminalTitle,
                Ok("/title model project-name model".to_owned())
            ),
            (
                Source::StatusLine,
                Ok("/statusline model project-name model --no-colors".to_owned())
            ),
        ]
    );
}

#[test]
fn a_source_that_cannot_be_recalled_says_why() {
    let mut app = ready();
    catalog(&mut app);
    let mut facts = facts();
    facts.effort = Some("removed-effort".into());
    facts.auto_review = true;
    facts.terminal_title = Some(vec!["removed-title-item".into()]);
    facts.items = Some(vec!["model".into()]);
    facts.colors = true;
    quick(&mut app, Open::Prefills(settings(&facts)));
    let rows = sources(&app);
    assert_eq!(rows[0].1, Err(Unavailable::Effort));
    assert_eq!(
        rows[1].1,
        Ok(
            "/permissions :workspace --approval-policy=on-request --reviewer=auto_review"
                .to_owned()
        )
    );
    assert_eq!(rows[2].1, Err(Unavailable::TitleItems));
    assert_eq!(rows[3].1, Ok("/statusline model --colors".to_owned()));
    // an unavailable row does nothing
    choose(&mut app, quick::TERMINAL_TITLE);
    assert!(matches!(menu(&app), Some(Quick::Sources(_))));

    // a model the catalog does not list, a speed it does not offer, no named profile
    let mut other = facts.clone();
    other.model = Some("unlisted".into());
    other.permission_profile = None;
    quick(&mut app, Open::Prefills(settings(&other)));
    let rows = sources(&app);
    assert_eq!(rows[0].1, Err(Unavailable::Model));
    assert_eq!(rows[1].1, Err(Unavailable::NoProfile));
    let mut speed = facts;
    speed.effort = None;
    speed.service_tier = Some("ultrafast".into());
    quick(&mut app, Open::Prefills(settings(&speed)));
    assert_eq!(sources(&app)[0].1, Err(Unavailable::Speed));
    speed.service_tier = Some("priority".into());
    quick(&mut app, Open::Prefills(settings(&speed)));
    assert_eq!(
        sources(&app)[0].1,
        Ok("/model gpt-5.5 --speed=priority".to_owned())
    );
}

#[test]
fn empty_layouts_are_cleared_and_granular_policies_and_names_are_quoted() {
    let mut app = ready();
    catalog(&mut app);
    let mut facts = facts();
    facts.terminal_title = Some(Vec::new());
    facts.items = Some(Vec::new());
    facts.colors = true;
    facts.approval_policy = Some("granular".into());
    facts.approval_granular = Some(r#"{"granular":{"rules":true}}"#.into());
    facts.permission_profile = Some("team review".into());
    quick(&mut app, Open::Prefills(settings(&facts)));
    let rows = sources(&app);
    assert_eq!(rows[2].1, Ok("/title --clear".to_owned()));
    assert_eq!(rows[3].1, Ok("/statusline --clear --colors".to_owned()));
    assert_eq!(
        rows[1].1,
        Ok(
            r#"/permissions 'team review' --approval-policy='{"granular":{"rules":true}}' --reviewer=user"#
                .to_owned()
        )
    );
}

#[test]
fn a_source_opens_the_editor_and_the_text_goes_to_a_slot_picker_on_the_first_empty_slot() {
    let mut app = ready();
    catalog(&mut app);
    slots(
        &mut app,
        vec![
            saved(1, "/compact", Origin::User),
            saved(2, "hello", Origin::User),
        ],
    );
    let facts = facts();
    quick(&mut app, Open::Prefills(settings(&facts)));
    choose(&mut app, quick::PERMISSIONS);
    let Some(Quick::Editor { target, dial }) = menu(&app) else {
        panic!("expected the editor, got {:?}", menu(&app));
    };
    assert_eq!(target, Target::PickSlot);
    assert!(dial.input.starts_with("/permissions :workspace"));
    // Esc shows the sources again, on the row that opened the editor
    select(&mut app, Select::Close);
    assert!(matches!(menu(&app), Some(Quick::Sources(_))));
    choose(&mut app, quick::PERMISSIONS);

    select(&mut app, Select::Draft("/permissions edited".into()));
    let Some(
        menu @ Quick::Slots {
            mode: PickerMode::Save,
            ..
        },
    ) = self::menu(&app)
    else {
        panic!("expected the save picker, got {:?}", self::menu(&app));
    };
    let page = menu.page(&app.screens().select.quick).expect("rows");
    assert_eq!(page.rows.len(), 10);
    assert_eq!(page.initial, Some(2), "the first empty slot is 3");
    assert!(page.rows.iter().all(|row| !row.disabled));
    assert_eq!(
        page.rows[0].saved.as_ref().map(|d| d.input.as_str()),
        Some("/compact")
    );
}

#[test]
fn saving_into_an_empty_slot_writes_the_config_and_the_slot_follows_the_answer() {
    let mut app = ready();
    quick(
        &mut app,
        Open::Editor {
            target: Target::PickSlot,
            dial: None,
            composer: Composer {
                text: "write the tests".into(),
                ..Composer::default()
            },
        },
    );
    let Some(Quick::Editor { dial, .. }) = menu(&app) else {
        panic!("expected the editor");
    };
    assert_eq!(dial.input, "write the tests");
    select(&mut app, Select::Draft("write the tests now".into()));
    let fx = choose(&mut app, &quick::slot_id(4));
    assert_eq!(app.screens().select.menu(), None, "every menu closes");
    let (ticket, slot, dial) = written(&fx);
    assert_eq!(slot, 4);
    let dial = dial.expect("a dial");
    assert_eq!(dial.input, "write the tests now");
    assert!(
        app.screens().select.quick.get(4).is_none(),
        "not before the config took it"
    );
    took(&mut app, ticket);
    let saved = app.screens().select.quick.get(4).expect("saved");
    assert_eq!(saved.dial.input, "write the tests now");
    assert_eq!(
        saved.dial.label.as_deref(),
        Some(""),
        "a saved slot has a label"
    );
    assert_eq!(saved.origin, Origin::User);
    assert_eq!(said(&app), vec![Note::PresetSaved(4)]);
}

#[test]
fn the_write_is_one_batch_edit_of_the_slot_table_and_a_delete_clears_the_path() {
    let edit = select::persist::preset_edit(
        3,
        Some(&Dial {
            input: "hi".into(),
            label: None,
        }),
    );
    assert_eq!(edit.key_path, "tui.presets.3");
    assert_eq!(edit.value, json!({"input": "hi", "label": ""}));
    let clear = select::persist::preset_edit(3, None);
    assert_eq!(clear.value, serde_json::Value::Null);
}

#[test]
fn replacing_or_deleting_a_saved_slot_asks_first() {
    let mut app = ready();
    slots(&mut app, vec![saved(3, "old text", Origin::User)]);
    quick(&mut app, Open::Save(Dial::new("new text")));
    let fx = choose(&mut app, &quick::slot_id(3));
    assert!(
        fx.commands().is_empty(),
        "nothing is written before the confirmation"
    );
    let confirm = menu(&app).expect("the confirmation");
    assert_eq!(confirm.action(), Some(Action::Replace));
    assert_eq!(
        confirm,
        Quick::Confirm {
            slot: 3,
            old: Some(Dial {
                input: "old text".into(),
                label: Some(String::new())
            }),
            new: Some(Dial::new("new text")),
        }
    );
    let fx = choose(&mut app, quick::CONFIRM);
    let (ticket, slot, dial) = written(&fx);
    assert_eq!(
        (slot, dial.map(|d| d.input)),
        (3, Some("new text".to_owned()))
    );
    took(&mut app, ticket);
    assert_eq!(
        app.screens()
            .select
            .quick
            .get(3)
            .map(|s| s.dial.input.clone()),
        Some("new text".to_owned())
    );

    quick(&mut app, Open::Picker(PickerMode::Clear));
    let Some(picker) = menu(&app) else {
        panic!("the delete picker")
    };
    let page = picker.page(&app.screens().select.quick).expect("rows");
    assert_eq!(
        page.rows.iter().map(|r| r.slot).collect::<Vec<_>>(),
        vec![3]
    );
    choose(&mut app, &quick::slot_id(3));
    assert_eq!(menu(&app).and_then(|m| m.action()), Some(Action::Delete));
    let fx = choose(&mut app, quick::CONFIRM);
    let (ticket, slot, dial) = written(&fx);
    assert_eq!((slot, dial), (3, None));
    took(&mut app, ticket);
    assert!(app.screens().select.quick.get(3).is_none());
}

#[test]
fn a_slot_of_another_layer_is_not_written_and_the_editor_comes_back() {
    let mut app = ready();
    slots(
        &mut app,
        vec![
            saved(1, "managed", Origin::Outside),
            saved(2, "shadowing", Origin::Shadowing),
        ],
    );
    // Edit shows the saved slots only
    quick(&mut app, Open::Picker(PickerMode::Edit));
    choose(&mut app, &quick::slot_id(1));
    select(&mut app, Select::Draft("changed".into()));
    let confirm = menu(&app).expect("the replace confirmation");
    assert_eq!(confirm.action(), Some(Action::Replace));
    let fx = choose(&mut app, quick::CONFIRM);
    assert!(fx.commands().is_empty(), "nothing is written");
    assert_eq!(said(&app), vec![Note::PresetOutside(1)]);
    let Some(Quick::Editor { target, dial }) = menu(&app) else {
        panic!("the editor is shown again, got {:?}", menu(&app));
    };
    assert_eq!((target, dial.input.as_str()), (Target::Slot(1), "changed"));

    // a shadowing user slot is replaced, but not cleared
    quick(&mut app, Open::Picker(PickerMode::Clear));
    choose(&mut app, &quick::slot_id(2));
    let fx = choose(&mut app, quick::CONFIRM);
    assert!(fx.commands().is_empty());
    assert_eq!(said(&app).last(), Some(&Note::PresetOutside(2)));
    quick(&mut app, Open::Save(Dial::new("again")));
    choose(&mut app, &quick::slot_id(2));
    let fx = choose(&mut app, quick::CONFIRM);
    written(&fx);
}

#[test]
fn a_refused_write_keeps_the_slot_and_shows_the_editor_again() {
    let mut app = ready();
    quick(&mut app, Open::Save(Dial::new("keep me")));
    let fx = choose(&mut app, &quick::slot_id(5));
    let (ticket, ..) = written(&fx);
    run(
        &mut app,
        AppEv::Reply(ticket, Reply::error(RpcError::Remote("read-only".into()))),
    );
    assert!(app.screens().select.quick.get(5).is_none());
    assert_eq!(
        said(&app),
        vec![Note::PresetFailed {
            slot: 5,
            error: "read-only".into()
        }]
    );
    let Some(Quick::Editor { target, dial }) = menu(&app) else {
        panic!("the editor is shown again");
    };
    assert_eq!((target, dial.input.as_str()), (Target::Slot(5), "keep me"));
}

#[test]
fn an_input_that_is_not_a_preset_is_refused() {
    let mut app = ready();
    quick(&mut app, Open::Save(Dial::new("!ls")));
    assert_eq!(menu(&app), None);
    assert_eq!(said(&app), vec![Note::PresetInvalid(Invalid::Shell)]);
    for (text, reason) in [("   ", Invalid::Empty), ("!ls", Invalid::Shell)] {
        assert_eq!(Dial::new(text).validate(), Err(reason));
    }
}

#[test]
fn presets_are_gated_where_the_user_is() {
    let mut app = ready();
    for gate in [Gate::Blocked, Gate::Starting] {
        select(
            &mut app,
            Select::Quick {
                gate,
                open: Open::Picker(PickerMode::Save),
            },
        );
        assert_eq!(menu(&app), None);
    }
    assert_eq!(
        said(&app),
        vec![Note::PresetsUnavailable, Note::PresetsStarting]
    );
}

#[test]
fn the_editor_starts_empty_when_the_composer_holds_images_or_mentions() {
    for composer in [
        Composer {
            text: "x".into(),
            images: true,
            mentions: false,
        },
        Composer {
            text: "x".into(),
            images: false,
            mentions: true,
        },
    ] {
        let composer_images = composer.images;
        let mut app = ready();
        quick(
            &mut app,
            Open::Editor {
                target: Target::PickSlot,
                dial: None,
                composer,
            },
        );
        let Some(Quick::Editor { dial, .. }) = menu(&app) else {
            panic!("the editor");
        };
        assert_eq!(dial.input, "");
        let note = if composer_images {
            Note::PresetImages
        } else {
            Note::PresetMentions
        };
        assert_eq!(said(&app), vec![note]);
    }
}

#[test]
fn a_saved_preset_is_recalled_as_typed_and_a_broken_one_says_so() {
    let mut app = ready();
    slots(
        &mut app,
        vec![
            saved(7, "/compact", Origin::User),
            saved(8, "!rm", Origin::User),
        ],
    );
    quick(&mut app, Open::Recall(7));
    assert_eq!(
        app.screens().select.picked,
        vec![Picked::Recall("/compact".to_owned())]
    );
    quick(&mut app, Open::Recall(9));
    quick(&mut app, Open::Recall(8));
    assert_eq!(
        app.screens().select.picked.len(),
        1,
        "a broken preset is not submitted"
    );
    assert_eq!(
        said(&app),
        vec![Note::PresetBroken {
            slot: 8,
            reason: Invalid::Shell
        }]
    );
}
