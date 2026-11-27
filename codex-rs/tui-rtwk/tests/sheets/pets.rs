//! Screen tests of S24 (the `/pets` picker). Expected text: the oracle's
//! snapshot (`dialogs::oracle`), `›` marker kept. Oracle tests:
//! `chatwidget/tests/slash_commands.rs` (`slash_pets_picker`),
//! `pets/picker.rs` tests.

use crate::common::{Harness, ms, start_driven_with, start_sized};
use crate::dialogs::oracle::assert_rows;
use codex_tui_core::app::effects::{Ask, Done, ImagePreview, ImageSize};
use codex_tui_core::app::{AppEv, Command};
use codex_tui_core::screens::pets::{AssetAsk, AssetDone, Pet, Picker};
use rtwk_core::graphics::GraphicsProtocol;
use rtwk_test::Options;

fn open(entry: &str) -> Harness {
    let mut h = start_sized(true, 80, 24);
    h.open_debug_row("pets", Some(entry));
    h.t.pump_for(ms(800));
    h
}

// chatwidget/tests/slash_commands.rs: slash_pets_picker
#[test]
fn slash_pets_picker() {
    let h = open("picker");
    assert_rows(
        &h.t.screen(),
        "chatwidget/snapshots/codex_tui__chatwidget__tests__slash_pets_picker.snap",
    );
}

#[test]
fn the_disabled_row_is_marked_current() {
    let h = open("picker_disabled");
    assert!(h.t.screen().contains("(current)"), "{}", h.t.screen());
}

#[test]
fn typing_filters_the_list() {
    let mut h = open("picker");
    for key in "dew".chars() {
        h.press(&key.to_string());
    }
    let screen = h.t.screen();
    assert!(screen.contains("Dewey"), "{screen}");
    assert!(!screen.contains("Fireball"), "{screen}");
}

#[test]
fn enter_picks_the_pet_and_closes() {
    let mut h = open("picker");
    h.press("enter");
    h.t.pump_for(ms(800));
    assert!(!h.t.screen().contains("Select Pet"), "{}", h.t.screen());
}

/// `/pets` through the real opener with the local host's capabilities answered by the fake.
fn pets_command(h: &mut Harness, configured: Option<&str>) {
    let pet = |selector: &str, name: &str| Pet {
        selector: selector.into(),
        name: name.into(),
        description: None,
        legacy_selector: None,
    };
    h.settle();
    let before = h.fake.commands().len();
    h.t.type_text("/pets");
    h.press("enter");
    h.wait_until("a fresh catalog request", |h| {
        h.fake.commands().iter().skip(before).any(|command| {
            matches!(
                command,
                Command::Effect {
                    ask: Ask::Pet(AssetAsk::Catalog { graphics: true }),
                    ..
                }
            )
        })
    });
    answer(
        h,
        AssetAsk::Catalog { graphics: true },
        AssetDone::Catalog(Ok(Picker::build(
            configured,
            vec![pet("codex", "Codex"), pet("dewey", "Dewey")],
        ))),
    );
    h.wait_for("the picker", |screen| screen.contains("Loading preview..."));
}

fn live(configured: Option<&str>) -> Harness {
    let options = Options::new()
        .size(80, 24)
        .graphics(GraphicsProtocol::Kitty)
        .cell_pixels(8, 16);
    let mut h = start_driven_with(false, options);
    h.wait_idle();
    pets_command(&mut h, configured);
    h
}

fn answer(h: &mut Harness, wanted: AssetAsk, done: AssetDone) {
    let mut held = None;
    h.wait_until("the pet capability request", |h| {
        held = h
            .fake
            .commands()
            .into_iter()
            .rev()
            .find_map(|command| match command {
                Command::Effect {
                    request,
                    ask: Ask::Pet(ask),
                    ..
                } if ask == wanted => Some(request),
                _ => None,
            });
        held.is_some()
    });
    let Some(request) = held else {
        panic!("pet request was not captured")
    };
    h.inject([AppEv::Effect {
        request,
        done: Done::Pet(done),
    }]);
}

fn native_preview() -> ImagePreview {
    let mut bytes = std::io::Cursor::new(Vec::new());
    let image = image::RgbaImage::from_pixel(192, 208, image::Rgba([103, 177, 54, 255]));
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap_or_else(|error| panic!("encode native pet preview: {error}"));
    ImagePreview {
        png: bytes.into_inner().into(),
        size: ImageSize {
            width: 192,
            height: 208,
        },
    }
}

