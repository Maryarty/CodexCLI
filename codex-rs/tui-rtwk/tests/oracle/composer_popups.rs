//! Shard `composer_popups` (screens S12, owner chat-nav-pane): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen};

/// The composer's prompt row (`›`): the last one, since a popup sits above the composer.
fn prompt(s: &Screen) -> usize {
    s.rows()
        .iter()
        .rposition(|r| r.starts_with('›'))
        .unwrap_or(usize::MAX)
}

/// The first row that starts with `text`: where a popup of the oracle snapshot begins.
fn row_starting(s: &Screen, text: &str) -> usize {
    s.rows()
        .iter()
        .position(|r| r.starts_with(text))
        .unwrap_or(usize::MAX)
}

/// `key` pressed `n` times, as one key string.
fn times(key: &str, n: usize) -> String {
    vec![key; n].join(" ")
}

/// History `remembered command`, draft `draft` with the cursor at 2, Ctrl+R, then `r`:
/// the preview shows the entry (history_search.rs, `history_search_ctrl_c_restores_original_draft`).
fn remembered_search_preview() -> Screen {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .script(include_str!("fixtures/composer_popups/h_remembered.jsonl"))
        .open();
    s.type_text("draft").keys(&times("left", 3)).keys("ctrl+r");
    s.type_text("r");
    s.assert_row(prompt(&s), "› remembered command");
    s
}

/// A 1001-char paste as the draft, then Ctrl+R and `git` (history_search_paste_tests.rs,
/// `history_search_paste_preserves_original_draft_on_miss_and_cancel`).
fn large_draft_search() -> Screen {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .script(include_str!("fixtures/composer_popups/h_paste.jsonl"))
        .open();
    s.type_text("draft");
    s.paste(&"x".repeat(1001));
    // the oracle's `set_cursor(2)`
    s.keys("home right right");
    s.keys("ctrl+r");
    s.paste("git");
    s.assert_row(prompt(&s), "› git log");
    s.paste(" missing");
    s.assert_row(prompt(&s), "› draft[Pasted Content 1001 chars]");
    s
}

// ---- oracle: bottom_pane/chat_composer/history_search.rs (16 screen-level tests here; 3 state tests left to chat-nav-pane)
// ORACLE bottom_pane/chat_composer/history_search.rs:554 vim_normal_j_k_navigate_history_at_history_boundaries kind=ui owner=chat-nav-pane
/// The match highlight of Ctrl+R (`history_search_highlight_ranges`): the matched cells reversed and bold.
/// G-new-1: rtwk's `TextArea` paints no style per range of its text (a Flutter `TextEditingController.buildTextSpan`
/// override), so the preview cannot mark the match.
const MATCH_HIGHLIGHT: &str = "G-new-1: rtwk TextArea paints no per-range style, so the matched part of the previewed entry is not reversed and bold";

/// G68: the oracle's popup is the pane's rows under the prompt, its name and description in aligned
/// columns with a `›` marker and a hint line; rtwk's `OptionList` in a `Popover` floats above the
/// prompt with a `▎` bar and a right-aligned detail.
const POPUP_ROWS: &str = "G68: the popup is a floating OptionList (a `▎` bar, the detail right-aligned, no `enter insert · esc close` line), the oracle's are the pane's rows under the prompt in aligned name/description columns with a `›` marker";

#[test]
fn vim_normal_j_k_navigate_history_at_history_boundaries() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .script(include_str!(
            "fixtures/composer_popups/h_first_second.jsonl"
        ))
        .keymap(&[("global.toggle_vim_mode", "f9")])
        .open();
    s.keys("f9 esc");
    s.keys("k");
    s.assert_row(prompt(&s), "› second");
    s.keys("k");
    s.assert_row(prompt(&s), "› first");
    s.keys("j");
    s.assert_row(prompt(&s), "› second");
    s.keys("j");
    s.assert_row(prompt(&s), "› Ask Codex to do anything");
}

// ORACLE bottom_pane/chat_composer/history_search.rs:602 remapped_vim_normal_history_navigation_does_not_fall_back_to_j_k kind=ui owner=chat-nav-pane
#[test]
fn remapped_vim_normal_history_navigation_does_not_fall_back_to_j_k() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .script(include_str!("fixtures/composer_popups/h_first.jsonl"))
        .keymap(&[
            ("global.toggle_vim_mode", "f9"),
            ("vim_normal.move_up", "f2"),
            ("vim_normal.move_down", "f3"),
        ])
        .open();
    s.keys("f9 esc");
    s.keys("k");
    s.assert_row(prompt(&s), "› Ask Codex to do anything");
    s.keys("f2");
    s.assert_row(prompt(&s), "› first");
    s.keys("f3");
    s.assert_row(prompt(&s), "› Ask Codex to do anything");
}

