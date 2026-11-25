//! Shard `composer_4` (screens S10, owner chat-live): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen};

/// A chat screen at `w` x `h`, the composer live and empty.
/// `global.toggle_vim_mode` is bound to F9 (it has no default key): the kit has no Vim seed, F9 turns Vim on.
fn chat(w: u16, h: u16) -> Screen {
    Screen::build(Owner::ChatLive)
        .size(w, h)
        .live()
        .keymap(&[("global.toggle_vim_mode", "f9")])
        .open()
}

/// The composer alone with its footer (the oracle's `snapshot_composer_state_with_width`), Vim
/// off; `global.toggle_vim_mode` is bound to F9, which turns Vim on in Normal mode.
fn vim_alone(w: u16, h: u16) -> Screen {
    Screen::build(Owner::ChatLive)
        .size(w, h)
        .keymap(&[("global.toggle_vim_mode", "f9")])
        .entry("composer", "alone")
        .open()
}

/// The oracle's `set_text_content(text)` then `set_vim_enabled(true)` on a composer whose Vim is on:
/// Vim goes off for the typing and on again, in Normal mode with a fresh history.
fn reset_vim_text(s: &mut Screen, text: &str) {
    s.keys("f9 ctrl+u");
    type_lines(s, text);
    s.keys("f9");
}

/// A chat whose connection is lost (`Reconnecting`): the composer only edits, and Enter runs
/// only what needs no connection (`RestrictedInputMode::Disconnected`).
fn offline(w: u16, h: u16) -> Screen {
    Screen::build(Owner::ChatLive)
        .size(w, h)
        .script(include_str!("fixtures/composer_4/offline.jsonl"))
        .open()
}

/// The composer alone with its footer, `running` or not (the oracle's composer
/// harness, `set_task_running`).
fn alone(w: u16, h: u16, running: bool) -> Screen {
    Screen::build(Owner::ChatLive)
        .size(w, h)
        .entry("composer", if running { "alone_running" } else { "alone" })
        .open()
}

/// The plain-text composer of the question sheets (`ChatComposerConfig::plain_text`) with its
/// footer, as the oracle's composer harness draws it: the debug entry `composer/plain`.
fn plain(w: u16, h: u16) -> Screen {
    Screen::build(Owner::ChatLive)
        .size(w, h)
        .entry("composer", "plain")
        .open()
}

/// A chat screen with a turn running (Tab queues the draft), the composer live.
fn chat_running(w: u16, h: u16) -> Screen {
    Screen::build(Owner::ChatLive)
        .size(w, h)
        .script(include_str!("fixtures/composer_2/running.jsonl"))
        .open()
}

/// The composer's prompt row (`›`), the last one: a popup sits above the composer.
fn prompt(s: &Screen) -> usize {
    s.rows()
        .iter()
        .rposition(|r| r.starts_with('›') || r.starts_with("! ") || r == "!")
        .unwrap_or(usize::MAX)
}

/// A row index as the row the mouse takes.
fn cell(row: usize) -> u16 {
    u16::try_from(row).unwrap_or(u16::MAX)
}

/// A double click inside the label `text` (on its fourth cell, not on the bracket): the
/// oracle double-clicks inside a placeholder, which selects the whole atomic element.
fn double_click_in(s: &mut Screen, text: &str) {
    let rows = s.rows();
    let Some((row, at)) = rows
        .iter()
        .enumerate()
        .find_map(|(row, r)| Some((row, r.find(text)?)))
    else {
        panic!("no `{text}` on the screen:\n{}", s.text());
    };
    // the column counts chars, not the bytes before the label
    let col = rows[row][..at].chars().count() + 3;
    s.double_click(u16::try_from(col).unwrap_or(u16::MAX), cell(row));
}

/// The empty composer's placeholder, as the oracle draws it.
const PLACEHOLDER: &str = "› Ask Codex to do anything";

/// The draft's rows from the prompt down: the first line after `› `, the others after two spaces.
/// A draft that begins with `!` is a shell command: the `!` is the prompt (`! ` and the rest).
fn draft_rows(s: &Screen, text: &str) {
    let p = prompt(s);
    for (i, line) in text.split('\n').enumerate() {
        let row = if i == 0 {
            match line.strip_prefix('!') {
                Some(command) => format!("! {command}"),
                None => format!("› {line}"),
            }
        } else {
            format!("  {line}")
        };
        s.assert_row(p + i, row.trim_end());
    }
}

/// Types a draft, one line per `alt+enter`: the oracle's newline is Ctrl+J, which rtwk's input
/// reads as Enter (LF is Enter, G66), so the draft's lines are broken with the chord rtwk tells apart.
fn type_lines(s: &mut Screen, text: &str) {
    for (i, line) in text.split('\n').enumerate() {
        if i > 0 {
            s.keys("alt+enter");
        }
        s.type_text(line);
    }
}

/// Submits `text` once, so that Up and Ctrl+R have history to show (the oracle's `record_local_submission`).
fn submit_once(s: &mut Screen, text: &str) {
    s.type_text(text);
    s.keys("enter");
}

/// The oracle's `vim_composer(text)`: the draft, Vim on, the caret on its first cell in Normal mode.
/// F9 turns Vim on (Normal mode) and `g g` goes to the first cell.
fn vim_draft(s: &mut Screen, text: &str) {
    type_lines(s, text);
    s.keys("f9 g g");
}

/// The oracle's `set_text_content(text)` in Vim's Normal mode: the draft is replaced and Normal
/// mode stays (`d$` leaves in the register the text a `y$` took of the same line, and `0` puts
/// the caret on the first cell, as the oracle's tests then do).
fn vim_replace_draft(s: &mut Screen, text: &str) {
    s.type_text("d$i");
    type_lines(s, text);
    s.keys("esc");
    s.type_text("0");
}

/// The oracle's `rich_composer()`: a `$skill` mention, an attached image, a 1001-character paste,
/// Normal mode on the first cell. The mention and the image have no kit seed yet.
fn rich_draft(s: &mut Screen) {
    s.type_text("$skill");
    s.paste(&"z".repeat(1001));
    s.keys("esc");
    s.type_text("0");
}

// ---- oracle: bottom_pane/chat_composer/blockquote_paste_tests.rs (7 screen-level tests here; 0 state tests left to chat-live)
// ORACLE bottom_pane/chat_composer/blockquote_paste_tests.rs:9 blockquote_paste_continues_current_line_prefix kind=ui owner=chat-live
#[test]
fn blockquote_paste_continues_current_line_prefix() {
    for (draft, pasted, expected) in [
        ("> ", "first\nsecond", "> first\n> second\n\n"),
        ("> ", "first\n", "> first\n\n"),
        ("intro\n> ", "α\r\n\r\nβ\r", "intro\n> α\n> \n> β\n\n"),
        (
            "> existing ",
            "first\nsecond",
            "> existing first\n> second\n\n",
        ),
        ("text > ", "first\nsecond", "text > first\nsecond"),
        (">", "first\nsecond", ">first\nsecond"),
        (
            "> quote\nplain ",
            "first\nsecond",
            "> quote\nplain first\nsecond",
        ),
        ("> ", "single", "> single"),
        ("!> ", "out\npwd", "!> out\npwd"),
        (" !echo hi\n> ", "out\npwd", " !echo hi\n> out\n> pwd\n\n"),
    ] {
        let mut s = chat(100, 24);
        type_lines(&mut s, draft);
        s.paste(pasted);
        draft_rows(&s, expected);
    }
}

// ORACLE bottom_pane/chat_composer/blockquote_paste_tests.rs:39 blockquote_paste_uses_cursor_line_and_preserves_suffix kind=ui owner=chat-live
#[test]
fn blockquote_paste_uses_cursor_line_and_preserves_suffix() {
    let mut s = chat(100, 24);
    type_lines(&mut s, "intro\n> suffix\nend");
    // the caret after `intro\n> `
    s.keys("up home right right");
    s.paste("first\nsecond");
    draft_rows(&s, "intro\n> first\n> second\n\nsuffix\nend");
    s.type_text("next ");
    draft_rows(&s, "intro\n> first\n> second\n\nnext suffix\nend");
}

