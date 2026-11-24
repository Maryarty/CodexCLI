//! What the host answers the small commands' asks with (`Done::LoggedOut`, `Done::Ide`,
//! `Done::Daybreak`, `Done::DebugConfig`): plain data, worded by `src/features/small_cmds/`.

/// What the IDE said of its context (oracle `has_prompt_context` of the fetched context).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdeReading {
    /// There is a selection or open tabs future messages will carry.
    pub has_context: bool,
}

/// Why the IDE context could not be read, with the two hints the oracle words it by
/// (`IdeContextError::user_facing_hint`, `prompt_skip_hint`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdeFailure {
    /// The hint of `/ide`'s own failure line.
    pub hint: String,
    /// The hint of the line that says a message went without IDE context.
    pub skip_hint: String,
}

/// Why a thread's Daybreak choice was not saved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DaybreakFailure {
    /// `thread/metadata/update` failed: the server's text.
    Save(String),
    /// `thread/read` failed: the server's text.
    Read(String),
}

/// What went wrong with the default preference after the thread's choice was saved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DefaultFailure {
    /// The config write failed.
    Save(String),
    /// The effective config could not be read back.
    Read(String),
}

/// A thread's Daybreak choice was saved (or the thread is ephemeral: nothing to save).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DaybreakSaved {
    pub defaults: Vec<DefaultFailure>,
}

/// How a row of `/debug-config` is drawn (oracle: the `Stylize` of the line).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Style {
    #[default]
    Plain,
    Bold,
    Dim,
    Magenta,
}

/// One line of `/debug-config`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Row {
    pub text: String,
    pub style: Style,
}

impl From<&str> for Row {
    fn from(text: &str) -> Self {
        Row {
            text: text.to_owned(),
            style: Style::Plain,
        }
    }
}

impl From<String> for Row {
    fn from(text: String) -> Self {
        Row {
            text,
            style: Style::Plain,
        }
    }
}
