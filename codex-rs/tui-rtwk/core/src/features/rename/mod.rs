//! `/rename` as a feature slice (ADR-012; oracle `chatwidget/interaction.rs`
//! `show_rename_prompt`, `app/thread_title.rs` `suggest_thread_name`): the name of the thread in
//! use, the title the host's generator suggests for it, and the rename a submit makes. A bare
//! `/rename` checks that the thread can be renamed and asks the surface for the name prompt
//! (`Wish::Name`, prefilled with the thread's name); the prompt is the UI half's
//! (`src/features/rename/`) and says it wants the thread's suggestion (`Interest::Title`), which
//! this feature asks the model for while a prompt is open (`on_interest`, `AppCx::structured_turn`:
//! the recent messages of the conversation, the title schema) and keeps by thread
//! (`State::titles`): the prompt puts it in the field only while the user has not typed. A cloud
//! conversation owns its settings, so it is suggested no title, and neither is one that has no
//! message to name it by (oracle `generate_thread_title`, `suggest_thread_name`).
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

pub mod title;

use std::collections::BTreeMap;

use codex_app_server_protocol::{
    ClientRequest, RequestId, ThreadSetNameParams, ThreadSetNameResponse,
};

use super::{
    AppCx, Feature, Never, Remote, Slash, StructuredFailure, StructuredKind, StructuredTurn,
    ThreadCx, ThreadView,
};
use crate::commands::SlashCommand;
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::threads::{Item, MessagePhase, Tone};
use codex_protocol::openai_models::ReasoningEffort;
use title::Said;

/// What it answered with: nothing the feature reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {
    Ack,
}

crate::rpc_list! {
    /// The one ask.
    pub enum Command => Reply, request thread: &ThreadId;
    /// `thread/name/set` of the thread the ask is about.
    SetName(String), Self::SetName(name)
        => ThreadSetName(ThreadSetNameParams { thread_id: thread.to_string(), name: name.clone() })
        -> ThreadSetNameResponse, |_| Reply::Ack;
}

/// The thread's name, when it has one.
pub fn name<'a>(thread: ThreadView<'a>) -> Option<&'a str> {
    thread.slots().chat.name.as_deref()
}

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
    Set {
        thread: ThreadId,
        generation: Gen,
        name: String,
    },
}

/// What is asked of one thread.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThreadIntent {
    /// Rename the thread to this name (an empty one renames nothing).
    Set(String),
    /// The name prompt owns queued input until it closes or submits.
    AwaitName(Gen),
    /// This rename was cancelled before a name was submitted.
    CancelName(Gen),
    /// Carry the accepted name from the form to the server request.
    Submit { generation: Gen, name: String },
}

/// What an ask was for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ctx {
    /// `thread/name/set`.
    SetName,
    /// The result belongs to this submission of the prompt.
    Submitted(Gen),
    /// The title the model was asked to suggest for this thread's prompt, by the request's
    /// generation: the answer of a prompt that was closed and opened again belongs to the
    /// earlier request and is dropped (`State::titles`).
    Title(ThreadId, Gen),
}

/// What the surface is asked to show once (`Scope::Thread`): the name prompt (`Rename thread`
/// over a thread that has a name, `Name thread` over one that has none), answered by
/// `Intent::Set`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Wish {
    Name {
        thread: ThreadId,
        generation: Gen,
        /// The thread's name; empty when it has none.
        existing: String,
    },
}

/// What a surface can want of `/rename`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Interest {
    /// A name prompt for `thread` is open: its title suggestion is asked for and kept.
    Title { thread: ThreadId, generation: Gen },
}

impl Interest {
    pub(crate) fn thread(&self) -> &ThreadId {
        match self {
            Self::Title { thread, .. } => thread,
        }
    }
}

/// The title suggestions asked for, by thread: a request record (the answer is the generator's,
/// none when nothing came out).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    titles: BTreeMap<ThreadId, (Gen, Remote<Option<String>>)>,
    /// Each prompt, its title suggestion and its submitted name share an identity.
    generation: Gen,
}

/// What the name prompt submitted for a single thread.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ThreadPart {
    prompt: Option<Gen>,
    awaiting: Option<(Gen, String)>,
}

impl ThreadPart {
    pub(crate) fn holds_queue(&self) -> bool {
        self.prompt.is_some() || self.awaiting.is_some()
    }

