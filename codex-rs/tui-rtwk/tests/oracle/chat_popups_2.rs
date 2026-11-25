//! Shard `chat_popups_2` (screens S14, owner dialogs-pane): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use serde_json::json;

use crate::chat_popups_3::{plugin, seed, select_tab};
use codex_app_server_protocol::ApprovalsReviewer;
use codex_tui_core::app::Command;
use codex_tui_core::screens::{ScreenCommand, select};

use crate::kit::{self, Owner, Screen};

/// The first row that contains `text`; past the screen when none does (the kit then names the row it looked for).
fn row_of(s: &Screen, text: &str) -> usize {
    s.rows()
        .iter()
        .position(|r| r.contains(text))
        .unwrap_or(usize::MAX)
}

/// The first row that starts with `text`: a history cell's row (`■`, `•`).
fn row_starting(s: &Screen, text: &str) -> usize {
    s.rows()
        .iter()
        .position(|r| r.starts_with(text))
        .unwrap_or(usize::MAX)
}

const NO_HISTORY: &str = "the oracle's chat has no thread, so its legacy list says `Permissions updated to X` at once (the row the config has too); the port's `/permissions` runs on a thread and says `Permission selection requested: X` once the server took the update, and nothing for the row the thread already has";
/// The server's answers to a `/permissions` read and a pick: no requirements, the three built-in
/// profiles, the thread's update taken.
const PERMISSION_CATALOG: &str = r#"{"step":"respond","method":"configRequirements/read","result":{"requirements":null}}
{"step":"respond","method":"permissionProfile/list","result":{"data":[{"id":":read-only","description":null,"allowed":true},{"id":":workspace","description":null,"allowed":true},{"id":":danger-full-access","description":null,"allowed":true}],"nextCursor":null}}
{"step":"respond","method":"thread/settings/update","result":{}}"#;

const NO_COVER_HOLD: &str = "feature the inline `/permissions <profile>` pick waits behind a covering modal (w2-approvals; the oracle's `apply_pending_permission_profile` runs when the picker's view is the active one again): the port applies the pick the moment the catalog is read, under the exec approval, so the thread's settings update is asked before the approval is answered; the select model has no `covered` fact and the app (core/src/app/select.rs, not this package's file) does not tell it when a request dialog is up";

/// The chat's status facts of the oracle's `make_chatwidget_manual` with the workspace profile
/// active: on-request approvals, the `:workspace` profile, the `Approve for me` row a feature.
const WORKSPACE_FACTS: &str = r#"{"step":"screen","dir":"chat","ev":{"status":{"approval_policy":"on-request","profile":"workspace","permission_profile":":workspace","permissions":{"guardian":true,"auto_available":true}}}}"#;

/// A modal that covers the picker: an exec approval of the live chat.
const COVERING_REQUEST: &str = r#"{"step":"request","message":{"method":"item/commandExecution/requestApproval","id":7,"params":{"kind":"command","itemId":"call-cover","approvalId":"call-cover","environmentId":null,"command":"echo hi","cwd":"/tmp","threadId":"thread-1","turnId":"turn-1","startedAtMs":0}}}"#;

/// A turn notification of `thread-1`'s turn `turn-1` with `status`.
fn turn_note(method: &str, status: &str) -> String {
    format!(
        r#"{{"step":"notify","message":{{"method":"{method}","params":{{"threadId":"thread-1","turn":{{"id":"turn-1","rootTurnId":null,"items":[],"itemsView":"full","status":"{status}","error":null,"startedAt":null,"completedAt":null,"durationMs":null}}}}}}}}"#
    )
}

/// A turn start that carries the queued follow-up.
fn is_turn_start_of_follow_up(command: &Command) -> bool {
    matches!(command, Command::StartTurn { message, .. } if message.text == "follow-up")
}

/// The thread session of `permissions_selection_marks_auto_review_current_with_custom_workspace_write_details`.
const AUTO_REVIEW_FACTS: &str = r#"{"step":"screen","dir":"chat","ev":{"status":{"approval_policy":"on-request","profile":"workspace","auto_review":true,"permissions":{"guardian":true,"auto_available":true}}}}"#;
/// The same thread with the user reviewing.
const USER_REVIEW_FACTS: &str = r#"{"step":"screen","dir":"chat","ev":{"status":{"approval_policy":"on-request","profile":"workspace","permission_profile":":workspace","permissions":{"guardian":true,"auto_available":true}}}}"#;

/// The asks the select screen made of the app, oldest first.
fn asked(s: &Screen, wanted: impl Fn(&select::Command) -> bool) -> Vec<Command> {
    s.commands()
        .into_iter()
        .filter(|command| {
            matches!(
                command,
                Command::Screen {
                    command: ScreenCommand::Select(asked),
                    ..
                } if wanted(asked)
            )
        })
        .collect()
}

/// The permission profile ids the thread was asked to change to, oldest first.
fn updates(s: &Screen) -> Vec<Option<String>> {
    s.commands()
        .into_iter()
        .filter_map(|command| match command {
            Command::Screen {
                command: ScreenCommand::Select(select::Command::Settings { patch, .. }),
                ..
            } => Some(patch.permissions),
            _ => None,
        })
        .collect()
}

/// The three built-in profiles, as the server lists them (the oracle's `Discovery::local`).
fn built_ins() -> serde_json::Value {
    json!([
        {"id": ":read-only", "description": null, "allowed": true},
        {"id": ":workspace", "description": null, "allowed": true},
        {"id": ":danger-full-access", "description": null, "allowed": true},
    ])
}