// ORACLE bottom_pane/chat_composer/history_search.rs:638 vim_normal_j_k_fall_back_to_multiline_cursor_movement kind=ui owner=chat-nav-pane
#[test]
fn vim_normal_j_k_fall_back_to_multiline_cursor_movement() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .script(include_str!("fixtures/composer_popups/h_empty.jsonl"))
        .keymap(&[("global.toggle_vim_mode", "f9")])
        .open();
    s.keys("f9");
    s.paste("one\ntwo");
    // the oracle's `set_cursor(0)`: the first line's start
    s.keys("esc k 0");
    let first = prompt(&s);
    s.keys("j");
    s.assert_cursor(2, first + 1);
    s.keys("k");
    s.assert_cursor(2, first);
}

// ORACLE bottom_pane/chat_composer/history_search.rs:663 vim_normal_operator_motion_does_not_navigate_history kind=ui owner=chat-nav-pane
#[test]
fn vim_normal_operator_motion_does_not_navigate_history() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .script(include_str!(
            "fixtures/composer_popups/h_first_second.jsonl"
        ))
        .keymap(&[("global.toggle_vim_mode", "f9")])
        .open();
    s.keys("f9 esc");
    s.keys("k");
    s.assert_row(prompt(&s), "› second");
    s.keys("d k");
    s.assert_row(prompt(&s), "› Ask Codex to do anything");
}

// ORACLE bottom_pane/chat_composer/history_search.rs:696 history_search_opens_without_previewing_latest_entry kind=ui owner=chat-nav-pane
#[test]
fn history_search_opens_without_previewing_latest_entry() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .script(include_str!("fixtures/composer_popups/h_remembered.jsonl"))
        .open();
    s.keys("ctrl+r");
    s.assert_absent("remembered command");
    s.assert_contains("reverse-i-search:");
}

// ORACLE bottom_pane/chat_composer/history_search.rs:777 history_search_accepts_matching_entry kind=ui owner=chat-nav-pane
#[test]
fn history_search_accepts_matching_entry() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .script(include_str!("fixtures/composer_popups/h_git_cargo.jsonl"))
        .open();
    s.type_text("draft").keys("ctrl+r");
    s.assert_row(prompt(&s), "› draft");
    s.type_text("git");
    s.assert_row(prompt(&s), "› git status");
    s.assert_contains("reverse-i-search: git");
    s.keys("enter");
    s.assert_row(prompt(&s), "› git status");
    s.assert_absent("reverse-i-search");
}

// ORACLE bottom_pane/chat_composer/history_search.rs:815 vim_normal_history_search_preview_places_cursor_on_last_char kind=ui owner=chat-nav-pane
#[test]
fn vim_normal_history_search_preview_places_cursor_on_last_char() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .script(include_str!("fixtures/composer_popups/h_git_status.jsonl"))
        // the oracle clears `vim_normal.redo`: Ctrl+R is free for the search
        .keymap(&[("global.toggle_vim_mode", "f9"), ("vim_normal.redo", "")])
        .open();
    s.keys("f9 esc");
    s.keys("ctrl+r");
    s.type_text("git");
    s.assert_row(prompt(&s), "› git status");
    s.assert_contains("reverse-i-search: git");
    // the caret is on the last character of the preview
    s.assert_cursor(2 + "git statu".len(), prompt(&s));
}

// ORACLE bottom_pane/chat_composer/history_search.rs:844 history_search_stays_on_single_match_at_boundaries kind=ui owner=chat-nav-pane
#[test]
fn history_search_stays_on_single_match_at_boundaries() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .script(include_str!("fixtures/composer_popups/h_find.jsonl"))
        .open();
    s.type_text("draft").keys("ctrl+r");
    s.type_text("bug");
    s.assert_row(prompt(&s), "› Find and fix a bug in @filename");
    s.keys(&times("ctrl+r", 3));
    s.assert_row(prompt(&s), "› Find and fix a bug in @filename");
    s.keys(&times("down", 3));
    s.assert_row(prompt(&s), "› Find and fix a bug in @filename");
}

// ORACLE bottom_pane/chat_composer/history_search.rs:899 history_search_footer_action_hints_are_emphasized kind=ui owner=chat-nav-pane
#[test]
fn history_search_footer_action_hints_are_emphasized() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .script(include_str!("fixtures/composer_popups/h_git_cargo.jsonl"))
        .open();
    s.keys("ctrl+r");
    s.type_text("c");
    s.assert_contains("reverse-i-search: c  enter accept · esc cancel");
    // The hints' emphasis: the keys bold, the words and the separator dim (their foreground is the
    // probed terminal foreground (240, 240, 240): the kit has no terminal-colour seed for that part).
    s.assert_text_style("enter", "bold !dim");
    s.assert_text_style("accept", "dim");
    s.assert_text_style("·", "dim");
    s.assert_text_style("esc", "bold !dim");
    s.assert_text_style("cancel", "dim");
}

