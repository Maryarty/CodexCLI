//! The asks of the app that the host runs outside the app-server's thread RPCs: the clipboard, the
//! git diff, the title generator, the export file and the config file, with the answers the app
//! keeps for whoever asked. (ADR-003: one `CodexApp`, one writer; a screen asks with an
//! `Intent::Effect { request, ask }` and reads `CodexApp::effect(request)`.)
//!
//! The UI chooses the `request` number (as the usage page chooses its stamp and the import flow its
//! request number): the answer comes back under it, a late answer of an earlier request is a
//! different number. The words of a failure are the UI's string tables'; the texts inside are the
//! host's own diagnostics.
//!
//! `Ask` is the list of what the host can do (a capability each: write a file, suggest a title,
//! copy), never a feature's: a feature slice asks with `AppCx::effect` (ADR-012) and the app
//! chooses the request number, `FEATURE_REQUEST_BIT | n`. Its answer goes to the feature's
//! `Feature::effect_done` and is kept for no screen.

use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Weak};

#[macro_use]
mod capability;

use super::Command;
use crate::features::FeatureCtx;
use crate::ids::{RequestKey, ThreadId, Ticket};
use crate::rpc::Address;
use crate::screens::approvals::verify::{AttemptId, Cause, Verifications};
use crate::screens::chat::facts::GitSummary;
use crate::screens::select::review::Commit;
use crate::screens::select::worktree::Entry as WorktreeEntry;
use crate::screens::transcript::diff;
use crate::screens::{import, onboarding, popovers, usage};

/// How many answers are kept: a screen reads the answer to its latest request.
const KEPT: usize = 32;

/// The bit of the request numbers of the asks a feature makes (`AppCx::effect`): the app mints
/// them, so they never meet the stamps a screen chooses (`1 << 59` to `1 << 62` are the
/// screens' own namespaces).
pub const FEATURE_REQUEST_BIT: u64 = 1 << 58;

/// `request` is the number of an ask a feature made.
pub fn is_feature_request(request: u64) -> bool {
    request & FEATURE_REQUEST_BIT != 0
}

/// Whether copied text also has a rendered HTML form and its Markdown source (oracle
/// `clipboard_copy.rs` `CopyFormat`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CopyFormat {
    PlainText,
    Markdown,
    /// The visible selection is copied literally; only the HTML uses the semantic Markdown.
    MarkdownSelection(Arc<str>),
}

/// Where a copy goes (oracle `clipboard_copy::worker::CopyDestination`): a mouse selection can
/// publish X11 PRIMARY as well as, or instead of, the clipboard.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum CopyDestination {
    #[default]
    Clipboard,
    /// The X11 PRIMARY selection only, with the copied text.
    Primary(PrimaryOwner),
    /// The clipboard, and PRIMARY with this plain text.
    ClipboardAndPrimary(Arc<str>, PrimaryOwner),
}

/// The lifetime of the UI selection whose text may still be published to PRIMARY.
#[derive(Clone, Debug)]
pub struct PrimaryOwner(Weak<()>);

impl PrimaryOwner {
    pub fn new(selection: &Arc<()>) -> Self {
        Self(Arc::downgrade(selection))
    }

    pub fn is_current(&self) -> bool {
        self.0.strong_count() != 0
    }
}

impl PartialEq for PrimaryOwner {
    fn eq(&self, other: &Self) -> bool {
        self.0.ptr_eq(&other.0)
    }
}

impl Eq for PrimaryOwner {}

/// The delivery status a clipboard backend reports (oracle `clipboard_copy.rs` `CopyStatus`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CopyStatus {
    /// A native or WSL write succeeded.
    Confirmed,
    /// Only a request to the user's terminal (OSC 52, tmux, SSH) went out.
    Unconfirmed,
    /// The submission is under way (a number of the backend's).
    Pending(u64),
    /// A copy is already in progress.
    Busy,
}

/// The raster's dimensions or the caller's preview bounds, in pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ImageSize {
    pub width: u16,
    pub height: u16,
}

