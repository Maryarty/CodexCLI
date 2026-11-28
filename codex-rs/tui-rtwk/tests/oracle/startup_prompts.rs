//! Shard `startup_prompts` (screens S05, owner startup-prompts): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.

use crate::kit::{self, Owner, Screen};

// ---- oracle: cwd_prompt.rs (11 screen-level tests here; 0 state tests left to startup-prompts)
// ORACLE cwd_prompt.rs:378 cwd_prompt_snapshot kind=snap owner=startup-prompts snap=cwd_prompt_modal
#[test]
fn cwd_prompt_snapshot() {
    let s = Screen::build(Owner::StartupPrompts)
        .size(80, 14)
        .entry("startup", "cwd_resume")
        .open();
    s.assert_snap("cwd_prompt_modal");
}

// ORACLE cwd_prompt.rs:389 cwd_prompt_fork_snapshot kind=snap owner=startup-prompts snap=cwd_prompt_fork_modal
#[test]
fn cwd_prompt_fork_snapshot() {
    let s = Screen::build(Owner::StartupPrompts)
        .size(80, 14)
        .entry("startup", "cwd_fork")
        .open();
    s.assert_snap("cwd_prompt_fork_modal");
}

// ORACLE cwd_prompt.rs:407 cwd_prompt_remote_exec_snapshot kind=snap owner=startup-prompts snap=cwd_prompt_remote_exec_modal
#[test]
fn cwd_prompt_remote_exec_snapshot() {
    let s = Screen::build(Owner::StartupPrompts)
        .size(80, 13)
        .entry("startup", "cwd_remote_exec")
        .open();
    s.assert_snap("cwd_prompt_remote_exec_modal");
}

// ORACLE cwd_prompt.rs:431 cwd_prompt_remembered_current_snapshot kind=snap owner=startup-prompts snap=cwd_prompt_remembered_current_modal
#[test]
fn cwd_prompt_remembered_current_snapshot() {
    let s = Screen::build(Owner::StartupPrompts)
        .size(80, 14)
        .entry("startup", "cwd_remembered_current")
        .open();
    s.assert_snap("cwd_prompt_remembered_current_modal");
}

// ORACLE cwd_prompt.rs:455 cwd_prompt_selects_session_by_default kind=ui owner=startup-prompts
#[test]
fn cwd_prompt_selects_session_by_default() {
    let mut s = Screen::build(Owner::StartupPrompts)
        .size(80, 14)
        .entry("startup", "cwd_resume")
        .open();
    s.assert_contains("› 1. Use session directory");
    s.keys("enter");
    s.assert_absent("Working directory");
}

// ORACLE cwd_prompt.rs:462 cwd_prompt_can_select_current kind=ui owner=startup-prompts
#[test]
fn cwd_prompt_can_select_current() {
    let mut s = Screen::build(Owner::StartupPrompts)
        .size(80, 14)
        .entry("startup", "cwd_resume")
        .open();
    s.keys("down");
    s.assert_contains("› 2. Use current directory");
    s.keys("enter");
    s.assert_absent("Working directory");
}

// ORACLE cwd_prompt.rs:470 narrow_picker_keeps_selected_choice_and_escape_uses_session kind=snap owner=startup-prompts
#[test]
fn narrow_picker_keeps_selected_choice_and_escape_uses_session() {
    let mut s = Screen::build(Owner::StartupPrompts)
        .size(40, 16)
        .entry("startup", "cwd_long_paths")
        .open();
    s.keys("up");
    s.assert_contains("› 4. Always use current directory");
    s.assert_contains("esc use session");
    kit::delta(
        Owner::StartupPrompts,
        "G49: rtwk's `TextWrap::Word` never breaks after `/`; the oracle's picker rows do (`› 4. Always use current directory (/` / `     Users/example/current)`), ours moves `(/Users/example/current)` whole to the next row. The window (rows 3 and 4 only, `↑` above) and the hanging indent are drawn as the oracle's",
    );
    s.assert_snap("cwd_picker_selected_40x16");
    s.keys("esc");
    s.assert_absent("Working directory");
}

// ORACLE cwd_prompt.rs:490 cwd_prompt_omits_unusable_remembered_current_choice kind=ui owner=startup-prompts
#[test]
fn cwd_prompt_omits_unusable_remembered_current_choice() {
    let mut s = Screen::build(Owner::StartupPrompts)
        .size(80, 13)
        .entry("startup", "cwd_remote_exec")
        .open();
    s.keys("4");
    s.assert_contains("Working directory");
    s.keys("up");
    s.assert_contains("› 3. Always use session directory");
}