// ORACLE bottom_pane/chat_composer/blockquote_paste_tests.rs:56 blockquote_paste_large_expands_quoted_text_on_submit kind=ui owner=chat-live
#[test]
fn blockquote_paste_large_expands_quoted_text_on_submit() {
    kit::pending(
        Owner::ChatLive,
        "the sent user message is drawn as Markdown, so `> x` / `> second` is one quote paragraph (the oracle prints the typed lines): the user message cell, chat-cells",
    );
    let mut s = chat(100, 24);
    s.type_text("> ");
    let first = "x".repeat(1000);
    s.paste(&format!("{first}\nsecond"));
    // the placeholder counts the quoted text: 1000 x, a newline, `> second`
    let p = prompt(&s);
    s.assert_row(p, "› > [Pasted Content 1009 chars]");
    s.type_text("Next block");
    s.keys("enter");
    s.assert_in_order(&["> second", "Next block"]);
}

// ORACLE bottom_pane/chat_composer/blockquote_paste_tests.rs:80 blockquote_paste_rendered kind=snap owner=chat-live snap=blockquote_paste
#[test]
fn blockquote_paste_rendered() {
    let mut s = alone(60, 9, false);
    type_lines(&mut s, "Please explain:\n> ");
    s.paste("first line\n\nlast line\n");
    s.type_text("Next block");
    s.assert_snap("blockquote_paste");
}

// ORACLE bottom_pane/chat_composer/blockquote_paste_tests.rs:94 blockquote_paste_replacing_selected_prefix_stays_literal kind=ui owner=chat-live
#[test]
fn blockquote_paste_replacing_selected_prefix_stays_literal() {
    let mut s = chat(100, 24);
    s.type_text("> selected");
    let p = prompt(&s);
    // the text starts after the `› ` prompt (column 2) and runs 10 columns
    s.drag(2, cell(p), 12, cell(p));
    s.paste("first\nsecond");
    draft_rows(&s, "first\nsecond");
}

// ORACLE bottom_pane/chat_composer/blockquote_paste_tests.rs:124 blockquote_paste_embedded_answer_starts_next_block kind=ui owner=chat-live
#[test]
fn blockquote_paste_embedded_answer_starts_next_block() {
    let mut s = plain(100, 24);
    s.type_text("> ");
    s.paste("first\nsecond");
    s.type_text("Next block");
    draft_rows(&s, "> first\n> second\n\nNext block");
}

// ORACLE bottom_pane/chat_composer/blockquote_paste_tests.rs:134 blockquote_paste_embedded_answer_rendered kind=snap owner=chat-live snap=blockquote_paste_embedded_answer
#[test]
fn blockquote_paste_embedded_answer_rendered() {
    let mut s = plain(60, 9);
    s.type_text("> ");
    s.paste("first line\n\nlast line");
    s.type_text("Next block");
    s.assert_snap("blockquote_paste_embedded_answer");
}

// ---- oracle: bottom_pane/chat_composer/embedded_input_tests.rs (4 screen-level tests here; 0 state tests left to chat-live)
// ORACLE bottom_pane/chat_composer/embedded_input_tests.rs:8 plain_text_composer_keeps_command_prefixes_literal kind=ui owner=chat-live
#[test]
fn plain_text_composer_keeps_command_prefixes_literal() {
    // the oracle submits each draft (Tab) and starts the next on an empty composer; the harness
    // keeps what is submitted, so each prefix gets its own composer (`?` once: the oracle's
    // Shift+`?` is the same character here). Tab as a submit is `composer_key`'s unit test.
    for prefix in ["!", "/", "?"] {
        let mut s = plain(100, 24);
        s.type_text(prefix);
        s.paste("keep this literal");
        s.keys("tab");
        // no shell prompt, no command popup, no shortcut overlay: the draft is text
        s.assert_row(prompt(&s), &format!("› {prefix}keep this literal"));
    }
}

// ORACLE bottom_pane/chat_composer/embedded_input_tests.rs:32 buffered_typing_survives_vim_changes_and_cancellation kind=ui owner=chat-live
#[test]
fn buffered_typing_survives_vim_changes_and_cancellation() {
    let mut s = chat(100, 24);
    s.type_text("x");
    // Vim on: the typed `x` is in the draft
    s.keys("f9");
    s.assert_row(prompt(&s), "› x");
    // Vim off, the draft goes on from its end
    s.keys("f9");
    s.type_text("y");
    s.assert_row(prompt(&s), "› xy");
    // Ctrl+C clears the draft whole and nothing is held back; Up recalls it
    s.keys("ctrl+c");
    s.assert_row(prompt(&s), PLACEHOLDER);
    s.keys("up");
    s.assert_row(prompt(&s), "› xy");
}

// ORACLE bottom_pane/chat_composer/embedded_input_tests.rs:46 paste_enter_retains_a_one_character_first_line kind=ui owner=chat-live
#[test]
fn paste_enter_retains_a_one_character_first_line() {
    let mut s = chat(100, 24);
    // the Enter comes while the `x` is still held: it joins it as a newline
    s.type_burst("x");
    s.keys("enter");
    draft_rows(&s, "x\n");
}

// ORACLE bottom_pane/chat_composer/embedded_input_tests.rs:54 embedded_composer_shows_validation_error_without_losing_draft kind=snap owner=chat-live
#[test]
fn embedded_composer_shows_validation_error_without_losing_draft() {
    let mut s = plain(80, 24);
    let text = "x".repeat(1_048_577);
    s.paste(&text);
    s.keys("enter");
    // the footer's flash says it, the draft stays in the composer (a large paste is its placeholder)
    s.assert_row(
        s.rows().len() - 1,
        "  Message too long; limit 1048576 characters",
    );
    s.assert_row(prompt(&s), "› [Pasted Content 1048577 chars]");
}

// ---- oracle: bottom_pane/chat_composer/mouse_tests.rs (7 screen-level tests here; 0 state tests left to chat-live)
// ORACLE bottom_pane/chat_composer/mouse_tests.rs:24 composer_mouse_edits_flush_typing_and_reconcile_placeholders kind=ui owner=chat-live
#[test]
fn composer_mouse_edits_flush_typing_and_reconcile_placeholders() {
    let mut s = chat(80, 24);
    s.type_text("hello");
    s.paste("\nworld");
    let p = prompt(&s);
    // a click on the first `h` moves the caret there
    s.click(2, cell(p));
    s.type_text("X");
    draft_rows(&s, "Xhello\nworld");
    s.paste(&"z".repeat(1001));
    s.assert_contains("[Pasted Content 1001 chars]");
    // a double click inside a placeholder selects the whole atomic element; the next edit
    // replaces it (an image) or takes it away with the pasted content it stood for (a letter)
    double_click_in(&mut s, "[Pasted Content 1001 chars]");
    s.attach_image("first.png");
    s.assert_absent("[Pasted Content");
    draft_rows(&s, "X[Image #1]hello\nworld");
    double_click_in(&mut s, "[Image #1]");
    s.attach_image("second.png");
    draft_rows(&s, "X[Image #1]hello\nworld");
    double_click_in(&mut s, "[Image #1]");
    s.type_text("\u{e9}");
    draft_rows(&s, "X\u{e9}hello\nworld");
    // nothing is attached any more: the draft sends no image
    s.keys("enter");
    s.wait_for("› Ask Codex to do anything");
    s.assert_sole_turn(kit::Turn::text("X\u{e9}hello\nworld").no_images());
}