/// A small validated PNG. An Arc is the revision of this snapshot: keep it while displaying
/// the same read, and use a new one when the file is read again, even at the same path.
#[derive(Clone)]
pub struct ImagePreview {
    pub png: Arc<[u8]>,
    pub size: ImageSize,
}

impl std::fmt::Debug for ImagePreview {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.debug_struct("ImagePreview")
            .field("size", &self.size)
            .field("encoded_bytes", &self.png.len())
            .finish()
    }
}

impl PartialEq for ImagePreview {
    fn eq(&self, other: &Self) -> bool {
        self.size == other.size && Arc::ptr_eq(&self.png, &other.png)
    }
}

impl Eq for ImagePreview {}

/// A host-authorized hyperlink and, when requested and decodable, the preview of a local file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ImageSnapshot {
    pub link: Option<String>,
    pub preview: Option<ImagePreview>,
    pub revision: Option<ImageRevision>,
}

/// A file identity and modification stamp produced by the local image reader.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImageRevision(pub u64);

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ImageRead {
    /// This workspace or host does not support the read.
    #[default]
    Unavailable,
    Snapshot(ImageSnapshot),
    /// The previous revision still identifies the file in this read.
    Unchanged,
}

/// Whether the local IDE's successful read included a selection or tabs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdeReading {
    pub has_context: bool,
}

/// The extension's failure hints for enabling IDE context or sending a message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdeFailure {
    pub hint: String,
    pub skip_hint: String,
}

/// The worktree and opt-in for the current visit to a conversation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct IdePermit {
    pub thread: ThreadId,
    pub cwd: String,
    pub generation: u64,
}

/// An IDE selection going to the model as part of a single turn.
#[derive(Clone, PartialEq, Eq)]
pub struct IdePrompt(Arc<str>);

impl IdePrompt {
    pub fn new(text: String) -> Self {
        Self(Arc::from(text))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for IdePrompt {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.debug_struct("IdePrompt")
            .field("bytes", &self.0.len())
            .finish()
    }
}

/// One edit of the user's config file (`config/batchWrite`: a dotted key path and its new value,
/// the value replaces).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigEdit {
    pub key_path: String,
    pub value: serde_json::Value,
}

/// Where a daemon update comes from (`DaemonUpdateSource`): the choice of the `/daemon` menu, and
/// what the update action runs after the exit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DaemonSource {
    /// `Install latest public stable`.
    PublicStable,
    /// `Use this CLI build`.
    ThisCli,
}

/// What `Ask::Daemon` asks of the local background server.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DaemonAsk {
    /// Whether the connected server is a daemon this CLI manages (`LifecycleCommand::Version`
    /// answers with a backend): `/daemon` asks before it offers a restart.
    Verify,
    /// Restarts the server (`LifecycleCommand::Restart`); the connection reconnects as it does
    /// after any loss.
    Restart,
    /// `Update and exit`: the host keeps the update to run once Codex has exited
    /// (`RunDaemonUpdate`: `pending_update_action = Daemon(source)`).
    Update(DaemonSource),
}

/// The answer of a `DaemonAsk`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DaemonDone {
    /// The server is a managed daemon, or is not (or could not be asked).
    Managed(bool),
    /// The restart is over: nothing, or the failure text.
    Restarted(Result<(), String>),
    /// The update is kept for the host to run after the exit: the app may end.
    UpdateQueued,
}

