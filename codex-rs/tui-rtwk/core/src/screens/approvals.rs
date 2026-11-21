//! `approvals` (S13, approvals and requests): the core half of the screen's directory. The model
//! part, its events and intents, its RPCs and their protocol wiring live here,
//! with no rtwk; the view half is `src/sheets/approvals/` in the UI crate. Nothing
//! outside these two directories (bar their test files) needs an edit when it
//! grows.

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::ServerRequest;

use crate::app::AppFx;
use crate::ids::{RequestKey, ServerRequestId, ThreadId};
use crate::rpc::RpcError;
use crate::screens::Kind;
use crate::threads::ThreadFx;

pub mod form;
pub mod input;

pub use form::{Entered, Form};

pub const NAME: &str = "approvals";
pub const ID: &str = "S13";
pub const KIND: Kind = Kind::Sheet;

/// One question of a user-input request (`item/tool/requestUserInput`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Question {
    pub id: String,
    pub header: String,
    pub text: String,
    pub is_other: bool,
    /// The answer is a secret: the notes are drawn masked.
    pub is_secret: bool,
    pub options: Vec<QuestionOption>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuestionOption {
    pub label: String,
    pub description: String,
}

/// One part of the rule of a permissions request, as the oracle's
/// `format_additional_permissions_rule` lists them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Rule {
    Network,
    Read(Vec<String>),
    Write(Vec<String>),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExecPrompt {
    /// The thread the request came from when it is not the one shown (the
    /// oracle's `thread_label`).
    pub thread_label: Option<String>,
    pub reason: Option<String>,
    /// The command, shell wrapper stripped (`strip_bash_lc_and_escape`).
    pub command: String,
    pub environment: Option<String>,
    pub network_host: Option<String>,
    /// The rule of the additional permissions the command asks for.
    pub permission_rule: Vec<Rule>,
    /// What a `writeStdin` request sends to the terminal (shown as `Input:`
    /// instead of the command).
    pub input: Option<String>,
    pub decisions: Vec<Decision>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PatchPrompt {
    pub thread_label: Option<String>,
    pub reason: Option<String>,
    /// The paths the edit writes, sorted and unique (the oracle's
    /// `Destination:` lines); empty for the app-server request, which
    /// carries no file changes.
    pub destinations: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PermissionsPrompt {
    pub thread_label: Option<String>,
    pub reason: Option<String>,
    pub environment: Option<String>,
    pub rule: Vec<Rule>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ElicitationPrompt {
    pub thread_label: Option<String>,
    pub server: String,
    pub message: String,
    /// The form the request's schema asks (`Form::parse`); none: the oracle's plain approval
    /// (Server, message, and Accept / Decline / Cancel).
    pub form: Option<Form>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UserInputPrompt {
    pub questions: Vec<Question>,
}

/// What the user can decide, the oracle's `ApprovalDecision` of every kind of
/// request in one list (`bottom_pane/approval_overlay.rs`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decision {
    Accept,
    /// Accept and remember the command prefix (the rendered prefix).
    AcceptWithPrefix(String),
    AcceptForSession,
    /// Allow the network host in the future.
    AllowHost,
    Decline,
    Cancel,
    GrantForTurn,
    GrantForTurnStrict,
    GrantForSession,
    DenyPermissions,
    /// A persistent network policy rule for a host: allow or block it.
    NetworkRule { host: String, allow: bool },
}

/// Input from the request sheet. The app validates it against the pending request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RequestAnswer {
    Choice(Decision),
    UserInput(std::collections::BTreeMap<String, Vec<String>>),
    Elicitation {
        decision: Decision,
        content: Option<serde_json::Value>,
        meta: Option<serde_json::Value>,
    },
}

/// What a request asks, in the form the dialog needs; made from the
/// protocol's own request (`prompt_of`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Prompt {
    Exec(ExecPrompt),
    Patch(PatchPrompt),
    Permissions(PermissionsPrompt),
    Elicitation(ElicitationPrompt),
    UserInput(UserInputPrompt),
}

impl Prompt {
    /// The thread the request came from, when it is not the one shown.
    pub fn thread_label(&self) -> Option<&str> {
        match self {
            Prompt::Exec(p) => p.thread_label.as_deref(),
            Prompt::Patch(p) => p.thread_label.as_deref(),
            Prompt::Permissions(p) => p.thread_label.as_deref(),
            Prompt::Elicitation(p) => p.thread_label.as_deref(),
            Prompt::UserInput(_) => None,
        }
    }

    /// The prompt with the label of the thread it came from.
    pub fn with_thread_label(mut self, label: Option<String>) -> Prompt {
        match &mut self {
            Prompt::Exec(p) => p.thread_label = label,
            Prompt::Patch(p) => p.thread_label = label,
            Prompt::Permissions(p) => p.thread_label = label,
            Prompt::Elicitation(p) => p.thread_label = label,
            Prompt::UserInput(_) => {}
        }
        self
    }

    /// The decisions the dialog lists, in order.
    pub fn decisions(&self) -> Vec<Decision> {
        match self {
            Prompt::Exec(exec) => exec.decisions.clone(),
            Prompt::Patch(_) => vec![Decision::Accept, Decision::AcceptForSession, Decision::Cancel],
            Prompt::Permissions(_) => vec![
                Decision::GrantForTurn,
                Decision::GrantForTurnStrict,
                Decision::GrantForSession,
                Decision::DenyPermissions,
            ],
            Prompt::Elicitation(_) => vec![Decision::Accept, Decision::Decline, Decision::Cancel],
            // The questions are answered as a whole (`Intent::Respond`), not by a decision.
            Prompt::UserInput(_) => Vec::new(),
        }
    }
}

/// A presented request: the thread and request id key it as the queue does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub thread: ThreadId,
    pub id: ServerRequestId,
    pub prompt: Prompt,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Answered {
    pub thread: ThreadId,
    pub id: ServerRequestId,
    pub decision: Decision,
}

/// The app-scoped model state of this screen, changed only by `apply`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Model {
    /// What the queued requests ask (the queue itself is `CodexApp.requests`).
    pub entries: Vec<Entry>,
    /// Answers given; the effect that sends them is the app's (handover).
    pub answered: Vec<Answered>,
}

