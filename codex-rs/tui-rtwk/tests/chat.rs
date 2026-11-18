//! Screen tests of the `chat` slot: the chat as the hub (`transitions`), the
//! slash commands typed into its composer (`commands`), and the helpers they
//! share. Snapshots in `tests/snapshots/chat/`.

use codex_tui_rtwk::l10n::Lang;
use codex_tui_rtwk::screens;
use codex_tui_rtwk::shell::fake::FAKE_THREAD;
use codex_tui_rtwk_core::fixtures::builders::{agent_completed, agent_delta, turn_completed};

use crate::common::{Harness, slot};

#[path = "commands.rs"]
mod commands;
#[path = "transitions.rs"]
mod transitions;

#[test]
fn chat_slot_is_registered_and_empty() {
    slot("chat");
}

/// A chat with one answered prompt and a draft in the composer.
pub(crate) fn chat_with_history(h: &mut Harness) {
    h.wait_for("ready", |s| s.contains("ready"));
    h.fake.script_turn(vec![
        agent_delta(FAKE_THREAD, "turn-1", "msg-1", "Hello"),
        agent_completed(FAKE_THREAD, "turn-1", "msg-1", "Hello"),
        turn_completed(FAKE_THREAD, "turn-1"),
    ]);
    h.t.type_text("hi");
    h.press("enter");
    h.wait_for("the answer", |s| s.contains("• Hello"));
    h.wait_for("the turn to end", |s| s.contains("ready"));
}

/// The chat as the user left it: the transcript, and the composer's text.
pub(crate) fn assert_chat_back(h: &Harness, draft: Option<&str>) {
    let screen = h.t.screen();
    assert!(screen.contains("› hi"), "transcript lost:\n{screen}");
    assert!(screen.contains("• Hello"), "transcript lost:\n{screen}");
    if let Some(draft) = draft {
        assert!(screen.contains(draft), "composer text lost:\n{screen}");
    }
    assert!(
        !screen.contains("empty slot"),
        "a target screen is still up:\n{screen}"
    );
}

/// Whether the screen of directory `dir` is up: its slot text, or its title.
pub(crate) fn shows(h: &Harness, dir: &str) -> bool {
    let screen = h.t.screen();
    let dirs = screens::dirs();
    let info = dirs
        .iter()
        .find(|d| d.name == dir)
        .unwrap_or_else(|| panic!("`{dir}` is not a screen directory"));
    screen.contains(&format!("{} ({}): empty slot", info.name, info.id))
        || screen.contains((info.title)(Lang::En))
}