#[test]
fn configured_path_shows_a_native_preview_without_changing_catalog_rows() {
    let path = "./my pets/owl/pet.json";
    let mut h = live(Some(path));
    answer(
        &mut h,
        AssetAsk::Preview {
            selector: path.into(),
        },
        AssetDone::Preview(Ok(native_preview())),
    );
    h.wait_until("native preview of configured path", |h| {
        !h.t.images().is_empty()
    });
    let screen = h.t.screen();
    assert!(screen.contains("Disable terminal pets"));
    assert!(screen.contains("Codex"));
    assert!(!screen.contains("(current)"));
    assert!(!screen.contains(path));

    h.press("down");
    answer(
        &mut h,
        AssetAsk::Preview {
            selector: "codex".into(),
        },
        AssetDone::Preview(Ok(native_preview())),
    );
    h.wait_until("native preview of catalog row", |h| {
        !h.t.images().is_empty()
    });
    h.press("esc");
    assert!(h.fake.commands().into_iter().all(|command| !matches!(
        command,
        Command::Effect {
            ask: Ask::Pet(AssetAsk::Save { .. }),
            ..
        }
    )));
}

/// The pick is saved: Enter writes `tui.pet` through the host (the oracle's `PetSelected`).
#[test]
fn enter_writes_the_pick_to_the_config() {
    let mut h = live(None);
    h.press("down");
    answer(
        &mut h,
        AssetAsk::Preview {
            selector: "dewey".into(),
        },
        AssetDone::Preview(Ok(native_preview())),
    );
    h.wait_until("native pet preview", |h| !h.t.images().is_empty());
    let images = h.t.images();
    assert_eq!(images.len(), 1);
    assert!(images[0].rows <= 4);
    h.press("enter");
    answer(
        &mut h,
        AssetAsk::Save {
            selector: "dewey".into(),
        },
        AssetDone::Saved(Ok(())),
    );
    h.wait_for("closed picker", |screen| !screen.contains("Loading Pet"));
    assert!(h.t.images().is_empty());

    h.advance(700);
    pets_command(&mut h, None);
    h.wait_for("acknowledged current pet", |screen| {
        screen.contains("Dewey (current)")
    });
}

/// The disable row is saved as the selector the oracle's `PetDisabled` writes.
#[test]
fn enter_on_the_disable_row_writes_the_disabled_pet() {
    let mut h = live(None);
    h.press("up");
    h.wait_for("disabled preview", |screen| {
        screen.contains("Terminal pets disabled")
    });
    h.press("enter");
    answer(
        &mut h,
        AssetAsk::Save {
            selector: "disabled".into(),
        },
        AssetDone::Saved(Ok(())),
    );
    h.wait_for("closed picker", |screen| !screen.contains("Loading Pet"));
}

#[test]
fn the_arrows_move_the_cursor_with_the_search_field_focused() {
    let mut h = open("picker");
    h.press("down");
    assert!(h.t.screen().contains("› Dewey"), "{}", h.t.screen());
    h.press("up");
    h.press("up");
    assert!(h.t.screen().contains("› BSOD"), "{}", h.t.screen());
}

#[test]
fn esc_clears_the_search_then_leaves() {
    let mut h = open("picker");
    for key in "dew".chars() {
        h.press(&key.to_string());
    }
    h.press("esc");
    let screen = h.t.screen();
    assert!(screen.contains("Fireball"), "{screen}");
    h.press("esc");
    h.t.pump_for(ms(800));
    assert!(!h.t.screen().contains("Select Pet"), "{}", h.t.screen());
}

/// The oracle scrolls the window to the cursor row (`ScrollState`), also when
/// the search field has the focus and the arrows are the screen's keys.
/// The list is a `ListView` with a `ListController` (G35 not a gap): the
/// screen's key moves the cursor and calls `ensure_index_visible`.
#[test]
fn the_cursor_row_stays_in_the_window_when_the_arrows_pass_its_end() {
    let mut h = open("picker");
    // The picker opens on Codex; six downs reach the ninth row, past the window of 8.
    for _ in 0..6 {
        h.press("down");
    }
    let screen = h.t.screen();
    let cursor = screen.lines().filter(|l| l.contains('›')).count();
    assert_eq!(cursor, 1, "{screen}");
}

/// ADR-011 (beyond the oracle): a click on a pet puts the cursor there, a
/// click on the pet the cursor is on picks it and closes.
#[test]
fn a_click_selects_a_pet_and_a_second_click_picks_it() {
    let mut h = open("picker");
    let screen = h.t.screen();
    let (y, line) = screen
        .lines()
        .enumerate()
        .find(|(_, line)| line.contains("Dewey"))
        .expect("Dewey is listed");
    let x = u16::try_from(line.find("Dewey").unwrap()).unwrap();
    let y = u16::try_from(y).unwrap();
    h.t.click(x, y);
    h.settle();
    assert!(h.t.screen().contains("› Dewey"), "{}", h.t.screen());
    h.t.click(x, y);
    h.settle();
    h.t.pump_for(ms(800));
    assert!(!h.t.screen().contains("Select Pet"), "{}", h.t.screen());
}