host_capability_list! {
    /// Put `text` on the clipboard (native first, then the terminal's forwarding), or on
    /// PRIMARY: `destination` says where. Answered by the delivery status, or the host's failure
    /// text (oracle: `Failed to copy ...: {error}`); a copy reports `Pending` first and how it
    /// ended after.
    copy => Copy {
        text: String,
        format: CopyFormat,
        destination: CopyDestination,
    } -> new Copy(Result<CopyStatus, String>);
    /// The diff of the working directory `cwd` (`None`: the process's own): the host runs the
    /// `diff::Diff` machine through its workspace executor.
    git_diff => GitDiff { cwd: Option<String> }
        -> new GitDiff(Result<diff::Report, diff::Failure>);
    /// The open pull request and committed branch changes of the given workspace.
    git_summary => GitSummary { thread: ThreadId, cwd: String, address: Address } by thread
        -> new GitSummary(GitSummary);
    /// The transcript of `thread`, as the Markdown the asker rendered from the cells' text,
    /// written to `path` (`~/` is the home directory, a relative path is under `cwd`: the
    /// launch directory for a remote workspace, else the thread's; an existing file is never
    /// overwritten). Oracle `transcript_export.rs`. Answered by the path written, or the host's
    /// failure text.
    export => Export {
        thread: ThreadId,
        path: String,
        cwd: String,
        markdown: String,
    } by thread -> new Export(Result<String, String>);
    /// Edits of the config file. Answered by nothing, or the host's failure text.
    config_write => ConfigWrite { edits: Vec<ConfigEdit> } -> new Config(Result<(), String>);
    /// The text the system clipboard holds (Ctrl+Shift+V, the right-click paste), or why it could
    /// not be read.
    clipboard_text => ClipboardText -> new ClipboardText(Result<String, String>);
    /// Check whether a workspace image is a trustworthy local file, optionally reading a
    /// bounded preview. Paths from remote workspaces cannot be resolved on the client.
    image_snapshot => ImageSnapshot {
        thread: ThreadId,
        path: String,
        preview: Option<ImageSize>,
        previous: Option<ImageRevision>,
    } by thread -> new ImageSnapshot(ImageRead);
    /// Probe the extension for a local workspace. Only its availability and whether it has
    /// context cross this reply; the extension's selection stays in the host.
    ide_context => IdeContext { thread: ThreadId, cwd: String }
        by thread -> new IdeContext(Result<IdeReading, IdeFailure>);
    /// Read the selection and tabs for one accepted user submission.
    ide_prompt => IdePrompt { thread: ThreadId, cwd: String }
        by thread -> new IdePrompt(Result<Option<IdePrompt>, IdeFailure>);
    /// Opens `thread` in the Desktop app (`codex://threads/{id}`, `/app`). Answered by nothing,
    /// or the launch's own failure text.
    open_desktop => OpenDesktop { thread: ThreadId } -> new Opened(Result<(), String>);
    /// Opens a page in the user's browser (oracle `AppEvent::OpenUrlInBrowser`): a feature's
    /// link (the sign-in page of a plugin's app). Answered by `Done::Opened`.
    open_url => OpenUrl { url: String, report: BrowserReport } -> shared Opened;
    /// The local branches of the repository in `cwd` (the default branch first) and the one
    /// checked out: the `/review` base-branch picker (oracle `local_git_branches`,
    /// `current_branch_name`). The answer has the current branch (none: detached HEAD, or no
    /// repository) and the branches; an empty list when it is no repository.
    git_branches => GitBranches { cwd: Option<String> } -> new GitBranches {
        current: Option<String>,
        branches: Vec<String>,
    };
    /// The last `limit` commits of `cwd`, newest first: the `/review` commit picker (oracle
    /// `recent_commits`).
    git_log => GitLog { cwd: Option<String>, limit: usize } -> new GitLog(Vec<Commit>);
    /// The managed worktrees of the repository `cwd` is in, each with its owner thread (oracle
    /// `worktree_browser::fetch`), or why they could not be listed.
    worktrees => Worktrees { cwd: String }
        -> new Worktrees(Result<Vec<WorktreeEntry>, String>);
    /// The local background server's lifecycle (`codex_app_server_daemon::run`): the probe of
    /// whether the connected server is a daemon this CLI manages, and the restart.
    background_server => BackgroundServer(ask: DaemonAsk) -> new BackgroundServer(DaemonDone);
    /// Removes the worktree at `root` (oracle `worktree_browser::remove`, `cwd` is the
    /// session's). Answered by nothing, or why the worktree could not be removed.
    worktree_remove => WorktreeRemove { cwd: String, root: String }
        -> new WorktreeRemoved(Result<(), String>);
    /// The external editor (oracle `external_editor::run_editor`): `draft` is written to a
    /// temporary file, `$VISUAL` / `$EDITOR` runs on it with the terminal handed over, and the
    /// file's text comes back. `cwd` is the thread's directory (the sandbox policy is read
    /// against it). Answered by what the editor left in the file, or why it could not run.
    editor => Editor { draft: String, cwd: String }
        -> new Editor(Result<String, EditorFailure>);
    /// Microphone capture and speaker playback of voice (oracle `voice.rs`, `audio_device.rs`).
    /// Listed for the voice package; the host answers it with its audio executor.
    audio => Audio(command: crate::screens::voice::AudioCommand) -> new Audio(crate::screens::voice::AudioReport);
    /// Machine-local audio settings and the originating backend's voice preference.
    voice_settings => VoiceSettings(request: crate::screens::voice::settings::Request) -> new VoiceSettings(Box<crate::screens::voice::settings::Report>);
    /// Read a terminal pet catalog or static frame, or persist the local selection.
    pet => Pet(ask: crate::screens::pets::AssetAsk) -> new Pet(crate::screens::pets::AssetDone);
    /// Suspends the process to the shell (Ctrl+Z, oracle `tui/job_control.rs`); the answer comes
    /// when the shell resumes it.
    suspend => Suspend -> new Resumed(Result<(), String>);
    /// A desktop notification of the terminal (OSC 9 or BEL, oracle `tui/notifications.rs`).
    notify => Notify { message: String } -> new Notified(Result<(), String>);
    /// The terminal's screen: the alternate screen on or off (the inline mode leaves it for an
    /// overlay and comes back, oracle `tui.rs` `enter_alt_screen`/`leave_alt_screen`).
    screen_mode => ScreenMode { alternate: bool } -> new ScreenModeSet(Result<(), String>);
    /// The terminal's default colors (oracle `terminal_palette.rs`), `None` when it did not say.
    probe => Probe -> new Probed(Option<TerminalColors>);
    /// Run a startup step. Its screen owns the continuation through a possible restart.
    startup => Startup { ask: onboarding::Ask };
    /// Detect or import settings under the import screen's ticket.
    import => Import { ask: import::Request };
    /// The popup's local history, mentions, clipboard image, or search service.
    popovers => Popovers { ask: popovers::Ask, thread: Option<ThreadId> };
    /// An analytics read under the usage screen's session and stamp.
    usage => Usage { ask: usage::Ask };
    /// Read the canonical directory and repository keys for the command center.
    directories => Directories { cwds: Vec<String> };
    /// Save the command center's grouping preference.
    save_grouping => SaveGrouping { grouping: String };
    /// Create a checkout under the app's existing ticket.
    create_worktree => CreateWorktree { ticket: Ticket, cwd: Option<String> };
    /// Bind the started thread to that checkout.
    bind_worktree => BindWorktree {
        ticket: Ticket, checkout: super::worktree::Checkout, thread: ThreadId, cwd: String,
    };
    /// Ask the local verification device to sign the owning request's challenge.
    verify_user => VerifyUser {
        key: RequestKey, attempt: Attempt, params: codex_app_server_protocol::UserVerificationVerifyParams,
    };
    /// Rejoin an idle thread on the same physical connection after its stream ends.
    reconnect_idle_thread => ReconnectIdleThread {
        address: Address, thread: ThreadId, error: IdleStreamError,
    };
}

