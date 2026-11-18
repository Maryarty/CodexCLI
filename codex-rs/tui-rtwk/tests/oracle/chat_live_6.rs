//! Shard `chat_live_6` (screens S10, owner chat-live): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen};

// ---- oracle: chatwidget/tests/usage.rs (26 screen-level tests here; 8 state tests left to chat-live)
// ORACLE chatwidget/tests/usage.rs:17 usage_menu_opens_analytics kind=ui owner=chat-live
#[test]
fn usage_menu_opens_analytics() {
    kit::na(
        Owner::ChatLive,
        "asserts only AppEvent::OpenAnalytics sent by Enter in the usage menu, not drawn",
    );
}

// ORACLE chatwidget/tests/usage.rs:115 usage_command_opens_menu_when_reset_is_available_snapshot kind=snap owner=chat-live snap=usage_command_menu snap=usage_command_menu_narrow
#[test]
fn usage_command_opens_menu_when_reset_is_available_snapshot() {
    kit::pending(
        Owner::ChatLive,
        "no rate-limit reset flow in the chat: the startup reset check with 2 reset credits is not seeded, and /usage draws no menu with a Redeem reset row",
    );
    // The bottom popup is bottom-anchored: its 8 rows end on row 23 of a 24-row screen.
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.type_text("/usage").keys("enter");
    s.assert_snap_region("usage_command_menu", 16, 0);
    let mut s = Screen::build(Owner::ChatLive).size(40, 24).live().open();
    s.type_text("/usage").keys("enter");
    s.assert_snap_region("usage_command_menu_narrow", 16, 0);
    // The oracle's last check, Enter emits AppEvent::OpenAnalytics, is not drawn: not asserted.
}

// ORACLE chatwidget/tests/usage.rs:140 usage_command_disables_reset_after_cached_zero_snapshot kind=snap owner=chat-live snap=usage_command_menu_without_resets
#[test]
fn usage_command_disables_reset_after_cached_zero_snapshot() {
    kit::pending(
        Owner::ChatLive,
        "no rate-limit reset flow in the chat: the startup reset check with 0 reset credits is not seeded, so the Redeem reset row is not drawn as None available",
    );
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.type_text("/usage").keys("enter");
    s.assert_snap_region("usage_command_menu_without_resets", 16, 0);
    // The oracle's RefreshRateLimits event and the OpenAnalytics event are not drawn: not asserted.
}

// ORACLE chatwidget/tests/usage.rs:167 usage_menu_refresh_enables_newly_available_reset kind=ui owner=chat-live
#[test]
fn usage_menu_refresh_enables_newly_available_reset() {
    kit::na(
        Owner::ChatLive,
        "asserts AppEvent::RefreshRateLimits and then AppEvent::OpenRateLimitResetCredits, both events, not drawn",
    );
}

// ORACLE chatwidget/tests/usage.rs:196 usage_menu_refresh_failure_preserves_disabled_known_zero kind=ui owner=chat-live
#[test]
fn usage_menu_refresh_failure_preserves_disabled_known_zero() {
    kit::pending(
        Owner::ChatLive,
        "no rate-limit reset flow in the chat: no startup reset check seeded with 0 credits and no usage-menu refresh that can fail (backend unavailable) as a script step",
    );
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.type_text("/usage").keys("enter");
    s.assert_contains("None available");
    // The oracle's OpenAnalytics event after Enter is not drawn: not asserted.
}

// ORACLE chatwidget/tests/usage.rs:258 usage_command_can_check_reset_availability_before_startup_refresh_finishes_snapshot kind=snap owner=chat-live snap=usage_command_menu_before_reset_refresh
#[test]
fn usage_command_can_check_reset_availability_before_startup_refresh_finishes_snapshot() {
    kit::pending(
        Owner::ChatLive,
        "no rate-limit reset flow in the chat: /usage while the startup reset check is still in flight draws no menu, no Check availability row",
    );
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.type_text("/usage").keys("enter");
    s.assert_snap_region("usage_command_menu_before_reset_refresh", 16, 0);
    // Down then Enter: the oracle emits AppEvent::OpenRateLimitResetCredits (not drawn, not asserted).
    s.keys("down enter");
}

