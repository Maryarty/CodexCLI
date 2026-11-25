//! Shard `chat_live_1` (screens S10, owner chat-live): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen};

/// The row a text is drawn on: the oracle's snapshot is one cell's rows, the port draws the cell
/// in the full screen, so the region starts where the cell does.
fn row_of(s: &Screen, text: &str) -> usize {
    s.rows()
        .iter()
        .position(|row| row.contains(text))
        .unwrap_or(usize::MAX)
}

// Expected rows, copied from the oracle (the header and safety notice texts of app_server.rs).
const SAFETY_HEADER: &str = "Giving this request a little extra thought";

// Reasons shared by several tests (what the chat screen does not have yet).
const SAFETY_PENDING: &str = "no safety-buffering state on the chat screen: model/safetyBuffering/updated is not applied, so no retry prompt, learn-more link or retry confirmation is drawn";
const COLLAB_PENDING: &str = "no collab-agent item on the chat screen: the chat model keeps only user and agent message items (core/src/threads.rs item_upsert), and the agent names come from set_collab_agent_metadata, which has no feed step";

/// Frame size of an oracle snapshot: its width as the oracle test sets it, its rows as stored.
fn oracle_size(name: &str, width: u16) -> (u16, u16) {
    let (_, rows) = kit::snap_size(name);
    (width, u16::try_from(rows).unwrap_or(u16::MAX))
}

