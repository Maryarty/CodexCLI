//! Shard `chat_commands_1` (screens S10, owner chat-nav-pane): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen, SubmissionReceipt};

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
    let mut s = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
    s.stream_lost("Cloud stream ended. Reopen the conversation to reconnect.");
    s.type_text("queued prompt");
    s.keys("enter");
    s.type_text("still editing");
    // The oracle's inline snapshot: its visible lines, trimmed, queued first.
    s.assert_in_order(&["↳ queued prompt", "› still editing"]);
    // nothing was sent while the stream was down
    s.assert_no_turn();
    // the stream is back: the queued prompt goes out as the next turn, the draft stays
    s.stream_restored();
    s.assert_turn_texts(&["queued prompt"]);
    s.assert_composer("still editing");
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
    for key in ["tab", "enter"] {
        let mut s = Screen::build(Owner::ChatNavPane)
            .size(80, 24)
            .script(RUNNING)
            .open();
        s.paste(&hidden_shell_payload());
        s.keys(key);
        if key == "tab" {
            // the oracle's chat widget alone: no agents hint, the status items as the config set them
            s.feed(r#"{"step":"screen","dir":"chat","ev":{"facts":{"agents_navigation":false}}}"#);
            s.feed(r#"{"step":"screen","dir":"chat","ev":{"status":{"model":"GPT-5.6-Sol","cwd":"/tmp/project","items":["model-with-reasoning","current-dir"]}}}"#);
            s.assert_snap_rows_blanks_aside("hidden_shell_paste_queued_preview");
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
    // Tab queues it behind the running turn, alt+up takes it back, Enter queues it again; the
    // edit key reads the queue the last frame drew, so each key is a call (a frame between)
    s.keys("tab");
    s.keys("alt+up");
    s.keys("enter");
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
    s.set_thread_model("");
    s.paste(&hidden_shell_payload());
    s.assert_contains("[Pasted Content 1006 chars]");
    s.keys("enter");
    s.assert_contains("Thread model is unavailable.");
    s.assert_no_turn();
    // The oracle restores the model, then Enter submits the literal payload.
    s.set_thread_model("gpt-5.2");
    s.keys("enter");
    s.assert_turn_texts(&[&hidden_shell_payload()]);
}

// ORACLE chatwidget/tests/composer_submission.rs:263 rejected_hidden_shell_paste_preserves_colliding_draft_paste kind=ui owner=chat-nav-pane
#[test]
fn rejected_hidden_shell_paste_preserves_colliding_draft_paste() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .script(RUNNING)
        .message_history(&["earlier prompt"])
        .open();
    s.paste(&hidden_shell_payload());
    s.assert_contains("[Pasted Content 1006 chars]");
    s.keys("tab");
    s.paste(&format!("draft {}", "y".repeat(1000)));
    // the model goes away while the turn runs (`set_model("")`), then a search is open
    s.set_thread_model("");
    s.keys("ctrl+r");
    s.type_text("e");
    // the turn ends: the drain refuses the queued prompt and gives it back to the composer,
    // merged onto the draft that has a paste of the same size
    s.feed(COMPLETED);
    s.assert_contains("Thread model is unavailable.");
    s.assert_no_turn();
    s.set_thread_model("gpt-5.2");
    s.keys("esc enter");
    let draft = "y".repeat(1000);
    s.assert_turn_texts(&[&format!("{}\ndraft {draft}", hidden_shell_payload())]);
}

// ORACLE chatwidget/tests/composer_submission.rs:289 hidden_shell_paste_queued_before_session_submits_literal_prompt kind=ui owner=chat-nav-pane
#[test]
fn hidden_shell_paste_queued_before_session_submits_literal_prompt() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .live()
        .held_start()
        .open();
    s.paste(&hidden_shell_payload());
    s.keys("enter");
    s.assert_no_turn();
    // The oracle configures the thread, then sends the queued input.
    s.start_thread();
    s.assert_turn_texts(&[&hidden_shell_payload()]);
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
        "The startup image reaches the session and survives an unsupported model. A definitive JSON-RPC turn/start rejection after switching to an image-capable model drops the original draft.",
    );
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .live()
        .held_start()
        .model_catalog(image_submission_catalog())
        .open();
    let read = s.ask_clipboard_image("alt+v");
    let path = kit::image_file("handoff-image.png");
    s.answer_clipboard_read(read, Ok(path.clone())).wait(300);
    s.assert_composer("[Image #1]");
    s.assert_no_turn();

    s.start_thread();
    s.assert_active_thread("thread-1");
    s.set_thread_model("text-only");
    s.keys("enter");
    s.assert_contains("Model text-only does not support image inputs");
    s.assert_composer("[Image #1]");
    s.assert_no_turn();

    s.set_thread_model("gpt-5.5");
    s.hold_image_preparation();
    s.keys("enter");
    s.assert_sole_turn(kit::Turn::text("[Image #1]").images(&[(&path, 0..10)]));
    s.reject_turn_start("submission failed");
    s.assert_contains("submission failed");
    s.assert_composer("[Image #1]");
    s.assert_sole_turn(kit::Turn::text("[Image #1]").images(&[(&path, 0..10)]));
}

