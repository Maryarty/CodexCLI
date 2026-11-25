//! Shard `chat_commands_1` (screens S10, owner chat-nav-pane): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen};

const SESSION: &str = include_str!("fixtures/chat_commands_2/session.jsonl");
const RUNNING: &str = include_str!("fixtures/chat_commands_2/running.jsonl");
const INTERRUPTED: &str = include_str!("fixtures/chat_commands_2/interrupted.jsonl");
const COMPLETED: &str = include_str!("fixtures/chat_commands_1/completed.jsonl");
/// The live chat with one task the `@` search found (`task_mentions_enabled`).
const TASK_SEED: &str = concat!(
    r#"{"step":"seed","seed":"session","thread":"thread-1"}"#,
    "\n",
    r#"{"step":"screen","dir":"popovers","ev":{"tasks":[{"thread":"task-123","title":"Review database migration","cwd":"/workspace/project"}]}}"#,
    "\n",
);
/// A thread its parent controls: the server says it takes no direct input (`set_parent_owned_thread`).
const PARENT_OWNED: &str = include_str!("fixtures/chat_commands_3/parent_owned.jsonl");

/// The oracle's `paste_hidden_shell_payload`: `!echo` and 1000 `x`, 1006 chars,
/// shown in the composer as one paste placeholder.
fn hidden_shell_payload() -> String {
    format!("!echo {}", "x".repeat(1000))
}

// ---- oracle: chatwidget/tests/composer_submission.rs (50 screen-level tests here; 20 state tests left to chat-nav-pane)
// ORACLE chatwidget/tests/composer_submission.rs:17 reconnect_queues_enter_and_resumes_ordinary_next_turn_submission kind=snap owner=chat-nav-pane
#[test]
fn reconnect_queues_enter_and_resumes_ordinary_next_turn_submission() {
    kit::pending(
        Owner::ChatNavPane,
        "no seed for a disconnected thread stream (the connection drops, then Enter queues a prompt); the kit cannot set the disconnect",
    );
    let mut s = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
    s.type_text("queued prompt");
    s.keys("enter");
    s.type_text("still editing");
    // The oracle's inline snapshot: its visible lines, trimmed, queued first.
    s.assert_in_order(&["↳ queued prompt", "› still editing"]);
}

// ORACLE chatwidget/tests/composer_submission.rs:76 composer_submission_sends_once_and_requests_latest kind=ui owner=chat-nav-pane
#[test]
fn composer_submission_sends_once_and_requests_latest() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .script(SESSION)
        .open();
    s.type_text("**send once**");
    s.keys("enter");
    // One user turn, with the typed text literal; the draft is empty. The
    // oracle's FollowTranscript event is page scroll state here: the prompt
    // is on screen, at the bottom.
    s.assert_turn_texts(&["**send once**"]);
    s.assert_composer("");
    s.assert_contains("send once");
}

// ORACLE chatwidget/tests/composer_submission.rs:147 hidden_shell_paste_submits_literal_prompt kind=ui owner=chat-nav-pane
#[test]
fn hidden_shell_paste_submits_literal_prompt() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .script(SESSION)
        .open();
    s.paste(&hidden_shell_payload());
    s.assert_composer("[Pasted Content 1006 chars]");
    s.keys("enter");
    s.assert_turn_texts(&[&hidden_shell_payload()]);
}

// ORACLE chatwidget/tests/composer_submission.rs:158 hidden_shell_paste_recalled_from_history_submits_literal_prompt kind=ui owner=chat-nav-pane
#[test]
fn hidden_shell_paste_recalled_from_history_submits_literal_prompt() {
    kit::pending(
        Owner::ChatNavPane,
        "screen feature (composer history): Up recalls the payload as the plain text `!echo xxx...` (the oracle's history keeps the large-paste placeholder and its pending paste), and Enter on it sends no turn; the kit reads the sent turns (`assert_turn_texts`)",
    );
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .script(SESSION)
        .open();
    s.paste(&hidden_shell_payload());
    s.keys("enter");
    s.assert_turn_texts(&[&hidden_shell_payload()]);
    // turn-1 starts and completes, then Up recalls the payload and Enter sends it again
    s.feed(RUNNING);
    s.feed(COMPLETED);
    s.keys("up enter");
    s.assert_turn_texts(&[&hidden_shell_payload(), &hidden_shell_payload()]);
}

