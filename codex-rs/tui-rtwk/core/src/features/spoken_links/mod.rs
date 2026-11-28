//! A spoken answer links the files of the workspace it names (oracle `history_cell/
//! spoken_artifacts.rs`, `TrustedWorkspaceFile::validate`). A word of the answer that reads as a
//! path (`src/lib.rs:42`) is a link only when the host finds that file in the workspace: a view
//! has no file system, so a mounted answer says which paths it names (`Interest`), the host
//! checks them (`Ask::WorkspaceFiles`) and the slice keeps what it found while the answer is on
//! the screen.

use std::collections::BTreeMap;
use std::ops::Range;

use super::AppCx;
use crate::app::effects::{Ask, Done};

pub struct SpokenLinks;

/// The most words of one answer the oracle looks at (`MAX_ARTIFACT_CANDIDATES`).
const MAX_CANDIDATES: usize = 16;

/// A path a spoken answer names, in the workspace `cwd`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Interest {
    pub cwd: String,
    pub path: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Found {
    Pending,
    Link(String),
    Absent,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    entries: BTreeMap<Interest, Found>,
}

impl State {
    /// The `file:` URL of the workspace file `interest` names, once the host found it.
    pub fn url(&self, interest: &Interest) -> Option<&str> {
        match self.entries.get(interest) {
            Some(Found::Link(url)) => Some(url),
            _ => None,
        }
    }
}

feature! {
    impl Feature for SpokenLinks {
        const NAME: &'static str = "spoken_links";
        type State = State;
        type Ctx = Interest;
        type Interest = Interest;
        {
            fn on_interest(state: &mut State, interest: &Interest, open: bool, cx: &mut AppCx<'_>) {
                if !open {
                    state.entries.remove(interest);
                    return;
                }
                if state.entries.contains_key(interest) {
                    return;
                }
                state.entries.insert(interest.clone(), Found::Pending);
                cx.effect(
                    Ask::WorkspaceFiles { cwd: interest.cwd.clone(), paths: vec![interest.path.clone()] },
                    interest.clone(),
                );
            }

            fn effect_done(state: &mut State, ctx: Interest, done: Done, _cx: &mut AppCx<'_>) {
                if let Done::WorkspaceFiles(files) = done
                    && let Some(entry) = state.entries.get_mut(&ctx)
                {
                    *entry = files
                        .into_iter()
                        .find(|file| file.path == ctx.path)
                        .map_or(Found::Absent, |file| Found::Link(file.url));
                }
            }
        }
    }
}

/// A word of a line that reads as a workspace path: its bytes in the line (the path with the
/// brackets, quotes and `:line:column` suffix around it that the oracle also underlines is the
/// `bytes`; `path` is the path alone).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub bytes: Range<usize>,
    pub path: String,
}

/// The words of the lines of one answer that read as paths, the oracle's scan: words apart by
/// ASCII blanks, the brackets and quotes before and the punctuation after a word cut off, a word
/// whose last part (after `/`) has a dot, up to two `:digits` suffixes off the path. The oracle
/// looks at 16 words of an answer and no more.
#[derive(Debug, Default)]
pub struct Scan {
    inspected: usize,
}

impl Scan {
    pub fn new() -> Self {
        Self::default()
    }

    /// The candidates of the next line of the answer.
    pub fn line(&mut self, text: &str) -> Vec<Candidate> {
        let mut found = Vec::new();
        let mut search_from = 0;
        for token in text.split_ascii_whitespace() {
            let Some(relative_start) = text[search_from..].find(token) else {
                continue;
            };
            let token_start = search_from + relative_start;
            search_from = token_start + token.len();
            let leading = token.trim_start_matches(['(', '[', '{', '<', '\'', '"', '`']);
            let candidate =
                leading.trim_end_matches([')', ']', '}', '>', '\'', '"', '`', ',', ';', '.', '!']);
            let Some(filename) = candidate.rsplit('/').next() else {
                continue;
            };
            if !filename.contains('.') {
                continue;
            }
            let mut path = candidate;
            for _ in 0..2 {
                if let Some((without_suffix, suffix)) = path.rsplit_once(':')
                    && !suffix.is_empty()
                    && suffix.bytes().all(|byte| byte.is_ascii_digit())
                {
                    path = without_suffix;
                }
            }
            self.inspected += 1;
            if self.inspected > MAX_CANDIDATES {
                break;
            }
            let start = token_start + token.len() - leading.len();
            found.push(Candidate {
                bytes: start..start + candidate.len(),
                path: path.to_owned(),
            });
        }
        found
    }

    /// The answer has no more words the oracle looks at.
    pub fn exhausted(&self) -> bool {
        self.inspected > MAX_CANDIDATES
    }
}

#[cfg(test)]
mod tests {
    use super::Scan;

    #[test]
    fn a_path_word_is_a_candidate_with_its_line_suffix_inside_the_bytes() {
        let line = "中 (src/lib.rs:42), and https://example.com";
        let found = Scan::new().line(line);
        assert_eq!(found[0].path, "src/lib.rs");
        assert_eq!(&line[found[0].bytes.clone()], "src/lib.rs:42");
        // a URL has a dot in its last part too: the host finds no such file
        assert_eq!(found[1].path, "https://example.com");
    }

    #[test]
    fn only_sixteen_words_are_looked_at() {
        let mut scan = Scan::new();
        let line = (0..20)
            .map(|n| format!("a{n}.rs"))
            .collect::<Vec<_>>()
            .join(" ");
        assert_eq!(scan.line(&line).len(), 16);
        assert!(scan.exhausted());
        assert!(scan.line("b.rs").is_empty());
    }

    #[test]
    fn a_windows_path_keeps_its_backslash() {
        let found = Scan::new().line("Updated src\\lib.rs:42");
        assert_eq!(found[0].path, "src\\lib.rs");
    }
}