// ORACLE chatwidget/tests/usage.rs:275 usage_command_can_check_reset_availability_for_workspace_accounts kind=ui owner=chat-live
#[test]
fn usage_command_can_check_reset_availability_for_workspace_accounts() {
    kit::na(
        Owner::ChatLive,
        "asserts only AppEvent::OpenRateLimitResetCredits after Down and Enter for a workspace plan, an event, not drawn",
    );
}

// ORACLE chatwidget/tests/usage.rs:288 usage_menu_rate_limit_reset_entry_opens_reset_flow kind=ui owner=chat-live
#[test]
fn usage_menu_rate_limit_reset_entry_opens_reset_flow() {
    kit::na(
        Owner::ChatLive,
        "asserts only AppEvent::OpenRateLimitResetCredits after Down and Enter in the usage menu, an event, not drawn",
    );
}

// ORACLE chatwidget/tests/usage.rs:306 rate_limit_reset_popup_states_snapshot kind=snap owner=chat-live snap=rate_limit_reset_popup_states
#[test]
fn rate_limit_reset_popup_states_snapshot() {
    kit::pending(
        Owner::ChatLive,
        "no rate-limit reset flow in the chat (loading, list, empty, load error, consuming, redeemed states); kit gap: the snapshot joins its 10 states with bare `---` lines, which tests/oracle_snap frame parsing does not split (only labelled headers), so each state needs a frame label",
    );
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    // State 1: the loading popup, then 2 credits listed (soonest expiry first).
    s.type_text("/usage").keys("enter down enter");
    s.assert_snap_section("rate_limit_reset_popup_states", "state 1");
    // State 2: no credits (dismissed with Esc, reopened empty).
    s.keys("esc");
    s.assert_snap_section("rate_limit_reset_popup_states", "state 2");
    // State 3: the load error.
    s.keys("esc");
    s.assert_snap_section("rate_limit_reset_popup_states", "state 3");
    // State 4: consuming (Using a reset...), then the timed-out redemption.
    s.keys("esc");
    s.assert_snap_section("rate_limit_reset_popup_states", "state 4");
    // State 5: the redemption answers NothingToReset.
    s.keys("esc");
    s.assert_snap_section("rate_limit_reset_popup_states", "state 5");
    // State 6: the redemption answers NoCredit.
    s.keys("esc");
    s.assert_snap_section("rate_limit_reset_popup_states", "state 6");
    // State 7: the redemption answers AlreadyRedeemed-free success (Reset).
    s.keys("esc");
    s.assert_snap_section("rate_limit_reset_popup_states", "state 7");
    // State 8: the post-consume refresh lands with 1 credit left.
    s.keys("esc");
    s.assert_snap_section("rate_limit_reset_popup_states", "state 8");
}

// ORACLE chatwidget/tests/usage.rs:417 rate_limit_reset_picker_wraps_expiry_details_snapshot kind=snap owner=chat-live snap=rate_limit_reset_picker_narrow
#[test]
fn rate_limit_reset_picker_wraps_expiry_details_snapshot() {
    kit::pending(
        Owner::ChatLive,
        "no rate-limit reset flow in the chat: no reset credits with expiries (12-hour clock) seeded for the reset picker at 44 columns",
    );
    // The picker is bottom-anchored: its 10 rows end on row 23 of a 24-row screen.
    let mut s = Screen::build(Owner::ChatLive).size(44, 24).live().open();
    s.type_text("/usage").keys("enter down enter");
    s.assert_snap_region("rate_limit_reset_picker_narrow", 14, 0);
}

