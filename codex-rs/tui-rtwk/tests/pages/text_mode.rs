// test root: pages
//! The widths the app's choices are made from are the terminal's: under
//! `TextMode::Legacy` a halfwidth katakana and its sound mark (`ｶﾞ`, two code points)
//! take two cells, under `Graphemes` one (G44). The footer rows, the warning notice and the
//! footer's pick of a spelling measure with `segmenter_of(cx)`, so the same room gives
//! the spelling the terminal's own count allows.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use rtwk::style::Style;
use rtwk::widgets::Text;
use rtwk_core::unicode::TextMode;
use rtwk_test::{Options, Tester};

use codex_tui_rtwk::widgets::page_keys::{Variant, footer_fit};
use codex_tui_rtwk::widgets::status_line::{Notice, with_warning_notice};

fn screen(mode: TextMode, width: u16, widget: impl Into<rtwk::AnyWidget>) -> String {
    let mut t = Tester::with(
        Options::new()
            .size(width, 1)
            .terminal(|terminal| terminal.text_mode = mode),
    );
    t.pump_widget(widget);
    t.screen()
}

/// A hint row of two spellings: the first (`a ｶﾞ`) takes 3 cells where a terminal
/// clusters the mark with its letter, 4 where it does not; the second (`b`) one.
fn row() -> rtwk::AnyWidget {
    let variants = vec![
        Variant {
            min_width: 0,
            entries: vec![("a".into(), "\u{ff76}\u{ff9e}".into())],
        },
        Variant {
            min_width: 0,
            entries: vec![("b".into(), String::new())],
        },
    ];
    footer_fit(variants, Style::new(), Style::new(), "  ", 0)
}

#[test]
fn the_footer_row_picks_its_spelling_by_the_terminals_count() {
    assert_eq!(screen(TextMode::Graphemes, 3, row()), "a \u{ff76}\u{ff9e}");
    assert_eq!(screen(TextMode::Legacy, 3, row()), "b");
}

fn notice() -> Notice<'static> {
    Notice {
        badge: "\u{ff76}\u{ff9e}",
        singular: "w",
        plural: "ws",
        key: "k",
        view: "v",
        accent: Style::new(),
        secondary: Style::new(),
        key_style: Style::new(),
    }
}

/// The notice's full label is 9 cells in graphemes, 10 in legacy; the room left of
/// a 10-cell footer (9) holds it only in the first.
#[test]
fn the_warning_notice_picks_its_label_by_the_terminals_count() {
    let footer = || -> rtwk::AnyWidget { Text::line("").into() };
    let graphemes = screen(
        TextMode::Graphemes,
        10,
        with_warning_notice(footer(), 1, notice()),
    );
    let legacy = screen(
        TextMode::Legacy,
        10,
        with_warning_notice(footer(), 1, notice()),
    );
    assert!(graphemes.contains("1 w"), "{graphemes:?}");
    assert!(
        !legacy.contains("1 w") && legacy.contains('k'),
        "{legacy:?}"
    );
}