// ORACLE cwd_prompt.rs:507 cwd_prompt_ctrl_c_exits_instead_of_selecting kind=ui owner=startup-prompts
#[test]
fn cwd_prompt_ctrl_c_exits_instead_of_selecting() {
    let mut s = Screen::build(Owner::StartupPrompts)
        .size(80, 14)
        .entry("startup", "cwd_resume")
        .open();
    s.keys("ctrl+c");
    s.assert_absent("Working directory");
}

// ORACLE cwd_prompt.rs:515 cwd_prompt_remembered_choices_select_and_persist_matching_directory kind=ui owner=startup-prompts
#[test]
fn cwd_prompt_remembered_choices_select_and_persist_matching_directory() {
    // The config.toml write is the host's (no screen shows it); what shows is
    // that the number key takes the row and the prompt closes.
    for key in ["3", "4"] {
        let mut s = Screen::build(Owner::StartupPrompts)
            .size(80, 14)
            .entry("startup", "cwd_resume")
            .open();
        s.keys(key);
        s.assert_absent("Working directory");
    }
}

// ORACLE cwd_prompt.rs:562 cwd_prompt_persistence_failure_snapshot kind=snap owner=startup-prompts snap=cwd_prompt_persistence_failure
#[test]
fn cwd_prompt_persistence_failure_snapshot() {
    let s = Screen::build(Owner::StartupPrompts)
        .size(100, 1)
        .entry("startup", "cwd_persistence_failure")
        .open();
    s.assert_snap("cwd_prompt_persistence_failure");
}

// ---- oracle: model_migration.rs (6 screen-level tests here; 0 state tests left to startup-prompts)
// ORACLE model_migration.rs:426 prompt_snapshot kind=snap owner=startup-prompts snap=model_migration_prompt
#[test]
fn prompt_snapshot() {
    let s = Screen::build(Owner::StartupPrompts)
        .size(60, 28)
        .entry("startup", "migration_opt_out")
        .open();
    kit::delta(
        Owner::StartupPrompts,
        "G48: rtwk's `TextWrap::Word` breaks after the hyphen of `gpt-5.1-codex-max.` (`Codex just got an upgrade. Introducing gpt-5.1-codex-` / `max.`); the oracle's `Paragraph` wraps at blanks only (the heading moves whole to the next row), and no `TextWrap` breaks at blanks only",
    );
    s.assert_snap("model_migration_prompt");
}

// ORACLE model_migration.rs:460 prompt_snapshot_gpt5_family kind=snap owner=startup-prompts snap=model_migration_prompt_gpt5_family
#[test]
fn prompt_snapshot_gpt5_family() {
    let s = Screen::build(Owner::StartupPrompts)
        .size(65, 22)
        .entry("startup", "migration_gpt5_family")
        .open();
    s.assert_snap("model_migration_prompt_gpt5_family");
}

// ORACLE model_migration.rs:487 escape_key_accepts_prompt kind=ui owner=startup-prompts
#[test]
fn escape_key_accepts_prompt() {
    let mut s = Screen::build(Owner::StartupPrompts)
        .size(60, 28)
        .entry("startup", "migration_opt_out")
        .open();
    s.keys("esc");
    s.assert_absent("Codex just got an upgrade");
}

// ORACLE model_migration.rs:516 selecting_use_existing_model_rejects_upgrade kind=ui owner=startup-prompts
#[test]
fn selecting_use_existing_model_rejects_upgrade() {
    let mut s = Screen::build(Owner::StartupPrompts)
        .size(60, 28)
        .entry("startup", "migration_opt_out")
        .open();
    s.keys("down");
    s.assert_contains("› 2. Use existing model");
    s.keys("enter");
    s.assert_absent("Codex just got an upgrade");
}

// ORACLE model_migration.rs:548 markdown_prompt_keeps_long_url_tail_visible_when_narrow kind=ui owner=startup-prompts
#[test]
fn markdown_prompt_keeps_long_url_tail_visible_when_narrow() {
    let s = Screen::build(Owner::StartupPrompts)
        .size(40, 16)
        .entry("startup", "migration_markdown_url")
        .open();
    s.assert_contains("tail42");
}

