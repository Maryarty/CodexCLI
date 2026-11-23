//! Shard `composer_2` (screens S10, owner chat-live): the oracle tests of the manifest below,
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

/// The live chat after the kit has started its session and drawn the composer.
fn ready(width: u16, height: u16) -> Screen {
    let mut s = Screen::build(Owner::ChatLive)
        .size(width, height)
        .live()
        .open();
    s.wait_for("› Ask Codex to do anything");
    s
}

/// The composer alone with its footer at `width` by `height`: the oracle's
/// composer harness (`snapshot_composer_state_with_width`).
fn alone(width: u16, height: u16) -> Screen {
    Screen::build(Owner::ChatLive)
        .size(width, height)
        .entry("composer", "alone")
        .open()
}

/// The composer with an empty draft: the placeholder of the oracle's composer.
const PLACEHOLDER: &str = "› Ask Codex to do anything";

/// `key` pressed `n` times, as one key string.
fn times(key: &str, n: usize) -> String {
    vec![key; n].join(" ")
}

// ---- oracle: bottom_pane/chat_composer.rs (50 screen-level tests here; 22 state tests left to chat-live)
// ORACLE bottom_pane/chat_composer.rs:7806 task_mention_popup_and_selected_composer_snapshots kind=snap owner=chat-live snap=task_mention_popup snap=task_mention_selected_composer
#[test]
fn task_mention_popup_and_selected_composer_snapshots() {
    kit::pending(
        Owner::ChatLive,
        "G-new-1: the text area paints a bound mention in no accent (the oracle's cell (2, 1) is cyan); the popup and the selected composer snapshots above it match",
    );
    let open = || {
        Screen::build(Owner::ChatLive)
            .size(100, 9)
            .entry("popovers", "alone_tasks")
            .open()
    };
    let mut s = open();
    s.type_text("@migr");
    s.assert_snap("task_mention_popup");

    let mut s = open();
    s.type_text("@migr").keys("enter");
    s.assert_snap("task_mention_selected_composer");
    // the oracle's cell check: the selected mention's first cell is cyan
    s.assert_style(2, 1, "fg=Cyan");
}

// ORACLE bottom_pane/chat_composer.rs:7855 task_mention_titles_are_normalized_and_bounded_before_insertion kind=ui owner=chat-live
#[test]
fn task_mention_titles_are_normalized_and_bounded_before_insertion() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(100, 24)
        .script(include_str!("fixtures/composer_2/tasks_long.jsonl"))
        .open();
    s.type_text("@task").keys("enter");
    // the visible half: the inserted mention reads `@task ` and the 155 chars of the title, the
    // title cut after the first `é` (the wrap takes the long word to the next rows); the
    // binding the oracle asserts is not a row
    let p = prompt(&s);
    s.assert_row(p, "› @task");
    s.assert_row(p + 1, &format!("  {}", "a".repeat(97)));
    s.assert_row(p + 2, &format!("  {}é", "a".repeat(57)));
}

// ORACLE bottom_pane/chat_composer.rs:7930 reopening_identical_unified_mention_restarts_file_search kind=ui owner=chat-live
#[test]
fn reopening_identical_unified_mention_restarts_file_search() {
    let mut s = ready(100, 24);
    // the draft `@foo @foo` with the cursor at `@f` (offset 2 of 9)
    s.type_text("@foo @foo").keys(&times("left", 7));
    s.keys("esc");
    s.assert_row(prompt(&s), "› @foo @foo");
    // the cursor at the end reopens the identical mention
    s.keys("end");
    s.assert_row(prompt(&s), "› @foo @foo");
}

// ORACLE bottom_pane/chat_composer.rs:7979 mention_popup_type_prefixes_snapshot kind=snap owner=chat-live snap=mention_popup_type_prefixes
#[test]
fn mention_popup_type_prefixes_snapshot() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(72, 9)
        .entry("popovers", "alone_mention_types")
        .open();
    s.type_text("$goog");
    s.assert_snap("mention_popup_type_prefixes");
}

