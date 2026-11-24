//! State tests of S21 (the Agent command center), ported from the oracle
//! (`codex-rs/tui/src/app/agents_overview_tests.rs`, `agent_center_tests.rs`,
//! `agents_overview_actions_tests.rs`, codex-tui 0.170.0). The threads are the
//! protocol's `Thread`, built like the oracle's `overview_thread`; the view's
//! rows are `Model::center()`. Expected values are the oracle's assertions.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::ServerNotification;
use codex_app_server_protocol::Thread;
use codex_app_server_protocol::ThreadClosedNotification;
use codex_app_server_protocol::ThreadStartedNotification;
use codex_app_server_protocol::ThreadStatus;
use codex_app_server_protocol::ThreadStatusChangedNotification;
use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command as AppCommand, Intent, apply_app};
use codex_tui_core::fixtures::sandbox::Sandbox;
use codex_tui_core::ids::{ThreadId, Ticket};
use codex_tui_core::rpc::RpcError;
use codex_tui_core::screens::agents::{
    Action, CenterRow, CenterThreads, Command, Ctx, DetailLine, DirKeys, Ev, Failure, Group,
    GroupLabel, Grouping, Intent as AgentsIntent, Lens, Model, Notice, Out, Page, Reply, Sel,
    apply_answer, observe, update, wire,
};
use codex_tui_core::screens::{ScreenCommand, ScreenEv, ScreenIntent, ScreenReply};
use codex_tui_core::threads::{ThreadAsk, ThreadPreviews};
use pretty_assertions::assert_eq;
use serde_json::{Value, json};

const SESSION: &str = r#"{"step":"seed","seed":"session","thread":"thread-1"}"#;
const TIMESTAMP: i64 = 1_700_000_000;

fn id(n: u128) -> String {
    format!("00000000-0000-0000-0000-{n:012}")
}

/// The oracle's `overview_thread` (`agents_overview_tests.rs:474`): a root
/// named `name`, or a child of `parent` (the source says so), in `/tmp/project`.
fn thread(id: &str, parent: Option<&str>, name: &str, status: Value) -> Thread {
    let source = parent.map_or(json!("cli"), |parent| {
        json!({"subAgent": {"thread_spawn": {"parent_thread_id": parent, "depth": 1,
            "agent_path": null, "agent_nickname": null, "agent_role": null}}})
    });
    serde_json::from_value(json!({
        "id": id, "environments": null, "extra": null, "sessionId": parent.unwrap_or(id),
        "forkedFromId": null, "attachedToThreadId": null, "parentThreadId": null,
        "preview": name, "ephemeral": false, "section": null, "sectionEnteredAt": null,
        "projectId": null, "historyMode": "legacy", "modelProvider": "openai",
        "model": null, "reasoningEffort": null, "createdAt": TIMESTAMP,
        "updatedAt": TIMESTAMP, "recencyAt": TIMESTAMP, "status": status, "path": null,
        "cwd": "/tmp/project", "cliVersion": "0.0.0", "originator": null, "source": source,
        "canAcceptDirectInput": parent.is_none(), "threadSource": null,
        "agentNickname": null, "agentRole": null, "gitInfo": null, "name": name,
        "daybreakEnabled": null, "turns": []
    }))
    .expect("a thread")
}

fn idle() -> Value {
    json!({"type": "idle"})
}
fn working() -> Value {
    json!({"type": "active", "activeFlags": []})
}
fn waiting_approval() -> Value {
    json!({"type": "active", "activeFlags": ["waitingOnApproval"]})
}

fn sandbox() -> Sandbox {
    CURSOR.with(|cursor| *cursor.borrow_mut() = None);
    Sandbox::play(&format!("{SESSION}\n")).expect("plays")
}

thread_local! {
    /// The list widget's cursor, which the view owns (SelectionController):
    /// core keeps none, and its intents carry the id the cursor is on.
    static CURSOR: std::cell::RefCell<Option<Sel>> = const { std::cell::RefCell::new(None) };
}

fn move_cursor(sel: Sel) {
    CURSOR.with(|cursor| *cursor.borrow_mut() = Some(sel));
}

/// The highlight as the view resolves it: the cursor if its row shows, else
/// the default row.
fn at(m: &Model) -> Option<Sel> {
    at_with(m, &CenterThreads::default())
}

fn at_with(m: &Model, state: &CenterThreads) -> Option<Sel> {
    let cursor = CURSOR.with(|cursor| cursor.borrow().clone());
    let center = m.center_at(state, cursor.as_ref(), &Lens::default());
    if center.show_more_selected {
        Some(Sel::ShowMore)
    } else {
        center
            .selected_row()
            .map(|row| Sel::Task(row.id().to_owned()))
    }
}

fn pick(m: &Model) -> Option<String> {
    match at(m) {
        Some(Sel::Task(id)) => Some(id),
        _ => None,
    }
}

/// Core's answer to an intent that acts on the task under the cursor.
fn up_t(m: &mut Model, f: fn(String) -> AgentsIntent) -> Vec<Out> {
    let intent = f(pick(m).unwrap_or_default());
    update(m, intent)
}

fn up_o(m: &mut Model, f: fn(Option<String>) -> AgentsIntent) -> Vec<Out> {
    let intent = f(pick(m));
    update(m, intent)
}

/// What the view does on `h`: the cursor moves to the task after the hidden one.
fn refresh_usage_at(m: &mut Model) -> Option<Out> {
    let at = pick(m);
    codex_tui_core::screens::agents::refresh_usage(m, at)
}

fn hide_cursor(m: &Model) {
    if let Some(id) = pick(m)
        && let Some(next) =
            m.selection_after_removal(&id, std::slice::from_ref(&id), &Lens::default())
    {
        move_cursor(Sel::Task(next));
    }
}

fn list(sandbox: &mut Sandbox, threads: Vec<Thread>) {
    sandbox.send(AppEv::Screen(ScreenEv::Agents(Ev::Listed(Page {
        threads,
        unread: Vec::new(),
        next_cursor: None,
        append: false,
        pins: None,
    }))));
}

fn current(sandbox: &mut Sandbox, thread: &str) {
    sandbox.send(AppEv::Screen(ScreenEv::Agents(Ev::Current(Some(
        ThreadId::new(thread),
    )))));
}

fn intent(sandbox: &mut Sandbox, intent: AgentsIntent) {
    sandbox.send(AppEv::Intent(Intent::Screen(ScreenIntent::Agents(intent))));
}

fn model(sandbox: &Sandbox) -> &Model {
    &sandbox.app().screens().agents
}

/// The task ids in display order (the `Task` entries of the center; `rows` is
/// the unfiltered, unsorted list and is not what the view shows).
fn ids(sandbox: &Sandbox) -> Vec<String> {
    let center = model(sandbox).center(&sandbox.app().agents_threads());
    center
        .entries
        .iter()
        .filter_map(|entry| match entry {
            CenterRow::Task(index) => Some(center.rows[*index].id().to_owned()),
            _ => None,
        })
        .collect()
}

/// The lens of a view with its search line open.
fn searching() -> Lens {
    Lens {
        searching: true,
        ..Lens::default()
    }
}

/// The task ids in display order under a view's lens (tab, rename).
fn ids_in(sandbox: &Sandbox, lens: &Lens) -> Vec<String> {
    let center = model(sandbox).center_at(&sandbox.app().agents_threads(), None, lens);
    center
        .entries
        .iter()
        .filter_map(|entry| match entry {
            CenterRow::Task(index) => Some(center.rows[*index].id().to_owned()),
            _ => None,
        })
        .collect()
}

fn selected(sandbox: &Sandbox) -> String {
    match at_with(model(sandbox), &sandbox.app().agents_threads()) {
        Some(Sel::Task(id)) => id,
        other => panic!("expected a highlighted task, got {other:?}"),
    }
}

/// `shared_overview_shows_only_root_sessions` (agents_overview_tests.rs:1588):
/// children and ephemeral side threads are not rows; the first root, whose
/// child waits for approval, needs the user; counts per tab; the default
/// grouping is the project.
#[test]
fn shared_overview_shows_only_root_sessions() {
    let (first, child, second, unloaded, side) = (id(101), id(1), id(2), id(102), id(3));
    let mut side_thread = thread(&side, None, "Ephemeral side", idle());
    side_thread.ephemeral = true;
    let mut sandbox = sandbox();
    current(&mut sandbox, &first);
    list(
        &mut sandbox,
        vec![
            thread(&first, None, "Build the dashboard", idle()),
            thread(
                &child,
                Some(&first),
                "Inspect keyboard shortcuts",
                waiting_approval(),
            ),
            thread(&second, None, "Repair authentication", working()),
            thread(
                &unloaded,
                None,
                "Review yesterday's changes",
                json!({"type": "notLoaded"}),
            ),
            side_thread,
        ],
    );
    assert_eq!(model(&sandbox).grouping, Grouping::Project);
    let center = model(&sandbox).center(&sandbox.app().agents_threads());
    assert_eq!(center.tab_counts, [3, 1, 1, 0, 1]);
    let mut got = ids(&sandbox);
    got.sort();
    let mut want = [first.clone(), unloaded, second.clone()];
    want.sort();
    assert_eq!(got, want);
    assert_eq!(center.rows[0].id(), first);
    assert_eq!(center.rows[0].group, Group::NeedsYou);
    // Search for a task, accept: the highlighted task is that one.
    intent(
        &mut sandbox,
        AgentsIntent::SetSearch {
            text: "Repair authentication".into(),
            lens: searching(),
        },
    );
    assert_eq!(selected(&sandbox), second);
}

/// `overview_model_grouping_shows_details_and_preserves_selection`
/// (agents_overview_tests.rs:1531): `g` three times groups by model, the
/// selection stays, newer tasks come first inside a model, navigation
/// crosses the model groups.
#[test]
fn overview_model_grouping_shows_details_and_preserves_selection() {
    let mut sandbox = sandbox();
    let threads = [
        ("Older task", Some("model-a"), 1),
        ("Other model", Some("model-b"), 2),
        ("Recent task", Some("model-a"), 3),
        ("Legacy task", None, 4),
    ]
    .map(|(name, model, index)| {
        let mut thread = thread(&id(index), None, name, idle());
        thread.model = model.map(str::to_owned);
        thread.updated_at = TIMESTAMP + index as i64 - 30;
        thread
    });
    list(&mut sandbox, threads.to_vec());
    move_cursor(Sel::Task(id(1)));
    for _ in 0..3 {
        intent(&mut sandbox, AgentsIntent::CycleGrouping);
    }
    assert_eq!(model(&sandbox).grouping, Grouping::Model);
    assert_eq!(selected(&sandbox), id(1));
    for (forward, expected) in [(false, 3), (true, 1), (true, 2), (true, 4)] {
        // The list widget moves the cursor (wrapping is its own); core is told
        // which task it is on.
        let _ = forward;
        move_cursor(Sel::Task(id(expected)));
        assert_eq!(selected(&sandbox), id(expected));
    }
}

