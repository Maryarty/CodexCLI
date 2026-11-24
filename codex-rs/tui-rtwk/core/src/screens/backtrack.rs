//! `backtrack` (S17, Esc Esc): the core half. The oracle's `app_backtrack.rs`
//! (+ `browsing.rs`, `legacy_input.rs`) is one state machine over `App` fields
//! and the cells it can see; here it is three pure parts and one RPC pipeline.
//!
//! - `Backtrack`: the machine of Esc, Esc, Left/Right, Enter. It holds view
//!   state only (which prompt is highlighted), never thread data, so the UI's
//!   mode controller owns one value of it (`UiApp.ui`, ADR-003: the thread
//!   state stays in `CodexApp`). Every transition names its inputs and returns
//!   what the view must do.
//! - `user_positions` & co: the prompts a transcript offers, counted the
//!   oracle's way (visible prompts, nth from the oldest).
//! - `revert_before_turn`: which persisted turn a chosen prompt reverts before.
//!   The oracle counts visible prompts through the persisted turns again
//!   (`backtrack_revert_before_turn_id`); a transcript item carries its server
//!   ids (`ItemKey`), so the choice is an id and a stale ordinal cannot occur.
//! - `ThreadPart` + `Command`: the edit's RPC pipeline in the thread's slot
//!   (ADR-003: only `ThreadFx` sends thread RPCs): read the turns, resolve,
//!   `thread/revert`, then wait for `ThreadReverted`.

use std::fmt;

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::ImageReference;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::ThreadItemPayload;
use codex_app_server_protocol::ThreadReadResponse;
use codex_app_server_protocol::ThreadRevertResponse;
use codex_app_server_protocol::Turn;
use codex_app_server_protocol::TurnStatus;
use codex_app_server_protocol::UserInput;

use crate::app::AppFx;
use crate::ids::ThreadId;
use crate::ids::Ticket;
use crate::ids::TurnId;
use crate::rpc::RpcError;
use crate::screens::Kind;
use crate::threads::ItemKey;
use crate::threads::ThreadAsk;
use crate::threads::ThreadFx;
use crate::threads::Transcript;
use crate::threads::LocalImage;
use crate::threads::UserMessage;
use crate::threads::UserPrompt;

pub use crate::kit::backtrack::{Backtrack, Chosen as Selection, EscStep, Targets};
use crate::threads::Item;

/// A prompt the backtrack can offer: the oracle's `has_visible_content`
/// (a prompt of control characters only is hidden unless it has elements or
/// images).
pub fn is_visible(prompt: &UserPrompt) -> bool {
    !crate::kit::paste::sanitize_paste(&prompt.text).trim().is_empty()
        || !prompt.elements.is_empty()
        || !prompt.images.is_empty()
}

/// Positions (in transcript order) of the prompts the backtrack offers.
/// A transcript is one thread's session, so it starts at its first item (the
/// oracle starts after the last `SessionInfoCell` of a shared cell list).
pub fn user_positions(transcript: &Transcript) -> impl Iterator<Item = usize> + '_ {
    transcript
        .iter()
        .enumerate()
        .filter_map(|(index, (_, item))| match item {
            Item::User(prompt) if is_visible(prompt) => Some(index),
            _ => None,
        })
}

pub fn user_count(transcript: &Transcript) -> usize {
    user_positions(transcript).count()
}

pub fn has_backtrack_target(transcript: &Transcript) -> bool {
    user_count(transcript) > 0
}

/// The transcript position of the `nth` offered prompt.
pub fn nth_user_position(transcript: &Transcript, nth: usize) -> Option<usize> {
    user_positions(transcript).nth(nth)
}

/// The server ids and the prompt of the `nth` offered prompt.
pub fn nth_user(transcript: &Transcript, nth: usize) -> Option<(&ItemKey, &UserPrompt)> {
    let (key, item) = transcript.get(nth_user_position(transcript, nth)?)?;
    match item {
        Item::User(prompt) => Some((key, prompt)),
        _ => None,
    }
}

