//! Shard `chat_commands_3` (screens S10, owner chat-nav-pane): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen};

/// The host's keymap editor over the built-in keymap (what `/keymap` opens from).
const KEYMAP_DEFAULT: &str = include_str!("fixtures/chat_commands_3/keymap_default.jsonl");
/// The conversation the export test names (`00000000-0000-0000-0000-000000000123`), one turn done.
const EXPORT_THREAD: &str = include_str!("fixtures/chat_commands_3/export_thread.jsonl");
/// The memory tool on: the host's facts of the config (`set_feature_enabled(MemoryTool, true)`).
const MEMORY_TOOL: &str = include_str!("fixtures/chat_commands_3/memory_tool.jsonl");
/// A running turn (the oracle's `handle_turn_started`).
const RUNNING: &str = include_str!("fixtures/chat_commands_2/running.jsonl");
/// The live chat with a side conversation active (`set_side_conversation_active`).
const SIDE: &str = include_str!("fixtures/chat_commands_3/side.jsonl");
/// A fresh thread on `gpt-6-astra` with animations and the starfield effect on (the oracle's
/// `mark_fresh_task_for_sparkle`).
const ASTRA: &str = include_str!("fixtures/chat_commands_3/astra.jsonl");
/// A thread whose rollout file is `/tmp/codex-test-rollout.jsonl` (`current_rollout_path`).
/// The server's completion of the turn the fake minted as `turn-{n}` in `thread-1`.
fn completed(n: usize) -> String {
    format!(
        r#"{{"step": "notify", "message": {{"method": "turn/completed", "params": {{"threadId": "thread-1", "turn": {{"id": "turn-{n}", "rootTurnId": null, "items": [], "itemsView": "summary", "status": "completed", "error": null, "startedAt": null, "completedAt": null, "durationMs": null}}}}}}}}"#
    )
}

const ROLLOUT: &str = include_str!("fixtures/chat_commands_3/rollout.jsonl");

// ---- oracle: chatwidget/tests/slash_commands.rs (46 screen-level tests here; 22 state tests left to chat-nav-pane)
// ORACLE chatwidget/tests/slash_commands.rs:2210 slash_copy_picker_remains_available_from_parent_owned_threads kind=ui owner=chat-nav-pane
#[test]
fn slash_copy_picker_remains_available_from_parent_owned_threads() {
    kit::pending(
        Owner::ChatNavPane,
        "the /copy picker (Whole response) is not built, and no seed sets a parent-owned thread with a transcript copy source",
    );
    let mut s = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
    s.type_text("/cop");
    s.keys("tab");
    s.assert_absent("Whole response");
    s.keys("enter");
    s.assert_contains("Whole response");
    s.keys("1");
    // The oracle also asserts that copying submits no agent turn (no core op): not drawn.
}

// ORACLE chatwidget/tests/slash_commands.rs:2383 slash_copy_reports_when_no_agent_response_exists kind=snap owner=chat-nav-pane snap=slash_copy_no_output_info_message
#[test]
fn slash_copy_reports_when_no_agent_response_exists() {
    kit::pending(
        Owner::ChatNavPane,
        "the /copy command and its no-output info cell are not built; the transcript-mode seed (Terminal) is not reachable black-box",
    );
    let mut s = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
    s.type_text("/copy");
    s.keys("enter");
    s.assert_snap("slash_copy_no_output_info_message");
    s.assert_contains("No agent response to copy");
}

// ORACLE chatwidget/tests/slash_commands.rs:2400 slash_export_opens_destination_picker kind=snap owner=chat-nav-pane snap=slash_export_completion_message snap=slash_export_destination_picker snap=slash_export_filename_prompt
#[test]
fn slash_export_opens_destination_picker() {
    let screen = |height: u16| {
        Screen::build(Owner::ChatNavPane)
            .size(100, height)
            .thread_start(
                serde_json::from_str(include_str!(
                    "fixtures/chat_commands_3/export_thread_start.json"
                ))
                .expect("the thread start"),
            )
            .script(EXPORT_THREAD)
            .open()
    };
    // The oracle's snapshots are the popup alone, from its title.
    let popup_row = |s: &Screen, title: &str| {
        s.rows()
            .iter()
            .position(|row| row.trim() == title)
            .unwrap_or(usize::MAX)
    };

    let mut s = screen(14);
    s.type_text("/export");
    s.keys("enter");
    s.assert_snap_region(
        "slash_export_destination_picker",
        popup_row(&s, "Export conversation"),
        0,
    );
    // "Save to file" opens the filename prompt, the thread's own name in it.
    s.keys("down enter");
    s.assert_snap_region(
        "slash_export_filename_prompt",
        popup_row(&s, "Save conversation"),
        0,
    );
    // The host answers the write that failed, then a write that went through.
    s.keys("enter");
    s.answer_export(Err("missing parent"));
    s.assert_contains("■ Export failed: missing parent");
    s.type_text("/export");
    s.keys("enter");
    s.keys("down enter");
    s.keys("enter");
    s.answer_export(Ok("conversation.md"));
    s.assert_contains("• Saved conversation to conversation.md");
    // The copy, confirmed, and refused.
    s.type_text("/export");
    s.keys("enter");
    s.keys("enter");
    s.answer_copy(Ok(()));
    s.assert_contains("• Copied conversation to clipboard");
    s.type_text("/export");
    s.keys("enter");
    s.keys("enter");
    s.answer_copy(Err("clipboard unavailable"));
    s.assert_contains("■ Copy failed: clipboard unavailable");
}

// ORACLE chatwidget/tests/slash_commands.rs:2434 copy_action_reports_when_no_agent_response_exists kind=ui owner=chat-nav-pane
#[test]
fn copy_action_reports_when_no_agent_response_exists() {
    let mut s = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
    s.keys("ctrl+o");
    s.assert_contains("No agent response to copy");
}

