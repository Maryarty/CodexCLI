//! The Vim engine (`screens::composer::vim`) over a plain text: the oracle's
//! `textarea/vim_commands_tests.rs` state tests (the ones that need no
//! grapheme or element a plain host cannot tell), keyed through the default
//! bindings the view resolves from the keymap.

use std::ops::Range;
use std::rc::Rc;

use codex_tui_core::kit::vim::{
    EditorAction, Find, Input, Motion, Normal, Object, Operator, OperatorKey, Register,
    RegisterKind, Vim, VimDirection, VimPending, VimScope, VimText,
};

/// A draft of plain text: a char is a grapheme, and `elements` are atomic.
struct Plain {
    text: String,
    cursor: usize,
    elements: Vec<Range<usize>>,
    register: Register,
}

impl VimText for Plain {
    fn text(&self) -> Rc<str> {
        Rc::from(self.text.as_str())
    }
    fn cursor(&self) -> usize {
        self.cursor
    }
    fn set_cursor(&mut self, pos: usize) {
        let mut pos = pos.min(self.text.len());
        if let Some(e) = self.elements.iter().find(|e| pos > e.start && pos < e.end) {
            pos = if pos - e.start < e.end - pos {
                e.start
            } else {
                e.end
            };
        }
        self.cursor = pos;
    }
    fn elements(&self) -> Vec<Range<usize>> {
        self.elements.clone()
    }
    fn replace(&mut self, range: Range<usize>, with: &str) {
        let (start, end) = (range.start, range.end);
        self.text.replace_range(range, with);
        let diff = with.len() as isize - (end - start) as isize;
        self.elements
            .retain(|e| !(e.start >= start && e.end <= end));
        for e in &mut self.elements {
            if e.start >= end {
                e.start = (e.start as isize + diff) as usize;
                e.end = (e.end as isize + diff) as usize;
            }
        }
        self.cursor = if self.cursor < start {
            self.cursor
        } else if self.cursor <= end {
            start + with.len()
        } else {
            (self.cursor as isize + diff) as usize
        }
        .min(self.text.len());
    }
    fn prev_grapheme(&self, pos: usize) -> usize {
        self.text[..pos]
            .char_indices()
            .next_back()
            .map_or(0, |(i, _)| i)
    }
    fn next_grapheme(&self, pos: usize) -> usize {
        self.text[pos..]
            .chars()
            .next()
            .map_or(pos, |c| pos + c.len_utf8())
    }
    fn grapheme_ends(&self, s: &str) -> Vec<usize> {
        s.char_indices().map(|(i, c)| i + c.len_utf8()).collect()
    }
    fn register(&self) -> Register {
        self.register.clone()
    }
    fn store_register(&mut self, range: Range<usize>, kind: RegisterKind) {
        self.register = Register {
            text: self.text[range].to_string(),
            kind,
        };
    }
    fn set_register_kind(&mut self, kind: RegisterKind) {
        self.register.kind = kind;
    }
    fn paste_register_elements(&mut self, _start: usize) {}
    fn move_vertical(&mut self, down: bool) {
        let text = &self.text;
        let bol = text[..self.cursor].rfind('\n').map_or(0, |i| i + 1);
        let column = self.cursor - bol;
        let to = if down {
            match text[self.cursor..].find('\n') {
                Some(i) => {
                    let next = self.cursor + i + 1;
                    let end = text[next..].find('\n').map_or(text.len(), |j| next + j);
                    (next + column).min(end)
                }
                None => text.len(),
            }
        } else if bol == 0 {
            0
        } else {
            let prev = text[..bol - 1].rfind('\n').map_or(0, |i| i + 1);
            (prev + column).min(bol - 1)
        };
        self.cursor = to;
    }
    fn editor(&mut self, action: EditorAction) -> bool {
        match action {
            EditorAction::DeleteBackward if self.cursor > 0 => {
                let from = self.prev_grapheme(self.cursor);
                self.replace(from..self.cursor, "");
            }
            EditorAction::DeleteForward if self.cursor < self.text.len() => {
                let to = self.next_grapheme(self.cursor);
                self.replace(self.cursor..to, "");
            }
            EditorAction::DeleteBackwardWord if self.cursor > 0 => {
                let head = &self.text[..self.cursor];
                let word = head
                    .trim_end()
                    .trim_end_matches(|c: char| !c.is_whitespace());
                let from = word.len();
                self.replace(from..self.cursor, "");
            }
            EditorAction::MoveLeft => self.cursor = self.prev_grapheme(self.cursor),
            EditorAction::MoveRight => self.cursor = self.next_grapheme(self.cursor),
            _ => return false,
        }
        true
    }
}

