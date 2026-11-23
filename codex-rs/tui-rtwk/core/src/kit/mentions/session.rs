//! The mention popup's life in a composer (`chat_composer.rs` `sync_popups`
//! for the `@` and `$` popups, `popup_state.rs`, the `insert_selected_*`
//! family): which popup the text before and around the caret calls for, what
//! a dismissed token stays hidden for, what the file search is asked, and
//! what a pick puts in the draft. Pure: the draft is read as a [`Doc`], a
//! pick comes back as a [`Pick`] the view applies to its controller, a
//! search as a [`FileSearchAsk`] the shell runs.

use std::collections::HashMap;
use std::ops::Range;

use super::catalog::build_search_catalog;
use super::codec::is_mention_name_char;
use super::dollar::{self, MentionItem, mention_items};
use super::popup::Popup;
use super::search_mode::SearchMode;
use super::target::{
    Doc, DollarQueryKind, current_prefixed_token_range,
    current_prefixed_token_range_with_dollar_predicate, dollar_query_kind,
    prefixed_token_range_is_editable,
};
use crate::data::files::{FileMatches, FileSearchAsk};
use crate::data::mention::Selection;
use crate::data::mention::{Mention, MentionKind};
use crate::data::task::{TaskMention, valid_thread_path};
use crate::data::usage::MentionUsage;

/// What the popups are fed from: the model's data.
#[derive(Clone, Copy)]
pub struct Sources<'a> {
    pub mentions: &'a [Mention],
    pub tasks: &'a [TaskMention],
    pub usage: &'a MentionUsage,
    pub file_matches: &'a FileMatches,
}

impl Sources<'_> {
    /// Whether `path` is a skill's (its separator may give way to punctuation:
    /// `skill_completion_space`).
    pub fn is_skill(&self, path: &str) -> bool {
        self.mentions
            .iter()
            .any(|m| m.kind == MentionKind::Skill && m.path.as_deref() == Some(path))
    }
}

/// One token occurrence whose popup stays hidden (`DismissedToken`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dismissed {
    /// The popup query, without the sigil.
    query: String,
    /// The token text, sigil included.
    token: String,
    /// How many identical complete tokens stand before it.
    occurrence: usize,
}

impl Dismissed {
    pub fn new(doc: &Doc, range: Range<usize>, query: String) -> Self {
        let token = doc.text[range.clone()].to_string();
        let occurrence = complete_token_occurrences_before(doc, &token, range.start);
        Self {
            query,
            token,
            occurrence,
        }
    }

    /// Whether `range` is the same occurrence in the current draft: byte
    /// offsets may shift, the text and its ordinal do not.
    pub fn matches(&self, doc: &Doc, range: &Range<usize>, query: &str) -> bool {
        if self.query != query || doc.text.get(range.clone()) != Some(self.token.as_str()) {
            return false;
        }
        complete_token_occurrences_before(doc, &self.token, range.start) == self.occurrence
    }
}

/// Complete editable-token occurrences before `before`: whitespace and the
/// edges of atomic elements delimit them, bare nested sigils do not.
pub fn complete_token_occurrences_before(doc: &Doc, token: &str, before: usize) -> usize {
    let text = doc.text;
    text[..before]
        .match_indices(token)
        .filter(|(start, _)| {
            let end = start + token.len();
            let ends_at_boundary = text[end..].chars().next().is_none_or(char::is_whitespace)
                || doc.elements.iter().any(|element| element.start == end);
            let starts_at_boundary = *start == 0
                || text[..*start]
                    .chars()
                    .next_back()
                    .is_some_and(char::is_whitespace)
                || doc.elements.iter().any(|element| element.end == *start);
            starts_at_boundary && ends_at_boundary
        })
        .count()
}

/// The `$` popup: skills and apps, ordered by the user's recent picks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DollarPopup {
    query: String,
    items: Vec<MentionItem>,
    frequencies: HashMap<String, usize>,
}

impl DollarPopup {
    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn items(&self) -> &[MentionItem] {
        &self.items
    }

    /// The rows the query leaves, best first.
    pub fn rows(&self) -> Vec<dollar::Match> {
        dollar::filtered(&self.items, &self.frequencies, &self.query)
    }