// ORACLE bottom_pane/chat_composer/history_search.rs:976 history_search_highlights_matches_until_accepted kind=ui owner=chat-nav-pane
#[test]
fn history_search_highlights_matches_until_accepted() {
    kit::pending(Owner::ChatNavPane, MATCH_HIGHLIGHT);
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(60, 8)
        .script(include_str!("fixtures/composer_popups/h_git_cargo.jsonl"))
        .open();
    s.keys("ctrl+r");
    s.type_text("git");
    s.assert_row(prompt(&s), "› git status");
    // The matched `git` is reversed and bold, the rest of the entry is not.
    for x in 2..5 {
        s.assert_style(x, prompt(&s), "reversed bold");
    }
    s.assert_style(5, prompt(&s), "!reversed");
    s.keys("enter");
    s.assert_row(prompt(&s), "› git status");
    // Accepted: the highlight is gone.
    for x in 2..5 {
        s.assert_style(x, prompt(&s), "!reversed !bold");
    }
}

// ORACLE bottom_pane/chat_composer/history_search.rs:1046 history_search_esc_restores_original_draft kind=ui owner=chat-nav-pane
#[test]
fn history_search_esc_restores_original_draft() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .script(include_str!("fixtures/composer_popups/h_remembered.jsonl"))
        .open();
    s.type_text("draft").keys(&times("left", 3)).keys("ctrl+r");
    s.assert_row(prompt(&s), "› draft");
    s.type_text("r");
    s.assert_row(prompt(&s), "› remembered command");
    s.keys("esc");
    s.assert_absent("reverse-i-search");
    s.assert_row(prompt(&s), "› draft");
    // The cursor is back at offset 2 of the draft, after the `› ` prompt.
    s.assert_cursor(4, prompt(&s));
}

// ORACLE bottom_pane/chat_composer/history_search.rs:1074 history_search_ctrl_c_restores_original_draft kind=ui owner=chat-nav-pane
#[test]
fn history_search_ctrl_c_restores_original_draft() {
    let mut s = remembered_search_preview();
    s.keys("ctrl+c");
    s.assert_absent("reverse-i-search");
    s.assert_row(prompt(&s), "› draft");
    s.assert_cursor(4, prompt(&s));

    let mut s = remembered_search_preview();
    s.type_text("\u{3}");
    s.assert_absent("reverse-i-search");
    s.assert_row(prompt(&s), "› draft");
    s.assert_cursor(4, prompt(&s));
}

// ORACLE bottom_pane/chat_composer/history_search.rs:1114 history_search_flushes_pending_first_char_before_snapshot kind=ui owner=chat-nav-pane
#[test]
fn history_search_flushes_pending_first_char_before_snapshot() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .script(include_str!("fixtures/composer_popups/h_empty.jsonl"))
        .open();
    s.keys("h ctrl+r");
    s.assert_row(prompt(&s), "› h");
    s.keys("esc");
    s.assert_row(prompt(&s), "› h");
    // The oracle also asserts the paste burst is flushed: the burst is not in the rows.
}

// ORACLE bottom_pane/chat_composer/history_search.rs:1142 history_search_flushes_buffered_paste_before_snapshot kind=ui owner=chat-nav-pane
#[test]
fn history_search_flushes_buffered_paste_before_snapshot() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .script(include_str!("fixtures/composer_popups/h_empty.jsonl"))
        .open();
    s.type_text("paste").keys("ctrl+r");
    s.assert_row(prompt(&s), "› paste");
    s.keys("esc");
    s.assert_row(prompt(&s), "› paste");
    // The oracle also asserts the paste burst is flushed: the burst is not in the rows.
}

// ORACLE bottom_pane/chat_composer/history_search.rs:1180 history_search_esc_resets_normal_history_navigation kind=ui owner=chat-nav-pane
#[test]
fn history_search_esc_resets_normal_history_navigation() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .script(include_str!("fixtures/composer_popups/h_reset.jsonl"))
        .open();
    s.keys("ctrl+r");
    s.type_text("match");
    s.assert_row(prompt(&s), "› oldest matching entry");
    s.keys("esc");
    s.assert_absent("oldest matching entry");
    s.assert_absent("newest entry");
    s.keys("up");
    s.assert_row(prompt(&s), "› newest entry");
}

// ORACLE bottom_pane/chat_composer/history_search.rs:1213 history_search_no_match_restores_preview_but_keeps_search_open kind=ui owner=chat-nav-pane
#[test]
fn history_search_no_match_restores_preview_but_keeps_search_open() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .script(include_str!("fixtures/composer_popups/h_git_status.jsonl"))
        .open();
    s.type_text("draft").keys("ctrl+r");
    s.type_text("zzz");
    s.assert_row(prompt(&s), "› draft");
    s.assert_contains("reverse-i-search:");
    s.assert_contains("no match");
}

