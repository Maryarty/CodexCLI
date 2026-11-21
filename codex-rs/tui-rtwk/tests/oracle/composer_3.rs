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
        .rposition(|r| r.starts_with('›'))
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
    kit::pending(
        Owner::ChatLive,
        "no file-search result on the chat composer (Tab completes '@ma' to 'src/main.rs') and no large-paste placeholder: a 1005-char paste draws its text, not '[Pasted Content 1005 chars]'",
    );
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
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
    kit::pending(
        Owner::ChatLive,
        "no file-search result on the chat composer: Tab completes '@ma' to '@scope/main.rs' in a two-line draft, the popup does not reopen, and Enter submits both lines",
    );
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
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
    kit::pending(
        Owner::ChatLive,
        "no file popup on the chat composer: '@ma' opens it, Esc Esc dismisses it, and the dismissal holds across leading-space edits",
    );
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
    kit::pending(
        Owner::ChatLive,
        "no file popup on the chat composer: after Esc Esc on '@ma', a paste at the start does not reopen the popup for the '@ma' inside 'email@ma.com'",
    );
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
    kit::pending(
        Owner::ChatLive,
        "no large-paste placeholder on the chat composer: pastes over 1000 chars draw '[Pasted Content N chars]' and expand to their text on submit",
    );
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
    kit::pending(
        Owner::ChatLive,
        "no large-paste placeholder on the chat composer: Backspace at the end of a placeholder removes it and its paste",
    );
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    s.paste(&"a".repeat(1005));
    s.paste(" and ");
    s.paste(&"b".repeat(1006));
    s.assert_row(
        prompt(&s),
        "› [Pasted Content 1005 chars] and [Pasted Content 1006 chars]",
    );
    // oracle: the cursor right after the first placeholder (27 chars in from the start), Backspace
    s.keys(&times("left", 32));
    s.keys("backspace");
    s.assert_row(prompt(&s), "›  and [Pasted Content 1006 chars]");
    // oracle: the cursor to the end, Backspace removes the second placeholder
    s.keys("end backspace");
    s.assert_row(prompt(&s), "›  and ");
}

// ORACLE bottom_pane/chat_composer.rs:10798 deleting_duplicate_length_pastes_removes_only_target kind=ui owner=chat-live
#[test]
fn deleting_duplicate_length_pastes_removes_only_target() {
    kit::pending(
        Owner::ChatLive,
        "no large-paste placeholder on the chat composer: two pastes of the same length get '#2' and Backspace at the end removes only the second",
    );
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
    kit::pending(
        Owner::ChatLive,
        "no large-paste placeholder on the chat composer: a same-length placeholder keeps its '#2' and a new paste gets '#3'",
    );
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    s.paste(&"x".repeat(1004));
    s.paste(&"x".repeat(1004));
    // oracle: the cursor after the first placeholder, Backspace removes it
    s.keys(&times("left", 30));
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
    kit::pending(
        Owner::ChatLive,
        "no large-paste placeholder on the chat composer: once every placeholder is deleted, the next paste is numbered from the base label",
    );
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
    kit::pending(
        Owner::ChatLive,
        "no large-paste placeholder on the chat composer: Backspace inside a placeholder breaks it, so its text no longer shows the placeholder",
    );
    // oracle case 1: the cursor 5 chars before the placeholder's end, Backspace
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    s.paste(&"x".repeat(1004));
    s.keys(&times("left", 5));
    s.keys("backspace");
    s.assert_absent("[Pasted Content 1004 chars]");
    // oracle case 2: the cursor at the end, Backspace
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    s.paste(&"x".repeat(1004));
    s.keys("backspace");
    s.assert_absent("[Pasted Content 1004 chars]");
}