/// `live_center_sorts_arrivals_and_status_transitions_by_update_time`
/// (agent_center_tests.rs:151).
#[test]
fn live_center_sorts_arrivals_and_status_transitions_by_update_time() {
    let [alpha, bravo, charlie, delta, child_id] = [1, 2, 3, 4, 5].map(id);
    let mut threads = vec![
        thread(&alpha, None, "Alpha", working()),
        thread(&bravo, None, "Bravo", working()),
        thread(&charlie, None, "Charlie", idle()),
    ];
    let mut sandbox = sandbox();
    current(&mut sandbox, &bravo);
    list(&mut sandbox, threads.clone());
    assert_row_ids(&sandbox, &[&alpha, &bravo, &charlie]);
    let mut arrival = thread(&delta, None, "Delta", working());
    arrival.updated_at += 100;
    threads.push(arrival);
    let mut child = thread(&child_id, None, "Child", json!({"type": "systemError"}));
    child.parent_thread_id = Some(bravo.clone());
    threads.push(child);
    list(&mut sandbox, threads.clone());
    assert_row_ids(&sandbox, &[&delta, &alpha, &bravo, &charlie]);
    threads[1] = {
        let mut bravo_thread = threads[1].clone();
        bravo_thread.status = serde_json::from_value(idle()).unwrap();
        bravo_thread
    };
    list(&mut sandbox, threads.clone());
    assert_row_ids(&sandbox, &[&delta, &alpha, &bravo, &charlie]);
    let center = model(&sandbox).center(&sandbox.app().agents_threads());
    assert_eq!(center.selected_row().unwrap().group, Group::Ready);
    assert!(center.selected_row().unwrap().details.lines.is_empty());
    // A request from a descendant can still bring its idle parent to the top.
    threads.last_mut().unwrap().status = serde_json::from_value(waiting_approval()).unwrap();
    list(&mut sandbox, threads.clone());
    assert_row_ids(&sandbox, &[&bravo, &delta, &alpha, &charlie]);
    // The oracle feeds whole lists; a refresh only adds rows, so the child
    // leaves by its archive notification.
    archive(&mut sandbox, &child_id);
    assert_row_ids(&sandbox, &[&delta, &alpha, &bravo, &charlie]);
    assert_eq!(selected(&sandbox), bravo);
    // Rediscovery restores the task to its position by update time.
    archive(&mut sandbox, &alpha);
    assert_row_ids(&sandbox, &[&delta, &bravo, &charlie]);
    intent(&mut sandbox, AgentsIntent::Open);
    threads.pop();
    list(&mut sandbox, threads);
    assert_row_ids(&sandbox, &[&delta, &alpha, &bravo, &charlie]);
}

fn archive(sandbox: &mut Sandbox, thread: &str) {
    sandbox.send(AppEv::Backend(
        codex_tui_core::app::BackendEv::Notification(Box::new(ServerNotification::ThreadArchived(
            codex_app_server_protocol::ThreadArchivedNotification {
                thread_id: thread.to_owned(),
            },
        ))),
    ));
}

/// `external_removals_preserve_adjacent_selection`
/// (agents_overview_actions_tests.rs:447), the project grouping, unfiltered:
/// hiding the highlighted task moves the highlight to the next displayed one,
/// the last one to the one before.
#[test]
fn external_removals_preserve_adjacent_selection() {
    let mut sandbox = sandbox();
    let threads = (0..4)
        .map(|index| {
            let mut thread = thread(&id(index + 1), None, "Task", idle());
            thread.updated_at = index as i64;
            thread
        })
        .collect();
    list(&mut sandbox, threads);
    let order = ids(&sandbox);
    move_cursor(Sel::Task(order[1].clone()));
    let hidden = AgentsIntent::Hide(selected(&sandbox));
    hide_cursor(model(&sandbox));
    intent(&mut sandbox, hidden);
    assert_eq!(selected(&sandbox), order[2]);
    move_cursor(Sel::Task(order[3].clone()));
    let hidden = AgentsIntent::Hide(selected(&sandbox));
    hide_cursor(model(&sandbox));
    intent(&mut sandbox, hidden);
    assert_eq!(selected(&sandbox), order[2]);
}

/// The oracle's `thread_ids()`: the view's rows in their own order (status,
/// newest, id), not the display order of the grouping.
fn assert_row_ids(sandbox: &Sandbox, expected: &[&String]) {
    let got: Vec<String> = model(sandbox)
        .center(&sandbox.app().agents_threads())
        .rows
        .iter()
        .map(|row| row.id().to_owned())
        .collect();
    let expected: Vec<&str> = expected.iter().map(|id| id.as_str()).collect();
    assert_eq!(got.iter().map(String::as_str).collect::<Vec<_>>(), expected);
}

fn assert_ids(sandbox: &Sandbox, expected: &[&String]) {
    let expected: Vec<&str> = expected.iter().map(|id| id.as_str()).collect();
    let got = ids(sandbox);
    assert_eq!(got.iter().map(String::as_str).collect::<Vec<_>>(), expected);
}

fn request(sandbox: &mut Sandbox, thread: &str, preview: Option<&str>) {
    sandbox.send(AppEv::Screen(ScreenEv::Agents(Ev::Request(
        ThreadId::new(thread),
        preview.map(str::to_owned),
    ))));
}

/// The activity block of a root row, without its blank separators.
fn activity(sandbox: &Sandbox, thread: &str) -> Vec<DetailLine> {
    let center = model(sandbox).center(&sandbox.app().agents_threads());
    let row = center
        .rows
        .iter()
        .find(|row| row.id() == thread)
        .expect("a root row");
    row.details
        .lines
        .iter()
        .filter(|line| **line != DetailLine::Blank)
        .cloned()
        .collect()
}

/// `agents_overview_details_show_available_attention_without_expanding_rows`
/// (agents_overview_tests.rs:1014), the model side: the child's question is
/// previewed on the root row's activity block in the oracle's order; the
/// answer removes the question and keeps the approval wait.
#[test]
fn agents_overview_details_show_available_attention_without_expanding_rows() {
    let [root, failed_agent, child, unloaded] = [1, 2, 3, 4].map(id);
    let waiting =
        json!({"type": "active", "activeFlags": ["waitingOnApproval", "waitingOnUserInput"]});
    let mut sandbox = sandbox();
    current(&mut sandbox, &root);
    list(
        &mut sandbox,
        vec![
            thread(&root, None, "Repair authentication", waiting.clone()),
            thread(&child, Some(&failed_agent), "Check dependencies", waiting),
            thread(
                &unloaded,
                None,
                "Investigate parser",
                json!({"type": "notLoaded"}),
            ),
            thread(
                &failed_agent,
                Some(&root),
                "Failed agent",
                json!({"type": "systemError"}),
            ),
        ],
    );
    let question = "Which dependency version should I use?";
    request(&mut sandbox, &child, Some(question));
    assert_eq!(
        model(&sandbox)
            .center(&sandbox.app().agents_threads())
            .rows
            .len(),
        2
    );
    assert_eq!(selected(&sandbox), root);
    // The oracle's snapshot (agents_overview_attention) has no "Needs attention"
    // head here: the root's details come from its child's question (`!is_child`
    // is false for the child source). The `● Needs input` status is the row
    // header, not this block.
    assert_eq!(
        activity(&sandbox, &root),
        [
            DetailLine::Agent("Check dependencies".into()),
            DetailLine::Request(question.into()),
            DetailLine::OpenToReview,
            DetailLine::WaitingApproval,
            DetailLine::WaitingResponse,
        ]
    );
    // The answer removes the question; the approval wait stays (the oracle's
    // two assertions after the resolution).
    request(&mut sandbox, &child, None);
    let after = activity(&sandbox, &root);
    assert!(
        !after.contains(&DetailLine::Request(question.into())),
        "{after:?}"
    );
    assert!(after.contains(&DetailLine::WaitingApproval), "{after:?}");
}

/// `command_center_refresh_failure_is_inline_and_clears_on_success`
/// (agents_overview_tests.rs:2971): a failed list shows "Error loading tasks"
/// inline and keeps the search text; a successful list clears both the
/// failure and the notice. GAP (listed in the result): the oracle's
/// `recent_seed_complete: false` reply, which keeps the failure, has no core
/// counterpart (a page carries no completeness flag), so that step is absent.
#[test]
fn command_center_refresh_failure_is_inline_and_clears_on_success() {
    let mut sandbox = sandbox();
    intent(
        &mut sandbox,
        AgentsIntent::SetSearch {
            text: "Keep searching".into(),
            lens: searching(),
        },
    );
    let mut m = model(&sandbox).clone();
    let _ = apply_answer(
        &mut m,
        &mut ThreadPreviews::default(),
        Ctx::List { append: false },
        Err(RpcError::Remote("unavailable".into())),
    );
    assert!(m.paging.failed);
    assert_eq!(m.notice, Some(Notice::LoadFailed));
    assert_eq!(m.search, "Keep searching");
    let _ = apply_answer(
        &mut m,
        &mut ThreadPreviews::default(),
        Ctx::List { append: false },
        Ok(Reply::Listed(Page {
            threads: Vec::new(),
            unread: Vec::new(),
            next_cursor: None,
            append: false,
            pins: None,
        })),
    );
    assert!(!m.paging.failed);
    assert_eq!(m.notice, None);
    assert_eq!(m.search, "Keep searching");
}

/// `command_center_action_failures_remain_visible`
/// (agents_overview_tests.rs:3029), the notice side: each failed action
/// leaves its server message as the notice. The heading "Unable to complete
/// action" and the sentences around the message are the view's strings.
#[test]
fn command_center_action_failures_remain_visible() {
    let mut sandbox = sandbox();
    list(&mut sandbox, Vec::new());
    let thread = id(1);
    let mut m = model(&sandbox).clone();
    for (ctx, message) in [
        (
            Ctx::Resume {
                thread: ThreadId::new(thread.clone()),
            },
            "is unavailable",
        ),
        (
            Ctx::Rename {
                thread: thread.clone(),
            },
            "Failed to rename task",
        ),
        (
            Ctx::Action {
                thread,
                action: None,
            },
            "Failed to stop background task",
        ),
    ] {
        // Oracle: a failed resume reads `Failed to attach to task: {error}`.
        let shown = if matches!(ctx, Ctx::Resume { .. }) {
            format!("Failed to attach to task: {message}")
        } else {
            message.to_owned()
        };
        let _ = apply_answer(
            &mut m,
            &mut ThreadPreviews::default(),
            ctx,
            Err(RpcError::Remote(message.into())),
        );
        assert_eq!(m.notice, Some(Notice::Error(shown)));
    }
}

/// `command_center_new_actions_use_selection_and_leave_metadata_text_alone`
/// (agents_overview_tests.rs:3325), the model's steps: `p` pins the
/// highlighted task; `n` starts a task in its directory, and without a
/// selection in no directory; `g` cycles the grouping, Directory not
/// persisted; `r` renames, and the letters typed are the name's text.
/// The `w` step is the red test `command_center_new_worktree_uses_the_selected_directory`.
#[test]
fn command_center_new_actions_use_selection_and_leave_metadata_text_alone() {
    let mut sandbox = sandbox();
    let target = id(1);
    let mut task = thread(&target, None, "Task", idle());
    task.cwd = serde_json::from_value(json!("/tmp/checkout/subdir")).expect("a path");
    sandbox.send(AppEv::Screen(ScreenEv::Agents(Ev::Pinned(
        Some(Vec::new()),
    ))));
    list(&mut sandbox, vec![task]);
    move_cursor(Sel::Task(target.clone()));
    assert_eq!(selected(&sandbox), target);
    let mut m = model(&sandbox).clone();
    let outs = up_t(&mut m, AgentsIntent::TogglePin);
    assert_eq!(outs.len(), 1);
    assert_eq!(
        outs[0].command,
        Command::Pin {
            thread: target.clone(),
            pinned: true
        }
    );
    // The second `p` while the first is in flight sends nothing.
    assert!(up_t(&mut m, AgentsIntent::TogglePin).is_empty());
    let outs = up_o(&mut m, AgentsIntent::New);
    assert_eq!(outs.len(), 1);
    assert_eq!(
        outs[0].command,
        Command::Thread(ThreadAsk::Start {
            cwd: Some("/tmp/checkout/subdir".into())
        })
    );
    for expected in [Grouping::Directory, Grouping::Status, Grouping::Model] {
        let _ = update(&mut m, AgentsIntent::CycleGrouping);
        assert_eq!(m.grouping, expected);
    }
    assert_eq!(Grouping::Directory.persisted(), None);
    assert_eq!(Grouping::Status.persisted(), Some("status"));
    move_cursor(Sel::Task(target.clone()));
    // The letters typed are the editor's text (the view's); Enter hands it over.
    let outs = update(
        &mut m,
        AgentsIntent::Rename {
            thread: target.clone(),
            name: "Task nwogrxfhap".into(),
        },
    );
    assert_eq!(outs.len(), 1);
    match &outs[0].command {
        Command::Rename { thread, name } => {
            assert_eq!(thread, &target);
            assert!(name.ends_with("nwogrxfhap"), "{name}");
        }
        other => panic!("expected a rename, got {other:?}"),
    }
    // The oracle's second view is a fresh empty center (no selection).
    let empty_sandbox = Sandbox::play(&format!("{SESSION}\n")).expect("plays");
    let mut empty = model(&empty_sandbox).clone();
    let outs = up_o(&mut empty, AgentsIntent::New);
    assert_eq!(outs.len(), 1);
    assert_eq!(
        outs[0].command,
        Command::Thread(ThreadAsk::Start { cwd: None })
    );
}