// ORACLE bottom_pane/chat_composer.rs:8082 shortcut_overlay_persists_while_task_running kind=ui owner=chat-live
#[test]
fn shortcut_overlay_persists_while_task_running() {
    let mut s = ready(100, 24);
    s.keys("?");
    s.assert_contains("Keyboard shortcuts");
    // the turn starts while the overlay is up: the overlay stays
    s.feed(include_str!("fixtures/composer_2/running.jsonl"));
    s.assert_contains("Keyboard shortcuts");
}

// ORACLE bottom_pane/chat_composer.rs:8516 bound_at_mentions_do_not_block_arrow_navigation kind=ui owner=chat-live
#[test]
fn bound_at_mentions_do_not_block_arrow_navigation() {
    // `go @figma now` with `@figma` bound (`plugin://figma@debug`): the pick binds it.
    let mut s = Screen::build(Owner::ChatLive)
        .size(100, 24)
        .script(include_str!("fixtures/composer_2/plugins.jsonl"))
        .open();
    s.type_text("go @figma").keys("tab");
    s.type_text("now").keys("home right right");
    let p = prompt(&s);
    s.assert_cursor(4, p); // `go` is 2 cells after the prompt
    s.keys("right");
    s.assert_cursor(5, p); // `go ` (3)
    s.keys("right");
    s.assert_cursor(11, p); // `go @figma` (9): the bound mention is one step
    s.keys("left");
    s.assert_cursor(5, p);
    s.keys("left");
    s.assert_cursor(4, p);
}

// ORACLE bottom_pane/chat_composer.rs:8561 restored_bound_at_mentions_do_not_open_mention_popup kind=ui owner=chat-live
#[test]
fn restored_bound_at_mentions_do_not_open_mention_popup() {
    // The oracle restores a draft with the `@sample` binding; ours recalls it from the history
    // (a submitted draft keeps its bindings). The first turn still runs, so a recalled draft the
    // Enter submits is queued (`↳ ..`); a popup that opened would take the Enter instead.
    for (typed, tail) in [("@sample", ""), ("Please ask @sample", ".")] {
        let mut s = Screen::build(Owner::ChatLive)
            .size(100, 24)
            .script(include_str!("fixtures/composer_2/plugins.jsonl"))
            .open();
        s.type_text(typed).keys("tab backspace");
        s.type_text(tail).keys("enter");
        s.assert_row(prompt(&s), PLACEHOLDER);
        s.keys("up end");
        s.keys("enter");
        s.assert_row(prompt(&s), PLACEHOLDER);
        s.assert_has_row(&format!("  ↳ {typed}{tail}"));
    }
}

// ORACLE bottom_pane/chat_composer.rs:8618 enter_submits_when_file_popup_has_no_selection kind=ui owner=chat-live
#[test]
fn enter_submits_when_file_popup_has_no_selection() {
    let input = "npx -y @kaeawc/auto-mobile@latest";
    let mut s = ready(100, 24);
    s.type_text(input).keys("enter");
    s.assert_row(prompt(&s), PLACEHOLDER);
    s.assert_contains(input);
}

// ORACLE bottom_pane/chat_composer.rs:8650 enter_submits_when_unified_mention_popup_has_no_selection kind=ui owner=chat-live
#[test]
fn enter_submits_when_unified_mention_popup_has_no_selection() {
    let input = "npx -y @kaeawc/auto-mobile@latest";
    let mut s = ready(100, 24);
    s.type_text(input).keys("enter");
    s.assert_row(prompt(&s), PLACEHOLDER);
    s.assert_contains(input);
}

// ORACLE bottom_pane/chat_composer.rs:8686 ascii_prefix_survives_non_ascii_followup kind=ui owner=chat-live
#[test]
fn ascii_prefix_survives_non_ascii_followup() {
    let mut s = ready(100, 24);
    s.type_text("1あ").keys("enter");
    s.assert_row(prompt(&s), PLACEHOLDER);
    s.assert_contains("1あ");
}

