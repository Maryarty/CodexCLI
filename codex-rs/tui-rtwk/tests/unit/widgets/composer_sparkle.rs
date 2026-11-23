use codex_tui_core::commands::CommandFlags;
use codex_tui_core::commands::ServiceTierCommand;
use codex_tui_core::commands::find_slash_command;
use codex_tui_core::commands::has_slash_command_prefix;
use codex_tui_rtwk::widgets::composer_sparkle::*;
use rtwk::foundation::ValueNotifier;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

fn typed(sparkle: &SparkleController, ch: char, text: &str) {
    sparkle.before_key(&Key {
        plain_char: Some(ch),
        empty: text.is_empty(),
        cursor: text.len(),
        text: text.to_string(),
        aside: false,
        vim_query: false,
        vim_normal: false,
    });
}

fn armed() -> SparkleController {
    let sparkle = SparkleController::default();
    sparkle.know(CommandFlags::default(), Vec::new(), false);
    sparkle.offer(true);
    sparkle
}

#[test]
fn a_typed_character_ends_the_opportunity() {
    let sparkle = armed();
    typed(&sparkle, 'x', "");
    assert_eq!(sparkle.draft(), Draft::Dismissed);
    assert_eq!(sparkle.phase(), Phase::Finished);
}

#[test]
fn a_command_keeps_it_and_the_slash_is_held_not_dismissed() {
    let sparkle = armed();
    typed(&sparkle, '/', "");
    assert_eq!(sparkle.draft(), Draft::Command);
    assert_eq!(sparkle.phase(), Phase::Waiting);
    typed(&sparkle, 's', "/");
    assert_eq!(sparkle.draft(), Draft::Command);
    // not a command's name: the opportunity goes
    typed(&sparkle, 'q', "/s");
    typed(&sparkle, 'z', "/sq");
    typed(&sparkle, 'z', "/sqz");
    assert_eq!(sparkle.draft(), Draft::Dismissed);
}

#[test]
fn erasing_a_command_restores_the_opportunity() {
    let sparkle = armed();
    typed(&sparkle, '/', "");
    sparkle.text_changed("/", false);
    sparkle.text_changed("", false);
    assert_eq!(sparkle.draft(), Draft::Untouched);
    assert_eq!(sparkle.phase(), Phase::Waiting);
}

#[test]
fn a_command_that_ran_keeps_it_and_a_message_ends_it() {
    let sparkle = armed();
    typed(&sparkle, '/', "");
    sparkle.line_ran(true);
    assert_eq!(sparkle.draft(), Draft::Untouched);
    sparkle.line_ran(false);
    assert_eq!(sparkle.draft(), Draft::Dismissed);
}

#[test]
fn the_history_search_previews_do_not_count() {
    let sparkle = armed();
    sparkle.text_changed("stored draft", true);
    assert_eq!(sparkle.draft(), Draft::Untouched);
    sparkle.text_changed("stored draft", false);
    assert_eq!(sparkle.draft(), Draft::Dismissed);
}

#[test]
fn the_deadline_runs_from_the_first_frame_and_not_again() {
    let sparkle = armed();
    sparkle.begin(Duration::from_secs(3));
    sparkle.begin(Duration::from_secs(9));
    assert_eq!(sparkle.phase(), Phase::Visible(Duration::from_secs(3)));
    sparkle.offer(false);
    assert_eq!(sparkle.phase(), Phase::Finished);
    sparkle.offer(true);
    assert_eq!(sparkle.phase(), Phase::Finished);
}

#[test]
fn a_pasted_command_is_a_command_and_other_text_is_not() {
    let sparkle = armed();
    sparkle.pasted("/status", "", 0, true);
    assert_eq!(sparkle.draft(), Draft::Command);
    let other = armed();
    other.pasted("hello", "", 0, true);
    assert_eq!(other.draft(), Draft::Dismissed);
}