    fn item_at(&self, position: usize) -> Option<&MentionItem> {
        let rows = self.rows();
        self.items.get(rows.get(position)?.index)
    }
}

/// The popup that is up (at most one).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Active {
    #[default]
    None,
    Dollar(DollarPopup),
    At(Popup),
}

/// The mention popups' state in one composer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MentionSession {
    active: Active,
    dismissed: Option<Dismissed>,
    current_file_query: Option<String>,
}

/// A token the caret is on.
struct MentionTarget {
    range: Range<usize>,
    query: String,
}

impl MentionSession {
    pub fn active(&self) -> &Active {
        &self.active
    }

    /// Whether a popup is up.
    pub fn is_open(&self) -> bool {
        !matches!(self.active, Active::None)
    }

    /// The `@` popup, when it is the one up.
    pub fn at_popup(&self) -> Option<&Popup> {
        match &self.active {
            Active::At(popup) => Some(popup),
            _ => None,
        }
    }

    /// Whether `@` or `$` calls for a popup at the caret: the slash popup
    /// then stays down (`allow_command_popup`).
    pub fn claims_caret(&self, doc: &Doc, src: &Sources) -> bool {
        at_target(doc).is_some() || dollar_target(doc, src.mentions).is_some()
    }

    /// Closes the popup without remembering a dismissal (history search, the
    /// Vim `/` query, browsing the history): a running search is ended.
    pub fn close(&mut self) -> Vec<FileSearchAsk> {
        self.active = Active::None;
        self.dismissed = None;
        self.end_file_search()
    }

    /// Closes the popup (a selection is being made with the mouse): a
    /// dismissal stays.
    pub fn hide(&mut self) {
        self.active = Active::None;
    }

    fn end_file_search(&mut self) -> Vec<FileSearchAsk> {
        if self.current_file_query.take().is_some() {
            vec![FileSearchAsk::Query(String::new())]
        } else {
            Vec::new()
        }
    }

    /// Brings the popup in line with the draft (`sync_popups`): the token
    /// at the caret decides which popup and its query. The asks are what the
    /// shell runs.
    pub fn sync(&mut self, doc: &Doc, src: &Sources) -> Vec<FileSearchAsk> {
        let mut at = at_target(doc);
        let mut dollar = dollar_target(doc, src.mentions);
        // the later token wins where both sigils touch the caret
        if let (Some(a), Some(d)) = (&at, &dollar) {
            if a.range.start > d.range.start {
                dollar = None;
            } else if d.range.start > a.range.start {
                at = None;
            }
        }
        if let Some(MentionTarget { range, query }) = at {
            return self.sync_at(doc, src, range, query);
        }
        let mut asks = self.end_file_search();
        if let Some(MentionTarget { range, query }) = dollar {
            self.sync_dollar(doc, src, range, query);
            return asks;
        }
        self.dismissed = None;
        if !matches!(self.active, Active::None) {
            self.active = Active::None;
        }
        asks.dedup();
        asks
    }

    fn sync_at(
        &mut self,
        doc: &Doc,
        src: &Sources,
        range: Range<usize>,
        query: String,
    ) -> Vec<FileSearchAsk> {
        if self
            .dismissed
            .as_ref()
            .is_some_and(|dismissed| dismissed.matches(doc, &range, &query))
        {
            return Vec::new();
        }
        let mut asks = Vec::new();
        if query.is_empty() {
            asks.push(FileSearchAsk::Query(String::new()));
            self.current_file_query = None;
        } else {
            let new_popup = !matches!(self.active, Active::At(_));
            if new_popup {
                // A fresh popup has no matches and the shell's search may hold an identical
                // query: it is reset first so the answer comes.
                asks.push(FileSearchAsk::Query(String::new()));
            }
            if new_popup || self.current_file_query.as_deref() != Some(query.as_str()) {
                asks.push(FileSearchAsk::Query(query.clone()));
                self.current_file_query = Some(query.clone());
            }
        }
        match &mut self.active {
            Active::At(popup) => popup.set_query(&query),
            _ => {
                self.active = Active::At(Popup::new(
                    build_search_catalog(src.mentions, src.tasks),
                    &query,
                ));
            }
        }
        if let Active::At(popup) = &mut self.active
            && !src.file_matches.query.is_empty()
        {
            popup.set_file_matches(&src.file_matches.query, src.file_matches.matches.clone());
        }
        self.dismissed = None;
        asks
    }

