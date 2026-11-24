//! Shard `pane_footer_2` (screens S10, owner ui-pane): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, ColorDepth, Owner, Screen};

/// The snapshot's rows equal ours, and the glyph cells carry the oracle's attributes
/// (`text` dim, the key bold). The oracle's `Buffer` dump also keeps DIM and
/// ITALIC on blank cells (the indent, the cells after a text); the terminal does not
/// show an attribute on a blank (G46), so `assert_snap`'s per-cell styles cannot hold
/// for them.
/// The one row that differs in the stack snapshots: the footer hint.
const AGENTS_FOOTER: &str = "rows above the footer equal; the footer hint row differs: the oracle's chat widget has agents navigation off (the app calls `enable_agents_navigation`), ours always has the command center and reads `← for agents` for `? for shortcuts` (needs a kit option or a CommandFlags/StatusFacts flag for it, as chat_status_2's rows)";

fn assert_rows_of(s: &Screen, snap: &str) {
    s.assert_snap_head(snap);
}

// ---- oracle: bottom_pane/effort_ignition_tests.rs (6 screen-level tests here; 8 state tests left to ui-pane)
// ORACLE bottom_pane/effort_ignition_tests.rs:165 max_and_ultra_prompts_render_their_accent_and_glyph kind=ui owner=ui-pane
#[test]
fn max_and_ultra_prompts_render_their_accent_and_glyph() {
    let max = Screen::build(Owner::UiPane)
        .size(1, 1)
        .terminal_colors(0xe0_dc_d6, 0x12_16_1c)
        .color_depth(ColorDepth::Ansi16)
        .entry("chat", "effort_prompt_max")
        .open();
    max.assert_text_at(0, 0, "›");
    max.assert_style(0, 0, "fg=Yellow bold");
    let ultra = Screen::build(Owner::UiPane)
        .size(1, 1)
        .terminal_colors(0xe0_dc_d6, 0x12_16_1c)
        .color_depth(ColorDepth::Ansi16)
        .entry("chat", "effort_prompt_ultra")
        .open();
    ultra.assert_text_at(0, 0, "»");
    ultra.assert_style(0, 0, "fg=Magenta bold");
}

// ORACLE bottom_pane/effort_ignition_tests.rs:189 effort_ignition_clock_waits_for_its_first_visible_frame kind=ui owner=ui-pane
#[test]
fn effort_ignition_clock_waits_for_its_first_visible_frame() {
    kit::na(
        Owner::UiPane,
        "asserts the ignition's started_at, is_finished and charge_alpha before its first frame, and that the render paints nothing; no drawn output is asserted",
    );
}

// ORACLE bottom_pane/effort_ignition_tests.rs:202 effort_ignition_finishes_when_palette_is_unavailable kind=ui owner=ui-pane
#[test]
fn effort_ignition_finishes_when_palette_is_unavailable() {
    kit::na(
        Owner::UiPane,
        "asserts the ignition's started_at and is_finished when the palette is unavailable; the render paints nothing and no drawn output is asserted",
    );
}

// ORACLE bottom_pane/effort_ignition_tests.rs:268 effort_ignition_styles_preserve_draft_and_paint_expected_content kind=ui owner=ui-pane
#[test]
fn effort_ignition_styles_preserve_draft_and_paint_expected_content() {
    let mut s = Screen::build(Owner::UiPane)
        .size(44, 3)
        .dark()
        .entry("chat", "effort_ignition_ultra")
        .open();
    let idle = s.shot();
    // The oracle's elapsed times 180, 420, 720, 1100, 1580 ms, as steps.
    for millis in [180, 240, 300, 380, 480] {
        s.wait(millis);
        s.assert_row_starts_with(1, "  > keep my draft exactly as typed");
    }
    // The oracle asserts some styles paint content; this checks the screen changed.
    s.assert_rows_differ(&idle);
}

// ORACLE bottom_pane/effort_ignition_tests.rs:299 effort_ignition_styles_are_safe_on_tiny_and_offset_areas kind=ui owner=ui-pane
#[test]
fn effort_ignition_styles_are_safe_on_tiny_and_offset_areas() {
    kit::na(
        Owner::UiPane,
        "paints every style on tiny and offset areas and asserts only that nothing panics; no drawn output is asserted",
    );
}

// ORACLE bottom_pane/effort_ignition_tests.rs:325 effort_ignition_animation_gallery_snapshot kind=snap owner=ui-pane snap=effort_ignition_animation_gallery
#[test]
fn effort_ignition_animation_gallery_snapshot() {
    kit::pending(
        Owner::UiPane,
        "no effort ignition gallery: no entry chat/effort_ignition_gallery (44x3: three styles, MAX and ULTRA, seven times each); the oracle snapshot is a sequence of 42 frames, the kit compares one screen (gap)",
    );
    let s = Screen::build(Owner::UiPane)
        .size(44, 3)
        .entry("chat", "effort_ignition_gallery")
        .open();
    s.assert_snap("effort_ignition_animation_gallery");
}

// ---- oracle: bottom_pane/effort_status_line_tests.rs (3 screen-level tests here; 5 state tests left to ui-pane)
// ORACLE bottom_pane/effort_status_line_tests.rs:18 clock_waits_for_the_first_visible_frame kind=ui owner=ui-pane
#[test]
fn clock_waits_for_the_first_visible_frame() {
    let s = Screen::build(Owner::UiPane)
        .size(40, 1)
        .entry("chat", "effort_transition_high_to_ultra")
        .first_frame()
        .open();
    s.assert_contains("gpt-5.4 high · main");
}

// ORACLE bottom_pane/effort_status_line_tests.rs:124 links_follow_the_visible_text_through_the_effort_transition kind=ui owner=ui-pane
#[test]
fn links_follow_the_visible_text_through_the_effort_transition() {
    let mut s = Screen::build(Owner::UiPane)
        .size(8, 1)
        .entry("chat", "effort_transition_links")
        .first_frame()
        .open();
    // The oracle's elapsed times 310, 1500, 2200 ms, as steps from the first frame.
    s.wait(310);
    s.assert_row(0, " a PR42 ");
    s.assert_link("PR42", "https://example.com/pr/42");
    s.wait(1190);
    s.assert_row(0, " M A X  ");
    s.assert_no_link("M A X");
    s.wait(700);
    s.assert_row(0, "b PR43  ");
    s.assert_link("PR43", "https://example.com/pr/43");
}

// ORACLE bottom_pane/effort_status_line_tests.rs:213 transition_frames_snapshot kind=snap owner=ui-pane snap=effort_status_line_transition_frames
#[test]
fn transition_frames_snapshot() {
    kit::pending(
        Owner::UiPane,
        "no effort status-line frames: no entry chat/effort_status_line_frames (32 columns, `gpt-5.4 high · main` to MAX and ULTRA at twelve times each); the oracle snapshot lists frames, the kit compares one screen (gap)",
    );
    let s = Screen::build(Owner::UiPane)
        .size(32, 1)
        .entry("chat", "effort_status_line_frames")
        .open();
    s.assert_snap("effort_status_line_transition_frames");
}