/// Whether the browser result goes back to the caller or only a failure is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrowserReport {
    Effect,
    ErrorLine,
}

/// The error to show if the thread's idle stream cannot be restored.
#[derive(Clone, Debug, PartialEq)]
pub struct IdleStreamError(pub Box<codex_app_server_protocol::ErrorNotification>);

// The pinned error and its payload are equatable; the wire type only derives PartialEq.
impl Eq for IdleStreamError {}

/// The terminal's default colors, as `[r, g, b]`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TerminalColors {
    pub foreground: Option<[u8; 3]>,
    pub background: Option<[u8; 3]>,
}

impl Ask {
    pub(super) fn workspace_receipt(&self) -> Option<WorkspaceReceipt> {
        match self {
            Self::ImageSnapshot { .. } => Some(WorkspaceReceipt::ImageSnapshot),
            Self::IdeContext { .. } => Some(WorkspaceReceipt::IdeContext),
            Self::IdePrompt { .. } => Some(WorkspaceReceipt::IdePrompt),
            _ => None,
        }
    }

    pub fn open_url(url: String) -> Self {
        Self::OpenUrl {
            url,
            report: BrowserReport::Effect,
        }
    }

    pub(crate) fn open_url_or_report(url: String) -> Self {
        Self::OpenUrl {
            url,
            report: BrowserReport::ErrorLine,
        }
    }

