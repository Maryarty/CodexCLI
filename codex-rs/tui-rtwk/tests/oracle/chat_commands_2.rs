//! Shard `chat_commands_2` (screens S10, owner chat-nav-pane): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen};

const ACTIVE: &str = include_str!("fixtures/chat_commands_2/active.jsonl");
const ACTIVE_DONE: &str = include_str!("fixtures/chat_commands_2/active_done.jsonl");
const COMMENTARY: &str = include_str!("fixtures/chat_commands_2/commentary.jsonl");
const DIRECTIVES: &str = include_str!("fixtures/chat_commands_2/directives.jsonl");
const DONE: &str = include_str!("fixtures/chat_commands_2/done.jsonl");
const INTERRUPTED: &str = include_str!("fixtures/chat_commands_2/interrupted.jsonl");
const NAMED: &str = include_str!("fixtures/chat_commands_2/named.jsonl");
const ORIGINAL: &str = include_str!("fixtures/chat_commands_2/original.jsonl");
const PLAN: &str = include_str!("fixtures/chat_commands_2/plan.jsonl");
const PREVIEW: &str = include_str!("fixtures/chat_commands_2/preview.jsonl");
const PREVIOUS: &str = include_str!("fixtures/chat_commands_2/previous.jsonl");
const READY: &str = include_str!("fixtures/chat_commands_2/ready.jsonl");
const RENAMED: &str = include_str!("fixtures/chat_commands_2/queued_rename.jsonl");
const RUNNING: &str = include_str!("fixtures/chat_commands_2/running.jsonl");
const SESSION: &str = include_str!("fixtures/chat_commands_2/session.jsonl");
const SIGNED_OUT: &str = include_str!("fixtures/chat_commands_2/signed_out.jsonl");
const SH_BLOCKS: &str = include_str!("fixtures/chat_commands_2/sh_blocks.jsonl");
const TURN2_REPLY: &str = include_str!("fixtures/chat_commands_2/turn2_reply.jsonl");
const TURN2_STARTED: &str = include_str!("fixtures/chat_commands_2/turn2_started.jsonl");
/// An exchange of thread-1 to name the thread by (`suggest_thread_name` asks the model only when
/// the conversation has a message): the oracle's tests of the suggestion send the event itself.
const TALKED: &str = r#"{"step":"notify","message":{"method":"item/completed","params":{"threadId":"thread-1","turnId":"turn-1","completedAtMs":0,"startedAtMs":0,"item":{"type":"userMessage","id":"u1","clientId":null,"content":[{"type":"text","text":"fix the login timeout","text_elements":[]}]}}}}
{"step":"notify","message":{"method":"item/completed","params":{"threadId":"thread-1","turnId":"turn-1","completedAtMs":0,"startedAtMs":0,"item":{"type":"agentMessage","id":"a1","text":"I raised the timeout.","phase":null}}}}"#;

// ---- oracle: chatwidget/tests/slash_commands.rs (46 screen-level tests here; 22 state tests left to chat-nav-pane)
// ORACLE chatwidget/tests/slash_commands.rs:110 service_tier_commands_and_saved_selection_respect_independent_speed_policy kind=snap owner=chat-nav-pane
#[test]
fn service_tier_commands_and_saved_selection_respect_independent_speed_policy() {
    let catalog = include_str!("fixtures/chat_commands_2/speed_catalog.json").trim();
    // (fast, ultrafast) = (false, true): the ultrafast command is offered and the tier is on;
    // (true, false): the ultrafast command is denied and the tier is off.
    for (fast, ultrafast, snap) in [
        (false, true, "ultrafast_command_allowed"),
        (true, false, "ultrafast_command_denied"),
    ] {
        let mut s = Screen::build(Owner::ChatNavPane).size(80, 8).live().open();
        s.feed(&format!(
            r#"{{"step":"respond","method":"model/list","result":{catalog}}}
{{"step":"screen","dir":"chat","ev":{{"status":{{"model":"gpt-5.5","service_tier":"ultrafast","cwd":"/tmp/project","fast_mode":{fast},"ultrafast_mode":{ultrafast}}}}}}}"#
        ));
        // the catalog is read when the picker opens (the oracle's chat holds it already)
        s.type_text("/model").keys("enter");
        s.keys("esc");
        s.type_text("/ultra");
        let row = s
            .rows()
            .iter()
            .position(|r| r.contains("› /ultra") && !r.contains("/ultrafast"))
            .expect("the composer row");
        if ultrafast {
            // the oracle draws the popup row above the composer, the port below it: the
            // popup row and the footer are compared as the snapshot has them
            s.assert_contains("/ultrafast  ultrafast processing");
            s.assert_has_row("  GPT-5.5 default ultrafast · /tmp/project");
        } else {
            s.assert_absent("/ultrafast");
            s.assert_snap_region(snap, row, 0);
        }
    }
}