    fn sync_dollar(&mut self, doc: &Doc, src: &Sources, range: Range<usize>, query: String) {
        if self
            .dismissed
            .as_ref()
            .is_some_and(|dismissed| dismissed.matches(doc, &range, &query))
        {
            return;
        }
        let items = mention_items(src.mentions);
        if items.is_empty() {
            self.active = Active::None;
            return;
        }
        match &mut self.active {
            Active::Dollar(popup) => {
                popup.query = query;
                popup.items = items;
            }
            _ => {
                self.active = Active::Dollar(DollarPopup {
                    query,
                    items,
                    frequencies: src.usage.frequencies(),
                });
            }
        }
    }

    /// Whether Left and Right switch the `@` popup's tab: only while the
    /// caret is on an editable `@` token (else they move the caret).
    pub fn can_switch_search_mode(&self, doc: &Doc) -> bool {
        matches!(self.active, Active::At(_)) && at_target(doc).is_some()
    }

    pub fn previous_search_mode(&mut self) {
        if let Active::At(popup) = &mut self.active {
            popup.previous_search_mode();
        }
    }

    /// The `@` popup's tab `mode` (a tap on a tab).
    pub fn set_search_mode(&mut self, mode: SearchMode) {
        if let Active::At(popup) = &mut self.active {
            popup.set_search_mode(mode);
        }
    }

    pub fn next_search_mode(&mut self) {
        if let Active::At(popup) = &mut self.active {
            popup.next_search_mode();
        }
    }

    /// Esc: the popup closes and stays closed for this very token.
    pub fn dismiss(&mut self, doc: &Doc, src: &Sources) {
        let target = match &self.active {
            Active::At(_) => at_target(doc),
            Active::Dollar(_) => dollar_target(doc, src.mentions),
            Active::None => None,
        };
        if let Some(MentionTarget { range, query }) = target {
            self.dismissed = Some(Dismissed::new(doc, range, query));
        }
        self.active = Active::None;
    }

    /// What picking row `position` of the popup puts in the draft; `None`
    /// when there is no such row or the caret left the token. The popup is
    /// closed by the caller through [`Session::finish_pick`].
    pub fn pick(&self, position: usize, doc: &Doc, src: &Sources) -> Option<MentionPick> {
        match &self.active {
            Active::At(popup) => {
                let (range, _) = at_token_range(doc)?;
                let row = popup.rows().get(position)?;
                Some(match &row.selection {
                    Selection::File(path) => MentionPick::file(range, path),
                    Selection::Tool { insert_text, path } => {
                        MentionPick::mention(range, insert_text, path.as_deref())
                    }
                })
            }
            Active::Dollar(popup) => {
                let target = dollar_target(doc, src.mentions)?;
                let item = popup.item_at(position)?;
                let mut pick =
                    MentionPick::mention(target.range, &item.insert_text, item.path.as_deref());
                pick.usage = item.path.clone();
                Some(pick)
            }
            Active::None => None,
        }
    }

    /// The pick was applied (`doc` is the draft after it): the popup closes
    /// and the completed token stays dismissed until its text changes.
    pub fn finish_pick(&mut self, pick: &MentionPick, inserted: Range<usize>, doc: &Doc) {
        self.active = Active::None;
        let text = pick.inserted_text();
        if let Some(sigil) = text.chars().next().filter(|c| matches!(c, '$' | '@')) {
            self.dismiss_completed(sigil, inserted, text, doc);
        }
    }

    fn dismiss_completed(&mut self, prefix: char, inserted: Range<usize>, text: &str, doc: &Doc) {
        let Some(completed) = text.strip_prefix(prefix) else {
            return;
        };
        // The caret rests on the separator, where token affinity would otherwise reopen the
        // popup for sigil-prefixed text.
        let Some((range, token)) = current_prefixed_token_range(doc, prefix, true) else {
            return;
        };
        if range != inserted || token != completed {
            return;
        }
        self.dismissed = Some(Dismissed::new(doc, range, token));
    }
}

