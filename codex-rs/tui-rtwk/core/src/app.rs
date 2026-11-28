//! `CodexApp`, the core model of the app (ADR-003): plain data, one writer.
//! It holds no UI state (the UI crate's `UiApp` wraps it) and names no rtwk.
//!
//! Backend events, RPC replies and user intents are `AppEv` values that enter
//! one ordered ingress stream (R10); `apply_app` changes the model and returns
//! effects as data (`AppFx`); the shell runs them (R1). Nothing here touches
//! the network, the clock or the file system (R12).

mod banners;
mod clock;
mod connection;
mod consent;
pub mod effects;
pub mod facts;
pub(crate) mod features;
mod git_summary;
mod host_asks;
mod ide_submit;
mod lifecycle;
mod mentions;
mod notices;
mod reset;
mod safety_retry;
mod select;
mod session;
mod structured;
mod transcript_text;
mod verbs;
pub use clock::{Now, Wake};
pub use transcript_text::TranscriptText;

/// What a surface says of its interest (`Intent::Interest`): the widgets of the UI name it here,
/// not through `features::` (a shared widget is independent of the feature it serves).
pub use crate::features::{FeatureInterest, InterestStep};
pub mod worktree;

use session::{
    active_backend, card_limits, confirmed, refresh_rates, removed, session_intent, session_note,
};

use crate::threads::{ThreadAsk, ThreadReply};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::rc::Rc;

use codex_app_server_protocol::ErrorNotification;
use codex_app_server_protocol::ServerNotification;
use codex_app_server_protocol::ServerRequest;
use codex_app_server_protocol::ThreadItemPayload;

use crate::features::AppFacts;
use crate::ids::BackendId;
use crate::ids::ConnEpoch;
use crate::ids::RequestKey;
use crate::ids::ThreadId;
use crate::ids::Ticket;
use crate::ids::TicketMint;
use crate::ids::TurnId;
use crate::keymap::Keymap;
use crate::protocol::requests;
use crate::protocol::targets;
use crate::rpc::Pending;
use crate::rpc::Reply;
use crate::rpc::RpcError;
use crate::rpc::{Address, Answer};
use crate::screens;
use crate::screens::ScreenCommand;
use crate::screens::ScreenEv;
use crate::screens::ScreenIntent;
use crate::screens::ScreenReply;
use crate::screens::ScreenStates;
use crate::screens::ScreenThreadEv;
use crate::screens::ScreenThreadIntent;
use crate::screens::agents;
use crate::screens::approvals;
use crate::screens::chat;
use crate::screens::onboarding::TrustCancel;
use crate::screens::onboarding::consent::{self as folder_consent, Target as ConsentTarget};
use crate::screens::resume;
use crate::screens::status;
use crate::threads;
use crate::threads::Failure;
use crate::threads::Origin;
use crate::threads::SubmitReceipt;
use crate::threads::ThreadEv;
use crate::threads::ThreadFx;
use crate::threads::ThreadModel;
use crate::threads::ThreadSet;
use crate::threads::TurnState;
use crate::threads::UserMessage;

/// Where the connection to the app-server stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Session {
    /// The client is starting or the first thread is being created; the
    /// composer is usable as a draft, a submit waits for `Ready`.
    Starting,
    Ready,
    Failed(Failure),
}

/// The session projection provided to the tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionSlice {
    pub status: Session,
    /// The command line names a session to resume or fork and it is not open yet: the startup
    /// draft says `Resuming session…` / `Forking session…`.
    pub launching: Option<crate::screens::startup::Action>,
    /// The first backend's connection epoch (`BackendId::default()`); the
    /// other backends' are `epochs`.
    pub epoch: ConnEpoch,
    /// The epoch of each further backend's connection: one per backend, so
    /// a reconnect of one never drops the other's events.
    pub epochs: BTreeMap<BackendId, ConnEpoch>,
    pub active: Option<ThreadId>,
    /// How many times an empty conversation moved to another backend, and
    /// the thread it left: the composer's draft goes with it.
    pub switches: u32,
    pub switched_from: Option<ThreadId>,
    /// The empty conversation is on its way to a Cloud backend (oracle `CloudSwitchProgress`):
    /// the draft stays editable and nothing is submitted. `Failed` is the apply that heard the
    /// refusal: the next one (or the wake it declares) takes it away, as the oracle's failure
    /// frame lasts until the error is on screen.
    pub cloud_switch: Option<CloudSwitch>,
    /// How many times the thread of the chat was archived or deleted: the
    /// chat is empty again and the command center takes its place
    /// (`return_to_agents_after_removal`).
    pub removed: u32,
    /// How many forks of each thread the server refused (`/fork` on a thread another process
    /// writes): the page of that thread, which waits on its copy, stops waiting; another thread's
    /// page does not hear of it.
    pub fork_failed: BTreeMap<ThreadId, u32>,
    pub rate_limits: BTreeMap<BackendId, status::RateLimits>,
    /// Every bucket the full reads of each backend left, and its reset credits.
    pub rate_snapshots: BTreeMap<BackendId, status::RateLimitStore>,
    pub connections: BTreeMap<BackendId, ConnectionState>,
    pub backend_notices: BTreeMap<BackendId, String>,
    pub backend_banners: BTreeMap<BackendId, chat::BannerOccurrence>,
    /// What the last account read said of the Luna Reserve, per backend.
    pub reserve: BTreeMap<BackendId, chat::Reserve>,
}

/// Where the empty conversation's move to a Cloud backend stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloudSwitch {
    /// The start is asked of Cloud and not answered.
    Connecting,
    /// Cloud refused the start; the draft stays where it was.
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectionState {
    Connected,
    Reconnecting,
    Disconnected,
}

/// The name of a backend as the control shows it (oracle:
/// `BackendSessions::displayed_name`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackendLabel {
    Local,
    Cloud,
    /// The n-th connection, counting from 1.
    Connection(u32),
}

/// A request waiting for the user: approvals, elicitations, user input. The
/// model's queue is the only source the presenter opens dialogs from.
#[derive(Clone, Debug, PartialEq)]
pub struct PendingRequest {
    pub key: RequestKey,
    /// The wire method, e.g. `item/commandExecution/requestApproval`.
    pub method: &'static str,
    pub preview: Option<String>,
    request: Rc<ServerRequest>,
    pub prompt: Option<Rc<approvals::Prompt>>,
}

/// The queue of requests the user answers, oldest first.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RequestQueue {
    pub entries: Vec<PendingRequest>,
}

impl RequestQueue {
    pub fn head(&self) -> Option<&PendingRequest> {
        self.entries.first()
    }

    pub fn get(&self, key: &RequestKey) -> Option<&PendingRequest> {
        self.entries.iter().find(|entry| &entry.key == key)
    }

    pub fn contains(&self, key: &RequestKey) -> bool {
        self.get(key).is_some()
    }

    pub fn preview(&self, thread: &str) -> Option<&str> {
        self.entries
            .iter()
            .filter(|entry| entry.key.thread.as_str() == thread)
            .find_map(|entry| entry.preview.as_deref())
    }
}

/// An event from the backend, as the shell received it.
#[derive(Clone, Debug)]
pub enum BackendEv {
    Notification(Box<ServerNotification>),
    Request(Box<ServerRequest>),
    Lagged(usize),
    /// The event stream of `thread` was lost while the connection stands (a Cloud backend's
    /// `ThreadStreamDisconnected`, `error` its words): what the user sends there waits in the
    /// queue (oracle `thread_stream_disconnected`), an idle displayed thread asks the host to
    /// attach again (`effects::Ask::ReconnectIdleThread`), and the error shows as a line otherwise.
    StreamLost {
        thread: ThreadId,
        error: Box<ErrorNotification>,
    },
    /// The event stream of `thread` is back (`ThreadStreamReconnected`).
    StreamRestored(ThreadId),
}

/// What the user asks for. Intents that touch backend-shared or
/// cross-thread state are values sent into the ingress (R10).
#[derive(Clone, Debug)]
pub enum Intent {
    /// Send `text` to `thread`; `None` is the startup draft, which waits for
    /// the session to be `Ready`.
    Submit {
        thread: Option<ThreadId>,
        message: UserMessage,
    },
    /// Reject an unanswered request.
    DismissRequest(RequestKey),
    DismissBackendBanner {
        backend: BackendId,
        ticket: Ticket,
    },
    /// The user chose row `row` of the banner `ticket` shows (a number key or
    /// a tap): what the action does is decided here, not by the view.
    PickBannerCta {
        backend: BackendId,
        ticket: Ticket,
        row: usize,
    },
    AnswerRequest {
        key: RequestKey,
        answer: approvals::RequestAnswer,
    },
    Interrupt(ThreadId),
    ThreadScreen {
        thread: ThreadId,
        intent: ScreenThreadIntent,
    },
    /// Move the empty conversation to the next connected backend (oracle:
    /// `CycleNewConversationBackend`).
    CycleBackend,
    /// Show `ThreadId` in the chat: the one door every screen uses to open a
    /// thread (ADR-003). A thread this process holds is shown again as it
    /// is; any other is resumed once, by `apply_app` alone.
    OpenThread(ThreadId),
    LoadOlderHistory(ThreadId),
    /// An ask the host runs outside the thread RPCs (the clipboard, the git
    /// diff, the title generator, the export, the config file), answered
    /// with `AppEv::Effect` under `request`; read it with `CodexApp::effect`.
    Effect {
        request: u64,
        ask: effects::Ask,
    },
    /// ctrl+c: the queued requests are dropped unanswered (rejected), the
    /// one on screen included (oracle `ChatWidget::on_ctrl_c` with approvals).
    CancelRequests,
    /// `/auto-review` on a thread: the list found nothing, or one denial was picked.
    AutoReview {
        thread: ThreadId,
        what: AutoReview,
    },
    Screen(ScreenIntent),
    /// An intent of a feature slice (ADR-012).
    Feature(crate::features::FeatureIntent),
    /// A surface wants, or lets go of, the data a feature shows (ADR-013 И): made only by the
    /// `Interest` widget.
    Interest(crate::features::InterestStep),
    /// The surface presented the wish `seq` (ADR-013 Ж): made only by the presenter.
    WishShown(u64),
}

/// What `/auto-review` does to a thread (oracle `open_auto_review_denials_popup`,
/// `approve_recent_auto_review_denial`): the denials kept are the thread's own
/// (`screen_slots().approvals.recent_denials`), the view opens the list and says which was
/// picked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AutoReview {
    /// The thread has no denial to list.
    NoDenials,
    /// The user picked the denial with this review id: one retry is approved.
    Approve { id: String },
}

/// Everything that reaches `apply_app`, through one ordered stream.
#[derive(Clone, Debug)]
pub enum AppEv {
    /// The backend client is up.
    Connected,
    /// The client did not start; the client's message.
    StartFailed(String),
    /// The backend stopped.
    Disconnected,
    /// The host is going to quit (`ExitMode::ShutdownFirst`): what a feature holds on the server
    /// is let go (`Feature::on_connection`).
    ShuttingDown,
    Backend(BackendEv),
    Reply(Ticket, Reply),
    ServerAnswerFailed {
        key: RequestKey,
        message: String,
    },
    Intent(Intent),
    Screen(ScreenEv),
    /// The host's answer to an `Intent::Effect`.
    Effect {
        request: u64,
        done: effects::Done,
    },
    /// The device's result of a user verification attempt
    /// (`effects::Ask::VerifyUser`).
    Verified {
        key: RequestKey,
        attempt: effects::Attempt,
        result: effects::Verified,
    },
    /// The keymap the config makes (`tui.keymap.*`), sent by the host once
    /// the config is read; until then the built-in defaults hold.
    Keymap(Box<Keymap>),
    /// A notice for the open conversation (a newer release, the server's other version): it
    /// goes into the history of the shown thread, or of the first one when none is open yet.
    Announce(threads::Notice),
    /// What the host knows about Daybreak (the account, the model catalog): it words the
    /// notice of a cyber refusal.
    Daybreak(threads::daybreak::Access),
    /// The wall clock, as the shell read it: at start, and at the moment the app asked to be
    /// woken (`AppFx::wake`: the nearest of what the features' and threads' `wake_at` and the
    /// screens that word ages declared). The app keeps it (`CodexApp::now`), tells the screens
    /// and delivers the wakes that are due; a script says it with its `clock` step.
    Clock(Now),
    /// The `[tui]` settings the config holds (the host sends them once the config is read, and
    /// again after a setting changed).
    Tui(facts::TuiSettings),
    /// The presets of the config (`tui.presets`) with the layer each comes from: sent with
    /// `Tui`, and never changed by anything but the saved write of a preset.
    Presets(Vec<crate::screens::select::quick_dials::Slot>),
    /// A fact the host sends unasked to a feature slice (ADR-012: `Feature::on_fact`); the
    /// host builds it with the feature's own `Fact` type and `From`.
    Feature(crate::features::FeatureFact),
    /// An error the host words for the shown conversation: the browser did not open a page
    /// (`effects::Ask::OpenUrl`, oracle `Failed to open browser for {url}: {err}`), the clipboard has
    /// no image to paste (`Failed to paste image: {err}`). The conversation keeps it as an error
    /// row; the words are the host's, core holds no prose.
    ErrorLine {
        message: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalSideStorage {
    Embedded,
    Daemon,
}

/// An event with the connection epoch it was attached under, and the
/// backend it came from (each backend has its own epoch).
#[derive(Clone, Debug)]
pub struct Ingress {
    pub backend: BackendId,
    pub epoch: ConnEpoch,
    transport_epoch: Option<ConnEpoch>,
    host_failure: Option<Ticket>,
    local_side_storage: Option<LocalSideStorage>,
    /// Without a host fact an empty event announces a replacement handle at `transport_epoch`.
    ev: Option<AppEv>,
}

impl Ingress {
    /// An event of the first backend.
    pub fn new(epoch: ConnEpoch, ev: AppEv) -> Self {
        Self::of(BackendId::default(), epoch, ev)
    }

    /// An event of `backend`.
    pub fn of(backend: BackendId, epoch: ConnEpoch, ev: AppEv) -> Self {
        Self {
            backend,
            epoch,
            transport_epoch: None,
            host_failure: None,
            local_side_storage: None,
            ev: Some(ev),
        }
    }

    /// A reply from the handle selected by a connector that may replace its transport.
    pub fn transport_reply(
        backend: BackendId,
        epoch: ConnEpoch,
        transport_epoch: ConnEpoch,
        ticket: Ticket,
        reply: Reply,
    ) -> Self {
        Self::transport_event(backend, epoch, transport_epoch, AppEv::Reply(ticket, reply))
    }

    /// An outcome of work executed against this exact request handle.
    pub fn transport_event(
        backend: BackendId,
        epoch: ConnEpoch,
        transport_epoch: ConnEpoch,
        ev: AppEv,
    ) -> Self {
        Self {
            backend,
            epoch,
            transport_epoch: Some(transport_epoch),
            host_failure: None,
            local_side_storage: None,
            ev: Some(ev),
        }
    }

    /// The selected in-process publisher orders unkeyed captions within this physical transport.
    pub fn legacy_voice_publisher(epoch: ConnEpoch, transport_epoch: ConnEpoch) -> Self {
        let backend = BackendId::default();
        let address = Address {
            backend,
            epoch: transport_epoch,
        };
        Self::transport_event(
            backend,
            epoch,
            transport_epoch,
            AppEv::Screen(screens::voice::Ev::OrderedLegacyPublisher(address).into()),
        )
    }

    /// The live request handle can potentially store attached side conversations locally.
    pub fn local_side_storage(
        epoch: ConnEpoch,
        transport_epoch: ConnEpoch,
        storage: LocalSideStorage,
    ) -> Self {
        Self {
            backend: BackendId::default(),
            epoch,
            transport_epoch: Some(transport_epoch),
            host_failure: None,
            local_side_storage: Some(storage),
            ev: None,
        }
    }

    /// A startup ask ended fatally. Its owner must still hold the request before it can end the app.
    pub fn host_failure(
        epoch: ConnEpoch,
        transport_epoch: Option<ConnEpoch>,
        ticket: Ticket,
        message: String,
    ) -> Self {
        Self {
            backend: BackendId::default(),
            epoch,
            transport_epoch,
            host_failure: Some(ticket),
            local_side_storage: None,
            ev: Some(AppEv::StartFailed(message)),
        }
    }

    /// The connector installed a request handle without replacing its ingress.
    pub fn transport_replaced(
        backend: BackendId,
        epoch: ConnEpoch,
        transport_epoch: ConnEpoch,
    ) -> Self {
        Self {
            backend,
            epoch,
            transport_epoch: Some(transport_epoch),
            host_failure: None,
            local_side_storage: None,
            ev: None,
        }
    }

    pub fn event(&self) -> Option<&AppEv> {
        self.ev.as_ref()
    }
}

/// An RPC or reply the shell performs. Thread RPCs are built through
/// `ThreadFx`, which stamps the thread id (R3).
#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    /// `model`: the model an accepted migration prompt chose, for the thread
    /// to start on (oracle: `config.model` set before `start_thread`); none
    /// leaves the host's own choice (`--model`, the config).
    #[non_exhaustive]
    StartThread {
        ticket: Ticket,
        backend: BackendId,
        model: Option<String>,
        cwd: Option<String>,
    },
    #[non_exhaustive]
    ReadRateLimits { ticket: Ticket, backend: BackendId },
    /// The backend e-mails the owner of the workspace (`account/sendAddCreditsNudgeEmail`; the
    /// banner's notify-owner action).
    #[non_exhaustive]
    SendAddCreditsNudge {
        ticket: Ticket,
        backend: BackendId,
        nudge: chat::Nudge,
    },
    #[non_exhaustive]
    StartTurn {
        ticket: Ticket,
        thread: ThreadId,
        message: UserMessage,
        client_id: String,
        /// The acknowledgement of a misalignment failure, epoch milliseconds: the request
        /// carries the `misalignment_override` metadata (`threads/safety.rs`).
        misalignment_override: Option<i64>,
        /// Fresh opt-in IDE data for wire serialization.
        ide_prompt: Option<effects::IdePrompt>,
    },
    #[non_exhaustive]
    RejectRequest { key: RequestKey },
    #[non_exhaustive]
    ResolveRequest {
        key: RequestKey,
        result: serde_json::Value,
    },
    #[non_exhaustive]
    InterruptTurn {
        ticket: Ticket,
        thread: ThreadId,
        turn: TurnId,
    },
    /// A capability run by the host. Some capabilities answer their own typed
    /// continuation; an ordinary effect reports `AppEv::Effect` under `request`.
    #[non_exhaustive]
    Effect { request: u64, ask: effects::Ask },
    /// An RPC of a screen slot (`ThreadFx::request`).
    #[non_exhaustive]
    Screen {
        ticket: Ticket,
        thread: ThreadId,
        command: ScreenCommand,
    },
    /// An RPC of a feature slice (`AppCx::ask`, `ThreadCx::request`): `thread` is empty for
    /// an ask about the app.
    #[non_exhaustive]
    Feature {
        ticket: Ticket,
        thread: ThreadId,
        command: crate::features::FeatureCommand,
        preparation_checkpoint: Option<PreparationCheckpoint>,
        attached_history: Option<(LocalSideStorage, AttachedHistory)>,
    },
}

/// A local attached-history operation carried by an ordinary feature ticket.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AttachedHistory {
    Discover,
    Verify { child: ThreadId },
    Archive { child: ThreadId },
}

/// A row whose exact attached relationship and checkpoint were read by the local host.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SavedAttachment {
    #[serde(with = "AttachmentId")]
    pub id: ThreadId,
    pub name: Option<String>,
    pub preview: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "ThreadId")]
struct AttachmentId(String);

/// The local rollout that must confirm a persistent fork's context before admission.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparationCheckpoint {
    pub parent: ThreadId,
    pub rollout: Option<std::path::PathBuf>,
}

impl Command {
    /// A host capability with its own continuation, or with no answer.
    pub fn host(ask: effects::Ask) -> Self {
        Self::Effect { request: 0, ask }
    }

    pub fn host_ask(&self) -> Option<&effects::Ask> {
        match self {
            Self::Effect { ask, .. } => Some(ask),
            _ => None,
        }
    }

    /// The identity a native host attaches to a fatal outcome of one startup ask.
    pub fn host_ticket(&self) -> Option<Ticket> {
        host_asks::ReplyKind::of_command(self).map(|(ticket, _)| ticket)
    }

    fn ticket(&self) -> Option<Ticket> {
        match self {
            Self::StartThread { ticket, .. }
            | Self::ReadRateLimits { ticket, .. }
            | Self::SendAddCreditsNudge { ticket, .. }
            | Self::StartTurn { ticket, .. }
            | Self::InterruptTurn { ticket, .. }
            | Self::Effect {
                ask:
                    effects::Ask::CreateWorktree { ticket, .. }
                    | effects::Ask::BindWorktree { ticket, .. },
                ..
            }
            | Self::Screen { ticket, .. }
            | Self::Feature { ticket, .. } => Some(*ticket),
            _ => None,
        }
    }
}

/// Effects of one `apply_app` call, as values.
#[derive(Debug, Default, PartialEq)]
pub struct AppFx {
    pub(crate) commands: Vec<Command>,
    /// The app loans its mint while a host-owning screen is reducing.
    host_mint: TicketMint,
    /// Only commands that expect no reply use a dispatch address in the effects themselves.
    tells: BTreeMap<Ticket, Address>,
    /// The app ends: the connection did not start (the oracle ends with the error,
    /// `run_main_inner` returning it), or a feature ended it (`AppCx::end_app`).
    pub(crate) ends: bool,
    pub(crate) desktop_notice: Option<chat::DesktopNotice>,
    /// The nearest moment the app wants the clock told (`AppEv::Clock`), as of the end of the
    /// step: the shell keeps one timer for it.
    pub(crate) wake: Option<Wake>,
}