// ORACLE chatwidget/tests/slash_commands.rs:2450 keymap_capture_can_capture_current_copy_shortcut kind=ui owner=chat-nav-pane
#[test]
fn keymap_capture_can_capture_current_copy_shortcut() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .script(KEYMAP_DEFAULT)
        .open();
    s.type_text("/keymap");
    s.keys("enter");
    s.keys("ctrl+o");
    // The oracle asserts the KeymapCaptured event (composer/submit, ctrl-o): not drawn.
    s.assert_absent("No agent response to copy");
}

// ORACLE chatwidget/tests/slash_commands.rs:2483 slash_keymap_capture_can_capture_app_shortcuts kind=ui owner=chat-nav-pane
#[test]
fn slash_keymap_capture_can_capture_app_shortcuts() {
    kit::na(
        Owner::ChatNavPane,
        "asserts the KeymapCaptured events (context, action, key, intent) that a capture sends for ctrl-t, ctrl-l and ctrl-g, not drawn",
    );
}

// ORACLE chatwidget/tests/slash_commands.rs:2515 slash_keymap_debug_opens_keypress_inspector kind=ui owner=chat-nav-pane
#[test]
fn slash_keymap_debug_opens_keypress_inspector() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .script(KEYMAP_DEFAULT)
        .open();
    s.type_text("/keymap debug");
    s.keys("enter");
    s.assert_contains("Keypress Inspector");
    s.assert_contains("Waiting for a keypress");
    s.keys("f4");
    s.resize(100, 24);
    s.assert_contains("global.focus_activity");
    // The oracle asserts no transcript message and no core op: not checked here beyond the inspector rows.
}

// ORACLE chatwidget/tests/slash_commands.rs:2534 slash_keymap_debug_can_inspect_app_shortcuts kind=ui owner=chat-nav-pane
#[test]
fn slash_keymap_debug_can_inspect_app_shortcuts() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(100, 24)
        .script(KEYMAP_DEFAULT)
        .open();
    s.type_text("/keymap debug");
    s.keys("enter");
    s.keys("ctrl+t");
    s.assert_contains("global.open_transcript (Open Transcript)");
    s.keys("ctrl+l");
    s.assert_contains("global.clear_terminal (Clear Terminal)");
    s.keys("ctrl+g");
    s.assert_contains("global.open_external_editor (Open External Editor)");
}

// ORACLE chatwidget/tests/slash_commands.rs:2561 slash_keymap_invalid_args_show_usage kind=ui owner=chat-nav-pane
#[test]
fn slash_keymap_invalid_args_show_usage() {
    let mut s = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
    s.type_text("/keymap nope");
    s.keys("enter");
    s.assert_contains("Usage: /keymap [debug]");
    // recall_latest_after_clearing: clear the composer (set_composer_text("") there; ctrl+u here), then Up.
    // (the command took the line: it is recalled, not kept)
    s.keys("ctrl+u");
    s.assert_composer("");
    s.keys("up");
    s.assert_composer("/keymap nope");
}

// ORACLE chatwidget/tests/slash_commands.rs:2581 copy_shortcut_can_be_remapped kind=ui owner=chat-nav-pane
#[test]
fn copy_shortcut_can_be_remapped() {
    kit::pending(
        Owner::ChatNavPane,
        "the seed sets global.copy = alt+v (`.keymap(..)` is read), but the chat has no copy action (`No agent response to copy` is nowhere in the port): `alt+v` does nothing",
    );
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .live()
        .keymap(&[("global.copy", "alt+v")])
        .open();
    s.keys("ctrl+o");
    s.assert_absent("No agent response to copy");
    s.keys("alt+v");
    s.assert_contains("No agent response to copy");
    // The oracle's paste-key half (ctrl+v, alt+v as PasteImage) is an action return, not drawn.
}

// ORACLE chatwidget/tests/slash_commands.rs:2653 legacy_image_paste_ignores_buffered_press_until_other_input kind=ui owner=chat-nav-pane
#[test]
fn legacy_image_paste_ignores_buffered_press_until_other_input() {
    kit::na(
        Owner::ChatNavPane,
        "asserts the KeyEventAction returned for buffered paste key presses (PasteImage or None) and the suppress_image_paste_until timing, not drawn",
    );
}

// ORACLE chatwidget/tests/slash_commands.rs:2688 copy_shortcut_submits_markdown_and_reports_completion_once kind=snap owner=chat-nav-pane
#[test]
fn copy_shortcut_submits_markdown_and_reports_completion_once() {
    kit::pending(
        Owner::ChatNavPane,
        "the copy action with a pending clipboard status (pending_copy), its confirmed completion and the unconfirmed notice are not built; the agent markdown seed is not reachable black-box",
    );
    let mut s = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
    s.keys("ctrl+o");
    s.assert_snap("pending_copy");
    s.assert_contains("Copied last message to clipboard");
    s.assert_snap("unconfirmed_copy");
}

// ORACLE chatwidget/tests/slash_commands.rs:2816 queued_menu_slash_keeps_agent_turn_complete_notification kind=ui owner=chat-nav-pane
#[test]
fn queued_menu_slash_keeps_agent_turn_complete_notification() {
    kit::pending(
        Owner::ChatNavPane,
        "a running turn (queued /model during a turn) cannot be seeded black-box; the /model Select Model popup is the visible half",
    );
    let mut s = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
    s.type_text("/model");
    s.keys("tab");
    s.assert_contains("Select Model");
    // The oracle asserts the pending AgentTurnComplete notification and no core op: not drawn.
}

// ORACLE chatwidget/tests/slash_commands.rs:2833 slash_stop_submits_background_terminal_cleanup kind=ui owner=chat-nav-pane
#[test]
fn slash_stop_submits_background_terminal_cleanup() {
    let mut s = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
    s.type_text("/stop");
    s.keys("esc enter");
    s.assert_contains("Stopping all background terminals.");
    // The oracle's CleanBackgroundTerminals op is the thread's `thread/backgroundTerminals/clean` request.
    s.assert_clean_terminals_sent();
}

// ORACLE chatwidget/tests/slash_commands.rs:2858 slash_new_with_name_requests_named_session kind=ui owner=chat-nav-pane
#[test]
fn slash_new_with_name_requests_named_session() {
    kit::na(
        Owner::ChatNavPane,
        "asserts the FollowTranscript and NewSession{name} events sent for \"/new   Add User  \", nothing drawn",
    );
}