// ORACLE chatwidget/tests/composer_submission.rs:174 hidden_shell_paste_queued_during_turn_submits_literal_prompt kind=snap owner=chat-nav-pane snap=hidden_shell_paste_queued_preview
#[test]
fn hidden_shell_paste_queued_during_turn_submits_literal_prompt() {
    kit::pending(
        Owner::ChatNavPane,
        "the queued preview shows the expanded `!echo xxx` text, the oracle keeps the `[Pasted Content 1006 chars]` placeholder in the queue row (the chat queue stores expanded text); the seed is the scripted running turn and the bottom-pane region",
    );
    for key in ["tab", "enter"] {
        let mut s = Screen::build(Owner::ChatNavPane)
            .size(80, 24)
            .script(RUNNING)
            .open();
        s.paste(&hidden_shell_payload());
        s.keys(key);
        if key == "tab" {
            // The oracle's snapshot is the queued preview alone: ours is the bottom pane's top.
            let row = s
                .rows()
                .iter()
                .position(|row| row.contains("Queued follow-up inputs"))
                .unwrap_or(usize::MAX);
            s.assert_snap_region("hidden_shell_paste_queued_preview", row, 0);
        }
        // The turn finishes: the queued payload goes out as the literal `!echo` text.
        s.feed(COMPLETED);
        s.assert_turn_texts(&[&hidden_shell_payload()]);
    }
}

// ORACLE chatwidget/tests/composer_submission.rs:195 hidden_shell_paste_restored_by_queue_edit_submits_literal_prompt kind=ui owner=chat-nav-pane
#[test]
fn hidden_shell_paste_restored_by_queue_edit_submits_literal_prompt() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .script(RUNNING)
        .open();
    s.paste(&hidden_shell_payload());
    s.assert_composer("[Pasted Content 1006 chars]");
    // Tab queues it behind the running turn, alt+up takes it back, Enter queues it again
    s.keys("tab alt+up enter");
    s.assert_no_turn();
    s.feed(COMPLETED);
    s.assert_turn_texts(&[&hidden_shell_payload()]);
}

// ORACLE chatwidget/tests/composer_submission.rs:211 hidden_shell_paste_restored_by_interrupt_submits_literal_prompt kind=ui owner=chat-nav-pane
#[test]
fn hidden_shell_paste_restored_by_interrupt_submits_literal_prompt() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .script(RUNNING)
        .open();
    s.paste(&hidden_shell_payload());
    s.assert_composer("[Pasted Content 1006 chars]");
    s.keys("tab");
    // the server interrupts the turn; the queued prompt comes back to the composer
    s.feed(INTERRUPTED);
    s.keys("enter");
    s.assert_turn_texts(&[&hidden_shell_payload()]);
}

// ORACLE chatwidget/tests/composer_submission.rs:225 hidden_shell_paste_restored_after_image_rejection_submits_literal_prompt kind=ui owner=chat-nav-pane
#[test]
fn hidden_shell_paste_restored_after_image_rejection_submits_literal_prompt() {
    let mut s = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
    s.paste(&hidden_shell_payload());
    s.assert_contains("[Pasted Content 1006 chars]");
    s.keys("enter");
    // The oracle clears the remote image, then Enter submits the literal payload.
    s.keys("enter");
}

// ORACLE chatwidget/tests/composer_submission.rs:248 hidden_shell_paste_restored_after_unavailable_model_submits_literal_prompt kind=ui owner=chat-nav-pane
#[test]
fn hidden_shell_paste_restored_after_unavailable_model_submits_literal_prompt() {
    let mut s = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
    s.paste(&hidden_shell_payload());
    s.assert_contains("[Pasted Content 1006 chars]");
    s.keys("enter");
    // The oracle restores the model, then Enter submits the literal payload.
    s.keys("enter");
}

// ORACLE chatwidget/tests/composer_submission.rs:263 rejected_hidden_shell_paste_preserves_colliding_draft_paste kind=ui owner=chat-nav-pane
#[test]
fn rejected_hidden_shell_paste_preserves_colliding_draft_paste() {
    kit::pending(
        Owner::ChatNavPane,
        "no seed for an unavailable model (set_model(\"\")) and no earlier prompt in the history (record_replayed_user_message_history) for the ctrl+r recall; the sent text is read by `assert_turn_texts`",
    );
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .script(RUNNING)
        .open();
    s.paste(&hidden_shell_payload());
    s.assert_contains("[Pasted Content 1006 chars]");
    s.keys("tab");
    s.paste(&format!("draft {}", "y".repeat(1000)));
    s.keys("ctrl+r");
    s.type_text("e");
    s.feed(COMPLETED);
    s.keys("esc enter");
    let draft = "y".repeat(1000);
    s.assert_turn_texts(&[&format!("{}\ndraft {draft}", hidden_shell_payload())]);
}