/// The server answers the requirements read (`requirements`, its JSON) and the one page of
/// profiles that follows it (`profiles`, the JSON list of summaries).
fn read_catalog(s: &mut Screen, requirements: serde_json::Value, profiles: serde_json::Value) {
    let reads = asked(s, |c| matches!(c, select::Command::Requirements { .. }));
    let read = reads.last().expect("the requirements were asked");
    s.answer_request(read, json!({ "requirements": requirements }));
    let pages = asked(s, |c| matches!(c, select::Command::Profiles { .. }));
    let page = pages.last().expect("a page of profiles was asked");
    s.answer_request(page, json!({ "data": profiles, "nextCursor": null }));
}

// ---- oracle: chatwidget/tests/permissions.rs (42 screen-level tests here; 11 state tests left to dialogs-pane)
// ORACLE chatwidget/tests/permissions.rs:41 permission_discovery_uses_server_catalog_for_remote_custom_selection kind=snap owner=dialogs-pane snap=permission_discovery_loading snap=permission_discovery_remote
#[test]
fn permission_discovery_uses_server_catalog_for_remote_custom_selection() {
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    // the oracle's chat: the workspace profile active, auto-review a feature of the config
    s.feed(WORKSPACE_FACTS);
    s.type_text("/permissions").keys("enter");
    s.assert_snap_region(
        "permission_discovery_loading",
        row_of(&s, "Update Model Permissions"),
        0,
    );
    read_catalog(
        &mut s,
        json!({
            "allowedApprovalsReviewers": ["user"],
            "allowedPermissionProfiles": {":workspace": true, "server-only": true},
        }),
        json!([
            {"id": ":read-only", "description": null, "allowed": true},
            {"id": ":workspace", "description": null, "allowed": true},
            {"id": ":danger-full-access", "description": null, "allowed": true},
            {
                "id": "server-only",
                "description": "Description from the connected server.",
                "allowed": true,
            },
        ]),
    );
    s.resize(110, 24);
    s.assert_snap_region(
        "permission_discovery_remote",
        row_of(&s, "Update Model Permissions"),
        0,
    );
    // The server-only alternative is routed to the app, not resolved by the popup.
    s.keys("2");
    s.assert_absent("Update Model Permissions");
    assert_eq!(updates(&s), [Some("server-only".to_string())]);
}

// ORACLE chatwidget/tests/permissions.rs:92 permissions_command_waits_for_covering_modal_before_selecting_profile kind=ui owner=dialogs-pane
#[test]
fn permissions_command_waits_for_covering_modal_before_selecting_profile() {
    kit::pending(Owner::DialogsPane, NO_COVER_HOLD);
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.type_text("/permissions :read-only").keys("enter");
    s.assert_contains("Loading permission profiles");
    // The oracle opens another modal first: the picker waits behind it.
    s.feed(COVERING_REQUEST);
    s.assert_contains("Would you like to run the following command?");
    read_catalog(&mut s, serde_json::Value::Null, built_ins());
    s.assert_contains("Would you like to run the following command?");
    assert!(updates(&s).is_empty());
    s.keys("esc");
    s.assert_absent("Update Model Permissions");
    assert_eq!(updates(&s), [Some(":read-only".to_string())]);
}

// ORACLE chatwidget/tests/permissions.rs:122 cancelled_permissions_command_does_not_apply_after_later_modal kind=ui owner=dialogs-pane
#[test]
fn cancelled_permissions_command_does_not_apply_after_later_modal() {
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    // a turn runs with a follow-up queued behind it
    s.feed(&turn_note("turn/started", "inProgress"));
    s.type_text("follow-up").keys("tab");
    s.type_text("/permissions :read-only").keys("enter");
    s.assert_contains("Loading permission profiles");
    s.keys("esc");
    s.assert_absent("Update Model Permissions");
    // the catalog comes after the cancel: it opens nothing and applies nothing
    read_catalog(&mut s, serde_json::Value::Null, built_ins());
    s.assert_absent("Update Model Permissions");
    assert!(updates(&s).is_empty(), "a cancelled profile must not apply");
    // the queued follow-up is held back (recovered): it is not sent when the turn ends
    s.feed(&turn_note("turn/completed", "completed"));
    s.assert_contains("follow-up");
    assert!(
        !s.commands().iter().any(is_turn_start_of_follow_up),
        "the follow-up is not sent as a turn"
    );
}

// ORACLE chatwidget/tests/permissions.rs:153 permissions_command_preserves_full_access_confirmation kind=ui owner=dialogs-pane
#[test]
fn permissions_command_preserves_full_access_confirmation() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .live()
        .script(PERMISSION_CATALOG)
        .open();
    s.feed(
        r#"{"step":"screen","dir":"chat","ev":{"status":{"permissions":{"guardian":true,"auto_available":true}}}}"#,
    );
    // a turn runs with a follow-up queued behind it
    s.feed(&turn_note("turn/started", "inProgress"));
    s.type_text("follow-up").keys("tab");
    s.type_text("/permissions :danger-full-access --reviewer=auto_review")
        .keys("enter");
    s.assert_contains("Enable full access?");
    // Cancel returns to the picker; closing that picker must still recover the queue.
    s.keys("down enter");
    s.assert_contains("Update Model Permissions");
    s.keys("esc");
    s.assert_absent("Update Model Permissions");
    assert!(updates(&s).is_empty());
    s.feed(&turn_note("turn/completed", "completed"));
    s.assert_contains("follow-up");
    assert!(
        !s.commands().iter().any(is_turn_start_of_follow_up),
        "the follow-up is not sent as a turn"
    );
}