// ORACLE bottom_pane/chat_composer.rs:10975 attach_image_and_submit_includes_local_image_paths kind=ui owner=chat-live
#[test]
fn attach_image_and_submit_includes_local_image_paths() {
    kit::pending(
        Owner::ChatLive,
        "no image attachment on the chat composer: the oracle attaches /tmp/image1.png through the API; the screen has no attach path, so a pasted png path stands in for it",
    );
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    // oracle: attach_image('/tmp/image1.png'), then ' hi'
    s.paste("/tmp/image1.png");
    s.paste(" hi");
    s.assert_row(prompt(&s), "› [Image #1] hi");
    // oracle: Enter submits '[Image #1] hi' with one image element at 0..10
    s.keys("enter");
    s.assert_contains("[Image #1] hi");
}

// ORACLE bottom_pane/chat_composer.rs:11013 submit_captures_recent_mention_bindings_before_clearing_textarea kind=ui owner=chat-live
#[test]
fn submit_captures_recent_mention_bindings_before_clearing_textarea() {
    kit::pending(
        Owner::ChatLive,
        "no skill mention bindings on the chat composer: '$figma' bound to a skill; the submit shows the text and clears the composer (the binding is not a row)",
    );
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    // oracle: the draft '$figma please' with a '$figma' mention binding; Enter submits and clears
    s.type_text("$figma please");
    s.keys("enter");
    s.assert_row(prompt(&s), "› Ask Codex to do anything");
    s.assert_contains("$figma please");
}

// ORACLE bottom_pane/chat_composer.rs:11047 history_navigation_restores_remote_and_local_image_attachments kind=ui owner=chat-live
#[test]
fn history_navigation_restores_remote_and_local_image_attachments() {
    kit::pending(
        Owner::ChatLive,
        "no remote image URLs and no prompt history on the chat composer: a submitted local image comes back as '[Image #2]' on Up, after the remote one",
    );
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    // oracle: one remote image URL, a local image attached, Enter, then Up in an empty draft
    s.paste("/tmp/image1.png");
    s.keys("enter");
    s.keys("up");
    s.assert_row(prompt(&s), "› [Image #2]");
}

// ORACLE bottom_pane/chat_composer.rs:11081 history_navigation_restores_remote_only_submissions kind=ui owner=chat-live
#[test]
fn history_navigation_restores_remote_only_submissions() {
    kit::pending(
        Owner::ChatLive,
        "no remote image URLs and no prompt history on the chat composer: a remote-only submission is recorded, and Up restores it with an empty draft",
    );
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    // oracle: two remote image URLs and no text; Enter records the empty submission
    s.keys("enter");
    // oracle: Up restores it: the text is empty and the remote images come back
    s.keys("up");
    s.assert_row(prompt(&s), "› Ask Codex to do anything");
}

// ORACLE bottom_pane/chat_composer.rs:11113 history_navigation_leaves_cursor_at_end_of_line kind=ui owner=chat-live
#[test]
fn history_navigation_leaves_cursor_at_end_of_line() {
    kit::pending(
        Owner::ChatLive,
        "no prompt history on the chat composer: Up recalls 'second' then 'first', and Down walks back, each with the cursor at the end of the line",
    );
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
    kit::pending(
        Owner::ChatLive,
        "no vim editing mode on the chat composer: 'd' leaves an operator pending, Enter is consumed (the draft stays), and the footer shows 'Vim: Normal'",
    );
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    // oracle: 'hello' typed, vim on, Normal mode, 'd' pending
    s.type_text("hello");
    s.keys("esc d");
    s.keys("enter");
    s.assert_row(prompt(&s), "› hello");
    s.assert_contains("Vim: Normal");
}

// ORACLE bottom_pane/chat_composer.rs:11197 remapped_editor_history_navigation_does_not_fall_back_to_up kind=ui owner=chat-live
#[test]
fn remapped_editor_history_navigation_does_not_fall_back_to_up() {
    kit::pending(
        Owner::ChatLive,
        "no keymap seed for editor.move_up on the chat composer, and no prompt history: F2 recalls 'first' while Up on an empty draft does nothing",
    );
    // oracle: editor.move_up remapped to F2
    let mut s = Screen::build(Owner::ChatLive)
        .size(100, 24)
        .keymap(&[("editor.move_up", "f2")])
        .live()
        .open();
    s.type_text("first").keys("enter");
    s.keys("up");
    s.assert_row(prompt(&s), "› Ask Codex to do anything");
    s.keys("f2");
    s.assert_row(prompt(&s), "› first");
}

