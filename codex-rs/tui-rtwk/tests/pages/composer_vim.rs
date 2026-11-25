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

/// The draft's own recovered-answer append (`append_recovered_drafts`), the
/// question sheet's turn-end recovery (the sheet is another feature's).
mod recovered {
    use codex_tui_core::kit::vim::{Input, Normal, Operator};
    use codex_tui_rtwk::widgets::composer_draft::{ComposerDraft, VimHost};
    use rtwk::editing::TextEditingController;

    fn vim_draft(text: &str) -> (ComposerDraft, TextEditingController) {
        let c = TextEditingController::new(text);
        let d = ComposerDraft::default();
        d.set_vim_enabled(true);
        let mut host = VimHost::new(&c, &d);
        d.with_vim(|v| v.enter_normal(&mut host));
        (d, c)
    }

    fn recover(d: &ComposerDraft, c: &TextEditingController, answer: &str) {
        d.append_recovered_drafts(c, answer, |n| format!("[Pasted Content {n} chars]"));
    }

    /// A recovered answer follows a blank line and is one step of undo.
    #[test]
    fn a_recovered_answer_is_one_undoable_edit() {
        let (d, c) = vim_draft("abc");
        let before = d.snapshot(&c);
        recover(&d, &c, "answer");
        assert_eq!(&*c.text(), "abc\n\nanswer");
        assert!(d.vim_step(&c, false));
        assert_eq!(d.snapshot(&c), before);
        assert!(d.vim_step(&c, true));
        assert_eq!(&*c.text(), "abc\n\nanswer");
        // an empty draft takes it without the blank line
        let (d, c) = vim_draft("");
        recover(&d, &c, "answer");
        assert_eq!(&*c.text(), "answer");
    }

    #[test]
    fn a_large_recovered_answer_is_a_placeholder() {
        let (d, c) = vim_draft("abc");
        let answer = "x".repeat(1001);
        recover(&d, &c, &answer);
        assert_eq!(&*c.text(), "abc\n\n[Pasted Content 1001 chars]");
        assert_eq!(d.expanded(&c), format!("abc\n\n{answer}"));
    }

    /// The draft becomes a prompt: a shell mode or a `/command` it began with
    /// is escaped.
    #[test]
    fn a_recovered_answer_escapes_the_command_prefix() {
        let (d, c) = vim_draft("echo");
        d.set_shell(true);
        let before = d.snapshot(&c);
        recover(&d, &c, "answer");
        assert_eq!((&*c.text(), d.shell()), ("\\!echo\n\nanswer", false));
        assert!(d.vim_step(&c, false));
        assert_eq!(d.snapshot(&c), before);
        let (d, c) = vim_draft("/diff");
        recover(&d, &c, "answer");
        assert_eq!(&*c.text(), "\\/diff\n\nanswer");
        // a path is not a command
        let (d, c) = vim_draft("/tmp/x");
        recover(&d, &c, "answer");
        assert_eq!(&*c.text(), "/tmp/x\n\nanswer");
    }

    /// A command that waited for its next key is cancelled.
    #[test]
    fn a_recovered_answer_cancels_a_waiting_query() {
        let (d, c) = vim_draft("existing draft");
        d.with_vim(|v| {
            let mut host = VimHost::new(&c, &d);
            v.input(
                &mut host,
                Input::Normal(Normal::StartOperator(Operator::Delete)),
            );
        });
        assert!(d.with_vim(|v| v.is_operator_pending()));
        recover(&d, &c, "answer");
        assert!(!d.with_vim(|v| v.is_operator_pending()));
        assert_eq!(&*c.text(), "existing draft\n\nanswer");
    }

    /// While the history search shows a preview, the answer goes to the
    /// draft Esc will bring back.
    #[test]
    fn a_recovered_answer_goes_to_the_draft_a_search_kept() {
        let (d, c) = vim_draft("bc");
        d.search_begin(&c);
        c.set_text("archived prompt");
        d.edit_stored(&c, || recover(&d, &c, "answer"));
        assert_eq!(&*c.text(), "archived prompt");
        assert!(d.searching());
        assert!(d.search_cancel(&c));
        assert_eq!(&*c.text(), "bc\n\nanswer");
    }
}

/// A file path picked in the `@` popup while Replace mode is on (the oracle's
/// `vim_replace_completion_preserves_suffix` calls `insert_selected_file_path` itself: the popup's
/// results come from a search over the session's directory, which no screen test can seed).
mod completion {
    use codex_tui_core::kit::mentions::session::{MentionPick, PickAction};
    use codex_tui_core::kit::vim::{Input, Normal};
    use codex_tui_rtwk::widgets::composer_draft::{ComposerDraft, VimHost};
    use rtwk::editing::{TextEditingController, TextSelection};

    #[test]
    fn a_path_in_replace_mode_keeps_the_suffix_and_what_backspace_could_restore() {
        for (text, caret, typed, path) in [
            ("@ma next", 3, "", "src/main.rs"),
            ("abc next", 0, "@ma", "ma"),
        ] {
            let c = TextEditingController::new(text);
            let d = ComposerDraft::default();
            d.set_vim_enabled(true);
            c.set_selection(TextSelection::collapsed(caret));
            d.vim_begin_edit(&c);
            let mut host = VimHost::new(&c, &d);
            d.with_vim(|v| {
                v.input(&mut host, Input::Normal(Normal::EnterReplaceMode));
                v.type_text(&mut host, typed);
            });
            assert!(d.with_vim(|v| v.is_replace()));
            assert_eq!(&*c.text(), "@ma next");
            let pick = MentionPick {
                range: 0.."@ma".len(),
                action: PickAction::Path(path.into()),
                usage: None,
            };
            d.apply_pick(&c, &pick, |n| format!("[Image #{n}]"), false);
            assert_eq!(
                (&*c.text(), c.selection().extent_offset),
                (format!("{path}  next").as_str(), path.len() + 1)
            );
            // what the completion replaced is not what Backspace puts back
            let mut host = VimHost::new(&c, &d);
            assert!(!d.with_vim(|v| v.replace_backspace(&mut host)));
            assert_eq!(&*c.text(), format!("{path}  next"));
        }
    }
}
