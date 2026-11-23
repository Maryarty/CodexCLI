//! `/rename` as a feature slice (ADR-012; oracle `chatwidget/interaction.rs`
//! `show_rename_prompt`, `app/thread_title.rs` `suggest_thread_name`): the name prompt of the
//! thread in use, the title the host's generator suggests for it, and the rename a submit
//! makes. One flow: a bare `/rename` opens the prompt (`Prompt`) prefilled with the thread's
//! name and asks the host for a suggestion (`AppCx::effect`); the suggestion fills the field only
//! while the user has not typed (`AppCx::suggest`: the dialog knows what was typed); a submit,
//! or `/rename <name>`, or `/new <name>`, renames the thread through the feature's own RPC
//! (`thread/name/set`, `Command::SetName`). Words: `src/features/rename/strings.rs`.
//!
//! `/new <name>` reaches the rename through the thread intent the router built
//! (`ThreadIntent::Set`), applied to the new thread once it is started: no hook, the app's
//! generic route for a feature's thread intent.
//!
//! A failed rename is the thread's last failure (the framework sets it for any feature RPC that
//! failed on a thread, `threads::feature_reply`); `ThreadAsk::Rename` stays in `threads` for
//! the agents' rename of another thread.

mod wire;

use codex_app_server_protocol::{ClientRequest, RequestId};

use super::{AppCx, Feature, Never, ThreadCx};
use crate::app::effects::{Ask, Done};
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::threads::Tone;

pub use wire::{Command, Reply};

/// The marker type of the feature.
pub struct Rename;

/// What the user asks of `/rename`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// `/rename` and `/rename <name>`, typed in the thread `source` (none before the session has
    /// started).
    Command {
        source: Option<ThreadId>,
        name: Option<String>,
    },
}

/// What is asked of one thread.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThreadIntent {
    /// Rename the thread to this name (an empty one renames nothing).
    Set(String),
}

/// What an ask was for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ctx {
    /// `thread/name/set`.
    SetName,
    /// The title the host was asked to suggest for this prompt.
    Title(Prompt),
}

/// The name prompt (`Rename thread` over a thread that has a name, `Name thread` over one that
/// has none).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Prompt {
    Name { thread: ThreadId, renaming: bool },
}

/// What `/rename` says in the transcript; all of them are errors in the oracle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Note {
    /// `Session is still starting; try /rename again in a moment.`
    SessionStarting,
    /// `Side conversations cannot be renamed.` (`ensure_thread_rename_allowed`)
    SideRename,
    /// `Thread name cannot be empty.` (a submitted prompt)
    NameEmpty,
}

/// `normalize_thread_name`: the trimmed name, none when nothing is left.
pub fn normalize_thread_name(name: &str) -> Option<String> {
    let trimmed = name.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

/// `ensure_thread_rename_allowed`: a side conversation cannot be renamed.
fn blocked(cx: &AppCx<'_>) -> bool {
    cx.view().screens().chat.flags.side_conversation_active
}

/// A bare `/rename`: the prompt over the thread in use, prefilled with its name, and the ask of
/// a suggestion from its conversation.
fn open(cx: &mut AppCx<'_>, source: Option<ThreadId>) {
    let Some(thread) = source.filter(|id| cx.view().thread(id).is_some()) else {
        return cx.note(None, Note::SessionStarting);
    };
    if blocked(cx) {
        return cx.note(Some(thread), Note::SideRename);
    }
    let existing = cx
        .view()
        .thread(&thread)
        .and_then(|view| view.name())
        .filter(|name| !name.is_empty())
        .map(str::to_owned);
    let prompt = Prompt::Name {
        thread: thread.clone(),
        renaming: existing.is_some(),
    };
    cx.prompt_suggesting(prompt.clone(), existing.unwrap_or_default());
    cx.effect(Ask::SuggestTitle { thread }, Ctx::Title(prompt));
}

impl Feature for Rename {
    const NAME: &'static str = "rename";

    type State = ();
    type ThreadPart = ();
    type Intent = Intent;
    type ThreadIntent = ThreadIntent;
    type Command = Command;
    type Reply = Reply;
    type Ctx = Ctx;
    type Menu = Never<Self>;
    type Pick = Never<Self>;
    type Prompt = Prompt;
    type Note = Note;
    type Fact = Never<Self>;
    type Wish = Never<Self>;
    type Interest = Never<Self>;

    fn intent(_state: &mut (), intent: Intent, cx: &mut AppCx<'_>) {
        let Intent::Command { source, name } = intent;
        match name {
            None => open(cx, source),
            Some(_) if blocked(cx) => cx.note(source, Note::SideRename),
            Some(name) => {
                if let Some(thread) = source {
                    cx.thread(&thread, ThreadIntent::Set(name));
                }
            }
        }
    }

    /// The generator's answer to the prompt's ask: it fills the field if the prompt is still
    /// open and untouched.
    fn effect_done(_state: &mut (), ctx: Ctx, done: Done, cx: &mut AppCx<'_>) {
        if let (Ctx::Title(prompt), Done::Title(title)) = (ctx, done) {
            cx.suggest(prompt, title);
        }
    }

    /// The name the user submitted: the thread is renamed, an empty name is the error line.
    fn prompt_submitted(_state: &mut (), prompt: Prompt, text: String, cx: &mut AppCx<'_>) {
        let Prompt::Name { thread, .. } = prompt;
        match normalize_thread_name(&text) {
            None => cx.note(Some(thread), Note::NameEmpty),
            Some(_) if blocked(cx) => cx.note(Some(thread), Note::SideRename),
            Some(name) => cx.thread(&thread, ThreadIntent::Set(name)),
        }
    }

    fn thread_intent(
        _part: &mut (),
        intent: ThreadIntent,
        cx: &mut ThreadCx<'_, '_>,
    ) {
        let ThreadIntent::Set(name) = intent;
        if !name.trim().is_empty() {
            cx.request(Ctx::SetName, Command::SetName(name));
        }
    }

    fn note_tone(_note: &Note) -> Tone {
        Tone::Error
    }

    fn wire(command: &Command, thread: &ThreadId, id: RequestId) -> ClientRequest {
        wire::request(command, thread, id)
    }

    fn decode(command: &Command, result: serde_json::Value) -> Result<Reply, RpcError> {
        wire::reply(command, result)
    }
}
