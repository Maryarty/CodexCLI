#![allow(clippy::expect_used, clippy::unwrap_used)]

//! The worktree pickers (oracle `chatwidget/tests/worktree_picker_tests.rs`): what the model asks
//! the sheet to show (`/new`, `/fork`, `/worktree`), the verbs the sheet says back, the host's
//! list and removal asked under the model's own requests and kept as facts the pages read, what
//! each row leads to (the pure `Menu::choose`) and the protection of the current checkout. The
//! pages, their Esc and the mouse are the sheet's own (`tests/pages/worktree.rs`).

use codex_tui_core::app::effects::{Ask, Done};
use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Intent, apply_app};
use codex_tui_core::commands::{CommandFlags, SessionIntent, SlashCommand};
use codex_tui_core::ids::ThreadId;
use codex_tui_core::screens::select::worktree::{
    self, Chosen, Conversation, Entry, Owner, Pick, Thread, Verb,
};
use codex_tui_core::screens::select::{Ev, Intent as Select, Note, Opening};
use codex_tui_core::screens::{ScreenEv, ScreenIntent, chat};
use codex_tui_core::threads::{NoteKey, Tone};
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

/// What a page of the sheet says to the model.
fn say(app: &mut CodexApp, verb: Verb) -> AppFx {
    select(app, Select::Worktree(verb))
}