    /// How many characters a copy puts on the clipboard (the oracle's feedback counts what was
    /// copied, not what is selected, and in characters, not bytes or cells); none for another ask.
    pub fn characters(&self) -> usize {
        match self {
            Ask::Copy { text, .. } => text.chars().count(),
            _ => 0,
        }
    }
}

/// Why the external editor did not give a text back (oracle `EditorError` and the failures of
/// `run_editor`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditorFailure {
    /// Neither `VISUAL` nor `EDITOR` is set.
    Missing,
    /// The host's reason: a command that does not parse, a file that could not be written or
    /// read, an editor that exited with a failure.
    Failed(String),
}

impl Done {
    fn workspace_receipt(&self) -> Option<WorkspaceReceipt> {
        match self {
            Self::ImageSnapshot(_) => Some(WorkspaceReceipt::ImageSnapshot),
            Self::IdeContext(_) => Some(WorkspaceReceipt::IdeContext),
            Self::IdePrompt(_) => Some(WorkspaceReceipt::IdePrompt),
            _ => None,
        }
    }

    /// No answer follows this one: a copy reports `Pending` first and how it ended after, every
    /// other ask answers once.
    pub fn is_final(&self) -> bool {
        !matches!(self, Done::Copy(Ok(CopyStatus::Pending(_))))
    }
}

/// The answers kept, and the user-verification controller (ADR-003: the app owns it).
#[derive(Debug, Default)]
pub struct Effects {
    done: VecDeque<(u64, Done)>,
    pub(super) verifications: Verifications,
    /// Why the last verifications failed (the oracle's warning line of the thread; the words are
    /// the approvals strings' `verification_failure`).
    failures: VecDeque<(RequestKey, Cause)>,
    /// What each ask a feature made is for, until its last answer comes (`AppCx::effect`).
    features: BTreeMap<u64, FeatureRequest>,
    /// The count of the asks features made: the low bits of their request numbers.
    feature_asks: u64,
}

#[derive(Debug)]
struct FeatureRequest {
    continuation: EffectContinuation,
    /// Some host capabilities depend on the physical connection's locality.
    workspace: Option<WorkspaceEffect>,
}