// ORACLE model_migration.rs:578 migration_choice_and_hints_stay_visible_in_a_short_viewport kind=snap owner=startup-prompts
#[test]
fn migration_choice_and_hints_stay_visible_in_a_short_viewport() {
    let mut s = Screen::build(Owner::StartupPrompts)
        .size(28, 12)
        .entry("startup", "migration_markdown")
        .open();
    s.keys("down");
    s.assert_contains("› 2. Use existing model");
    s.assert_contains("enter/esc confirm");
    s.assert_snap("model_picker_selected_28x12");
    s.keys("esc");
    s.assert_absent("Introducing gpt-new");
}

// ---- oracle: startup_hooks_review.rs (5 screen-level tests here; 2 state tests left to startup-prompts)
// ORACLE startup_hooks_review.rs:434 fragmented_terminal_response_cannot_grant_startup_hook_trust kind=ui owner=startup-prompts
#[test]
fn fragmented_terminal_response_cannot_grant_startup_hook_trust() {
    let mut s = Screen::build(Owner::StartupPrompts)
        .size(80, 24)
        .entry("startup", "hooks_review")
        .open();
    s.type_text("20;rgb:2222/ffff/ffff");
    s.assert_contains("Hooks need review");
    s.assert_absent("Trusting hooks...");
    s.assert_contains("› 2. Trust all and continue");
    s.keys("enter");
    s.assert_contains("Trusting hooks...");
}

// ORACLE startup_hooks_review.rs:463 non_trust_startup_hook_shortcuts_remain_immediate kind=ui owner=startup-prompts
#[test]
fn non_trust_startup_hook_shortcuts_remain_immediate() {
    for key in ["1", "3"] {
        let mut s = Screen::build(Owner::StartupPrompts)
            .size(80, 24)
            .entry("startup", "hooks_review")
            .open();
        s.keys(key);
        s.assert_absent("Hooks need review");
    }
}

// ORACLE startup_hooks_review.rs:484 renders_prompt kind=snap owner=startup-prompts snap=startup_hooks_review_prompt
#[test]
fn renders_prompt() {
    let s = Screen::build(Owner::StartupPrompts)
        .size(40, 24)
        .entry("startup", "hooks_review")
        .open();
    s.assert_snap("startup_hooks_review_prompt");
}

// ORACLE startup_hooks_review.rs:502 renders_prompt_with_trust_error kind=snap owner=startup-prompts snap=startup_hooks_review_prompt_with_trust_error
#[test]
fn renders_prompt_with_trust_error() {
    let s = Screen::build(Owner::StartupPrompts)
        .size(62, 24)
        .entry("startup", "hooks_review_error")
        .open();
    s.assert_snap("startup_hooks_review_prompt_with_trust_error");
}

// ORACLE startup_hooks_review.rs:522 picker_progress_blocks_selection kind=snap owner=startup-prompts snap=startup_hooks_picker_progress_40
#[test]
fn picker_progress_blocks_selection() {
    let mut s = Screen::build(Owner::StartupPrompts)
        .size(40, 24)
        .entry("startup", "hooks_trusting")
        .open();
    s.keys("enter");
    s.assert_contains("Trusting hooks...");
    s.assert_snap("startup_hooks_picker_progress_40");
}

// ---- oracle: unarchive_prompt_tests.rs (7 screen-level tests here; 0 state tests left to startup-prompts)
// ORACLE unarchive_prompt_tests.rs:32 resume_prompt_snapshot kind=snap owner=startup-prompts
#[test]
fn resume_prompt_snapshot() {
    let s = Screen::build(Owner::StartupPrompts)
        .size(80, 10)
        .entry("startup", "unarchive_resume")
        .open();
    s.assert_row(1, "  This conversation is archived");
    s.assert_row(2, "  019e72f4-e09a-70f2-b2c2-a153a57b8cc0");
    s.assert_row(3, "");
    s.assert_row(4, "› 1. Unarchive and resume");
    s.assert_row(5, "  2. Cancel");
    s.assert_row(6, "");
    s.assert_row(7, "  enter continue · esc cancel");
}

