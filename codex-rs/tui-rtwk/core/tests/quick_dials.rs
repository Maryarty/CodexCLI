#![allow(clippy::expect_used, clippy::unwrap_used)]

//! `/presets` (oracle `chatwidget/quick_dials.rs`, `app/quick_dials.rs`,
//! `tests/quick_dials_tests.rs`): the sheet the model asks for (`Opening::Quick`, a one-shot the
//! presenter shows), what each row and each text of the editor leads to (the pure `Menu::choose`
//! and `Menu::submit`: a page over this one, or a verb for the model), and the write of a slot
//! into the config (`tui.presets.<slot>`), which changes the slot only once the config took it.
//! The pages themselves, the cursor, the editor's text and what Esc does on each page are the
//! sheet's own (`tests/pages/quick_dials.rs`).

use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Intent, apply_app};
use codex_tui_core::ids::{ThreadId, Ticket};
use codex_tui_core::rpc::{Reply, RpcError};
use codex_tui_core::screens::chat::StatusFacts;
use codex_tui_core::screens::select::models::{Current, Preset};
use codex_tui_core::screens::select::quick_dials::{
    self as quick, Action, Ask, Chosen, Composer, Dial, Gate, Invalid, Menu as Page, Open, Origin,
    PickerMode, Settings, Slot, Source, Target, Unavailable, Write,
};
use codex_tui_core::screens::select::{self, Intent as Select, Note, Opening, Picked};
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

/// What a page of the sheet asked of the model.
fn ask(app: &mut CodexApp, ask: Ask) -> AppFx {
    select(app, Select::Preset(ask))
}

/// The pages the model asked the sheet to open, oldest first, taken as the presenter takes them
/// (it says it showed each).
fn opened(app: &mut CodexApp) -> Vec<Page> {
    // Collected first: the shows borrow `app`, and `select` below needs it mutably.
    #[allow(clippy::needless_collect)]
    let shows: Vec<_> = app
        .screens()
        .select
        .shows
        .pending()
        .map(|show| (show.seq, show.opening.clone()))
        .collect();
    shows
        .into_iter()
        .map(|(seq, opening)| {
            select(app, Select::Shown(seq));
            match opening {
                Opening::Quick(page) => *page,
                other => panic!("expected the presets sheet, got {other:?}"),
            }
        })
        .collect()
}

