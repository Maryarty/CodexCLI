//! Shard `composer_3` (screens S10, owner chat-live): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen};

/// The composer's prompt row (`›`): the last one, since a popup sits above the composer.
fn prompt(s: &Screen) -> usize {
    s.rows()
        .iter()
        .rposition(|r| r.starts_with('›') || r.starts_with("! ") || r == "!")
        .unwrap_or(usize::MAX)
}

/// `key` pressed `n` times, as one key string.
fn times(key: &str, n: usize) -> String {
    vec![key; n].join(" ")
}

// ---- oracle: bottom_pane/chat_composer.rs (50 screen-level tests here; 22 state tests left to chat-live)
// ORACLE bottom_pane/chat_composer.rs:10375 file_completion_preserves_large_paste_placeholder_elements kind=ui owner=chat-live
#[test]
fn file_completion_preserves_large_paste_placeholder_elements() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(100, 24)
        .script(include_str!("fixtures/composer_3/file_ma_main.jsonl"))
        .open();
    // oracle: a paste over the 1000-char threshold, ' @ma', then the file result 'src/main.rs' for 'ma' and Tab
    s.paste(&"x".repeat(1005));
    s.type_text(" @ma");
    s.keys("tab");
    s.assert_row(prompt(&s), "› [Pasted Content 1005 chars] src/main.rs ");
    s.keys("enter");
    s.assert_absent("[Pasted Content");
    s.assert_contains("src/main.rs");
}

// ORACLE bottom_pane/chat_composer.rs:10494 adjacent_image_completion_inserts_leading_separator kind=ui owner=chat-live
#[test]
fn adjacent_image_completion_inserts_leading_separator() {
    kit::pending(
        Owner::ChatLive,
        "no image attachment and no file completion on the chat composer, and no vim normal mode: Tab on a png path inserts '[Image #1]' after a leading space; Esc then u restores the draft",
    );
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    // oracle: '@sample@other' with the cursor between the two mentions; the completed file is a png
    s.type_text("@sample@other");
    s.keys("tab");
    s.assert_row(prompt(&s), "› @sample [Image #1] ");
    // oracle: Esc, then 'u' in vim normal mode restores the draft before the completion
    s.keys("esc u");
    s.assert_row(prompt(&s), "› @sample@other");
}

// ORACLE bottom_pane/chat_composer.rs:10560 file_completion_for_sigil_path_does_not_reopen_popup kind=ui owner=chat-live
#[test]
fn file_completion_for_sigil_path_does_not_reopen_popup() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(100, 24)
        .script(include_str!("fixtures/composer_3/file_ma_sigil.jsonl"))
        .open();
    // oracle: draft '@ma' + newline + 'next', cursor after '@ma', file result '@scope/main.rs' for 'ma', Tab
    s.paste("@ma\nnext");
    s.keys(&times("left", 5));
    s.keys("tab");
    s.assert_row(prompt(&s), "› @scope/main.rs ");
    s.assert_row(prompt(&s) + 1, "  next");
    s.keys("enter");
    s.assert_in_order(&["@scope/main.rs", "next"]);
}

// ORACLE bottom_pane/chat_composer.rs:10599 dismissed_file_popup_tracks_token_across_leading_whitespace_edits kind=ui owner=chat-live
#[test]
fn dismissed_file_popup_tracks_token_across_leading_whitespace_edits() {
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    s.type_text("@ma");
    s.keys("esc esc");
    s.assert_row(prompt(&s), "› @ma");
    // oracle: the cursor to 0, '   ' inserted, then three backspaces: no popup comes back
    s.keys("home");
    s.type_text("   ");
    s.assert_row(prompt(&s), "›    @ma");
    s.keys(&times("backspace", 3));
    s.assert_row(prompt(&s), "› @ma");
}

// ORACLE bottom_pane/chat_composer.rs:10622 dismissed_file_popup_ignores_token_substrings_in_leading_paste kind=ui owner=chat-live
#[test]
fn dismissed_file_popup_ignores_token_substrings_in_leading_paste() {
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    s.type_text("@ma");
    s.keys("esc esc");
    // oracle: the cursor to 0, then the paste 'email@ma.com ' at the start
    s.keys("home");
    s.paste("email@ma.com ");
    s.assert_row(prompt(&s), "› email@ma.com @ma");
}

// ORACLE bottom_pane/chat_composer.rs:10642 test_multiple_pastes_submission kind=ui owner=chat-live
#[test]
fn test_multiple_pastes_submission() {
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    s.paste(&"x".repeat(1003));
    s.paste(" and ");
    s.paste(&"y".repeat(1007));
    s.assert_row(
        prompt(&s),
        "› [Pasted Content 1003 chars] and [Pasted Content 1007 chars]",
    );
    s.keys("enter");
    s.assert_absent("[Pasted Content");
}

