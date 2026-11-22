//! A bounded window of the user's recent `$` picks, which orders the skill
//! popup (`bottom_pane/mention_usage.rs`). Pure: the window and its
//! frequencies. The file it is kept in (`mention_usage.json` under the Codex
//! home) is the shell's: it hands the window over as `Ev::MentionUsage` and
//! writes each pick it is asked to (`Ask::RecordMention`).

use std::collections::{HashMap, VecDeque};

/// How many picks the window keeps.
pub const HISTORY_LIMIT: usize = 50;

/// The recent picks, oldest first, by identity (the skill's path).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MentionUsage {
    recent: VecDeque<String>,
}

impl MentionUsage {
    /// The window of a file's lines, the newest `HISTORY_LIMIT` kept.
    pub fn from_recent(recent: impl IntoIterator<Item = String>) -> Self {
        let mut usage = Self::default();
        for identity in recent {
            usage.record(&identity);
        }
        usage
    }

    /// How often each identity is in the window.
    pub fn frequencies(&self) -> HashMap<String, usize> {
        let mut counts = HashMap::new();
        for identity in &self.recent {
            *counts.entry(identity.clone()).or_default() += 1;
        }
        counts
    }

    /// Adds a pick; the oldest leaves when the window is full.
    pub fn record(&mut self, identity: &str) {
        if self.recent.len() == HISTORY_LIMIT {
            self.recent.pop_front();
        }
        self.recent.push_back(identity.to_owned());
    }

    /// The window, oldest first (what the shell writes).
    pub fn recent(&self) -> impl Iterator<Item = &str> {
        self.recent.iter().map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_window_keeps_the_newest_picks_and_counts_them() {
        let mut usage = MentionUsage::default();
        for n in 0..HISTORY_LIMIT + 3 {
            usage.record(&["skill://", &(n % 2).to_string()].concat());
        }
        assert_eq!(usage.recent().count(), HISTORY_LIMIT);
        let counts = usage.frequencies();
        assert_eq!(counts.values().sum::<usize>(), HISTORY_LIMIT);
        usage.record("skill://new");
        assert_eq!(usage.recent().last(), Some("skill://new"));
        assert_eq!(usage.recent().count(), HISTORY_LIMIT);
    }
}