/// The `w` step of `command_center_new_actions_use_selection_and_leave_metadata_text_alone`
/// (agents_overview_tests.rs:3325): a new worktree in the highlighted task's
/// directory. RED: no `Intent` for `w` exists (oracle
/// `AppEvent::NewAgentsOverviewWorktree { backend_id, cwd }`).
#[test]
fn command_center_new_worktree_uses_the_selected_directory() {
    let mut sandbox = sandbox();
    let target = id(1);
    let mut task = thread(&target, None, "Task", idle());
    task.cwd = serde_json::from_value(json!("/tmp/checkout/subdir")).expect("a path");
    list(&mut sandbox, vec![task]);
    let mut m = model(&sandbox).clone();
    // Worktrees disabled: `w` does nothing.
    assert!(up_o(&mut m, AgentsIntent::NewWorktree).is_empty());
    m.worktrees = true;
    let outs = up_o(&mut m, AgentsIntent::NewWorktree);
    assert_eq!(outs.len(), 1);
    assert_eq!(
        outs[0].command,
        Command::NewWorktree {
            cwd: Some("/tmp/checkout/subdir".into())
        }
    );
    assert_eq!(m.notice, Some(Notice::CreatingWorktree));
    // While the worktree is created, the center holds the other intents back.
    assert!(up_o(&mut m, AgentsIntent::New).is_empty());
    // Without a selection: no directory.
    let empty_sandbox = Sandbox::play(&format!("{SESSION}\n")).expect("plays");
    let mut empty = model(&empty_sandbox).clone();
    empty.worktrees = true;
    let outs = up_o(&mut empty, AgentsIntent::NewWorktree);
    assert_eq!(outs[0].command, Command::NewWorktree { cwd: None });
}

/// `ForkAgentsOverviewThread` (agents_overview_view.rs:808): the highlighted
/// task is forked and the copy opens in the chat.
#[test]
fn fork_opens_the_copy_of_the_selected_task() {
    let mut sandbox = sandbox();
    let target = id(1);
    list(&mut sandbox, vec![thread(&target, None, "Task", idle())]);
    let mut m = model(&sandbox).clone();
    let outs = up_t(&mut m, AgentsIntent::Fork);
    assert_eq!(
        outs.iter()
            .map(|out| out.command.clone())
            .collect::<Vec<_>>(),
        vec![Command::Thread(ThreadAsk::Fork {
            thread: target.clone()
        })]
    );
    let copy = thread(&id(9), None, "Task", idle());
    let _ = apply_answer(
        &mut m,
        &mut ThreadPreviews::default(),
        Ctx::Fork { thread: target },
        Ok(Reply::Forked(Box::new(copy))),
    );
    assert_eq!(m.resumes, 1);
    let mut empty = Sandbox::play(&format!("{SESSION}\n")).expect("plays");
    assert!(up_t(&mut empty_model(&mut empty), AgentsIntent::Fork).is_empty());
}

fn empty_model(sandbox: &mut Sandbox) -> Model {
    model(sandbox).clone()
}

/// `worktrees_overview_groups_by_project_or_directory`
/// (agents_overview_tests.rs:1380). RED: the core groups by the raw `cwd`
/// (center.rs:174-247). Linked worktrees of one repository are one project in
/// the oracle (four rows under the primary path), which needs the repository
/// root of each cwd: a shell fact, read from the `.git` file and `commondir`,
/// that the core does not read.
#[test]
fn worktrees_overview_groups_by_project_or_directory() {
    // The oracle builds a repository with a linked worktree on disk; the
    // shell reads that (host `dirs`, tested there) and hands the keys over.
    let root = "/tmp/worktree-root";
    let keys = |project: &str, inside: &str, heading: &str, directory: &str| DirKeys {
        project: (project.into(), inside.into()),
        heading: heading.into(),
        directory: directory.into(),
    };
    let common = format!("{root}/primary/.git");
    let nested_common = format!("{root}/primary/subdir/.git");
    let cwds = [
        ("Primary README", format!("{root}/primary")),
        ("Primary tree", format!("{root}/primary")),
        ("Linked README", format!("{root}/linked")),
        ("Linked tree", format!("{root}/linked")),
        ("Primary child", format!("{root}/primary/shared/child")),
        ("Linked child", format!("{root}/linked/shared/child")),
        ("Nested repository", format!("{root}/primary/subdir/child")),
    ];
    let threads: Vec<Thread> = cwds
        .iter()
        .enumerate()
        .map(|(index, (title, cwd))| {
            let mut task = thread(&id(index as u128 + 1), None, title, idle());
            task.cwd = serde_json::from_value(json!(cwd)).expect("a path");
            task
        })
        .collect();
    let mut sandbox = sandbox();
    sandbox.send(AppEv::Screen(ScreenEv::Agents(Ev::Backend {
        cloud: false,
        worktrees: true,
    })));
    sandbox.send(AppEv::Screen(ScreenEv::Agents(Ev::Directories(vec![
        (
            format!("{root}/primary"),
            keys(
                &common,
                "",
                &format!("{root}/primary"),
                &format!("{root}/primary"),
            ),
        ),
        (
            format!("{root}/linked"),
            keys(
                &common,
                "",
                &format!("{root}/primary"),
                &format!("{root}/linked"),
            ),
        ),
        (
            format!("{root}/primary/shared/child"),
            keys(
                &common,
                "shared/child",
                &format!("{root}/primary/shared/child"),
                &format!("{root}/primary/shared/child"),
            ),
        ),
        (
            format!("{root}/linked/shared/child"),
            keys(
                &common,
                "shared/child",
                &format!("{root}/primary/shared/child"),
                &format!("{root}/linked/shared/child"),
            ),
        ),
        (
            format!("{root}/primary/subdir/child"),
            keys(
                &nested_common,
                "child",
                &format!("{root}/primary/subdir/child"),
                &format!("{root}/primary/subdir/child"),
            ),
        ),
    ]))));
    list(&mut sandbox, threads);
    let headings = |sandbox: &Sandbox| -> Vec<(String, String)> {
        model(sandbox)
            .center(&sandbox.app().agents_threads())
            .entries
            .iter()
            .filter_map(|entry| match entry {
                CenterRow::Group {
                    label: GroupLabel::Path(path),
                    count,
                } => Some((path.clone(), count.clone())),
                _ => None,
            })
            .collect()
    };
    // Project: the linked worktree merges into its repository (4 rows under
    // the primary path), the nested repository is its own project.
    let mut project = headings(&sandbox);
    project.sort();
    assert_eq!(
        project,
        vec![
            (format!("{root}/primary"), "4".to_owned()),
            (format!("{root}/primary/shared/child"), "2".to_owned()),
            (format!("{root}/primary/subdir/child"), "1".to_owned()),
        ]
    );
    // Directory: one group per canonical directory, by the raw `cwd` heading.
    intent(&mut sandbox, AgentsIntent::SetGrouping(Grouping::Directory));
    let mut directory = headings(&sandbox);
    directory.sort();
    assert_eq!(
        directory,
        vec![
            (format!("{root}/linked"), "2".to_owned()),
            (format!("{root}/linked/shared/child"), "1".to_owned()),
            (format!("{root}/primary"), "2".to_owned()),
            (format!("{root}/primary/shared/child"), "1".to_owned()),
            (format!("{root}/primary/subdir/child"), "1".to_owned()),
        ]
    );
}

/// `overview_directory_group_keys_normalize_aliases`
/// (agents_overview_tests.rs:1513). RED: the key is the canonical directory
/// (a symlink and its target are one group), which needs IO. The core has no
/// `directory_group_key`; the shell supplies the canonical path.
#[test]
fn overview_directory_group_keys_normalize_aliases() {
    // `checkout` and its symlink `alias` share one canonical key when
    // worktrees are enabled; disabled, the alias is its own directory. The
    // canonical path is the shell's fact (host `dirs::directory_key`).
    let [a, b] = [1, 2].map(id);
    let mut checkout = thread(&a, None, "Checkout", idle());
    checkout.cwd = serde_json::from_value(json!("/tmp/root/checkout")).expect("a path");
    let mut alias = thread(&b, None, "Alias", idle());
    alias.cwd = serde_json::from_value(json!("/tmp/root/alias")).expect("a path");
    let key = |cwd: &str| {
        (
            cwd.to_owned(),
            DirKeys {
                project: (cwd.to_owned(), String::new()),
                heading: cwd.to_owned(),
                directory: "/tmp/root/checkout".to_owned(),
            },
        )
    };
    let groups = |worktrees: bool| {
        let mut sandbox = sandbox();
        sandbox.send(AppEv::Screen(ScreenEv::Agents(Ev::Backend {
            cloud: false,
            worktrees,
        })));
        sandbox.send(AppEv::Screen(ScreenEv::Agents(Ev::Directories(vec![
            key("/tmp/root/checkout"),
            key("/tmp/root/alias"),
        ]))));
        list(&mut sandbox, vec![checkout.clone(), alias.clone()]);
        intent(&mut sandbox, AgentsIntent::SetGrouping(Grouping::Directory));
        model(&sandbox)
            .center(&sandbox.app().agents_threads())
            .entries
            .iter()
            .filter(|entry| matches!(entry, CenterRow::Group { .. }))
            .count()
    };
    assert_eq!(groups(true), 1);
    assert_eq!(groups(false), 2);
}

/// `live_center_status_order_tracks_updates_while_filtered_and_pinned`
/// (agent_center_tests.rs:232): inside a status group the order follows the
/// update time, also for the rows a search or a tab hides; pins rank first.
#[test]
fn live_center_status_order_tracks_updates_while_filtered_and_pinned() {
    let [alpha, bravo, charlie] = [1, 2, 3].map(id);
    let mut threads = vec![
        thread(&alpha, None, "Alpha", working()),
        thread(&bravo, None, "Bravo", working()),
        thread(&charlie, None, "Charlie", working()),
    ];
    let mut sandbox = sandbox();
    current(&mut sandbox, &bravo);
    list(&mut sandbox, threads.clone());
    intent(
        &mut sandbox,
        AgentsIntent::SetSearch {
            text: "Bravo".into(),
            lens: searching(),
        },
    );
    assert_eq!(model(&sandbox).search, "Bravo");
    // A hidden row picks up its timestamp change when the filter clears.
    threads[2].updated_at += 100;
    list(&mut sandbox, threads.clone());
    assert_ids(&sandbox, &[&bravo]);
    intent(&mut sandbox, AgentsIntent::ClearSearch);
    assert_ids(&sandbox, &[&charlie, &alpha, &bravo]);
    // The `Needs you` tab is empty; the working task updates while it shows.
    let tab = |filter| Lens {
        filter,
        ..Lens::default()
    };
    assert!(ids_in(&sandbox, &tab(1)).is_empty());
    threads[1].updated_at += 200;
    list(&mut sandbox, threads.clone());
    assert_eq!(
        ids_in(&sandbox, &tab(2)),
        [bravo.clone(), charlie.clone(), alpha.clone()]
    );
    sandbox.send(AppEv::Screen(ScreenEv::Agents(Ev::Pinned(Some(vec![
        alpha.clone(),
        charlie.clone(),
    ])))));
    assert_ids(&sandbox, &[&alpha, &charlie, &bravo]);
    sandbox.send(AppEv::Screen(ScreenEv::Agents(Ev::Pinned(
        Some(Vec::new()),
    ))));
    assert_ids(&sandbox, &[&bravo, &charlie, &alpha]);
}