/// A draft with Vim on: the keys of the oracle's tests, read by the default keymap.
struct Draft {
    t: Plain,
    vim: Vim,
    g: bool,
    query: String,
}

fn vim_draft(text: &str, cursor: usize) -> Draft {
    let mut vim = Vim::new();
    vim.set_enabled(true);
    Draft {
        t: Plain {
            text: text.into(),
            cursor,
            elements: Vec::new(),
            register: Register::default(),
        },
        vim,
        g: false,
        query: String::new(),
    }
}

impl Draft {
    fn text(&self) -> &str {
        &self.t.text
    }
    fn cursor(&self) -> usize {
        self.t.cursor
    }
    fn mode(&self) -> Option<&'static str> {
        self.vim.label()
    }

    fn keys(&mut self, keys: &str) {
        for key in keys.chars() {
            self.key(key);
        }
    }

    fn escape(&mut self) {
        self.g = false;
        if self.vim.searching().is_some() {
            self.query.clear();
            self.vim.abort_search();
        } else if self.vim.is_normal() {
            self.vim.input(&mut self.t, Input::Other);
        } else {
            self.vim.escape(&mut self.t);
        }
    }

    fn action(&mut self, action: EditorAction) {
        self.vim.insert_action(&mut self.t, action);
    }

    fn backspace(&mut self) {
        if !self.vim.replace_backspace(&mut self.t) {
            self.vim
                .insert_action(&mut self.t, EditorAction::DeleteBackward);
        }
    }

    fn key(&mut self, key: char) {
        if key == '\x1b' {
            return self.escape();
        }
        if self.vim.searching().is_some() {
            return self.search_key(key);
        }
        if !self.vim.is_normal() {
            let text = key.to_string();
            return self.vim.type_text(&mut self.t, &text);
        }
        let g = std::mem::take(&mut self.g);
        let pending = self.vim.pending();
        let input = match pending {
            VimPending::None => {
                if key == 'g' && !g {
                    self.g = true;
                    return;
                }
                if g && key == 'g' {
                    Input::Normal(Normal::JumpTop)
                } else {
                    normal_key(key)
                }
            }
            VimPending::Operator(op) => {
                if key == 'g' && !g {
                    self.g = true;
                    return;
                }
                if g && key == 'g' {
                    Input::Operator(OperatorKey::JumpTop)
                } else {
                    operator_key(op, key)
                }
            }
            VimPending::TextObject { .. } => object_key(key),
            VimPending::Replace | VimPending::Find { .. } => Input::Char(Some(key)),
        };
        match (pending, key) {
            (VimPending::None | VimPending::Operator(_), '/') => {
                self.vim.begin_search(VimDirection::Forward)
            }
            (VimPending::None | VimPending::Operator(_), '?') => {
                self.vim.begin_search(VimDirection::Backward)
            }
            (VimPending::None, 'n') => self.vim.repeat_search(&mut self.t, false),
            (VimPending::None, 'N') => self.vim.repeat_search(&mut self.t, true),
            (VimPending::Operator(_), 'n') => self.vim.repeat_search(&mut self.t, false),
            (VimPending::Operator(_), 'N') => self.vim.repeat_search(&mut self.t, true),
            _ => self.vim.input(&mut self.t, input),
        }
    }

    fn search_key(&mut self, key: char) {
        if key == '\n' {
            let query = std::mem::take(&mut self.query);
            self.vim.accept_search(&mut self.t, &query);
        } else {
            self.query.push(key);
        }
    }
}

