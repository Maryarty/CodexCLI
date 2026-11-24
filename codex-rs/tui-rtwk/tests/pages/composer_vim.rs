// test root: pages
//! The composer's modal (Vim) editing in the page: the keys go through the
//! keymap's Vim contexts into the engine (`core::screens::composer::vim`, its
//! own state tests), and the draft keeps its elements whole while the engine
//! edits. Vim is turned on with `global.toggle_vim_mode` bound to F9 (no
//! default key: the oracle toggles it by the `/vim` command).

use crate::common;

use common::{Harness, start};

/// The composer's prompt row (`›` or the shell's `!`), the last one.
fn prompt(h: &Harness) -> String {
    h.t.screen()
        .lines()
        .rfind(|row| row.starts_with('›') || row.starts_with('!'))
        .unwrap_or_default()
        .trim_end()
        .to_string()
}

/// A chat whose session is up and idle, `toggle_vim_mode` on F9.
fn chat() -> Harness {
    let mut h = start();
    h.wait_idle();
    h.set_keymap(&[("global.toggle_vim_mode", "f9")]);
    h
}

/// The keys one after another, as a person presses them.
fn keys(h: &mut Harness, keys: &str) {
    for key in keys.split(' ') {
        h.press(key);
    }
}

/// `text` typed, Vim on (Normal mode, the caret where typing left it).
fn vim_draft(h: &mut Harness, text: &str) {
    h.t.type_text(text);
    h.advance(100);
    h.press("f9");
}

#[test]
fn normal_mode_keys_are_commands_not_text() {
    let mut h = chat();
    vim_draft(&mut h, "hello");
    keys(&mut h, "0 x");
    assert_eq!(prompt(&h), "› ello");
    // a key bound to nothing types nothing
    keys(&mut h, "z");
    assert_eq!(prompt(&h), "› ello");
}

#[test]
fn escape_leaves_insert_mode_one_cell_left() {
    let mut h = chat();
    vim_draft(&mut h, "ab");
    keys(&mut h, "i");
    h.t.type_text("cd");
    h.advance(100);
    assert_eq!(prompt(&h), "› abcd");
    // Normal mode again with the caret on `d`: `x` takes it
    keys(&mut h, "esc x");
    assert_eq!(prompt(&h), "› abc");
}

#[test]
fn dot_repeats_the_last_change() {
    let mut h = chat();
    vim_draft(&mut h, "a b c");
    keys(&mut h, "0 d w");
    assert_eq!(prompt(&h), "› b c");
    keys(&mut h, ".");
    assert_eq!(prompt(&h), "› c");
}

#[test]
fn undo_and_redo_step_the_whole_draft() {
    let mut h = chat();
    vim_draft(&mut h, "hello");
    keys(&mut h, "0 x x");
    assert_eq!(prompt(&h), "› llo");
    keys(&mut h, "u");
    assert_eq!(prompt(&h), "› ello");
    keys(&mut h, "u");
    assert_eq!(prompt(&h), "› hello");
    keys(&mut h, "ctrl+r");
    assert_eq!(prompt(&h), "› ello");
}

#[test]
fn a_pending_operator_takes_enter_instead_of_submitting() {
    let mut h = chat();
    vim_draft(&mut h, "hello");
    keys(&mut h, "d enter");
    assert_eq!(
        prompt(&h),
        "› hello",
        "Enter ended the `d`, nothing was sent"
    );
    // the next `x` is a plain `x`, the `d` is gone
    keys(&mut h, "0 x");
    assert_eq!(prompt(&h), "› ello");
}

#[test]
fn a_search_moves_the_caret_to_the_match() {
    let mut h = chat();
    vim_draft(&mut h, "foo bar foo");
    keys(&mut h, "0 /");
    h.t.type_text("bar");
    h.advance(100);
    // the query is typed, not the draft's text
    assert_eq!(prompt(&h), "› foo bar foo");
    keys(&mut h, "enter x");
    assert_eq!(prompt(&h), "› foo ar foo");
    // a second search finds the first `foo` from the start of the line
    keys(&mut h, "0 /");
    h.t.type_text("foo");
    h.advance(100);
    keys(&mut h, "enter x");
    assert_eq!(prompt(&h), "› foo ar oo");
}

#[test]
fn a_slash_on_an_empty_draft_goes_into_insert_mode_and_types_it() {
    let mut h = chat();
    vim_draft(&mut h, "");
    keys(&mut h, "/");
    assert_eq!(prompt(&h), "› /");
}