impl AppFx {
    pub(crate) fn host_request_number(&mut self) -> u64 {
        self.host_mint.next().seq()
    }

    pub(crate) fn host(&mut self, ask: effects::Ask) {
        self.commands.push(Command::host(ask));
    }

    fn ask(
        &mut self,
        app: &mut CodexApp,
        context: AppCtx,
        build: impl FnOnce(&CodexApp, Ticket) -> Command,
    ) -> Ticket {
        let ticket = app.mint.next();
        let command = build(app, ticket);
        debug_assert_eq!(command.ticket(), Some(ticket));
        let backend = match &context {
            AppCtx::Opening {
                attached_to: Some(parent),
                ..
            } => app.screens.agents.owner_of(parent.as_str()),
            AppCtx::Worktree { backend, .. }
            | AppCtx::OpenThread { backend, .. }
            | AppCtx::NewConversation { backend, .. }
            | AppCtx::Redemption { backend, .. }
            | AppCtx::Select(
                screens::select::Ctx::Fallback { backend }
                | screens::select::Ctx::Switch { backend, .. },
            ) => *backend,
            AppCtx::TurnOperation(
                TurnOperation::Structured { backend, .. }
                | TurnOperation::SafetyRetry { backend, .. },
            ) => *backend,
            AppCtx::Resume(context) => context.backend(),
            AppCtx::SessionRead | AppCtx::OpenSession { .. } => BackendId::default(),
            AppCtx::Select(
                screens::select::Ctx::Requirements { .. } | screens::select::Ctx::Profiles { .. },
            ) => app
                .screens
                .select
                .profiles
                .address
                .map_or_else(|| app.choose_backend(&command), |address| address.backend),
            AppCtx::StartThread { .. }
            | AppCtx::Agents(_)
            | AppCtx::Select(_)
            | AppCtx::RateLimits { .. }
            | AppCtx::Nudge { .. }
            | AppCtx::Removal { .. }
            | AppCtx::Opening { .. }
            | AppCtx::Feature(_) => app.choose_backend(&command),
        };
        app.outgoing.insert(ticket, app.address(backend), context);
        self.commands.push(command);
        ticket
    }

    fn tell(
        &mut self,
        app: &mut CodexApp,
        backend: BackendId,
        build: impl FnOnce(Ticket) -> Command,
    ) {
        let ticket = app.mint.next();
        let command = build(ticket);
        debug_assert_eq!(command.ticket(), Some(ticket));
        self.tells.insert(ticket, app.address(backend));
        self.commands.push(command);
    }

    /// When the app wants to be told the time next (none: it waits for an event).
    pub fn wake(&self) -> Option<Wake> {
        self.wake
    }

    pub fn take_desktop_notice(&mut self) -> Option<chat::DesktopNotice> {
        self.desktop_notice.take()
    }

    /// The commands, taken: what the shell runs.
    pub fn into_commands(self) -> Vec<Command> {
        self.commands
    }

    /// The command and, for a fire-and-forget RPC, its already chosen address.
    pub fn into_dispatch(self) -> impl Iterator<Item = (Command, Option<Address>)> {
        let Self {
            commands, tells, ..
        } = self;
        commands.into_iter().map(move |command| {
            let address = command
                .ticket()
                .and_then(|ticket| tells.get(&ticket).copied());
            (command, address)
        })
    }

    /// Whether the app ends: its connection did not start, or a feature ended it.
    pub fn ends_app(&self) -> bool {
        self.ends
    }

    /// The commands requested, in order.
    pub fn commands(&self) -> &[Command] {
        &self.commands
    }
}

/// What an outgoing app-scoped RPC was for.
#[derive(Clone, Debug, PartialEq, Eq)]
enum AppCtx {
    /// The first thread of a connection, or one a cycle started on `backend`
    /// (`None`: the first).
    StartThread { switch: Option<BackendId> },
    /// An RPC of an app-scoped screen (the agents command center).
    Agents(agents::Ctx),
    /// An RPC of the resume picker (its pages, previews, archive).
    Resume(resume::Ctx),
    /// `model/list` and the permission profiles for the pickers of S14.
    Select(crate::screens::select::Ctx),
    /// A read of the account's limits and why it was asked.
    RateLimits {
        backend: BackendId,
        origin: status::RateLimitOrigin,
        superseded: bool,
    },
    /// A reset redemption, correlated by its account's idempotency key.
    Redemption { key: String, backend: BackendId },
    /// The e-mail to the workspace owner a banner action asked for; the notice goes to the
    /// thread that was open (`add_credits_nudge_email_in_flight`).
    Nudge {
        backend: BackendId,
        nudge: chat::Nudge,
        thread: Option<ThreadId>,
    },
    NewConversation {
        source: Option<ThreadId>,
        backend: BackendId,
        /// What the new thread is called once it has started (`/new <name>`: `name_started`).
        name: Option<String>,
        /// The approved plan the new thread starts from (the plan prompt's "clear context" row).
        plan: Option<String>,
        /// A text for the new thread's composer (the provider-error notice's continue key).
        draft: Option<String>,
        /// A feature asked for the thread (`AppCx::spawn`, `app/lifecycle.rs`): it is told when
        /// the thread is open here, and when it could not be made.
        spawned: Option<lifecycle::Spawned>,
    },
    /// `thread/archive` or `thread/delete` of the thread the chat shows
    /// (`/archive`, `/delete` after their confirmation).
    Removal {
        thread: ThreadId,
        action: agents::Action,
    },
    OpenThread {
        thread: ThreadId,
        backend: BackendId,
        cancelled: bool,
    },
    /// `thread/read` of a thread a feature asked to open (`AppCtx::OpenThread` takes over once
    /// the server has it): the feature is told with `ctx` (`app/lifecycle.rs`).
    Opening {
        thread: ThreadId,
        ctx: crate::features::FeatureCtx,
        attached_to: Option<ThreadId>,
    },
    Worktree {
        backend: BackendId,
        step: worktree::Step,
    },
    /// `thread/read` of the session `codex resume <id>` names (the launch
    /// goes on with the directory it recorded).
    SessionRead,
    /// The resume or fork of that session, in the directory the launch
    /// decided.
    OpenSession {
        action: crate::screens::startup::Action,
        thread: ThreadId,
    },
    /// An RPC owned by an app-scoped operation on a turn.
    TurnOperation(TurnOperation),
    /// An RPC of a feature slice about the app.
    Feature(crate::features::FeatureCtx),
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum TurnOperation {
    Structured {
        run: u64,
        backend: BackendId,
    },
    SafetyRetry {
        generation: Ticket,
        backend: BackendId,
        attaching: bool,
    },
}

/// An ask made before the first thread exists.
enum BeforeThread {
    Message(UserMessage),
    /// A user shell command (`!cmd`).
    Shell(String),
    /// The prompt of the command line: sent as any message, then kept for the composer's recall
    /// and the history file (`submit_initial_user_message_if_pending`).
    Initial(UserMessage),
    /// `/plan <prompt>`: the switch to Plan mode and the prompt, made when the session starts.
    Plan {
        command: crate::commands::SlashCommand,
        prompt: String,
    },
}

impl BeforeThread {
    /// The text the user wrote for the ask.
    fn text(&self) -> String {
        match self {
            BeforeThread::Message(message) | BeforeThread::Initial(message) => message.text.clone(),
            BeforeThread::Shell(command) => format!("!{command}"),
            BeforeThread::Plan { command, prompt } => format!("/{} {prompt}", command.command()),
        }
    }

    /// The row the queue draws for the ask while it waits (the oracle's queued input).
    fn queued(&self) -> threads::Queued {
        match self {
            BeforeThread::Message(message) | BeforeThread::Initial(message) => {
                threads::Queued::Message(message.clone())
            }
            BeforeThread::Shell(command) => {
                threads::Queued::Shell(UserMessage::text(format!("!{command}")))
            }
            BeforeThread::Plan { command, prompt } => threads::Queued::Command(UserMessage::text(
                format!("/{} {prompt}", command.command()),
            )),
        }
    }
}

/// Keeps `ask` until the first thread exists; the blank thread's queue shows it meanwhile.
fn queue_before_thread(app: &mut CodexApp, ask: BeforeThread, fx: &mut AppFx) {
    let queued = ask.queued();
    app.queued_submits.push(ask);
    threads::apply(
        app.threads.blank_mut(),
        ThreadEv::Deferred(threads::Deferred::Submit(queued)),
        &mut ThreadFx::new(&ThreadId::default(), fx, &mut app.mint),
    );
}

struct AttachEvent {
    backend: BackendId,
    thread: ThreadId,
    event: ThreadEv,
    permissions: Option<screens::select::permissions::Effective>,
}

/// The model.
pub struct CodexApp {
    session: Rc<SessionSlice>,
    /// The models of the threads (ADR-003): made only by `ThreadSet::open`.
    threads: ThreadSet,
    requests: Rc<RequestQueue>,
    request_epochs: BTreeMap<BackendId, ConnEpoch>,
    /// Request handles replaced in-place, independent of the connector and approval epochs.
    transport_epochs: BTreeMap<BackendId, ConnEpoch>,
    local_side_storage: Option<(Address, LocalSideStorage)>,
    preparing_forks: BTreeMap<ThreadId, lifecycle::Preparation>,
    screens: Rc<ScreenStates>,
    /// The app-scoped state of the feature slices (ADR-012), read through `features()`,
    /// changed only by the features' own steps (`app/features.rs`).
    features: Rc<crate::features::FeatureStates>,
    /// What the surfaces want loaded, by feature (ADR-013 И): counted, so the first and the last
    /// reach the feature, and asked again after a reconnect.
    interests: Rc<crate::features::Interests>,
    /// What the features want the surface to show, once (ADR-013 Ж).
    wishes: Rc<crate::features::Wishes>,
    mint: TicketMint,
    outgoing: Pending<AppCtx>,
    host_outgoing: Pending<host_asks::ReplyKind>,
    host_transport_lost: bool,
    /// The structured turns the features asked for and have no answer to yet.
    structured: structured::Runs,
    safety_retry: Option<safety_retry::Run>,
    /// What was asked before the first thread exists, in order: it goes out
    /// when the thread starts (oracle: input queued until the session is
    /// configured).
    queued_submits: Vec<BeforeThread>,
    /// Notifications may be delivered before the reply to the operation attaching the writer.
    attaching_events: VecDeque<AttachEvent>,
    /// What the spawns in progress asked for (thread, call id): the row of the completed call
    /// shows it when the completed item has none.
    spawn_requests: std::collections::HashMap<(String, String), threads::SpawnRequest>,
    /// When the first thread starts.
    first_thread: FirstThread,
    /// The first thread's start waits for the startup steps to end
    /// (`FirstThread::AfterLaunch`).
    start_held: bool,
    consent: Option<consent::Pending>,
    /// The key bindings every screen reads (`AppEv::Keymap`); one value, so a
    /// widget that holds a binding hint can never hold a stale one.
    keymap: Rc<Keymap>,
    /// The `[tui]` settings views read (`AppEv::Tui`).
    tui: facts::TuiSettings,
    /// The answers of the host's asks and the user-verification controller.
    effects: effects::Effects,
    ide_submissions: ide_submit::State,
    /// Notices announced before any conversation exists.
    pending_notices: Vec<threads::Notice>,
    /// The Daybreak facts (`AppEv::Daybreak`).
    daybreak: threads::daybreak::Access,
    /// The moment the shell last told (`AppEv::Clock`).
    now: Now,
    /// What renders a conversation as the Markdown of `/export` (the cells' text is the UI's).
    transcript_text: Option<TranscriptText>,
    /// The server's collaboration-mode presets (`collaborationMode/list`, read at connect):
    /// empty until the server has answered, and when it did not.
    modes: Vec<crate::screens::select::collaboration::Mask>,
    /// Where each thread returns to when Luna Reserve ends (`ReserveReturnModel`).
    reserve_returns: BTreeMap<ThreadId, banners::ReserveReturn>,
    /// Models rejecting images in the last catalog read on each connection.
    image_restrictions: BTreeMap<BackendId, ImageRestrictions>,
}

struct ImageRestrictions {
    address: Address,
    observed: Ticket,
    models: BTreeSet<String>,
}

/// When the process's first thread starts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FirstThread {
    /// On connect: no startup screen runs (tests, a host that does not
    /// answer the startup facts).
    #[default]
    OnConnect,
    /// The host answers the startup facts (`Ask::Facts`): the thread starts
    /// once the startup steps and the launch prompts (GovCloud, hooks review,
    /// model migration) are answered (oracle: `App::run` starts the session
    /// after them, with the model an accepted migration chose). Not on
    /// connect: the server's own sign-in (the browser login) would pre-empt
    /// the device-code screen.
    AfterLaunch,
    /// A page opens instead (`codex agents`, `codex resume`): a thread
    /// starts when the user asks for one (`Intent::StartFirstThread`, a task,
    /// the picker's cancel).
    Never,
}

impl Default for CodexApp {
    fn default() -> Self {
        Self::new()
    }
}

impl CodexApp {
    pub fn new() -> Self {
        Self {
            session: Rc::new(SessionSlice {
                status: Session::Starting,
                launching: None,
                epoch: ConnEpoch::default(),
                epochs: BTreeMap::new(),
                active: None,
                switches: 0,
                switched_from: None,
                cloud_switch: None,
                removed: 0,
                fork_failed: BTreeMap::new(),
                rate_limits: BTreeMap::new(),
                rate_snapshots: BTreeMap::new(),
                connections: BTreeMap::new(),
                backend_notices: BTreeMap::new(),
                backend_banners: BTreeMap::new(),
                reserve: BTreeMap::new(),
            }),
            threads: ThreadSet::default(),
            requests: Rc::new(RequestQueue::default()),
            request_epochs: BTreeMap::new(),
            transport_epochs: BTreeMap::new(),
            local_side_storage: None,
            preparing_forks: BTreeMap::new(),
            screens: Rc::new(ScreenStates::default()),
            features: Rc::new(crate::features::FeatureStates::default()),
            interests: Rc::new(crate::features::Interests::default()),
            wishes: Rc::new(crate::features::Wishes::default()),
            mint: TicketMint::default(),
            outgoing: Pending::default(),
            host_outgoing: Pending::default(),
            host_transport_lost: false,
            structured: structured::Runs::default(),
            safety_retry: None,
            queued_submits: Vec::new(),
            attaching_events: VecDeque::new(),
            spawn_requests: std::collections::HashMap::new(),
            first_thread: FirstThread::OnConnect,
            start_held: false,
            consent: None,
            keymap: Rc::new(Keymap::defaults()),
            tui: facts::TuiSettings::default(),
            effects: effects::Effects::default(),
            ide_submissions: ide_submit::State::default(),
            pending_notices: Vec::new(),
            daybreak: threads::daybreak::Access::default(),
            now: Now::default(),
            transcript_text: None,
            modes: Vec::new(),
            reserve_returns: BTreeMap::new(),
            image_restrictions: BTreeMap::new(),
        }
    }

    /// The renderer `/export` writes the conversation with (`TranscriptText`).
    pub fn with_transcript_text(mut self, text: TranscriptText) -> Self {
        self.transcript_text = Some(text);
        self
    }

    /// A namespace minted by the shell for submission receipts, which outlive a connection.
    pub fn with_submission_namespace(mut self, namespace: String) -> Self {
        self.mint.set_submission_namespace(namespace);
        self
    }

    /// The prompt of the command line (`codex "prompt"`, `-i image`): it goes out when the first
    /// thread starts, as the oracle's `initial_user_message`.
    pub fn with_initial_prompt(mut self, message: UserMessage) -> Self {
        self.queued_submits.push(BeforeThread::Initial(message));
        self
    }

    pub fn with_first_thread(mut self, when: FirstThread) -> Self {
        self.first_thread = when;
        self
    }

    /// What the user wrote and the first thread never took: the first ask they made that waits
    /// for it (later input is another draft, the oracle keeps the submitted one until it is
    /// acknowledged), else `composer` (the blank thread's draft, expanded, as the UI reads it).
    /// The command line's own prompt is not the user's draft: it is never printed. The text a
    /// launch that failed prints for the user to keep (oracle `startup_recovery`, printed after
    /// the terminal is restored); control characters other than newline and tab are escaped, so
    /// pasted bytes cannot drive the terminal. Once a thread exists the queue was sent and the
    /// composer is the thread's: nothing is unsent (the oracle's `acknowledged`).
    pub fn unsent_startup_text(&self, composer: &str) -> Option<String> {
        if self.session.active.is_some() {
            return None;
        }
        let asked = self
            .queued_submits
            .iter()
            .find(|ask| !matches!(ask, BeforeThread::Initial(_)));
        let text = asked.map_or_else(|| composer.to_owned(), BeforeThread::text);
        if text.is_empty() {
            return None;
        }
        let mut visible = String::with_capacity(text.len());
        for character in text.chars() {
            if character.is_control() && !matches!(character, '\n' | '\t') {
                visible.extend(character.escape_default());
            } else {
                visible.push(character);
            }
        }
        Some(visible)
    }

    /// `codex resume <id>` / `codex fork <id>`: the session is read once the
    /// connection is up, its directory decided after the startup steps (a
    /// prompt when it differs from this one), and the app opens it in place
    /// of a fresh thread. The first thread starts after the launch.
    pub fn with_start_session(mut self, start: crate::screens::onboarding::session::Start) -> Self {
        self.first_thread = FirstThread::AfterLaunch;
        Rc::make_mut(&mut self.session).launching = Some(start.action);
        crate::screens::onboarding::session::want(
            &mut Rc::make_mut(&mut self.screens).onboarding,
            start,
        );
        self
    }

    /// The host checks folder trust before session actions and confirms trust writes.
    pub fn with_folder_consent(mut self, on: bool) -> Self {
        folder_consent::arm(&mut Rc::make_mut(&mut self.screens).onboarding, on);
        self
    }

    pub fn session(&self) -> &Rc<SessionSlice> {
        &self.session
    }

    pub fn requests(&self) -> &Rc<RequestQueue> {
        &self.requests
    }

    pub fn screens(&self) -> &Rc<ScreenStates> {
        &self.screens
    }

    /// The key bindings (`tui.keymap.*` over the built-in defaults).
    /// The host's answer to the ask `request` (`Intent::Effect`), until
    /// newer answers push it out.
    pub fn effect(&self, request: u64) -> Option<&effects::Done> {
        self.effects.result(request)
    }

    /// The newest answer of any ask, with its request number.
    pub fn latest_effect(&self) -> Option<(u64, &effects::Done)> {
        self.effects.latest()
    }

    /// Why the user verification of request `key` failed, if it did (the words are the approvals
    /// strings' `verification_failure`).
    pub fn verification_failure(&self, key: &RequestKey) -> Option<approvals::verify::Cause> {
        self.effects.verification_failure(key)
    }

    pub fn keymap(&self) -> &Rc<Keymap> {
        &self.keymap
    }

    /// The collaboration-mode presets of the server: what `/plan` and Shift+Tab choose from.
    pub fn collaboration_modes(&self) -> &[crate::screens::select::collaboration::Mask] {
        &self.modes
    }

    /// The `[tui]` settings the host reported.
    /// The moment the shell last told the app (`AppEv::Clock`): the default one until it did.
    pub fn now(&self) -> Now {
        self.now
    }

    pub fn tui(&self) -> facts::TuiSettings {
        self.tui
    }

    pub fn agents_threads(&self) -> agents::CenterThreads {
        agents::CenterThreads::new(
            self.session.active.clone(),
            self.threads.previews().clone(),
            self.requests.clone(),
        )
    }

    pub fn cycle_label(&self) -> Option<BackendLabel> {
        self.session
            .active
            .as_ref()
            .and_then(|id| self.thread(id))
            .and_then(|thread| {
                self.session
                    .cycle_label(thread, self.screens.agents.cycle_offer())
            })
    }

    /// The invariants of the model (ADR-003 R7), checked by the tests after
    /// every event: the shown thread is held, each model sits under its own
    /// id, a ticket is awaited in one place only, a server request is queued
    /// once, and the blank projection is the blank one.
    pub fn assert_consistent(&self) {
        if let Some(active) = &self.session.active {
            assert!(
                self.threads.contains(active),
                "active {active:?} is not held"
            );
        }
        assert_eq!(self.threads.blank().id(), &ThreadId::default());
        let mut seen: Vec<Ticket> = self.outgoing.tickets().collect();
        seen.extend(self.host_outgoing.tickets());
        for (id, model) in self.threads.iter() {
            assert_eq!(model.id(), id, "a model under another thread's key");
            if let TurnState::Running(running) = model.turn() {
                assert_ne!(
                    model.last_ended(),
                    Some(&running.turn),
                    "{id:?} runs the turn it ended"
                );
            }
            seen.extend(model.outgoing().tickets());
        }
        let total = seen.len();
        seen.sort();
        seen.dedup();
        assert_eq!(seen.len(), total, "a ticket awaited twice");
        for run in self.structured.asking() {
            let asked = self
                .outgoing
                .any(|ctx| matches!(ctx, AppCtx::TurnOperation(TurnOperation::Structured { run: asked, .. }) if *asked == run));
            assert!(
                asked,
                "structured turn {run} is waiting for an answer nobody owes"
            );
        }
        let opening = self.screens.agents.opening.as_ref();
        assert_eq!(
            opening.is_some(),
            self.outgoing.any(|ctx| matches!(
                ctx,
                AppCtx::OpenThread {
                    cancelled: false,
                    ..
                }
            )),
            "the opening and its live resume ticket must agree",
        );
        if let Some(opening) = opening {
            assert!(self.outgoing.any(|ctx| matches!(ctx,
                AppCtx::OpenThread { thread, cancelled: false, .. } if thread == opening
            )));
        }
        let keys: Vec<_> = self.requests.entries.iter().map(|e| &e.key).collect();
        for (at, key) in keys.iter().enumerate() {
            assert!(!keys[..at].contains(key), "a request queued twice");
            assert_eq!(key.epoch, self.request_epoch(key.backend));
        }
    }

