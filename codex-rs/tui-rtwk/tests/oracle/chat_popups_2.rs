//! Shard `chat_popups_2` (screens S14, owner dialogs-pane): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use serde_json::json;

use crate::chat_popups_3::{plugin, seed, select_tab};
use crate::kit::{self, Owner, Screen};

/// The first row that contains `text`; past the screen when none does (the kit then names the row it looked for).
fn row_of(s: &Screen, text: &str) -> usize {
    s.rows()
        .iter()
        .position(|r| r.contains(text))
        .unwrap_or(usize::MAX)
}

/// The first row that starts with `text`: a history cell's row (`■`, `•`).
fn row_starting(s: &Screen, text: &str) -> usize {
    s.rows()
        .iter()
        .position(|r| r.starts_with(text))
        .unwrap_or(usize::MAX)
}

const NO_DISCOVERY: &str = "the discovery menus (loading, the server's catalog, failure with Retry, empty) are built (core `select::permissions`, entries `permission_discovery_*`, the 30 state tests of core/tests/permissions.rs, the sheet tests) but `/permissions` does not open them: the opener (src/pages/session) and the app's `permissionProfile/list` + `configRequirements/read` routing are other owners' files (parity/popups.md, Needs); the wait behind a covering modal is the app's hold, not the picker's";
const NO_WINDOWS: &str = "the Windows sandbox prompts, setup status and degraded-sandbox labels are not built (the oracle runs these tests on Windows only)";
const NO_FULL_ACCESS: &str = "the confirmation's rows are built (`Enable full access?`, Yes, continue anyway / Cancel, the cyber text) but its header is plain subtitle text (the oracle's header is a Paragraph: bold title, the caution sentence in red) and its Centered presentation over the retained composer (`› As` / `GP` at the left edge) is not in the Picker brick; the live open waits on the /permissions opener (parity/popups.md, Needs)";
const NO_HISTORY: &str = "the /permissions selection history cells (`Permissions updated to ...`) are not built: no transcript cell comes from a permissions pick";
const NO_AUTO_REVIEW: &str = "the Approve for me row, its feature gate and its (current) marker are built (core `select::permissions`, `Current.guardian`/`reviewer`, entry `permissions_unnamed_server_profile`) but `/permissions` does not open the menu: the opener (src/pages/session) reads the thread's policy and the feature gate (parity/popups.md, Needs)";
const NO_QUEUE: &str =
    "the queued-input recovery after a cancelled /permissions picker is not built";

// ---- oracle: chatwidget/tests/permissions.rs (42 screen-level tests here; 11 state tests left to dialogs-pane)
// ORACLE chatwidget/tests/permissions.rs:41 permission_discovery_uses_server_catalog_for_remote_custom_selection kind=snap owner=dialogs-pane snap=permission_discovery_loading snap=permission_discovery_remote
#[test]
fn permission_discovery_uses_server_catalog_for_remote_custom_selection() {
    kit::pending(Owner::DialogsPane, NO_DISCOVERY);
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.type_text("/permissions").keys("enter");
    s.assert_snap_region(
        "permission_discovery_loading",
        row_of(&s, "Update Model Permissions"),
        0,
    );
    s.resize(110, 24);
    s.assert_snap_region(
        "permission_discovery_remote",
        row_of(&s, "Update Model Permissions"),
        0,
    );
    // The server-only alternative is routed to the app, not resolved by the popup.
    s.keys("2");
    s.assert_absent("Update Model Permissions");
}

// ORACLE chatwidget/tests/permissions.rs:92 permissions_command_waits_for_covering_modal_before_selecting_profile kind=ui owner=dialogs-pane
#[test]
fn permissions_command_waits_for_covering_modal_before_selecting_profile() {
    kit::pending(Owner::DialogsPane, NO_DISCOVERY);
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.type_text("/permissions read-only").keys("enter");
    // The oracle opens another modal first: the picker waits behind it.
    s.assert_contains("Other modal");
    s.keys("esc");
    s.assert_absent("Update Model Permissions");
}