// ORACLE chatwidget/tests/slash_commands.rs:189 slash_compact_eagerly_queues_follow_up_before_turn_start kind=ui owner=chat-nav-pane
#[test]
fn slash_compact_eagerly_queues_follow_up_before_turn_start() {
    let mut s = Screen::build(Owner::ChatNavPane).live().open();
    s.type_text("/compact").keys("enter");
    // The compact request is out and the task is running before its turn starts: Enter queues
    // the follow-up (no steer, no turn: the request is `thread/compact/start`, not a turn).
    s.type_text("queued before compact turn start")
        .keys("enter");
    s.assert_has_row("• Queued follow-up inputs");
    s.assert_has_row("  ↳ queued before compact turn start");
    s.assert_absent("Messages to be submitted after next tool call");
    s.assert_no_turn();
}

// ORACLE chatwidget/tests/slash_commands.rs:285 queued_slash_review_with_args_restores_for_edit kind=ui owner=chat-nav-pane
#[test]
fn queued_slash_review_with_args_restores_for_edit() {
    let mut s = Screen::build(Owner::ChatNavPane).script(RUNNING).open();
    s.type_text("/review check regressions").keys("tab");
    // Tab queued the command with its arguments: the composer is empty, the queue holds the line
    s.assert_composer("");
    s.assert_has_row("  ↳ /review check regressions");
    s.keys("alt+up");
    s.assert_has_row("› /review check regressions");
    s.assert_absent("↳");
}

// ORACLE chatwidget/tests/slash_commands.rs:329 queued_empty_bang_shell_reports_help_when_dequeued_and_drains_next_input kind=ui owner=chat-nav-pane
#[test]
fn queued_empty_bang_shell_reports_help_when_dequeued_and_drains_next_input() {
    let mut s = Screen::build(Owner::ChatNavPane).script(RUNNING).open();
    s.type_text("!").keys("tab");
    s.type_text("hello after help").keys("tab");
    s.assert_absent("Prefix a command with ! to run it locally");
    s.assert_no_turn();
    s.feed(DONE);
    s.assert_contains("Prefix a command with ! to run it locally");
    // the next input drains: the message is the next turn
    s.assert_turn_texts(&["hello after help"]);
}

// ORACLE chatwidget/tests/slash_commands.rs:476 queued_settings_selection_applies_before_next_input kind=ui owner=chat-nav-pane
#[test]
fn queued_settings_selection_applies_before_next_input() {
    let mut s = Screen::build(Owner::ChatNavPane).script(RUNNING).open();
    // the server lists one model (the oracle narrows its catalog to one, `gpt-5.6-terra`)
    let catalog = include_str!("fixtures/chat_commands_2/speed_catalog.json").trim();
    s.feed(&format!(
        r#"{{"step":"respond","method":"model/list","result":{catalog}}}
{{"step":"screen","dir":"chat","ev":{{"status":{{"model":"gpt-5.2"}}}}}}"#
    ));
    s.type_text("/model").keys("tab");
    s.type_text("hello after selection").keys("tab");
    s.feed(DONE);
    // the queued /model opens its popup and the message waits for the selection
    s.assert_contains("Select Model and Effort");
    s.assert_no_turn();
    // the model, then its effort
    s.keys("enter");
    s.assert_no_turn();
    s.keys("enter");
    // the turn is the queued message, sent once the selection settled (which model and effort
    // it carries is the thread settings' own, not a field of the start command)
    s.assert_turn_texts(&["hello after selection"]);
}

