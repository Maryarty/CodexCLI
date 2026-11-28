//! Shard `composer_1` (screens S10, owner chat-live): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen};

const NO_FILES: &str = "the oracle draws these with `Feature::MentionsV2` off, where `@` opens the legacy file popup (a bare `loading...` list under the draft); the port is the default mode (V2 on: the unified popup with its header and tabs) and does not carry the legacy popup (decided: MentionsV2 off is not ported)";

/// The composer alone with its footer at `width` by `height`: the oracle's
/// composer harness (`snapshot_composer_state_with_width`).
fn alone(width: u16, height: u16) -> Screen {
    Screen::build(Owner::ChatLive)
        .size(width, height)
        .entry("composer", "alone")
        .open()
}

/// The oracle's composer with Vim on (`set_vim_enabled(true)`: Normal mode): the chat, where
/// `/vim` and Enter turn it on.
fn vim(width: u16, height: u16) -> Screen {
    let mut s = Screen::build(Owner::ChatLive)
        .size(width, height)
        .live()
        .open();
    s.type_text("/vim").keys("enter");
    s
}

/// Types `text`, then moves the cursor left `back` chars (the oracle's `set_cursor`).
/// The composer's prompt row, `›` or shell mode's `!` (the oracle's
/// composer rect sits there on the chat screen).
fn prompt_row(s: &Screen) -> usize {
    s.rows()
        .iter()
        .rposition(|r| r.starts_with('›') || r.starts_with("! ") || r == "!")
        .unwrap_or(usize::MAX)
}

fn type_then_back(s: &mut Screen, text: &str, back: usize) {
    s.type_text(text);
    if back > 0 {
        s.keys(&vec!["left"; back].join(" "));
    }
}

/// The composer alone over the skills of `set_skill_mentions` (`alone_skills`).
fn skills() -> Screen {
    Screen::build(Owner::ChatLive)
        .size(100, 9)
        .entry("popovers", "alone_skills")
        .open()
}

/// The composer alone over the plugins of `set_plugin_mentions` (`alone_plugins`).
fn plugins() -> Screen {
    Screen::build(Owner::ChatLive)
        .size(100, 9)
        .entry("popovers", "alone_plugins")
        .open()
}

/// `configure_partially_bound_skill_mentions`: `$unbound-skill  $bound-skill continue`, the
/// bound mention put in by a pick, the cursor before it.
fn unbound_left_of_bound() -> Screen {
    let mut s = skills();
    s.type_text("$bound-skill").keys("tab");
    s.type_text("continue").keys("home");
    s.type_text("$unbound-skill  ");
    s
}

// ---- oracle: bottom_pane/chat_composer.rs (50 screen-level tests here; 22 state tests left to chat-live)
// ORACLE bottom_pane/chat_composer.rs:5087 parent_owned_thread_placeholder_snapshot kind=snap owner=chat-live snap=parent_owned_thread_placeholder
#[test]
fn parent_owned_thread_placeholder_snapshot() {
    // The oracle draws the bare composer at 100 x 9 with a parent-owned thread set: the server
    // says the thread takes no direct input (`canAcceptDirectInput: false`).
    let mut s = Screen::build(Owner::ChatLive).size(100, 9).live().open();
    s.feed(
        r#"{"step":"notify","message":{"method":"thread/started","params":{"thread":{"id":"thread-1","environments":null,"extra":null,"sessionId":"thread-1","forkedFromId":null,"attachedToThreadId":null,"parentThreadId":null,"preview":"","ephemeral":false,"section":null,"sectionEnteredAt":null,"projectId":null,"historyMode":"legacy","modelProvider":"openai","model":null,"reasoningEffort":null,"createdAt":0,"updatedAt":0,"recencyAt":0,"status":{"type":"idle"},"path":null,"cwd":"/tmp/project","cliVersion":"0.0.0","originator":null,"source":"cli","canAcceptDirectInput":false,"threadSource":null,"agentNickname":null,"agentRole":null,"gitInfo":null,"name":null,"daybreakEnabled":null,"turns":[]}}}}"#,
    );
    s.assert_row(
        s.rows().iter().rposition(|r| r.starts_with('›')).unwrap(),
        "› Viewing sub-agent \u{2014} direct input is disabled",
    );
}