// ORACLE chatwidget/tests/permissions.rs:122 cancelled_permissions_command_does_not_apply_after_later_modal kind=ui owner=dialogs-pane
#[test]
fn cancelled_permissions_command_does_not_apply_after_later_modal() {
    kit::pending(Owner::DialogsPane, NO_QUEUE);
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.type_text("/permissions read-only").keys("enter");
    s.keys("esc");
    s.assert_absent("Update Model Permissions");
    // The queued follow-up comes back to the composer once the picker closes.
    s.assert_contains("follow-up");
}

// ORACLE chatwidget/tests/permissions.rs:153 permissions_command_preserves_full_access_confirmation kind=ui owner=dialogs-pane
#[test]
fn permissions_command_preserves_full_access_confirmation() {
    kit::pending(Owner::DialogsPane, NO_FULL_ACCESS);
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.type_text("/permissions danger-full-access --reviewer=auto_review")
        .keys("enter");
    s.assert_contains("Enable full access?");
    // Cancel returns to the picker.
    s.keys("down enter");
    s.assert_contains("Update Model Permissions");
    s.keys("esc");
    s.assert_contains("follow-up");
}

// ORACLE chatwidget/tests/permissions.rs:199 permissions_command_rejects_disabled_profile kind=snap owner=dialogs-pane
#[test]
fn permissions_command_rejects_disabled_profile() {
    kit::pending(
        Owner::DialogsPane,
        "the `/permissions <profile>` argument path and its `unavailable` history cell are not built (no requirement-disabled profile seed)",
    );
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.type_text("/permissions read-only").keys("enter");
    s.assert_snap_region(
        "permissions_command_disabled_profile",
        row_starting(&s, "■"),
        0,
    );
}

// ORACLE chatwidget/tests/permissions.rs:229 permissions_command_rejects_unknown_profile kind=snap owner=dialogs-pane
#[test]
fn permissions_command_rejects_unknown_profile() {
    kit::pending(
        Owner::DialogsPane,
        "the `/permissions <profile>` argument path and its unknown-profile history cell are not built",
    );
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.type_text("/permissions unknown-profile").keys("enter");
    s.assert_snap_region(
        "permissions_command_unknown_profile",
        row_starting(&s, "■"),
        0,
    );
}

// ORACLE chatwidget/tests/permissions.rs:269 inline_permission_discovery_retry_preserves_profile_id kind=ui owner=dialogs-pane
#[test]
fn inline_permission_discovery_retry_preserves_profile_id() {
    kit::pending(Owner::DialogsPane, NO_DISCOVERY);
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.type_text("/permissions server-only").keys("enter");
    s.assert_contains("Failed to load permissions: disconnected");
    // Enter retries the load; the retry keeps the profile id the picker was opened with.
    s.keys("enter");
    s.assert_contains("Update Model Permissions");
}

// ORACLE chatwidget/tests/permissions.rs:296 permission_discovery_discards_stale_results_and_preserves_covering_modal kind=ui owner=dialogs-pane
#[test]
fn permission_discovery_discards_stale_results_and_preserves_covering_modal() {
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.type_text("/permissions").keys("enter");
    s.keys("esc");
    s.assert_absent("Update Model Permissions");
    s.type_text("/permissions").keys("enter");
    s.keys("esc");
    s.assert_absent("Update Model Permissions");
}

// ORACLE chatwidget/tests/permissions.rs:349 permission_discovery_failure_can_retry_and_empty_catalog_keeps_current_profile kind=snap owner=dialogs-pane snap=permission_discovery_empty snap=permission_discovery_failure
#[test]
fn permission_discovery_failure_can_retry_and_empty_catalog_keeps_current_profile() {
    kit::pending(Owner::DialogsPane, NO_DISCOVERY);
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.type_text("/permissions").keys("enter");
    s.assert_snap_region(
        "permission_discovery_failure",
        row_of(&s, "Update Model Permissions"),
        0,
    );
    // Retry: the load comes back with an empty catalog, the current profile is kept.
    s.keys("enter");
    s.resize(100, 24);
    s.assert_snap_region(
        "permission_discovery_empty",
        row_of(&s, "Update Model Permissions"),
        0,
    );
}