// ---- oracle: bottom_pane/chat_composer/history_search_paste_tests.rs (6 screen-level tests here; 0 state tests left to chat-nav-pane)
// ORACLE bottom_pane/chat_composer/history_search_paste_tests.rs:46 history_search_paste_appends_query_and_accepts_match kind=snap owner=chat-nav-pane snap=codex_tui__bottom_pane__chat_composer__history_search__paste_tests__history_search_pasted_query
#[test]
fn history_search_paste_appends_query_and_accepts_match() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(70, 5)
        .entry("composer", "alone_history")
        .open();
    s.type_text("draft").keys("ctrl+r");
    s.paste("git");
    s.assert_row(prompt(&s), "› git log");
    s.type_text(" ");
    s.paste("status");
    s.assert_row(prompt(&s), "› git status");
    s.assert_snap("codex_tui__bottom_pane__chat_composer__history_search__paste_tests__history_search_pasted_query");
    s.keys("enter");
    s.assert_row(prompt(&s), "› git status");
    s.assert_absent("reverse-i-search");
    // The oracle's background lookup (apply_history_search_result, Found "git diff") is not injectable here.
}

// ORACLE bottom_pane/chat_composer/history_search_paste_tests.rs:83 history_search_large_paste_clamps_cursor kind=snap owner=chat-nav-pane snap=codex_tui__bottom_pane__chat_composer__history_search__paste_tests__history_search_large_paste_cursor
#[test]
fn history_search_large_paste_clamps_cursor() {
    kit::pending(
        Owner::ChatNavPane,
        "pending(G78): the oracle's caret stands at the end of the query on the footer row; the terminal caret of ours stays in the composer's text (the rows match, the cursor does not)",
    );
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 5)
        .entry("composer", "alone_history")
        .open();
    s.type_text("draft").keys("ctrl+r");
    s.paste(&"x".repeat(65536));
    s.assert_snap_region("codex_tui__bottom_pane__chat_composer__history_search__paste_tests__history_search_large_paste_cursor", 0, 0);
}

// ORACLE bottom_pane/chat_composer/history_search_paste_tests.rs:101 history_search_empty_paste_preserves_selected_match kind=snap owner=chat-nav-pane snap=codex_tui__bottom_pane__chat_composer__history_search__paste_tests__history_search_empty_paste
#[test]
fn history_search_empty_paste_preserves_selected_match() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(70, 5)
        .entry("composer", "alone_history")
        .open();
    s.type_text("draft").keys("ctrl+r");
    s.paste("git");
    s.assert_row(prompt(&s), "› git log");
    s.keys("ctrl+r");
    s.assert_row(prompt(&s), "› git status");
    s.paste("");
    s.paste("\u{1b}[31m\u{1b}[0m");
    s.assert_snap("codex_tui__bottom_pane__chat_composer__history_search__paste_tests__history_search_empty_paste");
}

// ORACLE bottom_pane/chat_composer/history_search_paste_tests.rs:121 history_search_paste_shows_separators_and_matches_original_query kind=snap owner=chat-nav-pane snap=codex_tui__bottom_pane__chat_composer__history_search__paste_tests__history_search_pasted_separators
#[test]
fn history_search_paste_shows_separators_and_matches_original_query() {
    kit::pending(
        Owner::ChatNavPane,
        "pending(G78): the oracle's caret stands at the end of the query on the footer row; the terminal caret of ours stays in the composer's text (the rows match, the cursor does not)",
    );
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(70, 5)
        .entry("composer", "alone_history_separators")
        .open();
    s.type_text("draft").keys("ctrl+r");
    s.paste("foo\nbar\tbaz");
    s.assert_contains("reverse-i-search: foo↵bar⇥baz  no match");
    s.assert_snap_region("codex_tui__bottom_pane__chat_composer__history_search__paste_tests__history_search_pasted_separators", 0, 0);
}

// ORACLE bottom_pane/chat_composer/history_search_paste_tests.rs:158 history_search_paste_preserves_original_draft_on_miss_and_cancel kind=ui owner=chat-nav-pane
#[test]
fn history_search_paste_preserves_original_draft_on_miss_and_cancel() {
    for cancel in ["esc", "ctrl+c"] {
        let mut s = large_draft_search();
        s.keys(cancel);
        s.assert_absent("reverse-i-search");
        s.assert_row(prompt(&s), "› draft[Pasted Content 1001 chars]");
        // The cursor is restored to offset 2 of the draft.
        s.assert_cursor(4, prompt(&s));
    }
}