// ORACLE bottom_pane/chat_composer.rs:8717 non_ascii_char_inserts_immediately_without_burst_state kind=ui owner=chat-live
#[test]
fn non_ascii_char_inserts_immediately_without_burst_state() {
    let mut s = ready(100, 24);
    s.type_text("あ");
    // the drawn half: the character is in the draft at once (the burst state is internal)
    s.assert_row(prompt(&s), "› あ");
}

// ORACLE bottom_pane/chat_composer.rs:8741 non_ascii_burst_buffers_enter_and_flushes_multiline kind=ui owner=chat-live
#[test]
fn non_ascii_burst_buffers_enter_and_flushes_multiline() {
    // the oracle forces the burst (`begin_with_retro_grabbed`); a screen test starts it the way a
    // paste does: the detector takes a leading ideographic space for a paste
    let mut s = ready(100, 24);
    s.type_burst("\u{3000}你好\nhi");
    // (the oracle also asserts the draft is empty while the burst holds; the screen is one frame
    // per 16.7 ms behind the keys, so that held state is a core unit test)
    s.wait(50);
    // the flushed draft is two rows: `你好`, then `hi` on the next line
    let p = prompt(&s);
    s.assert_row(p, "› \u{3000}你好");
    s.assert_row(p + 1, "  hi");
}

// ORACLE bottom_pane/chat_composer.rs:8775 non_ascii_burst_preserves_ideographic_space_and_ascii kind=ui owner=chat-live
#[test]
fn non_ascii_burst_preserves_ideographic_space_and_ascii() {
    let mut s = ready(100, 24);
    s.type_burst("你\u{3000}好\nhi");
    s.wait(50);
    let p = prompt(&s);
    s.assert_row(p, "› 你\u{3000}好");
    s.assert_row(p + 1, "  hi");
}

// ORACLE bottom_pane/chat_composer.rs:8812 non_ascii_burst_buffers_large_multiline_mixed_ascii_and_unicode kind=ui owner=chat-live
#[test]
fn non_ascii_burst_buffers_large_multiline_mixed_ascii_and_unicode() {
    let lines = [
        "天地玄黄 宇宙洪荒",
        "日月盈昃 辰宿列张",
        "寒来暑往 秋收冬藏",
        "",
        "你好世界 编码测试",
        "汉字处理 UTF-8",
        "终端显示 正确无误",
        "",
        "风吹竹林 月照大江",
        "白云千载 青山依旧",
        "程序员 与 Unicode 同行",
    ];
    let mut s = ready(100, 24);
    s.type_burst(&lines.join("\n"));
    s.wait(50);
    // the text area shows six rows, at the caret: the tail of the flushed payload, then (the
    // caret ten lines up) its head; the oracle compares the whole text
    s.assert_in_order(&[
        "汉字处理 UTF-8",
        "终端显示 正确无误",
        "风吹竹林 月照大江",
        "程序员 与 Unicode 同行",
    ]);
    s.keys("up up up up up up up up up up").wait(50);
    s.assert_in_order(&[
        "天地玄黄 宇宙洪荒",
        "日月盈昃 辰宿列张",
        "寒来暑往 秋收冬藏",
        "你好世界 编码测试",
    ]);
}

// ORACLE bottom_pane/chat_composer.rs:8862 ascii_burst_treats_enter_as_newline_even_when_parent_owned kind=ui owner=chat-live
#[test]
fn ascii_burst_treats_enter_as_newline_even_when_parent_owned() {
    kit::pending(
        Owner::ChatLive,
        "no parent-owned thread seed (set_parent_owned_thread) on the chat composer: the body holds the plain variant, which passes; the parent-owned one is not drawn differently, so a pass would be a half pass",
    );
    let mut s = ready(100, 24);
    s.type_burst("hi\nthere");
    s.wait(50);
    let p = prompt(&s);
    s.assert_row(p, "› hi");
    s.assert_row(p + 1, "  there");
}