// ORACLE chatwidget/tests/permissions.rs:442 approvals_selection_popup_snapshot kind=snap owner=dialogs-pane snap=approvals_selection_popup
#[test]
fn approvals_selection_popup_snapshot() {
    let s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .entry("select", "permissions_menu")
        .open();
    s.assert_snap_region(
        "approvals_selection_popup",
        row_of(&s, "Update Model Permissions"),
        0,
    );
}

// ORACLE chatwidget/tests/permissions.rs:458 profile_permissions_selection_popup_snapshot kind=snap owner=dialogs-pane snap=profile_permissions_selection_popup
#[test]
fn profile_permissions_selection_popup_snapshot() {
    let s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .entry("select", "profile_permissions")
        .open();
    s.assert_snap_region(
        "profile_permissions_selection_popup",
        row_of(&s, "Update Model Permissions"),
        0,
    );
}

// ORACLE chatwidget/tests/permissions.rs:478 profile_permissions_selection_popup_with_disallowed_full_access_snapshot kind=snap owner=dialogs-pane snap=profile_permissions_selection_popup_with_disallowed_full_access
#[test]
fn profile_permissions_selection_popup_with_disallowed_full_access_snapshot() {
    let s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .entry("select", "profile_permissions_disallowed_full_access")
        .open();
    s.assert_snap_region(
        "profile_permissions_selection_popup_with_disallowed_full_access",
        row_of(&s, "Update Model Permissions"),
        0,
    );
}

// ORACLE chatwidget/tests/permissions.rs:505 profile_permissions_selection_popup_with_custom_profiles_snapshot kind=snap owner=dialogs-pane snap=profile_permissions_selection_popup_with_custom_profiles
#[test]
fn profile_permissions_selection_popup_with_custom_profiles_snapshot() {
    let s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .entry("select", "profile_permissions_custom")
        .open();
    s.assert_snap_region(
        "profile_permissions_selection_popup_with_custom_profiles",
        row_of(&s, "Update Model Permissions"),
        0,
    );
}

// ORACLE chatwidget/tests/permissions.rs:537 profile_permissions_selection_emits_named_profile_event_only kind=ui owner=dialogs-pane
#[test]
fn profile_permissions_selection_emits_named_profile_event_only() {
    kit::na(
        Owner::DialogsPane,
        "asserts the one SelectPermissionProfile event Enter sends (named profile, no approval policy) and its display label; no drawn text is asserted",
    );
}

// ORACLE chatwidget/tests/permissions.rs:580 profile_permissions_selection_emits_active_custom_profile kind=ui owner=dialogs-pane
#[test]
fn profile_permissions_selection_emits_active_custom_profile() {
    kit::na(
        Owner::DialogsPane,
        "asserts the one SelectPermissionProfile event Enter sends for the active custom profile; no drawn text is asserted",
    );
}

// ORACLE chatwidget/tests/permissions.rs:613 profile_permissions_selection_emits_auto_review_mode_event kind=ui owner=dialogs-pane
#[test]
fn profile_permissions_selection_emits_auto_review_mode_event() {
    kit::na(
        Owner::DialogsPane,
        "asserts the SelectPermissionProfile event with the auto-review reviewer; no drawn text is asserted",
    );
}

// ORACLE chatwidget/tests/permissions.rs:760 profile_permissions_full_access_always_opens_confirmation kind=ui owner=dialogs-pane
#[test]
fn profile_permissions_full_access_always_opens_confirmation() {
    kit::na(
        Owner::DialogsPane,
        "asserts exactly one OpenFullAccessConfirmation event on Enter from the profile popup; the confirmation's drawn text is not asserted",
    );
}

// ORACLE chatwidget/tests/permissions.rs:792 approvals_selection_popup_snapshot_windows_degraded_sandbox kind=ui owner=dialogs-pane
#[test]
fn approvals_selection_popup_snapshot_windows_degraded_sandbox() {
    kit::pending(Owner::DialogsPane, NO_WINDOWS);
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.type_text("/permissions").keys("enter");
    s.assert_contains("Ask for approval (non-admin sandbox)");
    s.assert_contains("/setup-default-sandbox");
    s.assert_contains("non-admin sandbox");
}

