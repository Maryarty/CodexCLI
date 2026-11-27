//! The thread RPC catalog. Each entry pairs the pinned request with the part of
//! its answer the app needs. Callers still own the ticket and its continuation.

use std::borrow::Borrow;

use codex_app_server_protocol as wire;
use serde::Deserialize;
use serde_json::Value;

use super::safety::retry::{
    ForkRequest, ITEM_LIMIT, ItemsRequest, StartRequest, TURN_LIMIT, TurnsRequest,
};
use super::{ReviewTarget, SettingsPatch, UserMessage, structured};
use crate::app::effects::IdePrompt;
use crate::ids::{ThreadId, TurnId};
use crate::rpc::{RpcError, ServerError};
use crate::screens::select::permissions::Snapshot as PermissionSnapshot;
use crate::screens::status::AgentsSource;
use crate::screens::voice::Sdp;
use codex_protocol::models::BUILT_IN_PERMISSION_PROFILE_READ_ONLY;
use codex_protocol::openai_models::ReasoningEffort;
use codex_protocol::protocol::{RealtimeConversationVersion, RealtimeOutputModality};

/// An ask owned by an app or screen command. Transport can borrow a pending
/// turn's existing input through `ThreadRpc` while it builds the request.
pub type ThreadAsk = ThreadRpc<UserMessage>;

/// Encode the existing attachments before prepending context to the wire text.
fn submission_input(
    message: &UserMessage,
    thread: &str,
    context: Option<&IdePrompt>,
) -> Vec<wire::UserInput> {
    let mut input = crate::protocol::turn_input_in(message, Some(thread));
    if let Some(context) = context {
        super::user_prompt::prepend_ide_context(&mut input, context.as_str());
    }
    input
}

impl TurnsRequest {
    pub(super) fn params(&self) -> wire::ThreadTurnsListParams {
        wire::ThreadTurnsListParams {
            thread_id: self.thread().to_string(),
            cursor: None,
            limit: Some(TURN_LIMIT),
            sort_direction: Some(wire::SortDirection::Desc),
            items_view: Some(match self.mode() {
                wire::ThreadHistoryMode::Legacy => wire::TurnItemsView::Full,
                wire::ThreadHistoryMode::Paginated => wire::TurnItemsView::NotLoaded,
            }),
        }
    }
}

impl ItemsRequest {
    pub(super) fn params(&self) -> wire::ThreadItemsListParams {
        wire::ThreadItemsListParams {
            thread_id: self.thread().to_string(),
            turn_id: Some(self.turn().to_string()),
            cursor: None,
            limit: Some(ITEM_LIMIT),
            sort_direction: Some(wire::SortDirection::Asc),
            defer_output: None,
        }
    }
}

impl ForkRequest {
    pub(super) fn params(&self) -> wire::ThreadForkParams {
        let (turn, mode) = self.point();
        // Leave permission parameters absent so the server inherits the source's settings.
        wire::ThreadForkParams {
            thread_id: self.thread().to_string(),
            before_turn_id: Some(turn.to_string()),
            defer_goal_continuation: true,
            exclude_turns: mode == wire::ThreadHistoryMode::Paginated,
            ..Default::default()
        }
    }
}

impl StartRequest {
    pub(super) fn params(&self) -> wire::TurnStartParams {
        let (input, client_id, model) = self.submission();
        // The fork's effective permissions stay in force because the turn carries no overrides.
        wire::TurnStartParams {
            thread_id: self.thread().to_string(),
            input: input.to_vec(),
            client_user_message_id: Some(client_id.to_owned()),
            model: Some(model.to_owned()),
            effort: Some(ReasoningEffort::Low),
            ..Default::default()
        }
    }
}

/// Which part of the thread listing a caller wants. Each caller supplies the
/// filters it needs; unused filters stay absent on the wire.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ListQuery {
    pub current_account_only: bool,
    pub cursor: Option<String>,
    pub limit: Option<u32>,
    pub sort_key: Option<wire::ThreadSortKey>,
    pub sort_direction: Option<wire::SortDirection>,
    pub model_providers: Option<Vec<String>>,
    pub source_kinds: Option<Vec<wire::ThreadSourceKind>>,
    pub archived: Option<bool>,
    pub section_id: Option<String>,
    pub cwd: Option<wire::ThreadListCwdFilter>,
    pub ancestor_thread_id: Option<String>,
    pub use_state_db_only: bool,
}

// The pinned sort-key enum only derives PartialEq.
impl Eq for ListQuery {}

