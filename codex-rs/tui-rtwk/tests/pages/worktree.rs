//! `/worktree` in the session page (oracle `chatwidget/worktree_picker.rs`, `worktree_browser.rs`):
//! the three rows over the composer, the browser of the repository's worktrees (its loading row,
//! the list the host answers with, a worktree's actions and the delete confirmation), as the
//! routes of one sheet. Every pick of the oracle dismisses its view and shows the next one, so a
//! page replaces the one before it and Esc closes the sheet from any of them; a list given up (Esc
//! while it loads) is dropped when it arrives, and a failed one closes the loading row and is said
//! in the history.

use codex_tui_core::app::Command;
use codex_tui_core::app::effects::{Ask, Done};
use codex_tui_core::ids::ThreadId;
use codex_tui_core::screens::select::worktree::{Entry, Owner as Whose, Thread};

use crate::kit::{Owner, Screen};

const FLAGS: &str = r#"{"step":"screen","dir":"chat","ev":{"flags":{"worktrees_enabled":true}}}
{"step":"screen","dir":"chat","ev":{"status":{"cwd":"/pool/a/repo"}}}"#;

/// The session runs in `/pool/a/repo`, the checkout of the entry `/pool/a`.
const CWD: &str = "/pool/a/repo";

fn chat_screen() -> Screen {
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.feed(FLAGS);
    s
}

fn worktrees() -> Screen {
    let mut s = chat_screen();
    s.type_text("/worktree");
    s.keys("enter");
    s.assert_contains("Continue current conversation");
    s
}