// ORACLE chatwidget/tests/permissions.rs:199 permissions_command_rejects_disabled_profile kind=snap owner=dialogs-pane
#[test]
fn permissions_command_rejects_disabled_profile() {
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.type_text("/permissions :read-only").keys("enter");
    read_catalog(
        &mut s,
        json!({"allowedPermissionProfiles": {":read-only": false}}),
        built_ins(),
    );
    s.assert_snap_region(
        "permissions_command_disabled_profile",
        row_starting(&s, "■"),
        0,
    );
    s.assert_absent("Update Model Permissions");
    assert!(updates(&s).is_empty());
}

// ORACLE chatwidget/tests/permissions.rs:229 permissions_command_rejects_unknown_profile kind=snap owner=dialogs-pane
#[test]
fn permissions_command_rejects_unknown_profile() {
    // the oracle's snapshot is the cell's logical line (a cell has no width of its own): wide
    // enough that the transcript does not wrap it
    let mut s = Screen::build(Owner::DialogsPane)
        .size(100, 24)
        .live()
        .open();
    s.type_text("/permissions unknown-profile").keys("enter");
    read_catalog(&mut s, serde_json::Value::Null, built_ins());
    s.assert_snap_region(
        "permissions_command_unknown_profile",
        row_starting(&s, "■"),
        0,
    );
    s.assert_absent("Update Model Permissions");
    assert!(updates(&s).is_empty());
}

// ORACLE chatwidget/tests/permissions.rs:269 inline_permission_discovery_retry_preserves_profile_id kind=ui owner=dialogs-pane
#[test]
fn inline_permission_discovery_retry_preserves_profile_id() {
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.type_text("/permissions server-only").keys("enter");
    let reads = asked(&s, |c| matches!(c, select::Command::Requirements { .. }));
    s.answer_error(&reads[0], "disconnected");
    s.assert_contains("Failed to load permissions: disconnected");
    // Enter retries the load; the retry keeps the profile id the picker was opened with.
    s.keys("enter");
    s.assert_contains("Update Model Permissions");
    assert!(updates(&s).is_empty());
    let mut profiles = built_ins().as_array().cloned().unwrap_or_default();
    profiles.push(json!({"id": "server-only", "description": null, "allowed": true}));
    read_catalog(&mut s, serde_json::Value::Null, json!(profiles));
    assert_eq!(updates(&s), [Some("server-only".to_string())]);
    s.assert_absent("Update Model Permissions");
}

// ORACLE chatwidget/tests/permissions.rs:296 permission_discovery_discards_stale_results_and_preserves_covering_modal kind=ui owner=dialogs-pane
#[test]
fn permission_discovery_discards_stale_results_and_preserves_covering_modal() {
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.type_text("/permissions").keys("enter");
    s.keys("esc");
    s.assert_absent("Update Model Permissions");
    s.type_text("/permissions").keys("enter");
    s.keys("esc");
    s.assert_absent("Update Model Permissions");
}

// ORACLE chatwidget/tests/permissions.rs:349 permission_discovery_failure_can_retry_and_empty_catalog_keeps_current_profile kind=snap owner=dialogs-pane snap=permission_discovery_empty snap=permission_discovery_failure
#[test]
fn permission_discovery_failure_can_retry_and_empty_catalog_keeps_current_profile() {
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    // the oracle's chat: a named profile, `current-profile`, is the active one
    s.feed(
        r#"{"step":"screen","dir":"chat","ev":{"status":{"permission_profile":"current-profile"}}}"#,
    );
    s.type_text("/permissions").keys("enter");
    let reads = asked(&s, |c| matches!(c, select::Command::Requirements { .. }));
    s.answer_error(&reads[0], "disconnected");
    s.assert_snap_region(
        "permission_discovery_failure",
        row_of(&s, "Update Model Permissions"),
        0,
    );
    // Retry: the load comes back with an empty catalog, the current profile is kept.
    s.keys("enter");
    s.resize(100, 24);
    read_catalog(&mut s, serde_json::Value::Null, json!([]));
    s.assert_snap_region(
        "permission_discovery_empty",
        row_of(&s, "Update Model Permissions"),
        0,
    );
    // the current profile row is disabled: Enter asks nothing of the thread
    s.keys("enter");
    s.assert_contains("Update Model Permissions");
    assert!(updates(&s).is_empty());
}

// ORACLE chatwidget/tests/permissions.rs:442 approvals_selection_popup_snapshot kind=snap owner=dialogs-pane snap=approvals_selection_popup
#[test]
fn approvals_selection_popup_snapshot() {
    let s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .entry("select", "permissions_menu")
        .open();
    s.assert_snap_region(
        "approvals_selection_popup",
        row_of(&s, "Update Model Permissions"),
        0,
    );
}

// ORACLE chatwidget/tests/permissions.rs:458 profile_permissions_selection_popup_snapshot kind=snap owner=dialogs-pane snap=profile_permissions_selection_popup
#[test]
fn profile_permissions_selection_popup_snapshot() {
    let s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .entry("select", "profile_permissions")
        .open();
    s.assert_snap_region(
        "profile_permissions_selection_popup",
        row_of(&s, "Update Model Permissions"),
        0,
    );
}

// ORACLE chatwidget/tests/permissions.rs:478 profile_permissions_selection_popup_with_disallowed_full_access_snapshot kind=snap owner=dialogs-pane snap=profile_permissions_selection_popup_with_disallowed_full_access
#[test]
fn profile_permissions_selection_popup_with_disallowed_full_access_snapshot() {
    let s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .entry("select", "profile_permissions_disallowed_full_access")
        .open();
    s.assert_snap_region(
        "profile_permissions_selection_popup_with_disallowed_full_access",
        row_of(&s, "Update Model Permissions"),
        0,
    );
}

