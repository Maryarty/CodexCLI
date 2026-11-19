//! The chat is the hub: every screen-opening command and key opens its screen
//! from the chat, and Escape returns with the transcript and the composer's
//! text intact. One test per command and per key action; the commands and
//! chords are the oracle's (`slash_command.rs`, `keymap.rs:1908-1953`), the
//! screen each opens is the trigger table of `parity/chat-nav.md`.

use codex_tui_rtwk::pages::session::open;

use crate::chat::{assert_chat_back, chat_with_history, shows};
use crate::common::{Harness, ms, start};
use codex_tui_core::commands::Target;

fn chat() -> Harness {
    open::reset_last_opened();
    let mut h = start();
    chat_with_history(&mut h);
    h
}

/// Types `line` and sends it; dialogs fade in, so the frames run on.
fn run_command(h: &mut Harness, line: &str) {
    h.t.type_text(line);
    h.press("enter");
    h.t.pump_for(ms(800));
}

macro_rules! command_opens {
    ($($test:ident: $line:expr => $dir:expr;)*) => {$(
        #[test]
        fn $test() {
            let mut h = chat();
            let before = h.t.screen();
            run_command(&mut h, $line);
            assert!(shows(&h, $dir), "{} did not open {}:\n{}", $line, $dir, h.t.screen());
            h.press("escape");
            h.t.pump_for(ms(400));
            assert_chat_back(&h, None, &before);
        }
    )*};
}

command_opens! {
    slash_model_opens_the_selection: "/model" => "select";
    slash_permissions_opens_the_selection: "/permissions" => "select";
    slash_review_opens_the_selection: "/review" => "select";
    slash_archive_opens_the_confirmation: "/archive" => "select";
    slash_delete_opens_the_confirmation: "/delete" => "select";
    slash_skills_opens_the_settings: "/skills" => "settings";
    slash_config_opens_the_settings: "/config" => "settings";
    slash_theme_opens_the_settings: "/theme" => "settings";
    slash_title_opens_the_settings: "/title" => "settings";
    slash_statusline_opens_the_settings: "/statusline" => "settings";
    slash_experimental_opens_the_settings: "/experimental" => "settings";
    slash_memories_opens_the_settings: "/memories" => "settings";
    slash_keymap_opens_the_settings: "/keymap" => "settings";
    slash_agents_opens_the_agents: "/agents" => "agents";
    slash_resume_opens_the_picker: "/resume" => "resume";
    slash_import_opens_the_import: "/import" => "import";
    slash_status_opens_the_status_card: "/status" => "status";
    slash_usage_opens_the_usage_dashboard: "/usage" => "usage";
    slash_usage_weekly_opens_the_usage_dashboard: "/usage weekly" => "usage";
    slash_warnings_opens_the_dialogs: "/warnings" => "dialogs";
    slash_feedback_opens_the_dialogs: "/feedback" => "dialogs";
    slash_rename_opens_the_dialogs: "/rename" => "dialogs";
    slash_pets_opens_the_pets: "/pets" => "pets";
    slash_pet_alias_opens_the_pets: "/pet" => "pets";
    slash_test_approval_opens_the_approvals: "/test-approval" => "approvals";
}

/// Item 8: `/subagents` is the picker of this session's subagents (oracle:
/// `slash_dispatch.rs:412`, `AppEvent::OpenAgentPicker`), not the command
/// center; the picker is not built, so the sheet says so.
#[test]
fn slash_subagents_opens_the_agent_picker_not_the_command_center() {
    let mut h = chat();
    run_command(&mut h, "/subagents");
    assert_eq!(open::last_opened(), Some(Target::AgentPicker));
    assert!(
        h.t.screen()
            .contains("Not implemented yet: subagent picker"),
        "{}",
        h.t.screen()
    );
}

/// The command typed in the popup and picked with Enter, not typed in full.
#[test]
fn a_command_picked_in_the_popup_opens_its_screen() {
    let mut h = chat();
    let before = h.t.screen();
    h.t.type_text("/stat");
    h.t.pump_for(ms(100));
    h.press("enter");
    h.t.pump_for(ms(800));
    assert!(shows(&h, "status"), "{}", h.t.screen());
    h.press("escape");
    h.t.pump_for(ms(400));
    assert_chat_back(&h, None, &before);
}

macro_rules! chord_opens {
    ($($test:ident: $keys:expr => $dir:expr;)*) => {$(
        #[test]
        fn $test() {
            let mut h = chat();
            h.t.type_text("a draft");
            h.t.pump_for(ms(100));
            let before = h.t.screen();
            h.press($keys);
            h.t.pump_for(ms(800));
            assert!(shows(&h, $dir), "{} did not open {}:\n{}", $keys, $dir, h.t.screen());
            h.press("escape");
            h.t.pump_for(ms(400));
            assert_chat_back(&h, Some("a draft"), &before);
        }
    )*};
}

chord_opens! {
    f2_opens_the_warnings: "f2" => "dialogs";
}

/// Escape on an empty composer, twice, starts backtrack (`app_backtrack.rs`).
#[test]
fn esc_esc_starts_backtrack_and_esc_leaves_it() {
    let mut h = chat();
    let before = h.t.screen();
    h.press("escape");
    h.press("escape");
    h.t.pump_for(ms(400));
    assert!(shows(&h, "backtrack"), "{}", h.t.screen());
    h.press("escape");
    h.t.pump_for(ms(400));
    assert_chat_back(&h, None, &before);
}

/// Escape with text in the composer does not start backtrack.
#[test]
fn esc_with_a_draft_does_not_start_backtrack() {
    let mut h = chat();
    h.t.type_text("a draft");
    h.press("escape");
    h.press("escape");
    h.t.pump_for(ms(400));
    assert_ne!(
        open::last_opened(),
        Some(Target::Backtrack),
        "{}",
        h.t.screen()
    );
    assert!(h.t.screen().contains("a draft"), "{}", h.t.screen());
}

#[test]
fn f8_toggles_voice_which_opens_the_voice_strip() {
    let mut h = chat();
    let before = h.t.screen();
    h.press("f8");
    h.t.pump_for(ms(400));
    assert!(shows(&h, "voice"), "{}", h.t.screen());
    h.press("escape");
    h.t.pump_for(ms(400));
    assert_chat_back(&h, None, &before);
}

#[test]
fn quit_and_exit_leave_the_app() {
    for line in ["/quit", "/exit"] {
        let mut h = crate::common::start_quittable(false);
        chat_with_history(&mut h);
        h.t.type_text(line);
        h.press("enter");
        h.t.pump_for(ms(400));
        assert!(!h.t.is_running(), "{line} did not quit");
    }
}
