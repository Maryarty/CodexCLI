#![allow(clippy::expect_used, clippy::unwrap_used)]

//! `/review` (oracle `chatwidget/review_popups.rs`, `tests/review_picker_tests.rs`): the presets,
//! the host's branch list and commit log asked for under the menu's own request, the child menu
//! pushed over the presets (Esc shows them again on the row that opened it), the instructions
//! prompt, and the pick that starts `review/start` inline on the active thread.

use codex_tui_core::app::effects::{Ask, Done};
use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Intent, apply_app};
use codex_tui_core::ids::{ThreadId, Ticket};
use codex_tui_core::rpc::Reply;
use codex_tui_core::screens::select::review::{self, Commit, Menu as Review};
use codex_tui_core::screens::select::{Intent as Select, Menu};
use codex_tui_core::screens::{ScreenCommand, ScreenIntent, chat};
use codex_tui_core::threads::ReviewTarget;
use pretty_assertions::assert_eq;

const T: &str = "thread-1";

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

fn select(app: &mut CodexApp, intent: Select) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Select(intent))),
    )
}

fn menu(app: &CodexApp) -> Option<Review> {
    match app.screens().select.menu() {
        Some(Menu::Review(menu)) => Some((**menu).clone()),
        _ => None,
    }
}

fn open(app: &mut CodexApp) {
    select(
        app,
        Select::OpenReview {
            cwd: "/work/repo".into(),
        },
    );
    assert_eq!(menu(app), Some(Review::Presets));
}

/// The git list the menu asked the host for, with its request.
fn asked(fx: &AppFx) -> (u64, Ask) {
    let asks: Vec<_> = fx
        .commands()
        .iter()
        .filter_map(|c| match c {
            Command::Effect { request, ask, .. } => Some((*request, ask.clone())),
            _ => None,
        })
        .collect();
    let [one] = asks.as_slice() else {
        panic!("expected one ask, got {:?}", fx.commands());
    };
    one.clone()
}

fn started(fx: &AppFx) -> Vec<ReviewTarget> {
    fx.commands()
        .iter()
        .filter_map(|c| match c {
            Command::Screen {
                thread,
                command: ScreenCommand::Chat(chat::Command::Review(target)),
                ..
            } if thread.as_str() == T => Some(target.clone()),
            _ => None,
        })
        .collect()
}

fn answer(app: &mut CodexApp, request: u64, done: Done) {
    run(app, AppEv::Effect { request, done });
}

fn commits() -> Vec<Commit> {
    vec![
        Commit {
            sha: "abc123".into(),
            subject: "Fix the parser".into(),
        },
        Commit {
            sha: "def456".into(),
            subject: "Add review".into(),
        },
    ]
}

#[test]
fn the_presets_are_four_rows_and_uncommitted_starts_the_review_at_once() {
    let mut app = ready();
    open(&mut app);
    let rows: Vec<_> = Review::Presets.rows().into_iter().map(|r| r.id).collect();
    assert_eq!(
        rows,
        [
            review::BASE,
            review::UNCOMMITTED,
            review::COMMIT,
            review::CUSTOM
        ]
    );
    let fx = select(&mut app, Select::Choose(review::UNCOMMITTED.into()));
    assert_eq!(started(&fx), [ReviewTarget::UncommittedChanges]);
    assert_eq!(
        app.screens().select.menu(),
        None,
        "the presets are dismissed"
    );
}