// ORACLE chatwidget/tests/permissions.rs:890 full_access_confirmation_popup_snapshot kind=snap owner=dialogs-pane snap=full_access_confirmation_popup
#[test]
fn full_access_confirmation_popup_snapshot() {
    kit::pending(Owner::DialogsPane, NO_FULL_ACCESS);
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .entry("select", "permissions_menu")
        .open();
    s.keys("down enter");
    s.assert_snap_region(
        "full_access_confirmation_popup",
        row_of(&s, "Enable full access?"),
        0,
    );
}

// ORACLE chatwidget/tests/permissions.rs:907 windows_auto_mode_prompt_requests_enabling_sandbox_feature kind=ui owner=dialogs-pane
#[test]
fn windows_auto_mode_prompt_requests_enabling_sandbox_feature() {
    kit::pending(Owner::DialogsPane, NO_WINDOWS);
    let mut s = Screen::build(Owner::DialogsPane)
        .size(120, 24)
        .live()
        .open();
    s.type_text("/permissions").keys("enter");
    s.assert_contains("requires Administrator permissions");
    s.assert_contains("Use non-admin sandbox");
}

// ORACLE chatwidget/tests/permissions.rs:928 startup_prompts_for_windows_sandbox_when_agent_requested kind=snap owner=dialogs-pane snap=projectless_windows_sandbox_setup
#[test]
fn startup_prompts_for_windows_sandbox_when_agent_requested() {
    kit::pending(Owner::DialogsPane, NO_WINDOWS);
    let s = Screen::build(Owner::DialogsPane)
        .size(120, 24)
        .live()
        .open();
    s.assert_contains("requires Administrator permissions");
    s.assert_contains("Set up default sandbox");
    s.assert_contains("Use non-admin sandbox");
    s.assert_contains("Quit");
    s.assert_snap_region(
        "projectless_windows_sandbox_setup",
        row_of(&s, "Set up the Codex agent sandbox"),
        0,
    );
}

// ORACLE chatwidget/tests/permissions.rs:991 startup_windows_sandbox_prompt_blocks_disallowed_unelevated_fallback kind=ui owner=dialogs-pane
#[test]
fn startup_windows_sandbox_prompt_blocks_disallowed_unelevated_fallback() {
    kit::pending(Owner::DialogsPane, NO_WINDOWS);
    let s = Screen::build(Owner::DialogsPane)
        .size(120, 24)
        .live()
        .open();
    s.assert_contains("Your organization requires the default Codex agent sandbox");
    s.assert_absent("Use non-admin sandbox");
}

// ORACLE chatwidget/tests/permissions.rs:1011 windows_sandbox_help_links_keep_complete_wrapped_destinations kind=snap owner=dialogs-pane
#[test]
fn windows_sandbox_help_links_keep_complete_wrapped_destinations() {
    kit::pending(Owner::DialogsPane, NO_WINDOWS);
    let s = Screen::build(Owner::DialogsPane).size(40, 24).live().open();
    s.assert_snap("windows_sandbox_wrapped_help_url");
}

// ORACLE chatwidget/tests/permissions.rs:1054 windows_sandbox_required_enable_prompt_snapshot kind=snap owner=dialogs-pane snap=windows_sandbox_required_enable_prompt
#[test]
fn windows_sandbox_required_enable_prompt_snapshot() {
    kit::pending(Owner::DialogsPane, NO_WINDOWS);
    let s = Screen::build(Owner::DialogsPane)
        .size(120, 24)
        .live()
        .open();
    s.assert_snap_region(
        "windows_sandbox_required_enable_prompt",
        row_of(&s, "Set up the Codex agent sandbox"),
        0,
    );
}

// ORACLE chatwidget/tests/permissions.rs:1072 windows_sandbox_required_enable_prompt_reopens_on_cancel_when_unelevated_allowed kind=ui owner=dialogs-pane
#[test]
fn windows_sandbox_required_enable_prompt_reopens_on_cancel_when_unelevated_allowed() {
    kit::na(
        Owner::DialogsPane,
        "asserts the OpenWindowsSandboxEnablePrompt event Esc sends (the prompt reopening); no drawn text is asserted",
    );
}

