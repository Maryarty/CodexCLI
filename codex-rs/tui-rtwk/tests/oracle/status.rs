//! Shard `status` (screens S19, owner dialogs-pane): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen};

// ---- oracle: status/helpers.rs (4 screen-level tests here; 3 state tests left to dialogs-pane)
// ORACLE status/helpers.rs:202 plan_type_display_name_remaps_display_labels kind=snap owner=dialogs-pane snap=education_plan_display_names
#[test]
fn plan_type_display_name_remaps_display_labels() {
    kit::na(
        Owner::DialogsPane,
        "asserts plan_type_display_name's return values (inline snapshots of a helper), not rows; a state test, carried in core/tests/status.rs `plan_type_display_name_remaps_display_labels`",
    );
}

// ORACLE status/helpers.rs:242 format_directory_display_truncates_halfwidth_sound_marks kind=snap owner=dialogs-pane
#[test]
fn format_directory_display_truncates_halfwidth_sound_marks() {
    kit::na(
        Owner::DialogsPane,
        "asserts format_directory_display's return for a max_width (an inline snapshot of a helper), not rows: a state test; not carried, the card never passes a max_width (the oracle passes None), a long path is cut by the Text's wrap (kana: G44)",
    );
}

// ORACLE status/helpers.rs:267 compose_agents_summary_collapses_home_and_preserves_project_relative_paths kind=snap owner=dialogs-pane
#[test]
fn compose_agents_summary_collapses_home_and_preserves_project_relative_paths() {
    kit::na(
        Owner::DialogsPane,
        "asserts compose_agents_summary's return (an inline snapshot of a helper), not rows; a state test, carried in core/tests/status.rs `compose_agents_summary_collapses_home_and_preserves_project_relative_paths`",
    );
}

// ORACLE status/helpers.rs:306 compose_agents_summary_shows_relative_native_and_full_foreign_paths kind=snap owner=dialogs-pane
#[test]
fn compose_agents_summary_shows_relative_native_and_full_foreign_paths() {
    kit::na(
        Owner::DialogsPane,
        "asserts compose_agents_summary's return (an inline snapshot of a helper), not rows; a state test, carried in core/tests/status.rs `compose_agents_summary_shows_relative_native_and_full_foreign_paths`",
    );
}

// ---- oracle: status/rate_limits.rs (1 screen-level tests here; 1 state tests left to dialogs-pane)
// ORACLE status/rate_limits.rs:468 non_codex_single_limit_renders_combined_row kind=ui owner=dialogs-pane
#[test]
fn non_codex_single_limit_renders_combined_row() {
    kit::na(
        Owner::DialogsPane,
        "asserts the row labels of compose_rate_limit_data_many's return (an equality check on a helper), not rows; a state test, carried in core/tests/status.rs `non_codex_single_limit_renders_combined_row`",
    );
}

// ---- oracle: status/tests.rs (29 screen-level tests here; 13 state tests left to dialogs-pane)
// ORACLE status/tests.rs:270 status_snapshot_includes_reasoning_details kind=snap owner=dialogs-pane snap=status_snapshot_includes_reasoning_details
#[test]
fn status_snapshot_includes_reasoning_details() {
    // The oracle renders the card with display_lines(80) (no terminal size: 80x24).
    Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .entry("status", "reasoning_details")
        .open()
        .assert_snap("status_snapshot_includes_reasoning_details");
}

// ORACLE status/tests.rs:352 status_snapshot_shows_chatgpt_plan_without_email kind=snap owner=dialogs-pane snap=status_snapshot_shows_chatgpt_plan_without_email
#[test]
fn status_snapshot_shows_chatgpt_plan_without_email() {
    Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .entry("status", "chatgpt_plan_without_email")
        .open()
        .assert_snap("status_snapshot_shows_chatgpt_plan_without_email");
}

// ORACLE status/tests.rs:724 status_snapshot_shows_active_user_defined_profile kind=snap owner=dialogs-pane snap=status_snapshot_shows_active_user_defined_profile
#[test]
fn status_snapshot_shows_active_user_defined_profile() {
    Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .entry("status", "active_user_defined_profile")
        .open()
        .assert_snap("status_snapshot_shows_active_user_defined_profile");
}