// ---- oracle: bottom_pane/footer.rs (3 screen-level tests here; 1 state tests left to ui-pane)
// ORACLE bottom_pane/footer.rs:961 voice_footer_rendering_preserves_text_and_styles kind=snap owner=ui-pane snap=voice_footer_rendered_styles
#[test]
fn voice_footer_rendering_preserves_text_and_styles() {
    kit::pending(
        Owner::UiPane,
        "no footer voice item: no entry chat/footer_voice_listen (40x1: `voice ● listen` and `ctrl+m mute`, with the cell styles)",
    );
    let s = Screen::build(Owner::UiPane)
        .size(40, 1)
        .entry("chat", "footer_voice_listen")
        .open();
    s.assert_snap("voice_footer_rendered_styles");
}

// ORACLE bottom_pane/footer.rs:1260 footer_snapshots kind=snap owner=ui-pane snap=footer_active_agent_label snap=footer_composer_has_draft_queue_hint_enabled snap=footer_context_tokens_used snap=footer_ctrl_c_quit_idle snap=footer_esc_hint_idle snap=footer_esc_hint_primed snap=footer_mode_indicator_narrow_overlap_hides snap=footer_mode_indicator_running_hides_hint snap=footer_mode_indicator_wide snap=footer_shortcuts_collaboration_modes_enabled snap=footer_shortcuts_context_running snap=footer_shortcuts_default snap=footer_shortcuts_running snap=footer_shortcuts_shift_and_esc snap=footer_status_line_disabled_context_right snap=footer_status_line_enabled_mode_and_ide_context_right snap=footer_status_line_enabled_mode_right snap=footer_status_line_enabled_no_mode_right snap=footer_status_line_overrides_draft_idle snap=footer_status_line_overrides_shortcuts snap=footer_status_line_truncated_with_gap snap=footer_status_line_with_active_agent_label snap=footer_status_line_yields_to_queue_hint
#[test]
fn footer_snapshots() {
    kit::pending(
        Owner::UiPane,
        "no footer states: no entries chat/<name> for the 23 footer snapshots (one per FooterProps the oracle builds; 1 row, 80 columns unless noted)",
    );
    let s = Screen::build(Owner::UiPane)
        .size(80, 1)
        .entry("chat", "footer_active_agent_label")
        .open();
    s.assert_snap("footer_active_agent_label");
    let s = Screen::build(Owner::UiPane)
        .size(80, 1)
        .entry("chat", "footer_composer_has_draft_queue_hint_enabled")
        .open();
    s.assert_snap("footer_composer_has_draft_queue_hint_enabled");
    let s = Screen::build(Owner::UiPane)
        .size(80, 1)
        .entry("chat", "footer_context_tokens_used")
        .open();
    s.assert_snap("footer_context_tokens_used");
    let s = Screen::build(Owner::UiPane)
        .size(80, 1)
        .entry("chat", "footer_ctrl_c_quit_idle")
        .open();
    s.assert_snap("footer_ctrl_c_quit_idle");
    let s = Screen::build(Owner::UiPane)
        .size(80, 1)
        .entry("chat", "footer_esc_hint_idle")
        .open();
    s.assert_snap("footer_esc_hint_idle");
    let s = Screen::build(Owner::UiPane)
        .size(80, 1)
        .entry("chat", "footer_esc_hint_primed")
        .open();
    s.assert_snap("footer_esc_hint_primed");
    let s = Screen::build(Owner::UiPane)
        .size(40, 1)
        .entry("chat", "footer_mode_indicator_narrow_overlap_hides")
        .open();
    s.assert_snap("footer_mode_indicator_narrow_overlap_hides");
    let s = Screen::build(Owner::UiPane)
        .size(80, 1)
        .entry("chat", "footer_mode_indicator_running_hides_hint")
        .open();
    s.assert_snap("footer_mode_indicator_running_hides_hint");
    let s = Screen::build(Owner::UiPane)
        .size(120, 1)
        .entry("chat", "footer_mode_indicator_wide")
        .open();
    s.assert_snap("footer_mode_indicator_wide");
    let s = Screen::build(Owner::UiPane)
        .size(80, 1)
        .entry("chat", "footer_shortcuts_collaboration_modes_enabled")
        .open();
    s.assert_snap("footer_shortcuts_collaboration_modes_enabled");
    let s = Screen::build(Owner::UiPane)
        .size(80, 1)
        .entry("chat", "footer_shortcuts_context_running")
        .open();
    s.assert_snap("footer_shortcuts_context_running");
    let s = Screen::build(Owner::UiPane)
        .size(80, 1)
        .entry("chat", "footer_shortcuts_default")
        .open();
    s.assert_snap("footer_shortcuts_default");
    let s = Screen::build(Owner::UiPane)
        .size(80, 1)
        .entry("chat", "footer_shortcuts_running")
        .open();
    s.assert_snap("footer_shortcuts_running");
    let s = Screen::build(Owner::UiPane)
        .size(80, 1)
        .entry("chat", "footer_shortcuts_shift_and_esc")
        .open();
    s.assert_snap("footer_shortcuts_shift_and_esc");
    let s = Screen::build(Owner::UiPane)
        .size(80, 1)
        .entry("chat", "footer_status_line_disabled_context_right")
        .open();
    s.assert_snap("footer_status_line_disabled_context_right");
    let s = Screen::build(Owner::UiPane)
        .size(80, 1)
        .entry(
            "chat",
            "footer_status_line_enabled_mode_and_ide_context_right",
        )
        .open();
    s.assert_snap("footer_status_line_enabled_mode_and_ide_context_right");
    let s = Screen::build(Owner::UiPane)
        .size(80, 1)
        .entry("chat", "footer_status_line_enabled_mode_right")
        .open();
    s.assert_snap("footer_status_line_enabled_mode_right");
    let s = Screen::build(Owner::UiPane)
        .size(80, 1)
        .entry("chat", "footer_status_line_enabled_no_mode_right")
        .open();
    s.assert_snap("footer_status_line_enabled_no_mode_right");
    let s = Screen::build(Owner::UiPane)
        .size(80, 1)
        .entry("chat", "footer_status_line_overrides_draft_idle")
        .open();
    s.assert_snap("footer_status_line_overrides_draft_idle");
    let s = Screen::build(Owner::UiPane)
        .size(80, 1)
        .entry("chat", "footer_status_line_overrides_shortcuts")
        .open();
    s.assert_snap("footer_status_line_overrides_shortcuts");
    let s = Screen::build(Owner::UiPane)
        .size(40, 1)
        .entry("chat", "footer_status_line_truncated_with_gap")
        .open();
    s.assert_snap("footer_status_line_truncated_with_gap");
    let s = Screen::build(Owner::UiPane)
        .size(80, 1)
        .entry("chat", "footer_status_line_with_active_agent_label")
        .open();
    s.assert_snap("footer_status_line_with_active_agent_label");
    let s = Screen::build(Owner::UiPane)
        .size(80, 1)
        .entry("chat", "footer_status_line_yields_to_queue_hint")
        .open();
    s.assert_snap("footer_status_line_yields_to_queue_hint");
}