    /// The app-scoped RPCs whose answer is still awaited.
    pub fn in_flight(&self) -> usize {
        self.outgoing.len()
    }

    pub fn epoch(&self) -> ConnEpoch {
        self.session.epoch
    }

    /// The backend a live request was registered on, or the destination of an unticketed effect.
    pub fn backend_of(&self, command: &Command) -> crate::ids::BackendId {
        command
            .ticket()
            .and_then(|ticket| self.request_address(ticket))
            .map_or_else(|| self.choose_backend(command), |address| address.backend)
    }

    /// Only connection requests still registered on the dispatched connection may go out.
    pub fn dispatch_route(
        &self,
        command: &Command,
        tell: Option<Address>,
    ) -> Option<(BackendId, Option<Address>)> {
        if let Some((ticket, kind)) = host_asks::ReplyKind::of_command(command) {
            return host_asks::may_dispatch(self, ticket, kind)
                .then(|| (BackendId::default(), kind.transport_address(self)));
        }
        if host_asks::uses_transport_without_reply(command) {
            let address = self.address(BackendId::default());
            return Some((address.backend, Some(address)));
        }
        let address = match command {
            Command::Effect {
                ask: effects::Ask::Audio(command),
                ..
            } => {
                use screens::voice::AudioAction;
                let local = command.action == AudioAction::Close
                    || (self.voice_host_ready()
                        && self
                            .threads
                            .get(&command.owner.thread)
                            .is_some_and(|thread| {
                                thread.screen_slots().voice.owns(&command.owner)
                            }));
                return local.then_some((BackendId::default(), None));
            }
            Command::Effect {
                ask: effects::Ask::VoiceSettings(request),
                ..
            } => {
                if !self.screens.voice.settings.issued(request)
                    || self.voice_settings_origin(&request.origin.thread).as_ref()
                        != Some(&request.origin)
                {
                    return None;
                }
                if request.operation.local() {
                    return self
                        .voice_host_ready()
                        .then_some((BackendId::default(), None));
                }
                request.origin.address
            }
            Command::Effect {
                request,
                ask:
                    effects::Ask::GitSummary {
                        thread,
                        cwd,
                        address,
                    },
            } => {
                if !git_summary::may_dispatch(self, *request, thread, cwd, *address) {
                    return None;
                }
                *address
            }
            Command::Effect {
                ask: effects::Ask::ReconnectIdleThread { address, .. },
                ..
            } => *address,
            Command::Effect { request, ask } if ask.workspace_receipt().is_some() => {
                self.effects.workspace_dispatch(*request, ask)?
            }
            _ => match command.ticket() {
                Some(ticket) => self.request_address(ticket).or(tell)?,
                None => return Some((self.choose_backend(command), None)),
            },
        };
        (self.address(address.backend) == address).then_some((address.backend, Some(address)))
    }

    fn request_address(&self, ticket: Ticket) -> Option<Address> {
        self.outgoing
            .address(ticket)
            .or_else(|| self.threads.request(ticket).map(|(_, address)| address))
    }

    fn address(&self, backend: BackendId) -> Address {
        Address {
            backend,
            epoch: self.transport_epoch_of(backend),
        }
    }

    fn voice_settings_origin(&self, thread: &ThreadId) -> Option<screens::voice::settings::Origin> {
        let backend = self.screens.agents.owner_of(thread.as_str());
        if self.session.connections.get(&backend) != Some(&ConnectionState::Connected) {
            return None;
        }
        let cwd = self.threads.get(thread)?.screen_slots().chat.cwd.clone();
        Some(screens::voice::settings::Origin {
            thread: thread.clone(),
            address: self.address(backend),
            cwd,
        })
    }

    fn voice_host_ready(&self) -> bool {
        !matches!(
            self.session.connections.get(&BackendId::default()),
            Some(ConnectionState::Disconnected | ConnectionState::Reconnecting)
        )
    }

    fn local_side_storage(&self, backend: BackendId) -> Option<LocalSideStorage> {
        self.local_side_storage
            .filter(|(address, _)| *address == self.address(backend))
            .map(|(_, storage)| storage)
    }

    fn transport_epoch_of(&self, backend: BackendId) -> ConnEpoch {
        self.transport_epochs
            .get(&backend)
            .copied()
            .unwrap_or_else(|| self.epoch_of(backend))
    }

    /// Selects an address at registration, or for a local effect that has no RPC ticket.
    fn choose_backend(&self, command: &Command) -> BackendId {
        let agents = &self.screens.agents;
        match command {
            Command::Screen {
                command: ScreenCommand::Agents(command),
                ..
            } => agents::backend_of(agents, command),
            Command::Screen {
                command: ScreenCommand::Resume(command),
                ..
            } => command.backend(),
            Command::StartTurn { thread, .. } | Command::InterruptTurn { thread, .. } => {
                agents.owner_of(thread.as_str())
            }
            Command::Effect {
                ask:
                    effects::Ask::ReconnectIdleThread { address, .. }
                    | effects::Ask::GitSummary { address, .. },
                ..
            } => address.backend,
            Command::Effect {
                ask: effects::Ask::Audio(_),
                ..
            } => BackendId::default(),
            Command::RejectRequest { key }
            | Command::ResolveRequest { key, .. }
            | Command::Effect {
                ask: effects::Ask::VerifyUser { key, .. },
                ..
            } => key.backend,
            // An ask about a thread runs on the connection of its owner (`Ask::thread`).
            Command::Effect { ask, .. } if ask.thread().is_some() => {
                agents.owner_of(ask.thread().map_or("", |thread| thread.as_str()))
            }
            Command::StartThread { backend, .. }
            | Command::ReadRateLimits { backend, .. }
            | Command::SendAddCreditsNudge { backend, .. } => *backend,
            // The startup page reads and writes this machine's first backend.
            Command::Effect {
                ask: effects::Ask::Startup { .. } | effects::Ask::Import { .. },
                ..
            } => BackendId::default(),
            // Provider definitions configure the primary app server.
            Command::Feature {
                command: crate::features::FeatureCommand::Providers(_),
                ..
            } => BackendId::default(),
            Command::Screen { thread, .. } | Command::Feature { thread, .. }
                if !thread.as_str().is_empty() =>
            {
                agents.owner_of(thread.as_str())
            }
            _ => agents.displayed,
        }
    }

    /// The app-scoped state of the feature slices: provided to the views (a live menu of a
    /// feature reads its rows from its state), changed only by the features' own steps.
    pub fn features(&self) -> &Rc<crate::features::FeatureStates> {
        &self.features
    }

    /// The wishes of the features nobody has shown yet, provided to the surface's presenter.
    pub fn wishes(&self) -> &Rc<crate::features::Wishes> {
        &self.wishes
    }

    /// What the surfaces want loaded, with how many want each.
    pub fn interests(&self) -> &crate::features::Interests {
        &self.interests
    }

    /// The model of a thread this process holds.
    /// A native dialog can keep its keys while its original visit and connection are current.
    pub fn queue_modal_visible(&self, operation: &crate::threads::queue::ModalOperation) -> bool {
        let thread = operation.thread();
        self.session.active.as_ref() == Some(thread)
            && self.threads.get(thread).is_some_and(|model| {
                crate::threads::queue::modal_is_visible(
                    model,
                    operation,
                    features::modal_address(self, thread),
                )
            })
    }

    pub fn thread(&self, id: &ThreadId) -> Option<&Rc<ThreadModel>> {
        self.threads.get(id)
    }

    /// The projection of the active thread, or the blank one.
    pub fn active_thread(&self) -> Rc<ThreadModel> {
        self.session
            .active
            .as_ref()
            .and_then(|id| self.threads.get(id))
            .unwrap_or_else(|| self.threads.blank())
            .clone()
    }

    /// The epoch of `backend`'s connection.
    pub fn epoch_of(&self, backend: BackendId) -> ConnEpoch {
        if backend == BackendId::default() {
            self.session.epoch
        } else {
            self.session
                .epochs
                .get(&backend)
                .copied()
                .unwrap_or_default()
        }
    }

    /// Attaches the next connection of the first backend: the epoch every
    /// event of it is stamped with (R1). Called once per connection, by the
    /// shell's start.
    pub fn attach(&mut self) -> ConnEpoch {
        self.attach_backend(BackendId::default())
    }

    /// Attaches the next connection of `backend`: its own epoch, so that a
    /// reconnect of one backend never drops the events of another.
    pub fn attach_backend(&mut self, backend: BackendId) -> ConnEpoch {
        let epoch = ConnEpoch(
            self.request_epoch(backend)
                .0
                .max(self.transport_epoch_of(backend).0),
        )
        .next();
        self.request_epochs.remove(&backend);
        self.transport_epochs.remove(&backend);
        let session = Rc::make_mut(&mut self.session);
        if backend == BackendId::default() {
            session.epoch = epoch;
        } else {
            session.epochs.insert(backend, epoch);
        }
        self.drop_requests_of(backend);
        host_asks::lost(self, backend);
        self.retire_outgoing(backend, &mut AppFx::default());
        epoch
    }

    fn retire_outgoing(&mut self, backend: BackendId, fx: &mut AppFx) {
        for preparation in self.preparing_forks.values_mut() {
            if preparation.address.backend == backend {
                // The acknowledged fork might survive on disk without a confirmed boundary.
                // Keep it unavailable and do not let a replacement transport bless or delete it.
                preparation.retired = true;
            }
        }
        ide_submit::retire(self, backend, fx);
        safety_retry::lost(self, backend, fx);
        self.attaching_events
            .retain(|entry| entry.backend != backend);
        features::retire_rate_offer(self, backend);
        features::retire_workspace_effects(self, backend);
        git_summary::retire(self, backend);
        let retired = Rc::make_mut(&mut self.screens)
            .voice
            .settings
            .retire(backend);
        if let Some(thread) = retired.failed {
            use screens::voice::settings::{Feedback, Problem};
            voice_settings_note(self, &thread, Feedback::Failed(Problem::ConnectionLost), fx);
        }
        if let Some(request) = retired.next {
            send_voice_settings(self, request, fx);
        }
        if backend == BackendId::default() {
            let secondary_calls: Vec<_> = self
                .threads
                .iter()
                .filter(|(_, model)| {
                    model
                        .screen_slots()
                        .voice
                        .owner()
                        .is_some_and(|owner| owner.address.backend != backend)
                })
                .map(|(thread, _)| thread.clone())
                .collect();
            for thread in secondary_calls {
                apply_thread(
                    self,
                    &thread,
                    ThreadEv::Screen(screens::voice::ThreadEv::LocalHostLost.into()),
                    fx,
                );
            }
        }
        detach_threads(self, backend, fx);
        self.fail_in_flight(backend);
        select::retire_permission_writes(self, backend, fx);
        structured::lost(self, backend, fx);
    }

    /// The app-scoped requests the replaced connection of `backend` still owed an answer for: its
    /// answer carries the old epoch and never comes, so each is failed here (`Failed`, "connection
    /// lost") and its operation leaves `Loading` (the one-resume-in-flight rule of `OpenThread`
    /// would wait for it forever). What the failures would ask next is not sent: the commands of a
    /// lost connection are not run, and the tickets they minted are spent with them.
    fn fail_in_flight(&mut self, backend: BackendId) {
        let lost: Vec<Ticket> = self.outgoing.tickets_on(backend).collect();
        let mut fx = AppFx::default();
        for ticket in lost {
            apply_reply(self, ticket, Reply::failed(CONNECTION_LOST), &mut fx);
        }
        for ticket in fx.commands.iter().filter_map(Command::ticket) {
            self.outgoing.drop_ticket(ticket);
        }
    }

    fn request_epoch(&self, backend: BackendId) -> ConnEpoch {
        self.request_epochs
            .get(&backend)
            .copied()
            .unwrap_or_else(|| self.epoch_of(backend))
    }

    /// What the lost connection of `backend` told the session: its banner and its reserve.
    fn forget_reads_of(&mut self, backend: BackendId) {
        features::forget_rate_remedy(self, backend);
        let session = Rc::make_mut(&mut self.session);
        session.backend_banners.remove(&backend);
        session.reserve.remove(&backend);
    }

    fn reset_requests(&mut self, backend: BackendId) {
        let epoch = self.request_epoch(backend).next();
        self.request_epochs.insert(backend, epoch);
        self.drop_requests_of(backend);
    }

    /// The connection of `backend` is gone: its queued requests and the verifications of them
    /// (the oracle's `clear` on reconnect) go with it.
    fn drop_requests_of(&mut self, backend: BackendId) {
        let gone: Vec<RequestKey> = self
            .requests
            .entries
            .iter()
            .filter(|entry| entry.key.backend == backend)
            .map(|entry| entry.key.clone())
            .collect();
        if gone.is_empty() {
            return;
        }
        for key in &gone {
            self.effects.verifications.remove(key);
        }
        Rc::make_mut(&mut self.requests)
            .entries
            .retain(|entry| entry.key.backend != backend);
    }

    /// The ingress router: drops what an older connection of its backend
    /// sent (`None`, R1), applies the rest and returns the effects for the
    /// shell to run.
    pub fn ingest(&mut self, ingress: Ingress) -> Option<AppFx> {
        self.ingest_inner(ingress, false)
    }

    /// Scripted screen facts may describe a phase without making its host request.
    pub(crate) fn ingest_seeded_screen(&mut self, event: ScreenEv) -> Option<AppFx> {
        self.ingest_inner(Ingress::new(self.epoch(), AppEv::Screen(event)), true)
    }

    fn ingest_inner(&mut self, ingress: Ingress, allow_seed: bool) -> Option<AppFx> {
        if ingress.epoch != self.epoch_of(ingress.backend) {
            return None;
        }
        let mut fx = AppFx::default();
        let transport_epoch = ingress.transport_epoch.unwrap_or(ingress.epoch);
        let address = Address {
            backend: ingress.backend,
            epoch: transport_epoch,
        };
        if let Some(storage) = ingress.local_side_storage {
            if self.address(ingress.backend) != address {
                return None;
            }
            self.local_side_storage = Some((address, storage));
            return Some(fx);
        }
        let Some(event) = ingress.ev else {
            if transport_epoch != self.transport_epoch_of(ingress.backend).next() {
                return None;
            }
            self.transport_epochs
                .insert(ingress.backend, transport_epoch);
            host_asks::transport_lost(self, ingress.backend);
            self.retire_outgoing(ingress.backend, &mut fx);
            features::reconnect_images(self, ingress.backend, &mut fx);
            host_asks::transport_replaced(self, ingress.backend, &mut fx);
            if ingress.backend == BackendId::default() {
                read_session(self, &mut fx);
            }
            finish_apply(self, false, &mut fx);
            git_summary::sync(self, false, &mut fx);
            sync_picker_root(self);
            return Some(fx);
        };
        if ingress.transport_epoch.is_some() && self.address(ingress.backend) != address {
            return None;
        }
        if let AppEv::Screen(ScreenEv::Voice(screens::voice::Ev::OrderedLegacyPublisher(publisher))) =
            &event
            && (ingress.transport_epoch.is_none()
                || *publisher != address
                || address.backend != BackendId::default())
        {
            return None;
        }
        if let Some(ticket) = ingress.host_failure
            && !host_asks::accept_failure(self, address, ticket)
        {
            return None;
        }
        if !self.accepts_reply(address, &event)
            || !host_asks::accept(self, address, &event, allow_seed)
        {
            return None;
        }
        apply_at(
            self,
            ingress.backend,
            ingress.transport_epoch.map(|_| address),
            event,
            &mut fx,
        );
        Some(fx)
    }

    fn accepts_reply(&self, origin: Address, event: &AppEv) -> bool {
        match event {
            AppEv::Reply(ticket, _) => {
                self.address(origin.backend) == origin
                    && self.request_address(*ticket) == Some(origin)
            }
            AppEv::Effect {
                request,
                done: effects::Done::GitSummary(_),
            } => git_summary::accepts(self, origin, *request),
            AppEv::Effect { request, done } => self
                .effects
                .accepts_workspace_receipt(*request, origin, done),
            _ => true,
        }
    }
}

/// Applies one event to the app-scoped model, thread events to their thread
/// model (`threads::apply`), and returns effects as data (R3, R10).
pub fn apply_app(app: &mut CodexApp, ev: AppEv, fx: &mut AppFx) {
    // In-process and scripted events carry no transport stamp. An explicitly stamped event
    // always enters through `ingest` (or `apply_app_from` with its current connection).
    let origin = match &ev {
        AppEv::Reply(ticket, _) => app.request_address(*ticket).map(|address| address.backend),
        AppEv::Effect { request, .. } => app
            .effects
            .workspace_address(*request)
            .map(|address| address.backend),
        _ => None,
    };
    apply_app_from(app, origin.unwrap_or_default(), ev, fx);
}

/// `apply_app` for an event of backend `origin` (notifications and requests
/// carry the backend they came from).
pub fn apply_app_from(app: &mut CodexApp, origin: BackendId, ev: AppEv, fx: &mut AppFx) {
    let address = app.address(origin);
    if !app.accepts_reply(address, &ev) || !host_asks::accept(app, address, &ev, true) {
        return;
    }
    apply_at(app, origin, None, ev, fx);
}

fn apply_at(
    app: &mut CodexApp,
    origin: BackendId,
    transport: Option<Address>,
    ev: AppEv,
    fx: &mut AppFx,
) {
    let emitted_from = fx.commands.len();
    let clock = matches!(ev, AppEv::Clock(_));
    // Only events that can bring tasks of other directories into the center.
    let lists = matches!(ev, AppEv::Reply(..) | AppEv::Screen(_));
    let ancestry = matches!(ev, AppEv::Reply(..) | AppEv::Screen(ScreenEv::Agents(_)))
        || matches!(&ev, AppEv::Backend(BackendEv::Notification(note)) if matches!(
            &**note,
            ServerNotification::ThreadStarted(_)
                | ServerNotification::ThreadArchived(_)
                | ServerNotification::ThreadDeleted(_)
        ));
    let status = matches!(ev, AppEv::Screen(ScreenEv::Chat(chat::Ev::Status(_))));
    let workspace_changed = git_summary::observes(&ev);
    let finishing_turn = git_summary::finishing_turn(app, origin, &ev);
    let previous = Rc::clone(&app.session);
    // the refusal the last apply heard has been on screen for its frame
    if app.session.cloud_switch == Some(CloudSwitch::Failed) {
        Rc::make_mut(&mut app.session).cloud_switch = None;
    }
    apply_event(app, origin, transport, ev, fx);
    finish_apply(app, lists, fx);
    let active_changed =
        !Rc::ptr_eq(&previous, &app.session) && previous.active != app.session.active;
    if active_changed {
        select::retire_model_picker(app);
        features::retire_ide(app);
        if let Some(thread) = &previous.active {
            safety_retry::left(app, thread, fx);
            features::retire_queue_modal_choices(app, thread, fx);
        }
    }
    if status || active_changed {
        notices::invalid_status_items(app, fx);
    }
    if workspace_changed || active_changed {
        let refresh_git = finishing_turn
            .as_ref()
            .is_some_and(|turn| git_summary::ended(app, turn));
        git_summary::sync(app, refresh_git, fx);
    }
    if ancestry || active_changed {
        sync_picker_root(app);
    }
    ide_submit::prepare(app, fx, emitted_from);
    if let Some(thread) = app.session.active.clone()
        && let Some(at) = app
            .threads
            .get(&thread)
            .and_then(|model| model.screen_slots().voice.sample_at())
    {
        if clock && app.now.reached(at) {
            apply_thread(
                app,
                &thread,
                ThreadEv::Screen(screens::voice::ThreadEv::Sample.into()),
                fx,
            );
        } else {
            let wake = Wake::new(app.now, at);
            if fx
                .wake
                .is_none_or(|current| wake.at().millis() < current.at().millis())
            {
                fx.wake = Some(wake);
            }
        }
    }
}

fn sync_picker_root(app: &mut CodexApp) {
    let model = &app.screens.agents;
    let root = app
        .session
        .active
        .as_ref()
        .and_then(|active| model.descendants_root(active));
    if model.descendant_root != root {
        let model = &mut Rc::make_mut(&mut app.screens).agents;
        model.descendant_root = root;
        model.descendant_root_visit = model.descendant_root_visit.wrapping_add(1);
        model.descendants = None;
    }
}

fn finish_apply(app: &mut CodexApp, lists: bool, fx: &mut AppFx) {
    consent::follow(app, fx);
    sync_cloud_switch(app);
    mentions::sync(app, fx);
    drain_popover_asks(app, fx);
    features::sync_rate_nudge(app);
    if lists && !app.outgoing.any(is_attach) {
        app.attaching_events.clear();
    }
    if lists && agents::wants_dirs(&app.screens.agents) {
        let cwds = agents::wanted_dirs(&mut Rc::make_mut(&mut app.screens).agents);
        if !cwds.is_empty() {
            fx.host(effects::Ask::Directories { cwds });
        }
    }
    safety_retry::tick(app, fx);
    fx.wake = safety_retry::wake(app, features::wake(app));
}