#[derive(Debug)]
enum EffectContinuation {
    Feature(Box<FeatureCtx>),
    IdeSubmit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum WorkspaceReceipt {
    ImageSnapshot,
    IdeContext,
    IdePrompt,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct WorkspaceEffect {
    pub thread: ThreadId,
    pub address: Address,
    pub receipt: WorkspaceReceipt,
}

impl Effects {
    /// The answer to `request`, if it came and is not yet pushed out by newer ones.
    pub fn result(&self, request: u64) -> Option<&Done> {
        self.done
            .iter()
            .rev()
            .find(|(number, _)| *number == request)
            .map(|(_, done)| done)
    }

    /// The newest answer of any request, with its number.
    pub fn latest(&self) -> Option<(u64, &Done)> {
        self.done.back().map(|(number, done)| (*number, done))
    }

    /// The cause the verification of `key` failed with, if it did.
    pub fn verification_failure(&self, key: &RequestKey) -> Option<Cause> {
        self.failures
            .iter()
            .rev()
            .find(|(failed, _)| failed == key)
            .map(|(_, cause)| *cause)
    }

    pub(super) fn record_failure(&mut self, key: RequestKey, cause: Cause) {
        self.failures.push_back((key, cause));
        while self.failures.len() > KEPT {
            self.failures.pop_front();
        }
    }

    /// A feature's ask: its request number, with `ctx` kept until the last answer.
    pub(super) fn ask_for_feature(
        &mut self,
        ctx: FeatureCtx,
        workspace: Option<WorkspaceEffect>,
    ) -> u64 {
        self.register(EffectContinuation::Feature(Box::new(ctx)), workspace)
    }

    pub(super) fn ask_for_ide_submit(&mut self, thread: ThreadId, address: Address) -> u64 {
        self.register(
            EffectContinuation::IdeSubmit,
            Some(WorkspaceEffect {
                thread,
                address,
                receipt: WorkspaceReceipt::IdePrompt,
            }),
        )
    }

    fn register(
        &mut self,
        continuation: EffectContinuation,
        workspace: Option<WorkspaceEffect>,
    ) -> u64 {
        self.feature_asks += 1;
        let request = FEATURE_REQUEST_BIT | self.feature_asks;
        self.features.insert(
            request,
            FeatureRequest {
                continuation,
                workspace,
            },
        );
        request
    }

    pub(super) fn take_ide_submit(&mut self, request: u64) -> Option<Address> {
        let pending = self.features.get(&request)?;
        if !matches!(pending.continuation, EffectContinuation::IdeSubmit) {
            return None;
        }
        self.features
            .remove(&request)?
            .workspace
            .map(|effect| effect.address)
    }

    pub(super) fn workspace_address(&self, request: u64) -> Option<Address> {
        self.workspace_effect(request).map(|effect| effect.address)
    }

    pub(super) fn workspace_dispatch(&self, request: u64, ask: &Ask) -> Option<Address> {
        let effect = self.workspace_effect(request)?;
        (ask.thread() == Some(&effect.thread) && ask.workspace_receipt() == Some(effect.receipt))
            .then_some(effect.address)
    }

    pub(super) fn accepts_workspace_receipt(
        &self,
        request: u64,
        origin: Address,
        done: &Done,
    ) -> bool {
        match self.workspace_effect(request) {
            Some(effect) => {
                origin == effect.address && done.workspace_receipt() == Some(effect.receipt)
            }
            None => done.workspace_receipt().is_none(),
        }
    }

    fn workspace_effect(&self, request: u64) -> Option<&WorkspaceEffect> {
        self.features.get(&request)?.workspace.as_ref()
    }

    pub(super) fn retire_workspace_effects(&mut self, backend: crate::ids::BackendId) {
        self.features.retain(|_, ask| {
            ask.workspace
                .as_ref()
                .is_none_or(|effect| effect.address.backend != backend)
        });
    }

    /// What the answer `done` to the feature ask `request` is for: none for a number no feature
    /// waits for (a late answer of an ask that has ended). The last answer ends the ask.
    pub(super) fn feature_answered(&mut self, request: u64, done: &Done) -> Option<FeatureCtx> {
        let pending = self.features.get(&request)?;
        let EffectContinuation::Feature(ctx) = &pending.continuation else {
            return None;
        };
        if done.is_final() {
            let EffectContinuation::Feature(ctx) = self.features.remove(&request)?.continuation
            else {
                return None;
            };
            Some(*ctx)
        } else {
            Some(ctx.as_ref().clone())
        }
    }

    /// How many asks of features wait for their last answer.
    #[cfg(test)]
    pub(super) fn feature_asks_waiting(&self) -> usize {
        self.features.len()
    }

    /// An answer replaces an earlier one of the same request (a copy reports `Pending` first).
    pub(super) fn settle(&mut self, request: u64, done: Done) {
        self.done.retain(|(number, _)| *number != request);
        self.done.push_back((request, done));
        while self.done.len() > KEPT {
            self.done.pop_front();
        }
    }
}

/// The command for an ask.
pub(super) fn command(request: u64, ask: Ask) -> Command {
    Command::Effect { request, ask }
}

/// A user verification's device result (`userVerification/verify`): the proof, or the server's
/// error `data` (only the typed data counts, never its message).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verified {
    Proof(codex_app_server_protocol::UserVerificationProof),
    Failed(Option<serde_json::Value>),
    /// The call itself failed (a transport error): nothing to read.
    Unreachable,
}

/// The attempt a result answers.
pub type Attempt = AttemptId;