// ORACLE bottom_pane/chat_composer/history_search_paste_tests.rs:186 history_search_paste_uses_full_sanitized_text kind=ui owner=chat-nav-pane
#[test]
fn history_search_paste_uses_full_sanitized_text() {
    kit::pending(
        Owner::ChatNavPane,
        "screen bug (history search row): the sanitized text and the image path cases pass (the kit writes the png, `image_file`); a large paste (1001 chars) as the query overflows the search row by 2 columns (RenderFlex, the `+2` counter beside the wrapped query), which the app logs as an error",
    );
    // the pasted text is the query, sanitized (CR LF and escape codes), in place of a placeholder
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .script(include_str!("fixtures/composer_popups/h_sanitized.jsonl"))
        .open();
    s.type_text("draft").keys("ctrl+r");
    s.paste("é\r\n中\r\u{1b}[31mtext\u{1b}[0m");
    s.assert_contains("reverse-i-search: é↵中↵text");
    // an image path pasted into the search is its text, no attachment
    let path = kit::image_file("history.png");
    let path = path.to_string_lossy().into_owned();
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(200, 24)
        .script(include_str!("fixtures/composer_popups/h_sanitized.jsonl"))
        .open();
    s.type_text("draft").keys("ctrl+r");
    s.paste(&path);
    s.assert_contains(&format!("reverse-i-search: {path}"));
    s.assert_absent("[Image #1]");
    // a large paste is its text, no `[Pasted Content N chars]` placeholder
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .script(include_str!("fixtures/composer_popups/h_sanitized.jsonl"))
        .open();
    s.type_text("draft").keys("ctrl+r");
    s.paste(&"x".repeat(1001));
    s.assert_contains("reverse-i-search:");
    s.assert_contains(&"x".repeat(70));
    s.assert_absent("[Pasted Content");
}

// ---- oracle: bottom_pane/chat_composer/mentions_layout_tests.rs (3 screen-level tests here; 0 state tests left to chat-nav-pane)
// ORACLE bottom_pane/chat_composer/mentions_layout_tests.rs:86 mention_scopes_overlay_history_without_moving_composer_or_footer kind=snap owner=chat-nav-pane snap=codex_tui__bottom_pane__chat_composer__mentions_layout_tests__mention_scopes_overlay_history
#[test]
fn mention_scopes_overlay_history_without_moving_composer_or_footer() {
    kit::pending(
        Owner::ChatNavPane,
        "the transcript rows behind the menu ('Transcript row NN') and its footer (TRANSCRIPT STATUS) are not seeded by any entry",
    );
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 32)
        .entry("popovers", "composer_popups")
        .open();
    s.type_text("@");
    s.assert_snap("codex_tui__bottom_pane__chat_composer__mentions_layout_tests__mention_scopes_overlay_history");
    s.keys("right");
}

// ORACLE bottom_pane/chat_composer/mentions_layout_tests.rs:154 mention_menu_reserves_height_above_composer_and_owns_cursor_over_interactive_footer kind=snap owner=chat-nav-pane snap=codex_tui__bottom_pane__chat_composer__mentions_layout_tests__mention_menu_above_composer
#[test]
fn mention_menu_reserves_height_above_composer_and_owns_cursor_over_interactive_footer() {
    kit::pending(
        Owner::ChatNavPane,
        "the chat status line (MODEL STATUS) and the interactive transcript footer (Find: needle) are not seeded by any entry",
    );
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 21)
        .entry("popovers", "composer_popups")
        .open();
    s.type_text("@");
    s.assert_snap(
        "codex_tui__bottom_pane__chat_composer__mentions_layout_tests__mention_menu_above_composer",
    );
}

// ORACLE bottom_pane/chat_composer/mentions_layout_tests.rs:191 fallback_completions_keep_the_draft_cursor_and_footer_anchored kind=ui owner=chat-nav-pane
#[test]
fn fallback_completions_keep_the_draft_cursor_and_footer_anchored() {
    kit::pending(
        Owner::ChatNavPane,
        "the transcript footer (TRANSCRIPT STATUS) under the composer is not seeded, and the oracle sets the popups on the composer directly",
    );
    for (width, height) in [(80, 24), (40, 16), (20, 5)] {
        for (token, item) in [("$", "Example skill"), ("@", "src/example.rs")] {
            let mut s = Screen::build(Owner::ChatNavPane)
                .size(width, height)
                .script(include_str!(
                    "fixtures/composer_popups/skills_example.jsonl"
                ))
                .open();
            s.type_text(token);
            if height > 5 {
                s.assert_contains(item);
            }
        }
    }
}