// ORACLE chatwidget/tests/usage.rs:441 rate_limit_reset_confirmation_uses_backend_copy_snapshot kind=snap owner=chat-live snap=rate_limit_reset_confirmation
#[test]
fn rate_limit_reset_confirmation_uses_backend_copy_snapshot() {
    kit::pending(
        Owner::ChatLive,
        "no rate-limit reset flow in the chat: no reset credit with a backend title and description seeded, so the confirmation with Expires 09:39 on 18 Jun 2026 is not drawn",
    );
    // The confirmation is bottom-anchored: its 9 rows end on row 23 of a 24-row screen.
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.type_text("/usage").keys("enter down enter");
    s.keys("enter");
    // The oracle's OpenRateLimitResetConfirmation event carries the same fields: not asserted here.
    s.assert_snap_region("rate_limit_reset_confirmation", 15, 0);
}

// ORACLE chatwidget/tests/usage.rs:501 rate_limit_reset_confirmation_no_and_escape_return_to_picker kind=ui owner=chat-live
#[test]
fn rate_limit_reset_confirmation_no_and_escape_return_to_picker() {
    kit::pending(
        Owner::ChatLive,
        "no rate-limit reset flow in the chat: no reset picker seeded with one credit, so there is no picker to return to from the confirmation",
    );
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.type_text("/usage").keys("enter down enter");
    let picker = s.shot();
    s.keys("enter enter");
    // The oracle's second Enter opens the confirmation, and the event is not asserted.
    s.keys("enter");
    s.assert_same_rows(&picker);
    s.keys("enter");
    s.keys("esc");
    s.assert_same_rows(&picker);
    s.keys("enter");
    s.keys("enter");
    s.assert_same_rows(&picker);
}

// ORACLE chatwidget/tests/usage.rs:530 reset_picker_allows_only_one_pending_confirmation kind=ui owner=chat-live
#[test]
fn reset_picker_allows_only_one_pending_confirmation() {
    kit::pending(
        Owner::ChatLive,
        "no rate-limit reset flow in the chat: no reset picker seeded with two titled credits (First reset, Second reset)",
    );
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.type_text("/usage").keys("enter down enter");
    s.keys("enter down enter");
    // The first confirmation is open; the oracle asserts no consume event yet (not drawn).
    s.keys("enter enter");
    s.assert_contains("Second reset · Does not expire");
}

// ORACLE chatwidget/tests/usage.rs:561 rate_limit_reset_picker_starts_with_soonest_expiries_and_keeps_all_rows_reachable kind=ui owner=chat-live
#[test]
fn rate_limit_reset_picker_starts_with_soonest_expiries_and_keeps_all_rows_reachable() {
    kit::pending(
        Owner::ChatLive,
        "no rate-limit reset flow in the chat: no reset picker seeded with nine credits (one per day from 18 Jun 2026 09:39)",
    );
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.type_text("/usage").keys("enter down enter");
    s.assert_contains("Expires 09:39 on 18 Jun 2026");
    s.assert_absent("Full reset (");
    // Eight Downs reach the ninth credit; Enter opens its confirmation.
    s.keys("down down down down down down down down enter");
    s.assert_contains("Expires 09:39 on 26 Jun 2026");
}

// ORACLE chatwidget/tests/usage.rs:610 rate_limit_reset_confirmation_can_use_reset kind=ui owner=chat-live
#[test]
fn rate_limit_reset_confirmation_can_use_reset() {
    kit::pending(
        Owner::ChatLive,
        "no rate-limit reset flow in the chat: no reset picker seeded with one credit, and the redemption (consume) answer that closes the popup is a seed step the script does not have",
    );
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.type_text("/usage").keys("enter down enter");
    s.keys("enter");
    s.assert_contains("Use this reset?");
    // Up then Enter confirms the reset: the oracle emits ConsumeRateLimitResetCredit (not drawn).
    s.keys("up enter");
    // The consume outcome is NothingToReset (seed step); Esc closes the popup.
    s.keys("esc");
    s.assert_absent("Use this reset?");
    s.assert_absent("Usage limit resets");
}

// ORACLE chatwidget/tests/usage.rs:659 rate_limit_reset_retry_reuses_idempotency_key kind=ui owner=chat-live
#[test]
fn rate_limit_reset_retry_reuses_idempotency_key() {
    kit::na(
        Owner::ChatLive,
        "asserts the ConsumeRateLimitResetCredit event's idempotency key and start_rate_limit_reset_consumption's return, events and state, nothing drawn",
    );
}

