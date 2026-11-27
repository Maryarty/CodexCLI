#![allow(clippy::expect_used, clippy::unwrap_used)]

//! State tests of S24 (the `/pets` picker): `pets/picker.rs` tests. The pick is a write of the
//! config, `settings::Edit::PetSelected`: its tests are in `settings.rs`.

use codex_tui_core::app::{AppEv, Intent};
use codex_tui_core::fixtures::sandbox::Sandbox;
use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::pets::{DISABLED, Intent as PetsIntent, Picker, Preview};
use pretty_assertions::assert_eq;

const PLAIN: &str = include_str!("../../src/sheets/pets/fixtures/picker.jsonl");
const CUSTOM: &str = include_str!("../../src/sheets/pets/fixtures/picker_custom.jsonl");
const DISABLED_FIXTURE: &str = include_str!("../../src/sheets/pets/fixtures/picker_disabled.jsonl");

fn send(sandbox: &mut Sandbox, intent: PetsIntent) {
    sandbox.send(AppEv::Intent(Intent::Screen(ScreenIntent::Pets(intent))));
}

fn picker(sandbox: &Sandbox) -> Picker {
    sandbox
        .app()
        .screens()
        .pets
        .catalog
        .clone()
        .expect("the picker is open")
}

fn selectors(picker: &Picker) -> Vec<&str> {
    picker
        .pets
        .iter()
        .map(|pet| pet.selector.as_str())
        .collect()
}

#[test]
fn picker_lists_app_bundled_and_custom_pets() {
    let sandbox = Sandbox::play(CUSTOM).expect("plays");
    let picker = picker(&sandbox);
    // Disabled first, then by display name: BSOD, Chefito, Codex, ...
    assert_eq!(
        selectors(&picker),
        vec![
            DISABLED,
            "bsod",
            "custom:chefito",
            "codex",
            "dewey",
            "fireball",
            "null-signal",
            "rocky",
            "seedy",
            "stacky"
        ]
    );
    assert_eq!(picker.initial(), 2);
    assert_eq!(picker.pets[2].name, "Chefito");
}

#[test]
fn picker_preselects_codex_without_marking_it_current_when_no_pet_is_configured() {
    let sandbox = Sandbox::play(PLAIN).expect("plays");
    let picker = picker(&sandbox);
    assert_eq!(picker.initial(), 2);
    assert_eq!(picker.pets[2].name, "Codex");
    assert!(!picker.is_current(&picker.pets[2]));
}

#[test]
fn picker_marks_disabled_pet_as_current() {
    let sandbox = Sandbox::play(DISABLED_FIXTURE).expect("plays");
    let picker = picker(&sandbox);
    assert_eq!(picker.initial(), 0);
    assert_eq!(picker.pets[0].selector, DISABLED);
    assert_eq!(picker.pets[0].description, None);
    assert!(picker.is_current(&picker.pets[0]));
}

#[test]
fn a_custom_pet_is_current_through_its_legacy_selector() {
    let script = CUSTOM.replace("\"current\":\"custom:chefito\"", "\"current\":\"chefito\"");
    let sandbox = Sandbox::play(&script).expect("plays");
    let picker = picker(&sandbox);
    assert_eq!(picker.initial(), 2);
    assert!(picker.is_current(&picker.pets[2]));
}

#[test]
fn the_preview_of_a_row_is_requested_once_until_another_row_is_asked_for() {
    let mut sandbox = Sandbox::play(PLAIN).expect("plays");
    // Listing the pets asks for nothing: the sheet asks for the row it opens on.
    assert!(sandbox.app().screens().pets.previews.is_empty());
    send(&mut sandbox, PetsIntent::LoadPreview("codex".into()));
    assert_eq!(sandbox.app().screens().pets.previews, vec!["codex"]);
    assert_eq!(
        picker(&sandbox).preview,
        Some(("codex".to_string(), Preview::Loading))
    );
    // The cursor is the widget's: it asks for the preview of the row it is on.
    send(&mut sandbox, PetsIntent::LoadPreview("dewey".into()));
    assert_eq!(
        sandbox.app().screens().pets.previews,
        vec!["codex", "dewey"]
    );
    send(&mut sandbox, PetsIntent::LoadPreview("dewey".into()));
    assert_eq!(
        sandbox.app().screens().pets.previews,
        vec!["codex", "dewey"]
    );
    send(&mut sandbox, PetsIntent::LoadPreview("bsod".into()));
    assert_eq!(
        sandbox.app().screens().pets.previews,
        vec!["codex", "dewey", "bsod"]
    );
}