/// `failed_background_worker_only_needs_attention_for_a_pending_request`
/// (agent_center_tests.rs:121): a failed review worker is `Finished` with no
/// details; once its child waits for approval, the worker needs the user and
/// its details say so.
#[test]
fn failed_background_worker_only_needs_attention_for_a_pending_request() {
    let worker = id(1);
    let child = id(2);
    let mut review = thread(
        &worker,
        None,
        "Background review",
        json!({"type": "systemError"}),
    );
    review.source = serde_json::from_value(json!({"subAgent": "review"})).expect("a source");
    let mut sandbox = sandbox();
    current(&mut sandbox, &worker);
    list(&mut sandbox, vec![review.clone()]);
    let center = model(&sandbox).center(&sandbox.app().agents_threads());
    assert_eq!(center.rows[0].group, Group::Finished);
    assert!(center.rows[0].details.lines.is_empty());
    let waiting = json!({"type": "active", "activeFlags": ["waitingOnApproval"]});
    list(
        &mut sandbox,
        vec![
            review,
            thread(&child, Some(&worker), "Check dependencies", waiting),
        ],
    );
    let center = model(&sandbox).center(&sandbox.app().agents_threads());
    assert_eq!(center.rows[0].group, Group::NeedsYou);
    assert!(
        center.rows[0]
            .details
            .lines
            .contains(&DetailLine::WaitingApproval),
        "{:?}",
        center.rows[0].details.lines
    );
}

fn server_version(sandbox: &mut Sandbox, text: Option<&str>) {
    sandbox.send(AppEv::Screen(ScreenEv::Agents(Ev::ServerVersion(
        text.map(str::to_owned),
    ))));
}

/// `server_version_overview_notice_updates_and_clears`
/// (agents_overview_tests.rs:110): the notice is the server's words about its
/// version; a newer one replaces it and none clears it. The words are made by
/// the shell (oracle: `update_server_version_overview_notice`); the screen
/// holds them and paints them (the view's test is `older_server_notice_*`).
#[test]
fn server_version_overview_notice_updates_and_clears() {
    let older = "Service v0.152.1 < Codex CLI v0.153.0";
    let newer = "Service v0.151.0 < Codex CLI v0.153.0";
    let mut sandbox = sandbox();
    list(&mut sandbox, Vec::new());
    server_version(&mut sandbox, Some(older));
    server_version(&mut sandbox, Some(newer));
    assert_eq!(model(&sandbox).server_version.as_deref(), Some(newer));
    server_version(&mut sandbox, None);
    assert_eq!(model(&sandbox).server_version, None);
}

fn dispatch(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    app.assert_consistent();
    fx
}

fn listed_app(tasks: Vec<Thread>) -> CodexApp {
    let mut app = CodexApp::new();
    dispatch(
        &mut app,
        AppEv::Screen(ScreenEv::Agents(Ev::Listed(Page {
            threads: tasks,
            unread: vec![],
            next_cursor: None,
            append: false,
            pins: None,
        }))),
    );
    app
}

fn accept_task(app: &mut CodexApp, id: &str) -> AppFx {
    dispatch(app, AppEv::Intent(Intent::OpenThread(ThreadId::new(id))))
}

fn resumed(app: &mut CodexApp, ticket: Ticket, result: Result<Reply, RpcError>) -> AppFx {
    use codex_tui_core::rpc::Reply as RpcReply;
    let reply = match result {
        Ok(reply) => RpcReply::screen(ScreenReply::Agents(reply)),
        Err(error) => RpcReply::error(error),
    };
    dispatch(app, AppEv::Reply(ticket, reply))
}

fn resume_ticket(effects: &AppFx, id: &str) -> Ticket {
    match effects.commands() {
        [
            AppCommand::Screen {
                ticket,
                command: ScreenCommand::Agents(Command::Thread(ThreadAsk::Resume { thread })),
                ..
            },
        ] if thread == id => *ticket,
        other => panic!("expected a resume for {id}, got {other:?}"),
    }
}

/// Accept opens the highlighted task; a held attachment is reused.
#[test]
fn overview_right_opens_current_or_highlighted_task() {
    let [cur, other] = [1, 2].map(id);
    let mut app = listed_app(vec![
        thread(&cur, None, "Current", idle()),
        thread(&other, None, "Other", idle()),
    ]);
    dispatch(
        &mut app,
        AppEv::Screen(ScreenEv::Agents(Ev::Current(Some(ThreadId::new(&cur))))),
    );
    let effects = accept_task(&mut app, &cur);
    assert!(effects.commands().is_empty());
    assert_eq!(app.screens().agents.resumes, 1);
    let effects = accept_task(&mut app, &other);
    resume_ticket(&effects, &other);
}

/// The command center holds Accept and New while offline.
#[test]
fn overview_right_preserves_editors_and_offline_state() {
    let task = id(1);
    let mut app = listed_app(vec![thread(&task, None, "Task", idle())]);
    for notice in [Notice::Reconnecting, Notice::ReconnectFailed] {
        dispatch(
            &mut app,
            AppEv::Screen(ScreenEv::Agents(Ev::Notice(Some(notice)))),
        );
        assert!(accept_task(&mut app, &task).commands().is_empty());
        let effects = dispatch(
            &mut app,
            AppEv::Intent(Intent::Screen(ScreenIntent::Agents(AgentsIntent::New(
                Some(task.clone()),
            )))),
        );
        assert!(effects.commands().is_empty());
    }
    dispatch(&mut app, AppEv::Screen(ScreenEv::Agents(Ev::Notice(None))));
    let effects = accept_task(&mut app, &task);
    resume_ticket(&effects, &task);
}

/// `overview_grouping_persists_across_config_reloads`
/// (agents_overview_tests.rs:3243), the screen's part: `g` cycles the grouping
/// Project, Directory, Status, Model, and each one's configured value is what
/// the shell persists (`Directory` is view-local, never written). The config
/// file, the reload and the persistence event are the app's: n/a, owner
/// session-core.
#[test]
fn overview_grouping_persists_across_config_reloads() {
    let mut sandbox = sandbox();
    list(&mut sandbox, Vec::new());
    for (selected, persisted) in [
        (Grouping::Directory, None),
        (Grouping::Status, Some("status")),
        (Grouping::Model, Some("model")),
        (Grouping::Project, Some("project")),
    ] {
        intent(&mut sandbox, AgentsIntent::CycleGrouping);
        assert_eq!(model(&sandbox).grouping, selected);
        assert_eq!(selected.persisted(), persisted);
        if let Some(value) = persisted {
            assert_eq!(Grouping::from_config(value), Some(selected));
        }
    }
}

/// `live_center_keeps_status_order_across_progress_refreshes`
/// (agent_center_tests.rs:27), the order and the highlight: under the status
/// grouping the tasks that need the user come first, then the working ones by
/// update time (ties by id), then the rest; each progress refresh keeps the
/// order and the highlight on the current task. The snapshot is the view's.
#[test]
fn live_center_keeps_status_order_across_progress_refreshes() {
    let tasks = [1, 2, 3, 4, 5].map(id);
    let working_at = |updated_at: i64| (working(), updated_at);
    let rows = [
        ("Alpha", working_at(10)),
        ("Bravo", working_at(20)),
        ("Charlie", working_at(20)),
        ("Ready task", (idle(), 40)),
        ("Needs input", (json!({"type": "systemError"}), 0)),
    ];
    let mut threads: Vec<Thread> = rows
        .into_iter()
        .zip(tasks.iter())
        .map(|((name, (status, updated_at)), id)| {
            let mut thread = thread(id, None, name, status);
            // Future timestamps render as "now": the fixture is clock-independent.
            thread.updated_at = 4_000_000_000 + updated_at;
            thread
        })
        .collect();
    threads.reverse();
    let mut sandbox = sandbox();
    current(&mut sandbox, &tasks[0]);
    list(&mut sandbox, threads.clone());
    intent(&mut sandbox, AgentsIntent::CycleGrouping);
    intent(&mut sandbox, AgentsIntent::CycleGrouping);
    assert_eq!(model(&sandbox).grouping, Grouping::Status);
    assert_ids(
        &sandbox,
        &[&tasks[4], &tasks[1], &tasks[2], &tasks[0], &tasks[3]],
    );
    for (step, id) in [&tasks[0], &tasks[2], &tasks[1], &tasks[0]]
        .into_iter()
        .enumerate()
    {
        let thread = threads
            .iter_mut()
            .find(|thread| thread.id == *id)
            .expect("a listed task");
        thread.updated_at = 4_000_000_100 + step as i64;
        thread.preview = format!("Progress update {step}");
        threads.rotate_left(1);
        list(&mut sandbox, threads.clone());
        assert_eq!(ids(&sandbox)[1], *id, "step {step}");
        assert_eq!(selected(&sandbox), tasks[0], "step {step}");
    }
}

/// `live_center_rename_retains_target_when_status_leaves_filter`
/// (agent_center_tests.rs:488): a rename keeps its target when the task's
/// status moves it out of the filter; Enter commits the new name to that
/// target; Esc leaves the rename and the highlight stays on the other task.
/// The view's key handling (Esc, Enter) and the cursor shape are the view's.
#[test]
fn live_center_rename_retains_target_when_status_leaves_filter() {
    let [target, other] = [1, 2].map(id);
    let target_id = target.clone();
    let mut threads = vec![
        thread(&target, None, "Tar\nget", idle()),
        thread(&other, None, "Other", idle()),
    ];
    let mut sandbox = sandbox();
    list(&mut sandbox, threads.clone());
    // The third tab is the one the oracle's `Ready` filter stands for; the
    // view's lens names the task being renamed.
    let lens = Lens {
        filter: 3,
        renaming: Some(target.clone()),
        ..Lens::default()
    };
    move_cursor(Sel::Task(target.clone()));
    threads[0].status = serde_json::from_value(json!({"type": "systemError"})).expect("a status");
    list(&mut sandbox, threads.clone());
    let group = model(&sandbox)
        .center(&sandbox.app().agents_threads())
        .rows
        .iter()
        .find(|row| row.id() == target)
        .map(|row| row.group);
    assert_ne!(group, Some(Group::Ready), "the target left the Ready tab");
    assert!(
        ids_in(&sandbox, &lens).contains(&target),
        "the rename keeps the row visible"
    );
    assert!(
        !ids_in(
            &sandbox,
            &Lens {
                renaming: None,
                ..lens
            }
        )
        .contains(&target),
        "without the rename the row leaves the tab"
    );
    // The rename text is the field's; Enter hands it over.
    let mut m = model(&sandbox).clone();
    let outs: Vec<Command> = update(
        &mut m,
        AgentsIntent::Rename {
            thread: target,
            name: "Target!".into(),
        },
    )
    .into_iter()
    .map(|out| out.command)
    .collect();
    assert_eq!(
        outs,
        vec![Command::Rename {
            thread: target_id.clone(),
            name: "Target!".into(),
        }]
    );
    // Leaving the rename, the highlight moves to the task the tab shows.
    let tab = Lens {
        filter: 3,
        ..Lens::default()
    };
    let shown = model(&sandbox).center_at(
        &sandbox.app().agents_threads(),
        Some(&Sel::Task(target_id)),
        &tab,
    );
    assert_eq!(
        shown.selected_row().map(|row| row.id().to_owned()),
        Some(other)
    );
}