// ORACLE bottom_pane/chat_composer/mouse_tests.rs:62 recovery_keeps_unselected_text_after_a_held_replace_key kind=ui owner=chat-live
#[test]
fn recovery_keeps_unselected_text_after_a_held_replace_key() {
    for (text, expected) in [("abcd", "aXd"), ("!abcd", "!aXd")] {
        let mut s = chat(80, 24);
        s.type_text(text);
        s.keys("f9");
        s.type_text("0R");
        let p = prompt(&s);
        // the caret is on the first cell; the drag runs from one cell past it to three
        s.drag(3, cell(p), 5, cell(p));
        s.type_text("X");
        draft_rows(&s, expected);
    }
}

// ORACLE bottom_pane/chat_composer/mouse_tests.rs:92 backspace_deletes_a_body_selection_before_an_absorbed_prefix kind=ui owner=chat-live
#[test]
fn backspace_deletes_a_body_selection_before_an_absorbed_prefix() {
    let mut s = chat(80, 24);
    s.type_text("!review text");
    let p = prompt(&s);
    // the `!` is the prompt: the body `review text` runs from column 2 to
    // the caret's column 13 (the oracle drags from the caret leftwards)
    s.drag(13, cell(p), 2, cell(p));
    s.keys("backspace");
    // the oracle's `current_text` is `!`: the prompt alone
    s.assert_row(prompt(&s), "!");
}

// ORACLE bottom_pane/chat_composer/mouse_tests.rs:115 selected_text_navigation_does_not_recall_history_or_select_images kind=ui owner=chat-live
#[test]
fn selected_text_navigation_does_not_recall_history_or_select_images() {
    for (key, from, to) in [("up", 8, 2), ("down", 2, 8)] {
        let mut s = chat(80, 24);
        submit_once(&mut s, "first");
        submit_once(&mut s, "second");
        s.keys("up");
        let p = prompt(&s);
        s.drag(from, cell(p), to, cell(p));
        s.keys(key);
        s.assert_row(prompt(&s), "› second");
    }
}

// ORACLE bottom_pane/chat_composer/mouse_tests.rs:166 right_click_copy_uses_the_refreshed_editor_bounds kind=ui owner=chat-live
#[test]
fn right_click_copy_uses_the_refreshed_editor_bounds() {
    let mut s = chat(40, 24);
    s.type_text("hello world");
    let p = prompt(&s);
    s.drag(2, cell(p), 7, cell(p));
    // the right click inside the selection copies `hello`; the draft stays as typed
    s.right_click(4, cell(p));
    s.assert_row(prompt(&s), "› hello world");
}

// ORACLE bottom_pane/chat_composer/mouse_tests.rs:244 selection_release_copies_and_keeps_the_editable_selection kind=ui owner=chat-live
#[test]
fn selection_release_copies_and_keeps_the_editable_selection() {
    let mut s = chat(40, 24);
    s.type_text("hello world");
    let p = prompt(&s);
    s.drag(2, cell(p), 7, cell(p));
    // the selection stays highlighted after the release: five cells reversed, the next not
    s.assert_style(2, p, "reversed");
    s.assert_style(6, p, "reversed");
    s.assert_style(7, p, "!reversed");
    s.type_text("X");
    s.assert_row(prompt(&s), "› X world");
}

// ORACLE bottom_pane/chat_composer/mouse_tests.rs:302 selection_release_reports_only_clipboard_result kind=ui owner=chat-live
#[test]
fn selection_release_reports_only_clipboard_result() {
    kit::na(
        Owner::ChatLive,
        "state test: the oracle's `copy_selection_on_release` and `finish_copy` with a mocked clipboard closure and a `CopyStatus`; no screen shows the result, so the screen half is the highlight (as the test above)",
    );
    let mut s = chat(40, 24);
    s.type_text("hello world");
    let p = prompt(&s);
    s.drag(2, cell(p), 7, cell(p));
    // the clipboard result (Pending(7)) is not drawn; the selection stays until the copy finishes
    s.assert_style(2, p, "reversed");
}

// ---- oracle: bottom_pane/chat_composer/paste_tests.rs (6 screen-level tests here; 0 state tests left to chat-live)
// ORACLE bottom_pane/chat_composer/paste_tests.rs:11 pasting_fifo_path_keeps_it_editable kind=snap owner=chat-live
#[test]
fn pasting_fifo_path_keeps_it_editable() {
    let mut s = chat(100, 24);
    s.paste("/tmp/fifo-dir/tty.usbserial-110");
    s.keys("backspace");
    s.assert_row_ends_with(prompt(&s), "/tty.usbserial-11");
}

// ORACLE bottom_pane/chat_composer/paste_tests.rs:31 paste_burst_tabs_preserve_multiline_draft kind=snap owner=chat-live snap=codex_tui__bottom_pane__chat_composer__paste_tests__paste_burst_tab_indentation
#[test]
fn paste_burst_tabs_preserve_multiline_draft() {
    for running in [false, true] {
        for first_line in ["x", "界", "first line\n"] {
            let snapshot_case = running && first_line == "first line\n";
            let (w, h) = if snapshot_case { (60, 8) } else { (100, 24) };
            let mut s = alone(w, h, running);
            let payload = format!("{first_line}\t\tsecond line\n\tthird line\n");
            s.paste(&payload);
            if snapshot_case {
                s.assert_snap("paste_burst_tab_indentation");
            }
            s.keys("tab");
            s.assert_contains("second line");
        }
    }
}

// ORACLE bottom_pane/chat_composer/paste_tests.rs:84 paste_burst_tabs_refresh_idle_timeout kind=ui owner=chat-live
#[test]
fn paste_burst_tabs_refresh_idle_timeout() {
    for first_char in ["x", "界"] {
        let mut s = chat(100, 24);
        // the tabs come inside the burst's window: literal tabs, not a submit
        s.type_burst(&format!("{first_char}\t\t\t\t"));
        s.wait(50);
        s.assert_row_starts_with(prompt(&s), &format!("› {first_char}"));
    }
}

// ORACLE bottom_pane/chat_composer/paste_tests.rs:106 paste_burst_tab_does_not_accept_a_completion kind=ui owner=chat-live
#[test]
fn paste_burst_tab_does_not_accept_a_completion() {
    let mut s = chat(100, 24);
    s.type_text("/");
    // the paste arrives as fast keys, the Tab among them
    s.type_burst("review this\t");
    s.wait(100);
    // the Tab is a literal tab (`/review this\t`): the completion is not accepted
    s.assert_row_starts_with(prompt(&s), "› /review this");
}

// ORACLE bottom_pane/chat_composer/paste_tests.rs:128 paste_burst_expired_before_tab_still_queues kind=ui owner=chat-live
#[test]
fn paste_burst_expired_before_tab_still_queues() {
    let mut s = chat_running(100, 24);
    s.type_text("x");
    s.keys("tab");
    // queued, not typed: the composer is empty again
    s.assert_row(prompt(&s), PLACEHOLDER);
}

// ORACLE bottom_pane/chat_composer/paste_tests.rs:150 paste_burst_modified_queue_binding_still_dispatches kind=ui owner=chat-live
#[test]
fn paste_burst_modified_queue_binding_still_dispatches() {
    kit::pending(
        Owner::ChatLive,
        "no rebindable queue key: the default keymap queues on Tab only; Ctrl+Q (`queue_keys` override) is the keymap task's",
    );
    let mut s = chat_running(100, 24);
    s.type_text("x");
    s.keys("ctrl+q");
    s.assert_row(prompt(&s), PLACEHOLDER);
}

// ---- oracle: bottom_pane/chat_composer/reconnect_tests.rs (6 screen-level tests here; 5 state tests left to chat-live)
// ORACLE bottom_pane/chat_composer/reconnect_tests.rs:10 draft_handoff_preserves_newline_after_short_unicode_prefix kind=snap owner=chat-live
#[test]
fn draft_handoff_preserves_newline_after_short_unicode_prefix() {
    let mut s = chat(100, 24);
    s.type_text("猫");
    s.keys("alt+enter");
    s.type_text("犬");
    draft_rows(&s, "猫\n犬");
    let mut s = chat(100, 24);
    s.type_text("猫");
    s.keys("enter");
    s.assert_contains("猫");
}