// ORACLE bottom_pane/chat_composer.rs:11230 history_navigation_from_start_of_bang_command_recalls_older_entry kind=ui owner=chat-live
#[test]
fn history_navigation_from_start_of_bang_command_recalls_older_entry() {
    kit::pending(
        Owner::ChatLive,
        "no prompt history or bang (!) shell-command mode on the chat composer: Up from the start of '!git' recalls the older entry 'first'",
    );
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    s.type_text("first").keys("enter");
    s.type_text("!git").keys("enter");
    s.keys("up");
    s.assert_row(prompt(&s), "› !git");
    // oracle: the cursor to the start of the line, Up recalls the older entry
    s.keys("home");
    s.keys("up");
    s.assert_row(prompt(&s), "› first");
}

// ORACLE bottom_pane/chat_composer.rs:11266 vim_normal_history_navigation_from_start_of_bang_command_recalls_older_entry kind=ui owner=chat-live
#[test]
fn vim_normal_history_navigation_from_start_of_bang_command_recalls_older_entry() {
    kit::pending(
        Owner::ChatLive,
        "no vim normal mode or prompt history on the chat composer: 'k' from the start of '!git' recalls 'first'",
    );
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    s.type_text("first").keys("enter");
    s.type_text("!git").keys("enter");
    s.keys("esc");
    // oracle: 'k' in vim normal mode recalls '!git' with the cursor on its last character
    s.keys("k");
    s.assert_row(prompt(&s), "› !git");
    s.keys("k");
    s.assert_row(prompt(&s), "› first");
}

// ORACLE bottom_pane/chat_composer.rs:11326 large_paste_preserves_image_text_elements_on_submit kind=ui owner=chat-live
#[test]
fn large_paste_preserves_image_text_elements_on_submit() {
    kit::pending(
        Owner::ChatLive,
        "no large-paste placeholder and no image attachment on the chat composer: a 1005-char paste, a space and an image submit as the text then '[Image #1]'",
    );
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    // oracle: the large paste, ' ', then attach_image('/tmp/image_with_paste.png')
    s.paste(&"x".repeat(1005));
    s.paste(" ");
    s.paste("/tmp/image_with_paste.png");
    s.keys("enter");
    s.assert_contains("[Image #1]");
    s.assert_absent("[Pasted Content");
}

// ORACLE bottom_pane/chat_composer.rs:11369 large_paste_with_leading_whitespace_trims_and_shifts_elements kind=ui owner=chat-live
#[test]
fn large_paste_with_leading_whitespace_trims_and_shifts_elements() {
    kit::pending(
        Owner::ChatLive,
        "no large-paste placeholder and no image attachment on the chat composer: the leading spaces of a large paste are trimmed on submit, and the image label follows the text",
    );
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    // oracle: two leading spaces, 1005 x, ' ', then the image
    s.paste(&format!("  {}", "x".repeat(1005)));
    s.paste(" ");
    s.paste("/tmp/image_with_trim.png");
    s.keys("enter");
    s.assert_contains("[Image #1]");
    s.assert_absent("[Pasted Content");
}

// ORACLE bottom_pane/chat_composer.rs:11412 pasted_crlf_normalizes_newlines_for_elements kind=ui owner=chat-live
#[test]
fn pasted_crlf_normalizes_newlines_for_elements() {
    kit::pending(
        Owner::ChatLive,
        "no image attachment on the chat composer: a paste with CRLF line ends submits as 'line1' and 'line2' lines, then the image label",
    );
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    // oracle: 'line1\r\nline2\r\n', ' ', then the image: the CRLF pairs become '\n'
    s.paste("line1\r\nline2\r\n");
    s.paste(" ");
    s.paste("/tmp/image_crlf.png");
    s.keys("enter");
    s.assert_in_order(&["line1", "line2", "[Image #1]"]);
}