// ORACLE chatwidget/tests/composer_submission.rs:289 hidden_shell_paste_queued_before_session_submits_literal_prompt kind=ui owner=chat-nav-pane
#[test]
fn hidden_shell_paste_queued_before_session_submits_literal_prompt() {
    kit::pending(
        Owner::ChatNavPane,
        "no seed for submissions queued until the session is configured (set_queue_submissions_until_session_configured); the live chat starts configured",
    );
    let mut s = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
    s.paste(&hidden_shell_payload());
    s.keys("enter");
    // The oracle configures the thread, then sends the queued input.
}

// ORACLE chatwidget/tests/composer_submission.rs:302 parent_owned_thread_blocks_all_direct_input_entry_points kind=snap owner=chat-nav-pane snap=parent_owned_thread_rejects_input
#[test]
fn parent_owned_thread_blocks_all_direct_input_entry_points() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .script(PARENT_OWNED)
        .open();
    s.type_text("keep this draft");
    s.keys("esc enter");
    // The snapshot of the oracle is the one row it inserted.
    s.assert_has_row("■ This sub-agent is controlled by its parent. Direct input is disabled.");
    s.assert_composer("keep this draft");
    s.assert_no_turn();
    // The oracle then tries /init, /review, /side, /archive, /rename, /subagents,
    // /diff, !echo: each Enter keeps the draft and sends nothing.
    for command in [
        "/init",
        "/review check this",
        "/side inspect this",
        "/archive",
        "/rename",
        "/subagents parent",
        "/diff now",
        "!echo blocked",
        " !echo blocked",
    ] {
        // Clear the draft, and the shell mode with it (a second key on the
        // empty composer leaves that mode).
        s.keys("ctrl+u backspace");
        s.type_text(command);
        s.keys("esc enter");
        // A line that starts with `!` is the shell mode: its `!` is the prompt.
        match command.strip_prefix('!') {
            Some(shell) => s.assert_has_row(&format!("! {shell}")),
            None => s.assert_composer(command),
        }
        s.assert_no_turn();
        s.assert_no_shell_or_compact();
    }
}

// ORACLE chatwidget/tests/composer_submission.rs:361 parent_owned_thread_blocks_settings_shortcuts kind=snap owner=chat-nav-pane snap=parent_owned_thread_rejects_settings_shortcuts
#[test]
fn parent_owned_thread_blocks_settings_shortcuts() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .script(PARENT_OWNED)
        .open();
    s.keys("shift+tab alt+. alt+,");
    // The snapshot of the oracle is the three rows it inserted, one per shortcut.
    s.assert_row_count(
        "■ This sub-agent is controlled by its parent. Direct input is disabled.",
        3,
    );
    // and neither the mode nor the effort was asked for
    s.assert_no_request("thread/settings/update");
}

// ORACLE chatwidget/tests/composer_submission.rs:480 handoff_image_survives_model_rejection_and_submission_failure kind=snap owner=chat-nav-pane
#[test]
fn handoff_image_survives_model_rejection_and_submission_failure() {
    kit::pending(
        Owner::ChatNavPane,
        "no seed for a draft handoff with an image pasted by alt+v (a clipboard image on a model without image input) and a failed switch; the composer's image placeholder is the visible part",
    );
    let mut s = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
    s.keys("alt+v");
    s.assert_contains("[Image #1]");
    s.keys("enter");
}