impl ListQuery {
    /// Encode this read for the caller's request ID.
    pub fn wire(self, request_id: wire::RequestId) -> wire::ClientRequest {
        ThreadAsk::list(self).wire(request_id)
    }

    fn params(&self) -> wire::ThreadListParams {
        wire::ThreadListParams {
            include_attached_threads: false,
            current_account_only: self.current_account_only,
            cursor: self.cursor.clone(),
            limit: self.limit,
            excluded_thread_ids: None,
            sort_key: self.sort_key,
            sort_direction: self.sort_direction,
            model_providers: self.model_providers.clone(),
            source_kinds: self.source_kinds.clone(),
            originators: None,
            archived: self.archived,
            section_id: self.section_id.clone().map(Some),
            project_id: None,
            cwd: self.cwd.clone(),
            use_state_db_only: self.use_state_db_only,
            search_term: None,
            parent_thread_id: None,
            ancestor_thread_id: self.ancestor_thread_id.clone(),
            attached_to_thread_id: None,
        }
    }
}

fn voice_start_params(
    thread: &ThreadId,
    session: &str,
    offer: &Sdp,
) -> wire::ThreadRealtimeStartParams {
    wire::ThreadRealtimeStartParams {
        thread_id: thread.to_string(),
        client_managed_handoffs: None,
        delegation_ack_filler: None,
        flush_transcript_tail_on_session_end: None,
        codex_responses_as_items: None,
        codex_response_item_prefix: None,
        codex_response_handoff_mode: None,
        backend_reasoning_status: false,
        codex_response_handoff_channel_prefixes: None,
        model: None,
        output_modality: RealtimeOutputModality::Audio,
        include_startup_context: Some(false),
        initial_items: None,
        realtime_start_instructions: None,
        realtime_end_instructions: None,
        prompt: None,
        realtime_session_id: Some(session.to_string()),
        voice_session_id: None,
        transport: Some(wire::ThreadRealtimeStartTransport::Webrtc {
            sdp: offer.clone().into_inner(),
        }),
        version: Some(RealtimeConversationVersion::V3),
        voice: None,
    }
}