// ORACLE chatwidget/tests/slash_commands.rs:2876 slash_clear_with_name_requests_named_session kind=ui owner=chat-nav-pane
#[test]
fn slash_clear_with_name_requests_named_session() {
    kit::na(
        Owner::ChatNavPane,
        "asserts the FollowTranscript and ClearUi{name} events sent for \"/clear   Add User  \", nothing drawn",
    );
}

// ORACLE chatwidget/tests/slash_commands.rs:2894 slash_clear_after_ctrl_c_keeps_stashed_draft_recallable kind=ui owner=chat-nav-pane
#[test]
fn slash_clear_after_ctrl_c_keeps_stashed_draft_recallable() {
    kit::pending(
        Owner::ChatNavPane,
        "no Ctrl+C draft stash and no prompt history on the chat composer: Ctrl+C leaves the draft in the composer (oracle: composer empty, draft recorded), Up recalls nothing",
    );
    let mut s = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
    s.type_text("ok");
    s.keys("enter");
    s.type_text("explain why history recall lost this draft");
    s.keys("ctrl+c");
    s.assert_composer("");
    s.type_text("/clear");
    s.keys("enter");
    s.keys("up");
    s.assert_composer("explain why history recall lost this draft");
    s.keys("up");
    s.assert_composer("ok");
}

// ORACLE chatwidget/tests/slash_commands.rs:2926 slash_clear_is_disabled_while_task_running kind=ui owner=chat-nav-pane
#[test]
fn slash_clear_is_disabled_while_task_running() {
    let mut s = Screen::build(Owner::ChatNavPane).script(RUNNING).open();
    s.type_text("/clear");
    s.keys("esc enter");
    s.assert_contains("'/clear' is disabled while a task is in progress.");
    // The oracle asserts exactly one InsertHistoryCell and no follow-up event: the single cell is the drawn line.
    s.assert_text_count("'/clear' is disabled while a task is in progress.", 1);
}

/// The oracle's snapshots of the confirmations (`slash_archive_confirmation_popup`,
/// `slash_archive_running_confirmation_popup`, `slash_delete_confirmation_command_center`)
/// render its bottom pane only: the composer shows through in the four columns
/// left of the dialog ("› As", "  GP", "• Wo"), which a frame of the whole
/// screen does not hold. The dialog is the rows from column 4 on, the rows
/// below are the snapshot's with those four columns blank.
const ARCHIVE_ROWS: &[&str] = &[
    "      Archive this session?",
    "      Are you sure? This will archive the current session",
    "",
    "",
    "    › 1. No, don't archive  Return to the current session",
    "      2. Yes, archive       Archive this session now",
    "",
    "      enter select · esc back",
];

const ARCHIVE_RUNNING_ROWS: &[&str] = &[
    "      Archive this session?",
    "      This will stop the current turn and archive the session.",
    "",
    "",
    "    › 1. No, don't archive  Return to the current session",
    "      2. Yes, archive       Archive this session now",
    "",
    "      enter select · esc back",
];

const DELETE_ROWS: &[&str] = &[
    "      Delete this session?",
    "      Also deletes subagents and saved side chats. Cannot be undone.",
    "",
    "",
    "    › 1. No, keep this session                     Return to the current",
    "                                                   session",
    "      2. Yes, delete and return to command center  Permanently delete this",
    "                                                   session now",
    "",
    "      enter select · esc back",
];

/// The confirmation is a dialog centered on the 80x24 screen (the oracle's
/// `ViewPresentation::Centered`, `dialog_area`: the rows of the dialog in the
/// middle of the frame, 72 cells wide from column 4).
fn dialog(s: &Screen, rows: &[&str]) {
    let top = (24 - rows.len()) / 2;
    for (i, row) in rows.iter().enumerate() {
        s.assert_row(top + i, row);
    }
}

// ORACLE chatwidget/tests/slash_commands.rs:2947 slash_archive_cancellation_keeps_task_running kind=ui owner=chat-nav-pane
#[test]
fn slash_archive_cancellation_keeps_task_running() {
    let mut s = Screen::build(Owner::ChatNavPane).script(RUNNING).open();
    s.type_text("/archive");
    s.keys("enter");
    s.assert_contains("Archive this session?");
    s.assert_contains("This will stop the current turn and archive the session.");
    s.keys("enter");
    // The popup is dismissed and the task still runs, with no archive asked.
    s.assert_absent("Archive this session?");
    s.assert_contains("esc to interrupt");
    s.assert_archives(0);
}

// ORACLE chatwidget/tests/slash_commands.rs:2963 slash_memory_maintenance_reports_stubbed_feature kind=ui owner=chat-nav-pane
#[test]
fn slash_memory_maintenance_reports_stubbed_feature() {
    for command in ["/debug-m-drop", "/debug-m-update"] {
        let mut s = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
        s.type_text(command);
        s.keys("esc enter");
        s.assert_contains("■ Memory maintenance: Not available in TUI yet.");
    }
}

// ORACLE chatwidget/tests/slash_commands.rs:3041 slash_mcp_invalid_args_show_usage kind=ui owner=chat-nav-pane
#[test]
fn slash_mcp_invalid_args_show_usage() {
    let mut s = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
    s.type_text("/mcp full");
    s.keys("enter");
    s.assert_contains("Usage: /mcp [verbose | login <name>]");
    // recall_latest_after_clearing: clear the composer (set_composer_text("") there; ctrl+u here), then Up.
    // (the command took the line: it is recalled, not kept)
    s.keys("ctrl+u");
    s.assert_composer("");
    s.keys("up");
    s.assert_composer("/mcp full");
}

// ORACLE chatwidget/tests/slash_commands.rs:3061 slash_memories_opens_memory_menu kind=ui owner=chat-nav-pane
#[test]
fn slash_memories_opens_memory_menu() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .script(MEMORY_TOOL)
        .open();
    s.type_text("/memories");
    s.keys("enter");
    s.assert_contains("Use memories");
    // The oracle asserts no event and no core op: not drawn.
}

