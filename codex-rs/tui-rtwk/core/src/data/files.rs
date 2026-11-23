//! File search for the `@` popup as values: what the popup asks of a backend ([`FileSearchAsk`])
//! and what comes back ([`FileMatch`], [`FileMatches`]).

/// What kind of path a match is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatchType {
    File,
    Directory,
}

/// One path the search found (`codex_file_search::FileMatch`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileMatch {
    /// Relative to the search root.
    pub path: String,
    pub match_type: MatchType,
    pub score: u32,
    /// The character positions of the path the query matched.
    pub indices: Option<Vec<u32>>,
}

/// The answer of the file search that came in last (the model keeps it).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FileMatches {
    pub query: String,
    pub matches: Vec<FileMatch>,
}

/// What the popup needs of the backend: a query to run (the empty one resets
/// the session so an identical query answers again), or an end to the
/// session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileSearchAsk {
    Query(String),
    Stop,
}