// ORACLE status/tests.rs:771 status_uses_server_provider_id_and_auth_requirement kind=snap owner=dialogs-pane snap=status_server_auth_not_required snap=status_server_auth_required
#[test]
fn status_uses_server_provider_id_and_auth_requirement() {
    Screen::build(Owner::DialogsPane)
        .size(120, 24)
        .entry("status", "server_auth_not_required")
        .open()
        .assert_snap("status_server_auth_not_required");
    let s = Screen::build(Owner::DialogsPane)
        .size(120, 24)
        .entry("status", "server_auth_required")
        .open();
    s.assert_snap("status_server_auth_required");
    // The oracle walks the card's hyperlinks at widths 24, 42 and 120: the cells linked,
    // read in order, spell the usage URL, and every link goes to it.
    for width in [24u16, 42, 120] {
        let s = Screen::build(Owner::DialogsPane)
            .size(width, 60)
            .entry("status", "server_auth_required")
            .open();
        s.assert_links_spell("https://chatgpt.com/settings/usage");
    }
}

// ORACLE status/tests.rs:879 status_snapshot_shows_auto_review_permissions kind=snap owner=dialogs-pane snap=status_snapshot_shows_auto_review_permissions
#[test]
fn status_snapshot_shows_auto_review_permissions() {
    Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .entry("status", "auto_review_permissions")
        .open()
        .assert_snap("status_snapshot_shows_auto_review_permissions");
}

// ORACLE status/tests.rs:973 status_snapshot_includes_forked_from kind=snap owner=dialogs-pane snap=status_snapshot_includes_forked_from
#[test]
fn status_snapshot_includes_forked_from() {
    Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .entry("status", "forked_from")
        .open()
        .assert_snap("status_snapshot_includes_forked_from");
}

// ORACLE status/tests.rs:1027 status_snapshot_includes_monthly_limit kind=snap owner=dialogs-pane snap=status_snapshot_includes_monthly_limit
#[test]
fn status_snapshot_includes_monthly_limit() {
    Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .entry("status", "monthly_limit")
        .open()
        .assert_snap("status_snapshot_includes_monthly_limit");
}

// ORACLE status/tests.rs:1093 status_snapshot_includes_enterprise_monthly_credit_limit kind=snap owner=dialogs-pane snap=status_snapshot_includes_enterprise_monthly_credit_limit snap=status_snapshot_wraps_enterprise_monthly_credit_details_in_narrow_terminal
#[test]
fn status_snapshot_includes_enterprise_monthly_credit_limit() {
    // The narrow half differs in two wraps of one value each: `100% left (1.2K used / 272K)`
    // breaks before `used` (the oracle's UAX #14 words keep `used / ` together, `TextWrap::Word`
    // breaks before the `/`: G75), and `(resets 07:08 on 7 May)` breaks before `7 May)` (the
    // oracle's `textwrap::wrap` is optimal-fit, `TextWrap::Word` is first-fit: G76). The 92-column
    // half is the oracle's rows.
    kit::delta(
        Owner::DialogsPane,
        "G75 + G76 wrap: at 46 columns the Context window row breaks before `used` where the oracle keeps `used /` whole, and the reset row breaks before `7 May)` (first-fit against optimal-fit); the letters are the same, in order",
    );
    Screen::build(Owner::DialogsPane)
        .size(92, 24)
        .entry("status", "enterprise_monthly_credit")
        .open()
        .assert_snap("status_snapshot_includes_enterprise_monthly_credit_limit");
    Screen::build(Owner::DialogsPane)
        .size(46, 40)
        .entry("status", "enterprise_monthly_credit")
        .open()
        .assert_snap("status_snapshot_wraps_enterprise_monthly_credit_details_in_narrow_terminal");
}

