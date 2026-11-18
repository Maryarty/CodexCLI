//! The screen's `apply`: events, intents, replies and notifications change
//! the model and name the RPCs to send (`Out`); the app mints their tickets.
//! Oracle: `agents_overview.rs` (refresh apply, open), `agents_overview_threads.rs`
//! (`track_agents_overview_notification`), `agents_overview_actions.rs`,
//! `agents_overview_view.rs` (`handle_key_event` arms), `agents_overview_discovery.rs`.

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::ItemCompletedNotification;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::ServerNotification;
use codex_app_server_protocol::ThreadArchiveParams;
use codex_app_server_protocol::ThreadDeleteParams;
use codex_app_server_protocol::ThreadItemPayload;
use codex_app_server_protocol::ThreadListResponse;
use codex_app_server_protocol::ThreadResumeParams;
use codex_app_server_protocol::ThreadResumeResponse;
use codex_app_server_protocol::ThreadSetNameParams;
use codex_app_server_protocol::ThreadStatus;
use codex_app_server_protocol::ThreadStartParams;

use super::Action;
use super::Activity;
use super::Command;
use super::Confirm;
use super::Ctx;
use super::Ev;
use super::Intent;
use super::Model;
use super::Notice;
use super::PAGE_SIZE;
use super::Page;
use super::Rename;
use super::Reply;
use super::Sel;
use super::Usage;
use super::carried::is_overview_thread;
use super::carried::preview_markdown;
use super::carried::supports_shared_pinning;
use crate::rpc::RpcError;

/// An RPC to send and what it is for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Out {
    pub command: Command,
    pub ctx: Ctx,
}

fn out(command: Command, ctx: Ctx) -> Out {
    Out { command, ctx }
}

/// Oracle: `agents_overview_view`'s parent link: a spawned subagent hangs
/// under the thread that spawned it.
fn normalize(mut thread: codex_app_server_protocol::Thread) -> codex_app_server_protocol::Thread {
    if thread.parent_thread_id.is_none()
        && let codex_app_server_protocol::SessionSource::SubAgent(
            codex_protocol::protocol::SubAgentSource::ThreadSpawn {
                parent_thread_id, ..
            },
        ) = &thread.source
    {
        thread.parent_thread_id = Some(parent_thread_id.to_string());
    }
    thread
}

fn merge_page(model: &mut Model, page: Page) {
    let known: Vec<String> = page.threads.iter().map(|thread| thread.id.clone()).collect();
    let fresh = page
        .threads
        .into_iter()
        .filter(|thread| !thread.ephemeral)
        .map(normalize)
        .collect::<Vec<_>>();
    if page.append {
        for thread in fresh {
            match model.threads.iter_mut().find(|held| held.id == thread.id) {
                Some(held) => *held = thread,
                None => model.threads.push(thread),
            }
        }
    } else {
        // A refresh keeps what the list does not reach (children, rows past
        // the first page) and replaces what it does.
        let mut merged = fresh;
        merged.extend(
            model
                .threads
                .drain(..)
                .filter(|held| !known.contains(&held.id)),
        );
        model.threads = merged;
    }
    for id in page.unread {
        model.unread.insert(id);
    }
    model.paging.cursor = page.next_cursor.clone();
    model.paging.has_more = page.next_cursor.is_some();
    model.paging.loading = false;
    model.paging.failed = false;
    if matches!(model.notice, Some(Notice::LoadFailed)) {
        model.notice = None;
    }
    model.reconcile();
}

pub(super) fn apply_ev(model: &mut Model, ev: Ev) {
    match ev {
        Ev::Listed(page) => merge_page(model, page),
        Ev::Clock(now) => model.now = now,
        Ev::Home(home) => model.home = home,
        Ev::Current(current) => model.current = current,
        Ev::Pinned(pinned) => {
            model.pinned = pinned;
            model.reconcile();
        }
        Ev::ListFailed => {
            model.paging.loading = false;
            model.paging.failed = true;
        }
        Ev::Notice(notice) => model.notice = notice,
        Ev::Request(thread, preview) => match preview {
            Some(preview) => {
                model.requests.insert(thread, preview);
            }
            None => {
                model.requests.remove(&thread);
            }
        },
    }
}