// ORACLE chatwidget/tests/slash_commands.rs:529 queued_bare_rename_drains_next_input_after_name_update kind=ui owner=chat-nav-pane
#[test]
fn queued_bare_rename_drains_next_input_after_name_update() {
    for settlement in ["notify", "acknowledge", "refuse", "cancel"] {
        let mut s = Screen::build(Owner::ChatNavPane).script(RUNNING).open();
        s.type_text("/rename").keys("tab");
        s.type_text("hello after rename").keys("tab");
        s.assert_no_turn();
        s.feed(DONE);
        s.assert_contains("Name thread");
        s.assert_no_turn();

        if settlement == "cancel" {
            s.keys("esc");
            s.assert_set_names(&[]);
        } else {
            s.paste("Queued rename").keys("enter");
            s.assert_set_names(&["Queued rename"]);
            s.assert_no_turn();
            s.feed(r#"{"step":"notify","message":{"method":"thread/name/updated","params":{"threadId":"thread-1","threadName":"Earlier suggestion"}}}"#);
            s.assert_no_turn();

            let request = s
                .commands()
                .into_iter()
                .find(|command| format!("{command:?}").contains("SetName(\"Queued rename\")"))
                .expect("the rename request");
            match settlement {
                "notify" => {
                    s.feed(RENAMED);
                }
                "acknowledge" => s.answer_request(&request, serde_json::json!({})),
                "refuse" => {
                    s.answer_error(&request, "rename rejected");
                }
                _ => unreachable!(),
            }
        }
        s.assert_turn_texts(&["hello after rename"]);
        s.feed(RENAMED);
        s.assert_turn_texts(&["hello after rename"]);
    }

    let mut inline = Screen::build(Owner::ChatNavPane)
        .fake_turn_ids()
        .script(RUNNING)
        .open();
    for text in ["/rename Inline name", "first follow-up", "second follow-up"] {
        inline.type_text(text).keys("tab");
    }
    inline.assert_no_turn();
    inline.feed(DONE);
    inline.assert_absent("Name thread");
    inline.assert_set_names(&["Inline name"]);
    inline.assert_turn_texts(&["first follow-up"]);
}

// ORACLE chatwidget/tests/slash_commands.rs:673 queued_unknown_slash_reports_error_when_dequeued kind=ui owner=chat-nav-pane
#[test]
fn queued_unknown_slash_reports_error_when_dequeued() {
    let mut s = Screen::build(Owner::ChatNavPane).script(RUNNING).open();
    s.type_text("/does-not-exist").keys("tab");
    s.assert_absent("Unrecognized command '/does-not-exist'");
    s.feed(DONE);
    s.assert_contains("Unrecognized command '/does-not-exist'");
    // The oracle's second half (`/worktree` queued with local worktree operations
    // off, then in a non-Git directory) needs those settings seeded: not reachable here.
}

// ORACLE chatwidget/tests/slash_commands.rs:715 ctrl_d_quits_without_prompt kind=ui owner=chat-nav-pane
#[test]
fn ctrl_d_quits_without_prompt() {
    kit::na(
        Owner::ChatNavPane,
        "asserts the Exit(ShutdownFirst) event that ctrl+d sends; nothing is drawn for an exit event",
    );
}

// ORACLE chatwidget/tests/slash_commands.rs:723 ctrl_d_with_modal_open_does_not_quit kind=ui owner=chat-nav-pane
#[test]
fn ctrl_d_with_modal_open_does_not_quit() {
    kit::na(
        Owner::ChatNavPane,
        "asserts that ctrl+d with a modal open sends no Exit event; no drawn text is asserted",
    );
}

// ORACLE chatwidget/tests/slash_commands.rs:747 bare_slash_command_is_available_from_local_recall_after_dispatch kind=ui owner=chat-nav-pane
#[test]
fn bare_slash_command_is_available_from_local_recall_after_dispatch() {
    let mut s = Screen::build(Owner::ChatNavPane).live().open();
    s.type_text("/diff").keys("esc enter enter");
    s.keys("up");
    s.assert_has_row("› /diff");
}

// ORACLE chatwidget/tests/slash_commands.rs:758 inline_slash_command_is_available_from_local_recall_after_dispatch kind=ui owner=chat-nav-pane
#[test]
fn inline_slash_command_is_available_from_local_recall_after_dispatch() {
    let mut s = Screen::build(Owner::ChatNavPane).live().open();
    s.type_text("/rename Better title").keys("esc enter enter");
    s.keys("up");
    s.assert_has_row("› /rename Better title");
}

// ORACLE chatwidget/tests/slash_commands.rs:796 goal_slash_command_uses_plain_text_for_mentions kind=ui owner=chat-nav-pane
#[test]
fn goal_slash_command_uses_plain_text_for_mentions() {
    kit::na(
        Owner::ChatNavPane,
        "asserts the SetThreadGoalDraft objective (an event with the mention as plain text); the draft is not drawn in the test",
    );
}

// ORACLE chatwidget/tests/slash_commands.rs:823 goal_slash_command_emits_attached_images kind=ui owner=chat-nav-pane
#[test]
fn goal_slash_command_emits_attached_images() {
    kit::na(
        Owner::ChatNavPane,
        "asserts the SetThreadGoalDraft remote and local images and the cleared attachments; none is drawn in the test",
    );
}

// ORACLE chatwidget/tests/slash_commands.rs:861 bare_goal_slash_command_drains_pending_submission_state kind=ui owner=chat-nav-pane
#[test]
fn bare_goal_slash_command_drains_pending_submission_state() {
    kit::na(
        Owner::ChatNavPane,
        "asserts the OpenThreadGoalMenu event and the emptied image attachments; no drawn text is asserted",
    );
}

// ORACLE chatwidget/tests/slash_commands.rs:885 goal_control_slash_commands_emit_goal_events kind=ui owner=chat-nav-pane
#[test]
fn goal_control_slash_commands_emit_goal_events() {
    // The oracle enables the Goals feature and vim mode before each case.
    for command in ["/goal clear", "/goal pause", "/goal resume"] {
        let mut s = Screen::build(Owner::ChatNavPane).live().vim_mode().open();
        s.type_text(command).keys("esc enter");
        s.keys("x right esc .");
        s.assert_has_row("› xx");
    }
}

// ORACLE chatwidget/tests/slash_commands.rs:938 goal_control_slash_command_without_thread_shows_full_usage kind=snap owner=chat-nav-pane
#[test]
fn goal_control_slash_command_without_thread_shows_full_usage() {
    // no thread yet; the oracle's cell is one line (`new_info_event`: the hint follows the
    // message), wide enough here not to wrap
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(120, 24)
        .live()
        .held_start()
        .open();
    s.type_text("/goal pause").keys("esc enter");
    s.assert_has_row(
        "• Usage: /goal [<objective>|clear|edit|pause|resume] The session must start before you can change a goal.",
    );
}

// ORACLE chatwidget/tests/slash_commands.rs:1015 queued_goal_slash_command_restores_large_paste_for_edit kind=ui owner=chat-nav-pane
#[test]
fn queued_goal_slash_command_restores_large_paste_for_edit() {
    kit::na(
        Owner::ChatNavPane,
        "asserts the pending paste's content and the goal draft event; the paste content is not drawn, only its placeholder",
    );
}

// ORACLE chatwidget/tests/slash_commands.rs:1037 interrupt_disambiguates_same_sized_goal_pastes kind=ui owner=chat-nav-pane
#[test]
fn interrupt_disambiguates_same_sized_goal_pastes() {
    // MAX_USER_INPUT_TEXT_CHARS + 1 (protocol user_input.rs) is 1 << 20 plus one.
    let first = "a".repeat((1 << 20) + 1);
    let second = "b".repeat(first.len());
    let mut s = Screen::build(Owner::ChatNavPane).script(RUNNING).open();
    s.type_text("/goal ").paste(&first).keys("tab");
    s.type_text("/goal ").paste(&second);
    s.feed(INTERRUPTED);
    s.keys("up backspace");
    // The remaining paste's placeholder: the first of its size, with its suffix after the base.
    s.assert_contains("[Pasted Content 1048577 chars]");
}

// ORACLE chatwidget/tests/slash_commands.rs:1240 interrupted_merged_message_history_encodes_mentions_once kind=ui owner=chat-nav-pane
#[test]
fn interrupted_merged_message_history_encodes_mentions_once() {
    let mut s = Screen::build(Owner::ChatNavPane).script(RUNNING).open();
    s.type_text("use $figma now").keys("enter");
    s.assert_contains("use $figma now");
    s.keys("esc");
    s.assert_contains("use $figma now");
    // The encoded history line ("use [$figma](app://figma) now") is recall data, not drawn.
}

// ORACLE chatwidget/tests/slash_commands.rs:1299 slash_rename_prefills_existing_thread_name kind=snap owner=chat-nav-pane snap=slash_rename_prefilled_prompt
#[test]
fn slash_rename_prefills_existing_thread_name() {
    let mut s = Screen::build(Owner::ChatNavPane).script(NAMED).open();
    s.type_text("/rename").keys("enter");
    // The oracle's manual chat has no thread id, so it asks for no suggestion: a thread with no
    // conversation asks for none here either, which leaves the prompt as the oracle shows it.
    s.answer_title(None);
    s.assert_snap_region("slash_rename_prefilled_prompt", 19, 0);
    s.keys("enter");
    s.assert_set_names(&["Current project title"]);
}

// ORACLE chatwidget/tests/slash_commands.rs:1317 slash_rename_requests_and_prefills_an_editable_title_suggestion kind=snap owner=chat-nav-pane snap=slash_rename_suggestion_loading snap=slash_rename_suggestion_prefilled
#[test]
fn slash_rename_requests_and_prefills_an_editable_title_suggestion() {
    let mut s = Screen::build(Owner::ChatNavPane).script(NAMED).open();
    s.feed(TALKED);
    s.type_text("/rename").keys("enter");
    s.assert_snap_region("slash_rename_suggestion_loading", 18, 0);
    s.answer_title(Some("Fix login timeout"));
    s.assert_snap_region("slash_rename_suggestion_prefilled", 18, 0);
    s.keys("enter");
    s.assert_set_names(&["Fix login timeout"]);
}

// ORACLE chatwidget/tests/slash_commands.rs:1354 slash_rename_preserves_manual_edits_when_suggestion_arrives kind=ui owner=chat-nav-pane
#[test]
fn slash_rename_preserves_manual_edits_when_suggestion_arrives() {
    let mut s = Screen::build(Owner::ChatNavPane).script(SESSION).open();
    s.feed(TALKED);
    s.type_text("/rename").keys("enter");
    s.paste("My own title");
    s.answer_title(Some("Generated title"));
    s.assert_contains("My own title");
    s.assert_absent("Generated title");
    s.assert_absent("Generating a title suggestion");
    s.keys("enter");
}

// ORACLE chatwidget/tests/slash_commands.rs:1389 slash_rename_without_existing_thread_name_starts_empty kind=ui owner=chat-nav-pane
#[test]
fn slash_rename_without_existing_thread_name_starts_empty() {
    let mut s = Screen::build(Owner::ChatNavPane).live().open();
    s.type_text("/rename").keys("enter");
    s.assert_contains("Name thread");
    s.assert_contains("Type a name and press Enter");
    s.keys("enter");
}

// ORACLE chatwidget/tests/slash_commands.rs:1404 usage_error_slash_command_is_available_from_local_recall kind=ui owner=chat-nav-pane
#[test]
fn usage_error_slash_command_is_available_from_local_recall() {
    let mut s = Screen::build(Owner::ChatNavPane).live().open();
    s.type_text("/raw maybe").keys("esc enter enter");
    s.assert_contains("Usage: /raw [on|off]");
    s.assert_composer("");
    s.keys("up");
    s.assert_has_row("› /raw maybe");
}

// ORACLE chatwidget/tests/slash_commands.rs:1425 signed_out_usage_command_reports_chatgpt_login_requirement kind=snap owner=chat-nav-pane snap=signed_out_usage_command_reports_chatgpt_login_requirement
#[test]
fn signed_out_usage_command_reports_chatgpt_login_requirement() {
    let mut s = Screen::build(Owner::ChatNavPane).script(SIGNED_OUT).open();
    s.type_text("/usage").keys("esc enter enter");
    s.assert_contains("■ Sign in with ChatGPT to use /usage.");
    s.keys("up");
    s.assert_has_row("› /usage");
}

// ORACLE chatwidget/tests/slash_commands.rs:1444 signed_out_usage_command_with_args_reports_chatgpt_login_requirement kind=ui owner=chat-nav-pane
#[test]
fn signed_out_usage_command_with_args_reports_chatgpt_login_requirement() {
    let mut s = Screen::build(Owner::ChatNavPane).script(SIGNED_OUT).open();
    s.type_text("/usage weekly").keys("esc enter enter");
    s.assert_contains("Sign in with ChatGPT to use /usage.");
    s.keys("up");
    s.assert_has_row("› /usage weekly");
}

// ORACLE chatwidget/tests/slash_commands.rs:1463 usage_command_with_invalid_view_reports_usage_snapshot kind=snap owner=chat-nav-pane snap=usage_command_with_invalid_view_reports_usage
#[test]
fn usage_command_with_invalid_view_reports_usage_snapshot() {
    let mut s = Screen::build(Owner::ChatNavPane).live().open();
    s.type_text("/usage monthly").keys("esc enter enter");
    s.assert_contains("■ Usage: /usage [daily|weekly|cumulative]");
    s.assert_composer("");
    s.keys("up");
    s.assert_has_row("› /usage monthly");
}

// ORACLE chatwidget/tests/slash_commands.rs:1538 unrecognized_slash_command_is_not_added_to_local_recall kind=ui owner=chat-nav-pane
#[test]
fn unrecognized_slash_command_is_not_added_to_local_recall() {
    let mut s = Screen::build(Owner::ChatNavPane).live().open();
    s.type_text("/does-not-exist").keys("esc enter");
    s.assert_contains("Unrecognized command '/does-not-exist'");
    s.assert_has_row("› /does-not-exist");
    for _ in 0.."/does-not-exist".len() {
        s.keys("backspace");
    }
    s.keys("up");
    // Nothing recalled: the composer stays empty.
    s.assert_has_row("› Ask Codex to do anything");
}

// ORACLE chatwidget/tests/slash_commands.rs:1558 unavailable_slash_command_is_available_from_local_recall kind=ui owner=chat-nav-pane
#[test]
fn unavailable_slash_command_is_available_from_local_recall() {
    let mut s = Screen::build(Owner::ChatNavPane).script(RUNNING).open();
    s.type_text("/review").keys("esc enter");
    s.assert_contains("'/review' is disabled while a task is in progress.");
    // the composer keeps the refused line, to be corrected
    s.assert_composer("/review");
    for _ in 0.."/review".len() {
        s.keys("backspace");
    }
    s.keys("up");
    s.assert_has_row("› /review");
}

// ORACLE chatwidget/tests/slash_commands.rs:1578 no_op_stub_slash_command_is_available_from_local_recall kind=ui owner=chat-nav-pane
#[test]
fn no_op_stub_slash_command_is_available_from_local_recall() {
    let mut s = Screen::build(Owner::ChatNavPane).live().open();
    s.type_text("/debug-m-drop").keys("esc enter");
    s.assert_contains("Memory maintenance");
    for _ in 0.."/debug-m-drop".len() {
        s.keys("backspace");
    }
    s.keys("up");
    s.assert_has_row("› /debug-m-drop");
}

// ORACLE chatwidget/tests/slash_commands.rs:1627 slash_copy_picker_uses_completed_commentary_during_active_turn kind=snap owner=chat-nav-pane
#[test]
fn slash_copy_picker_uses_completed_commentary_during_active_turn() {
    let mut s = Screen::build(Owner::ChatNavPane).script(COMMENTARY).open();
    s.type_text("/copy").keys("enter");
    s.assert_contains("Whole response");
    s.assert_contains("sh code");
    s.keys("1");
    s.assert_copies_asked(&["Here is the command:\n\n```sh\necho current\n```"]);
    s.assert_no_turn();
}

// ORACLE chatwidget/tests/slash_commands.rs:1664 slash_copy_state_tracks_plan_item_completion kind=ui owner=chat-nav-pane
#[test]
fn slash_copy_state_tracks_plan_item_completion() {
    let mut s = Screen::build(Owner::ChatNavPane).script(PLAN).open();
    s.type_text("/copy").keys("enter");
    s.assert_contains("Whole response");
    s.assert_contains("sh code");
    s.keys("1");
    // The copied texts of the later rows (the plan, the sh code) are clipboard data, not drawn.
}

// ORACLE chatwidget/tests/slash_commands.rs:1723 slash_copy_picker_preserves_completed_source_whitespace_and_hides_directives kind=ui owner=chat-nav-pane
#[test]
fn slash_copy_picker_preserves_completed_source_whitespace_and_hides_directives() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(100, 24)
        .script(DIRECTIVES)
        .open();
    for key in ["1", "2", "3"] {
        s.type_text("/copy").keys("enter");
        // The oracle counts one "Blockquote" row; the kit has no count, so this checks it is there.
        s.assert_contains("Blockquote");
        s.keys(key);
    }
    // The copied texts (whitespace kept, directives hidden) are clipboard data, not drawn.
}