/// A `notify` step: a server notification of `method` with `params` (JSON text).
fn notify(method: &str, params: &str) -> String {
    format!(r#"{{"step":"notify","message":{{"method":"{method}","params":{params}}}}}"#)
}

/// A `request` step: a server request of `method` with `params`.
fn request(method: &str, id: u32, params: &str) -> String {
    format!(r#"{{"step":"request","message":{{"method":"{method}","id":{id},"params":{params}}}}}"#)
}

/// A turn object (`items` and `error` are JSON text).
fn turn_json(turn: &str, status: &str, items: &str, error: &str) -> String {
    format!(
        r#"{{"id":"{turn}","rootTurnId":null,"items":{items},"itemsView":"full","status":"{status}","error":{error},"startedAt":null,"completedAt":null,"durationMs":null}}"#
    )
}

/// The error object of a failed turn.
fn turn_error(message: &str) -> String {
    format!(
        r#"{{"message":"{message}","codexErrorInfo":null,"additionalDetails":null,"misalignment":null}}"#
    )
}

/// `turn/started` for a plain turn.
fn turn_started(turn: &str) -> String {
    notify(
        "turn/started",
        &format!(
            r#"{{"threadId":"thread-1","turn":{}}}"#,
            turn_json(turn, "inProgress", "[]", "null")
        ),
    )
}

/// `turn/completed` with the turn's items.
fn turn_completed(turn: &str, status: &str, items: &str) -> String {
    notify(
        "turn/completed",
        &format!(
            r#"{{"threadId":"thread-1","turn":{}}}"#,
            turn_json(turn, status, items, "null")
        ),
    )
}

/// `turn/completed` with status failed and the message of the error.
fn turn_failed(turn: &str, message: &str) -> String {
    notify(
        "turn/completed",
        &format!(
            r#"{{"threadId":"thread-1","turn":{}}}"#,
            turn_json(turn, "failed", "[]", &turn_error(message))
        ),
    )
}

/// `item/agentMessage/delta` of a streamed answer.
fn agent_delta(turn: &str, delta: &str) -> String {
    notify(
        "item/agentMessage/delta",
        &format!(
            r#"{{"threadId":"thread-1","turnId":"{turn}","itemId":"item-1","delta":"{delta}"}}"#
        ),
    )
}

/// `error` notification; `info` is the codexErrorInfo as JSON text.
fn error_note(turn: &str, message: &str, info: &str, will_retry: bool) -> String {
    notify(
        "error",
        &format!(
            r#"{{"threadId":"thread-1","turnId":"{turn}","willRetry":{will_retry},"error":{{"message":"{message}","codexErrorInfo":{info},"additionalDetails":null,"misalignment":null}}}}"#
        ),
    )
}

/// `model/safetyBuffering/updated` for one turn.
fn safety_buffering(turn: &str, show_ui: bool, faster_model: Option<&str>) -> String {
    let faster = faster_model.map_or("null".to_string(), |m| format!("\"{m}\""));
    notify(
        "model/safetyBuffering/updated",
        &format!(
            r#"{{"threadId":"thread-1","turnId":"{turn}","model":"current-model","useCases":[],"reasons":[],"showBufferingUi":{show_ui},"fasterModel":{faster}}}"#
        ),
    )
}

/// `thread/settings/updated` (wire shape not yet checked against a fixture).
fn thread_settings_updated(model: &str, provider: &str) -> String {
    notify(
        "thread/settings/updated",
        &format!(
            r#"{{"threadId":"thread-1","threadSettings":{{"cwd":"/tmp/project","approvalPolicy":"on-request","approvalsReviewer":"user","sandboxPolicy":{{"type":"dangerFullAccess"}},"activePermissionProfile":null,"model":"{model}","modelProvider":"{provider}","serviceTier":null,"effort":null,"summary":null,"collaborationMode":{{"mode":"default","settings":{{"model":"{model}","reasoning_effort":null,"developer_instructions":null}}}},"personality":null}}}}"#
        ),
    )
}

/// An agent message item (`text` is JSON-escaped text).
fn agent_item(text: &str) -> String {
    format!(
        r#"{{"type":"agentMessage","moderation":null,"id":"msg-1","text":"{text}","phase":"final_answer","memoryCitation":null,"delivery":null,"questions":null,"startedAtMs":null,"completedAtMs":0}}"#
    )
}

/// A user message item; `content` is the JSON array of its inputs.
fn user_item(content: &str) -> String {
    format!(r#"{{"type":"userMessage","id":"user-1","clientId":null,"content":{content}}}"#)
}

/// A command execution item (`command` and `output` are JSON-escaped text and JSON value).
fn exec_item(id: &str, command: &str, status: &str, output: &str) -> String {
    format!(
        r#"{{"type":"commandExecution","id":"{id}","command":"{command}","cwd":"/tmp","processId":null,"pluginId":null,"scriptPath":null,"source":"userShell","status":"{status}","commandActions":[],"aggregatedOutput":{output},"exitCode":null,"durationMs":null,"modelContext":null,"sandboxType":null}}"#
    )
}

/// A collab-agent tool call item; `extra` holds its prompt, model and effort fields as JSON text.
fn collab_item(
    id: &str,
    tool: &str,
    status: &str,
    receivers: &str,
    states: &str,
    extra: &str,
) -> String {
    format!(
        r#"{{"type":"collabAgentToolCall","id":"{id}","tool":"{tool}","status":"{status}","senderThreadId":"thread-sender","receiverThreadIds":{receivers},"agentsStates":{states},{extra}}}"#
    )
}

/// `item/started` of an item of the turn `turn-1`.
fn item_started(item: &str) -> String {
    notify(
        "item/started",
        &format!(r#"{{"threadId":"thread-1","turnId":"turn-1","item":{item},"startedAtMs":0}}"#),
    )
}

/// `item/completed` of an item of the turn `turn-1`.
fn item_completed(item: &str) -> String {
    notify(
        "item/completed",
        &format!(
            r#"{{"threadId":"thread-1","turnId":"turn-1","item":{item},"startedAtMs":null,"completedAtMs":0}}"#
        ),
    )
}

/// The idle footer of the oracle's harness: no agents hint, the status line (model, directory)
/// over `? for shortcuts`.
const IDLE_FOOTER: &str = concat!(
    r#"{"step":"screen","dir":"chat","ev":{"facts":{"agents_navigation":false}}}"#,
    "\n",
    r#"{"step":"screen","dir":"chat","ev":{"status":{"model":"GPT-5.6-Sol","cwd":"/tmp/project","items":["model-with-reasoning","current-dir"]}}}"#
);

/// The elicitation request that offers an install link (the oracle's tool suggestion).
fn tool_suggestion_request(install_url: &str) -> String {
    request(
        "mcpServer/elicitation/request",
        9,
        &format!(
            r#"{{"threadId":"thread-1","turnId":"turn-install","serverName":"connector-server","environmentId":null,"mode":"form","_meta":{{"codex_approval_kind":"tool_suggestion","tool_type":"connector","suggest_type":"install","suggest_reason":"Install the connector to continue","tool_id":"connector_test","tool_name":"Test Connector","install_url":"{install_url}"}},"message":"Install Test Connector","requestedSchema":{{"type":"object","properties":{{}}}}}}"#
        ),
    )
}

// ---- oracle: chatwidget/input_queue.rs (1 screen-level tests here; 1 state tests left to chat-live)
// ORACLE chatwidget/input_queue.rs:142 preview_keeps_queue_categories_separate kind=ui owner=chat-live
#[test]
fn preview_keeps_queue_categories_separate() {
    // oracle: one queued message, one pending steer and one rejected steer each show under their own category.
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&turn_started("turn-1"));
    s.type_text("queued");
    s.keys("enter");
    s.type_text("pending");
    s.keys("enter");
    s.type_text("rejected");
    s.keys("enter");
    s.assert_contains("queued");
    s.assert_contains("pending");
    s.assert_contains("rejected");
}

// ---- oracle: chatwidget/pets_tests.rs (1 screen-level tests here; 4 state tests left to chat-live)
// ORACLE chatwidget/pets_tests.rs:30 configured_pet_loads_are_scoped_across_widget_replacements kind=ui owner=chat-live
#[test]
fn configured_pet_loads_are_scoped_across_widget_replacements() {
    kit::na(
        Owner::ChatLive,
        "asserts the ConfiguredPetLoaded events, their generation and whether a finished load is kept, none of which is drawn",
    );
}

// ---- oracle: chatwidget/rendering_tests.rs (14 screen-level tests here; 0 state tests left to chat-live)
// ORACLE chatwidget/rendering_tests.rs:79 owned_bottom_pane_preserves_draft_cursor_and_read_only_notice kind=ui owner=chat-live
#[test]
fn owned_bottom_pane_preserves_draft_cursor_and_read_only_notice() {
    kit::pending(
        Owner::ChatLive,
        "no external-writer state on the chat screen: the thread open in another app, whose notice replaces the composer, has no feed step",
    );
    // oracle: the draft stays in the composer while live output lands in the transcript; the notice replaces the composer and has no cursor.
    let mut s = Screen::build(Owner::ChatLive).size(60, 8).live().open();
    s.type_text("draft stays here");
    s.assert_contains("draft stays here");
    s.feed(&agent_delta(
        "turn-1",
        "live output belongs in the transcript",
    ));
    s.assert_contains("draft stays here");
    // The external-writer step (no feed yet) is followed by the notice and a hidden cursor.
    s.assert_contains("This conversation is open in another app");
    s.assert_cursor_hidden();
}

// ORACLE chatwidget/rendering_tests.rs:130 owned_live_history_keeps_all_sources_in_each_presentation kind=snap owner=chat-live
#[test]
fn owned_live_history_keeps_all_sources_in_each_presentation() {
    kit::pending(
        Owner::ChatLive,
        "no active cell, realtime voice caption or rate-limit hint on the chat screen: the sources the test lists are not fed; the compact, raw and detailed presentations are not screen output",
    );
    let s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    // oracle: the inline frame keeps every source, in order, for either latest speaker.
    s.assert_in_order(&[
        "active",
        "deferred first",
        "deferred second",
        "user caption",
        "assistant caption",
        "rate limit hint",
    ]);
}

// ORACLE chatwidget/rendering_tests.rs:232 external_writer_view_shows_notice_instead_of_composer kind=snap owner=chat-live snap=external_writer_view_shows_notice_instead_of_composer
#[test]
fn external_writer_view_shows_notice_instead_of_composer() {
    let (w, h) = oracle_size("external_writer_view_shows_notice_instead_of_composer", 60);
    let s = Screen::build(Owner::ChatLive)
        .size(w, h)
        .foreign_writer("thread-foreign")
        .live()
        .open();
    // oracle: the notice rows at 60 columns, the bold label row, and no composer.
    s.assert_snap("external_writer_view_shows_notice_instead_of_composer");
}

// ORACLE chatwidget/rendering_tests.rs:268 external_writer_fork_progress_restores_the_previous_view kind=snap owner=chat-live snap=fork_pending
#[test]
fn external_writer_fork_progress_restores_the_previous_view() {
    // the frame the oracle renders is the bottom pane alone (three rows); the
    // notice (six rows) needs the room above it
    let mut s = Screen::build(Owner::ChatLive)
        .size(60, 8)
        .foreign_writer("thread-foreign")
        .live()
        .open();
    let previous = s.shot();
    // oracle: the fork-pending frame at 60 columns; once the fork ends, the previous frame is back.
    s.keys("f");
    s.assert_snap_region("fork_pending", 4, 0);
    // the server refuses the fork: the notice is back as it was
    let mut refused = Screen::build(Owner::ChatLive)
        .size(60, 8)
        .foreign_writer("thread-foreign")
        .fork_refused()
        .live()
        .open();
    refused.keys("f");
    refused.assert_same_rows(&previous);
}

// ORACLE chatwidget/rendering_tests.rs:297 external_writer_notice_uses_current_transcript_shortcut kind=ui owner=chat-live
#[test]
fn external_writer_notice_uses_current_transcript_shortcut() {
    let s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .foreign_writer("thread-foreign")
        .live()
        .keymap(&[("app.open_transcript", "ctrl+k")])
        .open();
    // oracle: the footer of the external-writer notice shows the bound key as ⌃k transcript.
    s.assert_contains("⌃k transcript");
}

// ORACLE chatwidget/rendering_tests.rs:311 active_transcript_preserves_clipped_markdown_hyperlinks kind=snap owner=chat-live
#[test]
fn active_transcript_preserves_clipped_markdown_hyperlinks() {
    kit::pending(
        Owner::ChatLive,
        "no finished agent message with a markdown link in the transcript on the chat screen: the view draws the streamed answer only, not a completed cell with its OSC 8 link",
    );
    let mut s = Screen::build(Owner::ChatLive).size(44, 5).live().open();
    s.feed(&turn_started("turn-1"));
    s.feed(&item_completed(&agent_item(
        "Earlier content\\n\\n[OSC8 label](https://example.com/)",
    )));
    // oracle: the clipped area holds the label and its URL, both as OSC 8 links.
    s.assert_has_row("  OSC8 label (https://example.com/)");
    s.assert_link("OSC8 label", "https://example.com/");
    s.assert_link("https://example.com/", "https://example.com/");
}

// ORACLE chatwidget/rendering_tests.rs:380 initial_session_header_starts_at_the_top_of_the_viewport kind=snap owner=chat-live
#[test]
fn initial_session_header_starts_at_the_top_of_the_viewport() {
    kit::pending(
        Owner::ChatLive,
        "no thread session is fed: the transcript draws its header (transcript_header.rs) once the chat knows the thread's cwd, and no kit step supplies one, so row 0 is empty",
    );
    let s = Screen::build(Owner::ChatLive).size(48, 24).live().open();
    // oracle: the header's first rows start at the top of the viewport; the version and the cwd are placeholders in the oracle.
    s.assert_row_starts_with(0, "  >_ OpenAI Codex (v");
}

// ORACLE chatwidget/rendering_tests.rs:410 active_cell_layout_reuses_heights_without_freezing_animation kind=ui owner=chat-live
#[test]
fn active_cell_layout_reuses_heights_without_freezing_animation() {
    kit::pending(
        Owner::ChatLive,
        "no counting active cell on the chat screen: a streamed cell whose frame advances on each render has no feed step, so the frame 1 and frame 2 rows are not drawn",
    );
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    // oracle: the first and the second render show frame 1 and frame 2 of the cell; the call counts are the cache's internals, not drawn.
    s.assert_contains("frame 1 row 0");
    s.wait(100);
    s.assert_contains("frame 2 row 0");
}

// ORACLE chatwidget/rendering_tests.rs:433 active_cell_layout_preserves_custom_height_and_bottom_scroll kind=snap owner=chat-live snap=cached_active_cell_bottom_scroll
#[test]
fn active_cell_layout_preserves_custom_height_and_bottom_scroll() {
    kit::pending(
        Owner::ChatLive,
        "no custom-height active cell on the chat screen (three rows in a one-row layout) and no feed step that keeps it scrolled to the bottom",
    );
    // oracle: the viewport is 80 by 2 rows; it shows the bottom two rows of the cell (frame 2 rows 1 and 2), not row 0.
    let s = Screen::build(Owner::ChatLive).size(80, 2).live().open();
    s.assert_snap("cached_active_cell_bottom_scroll");
}

// ORACLE chatwidget/rendering_tests.rs:466 active_cell_layout_invalidates_width_revision_mode_theme_and_identity kind=ui owner=chat-live
#[test]
fn active_cell_layout_invalidates_width_revision_mode_theme_and_identity() {
    kit::na(
        Owner::ChatLive,
        "asserts how often the active cell's desired height and lines are recomputed, and the cache key's revision; none of these counts is drawn",
    );
}

// ORACLE chatwidget/rendering_tests.rs:510 externally_mutable_active_cells_bypass_persistent_layout_cache kind=ui owner=chat-live
#[test]
fn externally_mutable_active_cells_bypass_persistent_layout_cache() {
    kit::na(
        Owner::ChatLive,
        "asserts the call counts of an externally mutable cell and that no layout is cached; neither is drawn",
    );
}

// ORACLE chatwidget/rendering_tests.rs:525 removing_active_cell_invalidates_layout_before_reusing_its_identity kind=ui owner=chat-live
#[test]
fn removing_active_cell_invalidates_layout_before_reusing_its_identity() {
    kit::na(
        Owner::ChatLive,
        "asserts the cached layout after take_active_cell, the revision and the recompute count; none is drawn",
    );
}

// ORACLE chatwidget/rendering_tests.rs:547 external_writer_notice_offers_command_center kind=snap owner=chat-live
#[test]
fn external_writer_notice_offers_command_center() {
    for width in [60u16, 100u16] {
        let name = format!("external_writer_command_center_{width}");
        let (w, h) = oracle_size(&name, width);
        // the notice's top padding row stands above the snapshot's first row
        let s = Screen::build(Owner::ChatLive)
            .size(w, h + 1)
            .foreign_writer("thread-foreign")
            .live()
            .open();
        // oracle: the bottom popup of the external-writer notice at this width.
        s.assert_snap_region(&name, 1, 0);
    }
}

/// A thread whose rollout file the server named: the saved chat the notice's `c` continues.
const SAVED_CHAT: &str = include_str!("fixtures/chat_commands_3/rollout.jsonl");

/// The server refused `thread-1` for its model provider (`error` is the failure's JSON).
fn provider_error(error: &str) -> String {
    format!(
        r#"{{"step":"screen","dir":"chat","ev":{{"provider_error":{{"thread":"thread-1","error":{error}}}}}}}"#
    )
}

// ORACLE chatwidget/rendering_tests.rs:557 provider_error_preserves_draft_and_survives_thread_navigation kind=snap owner=chat-live snap=provider_errors
#[test]
fn provider_error_preserves_draft_and_survives_thread_navigation() {
    kit::pending(
        Owner::ChatLive,
        "the mismatch frames name the provider the config calls `Team Gateway` and the remote server the connection is to: the app has no fact for the config's model providers or the launch's remote address (StatusFacts of core/src/screens/chat.rs and host/src/facts.rs), so the notice names the id and has no location row",
    );
    let not_configured = r#"{"type":"modelProviderNotConfigured","modelProvider":"old-provider"}"#;
    let mismatch = r#"{"type":"modelProviderMismatch","modelProvider":"custom-gateway","currentModelProvider":"amazon-bedrock-runtime"}"#;
    let policy_provider = r#"{"type":"modelProviderPolicyConflict","modelProvider":"openai","requiredModelProvider":"amazon-bedrock"}"#;
    let policy_settings = r#"{"type":"modelProviderPolicyConflict","modelProvider":"openai","requiredModelProvider":"openai"}"#;
    for (frame, width, error) in [
        ("provider_error_local_80", 80, not_configured),
        ("provider_error_mismatch_local_80", 80, mismatch),
        ("provider_error_mismatch_local_40", 40, mismatch),
        ("provider_error_mismatch_remote_80", 80, mismatch),
        (
            "provider_error_policy_provider_local_80",
            80,
            policy_provider,
        ),
        (
            "provider_error_policy_settings_local_80",
            80,
            policy_settings,
        ),
    ] {
        // the notice's top padding row stands above the frame's first row
        let rows = kit::snap_rows("provider_errors", frame).len();
        let height = u16::try_from(rows).unwrap_or(u16::MAX) + 1;
        let mut s = Screen::build(Owner::ChatLive)
            .size(width, height)
            .script(SAVED_CHAT)
            .open();
        s.type_text("unsent draft");
        s.feed(&provider_error(error));
        // the chat takes no input: typing, a paste and Enter send nothing
        s.type_text("x");
        s.paste("ignored paste");
        s.keys("enter");
        s.assert_no_turn();
        s.assert_absent("ignored paste");
        // oracle: the bottom pane of the notice at this width, in the frame the case names.
        s.snap("provider_errors").frame(frame).at(1, 0).assert();
        // the notice survives a visit to the other chats and back
        s.keys("esc");
        s.keys("esc");
        s.snap("provider_errors").frame(frame).at(1, 0).assert();
    }
}

// ---- oracle: chatwidget/tests/app_server.rs (40 screen-level tests here; 8 state tests left to chat-live)
// ORACLE chatwidget/tests/app_server.rs:75 session_header_uses_catalog_display_name_without_changing_model kind=snap owner=chat-live
#[test]
fn session_header_uses_catalog_display_name_without_changing_model() {
    kit::pending(
        Owner::ChatLive,
        "no thread session on the chat screen: a session with the catalog's display name is not fed, so the session header with that name, or with the raw model slug when the catalog has none, is not drawn",
    );
    for name in [
        "catalog_model_session_header_startup",
        "catalog_model_session_header_resume",
        "catalog_model_session_header_unknown_model",
    ] {
        let s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
        // oracle: the session header at the top of the transcript, version and cwd as placeholders.
        s.assert_snap_region(name, 0, 0);
    }
}

// ORACLE chatwidget/tests/app_server.rs:108 session_and_settings_sync_server_provider_id kind=snap owner=chat-live snap=daybreak_hidden_after_provider_settings_change
#[test]
fn session_and_settings_sync_server_provider_id() {
    kit::pending(
        Owner::ChatLive,
        "no status line, daybreak popup or thread-settings update on the chat screen: the provider in the status line and the daybreak entry are not drawn",
    );
    let (w, h) = oracle_size("daybreak_hidden_after_provider_settings_change", 80);
    let mut s = Screen::build(Owner::ChatLive).size(w, h).live().open();
    s.type_text("/daybreak");
    s.assert_contains("broader access");
    s.feed(&thread_settings_updated("gpt-5.4", "server-updated"));
    // oracle: the status line shows the new provider; the daybreak entry is hidden for it.
    s.assert_contains("profile: work · provider: server-updated");
    s.assert_absent("broader access");
    s.assert_snap("daybreak_hidden_after_provider_settings_change");
}

// ORACLE chatwidget/tests/app_server.rs:289 safety_buffering_offers_one_retry_with_app_wording kind=snap owner=chat-live snap=safety_buffering_retry_confirmation snap=safety_buffering_retry_confirmation_narrow snap=safety_buffering_retry_prompt
#[test]
fn safety_buffering_offers_one_retry_with_app_wording() {
    kit::pending(Owner::ChatLive, SAFETY_PENDING);
    let (w, h) = oracle_size("safety_buffering_retry_prompt", 80);
    let mut s = Screen::build(Owner::ChatLive).size(w, h).live().open();
    s.feed(&turn_started("turn-safety-buffering"));
    s.feed(&safety_buffering(
        "turn-safety-buffering",
        true,
        Some("faster-model"),
    ));
    s.feed(&safety_buffering(
        "turn-safety-buffering",
        true,
        Some("faster-model"),
    ));
    s.assert_snap("safety_buffering_retry_prompt");
    // oracle: down down enter opens the learn-more URL in the browser (not drawn), and the notice stays.
    s.keys("down down enter");
    s.assert_contains(SAFETY_HEADER);
    // up up enter opens the retry confirmation, at 80 columns and at 40 columns.
    s.keys("up up enter");
    let (cw, ch) = oracle_size("safety_buffering_retry_confirmation", 80);
    s.resize(cw, ch);
    s.assert_contains("Stop this attempt and retry?");
    s.assert_snap("safety_buffering_retry_confirmation");
    let (nw, nh) = oracle_size("safety_buffering_retry_confirmation_narrow", 40);
    s.resize(nw, nh);
    s.assert_snap("safety_buffering_retry_confirmation_narrow");
    s.resize(cw, ch);
    // oracle: down enter sends the retry (the retry event is not drawn); the confirmation is gone.
    s.keys("down enter");
    s.assert_absent("Press enter to confirm or esc to go back");
}

// ORACLE chatwidget/tests/app_server.rs:366 safety_buffering_retry_confirmation_can_keep_waiting kind=ui owner=chat-live
#[test]
fn safety_buffering_retry_confirmation_can_keep_waiting() {
    kit::pending(Owner::ChatLive, SAFETY_PENDING);
    for key in ["enter", "esc"] {
        let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
        s.feed(&turn_started("turn-safety-buffering"));
        s.feed(&safety_buffering(
            "turn-safety-buffering",
            true,
            Some("faster-model"),
        ));
        s.keys("enter");
        s.assert_contains("Stop this attempt and retry?");
        // oracle: enter or esc closes the confirmation and the turn keeps waiting; no retry is sent (not drawn).
        s.keys(key);
        s.assert_absent("Stop this attempt and retry?");
    }
}

// ORACLE chatwidget/tests/app_server.rs:392 safety_buffering_retry_confirmation_closes_when_turn_completes kind=ui owner=chat-live
#[test]
fn safety_buffering_retry_confirmation_closes_when_turn_completes() {
    kit::pending(Owner::ChatLive, SAFETY_PENDING);
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&turn_started("turn-safety-buffering"));
    s.feed(&safety_buffering(
        "turn-safety-buffering",
        true,
        Some("faster-model"),
    ));
    s.keys("enter");
    s.assert_contains("Stop this attempt and retry?");
    s.feed(&turn_completed("turn-safety-buffering", "completed", "[]"));
    // oracle: the completed turn closes the confirmation, and a queued request does not reopen it.
    s.assert_absent("Stop this attempt and retry?");
}

// ORACLE chatwidget/tests/app_server.rs:420 safety_buffering_does_not_offer_retry_in_side_conversation kind=snap owner=chat-live snap=safety_buffering_side_conversation_without_retry
#[test]
fn safety_buffering_does_not_offer_retry_in_side_conversation() {
    kit::pending(
        Owner::ChatLive,
        "no safety-buffering state and no side-conversation state on the chat screen: the notice is not drawn, and a side conversation has no feed step",
    );
    let (w, h) = oracle_size("safety_buffering_side_conversation_without_retry", 80);
    let mut s = Screen::build(Owner::ChatLive).size(w, h).live().open();
    s.feed(&turn_started("turn-safety-buffering"));
    s.feed(&safety_buffering(
        "turn-safety-buffering",
        true,
        Some("faster-model"),
    ));
    // oracle: in a side conversation the notice is drawn without the retry option.
    s.assert_snap("safety_buffering_side_conversation_without_retry");
}

// ORACLE chatwidget/tests/app_server.rs:439 safety_buffering_retry_confirmation_closes_when_response_starts kind=ui owner=chat-live
#[test]
fn safety_buffering_retry_confirmation_closes_when_response_starts() {
    kit::pending(Owner::ChatLive, SAFETY_PENDING);
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&turn_started("turn-safety-buffering"));
    s.feed(&safety_buffering(
        "turn-safety-buffering",
        true,
        Some("faster-model"),
    ));
    s.keys("enter");
    s.assert_contains("Stop this attempt and retry?");
    s.feed(&agent_delta("turn-safety-buffering", "Visible response"));
    // oracle: the first visible text closes the confirmation and the notice.
    s.assert_absent("Stop this attempt and retry?");
    s.assert_absent(SAFETY_HEADER);
    let popup = s.shot();
    // oracle: later updates, with the buffering UI hidden or without a faster model, leave the view as it is.
    for (show_ui, faster) in [
        (true, Some("faster-model")),
        (true, None),
        (false, None),
        (true, Some("faster-model")),
    ] {
        s.feed(&safety_buffering("turn-safety-buffering", show_ui, faster));
        s.assert_same_rows(&popup);
    }
    s.feed(&turn_completed("turn-safety-buffering", "completed", "[]"));
    s.assert_absent(SAFETY_HEADER);
}

// ORACLE chatwidget/tests/app_server.rs:482 safety_buffering_without_retry_shows_short_app_message kind=snap owner=chat-live snap=safety_buffering_status_without_retry
#[test]
fn safety_buffering_without_retry_shows_short_app_message() {
    kit::pending(Owner::ChatLive, SAFETY_PENDING);
    let (w, h) = oracle_size("safety_buffering_status_without_retry", 80);
    let mut s = Screen::build(Owner::ChatLive).size(w, h).live().open();
    s.feed(&turn_started("turn-safety-buffering"));
    s.feed(&safety_buffering("turn-safety-buffering", true, None));
    s.assert_snap("safety_buffering_status_without_retry");
    let popup = s.shot();
    // oracle: the same turn with a faster model, sent again, leaves the short message as it is.
    s.feed(&safety_buffering(
        "turn-safety-buffering",
        true,
        Some("faster-model"),
    ));
    s.assert_same_rows(&popup);
    s.keys("enter");
    s.assert_absent("Press enter to confirm or esc to go back");
}

// ORACLE chatwidget/tests/app_server.rs:522 safety_buffering_ignores_hidden_stale_and_historical_updates kind=ui owner=chat-live
#[test]
fn safety_buffering_ignores_hidden_stale_and_historical_updates() {
    kit::pending(Owner::ChatLive, SAFETY_PENDING);
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&turn_started("turn-safety-buffering"));
    s.feed(&safety_buffering(
        "turn-safety-buffering",
        false,
        Some("faster-model"),
    ));
    s.feed(&safety_buffering("stale-turn", true, Some("faster-model")));
    // oracle: a hidden update and a stale turn draw nothing; the historical (replayed) update has no feed step yet.
    s.assert_absent(SAFETY_HEADER);
    s.feed(&safety_buffering(
        "turn-safety-buffering",
        true,
        Some("faster-model"),
    ));
    s.assert_contains(SAFETY_HEADER);
    s.keys("enter");
    s.feed(&safety_buffering(
        "turn-safety-buffering",
        false,
        Some("faster-model"),
    ));
    s.assert_absent(SAFETY_HEADER);
    s.assert_absent("Stop this attempt and retry?");
}