// ORACLE chatwidget/tests/permissions.rs:505 profile_permissions_selection_popup_with_custom_profiles_snapshot kind=snap owner=dialogs-pane snap=profile_permissions_selection_popup_with_custom_profiles
#[test]
fn profile_permissions_selection_popup_with_custom_profiles_snapshot() {
    let s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .entry("select", "profile_permissions_custom")
        .open();
    s.assert_snap_region(
        "profile_permissions_selection_popup_with_custom_profiles",
        row_of(&s, "Update Model Permissions"),
        0,
    );
}

// ORACLE chatwidget/tests/permissions.rs:537 profile_permissions_selection_emits_named_profile_event_only kind=ui owner=dialogs-pane
#[test]
fn profile_permissions_selection_emits_named_profile_event_only() {
    kit::na(
        Owner::DialogsPane,
        "asserts the one SelectPermissionProfile event Enter sends (named profile, no approval policy) and its display label; no drawn text is asserted",
    );
}

// ORACLE chatwidget/tests/permissions.rs:580 profile_permissions_selection_emits_active_custom_profile kind=ui owner=dialogs-pane
#[test]
fn profile_permissions_selection_emits_active_custom_profile() {
    kit::na(
        Owner::DialogsPane,
        "asserts the one SelectPermissionProfile event Enter sends for the active custom profile; no drawn text is asserted",
    );
}

// ORACLE chatwidget/tests/permissions.rs:613 profile_permissions_selection_emits_auto_review_mode_event kind=ui owner=dialogs-pane
#[test]
fn profile_permissions_selection_emits_auto_review_mode_event() {
    kit::na(
        Owner::DialogsPane,
        "asserts the SelectPermissionProfile event with the auto-review reviewer; no drawn text is asserted",
    );
}

// ORACLE chatwidget/tests/permissions.rs:760 profile_permissions_full_access_always_opens_confirmation kind=ui owner=dialogs-pane
#[test]
fn profile_permissions_full_access_always_opens_confirmation() {
    kit::na(
        Owner::DialogsPane,
        "asserts exactly one OpenFullAccessConfirmation event on Enter from the profile popup; the confirmation's drawn text is not asserted",
    );
}

// ORACLE chatwidget/tests/permissions.rs:792 approvals_selection_popup_snapshot_windows_degraded_sandbox kind=ui owner=dialogs-pane
#[test]
fn approvals_selection_popup_snapshot_windows_degraded_sandbox() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .script(include_str!(
            "fixtures/chat_popups_2/windows_degraded.jsonl"
        ))
        .open();
    s.type_text("/permissions").keys("enter");
    s.assert_contains("Ask for approval (non-admin sandbox)");
    s.assert_contains("/setup-default-sandbox");
    // The oracle's snapshot is the legacy preset list (no thread); `/permissions` here opens the
    // server's catalog (its rows are in the catalog order), so the words are compared, not the
    // rows.
    s.assert_contains("The non-admin sandbox protects your files and prevents network access");
    s.assert_contains("upgrade to the default sandbox, run /setup-default-sandbox");
}

// ORACLE chatwidget/tests/permissions.rs:890 full_access_confirmation_popup_snapshot kind=snap owner=dialogs-pane snap=full_access_confirmation_popup
#[test]
fn full_access_confirmation_popup_snapshot() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .entry("select", "permissions_menu")
        .open();
    s.keys("down enter");
    s.assert_contains("Enable full access?");
    s.assert_contains("When Codex runs with full access, it can edit any file on your");
    s.assert_contains("computer and run commands with network, without your approval.");
    s.assert_contains("increases the risk of data loss, leaks, or unexpected behavior.");
    s.assert_contains("› 1. Yes, continue anyway  Apply full access for this session");
    s.assert_contains("  2. Cancel                Go back without enabling full access");
    // the oracle's frame is the dialog over its retained composer: `› As` and `GP` show in the
    // four columns left of it. Ours is over the debug list, which shows in those four and past
    // the dialog's right edge; the dialog itself is the rows between.
    assert_dialog_rows(&s, "full_access_confirmation_popup", 4, 76);
}

/// The rows of the oracle's snapshot `name` against ours from the row of its first text: the
/// columns before `from` and from `to` on belong to what is under the dialog in either screen.
fn assert_dialog_rows(s: &Screen, name: &str, from: usize, to: usize) {
    let want = kit::snap_rows(name, "");
    let Some(first) = want.iter().find(|row| !row.trim().is_empty()) else {
        panic!("the snapshot `{name}` has no text");
    };
    let first = first.trim();
    let ours = s.rows();
    let top = ours
        .iter()
        .position(|row| row.contains(first))
        .unwrap_or_else(|| panic!("no `{first}` on the screen:\n{}", s.text()));
    let cut = |row: &str| -> String {
        row.chars()
            .skip(from)
            .take(to - from)
            .collect::<String>()
            .trim_end()
            .to_owned()
    };
    let want: Vec<String> = want.iter().map(|row| cut(row)).collect();
    let have: Vec<String> = (0..want.len())
        .map(|at| ours.get(top + at).map_or(String::new(), |row| cut(row)))
        .collect();
    let want: Vec<&str> = want.iter().map(String::as_str).collect();
    kit::assert_rows_are(&have, &want);
}

// ORACLE chatwidget/tests/permissions.rs:907 windows_auto_mode_prompt_requests_enabling_sandbox_feature kind=ui owner=dialogs-pane
#[test]
fn windows_auto_mode_prompt_requests_enabling_sandbox_feature() {
    // a Windows host with no sandbox set up (the folder's trust is not read, so no startup prompt)
    let mut s = Screen::build(Owner::DialogsPane)
        .size(120, 24)
        .live()
        .script(PERMISSION_CATALOG)
        .open();
    s.feed(
        r#"{"step":"fact","feature":"windows_sandbox","fact":{"kind":"environment","mode":null,"host":"local"}}"#,
    );
    s.type_text("/permissions").keys("enter");
    s.assert_contains("1. Ask for approval");
    s.assert_absent("requires Administrator permissions");
    // the Default pick (Auto, approvals to the user) leads to the enable prompt
    s.keys("enter");
    s.assert_contains("requires Administrator permissions");
    s.assert_contains("Use non-admin sandbox");
}