// ORACLE bottom_pane/chat_composer.rs:5096 light_terminal_palette_renders_light_composer_snapshot kind=snap owner=chat-live snap=light_terminal_palette_composer
#[test]
fn light_terminal_palette_renders_light_composer_snapshot() {
    kit::pending(
        Owner::ChatLive,
        "G-new-footer-palette: the composer draws the light surface (bg Rgb(244,244,244) at the prompt row and the rows and text equal the oracle's); the footer's key hints and hint text take no colour from the terminal's own foreground and background (the oracle: the key bold in blend(fg, bg, 0.85) on a light terminal, the hint text in a blended grey; ours: Reset and Indexed(245)), and a blank cell keeps the bold of the glyph before it (the presenter writes it without resetting)",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(48, 6)
        .light()
        .composer_alone()
        .open();
    s.assert_style(0, 1, "bg=Rgb(244,244,244)");
    s.assert_snap("light_terminal_palette_composer");
}

// ORACLE bottom_pane/chat_composer.rs:5119 footer_hint_row_is_separated_from_composer kind=ui owner=chat-live
#[test]
fn footer_hint_row_is_separated_from_composer() {
    let s = alone(40, 6);
    s.assert_row_starts_with(5, "  ? for shortcuts");
    s.assert_row(4, "");
}

// ORACLE bottom_pane/chat_composer.rs:5173 footer_flash_overrides_footer_hint_override kind=ui owner=chat-live
#[test]
fn footer_flash_overrides_footer_hint_override() {
    // The oracle's bottom row shows `FLASH` and not the hint override `K label`.
    let s = Screen::build(Owner::ChatLive)
        .size(60, 6)
        .entry("composer", "alone_flash")
        .open();
    s.assert_row_starts_with(5, "  FLASH");
    s.assert_absent("K label");
}

// ORACLE bottom_pane/chat_composer.rs:5211 footer_flash_expires_and_falls_back_to_hint_override kind=ui owner=chat-live
#[test]
fn footer_flash_expires_and_falls_back_to_hint_override() {
    // The flash has run out (the oracle moves its expiry into the past; here ten seconds
    // pass): the bottom row shows the hint override `K label`, not `FLASH`.
    let mut s = Screen::build(Owner::ChatLive)
        .size(60, 6)
        .entry("composer", "alone_flash")
        .open();
    s.assert_contains("FLASH");
    s.wait(10_500);
    s.assert_row_starts_with(5, "  K label");
    s.assert_absent("FLASH");
}

// ORACLE bottom_pane/chat_composer.rs:5294 footer_mode_snapshots kind=snap owner=chat-live snap=footer_mode_ctrl_c_quit snap=footer_mode_ctrl_c_then_esc_hint snap=footer_mode_esc_hint_backtrack snap=footer_mode_esc_hint_from_overlay snap=footer_mode_hidden_while_typing snap=footer_mode_history_search snap=footer_mode_history_search_unavailable snap=footer_mode_overlay_then_external_esc_hint snap=footer_mode_shell_command_absorbs_bang snap=footer_mode_shell_command_escape_exits_empty_mode snap=footer_mode_shortcut_overlay snap=footer_mode_shortcut_overlay_queue_submissions
#[test]
fn footer_mode_snapshots() {
    // Each oracle snapshot is the bare composer with one setter or key before it: the composer
    // alone, its entries holding the setters (`esc_backtrack_hint`, `queue_submissions`, the
    // quit hint, the status line).
    let at = |height: u16, entry: &'static str| {
        Screen::build(Owner::ChatLive)
            .size(100, height)
            .entry("composer", entry)
            .open()
    };

    let mut s = at(21, "alone_esc_hint");
    s.keys("?");
    s.assert_snap("footer_mode_shortcut_overlay");

    let mut s = at(21, "alone_queue_submissions");
    s.keys("?");
    s.assert_snap("footer_mode_shortcut_overlay_queue_submissions");

    let s = at(9, "alone_quit");
    s.assert_snap("footer_mode_ctrl_c_quit");

    let mut s = at(9, "alone_quit");
    s.keys("esc");
    s.assert_snap("footer_mode_ctrl_c_then_esc_hint");

    let mut s = at(9, "alone");
    s.keys("? esc");
    s.assert_snap("footer_mode_esc_hint_from_overlay");

    let mut s = at(9, "alone_esc_hint");
    s.keys("esc");
    s.assert_snap("footer_mode_esc_hint_backtrack");

    let mut s = at(9, "alone_esc_after_overlay");
    s.keys("?");
    s.assert_snap("footer_mode_overlay_then_external_esc_hint");

    let mut s = at(9, "alone");
    s.type_text("h");
    s.assert_snap("footer_mode_hidden_while_typing");

    let mut s = at(9, "alone");
    s.type_text("cargo test").keys("enter");
    s.keys("ctrl+r");
    s.type_text("c");
    s.assert_snap("footer_mode_history_search");

    // the file's rows cannot be read: the search gives up without saying `no match`
    let mut s = at(9, "alone_history_file");
    s.type_text("draft");
    s.keys("ctrl+r");
    s.type_text("git");
    s.assert_snap("footer_mode_history_search_unavailable");

    let mut s = at(9, "alone_shell_status");
    s.type_text("!git status");
    s.assert_snap("footer_mode_shell_command_absorbs_bang");

    let mut s = at(9, "alone_shell_status");
    s.type_text("!").keys("esc");
    s.assert_snap("footer_mode_shell_command_escape_exits_empty_mode");
}