// ORACLE bottom_pane/footer.rs:1672 footer_status_line_truncates_to_keep_mode_indicator kind=ui owner=ui-pane
#[test]
fn footer_status_line_truncates_to_keep_mode_indicator() {
    kit::pending(
        Owner::UiPane,
        "no footer status line: no entry chat/footer_status_line_plan_mode (80 columns, Plan collaboration mode, a long status line, 50% context left)",
    );
    let s = Screen::build(Owner::UiPane)
        .size(80, 1)
        .entry("chat", "footer_status_line_plan_mode")
        .open();
    s.assert_contains("Plan mode");
    s.assert_absent("⇧tab to cycle");
    s.assert_contains("…");
}

// ---- oracle: bottom_pane/footer/footer_content_tests.rs (3 screen-level tests here; 0 state tests left to ui-pane)
// ORACLE bottom_pane/footer/footer_content_tests.rs:11 clipped_links_keep_their_cells_after_a_halfwidth_katakana_grapheme kind=ui owner=ui-pane
#[test]
fn clipped_links_keep_their_cells_after_a_halfwidth_katakana_grapheme() {
    kit::pending(
        Owner::UiPane,
        "no footer link line: no entry chat/clipped_links_katakana (10x1: `ｶﾞ` linked to the project, ` · `, `PR #42` linked to the PR, clipped with an ellipsis)",
    );
    let s = Screen::build(Owner::UiPane)
        .size(10, 1)
        .entry("chat", "clipped_links_katakana")
        .open();
    s.assert_link("ｶﾞ", "https://example.com/project");
    s.assert_text_at(2, 0, " · ");
    s.assert_link("PR #", "https://example.com/pr/42");
    s.assert_text_at(9, 0, "…");
}

// ORACLE bottom_pane/footer/footer_content_tests.rs:38 arabic_prefix_keeps_link_on_the_painted_cells_at_exact_fit_and_overflow kind=ui owner=ui-pane
#[test]
fn arabic_prefix_keeps_link_on_the_painted_cells_at_exact_fit_and_overflow() {
    kit::pending(
        Owner::UiPane,
        "no footer link line after an Arabic prefix: no entries chat/arabic_prefix_exact (11x1, `لا · PR #42`) and chat/arabic_prefix_overflow (10x1)",
    );
    let exact = Screen::build(Owner::UiPane)
        .size(11, 1)
        .entry("chat", "arabic_prefix_exact")
        .open();
    exact.assert_text_at(4, 0, " ");
    exact.assert_link("PR #42", "https://example.com/pr/42");
    let overflow = Screen::build(Owner::UiPane)
        .size(10, 1)
        .entry("chat", "arabic_prefix_overflow")
        .open();
    overflow.assert_text_at(4, 0, " ");
    overflow.assert_link("PR #", "https://example.com/pr/42");
    overflow.assert_text_at(9, 0, "…");
}

// ORACLE bottom_pane/footer/footer_content_tests.rs:60 prepared_tree_uses_its_natural_width_and_paints_only_the_footer_viewport kind=snap owner=ui-pane snap=prepared_tree_uses_its_natural_width_and_paints_only_the_footer_viewport
#[test]
fn prepared_tree_uses_its_natural_width_and_paints_only_the_footer_viewport() {
    kit::pending(
        Owner::UiPane,
        "no footer link tree: no entry chat/prepared_tree_link (a link with `ok`, `large build` and `a`..`d` lines, painted in the 6x4 viewport at (1,1) of an 8x7 `.` backdrop)",
    );
    let s = Screen::build(Owner::UiPane)
        .size(8, 7)
        .entry("chat", "prepared_tree_link")
        .open();
    s.assert_snap("prepared_tree_uses_its_natural_width_and_paints_only_the_footer_viewport");
    s.assert_text_at(1, 4, "a");
    s.assert_text_at(2, 4, "…");
}

// ---- oracle: bottom_pane/mod.rs (32 screen-level tests here; 8 state tests left to ui-pane)
// ORACLE bottom_pane/mod.rs:2801 flush_buffered_typing_in_background_and_covering_editors kind=ui owner=ui-pane
#[test]
fn flush_buffered_typing_in_background_and_covering_editors() {
    kit::pending(
        Owner::UiPane,
        "no covering request-user-input overlay in the live chat (question `Which way?`, typed `modal`): the composer's buffered `main draft` must survive its paste-burst flush",
    );
    let mut s = Screen::build(Owner::UiPane).live().open();
    s.type_text("main draft");
    s.assert_contains("main draft");
}

// ORACLE bottom_pane/mod.rs:2844 inline_banner_snapshot kind=snap owner=ui-pane snap=inline_banner_wrapped_and_truncated
#[test]
fn inline_banner_snapshot() {
    kit::pending(
        Owner::UiPane,
        "no inline banner in the chat: no entry chat/inline_banner_wrapped (44 columns, `Choose how to continue working`, the description repeated eight times, two actions `Open usage settings` and `Notify owner`)",
    );
    let s = Screen::build(Owner::UiPane)
        .size(44, 19)
        .entry("chat", "inline_banner_wrapped")
        .open();
    s.assert_snap("inline_banner_wrapped_and_truncated");
}

// ORACLE bottom_pane/mod.rs:2880 escape_closes_shortcut_help_before_inline_banner kind=ui owner=ui-pane
#[test]
fn escape_closes_shortcut_help_before_inline_banner() {
    kit::pending(
        Owner::UiPane,
        "no entry chat/inline_banner_short (an inline banner `Continue working`, 80 columns; chat-live registers entries): the keys are page-tested on the backend banner, tests/pages/chat_status.rs `escape_closes_the_help_before_it_dismisses_the_banner`",
    );
    let mut s = Screen::build(Owner::UiPane)
        .size(80, 4)
        .entry("chat", "inline_banner_short")
        .open();
    let banner = s.shot();
    s.keys("?");
    s.assert_rows_differ(&banner);
    s.keys("esc");
    s.assert_same_rows(&banner);
    s.keys("esc");
    s.assert_rows_differ(&banner);
}

// ORACLE bottom_pane/mod.rs:2909 backend_banner_snapshots_and_numbered_actions kind=snap owner=ui-pane
#[test]
fn backend_banner_snapshots_and_numbered_actions() {
    kit::pending(
        Owner::UiPane,
        "no backend banner: no entries chat/backend_banner_personal_limit and chat/backend_banner_workspace_member_credits (44x15, numbered actions; key 2 dispatches the action, which no screen draws)",
    );
    let mut s = Screen::build(Owner::UiPane)
        .size(44, 15)
        .entry("chat", "backend_banner_personal_limit")
        .open();
    s.assert_snap("backend_banner_personal_limit");
    s.keys("2");
    let mut s = Screen::build(Owner::UiPane)
        .size(44, 15)
        .entry("chat", "backend_banner_workspace_member_credits")
        .open();
    s.assert_snap("backend_banner_workspace_member_credits");
    s.keys("2");
}

