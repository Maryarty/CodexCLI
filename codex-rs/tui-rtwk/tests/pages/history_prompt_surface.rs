// test root: pages
//! The surface of a submitted prompt on a terminal that never told its background: the oracle's
//! fallback grays (`history_prompt_fallback`) where the terminal shows 256 colours or more, and
//! none (the terminal's own colours) on 16 colours or none.
use serde_json::json;

use crate::kit::{self, ColorDepth, Owner, Screen};

const PROMPT: &str = "hello from user";

fn screen(depth: Option<ColorDepth>) -> Screen {
    let turn = kit::replay_turn(
        "turn-1",
        json!([
            {"type": "userMessage", "id": "user-1", "content": [
                {"type": "text", "text": PROMPT, "text_elements": []}
            ]},
            {"type": "agentMessage", "id": "assistant-1", "text": "assistant reply", "phase": null, "memoryCitation": null}
        ]),
    );
    let open = Screen::build(Owner::ChatLive)
        .size(60, 14)
        .replay(vec![turn]);
    let open = match depth {
        Some(depth) => open.color_depth(depth),
        None => open,
    };
    let mut s = open.live().open();
    s.wait_for("assistant reply");
    s
}

#[test]
fn a_true_colour_terminal_without_a_background_gets_the_fallback_grays() {
    screen(None).assert_text_style(PROMPT, "fg=Rgb(238,238,238) bg=Rgb(68,68,68)");
}

#[test]
fn a_256_colour_terminal_without_a_background_gets_the_fallback_grays() {
    screen(Some(ColorDepth::Ansi256)).assert_text_style(PROMPT, "fg=Indexed(255) bg=Indexed(238)");
}

#[test]
fn a_16_colour_terminal_without_a_background_draws_no_surface() {
    screen(Some(ColorDepth::Ansi16)).assert_text_style(PROMPT, "fg=Reset bg=Reset");
}

#[test]
fn a_terminal_without_colours_draws_no_surface() {
    screen(Some(ColorDepth::None)).assert_text_style(PROMPT, "fg=Reset bg=Reset");
}
