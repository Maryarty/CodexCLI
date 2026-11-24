// test root: pages
//! Mouse tests of the startup page's screens (ADR-011): a tap on an option
//! row selects it, a tap on the selected row or a double tap accepts it, the
//! wheel scrolls the rows, the provider wizard's fields take focus on a
//! click. The oracle has no mouse; these hold the port's ADR-011 contract on
//! the oracle's own fixtures (the debug entries of `onboarding` and
//! `startup`), with the kit's mouse events. With the mouse unused every
//! screen equals the oracle's: the oracle shards say so.

use crate::kit;

use kit::{Owner, Screen};

fn onboarding(entry: &'static str, width: u16, height: u16) -> Screen {
    Screen::build(Owner::Onboarding)
        .size(width, height)
        .entry("onboarding", entry)
        .open()
}

fn prompts(entry: &'static str, width: u16, height: u16) -> Screen {
    Screen::build(Owner::StartupPrompts)
        .size(width, height)
        .entry("startup", entry)
        .open()
}

/// The row of the screen that starts with `mark`.
fn marked(s: &Screen, mark: &str) -> String {
    s.rows()
        .into_iter()
        .find(|row| row.trim_start().starts_with(mark))
        .unwrap_or_default()
}

// ---- the sign-in menu (S02): Options rows, `>` marks the highlight ----

#[test]
fn signin_a_click_selects_the_row_under_it() {
    let mut s = onboarding("sign_in_menu", 76, 19);
    assert!(marked(&s, "> ").contains("1. Sign in with ChatGPT"));
    s.click_text("2. Sign in with Device Code");
    assert!(
        marked(&s, "> ").contains("2. Sign in with Device Code"),
        "{}",
        s.text()
    );
    assert!(s.text().contains("  1. Sign in with ChatGPT"));
}

#[test]
fn signin_a_click_on_the_description_selects_the_row() {
    let mut s = onboarding("sign_in_menu", 76, 19);
    s.click_text("Sign in from another device");
    assert!(
        marked(&s, "> ").contains("2. Sign in with Device Code"),
        "{}",
        s.text()
    );
}

#[test]
fn signin_a_click_on_the_selected_row_accepts_it() {
    let mut s = onboarding("sign_in_menu", 76, 19);
    s.click_text("3. Sign in another way");
    // The first click only selects.
    s.assert_absent("Press esc to go back");
    assert!(marked(&s, "> ").contains("3. Sign in another way"));
    s.click_text("3. Sign in another way");
    s.assert_contains("> 1. Use an OpenAI API key");
    s.assert_contains("Press esc to go back");
}

#[test]
fn signin_a_double_click_accepts_the_row() {
    let mut s = onboarding("sign_in_menu", 76, 19);
    s.double_click_text("3. Sign in another way");
    s.assert_contains("> 1. Use an OpenAI API key");
    s.assert_contains("Press esc to go back");
}

#[test]
fn signin_a_click_selects_in_the_second_menu_too() {
    let mut s = onboarding("sign_in_bedrock", 76, 19);
    s.keys("3");
    s.click_text("2. Use Amazon Bedrock");
    assert!(
        marked(&s, "> ").contains("2. Use Amazon Bedrock"),
        "{}",
        s.text()
    );
    // The key moves on from where the click left the highlight.
    s.keys("up");
    assert!(marked(&s, "> ").contains("1. Use an OpenAI API key"));
}

#[test]
fn signin_a_click_in_the_compact_menu_selects() {
    // 76x8: the descriptions and gaps give way (the oracle's short pane).
    let mut s = onboarding("sign_in_menu", 76, 8);
    s.click_text("2. Sign in with Device Code");
    assert!(
        marked(&s, "> ").contains("2. Sign in with Device Code"),
        "{}",
        s.text()
    );
}