/// `SessionSlice::cloud_switch` follows the tickets: connecting while a start asked of a Cloud
/// backend is unanswered (a dropped ticket ends it with no arm of its own); a refusal the reply
/// arm set stays for this apply.
fn sync_cloud_switch(app: &mut CodexApp) {
    let now = match app.session.cloud_switch {
        Some(CloudSwitch::Failed) => return,
        _ if app.outgoing.any(|ctx| {
            matches!(ctx, AppCtx::StartThread { switch: Some(backend) }
                if app.screens.agents.is_cloud(*backend))
        }) =>
        {
            Some(CloudSwitch::Connecting)
        }
        _ => None,
    };
    if app.session.cloud_switch != now {
        Rc::make_mut(&mut app.session).cloud_switch = now;
    }
}

/// The composer's asks of the host (message history, file search, mention usage), taken once per
/// apply: one command each, in order.
fn drain_popover_asks(app: &mut CodexApp, fx: &mut AppFx) {
    if app.screens.popovers.pending_asks().is_empty() {
        return;
    }
    for ask in Rc::make_mut(&mut app.screens).popovers.take_asks() {
        let thread = match &ask {
            screens::popovers::Ask::ClipboardImage(read) => Some(read.thread.clone()),
            _ => app.session.active.clone(),
        };
        fx.host(effects::Ask::Popovers { ask, thread });
    }
}

fn apply_event(
    app: &mut CodexApp,
    origin: BackendId,
    transport: Option<Address>,
    ev: AppEv,
    fx: &mut AppFx,
) {
    let connection = match &ev {
        AppEv::Connected | AppEv::Screen(ScreenEv::Agents(agents::Ev::Reconnected)) => {
            Some(ConnectionState::Connected)
        }
        AppEv::StartFailed(_)
        | AppEv::Disconnected
        | AppEv::Screen(ScreenEv::Agents(agents::Ev::Notice(Some(
            agents::Notice::ReconnectFailed,
        )))) => Some(ConnectionState::Disconnected),
        AppEv::Screen(ScreenEv::Agents(agents::Ev::Reconnecting)) => {
            Some(ConnectionState::Reconnecting)
        }
        _ => None,
    };
    if let Some(connection) = connection {
        Rc::make_mut(&mut app.session)
            .connections
            .insert(origin, connection);
    }
    match ev {
        AppEv::Connected if origin != BackendId::default() => {
            if app.screens.chat.status.account_usage {
                refresh_rates(app, origin, status::RateLimitOrigin::Startup, fx);
            }
        }
        AppEv::Connected => {
            if app.first_thread == FirstThread::Never {
                Rc::make_mut(&mut app.session).status = Session::Ready;
            }
            host_asks::step(app, fx, |screens, fx| {
                crate::screens::onboarding::connected(&mut screens.onboarding, fx)
            });
            read_session(app, fx);
            // the model catalog is read at connect (`refresh_available_models`): the tier
            // commands of the model and the footer's model name come from it
            select::intent(app, crate::screens::select::Intent::LoadModels, fx);
            // the collaboration modes are discovered at bootstrap (`collaboration_modes::list`)
            select::ask(
                app,
                crate::screens::select::Command::Collaboration,
                crate::screens::select::Ctx::Collaboration,
                fx,
            );
            match app.first_thread {
                FirstThread::Never => {}
                FirstThread::OnConnect => start_first_thread(app, fx),
                FirstThread::AfterLaunch => {
                    app.start_held = true;
                    release_held_start(app, fx);
                }
            }
            // a reconnect: what the surfaces still want is asked again
            connection::up(app, origin, fx);
        }
        AppEv::StartFailed(message) => {
            fx.ends = true;
            set_status(app, Session::Failed(Failure::Message(message)));
        }
        AppEv::Disconnected => {
            host_asks::lost(app, origin);
            connection::lost(app, origin, fx);
            // after the failures of the asks in flight: the session says the connection stopped
            if active_backend(app) == origin {
                set_status(app, Session::Failed(Failure::Stopped));
            }
        }
        AppEv::ShuttingDown => {
            ide_submit::shutdown(app, fx);
            let voice_calls: Vec<_> = app
                .threads
                .iter()
                .filter(|(_, model)| model.screen_slots().voice.has_call())
                .map(|(id, _)| id.clone())
                .collect();
            for thread in voice_calls {
                apply_thread(
                    app,
                    &thread,
                    ThreadEv::Screen(screens::voice::ThreadEv::Retired.into()),
                    fx,
                );
            }
            features::connection(app, crate::features::Connection::ShuttingDown, fx);
        }
        AppEv::Backend(BackendEv::Notification(notification)) => {
            let ordered_legacy_publisher =
                transport.is_some_and(|address| app.screens.voice.legacy_captions_from(address));
            if let Some((thread, voice)) = screens::voice::project(&notification) {
                let needs_proof = app
                    .threads
                    .get(&thread)
                    .is_some_and(|thread| thread.screen_slots().voice.uses_legacy_captions());
                if app.screens.agents.owner_of(thread.as_str()) == origin
                    && (!needs_proof || ordered_legacy_publisher)
                {
                    let event = match voice {
                        screens::voice::ThreadEv::Closed(reason) if needs_proof => {
                            threads::voice_transcript::ThreadEvent::LegacyClosed(reason).into()
                        }
                        _ => ThreadEv::Screen(voice.into()),
                    };
                    apply_thread(app, &thread, event, fx);
                }
                return;
            }
            if ordered_legacy_publisher
                && let Some((thread, caption)) =
                    threads::voice_transcript::project_legacy(&notification, |thread| {
                        app.screens.agents.owner_of(thread.as_str()) == origin
                            && app.threads.get(thread).is_some_and(|model| {
                                model.screen_slots().voice.uses_legacy_captions()
                            })
                    })
            {
                apply_thread(
                    app,
                    &thread,
                    threads::voice_transcript::ThreadEvent::LegacyCaption {
                        caption,
                        show_user_preview: app.screens.chat.status.animations,
                    }
                    .into(),
                    fx,
                );
                return;
            }
            if let Some((thread, event)) = threads::voice_transcript::project(&notification) {
                if app.screens.agents.owner_of(thread.as_str()) == origin {
                    apply_thread(
                        app,
                        &thread,
                        threads::voice_transcript::ThreadEvent::Caption {
                            event,
                            show_user_preview: app.screens.chat.status.animations,
                        }
                        .into(),
                        fx,
                    );
                }
                return;
            }
            match &*notification {
                ServerNotification::AccountRateLimitsUpdated(note) => {
                    features::rolling_remedy(app, origin, &note.rate_limits);
                    // The notification is newer than a refresh nobody waits for; a
                    // read a card or a menu waits for still answers it.
                    app.outgoing.retain(|ctx| {
                        !matches!(ctx, AppCtx::RateLimits { backend, origin: read, .. }
                            if *backend == origin && read.folds_into_a_read_in_flight())
                    });
                    install_rates(app, origin, note.rate_limits.clone().into());
                    // a rolling update warns like a read does, but never writes the buckets
                    let sample = crate::screens::status::Sample::of(&note.rate_limits);
                    let read_out = app.outgoing.any(|ctx| {
                        matches!(ctx, AppCtx::RateLimits { backend, superseded: false, .. }
                            if *backend == origin)
                    });
                    let warnings = Rc::make_mut(&mut app.session)
                        .rate_snapshots
                        .entry(origin)
                        .or_default()
                        .rolling(&sample, read_out);
                    if active_backend(app) == origin {
                        reset::warn(app, warnings, fx);
                    }
                    // never reuse the permission from before a new hard stop
                    // (`invalidate_ordinary_usage_recovery`)
                    if (note.rate_limits.rate_limit_reached_type.is_some()
                        || note.rate_limits.spend_control_reached == Some(true))
                        && let Some(reserve) =
                            Rc::make_mut(&mut app.session).reserve.get_mut(&origin)
                    {
                        reserve.recovered = false;
                    }
                    if app.screens.chat.status.account_usage
                        && (note.rate_limits.rate_limit_reached_type.is_some()
                            || note.rate_limits.spend_control_reached == Some(true)
                            || app.session.backend_banners.contains_key(&origin))
                    {
                        refresh_rates(app, origin, status::RateLimitOrigin::Startup, fx);
                    }
                    features::rolling_limits(app, origin, &note.rate_limits, fx);
                    return;
                }
                // The app server says the apps directory changed
                // (`refresh_connector_directory_after_notification`): with a thread on display
                // it is read again, and the apps a user cannot call leave the mentions.
                ServerNotification::AppListUpdated(note) => {
                    if app.session.active.is_some() {
                        let listed: Vec<_> = note
                            .data
                            .iter()
                            .cloned()
                            .map(screens::select::apps::App::of)
                            .collect();
                        drop_uncallable_apps(app, &listed);
                        select::event(app, screens::select::Ev::AppsNotified(listed), fx);
                    }
                    return;
                }
                // The identity boundary (`update_account_state`): what was asked of, and shown
                // for, the previous account is not the new one's.
                ServerNotification::AccountUpdated(note) => {
                    // the apps directory and the permission catalog of the old account go
                    // (`invalidate_connector_scope`, `invalidate_permission_discovery`)
                    select::event(app, screens::select::Ev::ScopeChanged, fx);
                    features::retire_rate_offer(app, origin);
                    features::forget_rate_remedy(app, origin);
                    invalidate_image_restrictions(app, origin);
                    Rc::make_mut(&mut app.session)
                        .backend_banners
                        .remove(&origin);
                    Rc::make_mut(&mut app.session).reserve.remove(&origin);
                    // what was read for the old account goes; the new one's plan is what the
                    // next snapshot is judged by (`update_account_state` sets `plan_type`)
                    let store = Rc::make_mut(&mut app.session)
                        .rate_snapshots
                        .entry(origin)
                        .or_default();
                    store.clear();
                    store.limits_mut().set_plan(note.plan_type);
                    reset::account_changed(app, origin);
                    app.outgoing.retain(|ctx| {
                        !matches!(
                            ctx,
                            AppCtx::Nudge { backend, .. }
                                | AppCtx::RateLimits { backend, .. }
                                | AppCtx::Redemption { backend, .. } if *backend == origin
                        )
                    });
                    if active_backend(app) == origin {
                        status::apply(
                            &mut Rc::make_mut(&mut app.screens).status,
                            status::Ev::Account(status::account_from_auth_mode(
                                note.auth_mode,
                                note.plan_type,
                            )),
                            fx,
                        );
                    }
                    // The answers the cards waited for are gone: they end
                    // empty (oracle `update_account_state`).
                    finish_status_cells(app, origin, None, fx);
                }
                // With no thread open there is no transcript to show a thread-less
                // warning or deprecation notice in: the banner line keeps it. With a
                // thread open it is that thread's detailed-transcript row (below).
                ServerNotification::Warning(note)
                    if note.thread_id.is_none() && app.session.active.is_none() =>
                {
                    Rc::make_mut(&mut app.session)
                        .backend_notices
                        .insert(origin, note.message.clone());
                }
                ServerNotification::ConfigWarning(note) => {
                    Rc::make_mut(&mut app.session)
                        .backend_notices
                        .insert(origin, note.summary.clone());
                }
                ServerNotification::DeprecationNotice(note) if app.session.active.is_none() => {
                    Rc::make_mut(&mut app.session)
                        .backend_notices
                        .insert(origin, note.summary.clone());
                }
                _ => {}
            }
            // The startup page's sign-in: a login completed, the account.
            if matches!(
                &*notification,
                ServerNotification::AccountLoginCompleted(_)
                    | ServerNotification::AccountUpdated(_)
                    | ServerNotification::GatewayOAuthChanged(_)
            ) && origin == BackendId::default()
            {
                host_asks::step(app, fx, |screens, fx| {
                    crate::screens::onboarding::observe(&mut screens.onboarding, &notification, fx)
                });
            }
            // The import flow: the completion of the import this client started.
            if matches!(
                &*notification,
                ServerNotification::ExternalAgentConfigImportCompleted(_)
            ) {
                crate::screens::import::observe(
                    &mut Rc::make_mut(&mut app.screens).import,
                    &notification,
                    fx,
                );
            }
            let archived = match notification.as_ref() {
                ServerNotification::ThreadArchived(event) => Some(event.thread_id.as_str()),
                _ => None,
            };
            let attachment_change = archived.is_some()
                || matches!(
                    notification.as_ref(),
                    ServerNotification::ThreadUnarchived(_)
                );
            if !attachment_change {
                features::observe(app, &notification, fx);
            } else if app.local_side_storage(origin).is_some() {
                let owner = targets::notification_thread(&notification)
                    .and_then(|thread| app.screens.agents.owners.get(thread));
                if owner.is_none_or(|owner| *owner == origin) {
                    if let Some(child) = archived {
                        app.outgoing.retain(|ctx| !matches!(ctx,
                            AppCtx::Opening { thread, attached_to: Some(_), .. } if thread.as_str() == child));
                    }
                    features::observe(app, &notification, fx);
                }
            }
            structured::observe(app, &notification, fx);
            // `serverRequest/resolved`: the server answered or withdrew a
            // request first; it leaves the queue and nothing is rejected.
            if let ServerNotification::ServerRequestResolved(resolved) = &*notification {
                let key = targets::request_key(
                    origin,
                    app.request_epoch(origin),
                    &resolved.thread_id,
                    &resolved.request_id,
                );
                if app.requests.contains(&key) {
                    remove_request(app, &key);
                }
                return;
            }
            // A thread another backend owns is not this backend's to speak
            // of (oracle: `BackendRegistry::observe`); an unowned one is
            // claimed by the backend that did.
            if let Some(thread) = targets::notification_thread(&notification) {
                match app.screens.agents.owners.get(thread) {
                    Some(owner) if *owner != origin => return,
                    Some(_) => {}
                    None => {
                        Rc::make_mut(&mut app.screens)
                            .agents
                            .observe(origin, thread);
                    }
                }
            }
            if let ServerNotification::Error(error) = &*notification
                && !error.will_retry
            {
                features::rate_failure(app, origin, error, fx);
            }
            if let ServerNotification::TurnCompleted(note) = &*notification
                && note.turn.status == codex_app_server_protocol::TurnStatus::Completed
                && app.screens.chat.status.notification_policy.allows(chat::NoticeKind::TurnComplete.config_id())
                && app.threads.get(&ThreadId::new(&note.thread_id)).is_some_and(|model| {
                    matches!(model.turn(), TurnState::Running(running) if running.turn.as_str() == note.turn.id)
                })
            {
                let preview = note.turn.item_payloads().rev().find_map(|item| {
                    if let codex_app_server_protocol::ThreadItemPayload::AgentMessage { text, .. } = item {
                        text.lines().find(|line| !line.trim().is_empty()).map(str::to_owned)
                    } else {
                        None
                    }
                });
                fx.desktop_notice = Some(chat::DesktopNotice { kind: chat::NoticeKind::TurnComplete, preview });
            }
            let listed = matches!(&*notification, ServerNotification::ReasoningSummaryTextDelta(delta)
                if app.screens.agents.threads.iter().any(|thread| thread.id == delta.thread_id));
            app.threads.previews_mut().observe(&notification, listed);
            if agents::observes(&notification) {
                let outs =
                    agents::observe(&mut Rc::make_mut(&mut app.screens).agents, &notification);
                send_agents(app, outs, fx);
            }
            // A diagnostic is retained for the footer count and the warnings viewer; one the
            // session already showed (or the user dismissed unchanged) is not shown again.
            if let Some(warning) = crate::screens::dialogs::incoming(&notification)
                && !crate::screens::dialogs::take_in(
                    &mut Rc::make_mut(&mut app.screens).dialogs,
                    &warning,
                )
            {
                return;
            }
            if matches!(&*notification, ServerNotification::TurnStarted(_)) {
                Rc::make_mut(&mut app.screens)
                    .dialogs
                    .intake
                    .startup_complete = true;
            }
            // What the server sends without a thread (a deprecation notice) is
            // the displayed thread's: the chat shows it in the detailed transcript.
            if active_backend(app) == origin
                && let Some(ev) = threads::project_unrouted(&notification)
                && let Some(thread) = app.session.active.clone()
            {
                apply_thread(app, &thread, ev, fx);
            }
            if let ServerNotification::ItemStarted(started) = &*notification
                && let Some((call, request)) = threads::requested_spawn(&started.item.payload)
            {
                app.spawn_requests
                    .insert((started.thread_id.clone(), call), request);
            }
            if let Some((thread, ev)) = threads::project(&notification) {
                let ev = reworded(app, &thread, ev);
                let ev = named(app, &thread, ev);
                let attached = app.threads.contains(&thread);
                let attaching =
                    !attached && app.outgoing.any_on(origin, |ctx| may_attach(ctx, &thread));
                let permissions = match &*notification {
                    ServerNotification::ThreadSettingsUpdated(note) if attached || attaching => {
                        Some(screens::select::permissions::Effective::observed(
                            app.address(origin),
                            app.mint.next(),
                            &note.thread_settings,
                        ))
                    }
                    _ => None,
                };
                if attached {
                    let reverted = matches!(ev, ThreadEv::Reverted);
                    // a user message of a thread with no name (`automatic_title_user_message`)
                    let unnamed = unnamed_request(app, &thread, &notification);
                    apply_thread(app, &thread, ev, fx);
                    if let Some(permissions) = permissions {
                        select::record_permission_settings(app, &thread, permissions, fx);
                    }
                    if let Some(text) = unnamed {
                        features::on_user_message(app, &thread, &text, fx);
                    }
                    if reverted {
                        // the retained turns are the server's: read them again
                        fetch_history(app, &thread, fx);
                    }
                } else if attaching {
                    // Bound the startup/reconnect window. A loaded turn or terminal
                    // snapshot can recover the transcript if this buffer overflows.
                    const MAX_ATTACH_EVENTS: usize = 256;
                    if app.attaching_events.len() == MAX_ATTACH_EVENTS {
                        app.attaching_events.pop_front();
                    }
                    app.attaching_events.push_back(AttachEvent {
                        backend: origin,
                        thread,
                        event: ev,
                        permissions,
                    });
                }
            }
        }
        AppEv::Backend(BackendEv::Request(request)) => {
            let (method, thread) = targets::request_target(&request);
            features::observe_request(app, &request, fx);
            let key = targets::request_key(origin, app.request_epoch(origin), thread, request.id());
            if !app.requests.contains(&key) {
                app.effects.verifications.note(key.clone(), &request);
                let preview = agents::request_preview(&request);
                let kind = match &*request {
                    ServerRequest::CommandExecutionRequestApproval { .. } => {
                        Some(chat::NoticeKind::CommandApproval)
                    }
                    ServerRequest::FileChangeRequestApproval { .. } => {
                        Some(chat::NoticeKind::EditApproval)
                    }
                    ServerRequest::PermissionsRequestApproval { .. } => {
                        Some(chat::NoticeKind::CommandApproval)
                    }
                    ServerRequest::McpServerElicitationRequest { .. } => {
                        Some(chat::NoticeKind::Elicitation)
                    }
                    ServerRequest::ToolRequestUserInput { .. } => Some(chat::NoticeKind::Question),
                    _ => None,
                };
                if let Some(kind) = kind
                    && app
                        .screens
                        .chat
                        .status
                        .notification_policy
                        .allows(kind.config_id())
                {
                    fx.desktop_notice = Some(chat::DesktopNotice {
                        kind,
                        preview: preview.clone(),
                    });
                }
                if approvals::link::rejected(&request) {
                    // A link the oracle refuses (an install URL that is not https, a sign-in off
                    // ChatGPT) is declined as it arrives: no dialog, no queue entry.
                    let decline = approvals::RequestAnswer::Elicitation {
                        decision: approvals::Decision::Decline,
                        content: None,
                        meta: None,
                    };
                    if let Some(result) = requests::answer(&request, None, decline) {
                        fx.commands.push(Command::ResolveRequest { key, result });
                    }
                } else {
                    Rc::make_mut(&mut app.requests)
                        .entries
                        .push(PendingRequest {
                            key,
                            method,
                            preview,
                            prompt: approvals::prompt_of(&request).map(|entry| {
                                let label = approvals::thread_label(
                                    &app.screens.agents.threads,
                                    &entry.thread,
                                    app.session.active.as_ref(),
                                );
                                Rc::new(entry.prompt.with_thread_label(label))
                            }),
                            request: Rc::from(request),
                        });
                }
            }
        }
        AppEv::Backend(BackendEv::Lagged(skipped)) => {
            if transport.is_some_and(|address| app.screens.voice.legacy_captions_from(address)) {
                let legacy_calls: Vec<_> = app
                    .threads
                    .iter()
                    .filter_map(|(thread, model)| {
                        (app.screens.agents.owner_of(thread.as_str()) == origin
                            && model.screen_slots().voice.uses_legacy_captions())
                        .then_some(thread.clone())
                    })
                    .collect();
                for thread in legacy_calls {
                    apply_thread(
                        app,
                        &thread,
                        threads::voice_transcript::ThreadEvent::LostLegacyOrder.into(),
                        fx,
                    );
                }
            }
            // Events were dropped: what the command center derived from them
            // is stale and its list is read again (oracle: `Lagged`).
            app.threads.previews_mut().clear();
            let screens = Rc::make_mut(&mut app.screens);
            agents::apply(&mut screens.agents, agents::Ev::Lagged, fx);
            let outs = agents::update(&mut screens.agents, agents::Intent::Open);
            send_agents(app, outs, fx);
            if let Some(id) = app.session.active.clone()
                && active_backend(app) == origin
            {
                apply_thread(app, &id, ThreadEv::Failure(Failure::Lagged(skipped)), fx);
                // What was missed cannot settle MCP startup: it settles now.
                apply_thread(app, &id, ThreadEv::McpLagged, fx);
            }
        }
        // The stream of a thread this backend owns: what is sent there waits in the queue until it
        // is back.
        AppEv::Backend(BackendEv::StreamLost { thread, error }) => {
            stream_lost(app, origin, thread, error, fx);
        }
        AppEv::Backend(BackendEv::StreamRestored(thread)) => {
            // Only a thread the app marked lost has anything restored (oracle: the flag).
            if app.screens.agents.owner_of(thread.as_str()) == origin
                && let Some(model) = app.threads.get(&thread)
                && model.stream_lost()
            {
                let reconcile = model.reconciling_submissions();
                apply_thread(
                    app,
                    &thread,
                    ThreadEv::StreamRestored {
                        read: false,
                        reconciled: false,
                    },
                    fx,
                );
                if reconcile {
                    request_history(
                        app,
                        &thread,
                        None,
                        agents::HistoryRead::SubmissionRecovery,
                        fx,
                    );
                }
            }
        }
        AppEv::Reply(ticket, reply) => apply_reply(app, ticket, reply, fx),
        AppEv::ServerAnswerFailed { key, message } => {
            apply_thread(
                app,
                &key.thread,
                ThreadEv::Failure(Failure::Message(message)),
                fx,
            );
        }
        AppEv::Intent(intent) => {
            apply_intent(app, intent, fx);
            release_held_start(app, fx);
        }
        AppEv::Keymap(keymap) => app.keymap = Rc::new(*keymap),
        AppEv::Announce(notice) => notices::announce(app, notice, fx),
        AppEv::Daybreak(access) => app.daybreak = access,
        AppEv::Clock(now) => features::clock(app, now, fx),
        AppEv::Tui(settings) => app.tui = settings,
        AppEv::Presets(slots) => select::event(app, crate::screens::select::Ev::Dials(slots), fx),
        AppEv::Feature(fact) => features::fact(app, fact, fx),
        AppEv::ErrorLine { message } => notices::error_line(app, message, fx),
        AppEv::Effect {
            done: effects::Done::GitSummary(summary),
            ..
        } => {
            git_summary::answer(app, summary, fx);
        }
        AppEv::Effect {
            request,
            done: effects::Done::IdePrompt(reading),
        } => ide_submit::answered(app, request, reading),
        AppEv::Effect {
            done: effects::Done::VoiceSettings(report),
            ..
        } => {
            finish_voice_settings(app, origin, transport, *report, fx);
        }
        AppEv::Effect {
            done: effects::Done::Audio(report),
            ..
        } if origin == BackendId::default() => {
            let owner = &report.owner;
            let thread = owner.thread.clone();
            let call = app
                .threads
                .get(&thread)
                .map(|model| &model.screen_slots().voice);
            if app.address(owner.address.backend) == owner.address
                && app.screens.agents.owner_of(thread.as_str()) == owner.address.backend
                && call.is_some_and(|call| call.accepts_audio(&report))
            {
                apply_thread(
                    app,
                    &thread,
                    ThreadEv::Screen(screens::voice::ThreadEv::Audio(report).into()),
                    fx,
                );
            } else if matches!(&report.result, screens::voice::AudioResult::Prepared(Ok(_)))
                && !call
                    .and_then(screens::voice::ThreadPart::owner)
                    .is_some_and(|current| current.same_call(owner))
            {
                fx.host(effects::Ask::Audio(screens::voice::AudioCommand {
                    owner: report.owner,
                    action: screens::voice::AudioAction::Close,
                }));
            }
        }
        AppEv::Effect { request, done } if effects::is_feature_request(request) => {
            features::effect_done(app, request, done, fx);
        }
        AppEv::Effect { request, done } => {
            if !select::reviewed(app, request, &done, fx) {
                app.effects.settle(request, done);
            }
        }
        AppEv::Verified {
            key,
            attempt,
            result,
        } => verified(app, key, attempt, result, fx),
        AppEv::Screen(ScreenEv::Voice(screens::voice::Ev::OrderedLegacyPublisher(address)))
            if transport != Some(address) => {}
        AppEv::Screen(ev) => {
            if origin != BackendId::default() && matches!(ev, ScreenEv::Onboarding(_)) {
                return;
            }
            if matches!(
                ev,
                ScreenEv::Agents(agents::Ev::Notice(Some(agents::Notice::ReconnectFailed)))
            ) {
                host_asks::lost(app, origin);
            }
            let background_reconnect = origin != app.screens.agents.displayed
                && matches!(
                    ev,
                    ScreenEv::Agents(
                        agents::Ev::Reconnecting
                            | agents::Ev::Reconnected
                            | agents::Ev::Notice(Some(agents::Notice::ReconnectFailed))
                    )
                );
            if let ScreenEv::Chat(chat::Ev::Status(facts)) = &ev {
                select::configured_plan(app, origin, facts.plan_effort.clone());
            }
            match &ev {
                ScreenEv::Select(screens::select::Ev::Provider(provider))
                    if *provider != screens::select::models::Provider::Settled =>
                {
                    invalidate_image_restrictions(app, origin);
                }
                ScreenEv::Chat(chat::Ev::Status(facts)) if facts.account_usage => {
                    reset::startup(app, fx);
                }
                // The details under a thread's status row are the thread's.
                ScreenEv::Chat(chat::Ev::Details { thread, details }) => {
                    let ev = ThreadEv::Screen(ScreenThreadEv::Chat(chat::ThreadEv::Details(
                        details.clone(),
                    )));
                    apply_thread(app, &ThreadId::new(thread), ev, fx);
                }
                ScreenEv::Chat(chat::Ev::ProviderError { thread, error }) => {
                    let ev = ThreadEv::Screen(ScreenThreadEv::Chat(chat::ThreadEv::ProviderError(
                        error.clone(),
                    )));
                    apply_thread(app, &ThreadId::new(thread), ev, fx);
                }
                // A card a script seeds is an entry of the shown thread, as `/status` makes it.
                ScreenEv::Status(status::Ev::Card(card)) => {
                    if let Some(thread) = app.session.active.clone() {
                        let item = threads::Item::Status {
                            card: std::sync::Arc::new((**card).clone()),
                            request: None,
                        };
                        apply_thread(app, &thread, ThreadEv::Appended { turn: None, item }, fx);
                    }
                }
                ScreenEv::Chat(chat::Ev::Pending { thread, pending }) => {
                    apply_thread(
                        app,
                        &ThreadId::new(thread),
                        ThreadEv::Preview(pending.clone()),
                        fx,
                    );
                }
                ScreenEv::Agents(agents::Ev::Current(id)) => {
                    if let Some(id) = id {
                        app.threads.open(id, Origin::Resumed);
                    }
                    Rc::make_mut(&mut app.session).active = id.clone();
                }
                // A seed of the approvals screen replaces what the queued
                // request asks (the thread's own state in a fixture).
                ScreenEv::Approvals(approvals::Ev::Presented(entry)) => {
                    let queue = Rc::make_mut(&mut app.requests);
                    if let Some(queued) = queue
                        .entries
                        .iter_mut()
                        .find(|e| e.key.thread == entry.thread && e.key.id == entry.id)
                    {
                        queued.prompt = Some(Rc::new(entry.prompt.clone()));
                    }
                }
                ScreenEv::Agents(agents::Ev::Request(id, preview)) => {
                    app.threads.previews_mut().seed_request(id, preview.clone());
                }
                ScreenEv::Agents(agents::Ev::Reconnecting | agents::Ev::Lagged) => {
                    if !background_reconnect {
                        app.threads.previews_mut().clear();
                    }
                    if matches!(ev, ScreenEv::Agents(agents::Ev::Reconnecting)) {
                        host_asks::transport_lost(app, origin);
                        connection::lost(app, origin, fx);
                        // An older page that will never come is a failed one: asking again
                        // retries it.
                        let unanswered: Vec<_> = app
                            .threads
                            .iter()
                            .filter(|(id, model)| {
                                model.history_state() == threads::HistoryState::Loading
                                    && app.screens.agents.owner_of(id.as_str()) == origin
                            })
                            .map(|(id, _)| id.clone())
                            .collect();
                        for thread in unanswered {
                            apply_thread(app, &thread, ThreadEv::HistoryFailed, fx);
                        }
                        if !app.outgoing.any(|ctx| {
                            matches!(
                                ctx,
                                AppCtx::OpenThread {
                                    cancelled: false,
                                    ..
                                }
                            )
                        }) {
                            Rc::make_mut(&mut app.screens).agents.opening = None;
                        }
                    }
                }
                _ => {}
            }
            let reconnected = matches!(ev, ScreenEv::Agents(agents::Ev::Reconnected));
            let startup = matches!(ev, ScreenEv::Cells(screens::cells::Ev::Startup(_)));
            if !background_reconnect {
                match ev {
                    ScreenEv::Onboarding(_) | ScreenEv::Startup(_) | ScreenEv::Import(_) => {
                        host_asks::step(app, fx, |screens, fx| screens::apply_ev(screens, ev, fx));
                    }
                    ev => screens::apply_ev(Rc::make_mut(&mut app.screens), ev, fx),
                }
            }
            if startup {
                // The facts came after the first conversation attached: it opens with its card now.
                notices::startup(app, fx);
            }
            if reconnected && origin == BackendId::default() {
                // The new server is told the startup facts again.
                host_asks::reconnected(app, fx);
                read_session(app, fx);
            }
            release_held_start(app, fx);
            if reconnected {
                connection::up(app, origin, fx);
                if app.screens.chat.status.account_usage {
                    refresh_rates(app, origin, status::RateLimitOrigin::Startup, fx);
                }
                if let Some(id) = app.session.active.clone()
                    && app.screens.agents.owner_of(id.as_str()) == origin
                    && is_listed(app, &id)
                {
                    open_verified(app, &id, fx);
                }
                // A new connection: the list is read again (oracle:
                // `finish_reconnect` leaves the overview to refresh).
                if !background_reconnect {
                    let outs = agents::update(
                        &mut Rc::make_mut(&mut app.screens).agents,
                        agents::Intent::Open,
                    );
                    send_agents(app, outs, fx);
                }
            }
        }
    }
}