/// `backspace_edits_search_and_rename_without_deleting_tasks`
/// (agent_center_tests.rs:817): Backspace edits the search line and the
/// rename line, changes nothing else, and sends nothing.
#[test]
fn backspace_edits_search_and_rename_without_deleting_tasks() {
    let task = id(1);
    let mut sandbox = sandbox();
    list(&mut sandbox, vec![thread(&task, None, "Task", idle())]);
    // The search line is a text field; each change tells the model its text.
    intent(
        &mut sandbox,
        AgentsIntent::SetSearch {
            text: "!".into(),
            lens: searching(),
        },
    );
    let mut m = model(&sandbox).clone();
    assert!(
        update(
            &mut m,
            AgentsIntent::SetSearch {
                text: String::new(),
                lens: searching(),
            }
        )
        .is_empty()
    );
    assert_eq!(m.search, "");
    intent(
        &mut sandbox,
        AgentsIntent::SetSearch {
            text: String::new(),
            lens: searching(),
        },
    );
    assert_eq!(model(&sandbox).search, "");
    assert_ids(&sandbox, &[&task]);
    intent(&mut sandbox, AgentsIntent::ClearSearch);
    assert_ids(&sandbox, &[&task]);
}

/// `live_center_show_more_navigation_search_and_loading`
/// (agent_center_tests.rs:886): the Show more row is reachable, Enter asks for
/// the next page once, and a second Enter while it loads asks for nothing.
/// The history search (`SearchAgentsOverview`) has no command in the core yet:
/// it is the last assertion, a stub until agents-live adds it.
#[test]
fn live_center_show_more_navigation_search_and_loading() {
    let task = id(1);
    let mut sandbox = sandbox();
    sandbox.send(AppEv::Screen(ScreenEv::Agents(Ev::Listed(Page {
        threads: vec![thread(
            &task,
            None,
            "Recent task",
            json!({"type": "notLoaded"}),
        )],
        unread: Vec::new(),
        next_cursor: Some("cursor-2".into()),
        append: false,
        pins: None,
    }))));
    assert!(model(&sandbox).paging.has_more);
    move_cursor(Sel::ShowMore);
    let mut m = model(&sandbox).clone();
    let first: Vec<Command> = update(&mut m, AgentsIntent::ShowMore)
        .into_iter()
        .map(|out| out.command)
        .collect();
    assert_eq!(
        first,
        vec![Command::List {
            cursor: Some("cursor-2".into())
        }]
    );
    assert!(m.paging.loading);
    let second = update(&mut m, AgentsIntent::ShowMore);
    assert!(second.is_empty(), "no second page while one loads");
    // The page fails: the center stops loading (the oracle's `loading =
    // false`, `refresh_failed = true`); the cursor stays for a retry.
    let _ = apply_answer(
        &mut m,
        &mut ThreadPreviews::default(),
        Ctx::List { append: true },
        Err(RpcError::Remote("transient".into())),
    );
    assert!(!m.paging.loading && m.paging.failed);
    assert_eq!(m.paging.cursor.as_deref(), Some("cursor-2"));
    // `/` then the text "Older task": a history search starts at once and
    // asks for ten matches; the first page carries the debounce flag.
    let outs = update(
        &mut m,
        AgentsIntent::SetSearch {
            text: "Older task".into(),
            lens: searching(),
        },
    );
    assert_eq!(outs.len(), 1);
    let search = m.history.clone().expect("a history search");
    assert!(search.loading);
    assert_eq!(
        outs[0],
        Out {
            command: Command::Search {
                cursor: None,
                first: true
            },
            ctx: Ctx::Search {
                id: search.id,
                limit: 10
            }
        }
    );
    // Typing on starts a new search that keeps the wanted count.
    let outs = update(
        &mut m,
        AgentsIntent::SetSearch {
            text: "Older task 1".into(),
            lens: searching(),
        },
    );
    let newer = m.history.clone().expect("a history search");
    assert_ne!(newer.id, search.id);
    assert_eq!(newer.target, 10);
    assert_eq!(outs.len(), 1);
    // The answer of the old search is dropped.
    let stale = apply_answer(
        &mut m,
        &mut ThreadPreviews::default(),
        Ctx::Search {
            id: search.id,
            limit: 10,
        },
        Err(RpcError::Remote("late".into())),
    );
    assert!(stale.is_empty());
    assert!(m.history.as_ref().is_some_and(|held| held.loading));
    // The page fails: the search can be retried with "Show more matches".
    let _ = apply_answer(
        &mut m,
        &mut ThreadPreviews::default(),
        Ctx::Search {
            id: newer.id,
            limit: 10,
        },
        Err(RpcError::Remote("down".into())),
    );
    let failed = m.history.clone().expect("a history search");
    assert!(failed.failed && !failed.loading);
    // Enter on "Show more" while the search line is open.
    let retry = update(&mut m, AgentsIntent::MoreMatches(searching()));
    assert_eq!(retry.len(), 1);
    // A page without a match scans on at once; one with a match shows it
    // and keeps the rest of the scan for "Show more matches".
    let again = apply_answer(
        &mut m,
        &mut ThreadPreviews::default(),
        retry[0].ctx.clone(),
        Ok(Reply::Listed(Page {
            threads: vec![thread(&id(5), None, "Nothing here", idle())],
            unread: Vec::new(),
            next_cursor: Some("scan-2".into()),
            append: false,
            pins: None,
        })),
    );
    assert_eq!(
        again
            .iter()
            .map(|out| out.command.clone())
            .collect::<Vec<_>>(),
        vec![Command::Search {
            cursor: Some("scan-2".into()),
            first: false
        }]
    );
    let done = apply_answer(
        &mut m,
        &mut ThreadPreviews::default(),
        again[0].ctx.clone(),
        Ok(Reply::Listed(Page {
            threads: vec![thread(&id(6), None, "Older task 1 of many", idle())],
            unread: Vec::new(),
            next_cursor: Some("scan-3".into()),
            append: false,
            pins: None,
        })),
    );
    assert!(done.is_empty());
    assert_eq!(m.search_matches.len(), 1);
    assert!(
        m.history
            .as_ref()
            .is_some_and(|held| held.has_more() && !held.loading)
    );
    assert!(ids_of(&m).contains(&id(6)));
    // Leaving the search drops its rows.
    let _ = update(&mut m, AgentsIntent::ClearSearch);
    assert!(m.history.is_none() && m.search_matches.is_empty());
}

/// `hiding_rename_target_does_not_transfer_draft_to_neighbor`
/// (agents_overview_actions_tests.rs:679): the view drops the draft of a hidden
/// task; core sends no rename for a hidden task.
#[test]
fn hiding_rename_target_does_not_transfer_draft_to_neighbor() {
    let [target, neighbor] = [1, 2].map(id);
    let mut sandbox = sandbox();
    list(
        &mut sandbox,
        vec![
            thread(&target, None, "Rename target", idle()),
            thread(&neighbor, None, "Neighbor", idle()),
        ],
    );
    move_cursor(Sel::Task(target.clone()));
    let hidden = AgentsIntent::Hide(selected(&sandbox));
    hide_cursor(model(&sandbox));
    intent(&mut sandbox, hidden);
    let mut m = model(&sandbox).clone();
    let outs = update(
        &mut m,
        AgentsIntent::Rename {
            thread: target,
            name: "Draft".into(),
        },
    );
    assert!(
        outs.iter()
            .all(|out| !matches!(out.command, Command::Rename { .. })),
        "{outs:?}"
    );
}

/// Applies `outs` the way the app does: each RPC answered with `Done`.
fn answer_done(model: &mut Model, outs: Vec<Out>) {
    for out in outs {
        let _ = apply_answer(
            model,
            &mut ThreadPreviews::default(),
            out.ctx,
            Ok(Reply::Done),
        );
    }
}

fn ids_of(model: &Model) -> Vec<String> {
    let center = model.center(&CenterThreads::default());
    center
        .entries
        .iter()
        .filter_map(|entry| match entry {
            CenterRow::Task(index) => Some(center.rows[*index].id().to_owned()),
            _ => None,
        })
        .collect()
}

/// `pin_action_updates_shared_section_state` (agents_overview_actions_tests.rs:17),
/// the core half: each toggle asks for the opposite of the row's pin, and the
/// answer moves the task in or out of the shared pinned section. The app server
/// round trip (`list_pinned_threads`) and the refresh notifications are the app's.
#[test]
fn pin_action_updates_shared_section_state() {
    let mut sandbox = sandbox();
    let target = id(1);
    sandbox.send(AppEv::Screen(ScreenEv::Agents(Ev::Pinned(
        Some(Vec::new()),
    ))));
    list(&mut sandbox, vec![thread(&target, None, "Task", idle())]);
    // A single task is highlighted until one is selected.
    let mut m = model(&sandbox).clone();
    for pinned in [true, false] {
        let outs = up_t(&mut m, AgentsIntent::TogglePin);
        assert_eq!(outs.len(), 1);
        assert_eq!(
            outs[0].command,
            Command::Pin {
                thread: target.clone(),
                pinned
            }
        );
        answer_done(&mut m, outs);
        assert_eq!(m.pinned_ids().any(|pin| pin == target), pinned);
    }
}

/// VS1 review D1: the pin reply sets the requested state. A pin list that
/// already holds the task (arrived before the reply) is not inverted, and a new
/// pin goes last (oracle `complete_agents_overview_pin`).
#[test]
fn pin_reply_sets_state_and_new_pins_go_last() {
    let mut sandbox = sandbox();
    let (a, x) = (id(1), id(2));
    sandbox.send(AppEv::Screen(ScreenEv::Agents(Ev::Pinned(Some(vec![
        a.clone(),
    ])))));
    list(
        &mut sandbox,
        vec![thread(&a, None, "A", idle()), thread(&x, None, "X", idle())],
    );
    let mut m = model(&sandbox).clone();
    move_cursor(Sel::Task(x.clone()));
    let outs = up_t(&mut m, AgentsIntent::TogglePin);
    assert_eq!(outs.len(), 1);
    // The list with X already pinned arrives before the reply.
    m.set_pins(m.displayed, Some(vec![a.clone(), x.clone()]));
    answer_done(&mut m, outs);
    assert_eq!(m.pinned_ids().collect::<Vec<_>>(), [a.as_str(), x.as_str()]);
    // Without that early list: appended, not prepended.
    m.set_pins(m.displayed, Some(vec![a.clone()]));
    move_cursor(Sel::Task(x.clone()));
    let outs = up_t(&mut m, AgentsIntent::TogglePin);
    answer_done(&mut m, outs);
    assert_eq!(m.pinned_ids().collect::<Vec<_>>(), [a.as_str(), x.as_str()]);
}