fn normal_key(key: char) -> Input {
    let n = match key {
        'i' => Normal::EnterInsert,
        'a' => Normal::AppendAfterCursor,
        'A' => Normal::AppendLineEnd,
        'I' => Normal::InsertLineStart,
        'o' => Normal::OpenLineBelow,
        'O' => Normal::OpenLineAbove,
        'R' => Normal::EnterReplaceMode,
        'h' => Normal::MoveLeft,
        'l' => Normal::MoveRight,
        'k' => Normal::MoveUp,
        'j' => Normal::MoveDown,
        'w' => Normal::MoveWordForward,
        'b' => Normal::MoveWordBackward,
        'e' => Normal::MoveWordEnd,
        '0' => Normal::MoveLineStart,
        '$' => Normal::MoveLineEnd,
        'f' => Normal::Find(Find::Forward),
        'F' => Normal::Find(Find::Backward),
        't' => Normal::Find(Find::TillForward),
        'T' => Normal::Find(Find::TillBackward),
        'G' => Normal::JumpBottom,
        'x' => Normal::DeleteChar,
        'r' => Normal::ReplaceChar,
        '.' => Normal::RepeatLastChange,
        's' => Normal::SubstituteChar,
        'D' => Normal::DeleteToLineEnd,
        'C' => Normal::ChangeToLineEnd,
        'Y' => Normal::YankLine,
        'p' => Normal::PasteAfter,
        'd' => Normal::StartOperator(Operator::Delete),
        'y' => Normal::StartOperator(Operator::Yank),
        'c' => Normal::StartOperator(Operator::Change),
        _ => return Input::Other,
    };
    Input::Normal(n)
}

fn operator_key(op: Operator, key: char) -> Input {
    let k = match (op, key) {
        (Operator::Delete, 'd') | (Operator::Yank, 'y') | (Operator::Change, 'c') => {
            OperatorKey::Line
        }
        (_, 'i') => OperatorKey::VimScope(VimScope::Inner),
        (_, 'a') => OperatorKey::VimScope(VimScope::Around),
        (_, 'h') => OperatorKey::Motion(Motion::Left),
        (_, 'l') => OperatorKey::Motion(Motion::Right),
        (_, 'k') => OperatorKey::Motion(Motion::Up),
        (_, 'j') => OperatorKey::Motion(Motion::Down),
        (_, 'w') => OperatorKey::Motion(Motion::WordForward),
        (_, 'b') => OperatorKey::Motion(Motion::WordBackward),
        (_, 'e') => OperatorKey::Motion(Motion::WordEnd),
        (_, '0') => OperatorKey::Motion(Motion::LineStart),
        (_, '$') => OperatorKey::Motion(Motion::LineEnd),
        (_, 'f') => OperatorKey::Find(Find::Forward),
        (_, 'F') => OperatorKey::Find(Find::Backward),
        (_, 't') => OperatorKey::Find(Find::TillForward),
        (_, 'T') => OperatorKey::Find(Find::TillBackward),
        (_, 'G') => OperatorKey::JumpBottom,
        _ => return Input::Other,
    };
    Input::Operator(k)
}

fn object_key(key: char) -> Input {
    Input::Object(match key {
        'w' => Object::Word,
        'W' => Object::BigWord,
        '(' | ')' | 'b' => Object::Parentheses,
        '[' | ']' => Object::Brackets,
        '{' | '}' | 'B' => Object::Braces,
        '"' => Object::DoubleQuote,
        '\'' => Object::SingleQuote,
        '`' => Object::Backtick,
        _ => return Input::Other,
    })
}

