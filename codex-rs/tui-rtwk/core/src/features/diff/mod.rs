//! `/diff` as a feature slice (ADR-012; oracle `slash_dispatch.rs` `SlashCommand::Diff`,
//! `event_dispatch.rs` `DiffResult`, `get_git_diff.rs`): the git diff of the directory the session
//! is in, shown in the pager. The host computes it (`Ask::GitDiff`, which drives the
//! `screens::transcript::diff::Diff` machine through its workspace executor); the answer is
//! the pager's document, asked of the surface as `Wish::Pager` (the pager and the words of a note
//! are the UI half's, `src/features/diff/`).
//!
//! A diff asked for again before the first came back replaces it (the request carries its stamp);
//! an answer for a directory the session has left is dropped (oracle `AppEvent::DiffResult(cwd,
//! text)`: a reply for another directory is dropped).

use codex_app_server_protocol::{ClientRequest, RequestId};

use super::{AppCx, Feature, Never, Scope};
use crate::app::effects::{Ask, Done};
use crate::commands::SlashCommand;
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::screens::transcript::diff::{Failure, Report};
use crate::threads::Tone;

/// The marker type of the feature.
pub struct Diff;

/// What `/diff` names (`FeatureTarget`): the UI half answers it with `Intent::Run`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sheet;

/// What the user asks of `/diff`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// The diff of the session's directory is computed and shown.
    Run,
}

/// The diffs asked for: the stamp of the newest (a record by its domain key).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    stamp: u64,
}

/// What an ask was for: the directory the diff is of, and which ask of this feature it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ctx {
    cwd: String,
    stamp: u64,
}

/// What a pager document says instead of text: typed here, worded by the view (`No changes
/// detected.` in italics, `` `/diff` — _not inside a git repository_ ``, `Failed to compute diff:
/// {e}`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PagerNote {
    /// The diff is empty (`text.trim().is_empty()`).
    NoChanges,
    /// The directory is not inside a git work tree.
    NotRepo,
    /// The diff could not be computed.
    Failed(Failure),
}

/// What the surface is asked to show once (`Scope::Thread` of the thread in use): the pager on
/// the diff (its title, `D I F F`, is the view's). `note` is what the document says in place of
/// an empty `text`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Wish {
    Pager {
        text: String,
        note: Option<PagerNote>,
    },
}

/// The pager's document of a diff's answer: the text as it came (ANSI kept), or an empty text
/// and the note that says why.
pub fn document(result: Result<Report, Failure>) -> (String, Option<PagerNote>) {
    match result {
        Err(failure) => (String::new(), Some(PagerNote::Failed(failure))),
        Ok(report) if !report.in_repo => (String::new(), Some(PagerNote::NotRepo)),
        Ok(report) if report.text.trim().is_empty() => (String::new(), Some(PagerNote::NoChanges)),
        Ok(report) => (report.text, None),
    }
}

impl Feature for Diff {
    const NAME: &'static str = "diff";
    const OPENS: Option<(SlashCommand, Sheet)> = Some((SlashCommand::Diff, Sheet));

    type State = State;
    type ThreadPart = ();
    type Target = Sheet;
    type Intent = Intent;
    type ThreadIntent = Never<Self>;
    type Command = Never<Self>;
    type Reply = Never<Self>;
    type Ctx = Ctx;
    type Note = Never<Self>;
    type Fact = Never<Self>;
    type Wish = Wish;
    type Interest = Never<Self>;

    fn intent(state: &mut State, intent: Intent, cx: &mut AppCx<'_>) {
        let Intent::Run = intent;
        state.stamp += 1;
        let view = cx.view();
        let cwd = view.working_directory(view.active());
        let ctx = Ctx {
            cwd: cwd.clone(),
            stamp: state.stamp,
        };
        cx.effect(Ask::GitDiff { cwd: Some(cwd) }, ctx);
    }

    fn effect_done(state: &mut State, ctx: Ctx, done: Done, cx: &mut AppCx<'_>) {
        let Done::GitDiff(result) = done else {
            return;
        };
        let view = cx.view();
        if ctx.stamp != state.stamp || ctx.cwd != view.working_directory(view.active()) {
            return;
        }
        let (text, note) = document(result);
        let thread = view.active().cloned().unwrap_or_default();
        cx.wish(Scope::Thread(thread), Wish::Pager { text, note });
    }

    fn note_tone(note: &Never<Self>) -> Tone {
        note.absurd()
    }

    fn wire(command: &Never<Self>, _thread: &ThreadId, _id: RequestId) -> ClientRequest {
        command.absurd()
    }

    fn decode(command: &Never<Self>, _result: serde_json::Value) -> Result<Never<Self>, RpcError> {
        command.absurd()
    }
}
