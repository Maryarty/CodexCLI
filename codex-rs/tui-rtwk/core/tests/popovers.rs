//! The composer's triggers, the `$`/`@` candidates and reverse history
//! search. History search ports `chat_composer/history_search.rs` behaviour
//! (substring, case-insensitive, newest first; the end of the matches keeps
//! the current one).

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::disallowed_methods)]

use codex_tui_core::screens::composer::history::{History, Lookup, Store};
use codex_tui_core::screens::composer::{Trigger, TriggerKind, trigger};
use codex_tui_core::screens::popovers::*;
use pretty_assertions::assert_eq;

fn t(kind: TriggerKind, start: usize, query: &str) -> Option<Trigger> {
    Some(Trigger {
        kind,
        start,
        query: query.to_string(),
    })
}

#[test]
fn slash_at_the_start_triggers_the_command_popup() {
    assert_eq!(trigger("/"), t(TriggerKind::Slash, 0, ""));
    assert_eq!(trigger("/mo"), t(TriggerKind::Slash, 0, "mo"));
}

#[test]
fn a_blank_after_the_command_name_closes_the_popup() {
    assert_eq!(trigger("/model "), None);
    assert_eq!(trigger("/model gpt"), None);
}

#[test]
fn a_slash_inside_the_text_is_not_a_command() {
    assert_eq!(trigger("hello /mo"), None);
}

#[test]
fn dollar_and_at_trigger_at_a_word_start() {
    assert_eq!(trigger("$sha"), t(TriggerKind::Skill, 0, "sha"));
    assert_eq!(trigger("see @src/a"), t(TriggerKind::Mention, 4, "src/a"));
    assert_eq!(trigger("a $"), t(TriggerKind::Skill, 2, ""));
}

#[test]
fn a_blank_after_the_mention_closes_the_popup() {
    assert_eq!(trigger("@parser "), None);
    assert_eq!(trigger("mail me@x"), None);
}

fn model() -> Model {
    let mut m = Model::default();
    m.files = vec![
        "src/shared/long_directory/parser_alpha.rs".into(),
        "src/shared/long_directory/parser_beta.rs".into(),
        "README.md".into(),
    ];
    m.mentions = vec![
        Mention::new(MentionKind::App, "Shared", "Secondary details", "shared"),
        Mention::new(MentionKind::Skill, "Shared", "Secondary details", "shared"),
        Mention::new(
            MentionKind::Plugin,
            "Sample Plugin",
            "Plugin with skills and an MCP server",
            "sample",
        ),
    ];
    m
}

fn model_with_history() -> Model {
    let mut m = model();
    *m.history = Store::seeded(vec![
        "cargo build".into(),
        "git status".into(),
        "cargo test".into(),
    ]);
    m
}

/// A search over the model's history with what the draft shows: the
/// preview a step put in, `None` for the draft as it was.
struct Walk {
    s: HistorySearch,
    history: History,
    store: Store,
    fx: Vec<Lookup>,
    shown: Option<String>,
}

impl Walk {
    fn new() -> Self {
        Walk {
            s: HistorySearch::new(),
            history: History::new(),
            store: (*model_with_history().history).clone(),
            fx: Vec::new(),
            shown: None,
        }
    }

    fn show(&mut self, preview: Preview) {
        match preview {
            Preview::Show(entry) => self.shown = Some(entry.text),
            Preview::Original => self.shown = None,
            Preview::Keep => {}
        }
    }

    fn set_query(&mut self, query: &str) {
        let preview = self
            .s
            .set_query(query.into(), &mut self.history, &self.store, &mut self.fx);
        self.show(preview);
    }

    fn step(&mut self, direction: Direction) {
        let preview = self
            .s
            .step(direction, &mut self.history, &self.store, &mut self.fx);
        self.show(preview);
    }

    fn preview(&self) -> Option<&str> {
        self.shown.as_deref()
    }
}

#[test]
fn dollar_lists_skills_and_apps_not_plugins_or_files() {
    let rows = model().candidates(Sigil::Dollar, "sha");
    let kinds: Vec<_> = rows.iter().map(|r| r.kind).collect();
    assert_eq!(kinds, vec![CandidateKind::App, CandidateKind::Skill]);
}