// ORACLE bottom_pane/chat_composer.rs:8956 slash_context_enter_ignores_paste_burst_enter_suppression kind=ui owner=chat-live
#[test]
fn slash_context_enter_ignores_paste_burst_enter_suppression() {
    kit::na(
        Owner::ChatLive,
        "asserts the InputResult::Command(SlashCommand::Diff) that Enter returns on `/diff`; the result is not drawn and the test draws nothing else",
    );
}

// ORACLE bottom_pane/chat_composer.rs:8987 non_char_key_flushes_active_burst_before_input kind=ui owner=chat-live
#[test]
fn non_char_key_flushes_active_burst_before_input() {
    let mut s = ready(100, 24);
    s.type_text("hi");
    s.keys("left");
    // the flushed draft `hi` with the cursor after `h` (x = prompt width 2 + 1)
    let p = prompt(&s);
    s.assert_row(p, "› hi");
    s.assert_cursor(3, p);
}

// ORACLE bottom_pane/chat_composer.rs:9023 disable_paste_burst_flushes_pending_first_char_and_inserts_immediately kind=ui owner=chat-live
#[test]
fn disable_paste_burst_flushes_pending_first_char_and_inserts_immediately() {
    let mut s = ready(100, 24);
    s.type_text("a");
    s.assert_row(prompt(&s), "› a");
    s.type_text("b");
    s.assert_row(prompt(&s), "› ab");
}

// ORACLE bottom_pane/chat_composer.rs:9056 handle_paste_small_inserts_text kind=ui owner=chat-live
#[test]
fn handle_paste_small_inserts_text() {
    let mut s = ready(100, 24);
    s.paste("_count_r\x1b[13;2:3uows\tindent\n\0two\u{7f}");
    s.assert_in_order(&["_count_rows", "indent", "two"]);
    s.keys("enter");
    s.assert_row(prompt(&s), PLACEHOLDER);
    s.assert_contains("_count_rows");
}

// ORACLE bottom_pane/chat_composer.rs:9087 empty_enter_returns_none kind=ui owner=chat-live
#[test]
fn empty_enter_returns_none() {
    let mut s = ready(100, 24);
    s.keys("enter");
    s.assert_row(prompt(&s), PLACEHOLDER);
}

// ORACLE bottom_pane/chat_composer.rs:9116 handle_paste_large_uses_placeholder_and_replaces_on_submit kind=ui owner=chat-live
#[test]
fn handle_paste_large_uses_placeholder_and_replaces_on_submit() {
    let mut s = ready(100, 24);
    s.paste(&"x".repeat(1010));
    let p = prompt(&s);
    s.assert_row(p, "› [Pasted Content 1010 chars]");
    s.keys("enter");
    s.assert_absent("[Pasted Content");
    // oracle: the submitted text is the paste itself
    s.assert_contains(&"x".repeat(90));
}

// ORACLE bottom_pane/chat_composer.rs:9150 submit_at_character_limit_succeeds kind=ui owner=chat-live
#[test]
fn submit_at_character_limit_succeeds() {
    // MAX_USER_INPUT_TEXT_CHARS = 1 << 20 (protocol/src/user_input.rs:11)
    let mut s = ready(100, 24);
    s.paste(&"x".repeat(1 << 20));
    s.keys("enter");
    s.assert_row(prompt(&s), PLACEHOLDER);
}

// ORACLE bottom_pane/chat_composer.rs:9178 oversized_submit_reports_error_and_restores_draft kind=ui owner=chat-live
#[test]
fn oversized_submit_reports_error_and_restores_draft() {
    let mut s = ready(100, 24);
    s.paste(&"x".repeat((1 << 20) + 1));
    s.keys("enter");
    // oracle: the draft is restored as it was (the test sets the text with
    // `set_text_clearing_elements`, which the keys cannot reach: a paste of
    // this size is a placeholder in the composer, and stays one)
    s.assert_row(prompt(&s), "› [Pasted Content 1048577 chars]");
    s.assert_contains(
        "Message exceeds the maximum length of 1048576 characters (1048577 provided).",
    );
}