// ORACLE bottom_pane/chat_composer.rs:5457 shell_command_uses_shell_accent_style kind=ui owner=chat-live
#[test]
fn shell_command_uses_shell_accent_style() {
    let mut s = Screen::build(Owner::ChatLive).size(100, 9).live().open();
    s.type_text("!git status");
    // the oracle's composer row 1 is the chat screen's prompt row here
    let row = prompt_row(&s);
    s.assert_style(0, row, "fg=LightRed");
    s.assert_text_at(0, row, "!");
    s.assert_text_style("Shell mode", "fg=LightRed");
}

// ORACLE bottom_pane/chat_composer.rs:5544 plugin_at_mentions_render_with_plugin_accent_snapshot kind=snap owner=chat-live snap=plugin_at_mentions_render_with_plugin_accent
#[test]
fn plugin_at_mentions_render_with_plugin_accent_snapshot() {
    kit::pending(
        Owner::ChatLive,
        "G-new-1: the composer's text area paints its text in one style and takes no per-range style, so a bound plugin `@` mention keeps no magenta (and an element no cyan); the bound mention itself is made",
    );
    // The oracle's snapshot is two lines: `text:    › @sample plugin` and `magenta:   ^^^^^^^`
    // (the seven cells of `@sample` are magenta); the kit compares the row and the style.
    let mut s = plugins();
    s.type_text("@sample").keys("tab");
    s.type_text("plugin");
    s.assert_has_row("› @sample plugin");
    s.assert_text_style("@sample", "fg=Magenta");
}

// ORACLE bottom_pane/chat_composer.rs:5595 recalled_plugin_at_mentions_keep_plugin_accent_style kind=ui owner=chat-live
#[test]
fn recalled_plugin_at_mentions_keep_plugin_accent_style() {
    kit::pending(
        Owner::ChatLive,
        "G-new-1: the composer's text area paints its text in one style and takes no per-range style, so a bound plugin `@` mention keeps no magenta (and an element no cyan); the bound mention itself is made",
    );
    // The oracle binds the `@sample` plugin mention, submits it, clears the draft and recalls it
    // with Up.
    let mut s = plugins();
    s.type_text("@sample").keys("tab");
    s.type_text("plugin").keys("enter");
    s.keys("up");
    s.assert_has_row("› @sample plugin");
    s.assert_text_style("@sample", "fg=Magenta");
}

// ORACLE bottom_pane/chat_composer.rs:5631 status_line_hyperlink_marks_pr_number_cells kind=ui owner=chat-live
#[test]
fn status_line_hyperlink_marks_pr_number_cells() {
    let s = Screen::build(Owner::ChatLive)
        .size(40, 6)
        .script(include_str!("fixtures/composer_1/pr_link.jsonl"))
        .open();
    s.assert_link("PR #20252", "https://github.com/openai/codex/pull/20252");
}

// ORACLE bottom_pane/chat_composer.rs:5668 deleting_prefix_keeps_bang_literal_on_submission kind=ui owner=chat-live
#[test]
fn deleting_prefix_keeps_bang_literal_on_submission() {
    // The oracle deletes the leading `Y` of `Yes! I agree.`, so the `!` is no shell prefix;
    // the draft is submitted (Enter) or queued (Tab while a turn runs) as `! I agree.!`.
    let mut s = Screen::build(Owner::ChatLive).size(100, 9).live().open();
    s.type_text("Yes! I agree.")
        .keys("home delete delete delete");
    s.assert_has_row("› ! I agree.");
    s.keys("end");
    s.type_text("!");
    s.keys("enter");
    s.assert_contains("! I agree.!");
}

// ORACLE bottom_pane/chat_composer.rs:5708 deleting_prefix_keeps_bang_editable_snapshot kind=snap owner=chat-live snap=deleting_prefix_keeps_bang_literal
#[test]
fn deleting_prefix_keeps_bang_editable_snapshot() {
    let mut s = alone(60, 9);
    s.paste("Yes! I agree.").keys("home delete delete delete");
    s.assert_snap("deleting_prefix_keeps_bang_literal");
}