// ORACLE chatwidget/tests/permissions.rs:1095 fragmented_terminal_response_cannot_select_non_admin_windows_sandbox kind=ui owner=dialogs-pane
#[test]
fn fragmented_terminal_response_cannot_select_non_admin_windows_sandbox() {
    kit::pending(Owner::DialogsPane, NO_WINDOWS);
    let mut s = Screen::build(Owner::DialogsPane)
        .size(120, 24)
        .live()
        .open();
    // The fragment is typed into the prompt: the prompt stays on screen and no choice is made.
    s.type_text("20;rgb:2222/ffff/ffff");
    s.assert_contains("Use non-admin sandbox");
    // Enter then chooses the non-admin sandbox; the oracle asserts that event.
}

// ORACLE chatwidget/tests/permissions.rs:1130 windows_sandbox_setup_starts_a_fresh_status_clock kind=snap owner=dialogs-pane snap=windows_sandbox_setup_fresh_clock
#[test]
fn windows_sandbox_setup_starts_a_fresh_status_clock() {
    kit::pending(Owner::DialogsPane, NO_WINDOWS);
    let s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.assert_contains("Setting up sandbox");
    s.assert_contains("(0s");
    // The snapshot `windows_sandbox_setup_fresh_clock` holds `[CWD]` for the test cwd; it is not
    // compared as is. The working status after the clear is not reached here.
}

// ORACLE chatwidget/tests/permissions.rs:1250 windows_sandbox_required_fallback_prompt_snapshot kind=snap owner=dialogs-pane snap=windows_sandbox_required_fallback_prompt
#[test]
fn windows_sandbox_required_fallback_prompt_snapshot() {
    kit::pending(Owner::DialogsPane, NO_WINDOWS);
    let s = Screen::build(Owner::DialogsPane)
        .size(120, 24)
        .live()
        .open();
    s.assert_snap_region(
        "windows_sandbox_required_fallback_prompt",
        row_of(
            &s,
            "Your organization requires the default Codex agent sandbox",
        ),
        0,
    );
}

// ORACLE chatwidget/tests/permissions.rs:1280 approvals_popup_shows_disabled_presets kind=ui owner=dialogs-pane
#[test]
fn approvals_popup_shows_disabled_presets() {
    let s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .entry("select", "permissions_constrained")
        .open();
    // The oracle collapses the whitespace: the reason wraps at 80 columns.
    s.assert_reads("(disabled)");
    s.assert_reads("this message should be printed in the description");
}

// ORACLE chatwidget/tests/permissions.rs:1317 approvals_popup_navigation_skips_disabled kind=ui owner=dialogs-pane
#[test]
fn approvals_popup_navigation_skips_disabled() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .entry("select", "permissions_constrained")
        .open();
    s.assert_contains("Update Model Permissions");
    // Ten Downs: the selection never lands on the disabled row.
    s.keys("down down down down down down down down down down");
    s.assert_contains("Update Model Permissions");
    // Enter selects an enabled preset; the popup closes.
    s.keys("enter");
    s.assert_absent("Update Model Permissions");
}

// ORACLE chatwidget/tests/permissions.rs:1413 permissions_selection_emits_history_cell_when_selection_changes kind=ui owner=dialogs-pane
#[test]
fn permissions_selection_emits_history_cell_when_selection_changes() {
    kit::pending(Owner::DialogsPane, NO_HISTORY);
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.type_text("/permissions").keys("enter");
    s.keys("down enter");
    s.assert_contains("Permissions updated to");
}

// ORACLE chatwidget/tests/permissions.rs:1438 permissions_selection_history_snapshot_after_mode_switch kind=snap owner=dialogs-pane snap=permissions_selection_history_after_mode_switch
#[test]
fn permissions_selection_history_snapshot_after_mode_switch() {
    kit::pending(Owner::DialogsPane, NO_HISTORY);
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.type_text("/permissions").keys("enter");
    s.keys("down enter");
    // The full-access confirmation: Enter confirms.
    s.keys("enter");
    s.assert_snap_region(
        "permissions_selection_history_after_mode_switch",
        row_of(&s, "Permissions updated to"),
        0,
    );
}