/// The binding a mention element carries (`ComposerMentionBinding`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComposerBinding {
    pub sigil: char,
    pub mention: String,
    pub path: String,
}

/// What a pick puts in the draft in place of the token at `range`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MentionPick {
    pub range: Range<usize>,
    pub action: PickAction,
    /// The skill whose pick the usage file keeps (by its path).
    pub usage: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PickAction {
    /// A path as plain text (quoted when it holds whitespace).
    Path(String),
    /// An image path: the view attaches it as an image when it can, else
    /// it is the path as text.
    Image { path: String, text: String },
    /// A mention as one atomic element with its binding.
    Element {
        text: String,
        binding: Option<ComposerBinding>,
    },
}

impl MentionPick {
    fn file(range: Range<usize>, path: &str) -> Self {
        // A path with whitespace is quoted so an argument parser takes it whole; one that
        // holds a quote already is left alone.
        let needs_quotes = path.chars().any(char::is_whitespace);
        let text = if needs_quotes && !path.contains('"') {
            format!("\"{path}\"")
        } else {
            path.to_string()
        };
        let action = if is_image_path(path) {
            PickAction::Image {
                path: path.to_string(),
                text,
            }
        } else {
            PickAction::Path(text)
        };
        Self {
            range,
            action,
            usage: None,
        }
    }

    fn mention(range: Range<usize>, insert_text: &str, path: Option<&str>) -> Self {
        let binding = path.and_then(|path| {
            let token = if valid_thread_path(path).is_some() {
                insert_text
                    .strip_prefix('@')
                    .filter(|name| !name.is_empty())
                    .map(|name| ('@', name.to_string()))
            } else {
                mention_token_from_insert_text(insert_text)
            };
            token.map(|(sigil, mention)| ComposerBinding {
                sigil,
                mention,
                path: path.to_string(),
            })
        });
        Self {
            range,
            action: PickAction::Element {
                text: insert_text.to_string(),
                binding,
            },
            usage: None,
        }
    }

    /// The text that goes in.
    pub fn inserted_text(&self) -> &str {
        match &self.action {
            PickAction::Path(text) | PickAction::Image { text, .. } => text,
            PickAction::Element { text, .. } => text,
        }
    }

    /// Whether the pick names a task (the task list is spent).
    pub fn is_task(&self) -> bool {
        matches!(
            &self.action,
            PickAction::Element { binding: Some(b), .. } if valid_thread_path(&b.path).is_some()
        )
    }
}

fn is_image_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    [".png", ".jpg", ".jpeg", ".gif", ".webp"]
        .iter()
        .any(|ext| lower.ends_with(ext))
}

/// `$name` or `@name` as a sigil and a name made of mention characters.
pub fn mention_token_from_insert_text(insert_text: &str) -> Option<(char, String)> {
    let sigil = insert_text.chars().next()?;
    if !matches!(sigil, '$' | '@') {
        return None;
    }
    let name = &insert_text[sigil.len_utf8()..];
    if name.is_empty() {
        return None;
    }
    name.as_bytes()
        .iter()
        .all(|byte| is_mention_name_char(*byte))
        .then(|| (sigil, name.to_string()))
}

/// What to do about the separator after a completion
/// (`advance_past_completion_separator`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Separator {
    /// One horizontal separator follows: the caret goes after it.
    MoveCaretTo(usize),
    /// None does (or another would merge): a space is inserted at the caret.
    InsertSpaceAt(usize),
}

/// The caret rests after one horizontal separator following a completion.
/// Another separator is kept before any non-whitespace suffix so typing does
/// not merge into it; a line break is never reused, a space goes before it.
pub fn completion_separator(text: &str, cursor: usize) -> Separator {
    let existing = text[cursor..]
        .chars()
        .next()
        .filter(|c| {
            c.is_whitespace()
                && !matches!(
                    *c,
                    '\n' | '\r' | '\u{000B}' | '\u{000C}' | '\u{0085}' | '\u{2028}' | '\u{2029}'
                )
        })
        .map(char::len_utf8);
    if let Some(len) = existing {
        let after = cursor + len;
        let precedes_suffix = text[after..]
            .chars()
            .next()
            .is_some_and(|c| !c.is_whitespace());
        if !precedes_suffix {
            return Separator::MoveCaretTo(after);
        }
    }
    Separator::InsertSpaceAt(cursor)
}