// ORACLE chatwidget/tests/composer_submission.rs:573 local_clipboard_image_does_not_turn_a_held_bang_into_a_shell_command kind=ui owner=chat-nav-pane
#[test]
fn local_clipboard_image_does_not_turn_a_held_bang_into_a_shell_command() {
    for (delayed, read_first) in [(false, false), (true, false), (true, true)] {
        let mut s = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
        if !read_first {
            s.type_burst("!");
        }
        let read = s.ask_clipboard_image("alt+v");
        if read_first {
            s.type_burst("!");
        }
        let path = kit::image_file("image.png");
        if delayed {
            s.wait(600);
            s.keys("enter");
        }
        s.assert_no_turn();
        s.assert_no_shell_command();
        s.answer_clipboard_read(read, Ok(path.clone()));
        if !delayed {
            s.keys("enter");
        }
        s.wait(1000);
        s.assert_no_turn();
        s.assert_no_shell_command();
        s.assert_contains("[Image #1]!");
        s.keys("enter");
        s.assert_sole_turn(
            kit::Turn::text("[Image #1]!").images(&[(&path, 0.."[Image #1]".len())]),
        );
        s.assert_no_shell_command();
    }
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
    let catalog = image_submission_catalog();
    enum Submission {
        Direct,
        Queued,
        RefusedSteer,
    }
    for submission in [
        Submission::Direct,
        Submission::Queued,
        Submission::RefusedSteer,
    ] {
        let mut s = Screen::build(Owner::ChatNavPane)
            .size(100, 24)
            .live()
            .model_catalog(catalog.clone())
            .open();
        s.assert_requests_of("model/list", 1);
        image_submit_model(&mut s, "gpt-5.5");

        s.feed(r#"{"step":"screen","dir":"popovers","ev":{"mentions":[{"kind":"skill","name":"file","description":"Check a selected file","insert":"file","path":"/tmp/skills/file/SKILL.md"}]}}"#);
        let path = s.attach_image("blocked.png");
        s.type_text(" check $file");
        s.assert_contains("Check a selected file");
        s.keys("tab");
        s.type_text("now").keys("end");
        let input = "[Image #1] check $file now";
        s.assert_composer(input);

        match submission {
            Submission::Direct => {
                image_submit_model(&mut s, "text-only");
                s.keys("enter");
            }
            Submission::Queued => {
                s.feed(RUNNING);
                s.keys("tab");
                s.assert_contains("↳ [Image #1] check $file now");
                s.assert_composer("");
                image_submit_model(&mut s, "text-only");
                s.feed(COMPLETED);
            }
            Submission::RefusedSteer => {
                s.feed(RUNNING);
                let steer = s.hold_request("turn/steer");
                s.feed(
                    r#"{"step":"respond","method":"turn/steer","error":{"message":"no active turn to steer"}}"#,
                );
                s.keys("enter");
                s.assert_requests_of("turn/steer", 1);
                s.assert_composer("");
                image_submit_model(&mut s, "text-only");
                s.release(&steer);
                s.feed(COMPLETED);
            }
        }
        s.assert_contains("Model text-only does not support image inputs");
        s.assert_composer(input);
        s.assert_no_turn();
        s.assert_no_request("turn/start");
        s.assert_requests_of(
            "turn/steer",
            usize::from(matches!(submission, Submission::RefusedSteer)),
        );

        image_submit_model(&mut s, "gpt-5.5");
        s.keys("enter");
        s.assert_sole_turn(
            kit::Turn::text(input)
                .images(&[(&path, 0.."[Image #1]".len())])
                .skills(&[("file", "/tmp/skills/file/SKILL.md")]),
        );
    }
}

fn image_submission_catalog() -> serde_json::Value {
    let mut catalog: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/chat_commands_2/speed_catalog.json")).unwrap();
    let mut text_only = catalog["data"][0].clone();
    text_only["id"] = serde_json::json!("text-only");
    text_only["model"] = serde_json::json!("text-only");
    text_only["displayName"] = serde_json::json!("Text only");
    text_only["inputModalities"] = serde_json::json!(["text"]);
    catalog["data"].as_array_mut().unwrap().push(text_only);
    catalog
}

fn image_submit_model(s: &mut Screen, model: &str) {
    s.feed(
        &serde_json::json!({
            "step": "notify", "message": {"method": "thread/settings/updated", "params": {
                "threadId": "thread-1", "threadSettings": {
                    "cwd": "/tmp/project", "model": model, "modelProvider": "openai",
                    "effort": "medium", "approvalPolicy": "on-request",
                    "approvalsReviewer": "user", "sandboxPolicy": {"type": "dangerFullAccess"},
                    "collaborationMode": {"mode": "default", "settings": {
                        "model": model, "reasoning_effort": "medium", "developer_instructions": null
                    }}
                }
            }}
        })
        .to_string(),
    );
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
    kit::na(
        Owner::ChatNavPane,
        "the port has one composer: the startup draft is never handed from a pump to a chat composer, so there is no restore around the cursor to collide with its pastes; the oracle asserts the draft snapshot, not rows",
    );
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
    kit::na(
        Owner::ChatNavPane,
        "the port has one composer: no startup draft waits to be restored behind a protected view, so no file search starts at the restore; the oracle asserts the search request, not rows",
    );
}

// ORACLE chatwidget/tests/composer_submission.rs:1773 interrupted_turn_restore_keeps_active_mode_for_resubmission kind=ui owner=chat-nav-pane
#[test]
fn interrupted_turn_restore_keeps_active_mode_for_resubmission() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(100, 24)
        .live()
        .open();
    s.set_thread_model("gpt-5");
    s.server_answers("thread/settings/update", serde_json::json!({}));
    s.keys("shift+tab");
    s.assert_contains("Plan mode");
    s.assert_last_request_has("thread/settings/update", r#""mode":"plan""#);

    let draft = "Implement the plan.";
    s.feed(RUNNING.lines().last().expect("turn started notification"));
    s.type_text(draft).keys("tab");
    s.assert_contains("↳ Implement the plan.");
    s.assert_composer("");
    s.keys("esc");
    s.assert_interrupts(1);
    s.feed(INTERRUPTED);
    s.assert_composer(draft);
    s.assert_absent("↳ Implement the plan.");
    s.assert_no_turn();
    s.assert_contains("Plan mode");

    s.keys("enter");
    s.assert_turn_texts(&[draft]);
    s.assert_contains("Plan mode");
    s.assert_requests_of("thread/settings/update", 1);
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
    let notice = "• Couldn't confirm whether “first message” was sent. It hasn't been resent.";
    for pending_start in [false, true] {
        for receipt in [
            SubmissionReceipt::Legacy,
            SubmissionReceipt::Unrelated,
            SubmissionReceipt::Exact,
        ] {
            let mut s = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
            s.hold_reconnect_submission(pending_start);
            s.type_text("first message");
            s.keys("enter");
            s.type_text("follow-up");
            s.keys(if pending_start { "enter" } else { "tab" });
            s.recover_submission(receipt, "first message");
            s.assert_requests_of("thread/resume", 1);
            if receipt == SubmissionReceipt::Exact {
                s.assert_reconnect_queue(&[]);
                s.assert_absent(notice);
                if pending_start {
                    s.assert_turn_texts(&["first message", "follow-up"]);
                } else {
                    s.assert_turn_texts(&["follow-up"]);
                }
            } else {
                s.assert_reconnect_queue(&["first message", "follow-up"]);
                s.assert_contains(notice);
                if pending_start {
                    s.assert_turn_texts(&["first message"]);
                } else {
                    s.assert_no_turn();
                }
            }
        }
    }
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
        "pages::composer_images covers held failure and retry with local and remote attachments; remaining cases assert bound skill preservation, steer and oversized inputs",
    );
}

// ORACLE chatwidget/tests/composer_submission.rs:3060 image_preparation_keeps_input_responsive_and_preserves_pending_input kind=snap owner=chat-nav-pane snap=image_preparation_disconnected snap=image_preparation_in_progress
#[test]
fn image_preparation_keeps_input_responsive_and_preserves_pending_input() {
    kit::pending(
        Owner::ChatNavPane,
        "pages::composer_images covers editing and queued input during held preparation; the oracle also exercises nine interruption, ownership and connection variants",
    );
}