#[test]
fn replace_character_keeps_normal_mode_and_a_pending_one_ends_on_escape() {
    let mut d = vim_draft("abc", 1);
    d.keys("rZ");
    assert_eq!((d.text(), d.cursor(), d.mode()), ("aZc", 1, Some("Normal")));
    d.keys("r");
    assert_eq!(d.vim.pending(), VimPending::Replace);
    assert!(d.vim.owns_escape());
    d.escape();
    assert_eq!((d.text(), d.vim.pending()), ("aZc", VimPending::None));
    assert!(!d.vim.owns_escape());
    d.keys("r\n");
    assert_eq!((d.text(), d.cursor()), ("a\nc", 2));
}

#[test]
fn replace_mode_overwrites_and_appends_at_the_line_end() {
    let mut d = vim_draft("abc\nnext", 1);
    d.keys("RXY!");
    assert_eq!(d.text(), "aXY!\nnext");
    assert_eq!(d.mode(), Some("Replace"));
    d.escape();
    assert_eq!((d.mode(), d.cursor()), (Some("Normal"), 3));

    let mut d = vim_draft("abc", 0);
    d.keys("R\nX");
    assert_eq!(d.text(), "\nXbc");

    let mut d = vim_draft("abc def", 0);
    d.keys("R/?nN");
    assert_eq!(d.text(), "/?nNdef");
}

#[test]
fn replace_mode_backspace_restores_original_text() {
    let mut d = vim_draft("ab", 0);
    d.keys("RXYZ");
    assert_eq!(d.text(), "XYZ");
    for expected in ["XY", "Xb", "ab", "ab"] {
        d.backspace();
        assert_eq!(d.text(), expected);
    }
    assert_eq!(d.cursor(), 0);

    let mut d = vim_draft("abcd", 0);
    d.keys("RXY");
    d.action(EditorAction::MoveLeft);
    d.backspace();
    assert_eq!(d.text(), "Ycd");

    let mut d = vim_draft("abe", 0);
    d.keys("RXY");
    d.action(EditorAction::DeleteBackwardWord);
    d.action(EditorAction::MoveRight);
    d.backspace();
    assert_eq!(d.text(), "");
    assert_eq!(d.mode(), Some("Replace"));
}

#[test]
fn replace_mode_replays_with_dot() {
    let mut d = vim_draft("abc def", 0);
    d.keys("RXY");
    d.backspace();
    d.escape();
    assert_eq!(d.text(), "Xbc def");
    d.keys("w.");
    assert_eq!(d.text(), "Xbc Xef");
    assert_eq!(d.mode(), Some("Normal"));
}

#[test]
fn repeat_replays_delete_replace_and_the_complete_change() {
    let mut d = vim_draft("alpha beta gamma", 0);
    d.keys("dw.");
    assert_eq!(d.text(), "gamma");

    let mut d = vim_draft("abc", 0);
    d.keys("rXl.");
    assert_eq!(d.text(), "XXc");

    let mut d = vim_draft("one two three", 0);
    d.keys("cwX");
    d.escape();
    d.keys("w.");
    assert_eq!(d.text(), "X X three");
    assert_eq!(d.mode(), Some("Normal"));
}

#[test]
fn repeat_records_a_pasted_insertion() {
    let mut d = vim_draft("", 0);
    d.keys("i");
    d.vim.type_text(&mut d.t, "foo");
    d.escape();
    d.keys(".");
    assert_eq!(d.text(), "fofooo");
}

#[test]
fn repeat_aborts_when_the_change_motion_cannot_start() {
    let mut d = vim_draft("one\ntwo\nthree", 0);
    d.keys("cjfoo");
    d.escape();
    d.keys("j");
    let original = (d.text().to_owned(), d.cursor());
    d.keys(".");
    assert_eq!((d.text(), d.cursor()), (original.0.as_str(), original.1));
    assert_eq!(d.mode(), Some("Normal"));
}