// ORACLE chatwidget/tests/permissions.rs:1474 permissions_selection_history_snapshot_full_access_to_default kind=snap owner=dialogs-pane snap=permissions_selection_history_full_access_to_default
#[test]
fn permissions_selection_history_snapshot_full_access_to_default() {
    kit::pending(Owner::DialogsPane, NO_HISTORY);
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.type_text("/permissions").keys("enter");
    s.keys("up enter");
    s.assert_snap_region(
        "permissions_selection_history_full_access_to_default",
        row_of(&s, "Permissions updated to"),
        0,
    );
}

// ORACLE chatwidget/tests/permissions.rs:1516 permissions_selection_emits_history_cell_when_current_is_selected kind=ui owner=dialogs-pane
#[test]
fn permissions_selection_emits_history_cell_when_current_is_selected() {
    kit::pending(Owner::DialogsPane, NO_HISTORY);
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.type_text("/permissions").keys("enter");
    s.keys("enter");
    s.assert_contains("Permissions updated to");
}

// ORACLE chatwidget/tests/permissions.rs:1549 permissions_selection_hides_auto_review_when_feature_disabled kind=snap owner=dialogs-pane
#[test]
fn permissions_selection_hides_auto_review_when_feature_disabled() {
    kit::pending(Owner::DialogsPane, NO_AUTO_REVIEW);
    let mut s = Screen::build(Owner::DialogsPane)
        .size(120, 24)
        .live()
        .open();
    s.type_text("/permissions").keys("enter");
    s.assert_absent("Approve for me");
    s.keys("esc");
    s.type_text("/permissions approve-for-me").keys("enter");
    s.assert_contains(
        "■ Approve for me is unavailable. Run `/permissions` to see available profiles.",
    );
}

// ORACLE chatwidget/tests/permissions.rs:1574 permissions_selection_hides_auto_review_when_feature_disabled_even_if_auto_review_is_active kind=ui owner=dialogs-pane
#[test]
fn permissions_selection_hides_auto_review_when_feature_disabled_even_if_auto_review_is_active() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(120, 24)
        .live()
        .open();
    s.type_text("/permissions").keys("enter");
    s.assert_absent("Approve for me");
}

// ORACLE chatwidget/tests/permissions.rs:1603 permissions_selection_marks_auto_review_current_after_session_configured kind=snap owner=dialogs-pane snap=permissions_unnamed_server_profile
#[test]
fn permissions_selection_marks_auto_review_current_after_session_configured() {
    let s = Screen::build(Owner::DialogsPane)
        .size(120, 24)
        .entry("select", "permissions_unnamed_server_profile")
        .open();
    s.assert_snap_region(
        "permissions_unnamed_server_profile",
        row_of(&s, "Update Model Permissions"),
        0,
    );
}

// ORACLE chatwidget/tests/permissions.rs:1653 permissions_selection_marks_auto_review_current_with_custom_workspace_write_details kind=ui owner=dialogs-pane
#[test]
fn permissions_selection_marks_auto_review_current_with_custom_workspace_write_details() {
    kit::pending(Owner::DialogsPane, NO_AUTO_REVIEW);
    let mut s = Screen::build(Owner::DialogsPane)
        .size(120, 24)
        .live()
        .open();
    s.type_text("/permissions").keys("enter");
    s.assert_contains("Approve for me (current)");
}

// ORACLE chatwidget/tests/permissions.rs:1706 permissions_selection_can_disable_auto_review kind=ui owner=dialogs-pane
#[test]
fn permissions_selection_can_disable_auto_review() {
    kit::na(
        Owner::DialogsPane,
        "asserts the UpdateApprovalsReviewer(User) and no UpdateFeatureFlags events on Up then Enter; no drawn text is asserted",
    );
}

// ORACLE chatwidget/tests/permissions.rs:1745 permissions_selection_sends_approvals_reviewer_in_override_turn_context kind=ui owner=dialogs-pane
#[test]
fn permissions_selection_sends_approvals_reviewer_in_override_turn_context() {
    kit::pending(Owner::DialogsPane, NO_AUTO_REVIEW);
    let mut s = Screen::build(Owner::DialogsPane)
        .size(120, 24)
        .live()
        .open();
    s.type_text("/permissions").keys("enter");
    s.assert_row_starts_with(row_of(&s, "(current)"), "›");
    s.keys("down");
    s.assert_row_starts_with(row_of(&s, "Approve for me"), "›");
    // Enter sends the override turn context; the oracle asserts that op and the profile update.
    s.keys("enter");
}