// ORACLE chatwidget/tests/app_server.rs:575 tool_suggestion_install_url_is_validated_before_opening kind=snap owner=chat-live snap=declined_tool_suggestion
#[test]
fn tool_suggestion_install_url_is_validated_before_opening() {
    // the oracle's snapshot is the bottom pane alone; ours has the blank row over the composer
    let (w, h) = oracle_size("declined_tool_suggestion", 80);
    let h = h + 2;
    let offered = "https://example.test/install";
    for install_url in [
        "file:///tmp/connector",
        "http://example.test/install",
        "custom://example.test/install",
        "not a URL",
        "https://user:password@example.test/install",
        offered,
    ] {
        // the offered link's dialog needs the room of a window, the declined ones the bottom pane's
        let rows = if install_url == offered { 24 } else { h };
        let mut s = Screen::build(Owner::ChatLive).size(w, rows).live().open();
        s.feed(IDLE_FOOTER);
        let initial = s.shot();
        s.feed(&tool_suggestion_request(install_url));
        if install_url == offered {
            // oracle: a valid https link is offered, and enter opens it in the browser.
            s.assert_contains("Test Connector");
            s.assert_contains("Install on ChatGPT");
            s.assert_opened_urls(&[]);
            s.assert_no_resolved_requests();
            s.keys("enter");
            s.assert_opened_urls(&[install_url]);
            continue;
        }
        // oracle: an invalid link is declined at once; the popup is the initial one and the decline is sent.
        s.assert_same_rows(&initial);
        s.assert_snap_rows_at("declined_tool_suggestion", 2);
        s.assert_opened_urls(&[]);
        s.assert_resolved_actions(&["decline"]);
    }
}