fn list(model: &mut Model, cursor: Option<String>, append: bool) -> Out {
    model.paging.loading = true;
    out(Command::List { cursor }, Ctx::List { append })
}

/// The intent's change and the RPCs it asks for.
pub fn update(model: &mut Model, intent: Intent) -> Vec<Out> {
    let mut outs = Vec::new();
    match intent {
        Intent::Open | Intent::Refresh => outs.push(list(model, None, false)),
        Intent::Move { forward } => model.move_selection(forward),
        Intent::Jump { top } => model.jump(top),
        Intent::Page { forward, height } => model.page(forward, height),
        Intent::Select(sel) => {
            if model.rename.is_none() {
                model.selected = Some(sel);
                model.reconcile();
            }
        }
        Intent::CycleFilter { back } => {
            let count = super::center::FILTER_GROUPS.len();
            let step = if back { count - 1 } else { 1 };
            model.filter = (model.filter + step) % count;
            model.reconcile();
        }
        Intent::SetFilter(filter) => {
            model.filter = filter.min(super::center::FILTER_GROUPS.len() - 1);
            model.reconcile();
        }
        Intent::CycleGrouping => model.grouping = model.grouping.next(),
        Intent::SetGrouping(grouping) => model.grouping = grouping,
        Intent::ToggleHelp => model.help = !model.help,
        Intent::CloseHelp => model.help = false,
        Intent::BeginSearch => {
            if model.rename.is_none() {
                model.browse = model.selected.clone();
                model.searching = true;
            }
        }
        Intent::Type(text) => type_text(model, &text),
        Intent::Backspace => backspace(model),
        Intent::CancelEditing => cancel_editing(model),
        Intent::BeginRename => {
            let center = model.center();
            if model.rename.is_none()
                && let Some(row) = center.selected_row()
            {
                let text = row.thread.name.clone().unwrap_or_default();
                model.rename = Some(Rename {
                    target: row.id().to_owned(),
                    text,
                });
                model.searching = false;
                model.search.clear();
            }
        }
        Intent::Accept => accept(model, &mut outs),
        Intent::Stop => {
            if let Some(row) = model.center().selected_row()
                && matches!(row.thread.status, ThreadStatus::Active { .. })
            {
                let thread = row.id().to_owned();
                outs.push(out(
                    Command::ActiveTurn {
                        thread: thread.clone(),
                    },
                    Ctx::Stop { thread },
                ));
            }
        }
        Intent::Archive => confirm(model, Action::Archive),
        Intent::Delete => confirm(model, Action::Delete),
        Intent::Confirmed(yes) => {
            if let Some(confirm) = model.confirm.take()
                && yes
            {
                let next = model.selection_after_removal(std::slice::from_ref(&confirm.thread));
                model.notice = Some(Notice::Removing(confirm.action));
                let command = match confirm.action {
                    Action::Archive => Command::Archive {
                        thread: confirm.thread.clone(),
                    },
                    Action::Delete => Command::Delete {
                        thread: confirm.thread.clone(),
                    },
                };
                model.selected = next.map(Sel::Task);
                outs.push(out(
                    command,
                    Ctx::Action {
                        thread: confirm.thread,
                        action: Some(confirm.action),
                    },
                ));
            }
        }
        Intent::Hide => {
            let center = model.center();
            if model.rename.is_none()
                && let Some(row) = center.selected_row()
            {
                let id = row.id().to_owned();
                let next = model.selection_after_removal(std::slice::from_ref(&id));
                model.hidden.insert(id);
                model.selected = next.map(Sel::Task);
                model.reconcile();
            }
        }
        Intent::TogglePin => {
            let center = model.center();
            if model.pinned.is_some()
                && let Some(row) = center.selected_row()
                && supports_shared_pinning(&row.thread.source)
            {
                let thread = row.id().to_owned();
                let pinned = !row.pinned;
                outs.push(out(
                    Command::Pin {
                        thread: thread.clone(),
                        pinned,
                    },
                    Ctx::Pin { thread },
                ));
            }
        }
        Intent::New => {
            let center = model.center();
            let cwd = center
                .selected_row()
                .map(|row| row.thread.cwd.display().to_string());
            outs.push(out(Command::Start { cwd }, Ctx::Start));
        }
    }
    outs
}

