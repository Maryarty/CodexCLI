// test root: pages
//! Pager behavior driven through the active chat.

use codex_tui_core::app::Command;
use codex_tui_core::app::effects::{Ask, Done};
use codex_tui_core::screens::transcript::diff::Report;

use crate::kit::{Owner, Screen};

#[test]
fn diff_honors_its_pager_bindings_and_restores_chat_at_a_narrow_width() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(60, 10)
        .keymap(&[("pager.jump_bottom", "f8 f9"), ("pager.close", "x")])
        .live()
        .open();
    let chat = s.shot();
    s.type_text("/diff");
    s.keys("enter");
    let commands = s.commands();
    let Some(ask) = commands.iter().find(|command| {
        matches!(
            command,
            Command::Effect {
                ask: Ask::GitDiff { .. },
                ..
            }
        )
    }) else {
        panic!("the chat requests the diff from the host");
    };
    let text = (0..20).map(|n| format!("changed line {n:02}\n")).collect();
    s.answer_effect(
        ask,
        Done::GitDiff(Ok(Report {
            in_repo: true,
            text,
        })),
    );
    s.wait_for("D I F F");
    s.assert_contains("changed line 00");
    s.assert_contains("f8 f9");
    s.assert_contains("x close");
    s.keys("end q f8");
    s.assert_contains("changed line 00");
    s.keys("f9");
    s.assert_contains("changed line 19");
    s.resize(24, 9);
    s.keys("home");
    s.assert_contains("changed line 00");
    s.resize(60, 10);
    s.keys("x");
    s.assert_same_rows(&chat);
    s.type_text("composer has focus");
    s.assert_composer("composer has focus");
}

#[test]
fn the_transcript_overlay_closes_with_the_configured_transcript_key() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(40, 12)
        .keymap(&[("pager.close_transcript", "f8")])
        .entry("transcript", "sheet_basic")
        .open();
    s.keys("ctrl+t");
    s.assert_contains("T R A N S C R I P T");
    s.keys("f8");
    s.assert_absent("T R A N S C R I P T");
    s.assert_contains("debug screens");
}

#[test]
fn ctrl_t_searches_live_chat_and_returns_to_the_same_draft() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(40, 12)
        .script(include_str!(
            "../oracle/fixtures/transcript_view_4/find_jump.jsonl"
        ))
        .draft("still editing")
        .open();
    s.assert_composer("still editing");
    s.keys("ctrl+t f3");
    s.type_text("needle");
    s.assert_style_in("Needle two", 0, "reversed");
    s.keys("ctrl+p");
    s.assert_style_in("Needle one", 0, "reversed");
    s.keys("ctrl+n enter");
    s.assert_style_in("Needle two", 0, "reversed");
    s.keys("esc ctrl+t");
    s.assert_composer("still editing");
    s.type_text("!");
    s.assert_composer("still editing!");
}