/// Oracle `complete_agents_overview_pin`: whatever the answer, the list is read
/// again; a refusal says `Failed to pin task: ` / `Failed to unpin task: `.
#[test]
fn pin_answers_refresh_the_list_and_a_refusal_has_the_oracles_prefix() {
    let mut sandbox = sandbox();
    let target = id(1);
    sandbox.send(AppEv::Screen(ScreenEv::Agents(Ev::Pinned(
        Some(Vec::new()),
    ))));
    list(&mut sandbox, vec![thread(&target, None, "Task", idle())]);
    let mut m = model(&sandbox).clone();
    let outs = up_t(&mut m, AgentsIntent::TogglePin);
    let after = apply_answer(
        &mut m,
        &mut ThreadPreviews::default(),
        outs[0].ctx.clone(),
        Ok(Reply::Done),
    );
    assert_eq!(
        after
            .iter()
            .map(|out| out.command.clone())
            .collect::<Vec<_>>(),
        vec![Command::List { cursor: None }]
    );
    for (pinned, word) in [(true, "pin"), (false, "unpin")] {
        let ctx = Ctx::Pin {
            thread: target.clone(),
            pinned,
        };
        let after = apply_answer(
            &mut m,
            &mut ThreadPreviews::default(),
            ctx,
            Err(RpcError::Remote("denied".into())),
        );
        assert_eq!(
            m.notice,
            Some(Notice::Error(format!("Failed to {word} task: denied")))
        );
        assert_eq!(after.len(), 1);
        assert!(!m.pin_pending);
    }
}

/// The core half of `lifecycle_shortcuts_target_filtered_task_in_any_state`
/// (agents_overview_actions_tests.rs:223): the archive, delete and hide intents
/// target the highlighted task whatever its status. The filter (search `Target`)
/// and the F-key bindings are screen-level, and so is `Fork` (no intent yet).
#[test]
fn lifecycle_shortcuts_target_filtered_task_in_any_state() {
    for status in [idle(), json!({"type": "notLoaded"}), working()] {
        let target = id(1);
        let mut sandbox = sandbox();
        list(&mut sandbox, vec![thread(&target, None, "Target", status)]);
        let mut m = model(&sandbox).clone();
        let archived = up_t(&mut m.clone(), AgentsIntent::Archive);
        assert!(matches!(
            &archived[0].command,
            Command::Archive { thread } if thread == &target
        ));
        let deleted = up_t(&mut m.clone(), AgentsIntent::Delete);
        assert!(matches!(
            &deleted[0].command,
            Command::Delete { thread } if thread == &target
        ));
        let hidden = AgentsIntent::Hide(pick(&m).unwrap_or_default());
        hide_cursor(&m);
        let _ = update(&mut m, hidden);
        assert!(m.hidden.contains(&target));
        assert!(ids_of(&m).is_empty());
    }
}

/// The core half of `hidden_task_stays_hidden_through_activity_and_seed_until_explicit_resume`
/// (agents_overview_actions_tests.rs:734): a hidden task stays hidden through
/// its notifications and a list refresh that returns it, and an explicit
/// resume of it shows it again. RED: `Resume` does not unhide (the oracle's
/// `resume_target_session` shows the task; update.rs `apply_answer`).
#[test]
fn hidden_task_stays_hidden_through_activity_and_seed_until_explicit_resume() {
    let target = id(1);
    let task = thread(&target, None, "Hidden task", idle());
    let mut sandbox = sandbox();
    list(&mut sandbox, vec![task.clone()]);
    let mut m = model(&sandbox).clone();
    let hidden = AgentsIntent::Hide(pick(&m).unwrap_or_default());
    hide_cursor(&m);
    let _ = update(&mut m, hidden);
    assert!(ids_of(&m).is_empty());
    let started = ServerNotification::ThreadStarted(ThreadStartedNotification {
        thread: task.clone(),
    });
    observe(&mut m, &started);
    let status = ServerNotification::ThreadStatusChanged(ThreadStatusChangedNotification {
        thread_id: target.clone(),
        status: serde_json::from_value::<ThreadStatus>(working()).expect("a status"),
    });
    observe(&mut m, &status);
    let closed = ServerNotification::ThreadClosed(ThreadClosedNotification {
        thread_id: target.clone(),
    });
    observe(&mut m, &closed);
    let _ = apply_answer(
        &mut m,
        &mut ThreadPreviews::default(),
        Ctx::List { append: false },
        Ok(Reply::Listed(Page {
            threads: vec![task.clone()],
            unread: Vec::new(),
            next_cursor: None,
            append: false,
            pins: None,
        })),
    );
    assert!(ids_of(&m).is_empty());
    assert!(m.hidden.contains(&target));
    // The explicit resume (the app's `resume_target_session`): its answer.
    let _ = apply_answer(
        &mut m,
        &mut ThreadPreviews::default(),
        Ctx::Resume {
            thread: ThreadId::new(target.clone()),
        },
        Ok(Reply::Resumed(Box::new(task))),
    );
    assert_eq!(ids_of(&m), vec![target]);
}

/// The core half of `rejected_delete_preserves_a_live_attachment_and_draft`
/// (agents_overview_actions_tests.rs:819): a rejected delete keeps the task
/// and reports the error. The chat draft and the primary are the app's.
#[test]
fn rejected_delete_preserves_a_live_attachment() {
    let target = id(1);
    let mut sandbox = sandbox();
    list(&mut sandbox, vec![thread(&target, None, "Task", idle())]);
    let mut m = model(&sandbox).clone();
    let outs = up_t(&mut m, AgentsIntent::Delete);
    assert_eq!(outs.len(), 1);
    assert_eq!(
        outs[0].command,
        Command::Delete {
            thread: target.clone()
        }
    );
    let _ = apply_answer(
        &mut m,
        &mut ThreadPreviews::default(),
        outs[0].ctx.clone(),
        Err(RpcError::Remote("Could not delete task".into())),
    );
    assert_eq!(ids_of(&m), vec![target]);
    // The oracle shows the refusal as the `Could not delete task` dialog
    // (agents_overview_actions_tests.rs:845), not a footer notice.
    assert_eq!(m.notice, None);
    assert_eq!(
        m.failure,
        Some(Failure {
            action: Action::Delete,
            error: "Could not delete task".into(),
            attempted: true,
        })
    );
}

/// The core half of `lifecycle_removes_background_and_current_tasks_without_losing_the_dashboard`
/// (agents_overview_actions_tests.rs:851): removing a background task removes
/// its whole subtree (child and grandchild) from the list. RED: `remove`
/// drops only direct children (update.rs `apply_answer`, one level).
#[test]
fn lifecycle_removes_background_and_current_tasks() {
    for archive in [true, false] {
        let background = id(1);
        let child = id(2);
        let grandchild = id(3);
        let mut sandbox = sandbox();
        list(
            &mut sandbox,
            vec![
                thread(
                    &background,
                    None,
                    "Background task",
                    json!({"type": "notLoaded"}),
                ),
                thread(
                    &child,
                    Some(&background),
                    "Child",
                    json!({"type": "notLoaded"}),
                ),
                thread(
                    &grandchild,
                    Some(&child),
                    "Grandchild",
                    json!({"type": "notLoaded"}),
                ),
            ],
        );
        let mut m = model(&sandbox).clone();
        let outs = up_t(
            &mut m,
            if archive {
                AgentsIntent::Archive
            } else {
                AgentsIntent::Delete
            },
        );
        assert_eq!(outs.len(), 1);
        answer_done(&mut m, outs);
        assert!(
            m.threads.is_empty(),
            "archive={archive}: {:?}",
            m.threads.len()
        );
    }
}

/// The core half of `overview_cloud_discovery_uses_only_interactive_pages`
/// (agents_overview_discovery_tests.rs:21): the list asks for `sourceKinds: []`
/// with the cursor of the page, and a page's `nextCursor` says whether more
/// pages follow.
#[test]
fn overview_cloud_discovery_uses_only_interactive_pages() {
    let mut m = Model::default();
    let outs = update(&mut m, AgentsIntent::Open);
    assert_eq!(outs.len(), 1);
    assert_eq!(outs[0].command, Command::List { cursor: None });
    let thread_id = ThreadId::new("00000000-0000-0000-0000-000000000000");
    let ClientRequest::ThreadList { params, .. } =
        wire(&outs[0].command, &thread_id, RequestId::Integer(1))
    else {
        panic!("expected thread/list");
    };
    assert_eq!(params.source_kinds, Some(Vec::new()));
    assert_eq!(params.cursor, None);
    let first = thread(
        &id(1),
        None,
        "First Cloud task",
        json!({"type": "notLoaded"}),
    );
    let _ = apply_answer(
        &mut m,
        &mut ThreadPreviews::default(),
        Ctx::List { append: false },
        Ok(Reply::Listed(Page {
            threads: vec![first],
            unread: Vec::new(),
            next_cursor: Some("next".into()),
            append: false,
            pins: None,
        })),
    );
    assert!(m.paging.has_more);
    assert_eq!(m.paging.cursor.as_deref(), Some("next"));
    let second = thread(
        &id(2),
        None,
        "Second Cloud task",
        json!({"type": "notLoaded"}),
    );
    let _ = apply_answer(
        &mut m,
        &mut ThreadPreviews::default(),
        Ctx::List { append: true },
        Ok(Reply::Listed(Page {
            threads: vec![second],
            unread: Vec::new(),
            next_cursor: None,
            append: true,
            pins: None,
        })),
    );
    assert!(!m.paging.has_more);
    assert_eq!(ids_of(&m).len(), 2);
}

/// `cloud_archive_preserves_attached_child_and_other_tasks`
/// (agents_overview_actions_tests.rs:71): a cloud archive removes only the
/// selected task; a local one removes its subtree. The core has no backend
/// mode yet (the oracle's `ThreadParamsMode::Cloud`, agents_overview_actions.rs:296).
#[test]
fn cloud_archive_preserves_attached_child_and_other_tasks() {
    let [parent, child, other] = [1, 2, 3].map(id);
    let threads = vec![
        thread(&parent, None, "Cloud parent", idle()),
        thread(&child, Some(&parent), "Cloud child", working()),
        thread(&other, None, "Other task", idle()),
    ];
    let archive = |cloud: bool| {
        let mut sandbox = sandbox();
        sandbox.send(AppEv::Screen(ScreenEv::Agents(Ev::Backend {
            cloud,
            worktrees: false,
        })));
        list(&mut sandbox, threads.clone());
        let mut m = model(&sandbox).clone();
        move_cursor(Sel::Task(parent.clone()));
        let outs = up_t(&mut m, AgentsIntent::Archive);
        let _ = apply_answer(
            &mut m,
            &mut ThreadPreviews::default(),
            outs[0].ctx.clone(),
            Ok(Reply::Done),
        );
        m.threads
            .iter()
            .map(|held| held.id.clone())
            .collect::<Vec<_>>()
    };
    // The cloud archives only the selected task: its child and the other task
    // stay. Locally the child goes with its parent.
    assert_eq!(archive(true), vec![child, other.clone()]);
    assert_eq!(archive(false), vec![other]);
}