#[test]
fn the_preview_becomes_ready_only_for_the_row_it_was_asked_for() {
    use codex_tui_core::screens::ScreenEv;
    use codex_tui_core::screens::pets::Ev;
    let mut sandbox = Sandbox::play(PLAIN).expect("plays");
    send(&mut sandbox, PetsIntent::LoadPreview("dewey".into()));
    sandbox.send(AppEv::Screen(ScreenEv::Pets(Ev::PreviewReady(
        "codex".into(),
    ))));
    assert_eq!(
        picker(&sandbox).preview,
        Some(("dewey".to_string(), Preview::Loading))
    );
    sandbox.send(AppEv::Screen(ScreenEv::Pets(Ev::PreviewReady(
        "dewey".into(),
    ))));
    assert_eq!(
        picker(&sandbox).preview,
        Some(("dewey".to_string(), Preview::Ready))
    );
}

mod live {
    use codex_tui_core::app::effects::{Ask, Done, ImagePreview, ImageSize};
    use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Intent, apply_app};
    use codex_tui_core::features::pets::{Intent as PetIntent, Phase, Visit};
    use codex_tui_core::screens::pets::{
        AssetAsk, AssetDone, DISABLED, Pet, Picker, Preview, Unavailable,
    };

    fn run(app: &mut CodexApp, event: AppEv) -> AppFx {
        let mut out = AppFx::default();
        apply_app(app, event, &mut out);
        out
    }

    fn act(app: &mut CodexApp, intent: PetIntent) -> AppFx {
        run(app, AppEv::Intent(Intent::Feature(intent.into())))
    }

    fn answer(app: &mut CodexApp, request: u64, done: AssetDone) -> AppFx {
        run(
            app,
            AppEv::Effect {
                request,
                done: Done::Pet(done),
            },
        )
    }

    fn asked(out: &AppFx, wanted: AssetAsk) -> u64 {
        out.commands()
            .iter()
            .find_map(|command| match command {
                Command::Effect {
                    request,
                    ask: Ask::Pet(ask),
                    ..
                } if ask == &wanted => Some(*request),
                _ => None,
            })
            .unwrap_or_else(|| panic!("missing {wanted:?} in {:?}", out.commands()))
    }

    fn no_pet_ask(out: &AppFx) {
        assert!(
            !out.commands().iter().any(|command| matches!(
                command,
                Command::Effect {
                    ask: Ask::Pet(_),
                    ..
                }
            )),
            "{:?}",
            out.commands()
        );
    }

    fn candidate(selector: &str) -> Pet {
        Pet {
            selector: selector.into(),
            name: selector.into(),
            description: None,
            legacy_selector: None,
        }
    }

    fn list(current: Option<&str>) -> AssetDone {
        AssetDone::Catalog(Ok(Picker::build(
            current,
            vec![candidate("codex"), candidate("dewey")],
        )))
    }

    fn request(app: &mut CodexApp, visit: Visit, selector: Option<&str>, graphics: bool) -> AppFx {
        act(
            app,
            PetIntent::Request {
                visit,
                selector: selector.map(str::to_owned),
                graphics,
            },
        )
    }

    fn ready(app: &CodexApp, visit: Visit) -> &Picker {
        let Some(Phase::Ready(picker)) = app.features().pets.phase(visit) else {
            panic!("expected ready picker");
        };
        picker
    }

    fn open(app: &mut CodexApp, current: Option<&str>) -> Visit {
        let visit = Visit::fresh();
        let req = asked(
            &request(app, visit, None, true),
            AssetAsk::Catalog { graphics: true },
        );
        no_pet_ask(&answer(app, req, list(current)));
        visit
    }

    fn preview(app: &mut CodexApp, visit: Visit, selector: &str) -> u64 {
        asked(
            &act(
                app,
                PetIntent::ReadPreview {
                    visit,
                    selector: Some(selector.into()),
                },
            ),
            AssetAsk::Preview {
                selector: selector.into(),
            },
        )
    }

    fn image(marker: u8) -> AssetDone {
        AssetDone::Preview(Ok(ImagePreview {
            png: vec![marker].into(),
            size: ImageSize {
                width: 1,
                height: 1,
            },
        }))
    }

    #[test]
    fn configured_path_preview_is_separate_from_catalog_rows_and_visit_owned() {
        let mut app = CodexApp::default();
        let path = "./my pets/owl/pet.json";
        let visit = open(&mut app, Some(path));
        assert_eq!(ready(&app, visit).initial(), 0);
        assert!(
            !ready(&app, visit)
                .pets
                .iter()
                .any(|pet| pet.selector == path)
        );
        for selector in ["./other/pet.json", "unknown"] {
            no_pet_ask(&act(
                &mut app,
                PetIntent::ReadPreview {
                    visit,
                    selector: Some(selector.into()),
                },
            ));
        }
        let old = preview(&mut app, visit, path);
        no_pet_ask(&act(
            &mut app,
            PetIntent::ReadPreview {
                visit,
                selector: Some("./other/pet.json".into()),
            },
        ));
        assert_eq!(
            ready(&app, visit).preview,
            Some((path.into(), Preview::Loading))
        );
        let row = preview(&mut app, visit, "codex");
        answer(&mut app, old, image(1));
        assert_eq!(
            ready(&app, visit).preview,
            Some(("codex".into(), Preview::Loading))
        );
        answer(&mut app, row, image(2));
        no_pet_ask(&act(&mut app, PetIntent::RetirePreview(visit)));
        let reopened = open(&mut app, Some(path));
        no_pet_ask(&act(
            &mut app,
            PetIntent::ReadPreview {
                visit,
                selector: Some(path.into()),
            },
        ));
        assert_eq!(ready(&app, reopened).preview, None);
        preview(&mut app, reopened, path);
    }

    #[test]
    fn an_explicit_path_is_saved_only_after_asset_validation() {
        let path = "./my pets/owl/pet.json";
        let mut app = CodexApp::default();
        for (result, valid) in [
            (
                AssetDone::Preview(Err("animation idle references sprite index 999".into())),
                false,
            ),
            (image(2), true),
        ] {
            let visit = Visit::fresh();
            let catalog = asked(
                &request(&mut app, visit, Some(path), true),
                AssetAsk::Catalog { graphics: true },
            );
            let validation = asked(
                &answer(&mut app, catalog, list(None)),
                AssetAsk::Preview {
                    selector: path.into(),
                },
            );
            let out = answer(&mut app, validation, result);
            assert_eq!(app.features().pets.selected(), None);
            if valid {
                let write = asked(
                    &out,
                    AssetAsk::Save {
                        selector: path.into(),
                    },
                );
                answer(&mut app, write, AssetDone::Saved(Ok(())));
                assert_eq!(app.features().pets.selected(), Some(path));
            } else {
                no_pet_ask(&out);
                assert_eq!(app.features().pets.phase(visit), Some(&Phase::Finished));
            }
        }
    }

    #[test]
    fn a_late_boot_asset_cannot_reactivate_a_disabled_pet() {
        use std::sync::Arc;

        use codex_tui_core::screens::pets::{Ambient, Anchor, Clip};

        let mut app = CodexApp::default();
        let boot = asked(
            &act(&mut app, PetIntent::FetchAmbient),
            AssetAsk::Ambient { selector: None },
        );
        no_pet_ask(&act(&mut app, PetIntent::FetchAmbient));

        let disable = asked(
            &request(&mut app, Visit::fresh(), Some("off"), true),
            AssetAsk::Save {
                selector: DISABLED.into(),
            },
        );
        no_pet_ask(&answer(&mut app, disable, AssetDone::Saved(Ok(()))));
        let asset = Ambient {
            size: ImageSize {
                width: 192,
                height: 208,
            },
            anchor: Anchor::Composer,
            clips: std::array::from_fn(|_| Clip {
                data: Arc::new([]),
                after: None,
            }),
        };
        answer(&mut app, boot, AssetDone::Ambient(Some(Arc::new(asset))));
        assert!(app.features().pets.ambient().is_none());
        assert_eq!(app.features().pets.selected(), Some(DISABLED));
    }

    #[test]
    fn preview_results_cannot_fill_a_newer_visit_or_a_repeated_selector() {
        let mut app = CodexApp::default();
        let visit = open(&mut app, None);
        let first_a = preview(&mut app, visit, "codex");
        let b = preview(&mut app, visit, "dewey");
        let next_a = preview(&mut app, visit, "codex");
        answer(&mut app, first_a, image(1));
        answer(&mut app, b, image(2));
        assert_eq!(
            ready(&app, visit).preview,
            Some(("codex".into(), Preview::Loading))
        );
        answer(&mut app, next_a, image(3));
        let Some((_, Preview::Image(actual))) = &ready(&app, visit).preview else {
            panic!("owned preview should render");
        };
        assert_eq!(&*actual.png, &[3]);
        let old = preview(&mut app, visit, "dewey");
        act(&mut app, PetIntent::RetirePreview(visit));
        let reopened = open(&mut app, None);
        answer(&mut app, old, image(4));
        assert_eq!(ready(&app, reopened).preview, None);
    }

    #[test]
    fn a_named_pet_is_written_only_after_support_and_owned_asset_validation() {
        let mut app = CodexApp::default();
        let old = Visit::fresh();
        let host = asked(
            &request(&mut app, old, Some("dewey"), true),
            AssetAsk::Catalog { graphics: true },
        );
        let asset = asked(
            &answer(&mut app, host, list(None)),
            AssetAsk::Preview {
                selector: "dewey".into(),
            },
        );
        act(&mut app, PetIntent::RetirePreview(old));
        no_pet_ask(&answer(&mut app, asset, image(1)));
        assert_eq!(app.features().pets.selected(), None);

        let denied = Visit::fresh();
        let host = asked(
            &request(&mut app, denied, Some("codex"), false),
            AssetAsk::Catalog { graphics: false },
        );
        no_pet_ask(&answer(
            &mut app,
            host,
            AssetDone::Catalog(Err(Unavailable::Tmux)),
        ));
        assert_eq!(
            app.features().pets.phase(denied),
            Some(&Phase::Unavailable(Unavailable::Tmux))
        );
    }

    #[test]
    fn config_ack_controls_the_current_pet_when_boot_snapshot_is_stale() {
        let mut app = CodexApp::default();
        let visit = open(&mut app, Some("dewey"));
        let asset = asked(
            &act(
                &mut app,
                PetIntent::Apply {
                    visit,
                    selector: "codex".into(),
                },
            ),
            AssetAsk::Preview {
                selector: "codex".into(),
            },
        );
        let write = asked(
            &answer(&mut app, asset, image(1)),
            AssetAsk::Save {
                selector: "codex".into(),
            },
        );
        assert_eq!(app.features().pets.selected(), Some("dewey"));
        answer(&mut app, write, AssetDone::Saved(Err("denied".into())));
        let second = open(&mut app, Some("dewey"));
        assert_eq!(ready(&app, second).current.as_deref(), Some("dewey"));

        let disabled = Visit::fresh();
        let out = request(&mut app, disabled, Some("hide"), false);
        let write = asked(
            &out,
            AssetAsk::Save {
                selector: DISABLED.into(),
            },
        );
        assert_eq!(out.commands().len(), 1);
        answer(&mut app, write, AssetDone::Saved(Ok(())));
        let third = open(&mut app, Some("dewey"));
        assert_eq!(ready(&app, third).current.as_deref(), Some(DISABLED));
    }
}