// ORACLE bottom_pane/chat_composer.rs:9220 oversized_queued_submission_reports_error_and_restores_draft kind=ui owner=chat-live
#[test]
fn oversized_queued_submission_reports_error_and_restores_draft() {
    // oracle: `set_steer_enabled` is a no-op (`_enabled`), so the queued twin is
    // the same Enter; the port's queue key is Tab, the second route to the same refusal
    let mut s = ready(100, 24);
    s.paste(&"x".repeat((1 << 20) + 1));
    s.keys("tab");
    s.assert_row(prompt(&s), "› [Pasted Content 1048577 chars]");
    s.assert_contains(
        "Message exceeds the maximum length of 1048576 characters (1048577 provided).",
    );
}

// ORACLE bottom_pane/chat_composer.rs:9264 edit_clears_pending_paste kind=ui owner=chat-live
#[test]
fn edit_clears_pending_paste() {
    let mut s = ready(100, 24);
    s.paste(&"y".repeat(1001));
    let p = prompt(&s);
    s.assert_row(p, "› [Pasted Content 1001 chars]");
    s.keys("backspace");
    s.assert_row(prompt(&s), PLACEHOLDER);
}

// ORACLE bottom_pane/chat_composer.rs:9289 ui_snapshots kind=snap owner=chat-live snap=backspace_after_pastes snap=empty snap=large snap=multiple_pastes snap=short snap=small
#[test]
fn ui_snapshots() {
    let s = alone(100, 10);
    s.assert_snap("empty");

    let mut s = alone(100, 10);
    s.paste("short");
    s.assert_snap("small");

    let mut s = alone(100, 10);
    s.paste(&"z".repeat(1005));
    s.assert_snap("large");

    let mut s = alone(100, 10);
    s.paste(&"x".repeat(1003));
    s.paste(&"y".repeat(1007));
    s.paste(" another short paste");
    s.assert_snap("multiple_pastes");

    let mut s = alone(100, 10);
    s.paste(&"a".repeat(1002));
    s.paste(&"b".repeat(1004));
    s.paste(&"c".repeat(1006));
    s.keys("backspace");
    s.assert_snap("backspace_after_pastes");
}

// ORACLE bottom_pane/chat_composer.rs:9352 image_placeholder_snapshots kind=snap owner=chat-live snap=image_placeholder_multiple snap=image_placeholder_single
#[test]
fn image_placeholder_snapshots() {
    // attach_image(/tmp/image1.png), kill (ctrl+u), restore (ctrl+y)
    let mut s = alone(100, 9);
    s.attach_image("image1.png");
    s.keys("ctrl+u ctrl+y");
    s.assert_row(prompt(&s), "› [Image #1]");
    s.assert_snap("image_placeholder_single");

    // attach_image(/tmp/image1.png) and attach_image(/tmp/image2.png)
    let mut s = alone(100, 9);
    s.attach_image("image1.png");
    s.attach_image("image2.png");
    s.assert_row(prompt(&s), "› [Image #1][Image #2]");
    s.assert_snap("image_placeholder_multiple");
}

// ORACLE bottom_pane/chat_composer.rs:9376 remote_image_rows_snapshots kind=snap owner=chat-live snap=remote_image_rows snap=remote_image_rows_after_delete_first
#[test]
fn remote_image_rows_snapshots() {
    // the oracle's `set_remote_image_urls` of two URLs, then the text: the composer alone with
    // the two remote rows over it (the entry seeds the draft's URLs)
    let remote = || {
        Screen::build(Owner::ChatLive)
            .size(100, 9)
            .entry("composer", "alone_remote")
            .open()
    };
    let mut s = remote();
    s.type_text("describe these");
    s.assert_snap("remote_image_rows");

    let mut s = remote();
    s.type_text("describe these");
    s.keys("home up up delete");
    s.assert_snap("remote_image_rows_after_delete_first");
}

