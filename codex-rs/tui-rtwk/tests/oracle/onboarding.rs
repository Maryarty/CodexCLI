//! Shard `onboarding` (screens S01-S04, owner startup-flow): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Each screen is the startup page opened from a debug entry of the
//! `onboarding` directory, over the fixture script that makes the oracle
//! test's widget state (`src/pages/startup/onboarding/fixtures/`).

use crate::kit::{self, Owner, Screen};

fn open(entry: &'static str, width: u16, height: u16) -> Screen {
    Screen::build(Owner::StartupFlow)
        .size(width, height)
        .entry("onboarding", entry)
        .open()
}

// ---- oracle: onboarding/onboarding_screen.rs (2 screen-level tests here; 5 state tests left to startup-flow)
// ORACLE onboarding/onboarding_screen.rs:1064 buffered_enter_is_discarded_when_trust_step_becomes_active kind=ui owner=startup-flow
#[test]
fn buffered_enter_is_discarded_when_trust_step_becomes_active() {
    // The update prompt, then the trust step: an Enter typed in the same burst
    // as the key that ends the prompt never reaches the trust step; a fresh
    // Down after it does, and `q` quits from there.
    let mut s = open("flow_update_trust", 70, 14);
    s.keys("esc enter");
    s.assert_contains("› 1. Trust and continue");
    s.keys("down");
    s.assert_contains("› 2. Quit");
}

// ORACLE onboarding/onboarding_screen.rs:1130 restricted_acceptance_and_cancellation_do_not_persist_trust kind=ui owner=startup-flow
#[test]
fn restricted_acceptance_and_cancellation_do_not_persist_trust() {
    kit::na(
        Owner::StartupFlow,
        "what is persisted is no screen: core's state test of the same name holds that a restricted Enter and an Esc ask for no trust write; here the page closes on both",
    );
    let mut s = open("trust_restricted", 70, 18);
    s.keys("enter");
    s.assert_absent("Folder access");
    let mut s = open("trust_restricted", 70, 18);
    s.keys("esc");
    s.assert_absent("Folder access");
}

// ---- oracle: onboarding/trust_directory.rs (9 screen-level tests here; 0 state tests left to startup-flow)
// ORACLE onboarding/trust_directory.rs:296 release_event_does_not_change_selection kind=ui owner=startup-flow
#[test]
fn release_event_does_not_change_selection() {
    kit::na(
        Owner::StartupFlow,
        "key release and repeat kinds are rtwk input's: it delivers presses only, so a release cannot select; the press on `2. Quit` closes the page",
    );
    let mut s = open("trust_git_repo", 70, 14);
    s.keys("down");
    s.assert_contains("› 2. Quit");
    s.keys("enter");
    s.assert_absent("Folder access");
}

// ORACLE onboarding/trust_directory.rs:328 fragmented_terminal_response_cannot_grant_directory_trust kind=ui owner=startup-flow
#[test]
fn fragmented_terminal_response_cannot_grant_directory_trust() {
    let mut s = open("trust_git_repo", 70, 14);
    s.keys("down");
    s.assert_contains("› 2. Quit");
    s.type_text("10;rgb:ffff/ffff/ffff");
    // `1` moved the highlight back to trust; nothing was chosen.
    s.assert_contains("› 1. Trust and continue");
    s.assert_contains("Folder access");
}

// ORACLE onboarding/trust_directory.rs:346 renders_snapshot_for_git_repo kind=snap owner=startup-flow snap=renders_snapshot_for_git_repo
#[test]
fn renders_snapshot_for_git_repo() {
    let s = open("trust_git_repo", 70, 14);
    s.assert_snap("renders_snapshot_for_git_repo");
}

// ORACLE onboarding/trust_directory.rs:360 directory_change_consent_can_keep_current_directory kind=snap owner=startup-flow snap=directory_change_consent_can_keep_current_directory
#[test]
fn directory_change_consent_can_keep_current_directory() {
    let mut s = open("trust_keep_current", 70, 18);
    s.assert_snap("directory_change_consent_can_keep_current_directory");
    s.keys("esc");
    s.assert_absent("Folder access");
}