// ORACLE bottom_pane/chat_composer.rs:5733 esc_exits_empty_shell_mode kind=ui owner=chat-live
#[test]
fn esc_exits_empty_shell_mode() {
    let mut s = Screen::build(Owner::ChatLive).size(100, 9).live().open();
    s.type_text("!");
    s.assert_text_at(0, prompt_row(&s), "!");
    s.assert_contains("Shell mode");
    s.keys("esc");
    s.assert_absent("Shell mode");
    s.assert_contains("Ask Codex to do anything");
}

// ORACLE bottom_pane/chat_composer.rs:5762 esc_keeps_shell_mode_when_paste_burst_flushes_pending_text kind=ui owner=chat-live
#[test]
fn esc_keeps_shell_mode_when_paste_burst_flushes_pending_text() {
    // `g` right after `!` is held as a paste burst; Esc flushes it and keeps the shell mode.
    let mut s = Screen::build(Owner::ChatLive).size(100, 9).live().open();
    s.type_text("!g");
    s.keys("esc");
    // the oracle's `current_text` is `!g`: the `!` is the prompt
    s.assert_has_row("! g");
    s.assert_contains("Shell mode");
}

// ORACLE bottom_pane/chat_composer.rs:5792 footer_collapse_snapshots kind=snap owner=chat-live snap=footer_collapse_empty_full snap=footer_collapse_empty_mode_cycle_with_context snap=footer_collapse_empty_mode_cycle_without_context snap=footer_collapse_empty_mode_only snap=footer_collapse_plan_empty_full snap=footer_collapse_plan_empty_mode_cycle_with_context snap=footer_collapse_plan_empty_mode_cycle_without_context snap=footer_collapse_plan_empty_mode_only snap=footer_collapse_plan_queue_full snap=footer_collapse_plan_queue_message_without_context snap=footer_collapse_plan_queue_mode_only snap=footer_collapse_plan_queue_short_with_context snap=footer_collapse_plan_queue_short_without_context snap=footer_collapse_queue_full snap=footer_collapse_queue_message_without_context snap=footer_collapse_queue_mode_only snap=footer_collapse_queue_short_with_context snap=footer_collapse_queue_short_without_context
#[test]
fn footer_collapse_snapshots() {
    // Each oracle snapshot is the bare composer at its own width, 9 rows. The `queue` variants
    // type `Test` while a turn runs (98% of the context left); the `plan` variants are in Plan
    // mode.
    let empty = [
        (120, "full"),
        (60, "mode_cycle_with_context"),
        (44, "mode_cycle_without_context"),
        (26, "mode_only"),
    ];
    let queue = [
        (120, "full"),
        (50, "short_with_context"),
        (40, "message_without_context"),
        (30, "short_without_context"),
        (20, "mode_only"),
    ];
    for (entry, name, widths, typing) in [
        ("alone", "empty", &empty[..], false),
        ("alone_plan", "plan_empty", &empty[..], false),
        ("alone_queue", "queue", &queue[..], true),
        ("alone_plan_queue", "plan_queue", &queue[..], true),
    ] {
        for &(width, tail) in widths {
            let mut s = Screen::build(Owner::ChatLive)
                .size(width, 9)
                .entry("composer", entry)
                .open();
            if typing {
                s.type_text("Test");
            }
            s.assert_snap(&format!("footer_collapse_{name}_{tail}"));
        }
    }
}

// ORACLE bottom_pane/chat_composer.rs:6031 esc_hint_stays_hidden_with_draft_content kind=ui owner=chat-live
#[test]
fn esc_hint_stays_hidden_with_draft_content() {
    // With a draft, Esc does not raise the `esc again to edit previous message` hint.
    let mut s = Screen::build(Owner::ChatLive).size(100, 9).live().open();
    s.type_text("d");
    s.keys("esc");
    s.assert_has_row("› d");
    s.assert_absent("esc again to edit previous message");
}

// ORACLE bottom_pane/chat_composer.rs:6060 empty_vim_insert_escape_enters_normal_without_esc_hint kind=snap owner=chat-live snap=vim_replace_mode
#[test]
fn empty_vim_insert_escape_enters_normal_without_esc_hint() {
    let mut s = vim(100, 9);
    s.keys("i");
    s.assert_text_style("Vim: Insert", "fg=Green");
    s.keys("esc");
    s.assert_text_style("Vim: Normal", "fg=Magenta");
    s.assert_absent("esc again to edit previous message");
    s.keys("R");
    s.assert_text_style("Vim: Replace", "fg=Cyan");

    // The oracle's snapshot: vim on, `R`, then a paste over the large-paste threshold.
    let mut r = Screen::build(Owner::ChatLive)
        .size(100, 9)
        .keymap(&[("global.toggle_vim_mode", "f9")])
        .entry("composer", "alone")
        .open();
    r.keys("f9 R");
    r.paste(&"x".repeat(1001));
    r.assert_snap("vim_replace_mode");
}