// ORACLE chatwidget/tests/usage.rs:690 no_credit_outcome_disables_reset_entry_in_usage_menu kind=ui owner=chat-live
#[test]
fn no_credit_outcome_disables_reset_entry_in_usage_menu() {
    kit::pending(
        Owner::ChatLive,
        "no rate-limit reset flow in the chat: no redemption that answers NoCredit (seed step), no stale selected credit to consume, no That reset is no longer available. notice",
    );
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.type_text("/usage").keys("enter down enter");
    s.assert_contains("That reset is no longer available.");
    // The oracle's later events (RefreshRateLimits, OpenAnalytics, OpenRateLimitResetCredits) are not drawn: not asserted.
}

// ORACLE chatwidget/tests/usage.rs:738 rate_limit_reset_redemption_cannot_be_dismissed_while_in_flight kind=ui owner=chat-live
#[test]
fn rate_limit_reset_redemption_cannot_be_dismissed_while_in_flight() {
    kit::pending(
        Owner::ChatLive,
        "no rate-limit reset flow in the chat: no redemption in flight (Using a reset...) and no Refreshing... state after a Reset outcome (seed steps)",
    );
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.type_text("/usage").keys("enter down enter");
    s.keys("esc");
    s.assert_contains("Using a reset...");
    // The redemption answers Reset (seed step); Esc while refreshing.
    s.keys("esc");
    s.assert_contains("Refreshing...");
    // The post-consume refresh lands (seed step); Esc closes it.
    s.keys("esc");
    s.assert_absent("Refreshing...");
}

// ORACLE chatwidget/tests/usage.rs:765 rate_limit_reset_redemption_allows_ctrl_c_to_quit_while_in_flight kind=ui owner=chat-live
#[test]
fn rate_limit_reset_redemption_allows_ctrl_c_to_quit_while_in_flight() {
    kit::pending(
        Owner::ChatLive,
        "no rate-limit reset flow in the chat: no redemption in flight (Using a reset...) to stay on screen while Ctrl+C quits",
    );
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.type_text("/usage").keys("enter down enter");
    s.keys("ctrl+c");
    // The oracle's AppEvent::Exit(ShutdownFirst) is not drawn: not asserted.
    s.assert_contains("Using a reset...");
}

// ORACLE chatwidget/tests/usage.rs:776 already_redeemed_is_an_idempotent_success kind=ui owner=chat-live
#[test]
fn already_redeemed_is_an_idempotent_success() {
    kit::pending(
        Owner::ChatLive,
        "no rate-limit reset flow in the chat: no redemption answering AlreadyRedeemed and no post-consume refresh with 0 resets (seed steps)",
    );
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.type_text("/usage").keys("enter down enter");
    // AlreadyRedeemed and the refresh with 0 resets arrive (seed steps).
    s.assert_contains("Usage reset. You have 0 usage limit resets left.");
}

// ORACLE chatwidget/tests/usage.rs:798 failed_post_consume_refresh_does_not_keep_stale_reset_count kind=ui owner=chat-live
#[test]
fn failed_post_consume_refresh_does_not_keep_stale_reset_count() {
    kit::pending(
        Owner::ChatLive,
        "no rate-limit reset flow in the chat: no startup check with 2 credits, no Reset outcome followed by a failed refresh (seed steps)",
    );
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    // The redemption answers Reset and the post-consume refresh fails (seed steps).
    s.keys("esc");
    s.type_text("/usage").keys("enter");
    s.assert_contains("Check availability");
    s.assert_absent("2 available");
}

// ORACLE chatwidget/tests/usage.rs:871 rate_limit_reset_load_result_updates_popup_beneath_overlay kind=ui owner=chat-live
#[test]
fn rate_limit_reset_load_result_updates_popup_beneath_overlay() {
    kit::pending(
        Owner::ChatLive,
        "no rate-limit reset flow in the chat, and no way to put a selection overlay over the reset popup (the Covering overlay view is an oracle test helper)",
    );
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.type_text("/usage").keys("enter down enter");
    s.assert_contains("Covering overlay");
    // The credits load lands beneath the overlay (seed step).
    s.keys("esc");
    s.assert_contains("2 usage limit resets available.");
}