// ---- oracle: bottom_pane/chat_composer/skill_completion_tests.rs (5 screen-level tests here; 0 state tests left to chat-nav-pane)
// ORACLE bottom_pane/chat_composer/skill_completion_tests.rs:12 skill_completion_attaches_typed_punctuation_snapshot kind=snap owner=chat-nav-pane snap=skill_completion_attaches_typed_punctuation
#[test]
fn skill_completion_attaches_typed_punctuation_snapshot() {
    kit::pending(
        Owner::ChatNavPane,
        "footer modes (pane-footer): the row `› 请用 $rustdoc》` matches; the oracle's footer of a non-empty draft is `100% context left` at the right, ours the page's key hints",
    );
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(100, 9)
        .script(include_str!(
            "fixtures/composer_popups/skills_rustdoc.jsonl"
        ))
        .open();
    s.type_text("请用 $ru").keys("enter");
    s.type_text("》");
    s.assert_row(prompt(&s), "› 请用 $rustdoc》");
    s.assert_snap("skill_completion_attaches_typed_punctuation");
}

// ORACLE bottom_pane/chat_composer/skill_completion_tests.rs:43 unified_skill_completion_keeps_draft_spacing_after_punctuation kind=ui owner=chat-nav-pane
#[test]
fn unified_skill_completion_keeps_draft_spacing_after_punctuation() {
    let cases = [
        ("", ",", "$rustdoc,"),
        ("", "!", "$rustdoc!"),
        (" then continue", "!", "$rustdoc! then continue"),
        (" ", "!", "$rustdoc !"),
        ("", "。", "$rustdoc。"),
        ("", "，", "$rustdoc，"),
        ("", "、", "$rustdoc、"),
        ("", "！", "$rustdoc！"),
        ("", "？", "$rustdoc？"),
        ("", "》", "$rustdoc》"),
        ("", "/", "$rustdoc /"),
        ("", "\\", "$rustdoc \\"),
    ];
    for (suffix, typed, expected) in cases {
        let mut s = Screen::build(Owner::ChatNavPane)
            .size(80, 24)
            .script(include_str!(
                "fixtures/composer_popups/skills_rustdoc.jsonl"
            ))
            .open();
        s.type_text(&format!("@ru{suffix}"));
        if !suffix.is_empty() {
            s.keys(&times("left", suffix.chars().count()));
        }
        s.keys("tab");
        s.type_text(typed);
        s.assert_row(prompt(&s), &format!("› {expected}"));
    }
}

// ORACLE bottom_pane/chat_composer/skill_completion_tests.rs:84 skill_completion_keeps_space_after_cursor_navigation kind=ui owner=chat-nav-pane
#[test]
fn skill_completion_keeps_space_after_cursor_navigation() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .script(include_str!(
            "fixtures/composer_popups/skills_rustdoc.jsonl"
        ))
        .open();
    s.type_text("$ru").keys("tab left right");
    s.type_text(",");
    s.assert_row(prompt(&s), "› $rustdoc ,");
}

// ORACLE bottom_pane/chat_composer/skill_completion_tests.rs:102 skill_completion_keeps_space_before_a_punctuation_paste_burst kind=ui owner=chat-nav-pane
#[test]
fn skill_completion_keeps_space_before_a_punctuation_paste_burst() {
    for (typed, expected) in [(",", "› $rustdoc ,x"), ("。", "› $rustdoc 。x")] {
        let mut s = Screen::build(Owner::ChatNavPane)
            .size(80, 24)
            .script(include_str!(
                "fixtures/composer_popups/skills_rustdoc.jsonl"
            ))
            .open();
        s.type_text("$ru").keys("tab");
        // the punctuation and the next key come as a burst
        s.type_burst(&format!("{typed}x"));
        s.keys("enter");
        s.wait(100);
        let row = prompt(&s);
        s.assert_row(row, expected);
        s.assert_row(row + 1, "");
    }
}

// ORACLE bottom_pane/chat_composer/skill_completion_tests.rs:124 default_unified_mention_popup_snapshot kind=snap owner=chat-nav-pane snap=default_unified_mention_popup
#[test]
fn default_unified_mention_popup_snapshot() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(100, 9)
        .entry("popovers", "alone_skills_default")
        .open();
    s.type_text("@sa");
    s.assert_snap("codex_tui__bottom_pane__chat_composer__tests__default_unified_mention_popup");
}

// ---- oracle: bottom_pane/command_popup.rs (5 screen-level tests here; 11 state tests left to chat-nav-pane)
// ORACLE bottom_pane/command_popup.rs:337 unavailable_side_conversation_command_is_shown_only_when_searched kind=snap owner=chat-nav-pane snap=side_conversation_unavailable_command
#[test]
fn unavailable_side_conversation_command_is_shown_only_when_searched() {
    kit::pending(
        Owner::ChatNavPane,
        "the side-conversation flag has no seed in the port",
    );
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(76, 24)
        .entry("popovers", "composer_popups")
        .open();
    s.type_text("/");
    s.assert_absent("/archive");
    s.type_text("arch");
    s.assert_contains("/archive (disabled)");
    s.assert_snap_region(
        "codex_tui__bottom_pane__command_popup__tests__side_conversation_unavailable_command",
        row_starting(&s, "  /archive"),
        0,
    );
}