// ORACLE chatwidget/tests/permissions.rs:1823 permissions_full_access_history_cell_emitted_only_after_confirmation kind=ui owner=dialogs-pane
#[test]
fn permissions_full_access_history_cell_emitted_only_after_confirmation() {
    kit::pending(Owner::DialogsPane, NO_FULL_ACCESS);
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.type_text("/permissions").keys("enter");
    s.keys("down enter");
    s.assert_contains("Enable full access?");
    s.assert_absent("Permissions updated to");
    s.keys("enter");
    s.assert_contains("Permissions updated to Full Access");
}

// ORACLE chatwidget/tests/permissions.rs:1888 windows_sandbox_unelevated_policy_hides_admin_setup kind=snap owner=dialogs-pane snap=windows_sandbox_unelevated_policy
#[test]
fn windows_sandbox_unelevated_policy_hides_admin_setup() {
    kit::pending(Owner::DialogsPane, NO_WINDOWS);
    let s = Screen::build(Owner::DialogsPane)
        .size(100, 24)
        .live()
        .open();
    s.assert_snap_region(
        "windows_sandbox_unelevated_policy",
        row_of(&s, "Set up the Codex agent sandbox"),
        0,
    );
}

// ---- oracle: chatwidget/tests/plugin_catalog_tests.rs (4 screen-level tests here; 0 state tests left to dialogs-pane)
// ORACLE chatwidget/tests/plugin_catalog_tests.rs:5 plugins_popup_uses_product_labels_for_remote_and_personal_tabs kind=snap owner=dialogs-pane
#[test]
fn plugins_popup_uses_product_labels_for_remote_and_personal_tabs() {
    let remote =
        |name: &str, raw: &str, id: &str, plugin_name: &str, display: &str, description: &str| {
            seed::remote_marketplace(
                name,
                raw,
                vec![seed::remote_summary(
                    id,
                    plugin_name,
                    Some(display),
                    Some(description),
                    false,
                )],
            )
        };
    let shelf = seed::shelf(vec![seed::marketplace(
        "codex-curated",
        Some(seed::personal_path()),
        Some("Personal"),
        vec![plugin(
            "plugin-local-docs",
            "local-docs",
            "Local Docs",
            "Local editable docs.",
            false,
        )],
    )]);
    seed::with(&shelf, |held| {
        held.remote = vec![
            remote(
                "workspace-directory",
                "Raw Workspace Directory",
                "plugins~Plugin_buildkite",
                "buildkite",
                "Buildkite",
                "Workspace CI.",
            ),
            remote(
                "workspace-shared-with-me-private",
                "Raw Shared Private",
                "plugins~Plugin_docs",
                "docs",
                "Docs",
                "Shared docs.",
            ),
            remote(
                "workspace-shared-with-me-unlisted",
                "Raw Shared Link",
                "plugins~Plugin_link",
                "link",
                "Link Share",
                "Shared by link.",
            ),
        ];
    });
    let mut s = seed::open(120, 24, seed::config(), &shelf);
    s.wait_for("Buildkite");
    // Each marketplace tab shows its product label and not the raw marketplace label.
    let rows: Vec<String> = [
        ("Workspace.", "Buildkite", "Raw Workspace Directory."),
        ("Shared with me.", "Docs", "Raw Shared Private."),
        ("Shared with me (link).", "Link Share", "Raw Shared Link."),
        ("Local.", "Local Docs", "Personal."),
    ]
    .into_iter()
    .map(|(product_label, plugin_name, raw_label)| {
        select_tab(&mut s, product_label);
        s.assert_contains(product_label);
        s.assert_contains(plugin_name);
        s.assert_absent(raw_label);
        let row = s
            .rows()
            .into_iter()
            .find(|row| row.contains(plugin_name))
            .expect("the plugin's row");
        row.split_whitespace().collect::<Vec<_>>().join(" ")
    })
    .collect();
    kit::assert_rows_are(
        &rows,
        &[
            "› [-] Buildkite Available Press Enter to install or view plugin details",
            "› [-] Docs Available Press Enter to install or view plugin details",
            "› [-] Link Share Available Press Enter to install or view plugin details",
            "› [-] Local Docs Available Press Enter to install or view plugin details",
        ]
    );
}