// ORACLE bottom_pane/mod.rs:3016 transition_events_reach_a_view_covered_by_a_modal kind=ui owner=ui-pane
#[test]
fn transition_events_reach_a_view_covered_by_a_modal() {
    kit::na(
        Owner::UiPane,
        "asserts that key press, release and paste reach a view covered by a modal (observer flags on the view); nothing is drawn",
    );
}

// ORACLE bottom_pane/mod.rs:3129 ctrl_c_cancels_history_search_without_clearing_draft_or_showing_quit_hint kind=ui owner=ui-pane
#[test]
fn ctrl_c_cancels_history_search_without_clearing_draft_or_showing_quit_hint() {
    let mut s = Screen::build(Owner::UiPane).live().open();
    s.type_text("draft");
    let typed = s.shot();
    s.keys("ctrl+r");
    s.keys("ctrl+c");
    s.assert_contains("draft");
    s.assert_same_rows(&typed);
}

// ORACLE bottom_pane/mod.rs:3162 overlay_not_shown_above_approval_modal kind=ui owner=ui-pane
#[test]
fn overlay_not_shown_above_approval_modal() {
    kit::pending(
        Owner::UiPane,
        "no exec approval modal: no entry chat/approval_exec_request (60x6, `Would you like to run the following command?`); the status overlay must not show above it",
    );
    let s = Screen::build(Owner::UiPane)
        .size(60, 6)
        .entry("chat", "approval_exec_request")
        .open();
    s.assert_contains("Would you like to run the following command?");
    // The oracle checks row 0 only; the kit has no row-absent assert (gap).
    s.assert_absent("Working");
}

// ORACLE bottom_pane/mod.rs:3210 approval_request_is_delayed_after_recent_typing kind=ui owner=ui-pane
#[test]
fn approval_request_is_delayed_after_recent_typing() {
    kit::pending(
        Owner::UiPane,
        "no delayed exec approval in the live chat: a request pushed while the composer is being typed in waits for the 1 s typing-idle delay (the request arrives as a server message; no seed for it yet)",
    );
    let mut s = Screen::build(Owner::UiPane).live().open();
    s.type_text("composer");
    s.assert_absent("Would you like to run the following command?");
    s.wait(999);
    s.assert_absent("Would you like to run the following command?");
    s.wait(1);
    s.assert_contains("Would you like to run the following command?");
}

// ORACLE bottom_pane/mod.rs:3296 typed_approval_shortcuts_during_delay_stay_in_composer kind=ui owner=ui-pane
#[test]
fn typed_approval_shortcuts_during_delay_stay_in_composer() {
    kit::pending(
        Owner::UiPane,
        "no delayed exec approval in the live chat: typed `y` and `a` during the typing-idle delay stay in the composer as `ya kept`",
    );
    let mut s = Screen::build(Owner::UiPane).live().open();
    s.keys("y a");
    s.assert_contains("ya");
    s.assert_absent("Would you like to run the following command?");
    s.paste(" kept");
    s.assert_contains("ya kept");
    s.assert_absent("Would you like to run the following command?");
}

// ORACLE bottom_pane/mod.rs:3327 delayed_approval_shortcut_works_after_idle_deadline kind=ui owner=ui-pane
#[test]
fn delayed_approval_shortcut_works_after_idle_deadline() {
    kit::pending(
        Owner::UiPane,
        "no delayed exec approval in the live chat: a request past the 1 s typing-idle deadline shows, and `y` answers it",
    );
    let mut s = Screen::build(Owner::UiPane).live().open();
    s.wait(1000);
    s.assert_contains("Would you like to run the following command?");
    s.keys("y");
    s.assert_absent("Would you like to run the following command?");
}

// ORACLE bottom_pane/mod.rs:3415 apply_text_suggestion_updates_matching_prompt_beneath_an_overlay kind=ui owner=ui-pane
#[test]
fn apply_text_suggestion_updates_matching_prompt_beneath_an_overlay() {
    kit::pending(
        Owner::UiPane,
        "no rename prompt with a text suggestion under an overlay: no entry chat/rename_prompt_suggested (80x8: the suggestion `Fix login timeout` replaces the `Loading` text and shows `Ready`)",
    );
    let mut s = Screen::build(Owner::UiPane)
        .size(80, 8)
        .entry("chat", "rename_prompt_suggested")
        .open();
    s.keys("enter");
    s.assert_contains("Fix login timeout");
    s.assert_contains("Ready");
    s.assert_absent("Loading");
}

// ORACLE bottom_pane/mod.rs:3480 completing_top_view_preserves_underlying_view kind=ui owner=ui-pane
#[test]
fn completing_top_view_preserves_underlying_view() {
    kit::na(
        Owner::UiPane,
        "asserts the view stack length and the view ids after the top view completes; both test views draw nothing, so no screen shows them",
    );
}

// ORACLE bottom_pane/mod.rs:3505 composer_shown_after_denied_while_task_running kind=ui owner=ui-pane
#[test]
fn composer_shown_after_denied_while_task_running() {
    kit::pending(
        Owner::UiPane,
        "no denied exec approval while a task runs in the live chat: `n` denies the request; row 0 keeps the Working header and the composer shows below (40x6)",
    );
    let mut s = Screen::build(Owner::UiPane).size(40, 6).live().open();
    s.keys("n");
    s.wait(120);
    s.assert_contains("Working");
    s.assert_contains("Ask Codex");
}

// ORACLE bottom_pane/mod.rs:3574 status_indicator_visible_during_command_execution kind=snap owner=ui-pane snap=status_timer_survives_hidden_row
#[test]
fn status_indicator_visible_during_command_execution() {
    kit::pending(Owner::UiPane, AGENTS_FOOTER);
    let s = Screen::build(Owner::UiPane)
        .size(40, 6)
        .entry("chat", "status_timer_hidden_row")
        .first_frame()
        .open();
    s.assert_contains("• Working");
    s.assert_snap("status_timer_survives_hidden_row");
}

// ORACLE bottom_pane/mod.rs:3626 turn_start_keeps_an_outstanding_approval_paused kind=ui owner=ui-pane
#[test]
fn turn_start_keeps_an_outstanding_approval_paused() {
    kit::na(
        Owner::UiPane,
        "asserts the status timer's elapsed durations and the modal state after a turn starts; no drawn value is asserted",
    );
}

// ORACLE bottom_pane/mod.rs:3649 status_and_composer_fill_height_without_bottom_padding kind=snap owner=ui-pane snap=status_and_composer_fill_height_without_bottom_padding
#[test]
fn status_and_composer_fill_height_without_bottom_padding() {
    let s = Screen::build(Owner::UiPane)
        .size(30, 6)
        .entry("chat", "status_and_composer")
        .first_frame()
        .open();
    s.assert_snap("status_and_composer_fill_height_without_bottom_padding");
}

// ORACLE bottom_pane/mod.rs:3681 status_only_snapshot kind=snap owner=ui-pane snap=status_only_snapshot
#[test]
fn status_only_snapshot() {
    kit::pending(Owner::UiPane, AGENTS_FOOTER);
    let s = Screen::build(Owner::UiPane)
        .size(48, 6)
        .entry("chat", "status_only")
        .first_frame()
        .open();
    s.assert_snap("status_only_snapshot");
}