// ORACLE chatwidget/tests/composer_submission.rs:573 local_clipboard_image_does_not_turn_a_held_bang_into_a_shell_command kind=ui owner=chat-nav-pane
#[test]
fn local_clipboard_image_does_not_turn_a_held_bang_into_a_shell_command() {
    kit::pending(
        Owner::ChatNavPane,
        "the oracle attaches the clipboard's image in the same instant the `!` is held (a paste detector's 9 ms first-char hold, `attach_image` at once): the draft is `[Image #1]!` and the held `!` goes in after it. The host's answer reaches the composer one frame later (`ComposerMsg::ImageArrived` at the next build), by when the `!` has gone in as the shell prefix, and Enter in that window is a newline before the image",
    );
    let mut s = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
    // the `!` is the first char of the draft: the paste detector holds it for a moment, and the
    // clipboard's image arrives while it is held
    s.type_burst("!");
    let path = kit::image_file("image.png");
    s.paste_clipboard_image_at_once("alt+v", &path);
    s.keys("enter");
    // the oracle's draft is `[Image #1]!` with shell mode off: nothing is sent yet
    s.assert_no_turn();
    s.assert_no_shell_command();
    s.wait(1000);
    s.keys("enter");
    s.wait_for("› Ask Codex to do anything");
    // the turn is the image then the text, never a shell command
    s.assert_sole_turn(kit::Turn::text("[Image #1]!").images(&[(&path, 0.."[Image #1]".len())]));
}

// ORACLE chatwidget/tests/composer_submission.rs:626 submission_preserves_text_elements_and_local_images kind=ui owner=chat-nav-pane
#[test]
fn submission_preserves_text_elements_and_local_images() {
    kit::na(
        Owner::ChatNavPane,
        "asserts the submitted UserTurn items and the committed user history cell's fields (text elements, local image paths), not rows: a state test",
    );
}

// ORACLE chatwidget/tests/composer_submission.rs:713 submission_includes_configured_active_permission_profile kind=ui owner=chat-nav-pane
#[test]
fn submission_includes_configured_active_permission_profile() {
    kit::na(
        Owner::ChatNavPane,
        "asserts the active permission profile on the submitted UserTurn, not rows: a state test",
    );
}

// ORACLE chatwidget/tests/composer_submission.rs:788 submission_omits_active_permission_profile_for_legacy_snapshot kind=ui owner=chat-nav-pane
#[test]
fn submission_omits_active_permission_profile_for_legacy_snapshot() {
    kit::na(
        Owner::ChatNavPane,
        "asserts the active permission profile on the submitted UserTurn is absent, not rows: a state test",
    );
}

// ORACLE chatwidget/tests/composer_submission.rs:838 submission_with_remote_and_local_images_keeps_local_placeholder_numbering kind=ui owner=chat-nav-pane
#[test]
fn submission_with_remote_and_local_images_keeps_local_placeholder_numbering() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .live()
        .remote_images(&["https://example.com/remote.png"])
        .open();
    // oracle: the draft `[Image #2] submit mixed` with one local image beside the remote one
    let path = s.attach_image("submitted-mixed.png");
    s.type_text(" submit mixed");
    s.assert_contains("[Image #2] submit mixed");
    s.keys("enter");
    s.wait_for("› Ask Codex to do anything");
    // the turn is the remote image, then the local one, then the text: the local placeholder
    // keeps the number after the remote images
    s.assert_sole_turn(
        kit::Turn::text("[Image #2] submit mixed")
            .remote_images(&["https://example.com/remote.png"])
            .images(&[(&path, 0.."[Image #2]".len())]),
    );
    // ... and the history shows the submitted message
    s.assert_contains("[Image #2] submit mixed");
}

// ORACLE chatwidget/tests/composer_submission.rs:939 enter_with_only_remote_images_submits_user_turn kind=ui owner=chat-nav-pane
#[test]
fn enter_with_only_remote_images_submits_user_turn() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .live()
        .remote_images(&["https://example.com/remote-only.png"])
        .open();
    s.assert_composer("");
    s.keys("enter");
    s.wait_for("› Ask Codex to do anything");
    // the turn is the image alone; the draft's remote images are taken
    s.assert_sole_turn(
        kit::Turn::text("")
            .remote_images(&["https://example.com/remote-only.png"])
            .no_images(),
    );
}

// ORACLE chatwidget/tests/composer_submission.rs:1009 shift_enter_with_only_remote_images_does_not_submit_user_turn kind=ui owner=chat-nav-pane
#[test]
fn shift_enter_with_only_remote_images_does_not_submit_user_turn() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .live()
        .remote_images(&["https://example.com/remote-only.png"])
        .open();
    s.assert_composer("");
    s.keys("shift+enter");
    // the oracle asserts no submission and the remote image kept
    s.assert_no_turn();
    s.assert_contains("[Image #1]");
}