fn confirm(model: &mut Model, action: Action) {
    let center = model.center();
    if model.rename.is_none()
        && let Some(row) = center.selected_row()
    {
        model.confirm = Some(Confirm {
            thread: row.id().to_owned(),
            action,
        });
    }
}

fn editing(model: &Model) -> bool {
    model.searching || model.rename.is_some()
}

fn type_text(model: &mut Model, text: &str) {
    if let Some(rename) = &mut model.rename {
        rename.text.push_str(text);
    } else if model.searching {
        model.search.push_str(text);
        search_changed(model);
    }
}

fn backspace(model: &mut Model) {
    if let Some(rename) = &mut model.rename {
        rename.text.pop();
    } else if model.searching {
        model.search.pop();
        search_changed(model);
    }
}

/// The first match is highlighted again (oracle: `edit_input`).
fn search_changed(model: &mut Model) {
    let center = model.center();
    model.selected = center
        .entries
        .iter()
        .find_map(|entry| match entry {
            super::CenterRow::Task(index) => center.rows.get(*index).map(|row| Sel::Task(row.id().to_owned())),
            _ => None,
        });
}

/// Oracle: `on_ctrl_c` of the view for the editing case.
fn cancel_editing(model: &mut Model) {
    if !editing(model) {
        return;
    }
    let restore = model.searching.then(|| model.browse.take());
    model.searching = false;
    model.rename = None;
    model.search.clear();
    if let Some(selected) = restore {
        model.selected = selected;
    }
    model.reconcile();
}

fn accept(model: &mut Model, outs: &mut Vec<Out>) {
    let center = model.center();
    if center.show_more_selected {
        if !model.paging.loading {
            let cursor = model.paging.cursor.clone();
            outs.push(list(model, cursor, true));
        }
        return;
    }
    if let Some(rename) = model.rename.take() {
        let name = rename.text.trim().to_owned();
        if name.is_empty() {
            model.rename = Some(rename);
            return;
        }
        outs.push(out(
            Command::Rename {
                thread: rename.target.clone(),
                name,
            },
            Ctx::Rename {
                thread: rename.target,
            },
        ));
        model.reconcile();
        return;
    }
    if let Some(row) = center.selected_row() {
        let thread = row.id().to_owned();
        if model.current.as_deref() != Some(thread.as_str()) {
            outs.push(out(
                Command::Resume {
                    thread: thread.clone(),
                },
                Ctx::Resume { thread },
            ));
        }
        if model.searching {
            model.searching = false;
            model.search.clear();
        }
    }
}