// ORACLE bottom_pane/chat_composer.rs:10721 test_placeholder_deletion kind=ui owner=chat-live
#[test]
fn test_placeholder_deletion() {
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    s.paste(&"a".repeat(1005));
    s.paste(" and ");
    s.paste(&"b".repeat(1006));
    s.assert_row(
        prompt(&s),
        "› [Pasted Content 1005 chars] and [Pasted Content 1006 chars]",
    );
    // oracle: the cursor right after the first placeholder, Backspace. Left
    // steps over a placeholder whole (`prev_atomic_boundary`): one Left
    // over the second, five over " and ".
    s.keys(&times("left", 6));
    s.keys("backspace");
    s.assert_row(prompt(&s), "›  and [Pasted Content 1006 chars]");
    // oracle: the cursor to the end, Backspace removes the second placeholder
    s.keys("end backspace");
    s.assert_row(prompt(&s), "›  and ");
}

// ORACLE bottom_pane/chat_composer.rs:10798 deleting_duplicate_length_pastes_removes_only_target kind=ui owner=chat-live
#[test]
fn deleting_duplicate_length_pastes_removes_only_target() {
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    s.paste(&"x".repeat(1004));
    s.paste(&"x".repeat(1004));
    s.assert_row(
        prompt(&s),
        "› [Pasted Content 1004 chars][Pasted Content 1004 chars] #2",
    );
    s.keys("end backspace");
    s.assert_row(prompt(&s), "› [Pasted Content 1004 chars]");
}

// ORACLE bottom_pane/chat_composer.rs:10840 large_paste_numbering_continues_with_same_length_placeholder kind=ui owner=chat-live
#[test]
fn large_paste_numbering_continues_with_same_length_placeholder() {
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    s.paste(&"x".repeat(1004));
    s.paste(&"x".repeat(1004));
    // oracle: the cursor after the first placeholder, Backspace removes it;
    // one Left steps over the second placeholder whole
    s.keys("left");
    s.keys("backspace");
    s.assert_row(prompt(&s), "› [Pasted Content 1004 chars] #2");
    s.keys("end");
    s.paste(&"x".repeat(1004));
    s.assert_row(
        prompt(&s),
        "› [Pasted Content 1004 chars] #2[Pasted Content 1004 chars] #3",
    );
}

// ORACLE bottom_pane/chat_composer.rs:10885 large_paste_numbering_reuses_after_all_deleted kind=ui owner=chat-live
#[test]
fn large_paste_numbering_reuses_after_all_deleted() {
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    s.paste(&"x".repeat(1004));
    s.assert_row(prompt(&s), "› [Pasted Content 1004 chars]");
    s.keys("end backspace");
    s.assert_row(prompt(&s), "› Ask Codex to do anything");
    s.paste(&"x".repeat(1004));
    s.assert_row(prompt(&s), "› [Pasted Content 1004 chars]");
}

// ORACLE bottom_pane/chat_composer.rs:10922 test_partial_placeholder_deletion kind=ui owner=chat-live
#[test]
fn test_partial_placeholder_deletion() {
    // oracle case 1: the cursor 5 chars before the placeholder's end, Backspace
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    s.paste(&"x".repeat(1004));
    // the oracle's `set_cursor` inside the placeholder snaps to its nearer
    // end; a click there is the screen's way to put the caret inside it
    let label = "[Pasted Content 1004 chars]";
    let row = prompt(&s);
    s.assert_row(row, "› [Pasted Content 1004 chars]");
    s.click((2 + label.len() - 5) as u16, row as u16);
    s.keys("backspace");
    s.assert_row(prompt(&s), "› Ask Codex to do anything");
    // oracle case 2: the cursor at the end, Backspace
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    s.paste(&"x".repeat(1004));
    s.assert_row(prompt(&s), "› [Pasted Content 1004 chars]");
    s.keys("backspace");
    s.assert_row(prompt(&s), "› Ask Codex to do anything");
}

// ORACLE bottom_pane/chat_composer.rs:10975 attach_image_and_submit_includes_local_image_paths kind=ui owner=chat-live
#[test]
fn attach_image_and_submit_includes_local_image_paths() {
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    // oracle: attach_image('/tmp/image1.png'), then ' hi'
    s.attach_image("image1.png");
    s.paste(" hi");
    s.assert_row(prompt(&s), "› [Image #1] hi");
    // oracle: Enter submits '[Image #1] hi' with one image element at 0..10
    s.keys("enter");
    s.assert_contains("[Image #1] hi");
}