// ORACLE chatwidget/tests/composer_submission.rs:1051 enter_with_only_remote_images_does_not_submit_when_modal_is_active kind=ui owner=chat-nav-pane
#[test]
fn enter_with_only_remote_images_does_not_submit_when_modal_is_active() {
    kit::na(
        Owner::ChatNavPane,
        "asserts the remote image URLs held and that no UserTurn is sent while a review popup is open, not rows: a state test",
    );
}

// ORACLE chatwidget/tests/composer_submission.rs:1093 enter_with_only_remote_images_does_not_submit_when_input_disabled kind=ui owner=chat-nav-pane
#[test]
fn enter_with_only_remote_images_does_not_submit_when_input_disabled() {
    kit::na(
        Owner::ChatNavPane,
        "asserts the remote image URLs held and that no UserTurn is sent with the composer input disabled, not rows: a state test",
    );
}

// ORACLE chatwidget/tests/composer_submission.rs:1138 submission_prefers_selected_duplicate_skill_path kind=ui owner=chat-nav-pane
#[test]
fn submission_prefers_selected_duplicate_skill_path() {
    kit::na(
        Owner::ChatNavPane,
        "asserts the skill paths of the submitted UserTurn items, not rows: a state test",
    );
}

// ORACLE chatwidget/tests/composer_submission.rs:1286 blocked_image_restore_preserves_mention_bindings kind=ui owner=chat-nav-pane
#[test]
fn blocked_image_restore_preserves_mention_bindings() {
    kit::pending(
        Owner::ChatNavPane,
        "no model input modalities in the core's catalog: nothing marks a model as one without image input, so a submission with an image is never blocked and restored (restore_blocked_image_submission) with the warning \"does not support image inputs\"",
    );
    let s = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
    s.assert_contains("[Image #1] check $file");
    s.assert_contains("does not support image inputs");
}

// ORACLE chatwidget/tests/composer_submission.rs:1385 startup_draft_handoff_preserves_literal_and_shell_bang kind=ui owner=chat-nav-pane
#[test]
fn startup_draft_handoff_preserves_literal_and_shell_bang() {
    kit::na(
        Owner::ChatNavPane,
        "asserts the draft snapshots (text, shell mode) and the submitted ops, not rows: a state test",
    );
}

// ORACLE chatwidget/tests/composer_submission.rs:1421 startup_draft_handoff_preserves_cursor_and_large_paste kind=ui owner=chat-nav-pane
#[test]
fn startup_draft_handoff_preserves_cursor_and_large_paste() {
    kit::na(
        Owner::ChatNavPane,
        "asserts equality of the composer draft snapshot (cursor, pending paste), not rows: a state test",
    );
}

// ORACLE chatwidget/tests/composer_submission.rs:1437 startup_draft_handoff_merges_existing_prompt_images_and_large_pastes kind=ui owner=chat-nav-pane
#[test]
fn startup_draft_handoff_merges_existing_prompt_images_and_large_pastes() {
    kit::na(
        Owner::ChatNavPane,
        "asserts the text, cursor, pending pastes and local images of the composer draft snapshot, not rows: a state test",
    );
}

// ORACLE chatwidget/tests/composer_submission.rs:1491 startup_draft_handoff_rebases_cursor_around_colliding_large_pastes kind=ui owner=chat-nav-pane
#[test]
fn startup_draft_handoff_rebases_cursor_around_colliding_large_pastes() {
    kit::pending(
        Owner::ChatNavPane,
        "no seed for a startup draft handed over with colliding large pastes (restore_startup_draft) into a chat with its own pastes; the cursor is not a row",
    );
    for (marker, added) in [("é", 0), (" 中 ", 3), (" tail", 6)] {
        let _ = (marker, added);
        let mut s = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
        s.paste(&"a".repeat(1001));
        s.type_text(" ");
        s.paste(&"b".repeat(1002));
        // The oracle restores the startup draft around the cursor, then inserts ✓ at it.
        s.assert_contains("é[Pasted Content 1001 chars] #2 中 [Pasted Content 1002 chars] #2 tail");
    }
}

// ORACLE chatwidget/tests/composer_submission.rs:1536 startup_draft_handoff_preserves_cleared_history_across_session_configuration kind=ui owner=chat-nav-pane
#[test]
fn startup_draft_handoff_preserves_cleared_history_across_session_configuration() {
    for _configure_before_restore in [false, true] {
        let mut s = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
        s.type_text("first startup draft");
        s.keys("ctrl+c");
        s.type_text("second startup draft");
        s.keys("ctrl+c");
        s.paste(&"x".repeat(1001));
        s.keys("ctrl+c");
        s.keys("up");
        s.assert_contains("[Pasted Content 1001 chars]");
        s.keys("up");
        s.assert_contains("second startup draft");
        s.keys("up");
        s.assert_contains("first startup draft");
    }
}