// ORACLE chatwidget/tests/permissions.rs:928 startup_prompts_for_windows_sandbox_when_agent_requested kind=snap owner=dialogs-pane snap=projectless_windows_sandbox_setup
#[test]
fn startup_prompts_for_windows_sandbox_when_agent_requested() {
    let s = Screen::build(Owner::DialogsPane)
        .size(120, 24)
        .script(include_str!("fixtures/chat_popups_2/windows_enable.jsonl"))
        .open();
    s.assert_contains("requires Administrator permissions");
    s.assert_contains("Set up default sandbox");
    s.assert_contains("Use non-admin sandbox");
    s.assert_contains("Quit");
    s.assert_snap_region(
        "projectless_windows_sandbox_setup",
        row_of(&s, "Set up the Codex agent sandbox"),
        0,
    );
}

// ORACLE chatwidget/tests/permissions.rs:991 startup_windows_sandbox_prompt_blocks_disallowed_unelevated_fallback kind=ui owner=dialogs-pane
#[test]
fn startup_windows_sandbox_prompt_blocks_disallowed_unelevated_fallback() {
    let s = Screen::build(Owner::DialogsPane)
        .size(120, 24)
        .script(include_str!(
            "fixtures/chat_popups_2/windows_required.jsonl"
        ))
        .open();
    s.assert_contains("Your organization requires the default Codex agent sandbox");
    s.assert_absent("Use non-admin sandbox");
}

// ORACLE chatwidget/tests/permissions.rs:1011 windows_sandbox_help_links_keep_complete_wrapped_destinations kind=snap owner=dialogs-pane
#[test]
fn windows_sandbox_help_links_keep_complete_wrapped_destinations() {
    let s = Screen::build(Owner::DialogsPane)
        .size(40, 24)
        .script(include_str!("fixtures/chat_popups_2/windows_enable.jsonl"))
        .open();
    s.assert_links_spell("https://developers.openai.com/codex/windows");
}

// ORACLE chatwidget/tests/permissions.rs:1054 windows_sandbox_required_enable_prompt_snapshot kind=snap owner=dialogs-pane snap=windows_sandbox_required_enable_prompt
#[test]
fn windows_sandbox_required_enable_prompt_snapshot() {
    let s = Screen::build(Owner::DialogsPane)
        .size(120, 24)
        .script(include_str!(
            "fixtures/chat_popups_2/windows_required.jsonl"
        ))
        .open();
    s.assert_snap_region(
        "windows_sandbox_required_enable_prompt",
        row_of(
            &s,
            "Your organization requires the default Codex agent sandbox",
        ),
        0,
    );
}

// ORACLE chatwidget/tests/permissions.rs:1072 windows_sandbox_required_enable_prompt_reopens_on_cancel_when_unelevated_allowed kind=ui owner=dialogs-pane
#[test]
fn windows_sandbox_required_enable_prompt_reopens_on_cancel_when_unelevated_allowed() {
    kit::na(
        Owner::DialogsPane,
        "asserts the OpenWindowsSandboxEnablePrompt event Esc sends (the prompt reopening); no drawn text is asserted",
    );
}

// ORACLE chatwidget/tests/permissions.rs:1095 fragmented_terminal_response_cannot_select_non_admin_windows_sandbox kind=ui owner=dialogs-pane
#[test]
fn fragmented_terminal_response_cannot_select_non_admin_windows_sandbox() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(120, 24)
        .script(include_str!("fixtures/chat_popups_2/windows_enable.jsonl"))
        .open();
    // The fragment is typed into the prompt: the prompt stays on screen and no choice is made.
    s.type_text("20;rgb:2222/ffff/ffff");
    s.assert_contains("Use non-admin sandbox");
    // Enter then chooses the non-admin sandbox; the oracle asserts that event.
}

// ORACLE chatwidget/tests/permissions.rs:1130 windows_sandbox_setup_starts_a_fresh_status_clock kind=snap owner=dialogs-pane snap=windows_sandbox_setup_fresh_clock
#[test]
fn windows_sandbox_setup_starts_a_fresh_status_clock() {
    // The default sandbox is chosen: the setup runs and the status says so.
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .script(include_str!("fixtures/chat_popups_2/windows_enable.jsonl"))
        .open();
    s.keys("enter");
    s.assert_contains("Setting up sandbox");
    s.assert_contains("(0s");
    // The snapshot `windows_sandbox_setup_fresh_clock` holds `[CWD]` for the test cwd; it is not
    // compared as is. The working status after the clear is not reached here.
}

// ORACLE chatwidget/tests/permissions.rs:1250 windows_sandbox_required_fallback_prompt_snapshot kind=snap owner=dialogs-pane snap=windows_sandbox_required_fallback_prompt
#[test]
fn windows_sandbox_required_fallback_prompt_snapshot() {
    // The required prompt's setup is chosen and ends in failure: the fallback prompt comes.
    let mut s = Screen::build(Owner::DialogsPane)
        .size(120, 24)
        .script(include_str!(
            "fixtures/chat_popups_2/windows_required.jsonl"
        ))
        .open();
    s.keys("enter");
    s.feed(include_str!(
        "fixtures/chat_popups_2/windows_elevated_failed.jsonl"
    ));
    s.assert_snap_region(
        "windows_sandbox_required_fallback_prompt",
        row_of(&s, "Couldn't set up your sandbox with Administrator"),
        0,
    );
}