// ORACLE onboarding/trust_directory.rs:374 renders_snapshot_for_remote_git_subdirectory kind=snap owner=startup-flow snap=renders_snapshot_for_remote_git_subdirectory
#[test]
fn renders_snapshot_for_remote_git_subdirectory() {
    let s = open("trust_remote_subdirectory", 70, 18);
    s.assert_snap("renders_snapshot_for_remote_git_subdirectory");
}

// ORACLE onboarding/trust_directory.rs:402 renders_restricted_folder kind=snap owner=startup-flow snap=existing_untrusted_task snap=renders_restricted_folder
#[test]
fn renders_restricted_folder() {
    let s = open("trust_restricted", 70, 18);
    s.assert_snap("renders_restricted_folder");
    let s = open("trust_existing_task", 70, 18);
    s.assert_snap("existing_untrusted_task");
}

// ORACLE onboarding/trust_directory.rs:424 renders_snapshot_for_trust_error kind=snap owner=startup-flow snap=renders_snapshot_for_trust_error
#[test]
fn renders_snapshot_for_trust_error() {
    let s = open("trust_error", 70, 22);
    s.assert_snap("renders_snapshot_for_trust_error");
}

// ORACLE onboarding/trust_directory.rs:440 picker_wraps_paths_and_restricted_actions kind=snap owner=startup-flow snap=folder_picker_restricted_40x24
#[test]
fn picker_wraps_paths_and_restricted_actions() {
    let s = open("trust_restricted_long", 40, 24);
    s.assert_snap("folder_picker_restricted_40x24");
}

// ORACLE onboarding/trust_directory.rs:456 long_paths_keep_disclosure_and_choices_visible kind=snap owner=startup-flow snap=long_checkout_40x13 snap=long_repository_root_40x17 snap=only_repository_root_fits_40x16
#[test]
fn long_paths_keep_disclosure_and_choices_visible() {
    kit::pending(
        Owner::StartupFlow,
        "G51: the oracle gives the paths the rows the mandatory content leaves (inflexible children laid out in order, then the blanks share what remains and the last takes the slack); `Column` shares the free rows among the paths and the blanks by flex factor and gives the remainder to the FIRST flexible child, so the paths lose their rows to the blanks. 40x16: ours has the blank above `Folder access` and no root path row (the oracle: `workspace/…/repository` under the note); 40x17 and 40x13 likewise",
    );
    let s = open("trust_long_checkout", 40, 13);
    s.assert_snap("long_checkout_40x13");
    let s = open("trust_long_root", 40, 17);
    s.assert_snap("long_repository_root_40x17");
    let s = open("trust_long_root", 40, 16);
    s.assert_snap("only_repository_root_fits_40x16");
}

// ORACLE onboarding/welcome.rs:166 welcome_renders_animation_on_first_draw kind=ui owner=startup-flow
#[test]
fn welcome_renders_animation_on_first_draw() {
    // The logo stage (21 rows) and a blank row above the welcome line.
    let s = open("welcome", 64, 41);
    s.assert_row(22, "  Welcome to Codex, OpenAI's command-line coding agent");
}

// ORACLE onboarding/welcome.rs:184 welcome_skips_animation_below_height_breakpoint kind=ui owner=startup-flow
#[test]
fn welcome_skips_animation_below_height_breakpoint() {
    let s = open("welcome", 64, 40);
    s.assert_row(0, "  Welcome to Codex, OpenAI's command-line coding agent");
}