// ORACLE chatwidget/tests/composer_submission.rs:1598 startup_draft_handoff_keeps_vim_insert_mode_for_nonempty_drafts kind=ui owner=chat-nav-pane
#[test]
fn startup_draft_handoff_keeps_vim_insert_mode_for_nonempty_drafts() {
    for (text, expected) in [("draft", "draftx"), ("", "")] {
        let mut s = Screen::build(Owner::ChatNavPane)
            .size(80, 24)
            .live()
            .vim_mode()
            .open();
        s.type_text(text);
        s.keys("x right");
        if !expected.is_empty() {
            s.assert_contains(expected);
        }
    }
}

// ORACLE chatwidget/tests/composer_submission.rs:1642 startup_draft_file_search_waits_for_protected_view_and_enabled_input kind=ui owner=chat-nav-pane
#[test]
fn startup_draft_file_search_waits_for_protected_view_and_enabled_input() {
    kit::pending(
        Owner::ChatNavPane,
        "no seed for a startup draft waiting behind an open approvals popup and a disabled composer; the file-search request is an event, not a row",
    );
    let mut s = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
    // The oracle opens the approvals popup first: the view is active and the draft waits.
    s.keys("esc");
    // Closed, with input disabled the draft still waits; enabled, it restores and the file search starts for `src`.
}

// ORACLE chatwidget/tests/composer_submission.rs:1773 interrupted_turn_restore_keeps_active_mode_for_resubmission kind=ui owner=chat-nav-pane
#[test]
fn interrupted_turn_restore_keeps_active_mode_for_resubmission() {
    kit::pending(
        Owner::ChatNavPane,
        "no seed for the plan collaboration mode on a running turn with a queued message, interrupted; the active mode kind is not a row",
    );
    let mut s = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
    // The oracle queues "Implement the plan." in plan mode, then interrupts turn-1.
    s.assert_contains("Implement the plan.");
    s.keys("enter");
}

// ORACLE chatwidget/tests/composer_submission.rs:1950 empty_enter_during_task_does_not_queue kind=ui owner=chat-nav-pane
#[test]
fn empty_enter_during_task_does_not_queue() {
    kit::na(
        Owner::ChatNavPane,
        "asserts the queued user messages are empty after an empty Enter during a task (a queue field), not rows: a state test",
    );
}

// ORACLE chatwidget/tests/composer_submission.rs:1987 output_free_esc_interrupt_keeps_prompt_and_opens_blank_composer kind=ui owner=chat-nav-pane
#[test]
fn output_free_esc_interrupt_keeps_prompt_and_opens_blank_composer() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .script(SESSION)
        .open();
    s.type_text("revise this prompt");
    s.keys("enter");
    s.assert_turn_texts(&["revise this prompt"]);
    // the server starts the turn: it has no output yet
    s.feed(RUNNING);
    s.keys("esc");
    s.assert_interrupts(1);
    s.feed(INTERRUPTED);
    s.assert_has_row("› revise this prompt");
    s.assert_has_row("■ Conversation interrupted - use /feedback if something went wrong");
    s.assert_composer("");
}

// ORACLE chatwidget/tests/composer_submission.rs:2023 output_free_ctrl_c_interrupt_keeps_prompt_and_opens_blank_composer kind=snap owner=chat-nav-pane
#[test]
fn output_free_ctrl_c_interrupt_keeps_prompt_and_opens_blank_composer() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .script(SESSION)
        .open();
    s.type_text("revise this prompt");
    s.keys("enter");
    s.assert_turn_texts(&["revise this prompt"]);
    s.feed(RUNNING);
    s.keys("ctrl+c");
    s.assert_interrupts(1);
    s.feed(INTERRUPTED);
    // The oracle's snapshot: history (the prompt, a blank line, the notice), then an empty composer.
    s.assert_in_order(&[
        "› revise this prompt",
        "■ Conversation interrupted - use /feedback if something went wrong",
    ]);
    s.assert_composer("");
}

// ORACLE chatwidget/tests/composer_submission.rs:2049 pending_steer_esc_does_not_steal_vim_insert_escape kind=ui owner=chat-nav-pane
#[test]
fn pending_steer_esc_does_not_steal_vim_insert_escape() {
    kit::na(
        Owner::ChatNavPane,
        "asserts the pending steers, the vim insert escape decision and the Interrupt op sent, not rows: a state test",
    );
}