// ORACLE bottom_pane/chat_composer.rs:11013 submit_captures_recent_mention_bindings_before_clearing_textarea kind=ui owner=chat-live
#[test]
fn submit_captures_recent_mention_bindings_before_clearing_textarea() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(100, 24)
        .script(include_str!("fixtures/composer_3/skill_figma.jsonl"))
        .open();
    // oracle: the draft `$figma please` with `$figma` bound to the skill file; Enter submits
    s.type_text("$figma").keys("tab");
    s.type_text("please").keys("enter");
    // ... and the composer is cleared
    s.assert_row(prompt(&s), "› Ask Codex to do anything");
    s.assert_contains("$figma please");
    // oracle: the submission's bindings are the one `$figma`
    s.assert_sole_turn(kit::Turn::any().skills(&[("figma", "/tmp/user/figma/SKILL.md")]));
}

// ORACLE bottom_pane/chat_composer.rs:11047 history_navigation_restores_remote_and_local_image_attachments kind=ui owner=chat-live
#[test]
fn history_navigation_restores_remote_and_local_image_attachments() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(100, 24)
        .live()
        .remote_images(&["https://example.com/remote.png"])
        .open();
    // oracle: one remote image URL, a local image attached, Enter, then Up in an empty draft
    let path = s.attach_image("image1.png");
    s.assert_row(prompt(&s), "› [Image #2]");
    s.keys("enter");
    s.wait_for("› Ask Codex to do anything");
    // the turn carries the remote image and the local one numbered after it
    s.assert_sole_turn(
        kit::Turn::any()
            .remote_images(&["https://example.com/remote.png"])
            .image_paths(&[path.as_path()]),
    );
    s.keys("up");
    s.assert_row(prompt(&s), "› [Image #2]");
    s.assert_contains("[Image #1]");
}

// ORACLE bottom_pane/chat_composer.rs:11081 history_navigation_restores_remote_only_submissions kind=ui owner=chat-live
#[test]
fn history_navigation_restores_remote_only_submissions() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(100, 24)
        .live()
        .remote_images(&["https://example.com/one.png", "https://example.com/two.png"])
        .open();
    // oracle: two remote image URLs and no text; Enter records the empty submission
    s.keys("enter");
    s.wait_for("› Ask Codex to do anything");
    s.assert_sole_turn(
        kit::Turn::text("")
            .remote_images(&["https://example.com/one.png", "https://example.com/two.png"]),
    );
    // oracle: Up restores it: the text is empty and the remote images come back
    s.keys("up");
    s.assert_row(prompt(&s), "› Ask Codex to do anything");
    s.assert_contains("[Image #1]");
    s.assert_contains("[Image #2]");
}

// ORACLE bottom_pane/chat_composer.rs:11113 history_navigation_leaves_cursor_at_end_of_line kind=ui owner=chat-live
#[test]
fn history_navigation_leaves_cursor_at_end_of_line() {
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    s.type_text("first").keys("enter");
    s.type_text("second").keys("enter");
    s.keys("up");
    s.assert_row(prompt(&s), "› second");
    // oracle: the cursor at the end of the recalled text (its length in columns after the '› ')
    s.assert_cursor(2 + "second".len(), prompt(&s));
    s.keys("up");
    s.assert_row(prompt(&s), "› first");
    s.assert_cursor(2 + "first".len(), prompt(&s));
    s.keys("down");
    s.assert_row(prompt(&s), "› second");
    s.keys("down");
    s.assert_row(prompt(&s), "› Ask Codex to do anything");
}

// ORACLE bottom_pane/chat_composer.rs:11168 vim_normal_operator_pending_consumes_submit_key kind=ui owner=chat-live
#[test]
fn vim_normal_operator_pending_consumes_submit_key() {
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    // oracle: 'hello' typed, vim on, Normal mode, 'd' pending (`/vim` turns Vim on)
    s.type_text("/vim").keys("enter");
    s.keys("i").type_text("hello");
    s.keys("esc d");
    s.keys("enter");
    s.assert_row(prompt(&s), "› hello");
    s.assert_contains("Vim: Normal");
}

// ORACLE bottom_pane/chat_composer.rs:11197 remapped_editor_history_navigation_does_not_fall_back_to_up kind=ui owner=chat-live
#[test]
fn remapped_editor_history_navigation_does_not_fall_back_to_up() {
    // oracle: editor.move_up remapped to F2
    let mut s = Screen::build(Owner::ChatLive)
        .size(100, 24)
        .keymap(&[("editor.move_up", "f2")])
        .live()
        .open();
    s.type_text("first").keys("enter");
    s.wait(100);
    s.assert_row(prompt(&s), "› Ask Codex to do anything");
    s.keys("up");
    s.assert_row(prompt(&s), "› Ask Codex to do anything");
    s.keys("f2");
    s.assert_row(prompt(&s), "› first");
}