// ORACLE bottom_pane/chat_composer.rs:11455 suppressed_submission_restores_pending_paste_payload kind=ui owner=chat-live
#[test]
fn suppressed_submission_restores_pending_paste_payload() {
    kit::pending(
        Owner::ChatLive,
        "no large-paste placeholder on the chat composer: Enter on '/unknown [Pasted Content 1005 chars]' submits nothing and keeps the paste; after a space at the start, the paste is submitted",
    );
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
    kit::pending(
        Owner::ChatLive,
        "no image attachment on the chat composer: an image with no text submits as '[Image #1]' alone",
    );
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    // oracle: attach_image('/tmp/image2.png') with no text, Enter
    s.paste("/tmp/image2.png");
    s.keys("enter");
    s.assert_contains("[Image #1]");
}

// ORACLE bottom_pane/chat_composer.rs:11547 duplicate_image_placeholders_get_suffix kind=ui owner=chat-live
#[test]
fn duplicate_image_placeholders_get_suffix() {
    kit::pending(
        Owner::ChatLive,
        "no image attachment on the chat composer: the same image attached twice gets the labels '[Image #1]' and '[Image #2]'",
    );
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    // oracle: attach_image(same path), ' ', attach_image(same path)
    s.paste("/tmp/image_dup.png");
    s.paste(" ");
    s.paste("/tmp/image_dup.png");
    s.assert_row(prompt(&s), "› [Image #1] [Image #2]");
}

// ORACLE bottom_pane/chat_composer.rs:11576 image_placeholder_backspace_behaves_like_text_placeholder kind=ui owner=chat-live
#[test]
fn image_placeholder_backspace_behaves_like_text_placeholder() {
    kit::pending(
        Owner::ChatLive,
        "no image attachment on the chat composer: Backspace at the end removes the '[Image #1]' placeholder, and Backspace inside it keeps it",
    );
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    s.paste("/tmp/image3.png");
    s.assert_contains("[Image #1]");
    // oracle case 1: the cursor at the end, Backspace removes the placeholder
    s.keys("end backspace");
    s.assert_absent("[Image #1]");
    // oracle case 2: the cursor about the middle of the re-added placeholder, Backspace keeps it
    s.paste("/tmp/image3.png");
    s.keys("end");
    s.keys(&times("left", 5));
    s.keys("backspace");
    s.assert_contains("[Image #1]");
}

// ORACLE bottom_pane/chat_composer.rs:11615 backspace_with_multibyte_text_before_placeholder_does_not_panic kind=ui owner=chat-live
#[test]
fn backspace_with_multibyte_text_before_placeholder_does_not_panic() {
    kit::pending(
        Owner::ChatLive,
        "no image attachment on the chat composer: after '[Image #1]', typed '日本語' and Backspace leaves '[Image #1]日本' with the placeholder intact",
    );
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    s.paste("/tmp/image_multibyte.png");
    s.type_text("日本語");
    s.keys("backspace");
    s.assert_row(prompt(&s), "› [Image #1]日本");
}

// ORACLE bottom_pane/chat_composer.rs:11645 deleting_one_of_duplicate_image_placeholders_removes_one_entry kind=ui owner=chat-live
#[test]
fn deleting_one_of_duplicate_image_placeholders_removes_one_entry() {
    kit::pending(
        Owner::ChatLive,
        "no image attachment on the chat composer: Backspace after the first of two image placeholders removes it and relabels the second as '[Image #1]'",
    );
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    s.paste("/tmp/image_dup1.png");
    s.paste(" ");
    s.paste("/tmp/image_dup2.png");
    // oracle: the cursor right after the first placeholder (10 columns), Backspace
    s.keys("home");
    s.keys(&times("right", 10));
    s.keys("backspace");
    s.assert_row(prompt(&s), "›  [Image #1]");
}

