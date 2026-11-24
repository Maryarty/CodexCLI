#![allow(clippy::expect_used, clippy::unwrap_used)]

//! `/diff` as the pager's document (oracle `slash_dispatch.rs` `SlashCommand::Diff`,
//! `event_dispatch.rs` `DiffResult`): the verb asks the host for the diff of the directory the
//! session is in (`Ask::GitDiff`), the answer opens the pager on `D I F F` with the text as it came
//! (ANSI kept) or on the note that says there is nothing to show; an answer for an older ask, or
//! for a directory the session has left, is dropped. `/diff` is the feature slice `features::diff`
//! (`Target::Feature`); the pager is the surface's, asked for as a wish. The words of the notes
//! are the view's: these tests read data.

use codex_tui_core::app::effects::{Ask, Done, FEATURE_REQUEST_BIT};
use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Intent, apply_app};
use codex_tui_core::commands::{DispatchCx, Outcome, Target, dispatch};
use codex_tui_core::features::diff::{Intent as DiffIntent, PagerNote, Wish, document};
use codex_tui_core::features::{FeatureIntent, FeatureWish};
use codex_tui_core::ids::{ThreadId, Ticket};
use codex_tui_core::rpc::Reply;
use codex_tui_core::screens::ScreenEv;
use codex_tui_core::screens::chat::{self, StatusFacts};
use codex_tui_core::screens::transcript::diff::{Failure, Report};
use pretty_assertions::assert_eq;

const T: &str = "thread-1";
const COLOURED: &str = "\u{1b}[1mdiff --git a/x b/x\u{1b}[m\n+new\n";

fn run(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    app.assert_consistent();
    fx
}

fn ready() -> CodexApp {
    let mut app = CodexApp::new();
    let fx = run(&mut app, AppEv::Connected);
    let ticket: Ticket = fx
        .commands()
        .iter()
        .find_map(|c| match c {
            Command::StartThread { ticket, .. } => Some(*ticket),
            _ => None,
        })
        .expect("a StartThread");
    run(
        &mut app,
        AppEv::Reply(ticket, Reply::thread_started(ThreadId::new(T))),
    );
    app
}

fn in_directory(app: &mut CodexApp, cwd: &str) {
    run(
        app,
        AppEv::Screen(ScreenEv::Chat(chat::Ev::Status(Box::new(StatusFacts {
            cwd: Some(cwd.to_owned()),
            ..StatusFacts::default()
        })))),
    );
}

fn diff_verb(app: &mut CodexApp) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Feature(FeatureIntent::from(DiffIntent::Run))),
    )
}

fn diffs(fx: &AppFx) -> Vec<(u64, Option<String>)> {
    fx.commands()
        .iter()
        .filter_map(|c| match c {
            Command::Effect {
                request,
                ask: Ask::GitDiff { cwd },
                ..
            } => Some((*request, cwd.clone())),
            _ => None,
        })
        .collect()
}

fn answer(app: &mut CodexApp, request: u64, result: Result<Report, Failure>) {
    run(
        app,
        AppEv::Effect {
            request,
            done: Done::GitDiff(result),
        },
    );
}

fn wishes(app: &CodexApp) -> Vec<Wish> {
    app.wishes()
        .pending()
        .filter_map(|w| match &w.wish {
            FeatureWish::Diff(wish) => Some(wish.clone()),
            _ => None,
        })
        .collect()
}

fn changed() -> Report {
    Report {
        in_repo: true,
        text: COLOURED.to_owned(),
    }
}

#[test]
fn the_command_opens_the_feature_and_the_verb_asks_the_host_for_the_diff() {
    let outcome = dispatch("/diff", &DispatchCx::default());
    assert!(
        matches!(&outcome, Outcome::Open(open) if matches!(open.target, Target::Feature(_))),
        "{outcome:?}"
    );
    let mut app = ready();
    in_directory(&mut app, "/work/repo");
    let fx = diff_verb(&mut app);
    let asks = diffs(&fx);
    assert_eq!(asks.len(), 1, "{asks:?}");
    assert!(asks[0].0 & FEATURE_REQUEST_BIT != 0);
    assert_eq!(asks[0].1.as_deref(), Some("/work/repo"));
    // Nothing shows before the host answers.
    assert_eq!(wishes(&app), vec![]);
}

#[test]
fn the_answer_asks_for_the_pager_with_the_ansi_text_kept() {
    let mut app = ready();
    let (request, _) = diffs(&diff_verb(&mut app))[0].clone();
    answer(&mut app, request, Ok(changed()));
    assert_eq!(
        wishes(&app),
        vec![Wish::Pager {
            text: COLOURED.to_owned(),
            note: None,
        }]
    );
}

#[test]
fn an_empty_diff_a_non_repository_and_a_failure_are_notes_not_text() {
    for (result, note) in [
        (
            Ok(Report {
                in_repo: true,
                text: " \n".to_owned(),
            }),
            PagerNote::NoChanges,
        ),
        (
            Ok(Report {
                in_repo: false,
                text: String::new(),
            }),
            PagerNote::NotRepo,
        ),
        (
            Err(Failure::Shell("boom".to_owned())),
            PagerNote::Failed(Failure::Shell("boom".to_owned())),
        ),
    ] {
        let mut app = ready();
        let (request, _) = diffs(&diff_verb(&mut app))[0].clone();
        answer(&mut app, request, result);
        let shown = wishes(&app);
        let [Wish::Pager { text, note: got }] = shown.as_slice() else {
            panic!("one pager wish: {shown:?}");
        };
        assert_eq!(got, &Some(note.clone()), "{note:?}");
        assert_eq!(text, "");
    }
}

#[test]
fn only_the_newest_ask_is_answered() {
    let mut app = ready();
    let (first, _) = diffs(&diff_verb(&mut app))[0].clone();
    let (second, _) = diffs(&diff_verb(&mut app))[0].clone();
    assert!(second > first);
    answer(&mut app, first, Ok(changed()));
    assert_eq!(wishes(&app), vec![], "the old ask is dropped");
    answer(&mut app, second, Ok(changed()));
    assert_eq!(wishes(&app).len(), 1);
}

#[test]
fn an_answer_for_a_directory_the_session_left_is_dropped() {
    let mut app = ready();
    in_directory(&mut app, "/a");
    let (request, _) = diffs(&diff_verb(&mut app))[0].clone();
    in_directory(&mut app, "/b");
    answer(&mut app, request, Ok(changed()));
    assert_eq!(wishes(&app), vec![]);
    // Back where it was asked: the same ask, asked again, is shown.
    in_directory(&mut app, "/a");
    let (request, _) = diffs(&diff_verb(&mut app))[0].clone();
    answer(&mut app, request, Ok(changed()));
    assert_eq!(wishes(&app).len(), 1);
}

#[test]
fn the_document_is_the_text_or_the_note() {
    let (text, note) = document(Ok(changed()));
    assert_eq!((text.as_str(), note), (COLOURED, None));
    let (text, note) = document(Err(Failure::NoAnswer));
    assert_eq!(text, "");
    assert_eq!(note, Some(PagerNote::Failed(Failure::NoAnswer)));
}