// ORACLE bottom_pane/chat_composer/reconnect_tests.rs:31 handoff_edit_discards_stale_command_before_ordinary_enter kind=ui owner=chat-live
#[test]
fn handoff_edit_discards_stale_command_before_ordinary_enter() {
    for bracketed_paste in [true, false] {
        let mut s = chat(100, 24);
        if bracketed_paste {
            s.paste("/quit");
        } else {
            s.type_text("/quit");
        }
        s.assert_contains("/quit");
        s.keys("ctrl+u");
        s.assert_row(prompt(&s), PLACEHOLDER);
        // a failed handoff returns to ordinary input: Enter on the empty composer does nothing
        s.keys("enter");
        s.assert_row(prompt(&s), PLACEHOLDER);
    }
}

// ORACLE bottom_pane/chat_composer/reconnect_tests.rs:60 offline_delete_reveals_the_edited_row_without_moving_the_caret kind=snap owner=chat-live snap=offline_delete_after_wheel
#[test]
fn offline_delete_reveals_the_edited_row_without_moving_the_caret() {
    kit::pending(
        Owner::ChatLive,
        "screen: the page draws its `Reconnecting…` row above and the footer under the composer (the oracle's snapshot is the composer alone at 40x8), and a composer scrolled by the wheel shows no `↑` cue at the end of its first row; the offline Delete itself acts (tests/pages/composer_draft.rs)",
    );
    let mut s = offline(40, 8);
    let text = (1..=20)
        .map(|n| format!("line {n:02} abc"))
        .collect::<Vec<_>>()
        .join("\n");
    type_lines(&mut s, &text);
    // the caret before the `b` of the last line
    s.keys("left left");
    let row = s
        .rows()
        .iter()
        .position(|r| r.contains("line 20 abc"))
        .unwrap_or(0);
    s.wheel_up(2, cell(row), 3);
    s.keys("delete");
    s.assert_snap("offline_delete_after_wheel");
}

// ORACLE bottom_pane/chat_composer/reconnect_tests.rs:116 reconnect_expands_pastes_preserving_images_and_cursor kind=ui owner=chat-live
#[test]
fn reconnect_expands_pastes_preserving_images_and_cursor() {
    let mut s = offline(100, 24);
    s.paste(&"x".repeat(1001));
    s.attach_image("local.png");
    s.paste(&"x".repeat(1001));
    s.type_text("!");
    s.assert_contains("x!");
}

// ORACLE bottom_pane/chat_composer/reconnect_tests.rs:147 reconnect_edit_removes_deleted_attachment_and_paste_metadata kind=ui owner=chat-live
#[test]
fn reconnect_edit_removes_deleted_attachment_and_paste_metadata() {
    let mut s = offline(100, 24);
    s.attach_image("local.png");
    s.paste(&"x".repeat(1001));
    s.keys("ctrl+u");
    s.assert_row(prompt(&s), PLACEHOLDER);
    s.assert_absent("[Pasted Content");
    s.assert_absent("[Image #1]");
}

// ORACLE bottom_pane/chat_composer/reconnect_tests.rs:297 unavailable_thread_expands_pasted_commands_without_dispatching kind=ui owner=chat-live
#[test]
fn unavailable_thread_expands_pasted_commands_without_dispatching() {
    kit::pending(
        Owner::ChatLive,
        "no unavailable-thread composer on the chat screen (no seed): a `/clear` with a large paste is drawn as a placeholder, and Enter does not run it",
    );
    let mut s = chat(100, 24);
    s.paste(&format!("/clear {}", "x".repeat(1001)));
    s.keys("enter");
    s.assert_row_starts_with(prompt(&s), "› /clear [Pasted Content 1008 chars]");
}

// ---- oracle: bottom_pane/chat_composer/snapshot_tests.rs (3 screen-level tests here; 0 state tests left to chat-live)
// ORACLE bottom_pane/chat_composer/snapshot_tests.rs:21 plugin_mention_uses_display_name_for_opaque_id kind=snap owner=chat-live snap=plugin_mention_uses_display_name_for_opaque_id
#[test]
fn plugin_mention_uses_display_name_for_opaque_id() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(100, 9)
        .entry("popovers", "alone_plugin_opaque")
        .open();
    s.type_text("@Postm");
    s.keys("tab");
    s.assert_snap("plugin_mention_uses_display_name_for_opaque_id");
}