// ORACLE unarchive_prompt_tests.rs:49 fork_prompt_cancel_snapshot kind=snap owner=startup-prompts
#[test]
fn fork_prompt_cancel_snapshot() {
    let mut s = Screen::build(Owner::StartupPrompts)
        .size(80, 10)
        .entry("startup", "unarchive_fork")
        .open();
    s.keys("down");
    s.assert_row(1, "  This conversation is archived");
    s.assert_row(2, "  019e72f4-e09a-70f2-b2c2-a153a57b8cc0");
    s.assert_row(3, "");
    s.assert_row(4, "  1. Unarchive and fork");
    s.assert_row(5, "› 2. Cancel");
    s.assert_row(6, "");
    s.assert_row(7, "  enter continue · esc cancel");
    s.keys("enter");
    s.assert_absent("This conversation is archived");
}

// ORACLE unarchive_prompt_tests.rs:73 confirmation_requires_an_accept_key kind=ui owner=startup-prompts
#[test]
fn confirmation_requires_an_accept_key() {
    for key in ["esc", "n", "2", "ctrl+c", "ctrl+d"] {
        let mut s = Screen::build(Owner::StartupPrompts)
            .size(80, 10)
            .entry("startup", "unarchive_fork")
            .open();
        s.keys(key);
        s.assert_absent("This conversation is archived");
    }
    let mut s = Screen::build(Owner::StartupPrompts)
        .size(80, 10)
        .entry("startup", "unarchive_fork")
        .open();
    s.assert_contains("› 1. Unarchive and fork");
    s.keys("enter");
    s.assert_absent("This conversation is archived");
}

// ---- inline presentation ----------------------------------------------------

/// The app run inline (`--no-alt-screen`: a region of the normal screen that is as high as the
/// visible route) under the shell row `$ codex fork saved-session` and a blank row, with the
/// debug entry `startup/unarchive_fork` open: what the oracle's `render_inline` draws. The
/// terminal is as high as the prompt plus those two rows.
fn inline_unarchive(width: u16, height: u16) -> Screen {
    Screen::build(Owner::StartupPrompts)
        .size(width, height)
        .inline(&["$ codex fork saved-session", ""])
        .entry("startup", "unarchive_fork")
        .open()
}

// ORACLE unarchive_prompt_tests.rs:131 inline_prompt_preserves_shell_context_snapshot kind=snap owner=startup-prompts snap=inline_prompt
#[test]
fn inline_prompt_preserves_shell_context_snapshot() {
    inline_unarchive(80, 11).assert_snap("inline_prompt");
}

// ORACLE unarchive_prompt_tests.rs:136 narrow_inline_prompt_wraps_snapshot kind=snap owner=startup-prompts snap=narrow_inline_prompt
#[test]
fn narrow_inline_prompt_wraps_snapshot() {
    // The prompt is 12 rows high at 28 columns: a region that claimed more would scroll the shell row off.
    inline_unarchive(28, 14).assert_snap("narrow_inline_prompt");
}

// ORACLE unarchive_prompt_tests.rs:141 inline_prompt_only_claims_content_height kind=ui owner=startup-prompts
#[test]
fn inline_prompt_only_claims_content_height() {
    kit::na(
        Owner::StartupPrompts,
        "asserts the Tui viewport rect of the inline guard, not visible",
    );
}

// ORACLE unarchive_prompt_tests.rs:168 alternate_prompt_restores_original_viewport kind=ui owner=startup-prompts
#[test]
fn alternate_prompt_restores_original_viewport() {
    kit::na(
        Owner::StartupPrompts,
        "asserts the Tui viewport rect after the guard drops, not visible",
    );
}

// ---- oracle: oss_selection.rs (1 screen-level tests here; 1 state tests left to startup-prompts)
// ORACLE oss_selection.rs:399 ctrl_h_l_move_provider_selection kind=ui owner=startup-prompts
#[test]
fn ctrl_h_l_move_provider_selection() {
    let mut s = Screen::build(Owner::StartupPrompts)
        .size(80, 24)
        .entry("startup", "oss_unknown")
        .open();
    s.keys("ctrl+l");
    s.assert_contains("› 2. Ollama");
    s.keys("ctrl+h");
    s.assert_contains("› 1. LM Studio");
}