/// `overview_show_more_refills_archived_rows_and_retries_without_losing_rows`
/// (agents_overview_discovery_tests.rs:65): archived rows are refilled from the
/// next pages, and a failed page retries its cursor. The refill count and the
/// retry are not in the core's paging yet (oracle agents_overview_discovery.rs).
#[test]
fn overview_show_more_refills_archived_rows() {
    // 24 roots, newest first, listed in pages of ten.
    let tasks: Vec<Thread> = (0..24)
        .map(|index| {
            let mut task = thread(
                &id(index + 1),
                None,
                &format!("Task {index}"),
                json!({"type": "notLoaded"}),
            );
            task.updated_at += 24 - index as i64;
            task
        })
        .collect();
    let page = |from: usize, next: Option<&str>| {
        Ok(Reply::Listed(Page {
            threads: tasks[from..(from + 10).min(24)].to_vec(),
            unread: Vec::new(),
            next_cursor: next.map(str::to_owned),
            append: false,
            pins: None,
        }))
    };
    let mut m = Model::default();
    let open = update(&mut m, AgentsIntent::Open);
    assert_eq!(open[0].command, Command::List { cursor: None });
    let _ = apply_answer(
        &mut m,
        &mut ThreadPreviews::default(),
        open[0].ctx.clone(),
        page(0, Some("10")),
    );
    assert_eq!(m.threads.len(), 10);
    assert!(m.paging.has_more);
    // Show more: the next page fails (transient); nothing is lost and the
    // same cursor is asked again.
    move_cursor(Sel::ShowMore);
    let more = update(&mut m, AgentsIntent::ShowMore);
    assert_eq!(
        more[0].command,
        Command::List {
            cursor: Some("10".into())
        }
    );
    let _ = apply_answer(
        &mut m,
        &mut ThreadPreviews::default(),
        more[0].ctx.clone(),
        Err(RpcError::Remote("transient".into())),
    );
    assert_eq!(m.threads.len(), 10);
    assert!(m.paging.failed && m.paging.has_more);
    let retry = update(&mut m, AgentsIntent::ShowMore);
    assert_eq!(
        retry[0].command,
        Command::List {
            cursor: Some("10".into())
        }
    );
    let _ = apply_answer(
        &mut m,
        &mut ThreadPreviews::default(),
        retry[0].ctx.clone(),
        page(10, Some("20")),
    );
    assert_eq!(m.threads.len(), 20);
    assert!(!m.paging.failed);
    // A task is archived elsewhere: its vacancy is refilled from the next
    // page without growing the window, and a duplicate removal (the local
    // completion after the notification) refills nothing more.
    let gone = tasks[0].id.clone();
    let archived =
        ServerNotification::ThreadArchived(codex_app_server_protocol::ThreadArchivedNotification {
            thread_id: gone.clone(),
        });
    observe(&mut m, &archived);
    observe(&mut m, &archived);
    assert_eq!(m.paging.refill, 1);
    assert_eq!(m.threads.len(), 19);
    let refill = codex_tui_core::screens::agents::refill(&mut m);
    assert_eq!(
        refill[0].command,
        Command::List {
            cursor: Some("20".into())
        }
    );
    let _ = apply_answer(
        &mut m,
        &mut ThreadPreviews::default(),
        refill[0].ctx.clone(),
        page(20, None),
    );
    // One row came from the fetched page; the other three wait unshown.
    assert_eq!(m.threads.len(), 20);
    assert_eq!(m.paging.refill, 0);
    assert!(
        m.threads.iter().all(|held| held.id != gone),
        "a removed task never returns"
    );
    assert!(m.paging.has_more, "three fetched rows are still unshown");
    assert_eq!(m.paging.buffer.len(), 3);
    // Show more shows them from the buffer, without a request, and ends the list.
    move_cursor(Sel::ShowMore);
    let last = update(&mut m, AgentsIntent::ShowMore);
    assert!(last.is_empty());
    assert_eq!(m.threads.len(), 23);
    assert!(!m.paging.has_more);
    // Reopening sends the first page again; a removed task is not brought
    // back by a refresh, the window's cursor stays where it was.
    let refresh = update(&mut m, AgentsIntent::Refresh);
    let _ = apply_answer(
        &mut m,
        &mut ThreadPreviews::default(),
        refresh[0].ctx.clone(),
        page(0, Some("10")),
    );
    assert_eq!(m.threads.len(), 23);
    assert!(!m.paging.has_more);
}

/// `selected_usage_is_cached_and_account_changes_discard_old_results`
/// (agents_overview_usage_tests.rs:14): the core's `Usage` keeps tokens only;
/// the oracle caches the usage estimate (credits, USD micros, groups) per
/// selected task (agents_overview_usage.rs).
#[test]
fn selected_usage_is_cached() {
    use codex_app_server_protocol::ThreadUsage;
    use codex_protocol::account::PlanType;
    use codex_tui_core::screens::agents::{UsageOutcome, usage_wait};
    let [selected, other] = [1, 2].map(id);
    let mut sandbox = sandbox();
    list(
        &mut sandbox,
        vec![
            thread(&selected, None, "Review parser", idle()),
            thread(&other, None, "Other task", idle()),
        ],
    );
    sandbox.send(AppEv::Screen(ScreenEv::Agents(Ev::Account {
        plan: Some(PlanType::Business),
        backend_auth: true,
    })));
    sandbox.send(AppEv::Screen(ScreenEv::Agents(Ev::Clock(1_000))));
    let mut m = model(&sandbox).clone();
    move_cursor(Sel::Task(selected.clone()));
    let estimate = |credits: i64, cost: i64, groups: bool| -> ThreadUsage {
        serde_json::from_value(json!({
            "threadId": selected, "estimatedUsageCreditsMicros": credits,
            "estimatedUsageUsdMicros": cost,
            "groups": if groups { json!([{"estimatedUsageCreditsMicros": 3_400_000, "inputTokens": 12_000, "outputTokens": 3_000}]) } else { json!([]) }
        }))
        .unwrap()
    };
    let full = estimate(3_400_000, 140_000, true);
    // A request is pending: no second one.
    m.pending_usage = Some((selected.clone(), 99));
    assert!(refresh_usage_at(&mut m).is_none());
    let finish = |m: &mut Model, outcome: UsageOutcome| {
        let ctx = Ctx::Usage {
            thread: selected.clone(),
            id: m.pending_usage.as_ref().unwrap().1,
        };
        let _ = apply_answer(
            m,
            &mut ThreadPreviews::default(),
            ctx,
            Ok(Reply::Usage(outcome)),
        );
    };
    finish(&mut m, UsageOutcome::Available(full.clone()));
    assert_eq!(m.pending_usage, None);
    // Cached for a minute: the next refresh asks nothing and says when it is due.
    assert!(refresh_usage_at(&mut m).is_none());
    assert_eq!(usage_wait(&m, &pick(&m).unwrap()), Some(60));
    m.now += 61;
    let due = refresh_usage_at(&mut m).expect("stale: asked again");
    assert_eq!(
        due.command,
        Command::Usage {
            thread: selected.clone()
        }
    );
    // A settling estimate that reads zero keeps the known credits and cost.
    let mut expected = full.clone();
    for (credits, cost, kept_credits) in [(0, 200_000, 3_400_000), (5_000_000, 0, 5_000_000)] {
        expected.estimated_usage_credits_micros = kept_credits;
        expected.estimated_usage_usd_micros = Some(200_000);
        m.pending_usage = Some((selected.clone(), 7));
        let _ = apply_answer(
            &mut m,
            &mut ThreadPreviews::default(),
            Ctx::Usage {
                thread: selected.clone(),
                id: 7,
            },
            Ok(Reply::Usage(UsageOutcome::Available(estimate(
                credits, cost, false,
            )))),
        );
        assert_eq!(m.usage[&selected].estimate, Some(expected.clone()));
    }
    // Tokens come with the notifications and survive; closing drops them only.
    m.usage.get_mut(&selected).unwrap().tokens = Some((13_000, 4_000));
    observe(
        &mut m,
        &ServerNotification::ThreadClosed(ThreadClosedNotification {
            thread_id: selected.clone(),
        }),
    );
    assert_eq!(m.usage[&selected].tokens, None);
    assert_eq!(m.usage[&selected].estimate, Some(expected));
    // A reply for a task that is not the pending one is dropped; an account
    // without thread usage discards every estimate and is not asked again.
    m.pending_usage = Some((selected.clone(), 8));
    let _ = apply_answer(
        &mut m,
        &mut ThreadPreviews::default(),
        Ctx::Usage {
            thread: selected.clone(),
            id: 9,
        },
        Ok(Reply::Usage(UsageOutcome::Disabled)),
    );
    assert!(!m.usage_disabled);
    let _ = apply_answer(
        &mut m,
        &mut ThreadPreviews::default(),
        Ctx::Usage {
            thread: selected.clone(),
            id: 8,
        },
        Ok(Reply::Usage(UsageOutcome::Disabled)),
    );
    assert!(m.usage_disabled);
    assert!(m.usage.values().all(|usage| usage.estimate.is_none()));
    move_cursor(Sel::Task(other));
    m.now += 1_000;
    assert!(
        refresh_usage_at(&mut m).is_none(),
        "an unavailable capability is not retried"
    );
    // The account changing discards everything, estimates, tokens and the flag.
    m.usage.entry(selected.clone()).or_default().tokens = Some((1, 2));
    let mut ev_model = m.clone();
    codex_tui_core::screens::agents::apply(
        &mut ev_model,
        Ev::Account {
            plan: Some(PlanType::Ent26),
            backend_auth: true,
        },
        &mut Default::default(),
    );
    assert!(
        ev_model.usage.is_empty() && !ev_model.usage_disabled && ev_model.pending_usage.is_none()
    );
    // Another backend keeps the tokens and drops the estimates.
    let mut kept = full_usage_model(&selected, &full);
    codex_tui_core::screens::agents::reset_for_backend_switch(&mut kept);
    assert_eq!(kept.usage[&selected].tokens, Some((1, 2)));
    assert_eq!(kept.usage[&selected].estimate, None);
}

fn full_usage_model(selected: &str, full: &codex_app_server_protocol::ThreadUsage) -> Model {
    let mut m = Model::default();
    m.usage.insert(
        selected.to_owned(),
        codex_tui_core::screens::agents::Usage {
            tokens: Some((1, 2)),
            estimate: Some(full.clone()),
            fetched_at: Some(5),
        },
    );
    m
}

/// `prepare_agents_overview_removal` + `hidden_threads` (agents_overview_actions.rs:
/// run_agents_overview_action): the task is gone from the list at once, while
/// the request runs, and back when it fails (`modal_consumes_input_and_finishes_after_input_closes`,
/// agents_overview_action_progress_tests.rs:17: the screen shows 2 tasks).
#[test]
fn a_confirmed_removal_hides_the_task_until_the_request_ends() {
    for (action, delete) in [(Action::Archive, false), (Action::Delete, true)] {
        let mut sandbox = sandbox();
        list(
            &mut sandbox,
            vec![
                thread(&id(1), None, "Task 1", idle()),
                thread(&id(2), None, "Task 2", idle()),
            ],
        );
        let first = selected(&sandbox);
        let mut m = model(&sandbox).clone();
        let outs = up_t(
            &mut m,
            if delete {
                AgentsIntent::Delete
            } else {
                AgentsIntent::Archive
            },
        );
        assert_eq!(m.notice, Some(Notice::Removing(action)));
        let shown = |m: &Model| -> Vec<String> {
            let center = m.center(&CenterThreads::default());
            center.rows.iter().map(|row| row.id().to_owned()).collect()
        };
        assert!(!shown(&m).contains(&first), "hidden while it runs");
        assert_eq!(shown(&m).len(), 1);
        // A failure brings it back and holds the screen with the dialog.
        let mut failed = m.clone();
        let _ = apply_answer(
            &mut failed,
            &mut ThreadPreviews::default(),
            outs[0].ctx.clone(),
            Err(RpcError::Remote("refused".into())),
        );
        assert!(shown(&failed).contains(&first), "back after a failure");
        assert_eq!(failed.notice, None);
        assert_eq!(
            failed.failure,
            Some(Failure {
                action,
                error: "refused".into(),
                attempted: true
            })
        );
        // While the dialog shows, only `Back to agents` acts.
        let before = failed.clone();
        let _ = update(&mut failed, AgentsIntent::CycleGrouping);
        assert_eq!(failed, before);
        let _ = update(&mut failed, AgentsIntent::DismissFailure);
        assert_eq!(failed.failure, None);
        // Success removes it for good.
        let _ = apply_answer(
            &mut m,
            &mut ThreadPreviews::default(),
            outs[0].ctx.clone(),
            Ok(Reply::Done),
        );
        assert!(!shown(&m).contains(&first));
        assert!(!m.hidden.contains(&first), "no stale hidden entry");
    }
}

