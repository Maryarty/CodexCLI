//! Screen tests of the `chat` slot: the chat as the hub (`transitions`), the
//! slash commands typed into its composer (`commands`), and the helpers they
//! share. Snapshots in `tests/snapshots/chat/`.

use codex_tui_core::commands::Target;
use codex_tui_core::fixtures::builders::{agent_completed, agent_delta, turn_completed};
use codex_tui_rtwk::features::settings;
use codex_tui_rtwk::pages::session::backtrack;
use codex_tui_rtwk::pages::session::open;
use codex_tui_rtwk::pages::startup::import;
use codex_tui_rtwk::pages::{agents, resume, usage};
use codex_tui_rtwk::services::backend::fake::FAKE_THREAD;
use codex_tui_rtwk::sheets::{approvals, pets, voice};
use rtwk_test::{Finder, find};

use crate::common::{Harness, slot};

#[path = "chat_live.rs"]
mod chat_live;
#[path = "commands.rs"]
mod commands;
#[path = "permissions_live.rs"]
mod permissions_live;
#[path = "transitions.rs"]
mod transitions;

#[test]
fn chat_slot_is_registered_and_empty() {
    slot("chat");
}

/// A chat with one answered prompt and a draft in the composer.
pub(crate) fn chat_with_history(h: &mut Harness) {
    h.wait_idle();
    h.fake.script_turn(vec![
        agent_delta(FAKE_THREAD, "turn-1", "msg-1", "Hello"),
        agent_completed(FAKE_THREAD, "turn-1", "msg-1", "Hello"),
        turn_completed(FAKE_THREAD, "turn-1"),
    ]);
    h.t.type_text("hi");
    h.press("enter");
    h.wait_for("the answer", |s| s.contains("• Hello"));
    h.wait_idle();
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
/// (`select` and `dialogs` keep their dialogs private: found by type name.)
fn widget_of(dir: &str) -> Finder {
    match dir {
        "resume" => find::by_type::<resume::Screen>(),
        "agents" => find::by_type::<agents::Screen>(),
        "import" => find::by_type::<import::Screen>(),
        "usage" => find::by_type::<usage::Screen>(),
        "settings" => find::by_type::<settings::Screen>(),
        "dialogs" => find::widget_where("SmallDialog", |w| w.type_name() == "SmallDialog"),
        "pets" => find::by_type::<pets::Screen>(),
        "voice" => find::by_type::<voice::Screen>(),
        "approvals" => find::by_type::<approvals::Screen>(),
        "backtrack" => find::by_type::<backtrack::Screen>(),
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

// ---- the status line (oracle: bottom_pane/status_line_style.rs tests) ----

mod status_line {
    use codex_tui_rtwk::app::theme::{Tokens, data};
    use codex_tui_rtwk::widgets::status_line::{item_style, status_line};

    fn drawn(items: &[(&str, &str)], colors: bool) -> Option<String> {
        let tokens = Tokens::new(data());
        let line = status_line(
            items.iter().map(|(id, text)| (*id, (*text).to_string())),
            colors,
            &tokens,
        )?;
        let mut t = rtwk_test::Tester::with(rtwk_test::Options::new().size(60, 1));
        t.pump_widget(line);
        Some(
            t.screen()
                .lines()
                .next()
                .unwrap_or("")
                .trim_end()
                .to_string(),
        )
    }

    // status_line_segments_use_secondary_separators_and_theme_styles_first
    // (the text; the separator is the secondary style, each accent its own).
    #[test]
    fn status_line_segments_use_secondary_separators_and_theme_styles_first() {
        let items = [("model", "gpt-5"), ("context-used", "Context 12% used")];
        assert_eq!(
            drawn(&items, true).as_deref(),
            Some("gpt-5 · Context 12% used")
        );
        let tokens = Tokens::new(data());
        assert_eq!(item_style("model", true, &tokens), tokens.info());
        assert_eq!(item_style("context-used", true, &tokens), tokens.success());
        assert_ne!(item_style("model", true, &tokens), tokens.dim());
    }

    // thread_usage_items_share_an_accent_and_secondary_separator
    #[test]
    fn thread_usage_items_share_an_accent_and_secondary_separator() {
        let items = [
            ("thread-credits", "5.2 credits"),
            ("estimated-thread-cost", "~$0.21"),
        ];
        assert_eq!(drawn(&items, true).as_deref(), Some("5.2 credits · ~$0.21"));
        let tokens = Tokens::new(data());
        assert_eq!(
            item_style("thread-credits", true, &tokens),
            item_style("estimated-thread-cost", true, &tokens)
        );
    }

    // status_line_segments_can_disable_theme_colors
    #[test]
    fn status_line_segments_can_disable_theme_colors() {
        let items = [("model", "gpt-5"), ("context-used", "Context 12% used")];
        assert_eq!(
            drawn(&items, false).as_deref(),
            Some("gpt-5 · Context 12% used")
        );
        let tokens = Tokens::new(data());
        for id in ["model", "context-used"] {
            assert_eq!(item_style(id, false, &tokens), tokens.dim());
        }
    }

    // pull_request_number_uses_link_style
    #[test]
    fn pull_request_number_uses_link_style() {
        let tokens = Tokens::new(data());
        assert_ne!(
            item_style("pull-request-number", false, &tokens),
            tokens.dim()
        );
        assert_eq!(
            drawn(&[("pull-request-number", "PR #20252")], false).as_deref(),
            Some("PR #20252")
        );
    }

    // status_line_segments_return_none_when_empty
    #[test]
    fn status_line_segments_return_none_when_empty() {
        assert_eq!(drawn(&[], true), None);
    }

    // new_session_tests: `server-model high · <PROJECT>`, the order the config
    // gives (item order is the caller's list).
    #[test]
    fn items_keep_the_configured_order() {
        let items = [
            ("current-dir", "rtwk"),
            ("model-with-reasoning", "GPT-6.2-Sol high"),
        ];
        assert_eq!(
            drawn(&items, true).as_deref(),
            Some("rtwk · GPT-6.2-Sol high")
        );
    }
}