    /// A matching notification confirms the server already stored the requested name.
    pub(crate) fn name_updated(&mut self, name: Option<&str>) -> bool {
        if self
            .awaiting
            .as_ref()
            .is_some_and(|(_, asked)| Some(asked.as_str()) == name)
        {
            self.awaiting = None;
            true
        } else {
            false
        }
    }

    pub(crate) fn connection_lost(&mut self) {
        // Only the submitted RPC depends on this transport.
        self.awaiting = None;
    }
}

/// Which request for a title or a name an answer belongs to.
pub type Gen = u64;

impl State {
    /// What is known of the suggestion for `thread`'s prompt: none when it was not asked for.
    pub fn title(&self, thread: &ThreadId, generation: Gen) -> Option<&Remote<Option<String>>> {
        self.titles
            .get(thread)
            .filter(|(current, _)| *current == generation)
            .map(|(_, title)| title)
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
fn open(state: &mut State, cx: &mut AppCx<'_>, source: Option<ThreadId>) {
    let Some(thread) = source.filter(|id| cx.view().thread(id).is_some()) else {
        return cx.note(None, Note::SessionStarting);
    };
    if blocked(cx) {
        return cx.note(Some(thread), Note::SideRename);
    }
    let existing = cx
        .view()
        .thread(&thread)
        .and_then(name)
        .map(str::to_owned)
        .unwrap_or_default();
    state.generation += 1;
    let generation = state.generation;
    cx.thread(&thread, ThreadIntent::AwaitName(generation));
    cx.wish(
        thread.clone(),
        Wish::Name {
            thread,
            generation,
            existing,
        },
    );
}

/// The name the user submitted: the thread is renamed, an empty name is the error line.
fn set(cx: &mut AppCx<'_>, thread: ThreadId, generation: Gen, text: &str) {
    match normalize_thread_name(text) {
        None => cx.note(Some(thread), Note::NameEmpty),
        Some(_) if blocked(cx) => cx.note(Some(thread), Note::SideRename),
        Some(name) => cx.thread(&thread, ThreadIntent::Submit { generation, name }),
    }
}

/// `generate_thread_title` for a rename suggestion: the structured turn that names `thread` from
/// its recent messages. None where the oracle suggests nothing: a conversation of a cloud
/// backend (the server owns its settings), or one with no message to name it by. The title
/// model is asked for (with a low effort) when the account is a ChatGPT one on the OpenAI
/// provider and the catalog has it; else the thread's own model answers.
fn suggestion(cx: &AppCx<'_>, thread: &ThreadId) -> Option<StructuredTurn> {
    let view = cx.view();
    let agents = &view.screens().agents;
    if agents.is_cloud(agents.owner_of(thread.as_str())) {
        return None;
    }
    let held = view.thread(thread)?;
    let entries: Vec<(String, Option<bool>)> = held
        .core()
        .transcript()
        .iter()
        .filter_map(|(_, item)| match item {
            Item::User(prompt) => Some((prompt.shown().text.into_owned(), None)),
            Item::Agent { text, phase, .. } => {
                Some((text.clone(), Some(*phase == Some(MessagePhase::Commentary))))
            }
            _ => None,
        })
        .collect();
    let conversation =
        title::recent_conversation(entries.iter().map(|(text, commentary)| match commentary {
            None => Said::User(text),
            Some(commentary) => Said::Assistant {
                text,
                commentary: *commentary,
            },
        }))?;
    let provider = held.slots().chat.provider.as_deref().or(view
        .screens()
        .chat
        .status
        .provider
        .as_deref());
    let account = view.facts().status();
    let luna = provider == Some("openai")
        && account.has_chatgpt_account()
        && account
            .catalog
            .iter()
            .any(|model| model.model == title::MODEL);
    Some(StructuredTurn {
        kind: StructuredKind::Title,
        source: thread.clone(),
        model: luna.then(|| title::MODEL.to_owned()),
        effort: luna.then_some(ReasoningEffort::Low),
        prompt: title::suggestion_prompt(&conversation),
        schema: title::output_schema(),
    })
}

impl Feature for Rename {
    const NAME: &'static str = "rename";
    const RUNS: &'static [SlashCommand] = &[SlashCommand::Rename];

    type State = State;
    type ThreadPart = ThreadPart;
    type Target = Never<Self>;
    type Intent = Intent;
    type ThreadIntent = ThreadIntent;
    type Command = Command;
    type Reply = Reply;
    type Ctx = Ctx;
    type Note = Note;
    type Fact = Never<Self>;
    type Wish = Wish;
    type Interest = Interest;
    type Event = Never<Self>;

    /// `/rename` and `/rename <name>`.
    fn slash(state: &mut State, slash: Slash, cx: &mut AppCx<'_>) {
        let source = slash.source.clone();
        let name = slash.into_args();
        Self::intent(state, Intent::Command { source, name }, cx);
    }

    fn intent(state: &mut State, intent: Intent, cx: &mut AppCx<'_>) {
        let (source, name) = match intent {
            Intent::Command { source, name } => (source, name),
            Intent::Set {
                thread,
                generation,
                name,
            } => return set(cx, thread, generation, &name),
        };
        match name {
            None => open(state, cx, source),
            Some(_) if blocked(cx) => cx.note(source, Note::SideRename),
            Some(name) => {
                if let Some(thread) = source {
                    cx.thread(&thread, ThreadIntent::Set(name));
                }
            }
        }
    }

    /// The model's answer to the prompt's ask: the title it generated, none when the answer is
    /// not one; kept for the prompt that waits for it.
    fn structured_done(
        state: &mut State,
        ctx: Ctx,
        result: Result<String, StructuredFailure>,
        _cx: &mut AppCx<'_>,
    ) {
        if let Ctx::Title(thread, at) = ctx
            && let Some((current, slot)) = state.titles.get_mut(&thread)
            && *current == at
        {
            *slot = Remote::Ready(result.ok().as_deref().and_then(title::parse));
        }
    }

    /// A name prompt is open (`open`): the model is asked for a title from the thread's
    /// conversation, once; the last prompt closed forgets it.
    fn on_interest(state: &mut State, interest: &Interest, open: bool, cx: &mut AppCx<'_>) {
        let Interest::Title { thread, generation } = interest;
        let at = *generation;
        if open {
            if state
                .titles
                .get(thread)
                .is_some_and(|(current, _)| *current > at)
            {
                return;
            }
            match suggestion(cx, thread) {
                Some(turn) => {
                    state.titles.insert(thread.clone(), (at, Remote::loading()));
                    cx.structured_turn(turn, Ctx::Title(thread.clone(), at));
                }
                // nothing to ask: the suggestion is none (`apply_thread_name_suggestion`)
                None => {
                    state
                        .titles
                        .insert(thread.clone(), (at, Remote::Ready(None)));
                }
            }
        } else {
            if state
                .titles
                .get(thread)
                .is_some_and(|(current, _)| *current == at)
            {
                state.titles.remove(thread);
            }
            cx.thread(thread, ThreadIntent::CancelName(at));
        }
    }

    fn thread_fact(part: &mut ThreadPart, intent: ThreadIntent) -> Option<ThreadIntent> {
        match intent {
            ThreadIntent::AwaitName(generation) => {
                part.prompt = Some(generation);
                None
            }
            ThreadIntent::CancelName(generation) => {
                if part.prompt == Some(generation) {
                    part.prompt = None;
                }
                None
            }
            ThreadIntent::Submit { generation, .. } => {
                if part.prompt != Some(generation) {
                    return None;
                }
                part.prompt = None;
                Some(intent)
            }
            ThreadIntent::Set(_) => Some(intent),
        }
    }

    fn thread_intent(part: &mut ThreadPart, intent: ThreadIntent, cx: &mut ThreadCx<'_, '_>) {
        match intent {
            ThreadIntent::Set(name) if !name.trim().is_empty() => {
                cx.request(Ctx::SetName, Command::SetName(name));
            }
            ThreadIntent::Submit { generation, name } => {
                if cx
                    .request(Ctx::Submitted(generation), Command::SetName(name.clone()))
                    .is_some()
                {
                    part.awaiting = Some((generation, name));
                }
            }
            ThreadIntent::Set(_) | ThreadIntent::AwaitName(_) | ThreadIntent::CancelName(_) => {}
        }
    }

    fn thread_answered(
        part: &mut ThreadPart,
        ctx: Ctx,
        _result: Result<Reply, RpcError>,
        _cx: &mut ThreadCx<'_, '_>,
    ) {
        if let Ctx::Submitted(generation) = ctx
            && part
                .awaiting
                .as_ref()
                .is_some_and(|(current, _)| *current == generation)
        {
            part.awaiting = None;
        }
    }

    fn note_tone(_note: &Note) -> Tone {
        Tone::Error
    }

    fn wire(command: &Command, thread: &ThreadId, id: RequestId) -> ClientRequest {
        command.request(thread, id)
    }

    fn decode(command: &Command, result: serde_json::Value) -> Result<Reply, RpcError> {
        command.reply(result)
    }
}
