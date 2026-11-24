// test root: pages
//! `/review` in the session page (oracle `chatwidget/review_popups.rs`, `slash_dispatch.rs`): the
//! four presets over the composer, the base branches and the commits the host lists, the custom
//! instructions prompt, and `/review <text>` that starts the review at once. A pick starts
//! `review/start` on the thread; the menus are dismissed with it.

use codex_tui_core::app::Command;
use codex_tui_core::app::effects::{Ask, Done};
use codex_tui_core::screens::select::review::Commit;
use codex_tui_core::screens::{ScreenCommand, chat};
use codex_tui_core::threads::ReviewTarget;

use crate::kit::{Owner, Screen};

fn chat_screen() -> Screen {
    Screen::build(Owner::DialogsPane).size(80, 24).live().open()
}

fn presets() -> Screen {
    let mut s = chat_screen();
    s.type_text("/review");
    s.keys("enter");
    s.assert_contains("Select a review preset");
    s
}

/// The reviews the app started on the thread, oldest first.
fn started(s: &Screen) -> Vec<ReviewTarget> {
    s.commands()
        .into_iter()
        .filter_map(|command| match command {
            Command::Screen {
                command: ScreenCommand::Chat(chat::Command::Review(target)),
                ..
            } => Some(target),
            _ => None,
        })
        .collect()
}

/// The git lists the menus asked of the host, oldest first.
fn asked(s: &Screen) -> Vec<Command> {
    s.commands()
        .into_iter()
        .filter(|command| {
            matches!(
                command,
                Command::Effect {
                    ask: Ask::GitBranches { .. } | Ask::GitLog { .. },
                    ..
                }
            )
        })
        .collect()
}

fn branches(current: Option<&str>) -> Done {
    Done::GitBranches {
        current: current.map(str::to_owned),
        branches: vec!["main".into(), "feature/parser".into(), "release".into()],
    }
}

fn log() -> Done {
    Done::GitLog(vec![
        Commit {
            sha: "abc1234".into(),
            subject: "Fix the parser".into(),
        },
        Commit {
            sha: "def5678".into(),
            subject: "Add review".into(),
        },
    ])
}

#[test]
fn a_bare_review_lists_the_four_presets_in_the_oracles_order() {
    let s = presets();
    s.assert_in_order(&[
        "Select a review preset",
        "1. Review against a base branch  (PR Style)",
        "2. Review uncommitted changes",
        "3. Review a commit",
        "4. Custom review instructions",
        "enter select · esc back",
    ]);
}

#[test]
fn enter_on_uncommitted_changes_starts_the_review_and_closes_the_sheet() {
    let mut s = presets();
    s.keys("down enter");
    assert_eq!(started(&s), [ReviewTarget::UncommittedChanges]);
    s.assert_absent("Select a review preset");
}

#[test]
fn a_tap_on_a_preset_starts_the_review_and_a_tap_on_a_row_above_nothing_else_does() {
    let mut s = presets();
    s.click_text("Review uncommitted changes");
    assert_eq!(started(&s), [ReviewTarget::UncommittedChanges]);
    s.assert_absent("Select a review preset");
}

#[test]
fn the_base_branch_picker_names_the_current_branch_and_a_pick_starts_the_review() {
    let mut s = presets();
    s.keys("enter");
    let asks = asked(&s);
    assert_eq!(asks.len(), 1, "{asks:?}");
    s.answer_effect(&asks[0], branches(Some("main")));
    s.assert_in_order(&[
        "Select a base branch",
        "Current branch: main",
        "Type to search branches",
        "main",
        "feature/parser",
        "release",
    ]);
    // the filter narrows the rows, Enter picks the one left
    s.type_text("pars");
    s.assert_absent("release");
    s.keys("enter");
    assert_eq!(
        started(&s),
        [ReviewTarget::BaseBranch {
            branch: "feature/parser".into()
        }]
    );
    s.assert_absent("Select a base branch");
    s.assert_absent("Select a review preset");
}