/// Whether a completion starting at `range` follows an atomic element
/// directly: a space then goes before it (`separate_completion_from_adjacent_element`).
pub fn starts_after_element(doc: &Doc, range: &Range<usize>) -> bool {
    doc.elements
        .iter()
        .any(|element| element.end == range.start)
}

fn at_token_range(doc: &Doc) -> Option<(Range<usize>, String)> {
    let (range, token) = current_prefixed_token_range(doc, '@', true)?;
    prefixed_token_range_is_editable(doc, '@', &range, &token).then_some((range, token))
}

fn at_target(doc: &Doc) -> Option<MentionTarget> {
    at_token_range(doc).map(|(range, query)| MentionTarget { range, query })
}

/// The `$` token at the caret: definite shell syntax is refused, an ambiguous
/// shell-like query only when a bindable mention matches it.
fn dollar_target(doc: &Doc, mentions: &[Mention]) -> Option<MentionTarget> {
    if mentions.is_empty() {
        return None;
    }
    let items = std::cell::OnceCell::new();
    let completable = |query: &str| match dollar_query_kind(query) {
        DollarQueryKind::Completable => true,
        DollarQueryKind::AmbiguousShellParameter => items
            .get_or_init(|| {
                mention_items(mentions)
                    .into_iter()
                    .filter(|item| {
                        item.path.is_some()
                            && mention_token_from_insert_text(&item.insert_text).is_some()
                    })
                    .collect::<Vec<_>>()
            })
            .iter()
            .any(|item| item.fuzzy_match_query(query).is_some()),
        DollarQueryKind::ShellVariable
        | DollarQueryKind::DefiniteShellParameter
        | DollarQueryKind::Invalid => false,
    };
    let (range, query) =
        current_prefixed_token_range_with_dollar_predicate(doc, '$', true, completable)?;
    if !prefixed_token_range_is_editable(doc, '$', &range, &query) || !completable(&query) {
        return None;
    }
    Some(MentionTarget { range, query })
}

#[cfg(test)]
#[allow(clippy::single_range_in_vec_init)] // a draft with one element
mod tests {
    use super::*;
    use crate::data::mention::MentionKind;
    use crate::data::mention::uri;
    use crate::ids::ThreadId;
    use pretty_assertions::assert_eq;

    fn doc<'a>(text: &'a str, cursor: usize, elements: &'a [Range<usize>]) -> Doc<'a> {
        Doc {
            text,
            cursor,
            elements,
        }
    }

    fn plugin(display: &str, config: &str) -> Mention {
        let mut m = Mention::new(MentionKind::Plugin, display, "", config);
        m.path = Some(uri("plugin", config));
        m
    }

    fn skill(name: &str, insert: &str, path: &str) -> Mention {
        let mut m = Mention::new(MentionKind::Skill, name, "", insert);
        m.path = Some(path.to_string());
        m
    }

    struct Fixture {
        mentions: Vec<Mention>,
        tasks: Vec<TaskMention>,
        usage: MentionUsage,
        matches: FileMatches,
    }

    impl Fixture {
        fn new() -> Self {
            Self {
                mentions: vec![
                    plugin("Sample Plugin", "sample@debug"),
                    skill("Fig", "fig", "/s/fig"),
                ],
                tasks: Vec::new(),
                usage: MentionUsage::default(),
                matches: FileMatches::default(),
            }
        }