// ORACLE chatwidget/tests/slash_commands.rs:1777 slash_copy_picker_defers_queued_input_until_selection_or_cancellation_settles kind=ui owner=chat-nav-pane
#[test]
fn slash_copy_picker_defers_queued_input_until_selection_or_cancellation_settles() {
    for queued in ["/new", "/clear", "follow-up prompt"] {
        for close_key in ["enter", "esc"] {
            let mut s = Screen::build(Owner::ChatNavPane).script(PREVIOUS).open();
            s.feed(ACTIVE);
            s.type_text(queued).keys("tab");
            s.type_text("/copy").keys("enter");
            s.feed(ACTIVE_DONE);
            s.assert_contains("Previous response");
            s.keys(close_key);
            if queued == "follow-up prompt" {
                s.assert_contains("follow-up prompt");
            }
        }
    }
}

// ORACLE chatwidget/tests/slash_commands.rs:1834 slash_copy_picker_previews_whole_response_code_blocks_and_blockquotes kind=ui owner=chat-nav-pane
#[test]
fn slash_copy_picker_previews_whole_response_code_blocks_and_blockquotes() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(100, 24)
        .script(PREVIEW)
        .open();
    s.type_text("/copy").keys("enter");
    s.assert_contains("1. Whole response");
    s.assert_contains("Overview");
    s.assert_contains("2. python code");
    s.assert_contains("print('first')");
    s.assert_contains("3. Blockquote");
    s.assert_contains("Important quote");
    s.assert_contains("4. sh code");
    s.assert_contains("echo done");
}