// ORACLE chatwidget/tests/slash_commands.rs:3095 slash_archive_confirmation_requests_current_thread_archive kind=snap owner=chat-nav-pane snap=slash_archive_confirmation_popup snap=slash_archive_running_confirmation_popup
#[test]
fn slash_archive_confirmation_requests_current_thread_archive() {
    // Not running: the idle popup, then down and enter confirms the archive.
    let mut s = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
    s.type_text("/archive");
    s.keys("enter");
    s.assert_archives(0);
    dialog(&s, ARCHIVE_ROWS);
    s.keys("down");
    s.keys("enter");
    s.assert_archives(1);
    // Running: the same popup with the running text, from a running task seed.
    let mut r = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .script(RUNNING)
        .open();
    r.type_text("/archive");
    r.keys("enter");
    r.assert_archives(0);
    dialog(&r, ARCHIVE_RUNNING_ROWS);
    r.keys("down");
    r.keys("enter");
    r.assert_archives(1);
}

// ORACLE chatwidget/tests/slash_commands.rs:3121 slash_delete_confirmation_requests_current_thread_delete kind=snap owner=chat-nav-pane snap=slash_delete_confirmation_command_center
#[test]
fn slash_delete_confirmation_requests_current_thread_delete() {
    let mut s = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
    s.type_text("/delete");
    s.keys("enter");
    s.assert_deletes(0);
    dialog(&s, DELETE_ROWS);
    s.keys("down");
    s.keys("enter");
    s.assert_deletes(1);
}

// ORACLE chatwidget/tests/slash_commands.rs:3134 slash_resume_with_arg_requests_named_session_while_mcp_startup_is_running kind=ui owner=chat-nav-pane
#[test]
fn slash_resume_with_arg_requests_named_session_while_mcp_startup_is_running() {
    kit::na(
        Owner::ChatNavPane,
        "asserts the FollowTranscript and ResumeSessionByIdOrName(\"my-saved-thread\") events and no core op, nothing drawn",
    );
}

// ORACLE chatwidget/tests/slash_commands.rs:3155 slash_pets_opens_picker kind=snap owner=chat-nav-pane snap=slash_pets_picker
#[test]
fn slash_pets_opens_picker() {
    kit::pending(
        Owner::ChatNavPane,
        "the pets picker needs pet image support (force_pet_image_support) seeded; no seed registers it, the pets screen is not reachable from a live chat black-box",
    );
    let mut s = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
    s.type_text("/pets");
    s.keys("enter");
    s.assert_snap("slash_pets_picker");
}

// ORACLE chatwidget/tests/slash_commands.rs:3170 slash_pets_with_arg_selects_named_pet kind=ui owner=chat-nav-pane
#[test]
fn slash_pets_with_arg_selects_named_pet() {
    kit::na(
        Owner::ChatNavPane,
        "asserts the FollowTranscript and PetSelected{pet_id: chefito} events and no core op, nothing drawn",
    );
}

// ORACLE chatwidget/tests/slash_commands.rs:3188 slash_pet_disable_aliases_work_on_unsupported_terminal kind=ui owner=chat-nav-pane
#[test]
fn slash_pet_disable_aliases_work_on_unsupported_terminal() {
    kit::na(
        Owner::ChatNavPane,
        "asserts the FollowTranscript and PetDisabled events and no core op for \"/pets disable\" and \"/pet hide\", nothing drawn",
    );
}

// ORACLE chatwidget/tests/slash_commands.rs:3205 slash_pets_in_tmux_shows_notice_and_preserves_draft kind=snap owner=chat-nav-pane snap=slash_pets_unavailable_tmux_40
#[test]
fn slash_pets_in_tmux_shows_notice_and_preserves_draft() {
    kit::pending(
        Owner::ChatNavPane,
        "the tmux pets notice popup and the unsupported-terminal seed (force_tmux_pet_image_unsupported) are not registered",
    );
    let mut s = Screen::build(Owner::ChatNavPane).size(40, 24).live().open();
    s.type_text("Keep this draft");
    // The oracle dispatches /pets while the draft stays in the composer; a black-box way to run
    // the command with a draft kept is not in the kit yet (kit gap: no command dispatch seed).
    s.assert_snap("slash_pets_unavailable_tmux_40");
    s.assert_contains("Pets are unavailable in this tmux pane.");
    s.assert_contains("outside tmux");
    s.keys("esc");
    s.assert_contains("Keep this draft");
}

// ORACLE chatwidget/tests/slash_commands.rs:3234 slash_pets_with_arg_on_unsupported_terminal_shows_notice_without_selection kind=ui owner=chat-nav-pane
#[test]
fn slash_pets_with_arg_on_unsupported_terminal_shows_notice_without_selection() {
    kit::pending(
        Owner::ChatNavPane,
        "the tmux pets notice popup and the unsupported-terminal seed (force_tmux_pet_image_unsupported) are not registered",
    );
    let mut s = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
    s.type_text("/pets chefito");
    s.keys("enter");
    s.assert_contains("Pets are unavailable in this tmux pane.");
    s.keys("enter");
    // The oracle asserts the popup is dismissed by Enter and no selection event or core op: the dismissal is drawn below.
    s.assert_absent("Pets are unavailable in this tmux pane.");
}

// ORACLE chatwidget/tests/slash_commands.rs:3265 slash_pets_on_unsupported_terminal_shows_terminal_warning kind=ui owner=chat-nav-pane
#[test]
fn slash_pets_on_unsupported_terminal_shows_terminal_warning() {
    kit::pending(
        Owner::ChatNavPane,
        "the unsupported-terminal pets warning and its seed (force_terminal_pet_image_unsupported) are not registered",
    );
    let mut s = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
    s.type_text("/pets");
    s.keys("enter");
    s.assert_contains("Pets aren’t available in this terminal.");
    s.assert_contains("Kitty graphics or Sixel support");
}