#[test]
fn at_lists_files_and_plugins() {
    let m = model();
    let rows = m.candidates(Sigil::At, "parser");
    assert_eq!(
        rows.iter().map(|r| r.label.as_str()).collect::<Vec<_>>(),
        vec![
            "src/shared/long_directory/parser_alpha.rs",
            "src/shared/long_directory/parser_beta.rs"
        ]
    );
    let rows = m.candidates(Sigil::At, "sa");
    assert_eq!(rows[0].kind, CandidateKind::Plugin);
}

#[test]
fn history_search_shows_the_newest_match_and_walks_older_and_newer() {
    let mut w = Walk::new();
    assert_eq!(w.s.status, SearchStatus::Idle);
    w.set_query("c");
    assert_eq!(w.preview(), Some("cargo test"));
    w.step(Direction::Older);
    assert_eq!(w.preview(), Some("cargo build"));
    w.step(Direction::Older);
    assert_eq!(w.preview(), Some("cargo build"), "the end stays");
    assert_eq!(w.s.status, SearchStatus::Match, "no flicker at the end");
    w.step(Direction::Newer);
    assert_eq!(w.preview(), Some("cargo test"));
}

#[test]
fn history_search_is_case_insensitive_and_reports_no_match() {
    let mut w = Walk::new();
    w.set_query("GIT");
    assert_eq!(w.preview(), Some("git status"));
    w.set_query("zzz");
    assert_eq!(w.s.status, SearchStatus::NoMatch);
    assert_eq!(w.preview(), None);
    w.set_query("");
    assert_eq!(w.s.status, SearchStatus::Idle);
}

// ------------------------------------------------- the search's own state: the composer holds it

#[test]
fn a_query_built_key_by_key_matches_as_one_typed_at_once() {
    let mut w = Walk::new();
    let mut query = String::new();
    for typed in ["g", "it"] {
        query.push_str(typed);
        w.set_query(&query);
    }
    assert_eq!(w.s.query, "git");
    assert_eq!(w.s.status, SearchStatus::Match);
    assert_eq!(w.preview(), Some("git status"));
}

#[test]
fn shortening_or_emptying_the_query_restarts_from_the_newest() {
    let mut w = Walk::new();
    w.set_query("gitx");
    assert_eq!(w.s.status, SearchStatus::NoMatch);
    w.set_query("git");
    assert_eq!(w.s.status, SearchStatus::Match);
    w.set_query("");
    assert_eq!(w.s.status, SearchStatus::Idle);
    assert_eq!(w.preview(), None);
}

/// `history_search.rs:213-230`: Enter accepts only a match, which is the
/// composer's decision on `status`: no preview otherwise.
#[test]
fn only_a_match_has_a_preview() {
    let mut w = Walk::new();
    assert_eq!((w.s.status, w.preview()), (SearchStatus::Idle, None));
    w.set_query("zzz");
    assert_eq!((w.s.status, w.preview()), (SearchStatus::NoMatch, None));
    w.set_query("cargo");
    assert_eq!(
        (w.s.status, w.preview()),
        (SearchStatus::Match, Some("cargo test"))
    );
}

/// `apply_history_search_result`: a read that is out shows `searching` and
/// keeps the preview; a failed read returns to the draft saying nothing.
#[test]
fn a_pending_read_keeps_the_preview_and_an_unavailable_one_restores_the_draft() {
    use codex_tui_core::screens::composer::history::SearchResult;
    let mut w = Walk::new();
    w.set_query("cargo");
    assert_eq!(w.preview(), Some("cargo test"));
    let preview = w.s.apply(SearchResult::Pending);
    w.show(preview);
    assert_eq!(
        (w.s.status, w.preview()),
        (SearchStatus::Searching, Some("cargo test"))
    );
    let preview = w.s.apply(SearchResult::Unavailable);
    w.show(preview);
    assert_eq!((w.s.status, w.preview()), (SearchStatus::Idle, None));
}