// ORACLE status/tests.rs:1171 status_snapshot_uses_generic_limit_labels_for_unsupported_windows kind=snap owner=dialogs-pane snap=status_snapshot_uses_generic_limit_labels_for_unsupported_windows
#[test]
fn status_snapshot_uses_generic_limit_labels_for_unsupported_windows() {
    Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .entry("status", "generic_limit_labels")
        .open()
        .assert_snap("status_snapshot_uses_generic_limit_labels_for_unsupported_windows");
}

// ORACLE status/tests.rs:1241 status_snapshot_shows_unlimited_credits kind=ui owner=dialogs-pane
#[test]
fn status_snapshot_shows_unlimited_credits() {
    // The oracle asserts one row holds both `Credits:` and `Unlimited`; the kit has no
    // same-row contains, so each literal is checked on the screen.
    let s = Screen::build(Owner::DialogsPane)
        .size(120, 24)
        .entry("status", "unlimited_credits")
        .open();
    s.assert_contains("Credits:");
    s.assert_contains("Unlimited");
}

// ORACLE status/tests.rs:1294 status_snapshot_shows_positive_credits kind=ui owner=dialogs-pane
#[test]
fn status_snapshot_shows_positive_credits() {
    // The oracle asserts one row holds both `Credits:` and `13 credits`; see the note above.
    let s = Screen::build(Owner::DialogsPane)
        .size(120, 24)
        .entry("status", "positive_credits")
        .open();
    s.assert_contains("Credits:");
    s.assert_contains("13 credits");
}

// ORACLE status/tests.rs:1347 status_snapshot_shows_available_credits_without_display_balance kind=ui owner=dialogs-pane
#[test]
fn status_snapshot_shows_available_credits_without_display_balance() {
    for entry in [
        "credits_available_none",
        "credits_available_empty",
        "credits_available_zero",
        "credits_available_not_a_number",
        "credits_available_inf",
    ] {
        let s = Screen::build(Owner::DialogsPane)
            .size(120, 24)
            .entry("status", entry)
            .open();
        s.assert_contains("Credits:");
        s.assert_contains("Available");
    }
}

// ORACLE status/tests.rs:1408 status_snapshot_respects_unlimited_without_has_credits_flag kind=ui owner=dialogs-pane
#[test]
fn status_snapshot_respects_unlimited_without_has_credits_flag() {
    let s = Screen::build(Owner::DialogsPane)
        .size(120, 24)
        .entry("status", "unlimited_without_flag")
        .open();
    s.assert_contains("Credits:");
    s.assert_contains("Unlimited");
}

// ORACLE status/tests.rs:1461 status_card_token_usage_excludes_cached_tokens kind=ui owner=dialogs-pane
#[test]
fn status_card_token_usage_excludes_cached_tokens() {
    Screen::build(Owner::DialogsPane)
        .size(120, 24)
        .entry("status", "cached_tokens")
        .open()
        .assert_absent("cached");
}

// ORACLE status/tests.rs:1507 status_wraps_long_paths_and_session_ids_without_losing_text kind=ui owner=dialogs-pane
#[test]
fn status_wraps_long_paths_and_session_ids_without_losing_text() {
    // The oracle checks each width's rows fit the width (the kit's screen is the width: a
    // row cannot be wider), then joins the rows trimmed: the path and the session id must be
    // whole in them at every width, wrapped or not. Its copy_text half is the clipboard
    // (chunk 4, `copy_text`).
    for width in [7u16, 12, 17, 18, 24, 25, 40, 80] {
        let s = Screen::build(Owner::DialogsPane)
            .size(width, 400)
            .entry("status", "long_path")
            .open();
        s.assert_joined_rows_hold("/workspace/projects/界界/ｶﾞﾞ/a-very-long-directory-name/codex");
        s.assert_joined_rows_hold("00000000-0000-0000-0000-000000000123");
    }
}

// ORACLE status/tests.rs:1561 status_snapshot_wraps_in_narrow_terminal kind=snap owner=dialogs-pane snap=status_snapshot_truncates_in_narrow_terminal
#[test]
fn status_snapshot_wraps_in_narrow_terminal() {
    Screen::build(Owner::DialogsPane)
        .size(70, 24)
        .entry("status", "narrow_terminal")
        .open()
        .assert_snap("status_snapshot_truncates_in_narrow_terminal");
}

