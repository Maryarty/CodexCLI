//! `/rename` as a feature slice (ADR-012; oracle `chatwidget/interaction.rs`
//! `show_rename_prompt`, `app/thread_title.rs` `suggest_thread_name`): the name of the thread in
//! use, the title the host's generator suggests for it, and the rename a submit makes. A bare
//! `/rename` checks that the thread can be renamed and asks the surface for the name prompt
//! (`Wish::Name`, prefilled with the thread's name); the prompt is the UI half's
//! (`src/features/rename/`) and says it wants the thread's suggestion (`Interest::Title`), which
//! this feature asks the host for while a prompt is open (`on_interest`, `AppCx::effect`) and keeps
//! by thread (`State::titles`): the prompt puts it in the field only while the user has not typed.
//! A submit, or `/rename <name>`, or `/new <name>`, renames the thread through the feature's own
//! RPC (`thread/name/set`, `Command::SetName`). Words: `src/features/rename/strings.rs`.
//!
//! `/new <name>` reaches the rename through the thread intent the router built
//! (`ThreadIntent::Set`), applied to the new thread once it is started: no hook, the app's
//! generic route for a feature's thread intent.
//!
//! A failed rename is the thread's last failure (the framework sets it for any feature RPC that
//! failed on a thread, `threads::feature_reply`); `ThreadAsk::Rename` stays in `threads` for
//! the agents' rename of another thread.

mod wire;

use std::collections::BTreeMap;

use codex_app_server_protocol::{ClientRequest, RequestId};

use super::{AppCx, Feature, Never, Remote, ThreadCx};
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
    /// The name prompt submitted for `thread`: an empty name is the error line.
    Set { thread: ThreadId, name: String },
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
    /// The title the host was asked to suggest for this thread's prompt.
    Title(ThreadId),
}

/// What the surface is asked to show once (`Scope::Thread`): the name prompt (`Rename thread`
/// over a thread that has a name, `Name thread` over one that has none), answered by
/// `Intent::Set`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Wish {
    Name {
        thread: ThreadId,
        /// The thread's name; empty when it has none.
        existing: String,
    },
}

/// What a surface can want of `/rename`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Interest {
    /// A name prompt for `thread` is open: its title suggestion is asked for and kept.
    Title { thread: ThreadId },
}

/// The title suggestions asked for, by thread: a request record (the answer is the generator's,
/// none when nothing came out).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    titles: BTreeMap<ThreadId, Remote<Option<String>>>,
}

impl State {
    /// What is known of the suggestion for `thread`'s prompt: none when it was not asked for.
    pub fn title(&self, thread: &ThreadId) -> Option<&Remote<Option<String>>> {
        self.titles.get(thread)
    }
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

/// A bare `/rename`: the prompt over the thread in use is asked of the surface, prefilled with
/// the thread's name.
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
        .map(str::to_owned)
        .unwrap_or_default();
    cx.wish(
        thread.clone(),
        Wish::Name {
            thread,
            existing,
        },
    );
}

/// The name the user submitted: the thread is renamed, an empty name is the error line.
fn set(cx: &mut AppCx<'_>, thread: ThreadId, text: &str) {
    match normalize_thread_name(text) {
        None => cx.note(Some(thread), Note::NameEmpty),
        Some(_) if blocked(cx) => cx.note(Some(thread), Note::SideRename),
        Some(name) => cx.thread(&thread, ThreadIntent::Set(name)),
    }
}

impl Feature for Rename {
    const NAME: &'static str = "rename";

    type State = State;
    type ThreadPart = ();
    type Intent = Intent;
    type ThreadIntent = ThreadIntent;
    type Command = Command;
    type Reply = Reply;
    type Ctx = Ctx;
    type Note = Note;
    type Fact = Never<Self>;
    type Wish = Wish;
    type Interest = Interest;

    fn intent(_state: &mut State, intent: Intent, cx: &mut AppCx<'_>) {
        let (source, name) = match intent {
            Intent::Command { source, name } => (source, name),
            Intent::Set { thread, name } => return set(cx, thread, &name),
        };
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

    /// The generator's answer to the prompt's ask: kept for the prompt that waits for it.
    fn effect_done(state: &mut State, ctx: Ctx, done: Done, cx: &mut AppCx<'_>) {
        let _ = cx;
        if let (Ctx::Title(thread), Done::Title(title)) = (ctx, done)
            && state.titles.contains_key(&thread)
        {
            state.titles.insert(thread, Remote::Ready(title));
        }
    }

    /// A name prompt is open (`open`): the host is asked for a title from the thread's
    /// conversation, once; the last prompt closed forgets it.
    fn on_interest(state: &mut State, interest: &Interest, open: bool, cx: &mut AppCx<'_>) {
        let Interest::Title { thread } = interest;
        if open {
            state.titles.insert(thread.clone(), Remote::loading());
            cx.effect(
                Ask::SuggestTitle {
                    thread: thread.clone(),
                },
                Ctx::Title(thread.clone()),
            );
        } else {
            state.titles.remove(thread);
        }
    }

    fn thread_intent(_part: &mut (), intent: ThreadIntent, cx: &mut ThreadCx<'_, '_>) {
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
