//! Mouse tests of the startup page's screens (ADR-011): a tap on an option
//! row selects it, a tap on the selected row or a double tap accepts it, the
//! wheel scrolls the rows, the provider wizard's fields take focus on a
//! click. The oracle has no mouse; these hold the port's ADR-011 contract on
//! the oracle's own fixtures (the debug entries of `onboarding` and
//! `startup`), with the kit's mouse events. With the mouse unused every
//! screen equals the oracle's: the oracle shards say so.

#[path = "../common/mod.rs"]
mod common;
#[path = "../kit/mod.rs"]
mod kit;

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
    s.assert_absent("Use an OpenAI API key\n");
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
    s.click_text("2. Use existing model");
    s.assert_absent("Introducing gpt-new");
}

#[test]
fn oss_prompt_a_click_moves_the_description_with_the_cursor() {
    let mut s = prompts("oss_mixed", 80, 16);
    let before = s.text();
    s.click_text("2. ");
    assert_ne!(s.text(), before, "the picker did not follow the click");
    assert!(marked(&s, "›").contains("2. "), "{}", s.text());
}