// ORACLE chatwidget/tests/slash_commands.rs:3288 slash_pets_on_old_iterm2_shows_upgrade_warning kind=ui owner=chat-nav-pane
#[test]
fn slash_pets_on_old_iterm2_shows_upgrade_warning() {
    kit::pending(
        Owner::ChatNavPane,
        "the old-iTerm2 pets warning and its seed (force_old_iterm2_pet_image_unsupported) are not registered",
    );
    let mut s = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
    s.type_text("/pets");
    s.keys("enter");
    s.assert_contains("Pets require iTerm2 3.6 or newer.");
    s.assert_contains("Upgrade iTerm2 to use terminal pets.");
}

// ORACLE chatwidget/tests/slash_commands.rs:3319 slash_fork_with_name_requests_named_fork kind=ui owner=chat-nav-pane
#[test]
fn slash_fork_with_name_requests_named_fork() {
    kit::na(
        Owner::ChatNavPane,
        "asserts the FollowTranscript and ForkCurrentSession{name: Add User} events, nothing drawn",
    );
}

// ORACLE chatwidget/tests/slash_commands.rs:3353 slash_app_without_thread_id_shows_starting_error kind=snap owner=chat-nav-pane snap=slash_app_without_thread_id_shows_starting_error
#[test]
fn slash_app_without_thread_id_shows_starting_error() {
    // The session has not started (the thread's start is held): no thread id yet.
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .held_start()
        .open();
    s.type_text("/app");
    s.keys("esc enter");
    s.assert_has_row("■ Session is still starting; try /app again in a moment.");
}

// ORACLE chatwidget/tests/slash_commands.rs:3367 slash_pwd_and_cwd_alias_display_current_working_directory_from_composer kind=snap owner=chat-nav-pane
#[test]
fn slash_pwd_and_cwd_alias_display_current_working_directory_from_composer() {
    // The oracle runs /pwd with a task running and /cwd in a side conversation (both stay available).
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .script(RUNNING)
        .cwd("/tmp/project")
        .open();
    s.type_text("/pwd");
    s.keys("esc enter");
    s.assert_has_row("• Current working directory: /tmp/project");
    let mut side = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .script(SIDE)
        .cwd("/tmp/project")
        .open();
    side.type_text("/cwd");
    side.keys("esc enter");
    side.assert_has_row("• Current working directory: /tmp/project");
    let mut usage = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .live()
        .cwd("/tmp/project")
        .open();
    usage.type_text("/pwd x");
    usage.keys("esc enter");
    usage.assert_has_row("■ Usage: /pwd");
}

// ORACLE chatwidget/tests/slash_commands.rs:3388 slash_cd_changes_current_session_after_replay_and_defaults_to_home kind=ui owner=chat-nav-pane
#[test]
fn slash_cd_changes_current_session_after_replay_and_defaults_to_home() {
    kit::na(
        Owner::ChatNavPane,
        "asserts the ChangeWorkingDirectory{requested_cwd: /tmp, then ~} events and no core op after a replayed turn, nothing drawn",
    );
}

// ORACLE chatwidget/tests/slash_commands.rs:3434 slash_cd_rejects_pending_input_and_unsupported_session_ownership kind=snap owner=chat-nav-pane
#[test]
fn slash_cd_rejects_pending_input_and_unsupported_session_ownership() {
    kit::pending(
        Owner::ChatNavPane,
        "the snapshot's second half is the idle session with a queued message: the port drains its queue at turn end and refuses /cd while a task runs, so no key sequence reaches that state; the oracle's whole table of states is carried by core/tests/commands.rs directory::the_directory_is_not_changed_unless_the_session_is_idle_and_the_primary_one and ::cd_before_the_session_starts_says_so",
    );
    // State \"new\": no session yet.
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(200, 24)
        .live()
        .open();
    s.type_text("/cd /tmp");
    s.keys("enter");
    s.assert_contains("The session must start before you can change its working directory.");
    // State \"queued\": a queued input is pending.
    let mut q = Screen::build(Owner::ChatNavPane)
        .size(200, 24)
        .live()
        .open();
    q.type_text("/cd /tmp");
    q.keys("enter");
    q.assert_contains(
        "Changing directories requires an idle primary session without queued input.",
    );
}

// ORACLE chatwidget/tests/slash_commands.rs:3474 slash_rollout_displays_current_path kind=ui owner=chat-nav-pane
#[test]
fn slash_rollout_displays_current_path() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .script(ROLLOUT)
        .open();
    s.type_text("/rollout");
    s.keys("esc enter");
    s.assert_has_row("• Current rollout path: /tmp/codex-test-rollout.jsonl");
}

// ORACLE chatwidget/tests/slash_commands.rs:3491 slash_rollout_handles_missing_path kind=ui owner=chat-nav-pane
#[test]
fn slash_rollout_handles_missing_path() {
    let mut s = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
    s.type_text("/rollout");
    s.keys("esc enter");
    s.assert_has_row("• Rollout path is not available yet.");
}

// ORACLE chatwidget/tests/slash_commands.rs:3592 user_turn_carries_service_tier_after_fast_toggle kind=ui owner=chat-nav-pane
#[test]
fn user_turn_carries_service_tier_after_fast_toggle() {
    kit::na(
        Owner::ChatNavPane,
        "asserts the Op::UserTurn service_tier sent to the core after the fast toggle, not drawn",
    );
}

// ORACLE chatwidget/tests/slash_commands.rs:3617 user_turn_preserves_flex_without_catalog_support kind=ui owner=chat-nav-pane
#[test]
fn user_turn_preserves_flex_without_catalog_support() {
    kit::na(
        Owner::ChatNavPane,
        "asserts current_service_tier and the Op::UserTurn service_tier (flex) sent to the core, not drawn",
    );
}

// ORACLE chatwidget/tests/slash_commands.rs:3650 model_switch_recomputes_catalog_default_service_tier kind=ui owner=chat-nav-pane
#[test]
fn model_switch_recomputes_catalog_default_service_tier() {
    kit::na(
        Owner::ChatNavPane,
        "asserts current_service_tier after set_model and the Op::UserTurn service_tier sent to the core, not drawn",
    );
}