// ORACLE chatwidget/tests/permissions.rs:1280 approvals_popup_shows_disabled_presets kind=ui owner=dialogs-pane
#[test]
fn approvals_popup_shows_disabled_presets() {
    let s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .entry("select", "permissions_constrained")
        .open();
    // The oracle collapses the whitespace: the reason wraps at 80 columns.
    s.assert_reads("(disabled)");
    s.assert_reads("this message should be printed in the description");
}

// ORACLE chatwidget/tests/permissions.rs:1317 approvals_popup_navigation_skips_disabled kind=ui owner=dialogs-pane
#[test]
fn approvals_popup_navigation_skips_disabled() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .entry("select", "permissions_constrained")
        .open();
    s.assert_contains("Update Model Permissions");
    // Ten Downs: the selection never lands on the disabled row.
    s.keys("down down down down down down down down down down");
    s.assert_contains("Update Model Permissions");
    // Enter selects an enabled preset; the popup closes.
    s.keys("enter");
    s.assert_absent("Update Model Permissions");
}

// ORACLE chatwidget/tests/permissions.rs:1413 permissions_selection_emits_history_cell_when_selection_changes kind=ui owner=dialogs-pane
#[test]
fn permissions_selection_emits_history_cell_when_selection_changes() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .live()
        .script(PERMISSION_CATALOG)
        .open();
    s.feed(
        r#"{"step":"screen","dir":"chat","ev":{"status":{"permissions":{"guardian":true,"auto_available":true}}}}"#,
    );
    s.type_text("/permissions").keys("enter");
    s.keys("down enter");
    // the oracle's legacy list says `Permissions updated to ..` (`NO_HISTORY`): the thread's
    // pick says what was requested of the server
    s.assert_contains("Permission selection requested: Approve for me");
}

// ORACLE chatwidget/tests/permissions.rs:1438 permissions_selection_history_snapshot_after_mode_switch kind=snap owner=dialogs-pane snap=permissions_selection_history_after_mode_switch
#[test]
fn permissions_selection_history_snapshot_after_mode_switch() {
    kit::pending(Owner::DialogsPane, NO_HISTORY);
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.type_text("/permissions").keys("enter");
    s.keys("down enter");
    // The full-access confirmation: Enter confirms.
    s.keys("enter");
    s.assert_snap_region(
        "permissions_selection_history_after_mode_switch",
        row_of(&s, "Permissions updated to"),
        0,
    );
}

// ORACLE chatwidget/tests/permissions.rs:1474 permissions_selection_history_snapshot_full_access_to_default kind=snap owner=dialogs-pane snap=permissions_selection_history_full_access_to_default
#[test]
fn permissions_selection_history_snapshot_full_access_to_default() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .live()
        .script(PERMISSION_CATALOG)
        .open();
    s.feed(
        r#"{"step":"screen","dir":"chat","ev":{"status":{"approval_policy":"never","profile":"full","permission_profile":":danger-full-access"}}}"#,
    );
    s.type_text("/permissions").keys("enter");
    s.keys("up enter");
    s.assert_contains("Permission selection requested: Ask for approval");
    kit::pending(
        Owner::DialogsPane,
        "the oracle's chat has no thread, so its legacy list says `Permissions updated to Ask for approval` at once; the port's `/permissions` runs on a thread and says `Permission selection requested: Ask for approval` once the server took the update",
    );
    s.assert_snap_region(
        "permissions_selection_history_full_access_to_default",
        row_of(&s, "Permissions updated to"),
        0,
    );
}

// ORACLE chatwidget/tests/permissions.rs:1516 permissions_selection_emits_history_cell_when_current_is_selected kind=ui owner=dialogs-pane
#[test]
fn permissions_selection_emits_history_cell_when_current_is_selected() {
    kit::pending(Owner::DialogsPane, NO_HISTORY);
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.type_text("/permissions").keys("enter");
    s.keys("enter");
    s.assert_contains("Permissions updated to");
}

// ORACLE chatwidget/tests/permissions.rs:1549 permissions_selection_hides_auto_review_when_feature_disabled kind=snap owner=dialogs-pane
#[test]
fn permissions_selection_hides_auto_review_when_feature_disabled() {
    // the guardian feature is off in the chat's facts
    let mut s = Screen::build(Owner::DialogsPane)
        .size(120, 24)
        .live()
        .script(PERMISSION_CATALOG)
        .open();
    s.type_text("/permissions").keys("enter");
    s.assert_contains("Update Model Permissions");
    s.assert_absent("Loading permission profiles");
    s.assert_absent("Approve for me");
    s.keys("esc");
    s.type_text("/permissions :approve-for-me").keys("enter");
    s.assert_contains(
        "■ Approve for me is unavailable. Run `/permissions` to see available profiles.",
    );
    assert!(updates(&s).is_empty());
}

// ORACLE chatwidget/tests/permissions.rs:1574 permissions_selection_hides_auto_review_when_feature_disabled_even_if_auto_review_is_active kind=ui owner=dialogs-pane
#[test]
fn permissions_selection_hides_auto_review_when_feature_disabled_even_if_auto_review_is_active() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(120, 24)
        .live()
        .script(PERMISSION_CATALOG)
        .open();
    // the auto reviewer is the config's, the guardian feature is off
    s.feed(
        r#"{"step":"screen","dir":"chat","ev":{"status":{"approval_policy":"on-request","profile":"workspace","auto_review":true}}}"#,
    );
    s.type_text("/permissions").keys("enter");
    s.assert_contains("Update Model Permissions");
    s.assert_contains("Ask for approval");
    s.assert_absent("Approve for me");
}

