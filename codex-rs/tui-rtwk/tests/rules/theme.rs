//! The theme: one rtwk `ThemeData` at the root, the skeleton's style tokens
//! taken from it, never a literal colour.

use std::rc::Rc;

use codex_tui_rtwk::app::run::{app_view, root};
use codex_tui_rtwk::app::theme::Tokens;
use rtwk::prelude::ThemeData;

use codex_tui_rtwk::app::UiApp;
use codex_tui_rtwk::services::shell::Backend;

#[test]
fn the_tokens_come_from_the_theme_data() {
    let data = Rc::new(ThemeData::DARK);
    let tokens = Tokens::new(data.clone());
    assert_eq!(tokens.accent(), data.accent);
    assert_eq!(tokens.error(), data.error);
    assert_eq!(tokens.dim(), data.dim());
}

#[test]
fn another_theme_gives_other_tokens() {
    let dark = Tokens::new(Rc::new(ThemeData::DARK));
    let light = Tokens::new(Rc::new(ThemeData::LIGHT));
    assert_ne!(dark.error(), light.error());
}

#[test]
fn the_root_carries_a_theme() {
    let app = UiApp::new(Backend::None);
    assert!(root(app_view(app)).theme.is_some());
}

// ---- the per-thread accents (oracle: `thread_color.rs`, `theme_accents`
// over the default syntax theme of `render/highlight.rs`)

use codex_tui_rtwk::app::theme::ThreadPalette;
use rtwk_core::input::Brightness;
use rtwk_core::style::Color;

/// The fourteen accents the oracle derives from catppuccin-mocha (the
/// default theme on a dark terminal), in the order `theme_accents` finds them;
/// computed once with the oracle's own `theme_accents` over two-face 0.5.1.
const MOCHA: [(u8, u8, u8); 14] = [
    (166, 227, 161),
    (245, 194, 231),
    (250, 179, 135),
    (203, 166, 247),
    (148, 226, 213),
    (137, 180, 250),
    (249, 226, 175),
    (235, 160, 172),
    (243, 139, 168),
    (137, 220, 235),
    (242, 205, 205),
    (116, 199, 236),
    (180, 190, 254),
    (245, 224, 220),
];

/// The same from catppuccin-latte (the default theme on a light terminal).
const LATTE: [(u8, u8, u8); 14] = [
    (64, 160, 43),
    (234, 118, 203),
    (254, 100, 11),
    (136, 57, 239),
    (23, 146, 153),
    (30, 102, 245),
    (223, 142, 29),
    (230, 69, 83),
    (210, 15, 57),
    (4, 165, 229),
    (221, 120, 120),
    (32, 159, 181),
    (114, 135, 253),
    (220, 138, 120),
];

fn colors(table: [(u8, u8, u8); 14]) -> Vec<Color> {
    table.iter().map(|&(r, g, b)| Color::rgb(r, g, b)).collect()
}

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

#[test]
fn the_thread_palette_is_the_oracles_default_accents() {
    assert_eq!(
        ThreadPalette::for_brightness(Brightness::Dark).colors(),
        colors(MOCHA)
    );
    assert_eq!(
        ThreadPalette::for_brightness(Brightness::Light).colors(),
        colors(LATTE)
    );
}

/// The palette is read from the theme the view is under: its brightness
/// picks the oracle's adaptive default (mocha on dark, latte on light).
#[test]
fn the_palette_follows_the_theme() {
    assert_eq!(ThreadPalette::of(&ThemeData::DARK).colors(), colors(MOCHA));
    assert_eq!(ThreadPalette::of(&ThemeData::LIGHT).colors(), colors(LATTE));
}

/// Id 42 is the snapshot's `Rgb(242, 205, 205)`; the others are what the
/// oracle's `color_for_id` gives (FNV-1a of the id text over the accents).
#[test]
fn a_thread_gets_the_colour_the_oracle_gives_it() {
    let dark = ThreadPalette::for_brightness(Brightness::Dark);
    let light = ThreadPalette::for_brightness(Brightness::Light);
    let slots = [
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
    for (value, slot) in slots {
        assert_eq!(dark.color(&id(value)), Some(colors(MOCHA)[slot]), "{value}");
        assert_eq!(
            light.color(&id(value)),
            Some(colors(LATTE)[slot]),
            "{value}"
        );
    }
    assert_eq!(dark.color(&id(42)), Some(Color::rgb(242, 205, 205)));
}