/// The answer of an RPC of ours.
pub fn apply_answer(model: &mut Model, ctx: Ctx, result: Result<Reply, RpcError>) -> Vec<Out> {
    match (ctx, result) {
        (Ctx::List { append }, Ok(Reply::Listed(mut page))) => {
            page.append = append;
            merge_page(model, page);
        }
        (Ctx::List { .. }, Err(_) | Ok(_)) => {
            model.paging.loading = false;
            model.paging.failed = true;
            model.notice = Some(Notice::LoadFailed);
        }
        (Ctx::Rename { thread }, Ok(_)) => {
            let _ = thread;
        }
        (Ctx::Action { thread, action }, Ok(_)) => {
            if action.is_some() {
                model.threads.retain(|held| held.id != thread && held.parent_thread_id.as_deref() != Some(&thread));
                model.notice = None;
                model.reconcile();
            }
        }
        (Ctx::Action { action, .. }, Err(error)) => {
            let _ = action;
            model.notice = Some(Notice::Error(error_text(error)));
        }
        (Ctx::Stop { thread }, Ok(Reply::ActiveTurn(Some(turn)))) => {
            return vec![out(Command::Interrupt { thread, turn }, Ctx::Interrupt)];
        }
        (Ctx::Stop { .. } | Ctx::Interrupt, Ok(_)) => {}
        (
            Ctx::Rename { .. }
            | Ctx::Pin { .. }
            | Ctx::Resume { .. }
            | Ctx::Stop { .. }
            | Ctx::Interrupt,
            Err(error),
        ) => {
            model.notice = Some(Notice::Error(error_text(error)));
        }
        (Ctx::Pin { thread }, Ok(_)) => {
            if let Some(pins) = &mut model.pinned {
                if pins.contains(&thread) {
                    pins.retain(|pin| *pin != thread);
                } else {
                    pins.insert(0, thread);
                }
            }
        }
        (Ctx::Resume { thread }, Ok(_)) => model.current = Some(thread),
        (Ctx::Start, _) => {}
    }
    Vec::new()
}

fn error_text(error: RpcError) -> String {
    match error {
        RpcError::Remote(message) | RpcError::Malformed(message) => message,
        RpcError::Mismatch => "unexpected reply".to_owned(),
    }
}

/// Whether `observe` reads `notification`.
pub fn observes(notification: &ServerNotification) -> bool {
    matches!(
        notification,
        ServerNotification::ThreadStarted(_)
            | ServerNotification::ThreadStatusChanged(_)
            | ServerNotification::ThreadNameUpdated(_)
            | ServerNotification::ThreadArchived(_)
            | ServerNotification::ThreadDeleted(_)
            | ServerNotification::ThreadClosed(_)
            | ServerNotification::ThreadReadStateChanged(_)
            | ServerNotification::ThreadTokenUsageUpdated(_)
            | ServerNotification::ItemCompleted(_)
            | ServerNotification::ReasoningSummaryTextDelta(_)
            | ServerNotification::TurnStarted(_)
    )
}

/// A server notification: the oracle's `track_agents_overview_notification`
/// and `track_agents_overview_activity`.
pub fn observe(model: &mut Model, notification: &ServerNotification) {
    match notification {
        ServerNotification::ThreadStarted(started) => {
            if !started.thread.ephemeral {
                let mut thread = started.thread.clone();
                thread.turns.clear();
                let thread = normalize(thread);
                match model.threads.iter_mut().find(|held| held.id == thread.id) {
                    Some(held) => *held = thread,
                    None => model.threads.insert(0, thread),
                }
            }
        }
        ServerNotification::ThreadStatusChanged(change) => {
            if let Some(thread) = model.threads.iter_mut().find(|t| t.id == change.thread_id) {
                thread.status = change.status.clone();
            }
            if !matches!(change.status, ThreadStatus::Active { .. })
                && let Some(activity) = model.activity.get_mut(&change.thread_id)
            {
                activity.reasoning_header = None;
            }
        }
        ServerNotification::ThreadNameUpdated(update) => {
            if let Some(thread) = model.threads.iter_mut().find(|t| t.id == update.thread_id) {
                thread.name.clone_from(&update.thread_name);
            }
        }
        ServerNotification::ThreadArchived(gone) => remove(model, &gone.thread_id),
        ServerNotification::ThreadDeleted(gone) => remove(model, &gone.thread_id),
        ServerNotification::ThreadClosed(closed) => {
            model.activity.remove(&closed.thread_id);
            model.usage.remove(&closed.thread_id);
            if let Some(thread) = model.threads.iter_mut().find(|t| t.id == closed.thread_id) {
                thread.status = ThreadStatus::NotLoaded;
            }
        }
        ServerNotification::ThreadReadStateChanged(change) => {
            if change.read_state.first_unread.is_some() {
                model.unread.insert(change.thread_id.clone());
            } else {
                model.unread.remove(&change.thread_id);
            }
        }
        ServerNotification::ThreadTokenUsageUpdated(update) => {
            if model.threads.iter().any(|t| t.id == update.thread_id) {
                model.usage.insert(
                    update.thread_id.clone(),
                    Usage {
                        input: update.token_usage.total.input_tokens,
                        output: update.token_usage.total.output_tokens,
                    },
                );
            }
        }
        ServerNotification::ItemCompleted(ItemCompletedNotification { thread_id, item, .. }) => {
            if let ThreadItemPayload::AgentMessage { text, .. } = &item.payload {
                let activity = model.activity.entry(thread_id.clone()).or_default();
                activity.last_message = Some(preview_markdown(text));
                activity.reasoning_header = None;
            }
        }
        ServerNotification::ReasoningSummaryTextDelta(delta) => {
            let activity = model.activity.entry(delta.thread_id.clone()).or_default();
            if activity.reasoning_header.is_none() {
                let buffer = activity.latest_prompt.clone().unwrap_or_default();
                let _ = buffer;
                activity.reasoning_header = first_bold(&delta.delta).map(|h| super::carried::preview_text(&h));
            }
        }
        ServerNotification::TurnStarted(started) => {
            if let Some(activity) = model.activity.get_mut(&started.thread_id) {
                activity.reasoning_header = None;
            }
        }
        _ => return,
    }
    model.reconcile();
}