// ORACLE bottom_pane/mod.rs:3705 unified_exec_summary_does_not_increase_height_when_status_visible kind=ui owner=ui-pane
#[test]
fn unified_exec_summary_does_not_increase_height_when_status_visible() {
    let s = Screen::build(Owner::UiPane)
        // one row more than the oracle's pane: the running command's cell in the transcript
        // brings the gap above the status row
        .size(120, 7)
        .entry("chat", "status_unified_exec")
        .first_frame()
        .open();
    s.assert_contains("background terminal running · /ps to view");
}

// ORACLE bottom_pane/mod.rs:3735 status_with_details_and_queued_messages_snapshot kind=snap owner=ui-pane snap=status_with_details_and_queued_messages_snapshot
#[test]
fn status_with_details_and_queued_messages_snapshot() {
    kit::pending(Owner::UiPane, AGENTS_FOOTER);
    let s = Screen::build(Owner::UiPane)
        .size(48, 11)
        .entry("chat", "status_with_details_queued")
        .first_frame()
        .open();
    s.assert_snap("status_with_details_and_queued_messages_snapshot");
}

// ORACLE bottom_pane/mod.rs:3773 queued_messages_visible_when_status_hidden_snapshot kind=snap owner=ui-pane snap=queued_messages_visible_when_status_hidden_snapshot
#[test]
fn queued_messages_visible_when_status_hidden_snapshot() {
    kit::pending(Owner::UiPane, AGENTS_FOOTER);
    let s = Screen::build(Owner::UiPane)
        .size(48, 7)
        .entry("chat", "queued_status_hidden")
        .first_frame()
        .open();
    s.assert_snap("queued_messages_visible_when_status_hidden_snapshot");
}

// ORACLE bottom_pane/mod.rs:3806 status_and_queued_messages_snapshot kind=snap owner=ui-pane snap=status_and_queued_messages_snapshot
#[test]
fn status_and_queued_messages_snapshot() {
    kit::pending(Owner::UiPane, AGENTS_FOOTER);
    let s = Screen::build(Owner::UiPane)
        .size(48, 9)
        .entry("chat", "status_and_queued")
        .first_frame()
        .open();
    s.assert_snap("status_and_queued_messages_snapshot");
}

// ORACLE bottom_pane/mod.rs:3838 remote_images_render_above_composer_text kind=ui owner=ui-pane
#[test]
fn remote_images_render_above_composer_text() {
    kit::pending(
        Owner::UiPane,
        "no remote images in the composer: no entry chat/remote_images_two (48 columns, two image URLs, empty composer text; the height is the desired height)",
    );
    let s = Screen::build(Owner::UiPane)
        .size(48, 8)
        .entry("chat", "remote_images_two")
        .open();
    s.assert_contains("[Image #1]");
    s.assert_contains("[Image #2]");
}

// ORACLE bottom_pane/mod.rs:3892 esc_with_skill_popup_does_not_interrupt_task kind=ui owner=ui-pane
#[test]
fn esc_with_skill_popup_does_not_interrupt_task() {
    kit::pending(
        Owner::UiPane,
        "no running task with a skill popup open: no entry chat/running_skill_popup (the `$` prompt typed while a task runs)",
    );
    let mut s = Screen::build(Owner::UiPane).live().open();
    s.type_text("$");
    s.keys("esc");
    s.assert_contains("$");
}

// ORACLE bottom_pane/mod.rs:3941 esc_dismisses_slash_command_popup_without_interrupting_task kind=snap owner=ui-pane snap=slash_command_popup_dismissed
#[test]
fn esc_dismisses_slash_command_popup_without_interrupting_task() {
    kit::pending(
        Owner::UiPane,
        "no running task with the slash command popup: no entry chat/running_slash_popup (`/rev` typed, 60 columns, 6 rows)",
    );
    let mut s = Screen::build(Owner::UiPane)
        .size(60, 6)
        .entry("chat", "running_slash_popup")
        .open();
    s.keys("esc");
    s.assert_contains("/rev");
    s.assert_snap("slash_command_popup_dismissed");
}

// ORACLE bottom_pane/mod.rs:4010 esc_with_agent_command_without_popup_does_not_interrupt_task kind=ui owner=ui-pane
#[test]
fn esc_with_agent_command_without_popup_does_not_interrupt_task() {
    kit::pending(
        Owner::UiPane,
        "no running task with `/subagents ` typed: no entry chat/running_subagents_command",
    );
    let mut s = Screen::build(Owner::UiPane).live().open();
    s.type_text("/subagents ");
    s.keys("esc");
    s.assert_contains("/subagents");
}

// ORACLE bottom_pane/mod.rs:4047 esc_release_after_dismissing_agent_picker_does_not_interrupt_task kind=ui owner=ui-pane
#[test]
fn esc_release_after_dismissing_agent_picker_does_not_interrupt_task() {
    kit::pending(
        Owner::UiPane,
        "no running task with the agent picker open: no entry chat/running_agent_picker (picker titled `Agents`, a `Main` row); the kit cannot send a key release (gap)",
    );
    let mut s = Screen::build(Owner::UiPane)
        .entry("chat", "running_agent_picker")
        .open();
    s.keys("esc");
    s.assert_absent("Agents");
    s.assert_contains("Working");
}

// ORACLE bottom_pane/mod.rs:4096 esc_interrupts_running_task_when_no_popup kind=ui owner=ui-pane
#[test]
fn esc_interrupts_running_task_when_no_popup() {
    kit::na(
        Owner::UiPane,
        "asserts that Esc sends the interrupt operation to the thread and the second Esc sends none; the operations are not drawn",
    );
}

// ORACLE bottom_pane/mod.rs:4129 remapped_interrupt_turn_uses_configured_key_including_agent_drafts kind=ui owner=ui-pane
#[test]
fn remapped_interrupt_turn_uses_configured_key_including_agent_drafts() {
    kit::pending(
        Owner::UiPane,
        "no running task with `/subagents ` typed (entry chat/running_subagents_command), and the chat reads no `chat.interrupt_turn` binding (the seed sets f12)",
    );
    let s = Screen::build(Owner::UiPane)
        .keymap(&[("chat.interrupt_turn", "f12")])
        .entry("chat", "running_subagents_command")
        .open();
    s.assert_contains("/subagents");
}

// ORACLE bottom_pane/mod.rs:4160 selection_view_esc_respects_remapped_list_cancel kind=ui owner=ui-pane
#[test]
fn selection_view_esc_respects_remapped_list_cancel() {
    kit::pending(
        Owner::UiPane,
        "no selection view with a remapped list cancel: no entry chat/selection_view_list (a running list picker); the seed sets list.cancel = q",
    );
    let mut s = Screen::build(Owner::UiPane)
        .keymap(&[("list.cancel", "q")])
        .entry("chat", "selection_view_list")
        .open();
    let shown = s.shot();
    s.keys("esc");
    s.assert_same_rows(&shown);
    s.keys("q");
    s.assert_rows_differ(&shown);
}