// ORACLE chatwidget/tests/app_server.rs:684 thread_settings_updated_updates_visible_state_without_transcript kind=ui owner=chat-live
#[test]
fn thread_settings_updated_updates_visible_state_without_transcript() {
    kit::pending(
        Owner::ChatLive,
        "a settings update on a chat that has no session draws the session header: the transcript shows its header as soon as ThreadPart.cwd is known (transcript_header.rs), and the settings carry the cwd; the oracle draws the header from the session configuration only",
    );
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    let before = s.shot();
    s.feed(&thread_settings_updated("gpt-5.4", "server-ollama"));
    // oracle: the update draws no transcript history (the state it changes is internal and not asserted on screen).
    s.assert_same_rows(&before);
}

// ORACLE chatwidget/tests/app_server.rs:790 collab_spawn_end_shows_requested_model_and_effort kind=ui owner=chat-live
#[test]
fn collab_spawn_end_shows_requested_model_and_effort() {
    kit::pending(Owner::ChatLive, COLLAB_PENDING);
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&item_started(&collab_item(
        "call-spawn",
        "spawnAgent",
        "inProgress",
        "[]",
        "{}",
        r#""prompt":"Explore the repo","model":"gpt-5","reasoningEffort":"high""#,
    )));
    s.feed(&item_completed(&collab_item(
        "call-spawn",
        "spawnAgent",
        "completed",
        r#"["thread-spawned"]"#,
        r#"{"thread-spawned":{"status":"pendingInit","message":null}}"#,
        r#""prompt":"Explore the repo","model":null,"reasoningEffort":null"#,
    )));
    // oracle: the spawn line names the agent (Robie, role explorer) with the requested model and effort.
    s.assert_contains("Spawned Robie [explorer] (gpt-5 high)");
}

