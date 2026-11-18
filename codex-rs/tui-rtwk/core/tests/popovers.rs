//! The composer's triggers, the `$`/`@` candidates and reverse history
//! search. History search ports `chat_composer/history_search.rs` behaviour
//! (substring, case-insensitive, newest first; the end of the matches keeps
//! the current one).

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::disallowed_methods)]

use codex_tui_rtwk_core::screens::composer::{Trigger, TriggerKind, trigger};
use codex_tui_rtwk_core::screens::popovers::*;
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
        search: None,
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
    let mut s = HistorySearch::new("keep me".into());
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
    let mut s = HistorySearch::new(String::new());
    s.set_query("GIT".into(), &m.history);
    assert_eq!(s.preview(&m.history), Some("git status"));
    s.set_query("zzz".into(), &m.history);
    assert_eq!(s.status, SearchStatus::NoMatch);
    assert_eq!(s.preview(&m.history), None);
    s.set_query(String::new(), &m.history);
    assert_eq!(s.status, SearchStatus::Idle);
}

// ------------------------------------------------- chat-nav-fix (G2): the keys' decisions

fn search_run(intents: Vec<Intent>) -> Model {
    let mut m = model();
    let mut fx = codex_tui_rtwk_core::app::AppFx::default();
    for intent in intents {
        apply_intent(&mut m, intent, &mut fx);
    }
    m
}

fn start(draft: &str) -> Intent {
    Intent::SearchStart {
        draft: draft.into(),
    }
}

#[test]
fn typing_extends_the_query_from_the_model_not_from_a_read() {
    // Two keys of one flush: the second builds on the first.
    let m = search_run(vec![
        start("keep me"),
        Intent::SearchAppend("g".into()),
        Intent::SearchAppend("it".into()),
    ]);
    let s = m.search.as_ref().unwrap();
    assert_eq!(s.query, "git");
    assert_eq!(s.status, SearchStatus::Match);
    assert_eq!(s.preview(&m.history), Some("git status"));
}

#[test]
fn backspace_and_ctrl_u_edit_the_query() {
    let m = search_run(vec![
        start(""),
        Intent::SearchAppend("gitx".into()),
        Intent::SearchBackspace,
    ]);
    let s = m.search.as_ref().unwrap();
    assert_eq!(s.query, "git");
    assert_eq!(s.status, SearchStatus::Match);
    let m = search_run(vec![
        start(""),
        Intent::SearchAppend("git".into()),
        Intent::SearchClear,
    ]);
    let s = m.search.as_ref().unwrap();
    assert_eq!(s.query, "");
    assert_eq!(s.status, SearchStatus::Idle);
    // Backspace on an empty query leaves the search open and idle.
    let m = search_run(vec![start(""), Intent::SearchBackspace]);
    assert_eq!(m.search.as_ref().unwrap().status, SearchStatus::Idle);
}

/// `history_search.rs:213-230`: Enter accepts only a match.
#[test]
fn history_search_no_match_restores_preview_but_keeps_search_open() {
    let m = search_run(vec![
        start("keep me"),
        Intent::SearchAppend("zzz".into()),
        Intent::SearchAccept,
    ]);
    let s = m.search.as_ref().expect("the search is still open");
    assert_eq!(s.status, SearchStatus::NoMatch);
    assert_eq!(s.preview(&m.history), None);
    assert_eq!(s.draft, "keep me");
}

#[test]
fn enter_on_an_idle_search_keeps_it_open_and_on_a_match_ends_it() {
    let m = search_run(vec![start(""), Intent::SearchAccept]);
    assert!(m.search.is_some());
    let m = search_run(vec![
        start(""),
        Intent::SearchAppend("git".into()),
        Intent::SearchAccept,
    ]);
    assert!(m.search.is_none());
}

#[test]
fn cancel_ends_the_search_whatever_its_status() {
    let m = search_run(vec![
        start("keep me"),
        Intent::SearchAppend("zzz".into()),
        Intent::SearchEnd,
    ]);
    assert!(m.search.is_none());
}