// ORACLE chatwidget/tests/permissions.rs:1603 permissions_selection_marks_auto_review_current_after_session_configured kind=snap owner=dialogs-pane snap=permissions_unnamed_server_profile
#[test]
fn permissions_selection_marks_auto_review_current_after_session_configured() {
    let s = Screen::build(Owner::DialogsPane)
        .size(120, 24)
        .entry("select", "permissions_unnamed_server_profile")
        .open();
    s.assert_snap_region(
        "permissions_unnamed_server_profile",
        row_of(&s, "Update Model Permissions"),
        0,
    );
}

// ORACLE chatwidget/tests/permissions.rs:1653 permissions_selection_marks_auto_review_current_with_custom_workspace_write_details kind=ui owner=dialogs-pane
#[test]
fn permissions_selection_marks_auto_review_current_with_custom_workspace_write_details() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(120, 24)
        .live()
        .script(PERMISSION_CATALOG)
        .open();
    // the thread session: on-request approvals, the auto reviewer, a workspace-write profile
    // with no active profile id of its own; the guardian feature is on
    s.feed(AUTO_REVIEW_FACTS);
    s.type_text("/permissions").keys("enter");
    s.assert_contains("Approve for me (current)");
}

// ORACLE chatwidget/tests/permissions.rs:1706 permissions_selection_can_disable_auto_review kind=ui owner=dialogs-pane
#[test]
fn permissions_selection_can_disable_auto_review() {
    kit::na(
        Owner::DialogsPane,
        "asserts the UpdateApprovalsReviewer(User) and no UpdateFeatureFlags events on Up then Enter; no drawn text is asserted",
    );
}

// ORACLE chatwidget/tests/permissions.rs:1745 permissions_selection_sends_approvals_reviewer_in_override_turn_context kind=ui owner=dialogs-pane
#[test]
fn permissions_selection_sends_approvals_reviewer_in_override_turn_context() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(120, 24)
        .live()
        .script(PERMISSION_CATALOG)
        .open();
    // the workspace profile and on-request approvals, the user reviewing, the guardian feature on
    s.feed(USER_REVIEW_FACTS);
    s.type_text("/permissions").keys("enter");
    s.assert_row_starts_with(row_of(&s, "(current)"), "›");
    s.keys("down");
    s.assert_row_starts_with(row_of(&s, "Approve for me"), "›");
    // Enter asks the thread to change its settings; the oracle asserts the override turn context
    // with the auto reviewer and the workspace profile.
    s.keys("enter");
    let patches: Vec<_> = s
        .commands()
        .into_iter()
        .filter_map(|command| match command {
            Command::Screen {
                command: ScreenCommand::Select(select::Command::Settings { patch, .. }),
                ..
            } => Some(patch),
            _ => None,
        })
        .collect();
    assert_eq!(patches.len(), 1, "{patches:?}");
    assert_eq!(
        patches[0].approvals_reviewer,
        Some(ApprovalsReviewer::AutoReview)
    );
    assert_eq!(patches[0].permissions.as_deref(), Some(":workspace"));
}

// ORACLE chatwidget/tests/permissions.rs:1823 permissions_full_access_history_cell_emitted_only_after_confirmation kind=ui owner=dialogs-pane
#[test]
fn permissions_full_access_history_cell_emitted_only_after_confirmation() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .live()
        .script(PERMISSION_CATALOG)
        .open();
    s.type_text("/permissions").keys("enter");
    s.keys("down enter");
    s.assert_contains("Enable full access?");
    s.assert_absent("Permissions updated to");
    s.keys("enter");
    // The oracle's chat has no thread (the legacy list says `Permissions updated to ..` at
    // once); the port's `/permissions` runs on a thread and says so once the server took the
    // update.
    s.assert_contains("Permission selection requested: Full Access");
    s.assert_absent("Enable full access?");
}

// ORACLE chatwidget/tests/permissions.rs:1888 windows_sandbox_unelevated_policy_hides_admin_setup kind=snap owner=dialogs-pane snap=windows_sandbox_unelevated_policy
#[test]
fn windows_sandbox_unelevated_policy_hides_admin_setup() {
    let s = Screen::build(Owner::DialogsPane)
        .size(100, 24)
        .script(include_str!(
            "fixtures/chat_popups_2/windows_unelevated.jsonl"
        ))
        .open();
    s.assert_snap_region(
        "windows_sandbox_unelevated_policy",
        row_of(&s, "Set up the Codex agent sandbox"),
        0,
    );
}