#[test]
fn repeat_tells_insert_mode_deletions_from_noops() {
    let mut d = vim_draft("abcd", 3);
    d.keys("i");
    d.backspace();
    d.escape();
    d.keys("l.");
    assert_eq!(d.text(), "ad");

    let mut d = vim_draft("abc", 0);
    d.keys("xi");
    d.backspace();
    d.escape();
    d.keys(".");
    assert_eq!(d.text(), "c");

    let mut d = vim_draft("one two", 0);
    d.keys("i");
    d.backspace();
    d.keys("X");
    d.escape();
    d.keys("w.");
    assert_eq!(d.text(), "Xone Xtwo");
}

#[test]
fn change_takes_a_word_a_line_and_the_edge_motions() {
    let mut d = vim_draft("hello world\nnext", 0);
    d.keys("cw");
    assert_eq!((d.text(), d.mode()), (" world\nnext", Some("Insert")));

    let mut d = vim_draft("hello world\nnext", 1);
    d.keys("c$");
    assert_eq!((d.text(), d.mode()), ("h\nnext", Some("Insert")));

    let mut d = vim_draft("first\nsecond\nthird", 8);
    d.keys("cc");
    assert_eq!(d.text(), "first\n\nthird");
    assert_eq!(d.cursor(), "first\n".len());
    d.keys("X");
    d.escape();
    d.keys("p");
    assert_eq!(d.text(), "first\nX\nsecond\nthird");

    for (text, cursor, command, expected) in [
        ("a b", 0, "cw", " b"),
        ("\nnext", 0, "cw", "\nnext"),
        ("one\ntwo\nthree", 0, "cj", "\nthree"),
        ("one\ntwo\nthree", 8, "ck", "one\n"),
        ("one\ntwo\n", 8, "ck", "one\n"),
        ("\nnext", 0, "c$", "\nnext"),
    ] {
        let mut d = vim_draft(text, cursor);
        d.keys(command);
        assert_eq!(
            (d.text(), d.mode()),
            (expected, Some("Insert")),
            "{command}"
        );
    }
    for (cursor, command) in [(0, "ck"), (4, "cj")] {
        let mut d = vim_draft("one\ntwo", cursor);
        d.keys(command);
        assert_eq!((d.text(), d.mode()), ("one\ntwo", Some("Normal")));
    }
}

#[test]
fn character_find_is_line_local_and_works_as_operator_motion() {
    let mut d = vim_draft("alpha beta\nbeta", 0);
    d.keys("fb");
    assert_eq!(d.cursor(), "alpha ".len());
    d.keys("Fa");
    assert_eq!(d.cursor(), "alpha".len() - 1);
    d.keys("fz");
    assert_eq!(d.cursor(), "alpha".len() - 1);

    for (motion, cursor, destination, range) in [
        ("fr", 0, 3, 0..4),
        ("tr", 0, 2, 0..3),
        ("Fb", 3, 1, 1..3),
        ("Tb", 3, 2, 2..3),
    ] {
        let text = "abcrdef";
        let mut d = vim_draft(text, cursor);
        d.keys(motion);
        assert_eq!((d.text(), d.cursor()), (text, destination));
        for operator in ['c', 'd', 'y'] {
            let mut d = vim_draft(text, cursor);
            d.keys(&format!("{operator}{motion}"));
            let expected = if operator == 'y' {
                text.to_owned()
            } else {
                format!("{}{}", &text[..range.start], &text[range.end..])
            };
            let mode = if operator == 'c' { "Insert" } else { "Normal" };
            assert_eq!(
                (d.text(), d.cursor(), d.mode(), d.register_text().as_str()),
                (
                    expected.as_str(),
                    range.start,
                    Some(mode),
                    &text[range.clone()]
                ),
                "{operator}{motion}",
            );
        }
    }

    let mut d = vim_draft("\nabc", 0);
    d.keys("fx");
    assert_eq!((d.text(), d.cursor()), ("\nabc", 0));
}