// ---- oracle: oss_selection_tests.rs (3 screen-level tests here; 0 state tests left to startup-prompts)
// ORACLE oss_selection_tests.rs:9 provider_picker_snapshots kind=snap owner=startup-prompts snap=codex_tui__oss_selection__presentation_tests__oss_providers_mixed_short snap=codex_tui__oss_selection__presentation_tests__oss_providers_running snap=codex_tui__oss_selection__presentation_tests__oss_providers_stopped snap=codex_tui__oss_selection__presentation_tests__oss_providers_unknown_narrow
#[test]
fn provider_picker_snapshots() {
    for (entry, width, height, keys, snap) in [
        ("oss_stopped", 80, 24, "", "oss_providers_stopped"),
        ("oss_running", 80, 24, "down", "oss_providers_running"),
        ("oss_unknown", 40, 16, "", "oss_providers_unknown_narrow"),
        ("oss_mixed", 28, 12, "down", "oss_providers_mixed_short"),
    ] {
        let mut s = Screen::build(Owner::StartupPrompts)
            .size(width, height)
            .entry("startup", entry)
            .open();
        if !keys.is_empty() {
            s.keys(keys);
        }
        // The oracle's labels (oss_selection.rs:216-228): the hotkey letter
        // underlined, the status green, red or yellow.
        match snap {
            "oss_providers_stopped" => {
                s.assert_text_style("L", "underlined");
                s.assert_text_style("\u{25cb} Not running", "fg=Red");
            }
            "oss_providers_running" => s.assert_text_style("\u{25cf} Running", "fg=Green"),
            "oss_providers_unknown_narrow" => s.assert_text_style("? Unknown", "fg=Yellow"),
            _ => {}
        }
        if snap == "oss_providers_mixed_short" {
            kit::pending(
                Owner::StartupPrompts,
                "G50: at 28x12 the free space is 2 rows for five flex children; the oracle's `FlexRenderable` gives it to its LAST flex child (no top blank row, no subtitle), rtwk's `Flexible` loose hands the remainder to the first ones (a blank row and `Choose a local AI server` stay). The label hanging indent is drawn as the oracle's",
            );
        }
        s.assert_snap(snap);
    }
}

// ORACLE oss_selection_tests.rs:55 navigation_wraps_without_confirming kind=ui owner=startup-prompts
#[test]
fn navigation_wraps_without_confirming() {
    for key in ["up", "down", "j", "k"] {
        let mut s = Screen::build(Owner::StartupPrompts)
            .size(80, 24)
            .entry("startup", "oss_running")
            .open();
        s.keys(key);
        s.assert_contains("› 2. Ollama");
        s.keys(key);
        s.assert_contains("› 1. LM Studio");
        s.assert_contains("Select an open-source provider");
    }
}

// ORACLE oss_selection_tests.rs:71 shortcuts_select_the_provider_even_when_unavailable kind=ui owner=startup-prompts
#[test]
fn shortcuts_select_the_provider_even_when_unavailable() {
    for key in ["1", "2"] {
        let mut s = Screen::build(Owner::StartupPrompts)
            .size(80, 24)
            .entry("startup", "oss_stopped")
            .open();
        s.keys(key);
        s.assert_absent("Select an open-source provider");
    }
}

// ---- oracle: daemon_recovery_tests.rs (1 screen-level tests here; 0 state tests left to startup-prompts)
// ORACLE daemon_recovery_tests.rs:11 daemon_recovery_requires_explicit_restart_and_defaults_to_cancel kind=snap owner=startup-prompts snap=daemon_recovery_menu snap=non_restartable_daemon_recovery_menu snap=unmanaged_daemon_recovery_menu
#[test]
fn daemon_recovery_requires_explicit_restart_and_defaults_to_cancel() {
    for (entry, snap, restartable) in [
        ("daemon_menu", "daemon_recovery_menu", true),
        ("daemon_unmanaged", "unmanaged_daemon_recovery_menu", false),
        (
            "daemon_non_restartable",
            "non_restartable_daemon_recovery_menu",
            false,
        ),
    ] {
        let mut s = Screen::build(Owner::StartupPrompts)
            .size(90, 24)
            .entry("startup", entry)
            .open();
        s.assert_snap(snap);
        s.keys("enter");
        s.assert_absent("background server");
        let mut s = Screen::build(Owner::StartupPrompts)
            .size(90, 24)
            .entry("startup", entry)
            .open();
        s.keys("2");
        if restartable {
            s.assert_contains("Background server has incompatible feature settings");
            s.keys("enter");
        }
        s.assert_absent("ackground server");
    }
}