fn finish_voice_settings(
    app: &mut CodexApp,
    backend: BackendId,
    transport: Option<Address>,
    mut report: screens::voice::settings::Report,
    fx: &mut AppFx,
) {
    use screens::voice::settings::{Change, Intent, Operation, Problem};
    let issued = &report.request.origin;
    if backend != report.request.executor()
        || (!report.request.operation.local()
            && transport.is_some_and(|address| address != issued.address))
    {
        return;
    }
    let thread = issued.thread.clone();
    let current = app
        .voice_settings_origin(&thread)
        .filter(|current| current == issued);
    if current.is_none() {
        report.result = Err(Problem::ConnectionLost);
    }
    let refresh = match (&report.request.operation, &report.result) {
        (Operation::Persist(Change::Device { kind, .. }), result)
            if !matches!(result, Err(Problem::ConnectionLost)) =>
        {
            Some(*kind)
        }
        _ => None,
    };
    let completed = Rc::make_mut(&mut app.screens).voice.settings.finish(report);
    let answered = completed.feedback.is_some();
    if let Some(feedback) = completed.feedback {
        voice_settings_note(app, &thread, feedback, fx);
    }
    if let Some(request) = completed.next {
        send_voice_settings(app, request, fx);
    } else if answered
        && let (Some(kind), Some(current)) = (refresh, current)
        && app.session.active.as_ref() == Some(&thread)
        && let Some(request) = Rc::make_mut(&mut app.screens).voice.settings.begin(
            app.mint.next(),
            current,
            Intent::Read {
                thread,
                operation: Operation::ListDevices(kind),
            },
        )
    {
        send_voice_settings(app, request, fx);
    }
}

fn voice_settings_note(
    app: &mut CodexApp,
    thread: &ThreadId,
    feedback: screens::voice::settings::Feedback,
    fx: &mut AppFx,
) {
    apply_thread(
        app,
        thread,
        ThreadEv::Appended {
            turn: None,
            item: threads::Item::Note(threads::Note::from(threads::NoteKey::Voice(
                screens::voice::Note::Settings(feedback),
            ))),
        },
        fx,
    );
}

fn send_voice_settings(
    app: &mut CodexApp,
    request: screens::voice::settings::Request,
    fx: &mut AppFx,
) {
    use screens::voice::settings::{Intent, Operation, Problem, Report};
    let current = app.voice_settings_origin(&request.origin.thread);
    let host_ready = !request.operation.local() || app.voice_host_ready();
    if host_ready && current.as_ref() == Some(&request.origin) {
        fx.host(effects::Ask::VoiceSettings(request));
        return;
    }
    let retry = current
        .filter(|current| {
            host_ready
                && *current != request.origin
                && app.session.active.as_ref() == Some(&current.thread)
                && !matches!(request.operation, Operation::Persist(_))
        })
        .map(|current| (current, request.operation.clone()));
    let executor = request.executor();
    let report = Report {
        request,
        result: Err(Problem::ConnectionLost),
    };
    if let Some((current, operation)) = retry {
        let model = &mut Rc::make_mut(&mut app.screens).voice.settings;
        model.finish(report);
        let intent = Intent::Read {
            thread: current.thread.clone(),
            operation,
        };
        if let Some(request) = model.begin(app.mint.next(), current, intent) {
            fx.host(effects::Ask::VoiceSettings(request));
        }
    } else {
        finish_voice_settings(app, executor, None, report, fx);
    }
}

/// The first thread of the process, on the default backend.
fn start_first_thread(app: &mut CodexApp, fx: &mut AppFx) {
    fx.ask(app, AppCtx::StartThread { switch: None }, |app, ticket| {
        start_thread(app, ticket, BackendId::default())
    });
}

/// A new thread's start, on the model an accepted migration chose.
fn start_thread(app: &CodexApp, ticket: Ticket, backend: BackendId) -> Command {
    start_thread_in(app, ticket, backend, None)
}

fn start_thread_in(
    app: &CodexApp,
    ticket: Ticket,
    backend: BackendId,
    cwd: Option<String>,
) -> Command {
    Command::StartThread {
        ticket,
        backend,
        model: app.screens.startup.accepted_model().map(str::to_string),
        cwd,
    }
}

/// The held first thread goes out once the startup steps are over and no
/// startup prompt is due or on screen (oracle: the session starts after the
/// launch prompts). Checked after every event and intent that can end the
/// last of them.
fn release_held_start(app: &mut CodexApp, fx: &mut AppFx) {
    if app.start_held
        && app.screens.onboarding.launched()
        && !app.screens.onboarding.due()
        && app.screens.startup.asked.is_none()
    {
        app.start_held = false;
        if crate::screens::onboarding::session::pending(&app.screens.onboarding) {
            open_session(app, fx);
        } else {
            start_first_thread(app, fx);
        }
    }
}

/// The session the command line names is read once the connection is up
/// (asked again on a new connection if the first was lost with the read in
/// flight).
fn read_session(app: &mut CodexApp, fx: &mut AppFx) {
    use crate::screens::onboarding::session;
    if !session::pending(&app.screens.onboarding) {
        return;
    }
    let Some(start) = session::to_read(&mut Rc::make_mut(&mut app.screens).onboarding) else {
        return;
    };
    fx.ask(app, AppCtx::SessionRead, |_, ticket| Command::Screen {
        ticket,
        thread: ThreadId::default(),
        command: ScreenCommand::Agents(agents::Command::Thread(ThreadAsk::Read {
            thread: start.id,
        })),
    });
}

/// The session the command line names, opened in the directory the launch
/// decided, in place of a fresh thread. Nothing opens while the cwd prompt
/// has no answer (the user left instead).
fn open_session(app: &mut CodexApp, fx: &mut AppFx) {
    use crate::screens::onboarding::session;
    use crate::screens::startup::Action;
    let Some(opening) = session::opening(&app.screens.onboarding, &app.screens.startup.answers)
    else {
        return;
    };
    session::opened(&mut Rc::make_mut(&mut app.screens).onboarding);
    let thread = opening.start.id.clone();
    let context = AppCtx::OpenSession {
        action: opening.start.action,
        thread: ThreadId::new(&thread),
    };
    let ask = match opening.start.action {
        Action::Resume => ThreadAsk::ResumeAt {
            thread,
            cwd: opening.cwd,
        },
        Action::Fork => ThreadAsk::ForkAt {
            thread,
            cwd: opening.cwd,
            temporary: None,
        },
    };
    fx.ask(app, context, |_, ticket| Command::Screen {
        ticket,
        thread: ThreadId::default(),
        command: ScreenCommand::Agents(agents::Command::Thread(ask)),
    });
}

/// A fresh thread where none was started (`FirstThread::Never`): the picker
/// of `codex resume` was cancelled. Nothing starts while a thread is open or
/// on its way.
fn start_fresh_thread(app: &mut CodexApp, fx: &mut AppFx) {
    let coming = app.outgoing.any(|ctx| {
        matches!(
            ctx,
            AppCtx::StartThread { .. }
                | AppCtx::NewConversation { .. }
                | AppCtx::OpenThread {
                    cancelled: false,
                    ..
                }
        )
    });
    if app.first_thread == FirstThread::Never
        && app.session.active.is_none()
        && app.screens.agents.opening.is_none()
        && !coming
    {
        consent::begin(
            app,
            consent::Action::Fresh { feature: None },
            ConsentTarget::Folder(None),
            TrustCancel::AgentsOverview,
            fx,
        );
    }
}

fn set_status(app: &mut CodexApp, status: Session) {
    if app.session.status != status {
        Rc::make_mut(&mut app.session).status = status;
    }
}

fn invalidate_image_restrictions(app: &mut CodexApp, backend: BackendId) {
    app.image_restrictions.insert(
        backend,
        ImageRestrictions {
            address: app.address(backend),
            observed: app.mint.next(),
            models: BTreeSet::new(),
        },
    );
}