impl Model {
    pub fn prompt(&self, thread: &ThreadId, id: &ServerRequestId) -> Option<&Prompt> {
        self.entries
            .iter()
            .find(|entry| &entry.thread == thread && &entry.id == id)
            .map(|entry| &entry.prompt)
    }
}

/// The events of this screen (backend results, tickets); a variant of the
/// root `ScreenEv`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ev {
    /// A server request arrived; what it asks.
    Presented(Entry),
}

/// The intents of this screen; a variant of the root `ScreenIntent`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    Answer { key: RequestKey, decision: Decision },
    /// An answer with content (a form's values, a user-input answer): checked against the
    /// pending request like `Answer`; one that does not fit, or for a request no longer queued,
    /// is dropped.
    Respond { key: RequestKey, answer: RequestAnswer },
}

pub fn apply(model: &mut Model, ev: Ev, fx: &mut AppFx) {
    let _ = fx;
    match ev {
        Ev::Presented(entry) => {
            model
                .entries
                .retain(|e| !(e.thread == entry.thread && e.id == entry.id));
            model.entries.push(entry);
        }
    }
}

pub fn apply_intent(model: &mut Model, intent: Intent, fx: &mut AppFx) {
    let _ = fx;
    match intent {
        Intent::Answer { key, decision } => {
            let before = model.entries.len();
            model
                .entries
                .retain(|e| !(e.thread == key.thread && e.id == key.id));
            if model.entries.len() != before {
                model.answered.push(Answered {
                    thread: key.thread,
                    id: key.id,
                    decision,
                });
            }
        }
        Intent::Respond { key, answer } => {
            let before = model.entries.len();
            model
                .entries
                .retain(|e| !(e.thread == key.thread && e.id == key.id));
            if model.entries.len() != before {
                model.answered.push(Answered {
                    thread: key.thread,
                    id: key.id,
                    decision: match answer {
                        RequestAnswer::Choice(decision)
                        | RequestAnswer::Elicitation { decision, .. } => decision,
                        RequestAnswer::UserInput(_) => Decision::Accept,
                    },
                });
            }
        }
    }
}

