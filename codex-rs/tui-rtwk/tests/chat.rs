//! Screen tests of the `chat` slot: the chat as the hub (`transitions`), the
//! slash commands typed into its composer (`commands`), and the helpers they
//! share. Snapshots in `tests/snapshots/chat/`.

use codex_tui_rtwk::screens::chat::open;
use codex_tui_rtwk::screens::{
    agents, approvals, backtrack, import, pager, pets, resume, settings, usage, voice,
};
use codex_tui_rtwk::shell::fake::FAKE_THREAD;
use codex_tui_rtwk_core::commands::Target;
use codex_tui_rtwk_core::fixtures::builders::{agent_completed, agent_delta, turn_completed};
use rtwk_test::{Finder, find};

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

/// The chat as the user left it: the transcript, the composer's text, and
/// exactly the screen it showed before the target opened (so no target, stub
/// or real, is still up over it).
pub(crate) fn assert_chat_back(h: &Harness, draft: Option<&str>, before: &str) {
    assert_transcript_kept(h, draft);
    assert_eq!(h.t.screen(), before, "the chat is not as it was before");
}

/// The transcript and the composer's draft are on the screen.
pub(crate) fn assert_transcript_kept(h: &Harness, draft: Option<&str>) {
    let screen = h.t.screen();
    assert!(screen.contains("› hi"), "transcript lost:\n{screen}");
    assert!(screen.contains("• Hello"), "transcript lost:\n{screen}");
    if let Some(draft) = draft {
        assert!(screen.contains(draft), "composer text lost:\n{screen}");
    }
}

/// The target a screen directory is opened as from the chat.
fn target_of(dir: &str) -> Target {
    match dir {
        "resume" => Target::Resume,
        "agents" => Target::Agents,
        "import" => Target::Import,
        "usage" => Target::Usage,
        "pager" => Target::Pager,
        "status" => Target::Status,
        "select" => Target::Select,
        "settings" => Target::Settings,
        "dialogs" => Target::Dialogs,
        "pets" => Target::Pets,
        "voice" => Target::Voice,
        "approvals" => Target::Approvals,
        "backtrack" => Target::Backtrack,
        _ => panic!("`{dir}` is not a screen the chat opens"),
    }
}

/// The widget the screen of directory `dir` puts up: its own type, so a
/// stub or the real screen is found by what it is, never by what it prints.
/// (`status`, `select` and `dialogs` keep their dialogs private: found by type name.)
fn widget_of(dir: &str) -> Finder {
    match dir {
        "resume" => find::by_type::<resume::Screen>(),
        "agents" => find::by_type::<agents::Screen>(),
        "import" => find::by_type::<import::Screen>(),
        "usage" => find::by_type::<usage::Screen>(),
        "pager" => find::by_type::<pager::Screen>(),
        "settings" => find::by_type::<settings::Screen>(),
        "dialogs" => find::widget_where("SmallDialog", |w| w.type_name() == "SmallDialog"),
        "pets" => find::by_type::<pets::Screen>(),
        "voice" => find::by_type::<voice::Screen>(),
        "approvals" => find::by_type::<approvals::Screen>(),
        "backtrack" => find::by_type::<backtrack::Screen>(),
        "status" => find::widget_where("CardDialog", |w| w.type_name() == "CardDialog"),
        "select" => find::widget_where("MenuDialog", |w| w.type_name() == "MenuDialog"),
        _ => panic!("`{dir}` is not a screen the chat opens"),
    }
}

/// Whether the chat opened the screen of directory `dir`: the target the
/// opener recorded (after it ran) is `dir`'s, and the widget of `dir` is in
/// the tree. A no-op opener, or one that opens another directory, fails.
pub(crate) fn shows(h: &Harness, dir: &str) -> bool {
    open::last_opened() == Some(target_of(dir)) && widget_of(dir).count(&h.t) > 0
}