// ORACLE bottom_pane/chat_composer/snapshot_tests.rs:49 shortcut_footer_displays_configured_chords kind=snap owner=chat-live snap=footer_mode_configured_key_chords
#[test]
fn shortcut_footer_displays_configured_chords() {
    kit::pending(
        Owner::ChatLive,
        "no keymap seed on the chat screen: the configured chords (ctrl-g ctrl-g, ctrl-x enter) are not read, so the shortcut overlay draws the defaults",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(100, 21)
        .live()
        .keymap(&[
            ("global.open_external_editor", "ctrl+g ctrl+g"),
            ("editor.insert_newline", "ctrl+x enter"),
        ])
        .open();
    s.type_text("?");
    s.assert_snap("footer_mode_configured_key_chords");
}

// ORACLE bottom_pane/chat_composer/snapshot_tests.rs:94 draft_and_voice_composer_snapshots kind=snap owner=chat-live snap=codex_tui__bottom_pane__chat_composer__snapshot_tests__draft_composer snap=codex_tui__bottom_pane__chat_composer__snapshot_tests__voice_composer
#[test]
fn draft_and_voice_composer_snapshots() {
    kit::pending(
        Owner::ChatLive,
        "the oracle's snapshots are the composer alone at 60 columns, and no voice strip on the chat composer (no seed for set_voice_strip)",
    );
    let mut s = chat(60, 4);
    s.type_text("Explore the night sky");
    s.assert_snap("draft_composer");
    let s = chat(60, 6);
    s.assert_snap("voice_composer");
}

// ---- oracle: bottom_pane/chat_composer/vim_history_tests.rs (22 screen-level tests here; 9 state tests left to chat-live)
// ORACLE bottom_pane/chat_composer/vim_history_tests.rs:83 recovered_answers_undo_as_one_vim_edit kind=snap owner=chat-live snap=recovered_answer_cancels_vim_search
#[test]
fn recovered_answers_undo_as_one_vim_edit() {
    kit::pending(
        Owner::ChatLive,
        "Vim is built (tests/pages/composer_vim.rs) but the kit seeds no Vim state for this body and no recovered-answer append (append_recovered_drafts has no seed): the answer after a failed turn is not joined to the draft",
    );
    let mut s = chat(40, 9);
    vim_draft(&mut s, "abc");
    s.type_text("iX");
    s.keys("esc");
    s.assert_row(prompt(&s), "› Xabc");
    let mut s = chat(40, 9);
    vim_draft(&mut s, "existing draft");
    s.type_text("d/draft");
    // append_recovered_drafts("answer"): no kit call yet
    s.assert_snap("recovered_answer_cancels_vim_search");
}

// ORACLE bottom_pane/chat_composer/vim_history_tests.rs:209 undo_and_redo_preserve_confirmed_vim_search kind=ui owner=chat-live
#[test]
fn undo_and_redo_preserve_confirmed_vim_search() {
    let mut s = chat(100, 24);
    vim_draft(&mut s, "alpha beta alpha");
    s.type_text("/alpha");
    s.keys("enter");
    s.type_text("0xun");
    let p = prompt(&s);
    s.assert_row(p, "› alpha beta alpha");
    // the caret on the last `alpha` (offset 11)
    s.assert_cursor(13, p);
    s.type_text("0");
    s.paste("Z");
    s.type_text("n");
    s.assert_row(p, "› Zalpha beta alpha");
    s.assert_cursor(14, p);
    s.type_text("u");
    s.keys("ctrl+r");
    s.type_text("n");
    s.assert_row(p, "› Zalpha beta alpha");
    s.assert_cursor(3, p);
}

// ORACLE bottom_pane/chat_composer/vim_history_tests.rs:245 normal_mode_explicit_paste_is_its_own_edit_and_replaces_stale_redo kind=ui owner=chat-live
#[test]
fn normal_mode_explicit_paste_is_its_own_edit_and_replaces_stale_redo() {
    let mut s = chat(100, 24);
    vim_draft(&mut s, "abc");
    s.type_text("x");
    s.paste("Z");
    let p = prompt(&s);
    s.assert_row(p, "› Zbc");
    s.type_text("u");
    s.assert_row(p, "› bc");
    s.type_text("u");
    s.assert_row(p, "› abc");
    s.paste("Z");
    s.assert_row(p, "› Zabc");
    s.keys("ctrl+r");
    s.assert_row(p, "› Zabc");
}

// ORACLE bottom_pane/chat_composer/vim_history_tests.rs:300 deleting_a_selected_remote_image_is_one_undoable_vim_edit kind=ui owner=chat-live
#[test]
fn deleting_a_selected_remote_image_is_one_undoable_vim_edit() {
    kit::pending(
        Owner::ChatLive,
        "the kit toggles Vim with a bound key (F9) but this body is not written for it, and no seed for set_remote_image_urls: the port selects and deletes the row as one Vim edit (tests/pages/composer_images.rs, composer/editing.rs)",
    );
    let mut s = chat(100, 24);
    vim_draft(&mut s, "abc");
    s.type_text("xu");
    s.assert_row(prompt(&s), "› abc");
    s.keys("up");
    s.keys("delete");
    s.type_text("u");
    s.assert_row(prompt(&s), "› abc");
}

// ORACLE bottom_pane/chat_composer/vim_history_tests.rs:322 direct_edits_join_the_active_vim_insert_transaction kind=ui owner=chat-live
#[test]
fn direct_edits_join_the_active_vim_insert_transaction() {
    let mut s = chat(100, 24);
    // `vim_composer("abc")`: Normal mode on the first cell; `enable_vim_in_insert_mode`
    vim_draft(&mut s, "abc");
    s.keys("i");
    s.paste("ABC");
    s.attach_image("example.png");
    s.type_text("D");
    s.keys("esc");
    let p = prompt(&s);
    s.assert_row(p, "› ABC[Image #1]Dabc");
    // one transaction: a single `u` takes the paste, the image and the typing back, and Ctrl+R
    // puts them all back
    s.type_text("u");
    s.assert_row(p, "› abc");
    s.assert_absent("[Image #1]");
    s.keys("ctrl+r");
    s.assert_row(p, "› ABC[Image #1]Dabc");
}

// ORACLE bottom_pane/chat_composer/vim_history_tests.rs:339 vim_search_query_paste_does_not_invalidate_draft_redo kind=ui owner=chat-live
#[test]
fn vim_search_query_paste_does_not_invalidate_draft_redo() {
    kit::pending(
        Owner::ChatLive,
        "Vim is built (tests/pages/composer_vim.rs) but the kit seeds no Vim state for this body, and no Vim search prompt on the chat composer: `/` opens no query line, a paste into it is not drawn, and redo is not seeded",
    );
    let mut s = chat(100, 24);
    vim_draft(&mut s, "alpha beta alpha");
    s.type_text("xu/");
    s.paste("beta");
    let p = prompt(&s);
    s.assert_row(p, "› alpha beta alpha");
    s.assert_contains("/beta");
    s.keys("esc");
    s.keys("ctrl+r");
    s.assert_row(p, "› lpha beta alpha");
}

// ORACLE bottom_pane/chat_composer/vim_history_tests.rs:356 vim_redo_keeps_its_binding_when_history_is_empty kind=snap owner=chat-live snap=vim_empty_redo_stays_normal
#[test]
fn vim_redo_keeps_its_binding_when_history_is_empty() {
    let mut s = vim_alone(60, 9);
    type_lines(&mut s, "abc");
    s.keys("f9 g g");
    let before = s.shot();
    // no history to search: Ctrl+R is redo, which has nothing to redo
    s.keys("ctrl+r");
    s.assert_same_rows(&before);
    s.type_text("xu");
    s.assert_row(prompt(&s), "› abc");
    s.keys("ctrl+r");
    s.assert_row(prompt(&s), "› bc");
    let after = s.shot();
    s.keys("ctrl+r");
    s.assert_same_rows(&after);
    s.assert_text_style("Vim: Normal", "fg=Magenta");
    s.assert_snap("vim_empty_redo_stays_normal");
}

// ORACLE bottom_pane/chat_composer/vim_history_tests.rs:415 removing_the_shell_prefix_is_its_own_undoable_edit kind=ui owner=chat-live
#[test]
fn removing_the_shell_prefix_is_its_own_undoable_edit() {
    let mut s = chat(100, 24);
    s.type_text("!foo");
    s.keys("f9");
    s.type_text("0");
    s.keys("backspace");
    s.assert_row(prompt(&s), "› foo");
    // the oracle compares the whole draft (`draft_snapshot() == before`): shell
    // mode with "foo", drawn with the `!` prompt
    s.type_text("u");
    s.assert_row(prompt(&s), "! foo");
}

// ORACLE bottom_pane/chat_composer/vim_history_tests.rs:451 canceled_history_preview_preserves_active_vim_commands kind=ui owner=chat-live
#[test]
fn canceled_history_preview_preserves_active_vim_commands() {
    kit::pending(
        Owner::ChatLive,
        "Vim is built (tests/pages/composer_vim.rs) but the kit seeds no Vim state for this body, and no reverse history search (Ctrl+R) on the chat composer: the preview and its cancel are not drawn",
    );
    for query in ["", "archive", "missing"] {
        for (command, edited) in [
            ("iX", "XYalpha beta"),
            ("cwX", "XY beta"),
            ("RX", "XYpha beta"),
        ] {
            let mut s = chat(100, 24);
            submit_once(&mut s, "archived prompt");
            vim_draft(&mut s, "alpha beta");
            s.type_text(command);
            s.keys("ctrl+r");
            s.type_text(query);
            s.keys("esc");
            if command == "RX" {
                s.keys("backspace");
                s.type_text("X");
            }
            s.type_text("Y");
            s.keys("esc");
            s.assert_row(prompt(&s), &format!("› {edited}"));
            s.type_text("u.");
            s.assert_row(prompt(&s), &format!("› {edited}"));
        }
    }
}

// ORACLE bottom_pane/chat_composer/vim_history_tests.rs:497 undo_and_redo_use_configured_bindings_and_support_unbinding kind=ui owner=chat-live
#[test]
fn undo_and_redo_use_configured_bindings_and_support_unbinding() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(100, 24)
        .live()
        .keymap(&[
            ("vim_normal.undo", "z"),
            ("vim_normal.redo", "v"),
            ("global.toggle_vim_mode", "f9"),
        ])
        .open();
    vim_draft(&mut s, "abc");
    s.type_text("xu");
    s.assert_row(prompt(&s), "› bc");
    s.type_text("z");
    s.assert_row(prompt(&s), "› abc");
    s.type_text("v");
    s.assert_row(prompt(&s), "› bc");
    // the unbinding and the Ctrl+R bindings of the oracle follow the same pattern
}