/// Per-thread state of this screen: a part of every `ThreadModel`, changed
/// only by `apply_thread` and `apply_reply`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ThreadPart {}

/// An event of one thread for this screen; a variant of `ThreadEv::Screen`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThreadEv {}

/// The RPCs of this screen: requested with `ThreadFx::request`, which stamps
/// the thread and the ticket; the shell sends them through `wire`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {}

/// The replies of those RPCs, decoded by `decode`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {}

/// What an outgoing RPC of this screen was for, kept with its ticket.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ctx {}

pub fn apply_thread(part: &mut ThreadPart, ev: ThreadEv, fx: &mut ThreadFx<'_>) {
    let _ = (part, fx);
    match ev {}
}

pub fn apply_reply(
    part: &mut ThreadPart,
    ctx: Ctx,
    result: Result<Reply, RpcError>,
    fx: &mut ThreadFx<'_>,
) {
    let _ = (part, result, fx);
    match ctx {}
}

/// The wire request of `command` for `thread`.
pub fn wire(command: &Command, thread: &ThreadId, id: RequestId) -> ClientRequest {
    let _ = (thread, id);
    match *command {}
}

/// The reply `result` (the JSON result of the request) is for `command`.
pub fn decode(command: &Command, result: serde_json::Value) -> Result<Reply, RpcError> {
    let _ = result;
    match *command {}
}

/// The seed form of this screen's events: the protocol's own server request
/// (`{"kind":"request","request":{"method":..,"id":..,"params":..}}`).
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum SeedEv {
    /// `label` is the thread label of a request from a thread that is not
    /// shown, `destinations` the paths of a file-change request that has its
    /// changes (the app-server request has none): what the thread's own
    /// state gives the oracle's request.
    Request {
        request: ServerRequest,
        #[serde(default)]
        label: Option<String>,
        #[serde(default)]
        destinations: Vec<String>,
    },
}

/// A script's `screen` step for this directory.
pub fn seed(value: serde_json::Value) -> Result<Ev, crate::screens::SeedError> {
    let seed: SeedEv = serde_json::from_value(value)
        .map_err(|err| crate::screens::SeedError::Bad(err.to_string()))?;
    let SeedEv::Request {
        request,
        label,
        destinations,
    } = seed;
    let mut entry = prompt_of(&request).ok_or_else(|| {
        crate::screens::SeedError::Bad("not a request this screen presents".into())
    })?;
    if let Prompt::Patch(patch) = &mut entry.prompt {
        patch.destinations = destinations;
    }
    entry.prompt = entry.prompt.with_thread_label(label);
    Ok(Ev::Presented(entry))
}

/// The label of the thread a request came from, when it is not the thread
/// shown (the oracle's `App::thread_label` with
/// `format_agent_picker_item_name`: `nickname [role]`, and `Agent (suffix)`
/// for a thread with no nickname).
pub fn thread_label(
    threads: &[codex_app_server_protocol::Thread],
    thread: &ThreadId,
    active: Option<&ThreadId>,
) -> Option<String> {
    if active == Some(thread) {
        return None;
    }
    let known = threads.iter().find(|t| t.id == thread.as_str());
    let clean = |text: Option<&str>| text.map(str::trim).filter(|t| !t.is_empty()).map(String::from);
    let nickname = clean(known.and_then(|t| t.agent_nickname.as_deref()));
    let role = clean(known.and_then(|t| t.agent_role.as_deref()));
    let label = match (&nickname, &role) {
        (Some(nickname), Some(role)) => format!("{nickname} [{role}]"),
        (Some(nickname), None) => nickname.clone(),
        (None, Some(role)) => format!("[{role}]"),
        (None, None) => "Agent".to_string(),
    };
    Some(if nickname.is_none() {
        let id = thread.as_str();
        format!("{label} ({})", id.rsplit('-').next().unwrap_or(id))
    } else {
        label
    })
}

