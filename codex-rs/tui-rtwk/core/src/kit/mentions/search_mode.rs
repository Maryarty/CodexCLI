//! The tabs of the unified popup (`mentions_v2/search_mode.rs`, carried):
//! which kinds of row show. Left and Right walk them (`TabController` in
//! the view).

use crate::screens::composer::mentions::candidate::MentionType;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SearchMode {
    #[default]
    Results,
    FilesystemOnly,
    Tools,
}

impl SearchMode {
    /// The tabs in order.
    pub const ALL: [SearchMode; 3] = [Self::Results, Self::FilesystemOnly, Self::Tools];

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|mode| *mode == self).unwrap_or(0)
    }

    pub fn previous(self) -> Self {
        Self::ALL[(self.index() + Self::ALL.len() - 1) % Self::ALL.len()]
    }

    pub fn next(self) -> Self {
        Self::ALL[(self.index() + 1) % Self::ALL.len()]
    }

    pub fn accepts(self, mention_type: MentionType) -> bool {
        match self {
            Self::Results => true,
            Self::FilesystemOnly => mention_type.is_filesystem(),
            Self::Tools => matches!(mention_type, MentionType::Plugin | MentionType::Skill),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tabs_walk_round_both_ways() {
        assert_eq!(SearchMode::Results.next(), SearchMode::FilesystemOnly);
        assert_eq!(SearchMode::Tools.next(), SearchMode::Results);
        assert_eq!(SearchMode::Results.previous(), SearchMode::Tools);
        assert_eq!(SearchMode::FilesystemOnly.previous(), SearchMode::Results);
    }

    #[test]
    fn each_tab_accepts_its_kinds() {
        assert!(SearchMode::Tools.accepts(MentionType::Plugin));
        assert!(!SearchMode::Tools.accepts(MentionType::Task));
        assert!(SearchMode::FilesystemOnly.accepts(MentionType::Directory));
        assert!(!SearchMode::FilesystemOnly.accepts(MentionType::Skill));
        assert!(SearchMode::Results.accepts(MentionType::Task));
    }
}
