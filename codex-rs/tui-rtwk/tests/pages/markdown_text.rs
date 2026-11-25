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
use codex_tui_rtwk::pages::session::transcript::item_text::{Reading, item_text, mounted_text};
use rtwk::AnyWidget;
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
    selection_of(agent::message(
        source,
        false,
        None,
        &agent::Look {
            bullet: BULLET,
            bullet_style: Style::new(),
            markdown: &theme,
            cells: &CELLS,
        },
    ))
}

/// What a drag over the whole mounted `widget` selects.
fn selection_of(widget: AnyWidget) -> String {
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
        &Reading::new(&env(), BULLET),
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
    "[ref]\n\n[ref]: http://x.y",
    "a long paragraph that goes on and on past the sixty columns the message is drawn in, so it wraps",
];

/// Sources the model leaves out: it must say so, not guess.
const UNMODELLED: &[&str] = &["above\n\n---\n\nbelow"];

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

/// The texts of the `Text`s of `widget` as the list's selection reads a mounted row: the
/// selectables below the row, a gutter in its `ExcludeSelection` too, in the order of the tree.
fn nodes_of(widget: AnyWidget) -> Vec<String> {
    fn walk(tree: &rtwk_render::RenderTree, id: rtwk_render::RenderId, out: &mut Vec<String>) {
        if let Some(selectable) = tree.selectable(id) {
            out.push(selectable.selectable_text());
        } else {
            for child in tree.children(id) {
                walk(tree, child, out);
            }
        }
    }
    let mut t = rtwk_test::Tester::with(rtwk_test::Options::new().size(WIDTH, HEIGHT));
    t.pump_widget(widget);
    let tree = t.tree().expect("the app runs").render_tree();
    let mut out = Vec::new();
    if let Some(root) = tree.root() {
        walk(tree, root, &mut out);
    }
    out
}