#[test]
fn find_and_till_handle_missing_cancelled_and_adjacent_targets() {
    for motion in ['f', 'F', 't', 'T'] {
        for operator in ["", "c", "d", "y"] {
            let mut d = vim_draft("abc\nz", 1);
            d.keys(&format!("{operator}{motion}z"));
            assert_eq!(
                (d.text(), d.cursor(), d.mode()),
                ("abc\nz", 1, Some("Normal")),
                "{operator}{motion}"
            );
            d.keys(&format!("{operator}{motion}"));
            d.escape();
            assert!(!d.vim.is_operator_pending());
        }
    }
    for (cursor, command, expected) in [(0, "ctb", "bc"), (2, "cTb", "abc")] {
        let mut d = vim_draft("abc", cursor);
        d.keys(command);
        assert_eq!((d.text(), d.mode()), (expected, Some("Insert")));
    }
}

#[test]
fn dot_replays_character_find_and_buffer_jump_operators() {
    let mut d = vim_draft("one:two:three", 0);
    d.keys("df:.");
    assert_eq!(d.text(), "three");

    for (cursor, command, repeat, expected) in [
        (0, "cfrX", "w.", "X X"),
        (0, "ctrX", "w.", "Xr Xr"),
        (3, "cFbX", "wlll.", "aXr aXr"),
        (3, "cTbX", "wlll.", "abXr abXr"),
    ] {
        let mut d = vim_draft("abcr abcr", cursor);
        d.keys(command);
        d.escape();
        d.keys(repeat);
        assert_eq!(
            (d.text(), d.mode()),
            (expected, Some("Normal")),
            "{command}"
        );
    }

    let mut d = vim_draft("one\ntwo\nthree\nfour\nfive", "one\n".len());
    d.keys("dggj.");
    assert_eq!(d.text(), "five");
}

#[test]
fn buffer_jumps_land_on_the_first_non_blank_and_take_operators() {
    let mut d = vim_draft("  first\n  second\n  third", 2);
    d.keys("G");
    assert_eq!(d.cursor(), "  first\n  second\n  ".len());
    d.keys("gg");
    assert_eq!(d.cursor(), 2);

    let mut d = vim_draft("first\nsecond\nthird", "first\n".len());
    d.keys("dG");
    assert_eq!(d.text(), "first\n");

    let mut d = vim_draft("first\nsecond\nthird", "first\n".len());
    d.keys("dgg");
    assert_eq!(d.text(), "third");

    let mut d = vim_draft("one\ntwo\nthree\nfour", "one\n".len());
    d.keys("dggp");
    assert_eq!(d.text(), "three\none\ntwo\nfour");

    let mut d = vim_draft("one\ntwo\nthree", "one\n".len());
    d.keys("yGp");
    assert_eq!(d.text(), "one\ntwo\ntwo\nthree\nthree");

    let mut d = vim_draft("one\ntwo\nthree", "one\n".len());
    d.keys("cG");
    assert_eq!(d.mode(), Some("Insert"));
}