// ORACLE bottom_pane/chat_composer.rs:11701 deleting_reordered_image_one_renumbers_text_in_place kind=ui owner=chat-live
#[test]
fn deleting_reordered_image_one_renumbers_text_in_place() {
    kit::pending(
        Owner::ChatLive,
        "no image attachment and no draft seed on the chat composer: the oracle sets 'Test [Image #2] test [Image #1]' with both images; no key sequence builds that draft",
    );
    let s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    // oracle: the draft 'Test [Image #2] test [Image #1]' (set_text_content, two images), the cursor
    // after [Image #1], Backspace: the text becomes 'Test [Image #1] test ' (#2 renumbered in place)
    s.assert_row(prompt(&s), "› Test [Image #1] test ");
}

// ORACLE bottom_pane/chat_composer.rs:11764 deleting_first_text_element_renumbers_following_text_element kind=ui owner=chat-live
#[test]
fn deleting_first_text_element_renumbers_following_text_element() {
    kit::pending(
        Owner::ChatLive,
        "no image attachment on the chat composer: two adjacent images, Delete at the start removes the first and renumbers the second to '[Image #1]'",
    );
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    // oracle: attach_image twice: '[Image #1][Image #2]'
    s.paste("/tmp/image_first.png");
    s.paste("/tmp/image_second.png");
    s.assert_row(prompt(&s), "› [Image #1][Image #2]");
    s.keys("home delete");
    s.assert_row(prompt(&s), "› [Image #1]");
}

// ORACLE bottom_pane/chat_composer.rs:11803 pasting_filepath_attaches_image kind=ui owner=chat-live
#[test]
fn pasting_filepath_attaches_image() {
    kit::pending(
        Owner::ChatLive,
        "no image attachment on the chat composer: a pasted png path becomes '[Image #1] ' in the draft",
    );
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    // oracle: a 3x2 png written to a temp dir, its path pasted (the kit has no temp-file seed)
    s.paste("/tmp/codex_tui_test_paste_image.png");
    s.assert_row_starts_with(prompt(&s), "› [Image #1] ");
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
    kit::pending(
        Owner::ChatLive,
        "no paste-burst state on the chat composer, and keys() has no SUPER modifier: 'a', SUPER+space, 'b' must render as 'ab' with no space, and SUPER/HYPER/META spaces must not start a burst",
    );
    // oracle snapshot shortcut_modified_space_is_not_rendered: 'a', SUPER+space, 'b' typed fast, then the flush
    let mut s = Screen::build(Owner::ChatLive).size(100, 9).live().open();
    s.keys("a");
    s.keys("super+space");
    s.keys("b");
    s.assert_snap("shortcut_modified_space_is_not_rendered");
}

// ORACLE bottom_pane/chat_composer.rs:11949 plain_and_shift_spaces_insert_normally kind=snap owner=chat-live snap=plain_space_is_rendered
#[test]
fn plain_and_shift_spaces_insert_normally() {
    kit::pending(
        Owner::ChatLive,
        "no paste-burst state on the chat composer: 'a', a plain space, 'b' typed fast flush to 'a b' (the snapshot plain_space_is_rendered)",
    );
    // oracle snapshot plain_space_is_rendered: 'a', ' ', 'b' typed fast, then the flush
    let mut s = Screen::build(Owner::ChatLive).size(100, 9).live().open();
    s.type_text("a b");
    s.assert_snap("plain_space_is_rendered");
}

// ORACLE bottom_pane/chat_composer.rs:11987 non_text_input_flushes_pending_paste_without_inserting_text kind=ui owner=chat-live
#[test]
fn non_text_input_flushes_pending_paste_without_inserting_text() {
    kit::pending(
        Owner::ChatLive,
        "no paste-burst state on the chat composer, and keys() has no SUPER modifier: 'ab' typed fast is held until a non-text key flushes it, with no space inserted",
    );
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    // oracle: 'a', 'b' typed fast (the burst is pending), then SUPER+space, a non-text input
    s.type_text("ab");
    s.keys("super+space");
    s.assert_row(prompt(&s), "› ab");
}