// ORACLE chatwidget/tests/app_server.rs:860 live_app_server_user_message_item_completed_does_not_duplicate_rendered_prompt kind=ui owner=chat-live
#[test]
fn live_app_server_user_message_item_completed_does_not_duplicate_rendered_prompt() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.type_text("Hi, are you there?");
    s.keys("enter");
    s.assert_contains("Hi, are you there?");
    let before = s.shot();
    s.feed(&item_completed(&user_item(
        r#"[{"type":"text","text":"Hi, are you there?","textElements":[]}]"#,
    )));
    // oracle: the completed user message does not draw the prompt a second time.
    s.assert_same_rows(&before);
}

// ORACLE chatwidget/tests/app_server.rs:899 live_app_server_user_message_omits_unsupported_media kind=snap owner=chat-live snap=live_app_server_user_message_omits_unsupported_media
#[test]
fn live_app_server_user_message_omits_unsupported_media() {
    kit::pending(
        Owner::ChatLive,
        "no user-message cell drawn from a completed user message on the chat screen: the text input is not drawn yet, and the audio inputs are left out of it",
    );
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&item_completed(&user_item(
        r#"[{"type":"text","text":"Please inspect the attachments.","textElements":[]},{"type":"audio","audio":{"type":"inline","url":"https://example.com/one.wav"}},{"type":"localAudio","path":"/tmp/two.wav"}]"#,
    )));
    // oracle: the cell keeps the text and omits the unsupported audio (one cell, at the top of the transcript).
    s.assert_snap_region("live_app_server_user_message_omits_unsupported_media", 0, 0);
}

// ORACLE chatwidget/tests/app_server.rs:939 live_app_server_turn_completed_clears_working_status_after_answer_item kind=ui owner=chat-live
#[test]
fn live_app_server_turn_completed_clears_working_status_after_answer_item() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&turn_started("turn-1"));
    s.assert_contains("Working");
    s.feed(&item_completed(&agent_item("Yes. What do you need?")));
    s.assert_contains("Yes. What do you need?");
    s.assert_contains("Working");
    s.feed(&turn_completed(
        "turn-1",
        "completed",
        &format!("[{}]", agent_item("Yes. What do you need?")),
    ));
    // oracle: the completed turn clears the working status and adds the Worked for line.
    s.assert_absent("Working");
    s.assert_contains("Worked for");
}

// ORACLE chatwidget/tests/app_server.rs:1027 live_app_server_turn_started_sets_feedback_turn_id kind=ui owner=chat-live
#[test]
fn live_app_server_turn_started_sets_feedback_turn_id() {
    kit::na(
        Owner::ChatLive,
        "asserts the SubmitFeedback event (category, reason, the turn id and the destination) that Enter sends; none of it is drawn",
    );
}