// ORACLE bottom_pane/chat_composer.rs:11230 history_navigation_from_start_of_bang_command_recalls_older_entry kind=ui owner=chat-live
#[test]
fn history_navigation_from_start_of_bang_command_recalls_older_entry() {
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    s.type_text("first").keys("enter");
    s.type_text("!git").keys("enter");
    s.keys("up");
    // the recalled `!git` is shell mode again: `!` is the prompt
    s.assert_row(prompt(&s), "! git");
    // oracle: the cursor to the start of the line, Up recalls the older entry
    s.keys("home");
    s.keys("up");
    s.assert_row(prompt(&s), "› first");
}

// ORACLE bottom_pane/chat_composer.rs:11266 vim_normal_history_navigation_from_start_of_bang_command_recalls_older_entry kind=ui owner=chat-live
#[test]
fn vim_normal_history_navigation_from_start_of_bang_command_recalls_older_entry() {
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    s.type_text("/vim").keys("enter");
    s.keys("i").type_text("first").keys("enter");
    s.type_text("!git").keys("enter");
    s.keys("esc");
    // oracle: 'k' in vim normal mode recalls '!git' with the cursor on its last character
    // (shell mode draws its `!` in the prompt's place)
    s.keys("k");
    s.assert_row(prompt(&s), "! git");
    s.keys("k");
    s.assert_row(prompt(&s), "› first");
}

// ORACLE bottom_pane/chat_composer.rs:11326 large_paste_preserves_image_text_elements_on_submit kind=ui owner=chat-live
#[test]
fn large_paste_preserves_image_text_elements_on_submit() {
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    // oracle: the large paste, ' ', then attach_image('/tmp/image_with_paste.png')
    s.paste(&"x".repeat(1005));
    s.paste(" ");
    s.attach_image("image_with_paste.png");
    s.keys("enter");
    s.assert_contains("[Image #1]");
    s.assert_absent("[Pasted Content");
}

// ORACLE bottom_pane/chat_composer.rs:11369 large_paste_with_leading_whitespace_trims_and_shifts_elements kind=ui owner=chat-live
#[test]
fn large_paste_with_leading_whitespace_trims_and_shifts_elements() {
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    // oracle: two leading spaces, 1005 x, ' ', then the image
    s.paste(&format!("  {}", "x".repeat(1005)));
    s.paste(" ");
    s.attach_image("image_with_trim.png");
    s.keys("enter");
    s.assert_contains("[Image #1]");
    s.assert_absent("[Pasted Content");
}

// ORACLE bottom_pane/chat_composer.rs:11412 pasted_crlf_normalizes_newlines_for_elements kind=ui owner=chat-live
#[test]
fn pasted_crlf_normalizes_newlines_for_elements() {
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    // oracle: 'line1\r\nline2\r\n', ' ', then the image: the CRLF pairs become '\n'
    s.paste("line1\r\nline2\r\n");
    s.paste(" ");
    s.attach_image("image_crlf.png");
    s.keys("enter");
    s.assert_in_order(&["line1", "line2", "[Image #1]"]);
}

// ORACLE bottom_pane/chat_composer.rs:11455 suppressed_submission_restores_pending_paste_payload kind=ui owner=chat-live
#[test]
fn suppressed_submission_restores_pending_paste_payload() {
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    s.type_text("/unknown ");
    s.paste(&"x".repeat(1005));
    s.assert_row(prompt(&s), "› /unknown [Pasted Content 1005 chars]");
    // oracle: Enter is suppressed, the draft and the pending paste stay
    s.keys("enter");
    s.assert_row(prompt(&s), "› /unknown [Pasted Content 1005 chars]");
    // oracle: the cursor to 0, a space, Enter submits '/unknown' and the full paste
    s.keys("home");
    s.type_text(" ");
    s.keys("enter");
    s.assert_contains("/unknown");
    s.assert_absent("[Pasted Content");
}

// ORACLE bottom_pane/chat_composer.rs:11508 attach_image_without_text_submits_empty_text_and_images kind=ui owner=chat-live
#[test]
fn attach_image_without_text_submits_empty_text_and_images() {
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    // oracle: attach_image('/tmp/image2.png') with no text, Enter
    s.attach_image("image2.png");
    s.keys("enter");
    s.assert_contains("[Image #1]");
}

// ORACLE bottom_pane/chat_composer.rs:11547 duplicate_image_placeholders_get_suffix kind=ui owner=chat-live
#[test]
fn duplicate_image_placeholders_get_suffix() {
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    // oracle: attach_image(same path), ' ', attach_image(same path)
    s.attach_image("image_dup.png");
    s.paste(" ");
    s.attach_image("image_dup.png");
    s.assert_row(prompt(&s), "› [Image #1] [Image #2]");
}