// ORACLE bottom_pane/mod.rs:4191 esc_routes_to_handle_key_event_when_requested kind=ui owner=ui-pane
#[test]
fn esc_routes_to_handle_key_event_when_requested() {
    kit::na(
        Owner::UiPane,
        "asserts how many times a view's key and ctrl+c handlers were called; the call counts are not drawn",
    );
}

// ORACLE bottom_pane/mod.rs:4251 release_events_are_ignored_for_active_view kind=ui owner=ui-pane
#[test]
fn release_events_are_ignored_for_active_view() {
    kit::na(
        Owner::UiPane,
        "asserts a view's key-handler call count across a press and a release; the call count is not drawn",
    );
}

// ORACLE bottom_pane/mod.rs:4306 paste_completion_clears_stacked_views_and_restores_composer_input kind=ui owner=ui-pane
#[test]
fn paste_completion_clears_stacked_views_and_restores_composer_input() {
    kit::pending(
        Owner::UiPane,
        "no stacked views under a paste: no entry chat/paste_under_views (40x2, composer input disabled, two views in the stack); the kit has no assert that a cursor is shown at any cell (gap, reported)",
    );
    let mut s = Screen::build(Owner::UiPane)
        .size(40, 2)
        .entry("chat", "paste_under_views")
        .open();
    s.paste("hello");
    s.keys("down");
}

// ---- oracle: bottom_pane/pending_input_preview.rs (13 screen-level tests here; 0 state tests left to ui-pane)
// ORACLE bottom_pane/pending_input_preview.rs:257 pending_input_preview_desired_height_empty kind=ui owner=ui-pane
#[test]
fn pending_input_preview_desired_height_empty() {
    kit::na(
        Owner::UiPane,
        "asserts the preview's desired height is 0 with no messages, a layout value that draws no row",
    );
}

// ORACLE bottom_pane/pending_input_preview.rs:263 oversized_preview_urls_keep_complete_destinations kind=snap owner=ui-pane snap=render_oversized_preview_url
#[test]
fn oversized_preview_urls_keep_complete_destinations() {
    kit::pending(
        Owner::UiPane,
        "pending(G48): the oracle keeps a URL-like token whole and clips it to the row (`adaptive_wrap_line_with_source`, then `truncate_line_to_width`); `TextWrap::Word` breaks an oversized token over rows, a hand wrapper would be R15. The preview itself is built (entry chat/render_oversized_preview_url)",
    );
    let s = Screen::build(Owner::UiPane)
        .size(32, 4)
        .entry("chat", "render_oversized_preview_url")
        .open();
    assert_rows_of(&s, "render_oversized_preview_url");
}

// ORACLE bottom_pane/pending_input_preview.rs:304 narrow_preview_clips_the_overflow_and_edit_hint kind=snap owner=ui-pane snap=render_narrow_preview
#[test]
fn narrow_preview_clips_the_overflow_and_edit_hint() {
    kit::pending(
        Owner::UiPane,
        "pending(G48): at 4 columns the oracle's wrapper leaves the bullet alone on the first row and starts the header's first word on the next (`•   `, `  Qu`); `Text` with `TextWrap::Word` puts `Qu` beside the bullet (`• Qu`). The preview itself is built (entry chat/render_narrow_preview)",
    );
    let s = Screen::build(Owner::UiPane)
        .size(4, 17)
        .entry("chat", "render_narrow_preview")
        .open();
    assert_rows_of(&s, "render_narrow_preview");
}

// ORACLE bottom_pane/pending_input_preview.rs:317 render_one_message kind=snap owner=ui-pane snap=render_one_message
#[test]
fn render_one_message() {
    let s = Screen::build(Owner::UiPane)
        .size(40, 3)
        .entry("chat", "render_one_message")
        .open();
    assert_rows_of(&s, "render_one_message");
}

// ORACLE bottom_pane/pending_input_preview.rs:328 render_two_messages kind=snap owner=ui-pane snap=render_two_messages
#[test]
fn render_two_messages() {
    let s = Screen::build(Owner::UiPane)
        .size(40, 4)
        .entry("chat", "render_two_messages")
        .open();
    assert_rows_of(&s, "render_two_messages");
}

// ORACLE bottom_pane/pending_input_preview.rs:342 render_more_than_three_messages kind=snap owner=ui-pane snap=render_more_than_three_messages
#[test]
fn render_more_than_three_messages() {
    let s = Screen::build(Owner::UiPane)
        .size(40, 6)
        .entry("chat", "render_more_than_three_messages")
        .open();
    assert_rows_of(&s, "render_more_than_three_messages");
}

// ORACLE bottom_pane/pending_input_preview.rs:362 render_wrapped_message kind=snap owner=ui-pane snap=render_wrapped_message
#[test]
fn render_wrapped_message() {
    let s = Screen::build(Owner::UiPane)
        .size(40, 5)
        .entry("chat", "render_wrapped_message")
        .open();
    assert_rows_of(&s, "render_wrapped_message");
}

// ORACLE bottom_pane/pending_input_preview.rs:378 render_many_line_message kind=snap owner=ui-pane snap=render_many_line_message
#[test]
fn render_many_line_message() {
    let s = Screen::build(Owner::UiPane)
        .size(40, 6)
        .entry("chat", "render_many_line_message")
        .open();
    assert_rows_of(&s, "render_many_line_message");
}

// ORACLE bottom_pane/pending_input_preview.rs:391 long_url_like_message_does_not_expand_into_wrapped_ellipsis_rows kind=ui owner=ui-pane
#[test]
fn long_url_like_message_does_not_expand_into_wrapped_ellipsis_rows() {
    kit::pending(
        Owner::UiPane,
        "pending(G48): the oracle keeps a URL-like token whole and clips it to the row (`adaptive_wrap_line_with_source`, then `truncate_line_to_width`); `TextWrap::Word` breaks an oversized token over rows, a hand wrapper would be R15. The preview itself is built (entry chat/long_url_like_message)",
    );
    let s = Screen::build(Owner::UiPane)
        .size(36, 3)
        .entry("chat", "long_url_like_message")
        .open();
    s.assert_absent("…");
}

// ORACLE bottom_pane/pending_input_preview.rs:423 render_one_pending_steer kind=snap owner=ui-pane snap=render_one_pending_steer
#[test]
fn render_one_pending_steer() {
    let s = Screen::build(Owner::UiPane)
        .size(48, 3)
        .entry("chat", "render_one_pending_steer")
        .open();
    assert_rows_of(&s, "render_one_pending_steer");
}

// ORACLE bottom_pane/pending_input_preview.rs:434 render_one_pending_steer_with_remapped_interrupt_binding kind=snap owner=ui-pane snap=render_one_pending_steer_with_remapped_interrupt_binding
#[test]
fn render_one_pending_steer_with_remapped_interrupt_binding() {
    let s = Screen::build(Owner::UiPane)
        .size(48, 3)
        .keymap(&[("chat.interrupt_turn", "f12")])
        .entry(
            "chat",
            "render_one_pending_steer_with_remapped_interrupt_binding",
        )
        .open();
    assert_rows_of(
        &s,
        "render_one_pending_steer_with_remapped_interrupt_binding",
    );
}