impl Targets {
    /// What the machine needs of `transcript` when a key arrives.
    pub fn of(transcript: &Transcript, older_history: bool) -> Self {
        Self {
            count: user_count(transcript),
            older_history,
        }
    }
}

impl Selection {
    /// The edit of the chosen prompt: its server ids, and the prompt as the transcript
    /// shows it (what goes back to the composer if the edit fails).
    pub fn edit(&self, transcript: &Transcript) -> Option<Edit> {
        let (key, prompt) = nth_user(transcript, self.nth)?;
        Some(Edit { key: key.clone(), shown: prompt.clone() })
    }
}

pub const NAME: &str = "backtrack";
pub const ID: &str = "S17";
pub const KIND: Kind = Kind::Part;

/// How long the edit waits for the server's `ThreadReverted` after `thread/revert`
/// answered. The oracle waits without a limit; ours says so after this long (the
/// view arms the timer, core has no clock: `ThreadPart::awaiting`).
pub const CONFIRM_SECS: u64 = 10;

/// Why an edit did not happen. The words are the chat's strings (ADR-010):
/// core says which one, with the server's own message for a failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditError {
    /// The read or the revert failed before the server changed anything
    /// (oracle `restore_backtrack_prompt_after_revert_error`).
    Failed(String),
    /// The revert was not confirmed in time.
    NotConfirmed,
    /// The thread is a side conversation (the chat knows; the machine does not).
    Side,
}

// ---------------------------------------------------------------------------
// Which turn a prompt reverts before

/// Why a chosen prompt cannot be edited; the texts are the oracle's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RevertError {
    /// Another prompt came before it in its turn: the server cannot revert
    /// into the middle of a turn.
    Steer,
    InProgress,
    NotFound,
}

impl fmt::Display for RevertError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Steer => "the selected prompt is a steer and cannot be edited independently",
            Self::InProgress => "the selected prompt belongs to a turn that is still in progress",
            Self::NotFound => "the selected prompt was not found in the persisted thread",
        })
    }
}

/// The prompt a persisted turn holds, as the composer restores it (oracle
/// `user_message_display_from_inputs`, `UserMessage`): the text with its
/// elements (placeholders kept), the images, and the skills and mentions the
/// input carried (the composer derives its mention bindings from them).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Prompt {
    pub text: String,
    pub elements: Vec<Element>,
    pub local_images: Vec<std::path::PathBuf>,
    pub remote_images: Vec<String>,
    pub mentions: Vec<Mention>,
}

/// An inline element of the text (a pasted-content or image placeholder).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Element {
    pub range: std::ops::Range<usize>,
    pub placeholder: Option<String>,
}

/// A skill or a mention of the input, as the server stored it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mention {
    pub skill: bool,
    pub name: String,
    pub path: String,
}

impl Prompt {
    /// The prompt of a stored user message: the revert and the replay's seed of the composer's
    /// history read it the same way.
    pub(crate) fn from_inputs(content: &[UserInput]) -> Self {
        let mut prompt = Self::default();
        for input in content {
            match input {
                UserInput::Text { text, text_elements, .. } => {
                    let offset = prompt.text.len();
                    prompt.text.push_str(text);
                    prompt.elements.extend(text_elements.iter().map(|element| {
                        let range = element.byte_range.start..element.byte_range.end;
                        Element {
                            placeholder: element
                                .placeholder()
                                .or_else(|| text.get(range.clone()))
                                .map(str::to_string),
                            range: range.start + offset..range.end + offset,
                        }
                    }));
                }
                UserInput::Image { image: ImageReference::Inline { url }, .. } => {
                    prompt.remote_images.push(url.clone())
                }
                UserInput::LocalImage { path, .. } => prompt.local_images.push(path.clone()),
                UserInput::Skill { name, path } => prompt.mentions.push(Mention {
                    skill: true,
                    name: name.clone(),
                    path: path.to_string_lossy().into_owned(),
                }),
                UserInput::Mention { name, path } => prompt.mentions.push(Mention {
                    skill: false,
                    name: name.clone(),
                    path: path.clone(),
                }),
                _ => {}
            }
        }
        prompt
    }
}

