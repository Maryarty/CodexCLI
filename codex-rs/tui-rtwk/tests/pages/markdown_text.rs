#![allow(clippy::expect_used, clippy::unwrap_used)]

//! The text of an assistant message that is not mounted
//! (`transcript::item_text`, `transcript::markdown_text`): the model find,
//! select and copy read the scrolled-out items with, held to what a
//! `SelectionArea` over the mounted cell selects. A source the model answers
//! `None` for is not modelled (the item has no text there); one it answers
//! must be the selection.

use codex_tui_core::threads::Item;
use codex_tui_rtwk::pages::session::cells::agent;
use codex_tui_rtwk::pages::session::cells::strings::EN as CELLS;
use codex_tui_rtwk::pages::session::cells::text::Env;
use codex_tui_rtwk::pages::session::strings::EN as CHAT;
use codex_tui_rtwk::pages::session::transcript::item_text::item_text;
use rtwk::gestures::{SelectionArea, SelectionAreaController};
use rtwk::style::Style;
use rtwk_core::input::{Mods, MouseKind};
use rtwk_rich::markdown::MarkdownTheme;

const WIDTH: u16 = 60;
const HEIGHT: u16 = 40;
const BULLET: &str = "• ";

fn env() -> Env<'static> {
    Env {
        cells: &CELLS,
        chat: &CHAT,
        detailed: false,
        expanded: false,
    }
}

/// What a drag over the whole mounted message selects.
fn selected(source: &str) -> String {
    let theme = MarkdownTheme::default();
    let widget = agent::message(
        source,
        false,
        None,
        &agent::Look {
            bullet: BULLET,
            bullet_style: Style::new(),
            markdown: &theme,
            cells: &CELLS,
        },
    );
    let selection = SelectionAreaController::new();
    let mut t = rtwk_test::Tester::with(rtwk_test::Options::new().size(WIDTH, HEIGHT));
    t.pump_widget(
        SelectionArea::new(widget)
            .trim_trailing_blanks(false)
            .controller(selection.clone()),
    );
    t.mouse(MouseKind::Down, 0, 0, 0, Mods::NONE);
    t.mouse(MouseKind::Drag, WIDTH - 1, HEIGHT - 1, 0, Mods::NONE);
    t.mouse(MouseKind::Up, WIDTH - 1, HEIGHT - 1, 0, Mods::NONE);
    t.pump();
    selection.text()
}

fn modelled(source: &str) -> Option<String> {
    item_text(
        &Item::Agent {
            text: source.into(),
            phase: None,
        },
        &env(),
    )
}

/// Sources the model answers: each is one the selection must equal.
const CORPUS: &[&str] = &[
    "Needle one",
    "para one\n\npara two",
    "# Heading\ntext here",
    "# One\n\n## Two\n\n### Three\n\nx",
    "- a\n- b",
    "- a\n\n- b",
    "1. one\n2. two",
    "- a\n  - nested\n- b",
    "- [ ] todo\n- [x] done",
    "- first item\n\n  second paragraph of it\n- next",
    "- item\n\n  ```\n  code\n  ```\n- next",
    "- item\n  > quote\n- next",
    "1. a\n   1. b\n2. c",
    "- a\n\n  - nested\n\n- b",
    "> - a\n> - b",
    "> # head\n> text",
    "text\n\n```\ncode\n```\n\n> q",
    "**bold** and `code` and [link](http://x.y)",
    "<http://x.y>",
    "see http://x.y now",
    "![alt text](i.png)",
    "~~gone~~ and *em*",
    "```rust\nfn x() {}\n```",
    "```\na\n\nb\n```",
    "    indented code",
    "line one\nline two",
    "hard  \nbreak",
    "> quote\n> more",
    "> a\n>\n> b",
    "| a | b |\n|---|---|\n| 1 | 2 |",
    "| a | b |\n|---|---|\n| 1 | 2 |\n| 3 | 4 |",
    "| **a** | `b` |\n|---|---|\n| [x](http://y.z) | 2 |",
    "# T\n\nintro text\n\n- a\n- b\n\n```sh\nls\n```\n\nbye",
    "a long paragraph that goes on and on past the sixty columns the message is drawn in, so it wraps",
];

/// Sources the model leaves out: it must say so, not guess.
const UNMODELLED: &[&str] = &["above\n\n---\n\nbelow", "[ref]\n\n[ref]: http://x.y"];

#[test]
fn the_model_is_the_mounted_selection() {
    let mut wrong = Vec::new();
    for source in CORPUS {
        match modelled(source) {
            Some(model) => {
                let mounted = selected(source);
                if model != mounted {
                    wrong.push(format!(
                        "{source:?}\n   model   {model:?}\n   mounted {mounted:?}"
                    ));
                }
            }
            None => wrong.push(format!("{source:?}: not modelled")),
        }
    }
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

#[test]
fn what_the_model_does_not_know_it_leaves_out() {
    for source in UNMODELLED {
        assert_eq!(modelled(source), None, "{source:?}");
    }
}

#[test]
fn an_item_with_rows_of_its_own_has_no_text_yet() {
    let reasoning = Item::Reasoning {
        summary: "thinking".into(),
        with_raw: None,
    };
    assert_eq!(item_text(&reasoning, &env()), None);
}