        fn sources(&self) -> Sources<'_> {
            Sources {
                mentions: &self.mentions,
                tasks: &self.tasks,
                usage: &self.usage,
                file_matches: &self.matches,
            }
        }
    }

    #[test]
    fn a_new_at_popup_resets_the_search_before_it_asks_the_query() {
        let fx = Fixture::new();
        let mut session = MentionSession::default();
        let asks = session.sync(&doc("@fo", 3, &[]), &fx.sources());
        assert_eq!(
            asks,
            vec![
                FileSearchAsk::Query(String::new()),
                FileSearchAsk::Query("fo".into())
            ]
        );
        assert!(session.at_popup().is_some());
        // the same query again asks nothing
        assert!(session.sync(&doc("@fo", 3, &[]), &fx.sources()).is_empty());
        // another query asks only itself
        assert_eq!(
            session.sync(&doc("@foo", 4, &[]), &fx.sources()),
            vec![FileSearchAsk::Query("foo".into())]
        );
        // leaving the token ends the search
        assert_eq!(
            session.sync(&doc("@foo x", 6, &[]), &fx.sources()),
            vec![FileSearchAsk::Query(String::new())]
        );
        assert!(!session.is_open());
    }

    #[test]
    fn reopening_an_identical_mention_restarts_the_search() {
        let fx = Fixture::new();
        let mut session = MentionSession::default();
        let text = "@foo @foo";
        // the caret on the first token (Esc closes it), then at the end on the second,
        // the same text: a fresh popup, so the search is reset and asked again
        session.sync(&doc(text, 2, &[]), &fx.sources());
        session.dismiss(&doc(text, 2, &[]), &fx.sources());
        assert!(!session.is_open());
        let asks = session.sync(&doc(text, 9, &[]), &fx.sources());
        assert_eq!(
            asks,
            vec![
                FileSearchAsk::Query(String::new()),
                FileSearchAsk::Query("foo".into())
            ]
        );
    }

    #[test]
    fn a_dismissed_token_stays_closed_until_its_text_changes() {
        let fx = Fixture::new();
        let mut session = MentionSession::default();
        session.sync(&doc("@fo", 3, &[]), &fx.sources());
        session.dismiss(&doc("@fo", 3, &[]), &fx.sources());
        assert!(session.sync(&doc("@fo", 3, &[]), &fx.sources()).is_empty());
        assert!(!session.is_open());
        session.sync(&doc("@fo", 2, &[]), &fx.sources());
        assert!(!session.is_open());
        session.sync(&doc("@foo", 4, &[]), &fx.sources());
        assert!(session.is_open());
    }

    #[test]
    fn a_bound_mention_does_not_open_the_popup() {
        let fx = Fixture::new();
        let mut session = MentionSession::default();
        let elements = [0..7usize];
        session.sync(&doc("@sample", 7, &elements), &fx.sources());
        assert!(!session.is_open());
        session.sync(&doc("Please ask @sample.", 18, &[11..18]), &fx.sources());
        assert!(!session.is_open());
    }

    #[test]
    fn left_and_right_switch_the_tab_only_on_a_token() {
        let fx = Fixture::new();
        let mut session = MentionSession::default();
        session.sync(&doc("@fo", 3, &[]), &fx.sources());
        assert!(session.can_switch_search_mode(&doc("@fo", 3, &[])));
        assert!(!session.can_switch_search_mode(&doc("@fo x", 5, &[])));
        session.next_search_mode();
        assert_eq!(
            session.at_popup().map(Popup::search_mode),
            Some(super::super::search_mode::SearchMode::FilesystemOnly)
        );
    }

    #[test]
    fn dollar_lists_skills_and_a_pick_binds_the_skill_path() {
        let fx = Fixture::new();
        let mut session = MentionSession::default();
        let asks = session.sync(&doc("use $fi", 7, &[]), &fx.sources());
        assert!(asks.is_empty());
        let Active::Dollar(popup) = session.active() else {
            panic!("the $ popup is up")
        };
        assert_eq!(popup.rows().len(), 1);
        let pick = session
            .pick(0, &doc("use $fi", 7, &[]), &fx.sources())
            .unwrap();
        assert_eq!(pick.range, 4..7);
        assert_eq!(pick.usage.as_deref(), Some("/s/fig"));
        assert_eq!(
            pick.action,
            PickAction::Element {
                text: "$fig".into(),
                binding: Some(ComposerBinding {
                    sigil: '$',
                    mention: "fig".into(),
                    path: "/s/fig".into()
                })
            }
        );
    }

    #[test]
    fn shell_variables_do_not_open_the_dollar_popup() {
        let fx = Fixture::new();
        let mut session = MentionSession::default();
        session.sync(&doc("echo $HOME", 10, &[]), &fx.sources());
        assert!(!session.is_open());
    }

    #[test]
    fn a_task_pick_binds_a_thread_link_and_a_file_path_quotes_whitespace() {
        let mut fx = Fixture::new();
        fx.tasks = vec![TaskMention {
            thread_id: ThreadId::new("t1"),
            title: "migrate db".into(),
            cwd: String::new(),
            snippet: String::new(),
        }];
        let mut session = MentionSession::default();
        session.sync(&doc("@migr", 5, &[]), &fx.sources());
        let pick = session
            .pick(0, &doc("@migr", 5, &[]), &fx.sources())
            .unwrap();
        assert!(pick.is_task());
        assert_eq!(
            pick.action,
            PickAction::Element {
                text: "@migrate db".into(),
                binding: Some(ComposerBinding {
                    sigil: '@',
                    mention: "migrate db".into(),
                    path: "thread://t1".into()
                })
            }
        );
        assert_eq!(
            MentionPick::file(0..1, "my dir/a b.rs").action,
            PickAction::Path("\"my dir/a b.rs\"".into())
        );
        assert!(matches!(
            MentionPick::file(0..1, "shot.PNG").action,
            PickAction::Image { .. }
        ));
    }

    #[test]
    fn a_completed_token_is_dismissed_where_it_stands() {
        let fx = Fixture::new();
        let mut session = MentionSession::default();
        session.sync(&doc("@sa", 3, &[]), &fx.sources());
        let pick = session.pick(0, &doc("@sa", 3, &[]), &fx.sources()).unwrap();
        // the draft after the pick: the element `@Sample` and a space after it
        let text = "@Sample ";
        let elements = [0..7usize];
        session.finish_pick(&pick, 0..7, &doc(text, 8, &elements));
        assert!(!session.is_open());
        session.sync(&doc(text, 8, &elements), &fx.sources());
        assert!(!session.is_open());
    }

    #[test]
    fn the_separator_after_a_completion() {
        // one separator and nothing but whitespace after it: the caret steps over it
        assert_eq!(completion_separator("@a ", 2), Separator::MoveCaretTo(3));
        assert_eq!(completion_separator("@a  b", 2), Separator::MoveCaretTo(3));
        // none follows, or a word follows it: a space goes in so typing does not merge
        assert_eq!(completion_separator("@a", 2), Separator::InsertSpaceAt(2));
        assert_eq!(completion_separator("@a b", 2), Separator::InsertSpaceAt(2));
        // a line break is never reused
        assert_eq!(
            completion_separator("@a\nb", 2),
            Separator::InsertSpaceAt(2)
        );
    }

    #[test]
    fn dismissed_tokens_after_adjacent_elements_are_occurrence_scoped() {
        let first_bound = "$bound1";
        let second_bound = "$bound2";
        let token = "$other";
        let text = format!("{first_bound}{token} {second_bound}{token}");
        let first_start = first_bound.len();
        let second_start = text.rfind(token).expect("second token");
        let second_bound_start = text.find(second_bound).expect("second bound mention");
        let elements = [
            0..first_start,
            second_bound_start..second_bound_start + second_bound.len(),
        ];
        let d = doc(&text, 0, &elements);
        let dismissed = Dismissed::new(
            &d,
            first_start..first_start + token.len(),
            "other".to_string(),
        );
        assert!(!dismissed.matches(&d, &(second_start..second_start + token.len()), "other"));
    }

    #[test]
    fn nested_at_query_does_not_count_as_complete_dismissed_token() {
        let text = "@ma@latest @ma";
        let second_start = text.rfind("@ma").expect("second token");
        assert_eq!(
            complete_token_occurrences_before(&doc(text, 0, &[]), "@ma", second_start),
            0
        );
    }

    #[test]
    fn non_sigil_elements_delimit_dismissed_token_occurrences() {
        let token = "$other";
        let first_element = "[one]";
        let text = format!("{token}{first_element} {token}");
        let second_start = text.rfind(token).expect("second token");
        let elements = [token.len()..token.len() + first_element.len()];
        let d = doc(&text, 0, &elements);
        let dismissed = Dismissed::new(&d, 0..token.len(), "other".to_string());
        assert!(!dismissed.matches(&d, &(second_start..second_start + token.len()), "other"));
    }
}