// ORACLE bottom_pane/chat_composer.rs:11576 image_placeholder_backspace_behaves_like_text_placeholder kind=ui owner=chat-live
#[test]
fn image_placeholder_backspace_behaves_like_text_placeholder() {
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    s.attach_image("image3.png");
    s.assert_contains("[Image #1]");
    // oracle case 1: the cursor at the end, Backspace removes the placeholder
    s.keys("end backspace");
    s.assert_absent("[Image #1]");
    // oracle case 2: the cursor about the middle of the re-added placeholder, Backspace keeps it
    s.attach_image("image3.png");
    s.keys("end");
    s.keys(&times("left", 5));
    s.keys("backspace");
    s.assert_contains("[Image #1]");
}

// ORACLE bottom_pane/chat_composer.rs:11615 backspace_with_multibyte_text_before_placeholder_does_not_panic kind=ui owner=chat-live
#[test]
fn backspace_with_multibyte_text_before_placeholder_does_not_panic() {
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    s.attach_image("image_multibyte.png");
    s.type_text("日本語");
    s.keys("backspace");
    s.assert_row(prompt(&s), "› [Image #1]日本");
}

// ORACLE bottom_pane/chat_composer.rs:11645 deleting_one_of_duplicate_image_placeholders_removes_one_entry kind=ui owner=chat-live
#[test]
fn deleting_one_of_duplicate_image_placeholders_removes_one_entry() {
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    s.attach_image("image_dup1.png");
    s.paste(" ");
    s.attach_image("image_dup2.png");
    // oracle: the cursor right after the first placeholder (set_cursor 10), Backspace; an
    // element is one stop for the arrows, so one Right from the start lands there
    s.keys("home right");
    s.keys("backspace");
    s.assert_row(prompt(&s), "›  [Image #1]");
}

// ORACLE bottom_pane/chat_composer.rs:11701 deleting_reordered_image_one_renumbers_text_in_place kind=ui owner=chat-live
#[test]
fn deleting_reordered_image_one_renumbers_text_in_place() {
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    // oracle: the draft 'Test [Image #2] test [Image #1]' (set_text_content, two images): the
    // first image attached, then the second one before it
    s.attach_image("image_first.png");
    s.keys("home");
    s.type_text("Test ");
    let second = s.attach_image("image_second.png");
    s.type_text(" test ");
    s.assert_row(prompt(&s), "› Test [Image #2] test [Image #1]");
    // the cursor after [Image #1], Backspace: the text becomes 'Test [Image #1] test ' (#2
    // renumbered in place)
    s.keys("end backspace");
    s.assert_row(prompt(&s), "› Test [Image #1] test ");
    // what the draft sends is the second image alone
    s.keys("enter");
    s.wait_for("› Ask Codex to do anything");
    s.assert_sole_turn(kit::Turn::any().image_paths(&[second.as_path()]));
}

// ORACLE bottom_pane/chat_composer.rs:11764 deleting_first_text_element_renumbers_following_text_element kind=ui owner=chat-live
#[test]
fn deleting_first_text_element_renumbers_following_text_element() {
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    // oracle: attach_image twice: '[Image #1][Image #2]'
    s.attach_image("image_first.png");
    s.attach_image("image_second.png");
    s.assert_row(prompt(&s), "› [Image #1][Image #2]");
    s.keys("home delete");
    s.assert_row(prompt(&s), "› [Image #1]");
}

// ORACLE bottom_pane/chat_composer.rs:11803 pasting_filepath_attaches_image kind=ui owner=chat-live
#[test]
fn pasting_filepath_attaches_image() {
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    // oracle: a png written to a temp dir, its path pasted
    s.paste(&kit::image_file("codex_tui_test_paste_image.png").to_string_lossy());
    // (the space after the label is the draft's; the screen trims it)
    s.assert_row(prompt(&s), "› [Image #1]");
}

// ORACLE bottom_pane/chat_composer.rs:11829 slash_path_input_submits_without_command_error kind=ui owner=chat-live
#[test]
fn slash_path_input_submits_without_command_error() {
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    // oracle: the draft '/Users/example/project/src/main.rs', Enter
    s.type_text("/Users/example/project/src/main.rs");
    s.keys("enter");
    s.assert_row(prompt(&s), "› Ask Codex to do anything");
    s.assert_contains("/Users/example/project/src/main.rs");
}

// ORACLE bottom_pane/chat_composer.rs:11866 slash_with_leading_space_submits_as_text kind=ui owner=chat-live
#[test]
fn slash_with_leading_space_submits_as_text() {
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    // oracle: the draft ' /this-looks-like-a-command', Enter
    s.type_text(" /this-looks-like-a-command");
    s.keys("enter");
    s.assert_row(prompt(&s), "› Ask Codex to do anything");
    s.assert_contains("/this-looks-like-a-command");
}