/// `has_voice` (agents_overview.rs:446): the row whose session owns the
/// voice session is flagged, the others are not.
#[test]
fn the_row_of_the_voice_session_is_flagged() {
    let mut sandbox = sandbox();
    list(
        &mut sandbox,
        vec![
            thread(&id(1), None, "Task 1", idle()),
            thread(&id(2), None, "Task 2", idle()),
        ],
    );
    sandbox.send(AppEv::Screen(ScreenEv::Agents(Ev::Voice(Some(id(2))))));
    let center = model(&sandbox).center(&sandbox.app().agents_threads());
    let flagged: Vec<(String, bool)> = center
        .rows
        .iter()
        .map(|row| (row.id().to_owned(), row.has_voice))
        .collect();
    assert_eq!(
        flagged.iter().filter(|(_, voice)| *voice).count(),
        1,
        "{flagged:?}"
    );
    assert!(
        center
            .rows
            .iter()
            .any(|row| row.id() == id(2) && row.has_voice)
    );
}

/// `PlanType::Ent26` bills in dollars (agents_overview_usage.rs:209).
#[test]
fn only_the_ent26_plan_bills_in_dollars() {
    use codex_protocol::account::PlanType;
    let mut m = Model::default();
    assert!(!m.plan_bills_in_dollars());
    m.plan = Some(PlanType::Plus);
    assert!(!m.plan_bills_in_dollars());
    m.plan = Some(PlanType::Ent26);
    assert!(m.plan_bills_in_dollars());
}

/// The scripts seed the notices the oracle's tests set: the older-server
/// text, the busy and the reconnecting states.
#[test]
fn a_script_seeds_the_notices() {
    for (body, expected) in [
        (json!("reconnecting"), Some(Notice::Reconnecting)),
        (json!("creating_worktree"), Some(Notice::CreatingWorktree)),
        (json!(null), None),
    ] {
        let ev =
            codex_tui_core::screens::agents::seed(json!({ "notice": body })).expect("a notice");
        assert_eq!(ev, Ev::Notice(expected));
    }
    assert!(codex_tui_core::screens::agents::seed(json!({"notice": "bogus"})).is_err());
    assert_eq!(
        codex_tui_core::screens::agents::seed(json!({"server_version": "Service v1"}))
            .expect("a version"),
        Ev::ServerVersion(Some("Service v1".into()))
    );
}

/// VS1 review D5: a search the next keystroke supersedes never answers (the
/// shell drops it), so its ticket is released: typing a word leaves one
/// search in flight, not one per key.
#[test]
fn a_superseded_search_releases_its_ticket() {
    let mut sandbox = sandbox();
    intent(
        &mut sandbox,
        AgentsIntent::SetSearch {
            text: "a".into(),
            lens: searching(),
        },
    );
    let one = sandbox.app().in_flight();
    for text in ["ab", "abc", "abcd"] {
        intent(
            &mut sandbox,
            AgentsIntent::SetSearch {
                text: text.into(),
                lens: searching(),
            },
        );
    }
    assert_eq!(sandbox.app().in_flight(), one);
}

/// VS1 review P1: a notification that changes nothing copies no screen state:
/// a watcher holds the `Rc` of the screens, and a reasoning delta after the
/// header leaves it the same allocation.
#[test]
fn an_unchanging_reasoning_delta_copies_no_screen_state() {
    let mut sandbox = sandbox();
    let target = id(1);
    list(&mut sandbox, vec![thread(&target, None, "Task", idle())]);
    let delta = |text: &str| {
        AppEv::Backend(codex_tui_core::app::BackendEv::Notification(Box::new(
            ServerNotification::ReasoningSummaryTextDelta(
                codex_app_server_protocol::ReasoningSummaryTextDeltaNotification {
                    thread_id: target.clone(),
                    turn_id: "turn".into(),
                    item_id: "item".into(),
                    delta: text.to_owned(),
                    summary_index: 0,
                },
            ),
        )))
    };
    sandbox.send(delta("**Plan**"));
    let held = sandbox.app().screens().clone();
    sandbox.send(delta(" more"));
    assert!(std::rc::Rc::ptr_eq(&held, sandbox.app().screens()));
}

/// `ThreadUnarchived` and `ThreadStarted` take a task off the removed list
/// (oracle `agents_overview_threads.rs:129,147`): an unarchived task is shown
/// again without reopening the center.
#[test]
fn unarchived_and_started_threads_are_no_longer_removed() {
    let target = id(1);
    let task = thread(&target, None, "Archived elsewhere", idle());
    let mut sandbox = sandbox();
    list(&mut sandbox, vec![task.clone()]);
    let mut m = model(&sandbox).clone();
    let archived =
        ServerNotification::ThreadArchived(codex_app_server_protocol::ThreadArchivedNotification {
            thread_id: target.clone(),
        });
    observe(&mut m, &archived);
    assert!(m.removed.contains(&target));
    let unarchived = ServerNotification::ThreadUnarchived(
        codex_app_server_protocol::ThreadUnarchivedNotification {
            thread_id: target.clone(),
        },
    );
    observe(&mut m, &unarchived);
    assert!(!m.removed.contains(&target), "unarchived");
    observe(&mut m, &archived);
    assert!(m.removed.contains(&target));
    let started = ServerNotification::ThreadStarted(ThreadStartedNotification { thread: task });
    observe(&mut m, &started);
    assert!(!m.removed.contains(&target), "started");
    assert!(m.threads.iter().any(|held| held.id == target));
}

/// `ThreadSettingsUpdated` sets the task's directory, model and provider and
/// reads the task again (oracle `agents_overview_threads.rs:180-208`).
#[test]
fn thread_settings_updated_changes_the_row_and_reads_it_again() {
    let target = id(1);
    let mut sandbox = sandbox();
    list(&mut sandbox, vec![thread(&target, None, "Task", idle())]);
    let mut m = model(&sandbox).clone();
    let settings: codex_app_server_protocol::ThreadSettingsUpdatedNotification =
        serde_json::from_value(json!({
            "threadId": target,
            "threadSettings": {
                "disabledPluginIds": [], "cwd": "/tmp/elsewhere", "environments": null,
                "approvalPolicy": "on-request", "approvalsReviewer": "user",
                "sandboxPolicy": {"type": "readOnly", "networkAccess": false},
                "activePermissionProfile": null, "model": "gpt-next",
                "modelProvider": "other", "serviceTier": null, "effort": null,
                "summary": null,
                "collaborationMode": {"mode": "default", "settings": {
                    "model": "gpt-next", "reasoning_effort": null,
                    "developer_instructions": null}},
                "personality": null
            }
        }))
        .expect("settings");
    let notification = ServerNotification::ThreadSettingsUpdated(settings);
    let outs = observe(&mut m, &notification);
    let held = m.threads.iter().find(|held| held.id == target).unwrap();
    assert_eq!(held.cwd.to_string_lossy(), "/tmp/elsewhere");
    assert_eq!(held.model.as_deref(), Some("gpt-next"));
    assert_eq!(held.model_provider, "other");
    assert!(
        outs.iter()
            .any(|out| matches!(&out.command, Command::Thread(ThreadAsk::Read { thread }) if thread == &target)),
        "the task is read again: {outs:?}"
    );
}

/// Opening marks the task while awaiting the reply. A failure returns to the center.
#[test]
fn opening_a_task_is_marked_until_the_resume_answers() {
    let target = id(1);
    let task = thread(&target, None, "Task", idle());
    let mut app = listed_app(vec![task.clone()]);
    assert_eq!(app.screens().agents.opening, None);
    let ticket = resume_ticket(&accept_task(&mut app, &target), &target);
    assert_eq!(app.screens().agents.opening, Some(ThreadId::new(&target)));
    assert_eq!(app.screens().agents.resumes, 0);
    resumed(
        &mut app,
        ticket,
        Err(RpcError::Remote("no such session".into())),
    );
    let model = &app.screens().agents;
    assert_eq!(model.opening, None);
    assert_eq!(model.resumes, 0);
    assert_eq!(
        model.notice,
        Some(Notice::Error(
            "Failed to attach to task: no such session".into()
        ))
    );

    let ticket = resume_ticket(&accept_task(&mut app, &target), &target);
    resumed(&mut app, ticket, Ok(Reply::Resumed(Box::new(task))));
    assert_eq!(app.screens().agents.opening, None);
    assert_eq!(app.screens().agents.resumes, 1);
}

/// A late reply to an opening cancelled with Esc leaves the center in place.
#[test]
fn cancelling_the_opening_ignores_the_late_resume_answer() {
    let target = id(1);
    let task = thread(&target, None, "Task", idle());
    for reply in [
        Err(RpcError::Remote("no such session".into())),
        Ok(Reply::Resumed(Box::new(task.clone()))),
    ] {
        let mut app = listed_app(vec![task.clone()]);
        let ticket = resume_ticket(&accept_task(&mut app, &target), &target);
        assert_eq!(app.screens().agents.opening, Some(ThreadId::new(&target)));
        let effects = dispatch(
            &mut app,
            AppEv::Intent(Intent::Screen(ScreenIntent::Agents(
                AgentsIntent::CancelOpen,
            ))),
        );
        assert!(effects.commands().is_empty());
        assert_eq!(app.screens().agents.opening, None);
        let current = app.session().active.clone();
        resumed(&mut app, ticket, reply);
        assert_eq!(app.screens().agents.notice, None);
        assert_eq!(app.screens().agents.resumes, 0);
        assert_eq!(app.session().active, current);

        let effects = accept_task(&mut app, &target);
        if let [
            AppCommand::Screen {
                ticket,
                command: ScreenCommand::Agents(Command::Thread(ThreadAsk::Resume { .. })),
                ..
            },
        ] = effects.commands()
        {
            resumed(
                &mut app,
                *ticket,
                Ok(Reply::Resumed(Box::new(task.clone()))),
            );
        }
        assert_eq!(app.screens().agents.resumes, 1);
    }
}

/// Reopening while the cancelled attachment is still in flight reuses its ticket.
#[test]
fn reopening_before_the_cancelled_resume_answers_reuses_the_request() {
    let id = id(1);
    let task = thread(&id, None, "Task", idle());
    let mut app = listed_app(vec![task.clone()]);
    let pending = accept_task(&mut app, &id);
    let ticket = resume_ticket(&pending, &id);
    let ev = AppEv::Intent(Intent::Screen(ScreenIntent::Agents(
        AgentsIntent::CancelOpen,
    )));
    dispatch(&mut app, ev);
    assert!(accept_task(&mut app, &id).commands().is_empty());
    assert_eq!(app.screens().agents.opening, Some(ThreadId::new(&id)));
    resumed(&mut app, ticket, Ok(Reply::Resumed(Box::new(task))));
    assert_eq!(app.session().active, Some(ThreadId::new(&id)));
    assert_eq!(app.screens().agents.resumes, 1);
}

/// A resume asks for history, the newest page first.
#[test]
fn an_answered_resume_asks_for_the_history() {
    let target = id(1);
    let task = thread(&target, None, "Task", idle());
    let mut app = listed_app(vec![task.clone()]);
    let ticket = resume_ticket(&accept_task(&mut app, &target), &target);
    let effects = resumed(&mut app, ticket, Ok(Reply::Resumed(Box::new(task))));
    // the thread's goal is read after a resume (`/goal`: a stopped goal asks to resume), and
    // the lists the `@` and `$` popups show (skills, plugins) are asked for; the history is
    // asked for besides
    let asked: Vec<_> = effects
        .commands()
        .iter()
        .filter(|command| {
            !matches!(
                command,
                AppCommand::Feature {
                    command: codex_tui_core::features::FeatureCommand::Goal(_),
                    ..
                } | AppCommand::Screen {
                    command: ScreenCommand::Popovers(_),
                    ..
                }
            )
        })
        .collect();
    assert!(matches!(asked.as_slice(), [AppCommand::Screen {
        command: ScreenCommand::Agents(Command::Thread(ThreadAsk::History { thread, cursor: None })), ..
    }] if thread == &target));
}