// ORACLE chatwidget/tests/composer_submission.rs:2078 pending_steer_interrupt_uses_remapped_binding kind=ui owner=chat-nav-pane
#[test]
fn pending_steer_interrupt_uses_remapped_binding() {
    kit::na(
        Owner::ChatNavPane,
        "asserts the pending steers and the Interrupt op under a remapped key (a keymap struct), not rows: a state test",
    );
}

// ORACLE chatwidget/tests/composer_submission.rs:2287 default_shortcuts_edit_most_recent_queued_message kind=ui owner=chat-nav-pane
#[test]
fn default_shortcuts_edit_most_recent_queued_message() {
    kit::pending(
        Owner::ChatNavPane,
        "chat.edit_queued_message is not wired: shift+left opens the agent command center and alt+up does nothing, the queued message stays queued; the seed is the scripted running turn plus Tab",
    );
    for key in ["shift+left", "alt+up"] {
        // A running turn with two messages queued behind it (Tab queues a draft).
        let mut s = Screen::build(Owner::ChatNavPane)
            .size(80, 24)
            .script(RUNNING)
            .open();
        for message in ["first queued", "second queued"] {
            s.type_text(message);
            s.keys("tab");
        }
        s.keys(key);
        // The most recent one is back in the composer, the other still queued.
        s.assert_composer("second queued");
        s.assert_has_row("  ↳ first queued");
        s.assert_text_count("second queued", 1);
    }
}

// ORACLE chatwidget/tests/composer_submission.rs:2309 unbound_queued_message_edit_does_not_fall_back_to_alt_up kind=ui owner=chat-nav-pane
#[test]
fn unbound_queued_message_edit_does_not_fall_back_to_alt_up() {
    kit::pending(
        Owner::ChatNavPane,
        "chat.edit_queued_message is not wired (shift+left opens the agent command center, focus leaves the composer); the keymap seed with an unbound action and the queued Tab are in place",
    );
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(100, 24)
        .script(RUNNING)
        .keymap(&[("chat.edit_queued_message", "")])
        .open();
    s.type_text("queued");
    s.keys("tab");
    s.assert_absent("edit last queued message");
    s.keys("shift+left");
    s.keys("alt+up");
    // Nothing came back to the composer and the message is still queued.
    s.assert_composer("");
    s.assert_has_row("  ↳ queued");
}

// ORACLE chatwidget/tests/composer_submission.rs:2330 queued_message_edit_hint_displays_configured_chords kind=ui owner=chat-nav-pane
#[test]
fn queued_message_edit_hint_displays_configured_chords() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(100, 24)
        .script(RUNNING)
        .keymap(&[("chat.edit_queued_message", "ctrl-x up")])
        .open();
    s.type_text("queued");
    s.keys("tab");
    // The oracle's hint: the chord's label (`⌃x ↑`) then the action's text.
    s.assert_contains("⌃x ↑ edit last queued message");
}

// ORACLE chatwidget/tests/composer_submission.rs:2361 enqueueing_history_prompt_multiple_times_is_stable kind=ui owner=chat-nav-pane
#[test]
fn enqueueing_history_prompt_multiple_times_is_stable() {
    let mut s = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
    s.type_text("repeat me");
    s.keys("enter");
    for _ in 0..3 {
        s.keys("up");
        s.assert_contains("repeat me");
        s.keys("tab");
    }
}

// ORACLE chatwidget/tests/composer_submission.rs:2447 user_message_display_from_inputs_matches_flattened_user_message_shape kind=ui owner=chat-nav-pane
#[test]
fn user_message_display_from_inputs_matches_flattened_user_message_shape() {
    kit::na(
        Owner::ChatNavPane,
        "asserts the return value of the display helper against the flattened parts, not rows: a state test",
    );
}

// ORACLE chatwidget/tests/composer_submission.rs:2493 user_message_display_from_inputs_hides_prompt_context kind=ui owner=chat-nav-pane
#[test]
fn user_message_display_from_inputs_hides_prompt_context() {
    kit::na(
        Owner::ChatNavPane,
        "asserts the return value of the display helper (prompt context hidden), not rows: a state test",
    );
}

