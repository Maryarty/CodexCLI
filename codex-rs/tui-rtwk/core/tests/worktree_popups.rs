#![allow(clippy::expect_used, clippy::unwrap_used)]

//! The worktree pickers (oracle `chatwidget/tests/worktree_picker_tests.rs`): the checkout choice
//! of `/new` and `/fork`, `/worktree`, the browser with its loading row, its stale answers and its
//! actions, and the delete confirmation that protects the current checkout.

use codex_tui_core::app::effects::{Ask, Done};
use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Intent, apply_app};
use codex_tui_core::ids::ThreadId;
use codex_tui_core::screens::select::worktree::{self, Conversation, Entry, Owner, Thread};
use codex_tui_core::screens::select::{Intent as Select, Menu};
use codex_tui_core::screens::{ScreenEv, ScreenIntent};
use pretty_assertions::assert_eq;

fn run(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    app.assert_consistent();
    fx
}

fn select(app: &mut CodexApp, intent: Select) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Select(intent))),
    )
}

fn menu(app: &CodexApp) -> Option<worktree::Menu> {
    match app.screens().select.menu() {
        Some(Menu::Worktree(menu)) => Some((**menu).clone()),
        _ => None,
    }
}

fn entry(root: &str, owner: Owner) -> Entry {
    Entry {
        root: root.into(),
        cwd: format!("{root}/repo"),
        owner,
    }
}

fn thread(id: &str, title: &str) -> Thread {
    Thread {
        id: ThreadId::new(id),
        title: title.into(),
        updated_at: 1_000,
    }
}

fn open_worktrees(app: &mut CodexApp, cwd: &str) {
    select(
        app,
        Select::OpenWorktrees {
            available: true,
            cwd: cwd.into(),
            thread: None,
        },
    );
}

/// The one ask of a kind in `fx`, with its request.
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

fn answer(app: &mut CodexApp, request: u64, done: Done) -> AppFx {
    run(app, AppEv::Effect { request, done })
}

#[test]
fn the_checkout_choice_has_two_rows_and_without_a_repository_the_current_checkout_is_used_at_once()
{
    let mut app = CodexApp::new();
    select(
        &mut app,
        Select::OpenCheckout {
            mode: Conversation::Fork,
            name: Some("topic".into()),
            available: true,
            cwd: "/work/repo".into(),
            thread: None,
        },
    );
    let rows: Vec<_> = menu(&app)
        .unwrap()
        .rows()
        .into_iter()
        .map(|r| r.id)
        .collect();
    assert_eq!(rows, [worktree::CURRENT, worktree::CREATE]);
    select(&mut app, Select::Close);
    // no local repository: nothing is asked
    select(
        &mut app,
        Select::OpenCheckout {
            mode: Conversation::New,
            name: None,
            available: false,
            cwd: "/work/repo".into(),
            thread: None,
        },
    );
    assert_eq!(app.screens().select.menu(), None);
    // and `/worktree` says so
    select(
        &mut app,
        Select::OpenWorktrees {
            available: false,
            cwd: "/work/repo".into(),
            thread: None,
        },
    );
    assert_eq!(app.screens().select.menu(), None);
}

#[test]
fn the_browser_loads_lists_and_opens_a_worktrees_actions() {
    let mut app = CodexApp::new();
    open_worktrees(&mut app, "/pool/main/repo");
    assert_eq!(menu(&app), Some(worktree::Menu::Worktrees));
    let fx = select(&mut app, Select::Choose(worktree::BROWSE.into()));
    let (request, ask) = asked(&fx);
    assert_eq!(
        ask,
        Ask::Worktrees {
            cwd: "/pool/main/repo".into()
        }
    );
    assert_eq!(menu(&app), Some(worktree::Menu::Loading));
    let owned = entry("/pool/a", Owner::Resumable(thread("t-a", "Fix parser")));
    let archived = entry("/pool/b", Owner::Archived(thread("t-b", "Old work")));
    let free = entry("/pool/c", Owner::None);
    answer(
        &mut app,
        request,
        Done::Worktrees(Ok(vec![owned.clone(), archived, free.clone()])),
    );
    let Some(worktree::Menu::Browser(entries)) = menu(&app) else {
        panic!("expected the browser, got {:?}", menu(&app));
    };
    let rows = worktree::Menu::Browser(entries).rows();
    assert_eq!(rows[0].name.as_deref(), Some("Fix parser"));
    assert_eq!(rows[0].search.as_deref(), Some("Fix parser /pool/a/repo"));
    assert_eq!(
        rows[2].name.as_deref(),
        Some("/pool/c/repo"),
        "unowned: the directory"
    );
    // a resumable owner offers resume, copy, delete
    select(&mut app, Select::Choose(rows[0].id.clone()));
    let actions = menu(&app).unwrap();
    assert_eq!(
        actions,
        worktree::Menu::Actions {
            entry: owned,
            can_delete: true
        }
    );
    let ids: Vec<_> = actions.rows().into_iter().map(|r| r.id).collect();
    assert_eq!(ids, [worktree::RESUME, worktree::COPY, worktree::DELETE]);
    // an unowned one has no resume row
    select(&mut app, Select::Close);
    open_worktrees(&mut app, "/pool/main/repo");
    let fx = select(&mut app, Select::Choose(worktree::BROWSE.into()));
    let (request, _) = asked(&fx);
    answer(&mut app, request, Done::Worktrees(Ok(vec![free])));
    select(&mut app, Select::Choose("worktree:0".into()));
    let ids: Vec<_> = menu(&app)
        .unwrap()
        .rows()
        .into_iter()
        .map(|r| r.id)
        .collect();
    assert_eq!(ids, [worktree::COPY, worktree::DELETE]);
}