// ORACLE bottom_pane/chat_composer.rs:6123 slash_opens_command_popup_in_vim_insert_mode kind=ui owner=chat-live
#[test]
fn slash_opens_command_popup_in_vim_insert_mode() {
    // The command popup's rows are the chat_commands slice's; this test asserts the draft and
    // the mode, and the popup opens as the `/` is typed.
    let mut s = vim(100, 24);
    s.keys("i");
    s.type_text("/");
    s.assert_has_row("› /");
    s.assert_text_style("Vim: Insert", "fg=Green");
}

// ORACLE bottom_pane/chat_composer.rs:6155 slash_command_dispatches_after_vim_insert_slash kind=ui owner=chat-live
#[test]
fn slash_command_dispatches_after_vim_insert_slash() {
    let mut s = vim(100, 9);
    s.keys("i");
    s.type_text("/diff");
    s.assert_has_row("› /diff");
    s.keys("enter");
    s.assert_contains("Ask Codex to do anything");
    // (the command's sheet dims the footer under its barrier: the label's word, not its colour)
    s.assert_contains("Vim: Insert");
}

// ORACLE bottom_pane/chat_composer.rs:6190 inline_slash_command_dispatch_resets_vim_mode_to_insert kind=ui owner=chat-live
#[test]
fn inline_slash_command_dispatch_resets_vim_mode_to_insert() {
    let mut s = vim(100, 9);
    s.keys("i");
    s.type_text("/plan investigate this").keys("enter");
    s.assert_turn_texts(&["investigate this"]);
    s.assert_text_style("Vim: Insert", "fg=Green");
    s.type_text("x").keys("esc");
    s.type_text(".");
    s.assert_has_row("› xx");
}

// ORACLE bottom_pane/chat_composer.rs:6236 bang_enters_shell_mode_in_vim_normal_mode kind=ui owner=chat-live
#[test]
fn bang_enters_shell_mode_in_vim_normal_mode() {
    let mut s = vim(100, 9);
    s.keys("!");
    s.assert_text_at(0, prompt_row(&s), "!");
    s.assert_contains("Shell mode");
    s.assert_text_style("Vim: Insert", "fg=Green");
}

// ORACLE bottom_pane/chat_composer.rs:6267 shell_command_can_be_typed_after_vim_normal_bang kind=ui owner=chat-live
#[test]
fn shell_command_can_be_typed_after_vim_normal_bang() {
    let mut s = vim(100, 9);
    s.keys("! e c h o");
    s.assert_has_row("! echo");
}

// ORACLE bottom_pane/chat_composer.rs:6342 clear_for_ctrl_c_preserves_pending_paste_history_entry kind=ui owner=chat-live
#[test]
fn clear_for_ctrl_c_preserves_pending_paste_history_entry() {
    // The oracle pastes 1005 chars: the draft shows the placeholder; Ctrl+C clears it; Up
    // recalls the entry with the placeholder again.
    let mut s = Screen::build(Owner::ChatLive).size(100, 9).live().open();
    s.paste(&"x".repeat(1005));
    s.assert_has_row("› [Pasted Content 1005 chars]");
    s.keys("ctrl+c");
    s.assert_contains("Ask Codex to do anything");
    s.keys("up");
    s.assert_has_row("› [Pasted Content 1005 chars]");
}

// ORACLE bottom_pane/chat_composer.rs:6414 large_paste_numbering_reuses_after_ctrl_c_clear kind=ui owner=chat-live
#[test]
fn large_paste_numbering_reuses_after_ctrl_c_clear() {
    let mut s = Screen::build(Owner::ChatLive).size(100, 9).live().open();
    s.paste(&"x".repeat(1004));
    s.assert_has_row("› [Pasted Content 1004 chars]");
    s.keys("ctrl+c");
    s.assert_contains("Ask Codex to do anything");
    s.paste(&"x".repeat(1004));
    s.assert_has_row("› [Pasted Content 1004 chars]");
}

// ORACLE bottom_pane/chat_composer.rs:6443 vim_mode_resets_to_insert_after_submission kind=ui owner=chat-live
#[test]
fn vim_mode_resets_to_insert_after_submission() {
    let mut s = vim(100, 9);
    s.assert_text_style("Vim: Normal", "fg=Magenta");
    s.keys("i");
    s.type_text("h");
    s.keys("enter");
    s.assert_text_style("Vim: Insert", "fg=Green");
    s.assert_contains("Ask Codex to do anything");
}

