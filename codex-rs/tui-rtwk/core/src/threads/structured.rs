//! The temporary structured turn (oracle `temporary_structured_request.rs`): an isolated request
//! to the model that a feature makes through `AppCx::structured_turn`, run by the app
//! (`app/structured.rs`) as ephemeral thread RPCs (ADR-003: made into wire requests in
//! `threads.rs`, from this file's data): `config/read` for the MCP servers, `thread/start`
//! (ephemeral, read-only, no tools, every MCP server disabled), `turn/start` with an output
//! schema, the thread's last assistant message, and `thread/unsubscribe`.
//!
//! Structured prompts can contain untrusted transcript text, so the thread is started without
//! widening permissions or exposing tools and environment access. As on desktop, a title thread
//! asks for read-only permissions and the `never` approval policy; a recap thread keeps the
//! active permission profile when it is a named one.

use std::collections::HashMap;

use codex_app_server_protocol::ThreadStartResponse;
use codex_protocol::models::{BUILT_IN_PERMISSION_PROFILE_READ_ONLY, PermissionProfile};
use codex_protocol::openai_models::ReasoningEffort;
use serde_json::{Map, Value};

use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::threads::TurnEnd;

/// How long one phase of the request may take (`STRUCTURED_TURN_TIMEOUT`): the thread's start,
/// then its turn.
pub const TIMEOUT_MILLIS: i64 = 30_000;

/// The longest answer taken (`STRUCTURED_RESPONSE_MAX_BYTES`).
pub const RESPONSE_MAX_BYTES: usize = 8 * 1024;

/// What the request is for: the two differ in the permissions the thread asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A thread title (`thread_title`): read-only, `never` approval.
    Title,
    /// A recap (`system`): the active permission profile when it is a named one.
    Recap,
}

/// What a feature asks of the temporary thread (`AppCx::structured_turn`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructuredTurn {
    pub kind: Kind,
    /// The thread the request is about: the temporary one starts in its directory, with its
    /// provider and (unless `model` is set) its model.
    pub source: ThreadId,
    /// The model to ask, when it is not the source thread's.
    pub model: Option<String>,
    /// The reasoning effort of the turn, when the feature picks one.
    pub effort: Option<ReasoningEffort>,
    /// The only input of the turn.
    pub prompt: String,
    /// The JSON schema the last message follows.
    pub schema: Value,
}

/// Why a structured turn gave no answer (`Feature::structured_done`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Why {
    /// The server refused one of the requests, or the request was lost with its connection.
    Refused(RpcError),
    /// The thread did not start with the permissions that were asked for.
    NotReadOnly,
    /// The turn did not complete.
    Ended(TurnEnd),
    /// The turn completed without a message.
    Empty,
    /// The message is longer than `RESPONSE_MAX_BYTES`.
    TooLong,
    /// A phase took longer than `TIMEOUT_MILLIS`.
    TimedOut,
    /// The connection was lost while the turn ran.
    Lost,
    /// The feature that asked did not want the turn any more when its thread had started
    /// (`Feature::structured_wanted`): the turn never started.
    Unwanted,
}

/// The ephemeral thread to start (`start_temporary_thread`): what `thread/start` carries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Temporary {
    pub kind: Kind,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub cwd: Option<String>,
    /// The id of the permission profile in use (`:workspace`, a named profile).
    pub profile: Option<String>,
    /// The MCP servers the effective config holds: each is disabled for the thread.
    pub mcp_servers: Map<String, Value>,
}

impl Temporary {
    /// The permissions the thread asks for by name: the built-in read-only profile for a title,
    /// the active profile when it is a named one for a recap, none otherwise (the sandbox mode
    /// and the default permissions of the config say read-only then).
    pub(crate) fn permissions(&self) -> Option<String> {
        match self.kind {
            Kind::Title => Some(BUILT_IN_PERMISSION_PROFILE_READ_ONLY.to_owned()),
            Kind::Recap => self
                .profile
                .clone()
                .filter(|profile| !profile.starts_with(':')),
        }
    }

    /// The thread started as asked: a recap that named a profile has it active; every other
    /// thread has no entry that can write (`start_temporary_thread`).
    pub(crate) fn started_as_asked(&self, response: &ThreadStartResponse) -> bool {
        if self.kind == Kind::Recap
            && let Some(expected) = self.permissions()
        {
            return response
                .active_permission_profile
                .as_ref()
                .is_some_and(|profile| profile.id == expected);
        }
        let profile =
            PermissionProfile::from_legacy_sandbox_policy_template(&response.sandbox.to_core());
        matches!(profile, PermissionProfile::Managed { .. })
            && !profile
                .file_system_sandbox_policy()
                .entries
                .iter()
                .any(|entry| entry.access.can_write())
    }
}

/// The features a temporary thread has none of: no tools, no extensions, no network search.
pub(crate) fn off() -> HashMap<String, Value> {
    let mut config = HashMap::new();
    for feature in [
        "apps",
        "code_mode",
        "code_mode_only",
        "context_management",
        "current_time_reminder",
        "deferred_executor",
        "enable_fanout",
        "goals",
        "hooks",
        "image_generation",
        "memories",
        "multi_agent",
        "multi_agent_v2",
        "plugins",
        "request_permissions_tool",
        "shell_snapshot",
        "shell_tool",
        "standalone_web_search",
        "token_budget",
        "tool_suggest",
        "unified_exec",
        "view_image",
    ] {
        config.insert(["features.", feature].concat(), false.into());
    }
    config.insert("cloud.skills.enabled".to_owned(), false.into());
    config.insert("skills.include_instructions".to_owned(), false.into());
    config.insert(
        "tools.experimental_request_user_input.enabled".to_owned(),
        false.into(),
    );
    config.insert("tools.update_plan.enabled".to_owned(), false.into());
    config.insert("web_search".to_owned(), "disabled".into());
    config
}