// ---- oracle: chatwidget/tests/plugin_catalog_tests.rs (4 screen-level tests here; 0 state tests left to dialogs-pane)
// ORACLE chatwidget/tests/plugin_catalog_tests.rs:5 plugins_popup_uses_product_labels_for_remote_and_personal_tabs kind=snap owner=dialogs-pane
#[test]
fn plugins_popup_uses_product_labels_for_remote_and_personal_tabs() {
    let remote =
        |name: &str, raw: &str, id: &str, plugin_name: &str, display: &str, description: &str| {
            seed::remote_marketplace(
                name,
                raw,
                vec![seed::remote_summary(
                    id,
                    plugin_name,
                    Some(display),
                    Some(description),
                    false,
                )],
            )
        };
    let shelf = seed::shelf(vec![seed::marketplace(
        "codex-curated",
        Some(seed::personal_path()),
        Some("Personal"),
        vec![plugin(
            "plugin-local-docs",
            "local-docs",
            "Local Docs",
            "Local editable docs.",
            false,
        )],
    )]);
    seed::with(&shelf, |held| {
        held.remote = vec![
            remote(
                "workspace-directory",
                "Raw Workspace Directory",
                "plugins~Plugin_buildkite",
                "buildkite",
                "Buildkite",
                "Workspace CI.",
            ),
            remote(
                "workspace-shared-with-me-private",
                "Raw Shared Private",
                "plugins~Plugin_docs",
                "docs",
                "Docs",
                "Shared docs.",
            ),
            remote(
                "workspace-shared-with-me-unlisted",
                "Raw Shared Link",
                "plugins~Plugin_link",
                "link",
                "Link Share",
                "Shared by link.",
            ),
        ];
    });
    let mut s = seed::open(120, 24, seed::config(), &shelf);
    s.wait_for("Buildkite");
    // Each marketplace tab shows its product label and not the raw marketplace label.
    let rows: Vec<String> = [
        ("Workspace.", "Buildkite", "Raw Workspace Directory."),
        ("Shared with me.", "Docs", "Raw Shared Private."),
        ("Shared with me (link).", "Link Share", "Raw Shared Link."),
        ("Local.", "Local Docs", "Personal."),
    ]
    .into_iter()
    .map(|(product_label, plugin_name, raw_label)| {
        select_tab(&mut s, product_label);
        s.assert_contains(product_label);
        s.assert_contains(plugin_name);
        s.assert_absent(raw_label);
        let row = s
            .rows()
            .into_iter()
            .find(|row| row.contains(plugin_name))
            .expect("the plugin's row");
        row.split_whitespace().collect::<Vec<_>>().join(" ")
    })
    .collect();
    kit::assert_rows_are(
        &rows,
        &[
            "› [-] Buildkite Available Press Enter to install or view plugin details",
            "› [-] Docs Available Press Enter to install or view plugin details",
            "› [-] Link Share Available Press Enter to install or view plugin details",
            "› [-] Local Docs Available Press Enter to install or view plugin details",
        ],
    );
}

// ORACLE chatwidget/tests/plugin_catalog_tests.rs:117 plugins_popup_preserves_workspace_tab_across_load_and_detail_navigation kind=ui owner=dialogs-pane
#[test]
fn plugins_popup_preserves_workspace_tab_across_load_and_detail_navigation() {
    let buildkite = seed::remote_summary(
        "plugins~Plugin_buildkite",
        "buildkite",
        Some("Buildkite"),
        Some("Buildkite pipelines."),
        false,
    );
    let shelf = seed::shelf(vec![seed::curated(Vec::new())]);
    seed::with(&shelf, |held| {
        held.held.insert(seed::SECTIONS);
        held.remote = vec![seed::remote_marketplace(
            "workspace-directory",
            "Raw Workspace Directory",
            vec![buildkite.clone()],
        )];
        held.details.insert(
            "*".to_string(),
            seed::remote_detail(
                "workspace-directory",
                buildkite,
                Some("Buildkite pipelines."),
            ),
        );
    });
    let mut s = seed::open(100, 24, seed::config(), &shelf);
    select_tab(&mut s, "Loading Workspace plugins.");
    s.assert_contains("Loading Workspace plugins.");
    // The sections arrive: the Workspace tab stays the active one.
    seed::serve_sections(&mut s, &shelf);
    s.assert_contains("Workspace.");
    s.assert_contains("Buildkite");
    s.assert_absent("Loading Workspace plugins.");
    // Detail, then Back to plugins: the Workspace tab is still the active one.
    s.keys("enter");
    s.wait_for("Back to plugins");
    s.keys("enter");
    s.assert_contains("Workspace.");
    s.assert_contains("Buildkite");
}

// ORACLE chatwidget/tests/plugin_catalog_tests.rs:176 plugins_popup_remote_local_dedupe_prefers_installed_remote_after_mapped_shares kind=ui owner=dialogs-pane
#[test]
fn plugins_popup_remote_local_dedupe_prefers_installed_remote_after_mapped_shares() {
    let remote_id = "plugins~Plugin_docs";
    let local = seed::set(
        plugin(
            "plugin-docs",
            "docs",
            "Docs",
            "Local curated docs plugin.",
            false,
        ),
        &[("remotePluginId", json!(remote_id))],
    );
    let shelf = seed::shelf(vec![seed::curated(vec![local])]);
    seed::with(&shelf, |held| {
        held.remote = vec![seed::remote_marketplace(
            "workspace-shared-with-me-private",
            "Shared with me",
            vec![seed::remote_summary(
                remote_id,
                "docs",
                Some("Docs"),
                Some("Remote installed docs plugin."),
                true,
            )],
        )];
    });
    let mut s = seed::open(100, 24, seed::config(), &shelf);
    // The remote copy is the installed one: it wins over the local row that is no mapped share.
    s.wait_for("Installed 1 of 1 available plugins.");
    s.assert_row_of("Docs", &["Installed"], &["Local curated docs plugin."]);
}

// ORACLE chatwidget/tests/plugin_catalog_tests.rs:244 plugin_detail_not_installable_plugin_disables_install_action kind=ui owner=dialogs-pane
#[test]
fn plugin_detail_not_installable_plugin_disables_install_action() {
    let internal = seed::summary(
        "plugin-internal",
        "internal",
        Some("Internal"),
        Some("Internal only."),
        false,
        true,
        "NOT_AVAILABLE",
    );
    let shelf = seed::shelf(vec![seed::curated(vec![internal.clone()])]);
    seed::with(&shelf, |held| {
        held.details.insert(
            "internal".to_string(),
            seed::detail(internal, Some("Internal only."), &[], &[], &[], &[]),
        );
    });
    let mut s = seed::open(100, 24, seed::config(), &shelf);
    s.keys("enter");
    s.assert_row_of(
        "Install plugin",
        &["This plugin is not installable from this marketplace"],
        &[],
    );
    let before = s.shot();
    // Down skips the disabled install row: the popup is drawn the same.
    s.keys("down");
    s.assert_same_rows(&before);
}