#[test]
fn search_motions_compose_with_operators_and_repeat() {
    for (commands, text, cursor, mode) in [
        ("/b\n", "a b c b d b", 2, "Normal"),
        ("/b\nn", "a b c b d b", 6, "Normal"),
        ("/b\nnN", "a b c b d b", 2, "Normal"),
        ("/b\nnnn", "a b c b d b", 2, "Normal"),
        ("?b\n", "a b c b d b", 10, "Normal"),
        ("?b\nn", "a b c b d b", 6, "Normal"),
        ("?b\nN", "a b c b d b", 2, "Normal"),
        ("d/b\n", "b c b d b", 0, "Normal"),
        ("c/b\n", "b c b d b", 0, "Insert"),
        ("y/b\np", "aa  b c b d b", 3, "Normal"),
        ("/b\ndn", "a b d b", 2, "Normal"),
        ("?b\ndN", "a b", 2, "Normal"),
        ("?b\nd?b\n", "a b c b", 6, "Normal"),
        ("?b\ny?b\n", "a b c b d b", 6, "Normal"),
        ("d/missing\n", "a b c b d b", 0, "Normal"),
    ] {
        let mut d = vim_draft("a b c b d b", 0);
        d.keys(commands);
        assert_eq!(
            (d.text(), d.cursor(), d.mode()),
            (text, cursor, Some(mode)),
            "{commands:?}"
        );
    }
    for (cursor, commands, expected) in [
        (1, "d/xyz\n", "a\nxyz"),
        (0, "d/xyz\n", "xyz"),
        (0, "c/xyz\nQ\x1b", "Q\nxyz"),
        (1, "c/xyz\nQ\x1b", "aQ\nxyz"),
        (0, "y/xyz\np", "abc\nabc\nxyz"),
    ] {
        let mut d = vim_draft("abc\nxyz", cursor);
        d.keys(commands);
        assert_eq!(d.text(), expected, "{commands:?}");
    }
}

#[test]
fn a_search_skips_atomic_elements() {
    let mut d = vim_draft("[photo.png] x\n", 0);
    d.t.elements.push(0..11);
    d.keys("/photo\n");
    // the match starts inside the element: no command target, no move
    assert_eq!(d.cursor(), 0);
    d.keys("/x\n");
    assert_eq!(d.cursor(), "[photo.png] ".len());
}

#[test]
fn text_objects_take_words_pairs_and_quotes() {
    let mut d = vim_draft("call(foo, bar) now", 6);
    d.keys("di(");
    assert_eq!(d.text(), "call() now");
    let mut d = vim_draft("say \"hi there\" ok", 6);
    d.keys("ci\"X");
    assert_eq!(d.text(), "say \"X\" ok");
    let mut d = vim_draft("one two three", 5);
    d.keys("daw");
    assert_eq!(d.text(), "one three");
    let mut d = vim_draft("one two three", 5);
    d.keys("yiwP");
    assert_eq!(d.register_text(), "two");
}

impl Draft {
    fn register_text(&self) -> String {
        self.t.register.text.clone()
    }
}

#[test]
fn a_command_waiting_ends_with_an_unbound_key_and_the_buffer_swap_clears_it() {
    let mut d = vim_draft("one two", 0);
    d.keys("d");
    assert_eq!(d.vim.pending(), VimPending::Operator(Operator::Delete));
    d.keys("q");
    assert_eq!((d.vim.pending(), d.text()), (VimPending::None, "one two"));
    d.keys("x");
    d.vim.text_replaced();
    d.keys(".");
    assert_eq!(d.text(), "ne two", "no change left to repeat");
}

#[test]
fn x_p_and_the_line_kills_use_the_register() {
    let mut d = vim_draft("abc", 0);
    d.keys("xp");
    assert_eq!(d.text(), "bac");
    let mut d = vim_draft("one\ntwo", 0);
    d.keys("ddp");
    assert_eq!(d.text(), "two\none");
    let mut d = vim_draft("one\ntwo", 4);
    d.keys("Yp");
    assert_eq!(d.text(), "one\ntwo\ntwo");
}

#[test]
fn escape_from_insert_steps_one_back_and_o_opens_lines() {
    let mut d = vim_draft("ab\ncd", 2);
    d.keys("i");
    d.keys("X");
    d.escape();
    assert_eq!(
        (d.text(), d.cursor(), d.mode()),
        ("abX\ncd", 2, Some("Normal"))
    );
    d.keys("oY");
    d.escape();
    assert_eq!(d.text(), "abX\nY\ncd");
    d.keys("OZ");
    d.escape();
    assert_eq!(d.text(), "abX\nZ\nY\ncd");
}