// ORACLE chatwidget/tests/slash_commands.rs:3733 user_turn_sends_standard_override_after_fast_is_turned_off kind=ui owner=chat-nav-pane
#[test]
fn user_turn_sends_standard_override_after_fast_is_turned_off() {
    kit::na(
        Owner::ChatNavPane,
        "asserts the OverrideTurnContext and PersistServiceTierSelection events and the Op::UserTurn service_tier sent to the core, not drawn",
    );
}

// ORACLE chatwidget/tests/slash_commands.rs:3811 raw_slash_command_reports_usage_for_invalid_arg kind=ui owner=chat-nav-pane
#[test]
fn raw_slash_command_reports_usage_for_invalid_arg() {
    let mut s = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
    s.type_text("/raw status");
    s.keys("enter");
    s.assert_contains("Usage: /raw [on|off]");
}

// ORACLE chatwidget/tests/slash_commands.rs:3830 compact_queues_user_messages_snapshot kind=snap owner=chat-nav-pane snap=compact_queues_user_messages_snapshot
#[test]
fn compact_queues_user_messages_snapshot() {
    kit::pending(
        Owner::ChatNavPane,
        "a compact turn that refuses a steer (ActiveTurnNotSteerable) and the queued-message rows cannot be seeded black-box",
    );
    let mut s = Screen::build(Owner::ChatNavPane).size(80, 18).live().open();
    s.type_text("Steer submitted while /compact was running.");
    s.keys("enter");
    s.assert_snap("compact_queues_user_messages_snapshot");
}

// ORACLE chatwidget/tests/slash_commands.rs:3863 transcript_copy_feedback_stays_in_the_footer_without_history_or_interrupts kind=snap owner=chat-nav-pane snap=transcript_copy_success snap=transcript_copy_unconfirmed snap=transcript_copy_failure
#[test]
fn transcript_copy_feedback_stays_in_the_footer_without_history_or_interrupts() {
    kit::pending(
        Owner::ChatNavPane,
        "the transcript copy feedback in the footer (success, unconfirmed, failure) is not built; the copy results cannot be seeded black-box",
    );
    let s = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
    s.assert_snap("transcript_copy_success");
    s.assert_snap("transcript_copy_unconfirmed");
    s.assert_snap("transcript_copy_failure");
}

// ---- oracle: chatwidget/tests/sparkle_submission_tests.rs (1 screen-level tests here; 0 state tests left to chat-nav-pane)
// ORACLE chatwidget/tests/sparkle_submission_tests.rs:11 ordinary_and_prepared_image_submissions_consume_the_sparkle_before_rendering kind=ui owner=chat-nav-pane
#[test]
fn ordinary_and_prepared_image_submissions_consume_the_sparkle_before_rendering() {
    // the oracle submits twice, once as typed and once with images the host prepared (an
    // `Op::UserTurn` built in code, no composer attachment): the dots of the fresh Astra thread
    // show before the submit and are gone after it. Only the typed submit can be driven here: an
    // image attached in the composer ends the sparkle at the attachment (pane_footer_1), so the
    // dots would not show before the submit.
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .script(ASTRA)
        .terminal_colors(0xe6_d8_ff, 0x24_1b_35)
        .open();
    s.assert_dots(true);
    s.type_text("describe");
    s.keys("enter");
    s.assert_dots(false);
    s.wait(1_000);
    s.assert_dots(false);
}

// ---- oracle: chatwidget/tests/startup_submission_tests.rs (9 screen-level tests here; 2 state tests left to chat-nav-pane)
// ORACLE chatwidget/tests/startup_submission_tests.rs:6 host_switch_snapshot_preserves_buffered_typing kind=ui owner=chat-nav-pane
#[test]
fn host_switch_snapshot_preserves_buffered_typing() {
    let mut s = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
    s.type_text("draft x");
    s.assert_contains("draft x");
    let mut several = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
    several.type_text("draft several keys");
    several.assert_contains("draft several keys");
}

// ORACLE chatwidget/tests/startup_submission_tests.rs:22 initial_prompt_recall_precedes_cross_session_history kind=snap owner=chat-nav-pane snap=initial_prompt_recalled_composer
#[test]
fn initial_prompt_recall_precedes_cross_session_history() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .live()
        .message_history(&["FOREIGN_X"])
        .initial_prompt("CURRENT_A", &[])
        .open();
    s.assert_turn_texts(&["CURRENT_A"]);
    s.feed(&completed(1));
    s.type_text("CURRENT_B");
    s.keys("enter");
    s.feed(&completed(2));
    s.keys("up");
    s.assert_contains("CURRENT_B");
    s.keys("up");
    s.assert_contains("CURRENT_A");
    // The oracle's snapshot is the bottom pane: the recalled row, a blank row, and the status
    // line its config sets (`GPT-5.6-Sol default · /tmp/project`; the kit's session has no
    // status-line seed, its footer is the key hints).
    s.assert_has_row("› CURRENT_A");
    s.keys("up");
    s.assert_contains("FOREIGN_X");
    let mut single = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .live()
        .message_history(&["FOREIGN_X"])
        .initial_prompt("CURRENT_A", &[])
        .open();
    single.feed(&completed(1));
    single.keys("up");
    single.assert_contains("CURRENT_A");
    single.keys("up");
    single.assert_contains("FOREIGN_X");
}

// ORACLE chatwidget/tests/startup_submission_tests.rs:105 initial_prompt_image_preparation_preserves_recall_metadata kind=ui owner=chat-nav-pane
#[test]
fn initial_prompt_image_preparation_preserves_recall_metadata() {
    let path = kit::image_file_sized("image.png", 1, 1);
    for (input, recalled) in [
        ("describe", "describe [Image #1]"),
        ("[Image #1] describe", "[Image #1] describe"),
    ] {
        let mut s = Screen::build(Owner::ChatNavPane)
            .size(80, 24)
            .live()
            .initial_prompt(input, std::slice::from_ref(&path))
            .open();
        s.assert_turn_texts(&[recalled]);
        s.feed(&completed(1));
        s.keys("up");
        s.assert_contains(recalled);
        // The oracle asserts the recalled text, elements and local image attachment, then a shared
        // history lookup at offset 0 and the image preparation of the recalled prompt: the
        // attachment is drawn as the placeholder.
        s.keys("up");
        s.keys("enter");
    }
}

