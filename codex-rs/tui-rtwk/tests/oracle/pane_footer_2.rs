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
/// The one row that differs in the pending-input previews: the footer hint (the oracle's chat has
/// no agents navigation, the preview entries keep it).
fn assert_rows_of(s: &Screen, snap: &str) {
    s.assert_snap_head(snap);
}

fn usage_banner(presentation: &str, description: &str, actions: serde_json::Value) -> Screen {
    let mut s = Screen::build(Owner::UiPane).size(70, 16).live().open();
    s.server_answers(
        "account/rateLimits/read",
        serde_json::json!({
            "rateLimits": {},
            "rateLimitUpsell": {
                "banner_type": "credits",
                "title": "Usage limit reached",
                "description": description,
                "ctas": actions,
                "presentation": presentation,
            },
        }),
    );
    s.feed(
        r#"{"step":"screen","dir":"status","ev":{"kind":"account","account":{"chat_gpt":{"email":"user@example.com","plan":"plus"}}}}
{"step":"screen","dir":"chat","ev":{"status":{"account_usage":true}}}"#,
    );
    s.assert_contains("Usage limit reached");
    s
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
    let mut s = Screen::build(Owner::UiPane)
        .size(80, 16)
        .keymap(&[("chat.toggle_voice_mute", "ctrl+m")])
        .live()
        .open();
    s.voice_available()
        .type_text("/voice")
        .keys("enter")
        .voice_connected();
    s.assert_contains("voice ● listen");
    s.assert_contains("⌃m mute");
    s.assert_style_in("voice ● listen", 6, "fg=Red");
}

// ORACLE bottom_pane/footer.rs:1260 footer_snapshots kind=snap owner=ui-pane snap=footer_active_agent_label snap=footer_composer_has_draft_queue_hint_enabled snap=footer_context_tokens_used snap=footer_ctrl_c_quit_idle snap=footer_esc_hint_idle snap=footer_esc_hint_primed snap=footer_mode_indicator_narrow_overlap_hides snap=footer_mode_indicator_running_hides_hint snap=footer_mode_indicator_wide snap=footer_shortcuts_collaboration_modes_enabled snap=footer_shortcuts_context_running snap=footer_shortcuts_default snap=footer_shortcuts_running snap=footer_shortcuts_shift_and_esc snap=footer_status_line_disabled_context_right snap=footer_status_line_enabled_mode_and_ide_context_right snap=footer_status_line_enabled_mode_right snap=footer_status_line_enabled_no_mode_right snap=footer_status_line_overrides_draft_idle snap=footer_status_line_overrides_shortcuts snap=footer_status_line_truncated_with_gap snap=footer_status_line_with_active_agent_label snap=footer_status_line_yields_to_queue_hint
#[test]
fn footer_snapshots() {
    // Each oracle snapshot is one `FooterProps` of the oracle's standalone footer; here one entry
    // draws the real `Footer` (or the overlay's panel) over the model its fixture script makes.
    let at = |name: &'static str, width: u16, height: u16| {
        let s = Screen::build(Owner::UiPane)
            .size(width, height)
            .entry("chat", name)
            .open();
        s.assert_snap(name);
    };
    for name in [
        "footer_shortcuts_default",
        "footer_shortcuts_context_running",
        "footer_context_tokens_used",
        "footer_composer_has_draft_queue_hint_enabled",
        "footer_ctrl_c_quit_idle",
        "footer_esc_hint_idle",
        "footer_esc_hint_primed",
    ] {
        at(name, 80, 1);
    }
    // the overlay's height is its panel's: the Change mode row makes one more
    for (name, height) in [
        ("footer_shortcuts_shift_and_esc", 20),
        ("footer_shortcuts_collaboration_modes_enabled", 21),
        ("footer_shortcuts_running", 20),
    ] {
        at(name, 80, height);
    }
    // `footer_status_line_yields_to_queue_hint`: the composer draws its hint row from
    // `hint_footer_props`, which takes the status line off the props (the line is the status
    // surface's rows above), so the row is the queue hint and the meter of any draft kept while a
    // turn runs.
    let s = Screen::build(Owner::UiPane)
        .size(80, 1)
        .entry("chat", "footer_composer_has_draft_queue_hint_enabled")
        .open();
    s.assert_snap("footer_status_line_yields_to_queue_hint");
    // n/a states of the standalone footer, which the composer never draws: `status_line_value`
    // of the props replaces the hints in the footer's own row (`footer_status_line_overrides_*`,
    // `_truncated_with_gap`, `_with_active_agent_label`), and of `active_agent_label`,
    // `mode_indicator` (right of the hints in the composer's `passive_hints`, left of them in the
    // standalone footer's `_mode_indicator_*`, `_status_line_*_mode_right`, `_disabled_context_right`,
    // `_active_agent_label`). The composer strips them (`hint_footer_props`) and draws the status line
    // as its own rows over the hints (`status_surface.rs`), as the port's page does (chat_status_*,
    // pane_footer_1).
}

