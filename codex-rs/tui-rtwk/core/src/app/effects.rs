//! The asks of the app that the host runs outside the app-server's thread RPCs: the clipboard, the
//! git diff, the title generator, the export file and the config file, with the answers the app
//! keeps for whoever asked. (ADR-003: one `CodexApp`, one writer; a screen asks with an
//! `Intent::Effect { request, ask }` and reads `CodexApp::effect(request)`.)
//!
//! The UI chooses the `request` number (as the usage page chooses its stamp and the import flow its
//! request number): the answer comes back under it, a late answer of an earlier request is a
//! different number. The words of a failure are the UI's string tables'; the texts inside are the
//! host's own diagnostics.

use std::collections::VecDeque;
use std::sync::Arc;

use super::Command;
use crate::ids::{RequestKey, ThreadId};
use crate::screens::approvals::verify::{AttemptId, Cause, Verifications};
use crate::screens::transcript::diff;

/// How many answers are kept: a screen reads the answer to its latest request.
const KEPT: usize = 32;

/// Whether copied text also has a rendered HTML form and its Markdown source (oracle
/// `clipboard_copy.rs` `CopyFormat`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CopyFormat {
    PlainText,
    Markdown,
    /// The visible selection is copied literally; only the HTML uses the semantic Markdown.
    MarkdownSelection(Arc<str>),
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
    /// Put `text` on the clipboard (native first, then the terminal's forwarding).
    Copy { text: String, format: CopyFormat },
    /// The diff of the working directory `cwd` (`None`: the process's own): the host runs the
    /// `diff::Diff` machine through its workspace executor.
    GitDiff { cwd: Option<String> },
    /// A title for `thread`: an ephemeral thread with a structured turn, then unsubscribed.
    SuggestTitle { thread: ThreadId },
    /// The transcript of `thread` as a Markdown file under `dest` (`None`: the thread's cwd).
    Export {
        thread: ThreadId,
        dest: Option<String>,
    },
    /// Edits of the config file.
    ConfigWrite { edits: Vec<ConfigEdit> },
}

/// What the host answers, one variant per `Ask`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Done {
    Copy(CopyStatus),
    GitDiff(Result<diff::Report, diff::Failure>),
    /// The suggested title; none when nothing came out.
    Title(Option<String>),
    /// The path written, or the host's failure text.
    Export(Result<String, String>),
    /// Nothing, or the host's failure text.
    Config(Result<(), String>),
}

/// The answers kept, and the user-verification controller (ADR-003: the app owns it).
#[derive(Debug, Default)]
pub struct Effects {
    done: VecDeque<(u64, Done)>,
    pub(super) verifications: Verifications,
    /// Why the last verifications failed (the oracle's warning line of the thread; the words are
    /// the approvals strings' `verification_failure`).
    failures: VecDeque<(RequestKey, Cause)>,
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