// ORACLE bottom_pane/pending_input_preview.rs:449 render_pending_steers_above_queued_messages kind=snap owner=ui-pane snap=render_pending_steers_above_queued_messages
#[test]
fn render_pending_steers_above_queued_messages() {
    let s = Screen::build(Owner::UiPane)
        .size(52, 11)
        .entry("chat", "render_pending_steers_above_queued_messages")
        .open();
    assert_rows_of(&s, "render_pending_steers_above_queued_messages");
}

// ORACLE bottom_pane/pending_input_preview.rs:472 render_multiline_pending_steer_uses_single_prefix_and_truncates kind=snap owner=ui-pane snap=render_multiline_pending_steer_uses_single_prefix_and_truncates
#[test]
fn render_multiline_pending_steer_uses_single_prefix_and_truncates() {
    let s = Screen::build(Owner::UiPane)
        .size(48, 6)
        .entry(
            "chat",
            "render_multiline_pending_steer_uses_single_prefix_and_truncates",
        )
        .open();
    assert_rows_of(
        &s,
        "render_multiline_pending_steer_uses_single_prefix_and_truncates",
    );
}

// ---- oracle: bottom_pane/status_line_style.rs (2 screen-level tests here; 7 state tests left to ui-pane)
// ORACLE bottom_pane/status_line_style.rs:400 light_status_line_corrects_pale_custom_theme_colors kind=snap owner=ui-pane snap=light_status_line_corrects_pale_custom_theme_colors
#[test]
fn light_status_line_corrects_pale_custom_theme_colors() {
    kit::pending(
        Owner::UiPane,
        "screen feature (status line): no entry chat/light_status_line_pale_theme (42x1: `gpt-5`, `~/code`, `Custom permissions` on a pale custom theme); the kit seeds the terminal colours (`terminal_colors(0x1e1e1e, 0xffffff)`)",
    );
    let s = Screen::build(Owner::UiPane)
        .size(42, 1)
        .entry("chat", "light_status_line_pale_theme")
        .open();
    s.assert_snap("light_status_line_corrects_pale_custom_theme_colors");
}

// ORACLE bottom_pane/status_line_style.rs:443 status_line_theme_colors_at_limited_color_depths kind=snap owner=ui-pane
#[test]
fn status_line_theme_colors_at_limited_color_depths() {
    kit::pending(
        Owner::UiPane,
        "screen feature (status line): no status line entry to draw at colour depths 0 and 1; the kit seeds the depth (`color_depth(ColorDepth::None)`, `Ansi16`) as the oracle child process does with FORCE_COLOR",
    );
}

// ---- oracle: bottom_pane/tests/actionable_banner_tests.rs (6 screen-level tests here; 0 state tests left to ui-pane)
// ORACLE bottom_pane/tests/actionable_banner_tests.rs:6 promoted_banner_uses_picker_hints_and_preserves_the_draft kind=snap owner=ui-pane
#[test]
fn promoted_banner_uses_picker_hints_and_preserves_the_draft() {
    kit::pending(
        Owner::UiPane,
        "no promoted action banner: no entry chat/promoted_action_banner (40 columns, `View usage` with `f3 select · f2 back`, the draft `draft in progress` kept; the list keys remapped to f3 accept and f2 cancel)",
    );
    let mut s = Screen::build(Owner::UiPane)
        .size(40, 6)
        .keymap(&[("list.accept", "f3"), ("list.cancel", "f2")])
        .entry("chat", "promoted_action_banner")
        .open();
    s.assert_contains("View usage");
    s.assert_contains("f3 select · f2 back");
    s.assert_contains("draft in progress");
    s.assert_snap("promoted_action_banner_40");
    s.keys("f3");
    s.assert_absent("View usage");
    s.assert_contains("draft in progress");
    let mut s = Screen::build(Owner::UiPane)
        .size(40, 6)
        .keymap(&[("list.accept", "f3"), ("list.cancel", "f2")])
        .entry("chat", "promoted_action_banner")
        .open();
    s.keys("f2");
    s.assert_absent("View usage");
    s.assert_contains("draft in progress");
}

// ORACLE bottom_pane/tests/actionable_banner_tests.rs:66 hidden_banner_preserves_input_before_render_and_after_resize kind=snap owner=ui-pane
#[test]
fn hidden_banner_preserves_input_before_render_and_after_resize() {
    kit::pending(
        Owner::UiPane,
        "no inline banner over the composer: no entry chat/inline_banner_usage_limit (70 columns, `Usage limit reached`, action `View usage`); the composer-gap variant (actionable_banner_with_composer_gap) is not written yet (gap)",
    );
    let mut s = Screen::build(Owner::UiPane)
        .size(70, 4)
        .entry("chat", "inline_banner_usage_limit")
        .open();
    s.assert_absent("View usage");
    s.keys("1");
    s.assert_contains("1");
    s.keys("esc");
    s.assert_contains("View usage");
}

// ORACLE bottom_pane/tests/actionable_banner_tests.rs:153 partially_clipped_banner_preserves_hidden_action_shortcuts kind=ui owner=ui-pane
#[test]
fn partially_clipped_banner_preserves_hidden_action_shortcuts() {
    kit::pending(
        Owner::UiPane,
        "no inline banner clipped to the composer row: no entry chat/inline_banner_two_actions (70 columns, `Keep working` and `View usage`, 2 rows: the composer plus one footer row)",
    );
    let mut s = Screen::build(Owner::UiPane)
        .size(70, 2)
        .entry("chat", "inline_banner_two_actions")
        .open();
    s.assert_contains("Keep working");
    s.assert_absent("View usage");
    s.keys("2");
    s.assert_contains("2");
}

// ORACLE bottom_pane/tests/actionable_banner_tests.rs:195 banner_hidden_during_task_waits_for_redraw_before_accepting_dismissal kind=ui owner=ui-pane
#[test]
fn banner_hidden_during_task_waits_for_redraw_before_accepting_dismissal() {
    kit::pending(
        Owner::UiPane,
        "no inline banner that hides while a task runs: no entry chat/inline_banner_title_only (70 columns, `Usage limit reached`); the running-task step is not in the kit's seed (gap)",
    );
    let mut s = Screen::build(Owner::UiPane)
        .size(70, 3)
        .entry("chat", "inline_banner_title_only")
        .open();
    s.assert_contains("Usage limit reached");
    s.keys("esc");
    s.assert_contains("Usage limit reached");
}

// ORACLE bottom_pane/tests/actionable_banner_tests.rs:221 information_banner_preserves_input_and_honors_dismissal kind=snap owner=ui-pane
#[test]
fn information_banner_preserves_input_and_honors_dismissal() {
    kit::pending(
        Owner::UiPane,
        "no information banner: no entries chat/information_banner_persistent and chat/information_banner_dismissible (70 columns, `Usage limit reached`, `You can change models or request more usage.`)",
    );
    let mut s = Screen::build(Owner::UiPane)
        .size(70, 6)
        .entry("chat", "information_banner_persistent")
        .open();
    s.assert_absent("no matches");
    s.assert_absent("Press a number");
    s.assert_absent("esc to dismiss");
    s.assert_snap("information_banner_persistent");
    s.keys("esc");
    s.assert_contains("Usage limit reached");
    s.keys("1");
    s.assert_contains("1");
    let mut s = Screen::build(Owner::UiPane)
        .size(70, 6)
        .entry("chat", "information_banner_dismissible")
        .open();
    s.assert_absent("no matches");
    s.assert_absent("Press a number");
    s.assert_contains("esc to dismiss");
    s.assert_snap("information_banner_dismissible");
    s.keys("esc");
    s.assert_absent("Usage limit reached");
    s.keys("1");
    s.assert_contains("1");
}

