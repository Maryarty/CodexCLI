//! `composer` (S10, composer): the core half of the screen's directory. The model
//! part, its events and intents, its RPCs and their protocol wiring live here,
//! with no rtwk; the view half is `src/pages/session/composer/` in the UI crate. Nothing
//! outside these two directories (bar their test files) needs an edit when it
//! grows.

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;

use crate::app::AppFx;
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::screens::Kind;
use crate::threads::ThreadFx;

pub mod clipboard;
pub mod history;
pub mod mentions;
pub mod restricted;

pub use crate::kit::paste::{LARGE_PASTE_CHAR_THRESHOLD, blockquote_paste, large_paste, sanitize_paste};

pub const NAME: &str = "composer";
pub const ID: &str = "S10";
pub const KIND: Kind = Kind::Part;

/// The longest message the backend takes, in characters
/// (`MAX_USER_INPUT_TEXT_CHARS`, `chat_composer.rs`).
pub const MAX_INPUT_CHARS: usize = codex_protocol::user_input::MAX_USER_INPUT_TEXT_CHARS;

/// The size of a message over the limit, or none for one the backend takes
/// (`prepare_submission_text`: Enter refuses it and keeps the draft).
pub fn too_large(text: &str) -> Option<usize> {
    let chars = text.chars().count();
    (chars > MAX_INPUT_CHARS).then_some(chars)
}

/// What a line submitted from a draft in shell mode asks of the shell
/// (`submit_shell_command`, `chatwidget/input_submission.rs`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShellLine {
    /// Nothing after the `!`: the help line (`Prefix a command with ! to run it locally`).
    Help,
    /// The command to run, trimmed.
    Run(String),
}

/// The shell command of `line` when the draft is in shell mode (the user
/// typed or recalled the `!`). A line that merely starts with `!` (a large
/// paste whose payload does, `hidden_shell_paste_submits_literal_prompt`)
/// is a prompt for the model, whole.
pub fn shell_line(line: &str, shell_mode: bool) -> Option<ShellLine> {
    let command = line.strip_prefix('!').filter(|_| shell_mode)?.trim();
    Some(if command.is_empty() {
        ShellLine::Help
    } else {
        ShellLine::Run(command.to_string())
    })
}

/// The app-scoped model state of this screen, changed only by `apply`.
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

/// A script's `screen` step for this directory: the event `value` stands for
/// (`fixtures::script`). None is seedable yet; the slice gives its events a
/// serde form here and reads it, so the shared loader never changes.
pub fn seed(value: serde_json::Value) -> Result<Ev, crate::screens::SeedError> {
    let _ = value;
    Err(crate::screens::SeedError::NoSeeds)
}

/// What the text before the caret asks a popup for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Trigger {
    pub kind: TriggerKind,
    /// Byte index of the sigil in the text.
    pub start: usize,
    /// What is typed after the sigil.
    pub query: String,
}

/// Which popup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriggerKind {
    /// `/` at the start of the text: the commands.
    Slash,
    /// `$` at the start of a word: skills.
    Skill,
    /// `@` at the start of a word: files and plugins.
    Mention,
}

/// The popup the text before the caret asks for, if any: a `/` command name
/// not yet followed by a blank (`slash_input.rs`), or a word that starts with
/// `$` or `@`. A blank after the word closes the popup.
pub fn trigger(before_caret: &str) -> Option<Trigger> {
    if let Some(query) = before_caret.strip_prefix('/')
        && !query.contains(char::is_whitespace)
    {
        return Some(Trigger {
            kind: TriggerKind::Slash,
            start: 0,
            query: query.to_string(),
        });
    }
    let start = before_caret
        .rfind(char::is_whitespace)
        .map_or(0, |at| at + before_caret[at..].chars().next().map_or(1, char::len_utf8));
    let word = &before_caret[start..];
    let kind = match word.chars().next()? {
        '$' => TriggerKind::Skill,
        '@' => TriggerKind::Mention,
        _ => return None,
    };
    Some(Trigger {
        kind,
        start,
        query: word[1..].to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::{MAX_INPUT_CHARS, ShellLine, sanitize_paste, shell_line, too_large};

    #[test]
    fn a_message_over_the_limit_is_too_large_by_its_characters() {
        assert_eq!(too_large(&"x".repeat(MAX_INPUT_CHARS)), None);
        assert_eq!(
            too_large(&"x".repeat(MAX_INPUT_CHARS + 1)),
            Some(MAX_INPUT_CHARS + 1)
        );
        // characters, not bytes
        assert_eq!(too_large(&"é".repeat(MAX_INPUT_CHARS)), None);
    }

    #[test]
    fn only_a_draft_in_shell_mode_runs_a_bang_line() {
        assert_eq!(
            shell_line("!  ls -la ", true),
            Some(ShellLine::Run("ls -la".into()))
        );
        assert_eq!(shell_line("!   ", true), Some(ShellLine::Help));
        // a paste whose payload starts with `!` is a literal prompt
        assert_eq!(shell_line("!echo xxx", false), None);
        assert_eq!(shell_line("ls", true), None);
    }

    #[test]
    fn sanitize_paste_removes_control_sequences_keeping_tabs_and_newlines() {
        for (pasted, expected) in [
            ("clean\ttext\n", "clean\ttext\n"),
            ("\x07before", "before"),
            ("\x1b[31mbefore", "before"),
            ("before\x1b[31", "before"),
            ("\x07[31m", "[31m"),
            ("\x07", ""),
            ("before\x1b[31mafter\x07", "beforeafter"),
            ("é\u{85}中", "é中"),
            ("before\x1bafter", "beforeafter"),
            ("a\r\nb\rc", "a\nb\nc"),
            ("_count_r\x1b[13;2:3uows\tindent\n\0two\u{7f}", "_count_rows\tindent\ntwo"),
        ] {
            assert_eq!(sanitize_paste(pasted), expected, "{pasted:?}");
        }
    }
}