// ORACLE bottom_pane/chat_composer.rs:12014 platform_specific_altgr_input_remains_text kind=ui owner=chat-live
#[test]
fn platform_specific_altgr_input_remains_text() {
    kit::pending(
        Owner::ChatLive,
        "no paste-burst state on the chat composer: Ctrl+Alt+'@' is not a burst and inserts nothing on this platform (the oracle's non-Windows branch)",
    );
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    // oracle, non-Windows branch: Ctrl+Alt+'@' starts no paste burst and inserts no text
    s.keys("ctrl+alt+@");
    s.assert_row(prompt(&s), "› Ask Codex to do anything");
}

// ORACLE bottom_pane/chat_composer.rs:12044 pending_first_ascii_char_flushes_as_typed kind=ui owner=chat-live
#[test]
fn pending_first_ascii_char_flushes_as_typed() {
    kit::pending(
        Owner::ChatLive,
        "no paste-burst state on the chat composer: the first fast 'h' is held, then flushes as typed text after the 9 ms flush delay",
    );
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    // oracle: 'h' pressed fast: the burst holds it, nothing shows yet
    s.keys("h");
    s.assert_row(prompt(&s), "› Ask Codex to do anything");
    // oracle: sleep for the recommended flush delay (9 ms), then the flush shows 'h'
    s.wait(50);
    s.assert_row(prompt(&s), "› h");
}

// ORACLE bottom_pane/chat_composer.rs:12073 burst_paste_fast_small_buffers_and_flushes_on_stop kind=ui owner=chat-live
#[test]
fn burst_paste_fast_small_buffers_and_flushes_on_stop() {
    kit::pending(
        Owner::ChatLive,
        "no paste-burst state on the chat composer: 32 fast 'a' keys are held until the burst stops, then show as typed text with no placeholder",
    );
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    // oracle: 32 'a' keys 1 ms apart: nothing is drawn during the burst
    s.type_text(&"a".repeat(32));
    s.assert_row(prompt(&s), "› Ask Codex to do anything");
    // oracle: the burst stops, the flush shows the 32 characters
    s.wait(50);
    s.assert_row(prompt(&s), &format!("› {}", "a".repeat(32)));
}

// ORACLE bottom_pane/chat_composer.rs:12124 burst_paste_fast_large_inserts_placeholder_on_flush kind=ui owner=chat-live
#[test]
fn burst_paste_fast_large_inserts_placeholder_on_flush() {
    kit::pending(
        Owner::ChatLive,
        "no paste-burst state or large-paste placeholder on the chat composer: 1001 fast 'x' keys flush as '[Pasted Content 1001 chars]'",
    );
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    // oracle: 1001 'x' keys 1 ms apart: nothing is drawn during the burst
    s.type_text(&"x".repeat(1001));
    s.assert_row(prompt(&s), "› Ask Codex to do anything");
    // oracle: the burst stops, the flush inserts the placeholder
    s.wait(50);
    s.assert_row(prompt(&s), "› [Pasted Content 1001 chars]");
}

// ORACLE bottom_pane/chat_composer.rs:12163 humanlike_typing_1000_chars_appears_live_no_placeholder kind=ui owner=chat-live
#[test]
fn humanlike_typing_1000_chars_appears_live_no_placeholder() {
    kit::pending(
        Owner::ChatLive,
        "no paste-burst state on the chat composer: 1000 human-paced 'z' keys appear live with no placeholder",
    );
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    // oracle: 1000 'z' keys 9 ms apart: each one shows at once
    s.type_text(&"z".repeat(1000));
    s.assert_absent("[Pasted Content");
}