fn remove(model: &mut Model, id: &str) {
    model
        .threads
        .retain(|held| held.id != id && held.parent_thread_id.as_deref() != Some(id));
    model.activity.remove(id);
    model.last_messages.remove(id);
    model.usage.remove(id);
}

/// The first `**bold**` run of `text` (oracle: `extract_first_bold`).
fn first_bold(text: &str) -> Option<String> {
    let start = text.find("**")? + 2;
    let end = text[start..].find("**")? + start;
    Some(text[start..end].to_owned())
}

/// The wire request of `command` (oracle: `agents_overview_discovery.rs`,
/// `agents_overview_actions.rs`).
pub(super) fn wire_request(command: &Command, request_id: RequestId) -> ClientRequest {
    match command {
        Command::List { cursor } => ClientRequest::ThreadList {
            request_id,
            // Built from its wire form: the fields left out are the optional
            // ones the oracle leaves unset.
            params: serde_json::from_value(serde_json::json!({
                "includeAttachedThreads": false,
                "currentAccountOnly": true,
                "cursor": cursor,
                "limit": PAGE_SIZE,
                "sortKey": "recency_at",
                "modelProviders": [],
                "sourceKinds": [],
                "archived": false,
                "useStateDbOnly": true,
            }))
            .expect("the list params"),
        },
        Command::Rename { thread, name } => ClientRequest::ThreadSetName {
            request_id,
            params: ThreadSetNameParams {
                thread_id: thread.clone(),
                name: name.clone(),
            },
        },
        Command::Archive { thread } => ClientRequest::ThreadArchive {
            request_id,
            params: ThreadArchiveParams {
                thread_id: thread.clone(),
            },
        },
        Command::Delete { thread } => ClientRequest::ThreadDelete {
            request_id,
            params: ThreadDeleteParams {
                thread_id: thread.clone(),
            },
        },
        Command::ActiveTurn { thread } => ClientRequest::ThreadRead {
            request_id,
            params: codex_app_server_protocol::ThreadReadParams {
                thread_id: thread.clone(),
                include_turns: true,
            },
        },
        Command::Interrupt { thread, turn } => ClientRequest::TurnInterrupt {
            request_id,
            params: codex_app_server_protocol::TurnInterruptParams {
                thread_id: thread.clone(),
                turn_id: turn.clone(),
            },
        },
        Command::Pin { thread, pinned } => ClientRequest::ThreadSectionMove {
            request_id,
            params: codex_app_server_protocol::ThreadSectionMoveParams {
                thread_id: thread.clone(),
                section_id: pinned.then(|| PINNED_SECTION.to_owned()),
                before_thread_id: None,
            },
        },
        Command::Resume { thread } => ClientRequest::ThreadResume {
            request_id,
            params: ThreadResumeParams {
                thread_id: thread.clone(),
                ..ThreadResumeParams::default()
            },
        },
        Command::Start { cwd } => ClientRequest::ThreadStart {
            request_id,
            params: ThreadStartParams {
                cwd: cwd.clone(),
                ..ThreadStartParams::default()
            },
        },
    }
}