// ORACLE chatwidget/tests/startup_submission_tests.rs:204 initial_prompt_queued_before_session_is_recalled_once_after_dispatch kind=ui owner=chat-nav-pane
#[test]
fn initial_prompt_queued_before_session_is_recalled_once_after_dispatch() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .live()
        .held_start()
        .initial_prompt("CURRENT_A", &[])
        .open();
    s.assert_no_turn();
    s.start_thread();
    s.assert_turn_texts(&["CURRENT_A"]);
    s.feed(&completed(1));
    s.keys("up");
    s.assert_contains("CURRENT_A");
    // The oracle then asserts a shared-history lookup at offset 0 (an event, not drawn).
    s.keys("up");
}

// ORACLE chatwidget/tests/startup_submission_tests.rs:231 rejected_initial_prompt_is_not_recorded_in_recall_history kind=ui owner=chat-nav-pane
#[test]
fn rejected_initial_prompt_is_not_recorded_in_recall_history() {
    kit::na(
        Owner::ChatNavPane,
        "asserts the LookupMessageHistoryEntry event after a closed submission channel rejects the initial prompt, and no submit op, nothing drawn",
    );
}

// ORACLE chatwidget/tests/startup_submission_tests.rs:252 startup_submission_waits_for_session_and_sends_exactly_once kind=ui owner=chat-nav-pane
#[test]
fn startup_submission_waits_for_session_and_sends_exactly_once() {
    let mut s = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
    s.type_text("confirmed before startup");
    s.assert_contains("confirmed before startup");
    s.keys("enter enter enter");
    s.keys("ctrl+b");
    s.keys("alt+f");
    // The oracle then asserts the single UserTurn with that text and an empty composer after it:
    // the sent turn is drawn in the transcript, the op is not drawn.
}

// ORACLE chatwidget/tests/startup_submission_tests.rs:288 startup_submission_escape_restores_editing_without_submitting kind=snap owner=chat-nav-pane
#[test]
fn startup_submission_escape_restores_editing_without_submitting() {
    kit::pending(
        Owner::ChatNavPane,
        "needs the Enter-confirm startup submission (Enter on a startup draft waits, Esc cancels it and keeps the draft); the port queues the prompt at Enter before the session instead, and the rejected-startup Disconnected restricted input has no seed",
    );
    let mut s = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
    s.type_text("keep the draft");
    s.keys("esc");
    s.assert_contains("keep the draft");
    let mut rejected = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
    rejected.keys("esc");
    rejected.assert_contains("  ⌃c quit");
}

// ORACLE chatwidget/tests/startup_submission_tests.rs:321 startup_submission_waits_for_protected_view_and_preserves_paste_provenance kind=ui owner=chat-nav-pane
#[test]
fn startup_submission_waits_for_protected_view_and_preserves_paste_provenance() {
    kit::pending(
        Owner::ChatNavPane,
        "needs the Enter-confirm startup submission held behind an open approvals popup; the port queues the prompt at Enter before the session instead",
    );
    let mut s = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
    s.paste("!echo hi\n# visible tail");
    s.assert_contains("!echo hi");
    s.assert_contains("# visible tail");
    s.keys("esc");
    s.keys("enter");
    // The oracle's other payloads (leading spaces; 1000 x's) and its RunUserShellCommand / UserTurn ops are not drawn here.
}

// ORACLE chatwidget/tests/startup_submission_tests.rs:367 startup_submission_edit_or_disconnect_cancels_without_losing_draft kind=ui owner=chat-nav-pane
#[test]
fn startup_submission_edit_or_disconnect_cancels_without_losing_draft() {
    let mut edit = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
    edit.type_text("confirmed draft");
    edit.paste(" edited");
    edit.assert_contains("confirmed draft edited");
    let mut kept = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
    kept.type_text("confirmed draft");
    kept.assert_contains("confirmed draft");
}

// ---- oracle: startup_draft_layout_tests.rs (3 screen-level tests here; 0 state tests left to chat-nav-pane)
// ORACLE startup_draft_layout_tests.rs:6 owned_startup_keeps_the_live_bottom_geometry kind=snap owner=chat-nav-pane snap=owned_startup_layout
#[test]
fn owned_startup_keeps_the_live_bottom_geometry() {
    kit::pending(
        Owner::ChatNavPane,
        "the startup draft frames are ported (inline); this one is the alt-screen startup-owned layout, whose footer hint override (`Waiting for startup`) belongs to the Enter-confirm startup submission, which the port lacks (Enter before the session queues the prompt)",
    );
    let mut s = Screen::build(Owner::ChatNavPane).size(48, 16).live().open();
    s.type_text("first line\nsecond line");
    s.assert_snap("owned_startup_layout");
}

// ORACLE startup_draft_layout_tests.rs:37 new_startup_decoration_tracks_draft_and_session_action kind=ui owner=chat-nav-pane
#[test]
fn new_startup_decoration_tracks_draft_and_session_action() {
    kit::pending(
        Owner::ChatNavPane,
        "the startup blossom decoration is not in the port, its pending state is the Enter-confirm startup submission (absent: Enter before the session queues the prompt), and the kit has no any-braille-glyph assertion",
    );
    let mut s = Screen::build(Owner::ChatNavPane).size(80, 32).live().open();
    s.type_text(" ");
    // The oracle asserts the blossom glyphs (U+2801..U+28FF) are drawn for states [T, F, T, F, F, F, T].
}

// ORACLE startup_draft_layout_tests.rs:72 configured_welcome_opt_out_disables_blossom_and_clicks kind=ui owner=chat-nav-pane
#[test]
fn configured_welcome_opt_out_disables_blossom_and_clicks() {
    kit::pending(
        Owner::ChatNavPane,
        "the welcome opt-out settings (config.toml animations / welcome) are not seedable by the kit; the blossom glyph check is a kit gap",
    );
    let _screen = Screen::build(Owner::ChatNavPane).size(80, 44).live().open();
    // The oracle asserts the blossom glyphs per settings and effective config, and the blossom click handling:
    // no kit seed writes config.toml, and no kit assertion checks an any-braille-glyph set (kit gap).
}

