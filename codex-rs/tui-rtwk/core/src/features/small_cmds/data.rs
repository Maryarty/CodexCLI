//! What the small commands carry as data (the host's facts, the failures their notes word): plain
//! data, worded by `src/features/small_cmds/`.

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
