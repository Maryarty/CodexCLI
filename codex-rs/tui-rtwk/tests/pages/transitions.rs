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

/// Waits until the chat has opened `dir`: the ask goes through the shell and
/// comes back as an event, so the screen is up when the app applied it, not
/// after a fixed span of frames. Then the frames of the fade-in.
fn wait_shows(h: &mut Harness, what: &str, dir: &str) {
    h.wait_until(&format!("{what} to open {dir}"), |h| shows(h, dir));
    if dir == "import" {
        // The route is up before its flow starts; Escape needs the flow.
        h.wait_until("the import flow started", |h| {
            h.import_phase() != codex_tui_core::screens::import::Phase::Idle
        });
    }
    h.t.pump_for(ms(800));
}

/// Escape closes what the chat opened: the page or sheet answers through the
/// shell too (the import page cancels its flow there), so the chat is back
/// when the app applied that, not after a fixed span of frames. Waits for the
/// screen the chat showed before, then the fade-out.
fn escape_closes(h: &mut Harness, before: &str) {
    h.press("escape");
    h.wait_until("the chat back as it was", |h| h.t.screen() == before);
    h.t.pump_for(ms(400));
}

/// Types `line` and sends it; the caller waits for what it opens.
fn run_command(h: &mut Harness, line: &str) {
    h.t.type_text(line);
    h.press("enter");
}

macro_rules! command_opens {
    ($($test:ident: $line:expr => $dir:expr;)*) => {$(
        #[test]
        fn $test() {
            let mut h = chat();
            let before = h.t.screen();
            run_command(&mut h, $line);
            wait_shows(&mut h, $line, $dir);
            assert!(shows(&h, $dir), "{} did not open {}:\n{}", $line, $dir, h.t.screen());
            escape_closes(&mut h, &before);
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
    slash_usage_opens_the_usage_dashboard: "/usage" => "usage";
    slash_usage_weekly_opens_the_usage_dashboard: "/usage weekly" => "usage";
    slash_warnings_opens_the_dialogs: "/warnings" => "dialogs";
    slash_feedback_opens_the_dialogs: "/feedback" => "dialogs";
    slash_pets_opens_the_pets: "/pets" => "pets";
    slash_pet_alias_opens_the_pets: "/pet" => "pets";
    slash_test_approval_opens_the_approvals: "/test-approval" => "approvals";
}

/// `/rename` is the feature's own prompt (`features/rename`): the feature wishes it, the presenter
/// shows it over the chat, Esc gives it up.
#[test]
fn slash_rename_shows_the_features_name_prompt() {
    let mut h = chat();
    let before = h.t.screen();
    run_command(&mut h, "/rename");
    h.wait_for("the name prompt", |s| s.contains("Name thread"));
    h.t.pump_for(ms(800));
    escape_closes(&mut h, &before);
    assert_chat_back(&h, None, &before);
}

/// Item 8: `/subagents` is the picker of this session's subagents (oracle:
/// `slash_dispatch.rs:412`, `AppEvent::OpenAgentPicker`), not the command
/// center: a sheet titled `Subagents` with the main thread's row.
#[test]
fn slash_subagents_opens_the_agent_picker_not_the_command_center() {
    let mut h = chat();
    run_command(&mut h, "/subagents");
    h.wait_for("the sheet", |s| s.contains("Main [default]"));
    h.t.pump_for(ms(800));
    assert_eq!(open::last_opened(), Some(Target::AgentPicker));
    assert!(h.t.screen().contains("Subagents"), "{}", h.t.screen());
}

/// The command typed in the popup and picked with Enter, not typed in full.
#[test]
fn a_command_picked_in_the_popup_opens_its_screen() {
    let mut h = chat();
    let before = h.t.screen();
    h.t.type_text("/usag");
    h.t.pump_for(ms(100));
    h.press("enter");
    wait_shows(&mut h, "/usag + enter", "usage");
    assert!(shows(&h, "usage"), "{}", h.t.screen());
    escape_closes(&mut h, &before);
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
            wait_shows(&mut h, $keys, $dir);
            assert!(shows(&h, $dir), "{} did not open {}:\n{}", $keys, $dir, h.t.screen());
            escape_closes(&mut h, &before);
            assert_chat_back(&h, Some("a draft"), &before);
        }
    )*};
}

chord_opens! {
    f2_opens_the_warnings: "f2" => "dialogs";
}

/// Escape on an empty composer, twice, is no page any more (`app_backtrack.rs`): the
/// detailed transcript opens in place of the chat's (`tests/pages/chat_live.rs`).
#[test]
fn esc_esc_opens_no_page() {
    let mut h = chat();
    h.press("escape");
    h.press("escape");
    h.t.pump_for(ms(400));
    assert_ne!(
        open::last_opened(),
        Some(Target::Backtrack),
        "{}",
        h.t.screen()
    );
    assert!(!shows(&h, "backtrack"), "{}", h.t.screen());
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
    wait_shows(&mut h, "f8", "voice");
    assert!(shows(&h, "voice"), "{}", h.t.screen());
    escape_closes(&mut h, &before);
    assert_chat_back(&h, None, &before);
}

#[test]
fn quit_and_exit_leave_the_app() {
    for line in ["/quit", "/exit"] {
        let mut h = crate::common::start_quittable(false);
        chat_with_history(&mut h);
        h.t.type_text(line);
        h.press("enter");
        h.wait_until("the app to end", |h| !h.t.is_running());
        assert!(!h.t.is_running(), "{line} did not quit");
    }
}