#[test]
fn a_detached_head_says_so_and_esc_returns_to_the_presets_on_the_row_that_opened_it() {
    let mut s = presets();
    s.keys("enter");
    let asks = asked(&s);
    s.answer_effect(&asks[0], branches(None));
    s.assert_contains("Current branch: (detached HEAD)");
    s.keys("esc");
    s.assert_absent("Select a base branch");
    s.assert_contains("› 1. Review against a base branch");
}

#[test]
fn a_tap_on_a_commit_starts_the_review_of_it_with_its_subject() {
    let mut s = presets();
    s.keys("down down enter");
    let asks = asked(&s);
    assert_eq!(asks.len(), 1, "{asks:?}");
    s.answer_effect(&asks[0], log());
    s.assert_in_order(&[
        "Select a commit to review",
        "Type to search commits",
        "Fix the parser",
        "Add review",
    ]);
    s.click_text("Add review");
    assert_eq!(
        started(&s),
        [ReviewTarget::Commit {
            sha: "def5678".into(),
            title: Some("Add review".into())
        }]
    );
    s.assert_absent("Select a commit to review");
}

#[test]
fn the_wheel_scrolls_a_long_commit_list_and_a_tap_picks_a_row_it_brought_in() {
    let mut s = presets();
    s.keys("down down enter");
    let asks = asked(&s);
    let commits = (0..40)
        .map(|n| Commit {
            sha: format!("{n:07x}"),
            subject: format!("Change number {n}"),
        })
        .collect();
    s.answer_effect(&asks[0], Done::GitLog(commits));
    s.assert_contains("Change number 0");
    s.assert_absent("Change number 30");
    // the picker sits at the bottom of the 80x24 screen: row 18 is one of its rows
    s.wheel_down(10, 18, 20);
    s.assert_absent("Change number 0");
    s.assert_contains("Change number 30");
    s.click_text("Change number 30");
    assert_eq!(
        started(&s),
        [ReviewTarget::Commit {
            sha: format!("{:07x}", 30),
            title: Some("Change number 30".into())
        }]
    );
}

#[test]
fn the_commit_filter_matches_the_sha_as_well_as_the_subject() {
    let mut s = presets();
    s.keys("down down enter");
    let asks = asked(&s);
    s.answer_effect(&asks[0], log());
    s.type_text("def5");
    s.assert_absent("Fix the parser");
    s.assert_contains("Add review");
}

#[test]
fn the_instructions_prompt_takes_text_and_enter_starts_a_custom_review() {
    let mut s = presets();
    s.keys("down down down enter");
    s.assert_in_order(&[
        "Custom review instructions",
        "› Type instructions and press Enter",
        "enter submit · esc back",
    ]);
    // an empty answer is not one: the prompt stays
    s.keys("enter");
    s.assert_contains("Type instructions and press Enter");
    assert!(started(&s).is_empty());
    s.type_text("  look at the tests ");
    s.assert_contains("look at the tests");
    s.keys("enter");
    assert_eq!(
        started(&s),
        [ReviewTarget::Custom {
            instructions: "look at the tests".into()
        }]
    );
    s.assert_absent("Custom review instructions");
}

#[test]
fn esc_in_the_instructions_prompt_returns_to_the_presets_on_its_row() {
    let mut s = presets();
    s.keys("down down down enter");
    s.type_text("draft text");
    s.keys("esc");
    s.assert_absent("Type instructions and press Enter");
    s.assert_contains("Select a review preset");
    s.assert_contains("› 4. Custom review instructions");
    assert!(started(&s).is_empty());
}

#[test]
fn a_review_with_text_starts_a_custom_review_at_once() {
    let mut s = chat_screen();
    s.type_text("/review check the error paths");
    s.keys("enter");
    assert_eq!(
        started(&s),
        [ReviewTarget::Custom {
            instructions: "check the error paths".into()
        }]
    );
    s.assert_absent("Select a review preset");
}