// ORACLE bottom_pane/chat_composer.rs:6484 vim_mode_resets_to_insert_after_queued_submission kind=ui owner=chat-live
#[test]
fn vim_mode_resets_to_insert_after_queued_submission() {
    // A turn is running (the first line's), so Tab queues the draft.
    let mut s = vim(100, 12);
    s.keys("i");
    s.type_text("go").keys("enter");
    s.type_text("queued").keys("tab");
    s.assert_text_style("Vim: Insert", "fg=Green");
    s.assert_contains("queued");
}

// ORACLE bottom_pane/chat_composer.rs:6518 vim_mode_stays_insert_after_suppressed_submission kind=ui owner=chat-live
#[test]
fn vim_mode_stays_insert_after_suppressed_submission() {
    let mut s = vim(100, 9);
    s.keys("i");
    s.type_text("/not-a-command");
    s.keys("enter");
    s.assert_has_row("› /not-a-command");
    s.assert_text_style("Vim: Insert", "fg=Green");
}

// ORACLE bottom_pane/chat_composer.rs:6549 esc_switches_vim_insert_to_normal kind=ui owner=chat-live
#[test]
fn esc_switches_vim_insert_to_normal() {
    let mut s = vim(100, 9);
    s.keys("i");
    s.type_text("hey");
    s.assert_text_style("Vim: Insert", "fg=Green");
    s.assert_cursor(5, prompt_row(&s));
    s.keys("esc");
    s.assert_text_style("Vim: Normal", "fg=Magenta");
    s.assert_cursor(4, prompt_row(&s));
}

// ORACLE bottom_pane/chat_composer.rs:6586 vim_insert_uses_bar_cursor_style kind=ui owner=chat-live
#[test]
fn vim_insert_uses_bar_cursor_style() {
    kit::na(
        Owner::ChatLive,
        "asserts the cursor-shape escape the composer queues (default, then steady bar in vim insert, default again in normal and replace), a terminal state with no cell on the screen; the kit compares cells and the cursor position only",
    );
}

// ORACLE bottom_pane/chat_composer.rs:6756 question_mark_only_toggles_on_first_char kind=ui owner=chat-live
#[test]
fn question_mark_only_toggles_on_first_char() {
    let mut s = alone(100, 9);
    let prompt = s.shot();
    s.keys("?");
    s.assert_rows_differ(&prompt);
    s.keys("?");
    s.assert_same_rows(&prompt);
    s.type_text("h");
    s.assert_has_row("› h");
    s.keys("?");
    s.assert_has_row("› h?");
}

// ORACLE bottom_pane/chat_composer.rs:6796 shift_question_mark_toggles_shortcut_overlay_when_empty kind=ui owner=chat-live
#[test]
fn shift_question_mark_toggles_shortcut_overlay_when_empty() {
    let mut s = alone(100, 9);
    let prompt = s.shot();
    // a shifted `?` is the character `?` (the kit names the key by its text)
    s.keys("?");
    s.assert_rows_differ(&prompt);
}

// ORACLE bottom_pane/chat_composer.rs:6822 question_mark_does_not_toggle_during_paste_burst kind=ui owner=chat-live
#[test]
fn question_mark_does_not_toggle_during_paste_burst() {
    // The keys arrive as one burst, so the `?` is pasted text, not the overlay key.
    let mut s = Screen::build(Owner::ChatLive).size(100, 9).live().open();
    s.type_text("hi?there");
    s.assert_has_row("› hi?there");
}

// ORACLE bottom_pane/chat_composer.rs:7128 skill_popup_targets_unbound_mention_left_of_bound_mention kind=snap owner=chat-live snap=skill_popup_targets_unbound_mention_left_of_bound_mention
#[test]
fn skill_popup_targets_unbound_mention_left_of_bound_mention() {
    // The oracle's seed: `$unbound-skill  $bound-skill continue`, `$bound-skill` bound, the cursor
    // before it. Ours binds it by a pick and writes the rest around it.
    let mut s = unbound_left_of_bound();
    s.keys("enter");
    s.assert_has_row("› $unbound-skill  $bound-skill continue");
    s.type_text("foo");
    s.assert_has_row("› $unbound-skill foo $bound-skill continue");
}

// ORACLE bottom_pane/chat_composer.rs:7153 skill_popup_targets_unbound_mention_left_of_bound_mention_snapshot kind=snap owner=chat-live snap=skill_popup_targets_unbound_mention_left_of_bound_mention
#[test]
fn skill_popup_targets_unbound_mention_left_of_bound_mention_snapshot() {
    let s = unbound_left_of_bound();
    s.assert_snap("skill_popup_targets_unbound_mention_left_of_bound_mention");
}