#[test]
fn a_remapped_search_key_opens_the_search_prompt_on_an_empty_draft() {
    let mut h = start();
    h.wait_idle();
    h.set_keymap(&[
        ("global.toggle_vim_mode", "f9"),
        ("vim_search.forward", "s"),
    ]);
    vim_draft(&mut h, "");
    // `s` is the search key now: it starts the query (not the draft's text), and `/` is no longer
    // one, so the oracle's slash rule (a search key AND a plain `/`) leaves Normal mode alone
    keys(&mut h, "s");
    h.t.type_text("x");
    h.advance(100);
    assert_eq!(prompt(&h), "› Ask Codex to do anything");
    keys(&mut h, "esc /");
    assert_eq!(prompt(&h), "› Ask Codex to do anything");
}

#[test]
fn escape_drops_the_search_query() {
    let mut h = chat();
    vim_draft(&mut h, "draft");
    keys(&mut h, "0 /");
    h.t.type_text("raf");
    h.advance(100);
    keys(&mut h, "esc x");
    // the query is gone and `x` is the command it is
    assert_eq!(prompt(&h), "› raft");
}

#[test]
fn toggling_vim_off_types_text_again() {
    let mut h = chat();
    vim_draft(&mut h, "ab");
    keys(&mut h, "f9");
    h.t.type_text("c");
    h.advance(100);
    assert_eq!(prompt(&h), "› abc");
}

#[test]
fn e_advances_from_each_word_end() {
    // the oracle's `vim_e_advances_from_each_word_end`: "alpha beta gamma"
    // with the caret after "alph", three `e` presses (one state each)
    for (presses, left) in [
        ("e", "alpha bet gamma"),
        ("e e", "alpha beta gamm"),
        ("e e e", "alpha beta gamm"),
    ] {
        let mut h = chat();
        vim_draft(&mut h, "alpha beta gamma");
        keys(&mut h, "0 right right right right");
        keys(&mut h, presses);
        keys(&mut h, "x");
        // `e` from the last cell of a word goes to the end of the next one
        // and stays at the end of the draft
        assert_eq!(prompt(&h), format!("› {left}"), "after `{presses}`");
    }
}

#[test]
fn a_paste_in_normal_mode_is_its_own_undo_step() {
    let mut h = chat();
    vim_draft(&mut h, "abc");
    keys(&mut h, "0 x");
    h.t.paste("Z");
    h.advance(100);
    assert_eq!(prompt(&h), "› Zbc");
    keys(&mut h, "u");
    assert_eq!(prompt(&h), "› bc");
    keys(&mut h, "u");
    assert_eq!(prompt(&h), "› abc");
}

#[test]
fn a_placeholder_is_one_unit_to_normal_mode_commands() {
    let mut h = chat();
    h.t.paste(&"z".repeat(1001));
    h.advance(100);
    let shown = prompt(&h);
    assert!(shown.contains("[Pasted Content 1001 chars]"), "{shown}");
    h.press("f9");
    keys(&mut h, "0 x");
    assert_eq!(
        prompt(&h),
        "› Ask Codex to do anything",
        "`x` took the whole placeholder"
    );
}

#[test]
fn slash_vim_turns_modal_editing_on_and_off_and_says_so() {
    let mut h = start();
    h.wait_idle();
    h.t.type_text("/vim");
    h.advance(100);
    h.press("enter");
    h.advance(100);
    let screen = h.t.screen();
    assert!(screen.contains("Vim mode enabled."), "{screen}");
    assert!(screen.contains("Vim: Normal"), "{screen}");
    assert_eq!(prompt(&h), "› Ask Codex to do anything");
    // the footer names the mode the keys change
    keys(&mut h, "i");
    assert!(h.t.screen().contains("Vim: Insert"), "{}", h.t.screen());
    h.t.type_text("/vim");
    h.advance(100);
    h.press("enter");
    h.advance(100);
    let screen = h.t.screen();
    assert!(screen.contains("Vim mode disabled."), "{screen}");
    assert!(!screen.contains("Vim: "), "{screen}");
}

#[test]
fn a_typed_character_is_what_dot_repeats_after_escape() {
    let mut h = chat();
    keys(&mut h, "f9 i");
    h.t.type_text("x");
    h.advance(100);
    keys(&mut h, "esc .");
    assert_eq!(prompt(&h), "› xx");
}