// ORACLE chatwidget/tests/app_server.rs:1069 live_app_server_warning_notification_renders_message kind=snap owner=chat-live
#[test]
fn live_app_server_warning_notification_renders_message() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&notify(
        "warning",
        r#"{"threadId":null,"message":"Exceeded skills context budget of 2%. All skill descriptions were removed and 2 additional skills were not included in the model-visible skills list."}"#,
    ));
    // oracle: the warning is a hidden cell the footer counts; F2 opens it in the warnings panel.
    s.keys("f2");
    s.assert_reads("Exceeded skills context budget of 2%.");
    s.assert_reads("All skill descriptions were removed and 2 additional skills were not included in the model-visible skills list.");
    s.keys("esc");
    s.feed(&notify(
        "modelProvider/authRecoveryStarted",
        r#"{"threadId":"thread-1","turnId":"turn-1","provider":"example","message":"Session expired. Sign in again."}"#,
    ));
    s.feed(&notify(
        "modelProvider/authRecoveryCompleted",
        r#"{"threadId":"thread-1","turnId":"turn-1","provider":"example","message":"Signed in successfully."}"#,
    ));
    // oracle: the two recovery cells, one after the other.
    s.assert_in_order(&[
        "• Session expired. Sign in again.",
        "✓ Signed in successfully.",
    ]);
}

// ORACLE chatwidget/tests/app_server.rs:1123 live_app_server_guardian_warning_notification_renders_message kind=ui owner=chat-live
#[test]
fn live_app_server_guardian_warning_notification_renders_message() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&notify(
        "guardianWarning",
        r#"{"threadId":"thread-1","message":"Automatic approval review denied the requested action."}"#,
    ));
    s.keys("f2");
    s.assert_reads("Automatic approval review denied the requested action.");
}

// ORACLE chatwidget/tests/app_server.rs:1144 live_app_server_strict_review_required_notification_renders_message kind=snap owner=chat-live snap=strict_review_required
#[test]
fn live_app_server_strict_review_required_notification_renders_message() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&turn_started("turn-1"));
    s.feed(&item_started(&exec_item(
        "cmd-1",
        r#"printf 'streamed output\\n'"#,
        "inProgress",
        "null",
    )));
    s.feed(&notify(
        "autoApprovalReview/strictReviewRequired",
        r#"{"threadId":"thread-1","turnId":"turn-1","startedAtMs":1000}"#,
    ));
    // oracle: the warning cell (the detailed transcript's), then the streamed output stays in the active command, and the task is still running.
    s.keys("ctrl+t");
    s.assert_snap_region("strict_review_required", row_of(&s, "⚠"), 0);
    s.keys("esc");
    s.feed(&notify(
        "item/commandExecution/outputDelta",
        r#"{"threadId":"thread-1","turnId":"turn-1","itemId":"cmd-1","delta":"streamed output\n"}"#,
    ));
    s.assert_contains("streamed output");
    s.assert_contains("Working");
}

// ORACLE chatwidget/tests/app_server.rs:1177 config_warning_during_turn_retains_transcript_details kind=snap owner=chat-live
#[test]
fn config_warning_during_turn_retains_transcript_details() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&turn_started("turn-1"));
    s.feed(&notify(
        "configWarning",
        r#"{"summary":"Invalid configuration; using defaults.","details":"Check the project configuration.","path":null,"range":null}"#,
    ));
    // oracle: the config warning cell keeps its details line.
    s.keys("ctrl+t");
    s.assert_snap_region("runtime_config_warning", row_of(&s, "⚠"), 0);
}

// ORACLE chatwidget/tests/app_server.rs:1200 sqlite_recovery_warning_shows_backup_and_metadata_limitations kind=snap owner=chat-live
#[test]
fn sqlite_recovery_warning_shows_backup_and_metadata_limitations() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&notify(
        "configWarning",
        r#"{"summary":"Codex rebuilt its local database","details":"Damaged local databases were rebuilt. Saved conversations remain in rollout files and can restore the thread list and history. Some database-only metadata may be unavailable. The original database files were preserved at the backup locations below.\n\nDatabase path: /codex/state_5.sqlite\nBackup folder: /codex/db-backups/recovery","path":null,"range":null}"#,
    ));
    // oracle: the recovery warning shows the backup folder and the metadata limits.
    s.keys("ctrl+t");
    s.assert_snap_region("sqlite_recovery_warning", row_of(&s, "⚠"), 0);
}

// ORACLE chatwidget/tests/app_server.rs:1222 startup_config_warning_is_not_repeated_by_thread kind=snap owner=chat-live snap=startup_config_warning_panel
#[test]
fn startup_config_warning_is_not_repeated_by_thread() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    let message = "Codex is ignoring 1 unrecognized configuration setting. Check for typos or deprecated settings.";
    s.feed(&notify(
        "configWarning",
        &format!(r#"{{"summary":"{message}","details":null,"path":null,"range":null}}"#),
    ));
    s.feed(&notify(
        "warning",
        &format!(r#"{{"threadId":"thread-1","message":"{message}"}}"#),
    ));
    s.feed(&notify(
        "configWarning",
        r#"{"summary":"Second config warning","details":"A different setting needs attention.","path":null,"range":null}"#,
    ));
    // oracle: without a local daemon the panel shows the warning once, under its label; the second warning is one step right.
    s.keys("f2");
    s.assert_reads(message);
    // The snapshot's two frames hang on the daemon state, which is not fed; the first frame's
    // title and text (Missed: the panel's layout, as in sqlite_log_write_warning).
    s.assert_contains("Warnings · 1 of 2 · Startup");
    s.keys("right");
    s.assert_contains("Second config warning");
    s.assert_absent("/daemon");
    s.keys("esc");
}

// ORACLE chatwidget/tests/app_server.rs:1294 live_app_server_file_change_item_started_preserves_changes kind=snap owner=chat-live
#[test]
fn live_app_server_file_change_item_started_preserves_changes() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&turn_started("turn-1"));
    s.feed(&item_started(
        r#"{"type":"fileChange","id":"patch-1","changes":[{"path":"foo.txt","kind":{"type":"add"},"diff":"hello\n"}],"status":"inProgress"}"#,
    ));
    // oracle: the patch cell shows the added file and its one-line diff; a replay (resume) keeps it, which has no feed step yet.
    s.assert_in_order(&["• Added foo.txt (+1 -0)", "1 +hello"]);
}

// ORACLE chatwidget/tests/app_server.rs:1341 live_app_server_command_execution_strips_shell_wrapper kind=snap owner=chat-live snap=live_app_server_command_execution_strips_shell_wrapper
#[test]
fn live_app_server_command_execution_strips_shell_wrapper() {
    kit::pending(
        Owner::ChatLive,
        "no command cell on the chat screen: the chat model keeps only user and agent message items (core/src/threads.rs item_upsert), so the completed command is not drawn",
    );
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    let command = r#"/bin/zsh -lc 'python3 -c '\"'\"'print(\"Hello, world!\")'\"'\"''"#;
    s.feed(&item_started(&exec_item(
        "cmd-1",
        command,
        "inProgress",
        "null",
    )));
    s.feed(&item_completed(&exec_item(
        "cmd-1",
        command,
        "completed",
        r#""Hello, world!\n""#,
    )));
    // oracle: the completed cell shows the script without the zsh wrapper.
    s.assert_snap_region(
        "live_app_server_command_execution_strips_shell_wrapper",
        0,
        0,
    );
}