// ORACLE bottom_pane/chat_composer.rs:9412 slash_popup_model_clipping_snapshot kind=snap owner=chat-live snap=slash_popup_mo_clipped
#[test]
fn slash_popup_model_clipping_snapshot() {
    kit::pending(
        Owner::ChatNavPane,
        "G68: the popup rows float in a different layout from the oracle's: a `▎` bar and a right-aligned detail, not aligned name/description columns with a `›` marker, no `enter insert · esc close` line, no `100% context left` footer",
    );
    let mut s = ready(60, 3);
    s.type_text("/mo");
    // the draft and its cursor stay visible above the clipped picker
    s.assert_cursor(5, 1);
    s.assert_snap("slash_popup_mo_clipped");
}

// ORACLE bottom_pane/chat_composer.rs:9479 service_tier_slash_command_dispatches_from_catalog_name kind=ui owner=chat-live
#[test]
fn service_tier_slash_command_dispatches_from_catalog_name() {
    kit::na(
        Owner::ChatLive,
        "asserts the InputResult::ServiceTierCommand that Enter returns for `/fast` from the catalog; the result is not drawn",
    );
}

// ORACLE bottom_pane/chat_composer.rs:9550 slash_init_dispatches_command_and_does_not_submit_literal_text kind=ui owner=chat-live
#[test]
fn slash_init_dispatches_command_and_does_not_submit_literal_text() {
    let mut s = ready(100, 24);
    s.type_text("/init").keys("enter");
    // the command is dispatched and the draft cleared: no literal `/init` is submitted
    s.assert_row(prompt(&s), PLACEHOLDER);
    s.assert_absent("/init");
}

// ORACLE bottom_pane/chat_composer.rs:9602 kill_buffer_preserves_image_after_submit kind=ui owner=chat-live
#[test]
fn kill_buffer_preserves_image_after_submit() {
    let mut s = ready(100, 24);
    s.type_text("restore me ");
    s.attach_image("killed-image.png");
    s.keys("ctrl+u");
    s.assert_row(prompt(&s), PLACEHOLDER);
    s.type_text("hello").keys("enter");
    s.attach_image("other-image.png");
    s.paste(" ");
    s.keys("ctrl+y");
    s.assert_row(prompt(&s), "› [Image #1] restore me [Image #2]");
}

// ORACLE bottom_pane/chat_composer.rs:9655 kill_buffer_persists_after_slash_command_dispatch kind=ui owner=chat-live
#[test]
fn kill_buffer_persists_after_slash_command_dispatch() {
    kit::pending(
        Owner::ChatNavPane,
        "`/diff` opens the transcript pager, which is not built (\"Not implemented yet: expanded transcript\") and takes the Ctrl+Y that should yank the killed text back",
    );
    let mut s = ready(100, 24);
    s.type_text("restore me").keys("home ctrl+k");
    s.assert_row(prompt(&s), PLACEHOLDER);
    s.type_text("/diff").keys("enter");
    s.keys("ctrl+y");
    s.assert_row(prompt(&s), "› restore me");
}

// ORACLE bottom_pane/chat_composer.rs:9693 slash_command_disabled_while_task_running_keeps_text kind=ui owner=chat-live
#[test]
fn slash_command_disabled_while_task_running_keeps_text() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(100, 24)
        .script(include_str!("fixtures/composer_2/running.jsonl"))
        .open();
    s.type_text("/compact").keys("enter");
    s.assert_row(prompt(&s), "› /compact");
    s.assert_contains("disabled while a task is in progress");
}

// ORACLE bottom_pane/chat_composer.rs:9737 enter_queues_when_queue_submissions_is_enabled kind=ui owner=chat-live
#[test]
fn enter_queues_when_queue_submissions_is_enabled() {
    kit::na(
        Owner::ChatLive,
        "asserts the InputResult::Queued { text, action, .. } that Enter returns with the queue flag on; the result is not drawn",
    );
}