// ORACLE bottom_pane/chat_composer.rs:12191 slash_popup_not_activated_for_slash_space_text_history_like_input kind=ui owner=chat-live
#[test]
fn slash_popup_not_activated_for_slash_space_text_history_like_input() {
    kit::pending(
        Owner::ChatLive,
        "no slash popup and no prompt history on the chat composer: '/ test' opens no command popup, and Up goes to history, not the popup",
    );
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
    kit::pending(
        Owner::ChatLive,
        "no slash-command history on the chat composer: '/diff' submitted, then Up recalls '/diff'",
    );
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    // oracle: '/diff' submitted as the Diff command, its history recorded
    s.type_text("/diff").keys("enter");
    s.keys("up");
    s.assert_row(prompt(&s), "› /diff");
}

// ORACLE bottom_pane/chat_composer.rs:12294 popup_selected_slash_command_records_canonical_command_history kind=ui owner=chat-live
#[test]
fn popup_selected_slash_command_records_canonical_command_history() {
    kit::pending(
        Owner::ChatLive,
        "no slash-command popup and no slash-command history on the chat composer: '/di' with Enter selects '/diff', and Up recalls '/diff'",
    );
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    // oracle: '/di' with the popup open: Enter selects the Diff command
    s.type_text("/di").keys("enter");
    s.keys("up");
    s.assert_row(prompt(&s), "› /diff");
}

// ORACLE bottom_pane/chat_composer.rs:12319 inline_slash_command_can_be_recalled_after_recording_pending_history kind=ui owner=chat-live
#[test]
fn inline_slash_command_can_be_recalled_after_recording_pending_history() {
    kit::pending(
        Owner::ChatLive,
        "no slash-command history on the chat composer: '/plan investigate this' submitted with arguments, then Up recalls it",
    );
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    // oracle: '/plan investigate this' with the popup closed: the Plan command with args
    s.type_text("/plan investigate this").keys("enter");
    s.keys("up");
    s.assert_row(prompt(&s), "› /plan investigate this");
}

// ORACLE bottom_pane/chat_composer.rs:12545 current_text_with_pending_expands_overlapping_placeholders kind=ui owner=chat-live
#[test]
fn current_text_with_pending_expands_overlapping_placeholders() {
    kit::pending(
        Owner::ChatLive,
        "no large-paste placeholder on the chat composer: two 1004-char pastes draw the base placeholder then '#2', and submit expands both",
    );
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
    kit::pending(
        Owner::ChatLive,
        "no remote image URLs on the chat composer: two remote images sit above the draft, Up selects one and Delete removes it, relabelling the local image",
    );
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    // oracle: two remote image URLs; a local image attached (label #3); the cursor to 0
    s.paste("/tmp/local.png");
    s.keys("home up delete");
    s.assert_row(prompt(&s), "› [Image #2]");
    // oracle: Up selects the last remote image, Delete removes it
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
    kit::pending(
        Owner::ChatLive,
        "no shutdown-in-progress state on the chat composer: the composer shows '› Shutting down...' with no footer and ignores input (the snapshot shutdown_in_progress at 40x5)",
    );
    // oracle snapshot shutdown_in_progress at 40x5: the draft 'hello', then show_shutdown_in_progress
    let mut s = Screen::build(Owner::ChatLive).size(40, 5).live().open();
    s.type_text("hello");
    s.assert_snap("shutdown_in_progress");
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
    kit::pending(
        Owner::ChatLive,
        "no agents command center on the chat composer: Left in a draft moves the cursor, and Left on an empty prompt opens the agents overview",
    );
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
}

// ORACLE bottom_pane/chat_composer/agents_navigation_tests.rs:82 left_respects_attachments_pastes_and_other_input_surfaces kind=ui owner=chat-live
#[test]
fn left_respects_attachments_pastes_and_other_input_surfaces() {
    kit::pending(
        Owner::ChatLive,
        "no agents command center on the chat composer: Left with text, a paste or a remote image does not open the agents overview, and the rows do not change",
    );
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