/// The turn to revert before and the persisted prompt (oracle
/// `backtrack_revert_before_turn_id`, resolved by the server ids of the
/// transcript item).
pub fn revert_before_turn(turns: &[Turn], key: &ItemKey) -> Result<(TurnId, Prompt), RevertError> {
    let turn = turns
        .iter()
        .find(|turn| turn.id == key.turn.as_str())
        .ok_or(RevertError::NotFound)?;
    let mut earlier_prompts = 0;
    for payload in turn.item_payloads() {
        let ThreadItemPayload::UserMessage { id, content, .. } = payload else { continue };
        if id != key.item.as_str() {
            earlier_prompts += 1;
            continue;
        }
        if earlier_prompts > 0 {
            return Err(RevertError::Steer);
        }
        if matches!(turn.status, TurnStatus::InProgress) {
            return Err(RevertError::InProgress);
        }
        return Ok((key.turn.clone(), Prompt::from_inputs(content)));
    }
    Err(RevertError::NotFound)
}

// ---------------------------------------------------------------------------
// The edit's RPCs, in the thread's slot

/// The edit of one prompt: the transcript item `key` names it; `shown` is what
/// goes back to the composer when the edit fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edit {
    pub key: ItemKey,
    pub shown: UserPrompt,
}

/// What goes back to the composer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Draft {
    /// The persisted prompt, after the server reverted before it.
    Persisted(Prompt),
    /// The prompt as the transcript shows it, when the edit failed.
    Shown(UserPrompt),
}

impl Draft {
    /// The draft as the composer takes a message (`restore_user_message_to_composer`):
    /// the text, the remote images, and each image placeholder of the persisted prompt paired
    /// with its local image in order. Skills, mentions and pasted-content elements are the
    /// composer's to bind (Need chunk-composer): their text stays as the server stored it.
    pub fn message(&self) -> UserMessage {
        match self {
            Draft::Shown(prompt) => UserMessage::text(prompt.text.clone()),
            Draft::Persisted(prompt) => {
                let placeholders = prompt.elements.iter().filter(|element| {
                    element
                        .placeholder
                        .as_deref()
                        .is_some_and(|text| text.starts_with("[Image #"))
                });
                UserMessage {
                    text: prompt.text.clone(),
                    images: placeholders
                        .zip(&prompt.local_images)
                        .map(|(element, path)| LocalImage {
                            path: path.clone(),
                            range: element.range.clone(),
                        })
                        .collect(),
                    remote_images: prompt.remote_images.clone(),
                    mentions: Vec::new(),
                }
            }
        }
    }
}

/// A draft for the composer, once per `seq` (the page applies each once).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DraftRestore {
    pub seq: u64,
    pub draft: Draft,
}

/// Where the edit is.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Stage {
    Reading,
    /// `thread/revert` is out; `confirmed` = `ThreadReverted` came before its answer.
    Reverting { prompt: Prompt, confirmed: bool },
    /// Reverted; waiting for the server's `ThreadReverted` (10 s; the edit's key guards the expiry).
    Awaiting(Ticket),
}

/// Per-thread state of this screen: a part of every `ThreadModel`, changed
/// only by `apply_thread`, `apply_thread_intent` and `apply_reply`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ThreadPart {
    edit: Option<(Edit, Stage)>,
    /// The draft to put back into the composer.
    pub restore: Option<DraftRestore>,
    /// The error line to show.
    pub error: Option<EditError>,
    seq: u64,
}

impl ThreadPart {
    /// An edit is under way: Esc Esc must not start another.
    pub fn editing(&self) -> bool {
        self.edit.is_some()
    }