fn apply_reply(app: &mut CodexApp, ticket: Ticket, reply: Reply, fx: &mut AppFx) {
    if let Some(address) = app.outgoing.address(ticket) {
        let resolved = if matches!(
            app.outgoing.context(ticket),
            Some(AppCtx::TurnOperation(TurnOperation::SafetyRetry { .. }))
        ) {
            let retry = app.safety_retry.as_ref();
            app.outgoing.resolve_if(ticket, reply, |ctx, answer| {
                matches!(ctx, AppCtx::TurnOperation(TurnOperation::SafetyRetry { generation, .. })
                    if retry.is_some_and(|run| run.accepts(ticket, *generation, answer)))
            })
        } else {
            app.outgoing.resolve(ticket, reply)
        };
        let Some((ctx, answer)) = resolved else {
            return;
        };
        let permissions = select::opening_permission_baseline(&ctx, &answer);
        match ctx {
            AppCtx::StartThread { switch } => {
                match (switch, answer) {
                    (
                        Some(backend),
                        Answer::Thread(ThreadReply::SessionStarted {
                            thread,
                            details,
                            sources,
                            ..
                        }),
                    ) => {
                        switch_to(app, backend, thread, details.as_deref(), sources, fx);
                    }
                    (Some(backend), Answer::Failed(error)) => {
                        // The oracle shows the error and stays where it was.
                        if app.screens.agents.is_cloud(backend) {
                            Rc::make_mut(&mut app.session).cloud_switch = Some(CloudSwitch::Failed);
                        }
                        if let Some(active) = app.session.active.clone() {
                            apply_thread(app, &active, ThreadEv::Failure(Failure::from(error)), fx);
                        }
                    }
                    (Some(_), _) => {}
                    (None, answer) => started(app, answer, fx),
                }
            }
            AppCtx::Agents(ctx) => screen_answer(app, address.backend, ctx, answer, fx),
            AppCtx::Resume(ctx) => resume_answer(app, ctx, answer, fx),
            AppCtx::Select(ctx) => {
                if let Answer::Screen(ScreenReply::Select(screens::select::Reply::Models(models))) =
                    &answer
                    && app
                        .image_restrictions
                        .get(&address.backend)
                        .is_none_or(|old| old.observed < ticket)
                    && match &ctx {
                        screens::select::Ctx::Models { request } => {
                            app.screens.select.catalog.pending == Some(*request)
                        }
                        screens::select::Ctx::Fallback { .. } => true,
                        _ => false,
                    }
                {
                    let restricted = models
                        .iter()
                        .filter(|model| !model.accepts_images)
                        .map(|model| model.model.clone())
                        .collect();
                    app.image_restrictions.insert(
                        address.backend,
                        ImageRestrictions {
                            address,
                            observed: ticket,
                            models: restricted,
                        },
                    );
                }
                select::answer(app, ctx, answer, fx);
            }
            AppCtx::RateLimits {
                backend,
                origin,
                superseded,
            } => {
                let credits = match &answer {
                    Answer::RateLimits { read, .. } => Ok(read.reset_credits.clone()),
                    _ => Err(()),
                };
                reset::read_done(app, backend, origin, credits, fx);
                if let Answer::RateLimits {
                    limits,
                    banner,
                    reserve,
                    read,
                } = answer
                    && !superseded
                {
                    supersede_rate_reads(app, backend, Some(ticket));
                    Rc::make_mut(&mut app.session)
                        .reserve
                        .insert(backend, reserve);
                    // a read a rolling update overtook does not replace the summary it wrote
                    let overtaken = app
                        .session
                        .rate_snapshots
                        .get(&backend)
                        .is_some_and(|store| store.limits().next_read_is_stale());
                    let told = status::RateRead {
                        overtaken,
                        active: active_backend(app) == backend,
                        ..(*read).clone()
                    };
                    if !overtaken {
                        install_rates(app, backend, limits);
                    }
                    let now = app.now.secs();
                    let warnings = Rc::make_mut(&mut app.session)
                        .rate_snapshots
                        .entry(backend)
                        .or_default()
                        .install(*read, now);
                    if active_backend(app) == backend {
                        reset::warn(app, warnings, fx);
                    }
                    install_banner(app, backend, banner.map(|banner| *banner));
                    banners::fallback(app, backend, fx);
                    features::on_rate_limits_read(app, backend, &told, fx);
                } else if !superseded
                    && let Some(store) = Rc::make_mut(&mut app.session)
                        .rate_snapshots
                        .get_mut(&backend)
                {
                    store.limits_mut().read_failed();
                }
                // The card that asked ends with what the app holds now: the new
                // read, or the cache when the read failed.
                if let status::RateLimitOrigin::Status(request) = origin {
                    finish_status_cells(app, backend, Some(request), fx);
                }
            }
            AppCtx::Redemption { key, .. } => {
                reset::answered(app, address.backend, &key, answer, fx)
            }
            AppCtx::Nudge { nudge, thread, .. } => {
                let outcome = match answer {
                    Answer::AddCreditsNudge(outcome) => outcome,
                    _ => chat::NudgeOutcome::Failed,
                };
                if let Some(thread) = thread {
                    let notice =
                        crate::threads::Item::Notice(crate::threads::Notice::AddCreditsNudge {
                            nudge,
                            outcome,
                        });
                    apply_thread(
                        app,
                        &thread,
                        ThreadEv::Appended {
                            turn: None,
                            item: notice,
                        },
                        fx,
                    );
                }
            }
            AppCtx::Removal { thread, action } => {
                // The server removed the thread of the chat: the chat is empty
                // again (the shown thread is decided here alone).
                if removed(app, &thread, action, answer, fx)
                    && app.session.active.as_ref() == Some(&thread)
                {
                    let session = Rc::make_mut(&mut app.session);
                    session.active = None;
                    session.removed += 1;
                }
            }
            AppCtx::NewConversation {
                spawned: Some(lifecycle::Spawned::PreparedFork(side)),
                ..
            } => {
                lifecycle::side_reply(app, side, address, ticket, answer, fx);
            }
            AppCtx::NewConversation {
                spawned: Some(lifecycle::Spawned::Rollback { thread, ctx }),
                ..
            } => {
                lifecycle::rolled_back(app, thread, ctx, answer, fx);
            }
            AppCtx::NewConversation {
                source,
                backend,
                name,
                plan,
                draft,
                spawned,
            } => {
                let result = match answer {
                    Answer::Thread(ThreadReply::SessionStarted {
                        thread,
                        details,
                        sources,
                        ..
                    }) => Ok((thread, details, sources)),
                    Answer::Thread(ThreadReply::Forked(thread) | ThreadReply::Started(thread)) => {
                        Ok((
                            ThreadId::new(&thread.thread.id),
                            Some(thread.thread),
                            Vec::new(),
                        ))
                    }
                    Answer::Failed(error) => Err(error),
                    _ => Err(RpcError::Mismatch),
                };
                match result {
                    Ok((thread, details, sources)) => {
                        attach_thread_with_metadata(
                            app,
                            &thread,
                            backend,
                            Origin::Started,
                            details.as_deref(),
                            fx,
                        );
                        note_instruction_sources(app, &thread, sources, fx);
                        if let Some(name) = name {
                            session::name_started(app, &thread, name, fx);
                        }
                        // The feature that asked for the thread is told before it is shown; an
                        // ephemeral fork starts empty: its history is the model's, not the
                        // screen's.
                        let ephemeral = matches!(
                            spawned,
                            Some(lifecycle::Spawned::Plain {
                                ephemeral: true,
                                ..
                            })
                        );
                        if let Some(lifecycle::Spawned::Plain { ctx, .. }) = spawned {
                            features::thread_opened(app, ctx, Ok(thread.clone()), fx);
                        }
                        if app.session.active == source {
                            show_held_thread(app, thread.clone(), fx);
                        }
                        if !ephemeral {
                            fetch_history(app, &thread, fx);
                        }
                        if let Some(markdown) = plan {
                            apply_thread(
                                app,
                                &thread,
                                ThreadEv::ScreenIntent(ScreenThreadIntent::Chat(
                                    chat::ThreadIntent::ApprovePlan(markdown),
                                )),
                                fx,
                            );
                        }
                        if let Some(text) = draft {
                            apply_thread(
                                app,
                                &thread,
                                ThreadEv::ScreenIntent(ScreenThreadIntent::Chat(
                                    chat::ThreadIntent::Draft(text),
                                )),
                                fx,
                            );
                        }
                    }
                    Err(error) => {
                        if let Some(lifecycle::Spawned::Plain { ctx, .. }) = spawned {
                            let refusal = crate::features::Refusal::Failed(error);
                            features::thread_opened(app, ctx, Err(refusal), fx);
                        } else if let Some(source) = source {
                            apply_thread(app, &source, ThreadEv::Failure(error.into()), fx);
                        }
                    }
                }
            }
            AppCtx::OpenThread {
                thread,
                cancelled: false,
                ..
            } => {
                screen_answer(
                    app,
                    address.backend,
                    agents::Ctx::Resume { thread },
                    answer,
                    fx,
                );
            }
            AppCtx::OpenThread {
                thread,
                backend,
                cancelled: true,
            } => {
                // A successful resume still attached a writer on the server.
                // Keep it without showing it, so a later open need not resume again.
                if let Answer::Thread(ThreadReply::Resumed(resumed)) = answer {
                    attach_thread_with_metadata(
                        app,
                        &thread,
                        backend,
                        Origin::Resumed,
                        Some(&resumed.thread),
                        fx,
                    );
                    apply_thread(app, &thread, threads::history_page_of(&resumed.thread), fx);
                    let read = if app
                        .threads
                        .get(&thread)
                        .is_some_and(|model| model.reconciling_submissions())
                    {
                        apply_thread(
                            app,
                            &thread,
                            ThreadEv::StreamRestored {
                                read: false,
                                reconciled: false,
                            },
                            fx,
                        );
                        agents::HistoryRead::SubmissionRecovery
                    } else {
                        agents::HistoryRead::Latest
                    };
                    request_history(app, &thread, None, read, fx);
                }
            }
            AppCtx::Worktree { step, .. } => step.advance(app, address, ticket, answer, fx),
            AppCtx::SessionRead => {
                // An id nothing is saved under (or a server that cannot read
                // it) ends the launch with the oracle's message.
                let read = match answer {
                    Answer::Thread(ThreadReply::Read { thread, .. }) => {
                        Ok(crate::screens::onboarding::session::Found {
                            cwd: Some(thread.cwd.display().to_string()),
                            thread: Some(crate::screens::onboarding::session::Resumed {
                                provider: thread.model_provider.clone(),
                                loaded: thread.status
                                    != codex_app_server_protocol::ThreadStatus::NotLoaded,
                            }),
                        })
                    }
                    _ => Err(()),
                };
                host_asks::step(app, fx, |screens, fx| {
                    crate::screens::onboarding::session::read(&mut screens.onboarding, read, fx);
                });
            }
            AppCtx::OpenSession { action, thread } => {
                session_opened(app, action, thread, answer, fx);
            }
            AppCtx::Opening {
                thread,
                ctx,
                attached_to,
            } => {
                lifecycle::read(app, thread, ctx, attached_to, (address, ticket), answer, fx);
            }
            AppCtx::TurnOperation(TurnOperation::Structured { run, .. }) => {
                structured::answered(app, run, answer, fx);
            }
            AppCtx::TurnOperation(TurnOperation::SafetyRetry { generation, .. }) => {
                safety_retry::answered(app, ticket, generation, address, answer, fx);
            }
            AppCtx::Feature(ctx) => features::answer(app, ctx, answer, fx),
        }
        if let Some((thread, snapshot)) = permissions {
            select::record_permission_settings(
                app,
                &thread,
                screens::select::permissions::Effective {
                    address,
                    at: ticket,
                    snapshot,
                },
                fx,
            );
        }
        return;
    }
    // A thread RPC: the thread whose model holds the ticket takes it; a
    // ticket nobody holds is stale and dropped.
    if let Some((thread, _)) = app.threads.request(ticket) {
        let thread = thread.clone();
        apply_thread(app, &thread, ThreadEv::Reply { ticket, reply }, fx);
    }
}

/// The workspace owner is e-mailed (`account/sendAddCreditsNudgeEmail`): the one path of the
/// banner's notify-owner action and of a feature's `PublicVerb::NotifyOwner`. One e-mail at a
/// time, as the oracle keeps one request in flight; the notice goes to the thread that was open.
fn notify_owner(app: &mut CodexApp, backend: BackendId, nudge: chat::Nudge, fx: &mut AppFx) {
    if owner_nudge_in_flight(app) {
        return;
    }
    let thread = app.session.active.clone();
    fx.ask(
        app,
        AppCtx::Nudge {
            backend,
            nudge,
            thread,
        },
        |_, ticket| Command::SendAddCreditsNudge {
            ticket,
            backend,
            nudge,
        },
    );
}

fn owner_nudge_in_flight(app: &CodexApp) -> bool {
    app.outgoing.any(|ctx| matches!(ctx, AppCtx::Nudge { .. }))
}

fn apply_intent(app: &mut CodexApp, intent: Intent, fx: &mut AppFx) {
    match intent {
        Intent::DismissBackendBanner { backend, ticket } => {
            if let Some(banner) = Rc::make_mut(&mut app.session)
                .backend_banners
                .get_mut(&backend)
            {
                banner.dismiss(ticket);
            }
        }
        Intent::PickBannerCta {
            backend,
            ticket,
            row,
        } => {
            let action = app
                .session
                .backend_banners
                .get(&backend)
                .filter(|occurrence| occurrence.ticket == ticket)
                .and_then(|occurrence| occurrence.banner.ctas().into_iter().nth(row))
                .map(|cta| cta.action);
            // The e-mail to the owner and the reset-credit flow are the
            // usage chunk's; a url opens in the browser, and the banner stays
            // (oracle: `dismiss_on_select: false`), but the Luna selector
            // closes on every pick (`picker_dismissed`).
            match action {
                Some(chat::BannerAction::OpenUrl(url)) => {
                    fx.host(effects::Ask::open_url_or_report(url))
                }
                Some(chat::BannerAction::NotifyOwner(nudge)) => {
                    notify_owner(app, backend, nudge, fx);
                }
                _ => {}
            }
            if let Some(occurrence) = Rc::make_mut(&mut app.session)
                .backend_banners
                .get_mut(&backend)
                && occurrence.banner.is_reserve()
            {
                occurrence.dismiss(ticket);
            }
        }
        Intent::Submit { thread, message } => {
            let target = thread.or_else(|| app.session.active.clone());
            match target {
                Some(thread) => submit(app, &thread, message, fx),
                None => {
                    // Before the first thread: kept until it exists; the
                    // composer's draft clears (the blank projection's ticket).
                    queue_before_thread(app, BeforeThread::Message(message), fx);
                }
            }
        }
        Intent::Screen(ScreenIntent::Popovers(crate::screens::popovers::Intent::ListApps)) => {
            mentions::list_apps(app, fx)
        }
        // the task search is the thread in use's: its RPCs carry the thread
        Intent::Screen(ScreenIntent::Popovers(crate::screens::popovers::Intent::SearchTasks(
            query,
        ))) => {
            if let Some(thread) = app.session.active.clone() {
                let ev = crate::screens::popovers::ThreadEv::SearchTasks {
                    query,
                    current: thread.clone(),
                };
                apply_thread(
                    app,
                    &thread,
                    ThreadEv::Screen(ScreenThreadEv::Popovers(ev)),
                    fx,
                );
            }
        }
        Intent::Screen(ScreenIntent::Usage(intent)) if reset::intent(app, &intent, fx) => {}
        Intent::Screen(ScreenIntent::Usage(crate::screens::usage::Intent::RefreshRateLimits)) => {
            refresh_rates(
                app,
                active_backend(app),
                status::RateLimitOrigin::UsageMenu,
                fx,
            )
        }
        Intent::Interrupt(thread) => {
            apply_thread(app, &thread, ThreadEv::Interrupt, fx);
            features::interrupted(app, &thread, fx);
        }
        Intent::ThreadScreen {
            thread,
            intent: ScreenThreadIntent::ApplyAdjacentPermissions { forward },
        } => {
            select::cycle_permissions(app, &thread, forward, fx);
        }
        Intent::ThreadScreen { thread, intent } => {
            if let Err(intent) = features::finish_queue_modal(app, &thread, intent, fx) {
                apply_thread(app, &thread, ThreadEv::ScreenIntent(intent), fx);
            }
        }
        Intent::AutoReview { thread, what } => {
            let ev = match what {
                AutoReview::NoDenials => approvals::ThreadEv::NoDenials,
                AutoReview::Approve { id } => approvals::ThreadEv::ApproveDenied { id },
            };
            apply_thread(
                app,
                &thread,
                ThreadEv::Screen(ScreenThreadEv::Approvals(ev)),
                fx,
            );
        }
        Intent::CycleBackend => consent::switch(app, fx),
        Intent::DismissRequest(key) => {
            if app.requests.contains(&key) {
                remove_request(app, &key);
                fx.commands.push(Command::RejectRequest { key });
            }
        }
        Intent::AnswerRequest { key, answer } => {
            resolve_request(app, key, answer, fx);
        }
        // Verifying is not a response: the device signs first (oracle `start_user_verification`).
        Intent::Screen(ScreenIntent::Approvals(approvals::Intent::Answer {
            key,
            decision: approvals::Decision::Verify,
        })) => start_verification(app, key, fx),
        Intent::Screen(ScreenIntent::Approvals(approvals::Intent::Answer { key, decision })) => {
            resolve_request(app, key, approvals::RequestAnswer::Choice(decision), fx);
        }
        Intent::Screen(ScreenIntent::Approvals(approvals::Intent::Respond { key, answer })) => {
            resolve_request(app, key, answer, fx);
        }
        Intent::Screen(ScreenIntent::Approvals(approvals::Intent::OpenLink { key })) => {
            if let Some(link) = app_link(app, &key) {
                fx.host(effects::Ask::open_url_or_report(link.url));
            }
        }
        Intent::Screen(ScreenIntent::Approvals(approvals::Intent::LinkDone { key })) => {
            if app_link(app, &key).is_some_and(|link| link.refreshes_apps()) {
                select::intent(app, crate::screens::select::Intent::RefreshApps, fx);
            }
            accept_link(app, key, fx);
        }
        Intent::Screen(ScreenIntent::Approvals(approvals::Intent::LinkEnable { key })) => {
            if let Some(link) = app_link(app, &key) {
                let intent = crate::screens::select::Intent::app_action(
                    crate::screens::select::apps::ModalAction::Enable {
                        id: link.app_id,
                        enabled: true,
                    },
                );
                select::intent(app, intent, fx);
            }
            accept_link(app, key, fx);
        }
        Intent::Screen(ScreenIntent::Agents(agents::Intent::CancelOpen)) => {
            consent::cancel_center_action(app, fx);
            if let Some(id) = Rc::make_mut(&mut app.screens).agents.opening.take() {
                set_open_cancelled(app, &id, true);
            }
        }
        Intent::Screen(ScreenIntent::Agents(agents::Intent::ChooseDescendant {
            picker,
            thread,
        })) => {
            if app.screens.agents.can_choose_descendant(
                app.session.active.as_ref(),
                &picker,
                &thread,
            ) {
                open_thread(app, &thread, fx);
            }
        }
        Intent::Screen(ScreenIntent::Agents(intent)) => {
            if let agents::Intent::ReadDescendants {
                active, root_visit, ..
            } = &intent
            {
                let agents = &app.screens.agents;
                if *root_visit != agents.descendant_root_visit
                    || agents.descendant_root.is_none()
                    || agents.descendant_root != agents.descendants_root(active)
                {
                    return;
                }
            }
            let screens = Rc::make_mut(&mut app.screens);
            let outs = agents::update(&mut screens.agents, intent);
            send_agents(app, outs, fx);
        }
        Intent::LoadOlderHistory(id) => {
            if let Some(cursor) = app
                .threads
                .get(&id)
                .and_then(|thread| thread.history_cursor())
                && !app.outgoing.any(|ctx| {
                    matches!(ctx,
                    AppCtx::Agents(agents::Ctx::History { thread, .. }) if thread == id.as_str())
                })
            {
                request_history(
                    app,
                    &id,
                    Some(cursor.to_owned()),
                    agents::HistoryRead::Older,
                    fx,
                );
            } else if app
                .threads
                .get(&id)
                .is_some_and(|thread| thread.can_load_voice_history())
            {
                apply_thread(
                    app,
                    &id,
                    threads::voice_transcript::ThreadEvent::LoadOlder.into(),
                    fx,
                );
            }
        }
        Intent::OpenThread(id) => {
            open_thread(app, &id, fx);
        }
        Intent::Effect { request, ask } => fx.commands.push(effects::command(request, ask)),
        Intent::CancelRequests => cancel_requests(app, fx),
        Intent::Screen(ScreenIntent::Resume(intent)) => resume_picker(app, intent, fx),
        // The page the command line opened instead of a thread was left.
        Intent::Screen(ScreenIntent::Onboarding(
            crate::screens::onboarding::Intent::StartPageLeft,
        )) => {
            start_fresh_thread(app, fx);
        }
        Intent::Screen(ScreenIntent::Chat(chat::Intent::Session {
            thread,
            command,
            intent,
        })) => session_intent(app, thread, command, intent, fx),
        Intent::Screen(ScreenIntent::Chat(chat::Intent::Note { thread, note })) => {
            session_note(app, thread, note, fx)
        }
        Intent::Screen(ScreenIntent::Chat(chat::Intent::Shell { thread, command })) => {
            match thread.or_else(|| app.session.active.clone()) {
                Some(thread) => run_shell(app, &thread, command, fx),
                // Before the first thread: kept in order with the messages.
                None => queue_before_thread(app, BeforeThread::Shell(command), fx),
            }
        }
        // a verb of the thread's safety state: the thread's own, not the screen's
        Intent::Screen(ScreenIntent::Chat(chat::Intent::Safety {
            thread,
            verb: crate::threads::safety::Verb::Retry(offer),
        })) => safety_retry::begin(app, thread, offer, fx),
        Intent::Screen(ScreenIntent::Chat(chat::Intent::Safety { thread, verb })) => {
            if matches!(&verb, crate::threads::safety::Verb::Continue { .. })
                && app.session.active.as_ref() != Some(&thread)
            {
                return;
            }
            let ev = ThreadEv::Screen(ScreenThreadEv::Chat(chat::ThreadEv::Safety(
                crate::threads::safety::SafetyEv::Verb(verb),
            )));
            apply_thread(app, &thread, ev, fx);
        }
        Intent::Screen(ScreenIntent::Select(intent)) => {
            select::intent(app, intent, fx);
            confirmed(app, fx);
        }
        Intent::Screen(
            intent @ (ScreenIntent::Onboarding(_)
            | ScreenIntent::Startup(_)
            | ScreenIntent::Import(_)),
        ) => {
            host_asks::step(app, fx, |screens, fx| {
                screens::apply_intent(screens, intent, fx)
            });
        }
        Intent::Screen(ScreenIntent::Voice(intent)) => {
            if let Some(origin) = app
                .session
                .active
                .as_ref()
                .and_then(|thread| app.voice_settings_origin(thread))
                && let Some(request) = Rc::make_mut(&mut app.screens).voice.settings.begin(
                    app.mint.next(),
                    origin,
                    intent,
                )
            {
                send_voice_settings(app, request, fx);
            }
        }
        Intent::Screen(intent) => screens::apply_intent(Rc::make_mut(&mut app.screens), intent, fx),
        Intent::Feature(intent) => features::intent(app, intent, fx),
        Intent::Interest(step) => features::interest(app, step, fx),
        Intent::WishShown(seq) => features::wish_shown(app, seq),
    }
}

/// The `/status` cells of `backend`'s threads that still wait end with the
/// limits the app holds: those of `request` only, or all of them (`None`).
fn finish_status_cells(
    app: &mut CodexApp,
    backend: BackendId,
    request: Option<u64>,
    fx: &mut AppFx,
) {
    let limits = card_limits(app, backend, false);
    let waiting: Vec<(ThreadId, u64)> = app
        .threads
        .iter()
        .filter(|(id, _)| app.screens.agents.owner_of(id.as_str()) == backend)
        .flat_map(|(id, model)| {
            model
                .transcript()
                .waiting_status()
                .into_iter()
                .filter(move |waiting| request.is_none_or(|request| request == *waiting))
                .map(move |waiting| (id.clone(), waiting))
        })
        .collect();
    for (thread, request) in waiting {
        let limits = limits.clone();
        apply_thread(
            app,
            &thread,
            ThreadEv::Screen(ScreenThreadEv::from(status::ThreadEv::Refreshed {
                request,
                limits,
            })),
            fx,
        );
    }
}