/// The presentation of a server request, read through its wire form (the
/// oracle's `exec_approval_request_from_params` and its siblings in
/// `chatwidget.rs`).
pub fn prompt_of(request: &ServerRequest) -> Option<Entry> {
    if matches!(request, ServerRequest::ToolRequestUserInput { params, .. } if params.questions.is_empty()) {
        return None;
    }
    let wire = serde_json::to_value(request).ok()?;
    let method = wire.get("method")?.as_str()?;
    let id = wire.get("id")?.to_string();
    let params = wire.get("params")?;
    let text = |key: &str| params.get(key).and_then(|v| v.as_str()).map(str::to_string);
    let thread = ThreadId::new(text("threadId").unwrap_or_default());
    let prompt = match method {
        "item/commandExecution/requestApproval" => Prompt::Exec(exec_prompt(params)),
        "item/fileChange/requestApproval" => Prompt::Patch(PatchPrompt {
            reason: text("reason"),
            ..PatchPrompt::default()
        }),
        "item/permissions/requestApproval" => Prompt::Permissions(PermissionsPrompt {
            thread_label: None,
            reason: text("reason"),
            environment: text("environmentId"),
            rule: rule_of(params.get("permissions")),
        }),
        "mcpServer/elicitation/request" => {
            if params.get("mode").and_then(|m| m.as_str()) != Some("form") {
                return None;
            }
            Prompt::Elicitation(ElicitationPrompt {
                thread_label: None,
                server: text("serverName").unwrap_or_default(),
                message: text("message").unwrap_or_default(),
                form: Form::parse(
                    params.get("_meta").filter(|meta| !meta.is_null()),
                    params.get("requestedSchema").unwrap_or(&serde_json::Value::Null),
                ),
            })
        }
        "item/tool/requestUserInput" => Prompt::UserInput(UserInputPrompt {
            questions: params
                .get("questions")
                .and_then(|q| q.as_array())
                .map(|questions| questions.iter().map(question_of).collect())
                .unwrap_or_default(),
        }),
        _ => return None,
    };
    Some(Entry {
        thread,
        id: ServerRequestId::new(id),
        prompt,
    })
}