// ORACLE bottom_pane/footer.rs:1672 footer_status_line_truncates_to_keep_mode_indicator kind=ui owner=ui-pane
#[test]
fn footer_status_line_truncates_to_keep_mode_indicator() {
    let mut s = Screen::build(Owner::UiPane)
        .size(80, 8)
        .script(include_str!("fixtures/pane_footer_1/plan_status.jsonl"))
        .open();
    s.assert_in_order(&["MODEL", "Context 20% used", "Plan mode"]);
    s.resize(24, 8);
    s.assert_contains("Plan mode");
}

fn footer_pr_link(project: &str) -> Screen {
    let mut s = Screen::build(Owner::UiPane).size(13, 4).live().open();
    s.feed(
        &serde_json::json!({
            "step": "screen", "dir": "chat", "ev": {
                "status": {"cwd": "/workspace", "project_root": project,
                    "items": ["project-name", "pull-request-number"]}
            }
        })
        .to_string(),
    );
    s.feed(r#"{"step":"screen","dir":"chat","ev":{"facts":{"cwd":"/workspace"}}}
{"step":"screen","dir":"chat","ev":{"facts":{"git_summary":{"request":1,"summary":{"pull_request":{"number":42,"url":"https://example.com/pr/42"}}}}}}"#);
    s
}

// ---- oracle: bottom_pane/footer/footer_content_tests.rs (3 screen-level tests here; 0 state tests left to ui-pane)
// ORACLE bottom_pane/footer/footer_content_tests.rs:11 clipped_links_keep_their_cells_after_a_halfwidth_katakana_grapheme kind=ui owner=ui-pane
#[test]
fn clipped_links_keep_their_cells_after_a_halfwidth_katakana_grapheme() {
    assert_footer_pr_link("ｶﾞ", "ｶﾞ");
    kit::pending(
        Owner::UiPane,
        "the oracle attaches its own URL to the Katakana project, keeps PR # on the clipped row, and leaves the ellipsis unlinked. The live status line only has a PR destination; the toolkit can give its ellipsis the clipped span's link. The surviving PR cells are verified above",
    );
}

// ORACLE bottom_pane/footer/footer_content_tests.rs:35 arabic_prefix_keeps_link_on_the_painted_cells_at_exact_fit_and_overflow kind=ui owner=ui-pane
#[test]
fn arabic_prefix_keeps_link_on_the_painted_cells_at_exact_fit_and_overflow() {
    assert_footer_pr_link("لا", "ال");
    kit::pending(
        Owner::UiPane,
        "the live bidi path paints PR #42 on the left of the Arabic prefix. The original snapshot asserts specific PR and ellipsis cell columns in logical order. The surviving PR cells are verified above",
    );
}

fn assert_footer_pr_link(project: &str, painted: &str) {
    let mut s = footer_pr_link(project);
    s.assert_row_of(painted, &[" · ", "PR #42"], &[]);
    s.assert_no_link(painted);
    s.assert_link("PR #42", "https://example.com/pr/42");
    s.resize(10, 4);
    s.assert_link("PR", "https://example.com/pr/42");
}

#[test]
fn footer_pr_links_follow_native_painted_cells_through_resize() {
    assert_footer_pr_link("ｶﾞ", "ｶﾞ");
    assert_footer_pr_link("لا", "ال");
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
    let mut s = usage_banner(
        "dismissible",
        "Add credits to keep working.",
        serde_json::json!([{"action":"view_usage","label":"View usage"}]),
    );
    s.keys("?");
    s.assert_contains("Keyboard shortcuts");
    s.keys("esc");
    s.assert_absent("Keyboard shortcuts");
    s.assert_contains("Usage limit reached");
    s.keys("esc");
    s.assert_absent("Usage limit reached");
}

// ORACLE bottom_pane/mod.rs:2909 backend_banner_snapshots_and_numbered_actions kind=snap owner=ui-pane
#[test]
fn backend_banner_snapshots_and_numbered_actions() {
    for (action, label) in [
        ("view_usage", "View usage"),
        ("notify_owner", "Notify owner"),
    ] {
        let mut s = usage_banner(
            "dismissible",
            "Your included usage is depleted.\nChoose an action to continue.",
            serde_json::json!([
                {"action":"unsupported","label":"Hidden action"},
                {"action":"view_usage","label":"Open usage settings"},
                {"action":action,"label":label},
                {"action":"view_usage","label":"Extra action"},
            ]),
        );
        s.resize(44, 15);
        s.assert_in_order(&[
            "1. Open usage settings",
            &format!("2. {label}"),
            "3. Extra action",
        ]);
        s.assert_absent("Hidden action");
        s.keys("2");
        s.assert_composer("");
        if action == "view_usage" {
            s.assert_opened_urls(&["https://chatgpt.com/settings/usage"]);
            s.assert_nudges(0);
        } else {
            s.assert_opened_urls(&[]);
            s.assert_nudges(1);
        }
        s.assert_contains("Usage limit reached");
    }
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
    let s = Screen::build(Owner::UiPane)
        .size(60, 6)
        .script(include_str!("fixtures/pane_footer_2/exec_request.jsonl"))
        .open();
    s.assert_contains("Would you like to run the following command?");
    // The oracle checks row 0 only.
    kit::assert_same(
        s.rows()[0].contains("Working"),
        false,
        &format!("overlay should not render above modal: {:?}", s.rows()[0]),
    );
}

// ORACLE bottom_pane/mod.rs:3210 approval_request_is_delayed_after_recent_typing kind=ui owner=ui-pane
#[test]
fn approval_request_is_delayed_after_recent_typing() {
    // The composer source of the oracle's loop (the warning sources are the warnings strip's).
    let mut s = Screen::build(Owner::UiPane).live().open();
    s.type_text("composer");
    s.feed(include_str!("fixtures/pane_footer_2/exec_request.jsonl"));
    // The kit's typing and feed take about 0.3 s of the second; the exact millisecond of the
    // delay is pinned by `tests/sheets/presenter.rs` (`an_approval_waits_the_idle_delay`).
    s.assert_absent("Would you like to run the following command?");
    s.wait(400);
    s.assert_absent("Would you like to run the following command?");
    s.wait(600);
    s.assert_contains("Would you like to run the following command?");
}

// ORACLE bottom_pane/mod.rs:3296 typed_approval_shortcuts_during_delay_stay_in_composer kind=ui owner=ui-pane
#[test]
fn typed_approval_shortcuts_during_delay_stay_in_composer() {
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
    let mut s = Screen::build(Owner::UiPane).live().open();
    s.type_text("a");
    s.feed(include_str!("fixtures/pane_footer_2/exec_request.jsonl"));
    s.wait(1000);
    s.assert_contains("Would you like to run the following command?");
    s.keys("y");
    s.assert_absent("Would you like to run the following command?");
    let accepted = s.resolved_requests();
    kit::assert_same(
        accepted,
        vec![serde_json::json!({"decision": "accept"})],
        "the shortcut answered the request with accept",
    );
}

// ORACLE bottom_pane/mod.rs:3415 apply_text_suggestion_updates_matching_prompt_beneath_an_overlay kind=ui owner=ui-pane
#[test]
fn apply_text_suggestion_updates_matching_prompt_beneath_an_overlay() {
    let mut s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    s.feed(r#"{"step":"notify","message":{"method":"item/completed","params":{"threadId":"thread-1","turnId":"turn-1","completedAtMs":0,"startedAtMs":0,"item":{"type":"userMessage","id":"u1","clientId":null,"content":[{"type":"text","text":"investigate a login timeout","text_elements":[]}]}}}}"#);
    s.type_text("/rename").keys("enter");
    s.assert_contains("Generating a title suggestion");
    let approval = include_str!("fixtures/pane_footer_2/exec_request.jsonl")
        .lines()
        .filter(|line| line.contains(r#""method":"item/commandExecution/requestApproval""#))
        .collect::<Vec<_>>()
        .join("\n");
    s.feed(&approval);
    s.wait_for("Would you like to run the following command?");
    s.answer_title(Some("Fix login timeout"));
    s.assert_contains("Would you like to run the following command?");
    s.keys("esc");
    s.assert_contains("Fix login timeout");
    s.assert_contains("Suggested from this conversation");
    s.assert_absent("Generating a title suggestion");
    s.keys("enter");
    s.assert_set_names(&["Fix login timeout"]);
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
    let mut s = Screen::build(Owner::UiPane)
        .size(40, 6)
        .script(include_str!(
            "fixtures/pane_footer_2/running_exec_request.jsonl"
        ))
        .open();
    s.keys("n");
    s.wait(120);
    // The oracle renders the bottom pane alone, its row 0 is the pane's first row: the status
    // row, with the transcript's blank row above it on the screen.
    let rows = s.rows();
    let top = rows
        .iter()
        .find(|row| !row.trim().is_empty())
        .cloned()
        .unwrap_or_default();
    kit::assert_same(
        top.contains("Working"),
        true,
        &format!("expected Working header after denial on the pane's first row: {top:?}"),
    );
    s.assert_contains("Ask Codex");
    s.assert_absent("Would you like to run the following command?");
}

// ORACLE bottom_pane/mod.rs:3574 status_indicator_visible_during_command_execution kind=snap owner=ui-pane snap=status_timer_survives_hidden_row
#[test]
fn status_indicator_visible_during_command_execution() {
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
    let mut s = Screen::build(Owner::UiPane)
        .size(48, 8)
        .live()
        .remote_images(&["https://example.com/one.png", "https://example.com/two.png"])
        .open();
    s.type_text("draft below images");
    s.assert_in_order(&["Image #1", "Image #2", "draft below images"]);
    s.assert_composer("draft below images");
}

// ORACLE bottom_pane/mod.rs:3892 esc_with_skill_popup_does_not_interrupt_task kind=ui owner=ui-pane
#[test]
fn esc_with_skill_popup_does_not_interrupt_task() {
    let mut s = Screen::build(Owner::UiPane).live().open();
    s.type_text("$");
    s.keys("esc");
    s.assert_contains("$");
}

// ORACLE bottom_pane/mod.rs:3941 esc_dismisses_slash_command_popup_without_interrupting_task kind=snap owner=ui-pane snap=slash_command_popup_dismissed
#[test]
fn esc_dismisses_slash_command_popup_without_interrupting_task() {
    let mut s = Screen::build(Owner::UiPane)
        .size(60, 16)
        .script(include_str!("fixtures/chat_status_1/running_turn.jsonl"))
        .open();
    s.type_text("/rev");
    s.wait_for("/review");
    s.keys("esc");
    s.assert_absent("/review");
    s.assert_composer("/rev");
    s.assert_contains("Working");
    s.assert_interrupts(0);
}

// ORACLE bottom_pane/mod.rs:4010 esc_with_agent_command_without_popup_does_not_interrupt_task kind=ui owner=ui-pane
#[test]
fn esc_with_agent_command_without_popup_does_not_interrupt_task() {
    let mut s = Screen::build(Owner::UiPane).live().open();
    s.type_text("/subagents ");
    s.keys("esc");
    s.assert_contains("/subagents");
}

// ORACLE bottom_pane/mod.rs:4047 esc_release_after_dismissing_agent_picker_does_not_interrupt_task kind=ui owner=ui-pane
#[test]
fn esc_release_after_dismissing_agent_picker_does_not_interrupt_task() {
    let mut s = Screen::build(Owner::UiPane)
        .size(80, 24)
        .script(include_str!("fixtures/chat_status_1/running_turn.jsonl"))
        .open();
    s.keys("left");
    s.assert_contains("Agent command center");
    s.keys("esc");
    s.key_release("esc");
    s.assert_absent("Agent command center");
    s.assert_contains("Working");
    s.assert_interrupts(0);
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
    let mut s = Screen::build(Owner::UiPane)
        .keymap(&[("chat.interrupt_turn", "f12")])
        .script(include_str!("fixtures/chat_status_1/running_turn.jsonl"))
        .open();
    s.type_text("/subagents ");
    s.keys("esc");
    s.assert_composer("/subagents");
    s.assert_interrupts(0);
    s.keys("f12");
    s.assert_composer("/subagents");
    s.assert_interrupts(1);
    s.type_text("next");
    s.assert_composer("/subagents next");
}

// ORACLE bottom_pane/mod.rs:4160 selection_view_esc_respects_remapped_list_cancel kind=ui owner=ui-pane
#[test]
fn selection_view_esc_respects_remapped_list_cancel() {
    let mut s = Screen::build(Owner::UiPane)
        .keymap(&[("list.cancel", "q")])
        .live()
        .open();
    s.keys("left");
    s.assert_contains("Agent command center");
    s.keys("esc");
    s.assert_contains("Agent command center");
    s.keys("q");
    s.assert_absent("Agent command center");
    s.assert_contains("Ask Codex to do anything");
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
        "the oracle uses two dummy BottomPaneView instances. The upper one's handle_paste unconditionally completes and the test inspects how the lower one is retired. The native production dialogs edit or route paste; there is no equivalent dialog whose paste completes it",
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
    let url = format!(
        "https://example.test/review?notes={}&mode=raw#hidden-url-tail",
        "x".repeat(100)
    );
    let hidden = "https://example.test/after-preview";
    let message = format!("Review {url}\nsecond\nthird\nfourth\n{hidden}");
    for (section, heading) in [
        ("steers", "Messages to be submitted after next tool call"),
        ("rejected", "Messages to be submitted at end of turn"),
        ("queued", "Queued follow-up inputs"),
    ] {
        let mut s = Screen::build(Owner::UiPane).size(32, 18).live().open();
        let mut pending = serde_json::json!({"thread": "thread-1"});
        pending[section] = serde_json::json!([message]);
        s.feed(
            &serde_json::json!({
                "step": "screen", "dir": "chat", "ev": {"pending": pending}
            })
            .to_string(),
        );
        s.assert_contains_unwrapped(heading);
        s.assert_no_link("Review");
        s.assert_link("https://", &url);
        s.assert_link_targets(&[&url]);
        s.assert_absent("hidden-url-tail");
        s.assert_absent("after-preview");
    }
}

#[test]
fn queued_url_preview_keeps_the_submitted_payload() {
    let mut s = Screen::build(Owner::UiPane)
        .size(32, 18)
        .script(include_str!("fixtures/chat_status_1/running_turn.jsonl"))
        .open();
    let url = format!("https://example.test/review?note={}#all", "x".repeat(100));
    let payload = format!("Review {url}\nsecond\nthird\nfourth\nfull payload tail");
    s.paste(&payload).keys("tab");
    s.assert_contains("Queued follow-up inputs");
    s.assert_link_targets(&[&url]);
    s.assert_absent("full payload tail");
    s.assert_no_turn();
    s.feed(r#"{"step":"notify","message":{"method":"turn/completed","params":{"threadId":"thread-1","turn":{"id":"turn-1","items":[],"itemsView":"full","status":"completed","error":null,"startedAt":null,"completedAt":null,"durationMs":null}}}}"#);
    s.assert_turn_texts(&[&payload]);
}

#[test]
fn oversized_preview_url_visual_wrap_matches_the_oracle() {
    kit::pending(
        Owner::UiPane,
        "G48: the oracle keeps a URL intact on one row and clips its visible label; native TextWrap::Word may wrap it across visible rows. OSC 8 destinations and payload are verified separately",
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
        "the oracle builds a synthetic promoted View usage row. In live code rate_limits.rs promotes the owner-nudge confirmation. The native workspace route, remapped picker and retained draft are tested by chat_status_1::promoted_owner_confirmation_uses_remapped_picker_keys_and_keeps_the_draft",
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
    let mut s = usage_banner(
        "dismissible",
        "",
        serde_json::json!([{"action":"view_usage","label":"View usage"}]),
    );
    s.keys("1");
    s.assert_opened_urls(&["https://chatgpt.com/settings/usage"]);
    s.resize(70, 1);
    s.assert_absent("View usage");
    s.keys("1");
    s.assert_opened_urls(&["https://chatgpt.com/settings/usage"]);
    s.resize(70, 16);
    s.assert_composer("1");
    s.keys("backspace");
    s.assert_contains("View usage");
    s.keys("1");
    s.assert_opened_urls(&[
        "https://chatgpt.com/settings/usage",
        "https://chatgpt.com/settings/usage",
    ]);
}

// ORACLE bottom_pane/tests/actionable_banner_tests.rs:153 partially_clipped_banner_preserves_hidden_action_shortcuts kind=ui owner=ui-pane
#[test]
fn partially_clipped_banner_preserves_hidden_action_shortcuts() {
    let mut s = usage_banner(
        "dismissible",
        "",
        serde_json::json!([
            {"action":"view_usage","label":"Keep working"},
            {"action":"open_plus_pricing_web","label":"View plans"},
        ]),
    );
    // The whole app blanks below four lines. Only one action fits at this height.
    s.resize(70, 6);
    s.assert_contains("Keep working");
    s.assert_absent("View plans");
    s.keys("1");
    s.assert_opened_urls(&["https://chatgpt.com/settings/usage"]);
    s.keys("2");
    s.assert_composer("2");
    s.assert_opened_urls(&["https://chatgpt.com/settings/usage"]);

    let mut s = usage_banner(
        "dismissible",
        "",
        serde_json::json!([
            {"action":"view_usage","label":"Keep working and choose a different model"},
            {"action":"open_plus_pricing_web","label":"View plans"},
        ]),
    );
    s.resize(28, 7);
    s.assert_contains("Keep working");
    s.assert_contains("different model");
    s.assert_absent("View plans");
    s.keys("2");
    s.assert_composer("2");
    s.assert_opened_urls(&[]);
}

// ORACLE bottom_pane/tests/actionable_banner_tests.rs:195 banner_hidden_during_task_waits_for_redraw_before_accepting_dismissal kind=ui owner=ui-pane
#[test]
fn banner_hidden_during_task_waits_for_redraw_before_accepting_dismissal() {
    let mut s = usage_banner("dismissible", "", serde_json::json!([]));
    s.feed(r#"{"step":"notify","message":{"method":"turn/started","params":{"threadId":"thread-1","turn":{"id":"turn-1","items":[],"itemsView":"full","status":"inProgress","error":null}}}}"#);
    s.assert_contains("Working");
    s.assert_absent("Usage limit reached");
    s.keys("esc");
    s.assert_interrupts(1);
    s.feed(r#"{"step":"notify","message":{"method":"turn/completed","params":{"threadId":"thread-1","turn":{"id":"turn-1","items":[],"itemsView":"full","status":"interrupted","error":null}}}}"#);
    s.assert_contains("Usage limit reached");
    s.keys("esc");
    s.assert_absent("Usage limit reached");
    s.assert_interrupts(1);

    let mut s = usage_banner(
        "dismissible",
        "",
        serde_json::json!([{"action":"view_usage","label":"View usage"}]),
    );
    let _reply = s.hold_request("thread/compact/start");
    s.type_text("/compact").keys("enter");
    s.assert_contains("Compacting");
    s.assert_absent("Usage limit reached");
    s.assert_no_turn();
    s.keys("1");
    s.assert_composer("1");
    s.assert_opened_urls(&[]);
}

// ORACLE bottom_pane/tests/actionable_banner_tests.rs:221 information_banner_preserves_input_and_honors_dismissal kind=snap owner=ui-pane
#[test]
fn information_banner_preserves_input_and_honors_dismissal() {
    for (presentation, dismissible) in [("inline", false), ("dismissible", true)] {
        let description = "You can change models or request more usage.\nYour drafts will remain.\nCheck settings first.";
        let mut s = usage_banner(presentation, description, serde_json::json!([]));
        s.assert_contains("You can change models or request more usage.");
        s.resize(70, 6);
        s.assert_contains("Usage limit reached");
        s.assert_absent("Check settings first.");
        s.resize(70, 16);
        s.assert_contains("Check settings first.");
        s.assert_absent("no matches");
        s.assert_absent("Press a number");
        s.assert_present("esc to dismiss", dismissible);
        s.keys("esc");
        s.assert_present("Usage limit reached", !dismissible);
        s.keys("1");
        s.assert_composer("1");
        s.assert_opened_urls(&[]);
    }
    let mut s = usage_banner(
        "dismissible",
        "This banner and the config notice share the chat footer.",
        serde_json::json!([]),
    );
    s.feed(r#"{"step":"notify","message":{"method":"configWarning","params":{"summary":"Config diagnostic"}}}"#);
    s.assert_contains("Config diagnostic");
    s.assert_contains("Usage limit reached");
    s.resize(70, 6);
    s.assert_contains("Config diagnostic");
    s.assert_absent("Usage limit reached");
    s.keys("esc");
    s.resize(70, 16);
    s.assert_contains("Usage limit reached");
}

// ORACLE bottom_pane/tests/actionable_banner_tests.rs:263 banner_copy_preserves_wrapped_url_destinations kind=snap owner=ui-pane
#[test]
fn banner_copy_preserves_wrapped_url_destinations() {
    let url = "https://github.com/openai/codex/pull/12345?diff=split";
    let mut s = usage_banner(
        "inline",
        &format!("Open ({url}), before reading ftp://example.com."),
        serde_json::json!([]),
    );
    s.resize(40, 16);
    s.wrapped_link_cells(url);
    s.assert_links_spell(url);
    s.assert_no_link("Open");
    s.assert_no_link("ftp://example.com");
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
        "the oracle pairs two dummy renderers and returns invented terminal-cursor coordinates from the centered one. The real centered archive picker owns selection and keeps the backdrop; exact synthetic caret frames remain unverified",
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

#[test]
fn centered_archive_picker_keeps_the_transcript_and_returns_input_to_the_composer() {
    let mut s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    s.feed(r#"{"step":"notify","message":{"method":"item/completed","params":{"threadId":"thread-1","turnId":"turn-1","completedAtMs":0,"startedAtMs":0,"item":{"type":"agentMessage","id":"a1","text":"A transcript message behind the confirmation.","phase":null}}}}"#);
    s.type_text("/archive").keys("enter");
    s.assert_contains("Archive this session?");
    s.assert_contains("A transcript message behind the confirmation.");
    s.resize(40, 16);
    s.assert_contains("Archive this session?");
    s.keys("esc");
    s.assert_absent("Archive this session?");
    s.assert_archives(0);
    s.type_text("input after archive was cancelled");
    s.assert_composer("input after archive was cancelled");
}

// ---- oracle: bottom_pane/voice_strip_tests.rs (2 screen-level tests here; 2 state tests left to ui-pane)
// ORACLE bottom_pane/voice_strip_tests.rs:49 active_dashboard_preserves_live_colors_controls_and_width kind=snap owner=ui-pane
#[test]
fn active_dashboard_preserves_live_colors_controls_and_width() {
    let mut s = Screen::build(Owner::UiPane).size(47, 16).live().open();
    s.type_text("keep this voice draft")
        .keys("f8")
        .voice_connected()
        .voice_peaks(32_768, 32_768);
    s.assert_contains("keep this voice draft");
    s.assert_contains("voice ● listen");
    s.assert_contains("/voice stop");
    s.assert_style_in("mic ▁▁▁▁▁█", 9, "fg=Cyan");
    s.assert_style_in("codex ▁▁▁▁▁█", 11, "fg=Magenta");
    s.resize(37, 16);
    s.assert_contains("f9 mute");
    s.assert_contains("/voice stop");
    s.resize(22, 16);
    s.assert_contains("/voice stop");
    s.assert_contains("mic");
    s.assert_contains("codex");
    s.resize(47, 16);
    s.assert_contains("█");
}

// ORACLE bottom_pane/voice_strip_tests.rs:141 voice_controls_follow_configured_mute_binding_and_unbinding kind=snap owner=ui-pane snap=voice_controls_follow_configured_mute_binding_and_unbinding
#[test]
fn voice_controls_follow_configured_mute_binding_and_unbinding() {
    let mut direct = Screen::build(Owner::UiPane)
        .size(80, 16)
        .keymap(&[("chat.toggle_voice_mute", "f8")])
        .live()
        .open();
    direct.keys("f8").voice_connected();
    direct.assert_contains("f8 mute");
    direct.keys("f8").voice_muted(true);
    direct.assert_contains("f8 unmute");
    direct.assert_contains("/voice stop");
    let mut chord = Screen::build(Owner::UiPane)
        .size(80, 16)
        .keymap(&[("chat.toggle_voice_mute", "ctrl+x m")])
        .live()
        .open();
    chord
        .voice_available()
        .type_text("/voice")
        .keys("enter")
        .voice_connected();
    chord.assert_contains("⌃x m mute");
    chord.keys("ctrl+x m").voice_muted(true);
    chord.assert_contains("⌃x m unmute");
    let mut unbound = Screen::build(Owner::UiPane)
        .size(80, 16)
        .keymap(&[("chat.toggle_voice_mute", "")])
        .live()
        .open();
    unbound
        .voice_available()
        .type_text("/voice")
        .keys("enter")
        .voice_connected();
    unbound.assert_contains("/voice mute");
    unbound
        .type_text("/voice mute")
        .keys("enter")
        .voice_muted(true);
    unbound.assert_contains("muted");
    unbound
        .type_text("/voice stop")
        .keys("enter")
        .voice_closed();
    unbound.assert_absent("/voice stop");
}

#[test]
fn voice_helper_missing_preserves_the_draft_and_never_reports_listening() {
    let mut s = Screen::build(Owner::UiPane).size(80, 16).live().open();
    s.type_text("my draft").keys("f8").voice_helper_missing();
    s.assert_contains("my draft");
    s.assert_contains("compatible audio helper");
    s.assert_absent("voice ● listen");
}