#[test]
fn typing_filters_the_pets_and_the_disable_row_answers_to_its_words() {
    let sandbox = Sandbox::play(PLAIN).expect("plays");
    // The search text is the widget's; the model filters by it on request.
    let picker_now = picker(&sandbox);
    assert_eq!(picker_now.visible("dew").len(), 1);
    assert_eq!(
        picker_now.pets[picker_now.visible("dew")[0]].selector,
        "dewey"
    );
    assert_eq!(
        picker_now.pets[picker_now.visible("hidden")[0]].selector,
        DISABLED
    );
    assert_eq!(picker_now.visible("").len(), picker_now.pets.len());
}

#[test]
fn leaving_the_picker_releases_its_preview_without_a_choice() {
    // Esc clears a typed search first and closes the sheet: those are the widget's; the
    // model hears only that the preview is not wanted.
    let mut sandbox = Sandbox::play(PLAIN).expect("plays");
    send(&mut sandbox, PetsIntent::LoadPreview("codex".into()));
    send(&mut sandbox, PetsIntent::ReleasePreview);
    assert_eq!(picker(&sandbox).preview, None);
    send(&mut sandbox, PetsIntent::LoadPreview("codex".into()));
    assert_eq!(
        sandbox.app().screens().pets.previews,
        vec!["codex", "codex"]
    );
}

#[test]
fn bad_seeds_are_refused() {
    assert!(Sandbox::play(r#"{"step":"screen","dir":"pets","ev":{"kind":"nope"}}"#).is_err());
}