// ORACLE bottom_pane/chat_composer.rs:7162 skill_popup_prefers_confirmed_selections_within_match_tier kind=snap owner=chat-live snap=skill_popup_prefers_confirmed_selections_within_match_tier
#[test]
fn skill_popup_prefers_confirmed_selections_within_match_tier() {
    // The oracle confirms `$hello-statsig` twice and `$status` once, then types `$stat`: the
    // confirmed picks are the usage file the fixture holds.
    let mut s = Screen::build(Owner::ChatLive)
        .size(100, 9)
        .entry("popovers", "alone_skills_confirmed")
        .open();
    s.type_text("$stat");
    s.assert_snap("skill_popup_prefers_confirmed_selections_within_match_tier");
}

// ORACLE bottom_pane/chat_composer.rs:7197 skill_popup_targets_unbound_mention_right_of_adjacent_bound_mention_snapshot kind=snap owner=chat-live snap=skill_popup_targets_unbound_mention_right_of_adjacent_bound_mention
#[test]
fn skill_popup_targets_unbound_mention_right_of_adjacent_bound_mention_snapshot() {
    // `$bound-skill$unbound-skill`: the bound mention first, the cursor after it.
    let mut s = skills();
    s.type_text("$bound-skill").keys("tab");
    s.keys("backspace");
    s.type_text("$unbound-skill")
        .keys(&vec!["left"; 14].join(" "));
    s.assert_snap("skill_popup_targets_unbound_mention_right_of_adjacent_bound_mention");
}

// ORACLE bottom_pane/chat_composer.rs:7206 skill_popup_falls_back_from_bound_skill_with_path_suffix_snapshot kind=snap owner=chat-live snap=skill_popup_falls_back_from_bound_skill_with_path_suffix
#[test]
fn skill_popup_falls_back_from_bound_skill_with_path_suffix_snapshot() {
    // `$left  $bound/path` with `$bound` bound and the cursor after `$left  `.
    let mut s = skills();
    s.type_text("$bound").keys("tab backspace");
    s.type_text("/path").keys("home");
    s.type_text("$left  ");
    s.assert_snap("skill_popup_falls_back_from_bound_skill_with_path_suffix");
}

// ORACLE bottom_pane/chat_composer.rs:7225 skill_popup_closes_after_trailing_space_snapshot kind=snap owner=chat-live snap=skill_popup_closes_after_trailing_space
#[test]
fn skill_popup_closes_after_trailing_space_snapshot() {
    let mut s = skills();
    s.type_text("$figma ");
    s.assert_snap("skill_popup_closes_after_trailing_space");
}

// ORACLE bottom_pane/chat_composer.rs:7234 unified_mention_popup_falls_back_from_bound_plugin_on_right_snapshot kind=snap owner=chat-live snap=unified_mention_popup_falls_back_from_bound_plugin_on_right
#[test]
fn unified_mention_popup_falls_back_from_bound_plugin_on_right_snapshot() {
    // `@left  @bound` with `@bound` bound and the cursor after `@left `.
    let mut s = plugins();
    s.type_text("@bound").keys("tab home");
    s.type_text("@left  ").keys("left");
    s.assert_snap("unified_mention_popup_falls_back_from_bound_plugin_on_right");
}

// ORACLE bottom_pane/chat_composer.rs:7243 adjacent_plugin_completion_inserts_separator_snapshot kind=snap owner=chat-live snap=adjacent_plugin_completion_inserts_separator
#[test]
fn adjacent_plugin_completion_inserts_separator_snapshot() {
    // `@sample@other` with `@sample` bound and the cursor on it; Enter completes the other plugin.
    let mut s = plugins();
    s.type_text("@sample").keys("tab backspace");
    s.type_text("@other").keys(&["left"; 6].join(" "));
    s.keys("enter");
    s.assert_snap("adjacent_plugin_completion_inserts_separator");
}

// ORACLE bottom_pane/chat_composer.rs:7256 skill_completion_replaces_only_middle_adjacent_target kind=ui owner=chat-live
#[test]
fn skill_completion_replaces_only_middle_adjacent_target() {
    // The oracle's seed: `$bound1 $oth $bound2` with the spaces cut out, the cursor in `$oth`.
    let mut s = skills();
    s.type_text("$bound2").keys("tab backspace home");
    s.type_text("$oth").keys("home");
    s.type_text("$bound1").keys("tab backspace");
    s.keys("right right enter");
    s.assert_has_row("› $bound1 $other $bound2");
}