// ORACLE chatwidget/tests/usage.rs:891 rate_limit_reset_success_updates_popup_beneath_overlay kind=ui owner=chat-live
#[test]
fn rate_limit_reset_success_updates_popup_beneath_overlay() {
    kit::pending(
        Owner::ChatLive,
        "no rate-limit reset flow in the chat, and no way to put a selection overlay over the consuming popup (the Covering overlay view is an oracle test helper)",
    );
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.type_text("/usage").keys("enter down enter");
    s.assert_contains("Covering overlay");
    // The redemption succeeds and the post-consume refresh lands beneath the overlay (seed steps).
    s.keys("esc");
    s.assert_contains("Usage reset. You have 1 usage limit reset left.");
}

// ORACLE chatwidget/tests/usage.rs:920 account_change_dismisses_reset_popup_beneath_overlay kind=ui owner=chat-live
#[test]
fn account_change_dismisses_reset_popup_beneath_overlay() {
    kit::pending(
        Owner::ChatLive,
        "no rate-limit reset flow in the chat: no confirmation over an overlay, and no account change as a script step (seed steps)",
    );
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.type_text("/usage").keys("enter down enter");
    s.keys("enter");
    // Up then Enter: the oracle emits ConsumeRateLimitResetCredit (not drawn).
    s.keys("up enter");
    s.assert_contains("Covering overlay");
    // The account changes (seed step); the stale confirmation is ignored.
    s.assert_absent("Stale reset");
    s.keys("esc");
    s.assert_absent("Covering overlay");
}

// ORACLE chatwidget/tests/usage.rs:974 startup_check_shows_available_reset_hint_snapshot kind=snap owner=chat-live snap=rate_limit_reset_available_hint
#[test]
fn startup_check_shows_available_reset_hint_snapshot() {
    kit::pending(
        Owner::ChatLive,
        "no rate-limit reset flow in the chat: the startup reset check with 2 available is not seeded, so the Run /usage to use one hint is not drawn",
    );
    let s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.assert_contains("• You have 2 usage limit resets available. Run /usage to use one.");
}

// ORACLE chatwidget/tests/usage.rs:994 startup_reset_hint_waits_for_active_output_snapshot kind=snap owner=chat-live snap=rate_limit_reset_hint_waits_for_active_output
#[test]
fn startup_reset_hint_waits_for_active_output_snapshot() {
    kit::pending(
        Owner::ChatLive,
        "no rate-limit reset flow in the chat: no startup reset check with 2 available that waits behind an active tool cell (seed step), and no flush of the active cell",
    );
    let s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    // The active tool output is on screen, the hint is held back (seed steps).
    s.assert_absent("usage limit resets available");
    // Flushing the active cell commits the hint after the output.
    s.assert_in_order(&[
        "active tool",
        "• You have 2 usage limit resets available. Run /usage to use one.",
    ]);
}

// ---- oracle: chatwidget/thread_usage_tests.rs (5 screen-level tests here; 12 state tests left to chat-live)
// ORACLE chatwidget/thread_usage_tests.rs:104 status_history_survives_exhausted_billing_retries kind=ui owner=chat-live
#[test]
fn status_history_survives_exhausted_billing_retries() {
    kit::pending(
        Owner::ChatLive,
        "no thread billing in the chat: no estimated-thread-cost status line item, no thread usage refresh that fails with retries (seed steps), and no /status Thread usage row",
    );
    let mut s = Screen::build(Owner::ChatLive).size(90, 24).live().open();
    s.type_text("/status").keys("enter");
    // The billing refreshes fail up to the retry bound, then the retry answers (seed steps).
    s.type_text("/status").keys("enter");
    s.assert_contains("50 credits · ~$2.10");
}

