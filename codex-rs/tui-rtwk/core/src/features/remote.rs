//! `Remote<T>`: a request record (ADR-013 З) — what the server answered for a key, as the
//! domain keeps it. A cache by key, never "the current X": the same feature asked about two
//! directories holds two. The Flutter counterpart is `AsyncValue<T>` of riverpod.

/// What is known of one asked thing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Remote<T> {
    /// The ask is out. The last good answer, if there was one, is still shown meanwhile
    /// (a refresh does not blank the page).
    Loading(Option<T>),
    /// The server's answer.
    Ready(T),
    /// The ask failed: the words of the failure.
    Failed(String),
}

impl<T> Remote<T> {
    /// A first ask, nothing to show yet.
    pub fn loading() -> Self {
        Remote::Loading(None)
    }

    /// The ask is out again: what was known stays on show.
    pub fn reloading(self) -> Self {
        match self {
            Remote::Ready(value) | Remote::Loading(Some(value)) => Remote::Loading(Some(value)),
            Remote::Loading(None) | Remote::Failed(_) => Remote::Loading(None),
        }
    }

    /// The data on show: the answer, or the one a refresh is replacing.
    pub fn value(&self) -> Option<&T> {
        match self {
            Remote::Ready(value) | Remote::Loading(Some(value)) => Some(value),
            Remote::Loading(None) | Remote::Failed(_) => None,
        }
    }

    pub fn value_mut(&mut self) -> Option<&mut T> {
        match self {
            Remote::Ready(value) | Remote::Loading(Some(value)) => Some(value),
            Remote::Loading(None) | Remote::Failed(_) => None,
        }
    }

    /// An ask is out.
    pub fn is_loading(&self) -> bool {
        matches!(self, Remote::Loading(_))
    }

    /// The failure, when it failed.
    pub fn failure(&self) -> Option<&str> {
        match self {
            Remote::Failed(why) => Some(why),
            _ => None,
        }
    }
}