#[test]
fn a_stale_list_and_a_failed_list_open_no_browser() {
    let mut app = CodexApp::new();
    open_worktrees(&mut app, "/pool/main/repo");
    let fx = select(&mut app, Select::Choose(worktree::BROWSE.into()));
    let (request, _) = asked(&fx);
    // the user closed the loading view: the answer is dropped
    select(&mut app, Select::Close);
    answer(&mut app, request, Done::Worktrees(Ok(Vec::new())));
    assert_eq!(app.screens().select.menu(), None);
    // another request replaces the first: the old answer is dropped
    open_worktrees(&mut app, "/pool/main/repo");
    let fx = select(&mut app, Select::Choose(worktree::BROWSE.into()));
    let (second, _) = asked(&fx);
    assert!(second != request);
    answer(&mut app, request, Done::Worktrees(Ok(Vec::new())));
    assert_eq!(menu(&app), Some(worktree::Menu::Loading));
    // a failure closes the loading row
    answer(&mut app, second, Done::Worktrees(Err("no git".into())));
    assert_eq!(app.screens().select.menu(), None);
}

#[test]
fn the_current_checkout_cannot_be_deleted_and_a_delete_is_confirmed_then_asked() {
    let mut app = CodexApp::new();
    open_worktrees(&mut app, "/pool/a/repo");
    let fx = select(&mut app, Select::Choose(worktree::BROWSE.into()));
    let (request, _) = asked(&fx);
    let here = entry("/pool/a", Owner::None);
    let other = entry("/pool/b", Owner::None);
    answer(
        &mut app,
        request,
        Done::Worktrees(Ok(vec![here, other.clone()])),
    );
    // the session runs in /pool/a: its delete row is disabled and does nothing
    select(&mut app, Select::Choose("worktree:0".into()));
    let rows = menu(&app).unwrap().rows();
    assert!(
        rows.iter()
            .find(|r| r.id == worktree::DELETE)
            .unwrap()
            .disabled
    );
    let fx = select(&mut app, Select::Choose(worktree::DELETE.into()));
    assert!(fx.commands().is_empty());
    assert!(matches!(menu(&app), Some(worktree::Menu::Actions { .. })));
    select(&mut app, Select::Close);
    // another one: the confirmation, Cancel first
    open_worktrees(&mut app, "/pool/a/repo");
    let fx = select(&mut app, Select::Choose(worktree::BROWSE.into()));
    let (request, _) = asked(&fx);
    answer(&mut app, request, Done::Worktrees(Ok(vec![other])));
    select(&mut app, Select::Choose("worktree:0".into()));
    select(&mut app, Select::Choose(worktree::DELETE.into()));
    let confirm = menu(&app).unwrap();
    assert_eq!(
        confirm,
        worktree::Menu::Confirm {
            root: "/pool/b".into()
        }
    );
    let ids: Vec<_> = confirm.rows().into_iter().map(|r| r.id).collect();
    assert_eq!(ids, [worktree::CANCEL, worktree::DELETE]);
    let fx = select(&mut app, Select::Choose(worktree::CANCEL.into()));
    assert!(fx.commands().is_empty());
    assert_eq!(app.screens().select.menu(), None);
}

#[test]
fn a_removal_is_asked_in_the_sessions_directory_and_answered_with_a_note() {
    let mut app = CodexApp::new();
    open_worktrees(&mut app, "/pool/a/repo");
    let fx = select(&mut app, Select::Choose(worktree::BROWSE.into()));
    let (request, _) = asked(&fx);
    answer(
        &mut app,
        request,
        Done::Worktrees(Ok(vec![entry("/pool/b", Owner::None)])),
    );
    select(&mut app, Select::Choose("worktree:0".into()));
    select(&mut app, Select::Choose(worktree::DELETE.into()));
    let fx = select(&mut app, Select::Choose(worktree::DELETE.into()));
    let (removal, ask) = asked(&fx);
    assert_eq!(
        ask,
        Ask::WorktreeRemove {
            cwd: "/pool/a/repo".into(),
            root: "/pool/b".into()
        }
    );
    assert_eq!(app.screens().select.menu(), None);
    assert!(app.screens().select.worktree.waits_for(removal));
    answer(&mut app, removal, Done::WorktreeRemoved(Ok(())));
    assert!(
        !app.screens().select.worktree.waits_for(removal),
        "answered once"
    );
}

#[test]
fn a_scope_change_dismisses_the_worktree_menus() {
    let mut app = CodexApp::new();
    open_worktrees(&mut app, "/pool/a/repo");
    run(
        &mut app,
        AppEv::Screen(ScreenEv::Select(
            codex_tui_core::screens::select::Ev::ScopeChanged,
        )),
    );
    assert_eq!(app.screens().select.menu(), None);
}
