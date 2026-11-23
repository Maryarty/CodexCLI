#![allow(clippy::expect_used, clippy::unwrap_used)]

//! `/review` (oracle `chatwidget/review_popups.rs`, `tests/review_picker_tests.rs`): the sheet the
//! model asks for, the host's branch list and commit log asked for under the menu's own request and
//! kept as a fact, what each row leads to (the pure `Menu::choose`), and the pick that starts
//! `review/start` inline on the active thread. The pages and what Esc does on them are the
//! sheet's own (`tests/pages/review.rs`).

use codex_tui_core::app::effects::{Ask, Done};
use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Intent, apply_app};
use codex_tui_core::ids::{ThreadId, Ticket};
use codex_tui_core::rpc::Reply;
use codex_tui_core::screens::select::review::{self, Chosen, Commit, Git, Listing, Menu as Review};
use codex_tui_core::screens::select::{Intent as Select, Opening};
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

fn open(app: &mut CodexApp) {
    select(
        app,
        Select::OpenReview {
            cwd: "/work/repo".into(),
        },
    );
    let shows: Vec<_> = app.screens().select.shows.pending().collect();
    assert_eq!(
        shows.last().map(|show| &show.opening),
        Some(&Opening::Review)
    );
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

fn branches() -> Done {
    Done::GitBranches {
        current: Some("feature".into()),
        branches: vec!["main".into(), "feature".into()],
    }
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
    assert!(matches!(
        Review::Presets.choose(review::UNCOMMITTED),
        Some(Chosen::Target(ReviewTarget::UncommittedChanges))
    ));
    let fx = select(&mut app, Select::Review(ReviewTarget::UncommittedChanges));
    assert_eq!(started(&fx), [ReviewTarget::UncommittedChanges]);
}

#[test]
fn the_presets_lead_to_a_list_of_the_host_a_prompt_or_a_target() {
    assert!(matches!(
        Review::Presets.choose(review::BASE),
        Some(Chosen::Ask(Git::Branches))
    ));
    assert!(matches!(
        Review::Presets.choose(review::COMMIT),
        Some(Chosen::Ask(Git::Log))
    ));
    assert!(matches!(
        Review::Presets.choose(review::CUSTOM),
        Some(Chosen::Prompt)
    ));
    assert!(Review::Presets.choose("nope").is_none());
    assert!(Review::Instructions.choose(review::CUSTOM).is_none());
}

#[test]
fn the_base_branch_picker_lists_the_hosts_branches_and_a_pick_starts_the_review() {
    let mut app = ready();
    open(&mut app);
    let fx = select(&mut app, Select::ReviewList(Git::Branches));
    let (request, ask) = asked(&fx);
    assert_eq!(
        ask,
        Ask::GitBranches {
            cwd: Some("/work/repo".into())
        }
    );
    assert!(request & review::REQUEST_BIT != 0);
    assert_eq!(
        app.screens().select.review.listing(),
        None,
        "the list is not there yet"
    );
    answer(&mut app, request, branches());
    let Some((answered, Listing::Branches { current, branches })) = app
        .screens()
        .select
        .review
        .listing()
        .map(|(r, l)| (r, l.clone()))
    else {
        panic!("expected the branches");
    };
    assert_eq!(answered, request, "the list carries its request");
    let page = Listing::Branches { current, branches }.menu();
    let rows = page.rows();
    assert_eq!(rows[1].name.as_deref(), Some("feature"));
    assert_eq!(rows[1].search.as_deref(), Some("feature"));
    let Some(Chosen::Target(target)) = page.choose(&rows[0].id) else {
        panic!("a branch row is a target");
    };
    let fx = select(&mut app, Select::Review(target));
    assert_eq!(
        started(&fx),
        [ReviewTarget::BaseBranch {
            branch: "main".into()
        }]
    );
}

#[test]
fn the_commit_picker_searches_subject_and_sha() {
    let mut app = ready();
    open(&mut app);
    let fx = select(&mut app, Select::ReviewList(Git::Log));
    let (request, ask) = asked(&fx);
    assert_eq!(
        ask,
        Ask::GitLog {
            cwd: Some("/work/repo".into()),
            limit: 100
        }
    );
    answer(&mut app, request, Done::GitLog(commits()));
    let Some((_, listing)) = app
        .screens()
        .select
        .review
        .listing()
        .map(|(r, l)| (r, l.clone()))
    else {
        panic!("expected the commits");
    };
    let page = listing.menu();
    let rows = page.rows();
    assert_eq!(rows[0].name.as_deref(), Some("Fix the parser"));
    assert_eq!(rows[0].search.as_deref(), Some("Fix the parser abc123"));
    let Some(Chosen::Target(target)) = page.choose("commit:def456") else {
        panic!("a commit row is a target");
    };
    let fx = select(&mut app, Select::Review(target));
    assert_eq!(
        started(&fx),
        [ReviewTarget::Commit {
            sha: "def456".into(),
            title: Some("Add review".into())
        }]
    );
}

#[test]
fn custom_instructions_are_a_review_target_of_their_own() {
    let mut app = ready();
    open(&mut app);
    let fx = select(
        &mut app,
        Select::Review(ReviewTarget::Custom {
            instructions: "the auth module".into(),
        }),
    );
    assert_eq!(
        started(&fx),
        [ReviewTarget::Custom {
            instructions: "the auth module".into()
        }]
    );
}

#[test]
fn a_newer_list_replaces_the_older_and_a_stale_answer_is_not_kept() {
    let mut app = ready();
    open(&mut app);
    let fx = select(&mut app, Select::ReviewList(Git::Branches));
    let (first, _) = asked(&fx);
    answer(&mut app, first, branches());
    let fx = select(&mut app, Select::ReviewList(Git::Log));
    let (second, _) = asked(&fx);
    // the first request was answered: a second answer to it is not the one waited for
    answer(&mut app, first, branches());
    assert_eq!(
        app.screens()
            .select
            .review
            .listing()
            .map(|(request, _)| request),
        Some(first)
    );
    answer(&mut app, second, Done::GitLog(commits()));
    assert_eq!(
        app.screens()
            .select
            .review
            .listing()
            .map(|(request, _)| request),
        Some(second)
    );
    // opening the sheet again forgets what the last one was told
    open(&mut app);
    assert_eq!(app.screens().select.review.listing(), None);
    // an answer of the UI's own request is kept for the UI
    answer(&mut app, 7, Done::GitLog(Vec::new()));
    assert_eq!(app.effect(7), Some(&Done::GitLog(Vec::new())));
}