// ORACLE onboarding/welcome.rs:199 welcome_logo_layout kind=snap owner=startup-flow
#[test]
fn welcome_logo_layout() {
    kit::pending(
        Owner::StartupFlow,
        "the frame at zero elapsed time: opening a debug entry costs the kit 100 ms of fake time (the Enter of open_debug_row), so the first frame it can read is the logo 2 frames into its spin; the app paints the oracle's frame first (seen with the phase held at 0). Needs a kit seed that reads an entry's first frame",
    );
    // The first draw shows the intact logo at zero elapsed time. insta keeps
    // the snapshot without its two leading blank rows (the stage is 21 rows,
    // the welcome line on row 22), so the snapshot is the screen from row 2.
    let s = Screen::build(Owner::StartupFlow)
        .size(160, 48)
        .entry("onboarding", "welcome")
        .early()
        .open();
    s.assert_row(0, "");
    s.assert_row(1, "");
    s.assert_snap_region("welcome_logo_160x48", 2, 0);
}

// ---- oracle: update_prompt.rs (7 screen-level tests here; 0 state tests left to startup-flow)
// ORACLE update_prompt.rs:294 update_prompt_snapshot kind=snap owner=startup-flow snap=update_prompt_modal
#[test]
fn update_prompt_snapshot() {
    let s = open("update_prompt", 80, 12);
    s.assert_snap("update_prompt_modal");
}

// ORACLE update_prompt.rs:305 update_prompt_confirm_selects_update kind=ui owner=startup-flow
#[test]
fn update_prompt_confirm_selects_update() {
    kit::na(
        Owner::StartupFlow,
        "the choice is an outcome, not drawn: core's state test of the same name holds `Update now`; here the prompt closes",
    );
    let mut s = open("update_prompt", 80, 12);
    s.keys("enter");
    s.assert_absent("Update available");
}

// ORACLE update_prompt.rs:313 update_prompt_dismiss_option_leaves_prompt_in_normal_state kind=ui owner=startup-flow
#[test]
fn update_prompt_dismiss_option_leaves_prompt_in_normal_state() {
    kit::na(
        Owner::StartupFlow,
        "the choice is an outcome, not drawn: core's state test of the same name holds `Skip`; here the highlight moves and the prompt closes",
    );
    let mut s = open("update_prompt", 80, 12);
    s.keys("down");
    s.assert_contains("› 2. Skip");
    s.keys("enter");
    s.assert_absent("Update available");
}

// ORACLE update_prompt.rs:322 update_prompt_dont_remind_selects_dismissal kind=ui owner=startup-flow
#[test]
fn update_prompt_dont_remind_selects_dismissal() {
    kit::na(
        Owner::StartupFlow,
        "the choice is an outcome, not drawn: core's state test of the same name holds `Skip until next version`; here the highlight moves and the prompt closes",
    );
    let mut s = open("update_prompt", 80, 12);
    s.keys("down down");
    s.assert_contains("› 3. Skip until next version");
    s.keys("enter");
    s.assert_absent("Update available");
}

// ORACLE update_prompt.rs:332 update_prompt_ctrl_c_skips_update kind=ui owner=startup-flow
#[test]
fn update_prompt_ctrl_c_skips_update() {
    kit::na(
        Owner::StartupFlow,
        "the choice is an outcome, not drawn: core's state test of the same name holds `Skip`; here the prompt closes",
    );
    let mut s = open("update_prompt", 80, 12);
    s.keys("ctrl+c");
    s.assert_absent("Update available");
}

// ORACLE update_prompt.rs:340 update_prompt_navigation_wraps_between_entries kind=ui owner=startup-flow
#[test]
fn update_prompt_navigation_wraps_between_entries() {
    let mut s = open("update_prompt", 80, 12);
    s.keys("up");
    s.assert_contains("› 3. Skip until next version");
    s.keys("down");
    s.assert_contains("› 1. Update now (runs `npm install -g @openai/codex`)");
}

// ORACLE update_prompt.rs:349 long_update_command_keeps_selected_skip_visible_in_a_short_viewport kind=snap owner=startup-flow
#[test]
fn long_update_command_keeps_selected_skip_visible_in_a_short_viewport() {
    let mut s = open("update_prompt_windows", 28, 12);
    s.keys("up");
    s.assert_contains("› 3. Skip until next version");
    s.assert_snap("update_picker_selected_28x12");
    s.keys("esc");
    s.assert_absent("Update available");
}