fn exec_prompt(params: &serde_json::Value) -> ExecPrompt {
    let text = |key: &str| params.get(key).and_then(|v| v.as_str()).map(str::to_string);
    let network_host = params
        .get("networkApprovalContext")
        .and_then(|c| c.get("host"))
        .and_then(|h| h.as_str())
        .map(str::to_string);
    let amendment = params
        .get("proposedExecpolicyAmendment")
        .and_then(|a| a.as_array())
        .map(|words| {
            words
                .iter()
                .filter_map(|w| w.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        });
    let permission_rule = rule_of(params.get("additionalPermissions"));
    let has_permissions = params
        .get("additionalPermissions")
        .is_some_and(|p| !p.is_null());
    let network_allow = params
        .get("proposedNetworkPolicyAmendments")
        .and_then(|list| list.as_array())
        .and_then(|list| {
            list.iter()
                .find(|a| a.get("action").and_then(|v| v.as_str()) == Some("allow"))
        })
        .and_then(|a| a.get("host").and_then(|h| h.as_str()))
        .map(|host| Decision::NetworkRule {
            host: host.to_string(),
            allow: true,
        });
    // `ExecApprovalRequestEvent::effective_available_decisions`
    // (tui/src/approval_events.rs): the request's own list, else the default.
    let decisions = match params.get("availableDecisions").and_then(|d| d.as_array()) {
        Some(list) => list.iter().filter_map(decision_of).collect(),
        None if network_host.is_some() => {
            let mut decisions = vec![Decision::Accept, Decision::AcceptForSession];
            decisions.extend(network_allow);
            decisions.push(Decision::Cancel);
            decisions
        }
        None if has_permissions => vec![Decision::Accept, Decision::Cancel],
        None => {
            let mut decisions = vec![Decision::Accept];
            decisions.extend(amendment.map(Decision::AcceptWithPrefix));
            decisions.push(Decision::Cancel);
            decisions
        }
    };
    let write_stdin = params.get("kind").and_then(|k| k.as_str()) == Some("writeStdin");
    let command = text("command").unwrap_or_default();
    ExecPrompt {
        thread_label: None,
        reason: text("reason"),
        input: write_stdin.then(|| split_words(&command).pop().unwrap_or_default()),
        command: strip_shell_wrapper(&command),
        environment: text("environmentId"),
        network_host,
        permission_rule,
        decisions,
    }
}

fn decision_of(value: &serde_json::Value) -> Option<Decision> {
    if let Some(name) = value.as_str() {
        return match name {
            "accept" => Some(Decision::Accept),
            "acceptForSession" => Some(Decision::AcceptForSession),
            "decline" => Some(Decision::Decline),
            "cancel" => Some(Decision::Cancel),
            _ => None,
        };
    }
    if let Some(amendment) = value.get("applyNetworkPolicyAmendment").and_then(|a| {
        a.get("network_policy_amendment")
            .or_else(|| a.get("networkPolicyAmendment"))
    }) {
        return Some(Decision::NetworkRule {
            host: amendment.get("host")?.as_str()?.to_string(),
            allow: amendment.get("action")?.as_str()? == "allow",
        });
    }
    let prefix = value
        .get("acceptWithExecpolicyAmendment")?
        .get("execpolicy_amendment")
        .or_else(|| value.get("acceptWithExecpolicyAmendment")?.get("execpolicyAmendment"))?;
    let words = prefix
        .as_array()
        .or_else(|| prefix.get("command")?.as_array())?;
    Some(Decision::AcceptWithPrefix(
        words.iter().filter_map(|w| w.as_str()).collect::<Vec<_>>().join(" "),
    ))
}

fn rule_of(permissions: Option<&serde_json::Value>) -> Vec<Rule> {
    let Some(permissions) = permissions else {
        return Vec::new();
    };
    let mut rule = Vec::new();
    if permissions
        .get("network")
        .and_then(|n| n.get("enabled"))
        .and_then(serde_json::Value::as_bool)
        == Some(true)
    {
        rule.push(Rule::Network);
    }
    let paths = |key: &str| -> Vec<String> {
        permissions
            .get("fileSystem")
            .and_then(|fs| fs.get(key))
            .and_then(|list| list.as_array())
            .map(|list| list.iter().filter_map(|p| p.as_str()).map(str::to_string).collect())
            .unwrap_or_default()
    };
    let (read, write) = (paths("read"), paths("write"));
    if !read.is_empty() {
        rule.push(Rule::Read(read));
    }
    if !write.is_empty() {
        rule.push(Rule::Write(write));
    }
    rule
}

fn question_of(value: &serde_json::Value) -> Question {
    let text = |key: &str| {
        value
            .get(key)
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string()
    };
    Question {
        id: text("id"),
        header: text("header"),
        text: text("question"),
        is_other: value.get("isOther").and_then(serde_json::Value::as_bool).unwrap_or(false),
        is_secret: value.get("isSecret").and_then(serde_json::Value::as_bool).unwrap_or(false),
        options: value
            .get("options")
            .and_then(|o| o.as_array())
            .map(|options| {
                options
                    .iter()
                    .map(|option| QuestionOption {
                        label: option
                            .get("label")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string(),
                        description: option
                            .get("description")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string(),
                    })
                    .collect()
            })
            .unwrap_or_default(),
    }
}

/// `strip_bash_lc_and_escape` of the oracle (`tui/src/exec_command.rs:17`),
/// over a command string: `bash -lc '<script>'` is its script.
fn strip_shell_wrapper(command: &str) -> String {
    let words = split_words(command);
    if let [shell, flag, script] = words.as_slice()
        && matches!(flag.as_str(), "-lc" | "-c")
        && matches!(shell.rsplit('/').next(), Some("bash" | "zsh" | "sh"))
    {
        return script.clone();
    }
    command.to_string()
}

/// A minimal POSIX word split (quotes and backslashes), enough for
/// `bash -lc '<script>'`.
fn split_words(text: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut started = false;
    let mut quote: Option<char> = None;
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some('"'), '\\') => {
                if let Some(next) = chars.next() {
                    word.push(next);
                }
            }
            (Some(_), c) => word.push(c),
            (None, '\'' | '"') => {
                quote = Some(c);
                started = true;
            }
            (None, '\\') => {
                if let Some(next) = chars.next() {
                    word.push(next);
                    started = true;
                }
            }
            (None, c) if c.is_whitespace() => {
                if started {
                    words.push(std::mem::take(&mut word));
                    started = false;
                }
            }
            (None, c) => {
                word.push(c);
                started = true;
            }
        }
    }
    if started {
        words.push(word);
    }
    words
}
