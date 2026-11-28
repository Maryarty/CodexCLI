// test root: pages
//! The composer's sparkle tells `/daybreak` from a message by the entry the popup lists: the
//! entry follows the account and the thread's provider (`Access::row`), the same as the popup's.
//! A draft that is a command leaves a fresh task's opportunity whole (the oracle's
//! `arbitrary_commands_preserve_a_never_used_fresh_opportunity`), one that is a message ends it.
use crate::kit::{Owner, Screen};

const PLAIN: &str = include_str!("../oracle/fixtures/pane_footer_1/plain.jsonl");
/// The oracle's sparkle palette (`palette()`).
const FOREGROUND: u32 = 0xe6_d8_ff;
const BACKGROUND: u32 = 0x24_1b_35;

/// The host's facts: the model the status line names and the provider the thread runs on.
fn facts(model: &str) -> String {
    format!(
        r#"{{"step":"screen","dir":"chat","ev":{{"status":{{"model":"{model}","provider":"openai"}}}}}}"#
    )
}

/// A fresh task on a model that is no Astra, `/dayb` typed and cleared, then the Astra model: the
/// dots are there if the typed draft was a command; `eligible`: the account has cyber access on
/// the thread's provider.
fn after_typing_daybreak(eligible: bool) -> Screen {
    let open = Screen::build(Owner::UiPane)
        .size(80, 24)
        .script(PLAIN)
        .terminal_colors(FOREGROUND, BACKGROUND);
    let mut s = if eligible {
        open.daybreak(&[]).open()
    } else {
        open.open()
    };
    s.feed(&facts("gpt-5.5"));
    s.assert_no_dots();
    s.type_text("/dayb");
    s.keys("ctrl+c");
    s.feed(&facts("gpt-6-astra"));
    s
}

#[test]
fn typing_daybreak_is_a_command_for_an_eligible_account() {
    after_typing_daybreak(true).assert_dots_lit(true, "`/dayb` is the prefix of a listed command");
}

#[test]
fn typing_daybreak_is_a_message_without_cyber_access() {
    after_typing_daybreak(false).assert_dots_lit(false, "`/dayb` is no command");
}