/// A failed turn's error in the words a feature says for it, when one does
/// (`Feature::fault_words`); the server's own words stand otherwise.
/// A multi-agent row names its agents as the app knows them when the row is made (oracle
/// `AgentMetadata`): the nickname and role of the thread, read from the agents' list.
fn named(app: &mut CodexApp, thread: &ThreadId, ev: ThreadEv) -> ThreadEv {
    let requested = match &ev {
        ThreadEv::ItemUpsert { key, .. } => app
            .spawn_requests
            .remove(&(thread.as_str().to_owned(), key.item.as_str().to_owned())),
        _ => None,
    };
    let lookup = |id: &str| {
        app.screens
            .agents
            .threads
            .iter()
            .find(|thread| thread.id == id)
            .map_or((None, None), |thread| {
                (thread.agent_nickname.clone(), thread.agent_role.clone())
            })
    };
    let rename = |item: threads::Item| match item {
        threads::Item::Collab(event) => {
            let mut event = event.named(&lookup);
            if let threads::CollabEvent::Spawned { request, .. } = &mut event
                && request.is_none()
            {
                *request = requested.clone();
            }
            threads::Item::Collab(std::sync::Arc::new(event))
        }
        item => item,
    };
    match ev {
        ThreadEv::ItemUpsert { key, item, receipt } => ThreadEv::ItemUpsert {
            key,
            receipt,
            item: rename(item),
        },
        ThreadEv::ItemStarted { key, item, receipt } => ThreadEv::ItemStarted {
            key,
            receipt,
            item: rename(item),
        },
        ev => ev,
    }
}

fn reworded(app: &CodexApp, thread: &ThreadId, ev: ThreadEv) -> ThreadEv {
    let ThreadEv::TurnError {
        turn,
        fault: threads::Fault::Message(message),
    } = ev
    else {
        return ev;
    };
    let message = features::fault_words(app, thread, &message).unwrap_or(message);
    ThreadEv::TurnError {
        turn,
        fault: threads::Fault::Message(message),
    }
}

/// What the user asked, when `notification` completes a user message of `thread` that has no
/// name yet and says something.
fn unnamed_request(
    app: &CodexApp,
    thread: &ThreadId,
    notification: &ServerNotification,
) -> Option<String> {
    let ServerNotification::ItemCompleted(done) = notification else {
        return None;
    };
    let ThreadItemPayload::UserMessage { content, .. } = &done.item.payload else {
        return None;
    };
    let unnamed = app
        .threads
        .get(thread)
        .is_some_and(|model| model.screen_slots().chat.name.is_none());
    let text = threads::request_text(content);
    (unnamed && !text.trim().is_empty()).then_some(text)
}

fn install_rates(app: &mut CodexApp, backend: BackendId, limits: status::RateLimits) {
    Rc::make_mut(&mut app.session)
        .rate_limits
        .insert(backend, limits);
}

/// Already-issued reads cannot publish an old remedy after a limit transition, or overwrite
/// what a newer successful read published. A visible card or picker still settles its own ask.
fn supersede_rate_reads(app: &mut CodexApp, backend: BackendId, before: Option<Ticket>) {
    for (ticket, ctx) in app.outgoing.iter_mut() {
        if let AppCtx::RateLimits {
            backend: read,
            superseded,
            ..
        } = ctx
            && *read == backend
            && before.is_none_or(|newer| ticket < newer)
        {
            *superseded = true;
        }
    }
}

fn supersede_rate_banner(app: &mut CodexApp, backend: BackendId) {
    install_banner(app, backend, None);
    supersede_rate_reads(app, backend, None);
}

fn install_banner(app: &mut CodexApp, backend: BackendId, banner: Option<chat::Banner>) {
    let banners = &mut Rc::make_mut(&mut app.session).backend_banners;
    let Some(banner) = banner else {
        banners.remove(&backend);
        return;
    };
    if let Some(current) = banners.get_mut(&backend)
        && current.banner.same_occurrence(&banner)
    {
        current.banner = banner;
    } else {
        banners.insert(
            backend,
            chat::BannerOccurrence::new(banner, app.mint.next()),
        );
    }
}

/// A change of the banner occurrence `backend` shows (`app/banners.rs`); none shown, none made.
fn with_banner(
    app: &mut CodexApp,
    backend: BackendId,
    change: impl FnOnce(&mut chat::BannerOccurrence),
) {
    if app.session.backend_banners.contains_key(&backend)
        && let Some(occurrence) = Rc::make_mut(&mut app.session)
            .backend_banners
            .get_mut(&backend)
    {
        change(occurrence);
    }
}

fn remove_request(app: &mut CodexApp, key: &RequestKey) {
    app.effects.verifications.remove(key);
    Rc::make_mut(&mut app.requests)
        .entries
        .retain(|entry| &entry.key != key);
}

fn resolve_request(
    app: &mut CodexApp,
    key: RequestKey,
    answer: approvals::RequestAnswer,
    fx: &mut AppFx,
) -> bool {
    let Some(entry) = app.requests.get(&key) else {
        return false;
    };
    // What the answer leaves in the thread's history (the oracle's decision cell, the result
    // cell of a question): only for the requests it records, none for a thread that is not the
    // one shown.
    let recorded = entry
        .prompt
        .as_deref()
        .and_then(|prompt| approvals::recorded(prompt, &answer));
    let Some(result) = requests::answer(&entry.request, entry.prompt.as_deref(), answer) else {
        return false;
    };
    remove_request(app, &key);
    if let Some(recorded) = recorded {
        let ev = ThreadEv::Screen(ScreenThreadEv::Approvals(recorded));
        apply_thread(app, &key.thread, ev, fx);
    }
    fx.commands.push(Command::ResolveRequest { key, result });
    true
}

/// The app link request `key` holds, while it is queued.
fn app_link(app: &CodexApp, key: &RequestKey) -> Option<approvals::link::AppLinkPrompt> {
    match app.requests.get(key)?.prompt.as_deref()? {
        approvals::Prompt::AppLink(link) => Some(link.clone()),
        _ => None,
    }
}

/// An app link request is accepted with no content (`resolve_elicitation(Accept, None, None)`).
fn accept_link(app: &mut CodexApp, key: RequestKey, fx: &mut AppFx) {
    let accept = approvals::RequestAnswer::Elicitation {
        decision: approvals::Decision::Accept,
        content: None,
        meta: None,
    };
    resolve_request(app, key, accept, fx);
}

/// The user chose to verify: one attempt per request, the host runs the device and answers with
/// `AppEv::Verified`; the dialog shows the waiting state meanwhile.
fn start_verification(app: &mut CodexApp, key: RequestKey, fx: &mut AppFx) {
    if !app.requests.contains(&key) {
        return;
    }
    let Some(attempt) = app.effects.verifications.begin(&key) else {
        return;
    };
    let queue = Rc::make_mut(&mut app.requests);
    if let Some(prompt) = queue
        .entries
        .iter_mut()
        .find(|entry| entry.key == key)
        .and_then(|entry| entry.prompt.as_mut())
        && let approvals::Prompt::UserVerification(verification) = Rc::make_mut(prompt)
    {
        verification.waiting = true;
    }
    fx.host(effects::Ask::VerifyUser {
        key,
        attempt: attempt.id,
        params: attempt.params,
    });
}

/// The device's result: only the attempt that is still pending may answer the request (a late
/// result of one cancelled, resolved or left behind by a reconnect is dropped).
fn verified(
    app: &mut CodexApp,
    key: RequestKey,
    attempt: effects::Attempt,
    result: effects::Verified,
    fx: &mut AppFx,
) {
    use approvals::verify::{Failure as Refused, Outcome};
    if !app.effects.verifications.is_pending(&key, attempt) {
        return;
    }
    let outcome = match result {
        effects::Verified::Proof(proof) => Outcome::Accepted(proof),
        failed => {
            let refused = match failed {
                effects::Verified::Failed(data) => Refused::of_server(data.as_ref()),
                _ => Refused::unreachable(),
            };
            app.effects.record_failure(key.clone(), refused.cause);
            Outcome::Failed(refused.details)
        }
    };
    if let Some(answer) = outcome.answer() {
        resolve_request(app, key, answer, fx);
    }
}

/// Ctrl+C on the approval dialog: the request shown is cancelled (answered with its cancelling
/// decision) and the dialog's own queue is dropped unanswered, as the oracle's overlay does
/// (`ApprovalOverlay::cancel_current_request` clears its queue); questions are not approvals and
/// stay.
fn cancel_requests(app: &mut CodexApp, fx: &mut AppFx) {
    let asks_approval = |entry: &PendingRequest| {
        !matches!(
            entry.prompt.as_deref(),
            Some(approvals::Prompt::UserInput(_))
        )
    };
    let approvals: Vec<PendingRequest> = app
        .requests
        .entries
        .iter()
        .filter(|entry| asks_approval(entry))
        .cloned()
        .collect();
    let Some((current, queued)) = approvals.split_first() else {
        return;
    };
    for entry in queued {
        remove_request(app, &entry.key);
    }
    let cancel = current
        .prompt
        .as_deref()
        .and_then(approvals::Prompt::cancel_decision);
    if let Some(decision) = cancel {
        resolve_request(
            app,
            current.key.clone(),
            approvals::RequestAnswer::Choice(decision),
            fx,
        );
    }
}

/// The resume picker: opening it lists the sessions of every connected
/// backend, the toolbar and the search ask for new listings, a pick opens
/// that thread through `OpenThread` (ADR-003: the picker never resumes).
fn resume_picker(app: &mut CodexApp, intent: resume::Intent, fx: &mut AppFx) {
    let opening = matches!(intent, resume::Intent::Open);
    let screens = Rc::make_mut(&mut app.screens);
    if opening {
        let agents = &screens.agents;
        let mut sources: Vec<BackendId> = agents.clouds.iter().copied().collect();
        sources.push(BackendId::default());
        sources.push(agents.displayed);
        sources.sort();
        sources.dedup();
        screens.resume.sources = sources;
    }
    let outs = resume::update(&mut screens.resume, intent);
    send_resume(app, outs, fx);
    let model = &mut Rc::make_mut(&mut app.screens).resume;
    if let Some(id) = model.resuming.take().or_else(|| model.restored.take()) {
        let backend = app
            .screens
            .resume
            .row(&id)
            .map_or(app.screens.agents.displayed, |row| row.backend);
        Rc::make_mut(&mut app.screens)
            .agents
            .observe(backend, id.as_str());
        open_thread(app, &id, fx);
    }
}

/// Mints a ticket for each RPC the picker asks for and sends it.
fn send_resume(app: &mut CodexApp, outs: Vec<resume::Out>, fx: &mut AppFx) {
    for out in outs {
        let command = match out.ask {
            resume::Ask::Rpc(command) => ScreenCommand::Resume(command),
            // A thread's turns are the thread's own RPC (ADR-003): asked as
            // its history, on the backend that owns the thread.
            resume::Ask::Turns { backend, thread } => {
                Rc::make_mut(&mut app.screens)
                    .agents
                    .observe(backend, thread.as_str());
                ScreenCommand::Agents(agents::Command::Thread(ThreadAsk::History {
                    thread: thread.to_string(),
                    cursor: None,
                }))
            }
        };
        fx.ask(app, AppCtx::Resume(out.ctx), |_, ticket| Command::Screen {
            ticket,
            thread: ThreadId::default(),
            command,
        });
    }
}

/// The reply of an RPC of the picker.
fn resume_answer(app: &mut CodexApp, ctx: resume::Ctx, answer: Answer, fx: &mut AppFx) {
    let result = match answer {
        Answer::Screen(ScreenReply::Resume(reply)) => Ok(reply),
        Answer::Thread(ThreadReply::History { turns, .. }) => Ok(resume::Reply::Turns(turns)),
        Answer::Failed(error) => Err(error),
        _ => Err(RpcError::Mismatch),
    };
    let outs = resume::apply_answer(&mut Rc::make_mut(&mut app.screens).resume, ctx, result);
    send_resume(app, outs, fx);
    if let Some(id) = Rc::make_mut(&mut app.screens).resume.restored.take() {
        open_thread(app, &id, fx);
    }
}

/// The sole decision to use a held writer, await an existing resume or
/// ask for an attachment. An unattached local task needs its folder checked first.
fn open_thread(app: &mut CodexApp, id: &ThreadId, fx: &mut AppFx) {
    let model = &app.screens.agents;
    if !is_listed(app, id)
        || app.preparing_forks.contains_key(id)
        || model.held()
        || model.failure.is_some()
        || folder_consent::pending(&app.screens.onboarding)
    {
        return;
    }
    if thread_held(app, id)
        || model.owner_of(id.as_str()) != BackendId::default()
        || model.opening.as_ref() == Some(id)
    {
        open_verified(app, id, fx);
        return;
    }
    consent::begin(
        app,
        consent::Action::Open(id.clone()),
        ConsentTarget::Task(id.clone()),
        TrustCancel::AgentsOverview,
        fx,
    );
}

/// This process holds the thread or a list (the resume picker, the command center) has its row.
fn is_listed(app: &CodexApp, id: &ThreadId) -> bool {
    let model = &app.screens.agents;
    app.threads.contains(id)
        || app.screens.resume.row(id).is_some()
        || model
            .threads
            .iter()
            .chain(&model.search_matches)
            .any(|row| row.id == id.as_str())
}

/// `open_thread` for a thread the server is known to have.
fn open_verified(app: &mut CodexApp, id: &ThreadId, fx: &mut AppFx) -> bool {
    open_verified_at(app, id, None, fx)
}

fn open_verified_at(
    app: &mut CodexApp,
    id: &ThreadId,
    cwd: Option<String>,
    fx: &mut AppFx,
) -> bool {
    let model = &app.screens.agents;
    if model.held() || model.failure.is_some() || app.preparing_forks.contains_key(id) {
        return false;
    }
    let held = thread_held(app, id);
    if let Some(previous) = model.opening.clone().filter(|previous| previous != id) {
        set_open_cancelled(app, &previous, true);
    }
    let agents = &mut Rc::make_mut(&mut app.screens).agents;
    if held {
        agents::attach_loaded(agents, id.as_str());
        show_held_thread(app, id.clone(), fx);
        return true;
    }
    agents.opening = Some(id.clone());
    if !set_open_cancelled(app, id, false) {
        let backend = app.screens.agents.owner_of(id.as_str());
        let context = AppCtx::OpenThread {
            thread: id.clone(),
            backend,
            cancelled: false,
        };
        let ask = if cwd.is_some() {
            ThreadAsk::ResumeAt {
                thread: id.to_string(),
                cwd,
            }
        } else {
            ThreadAsk::Resume {
                thread: id.to_string(),
            }
        };
        fx.ask(app, context, |_, ticket| Command::Screen {
            ticket,
            thread: ThreadId::default(),
            command: ScreenCommand::Agents(agents::Command::Thread(ask)),
        });
    }
    true
}

/// A live writer can be reused while its history is current or the recovery page is in flight.
fn thread_held(app: &CodexApp, id: &ThreadId) -> bool {
    app.threads.get(id).is_some_and(|thread| {
        thread.has_writer()
            && !thread.stream_lost()
            && (!thread.history_may_be_incomplete()
                || app.outgoing.any(|ctx| {
                    matches!(
                        ctx,
                        AppCtx::Agents(agents::Ctx::History {
                            thread: requested,
                            read: agents::HistoryRead::SubmissionRecovery,
                        }) if requested == id.as_str()
                    )
                }))
    })
}

fn set_open_cancelled(app: &mut CodexApp, id: &ThreadId, value: bool) -> bool {
    let Some(ctx) = app.outgoing.find_mut(|ctx| {
        matches!(
            ctx, AppCtx::OpenThread { thread, .. } if thread == id
        )
    }) else {
        return false;
    };
    if let AppCtx::OpenThread { cancelled, .. } = ctx {
        *cancelled = value;
    }
    true
}

fn fetch_history(app: &mut CodexApp, id: &ThreadId, fx: &mut AppFx) {
    request_history(app, id, None, agents::HistoryRead::Latest, fx);
}

fn request_history(
    app: &mut CodexApp,
    id: &ThreadId,
    cursor: Option<String>,
    read: agents::HistoryRead,
    fx: &mut AppFx,
) {
    if read == agents::HistoryRead::Older {
        apply_thread(app, id, ThreadEv::HistoryRequested, fx);
    }
    send_agents(
        app,
        vec![agents::Out {
            command: agents::Command::Thread(ThreadAsk::History {
                thread: id.to_string(),
                cursor,
            }),
            ctx: agents::Ctx::History {
                thread: id.to_string(),
                read,
            },
        }],
        fx,
    );
}

/// Mints a ticket for each RPC the agents screen asks for and sends it. The
/// RPCs are app-scoped: the command carries no thread.
fn send_agents(app: &mut CodexApp, outs: Vec<agents::Out>, fx: &mut AppFx) {
    for out in outs {
        match out.command {
            agents::Command::Thread(ThreadAsk::Start { cwd })
                if out.ctx == agents::Ctx::Start
                    && app.screens.agents.displayed == BackendId::default() =>
            {
                let cwd = cwd.or_else(|| app.screens.chat.status.cwd.clone());
                consent::begin(
                    app,
                    consent::Action::New { cwd: cwd.clone() },
                    ConsentTarget::Folder(cwd),
                    TrustCancel::AgentsOverview,
                    fx,
                );
                continue;
            }
            agents::Command::NewWorktree { cwd } => {
                worktree::Step::create(app, cwd, fx);
                continue;
            }
            agents::Command::SaveGrouping { grouping } => {
                fx.host(effects::Ask::SaveGrouping { grouping });
                continue;
            }
            _ => {}
        }
        // The shell drops the search before a new one (it never answers):
        // its ticket is released here (R5).
        if matches!(out.command, agents::Command::Search { .. }) {
            app.outgoing
                .retain(|ctx| !matches!(ctx, AppCtx::Agents(agents::Ctx::Search { .. })));
        }
        send_agent_command(app, out, fx);
    }
}

fn send_agent_command(app: &mut CodexApp, out: agents::Out, fx: &mut AppFx) {
    fx.ask(app, AppCtx::Agents(out.ctx), |_, ticket| Command::Screen {
        ticket,
        thread: ThreadId::default(),
        command: ScreenCommand::Agents(out.command),
    });
}

/// A resume the server refused for the thread's model provider: the thread says why.
fn provider_refusal(app: &mut CodexApp, thread: &ThreadId, error: &RpcError, fx: &mut AppFx) {
    if let RpcError::Provider(error) = error {
        let ev = ThreadEv::Screen(ScreenThreadEv::Chat(chat::ThreadEv::ProviderError(
            error.clone(),
        )));
        apply_thread(app, thread, ev, fx);
    }
}

/// An already known conversation can still be shown from cache if its non-embedded backend
/// answered the resume without attaching it.
fn can_show_unavailable(app: &CodexApp, thread: &ThreadId) -> bool {
    let backend = app.screens.agents.owner_of(thread.as_str());
    let non_embedded = app.screens.agents.is_cloud(backend)
        || backend == BackendId::default() && app.screens.status.remote.is_some();
    non_embedded
        && app.session.connections.get(&backend) == Some(&ConnectionState::Connected)
        && app.threads.get(thread).is_some_and(|held| {
            !held.read_only()
                && held.provider_error().is_none()
                && (!held.has_writer() || held.stream_lost())
        })
}

fn agents_answer(answer: Answer) -> Result<agents::Reply, RpcError> {
    match answer {
        Answer::Screen(ScreenReply::Agents(reply)) => Ok(reply),
        Answer::Thread(reply) => crate::protocol::agents::from_thread(reply),
        Answer::Failed(error) => Err(error),
        _ => Err(RpcError::Mismatch),
    }
}