// ORACLE chatwidget/tests/slash_commands.rs:1861 slash_copy_picker_copies_status_fields_and_preserves_source_after_copying kind=snap owner=chat-nav-pane snap=slash_copy_picker_status_fields snap=slash_copy_whole_status
#[test]
fn slash_copy_picker_copies_status_fields_and_preserves_source_after_copying() {
    kit::pending(
        Owner::ChatNavPane,
        "no /copy picker: the port answers /copy with 'not available yet', so the picker rows are not drawn; the snapshot's [[workspace]] and VERSION placeholders need the cwd and version seeds",
    );
    // The oracle sets the cwd, the thread name and the model before /status; the kit cannot set them.
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(100, 24)
        .script(SESSION)
        .open();
    s.type_text("/status").keys("esc enter enter");
    s.type_text("/copy").keys("enter");
    s.assert_snap("slash_copy_picker_status_fields");
    s.keys("enter");
    // The whole status (slash_copy_whole_status) is clipboard data, not drawn.
}

// ORACLE chatwidget/tests/slash_commands.rs:1914 slash_copy_status_omits_missing_fields_and_copies_full_directory kind=ui owner=chat-nav-pane
#[test]
fn slash_copy_status_omits_missing_fields_and_copies_full_directory() {
    kit::pending(
        Owner::ChatNavPane,
        "no /copy picker: the port answers /copy with 'not available yet', so the picker rows are not drawn; the status card above the picker shows the rows the test asserts absent, which the kit cannot scope",
    );
    let mut s = Screen::build(Owner::ChatNavPane).live().open();
    s.type_text("/status").keys("esc enter enter");
    s.type_text("/copy").keys("enter");
    s.assert_contains("2. Model");
    s.assert_contains("3. Directory");
    s.assert_absent("Thread name");
    s.assert_absent("Session ID");
    s.keys("3");
}