// ORACLE bottom_pane/chat_composer/vim_history_tests.rs:561 undo_and_redo_restore_deleted_large_paste_payload kind=ui owner=chat-live
#[test]
fn undo_and_redo_restore_deleted_large_paste_payload() {
    let mut s = chat(100, 24);
    s.keys("f9");
    s.type_text("i");
    s.type_text("alpha ");
    // the oracle sets the caret after "alpha " and presses `i`: `A` is that
    // (Esc leaves the caret on the space)
    s.keys("esc");
    s.type_text("A");
    s.paste(&"x".repeat(1001));
    s.keys("esc");
    let p = prompt(&s);
    s.type_text("x");
    s.assert_row(p, "› alpha");
    s.assert_absent("[Pasted Content");
    s.type_text("u");
    s.assert_row(p, "› alpha [Pasted Content 1001 chars]");
    s.keys("ctrl+r");
    s.assert_row(p, "› alpha");
}

// ORACLE bottom_pane/chat_composer/vim_history_tests.rs:584 undo_and_redo_restore_deleted_image_attachment kind=ui owner=chat-live
#[test]
fn undo_and_redo_restore_deleted_image_attachment() {
    kit::pending(
        Owner::ChatLive,
        "Vim is built (tests/pages/composer_vim.rs) but the kit seeds no Vim state for this body, and no kit seed for remote image URLs (the page draws the rows, tests/pages/composer_images.rs): the `[Image #1]` placeholder is not drawn and Vim undo does not restore it",
    );
    let mut s = chat(100, 24);
    s.type_text("alpha ");
    // attach_image(example.png) at the caret: no kit call yet
    let p = prompt(&s);
    s.assert_row(p, "› alpha [Image #1]");
    s.keys("esc");
    s.type_text("x");
    s.assert_row(p, "› alpha");
    s.type_text("u");
    s.assert_row(p, "› alpha [Image #1]");
    s.keys("ctrl+r");
    s.assert_row(p, "› alpha");
}

// ORACLE bottom_pane/chat_composer/vim_history_tests.rs:614 vim_replace_yank_preserves_existing_image_and_recovers_atomically kind=ui owner=chat-live
#[test]
fn vim_replace_yank_preserves_existing_image_and_recovers_atomically() {
    let mut s = chat(100, 24);
    s.type_text("abcdefghij");
    s.keys("esc");
    s.type_text("0R");
    s.keys("ctrl+y");
    s.keys("backspace");
    s.assert_contains("abcdefghij");
}

// ORACLE bottom_pane/chat_composer/vim_history_tests.rs:680 vim_replace_preserves_slash_completion_and_dispatch kind=ui owner=chat-live
#[test]
fn vim_replace_preserves_slash_completion_and_dispatch() {
    let mut s = chat(100, 24);
    s.keys("f9");
    s.type_text("R/dif");
    s.keys("tab");
    s.assert_row(prompt(&s), "› /diff");
    // Enter runs /diff (InputResult::Command(Diff)): the diff screen is not a composer assertion
}

// ORACLE bottom_pane/chat_composer/vim_history_tests.rs:690 vim_replace_backspace_restores_characters_in_new_slash_element kind=snap owner=chat-live snap=vim_replace_slash_recovery
#[test]
fn vim_replace_backspace_restores_characters_in_new_slash_element() {
    let mut s = chat(100, 9);
    s.type_text("abcde tail");
    s.keys("f9");
    s.type_text("0R/diff");
    s.keys("backspace");
    s.keys("backspace");
    let p = prompt(&s);
    s.assert_row(p, "› /dide tail");
    s.assert_cursor(5, p);
    // the oracle snapshot is the composer alone: here the page adds the slash popup and the footer
    // under it, so the rows and the caret are what is compared
}

// ORACLE bottom_pane/chat_composer/vim_history_tests.rs:714 vim_replace_delete_backward_restores_characters_hidden_by_prompt_prefixes kind=ui owner=chat-live
#[test]
fn vim_replace_delete_backward_restores_characters_hidden_by_prompt_prefixes() {
    let mut s = chat(100, 24);
    vim_draft(&mut s, "ébc");
    s.type_text("R!");
    s.keys("ctrl+h");
    let p = prompt(&s);
    s.assert_row(p, "› ébc");
    s.assert_cursor(2, p);
}

// ORACLE bottom_pane/chat_composer/vim_history_tests.rs:725 vim_editing_modes_use_the_same_large_and_image_paste_handling kind=ui owner=chat-live
#[test]
fn vim_editing_modes_use_the_same_large_and_image_paste_handling() {
    kit::pending(
        Owner::ChatLive,
        "Vim is built (tests/pages/composer_vim.rs) but the kit seeds no Vim state for this body; the image-path paste needs a file on disk (no kit call writes one)",
    );
    let big = "x".repeat(1001);
    let mut insert = chat(100, 24);
    insert.type_text("next");
    insert.keys("esc");
    insert.type_text("i");
    insert.paste(&big);
    let shown = insert.shot();
    let mut replace = chat(100, 24);
    replace.type_text("next");
    replace.keys("esc");
    replace.type_text("R");
    replace.paste(&big);
    replace.assert_same_rows(&shown);
}

// ORACLE bottom_pane/chat_composer/vim_history_tests.rs:746 vim_replace_paste_bursts_preserve_overwritten_text_and_repeat kind=ui owner=chat-live
#[test]
fn vim_replace_paste_bursts_preserve_overwritten_text_and_repeat() {
    for pasted in ["XYZ", "日\u{3000}語"] {
        let mut s = chat(100, 24);
        vim_draft(&mut s, "abcd");
        s.type_text("R");
        s.type_text(pasted);
        let p = prompt(&s);
        s.assert_row(p, &format!("› {pasted}d"));
        s.keys("esc");
        s.type_text("u.");
        s.assert_row(p, &format!("› {pasted}d"));
    }
}

// ORACLE bottom_pane/chat_composer/vim_history_tests.rs:765 vim_replace_completion_preserves_suffix kind=snap owner=chat-live snap=vim_replace_completion_suffix
#[test]
fn vim_replace_completion_preserves_suffix() {
    kit::pending(
        Owner::ChatLive,
        "no Vim Replace mode and no file-path completion on the chat composer (insert_selected_file_path has no kit seed); the snapshot is the composer alone at 100 columns",
    );
    let mut s = chat(100, 9);
    s.type_text("abc next");
    s.keys("esc");
    s.type_text("0R@ma");
    // the completion of `@ma` to the path `ma` (insert_selected_file_path): no kit call yet
    s.keys("backspace");
    s.assert_row(prompt(&s), "› ma next");
    s.assert_snap("vim_replace_completion_suffix");
}

// ORACLE bottom_pane/chat_composer/vim_history_tests.rs:797 external_editor_import_preserves_replace_backspace_recovery kind=ui owner=chat-live
#[test]
fn external_editor_import_preserves_replace_backspace_recovery() {
    let mut s = chat(100, 24);
    s.type_text("old");
    s.keys("f9");
    s.type_text("0RX");
    // Ctrl+G sends the draft to the editor, which returns `abc`
    s.keys("ctrl+g");
    s.answer_editor("Xld", "abc");
    s.assert_row(prompt(&s), "› abc");
    // the caret is after its first cell: Backspace deletes `a`, nothing of the old Replace is put back
    s.keys("home right backspace");
    s.assert_row(prompt(&s), "› bc");
    s.type_text("X");
    s.keys("backspace");
    s.assert_row(prompt(&s), "› bc");
    s.assert_contains("Replace");
}

// ORACLE bottom_pane/chat_composer/vim_history_tests.rs:829 cancelled_history_search_preserves_literal_bang_draft kind=ui owner=chat-live
#[test]
fn cancelled_history_search_preserves_literal_bang_draft() {
    let mut s = chat(100, 24);
    s.type_text("x!echo hello");
    s.keys("home delete");
    s.assert_row(prompt(&s), "› !echo hello");
    s.keys("ctrl+r");
    s.keys("esc");
    s.assert_row(prompt(&s), "› !echo hello");
    s.keys("enter");
    s.assert_row(prompt(&s), PLACEHOLDER);
}