// ORACLE bottom_pane/tests/actionable_banner_tests.rs:263 banner_copy_preserves_wrapped_url_destinations kind=snap owner=ui-pane
#[test]
fn banner_copy_preserves_wrapped_url_destinations() {
    kit::pending(
        Owner::UiPane,
        "no inline banner with a wrapped URL: no entry chat/banner_wrapped_url (40 columns, `Review the release`, the description `Open https://github.com/openai/codex/pull/12345?diff=split before continuing.`)",
    );
    let s = Screen::build(Owner::UiPane)
        .size(40, 4)
        .entry("chat", "banner_wrapped_url")
        .open();
    s.assert_contains("Review the release");
    // The oracle checks the wrapped destination's cells link to the whole URL; the kit's link assert takes one text on one row, and the URL wraps (gap, reported).
}

// ---- oracle: bottom_pane/unified_exec_footer.rs (3 screen-level tests here; 0 state tests left to ui-pane)
// ORACLE bottom_pane/unified_exec_footer.rs:91 unified_exec_footer_desired_height_empty kind=ui owner=ui-pane
#[test]
fn unified_exec_footer_desired_height_empty() {
    kit::na(
        Owner::UiPane,
        "asserts the footer's desired height is 0 with no processes, a layout value that draws no row",
    );
}

// ORACLE bottom_pane/unified_exec_footer.rs:97 render_more_sessions kind=snap owner=ui-pane snap=render_more_sessions
#[test]
fn render_more_sessions() {
    let s = Screen::build(Owner::UiPane)
        .size(50, 1)
        .entry("chat", "render_more_sessions")
        .open();
    assert_rows_of(&s, "render_more_sessions");
}

// ORACLE bottom_pane/unified_exec_footer.rs:108 render_many_sessions kind=snap owner=ui-pane snap=render_many_sessions
#[test]
fn render_many_sessions() {
    let s = Screen::build(Owner::UiPane)
        .size(50, 1)
        .entry("chat", "render_many_sessions")
        .open();
    assert_rows_of(&s, "render_many_sessions");
}

// ---- oracle: bottom_pane/view_stack_tests.rs (1 screen-level tests here; 0 state tests left to ui-pane)
// ORACLE bottom_pane/view_stack_tests.rs:45 centered_dialog_retains_backdrop_and_routes_cursor kind=snap owner=ui-pane snap=centered_dialog_retains_backdrop_and_routes_cursor
#[test]
fn centered_dialog_retains_backdrop_and_routes_cursor() {
    kit::pending(
        Owner::UiPane,
        "no centered dialog over a backdrop: no entry chat/centered_dialog (80x9, 24x7 and 12x2; the cursor at (4,3), (2,2) and (2,0)); the oracle joins three frames, the kit's section header has no WxH form (gap)",
    );
    let s = Screen::build(Owner::UiPane)
        .size(80, 9)
        .entry("chat", "centered_dialog")
        .open();
    s.assert_cursor(4, 3);
    s.assert_snap_section("centered_dialog_retains_backdrop_and_routes_cursor", "80x9");
    let s = Screen::build(Owner::UiPane)
        .size(24, 7)
        .entry("chat", "centered_dialog")
        .open();
    s.assert_cursor(2, 2);
    s.assert_snap_section("centered_dialog_retains_backdrop_and_routes_cursor", "24x7");
    let s = Screen::build(Owner::UiPane)
        .size(12, 2)
        .entry("chat", "centered_dialog")
        .open();
    s.assert_cursor(2, 0);
    s.assert_snap_section("centered_dialog_retains_backdrop_and_routes_cursor", "12x2");
}

// ---- oracle: bottom_pane/voice_strip_tests.rs (2 screen-level tests here; 2 state tests left to ui-pane)
// ORACLE bottom_pane/voice_strip_tests.rs:49 active_dashboard_preserves_live_colors_controls_and_width kind=snap owner=ui-pane
#[test]
fn active_dashboard_preserves_live_colors_controls_and_width() {
    kit::pending(
        Owner::UiPane,
        "no voice strip while listening: no entry chat/voice_strip_active (47 columns, 2 rows, mic history and codex waveform, `f9 mute` and `/voice stop`; re-laid at 37, 22 and 45 columns)",
    );
    let mut s = Screen::build(Owner::UiPane)
        .size(47, 2)
        .entry("chat", "voice_strip_active")
        .open();
    s.assert_row_starts_with(0, " voice ");
    s.assert_row_starts_with(1, "   mic ");
    s.assert_text_at(45, 0, "p");
    s.assert_text_at(46, 0, " ");
    s.assert_style(7, 0, "fg=Red");
    s.assert_style(7, 1, "fg=Cyan");
    s.assert_style(22, 1, "fg=Magenta");
    s.assert_contains("/voice stop");
    s.resize(37, 2);
    s.assert_contains("f9 mute");
    s.assert_contains("/voice stop");
    s.resize(22, 2);
    s.assert_contains("/voice stop");
    s.assert_contains("mic");
    s.assert_contains("codex");
    s.resize(47, 2);
    s.assert_contains("█");
}

// ORACLE bottom_pane/voice_strip_tests.rs:141 voice_controls_follow_configured_mute_binding_and_unbinding kind=snap owner=ui-pane snap=voice_controls_follow_configured_mute_binding_and_unbinding
#[test]
fn voice_controls_follow_configured_mute_binding_and_unbinding() {
    kit::pending(
        Owner::UiPane,
        "no voice strip with a configurable mute key: entry chat/voice_strip_active with its mute key (f8, ctrl+x m, unbound: the seed sets chat.toggle_voice_mute); the oracle joins twelve states into one snapshot, the kit compares one screen (gap)",
    );
    let s = Screen::build(Owner::UiPane)
        .size(80, 2)
        .keymap(&[("chat.toggle_voice_mute", "f8")])
        .entry("chat", "voice_strip_active")
        .open();
    s.assert_contains("f8 mute");
    s.assert_contains("/voice stop");
    let s = Screen::build(Owner::UiPane)
        .size(80, 2)
        .keymap(&[("chat.toggle_voice_mute", "ctrl+x m")])
        .entry("chat", "voice_strip_active")
        .open();
    s.assert_contains("⌃x m mute");
    s.assert_contains("/voice stop");
    // Unbound (the config's `[]`): the kit's keymap seed has no form for it yet (gap).
    let s = Screen::build(Owner::UiPane)
        .size(80, 2)
        .entry("chat", "voice_strip_active")
        .open();
    s.assert_contains("/voice mute");
    s.assert_contains("/voice stop");
}