// ORACLE bottom_pane/chat_composer.rs:11903 shortcut_modified_input_does_not_enter_paste_bursts kind=snap owner=chat-live snap=shortcut_modified_space_is_not_rendered
#[test]
fn shortcut_modified_input_does_not_enter_paste_bursts() {
    // oracle snapshot shortcut_modified_space_is_not_rendered: 'a', SUPER+space, 'b' typed fast, then the flush
    let mut s = Screen::build(Owner::ChatLive)
        .size(100, 9)
        .entry("composer", "alone")
        .open();
    s.keys("a");
    s.keys("super+space");
    s.keys("b");
    s.assert_snap("shortcut_modified_space_is_not_rendered");
}

// ORACLE bottom_pane/chat_composer.rs:11949 plain_and_shift_spaces_insert_normally kind=snap owner=chat-live snap=plain_space_is_rendered
#[test]
fn plain_and_shift_spaces_insert_normally() {
    // oracle snapshot plain_space_is_rendered: 'a', ' ', 'b' typed fast, then the flush
    let mut s = Screen::build(Owner::ChatLive)
        .size(100, 9)
        .entry("composer", "alone")
        .open();
    s.type_burst("a b");
    s.wait(50);
    s.assert_snap("plain_space_is_rendered");
}

// ORACLE bottom_pane/chat_composer.rs:11987 non_text_input_flushes_pending_paste_without_inserting_text kind=ui owner=chat-live
#[test]
fn non_text_input_flushes_pending_paste_without_inserting_text() {
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    // oracle: 'a', 'b' typed fast (the burst is pending), then SUPER+space, a non-text input
    s.type_text("ab");
    s.keys("super+space");
    s.assert_row(prompt(&s), "› ab");
}

// ORACLE bottom_pane/chat_composer.rs:12014 platform_specific_altgr_input_remains_text kind=ui owner=chat-live
#[test]
fn platform_specific_altgr_input_remains_text() {
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    // oracle, non-Windows branch: Ctrl+Alt+'@' starts no paste burst and inserts no text
    s.keys("ctrl+alt+@");
    s.assert_row(prompt(&s), "› Ask Codex to do anything");
}

// ORACLE bottom_pane/chat_composer.rs:12044 pending_first_ascii_char_flushes_as_typed kind=ui owner=chat-live
#[test]
fn pending_first_ascii_char_flushes_as_typed() {
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    // oracle: 'h' pressed fast: the burst holds it, nothing shows yet
    s.type_burst("h");
    s.assert_row(prompt(&s), "› Ask Codex to do anything");
    // oracle: sleep for the recommended flush delay (9 ms), then the flush shows 'h'
    s.wait(50);
    s.assert_row(prompt(&s), "› h");
}

// ORACLE bottom_pane/chat_composer.rs:12073 burst_paste_fast_small_buffers_and_flushes_on_stop kind=ui owner=chat-live
#[test]
fn burst_paste_fast_small_buffers_and_flushes_on_stop() {
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    // oracle: 32 'a' keys 1 ms apart: nothing is drawn during the burst
    s.type_burst(&"a".repeat(32));
    s.assert_row(prompt(&s), "› Ask Codex to do anything");
    // oracle: the burst stops, the flush shows the 32 characters
    s.wait(50);
    s.assert_row(prompt(&s), &format!("› {}", "a".repeat(32)));
}

// ORACLE bottom_pane/chat_composer.rs:12124 burst_paste_fast_large_inserts_placeholder_on_flush kind=ui owner=chat-live
#[test]
fn burst_paste_fast_large_inserts_placeholder_on_flush() {
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    // oracle: 1001 'x' keys 1 ms apart: nothing is drawn during the burst
    s.type_burst(&"x".repeat(1001));
    s.assert_row(prompt(&s), "› Ask Codex to do anything");
    // oracle: the burst stops, the flush inserts the placeholder
    s.wait(50);
    s.assert_row(prompt(&s), "› [Pasted Content 1001 chars]");
}

// ORACLE bottom_pane/chat_composer.rs:12163 humanlike_typing_1000_chars_appears_live_no_placeholder kind=ui owner=chat-live
#[test]
fn humanlike_typing_1000_chars_appears_live_no_placeholder() {
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    // oracle: 1000 'z' keys 9 ms apart: each one shows at once
    s.type_text(&"z".repeat(1000));
    s.assert_absent("[Pasted Content");
}