// ORACLE bottom_pane/chat_composer.rs:9772 tab_queues_slash_led_prompts_while_task_running_without_validation kind=ui owner=chat-live
#[test]
fn tab_queues_slash_led_prompts_while_task_running_without_validation() {
    kit::pending(
        Owner::ChatLive,
        "no draft-without-popup seed: a pasted slash-led draft opens the command popup in the port, so Tab completes instead of queueing; the queue row and the empty composer are not reachable through the kit yet",
    );
    for input in [
        "/compact",
        "/review check regressions",
        "/fast",
        "/does-not-exist",
    ] {
        let mut s = Screen::build(Owner::ChatLive)
            .size(100, 24)
            .script(include_str!("fixtures/composer_2/running.jsonl"))
            .open();
        s.paste(input).keys("tab");
        // the draft is queued, not validated: the composer is empty and no error is drawn
        s.assert_row(prompt(&s), PLACEHOLDER);
        s.assert_absent("disabled while a task is in progress");
    }
}

// ORACLE bottom_pane/chat_composer.rs:9820 remapped_submit_does_not_fall_back_to_enter kind=ui owner=chat-live
#[test]
fn remapped_submit_does_not_fall_back_to_enter() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(100, 24)
        .live()
        .keymap(&[("composer.submit", "ctrl+j")])
        .open();
    s.type_text("explain the change").keys("enter");
    // Enter inserts a newline: the draft keeps its text and the cursor is on the next row
    s.assert_row(prompt(&s), "› explain the change");
}

// ORACLE bottom_pane/chat_composer.rs:9856 remapped_queue_does_not_fall_back_to_tab kind=ui owner=chat-live
#[test]
fn remapped_queue_does_not_fall_back_to_tab() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(100, 24)
        .script(include_str!("fixtures/composer_2/running.jsonl"))
        .keymap(&[("composer.queue", "ctrl+q")])
        .open();
    s.type_text("queue me").keys("tab");
    s.assert_row(prompt(&s), "› queue me");
}

// ORACLE bottom_pane/chat_composer.rs:9889 remapped_history_search_does_not_fall_back_to_ctrl_r kind=ui owner=chat-live
#[test]
fn remapped_history_search_does_not_fall_back_to_ctrl_r() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(100, 24)
        .live()
        .keymap(&[("composer.history_search_previous", "f2")])
        .open();
    s.keys("ctrl+r");
    s.assert_absent("reverse-i-search:");
    s.keys("f2");
    s.assert_contains("reverse-i-search:");
}

// ORACLE bottom_pane/chat_composer.rs:9917 tab_queues_leading_space_slash_as_plain_text_while_task_running kind=ui owner=chat-live
#[test]
fn tab_queues_leading_space_slash_as_plain_text_while_task_running() {
    kit::na(
        Owner::ChatLive,
        "asserts the InputResult::Queued { text, action } that Tab returns for a leading-space `/does-not-exist`; the result is not drawn",
    );
}

// ORACLE bottom_pane/chat_composer.rs:9950 tab_queues_bang_shell_prompts_while_task_running_without_execution kind=ui owner=chat-live
#[test]
fn tab_queues_bang_shell_prompts_while_task_running_without_execution() {
    for input in ["!echo hi", "!", " !echo hi"] {
        let mut s = Screen::build(Owner::ChatLive)
            .size(100, 24)
            .script(include_str!("fixtures/composer_2/running.jsonl"))
            .open();
        s.paste(input).keys("tab");
        // queued, not run: the composer is empty
        s.assert_row(prompt(&s), PLACEHOLDER);
    }
}

// ORACLE bottom_pane/chat_composer.rs:10001 slash_tab_completion_moves_cursor_to_end kind=ui owner=chat-live
#[test]
fn slash_tab_completion_moves_cursor_to_end() {
    let mut s = ready(100, 24);
    s.type_text("/c").keys("tab");
    let p = prompt(&s);
    s.assert_row(p, "› /compact");
    // the cursor is at the end of `/compact ` (2 + 9 cells)
    s.assert_cursor(11, p);
}

