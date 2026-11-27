// test root: pages
use crate::kit::{Owner, Screen};

#[test]
fn ambient_pet_native_animation_writes_only_while_visible_and_enabled() {
    let mut screen = Screen::build(Owner::UiPane)
        .size(80, 24)
        .kitty()
        .live()
        .open();
    screen.pets_animated_ambient();
    screen.assert_ambient_pet(Some((71, 18, 9, 5)));
    let animated = screen.ambient_frame_counts(1_000);
    println!("visible for 1s: {animated:?}");
    // GIF delays use hundredths of a second, so the 125 ms fixture ticks every 120 ms.
    assert!((8..=9).contains(&animated.0));
    assert_eq!(animated.0, animated.1);
    assert!(animated.2 > 0);

    screen.type_text("/theme").keys("enter").wait(200);
    screen.assert_ambient_pet(None);
    let hidden = screen.ambient_frame_counts(1_000);
    println!("theme modal for 1s: {hidden:?}");
    assert_eq!(hidden, (0, 0, 0));
    screen.keys("esc");
    screen.assert_ambient_pet(Some((71, 18, 9, 5)));

    screen.focus_out().wait(200);
    screen.assert_ambient_pet(None);
    let unfocused = screen.ambient_frame_counts(1_000);
    println!("terminal unfocused for 1s: {unfocused:?}");
    assert_eq!(unfocused, (0, 0, 0));
    screen.focus_in();
    screen.assert_ambient_pet(Some((71, 18, 9, 5)));

    screen.type_text("/pets disable").keys("enter");
    screen.pets_save_accepted("disabled");
    screen.wait(200);
    screen.assert_ambient_pet(None);
    let disabled = screen.ambient_frame_counts(1_000);
    println!("disabled for 1s: {disabled:?}");
    assert_eq!(disabled, (0, 0, 0));
    screen.assert_single_ambient_fetch();
}

#[test]
fn ambient_pet_static_image_reuses_host_asset_across_session_switches() {
    let mut screen = Screen::build(Owner::UiPane)
        .size(80, 24)
        .kitty()
        .live()
        .side()
        .open();
    screen.pets_ambient(None, false);
    for thread in ["side-1", "thread-1", "side-1"] {
        screen.assert_active_thread(thread);
        screen.assert_ambient_pet(Some((71, 18, 9, 5)));
        let static_pet = screen.ambient_frame_counts(1_000);
        println!("reduced motion for 1s ({thread}): {static_pet:?}");
        assert_eq!(static_pet, (0, 0, 0));
        screen.keys("ctrl+/");
    }
    screen.assert_single_ambient_fetch();
}

#[test]
fn ambient_pet_both_anchor_modes_coincide_in_fullscreen_chat() {
    for screen_bottom in [false, true] {
        let mut screen = Screen::build(Owner::UiPane)
            .size(80, 24)
            .kitty()
            .live()
            .open();
        screen.pets_ambient(None, screen_bottom);
        screen.assert_ambient_pet(Some((71, 18, 9, 5)));
        screen.feed(include_str!(
            "../oracle/fixtures/chat_status_1/running_turn.jsonl"
        ));
        screen.assert_ambient_pet(Some((71, 18, 9, 5)));
    }
}

#[test]
fn ambient_pet_unsupported_graphics_keeps_the_host_idle() {
    let mut screen = Screen::build(Owner::UiPane).size(80, 24).live().open();
    screen.assert_ambient_pet(None);
    assert!(screen.commands().into_iter().all(|command| !matches!(
        command,
        codex_tui_core::app::Command::Effect {
            ask: codex_tui_core::app::effects::Ask::Pet(
                codex_tui_core::screens::pets::AssetAsk::Ambient { .. }
            ),
            ..
        }
    )));
}

#[test]
fn ambient_pet_plays_manifest_fallback_during_running_turn() {
    let mut screen = Screen::build(Owner::UiPane)
        .size(80, 24)
        .kitty()
        .live()
        .open();
    screen.feed(include_str!(
        "../oracle/fixtures/chat_status_1/running_turn.jsonl"
    ));
    screen.pets_running_fallback();
    screen.assert_ambient_pet(Some((71, 18, 9, 5)));
    // The running clip has two 125 ms frames. The continued transmissions after 500 ms
    // come from the waiting fallback, since idle is a still image.
    screen.wait(500);
    assert!((8..=9).contains(&screen.ambient_kitty_transfers(1_000)));
    screen.focus_out().wait(200);
    screen.assert_ambient_pet(None);
    assert_eq!(screen.ambient_kitty_transfers(1_000), 0);
    screen.focus_in().wait(200);
    assert!((8..=9).contains(&screen.ambient_kitty_transfers(1_000)));
}

#[test]
fn ambient_pet_fits_short_running_chat_without_unused_label_rows() {
    for (height, top) in [(6, 0), (7, 1)] {
        let mut screen = Screen::build(Owner::UiPane)
            .size(80, height)
            .kitty()
            .live()
            .open();
        screen.pets_ambient(None, false);
        screen.feed(include_str!(
            "../oracle/fixtures/chat_status_1/running_turn.jsonl"
        ));
        screen.type_text("draft stays visible");
        screen.assert_ambient_pet(Some((71, top, 9, 5)));
        screen.assert_contains("draft stays visible");
    }
}