// ORACLE status/tests.rs:1630 status_snapshot_wraps_halfwidth_kana_in_narrow_terminal kind=snap owner=dialogs-pane snap=status_snapshot_truncates_halfwidth_kana_in_narrow_terminal
#[test]
fn status_snapshot_wraps_halfwidth_kana_in_narrow_terminal() {
    // G44 is closed (`legacy_text`: the sound mark is a cell of its own, the rows' widths equal).
    // What is left: the oracle breaks an oversized token (`ｶﾞﾊﾟ..ﾊﾟ-model`, `ｶﾞﾊﾟ..@example.com`)
    // at its own break points (before `@`, between kana, never after the hyphen) where
    // `TextWrap::Word` breaks after the hyphen or by cells; and the account's plan is the free
    // text `ｶﾞﾊﾟ plan` there, the protocol's plan enum (Unknown) here.
    kit::pending(
        Owner::DialogsPane,
        "pending(G48): the Model row's oversized token breaks after the hyphen (`ｶﾞﾊﾟｶﾞﾊﾟｶﾞﾊﾟｶﾞﾊﾟ-`), the oracle's does not (`ｶﾞﾊﾟｶﾞﾊﾟｶﾞﾊﾟｶﾞ` / `ﾊﾟ-model`), and the Account row breaks by cells inside `@example.com` where the oracle breaks before `@`; the plan is the free text `ｶﾞﾊﾟ plan` in the oracle, the protocol's plan enum (Unknown) in the entry (a state the port cannot have)",
    );
    Screen::build(Owner::DialogsPane)
        .size(42, 24)
        // the oracle counts a halfwidth kana's sound mark as a cell of its own
        .legacy_text()
        .entry("status", "halfwidth_kana_narrow")
        .open()
        .assert_snap("status_snapshot_truncates_halfwidth_kana_in_narrow_terminal");
}

// ORACLE status/tests.rs:1670 status_snapshot_shows_missing_limits_message kind=snap owner=dialogs-pane snap=status_snapshot_shows_missing_limits_message
#[test]
fn status_snapshot_shows_missing_limits_message() {
    Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .entry("status", "missing_limits")
        .open()
        .assert_snap("status_snapshot_shows_missing_limits_message");
}

// ORACLE status/tests.rs:1718 status_snapshot_uses_default_reasoning_when_config_empty kind=snap owner=dialogs-pane snap=status_snapshot_local_background_server snap=status_snapshot_remote_server snap=status_snapshot_uses_default_reasoning_when_config_empty
#[test]
fn status_snapshot_uses_default_reasoning_when_config_empty() {
    for (entry, snapshot) in [
        (
            "default_reasoning",
            "status_snapshot_uses_default_reasoning_when_config_empty",
        ),
        ("remote_server", "status_snapshot_remote_server"),
        (
            "local_background_server",
            "status_snapshot_local_background_server",
        ),
    ] {
        Screen::build(Owner::DialogsPane)
            .size(80, 24)
            .entry("status", entry)
            .open()
            .assert_snap(snapshot);
    }
}

// ORACLE status/tests.rs:1787 status_snapshot_shows_refreshing_limits_notice kind=snap owner=dialogs-pane snap=status_snapshot_shows_refreshing_limits_notice
#[test]
fn status_snapshot_shows_refreshing_limits_notice() {
    Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .entry("status", "refreshing_limits")
        .open()
        .assert_snap("status_snapshot_shows_refreshing_limits_notice");
}