// ORACLE bottom_pane/chat_composer.rs:10029 slash_tab_completion_wins_over_queueing_while_task_running kind=ui owner=chat-live
#[test]
fn slash_tab_completion_wins_over_queueing_while_task_running() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(100, 24)
        .script(include_str!("fixtures/composer_2/running.jsonl"))
        .open();
    s.type_text("/mo").keys("tab");
    let p = prompt(&s);
    s.assert_row(p, "› /model");
    // the cursor is at the end of `/model ` (2 + 7 cells)
    s.assert_cursor(9, p);
}

// ORACLE bottom_pane/chat_composer.rs:10059 slash_key_completes_selected_slash_command_as_text kind=ui owner=chat-live
#[test]
fn slash_key_completes_selected_slash_command_as_text() {
    let mut s = ready(100, 24);
    s.type_text("/m");
    s.type_text("/");
    let p = prompt(&s);
    s.assert_row(p, "› /model");
    s.assert_cursor(9, p);
}

// ORACLE bottom_pane/chat_composer.rs:10088 slash_tab_then_enter_dispatches_builtin_command kind=ui owner=chat-live
#[test]
fn slash_tab_then_enter_dispatches_builtin_command() {
    let mut s = ready(100, 24);
    s.type_text("/di").keys("tab");
    s.assert_row(prompt(&s), "› /diff");
    s.keys("enter");
    // the command is dispatched, not the literal text: the draft is empty again
    s.assert_row(prompt(&s), PLACEHOLDER);
}

// ORACLE bottom_pane/chat_composer.rs:10203 tab_submits_when_no_task_running kind=ui owner=chat-live
#[test]
fn tab_submits_when_no_task_running() {
    let mut s = ready(100, 24);
    s.type_text("hi").keys("tab");
    s.assert_row(prompt(&s), PLACEHOLDER);
    s.assert_contains("hi");
}

// ORACLE bottom_pane/chat_composer.rs:10231 tab_does_not_submit_for_bang_shell_command kind=ui owner=chat-live
#[test]
fn tab_does_not_submit_for_bang_shell_command() {
    let mut s = ready(100, 24);
    s.type_text("!ls").keys("tab");
    // Tab neither submits nor clears a `!` command: the draft stays, its
    // `!` the prompt (shell mode)
    s.assert_row(prompt(&s), "! ls");
}

// ORACLE bottom_pane/chat_composer.rs:10260 bang_prefixed_slash_text_submits_literal_shell_command kind=ui owner=chat-live
#[test]
fn bang_prefixed_slash_text_submits_literal_shell_command() {
    let mut s = ready(100, 24);
    s.type_text("!/diff").keys("enter");
    // oracle: `Submitted { text: "!/diff" }`, a shell command, not the `/diff`
    // command. The line goes out as the user's shell command and the result
    // comes back from the server (nothing is echoed locally, `submit_shell_command`);
    // what the screen shows is the composer cleared and no pager opened
    // (tests/pages/composer_draft.rs holds the request: `thread/shellCommand` `/diff`).
    s.assert_row(prompt(&s), PLACEHOLDER);
    s.assert_absent("Not implemented yet");
}

// ORACLE bottom_pane/chat_composer.rs:10287 slash_mention_dispatches_command_and_inserts_at kind=ui owner=chat-live
#[test]
fn slash_mention_dispatches_command_and_inserts_at() {
    let mut s = ready(100, 24);
    s.type_text("/mention").keys("enter");
    // the command clears the draft and the chat inserts the `@` itself (the
    // oracle's test plays the chat by calling `insert_str("@")` on the composer)
    s.assert_row(prompt(&s), "› @");
}

// ORACLE bottom_pane/chat_composer.rs:10337 slash_plan_args_preserve_text_elements kind=ui owner=chat-live
#[test]
fn slash_plan_args_preserve_text_elements() {
    let mut s = ready(100, 24);
    s.type_text("/plan ");
    s.attach_image("plan.png");
    s.assert_row(prompt(&s), "› /plan [Image #1]");
}