/// The one page the model asked for.
fn page(app: &mut CodexApp) -> Page {
    let mut pages = opened(app);
    assert_eq!(pages.len(), 1, "one sheet, got {pages:?}");
    pages.remove(0)
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

/// The sources the sheet opened on, as the settings gave them.
fn sources(app: &mut CodexApp, facts: &StatusFacts) -> Vec<(Source, Result<String, Unavailable>)> {
    quick(app, Open::Prefills(settings(facts)));
    match page(app) {
        Page::Sources(rows) => rows
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

/// Where a row leads, over the slots the model holds.
fn chose(app: &CodexApp, on: &Page, id: &str) -> Option<Chosen> {
    on.choose(id, &app.screens().select.quick)
}

/// Where the editor's text leads, over the slots the model holds.
fn submitted(app: &CodexApp, on: &Page, text: &str) -> Option<Chosen> {
    on.submit(text, &app.screens().select.quick)
}

/// The page a pick or a text opens over this one.
fn pushed(chosen: Option<Chosen>) -> Page {
    match chosen {
        Some(Chosen::Push(page)) => page,
        other => panic!("expected a page over this one, got {other:?}"),
    }
}

/// What a pick or a text settles with the model.
fn asked(chosen: Option<Chosen>) -> Ask {
    match chosen {
        Some(Chosen::Ask(ask)) => ask,
        other => panic!("expected a verb for the model, got {other:?}"),
    }
}

fn dial(input: &str) -> Dial {
    Dial::new(input)
}

#[test]
fn the_current_settings_are_offered_as_model_permissions_title_and_status_layout() {
    let mut app = ready();
    catalog(&mut app);
    assert_eq!(
        sources(&mut app, &facts()),
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
    let rows = sources(&mut app, &facts);
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

    // an unavailable row leads nowhere
    quick(&mut app, Open::Prefills(settings(&facts)));
    let page = page(&mut app);
    assert_eq!(chose(&app, &page, quick::TERMINAL_TITLE), None);
    assert_eq!(chose(&app, &page, quick::MODEL), None);
    assert!(
        chose(&app, &page, quick::STATUS_LINE).is_some(),
        "an available one does"
    );

    // a model the catalog does not list, a speed it does not offer, no named profile
    let mut other = facts.clone();
    other.model = Some("unlisted".into());
    other.permission_profile = None;
    let rows = sources(&mut app, &other);
    assert_eq!(rows[0].1, Err(Unavailable::Model));
    assert_eq!(rows[1].1, Err(Unavailable::NoProfile));
    let mut speed = facts;
    speed.effort = None;
    speed.service_tier = Some("ultrafast".into());
    assert_eq!(sources(&mut app, &speed)[0].1, Err(Unavailable::Speed));
    speed.service_tier = Some("priority".into());
    assert_eq!(
        sources(&mut app, &speed)[0].1,
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
    let rows = sources(&mut app, &facts);
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
fn a_source_leads_to_the_editor_and_its_text_to_a_slot_picker_on_the_first_empty_slot() {
    let mut app = ready();
    catalog(&mut app);
    slots(
        &mut app,
        vec![
            saved(1, "/compact", Origin::User),
            saved(2, "hello", Origin::User),
        ],
    );
    quick(&mut app, Open::Prefills(settings(&facts())));
    let sources = page(&mut app);
    let editor = pushed(chose(&app, &sources, quick::PERMISSIONS));
    let Page::Editor { target, dial } = &editor else {
        panic!("expected the editor, got {editor:?}");
    };
    assert_eq!(*target, Target::PickSlot);
    assert!(dial.input.starts_with("/permissions :workspace"));

    let picker = pushed(submitted(&app, &editor, "/permissions edited"));
    assert_eq!(
        picker,
        Page::Slots {
            mode: PickerMode::Save,
            dial: Some(self::dial("/permissions edited")),
        }
    );
    let rows = picker.page(&app.screens().select.quick).expect("rows");
    assert_eq!(rows.rows.len(), 10);
    assert_eq!(rows.initial, Some(2), "the first empty slot is 3");
    assert!(rows.rows.iter().all(|row| !row.disabled));
    assert_eq!(
        rows.rows[0].saved.as_ref().map(|d| d.input.as_str()),
        Some("/compact")
    );
    // the pages that lead nowhere
    assert_eq!(chose(&app, &editor, quick::CONFIRM), None);
    assert_eq!(submitted(&app, &sources, "text"), None);
}

#[test]
fn the_editors_text_is_checked_before_a_page_opens() {
    let app = ready();
    let editor = Page::Editor {
        target: Target::PickSlot,
        dial: dial("seed"),
    };
    assert_eq!(
        submitted(&app, &editor, "   "),
        None,
        "a blank text submits nothing"
    );
    assert_eq!(
        asked(submitted(&app, &editor, "!ls")),
        Ask::Invalid(Invalid::Shell)
    );
    // whitespace the user typed stays as typed
    let picker = pushed(submitted(&app, &editor, "  two words  "));
    assert_eq!(
        picker,
        Page::Slots {
            mode: PickerMode::Save,
            dial: Some(dial("  two words  ")),
        }
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
    let editor = page(&mut app);
    let Page::Editor { dial, .. } = &editor else {
        panic!("expected the editor, got {editor:?}");
    };
    assert_eq!(dial.input, "write the tests");
    let picker = pushed(submitted(&app, &editor, "write the tests now"));
    let write = asked(chose(&app, &picker, &quick::slot_id(4)));
    assert_eq!(
        write,
        Ask::Write(Write {
            slot: 4,
            dial: Some(self::dial("write the tests now")),
        })
    );
    let fx = ask(&mut app, write);
    assert_eq!(opened(&mut app), vec![], "the sheet is gone, nothing opens");
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
    let fx = quick(&mut app, Open::Save(dial("new text")));
    assert!(fx.commands().is_empty());
    let picker = page(&mut app);
    // a saved slot leads to the confirmation, nothing is written before it
    let confirm = pushed(chose(&app, &picker, &quick::slot_id(3)));
    assert_eq!(confirm.action(), Some(Action::Replace));
    assert_eq!(
        confirm,
        Page::Confirm {
            slot: 3,
            old: Some(Dial {
                input: "old text".into(),
                label: Some(String::new())
            }),
            new: Some(dial("new text")),
        }
    );
    assert_eq!(chose(&app, &confirm, &quick::slot_id(3)), None);
    let write = asked(chose(&app, &confirm, quick::CONFIRM));
    let fx = ask(&mut app, write);
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
    let picker = page(&mut app);
    let rows = picker.page(&app.screens().select.quick).expect("rows");
    assert_eq!(
        rows.rows.iter().map(|r| r.slot).collect::<Vec<_>>(),
        vec![3]
    );
    let confirm = pushed(chose(&app, &picker, &quick::slot_id(3)));
    assert_eq!(confirm.action(), Some(Action::Delete));
    let write = asked(chose(&app, &confirm, quick::CONFIRM));
    let fx = ask(&mut app, write);
    let (ticket, slot, dial) = written(&fx);
    assert_eq!((slot, dial), (3, None));
    took(&mut app, ticket);
    assert!(app.screens().select.quick.get(3).is_none());
}

#[test]
fn the_slot_pickers_list_the_slots_their_mode_acts_on() {
    let mut app = ready();
    slots(
        &mut app,
        vec![
            saved(2, "two", Origin::User),
            saved(0, "zero", Origin::User),
        ],
    );
    let rows = |mode, dial: Option<Dial>| {
        Page::Slots { mode, dial }
            .page(&app.screens().select.quick)
            .expect("rows")
    };
    let slots_of = |page: &quick::Page| page.rows.iter().map(|r| r.slot).collect::<Vec<_>>();
    // Edit and Delete list the saved slots, in the oracle's order (1 to 9, then 0)
    for mode in [PickerMode::Edit, PickerMode::Clear] {
        let page = rows(mode, None);
        assert_eq!(slots_of(&page), vec![2, 0]);
        assert_eq!(page.initial, None);
    }
    // Save lists all ten and opens on the first empty one
    let save = rows(PickerMode::Save, Some(dial("x")));
    assert_eq!(save.rows.len(), 10);
    assert_eq!(save.initial, Some(0));
    // with nothing to save every row is disabled and leads nowhere
    let none = Page::Slots {
        mode: PickerMode::Save,
        dial: None,
    };
    let page = none.page(&app.screens().select.quick).expect("rows");
    assert!(page.rows.iter().all(|row| row.disabled));
    assert_eq!(chose(&app, &none, &quick::slot_id(4)), None);
    // Edit leads to the editor on the slot's own text, an empty slot to nothing
    let edit = Page::Slots {
        mode: PickerMode::Edit,
        dial: None,
    };
    assert_eq!(
        pushed(chose(&app, &edit, &quick::slot_id(2))),
        Page::Editor {
            target: Target::Slot(2),
            dial: Dial {
                input: "two".into(),
                label: Some(String::new()),
            },
        }
    );
    assert_eq!(chose(&app, &edit, &quick::slot_id(5)), None);
}

#[test]
fn an_edited_slot_asks_to_replace_and_a_slot_of_another_layer_is_not_written() {
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
    let picker = page(&mut app);
    let editor = pushed(chose(&app, &picker, &quick::slot_id(1)));
    let confirm = pushed(submitted(&app, &editor, "changed"));
    assert_eq!(confirm.action(), Some(Action::Replace));
    let write = asked(chose(&app, &confirm, quick::CONFIRM));
    let fx = ask(&mut app, write);
    assert!(fx.commands().is_empty(), "nothing is written");
    assert_eq!(said(&app), vec![Note::PresetOutside(1)]);
    // the editor comes back on the slot, with what was typed
    assert_eq!(
        page(&mut app),
        Page::Editor {
            target: Target::Slot(1),
            dial: Dial {
                input: "changed".into(),
                label: Some(String::new()),
            },
        }
    );

    // a shadowing user slot is replaced, but not cleared
    quick(&mut app, Open::Picker(PickerMode::Clear));
    let picker = page(&mut app);
    let confirm = pushed(chose(&app, &picker, &quick::slot_id(2)));
    let write = asked(chose(&app, &confirm, quick::CONFIRM));
    let fx = ask(&mut app, write);
    assert!(fx.commands().is_empty());
    assert_eq!(said(&app).last(), Some(&Note::PresetOutside(2)));
    assert_eq!(
        opened(&mut app),
        vec![],
        "a delete has no editor to restore"
    );
    quick(&mut app, Open::Save(dial("again")));
    let picker = page(&mut app);
    let confirm = pushed(chose(&app, &picker, &quick::slot_id(2)));
    assert_eq!(confirm.action(), Some(Action::Replace));
    let write = asked(chose(&app, &confirm, quick::CONFIRM));
    let fx = ask(&mut app, write);
    written(&fx);
}

#[test]
fn a_refused_write_keeps_the_slot_and_shows_the_editor_again() {
    let mut app = ready();
    quick(&mut app, Open::Save(dial("keep me")));
    let picker = page(&mut app);
    let write = asked(chose(&app, &picker, &quick::slot_id(5)));
    let fx = ask(&mut app, write);
    let (ticket, ..) = written(&fx);
    assert_eq!(opened(&mut app), vec![], "the sheet closed with the write");
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
    assert_eq!(
        page(&mut app),
        Page::Editor {
            target: Target::Slot(5),
            dial: dial("keep me"),
        }
    );
}

#[test]
fn an_input_that_is_not_a_preset_is_refused() {
    let mut app = ready();
    quick(&mut app, Open::Save(dial("!ls")));
    assert_eq!(opened(&mut app), vec![]);
    assert_eq!(said(&app), vec![Note::PresetInvalid(Invalid::Shell)]);
    for (text, reason) in [("   ", Invalid::Empty), ("!ls", Invalid::Shell)] {
        assert_eq!(dial(text).validate(), Err(reason));
    }
    // the editor says so for the text typed, and stays (the sheet's page is not the model's)
    ask(&mut app, Ask::Invalid(Invalid::Shell));
    assert_eq!(
        said(&app),
        vec![
            Note::PresetInvalid(Invalid::Shell),
            Note::PresetInvalid(Invalid::Shell)
        ]
    );
    assert_eq!(opened(&mut app), vec![]);
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
        assert_eq!(opened(&mut app), vec![]);
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
        let Page::Editor { dial, .. } = page(&mut app) else {
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
    assert_eq!(opened(&mut app), vec![], "a recall opens no sheet");
}
