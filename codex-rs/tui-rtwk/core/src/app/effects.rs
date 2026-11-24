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
use std::sync::Arc;

use super::Command;
use crate::features::FeatureCtx;
use crate::ids::{RequestKey, ThreadId};
use crate::screens::approvals::verify::{AttemptId, Cause, Verifications};
use crate::screens::select::review::Commit;
use crate::screens::select::worktree::Entry as WorktreeEntry;
use crate::screens::transcript::diff;

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
    Primary,
    /// The clipboard, and PRIMARY with this plain text.
    ClipboardAndPrimary(Arc<str>),
}

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

/// One edit of the user's config file (`config/batchWrite`: a dotted key path and its new value,
/// the value replaces).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigEdit {
    pub key_path: String,
    pub value: serde_json::Value,
}

/// What the host is asked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ask {
    /// Put `text` on the clipboard (native first, then the terminal's forwarding), or on
    /// PRIMARY: `destination` says where.
    Copy {
        text: String,
        format: CopyFormat,
        destination: CopyDestination,
    },
    /// The diff of the working directory `cwd` (`None`: the process's own): the host runs the
    /// `diff::Diff` machine through its workspace executor.
    GitDiff { cwd: Option<String> },
    /// A title for `thread`: an ephemeral thread with a structured turn, then unsubscribed.
    SuggestTitle { thread: ThreadId },
    /// The transcript of `thread`, as the Markdown the asker rendered from the cells' text,
    /// written to `path` (`~/` is the home directory, a relative path is under `cwd`: the
    /// launch directory for a remote workspace, else the thread's; an existing file is never
    /// overwritten). Oracle `transcript_export.rs`.
    Export {
        thread: ThreadId,
        path: String,
        cwd: String,
        markdown: String,
    },
    /// A feedback report: staged with the client logs, uploaded or saved as a ZIP through the
    /// server (oracle `feedback_upload.rs`). Answered once by `Done::Feedback`.
    FeedbackUpload(crate::features::feedback::Upload),
    /// Edits of the config file.
    ConfigWrite { edits: Vec<ConfigEdit> },
    /// The text the system clipboard holds (Ctrl+Shift+V, the right-click paste).
    ClipboardText,
    /// Opens `thread` in the Desktop app (`codex://threads/{id}`, `/app`).
    OpenDesktop { thread: ThreadId },
    /// Opens a page in the user's browser (oracle `AppEvent::OpenUrlInBrowser`): a feature's
    /// link (the sign-in page of a plugin's app). Answered by `Done::Opened`.
    OpenUrl { url: String },
    /// The local branches of the repository in `cwd` (the default branch first) and the one
    /// checked out: the `/review` base-branch picker (oracle `local_git_branches`,
    /// `current_branch_name`). Answered by `Done::GitBranches`; an empty list when it is no
    /// repository.
    GitBranches { cwd: Option<String> },
    /// The last `limit` commits of `cwd`: the `/review` commit picker (oracle `recent_commits`).
    GitLog { cwd: Option<String>, limit: usize },
    /// The managed worktrees of the repository `cwd` is in, each with its owner thread
    /// (oracle `worktree_browser::fetch`). Answered by `Done::Worktrees`.
    Worktrees { cwd: String },
    /// Removes the worktree at `root` (oracle `worktree_browser::remove`, `cwd` is the session's).
    WorktreeRemove { cwd: String, root: String },
}

impl Ask {
    /// How many characters a copy puts on the clipboard (the oracle's feedback counts what was
    /// copied, not what is selected, and in characters, not bytes or cells); none for another ask.
    pub fn characters(&self) -> usize {
        match self {
            Ask::Copy { text, .. } => text.chars().count(),
            _ => 0,
        }
    }
}

/// What the host answers, one variant per `Ask`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Done {
    /// The delivery status, or the host's failure text (oracle: `Failed to copy ...: {error}`).
    Copy(Result<CopyStatus, String>),
    GitDiff(Result<diff::Report, diff::Failure>),
    /// The suggested title; none when nothing came out.
    Title(Option<String>),
    /// The path written, or the host's failure text.
    Export(Result<String, String>),
    /// What the delivery came to, or the failure text (`Failed to upload feedback: ...`).
    Feedback(Result<crate::features::feedback::Receipt, String>),
    /// Nothing, or the host's failure text.
    Config(Result<(), String>),
    /// The clipboard's text, or why it could not be read.
    ClipboardText(Result<String, String>),
    /// The Desktop app or the browser was asked to open something: nothing, or the launch's own
    /// failure text.
    Opened(Result<(), String>),
    /// The branches, and the current one (none: detached HEAD, or no repository).
    GitBranches {
        current: Option<String>,
        branches: Vec<String>,
    },
    /// The commits, newest first.
    GitLog(Vec<Commit>),
    /// The worktrees, or why they could not be listed.
    Worktrees(Result<Vec<WorktreeEntry>, String>),
    /// Nothing, or why the worktree could not be removed.
    WorktreeRemoved(Result<(), String>),
}

impl Done {
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
    features: BTreeMap<u64, FeatureCtx>,
    /// The count of the asks features made: the low bits of their request numbers.
    feature_asks: u64,
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
    pub(super) fn ask_for_feature(&mut self, ctx: FeatureCtx) -> u64 {
        self.feature_asks += 1;
        let request = FEATURE_REQUEST_BIT | self.feature_asks;
        self.features.insert(request, ctx);
        request
    }

    /// What the answer `done` to the feature ask `request` is for: none for a number no feature
    /// waits for (a late answer of an ask that has ended). The last answer ends the ask.
    pub(super) fn feature_answered(&mut self, request: u64, done: &Done) -> Option<FeatureCtx> {
        if done.is_final() {
            self.features.remove(&request)
        } else {
            self.features.get(&request).cloned()
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