// ORACLE status/tests.rs:1855 transcript_overlay_remeasures_status_after_rate_limit_refresh kind=snap owner=dialogs-pane snap=transcript_overlay_status_rate_limit_refresh
#[test]
fn transcript_overlay_remeasures_status_after_rate_limit_refresh() {
    kit::pending(
        Owner::DialogsPane,
        "S16 pager and chunk 3 (the refresh handle): the transcript overlay (80x30) holding the /status card before and after a rate-limit refresh finishes has no entry (status/transcript_refresh_before, status/transcript_refresh_after)",
    );
    // The oracle's snapshot joins two frames: `before:` and `after:`.
    let before = Screen::build(Owner::DialogsPane)
        .size(80, 30)
        .entry("status", "transcript_refresh_before")
        .open();
    before.assert_snap_section("transcript_overlay_status_rate_limit_refresh", "before");
    let after = Screen::build(Owner::DialogsPane)
        .size(80, 30)
        .entry("status", "transcript_refresh_after")
        .open();
    after.assert_snap_section("transcript_overlay_status_rate_limit_refresh", "after");
    after.assert_contains("spark limit");
    after.assert_contains("5h limit");
    after.assert_contains("Weekly limit");
}

// ORACLE status/tests.rs:1940 status_snapshot_includes_credits_and_limits kind=snap owner=dialogs-pane snap=status_snapshot_includes_credits_and_limits
#[test]
fn status_snapshot_includes_credits_and_limits() {
    Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .entry("status", "credits_and_limits")
        .open()
        .assert_snap("status_snapshot_includes_credits_and_limits");
}

// ORACLE status/tests.rs:2017 status_snapshot_shows_unavailable_limits_message kind=snap owner=dialogs-pane snap=status_snapshot_shows_unavailable_limits_message
#[test]
fn status_snapshot_shows_unavailable_limits_message() {
    Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .entry("status", "unavailable_limits")
        .open()
        .assert_snap("status_snapshot_shows_unavailable_limits_message");
}

// ORACLE status/tests.rs:2078 status_snapshot_treats_refreshing_empty_limits_as_unavailable kind=ui owner=dialogs-pane
#[test]
fn status_snapshot_treats_refreshing_empty_limits_as_unavailable() {
    Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .entry("status", "refreshing_empty_limits")
        .open()
        .assert_contains("Limits:          not available for this account");
}

// ORACLE status/tests.rs:2139 status_snapshot_shows_stale_limits_message kind=snap owner=dialogs-pane snap=status_snapshot_shows_stale_limits_message
#[test]
fn status_snapshot_shows_stale_limits_message() {
    Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .entry("status", "stale_limits")
        .open()
        .assert_snap("status_snapshot_shows_stale_limits_message");
}

// ORACLE status/tests.rs:2209 status_snapshot_cached_limits_hide_credits_without_flag kind=snap owner=dialogs-pane snap=status_snapshot_cached_limits_hide_credits_without_flag
#[test]
fn status_snapshot_cached_limits_hide_credits_without_flag() {
    Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .entry("status", "cached_limits_hide_credits")
        .open()
        .assert_snap("status_snapshot_cached_limits_hide_credits_without_flag");
}

// ORACLE status/tests.rs:2283 status_context_window_uses_last_usage kind=ui owner=dialogs-pane
#[test]
fn status_context_window_uses_last_usage() {
    let s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .entry("status", "context_last_usage")
        .open();
    s.assert_contains("13.7K used / 272K");
    // The oracle checks the Context window row only; the whole screen holds no total.
    s.assert_absent("102K");
}

// ORACLE status/tests.rs:2347 status_permissions_include_executor_profile_root kind=snap owner=dialogs-pane
#[test]
fn status_permissions_include_executor_profile_root() {
    kit::na(
        Owner::DialogsPane,
        "asserts permissions_text_for_width's return (an inline snapshot of a helper, not a card row), not rows; a state test, carried in core/tests/status.rs `status_permissions_include_executor_profile_root`",
    );
}

// ORACLE status/tests.rs:2380 reset_timestamps_follow_clock_preference kind=snap owner=dialogs-pane
#[test]
fn reset_timestamps_follow_clock_preference() {
    kit::na(
        Owner::DialogsPane,
        "asserts format_reset_timestamp's return for each clock format (an inline snapshot of a helper), not rows; a state test, carried in core/tests/status.rs `reset_timestamps_follow_clock_preference`",
    );
}