// ORACLE bottom_pane/chat_composer/vim_history_tests.rs:851 editor_actions_insert_shell_prefixes kind=ui owner=chat-live
#[test]
fn editor_actions_insert_shell_prefixes() {
    let mut s = chat(100, 24);
    s.type_text("x!echo hi");
    s.keys("home right ctrl+k backspace ctrl+y");
    // the yank at the start enters shell mode: `!` is the prompt
    s.assert_row(prompt(&s), "! echo hi");
    // Vim: `r!` and `.` put a `!` at the start of the draft, which is shell mode too
    for (commands, expected) in [("r!", "! xecho hi"), ("lr!0.", "! !echo hi")] {
        let mut s = chat(100, 24);
        vim_draft(&mut s, "xxecho hi");
        s.type_text(commands);
        s.assert_row(prompt(&s), expected);
    }
}

// ---- oracle: bottom_pane/chat_composer/vim_register_tests.rs (11 screen-level tests here; 4 state tests left to chat-live)
// ORACLE bottom_pane/chat_composer/vim_register_tests.rs:97 composer_handoff_preserves_only_the_live_rich_register kind=ui owner=chat-live
#[test]
fn composer_handoff_preserves_only_the_live_rich_register() {
    // the oracle's `rich_composer()`: a `$figma` skill mention, an attached image, a 1001-character
    // paste, in Normal mode
    let mut s = Screen::build(Owner::ChatLive)
        .size(100, 24)
        .keymap(&[("global.toggle_vim_mode", "f9")])
        .script(include_str!("fixtures/composer_3/skill_figma.jsonl"))
        .open();
    s.keys("f9 i");
    s.type_text("$figma").keys("tab");
    let photo = s.attach_image("photo.png");
    s.paste(&"z".repeat(1001));
    s.keys("esc");
    s.type_text("0y$A");
    // Ctrl+Y puts the yank in once more; `yy` then takes the whole line, rich objects and all
    s.keys("ctrl+y esc");
    s.type_text("yy");
    // a new session's composer starts from the old one's register
    // (`restore_kill_buffer_snapshot`); the old draft is cleared by a selection, not a kill
    s.keys("f9 home shift+end delete");
    s.assert_row(prompt(&s), PLACEHOLDER);
    s.type_text("/new").keys("enter");
    s.keys("f9 i");
    s.type_text("new").keys("esc");
    // nothing to undo or repeat in the fresh composer
    s.type_text("u.");
    s.assert_row(prompt(&s), "› new");
    s.type_text("p");
    s.assert_in_order(&["new", "$figma", "[Image #1]", "[Pasted Content 1001 chars]"]);
    // the pasted line carries its objects over: the submission binds the skill and takes the
    // image (the oracle counts two of each: fresh numbering of a repeated object is the image
    // attachment's, `rich_paste_renders_unique_image_and_paste_labels`)
    s.keys("enter");
    let turns = s.turns();
    assert_eq!(turns.len(), 1, "one turn");
    assert!(
        turns[0].mentions.iter().any(|m| matches!(
            m,
            codex_tui_core::data::mention::ToolMention::Skill { name, path }
                if name == "figma" && path == "/tmp/user/figma/SKILL.md"
        )),
        "the skill the register carried is bound: {:?}",
        turns[0].mentions
    );
    assert!(
        turns[0].images.iter().any(|image| image.path == photo),
        "the image the register carried is attached: {:?}",
        turns[0].images
    );
}

// ORACLE bottom_pane/chat_composer/vim_register_tests.rs:137 insert_yank_and_dot_preserve_rich_payloads kind=ui owner=chat-live
#[test]
fn insert_yank_and_dot_preserve_rich_payloads() {
    for mode in ["i", "R"] {
        let mut s = chat(100, 24);
        rich_draft(&mut s);
        s.type_text("d$");
        s.type_text(mode);
        s.keys("ctrl+y");
        s.type_text(" foo");
        s.keys("ctrl+w ctrl+y esc");
        s.type_text("u");
        s.keys("ctrl+r");
        s.type_text("0.");
        // `foo` appears twice after Insert and once after Replace (the oracle's count)
        s.assert_contains("foo");
    }
}

// ORACLE bottom_pane/chat_composer/vim_register_tests.rs:172 insert_yank_repeat_keeps_original_text_after_word_deletion kind=ui owner=chat-live
#[test]
fn insert_yank_repeat_keeps_original_text_after_word_deletion() {
    let mut s = chat(100, 24);
    vim_draft(&mut s, "foo bar");
    s.type_text("0y$");
    vim_replace_draft(&mut s, "_");
    s.type_text("0i");
    s.keys("ctrl+y ctrl+w esc");
    s.assert_row(prompt(&s), "› foo _");
    s.type_text("0.");
    s.assert_row(prompt(&s), "› foo foo _");
    s.type_text("p");
    s.assert_contains("bar");
}

// ORACLE bottom_pane/chat_composer/vim_register_tests.rs:204 rich_paste_renders_unique_image_and_paste_labels kind=snap owner=chat-live snap=vim_rich_register
#[test]
fn rich_paste_renders_unique_image_and_paste_labels() {
    kit::pending(
        Owner::ChatLive,
        "Vim is built (tests/pages/composer_vim.rs) but the kit seeds no Vim state for this body, no mention, image or paste payload on the chat composer (no kit seeds); the snapshot is the composer alone at 100 columns",
    );
    let mut s = chat(100, 9);
    rich_draft(&mut s);
    s.type_text("p");
    s.assert_snap("vim_rich_register");
}

// ORACLE bottom_pane/chat_composer/vim_register_tests.rs:220 replace_yank_inserts_rich_objects_without_stale_character_recovery kind=ui owner=chat-live
#[test]
fn replace_yank_inserts_rich_objects_without_stale_character_recovery() {
    for existing_element in [false, true] {
        let mut s = chat(100, 24);
        rich_draft(&mut s);
        s.type_text("y$");
        s.keys("ctrl+u");
        s.type_text("abcdefghij");
        if existing_element {
            s.keys("home");
            // attach_image(other.png) at the caret: no kit call yet
        }
        s.type_text("0R");
        s.keys("ctrl+y");
        s.keys("backspace");
        s.assert_contains("abcdefghij");
    }
}

// ORACLE bottom_pane/chat_composer/vim_register_tests.rs:250 equal_length_plain_replace_yank_records_dot kind=ui owner=chat-live
#[test]
fn equal_length_plain_replace_yank_records_dot() {
    let mut s = chat(100, 24);
    vim_draft(&mut s, "xyz");
    s.type_text("0y$");
    vim_replace_draft(&mut s, "abcdefghi");
    s.type_text("0R");
    s.keys("ctrl+y esc");
    s.type_text("l.");
    s.assert_row(prompt(&s), "› xyzxyzghi");
}

// ORACLE bottom_pane/chat_composer/vim_register_tests.rs:265 empty_insert_and_replace_yanks_do_not_replay_a_later_register kind=ui owner=chat-live
#[test]
fn empty_insert_and_replace_yanks_do_not_replay_a_later_register() {
    for mode in ["i", "R"] {
        let mut s = chat(100, 24);
        s.keys("f9");
        s.type_text(mode);
        s.keys("ctrl+y");
        s.type_text("x");
        s.keys("esc");
        s.type_text("0yl.");
        let expected = if mode == "i" { "› xx" } else { "› x" };
        s.assert_row(prompt(&s), expected);
    }
}

// ORACLE bottom_pane/chat_composer/vim_register_tests.rs:283 linewise_rich_paste_preserves_the_cursor_on_the_leading_element kind=snap owner=chat-live
#[test]
fn linewise_rich_paste_preserves_the_cursor_on_the_leading_element() {
    let mut s = chat(100, 24);
    s.paste(&"z".repeat(1001));
    s.keys("f9");
    s.type_text("0yyp");
    let p = prompt(&s);
    s.assert_row(p, "› [Pasted Content 1001 chars]");
    s.assert_row(p + 1, "  [Pasted Content 1001 chars] #2");
    s.assert_cursor(2, p + 1);
}

