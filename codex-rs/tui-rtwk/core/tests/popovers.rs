//! The composer's triggers, the `$`/`@` candidates and reverse history
//! search. History search ports `chat_composer/history_search.rs` behaviour
//! (substring, case-insensitive, newest first; the end of the matches keeps
//! the current one).

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::disallowed_methods)]

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
    Model {
        files: vec![
            "src/shared/long_directory/parser_alpha.rs".into(),
            "src/shared/long_directory/parser_beta.rs".into(),
            "README.md".into(),
        ],
        mentions: vec![
            Mention {
                kind: MentionKind::App,
                name: "Shared".into(),
                description: "Secondary details".into(),
                insert: "shared".into(),
            },
            Mention {
                kind: MentionKind::Skill,
                name: "Shared".into(),
                description: "Secondary details".into(),
                insert: "shared".into(),
            },
            Mention {
                kind: MentionKind::Plugin,
                name: "Sample Plugin".into(),
                description: "Plugin with skills and an MCP server".into(),
                insert: "sample".into(),
            },
        ],
        history: vec![
            "cargo build".into(),
            "git status".into(),
            "cargo test".into(),
        ],
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
    let m = model();
    let mut s = HistorySearch::new();
    assert_eq!(s.status, SearchStatus::Idle);
    s.set_query("c".into(), &m.history);
    assert_eq!(s.preview(&m.history), Some("cargo test"));
    s.step(Direction::Older, &m.history);
    assert_eq!(s.preview(&m.history), Some("cargo build"));
    s.step(Direction::Older, &m.history);
    assert_eq!(s.preview(&m.history), Some("cargo build"), "the end stays");
    s.step(Direction::Newer, &m.history);
    assert_eq!(s.preview(&m.history), Some("cargo test"));
}

#[test]
fn history_search_is_case_insensitive_and_reports_no_match() {
    let m = model();
    let mut s = HistorySearch::new();
    s.set_query("GIT".into(), &m.history);
    assert_eq!(s.preview(&m.history), Some("git status"));
    s.set_query("zzz".into(), &m.history);
    assert_eq!(s.status, SearchStatus::NoMatch);
    assert_eq!(s.preview(&m.history), None);
    s.set_query(String::new(), &m.history);
    assert_eq!(s.status, SearchStatus::Idle);
}

// ------------------------------------------------- the search's own state: the composer holds it

#[test]
fn a_query_built_key_by_key_matches_as_one_typed_at_once() {
    let m = model();
    let mut s = HistorySearch::new();
    let mut query = String::new();
    for typed in ["g", "it"] {
        query.push_str(typed);
        s.set_query(query.clone(), &m.history);
    }
    assert_eq!(s.query, "git");
    assert_eq!(s.status, SearchStatus::Match);
    assert_eq!(s.preview(&m.history), Some("git status"));
}

#[test]
fn shortening_or_emptying_the_query_restarts_from_the_newest() {
    let m = model();
    let mut s = HistorySearch::new();
    s.set_query("gitx".into(), &m.history);
    assert_eq!(s.status, SearchStatus::NoMatch);
    s.set_query("git".into(), &m.history);
    assert_eq!(s.status, SearchStatus::Match);
    s.set_query(String::new(), &m.history);
    assert_eq!(s.status, SearchStatus::Idle);
    assert_eq!(s.index(), None);
}

/// `history_search.rs:213-230`: Enter accepts only a match, which is the
/// composer's decision on `status`: no preview and no index otherwise.
#[test]
fn only_a_match_has_a_preview_and_an_index() {
    let m = model();
    let mut s = HistorySearch::new();
    assert_eq!((s.status, s.index()), (SearchStatus::Idle, None));
    s.set_query("zzz".into(), &m.history);
    assert_eq!((s.status, s.index()), (SearchStatus::NoMatch, None));
    s.set_query("cargo".into(), &m.history);
    assert_eq!((s.status, s.index()), (SearchStatus::Match, Some(2)));
}

/// `display_query`: a newline and a tab are shown as marks; the query that
/// is matched is the original.
#[test]
fn the_displayed_query_marks_newlines_and_tabs() {
    let m = model();
    let mut s = HistorySearch::new();
    s.set_query("foo\nbar\tbaz".into(), &m.history);
    assert_eq!(s.display_query(), "foo\u{21b5}bar\u{21e5}baz");
    assert_eq!(s.query, "foo\nbar\tbaz");
}