// ORACLE bottom_pane/chat_composer.rs:12191 slash_popup_not_activated_for_slash_space_text_history_like_input kind=ui owner=chat-live
#[test]
fn slash_popup_not_activated_for_slash_space_text_history_like_input() {
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    // oracle: the draft '/ test' (history-like input): no slash popup
    s.type_text("/ test");
    s.assert_row(prompt(&s), "› / test");
    // oracle: Up is handled by history navigation, the result is None
    s.keys("up");
    s.assert_row(prompt(&s), "› / test");
}

// ORACLE bottom_pane/chat_composer.rs:12269 bare_slash_command_can_be_recalled_after_recording_pending_history kind=ui owner=chat-live
#[test]
fn bare_slash_command_can_be_recalled_after_recording_pending_history() {
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    // oracle: '/diff' submitted as the Diff command, its history recorded
    s.type_text("/diff").keys("enter");
    // the diff overlay (a popup until the pager is built) is closed first: Up is the composer's
    s.keys("esc up");
    s.assert_row(prompt(&s), "› /diff");
}

// ORACLE bottom_pane/chat_composer.rs:12294 popup_selected_slash_command_records_canonical_command_history kind=ui owner=chat-live
#[test]
fn popup_selected_slash_command_records_canonical_command_history() {
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    // oracle: '/di' with the popup open: Enter selects the Diff command
    s.type_text("/di").keys("enter");
    s.keys("esc up");
    s.assert_row(prompt(&s), "› /diff");
}

// ORACLE bottom_pane/chat_composer.rs:12319 inline_slash_command_can_be_recalled_after_recording_pending_history kind=ui owner=chat-live
#[test]
fn inline_slash_command_can_be_recalled_after_recording_pending_history() {
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    // oracle: '/plan investigate this' with the popup closed: the Plan command with args
    s.type_text("/plan investigate this").keys("enter");
    s.keys("up");
    s.assert_row(prompt(&s), "› /plan investigate this");
}

// ORACLE bottom_pane/chat_composer.rs:12545 current_text_with_pending_expands_overlapping_placeholders kind=ui owner=chat-live
#[test]
fn current_text_with_pending_expands_overlapping_placeholders() {
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    s.paste(&"a".repeat(1004));
    s.paste(&"b".repeat(1004));
    s.assert_row(
        prompt(&s),
        "› [Pasted Content 1004 chars][Pasted Content 1004 chars] #2",
    );
    s.keys("enter");
    s.assert_absent("[Pasted Content");
}

// ORACLE bottom_pane/chat_composer.rs:12706 delete_selected_remote_image_relabels_local_placeholders kind=ui owner=chat-live
#[test]
fn delete_selected_remote_image_relabels_local_placeholders() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(100, 24)
        .live()
        .remote_images(&["https://example.com/one.png", "https://example.com/two.png"])
        .open();
    // oracle: two remote image URLs; a local image attached (label #3); the cursor to 0
    s.attach_image("local.png");
    s.assert_row(prompt(&s), "› [Image #3]");
    // oracle: Up selects the last remote image, Delete removes it
    s.keys("home up delete");
    s.assert_row(prompt(&s), "› [Image #2]");
    s.keys("up delete");
    s.assert_row(prompt(&s), "› [Image #1]");
}

// ORACLE bottom_pane/chat_composer.rs:12747 input_disabled_ignores_keypresses_and_hides_cursor kind=ui owner=chat-live
#[test]
fn input_disabled_ignores_keypresses_and_hides_cursor() {
    kit::pending(
        Owner::ChatLive,
        "no input-disabled state on the chat composer: while input is off, a key is ignored, the draft stays, and the cursor is hidden",
    );
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    // oracle: the draft 'hello', input disabled ('Input disabled for test.'), 'x' pressed
    s.type_text("hello");
    s.keys("x");
    s.assert_row(prompt(&s), "› hello");
    s.assert_cursor_hidden();
}