/// `codex_state::PINNED_THREAD_SECTION_ID`, which core does not link.
const PINNED_SECTION: &str = "01984de2-8f74-7c91-a3b2-5c5e937cf318";

pub(super) fn decode_reply(
    command: &Command,
    result: serde_json::Value,
) -> Result<Reply, RpcError> {
    let malformed = |error: serde_json::Error| RpcError::Malformed(error.to_string());
    match command {
        Command::List { .. } => {
            let page: ThreadListResponse = serde_json::from_value(result).map_err(malformed)?;
            let unread = page
                .read_states
                .iter()
                .flatten()
                .filter(|(_, state)| state.first_unread.is_some())
                .map(|(id, _)| id.clone())
                .collect();
            Ok(Reply::Listed(Page {
                threads: page
                    .data
                    .into_iter()
                    .filter(is_overview_thread)
                    .collect(),
                unread,
                next_cursor: page.next_cursor,
                append: false,
            }))
        }
        Command::Resume { .. } => {
            let response: ThreadResumeResponse = serde_json::from_value(result).map_err(malformed)?;
            Ok(Reply::Resumed(Box::new(response.thread)))
        }
        Command::ActiveTurn { .. } => {
            let response: codex_app_server_protocol::ThreadReadResponse =
                serde_json::from_value(result).map_err(malformed)?;
            Ok(Reply::ActiveTurn(
                response
                    .thread
                    .turns
                    .into_iter()
                    .find(|turn| turn.status == codex_app_server_protocol::TurnStatus::InProgress)
                    .map(|turn| turn.id),
            ))
        }
        _ => Ok(Reply::Done),
    }
}

/// A script's seed, see `seed` in `mod.rs`.
pub(super) fn seed_ev(value: serde_json::Value) -> Result<Ev, String> {
    let (key, body) = value
        .as_object()
        .and_then(|object| object.iter().next())
        .map(|(key, body)| (key.clone(), body.clone()))
        .ok_or_else(|| "an agents event is an object with one key".to_owned())?;
    let text = |error: serde_json::Error| error.to_string();
    match key.as_str() {
        "listed" => {
            #[derive(serde::Deserialize)]
            struct Seed {
                threads: Vec<codex_app_server_protocol::Thread>,
                #[serde(default)]
                unread: Vec<String>,
                #[serde(default)]
                next_cursor: Option<String>,
            }
            let seed: Seed = serde_json::from_value(body).map_err(text)?;
            Ok(Ev::Listed(Page {
                threads: seed.threads,
                unread: seed.unread,
                next_cursor: seed.next_cursor,
                append: false,
            }))
        }
        "home" => serde_json::from_value(body).map(Ev::Home).map_err(text),
        "clock" => serde_json::from_value(body).map(Ev::Clock).map_err(text),
        "current" => serde_json::from_value(body).map(Ev::Current).map_err(text),
        "pinned" => serde_json::from_value(body).map(Ev::Pinned).map_err(text),
        "request" => serde_json::from_value::<(String, String)>(body)
            .map(|(thread, preview)| Ev::Request(thread, Some(preview)))
            .map_err(text),
        other => Err(other.to_owned()),
    }
}

#[allow(dead_code)]
fn _unused(_: &Activity) {}