// ORACLE chatwidget/tests/slash_commands.rs:1936 slash_copy_status_yields_to_later_turns_and_commands_even_after_refresh kind=ui owner=chat-nav-pane
#[test]
fn slash_copy_status_yields_to_later_turns_and_commands_even_after_refresh() {
    kit::pending(
        Owner::ChatNavPane,
        "no /copy picker: the port answers /copy with 'not available yet', so the picker rows are not drawn; the ServiceTier case needs a model catalog seed",
    );
    for next in [
        "response",
        "user_turn",
        "command",
        "inline_command",
        "rejected_inline_command",
        "shell_command",
    ] {
        let mut s = Screen::build(Owner::ChatNavPane).script(ORIGINAL).open();
        s.type_text("/status").keys("esc enter enter");
        s.type_text("/copy").keys("enter");
        s.assert_contains("1. Whole status");
        s.keys("esc");
        match next {
            "response" => {
                s.feed(TURN2_REPLY);
            }
            "user_turn" => {
                s.type_text("Next question").keys("esc enter enter");
            }
            "command" => {
                s.type_text("/pwd").keys("esc enter enter");
            }
            "inline_command" => {
                s.type_text("/rename Renamed thread")
                    .keys("esc enter enter");
            }
            "rejected_inline_command" => {
                s.feed(TURN2_STARTED);
                s.type_text("/cd /tmp").keys("esc enter enter");
            }
            _ => {
                s.type_text("!echo hello").keys("esc enter enter");
            }
        }
        s.type_text("/copy").keys("enter");
        s.assert_contains("1. Whole response");
        s.keys("esc");
    }
    // ServiceTier (`handle_service_tier_command_dispatch`) needs a model catalog with service
    // tiers, which the kit cannot seed: that case is the pending reason's.
}