    /// The prompt whose edit waits for the server's `ThreadReverted`, by its transcript key (the
    /// domain key of the operation): the view starts its `CONFIRM_SECS` timer when it appears and
    /// sends `ThreadIntent::Expired` with it.
    pub fn awaiting(&self) -> Option<ItemKey> {
        match &self.edit {
            Some((edit, Stage::Awaiting(_))) => Some(edit.key.clone()),
            _ => None,
        }
    }

    fn give(&mut self, draft: Draft) {
        self.seq += 1;
        self.restore = Some(DraftRestore { seq: self.seq, draft });
    }

    /// The edit failed before the server changed anything: the prompt goes back
    /// to the composer with the error (oracle `restore_backtrack_prompt_after_revert_error`).
    fn fail(&mut self, error: &dyn fmt::Display) {
        if let Some((edit, _)) = self.edit.take() {
            self.give(Draft::Shown(edit.shown));
        }
        self.error = Some(EditError::Failed(error.to_string()));
    }

    /// The turns were read: the prompt resolved (or the edit failed). Returns
    /// the turn to revert before.
    fn resolved(&mut self, found: Result<(TurnId, Prompt), RevertError>) -> Option<TurnId> {
        match found {
            Ok((turn, prompt)) => {
                let (_, stage) = self.edit.as_mut()?;
                *stage = Stage::Reverting { prompt, confirmed: false };
                Some(turn)
            }
            Err(error) => {
                self.fail(&error);
                None
            }
        }
    }

    /// The server reverted: the persisted prompt goes to the composer now (the
    /// oracle does not wait for the notification) and the edit waits for it.
    fn reverted(&mut self, ticket: Ticket) {
        let Some((edit, stage)) = self.edit.take() else { return };
        let Stage::Reverting { prompt, confirmed } = stage else {
            self.edit = Some((edit, stage));
            return;
        };
        self.give(Draft::Persisted(prompt));
        // the notification may have come before the answer: nothing to wait for
        if !confirmed {
            self.edit = Some((edit, Stage::Awaiting(ticket)));
        }
    }
}

/// The intents of this screen aimed at one thread; a variant of
/// `ScreenThreadIntent`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThreadIntent {
    EditPrompt(Edit),
    /// The chosen prompt cannot be edited: the thread is a side conversation (the chat's
    /// flag, which the thread slot does not see).
    Unavailable,
    /// The view's `CONFIRM_SECS` timer for `ThreadPart::awaiting` ran out.
    Expired(ItemKey),
}

/// An event of one thread for this screen; a variant of `ThreadEv::Screen`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThreadEv {
    /// `ServerNotification::ThreadReverted`: the server has the shorter history.
    Reverted,
    /// The 10 s wait for `Reverted` ran out, for the edit of the prompt `ItemKey` names.
    NotConfirmed(ItemKey),
}

/// The RPCs of this screen: requested with `ThreadFx::request`, which stamps
/// the thread and the ticket; the shell sends them through `wire`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// `thread/read` with the turns (the persisted truth), resolved to the
    /// prompt `key` names by `decode`.
    Read { key: ItemKey },
    /// `thread/revert`: drop `before_turn` and every later turn.
    Revert { before_turn: TurnId },
}

/// The replies of those RPCs, decoded by `decode`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {
    Resolved(Result<(TurnId, Prompt), RevertError>),
    Reverted,
}

/// What an outgoing RPC of this screen was for, kept with its ticket.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ctx {
    Read,
    Revert,
}

/// The app-scoped model state of this screen: none (the machine is the
/// view's, the edit is the thread's).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Model {}

/// The events of this screen (backend results, tickets); a variant of the
/// root `ScreenEv`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ev {}

/// The intents of this screen; a variant of the root `ScreenIntent`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {}

pub fn apply(model: &mut Model, ev: Ev, fx: &mut AppFx) {
    let _ = (model, fx);
    match ev {}
}