fn entry(root: &str, owner: Whose) -> Entry {
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

/// The pool the host lists: a resumable worktree, an archived one, one nobody owns, the
/// session's own checkout and one whose owner cannot be read.
fn pool() -> Vec<Entry> {
    vec![
        entry("/pool/b", Whose::Resumable(thread("t-b", "Fix parser"))),
        entry("/pool/c", Whose::Archived(thread("t-c", "Old work"))),
        entry("/pool/d", Whose::None),
        entry("/pool/a", Whose::None),
        entry("/pool/e", Whose::Unavailable(ThreadId::new("t-e"))),
    ]
}

/// The lists the sheet asked of the host, oldest first.
fn asked(s: &Screen) -> Vec<Command> {
    s.commands()
        .into_iter()
        .filter(|command| {
            matches!(
                command,
                Command::Effect {
                    ask: Ask::Worktrees { .. },
                    ..
                }
            )
        })
        .collect()
}

/// The removals the sheet asked of the host: the directory and the root, oldest first.
fn removals(s: &Screen) -> Vec<(String, String)> {
    s.commands()
        .into_iter()
        .filter_map(|command| match command {
            Command::Effect {
                ask: Ask::WorktreeRemove { cwd, root },
                ..
            } => Some((cwd, root)),
            _ => None,
        })
        .collect()
}

/// The browser, loaded with `list`.
fn browser(list: Vec<Entry>) -> Screen {
    let mut s = worktrees();
    s.keys("down down enter");
    let asks = asked(&s);
    assert_eq!(asks.len(), 1, "{asks:?}");
    s.answer_effect(&asks[0], Done::Worktrees(Ok(list)));
    s.assert_contains("Managed worktrees");
    s
}

#[test]
fn worktree_lists_the_three_rows_in_the_oracles_order() {
    let s = worktrees();
    s.assert_in_order(&[
        "Worktrees",
        "1. Continue current conversation",
        "Preserve this conversation in the new checkout",
        "2. Start new conversation",
        "Open a fresh conversation in the new checkout",
        "3. Browse worktrees",
        "Resume an owner thread or copy a working directory",
        "enter select · esc back",
    ]);
}

#[test]
fn esc_closes_the_sheet() {
    let mut s = worktrees();
    s.keys("esc");
    s.assert_absent("Continue current conversation");
}

#[test]
fn continuing_the_conversation_picks_and_closes_the_sheet() {
    let mut s = worktrees();
    s.keys("enter");
    s.assert_absent("Continue current conversation");
    assert!(asked(&s).is_empty());
}

#[test]
fn browsing_shows_the_loading_row_in_the_place_of_the_rows_and_asks_the_host_in_the_sessions_directory()
 {
    let mut s = worktrees();
    s.keys("down down enter");
    s.assert_in_order(&["Managed worktrees", "Loading worktrees…", "esc back"]);
    s.assert_absent("Browse worktrees");
    let asks = asked(&s);
    assert_eq!(asks.len(), 1, "{asks:?}");
    assert!(
        matches!(
            &asks[0],
            Command::Effect { ask: Ask::Worktrees { cwd }, .. } if cwd == CWD
        ),
        "{asks:?}"
    );
}

#[test]
fn the_list_replaces_the_loading_row_with_each_worktrees_owner_and_where_it_is() {
    let s = browser(pool());
    s.assert_absent("Loading worktrees");
    s.assert_in_order(&[
        "Managed worktrees",
        "Select a worktree to resume, copy its path, or delete it",
        "Fix parser",
        "updated",
        "· /pool/b/repo",
        "Old work",
        "Archived · updated",
        "· /pool/c/repo",
        "/pool/d/repo",
        "No attached thread · /pool/d/repo",
        "Owner thread unavailable · /pool/e/repo",
    ]);
}

#[test]
fn an_empty_pool_says_so() {
    let s = browser(Vec::new());
    s.assert_contains("No worktrees in this repository's configured pool");
}

#[test]
fn esc_while_the_list_loads_closes_the_sheet_and_the_late_list_is_dropped() {
    let mut s = worktrees();
    s.keys("down down enter");
    let asks = asked(&s);
    s.keys("esc");
    s.assert_absent("Managed worktrees");
    s.assert_absent("Browse worktrees");
    s.answer_effect(&asks[0], Done::Worktrees(Ok(pool())));
    s.assert_absent("Managed worktrees");
    s.assert_absent("Fix parser");
}

#[test]
fn a_list_that_fails_after_esc_says_nothing() {
    let mut s = worktrees();
    s.keys("down down enter");
    let asks = asked(&s);
    s.keys("esc");
    s.answer_effect(
        &asks[0],
        Done::Worktrees(Err("not a git repository".into())),
    );
    s.assert_absent("Cannot list managed worktrees");
}

#[test]
fn a_list_that_fails_closes_the_loading_row_and_the_history_says_why() {
    let mut s = worktrees();
    s.keys("down down enter");
    let asks = asked(&s);
    s.answer_effect(
        &asks[0],
        Done::Worktrees(Err("not a git repository".into())),
    );
    s.assert_absent("Managed worktrees");
    s.assert_contains("Cannot list managed worktrees: not a git repository");
}

#[test]
fn the_filter_narrows_the_browser_by_name_and_directory() {
    let mut s = browser(pool());
    s.type_text("old");
    s.assert_absent("Fix parser");
    s.assert_contains("Old work");
    s.keys("backspace backspace backspace");
    s.type_text("pool/d");
    s.assert_absent("Old work");
    s.assert_contains("No attached thread · /pool/d/repo");
}

#[test]
fn a_pick_opens_the_worktrees_actions_in_the_place_of_the_list_and_esc_closes_everything() {
    let mut s = browser(pool());
    s.keys("enter");
    s.assert_absent("Select a worktree to resume");
    s.assert_in_order(&[
        "Worktree: Fix parser",
        "/pool/b/repo",
        "1. Resume owner thread",
        "2. Copy working directory",
        "3. Delete worktree",
    ]);
    s.keys("esc");
    s.assert_absent("Worktree: Fix parser");
    s.assert_absent("Managed worktrees");
}

#[test]
fn a_worktree_nobody_owns_has_no_resume_row_and_its_title_is_the_plain_one() {
    let mut s = browser(pool());
    s.keys("down down enter");
    s.assert_in_order(&[
        "Worktree",
        "/pool/d/repo",
        "1. Copy working directory",
        "2. Delete worktree",
    ]);
    s.assert_absent("Resume owner thread");
}

#[test]
fn the_checkout_the_session_runs_in_cannot_be_deleted() {
    let mut s = browser(pool());
    s.keys("down down down enter");
    s.assert_in_order(&[
        "/pool/a/repo",
        "Copy working directory",
        "Delete worktree",
        "Switch to another checkout before deleting this one",
    ]);
    // the disabled row does nothing: the actions stay and the host is not asked
    s.keys("down enter");
    s.assert_contains("Switch to another checkout before deleting this one");
    s.assert_absent("Delete this worktree?");
    assert!(removals(&s).is_empty());
}

#[test]
fn deleting_asks_first_and_cancel_is_the_first_row() {
    let mut s = browser(pool());
    s.keys("enter down down enter");
    s.assert_in_order(&[
        "Delete this worktree?",
        "/pool/b",
        "1. Cancel",
        "2. Delete worktree",
        "Keeps thread history; may disrupt other sessions",
    ]);
    s.assert_absent("Resume owner thread");
    s.keys("enter");
    s.assert_absent("Delete this worktree?");
    assert!(removals(&s).is_empty());
}

#[test]
fn esc_on_the_confirmation_closes_everything_and_asks_nothing() {
    let mut s = browser(pool());
    s.keys("enter down down enter");
    s.keys("esc");
    s.assert_absent("Delete this worktree?");
    s.assert_absent("Managed worktrees");
    assert!(removals(&s).is_empty());
}

#[test]
fn confirming_asks_the_host_to_remove_the_worktree_from_the_sessions_directory() {
    let mut s = browser(pool());
    s.keys("enter down down enter");
    s.keys("down enter");
    s.assert_absent("Delete this worktree?");
    assert_eq!(removals(&s), [(CWD.to_owned(), "/pool/b".to_owned())]);
}

#[test]
fn copying_asks_the_clipboard_for_the_worktrees_working_directory() {
    let mut s = browser(pool());
    s.keys("enter down enter");
    s.assert_absent("Worktree: Fix parser");
    let copied: Vec<_> = s
        .commands()
        .into_iter()
        .filter_map(|command| match command {
            Command::Effect {
                ask: Ask::Copy { text, .. },
                ..
            } => Some(text),
            _ => None,
        })
        .collect();
    assert_eq!(copied, ["/pool/b/repo"]);
}

#[test]
fn a_tap_on_browse_loads_and_a_tap_on_a_worktree_opens_its_actions() {
    let mut s = worktrees();
    s.click_text("Browse worktrees");
    s.assert_contains("Loading worktrees…");
    let asks = asked(&s);
    assert_eq!(asks.len(), 1, "{asks:?}");
    s.answer_effect(&asks[0], Done::Worktrees(Ok(pool())));
    s.click_text("Old work");
    s.assert_in_order(&[
        "Worktree: Old work",
        "/pool/c/repo",
        "Copy working directory",
    ]);
    s.assert_absent("Resume owner thread");
}

#[test]
fn the_wheel_scrolls_a_long_list_and_a_tap_picks_a_row_it_brought_in() {
    let long = (0..40)
        .map(|n| entry(&format!("/pool/w{n:02}"), Whose::None))
        .collect();
    let mut s = browser(long);
    s.assert_contains("/pool/w00/repo");
    s.assert_absent("/pool/w30/repo");
    s.wheel_down(10, 18, 20);
    s.assert_absent("/pool/w00/repo");
    s.assert_contains("/pool/w30/repo");
    s.click_text("/pool/w30/repo");
    s.assert_in_order(&["Worktree", "/pool/w30/repo", "Copy working directory"]);
}