#[test]
fn signin_a_click_on_a_disabled_row_does_nothing() {
    let mut s = Screen::build(Owner::Onboarding)
        .size(76, 19)
        .script(concat!(
            r#"{"step": "screen", "dir": "onboarding", "ev": {"sign_in": {"chatgpt": false}}}"#,
            "\n"
        ))
        .open();
    let before = s.rows();
    s.click_text("1. Sign in with ChatGPT");
    s.click_text("1. Sign in with ChatGPT");
    assert_eq!(s.rows(), before, "{}", s.text());
}

// ---- the Google Cloud wizard's fields ----

#[test]
fn gcp_a_click_on_the_field_focuses_it() {
    let mut s = onboarding("sign_in_cloud", 76, 12);
    s.keys("3");
    s.wait(1100);
    s.keys("3");
    s.wait(1100);
    s.assert_contains("Google Cloud project");
    // Focus moves off the field (Tab: the step's node takes it, the field
    // gets no text), a click on the field brings it back: the text typed
    // after it lands in the field.
    s.keys("tab");
    s.type_text("zzz");
    s.assert_absent("zzz");
    s.click_text("my-project");
    s.type_text("abc");
    s.assert_contains("abc");
}

// ---- the update prompt (S01) ----

#[test]
fn update_prompt_a_click_selects_the_row() {
    let mut s = onboarding("update_prompt", 80, 12);
    s.click_text("2. Skip");
    s.assert_contains("› 2. Skip");
    s.assert_contains("Update available");
}

#[test]
fn update_prompt_a_click_on_the_selected_row_accepts_it() {
    let mut s = onboarding("update_prompt", 80, 12);
    s.click_text("2. Skip");
    s.assert_contains("Update available");
    s.click_text("2. Skip");
    s.assert_absent("Update available");
}

#[test]
fn update_prompt_a_double_click_accepts_the_row() {
    let mut s = onboarding("update_prompt", 80, 12);
    s.double_click_text("3. Skip until next version");
    s.assert_absent("Update available");
}

#[test]
fn update_prompt_the_wheel_scrolls_the_rows() {
    // Eight rows: one choice is out of view below; the wheel brings it.
    let mut s = onboarding("update_prompt", 80, 8);
    s.assert_absent("3. Skip until next version");
    s.wheel_down(10, 4, 3);
    s.assert_contains("3. Skip until next version");
    s.wheel_up(10, 4, 3);
    s.assert_absent("3. Skip until next version");
}

// ---- the startup prompts (S05): the shared panel ----

#[test]
fn cwd_prompt_a_click_selects_the_row() {
    let mut s = prompts("cwd_resume", 80, 14);
    s.assert_contains("› 1. Use session directory");
    s.click_text("2. Use current directory");
    s.assert_contains("› 2. Use current directory");
    s.assert_contains("Working directory");
}

#[test]
fn cwd_prompt_a_click_on_the_selected_row_accepts_it() {
    let mut s = prompts("cwd_resume", 80, 14);
    s.click_text("2. Use current directory");
    s.click_text("2. Use current directory");
    s.assert_absent("Working directory");
}

#[test]
fn cwd_prompt_a_double_click_accepts_the_row() {
    let mut s = prompts("cwd_resume", 80, 14);
    s.double_click_text("2. Use current directory");
    s.assert_absent("Working directory");
}

#[test]
fn migration_prompt_a_click_selects_and_a_second_accepts() {
    let mut s = prompts("migration_opt_out", 80, 20);
    s.click_text("2. Use existing model");
    s.assert_contains("› 2. Use existing model");
    s.assert_contains("Codex just got an upgrade");
    s.click_text("2. Use existing model");
    s.assert_absent("Codex just got an upgrade");
}

#[test]
fn oss_prompt_a_click_moves_the_description_with_the_cursor() {
    let mut s = prompts("oss_mixed", 80, 16);
    s.assert_contains("Local LM Studio server (default port 1234)");
    s.click_text("2. Ollama");
    assert!(marked(&s, "›").contains("2. Ollama"), "{}", s.text());
    s.assert_contains("Local Ollama server (Responses API, default port 11434)");
    s.assert_absent("Local LM Studio server");
}