// ORACLE chatwidget/tests/slash_commands.rs:2002 slash_copy_status_yields_to_queued_commands_after_queued_status kind=ui owner=chat-nav-pane
#[test]
fn slash_copy_status_yields_to_queued_commands_after_queued_status() {
    for command in ["/rename Queued rename", "!echo hello"] {
        let mut s = Screen::build(Owner::ChatNavPane).script(RUNNING).open();
        s.type_text("/status").keys("tab");
        s.type_text(command).keys("tab");
        s.feed(ORIGINAL);
        s.assert_contains("/status");
        s.type_text("/copy").keys("enter");
        s.assert_contains("1. Whole response");
        s.keys("esc");
    }
}

// ORACLE chatwidget/tests/slash_commands.rs:2029 slash_copy_picker_waits_for_submission_after_typing_or_autocomplete kind=ui owner=chat-nav-pane
#[test]
fn slash_copy_picker_waits_for_submission_after_typing_or_autocomplete() {
    for select_from_autocomplete in [false, true] {
        let mut s = Screen::build(Owner::ChatNavPane).script(READY).open();
        s.type_text("/cop");
        if select_from_autocomplete {
            s.keys("tab");
        } else {
            s.keys("y");
        }
        s.assert_has_row("› /copy");
        s.assert_absent("Whole response");
        s.keys("enter");
        s.assert_contains("Whole response");
        // the picker takes the composer's rows (`render_bottom_popup`); the oracle's
        // `composer_text().is_empty()` is read once it is dismissed
        s.keys("esc");
        s.assert_has_row("› Ask Codex to do anything");
    }
}