/// A range of a row is a node and an offset in the node's text: the model of a row the list's
/// selection maps through is the mounted `Text`s joined by one line break, whatever lies between
/// two blocks on the screen (that is in a copy, which the area derives from the layout).
#[test]
fn the_row_model_is_the_mounted_nodes_joined_by_a_line_break() {
    let theme = MarkdownTheme::default();
    let mut wrong = Vec::new();
    for source in CORPUS {
        let item = Item::Agent {
            text: (*source).into(),
            phase: None,
        };
        let model = mounted_text(&item, &Reading::new(&env(), BULLET));
        let mounted = nodes_of(agent::message(
            source,
            false,
            None,
            &agent::Look {
                bullet: BULLET,
                bullet_style: Style::new(),
                markdown: &theme,
                cells: &CELLS,
            },
        ))
        .join("\n");
        if model.as_deref() != Some(mounted.as_str()) {
            wrong.push(format!(
                "{source:?}\n   model   {model:?}\n   mounted {mounted:?}"
            ));
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

/// The Markdown cells besides the assistant's message: the model is what a selection over the
/// mounted cell gives.
mod others {
    use std::rc::Rc;
    use std::sync::Arc;

    use codex_tui_core::screens::cells::SessionInfo;
    use codex_tui_core::threads::{
        AgentRef, CollabEvent, DelegatedMessage, Item, Notice, PlanStep, PlanUpdate, SpawnRequest,
        StepStatus,
    };
    use codex_tui_rtwk::pages::session::cells::text::Env;
    use codex_tui_rtwk::pages::session::cells::{
        collab, delegated, plan, plan_update, reasoning, session_info,
    };
    use codex_tui_rtwk::pages::session::transcript::item_text::{Reading, item_text};
    use rtwk_rich::markdown::MarkdownTheme;

    use super::{BULLET, CELLS, env, selection_of};

    fn tokens() -> codex_tui_rtwk::app::theme::Tokens {
        codex_tui_rtwk::app::theme::Tokens::new(Rc::new(rtwk::prelude::ThemeData::DARK))
    }

    #[track_caller]
    fn agrees(item: &Item, mounted: rtwk::AnyWidget) {
        let model = item_text(item, &Reading::new(&env(), BULLET));
        assert_eq!(model, Some(selection_of(mounted)), "{item:?}");
    }

    #[test]
    fn a_reasoning_summary_is_its_markdown() {
        for summary in [
            "thinking",
            "**Plan**\n\nfirst\n\nsecond",
            "# Head\n- a\n- b",
        ] {
            let item = Item::Reasoning {
                summary: summary.into(),
                with_raw: None,
            };
            agrees(
                &item,
                reasoning::ReasoningCell {
                    summary: summary.into(),
                }
                .into(),
            );
        }
    }

    /// A summary with nothing to say draws nothing: the item has no text.
    #[test]
    fn a_blank_reasoning_summary_has_no_text() {
        let item = Item::Reasoning {
            summary: "  ".into(),
            with_raw: None,
        };
        assert_eq!(item_text(&item, &Reading::new(&env(), BULLET)), None);
    }

    #[test]
    fn a_delegated_message_is_its_attribution_and_markdown() {
        let message = Arc::new(DelegatedMessage {
            source: "abc".into(),
            markdown: "do **this**\n\nthen that".into(),
        });
        agrees(
            &Item::Delegated(message.clone()),
            delegated::DelegatedCell { message }.into(),
        );
    }

    #[test]
    fn a_proposed_plan_is_its_header_a_blank_and_its_markdown() {
        let theme = MarkdownTheme::default();
        for text in ["- step one\n- step two", "", "# Plan\n\nwords"] {
            agrees(
                &Item::Plan { text: text.into() },
                plan::proposed_plan(
                    text,
                    None,
                    rtwk::style::Style::new(),
                    &theme,
                    &tokens(),
                    &CELLS,
                ),
            );
        }
    }

    #[test]
    fn the_session_card_with_a_tip_is_the_tip_and_the_model_note() {
        for model_changed in [None, Some(("gpt-a".to_owned(), "gpt-b".to_owned()))] {
            let info = SessionInfo {
                help: false,
                tip: Some("press `?` for help".into()),
                model_changed,
            };
            agrees(
                &Item::Notice(Notice::SessionInfo(info.clone())),
                session_info::cell(&info, MarkdownTheme::default(), &tokens(), &CELLS),
            );
        }
    }

    fn step(text: &str, status: StepStatus) -> PlanStep {
        PlanStep {
            text: text.into(),
            status,
        }
    }

    /// The detailed transcript reveals the note and every step in plan order; the compact cell
    /// lists the running step first. Both are what the area selects (the room is wide).
    #[test]
    fn a_plan_update_is_its_rows_revealed_or_compact() {
        let update = Arc::new(PlanUpdate {
            explanation: Some("why".into()),
            steps: vec![
                step("done one", StepStatus::Completed),
                step("doing two", StepStatus::InProgress),
                step("later three", StepStatus::Pending),
            ],
        });
        let item = Item::PlanUpdate(update.clone());
        let detailed = Env {
            detailed: true,
            ..env()
        };
        assert_eq!(
            item_text(&item, &Reading::new(&detailed, BULLET)),
            Some(selection_of(
                plan_update::PlanUpdateCell {
                    update,
                    detailed: true
                }
                .into()
            )),
        );
        let quiet = Arc::new(PlanUpdate {
            explanation: None,
            steps: vec![
                step("done one", StepStatus::Completed),
                step("doing two", StepStatus::InProgress),
            ],
        });
        agrees(
            &Item::PlanUpdate(quiet.clone()),
            plan_update::PlanUpdateCell {
                update: quiet,
                detailed: false,
            }
            .into(),
        );
    }

    #[test]
    fn a_collab_event_is_its_title_and_detail_rows() {
        let agent = AgentRef {
            id: "a1".into(),
            nickname: Some("Robie".into()),
            role: Some("worker".into()),
        };
        for event in [
            CollabEvent::Spawned {
                agent: agent.clone(),
                request: Some(SpawnRequest {
                    model: "gpt-a".into(),
                    effort: "high".into(),
                }),
                prompt: Some("go and look".into()),
            },
            CollabEvent::SentInput {
                agent,
                prompt: Some("more".into()),
            },
            CollabEvent::SpawnFailed { prompt: None },
        ] {
            let event = Arc::new(event);
            agrees(
                &Item::Collab(event.clone()),
                collab::CollabCell { event }.into(),
            );
        }
    }
}