// ---- the trust step (S04) ----

#[test]
fn trust_a_click_selects_the_row() {
    let mut s = onboarding("trust_git_repo", 80, 20);
    s.assert_contains("› 1. Trust and continue");
    s.click_text("2. Quit");
    assert!(marked(&s, "›").contains("2. Quit"), "{}", s.text());
    s.assert_contains("Trust and continue");
}

#[test]
fn trust_a_click_on_the_selected_row_accepts_it() {
    // The fixture writes no trust (no host answers it): the accepted row is
    // `2. Quit`, which closes the page as Enter on it does
    // (`trust_directory.rs` `release_event_does_not_change_selection`).
    let mut s = onboarding("trust_git_repo", 80, 20);
    s.click_text("2. Quit");
    s.assert_contains("Folder access");
    s.click_text("2. Quit");
    s.assert_absent("Folder access");
}

#[test]
fn trust_a_double_click_accepts_the_row() {
    let mut s = onboarding("trust_git_repo", 80, 20);
    s.double_click_text("2. Quit");
    s.assert_absent("Folder access");
}

// ---- the import (S06): the source picker and the review ----

fn import(entry: &'static str, width: u16, height: u16) -> Screen {
    Screen::build(Owner::RoutesLists)
        .size(width, height)
        .entry("import", entry)
        .open()
}

#[test]
fn import_source_a_click_selects_the_row() {
    let mut s = import("source_prompt", 80, 16);
    s.assert_contains("› 1. Claude Code");
    s.click_text("2. Cursor");
    assert!(marked(&s, "›").contains("2. Cursor"), "{}", s.text());
}

#[test]
fn import_source_a_double_click_accepts_the_row() {
    let mut s = import("source_prompt", 80, 16);
    s.double_click_text("2. Cursor");
    s.assert_absent("1. Claude Code");
}

#[test]
fn import_review_a_click_moves_the_cursor_to_the_item() {
    let mut s = import("review_sample", 80, 30);
    // The customize view lists the items (`c`).
    s.keys("c");
    let before = marked(&s, "›");
    s.click_text("[x] Instructions");
    let after = marked(&s, "›");
    assert!(after.contains("Instructions"), "{before} -> {}", s.text());
}

#[test]
fn import_review_a_click_on_the_item_under_the_cursor_toggles_it() {
    let mut s = import("review_sample", 80, 30);
    // The customize view lists the items (`c`).
    s.keys("c");
    s.click_text("[x] Instructions");
    s.click_text("[x] Instructions");
    s.assert_contains("[ ] Instructions");
}

#[test]
fn import_review_a_click_on_the_action_selects_it() {
    let mut s = import("review_sample", 80, 30);
    // The customize view lists the items (`c`).
    s.keys("c");
    s.click_text("1. Review selection");
    assert!(marked(&s, "›").contains("Review selection"), "{}", s.text());
}

// ---- the other startup prompts (S05): hooks, unarchive, daemon ----

#[test]
fn hooks_prompt_a_double_click_accepts_the_row() {
    let mut s = prompts("hooks_review", 80, 20);
    s.assert_contains("3. ");
    s.double_click_text("3. ");
    s.assert_absent("1. ");
}

#[test]
fn unarchive_prompt_a_click_selects_the_row() {
    let mut s = prompts("unarchive_resume", 80, 16);
    s.click_text("2. ");
    assert!(marked(&s, "›").contains("2. "), "{}", s.text());
}

#[test]
fn daemon_prompt_a_click_selects_the_row() {
    let mut s = prompts("daemon_menu", 80, 20);
    s.click_text("2. ");
    assert!(marked(&s, "›").contains("2. "), "{}", s.text());
}