fn shows(app: &CodexApp) -> Vec<Opening> {
    app.screens()
        .select
        .shows
        .pending()
        .map(|show| show.opening.clone())
        .collect()
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

/// The one ask in `fx`, with its request.
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

/// The keys of the notes the screens hold for the history (no thread is active in these tests).
fn notes(app: &CodexApp) -> Vec<(NoteKey, Tone)> {
    app.screens()
        .chat
        .notes
        .iter()
        .map(|note| (note.key.clone(), note.tone))
        .collect()
}

fn note(note: Note) -> (NoteKey, Tone) {
    let tone = note.tone();
    (NoteKey::Select(note), tone)
}

fn ids(menu: &worktree::Menu) -> Vec<String> {
    menu.rows().into_iter().map(|row| row.id).collect()
}

#[test]
fn the_checkout_choice_is_a_show_of_two_rows_and_without_a_repository_it_is_not_asked() {
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
    let menu = worktree::Menu::Checkout {
        mode: Conversation::Fork,
        name: Some("topic".into()),
    };
    assert_eq!(shows(&app), [Opening::Worktree(menu.clone())]);
    assert_eq!(ids(&menu), [worktree::CURRENT, worktree::CREATE]);
    // no local repository: the current checkout is used at once and nothing is shown
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
    assert_eq!(shows(&app).len(), 1);
}

#[test]
fn the_rows_of_the_checkout_choice_and_of_worktrees_lead_to_picks_and_to_the_browser() {
    let named = worktree::Menu::Checkout {
        mode: Conversation::New,
        name: Some("topic".into()),
    };
    assert_eq!(
        named.choose(worktree::CURRENT),
        Some(Chosen::Pick(Pick::Current {
            mode: Conversation::New,
            name: Some("topic".into())
        }))
    );
    assert_eq!(
        named.choose(worktree::CREATE),
        Some(Chosen::Pick(Pick::Start {
            mode: Conversation::New,
            name: Some("topic".into())
        }))
    );
    let menu = worktree::Menu::Worktrees;
    assert_eq!(
        ids(&menu),
        [worktree::CONTINUE, worktree::START, worktree::BROWSE]
    );
    // continuing keeps the conversation: a fork; starting is a new one
    assert_eq!(
        menu.choose(worktree::CONTINUE),
        Some(Chosen::Pick(Pick::Start {
            mode: Conversation::Fork,
            name: None
        }))
    );
    assert_eq!(
        menu.choose(worktree::START),
        Some(Chosen::Pick(Pick::Start {
            mode: Conversation::New,
            name: None
        }))
    );
    assert_eq!(menu.choose(worktree::BROWSE), Some(Chosen::Browse));
    assert_eq!(
        menu.choose(worktree::DELETE),
        None,
        "not a row of this page"
    );
    // the loading row is disabled, a pick of it does nothing
    let loading = worktree::Menu::Loading;
    assert!(loading.rows()[0].disabled);
    assert_eq!(loading.choose("loading"), None);
}

#[test]
fn the_list_is_asked_under_the_models_request_and_kept_as_a_fact() {
    let mut app = CodexApp::new();
    open_worktrees(&mut app, "/pool/main/repo");
    assert_eq!(shows(&app), [Opening::Worktree(worktree::Menu::Worktrees)]);
    assert_eq!(app.screens().select.worktree.listing(), None);
    let fx = say(&mut app, Verb::List);
    let (request, ask) = asked(&fx);
    assert_eq!(
        ask,
        Ask::Worktrees {
            cwd: "/pool/main/repo".into()
        }
    );
    assert!(app.screens().select.worktree.waits_for(request));
    assert_eq!(
        app.screens().select.worktree.listing(),
        None,
        "nothing until the host answers"
    );
    let owned = entry("/pool/a", Owner::Resumable(thread("t-a", "Fix parser")));
    let archived = entry("/pool/b", Owner::Archived(thread("t-b", "Old work")));
    let free = entry("/pool/c", Owner::None);
    let list = vec![owned.clone(), archived, free.clone()];
    answer(&mut app, request, Done::Worktrees(Ok(list.clone())));
    assert_eq!(
        app.screens().select.worktree.listing(),
        Some(&Ok(list.clone()))
    );
    assert!(
        !app.screens().select.worktree.waits_for(request),
        "answered once"
    );
    assert_eq!(notes(&app), []);
    // the browser's rows: the owner's title, else the directory; a search matches both
    let browser = worktree::Menu::Browser(list);
    let rows = browser.rows();
    assert_eq!(rows[0].name.as_deref(), Some("Fix parser"));
    assert_eq!(rows[0].search.as_deref(), Some("Fix parser /pool/a/repo"));
    assert_eq!(
        rows[2].name.as_deref(),
        Some("/pool/c/repo"),
        "unowned: the directory"
    );
    // a pick of a row opens that worktree's actions; a resumable owner offers resume first
    assert_eq!(
        browser.choose(&rows[0].id),
        Some(Chosen::Actions(owned.clone()))
    );
    let actions = worktree::Menu::Actions {
        entry: owned,
        can_delete: true,
    };
    assert_eq!(
        ids(&actions),
        [worktree::RESUME, worktree::COPY, worktree::DELETE]
    );
    // an unowned one has no resume row
    let actions = worktree::Menu::Actions {
        entry: free,
        can_delete: true,
    };
    assert_eq!(ids(&actions), [worktree::COPY, worktree::DELETE]);
    assert_eq!(browser.choose("worktree:9"), None, "past the end");
}

#[test]
fn an_abandoned_list_and_a_stale_one_are_dropped_and_a_failure_is_said() {
    let mut app = CodexApp::new();
    open_worktrees(&mut app, "/pool/main/repo");
    let (request, _) = asked(&say(&mut app, Verb::List));
    // the loading page was left: the answer is dropped, and so is a failure
    say(&mut app, Verb::Abandon);
    assert!(!app.screens().select.worktree.waits_for(request));
    answer(&mut app, request, Done::Worktrees(Ok(Vec::new())));
    assert_eq!(app.screens().select.worktree.listing(), None);
    answer(&mut app, request, Done::Worktrees(Err("no git".into())));
    assert_eq!(notes(&app), []);
    // another request replaces the first: the old answer is dropped
    let (second, _) = asked(&say(&mut app, Verb::List));
    assert!(second != request);
    answer(&mut app, request, Done::Worktrees(Ok(Vec::new())));
    assert_eq!(app.screens().select.worktree.listing(), None);
    assert!(app.screens().select.worktree.waits_for(second));
    // a failure of the one asked for is the fact the page reads and a line of the history
    answer(&mut app, second, Done::Worktrees(Err("no git".into())));
    assert_eq!(
        app.screens().select.worktree.listing(),
        Some(&Err("no git".into()))
    );
    assert_eq!(
        notes(&app),
        [note(Note::WorktreeListFailed("no git".into()))]
    );
    assert_eq!(notes(&app)[0].1, Tone::Error);
}

#[test]
fn a_scope_change_gives_up_the_list_and_moves_the_epoch_the_sheets_close_on() {
    let mut app = CodexApp::new();
    open_worktrees(&mut app, "/pool/a/repo");
    let epoch = app.screens().select.worktree.epoch();
    let (request, _) = asked(&say(&mut app, Verb::List));
    run(&mut app, AppEv::Screen(ScreenEv::Select(Ev::ScopeChanged)));
    assert!(app.screens().select.worktree.epoch() > epoch);
    assert!(!app.screens().select.worktree.waits_for(request));
    answer(&mut app, request, Done::Worktrees(Ok(Vec::new())));
    assert_eq!(app.screens().select.worktree.listing(), None);
    // opening again is a scope of its own
    let epoch = app.screens().select.worktree.epoch();
    open_worktrees(&mut app, "/pool/a/repo");
    assert!(app.screens().select.worktree.epoch() > epoch);
}

#[test]
fn the_current_checkout_cannot_be_deleted_and_a_delete_is_confirmed() {
    let here = entry("/pool/a", Owner::None);
    let other = entry("/pool/b", Owner::None);
    assert!(
        !here.can_delete("/pool/a/repo"),
        "the session runs inside it"
    );
    assert!(!here.can_delete("/pool/a"));
    assert!(other.can_delete("/pool/a/repo"));
    assert!(
        entry("/pool/ab", Owner::None).can_delete("/pool/a/repo"),
        "a path prefix is not a parent directory"
    );
    // the delete row of the current checkout is disabled and does nothing
    let blocked = worktree::Menu::Actions {
        entry: here,
        can_delete: false,
    };
    let rows = blocked.rows();
    assert!(
        rows.iter()
            .find(|r| r.id == worktree::DELETE)
            .unwrap()
            .disabled
    );
    assert_eq!(blocked.choose(worktree::DELETE), None);
    // another one asks first: Cancel, then Delete
    let open = worktree::Menu::Actions {
        entry: other,
        can_delete: true,
    };
    assert_eq!(
        open.choose(worktree::DELETE),
        Some(Chosen::Confirm {
            root: "/pool/b".into()
        })
    );
    let confirm = worktree::Menu::Confirm {
        root: "/pool/b".into(),
    };
    assert_eq!(ids(&confirm), [worktree::CANCEL, worktree::DELETE]);
    assert_eq!(confirm.choose(worktree::CANCEL), Some(Chosen::Close));
    assert_eq!(
        confirm.choose(worktree::DELETE),
        Some(Chosen::Pick(Pick::Remove {
            root: "/pool/b".into()
        }))
    );
    // even said by a page that should not have, the model does not remove the checkout the
    // session runs in
    let mut app = CodexApp::new();
    open_worktrees(&mut app, "/pool/a/repo");
    let fx = say(
        &mut app,
        Verb::Pick(Pick::Remove {
            root: "/pool/a".into(),
        }),
    );
    assert!(fx.commands().is_empty(), "{:?}", fx.commands());
}

#[test]
fn a_removal_is_asked_in_the_sessions_directory_and_answered_with_a_note() {
    let mut app = CodexApp::new();
    open_worktrees(&mut app, "/pool/a/repo");
    let fx = say(
        &mut app,
        Verb::Pick(Pick::Remove {
            root: "/pool/b".into(),
        }),
    );
    let (removal, ask) = asked(&fx);
    assert_eq!(
        ask,
        Ask::WorktreeRemove {
            cwd: "/pool/a/repo".into(),
            root: "/pool/b".into()
        }
    );
    assert!(app.screens().select.worktree.waits_for(removal));
    answer(&mut app, removal, Done::WorktreeRemoved(Ok(())));
    assert!(
        !app.screens().select.worktree.waits_for(removal),
        "answered once"
    );
    assert_eq!(
        notes(&app),
        [note(Note::WorktreeRemoved {
            root: "/pool/b".into()
        })]
    );
    // a late second answer is nobody's
    answer(&mut app, removal, Done::WorktreeRemoved(Ok(())));
    assert_eq!(notes(&app).len(), 1);
    // a failure says why
    let (failing, _) = asked(&say(
        &mut app,
        Verb::Pick(Pick::Remove {
            root: "/pool/c".into(),
        }),
    ));
    answer(&mut app, failing, Done::WorktreeRemoved(Err("busy".into())));
    assert_eq!(
        notes(&app)[1],
        note(Note::WorktreeRemoveFailed {
            root: "/pool/c".into(),
            error: "busy".into()
        })
    );
}

#[test]
fn copying_the_working_directory_asks_the_clipboard() {
    let mut app = CodexApp::new();
    open_worktrees(&mut app, "/pool/a/repo");
    let fx = say(&mut app, Verb::Pick(Pick::Copy("/pool/b/repo".into())));
    let (_, ask) = asked(&fx);
    assert!(
        matches!(&ask, Ask::Copy { text, .. } if text == "/pool/b/repo"),
        "{ask:?}"
    );
}

#[test]
fn the_worktree_command_shows_the_sheet_where_worktrees_are_on_and_says_so_where_they_are_not() {
    let mut app = CodexApp::new();
    let command = |app: &mut CodexApp| {
        run(
            app,
            AppEv::Intent(Intent::Screen(ScreenIntent::Chat(chat::Intent::Session {
                thread: None,
                command: SlashCommand::Worktree,
                intent: SessionIntent::Worktree,
            }))),
        )
    };
    command(&mut app);
    assert_eq!(shows(&app), []);
    assert_eq!(notes(&app), [note(Note::WorktreesUnavailable)]);
    run(
        &mut app,
        AppEv::Screen(ScreenEv::Chat(chat::Ev::Flags(CommandFlags {
            worktrees_enabled: true,
            ..CommandFlags::default()
        }))),
    );
    command(&mut app);
    assert_eq!(shows(&app), [Opening::Worktree(worktree::Menu::Worktrees)]);
}