// ORACLE chatwidget/tests/app_server.rs:1415 live_app_server_command_output_delta_transcript_snapshot kind=snap owner=chat-live snap=live_app_server_command_output_delta_active snap=live_app_server_command_output_delta_interrupted snap=live_app_server_command_output_delta_transcript
#[test]
fn live_app_server_command_output_delta_transcript_snapshot() {
    kit::pending(
        Owner::ChatLive,
        "no command cell with streamed output on the chat screen: the chat model keeps only user and agent message items, and the transcript view has no key in the port yet",
    );
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&turn_started("turn-1"));
    s.feed(&item_started(&exec_item(
        "cmd-1",
        r#"printf 'stdout\\nstderr\\n'"#,
        "inProgress",
        "null",
    )));
    s.feed(&notify(
        "item/commandExecution/outputDelta",
        r#"{"threadId":"thread-1","turnId":"turn-1","itemId":"cmd-1","delta":"stdout\n"}"#,
    ));
    s.feed(&notify(
        "item/commandExecution/outputDelta",
        r#"{"threadId":"thread-1","turnId":"turn-1","itemId":"cmd-1","delta":"stderr\n"}"#,
    ));
    // oracle: the active command shows both output lines.
    s.assert_snap_region("live_app_server_command_output_delta_active", 0, 0);
    // The transcript view of the same command (live_app_server_command_output_delta_transcript) has no key or step in the port yet.
    s.feed(&turn_completed("turn-1", "interrupted", "[]"));
    // oracle: the interrupted command in history, its duration shown as <duration>; the kit does not normalise durations yet.
    s.assert_snap_region("live_app_server_command_output_delta_interrupted", 0, 0);
}

// ORACLE chatwidget/tests/app_server.rs:1466 live_app_server_sub_agent_activity_renders_once kind=snap owner=chat-live snap=app_server_sub_agent_activity_renders_once
#[test]
fn live_app_server_sub_agent_activity_renders_once() {
    kit::pending(
        Owner::ChatLive,
        "no sub-agent activity cell on the chat screen: the chat model keeps only user and agent message items (core/src/threads.rs item_upsert), so the activity line is not drawn",
    );
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    let activity = r#"{"type":"subAgentActivity","id":"activity-1","kind":"completed","agentThreadId":"thread-agent","agentPath":"/root/researcher","model":null,"reasoningEffort":null}"#;
    s.feed(&item_started(activity));
    s.feed(&item_completed(activity));
    // oracle: the completed activity renders once, from its started and its completed item.
    s.assert_snap_region("app_server_sub_agent_activity_renders_once", 0, 0);
}

// ORACLE chatwidget/tests/app_server.rs:1504 live_app_server_collab_wait_items_render_history kind=snap owner=chat-live snap=app_server_collab_wait_items_render_history
#[test]
fn live_app_server_collab_wait_items_render_history() {
    kit::pending(Owner::ChatLive, COLLAB_PENDING);
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    let receivers =
        r#"["019cff70-2599-75e2-af72-b958ce5dc1cc","019cff70-2599-75e2-af72-b96db334332d"]"#;
    s.feed(&item_started(&collab_item(
        "wait-1",
        "wait",
        "inProgress",
        receivers,
        "{}",
        r#""prompt":null,"model":null,"reasoningEffort":null"#,
    )));
    s.feed(&item_completed(&collab_item(
        "wait-1",
        "wait",
        "completed",
        receivers,
        r#"{"019cff70-2599-75e2-af72-b958ce5dc1cc":{"status":"completed","message":"Done"},"019cff70-2599-75e2-af72-b96db334332d":{"status":"running","message":null}}"#,
        r#""prompt":null,"model":null,"reasoningEffort":null"#,
    )));
    // oracle: the wait line lists both agents, one done and one still running.
    s.assert_snap_region("app_server_collab_wait_items_render_history", 0, 0);
}

// ORACLE chatwidget/tests/app_server.rs:1594 live_app_server_collab_spawn_completed_renders_requested_model_and_effort kind=snap owner=chat-live snap=app_server_collab_spawn_completed_renders_requested_model_and_effort
#[test]
fn live_app_server_collab_spawn_completed_renders_requested_model_and_effort() {
    kit::pending(Owner::ChatLive, COLLAB_PENDING);
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&item_started(&collab_item(
        "spawn-1",
        "spawnAgent",
        "inProgress",
        "[]",
        "{}",
        r#""prompt":"Explore the repo","model":"gpt-5","reasoningEffort":"high""#,
    )));
    s.feed(&item_completed(&collab_item(
        "spawn-1",
        "spawnAgent",
        "completed",
        r#"["019cff70-2599-75e2-af72-b91781b41a8e"]"#,
        r#"{"019cff70-2599-75e2-af72-b91781b41a8e":{"status":"pendingInit","message":null}}"#,
        r#""prompt":"Explore the repo","model":"gpt-5","reasoningEffort":"high""#,
    )));
    // oracle: the spawn line carries the requested model and effort.
    s.assert_snap_region(
        "app_server_collab_spawn_completed_renders_requested_model_and_effort",
        0,
        0,
    );
}

// ORACLE chatwidget/tests/app_server.rs:1660 live_app_server_failed_turn_does_not_duplicate_error_history kind=ui owner=chat-live
#[test]
fn live_app_server_failed_turn_does_not_duplicate_error_history() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&turn_started("turn-1"));
    s.feed(&error_note("turn-1", "permission denied", "null", false));
    s.assert_contains("permission denied");
    let before = s.shot();
    s.feed(&turn_failed("turn-1", "permission denied"));
    // oracle: the failed turn draws no second error, and the task stops.
    s.assert_same_rows(&before);
    s.assert_absent("Working");
}

// ORACLE chatwidget/tests/app_server.rs:1763 live_app_server_turn_completion_repairs_dropped_message_deltas kind=snap owner=chat-live snap=live_app_server_turn_completion_repairs_dropped_message_deltas
#[test]
fn live_app_server_turn_completion_repairs_dropped_message_deltas() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&turn_started("turn-1"));
    s.feed(&agent_delta("turn-1", r#"The transport kept this.\n"#));
    s.feed(&turn_completed(
        "turn-1",
        "completed",
        &format!(
            "[{}]",
            agent_item(
                r#"The transport kept this.\nAnd dropped this.\n\n::code-comment{title=\"Finding\" body=\"Keep ::git-stage{cwd=/tmp} literal.\" file=\"/tmp/file.rs\"}"#
            )
        ),
    ));
    // oracle: the completion adds the text the deltas dropped; the answer's question prompt is answered (internal, not drawn).
    s.assert_contains("And dropped this.");
}

// ORACLE chatwidget/tests/app_server.rs:1881 live_app_server_stream_recovery_restores_previous_status_header kind=snap owner=chat-live
#[test]
fn live_app_server_stream_recovery_restores_previous_status_header() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&turn_started("turn-1"));
    s.feed(&error_note(
        "turn-1",
        "Reconnecting... 2/5",
        r#"{"responseStreamDisconnected":{"httpStatusCode":429}}"#,
        true,
    ));
    s.assert_contains("Server is busy, reconnecting 2/5");
    s.feed(&agent_delta("turn-1", "hello"));
    // oracle: the first answer text restores the Working header and clears the retry details.
    s.assert_contains("Working");
    s.assert_absent("Server is busy, reconnecting 2/5");
}

// ORACLE chatwidget/tests/app_server.rs:1946 live_app_server_errors_render_upstream_message kind=snap owner=chat-live snap=rate_limit_exceeded_error snap=session_budget_exceeded_error
#[test]
fn live_app_server_errors_render_upstream_message() {
    for (message, info, name) in [
        (
            "rate limit exceeded: Please try again in 10s.",
            "\"rateLimitExceeded\"",
            "rate_limit_exceeded_error",
        ),
        (
            "budget reached",
            "\"sessionBudgetExceeded\"",
            "session_budget_exceeded_error",
        ),
    ] {
        let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
        s.feed(&turn_started("turn-1"));
        s.feed(&error_note("turn-1", message, info, false));
        // oracle: the error cell carries the upstream message text.
        s.assert_snap_region(name, row_of(&s, "■"), 0);
    }
}