// ORACLE chatwidget/tests/plugin_catalog_tests.rs:117 plugins_popup_preserves_workspace_tab_across_load_and_detail_navigation kind=ui owner=dialogs-pane
#[test]
fn plugins_popup_preserves_workspace_tab_across_load_and_detail_navigation() {
    let buildkite = seed::remote_summary(
        "plugins~Plugin_buildkite",
        "buildkite",
        Some("Buildkite"),
        Some("Buildkite pipelines."),
        false,
    );
    let shelf = seed::shelf(vec![seed::curated(Vec::new())]);
    seed::with(&shelf, |held| {
        held.held.insert(seed::SECTIONS);
        held.remote = vec![seed::remote_marketplace(
            "workspace-directory",
            "Raw Workspace Directory",
            vec![buildkite.clone()],
        )];
        held.details.insert(
            "*".to_string(),
            seed::remote_detail(
                "workspace-directory",
                buildkite,
                Some("Buildkite pipelines."),
            ),
        );
    });
    let mut s = seed::open(100, 24, seed::config(), &shelf);
    select_tab(&mut s, "Loading Workspace plugins.");
    s.assert_contains("Loading Workspace plugins.");
    // The sections arrive: the Workspace tab stays the active one.
    seed::serve_sections(&mut s, &shelf);
    s.assert_contains("Workspace.");
    s.assert_contains("Buildkite");
    s.assert_absent("Loading Workspace plugins.");
    // Detail, then Back to plugins: the Workspace tab is still the active one.
    s.keys("enter");
    s.wait_for("Back to plugins");
    s.keys("enter");
    s.assert_contains("Workspace.");
    s.assert_contains("Buildkite");
}

// ORACLE chatwidget/tests/plugin_catalog_tests.rs:176 plugins_popup_remote_local_dedupe_prefers_installed_remote_after_mapped_shares kind=ui owner=dialogs-pane
#[test]
fn plugins_popup_remote_local_dedupe_prefers_installed_remote_after_mapped_shares() {
    let remote_id = "plugins~Plugin_docs";
    let local = seed::set(
        plugin(
            "plugin-docs",
            "docs",
            "Docs",
            "Local curated docs plugin.",
            false,
        ),
        &[("remotePluginId", json!(remote_id))],
    );
    let shelf = seed::shelf(vec![seed::curated(vec![local])]);
    seed::with(&shelf, |held| {
        held.remote = vec![seed::remote_marketplace(
            "workspace-shared-with-me-private",
            "Shared with me",
            vec![seed::remote_summary(
                remote_id,
                "docs",
                Some("Docs"),
                Some("Remote installed docs plugin."),
                true,
            )],
        )];
    });
    let mut s = seed::open(100, 24, seed::config(), &shelf);
    // The remote copy is the installed one: it wins over the local row that is no mapped share.
    s.wait_for("Installed 1 of 1 available plugins.");
    s.assert_row_of("Docs", &["Installed"], &["Local curated docs plugin."]);
}

// ORACLE chatwidget/tests/plugin_catalog_tests.rs:244 plugin_detail_not_installable_plugin_disables_install_action kind=ui owner=dialogs-pane
#[test]
fn plugin_detail_not_installable_plugin_disables_install_action() {
    let internal = seed::summary(
        "plugin-internal",
        "internal",
        Some("Internal"),
        Some("Internal only."),
        false,
        true,
        "NOT_AVAILABLE",
    );
    let shelf = seed::shelf(vec![seed::curated(vec![internal.clone()])]);
    seed::with(&shelf, |held| {
        held.details.insert(
            "internal".to_string(),
            seed::detail(internal, Some("Internal only."), &[], &[], &[], &[]),
        );
    });
    let mut s = seed::open(100, 24, seed::config(), &shelf);
    s.keys("enter");
    s.assert_row_of(
        "Install plugin",
        &["This plugin is not installable from this marketplace"],
        &[],
    );
    let before = s.shot();
    // Down skips the disabled install row: the popup is drawn the same.
    s.keys("down");
    s.assert_same_rows(&before);
}