// ORACLE chatwidget/tests/slash_commands.rs:2066 slash_copy_picker_waits_for_submission_after_paste kind=ui owner=chat-nav-pane
#[test]
fn slash_copy_picker_waits_for_submission_after_paste() {
    let mut s = Screen::build(Owner::ChatNavPane).script(READY).open();
    s.paste("/copy");
    s.assert_has_row("› /copy");
    s.assert_absent("Whole response");
    s.keys("enter");
    s.assert_contains("Whole response");
    // the picker takes the composer's rows (`render_bottom_popup`); the oracle's
    // `composer_text().is_empty()` is read once it is dismissed
    s.keys("esc");
    s.assert_has_row("› Ask Codex to do anything");
}

// ORACLE chatwidget/tests/slash_commands.rs:2086 slash_copy_picker_numeric_shortcuts_copy_whole_response_and_exact_code kind=ui owner=chat-nav-pane
#[test]
fn slash_copy_picker_numeric_shortcuts_copy_whole_response_and_exact_code() {
    kit::na(
        Owner::ChatNavPane,
        "asserts the CopySelection text and label sent on each numeric shortcut; the clipboard is not drawn",
    );
}

// ORACLE chatwidget/tests/slash_commands.rs:2111 slash_copy_picker_renders_only_whole_responses_as_markdown kind=ui owner=chat-nav-pane
#[test]
fn slash_copy_picker_renders_only_whole_responses_as_markdown() {
    kit::na(
        Owner::ChatNavPane,
        "asserts the CopySelection text and CopyFormat per row; the clipboard and its format are not drawn",
    );
}

// ORACLE chatwidget/tests/slash_commands.rs:2143 slash_copy_picker_preserves_nested_blockquote_markdown_and_code kind=ui owner=chat-nav-pane
#[test]
fn slash_copy_picker_preserves_nested_blockquote_markdown_and_code() {
    kit::na(
        Owner::ChatNavPane,
        "asserts the CopySelection texts for the blockquote and code rows; the clipboard is not drawn",
    );
}

// ORACLE chatwidget/tests/slash_commands.rs:2168 slash_copy_picker_supports_arrow_navigation_enter_and_scrolling kind=ui owner=chat-nav-pane
#[test]
fn slash_copy_picker_supports_arrow_navigation_enter_and_scrolling() {
    let mut s = Screen::build(Owner::ChatNavPane).script(SH_BLOCKS).open();
    s.type_text("/copy").keys("enter");
    for _ in 0..11 {
        s.keys("down");
    }
    s.assert_contains("example_11()");
    s.keys("up enter");
}

// ORACLE chatwidget/tests/slash_commands.rs:2195 slash_copy_picker_escape_dismisses_without_copying kind=ui owner=chat-nav-pane
#[test]
fn slash_copy_picker_escape_dismisses_without_copying() {
    let mut s = Screen::build(Owner::ChatNavPane).script(READY).open();
    s.type_text("/copy").keys("enter");
    s.assert_contains("Whole response");
    s.keys("esc");
    s.assert_absent("Whole response");
}