// ORACLE bottom_pane/chat_composer.rs:7274 unified_completion_replaces_only_middle_adjacent_target kind=ui owner=chat-live
#[test]
fn unified_completion_replaces_only_middle_adjacent_target() {
    // The oracle's seed: `@bound1 @oth @bound2` with the spaces cut out, the cursor in `@oth`.
    let mut s = plugins();
    // Written as the oracle writes it, `@bound1 @oth @bound2` with the two spaces cut out:
    // typed next to each other the three would be one token.
    s.type_text("@bound1").keys("tab");
    s.type_text("@oth ");
    // (Left and Right are the popup's tabs while the caret is in a token: Ctrl+B and Ctrl+F move.)
    s.type_text("@bound2")
        .keys("tab backspace ctrl+b backspace");
    s.keys("ctrl+b ctrl+b ctrl+b ctrl+b backspace");
    s.keys("ctrl+f ctrl+f enter");
    s.assert_has_row("› @bound1 @other @bound2");
}

// ORACLE bottom_pane/chat_composer.rs:7312 file_popup_ignores_shell_positional_parameter_snapshot kind=snap owner=chat-live snap=file_popup_ignores_shell_positional_parameter
#[test]
fn file_popup_ignores_shell_positional_parameter_snapshot() {
    kit::na(Owner::ChatLive, NO_FILES);
}

// ORACLE bottom_pane/chat_composer.rs:7328 file_popup_ignores_bare_shell_parameter_with_matching_skill_snapshot kind=snap owner=chat-live snap=file_popup_ignores_bare_shell_parameter_with_matching_skill
#[test]
fn file_popup_ignores_bare_shell_parameter_with_matching_skill_snapshot() {
    kit::na(Owner::ChatLive, NO_FILES);
}

// ORACLE bottom_pane/chat_composer.rs:7377 file_popup_ignores_unbindable_qualified_skill_snapshot kind=snap owner=chat-live snap=file_popup_ignores_unbindable_qualified_skill
#[test]
fn file_popup_ignores_unbindable_qualified_skill_snapshot() {
    kit::na(Owner::ChatLive, NO_FILES);
}

// ORACLE bottom_pane/chat_composer.rs:7393 skill_popup_preserves_normal_target_after_ambiguous_probe_snapshot kind=snap owner=chat-live snap=skill_popup_preserves_normal_target_after_ambiguous_probe
#[test]
fn skill_popup_preserves_normal_target_after_ambiguous_probe_snapshot() {
    let mut s = skills();
    type_then_back(&mut s, "$1x  $plugin:d", 10);
    s.assert_snap("skill_popup_preserves_normal_target_after_ambiguous_probe");
}

// ORACLE bottom_pane/chat_composer.rs:7446 skill_popup_accepts_digit_leading_skill_snapshot kind=snap owner=chat-live snap=skill_popup_accepts_digit_leading_skill
#[test]
fn skill_popup_accepts_digit_leading_skill_snapshot() {
    let mut s = skills();
    s.type_text("$1p");
    s.assert_snap("skill_popup_accepts_digit_leading_skill");
}

// ORACLE bottom_pane/chat_composer.rs:7462 skill_popup_does_not_fuzzy_match_shell_variable_snapshot kind=snap owner=chat-live snap=skill_popup_does_not_fuzzy_match_shell_variable
#[test]
fn skill_popup_does_not_fuzzy_match_shell_variable_snapshot() {
    let mut s = skills();
    s.type_text("$HOME");
    s.assert_snap("skill_popup_does_not_fuzzy_match_shell_variable");
}

// ORACLE bottom_pane/chat_composer.rs:7605 skill_popup_closes_between_spaces_before_plain_text_snapshot kind=snap owner=chat-live snap=skill_popup_closes_between_spaces_before_plain_text
#[test]
fn skill_popup_closes_between_spaces_before_plain_text_snapshot() {
    // `$old  word` with the cursor after `$old `: the popup closes.
    let mut s = skills();
    type_then_back(&mut s, "$old  word", 5);
    s.assert_snap("skill_popup_closes_between_spaces_before_plain_text");
}

// ORACLE bottom_pane/chat_composer.rs:7783 plugin_mention_popup_snapshot kind=snap owner=chat-live snap=plugin_mention_popup
#[test]
fn plugin_mention_popup_snapshot() {
    kit::na(
        Owner::ChatLive,
        "the oracle draws this with `Feature::MentionsV2` off, where the `$` list holds plugins; the port is the default mode (V2 on: `$` lists skills and apps, plugins under `@`) and does not carry the legacy lists (decided: MentionsV2 off is not ported)",
    );
}