/// The reply of an app-scoped screen RPC.
fn screen_answer(
    app: &mut CodexApp,
    backend: BackendId,
    ctx: agents::Ctx,
    answer: Answer,
    fx: &mut AppFx,
) {
    if !app
        .screens
        .agents
        .accepts_descendants(&ctx, app.session.active.as_ref())
    {
        return;
    }
    let other_backend = app.screens.agents.displayed != backend;
    // An older start can still attach to its backend without taking the newly displayed view.
    let background_start =
        other_backend && matches!(ctx, agents::Ctx::Start | agents::Ctx::Fork { .. });
    let mut recovery_history = None;
    let result = agents_answer(answer);
    if other_backend {
        match ctx {
            agents::Ctx::List { append } => {
                agents::settle_parked_list(
                    &mut Rc::make_mut(&mut app.screens).agents,
                    backend,
                    append,
                    result,
                );
                return;
            }
            agents::Ctx::Search { .. } | agents::Ctx::Loaded => return,
            _ => {}
        }
    }
    if let (
        agents::Ctx::Resume { .. } | agents::Ctx::Fork { .. } | agents::Ctx::Start,
        Ok(
            agents::Reply::Resumed(thread)
            | agents::Reply::Forked(thread)
            | agents::Reply::Started(thread),
        ),
    ) = (&ctx, &result)
    {
        let id = ThreadId::new(&thread.id);
        // The thread is held and its stream was lost or has a gap (oracle `was_reconnecting`,
        // `refreshing_history`): the server's turns replace what is held, and what waited in the
        // queue goes on.
        let stale = matches!(ctx, agents::Ctx::Resume { .. })
            && app
                .threads
                .get(&id)
                .is_some_and(|held| held.stream_lost() || held.history_may_be_incomplete());
        if stale {
            apply_thread(app, &id, threads::history_reset(), fx);
        }
        attach_thread_with_metadata(app, &id, backend, Origin::Resumed, Some(thread), fx);
        if !background_start {
            show_held_thread(app, id.clone(), fx);
        }
        apply_thread(app, &id, threads::history_page_of(thread), fx);
        if stale {
            let reconciling = app
                .threads
                .get(&id)
                .is_some_and(|model| model.reconciling_submissions());
            if reconciling {
                recovery_history = Some(id.clone());
            }
            apply_thread(
                app,
                &id,
                ThreadEv::StreamRestored {
                    read: !reconciling,
                    reconciled: false,
                },
                fx,
            );
        }
        // `maybe_prompt_resume_paused_goal_after_resume`: a resumed thread's stopped goal asks.
        if matches!(ctx, agents::Ctx::Resume { .. }) {
            features::resumed(app, &id, fx);
            apply_thread(app, &id, ThreadEv::Command(threads::CommandEv::Restore), fx);
        }
    }
    if background_start {
        return;
    }
    if let (agents::Ctx::Resume { thread }, Err(error)) = (&ctx, &result)
        && agents::is_active_writer(error)
    {
        // Another process writes the thread, or its model provider is refused:
        // the chat shows it read-only.
        let id = thread.clone();
        app.threads.open(&id, Origin::ForeignWriter);
        let session = Rc::make_mut(&mut app.session);
        session.active = Some(id.clone());
        session.status = Session::Ready;
        provider_refusal(app, &id, error, fx);
    } else if let (agents::Ctx::Resume { thread }, Err(_)) = (&ctx, &result)
        && can_show_unavailable(app, thread)
    {
        app.threads.open(thread, Origin::Unavailable);
        agents::attach_loaded(&mut Rc::make_mut(&mut app.screens).agents, thread.as_str());
        show_held_thread(app, thread.clone(), fx);
        return;
    }
    if let (agents::Ctx::Fork { thread }, Err(_)) = (&ctx, &result) {
        let session = Rc::make_mut(&mut app.session);
        *session
            .fork_failed
            .entry(ThreadId::new(thread))
            .or_default() += 1;
    }
    if let agents::Ctx::History {
        thread,
        read: purpose,
    } = &ctx
    {
        let thread = ThreadId::new(thread);
        let read = if let Ok(agents::Reply::History { turns, next }) = &result {
            apply_thread(app, &thread, threads::history_page(turns, next.clone()), fx);
            if !app.screens.agents.is_cloud(backend)
                && app.screens.agents.owner_of(thread.as_str()) == backend
                && app
                    .threads
                    .get(&thread)
                    .is_some_and(|thread| thread.voice_history_supported())
            {
                apply_thread(
                    app,
                    &thread,
                    threads::voice_transcript::ThreadEvent::Read {
                        oldest: turns.last().map(|turn| TurnId::new(&turn.id)),
                        more_history: next.is_some(),
                    }
                    .into(),
                    fx,
                );
            }
            true
        } else {
            apply_thread(app, &thread, ThreadEv::HistoryFailed, fx);
            false
        };
        if *purpose == agents::HistoryRead::SubmissionRecovery
            && app.threads.get(&thread).is_some_and(|model| {
                model.has_writer() && !model.stream_lost() && model.reconciling_submissions()
            })
        {
            apply_thread(
                app,
                &thread,
                ThreadEv::StreamRestored {
                    read,
                    reconciled: true,
                },
                fx,
            );
        }
    }
    let mut outs = agents::apply_answer(
        &mut Rc::make_mut(&mut app.screens).agents,
        app.threads.previews_mut(),
        ctx,
        result,
    );
    if let Some(id) = recovery_history {
        for out in &mut outs {
            if let agents::Ctx::History { thread, read } = &mut out.ctx
                && thread == id.as_str()
                && *read == agents::HistoryRead::Latest
            {
                *read = agents::HistoryRead::SubmissionRecovery;
            }
        }
    }
    send_agents(app, outs, fx);
}

/// A user shell command on `thread`, a turn running or not.
fn run_shell(app: &mut CodexApp, thread: &ThreadId, command: String, fx: &mut AppFx) {
    let intent = ScreenThreadIntent::Chat(chat::ThreadIntent::Shell(command));
    apply_thread(app, thread, ThreadEv::ScreenIntent(intent), fx);
}

/// The first thread's answer.
fn started(app: &mut CodexApp, answer: Answer, fx: &mut AppFx) {
    match answer {
        Answer::Thread(ThreadReply::SessionStarted {
            thread,
            details,
            sources,
            ..
        }) => {
            attach_thread_with_metadata(
                app,
                &thread,
                BackendId::default(),
                Origin::Started,
                details.as_deref(),
                fx,
            );
            note_instruction_sources(app, &thread, sources, fx);
            let session = Rc::make_mut(&mut app.session);
            session.active = Some(thread.clone());
            session.status = Session::Ready;
            send_queued(app, &thread, fx);
        }
        Answer::Failed(error) => set_status(app, Session::Failed(Failure::from(error))),
        _ => {}
    }
}

/// The instruction files a thread's start named are the thread's (`/status`
/// lists them); an empty list says nothing.
fn note_instruction_sources(
    app: &mut CodexApp,
    thread: &ThreadId,
    sources: Vec<status::AgentsSource>,
    fx: &mut AppFx,
) {
    if sources.is_empty() {
        return;
    }
    let ev = ScreenThreadEv::from(status::ThreadEv::InstructionSources(sources));
    apply_thread(app, thread, ThreadEv::Screen(ev), fx);
}

/// What was asked before the first thread existed goes out on it.
fn send_queued(app: &mut CodexApp, thread: &ThreadId, fx: &mut AppFx) {
    let asks = std::mem::take(&mut app.queued_submits);
    if !asks.is_empty() {
        threads::apply(
            app.threads.blank_mut(),
            ThreadEv::Deferred(threads::Deferred::Sent),
            &mut ThreadFx::new(&ThreadId::default(), fx, &mut app.mint),
        );
    }
    for ask in asks {
        match ask {
            BeforeThread::Plan { command, prompt } => {
                session::plan_after_start(app, thread, command, prompt, fx)
            }
            BeforeThread::Message(message) => submit(app, thread, message, fx),
            BeforeThread::Initial(message) => {
                let text = message.text.clone();
                submit(app, thread, message.clone(), fx);
                apply_thread(
                    app,
                    thread,
                    ThreadEv::Deferred(threads::Deferred::Recorded(message)),
                    fx,
                );
                let popovers = &mut Rc::make_mut(&mut app.screens).popovers;
                screens::popovers::apply_intent(
                    popovers,
                    screens::popovers::Intent::Record(text),
                    fx,
                );
            }
            BeforeThread::Shell(command) => run_shell(app, thread, command, fx),
        }
    }
}

/// The answer to the launch's resume or fork: the session is the first
/// thread (its history read, the queued asks sent on it); a refusal fails
/// the start as a fresh thread's would, except another writer, which the chat
/// shows read-only.
fn session_opened(
    app: &mut CodexApp,
    action: crate::screens::startup::Action,
    requested: ThreadId,
    answer: Answer,
    fx: &mut AppFx,
) {
    use crate::screens::startup::Action;
    Rc::make_mut(&mut app.session).launching = None;
    match answer {
        Answer::Thread(ThreadReply::Resumed(opened) | ThreadReply::Forked(opened)) => {
            let opened = opened.thread;
            let id = ThreadId::new(&opened.id);
            let origin = match action {
                Action::Resume => Origin::Resumed,
                Action::Fork => Origin::Started,
            };
            attach_thread_with_metadata(app, &id, BackendId::default(), origin, Some(&opened), fx);
            show_held_thread(app, id.clone(), fx);
            apply_thread(app, &id, threads::history_page_of(&opened), fx);
            fetch_history(app, &id, fx);
            send_queued(app, &id, fx);
            if action == Action::Resume {
                features::resumed(app, &id, fx);
                apply_thread(app, &id, ThreadEv::Command(threads::CommandEv::Restore), fx);
            }
        }
        Answer::Failed(error) if action == Action::Resume && agents::is_active_writer(&error) => {
            app.threads.open(&requested, Origin::ForeignWriter);
            provider_refusal(app, &requested, &error, fx);
            show_held_thread(app, requested.clone(), fx);
            fetch_history(app, &requested, fx);
        }
        Answer::Failed(error) => set_status(app, Session::Failed(Failure::from(error))),
        _ => {}
    }
}

/// An empty conversation moved to `backend` (oracle:
/// `start_new_session_on_other_backend`): the new thread is the chat's, its
/// backend the displayed one; the composer's draft goes with it.
fn switch_to(
    app: &mut CodexApp,
    backend: BackendId,
    thread: ThreadId,
    details: Option<&codex_app_server_protocol::Thread>,
    sources: Vec<status::AgentsSource>,
    fx: &mut AppFx,
) {
    attach_thread_with_metadata(app, &thread, backend, Origin::Started, details, fx);
    note_instruction_sources(app, &thread, sources, fx);
    let session = Rc::make_mut(&mut app.session);
    session.switched_from = session.active.replace(thread.clone());
    session.switches += 1;
    session.status = Session::Ready;
    let agents = &mut Rc::make_mut(&mut app.screens).agents;
    agents.observe(backend, thread.as_str());
    agents.display(backend);
    agents::reset_for_backend_switch(agents);
}

impl SessionSlice {
    /// The backend control is offered while an attached conversation is empty
    /// and another backend is connected (oracle: `visible_new_backend_control`).
    pub fn cycle_label(
        &self,
        thread: &ThreadModel,
        offer: Option<BackendLabel>,
    ) -> Option<BackendLabel> {
        (self.status == Session::Ready
            && self.active.as_ref() == Some(thread.id())
            && thread.transcript().is_empty()
            && !thread.is_running()
            && thread.queue().is_empty()
            && thread.last_submit().is_none())
        .then_some(offer)
        .flatten()
    }
}

/// The stream of `thread` was lost (oracle `ThreadStreamDisconnected`): the displayed thread of
/// the backend that owns it is marked, and when no user turn is pending or running the backend is
/// asked to attach it again, which ends there; every other case shows the error as the line the
/// server's `Error` notification makes.
fn stream_lost(
    app: &mut CodexApp,
    origin: BackendId,
    thread: ThreadId,
    mut error: Box<ErrorNotification>,
    fx: &mut AppFx,
) {
    let owned = app.screens.agents.owner_of(thread.as_str()) == origin;
    if owned
        && app.session.active.as_ref() == Some(&thread)
        && let Some(model) = app.threads.get(&thread)
    {
        let idle = !model.user_turn_in_flight();
        ide_submit::retire_thread(app, &thread, fx);
        apply_thread(app, &thread, ThreadEv::StreamLost, fx);
        if app
            .threads
            .get(&thread)
            .is_some_and(|model| model.reconciling_submissions())
        {
            // A read sent before the gap cannot settle recovery across it.
            app.outgoing.retain(|ctx| {
                !matches!(
                    ctx, AppCtx::Agents(agents::Ctx::History {
                        thread: requested,
                        read: agents::HistoryRead::Latest | agents::HistoryRead::SubmissionRecovery,
                    }) if requested == thread.as_str()
                )
            });
        }
        if idle {
            fx.host(effects::Ask::ReconnectIdleThread {
                address: app.address(origin),
                thread,
                error: effects::IdleStreamError(error),
            });
            return;
        }
    }
    // The server's words name no turn; the error ends the one that runs (oracle `on_error`
    // finalizes the running turn).
    if error.turn_id.is_empty()
        && let Some(running) = app
            .threads
            .get(&thread)
            .and_then(|model| model.running_turn())
    {
        error.turn_id = running.to_string();
    }
    apply_event(
        app,
        origin,
        None,
        AppEv::Backend(BackendEv::Notification(Box::new(
            ServerNotification::Error(*error),
        ))),
        fx,
    );
}

/// Clear the writer and its pending RPCs after the connection is lost.
fn detach_threads(app: &mut CodexApp, backend: BackendId, fx: &mut AppFx) {
    let cloud = app.screens.agents.is_cloud(backend);
    let held: Vec<_> = app
        .threads
        .iter()
        .filter(|(id, model)| {
            model.outgoing().tickets_on(backend).next().is_some()
                || (app.screens.agents.owner_of(id.as_str()) == backend
                    && (model.has_writer() || (cloud && model.stream_lost())))
        })
        .map(|(id, _)| id.clone())
        .collect();
    for id in held {
        apply_thread(app, &id, ThreadEv::ConnectionLost, fx);
    }
}

/// What a request that lost its connection fails with: the client's own words for a closed
/// channel (the ticket's answer is a failure like any the server gives).
pub(crate) const CONNECTION_LOST: &str = "the connection to the app server was closed";

fn is_attach(ctx: &AppCtx) -> bool {
    matches!(
        ctx,
        AppCtx::StartThread { .. }
            | AppCtx::NewConversation {
                spawned: None
                    | Some(
                        lifecycle::Spawned::Plain { .. }
                            | lifecycle::Spawned::PreparedFork(lifecycle::SideFork {
                                phase: lifecycle::SidePhase::Attached
                                    | lifecycle::SidePhase::Temporary,
                                ..
                            })
                    ),
                ..
            }
            | AppCtx::OpenThread { .. }
            | AppCtx::OpenSession { .. }
            | AppCtx::TurnOperation(TurnOperation::SafetyRetry {
                attaching: true,
                ..
            })
            | AppCtx::Agents(agents::Ctx::Start | agents::Ctx::Fork { .. })
    )
}

fn may_attach(ctx: &AppCtx, id: &ThreadId) -> bool {
    match ctx {
        AppCtx::Opening {
            thread,
            attached_to: Some(_),
            ..
        }
        | AppCtx::OpenThread { thread, .. }
        | AppCtx::OpenSession {
            action: screens::startup::Action::Resume,
            thread,
        } => thread == id,
        AppCtx::Worktree { step, .. } => step.attaching(id),
        _ => is_attach(ctx),
    }
}

fn forget_thread(app: &mut CodexApp, thread: &ThreadId, fx: &mut AppFx) {
    app.preparing_forks.remove(thread);
    if app
        .threads
        .get(thread)
        .is_some_and(|model| model.screen_slots().voice.has_call())
    {
        apply_thread(
            app,
            thread,
            ThreadEv::Screen(screens::voice::ThreadEv::Retired.into()),
            fx,
        );
    }
    app.threads.forget(thread);
    app.attaching_events.retain(|entry| entry.thread != *thread);
    select::forget_permission_settings(app, thread, fx);
}

fn attach_thread(
    app: &mut CodexApp,
    thread: &ThreadId,
    backend: BackendId,
    origin: Origin,
    fx: &mut AppFx,
) {
    attach_thread_with_metadata(app, thread, backend, origin, None, fx);
}

fn attach_thread_with_metadata(
    app: &mut CodexApp,
    thread: &ThreadId,
    backend: BackendId,
    origin: Origin,
    details: Option<&codex_app_server_protocol::Thread>,
    fx: &mut AppFx,
) {
    app.threads.open(thread, origin);
    if let Some(details) = details {
        let voice_history = threads::voice_transcript::History::of(details);
        apply_thread(
            app,
            thread,
            threads::voice_transcript::ThreadEvent::Supported(voice_history).into(),
            fx,
        );
        apply_thread(
            app,
            thread,
            ThreadEv::Screen(ScreenThreadEv::Chat(chat::metadata(details))),
            fx,
        );
        notices::forked_line(app, thread, details, fx);
    }
    Rc::make_mut(&mut app.screens)
        .agents
        .observe(backend, thread.as_str());
    let count = app.attaching_events.len();
    for _ in 0..count {
        let Some(entry) = app.attaching_events.pop_front() else {
            break;
        };
        if entry.thread == *thread && entry.backend == backend {
            apply_thread(app, thread, entry.event, fx);
            if let Some(permissions) = entry.permissions {
                select::record_permission_settings(app, thread, permissions, fx);
            }
        } else {
            app.attaching_events.push_back(entry);
        }
    }
    notices::session_card(app, thread, fx);
    notices::flush(app, thread, fx);
}

/// The apps the mentions offer are the installed and enabled ones the directory lists
/// (`connectors_for_mentions`): an app it does not list as accessible and enabled leaves them.
fn drop_uncallable_apps(app: &mut CodexApp, listed: &[screens::select::apps::App]) {
    use crate::data::mention::MentionKind;
    let callable = |path: &Option<String>| {
        path.as_deref()
            .and_then(|path| path.strip_prefix("app://"))
            .is_none_or(|id| {
                listed
                    .iter()
                    .any(|app| app.id == id && app.installed && app.enabled)
            })
    };
    if app
        .screens
        .popovers
        .mentions
        .iter()
        .all(|mention| mention.kind != MentionKind::App || callable(&mention.path))
    {
        return;
    }
    Rc::make_mut(&mut app.screens)
        .popovers
        .mentions
        .retain(|mention| mention.kind != MentionKind::App || callable(&mention.path));
}

fn show_held_thread(app: &mut CodexApp, id: ThreadId, fx: &mut AppFx) {
    // Another thread on display: what the pickers hold for the one before is not trusted
    // (oracle `invalidate_connector_scope` on a displayed-thread change).
    let switched = app
        .session
        .active
        .as_ref()
        .is_some_and(|shown| *shown != id);
    let session = Rc::make_mut(&mut app.session);
    session.active = Some(id.clone());
    session.status = Session::Ready;
    features::on_shown(app, &id, fx);
    if switched {
        select::event(app, screens::select::Ev::ScopeChanged, fx);
    }
}

fn submit(app: &mut CodexApp, thread: &ThreadId, message: UserMessage, fx: &mut AppFx) {
    submit_with_receipt(app, thread, message, fx, SubmitReceipt::Composer);
}

fn submit_with_receipt(
    app: &mut CodexApp,
    thread: &ThreadId,
    message: UserMessage,
    fx: &mut AppFx,
    receipt: SubmitReceipt,
) {
    let message = message.trimmed();
    if message.is_empty()
        || !app.threads.contains(thread)
        || app.preparing_forks.contains_key(thread)
    {
        return;
    }
    dismiss_banner_for_new_turn(app, thread);
    apply_thread_with_receipt(app, thread, ThreadEv::Submit { message }, fx, receipt);
}

/// The model the thread runs on: its own settings, else the config's.
fn model_in_use(app: &CodexApp, thread: &ThreadId) -> Option<String> {
    app.threads
        .get(thread)
        .and_then(|model| model.screen_slots().chat.model.clone())
        .or_else(|| app.screens.chat.status.model.clone())
}

/// Oracle `dismiss_backend_banner_for_new_turn`, at the submit.
fn dismiss_banner_for_new_turn(app: &mut CodexApp, thread: &ThreadId) {
    let backend = app.screens.agents.owner_of(thread.as_str());
    let model = model_in_use(app, thread);
    if app.session.backend_banners.contains_key(&backend)
        && let Some(occurrence) = Rc::make_mut(&mut app.session)
            .backend_banners
            .get_mut(&backend)
    {
        occurrence.dismiss_for_new_turn(model.as_deref());
    }
}

/// A thread event routed to its thread. Used by `apply_app` and by tests.
pub fn apply_thread(app: &mut CodexApp, thread: &ThreadId, ev: ThreadEv, fx: &mut AppFx) {
    apply_thread_with_receipt(app, thread, ev, fx, SubmitReceipt::Composer);
}

fn apply_thread_with_receipt(
    app: &mut CodexApp,
    thread: &ThreadId,
    ev: ThreadEv,
    fx: &mut AppFx,
    receipt: SubmitReceipt,
) {
    let ev = if matches!(
        &ev,
        ThreadEv::ScreenIntent(ScreenThreadIntent::Voice(
            screens::voice::ThreadIntent::Start
        ))
    ) && !app.voice_host_ready()
    {
        ThreadEv::Failure(Failure::VoiceHostLost)
    } else {
        ev
    };
    safety_retry::observe(app, thread, &ev, fx);
    // A cyber refusal is worded by the setting of the moment (a delta carries none).
    let daybreak = if ev.may_fail_a_turn() {
        notices::daybreak_of(app, thread)
    } else {
        crate::screens::cells::Daybreak::Apply
    };
    let agents = &app.screens.agents;
    let backend = agents.owner_of(thread.as_str());
    let cloud = agents.is_cloud(backend);
    let address = app.address(backend);
    let priority = features::completion_priority(app, thread, &ev, address, fx);
    let image_restrictions = app
        .image_restrictions
        .get(&backend)
        .filter(|restricted| restricted.address == address)
        .map(|restricted| &restricted.models);
    if let Some(model) = app.threads.get_mut(thread) {
        let realtime_voice = match &ev {
            ThreadEv::ScreenIntent(ScreenThreadIntent::Voice(
                screens::voice::ThreadIntent::Start,
            )) => app
                .screens
                .voice
                .settings
                .voice(address, model.screen_slots().chat.cwd.as_ref()),
            _ => None,
        };
        threads::apply_with_receipt(
            model,
            ev,
            &mut ThreadFx::new(thread, fx, &mut app.mint)
                .addressed(address)
                .with_legacy_voice(app.screens.voice.legacy_captions_from(address))
                .with_realtime_voice(realtime_voice)
                .with_prompt_priority(priority)
                .with_image_restrictions(image_restrictions)
                .stamped(daybreak)
                .with_modes(&app.modes, app.screens.select.plan_defaults.get(&backend))
                .with_cloud(cloud)
                .with_facts(AppFacts::new(
                    app.now,
                    &app.tui,
                    &app.screens.status,
                    &app.daybreak,
                    &app.keymap,
                )),
            receipt,
        );
        crate::features::revalidate_rate_offer(&mut app.features, model.view());
    }
    reset::settle_hint(app, thread, fx);
}