// ORACLE chatwidget/thread_usage_tests.rs:169 status_history_updates_again_after_billing_settles kind=ui owner=chat-live
#[test]
fn status_history_updates_again_after_billing_settles() {
    // The oracle loops over three status-line selections (estimated-thread-cost, thread-credits,
    // estimated-thread-cost) with stale then settled billing; each run ends on the settled card.
    for _case in 0..3 {
        kit::pending(
            Owner::ChatLive,
            "no thread billing in the chat: no status line item to select (estimated-thread-cost or thread-credits), no settlement refresh with stale then settled billing (seed steps)",
        );
        let mut s = Screen::build(Owner::ChatLive).size(90, 24).live().open();
        s.type_text("/status").keys("enter");
        s.assert_contains("50 credits · ~$2.10");
    }
}

// ORACLE chatwidget/thread_usage_tests.rs:649 billing_surfaces_render_for_every_supported_enterprise_plan kind=snap owner=chat-live
#[test]
fn billing_surfaces_render_for_every_supported_enterprise_plan() {
    // The oracle loops over Business, EnterpriseCbpUsageBased, EnterpriseCbpAutomation and Ent26.
    for _plan in 0..4 {
        kit::pending(
            Owner::ChatLive,
            "no thread billing in the chat: no plan-gated thread-credits and estimated-thread-cost status line and terminal title items, no shared billing request (seed steps)",
        );
        let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
        // Business and the enterprise plans: the status line and the terminal title show 5.2 credits · ~$0.21.
        s.assert_contains("5.2 credits · ~$0.21");
        s.assert_title("5.2 credits | ~$0.21");
        // Ent26 (no credits, native usage 0.75 USD): the status line and the title show ~$0.75.
        s.assert_contains("~$0.75");
        s.assert_title("~$0.75");
        // The /status card's Thread usage row.
        s.type_text("/status").keys("enter");
        s.assert_contains("Thread usage:        ~$0.75");
    }
}

// ORACLE chatwidget/thread_usage_tests.rs:722 early_draw_rearms_thread_usage_retry_after_immediate_redraw kind=ui owner=chat-live
#[test]
fn early_draw_rearms_thread_usage_retry_after_immediate_redraw() {
    kit::na(
        Owner::ChatLive,
        "asserts the frame requester's scheduled deadlines and the retry due time, frame scheduling, nothing drawn",
    );
}

// ORACLE chatwidget/thread_usage_tests.rs:822 early_draw_rearms_thread_usage_settlement_after_immediate_redraw kind=ui owner=chat-live
#[test]
fn early_draw_rearms_thread_usage_settlement_after_immediate_redraw() {
    kit::na(
        Owner::ChatLive,
        "asserts the frame requester's scheduled deadlines and the settlement due time, frame scheduling, nothing drawn",
    );
}

// ---- oracle: chatwidget/usage_notice_tests.rs (1 screen-level tests here; 11 state tests left to chat-live)
// ORACLE chatwidget/usage_notice_tests.rs:30 notice_details_fit_without_truncating_the_warning kind=snap owner=chat-live snap=usage_notice_widths snap=weekly_usage_notice_widths
#[test]
fn notice_details_fit_without_truncating_the_warning() {
    kit::pending(
        Owner::ChatLive,
        "no usage notice in the chat: no rate-limit warning line (5h limit at 8% left, resets in an hour; weekly at 2% left) in the footer at each width",
    );
    // The oracle draws each width's section from one notice state; each width is one screen.
    for width in [98, 97, 58, 57, 38, 37, 12, 11, 0] {
        let s = Screen::build(Owner::ChatLive)
            .size(width.max(1), 24)
            .live()
            .open();
        s.assert_snap_section("usage_notice_widths", &format!("{width} columns"));
    }
    for width in [138, 137, 76, 75, 56, 55, 16, 15] {
        let s = Screen::build(Owner::ChatLive).size(width, 24).live().open();
        s.assert_snap_section("weekly_usage_notice_widths", &format!("{width} columns"));
    }
}