#[test]
fn the_base_branch_picker_lists_the_hosts_branches_and_a_pick_dismisses_both() {
    let mut app = ready();
    open(&mut app);
    let fx = select(&mut app, Select::Choose(review::BASE.into()));
    let (request, ask) = asked(&fx);
    assert_eq!(
        ask,
        Ask::GitBranches {
            cwd: Some("/work/repo".into())
        }
    );
    assert!(request & review::REQUEST_BIT != 0);
    assert_eq!(
        menu(&app),
        Some(Review::Presets),
        "the list is not there yet"
    );
    answer(
        &mut app,
        request,
        Done::GitBranches {
            current: Some("feature".into()),
            branches: vec!["main".into(), "feature".into()],
        },
    );
    let Some(branches @ Review::Branches { .. }) = menu(&app) else {
        panic!("expected the branches, got {:?}", menu(&app));
    };
    select(&mut app, Select::Close);
    assert_eq!(menu(&app), Some(Review::Presets));
    let fx = select(&mut app, Select::Choose(review::BASE.into()));
    let (request, _) = asked(&fx);
    answer(
        &mut app,
        request,
        Done::GitBranches {
            current: Some("feature".into()),
            branches: vec!["main".into(), "feature".into()],
        },
    );
    let rows = branches.rows();
    assert_eq!(rows[1].name.as_deref(), Some("feature"));
    assert_eq!(rows[1].search.as_deref(), Some("feature"));
    let fx = select(&mut app, Select::Choose(rows[0].id.clone()));
    assert_eq!(
        started(&fx),
        [ReviewTarget::BaseBranch {
            branch: "main".into()
        }]
    );
    assert_eq!(app.screens().select.menu(), None);
}

#[test]
fn the_commit_picker_searches_subject_and_sha_and_esc_returns_to_the_presets() {
    let mut app = ready();
    open(&mut app);
    let fx = select(&mut app, Select::Choose(review::COMMIT.into()));
    let (request, ask) = asked(&fx);
    assert_eq!(
        ask,
        Ask::GitLog {
            cwd: Some("/work/repo".into()),
            limit: 100
        }
    );
    answer(&mut app, request, Done::GitLog(commits()));
    let Some(Review::Commits(listed)) = menu(&app) else {
        panic!("expected the commits");
    };
    let rows = Review::Commits(listed).rows();
    assert_eq!(rows[0].name.as_deref(), Some("Fix the parser"));
    assert_eq!(rows[0].search.as_deref(), Some("Fix the parser abc123"));
    select(&mut app, Select::Close);
    assert_eq!(menu(&app), Some(Review::Presets));
    // again, and pick the second commit
    let fx = select(&mut app, Select::Choose(review::COMMIT.into()));
    let (request, _) = asked(&fx);
    answer(&mut app, request, Done::GitLog(commits()));
    let fx = select(&mut app, Select::Choose("commit:def456".into()));
    assert_eq!(
        started(&fx),
        [ReviewTarget::Commit {
            sha: "def456".into(),
            title: Some("Add review".into())
        }]
    );
    assert_eq!(app.screens().select.menu(), None);
}

#[test]
fn custom_instructions_are_a_prompt_over_the_presets() {
    let mut app = ready();
    open(&mut app);
    select(&mut app, Select::Choose(review::CUSTOM.into()));
    assert_eq!(menu(&app), Some(Review::Instructions));
    // an empty answer starts nothing, the prompt stays
    let fx = select(&mut app, Select::Instructions("   ".into()));
    assert_eq!(started(&fx), []);
    assert_eq!(menu(&app), Some(Review::Instructions));
    // Esc shows the presets
    select(&mut app, Select::Close);
    assert_eq!(menu(&app), Some(Review::Presets));
    select(&mut app, Select::Choose(review::CUSTOM.into()));
    let fx = select(&mut app, Select::Instructions("  the auth module ".into()));
    assert_eq!(
        started(&fx),
        [ReviewTarget::Custom {
            instructions: "the auth module".into()
        }]
    );
    assert_eq!(app.screens().select.menu(), None);
}

#[test]
fn a_list_that_arrives_after_the_menu_was_closed_opens_nothing() {
    let mut app = ready();
    open(&mut app);
    let fx = select(&mut app, Select::Choose(review::BASE.into()));
    let (request, _) = asked(&fx);
    select(&mut app, Select::Close);
    answer(
        &mut app,
        request,
        Done::GitBranches {
            current: None,
            branches: vec!["main".into()],
        },
    );
    assert_eq!(app.screens().select.menu(), None);
    // an answer of the UI's own request is kept for the UI
    answer(&mut app, 7, Done::GitLog(Vec::new()));
    assert_eq!(app.effect(7), Some(&Done::GitLog(Vec::new())));
}