// ORACLE chatwidget/tests/composer_submission.rs:2564 task_mention_submission_and_transcript_preserve_the_visible_title kind=snap owner=chat-nav-pane snap=task_mention_transcript
#[test]
fn task_mention_submission_and_transcript_preserve_the_visible_title() {
    // The oracle submits a message holding the task binding (its `enabled = false` half is a
    // thread resumed from a directory without the capability file: a state the port cannot
    // have, every thread is a task-mentions thread here). The port picks the task from the `@`
    // popup, as the user does.
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .script(TASK_SEED)
        .open();
    s.type_text("Inspect @Review");
    s.keys("enter");
    s.keys("enter");
    s.assert_last_turn_text(
        &["MUST call `read_thread`", "\"threadId\":\"task-123\""],
        "Inspect [@Review database migration](thread://task-123)",
    );
    let row = s
        .rows()
        .iter()
        .position(|row| row.starts_with("› Inspect"))
        .unwrap_or(usize::MAX);
    s.assert_snap_region("task_mention_transcript", row, 0);
}

// ORACLE chatwidget/tests/composer_submission.rs:2617 committed_user_message_with_hidden_prompt_context_renders_local_images kind=ui owner=chat-nav-pane
#[test]
fn committed_user_message_with_hidden_prompt_context_renders_local_images() {
    kit::na(
        Owner::ChatNavPane,
        "asserts the message and local image paths of the committed user history cell (its fields), not rows: a state test",
    );
}

// ORACLE chatwidget/tests/composer_submission.rs:2721 reconnect_resumes_unsent_input_and_reconciles_confirmed_submissions kind=snap owner=chat-nav-pane
#[test]
fn reconnect_resumes_unsent_input_and_reconciles_confirmed_submissions() {
    kit::pending(
        Owner::ChatNavPane,
        "no seed for a reconnect with a first message in flight (pause for disconnect, capture and restore of the input); the kit cannot drive the reconnect",
    );
    let mut s = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
    s.type_text("first message");
    // The oracle's notice when nothing confirms the first message (the loop runs three id sets).
    s.assert_contains(
        "• Couldn't confirm whether “first message” was sent. It hasn't been resent.",
    );
}

// ORACLE chatwidget/tests/composer_submission.rs:2827 image_submission_is_portable_for_new_turns_and_steers kind=snap owner=chat-nav-pane snap=portable_image_echo_renders_once
#[test]
fn image_submission_is_portable_for_new_turns_and_steers() {
    kit::pending(
        Owner::ChatNavPane,
        "no seed for a local PNG attached and submitted on a new turn or a steer (the kit cannot attach an image file); the echo snapshot is one history cell",
    );
    let mut s = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
    s.type_text("[Image #1] describe");
    s.keys("enter");
    // The oracle's snapshot: the single history cell of the echoed image turn, once per submission.
    s.assert_snap("portable_image_echo_renders_once");
}

// ORACLE chatwidget/tests/composer_submission.rs:2939 image_preparation_failure_restores_full_input_without_submitting kind=snap owner=chat-nav-pane snap=image_preparation_restored_input
#[test]
fn image_preparation_failure_restores_full_input_without_submitting() {
    kit::pending(
        Owner::ChatNavPane,
        "no seed for an image that fails preparation (a removed or oversized file) on a draft with a remote image and a $file mention; the kit cannot remove or grow a file under the app",
    );
    let mut s = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
    s.keys("enter");
    // The oracle's history row names the failure: "Failed to prepare image:", or "32 MiB" when oversized.
    s.assert_contains("Failed to prepare image:");
    // The oracle's restored input is the bottom pane snapshot: its region row is set when the pane is laid out.
    s.assert_snap("image_preparation_restored_input");
}

// ORACLE chatwidget/tests/composer_submission.rs:3060 image_preparation_keeps_input_responsive_and_preserves_pending_input kind=snap owner=chat-nav-pane snap=image_preparation_disconnected snap=image_preparation_in_progress
#[test]
fn image_preparation_keeps_input_responsive_and_preserves_pending_input() {
    kit::pending(
        Owner::ChatNavPane,
        "no seed for an image still being prepared on a held worker (nine scenarios: thread switch, disconnect, interrupt, escape, ...); the kit cannot hold the worker",
    );
    let mut s = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
    s.keys("x right");
    s.assert_contains("x");
    s.assert_snap("image_preparation_in_progress");
    // The oracle's disconnected scenario compares image_preparation_disconnected.
}