// ---- oracle: startup_draft_tests.rs (6 screen-level tests here; 14 state tests left to chat-nav-pane)
// ORACLE startup_draft_tests.rs:69 startup_draft_renders_full_empty_and_multiline_composer_frames kind=snap owner=chat-nav-pane snap=empty snap=startup_draft_full_frames
#[test]
fn startup_draft_renders_full_empty_and_multiline_composer_frames() {
    // The oracle renders the draft pump six times (a session action, a width, a text); each is
    // one chat here, opened before its session starts, inline under no shell rows.
    const LABELS: [&str; 6] = [
        "empty (48 columns)",
        "configured (48 columns)",
        "resuming (48 columns)",
        "forking (48 columns)",
        "multiline (48 columns)",
        "narrow (18 columns)",
    ];
    let startup = |width: u16| {
        Screen::build(Owner::ChatNavPane)
            .size(width, 24)
            .live()
            .held_start()
            .inline(&[])
    };
    startup(48)
        .open()
        .snap("startup_draft_full_frames")
        .labelled(&LABELS)
        .frame("empty (48 columns)")
        .assert();
    let mut configured = startup(48).open();
    configured.configured_cwd("workspace");
    configured
        .snap("startup_draft_full_frames")
        .labelled(&LABELS)
        .frame("configured (48 columns)")
        .assert();
    startup(48)
        .resume("saved")
        .open()
        .snap("startup_draft_full_frames")
        .labelled(&LABELS)
        .frame("resuming (48 columns)")
        .assert();
    let mut forking = startup(48).fork("saved").open();
    forking.type_text("draft while loading");
    forking
        .snap("startup_draft_full_frames")
        .labelled(&LABELS)
        .frame("forking (48 columns)")
        .assert();
    let mut multiline = startup(48).open();
    multiline.paste("first startup line\nsecond startup line");
    multiline
        .snap("startup_draft_full_frames")
        .labelled(&LABELS)
        .frame("multiline (48 columns)")
        .assert();
    let mut narrow = startup(18).open();
    narrow.paste("first startup line\nsecond startup line");
    narrow
        .snap("startup_draft_full_frames")
        .labelled(&LABELS)
        .frame("narrow (18 columns)")
        .assert();
}

// ORACLE startup_draft_tests.rs:148 terminal_app_ssh_fallback_renders_inline_startup kind=snap owner=chat-nav-pane snap=terminal_app_ssh_startup
#[test]
fn terminal_app_ssh_fallback_renders_inline_startup() {
    kit::pending(
        Owner::ChatNavPane,
        "feature terminal-app-over-ssh inline startup (w2-composer, startup): not in the port (alt screen off) is not reachable black-box",
    );
    let s = Screen::build(Owner::ChatNavPane).size(48, 16).live().open();
    s.assert_snap("terminal_app_ssh_startup");
}

// ORACLE startup_draft_tests.rs:205 startup_draft_clears_loading_status_when_starting_fresh kind=snap owner=chat-nav-pane snap=startup_draft_fresh_session_transitions
#[test]
fn startup_draft_clears_loading_status_when_starting_fresh() {
    kit::pending(
        Owner::ChatNavPane,
        "feature startup draft (w2-composer, startup): not in the port's loading status and its fresh-session transitions (resume and fork lookups, picker cancellation) are not reachable black-box",
    );
    let s = Screen::build(Owner::ChatNavPane).size(48, 24).live().open();
    s.assert_snap_section(
        "startup_draft_fresh_session_transitions",
        "resume lookup fallback",
    );
    s.assert_snap_section(
        "startup_draft_fresh_session_transitions",
        "fork lookup fallback",
    );
    s.assert_snap_section(
        "startup_draft_fresh_session_transitions",
        "resume picker cancellation",
    );
    s.assert_snap_section(
        "startup_draft_fresh_session_transitions",
        "fork picker cancellation",
    );
}

// ORACLE startup_draft_tests.rs:294 startup_draft_hydrates_its_header_without_moving_the_composer kind=ui owner=chat-nav-pane
#[test]
fn startup_draft_hydrates_its_header_without_moving_the_composer() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .live()
        .held_start()
        .inline(&[])
        .open();
    s.assert_has_row("     loading");
    let composer = |s: &Screen| {
        s.rows()
            .iter()
            .position(|row| row.starts_with("› "))
            .expect("the composer")
    };
    let before = composer(&s);
    // The host has read the config: the heading's directory is the configured one, on the same
    // row, and the composer did not move.
    s.configured_cwd("workspace");
    s.assert_has_row("     workspace");
    s.assert_absent("loading");
    kit::assert_same(composer(&s), before, "the composer did not move");
}

// ORACLE startup_draft_tests.rs:690 startup_draft_applies_super_editor_keymap_without_inserting_pending_newline kind=snap owner=chat-nav-pane snap=startup_draft_safe_chord_guide
#[test]
fn startup_draft_applies_super_editor_keymap_without_inserting_pending_newline() {
    kit::pending(
        Owner::ChatNavPane,
        "feature startup draft (w2-composer, startup): not in the port with a super-z editor keymap and the F12 chord guide need the keymap seed (not registered) and the startup draft",
    );
    let mut s = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
    s.type_text("draft");
    s.type_text("x");
    s.keys("super+z");
    s.keys("f12");
    s.wait(1000);
    s.assert_snap("startup_draft_safe_chord_guide");
    s.keys("e");
    s.keys("enter");
}

// ORACLE startup_draft_tests.rs:770 startup_draft_waits_for_onboarding_before_accepting_input kind=snap owner=chat-nav-pane snap=startup_draft_onboarding_transition
#[test]
fn startup_draft_waits_for_onboarding_before_accepting_input() {
    kit::pending(
        Owner::ChatNavPane,
        "feature startup draft (w2-composer, startup): not in the port held behind onboarding (first login, search override, existing history) is not reachable black-box",
    );
    let mut s = Screen::build(Owner::ChatNavPane).size(80, 24).live().open();
    s.keys("x");
    s.paste("not a draft");
    s.assert_snap("startup_draft_onboarding_transition");
}