// ORACLE bottom_pane/chat_composer/vim_register_tests.rs:339 yank_replaces_selection_without_recording_pointer_edits kind=ui owner=chat-live
#[test]
fn yank_replaces_selection_without_recording_pointer_edits() {
    let mut s = chat(100, 24);
    vim_draft(&mut s, "xyz");
    s.type_text("0y$");
    vim_replace_draft(&mut s, "abcdef");
    s.type_text("i");
    let p = prompt(&s);
    // the drag selects `bc` (draft columns 1 to 3)
    s.drag(3, cell(p), 5, cell(p));
    s.keys("ctrl+y");
    s.assert_row(p, "› axyzdef");
}

// ORACLE bottom_pane/chat_composer/vim_register_tests.rs:385 plain_replace_yank_preserves_previous_backspace_recovery kind=ui owner=chat-live
#[test]
fn plain_replace_yank_preserves_previous_backspace_recovery() {
    let mut s = chat(100, 24);
    vim_draft(&mut s, "xyz");
    s.type_text("0y$");
    vim_replace_draft(&mut s, "abcdefghi");
    s.type_text("0RQ");
    s.keys("ctrl+y");
    s.assert_row(prompt(&s), "› Qxyzefghi");
    s.keys("backspace backspace backspace backspace");
    s.assert_row(prompt(&s), "› abcdefghi");
}

// ORACLE bottom_pane/chat_composer/vim_register_tests.rs:400 internal_snapshot_restores_exact_mention_identity kind=ui owner=chat-live
#[test]
fn internal_snapshot_restores_exact_mention_identity() {
    let mut s = chat(100, 24);
    s.type_text("@plugin@plugin@plugin");
    s.assert_contains("@plugin");
}

// ---- oracle: bottom_pane/chat_composer/vim_search_tests.rs (4 screen-level tests here; 0 state tests left to chat-live)
// ORACLE bottom_pane/chat_composer/vim_search_tests.rs:16 fresh_vim_draft_records_typing_after_submission kind=snap owner=chat-live snap=vim_fresh_draft
#[test]
fn fresh_vim_draft_records_typing_after_submission() {
    let mut s = vim_alone(40, 9);
    for entry in ["i", "esc"] {
        // `set_text_content("sent")`, `set_vim_enabled(true)`
        if entry == "i" {
            s.type_text("sent");
            s.keys("f9");
        } else {
            reset_vim_text(&mut s, "sent");
        }
        s.keys(entry);
        s.keys("enter");
        // the submitted draft is gone and Vim is in Insert mode again
        s.assert_row(prompt(&s), PLACEHOLDER);
        s.assert_text_style("Vim: Insert", "fg=Green");
        s.type_text("x");
        s.keys("esc");
        s.type_text(".");
        s.assert_row(prompt(&s), "› xx");
    }
    s.keys("enter");
    s.assert_snap("vim_fresh_draft");
}

// ORACLE bottom_pane/chat_composer/vim_search_tests.rs:41 backspace_cancels_empty_vim_search kind=snap owner=chat-live snap=vim_search_cancelled
#[test]
fn backspace_cancels_empty_vim_search() {
    let mut s = vim_alone(40, 9);
    s.keys("f9");
    for text in ["", "draft"] {
        if !text.is_empty() {
            reset_vim_text(&mut s, text);
        }
        let before = s.shot();
        let cursor = s.grid().cursor;
        for prefix in ["/", "?", "d/", "d?", "c/", "c?", "y/", "y?"] {
            // `/` on an empty draft types the slash (the next test)
            if text.is_empty() && prefix == "/" {
                continue;
            }
            s.type_text(prefix);
            // the query line stands in the footer's place, led by the search key
            let query = prefix.chars().last().map(String::from).unwrap_or_default();
            s.assert_row(8, &format!("  {query}"));
            s.keys("backspace");
            // no operator waits, Normal mode, the draft and the caret as they were
            s.assert_same_rows(&before);
            assert_eq!(s.grid().cursor, cursor, "the caret after `{prefix}` Backspace");
        }
    }
    s.snap("vim_search_cancelled").assert();
}

// ORACLE bottom_pane/chat_composer/vim_search_tests.rs:74 vim_search_query_edits_and_paste_preserve_the_draft kind=snap owner=chat-live snap=vim_composer_search
#[test]
fn vim_search_query_edits_and_paste_preserve_the_draft() {
    kit::pending(
        Owner::ChatLive,
        "Vim is built (tests/pages/composer_vim.rs) but the kit seeds no Vim state for this body, no `[vim_search]` keymap (forward = `g s`) and no search query line on the chat composer; the snapshot is the composer alone at 40 columns",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(40, 9)
        .live()
        .keymap(&[("vim_search.forward", "g s")])
        .open();
    vim_draft(&mut s, "a g\u{301}g b हि gg");
    s.type_text("dgse\u{301}");
    s.keys("backspace");
    s.type_text("gg");
    s.paste("photo.png");
    s.assert_contains("ggphoto.png");
    s.assert_snap("vim_composer_search");
}

// ORACLE bottom_pane/chat_composer/vim_search_tests.rs:116 empty_vim_normal_slash_opens_commands kind=snap owner=chat-live snap=vim_empty_normal_slash
#[test]
fn empty_vim_normal_slash_opens_commands() {
    kit::pending(
        Owner::ChatNavPane,
        "G68: the command popup is the Popover rows (a `▎` bar, right-aligned details, one row per command) and not the oracle's `› /model` table with wrapped descriptions over the draft's three rows; Normal-mode `/` itself opens Insert with the popup (asserted below)",
    );
    let mut s = vim_alone(60, 9);
    s.keys("f9");
    s.type_text("/");
    s.assert_row(prompt(&s), "› /");
    s.assert_text_style("Vim: Insert", "fg=Green");
    s.assert_snap("vim_empty_normal_slash");
}

// ---- oracle: bottom_pane/chat_composer_enter_tests.rs (1 screen-level tests here; 1 state tests left to chat-live)
// ORACLE bottom_pane/chat_composer_enter_tests.rs:61 plain_enter_submits_expired_vim_insert_input kind=snap owner=chat-live snap=vim_enter_after_expired_paste
#[test]
fn plain_enter_submits_expired_vim_insert_input() {
    let mut s = vim_alone(60, 9);
    s.keys("f9 i");
    // the `x` is typed a moment before Enter: no paste is in the making
    s.type_text("x");
    s.wait(1000);
    s.keys("enter");
    s.assert_row(prompt(&s), PLACEHOLDER);
    s.assert_snap("vim_enter_after_expired_paste");
}

// ---- oracle: bottom_pane/paste_burst.rs (2 screen-level tests here; 3 state tests left to chat-live)
// ORACLE bottom_pane/paste_burst.rs:525 ascii_two_fast_chars_start_buffer_from_pending_and_flush_as_paste kind=ui owner=chat-live
#[test]
fn ascii_two_fast_chars_start_buffer_from_pending_and_flush_as_paste() {
    let mut s = chat(100, 24);
    s.type_text("ab");
    s.assert_row(prompt(&s), "› ab");
}

// ORACLE bottom_pane/paste_burst.rs:587 newline_suppression_window_outlives_buffer_flush kind=ui owner=chat-live
#[test]
fn newline_suppression_window_outlives_buffer_flush() {
    kit::na(
        Owner::ChatLive,
        "a state test of the pure PasteBurst (the window and the flush are internal), carried to core: `screens::composer::paste_burst::tests::newline_suppression_window_outlives_buffer_flush`; the composer clears the window when a flushed burst is applied (`apply_paste`), as the oracle's does, so no screen shows it outliving the flush",
    );
}