// ORACLE chatwidget/tests/app_server.rs:1979 live_app_server_server_overloaded_error_renders_error kind=snap owner=chat-live
#[test]
fn live_app_server_server_overloaded_error_renders_error() {
    for (message, info, expected) in [
        (
            "server overloaded",
            "\"serverOverloaded\"",
            "■ server overloaded",
        ),
        (
            "server overloaded",
            "\"policyCapacityUnavailable\"",
            "■ server overloaded",
        ),
        (
            "",
            "\"serverOverloaded\"",
            "■ Codex is currently experiencing high load.",
        ),
        (
            "",
            "\"policyCapacityUnavailable\"",
            "■ Codex is currently experiencing high load.",
        ),
    ] {
        let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
        s.feed(&turn_started("turn-1"));
        s.feed(&error_note("turn-1", message, info, false));
        // oracle: one error cell; an empty message shows the high-load wording.
        s.assert_has_row(expected);
    }
}

// ORACLE chatwidget/tests/app_server.rs:2040 live_app_server_cyber_policy_error_renders_dedicated_notice kind=ui owner=chat-live
#[test]
fn live_app_server_cyber_policy_error_renders_dedicated_notice() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&turn_started("turn-1"));
    s.feed(&error_note(
        "turn-1",
        "server fallback message",
        "\"cyberPolicy\"",
        false,
    ));
    // oracle: the dedicated cyber notice replaces the server's message.
    s.assert_contains("This content can’t be shown");
    s.assert_contains("We take extra care with some cybersecurity requests");
    s.assert_absent("server fallback message");
}

// ORACLE chatwidget/tests/app_server.rs:2087 app_server_safety_access_errors_render_dedicated_notice kind=snap owner=chat-live
#[test]
fn app_server_safety_access_errors_render_dedicated_notice() {
    let mut reference = None;
    for message in [
        "Invalid prompt: we've limited access to this content for safety reasons.",
        r#"{\"error\":{\"message\":\"Invalid prompt: we've limited access to this content for safety reasons.\"}}"#,
        "This content was flagged for possible biological risk.",
        r#"{\"error\":{\"message\":\"This content was flagged for possible biological risk.\"}}"#,
        r#"{\"error\":{\"code\":\"bio_policy\",\"message\":\"copy may change\"}}"#,
    ] {
        let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
        s.feed(&error_note("turn-1", message, "null", false));
        // oracle: every case draws the same dedicated notice, the last one being the snapshot.
        s.assert_contains("This content can't be shown");
        s.assert_contains("biological research");
        s.assert_snap_region(
            "app_server_bio_policy_error_renders_dedicated_notice",
            row_of(&s, "ⓘ"),
            0,
        );
        if let Some(first) = &reference {
            s.assert_same_rows(first);
        } else {
            reference = Some(s.shot());
        }
    }
}

// ORACLE chatwidget/tests/app_server.rs:2130 live_app_server_model_verification_renders_warning kind=ui owner=chat-live
#[test]
fn live_app_server_model_verification_renders_warning() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&notify(
        "model/verification",
        r#"{"threadId":"thread-1","turnId":"turn-1","verifications":["trustedAccessForCyber"]}"#,
    ));
    s.keys("f2");
    for part in [
        "multiple flags for possible cybersecurity risk",
        "extra safety checks are on",
        "Trusted Access for Cyber",
        "https://chatgpt.com/cyber",
    ] {
        s.assert_reads(part);
    }
}

// ORACLE chatwidget/tests/app_server.rs:2173 live_app_server_manual_thread_name_updates_status_surfaces kind=snap owner=chat-live snap=manual_thread_name_status_surfaces
#[test]
fn live_app_server_manual_thread_name_updates_status_surfaces() {
    kit::pending(
        Owner::ChatLive,
        "the name is applied (ThreadPart.name, read by status_surfaces.rs) but the oracle's snapshot has the sections `transcript:` and `footer:` and a `title:` line, which the kit's frame parser does not read (no frame `transcript`); the footer and the title are the chat-status chunk's",
    );
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&notify(
        "thread/name/updated",
        r#"{"threadId":"123e4567-e89b-12d3-a456-426614174000","threadName":"review-fix"}"#,
    ));
    // oracle: the transcript line, the footer and the title carry the new name.
    s.assert_snap_section("manual_thread_name_status_surfaces", "transcript");
    s.assert_snap_section("manual_thread_name_status_surfaces", "footer");
    s.assert_title("review-fix | project");
}

// ORACLE chatwidget/tests/app_server.rs:2208 live_app_server_automatic_thread_name_update_is_silent kind=snap owner=chat-live
#[test]
fn live_app_server_automatic_thread_name_update_is_silent() {
    kit::pending(
        Owner::ChatLive,
        "the generated name is applied and drawn by the status surfaces (status_surfaces.rs reads ThreadPart.name), so the whole-screen comparison differs; the oracle's test compares the transcript only, and the replay kinds have no feed step",
    );
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    let before = s.shot();
    s.feed(&notify(
        "thread/name/updated",
        r#"{"threadId":"thread-1","threadName":"Generated title"}"#,
    ));
    // oracle: a generated name draws nothing in the transcript (live, snapshot and resume replays alike; the replay kinds have no feed step yet).
    s.assert_same_rows(&before);
}

// ORACLE chatwidget/tests/app_server.rs:2268 thread_name_suggestion_requires_matching_thread_and_request kind=ui owner=chat-live
#[test]
fn thread_name_suggestion_requires_matching_thread_and_request() {
    kit::pending(
        Owner::ChatLive,
        "no thread-rename prompt on the chat screen: a rename with a pending name suggestion has no key or feed step, so its loading and suggested rows are not drawn",
    );
    let s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    // oracle: the rename prompt shows Loading until the suggestion for its own thread and request arrives; the others are ignored.
    s.assert_contains("Loading");
    s.assert_absent("Wrong");
    // The matching suggestion (no feed step yet) fills the prompt with its title.
    s.assert_contains("Ready");
    s.assert_contains("Suggested title");
}

// ORACLE chatwidget/tests/app_server.rs:2337 sqlite_log_write_warning_is_visible_in_warnings kind=snap owner=chat-live
#[test]
fn sqlite_log_write_warning_is_visible_in_warnings() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    // oracle: the test marks startup complete; a turn starting does.
    s.feed(&turn_started("turn-1"));
    s.feed(&notify(
        "warning",
        r#"{"threadId":null,"message":"Codex couldn't save diagnostic logs to its local database. Use /feedback with logs included before closing Codex, or run `codex doctor` for diagnostics."}"#,
    ));
    // oracle: the warning is listed in the warnings panel at 80 columns (the panel opens after startup: no key or step yet).
    s.keys("f2");
    // The oracle's panel is a bottom popup as wide as the screen; the port's is a modal sheet of
    // its own width over the chat, so its rows wrap differently: the title, the text and the keys
    // are compared, not the rows (Missed: the layout of the panel, small_dialogs' sheet).
    s.assert_contains("Warnings · 1 of 1 · Warning");
    s.assert_reads("Codex couldn't save diagnostic logs to its local database. Use /feedback with logs included before closing Codex, or run `codex doctor` for diagnostics.");
    s.assert_contains("k keep & next · esc dismiss & close · ⌃o copy · ←/→ warning");
}