// ORACLE bottom_pane/chat_composer.rs:12785 shutdown_in_progress_disables_input_and_uses_hint_without_footer kind=snap owner=chat-live snap=shutdown_in_progress
#[test]
fn shutdown_in_progress_disables_input_and_uses_hint_without_footer() {
    // oracle snapshot shutdown_in_progress at 40x5: the draft 'hello', then show_shutdown_in_progress
    let mut s = Screen::build(Owner::ChatLive).size(40, 5).live().open();
    s.type_text("hello");
    s.feed(r#"{"step":"screen","dir":"chat","ev":{"facts":{"shutting_down":true}}}"#);
    // the oracle draws the composer alone (the snapshot's rows 1..): the page keeps it at the
    // bottom of the screen, with no footer under it
    s.assert_row(prompt(&s), "› Shutting down...");
    s.assert_absent("hello");
    s.assert_absent("? for shortcuts");
    s.assert_cursor_hidden();
}

// ---- oracle: bottom_pane/chat_composer/agents_navigation_tests.rs (6 screen-level tests here; 0 state tests left to chat-live)
// ORACLE bottom_pane/chat_composer/agents_navigation_tests.rs:9 parent_owned_thread_allows_local_commands kind=ui owner=chat-live
#[test]
fn parent_owned_thread_allows_local_commands() {
    kit::na(
        Owner::ChatLive,
        "asserts the InputResult of Enter on each local command (Command and CommandWithArgs values) in a parent-owned thread, not rows: a state test",
    );
}

// ORACLE bottom_pane/chat_composer/agents_navigation_tests.rs:46 parent_owned_thread_allows_safe_command_selected_from_prefix kind=ui owner=chat-live
#[test]
fn parent_owned_thread_allows_safe_command_selected_from_prefix() {
    kit::na(
        Owner::ChatLive,
        "asserts the InputResult of Enter on the popup's selected '/agents' command, not rows: a state test",
    );
}

// ORACLE bottom_pane/chat_composer/agents_navigation_tests.rs:59 left_navigates_only_from_an_empty_prompt kind=ui owner=chat-live
#[test]
fn left_navigates_only_from_an_empty_prompt() {
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    // oracle: 'ab', Left, '!' inserted between: 'a!b'
    s.type_text("ab");
    s.keys("left");
    s.type_text("!");
    s.assert_row(prompt(&s), "› a!b");
    // oracle: clear_for_ctrl_c empties the draft; Left on the empty prompt opens the agents overview
    s.keys("ctrl+c");
    s.assert_row(prompt(&s), "› Ask Codex to do anything");
    s.keys("left");
    s.wait(100);
    s.assert_contains("Agent command center");
}

// ORACLE bottom_pane/chat_composer/agents_navigation_tests.rs:82 left_respects_attachments_pastes_and_other_input_surfaces kind=ui owner=chat-live
#[test]
fn left_respects_attachments_pastes_and_other_input_surfaces() {
    // oracle: Left with a typed 'x' in the draft: no agents overview, the rows stay
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    s.type_text("x");
    let before = s.shot();
    s.keys("left");
    s.assert_same_rows(&before);
    // oracle: Left with a paste in the draft: the same
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    s.paste("pasted");
    let before = s.shot();
    s.keys("left");
    s.assert_same_rows(&before);
}

// ORACLE bottom_pane/chat_composer/agents_navigation_tests.rs:106 agents_navigation_hint_snapshots kind=snap owner=chat-live snap=agents_navigation_enabled snap=agents_navigation_help snap=agents_navigation_status
#[test]
fn agents_navigation_hint_snapshots() {
    kit::pending(
        Owner::ChatLive,
        "no agents-navigation hint on the chat composer: the enabled hint (80x9), the '?' help (80x28) and the status line 'model · project' (80x9) are not drawn",
    );
    // oracle snapshot agents_navigation_enabled (80 wide)
    let s = Screen::build(Owner::ChatLive).size(80, 9).live().open();
    s.assert_snap("agents_navigation_enabled");
    // oracle snapshot agents_navigation_help: '?' pressed
    let mut s = Screen::build(Owner::ChatLive).size(80, 28).live().open();
    s.keys("?");
    s.assert_snap("agents_navigation_help");
    // oracle snapshot agents_navigation_status: the status line 'model · project'
    let s = Screen::build(Owner::ChatLive).size(80, 9).live().open();
    s.assert_snap("agents_navigation_status");
}

// ORACLE bottom_pane/chat_composer/agents_navigation_tests.rs:134 left_respects_editor_and_vim_history_remaps kind=ui owner=chat-live
#[test]
fn left_respects_editor_and_vim_history_remaps() {
    kit::pending(
        Owner::ChatLive,
        "no keymap seed for editor.move_left, editor.move_up or vim_normal.move_left, and no agents navigation on the chat composer: a remapped Left recalls history and does not open the overview",
    );
    // oracle, editor remap: move_left = ctrl-b, move_up = left; 'previous' submitted, then Left recalls it
    let mut s = Screen::build(Owner::ChatLive)
        .size(100, 24)
        .keymap(&[("editor.move_left", "ctrl-b"), ("editor.move_up", "left")])
        .live()
        .open();
    s.type_text("previous").keys("enter");
    s.keys("left");
    s.assert_row(prompt(&s), "› previous");
    // oracle, vim remap: vim_normal.move_left = h, move_up = left
    let mut s = Screen::build(Owner::ChatLive)
        .size(100, 24)
        .keymap(&[
            ("vim_normal.move_left", "h"),
            ("vim_normal.move_up", "left"),
        ])
        .live()
        .open();
    s.type_text("previous").keys("enter");
    s.keys("esc left");
    s.assert_row(prompt(&s), "› previous");
}