// ORACLE bottom_pane/command_popup.rs:459 command_popup_wrap_boundary_preserves_following_choice kind=snap owner=chat-nav-pane snap=command_popup_wrap_boundary
#[test]
fn command_popup_wrap_boundary_preserves_following_choice() {
    kit::pending(Owner::ChatNavPane, POPUP_ROWS);
    for width in [32, 33, 34] {
        let mut s = Screen::build(Owner::ChatNavPane)
            .size(width, 24)
            .entry("popovers", "composer_popups")
            .open();
        s.type_text("/tier");
        s.assert_contains("tier-two");
        // The oracle's frame of this width: the popup alone, from its first row.
        s.snap("codex_tui__bottom_pane__command_popup__tests__command_popup_wrap_boundary")
            .frame(&format!("width {width}"))
            .at(row_starting(&s, "› /tier"), 0)
            .assert();
    }
}

// ORACLE bottom_pane/command_popup.rs:529 app_command_popup_snapshot kind=snap owner=chat-nav-pane snap=command_popup_app
#[test]
fn app_command_popup_snapshot() {
    kit::pending(Owner::ChatNavPane, POPUP_ROWS);
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(72, 24)
        .entry("popovers", "composer_popups")
        .open();
    s.type_text("/app");
    s.assert_snap_region(
        "codex_tui__bottom_pane__command_popup__tests__command_popup_app",
        row_starting(&s, "› /app"),
        0,
    );
}

// ORACLE bottom_pane/command_popup.rs:548 voice_command_popup_snapshot kind=snap owner=chat-nav-pane snap=command_popup_voice
#[test]
fn voice_command_popup_snapshot() {
    kit::pending(Owner::ChatNavPane, POPUP_ROWS);
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(72, 24)
        .entry("popovers", "composer_popups")
        .open();
    s.type_text("/voice");
    s.assert_snap_region(
        "codex_tui__bottom_pane__command_popup__tests__command_popup_voice",
        row_starting(&s, "› /voice"),
        0,
    );
}

// ORACLE bottom_pane/command_popup.rs:591 changing_filter_resets_selection_after_scrolling kind=snap owner=chat-nav-pane snap=command_popup_filter_reset_after_scroll
#[test]
fn changing_filter_resets_selection_after_scrolling() {
    kit::pending(Owner::ChatNavPane, POPUP_ROWS);
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(72, 24)
        .entry("popovers", "composer_popups")
        .open();
    s.type_text("/");
    s.keys(&times("down", 8));
    s.type_text("st");
    s.assert_snap_region(
        "codex_tui__bottom_pane__command_popup__tests__command_popup_filter_reset_after_scroll",
        row_starting(&s, "› /status"),
        0,
    );
}

// ---- oracle: bottom_pane/file_search_popup.rs (1 screen-level tests here; 1 state tests left to chat-nav-pane)
// ORACLE bottom_pane/file_search_popup.rs:209 long_paths_keep_distinguishing_filenames_and_original_selection kind=snap owner=chat-nav-pane snap=long_paths_keep_distinguishing_filenames_and_original_selection
#[test]
fn long_paths_keep_distinguishing_filenames_and_original_selection() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .entry("popovers", "composer_popups")
        .open();
    s.type_text("@parser");
    s.assert_contains("parser_alpha.rs");
    s.assert_contains("parser_beta.rs");
}

// ---- oracle: bottom_pane/mentions_v2/render_tests.rs (4 screen-level tests here; 0 state tests left to chat-nav-pane)
// ORACLE bottom_pane/mentions_v2/render_tests.rs:50 live_picker_frame_snapshots kind=snap owner=chat-nav-pane snap=narrow_scrolled snap=one_row snap=short snap=wide
#[test]
fn live_picker_frame_snapshots() {
    kit::pending(
        Owner::ChatNavPane,
        "the 7-row and 1-row frames are the popup alone at that height: a chat screen sizes its popup and always holds the composer",
    );
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .script(include_str!(
            "fixtures/composer_popups/mentions_plugins.jsonl"
        ))
        .open();
    s.type_text("@");
    s.assert_snap_region(
        "codex_tui__bottom_pane__mentions_v2__render__tests__wide",
        row_starting(&s, "  Mentions"),
        0,
    );
    s.resize(40, 24);
    s.assert_snap_region(
        "codex_tui__bottom_pane__mentions_v2__render__tests__narrow_scrolled",
        row_starting(&s, "  Mentions"),
        0,
    );
}