pub fn apply_intent(model: &mut Model, intent: Intent, fx: &mut AppFx) {
    let _ = (model, fx);
    match intent {}
}

pub fn apply_thread_intent(part: &mut ThreadPart, intent: ThreadIntent, fx: &mut ThreadFx<'_>) {
    match intent {
        ThreadIntent::EditPrompt(edit) if !part.editing() => {
            part.error = None;
            let key = edit.key.clone();
            part.edit = Some((edit, Stage::Reading));
            fx.request(Ctx::Read, Command::Read { key });
        }
        ThreadIntent::EditPrompt(_) => {}
        ThreadIntent::Unavailable => {
            part.error = Some(EditError::Side);
        }
        ThreadIntent::Expired(key) => apply_thread(part, ThreadEv::NotConfirmed(key), fx),
    }
}

pub fn apply_thread(part: &mut ThreadPart, ev: ThreadEv, fx: &mut ThreadFx<'_>) {
    let _ = fx;
    match ev {
        ThreadEv::Reverted => match &mut part.edit {
            Some((_, Stage::Awaiting(_))) => part.edit = None,
            Some((_, Stage::Reverting { confirmed, .. })) => *confirmed = true,
            _ => {}
        },
        ThreadEv::NotConfirmed(key) => {
            if matches!(&part.edit, Some((edit, Stage::Awaiting(_))) if edit.key == key) {
                part.edit = None;
                part.error = Some(EditError::NotConfirmed);
            }
        }
    }
}

pub fn apply_reply(
    part: &mut ThreadPart,
    ctx: Ctx,
    result: Result<Reply, RpcError>,
    fx: &mut ThreadFx<'_>,
) {
    match (ctx, result) {
        (Ctx::Read, Ok(Reply::Resolved(found))) => {
            if let Some(before_turn) = part.resolved(found) {
                fx.request(Ctx::Revert, Command::Revert { before_turn });
            }
        }
        (Ctx::Revert, Ok(Reply::Reverted)) => {
            let ticket = fx.mint();
            part.reverted(ticket);
        }
        (_, Ok(_)) => {}
        (_, Err(error)) => part.fail(&rpc_message(&error)),
    }
}

fn rpc_message(error: &RpcError) -> String {
    match error {
        RpcError::Remote(message) | RpcError::Malformed(message) => message.clone(),
        RpcError::Mismatch => "the reply is for another request".to_string(),
        RpcError::Prepare(text) => text.clone(),
        RpcError::Provider(error) => error.to_string(),
    }
}

/// The wire request of `command` for `thread`: the thread RPCs are the
/// thread's own (ADR-003, `threads::ThreadAsk`), this screen only names which.
pub fn wire(command: &Command, thread: &ThreadId, id: RequestId) -> ClientRequest {
    let thread = thread.to_string();
    match command {
        Command::Read { .. } => ThreadAsk::ReadTurns { thread },
        Command::Revert { before_turn } => ThreadAsk::Revert {
            thread,
            before_turn: before_turn.clone(),
        },
    }
    .wire(id)
}

/// The reply `result` (the JSON result of the request) is for `command`.
pub fn decode(command: &Command, result: serde_json::Value) -> Result<Reply, RpcError> {
    let malformed = |error: serde_json::Error| RpcError::Malformed(error.to_string());
    match command {
        Command::Read { key } => serde_json::from_value::<ThreadReadResponse>(result)
            .map(|response| Reply::Resolved(revert_before_turn(&response.thread.turns, key)))
            .map_err(malformed),
        Command::Revert { .. } => serde_json::from_value::<ThreadRevertResponse>(result)
            .map(|_| Reply::Reverted)
            .map_err(malformed),
    }
}

/// A script's `screen` step for this directory: none is seedable (the machine
/// is the view's state; the edit needs the server).
pub fn seed(value: serde_json::Value) -> Result<Ev, crate::screens::SeedError> {
    let _ = value;
    Err(crate::screens::SeedError::NoSeeds)
}