crate::rpc_list! {
    /// The wire operations for thread lifecycle, input and discovery.
    pub enum ThreadRpc<M: Borrow<UserMessage>> => ThreadReply, request model_override: Option<&str>;

    /// Open the backend side of a call once local WebRTC has prepared an offer.
    RealtimeStart { thread: ThreadId, session: String, offer: Sdp },
        Self::RealtimeStart { thread, session, offer }
        => ThreadRealtimeStart(voice_start_params(thread, session, offer))
        -> wire::ThreadRealtimeStartResponse, |_| ThreadReply::Done;
    /// Stop the backend side of a voice call after closing the local peer.
    RealtimeStop { thread: ThreadId }, Self::RealtimeStop { thread }
        => ThreadRealtimeStop(wire::ThreadRealtimeStopParams { thread_id: thread.to_string() })
        -> wire::ThreadRealtimeStopResponse, |_| ThreadReply::Done;

    /// Read the canonical rollout page that contains durable voice captions.
    RealtimeTimeline { thread: ThreadId, cursor: Option<String> },
        Self::RealtimeTimeline { thread, cursor }
        => ThreadTimelineList(wire::ThreadTimelineListParams {
            thread_id: thread.to_string(),
            cursor: cursor.clone(),
            limit: Some(super::voice_transcript::PAGE_LIMIT),
        })
        -> wire::ThreadTimelineListResponse, |answer| ThreadReply::RealtimeTimeline(answer.into());

    /// Launch the chat with its configured model and working directory.
    StartSession { model: Option<String>, cwd: Option<String> }, Self::StartSession { model, cwd }
        => ThreadStart(wire::ThreadStartParams {
            model: model.clone().or_else(|| model_override.map(str::to_owned)),
            cwd: cwd.clone(),
            ..Default::default()
        })
        -> wire::ThreadStartResponse, |answer| session_started(answer);
    /// Create a task. The center reads its row independently of the rest of the settings.
    Start { cwd: Option<String> }, Self::Start { cwd }
        => ThreadStart(wire::ThreadStartParams { cwd: cwd.clone(), ..Default::default() })
        -> StartedThread, |answer| ThreadReply::Started(answer.opened());
    /// Resume with paged history. The initial response carries only the thread row.
    Resume { thread: String }, Self::Resume { thread }
        => ThreadResume(resume(thread, None))
        -> wire::ThreadResumeResponse, |answer| resumed(answer);
    ResumeAt { thread: String, cwd: Option<String> }, Self::ResumeAt { thread, cwd }
        => ThreadResume(resume(thread, cwd.clone()))
        -> wire::ThreadResumeResponse, |answer| resumed(answer);
    Fork { thread: String }, Self::Fork { thread }
        => ThreadFork(fork(thread, None, None))
        -> wire::ThreadForkResponse, |answer| forked(answer);
    /// A side conversation may be ephemeral and start empty with developer instructions.
    ForkAt { thread: String, cwd: Option<String>, temporary: Option<String> },
        Self::ForkAt { thread, cwd, temporary }
        => ThreadFork(fork(thread, cwd.clone(), temporary.clone()))
        -> wire::ThreadForkResponse, |answer| forked(answer);
    /// Prove that a local daemon recognizes the attached-side filter.
    ProbeAttachedSide { thread: ThreadId }, Self::ProbeAttachedSide { thread }
        => ThreadList(wire::ThreadListParams {
            parent_thread_id: Some(thread.to_string()),
            attached_to_thread_id: Some(thread.to_string()),
            ..ListQuery { limit: Some(1), use_state_db_only: true, ..ListQuery::default() }.params()
        })
        -> wire::ThreadListResponse, |_| ThreadReply::AttachedSideSupported(false);
    ListAttachedSide { parent: ThreadId, cursor: Option<String> }, Self::ListAttachedSide { parent, cursor }
        => ThreadList(wire::ThreadListParams {
            attached_to_thread_id: Some(parent.to_string()),
            ..ListQuery {
                current_account_only: true, cursor: cursor.clone(), sort_direction: Some(wire::SortDirection::Asc),
                archived: Some(false), model_providers: Some(Vec::new()), use_state_db_only: true,
                ..ListQuery::default()
            }.params()
        })
        -> wire::ThreadListResponse as listed, |answer| ThreadReply::Listed(Box::new(answer));
    FirstAttachedPrompt { thread: ThreadId, cursor: Option<String> }, Self::FirstAttachedPrompt { thread, cursor }
        => ThreadItemsList(wire::ThreadItemsListParams {
            thread_id: thread.to_string(), turn_id: None,
            cursor: cursor.clone().map(wire::ThreadItemsListCursor::Opaque), limit: Some(16),
            sort_direction: Some(wire::SortDirection::Asc), defer_output: None,
        })
        -> wire::ThreadItemsListResponse, |answer| ThreadReply::RetryItems(Box::new(answer));
    /// Persist a side fork linked to this paginated parent, without replaying its turns.
    ForkAttachedSide { thread: ThreadId, instructions: String },
        Self::ForkAttachedSide { thread, instructions }
        => ThreadFork(wire::ThreadForkParams {
            thread_id: thread.to_string(),
            exclude_turns: true,
            developer_instructions: Some(instructions.clone()),
            attach_to_source: true,
            thread_source: Some(wire::ThreadSource::Feature("tui_side_chat".to_owned())),
            ..Default::default()
        })
        -> wire::ThreadForkResponse, |answer| forked(answer);
    Read { thread: String }, Self::Read { thread }
        => ThreadRead(read(thread, false))
        -> wire::ThreadReadResponse, |answer| ThreadReply::Read {
            unread: answer.read_state.is_some_and(|state| state.first_unread.is_some()),
            thread: Box::new(answer.thread),
        };
    ActiveTurn { thread: String }, Self::ActiveTurn { thread }
        => ThreadRead(read(thread, true))
        -> wire::ThreadReadResponse, |answer| ThreadReply::ActiveTurn(
            answer.thread.turns.into_iter()
                .find(|turn| turn.status == wire::TurnStatus::InProgress)
                .map(|turn| turn.id)
        );
    ReadTurns { thread: String }, Self::ReadTurns { thread }
        => ThreadRead(read(thread, true))
        -> wire::ThreadReadResponse, |answer| ThreadReply::ReadTurns(answer.thread.turns);
    Revert { thread: String, before_turn: TurnId }, Self::Revert { thread, before_turn }
        => ThreadRevert(wire::ThreadRevertParams {
            thread_id: thread.clone(),
            before_turn_id: before_turn.to_string(),
        })
        -> wire::ThreadRevertResponse, |_| ThreadReply::Done;
    Turns { thread: String }, Self::Turns { thread }
        => ThreadTurnsList(wire::ThreadTurnsListParams {
            thread_id: thread.clone(), cursor: None, limit: Some(1),
            sort_direction: None, items_view: None,
        })
        -> wire::ThreadTurnsListResponse, |answer| ThreadReply::LatestTurn(answer.data.into_iter().next().map(Box::new));
    History { thread: String, cursor: Option<String> }, Self::History { thread, cursor }
        => ThreadTurnsList(wire::ThreadTurnsListParams {
            thread_id: thread.clone(), cursor: cursor.clone(), limit: Some(HISTORY_PAGE),
            sort_direction: Some(wire::SortDirection::Desc),
            items_view: Some(wire::TurnItemsView::Full),
        })
        -> wire::ThreadTurnsListResponse, |answer| ThreadReply::History {
            turns: answer.data, next: answer.next_cursor,
        };
    StartUserTurn {
        thread: ThreadId, message: M, client_id: String,
        misalignment_override: Option<i64>, ide_prompt: Option<IdePrompt>
    }, Self::StartUserTurn { thread, message, client_id, misalignment_override, ide_prompt }
        => TurnStart(wire::TurnStartParams {
            thread_id: thread.to_string(),
            input: submission_input(message.borrow(), thread.as_str(), ide_prompt.as_ref()),
            client_user_message_id: Some(client_id.clone()),
            responsesapi_client_metadata: misalignment_override.map(|timestamp| {
                std::collections::HashMap::from([(
                    "misalignment_override".to_owned(),
                    serde_json::json!({ "timestamp": timestamp }).to_string(),
                )])
            }),
            ..Default::default()
        })
        -> wire::TurnStartResponse, |answer| ThreadReply::TurnStarted {
            thread: thread.clone(), turn: TurnId::new(answer.turn.id),
        };
    Interrupt { thread: String, turn: String }, Self::Interrupt { thread, turn }
        => TurnInterrupt(wire::TurnInterruptParams { thread_id: thread.clone(), turn_id: turn.clone() })
        -> wire::TurnInterruptResponse, |_| ThreadReply::Interrupted;
    Steer { thread: String, turn: String, client_id: String, message: M, ide_prompt: Option<IdePrompt> },
        Self::Steer { thread, turn, client_id, message, ide_prompt }
        => TurnSteer(wire::TurnSteerParams {
            thread_id: thread.clone(),
            input: submission_input(message.borrow(), thread.as_str(), ide_prompt.as_ref()),
            expected_turn_id: turn.clone(),
            client_user_message_id: Some(client_id.clone()),
            ..Default::default()
        })
        -> wire::TurnSteerResponse, |answer| ThreadReply::Steered(TurnId::new(answer.turn_id));
    /// A confirmed safety retry needs a decoded interrupt receipt, not the center's generic done.
    RetryInterrupt { thread: ThreadId, turn: TurnId }, Self::RetryInterrupt { thread, turn }
        => TurnInterrupt(wire::TurnInterruptParams { thread_id: thread.to_string(), turn_id: turn.to_string() })
        -> wire::TurnInterruptResponse, |_| ThreadReply::RetryInterrupted;
    RetryTurns(TurnsRequest), Self::RetryTurns(params)
        => ThreadTurnsList(params.params())
        -> wire::ThreadTurnsListResponse, |answer| ThreadReply::RetryTurns(Box::new(answer));
    RetryItems(ItemsRequest), Self::RetryItems(params)
        => ThreadItemsList(params.params())
        -> wire::ThreadItemsListResponse, |answer| ThreadReply::RetryItems(Box::new(answer));
    RetryFork(ForkRequest), Self::RetryFork(params)
        => ThreadFork(params.params())
        -> wire::ThreadForkResponse, |answer| ThreadReply::RetryForked(Box::new(answer.thread));
    RetryStart(StartRequest), Self::RetryStart(params)
        => TurnStart(params.params())
        -> wire::TurnStartResponse, |answer| ThreadReply::TurnStarted {
            thread: params.thread().clone(), turn: TurnId::new(answer.turn.id),
        };
    Compact { thread: String }, Self::Compact { thread }
        => ThreadCompactStart(wire::ThreadCompactStartParams { thread_id: thread.clone() })
        -> wire::ThreadCompactStartResponse, |_| ThreadReply::Done;
    CleanTerminals { thread: String }, Self::CleanTerminals { thread }
        => ThreadBackgroundTerminalsClean(wire::ThreadBackgroundTerminalsCleanParams { thread_id: thread.clone() })
        -> wire::ThreadBackgroundTerminalsCleanResponse, |_| ThreadReply::Done;
    /// A page of the resumed thread's live background terminals.
    ListTerminals { thread: ThreadId, cursor: Option<String> }, Self::ListTerminals { thread, cursor }
        => ThreadBackgroundTerminalsList(wire::ThreadBackgroundTerminalsListParams {
            thread_id: thread.to_string(), cursor: cursor.clone(), limit: None,
        })
        -> wire::ThreadBackgroundTerminalsListResponse, |answer| ThreadReply::Terminals {
            data: answer.data.into_iter().map(|terminal| super::Listed {
                item: terminal.item_id, process: terminal.process_id, command: terminal.command,
            }).collect(),
            next: answer.next_cursor,
        };
    Shell { thread: String, command: String }, Self::Shell { thread, command }
        => ThreadShellCommand(wire::ThreadShellCommandParams {
            thread_id: thread.clone(), command: command.clone(), timeout_ms: None,
        })
        -> wire::ThreadShellCommandResponse, |_| ThreadReply::Done;
    Rename { thread: String, name: String }, Self::Rename { thread, name }
        => ThreadSetName(wire::ThreadSetNameParams { thread_id: thread.clone(), name: name.clone() })
        -> Value, |_| ThreadReply::Done;
    Unsubscribe { thread: String }, Self::Unsubscribe { thread }
        => ThreadUnsubscribe(wire::ThreadUnsubscribeParams { thread_id: thread.clone() })
        -> Value, |_| ThreadReply::Done;
    Review { thread: String, target: ReviewTarget }, Self::Review { thread, target }
        => ReviewStart(wire::ReviewStartParams {
            thread_id: thread.clone(), target: target.wire(), delivery: Some(wire::ReviewDelivery::Inline),
        })
        -> wire::ReviewStartResponse, |_| ThreadReply::Done;
    SettingsUpdate { thread: String, patch: SettingsPatch }, Self::SettingsUpdate { thread, patch }
        => ThreadSettingsUpdate(patch.wire(thread))
        -> Value, |_| ThreadReply::Done;
    PrepareApprovedPlan { thread: String, markdown: String }, Self::PrepareApprovedPlan { thread, markdown }
        => ThreadPrepareApprovedPlan(wire::ThreadPrepareApprovedPlanParams {
            thread_id: thread.clone(), markdown: markdown.clone(), attachment_path: None,
        })
        -> wire::ThreadPrepareApprovedPlanResponse, |answer| ThreadReply::Prepared(answer.prompt);
    McpServers { cwd: Option<String> }, Self::McpServers { cwd }
        => ConfigRead(wire::ConfigReadParams { include_layers: false, cwd: cwd.clone() })
        -> wire::ConfigReadResponse, |answer| mcp_servers(answer);
    StartTemporary(Box<structured::Temporary>), Self::StartTemporary(temporary)
        => ThreadStart(start_temporary(temporary))
        -> wire::ThreadStartResponse, |answer| ThreadReply::Temporary {
            as_asked: temporary.started_as_asked(&answer), thread: ThreadId::new(answer.thread.id),
        };
    StructuredTurn {
        thread: String,
        kind: structured::Kind,
        prompt: String,
        schema: Value,
        effort: Option<codex_protocol::openai_models::ReasoningEffort>,
    }, Self::StructuredTurn { thread, kind, prompt, schema, effort }
        => TurnStart(structured_turn(thread, *kind, prompt, schema, effort.clone()))
        -> wire::TurnStartResponse, |answer| ThreadReply::StructuredTurn { turn: TurnId::new(answer.turn.id) };
    List(Box<ListQuery>), Self::List(query)
        => ThreadList(query.params())
        -> wire::ThreadListResponse as listed, |answer| ThreadReply::Listed(Box::new(answer));
    Search { term: String, limit: u32 }, Self::Search { term, limit }
        => ThreadSearch(wire::ThreadSearchParams {
            include_attached_threads: false,
            current_account_only: true,
            cursor: None,
            limit: Some(*limit),
            sort_key: Some(wire::ThreadSearchSortKey::RecencyAt),
            sort_direction: Some(wire::SortDirection::Desc),
            source_kinds: Some(Vec::new()),
            archived: Some(false),
            search_term: term.trim().to_owned(),
        })
        -> wire::ThreadSearchResponse, |answer| ThreadReply::Searched(answer.data);
    Loaded, Self::Loaded
        => ThreadLoadedList(wire::ThreadLoadedListParams { cursor: None, limit: None })
        -> wire::ThreadLoadedListResponse, |answer| ThreadReply::Loaded(answer.data);
    Archive { thread: ThreadId }, Self::Archive { thread }
        => ThreadArchive(wire::ThreadArchiveParams { thread_id: thread.to_string() })
        -> Value, |_| ThreadReply::Done;
    Unarchive { thread: ThreadId }, Self::Unarchive { thread }
        => ThreadUnarchive(wire::ThreadUnarchiveParams { thread_id: thread.to_string() })
        -> wire::ThreadUnarchiveResponse, |answer| ThreadReply::Unarchived(Box::new(answer.thread));
    Delete { thread: ThreadId }, Self::Delete { thread }
        => ThreadDelete(wire::ThreadDeleteParams { thread_id: thread.to_string() })
        -> Value, |_| ThreadReply::Done;
    MoveSection { thread: ThreadId, section: Option<String> }, Self::MoveSection { thread, section }
        => ThreadSectionMove(wire::ThreadSectionMoveParams {
            thread_id: thread.to_string(), section_id: section.clone(), before_thread_id: None,
        })
        -> Value, |_| ThreadReply::Done;
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StartedThread {
    thread: wire::Thread,
    approval_policy: Option<wire::AskForApproval>,
    approvals_reviewer: Option<wire::ApprovalsReviewer>,
    active_permission_profile: Option<wire::ActivePermissionProfile>,
}

impl StartedThread {
    fn opened(self) -> OpenedThread {
        OpenedThread {
            thread: Box::new(self.thread),
            permissions: self.approval_policy.zip(self.approvals_reviewer).map(
                |(approval, reviewer)| {
                    PermissionSnapshot::from_opening(
                        self.active_permission_profile,
                        approval,
                        reviewer,
                    )
                },
            ),
        }
    }
}

/// The attached thread and the permissions its opening reply reports.
#[derive(Clone, Debug, PartialEq)]
pub struct OpenedThread {
    pub thread: Box<wire::Thread>,
    pub permissions: Option<PermissionSnapshot>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ThreadReply {
    SessionStarted {
        thread: ThreadId,
        details: Option<Box<wire::Thread>>,
        sources: Vec<AgentsSource>,
        permissions: Option<PermissionSnapshot>,
    },
    TurnStarted {
        thread: ThreadId,
        turn: TurnId,
    },
    Interrupted,
    InterruptTurnChanged(TurnId),
    RetryInterrupted,
    RetryTurns(Box<wire::ThreadTurnsListResponse>),
    RetryItems(Box<wire::ThreadItemsListResponse>),
    RetryForked(Box<wire::Thread>),
    Started(OpenedThread),
    Resumed(OpenedThread),
    Forked(OpenedThread),
    AttachedSideSupported(bool),
    AttachedSideStoreUnavailable,
    Read {
        thread: Box<wire::Thread>,
        unread: bool,
    },
    ActiveTurn(Option<String>),
    ReadTurns(Vec<wire::Turn>),
    LatestTurn(Option<Box<wire::Turn>>),
    RealtimeTimeline(super::voice_transcript::Page),
    History {
        turns: Vec<wire::Turn>,
        next: Option<String>,
    },
    Terminals {
        data: Vec<super::Listed>,
        next: Option<String>,
    },
    Steered(TurnId),
    Prepared(String),
    McpServers(serde_json::Map<String, Value>),
    Temporary {
        thread: ThreadId,
        as_asked: bool,
    },
    StructuredTurn {
        turn: TurnId,
    },
    Listed(Box<wire::ThreadListResponse>),
    Searched(Vec<wire::ThreadSearchResult>),
    Loaded(Vec<String>),
    Unarchived(Box<wire::Thread>),
    Done,
}

impl Eq for ThreadReply {}

impl ThreadReply {
    pub(crate) fn permission_baseline(&self) -> Option<(&str, &PermissionSnapshot)> {
        match self {
            Self::SessionStarted {
                thread,
                permissions: Some(permissions),
                ..
            } => Some((thread.as_str(), permissions)),
            Self::Started(opened) | Self::Resumed(opened) | Self::Forked(opened) => {
                Some((&opened.thread.id, opened.permissions.as_ref()?))
            }
            _ => None,
        }
    }
}

/// How many turns one page of a thread's history holds.
pub const HISTORY_PAGE: u32 = 20;

impl<M: Borrow<UserMessage>> ThreadRpc<M> {
    /// The thread this asks about, when it targets one.
    pub fn thread(&self) -> Option<&str> {
        match self {
            Self::Resume { thread }
            | Self::ResumeAt { thread, .. }
            | Self::Fork { thread }
            | Self::ForkAt { thread, .. }
            | Self::Read { thread }
            | Self::ActiveTurn { thread }
            | Self::ReadTurns { thread }
            | Self::Revert { thread, .. }
            | Self::Turns { thread }
            | Self::History { thread, .. }
            | Self::Interrupt { thread, .. }
            | Self::Steer { thread, .. }
            | Self::Compact { thread }
            | Self::CleanTerminals { thread }
            | Self::Shell { thread, .. }
            | Self::Rename { thread, .. }
            | Self::Review { thread, .. }
            | Self::SettingsUpdate { thread, .. }
            | Self::PrepareApprovedPlan { thread, .. }
            | Self::StructuredTurn { thread, .. }
            | Self::Unsubscribe { thread } => Some(thread),
            Self::RetryTurns(params) => Some(params.thread().as_str()),
            Self::RetryItems(params) => Some(params.thread().as_str()),
            Self::RetryFork(params) => Some(params.thread().as_str()),
            Self::RetryStart(params) => Some(params.thread().as_str()),
            Self::RealtimeStart { thread, .. }
            | Self::RealtimeStop { thread }
            | Self::ProbeAttachedSide { thread }
            | Self::ForkAttachedSide { thread, .. }
            | Self::FirstAttachedPrompt { thread, .. }
            | Self::RealtimeTimeline { thread, .. }
            | Self::RetryInterrupt { thread, .. }
            | Self::StartUserTurn { thread, .. }
            | Self::ListTerminals { thread, .. }
            | Self::Archive { thread }
            | Self::Unarchive { thread }
            | Self::Delete { thread }
            | Self::MoveSection { thread, .. } => Some(thread.as_str()),
            Self::Start { .. }
            | Self::StartSession { .. }
            | Self::McpServers { .. }
            | Self::StartTemporary(_)
            | Self::List(_)
            | Self::ListAttachedSide { .. }
            | Self::Search { .. }
            | Self::Loaded => None,
        }
    }

    pub fn wire(&self, request_id: wire::RequestId) -> wire::ClientRequest {
        self.request(None, request_id)
    }

    pub fn decode(&self, json: Value) -> Result<ThreadReply, RpcError> {
        if matches!(self, Self::Start { .. }) && json.get("thread").is_none() {
            return Err(RpcError::Malformed("thread/start: no thread".to_owned()));
        }
        self.reply(json)
    }
}

impl ThreadAsk {
    pub fn list(query: ListQuery) -> Self {
        Self::List(Box::new(query))
    }
}

/// The listing's raw receipt; each caller owns its filtering and sorting policy.
pub fn listed(json: Value) -> Result<wire::ThreadListResponse, RpcError> {
    serde_json::from_value(json).map_err(|error| RpcError::Malformed(error.to_string()))
}

/// Decode a refusal to stop a turn that ended between intent and delivery.
pub(crate) fn changed_turn(expected: &TurnId, error: &ServerError) -> Option<ThreadReply> {
    if error.code != Some(-32602) {
        return None;
    }
    let (sent, actual) = error
        .message
        .strip_prefix("expected active turn id ")?
        .split_once(" but found ")?;
    let actual = actual.trim_matches('`');
    (sent.trim_matches('`') == expected.as_str()
        && !actual.is_empty()
        && !actual.contains(char::is_whitespace)
        && actual != expected.as_str())
    .then(|| ThreadReply::InterruptTurnChanged(TurnId::new(actual)))
}

fn resume(thread: &str, cwd: Option<String>) -> wire::ThreadResumeParams {
    wire::ThreadResumeParams {
        thread_id: thread.to_owned(),
        exclude_turns: true,
        cwd,
        ..Default::default()
    }
}

fn fork(thread: &str, cwd: Option<String>, temporary: Option<String>) -> wire::ThreadForkParams {
    wire::ThreadForkParams {
        thread_id: thread.to_owned(),
        cwd,
        ephemeral: temporary.is_some(),
        exclude_turns: temporary.is_some(),
        developer_instructions: temporary,
        ..Default::default()
    }
}

fn read(thread: &str, include_turns: bool) -> wire::ThreadReadParams {
    wire::ThreadReadParams {
        thread_id: thread.to_owned(),
        include_turns,
    }
}

fn session_started(response: wire::ThreadStartResponse) -> ThreadReply {
    // Same-platform paths are displayed relative to the directory. Remote
    // paths with a foreign convention are kept as the server spells them.
    let sources = response
        .instruction_source_path_uris()
        .into_iter()
        .map(|uri| match uri.to_abs_path() {
            Ok(path) => AgentsSource::Native(path.as_path().display().to_string()),
            Err(_) => AgentsSource::Foreign(uri.inferred_native_path_string()),
        })
        .collect();
    ThreadReply::SessionStarted {
        thread: ThreadId::new(&response.thread.id),
        details: Some(configured_model(
            response.thread,
            response.model,
            response.reasoning_effort,
        )),
        sources,
        permissions: Some(PermissionSnapshot::from_opening(
            response.active_permission_profile,
            response.approval_policy,
            response.approvals_reviewer,
        )),
    }
}

fn resumed(response: wire::ThreadResumeResponse) -> ThreadReply {
    ThreadReply::Resumed(OpenedThread {
        thread: configured_model(response.thread, response.model, response.reasoning_effort),
        permissions: Some(PermissionSnapshot::from_opening(
            response.active_permission_profile,
            response.approval_policy,
            response.approvals_reviewer,
        )),
    })
}

fn forked(response: wire::ThreadForkResponse) -> ThreadReply {
    ThreadReply::Forked(OpenedThread {
        thread: configured_model(response.thread, response.model, response.reasoning_effort),
        permissions: Some(PermissionSnapshot::from_opening(
            response.active_permission_profile,
            response.approval_policy,
            response.approvals_reviewer,
        )),
    })
}

/// The opening response names the live configuration, while the embedded row can omit it.
fn configured_model(
    mut thread: wire::Thread,
    model: String,
    effort: Option<codex_protocol::openai_models::ReasoningEffort>,
) -> Box<wire::Thread> {
    thread.model = Some(model);
    thread.reasoning_effort = effort;
    Box::new(thread)
}

fn mcp_servers(mut response: wire::ConfigReadResponse) -> ThreadReply {
    ThreadReply::McpServers(match response.config.additional.remove("mcp_servers") {
        Some(Value::Object(servers)) => servers,
        _ => serde_json::Map::new(),
    })
}

fn structured_turn(
    thread: &str,
    kind: structured::Kind,
    prompt: &str,
    schema: &Value,
    effort: Option<codex_protocol::openai_models::ReasoningEffort>,
) -> wire::TurnStartParams {
    let title = kind == structured::Kind::Title;
    wire::TurnStartParams {
        thread_id: thread.to_owned(),
        input: vec![wire::UserInput::Text {
            text: prompt.to_owned(),
            text_elements: Vec::new(),
        }],
        permissions: title.then(|| BUILT_IN_PERMISSION_PROFILE_READ_ONLY.to_owned()),
        runtime_workspace_roots: title.then(Vec::new),
        output_schema: Some(schema.clone()),
        effort,
        ..Default::default()
    }
}

/// `thread/start` of the ephemeral thread of a structured turn (`start_temporary_thread`): no
/// tools, no environments, every MCP server of the config disabled, and read-only: the built-in
/// read-only profile by name for a title, the active profile when a recap's is a named one, else
/// the default permissions and the sandbox mode say read-only. The thread's own config has
/// everything else off (`structured::off`).
fn start_temporary(temporary: &structured::Temporary) -> wire::ThreadStartParams {
    let (source, approval_policy) = match temporary.kind {
        structured::Kind::Title => ("thread_title", Some(wire::AskForApproval::Never)),
        structured::Kind::Recap => ("system", None),
    };
    let permissions = temporary.permissions();
    let mut config = structured::off();
    if permissions.is_none() {
        // Managed profiles take precedence over the legacy sandbox override below.
        config.insert(
            "default_permissions".to_owned(),
            BUILT_IN_PERMISSION_PROFILE_READ_ONLY.into(),
        );
    }
    let servers = temporary
        .mcp_servers
        .keys()
        .map(|name| (name.clone(), serde_json::json!({ "enabled": false })))
        .collect::<serde_json::Map<_, _>>();
    config.insert("mcp_servers".to_owned(), serde_json::Value::Object(servers));
    wire::ThreadStartParams {
        model: temporary.model.clone(),
        model_provider: temporary.provider.clone(),
        cwd: temporary.cwd.clone(),
        approval_policy,
        sandbox: permissions.is_none().then_some(wire::SandboxMode::ReadOnly),
        permissions,
        runtime_workspace_roots: Some(Vec::new()),
        ephemeral: Some(true),
        thread_source: Some(wire::ThreadSource::Feature(source.to_owned())),
        environments: Some(Vec::new()),
        dynamic_tools: Some(Vec::new()),
        selected_capability_roots: Some(Vec::new()),
        config: Some(config),
        ..wire::ThreadStartParams::default()
    }
}

/// Lower both local submission intents to the pinned Cloud StartOrSteer input.
pub fn cloud_submit_params(request: wire::ClientRequest) -> Option<wire::TurnStartParams> {
    match request {
        wire::ClientRequest::TurnStart { params, .. } => Some(params),
        wire::ClientRequest::TurnSteer { params, .. } => Some(wire::TurnStartParams {
            thread_id: params.thread_id,
            input: params.input,
            client_user_message_id: params.client_user_message_id,
            ..Default::default()
        }),
        _ => None,
    }
}