// ORACLE bottom_pane/mentions_v2/render_tests.rs:76 selection_covers_the_row_and_empty_filter_clears_previous_results kind=snap owner=chat-nav-pane snap=empty_filter
#[test]
fn selection_covers_the_row_and_empty_filter_clears_previous_results() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .script(include_str!(
            "fixtures/composer_popups/mentions_plugins.jsonl"
        ))
        .open();
    s.type_text("@");
    // the filesystem tab with a new query: the earlier results are gone, the
    // list says it is loading
    s.keys("right");
    s.type_text("new-query");
    s.assert_snap_region(
        "codex_tui__bottom_pane__mentions_v2__render__tests__empty_filter",
        row_starting(&s, "  Mentions"),
        0,
    );
}

// ORACLE bottom_pane/mentions_v2/render_tests.rs:135 filesystem_columns_use_terminal_cell_widths kind=ui owner=chat-nav-pane
#[test]
fn filesystem_columns_use_terminal_cell_widths() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(60, 24)
        .script(include_str!(
            "fixtures/composer_popups/mentions_files.jsonl"
        ))
        .open();
    s.type_text("@src/");
    s.assert_contains("src/");
    // The parent path of each file row starts at column 12, whatever the name's glyph widths.
    s.assert_rows_ending_have_text_at("File", 12, "src/", 2);
}

// ORACLE bottom_pane/mentions_v2/render_tests.rs:166 long_mentions_leave_room_for_distinguishing_parent_paths kind=snap owner=chat-nav-pane snap=bounded_filesystem_columns_narrow snap=bounded_filesystem_columns_wide
#[test]
fn long_mentions_leave_room_for_distinguishing_parent_paths() {
    kit::pending(
        Owner::ChatNavPane,
        "the snapshots are the popup alone at three rows: a chat screen sizes its popup itself, so the 3-row frame is not a screen state",
    );
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .script(include_str!("fixtures/composer_popups/mentions_long.jsonl"))
        .open();
    s.type_text("@");
    s.assert_snap_region(
        "codex_tui__bottom_pane__mentions_v2__render__tests__bounded_filesystem_columns_wide",
        row_starting(&s, "› 界.rs"),
        0,
    );
}

// ---- oracle: bottom_pane/skill_popup.rs (3 screen-level tests here; 3 state tests left to chat-nav-pane)
// ORACLE bottom_pane/skill_popup.rs:350 scrolling_mentions_shifts_rendered_window_snapshot kind=snap owner=chat-nav-pane snap=skill_popup_scrolled
#[test]
fn scrolling_mentions_shifts_rendered_window_snapshot() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(72, 24)
        .entry("popovers", "alone_mentions_ten")
        .open();
    s.type_text("$");
    // the oracle's popup starts with no selection, so its nine moves end on the
    // ninth row; ours starts on the first row
    s.keys(&times("down", 8));
    let top = row_starting(&s, "↑");
    s.assert_snap_rows_at("codex_tui__bottom_pane__skill_popup__tests__skill_popup_scrolled", top);
    // the cursor row is bold and reversed, the tag and the description dim
    s.assert_style(0, top + 8, "bold reversed");
    s.assert_style(14, top + 8, "reversed !bold");
    s.assert_style(14, top + 1, "dim");
    s.assert_style(36, top + 8, "bold reversed");
}

// ORACLE bottom_pane/skill_popup.rs:361 category_tags_distinguish_mentions_when_descriptions_hide kind=snap owner=chat-nav-pane snap=skill_popup_category_tags
#[test]
fn category_tags_distinguish_mentions_when_descriptions_hide() {
    for (name, width) in [("Shared", 32), ("Shared", 72)] {
        let mut s = Screen::build(Owner::ChatNavPane)
            .size(width, 24)
            .entry("popovers", "composer_popups")
            .open();
        s.type_text(&format!("${name}"));
        s.assert_contains("[App]");
        s.assert_contains("[Skill]");
        // the description hides where the row is narrow, the tags stay
        if width == 72 {
            s.assert_contains("Secondary details");
        } else {
            s.assert_absent("Secondary details");
        }
    }
}

// ORACLE bottom_pane/skill_popup.rs:387 lowercase_expansion_preserves_match_ranking_and_highlighting kind=snap owner=chat-nav-pane snap=skill_popup_lowercase_expansion
#[test]
fn lowercase_expansion_preserves_match_ranking_and_highlighting() {
    kit::pending(
        Owner::ChatNavPane,
        "the query U+0307 x is not a mention name: typed after `$` the composer closes the popup (the combining dot joins the `$`), so the oracle's `SkillPopup::set_query` state has no screen",
    );
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .entry("popovers", "composer_popups")
        .open();
    s.type_text("$\u{307}x");
    s.assert_snap_region(
        "codex_tui__bottom_pane__skill_popup__tests__skill_popup_lowercase_expansion",
        row_starting(&s, "› "),
        0,
    );
}
