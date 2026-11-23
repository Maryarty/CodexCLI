//! The composer's modal keys: with Vim on, a key is read through the keymap's
//! Vim contexts (`vim_normal`, `vim_operator`, `vim_text_object`, `vim_search`)
//! into the engine's `Input` (`core::screens::composer::vim`), the way the
//! oracle's `handle_vim_*` ask the bindings, in the same order. The engine
//! edits through `VimHost` (the draft's elements stay whole); the draft's
//! whole-text undo history opens before a command that can edit and closes
//! when the command is done (`chat_composer/vim_history.rs`).
//!
//! Chords (`g g`) wait in the draft (`vim_chord`): the keymap says which
//! action a completed chord is.

use rtwk::editing::{StoredStops, TextEditingController, editing_key};
use rtwk::focus::KeyPress;
use rtwk::prelude::{KeyCode, Mods};

use codex_tui_core::keymap::actions::{
    VIM_NORMAL_APPEND_AFTER_CURSOR, VIM_NORMAL_APPEND_LINE_END, VIM_NORMAL_CANCEL_OPERATOR,
    VIM_NORMAL_CHANGE_TO_LINE_END, VIM_NORMAL_DELETE_CHAR, VIM_NORMAL_DELETE_TO_LINE_END,
    VIM_NORMAL_ENTER_INSERT, VIM_NORMAL_ENTER_REPLACE_MODE, VIM_NORMAL_FIND_BACKWARD,
    VIM_NORMAL_FIND_FORWARD, VIM_NORMAL_INSERT_LINE_START, VIM_NORMAL_JUMP_BOTTOM,
    VIM_NORMAL_JUMP_TOP, VIM_NORMAL_MOVE_DOWN, VIM_NORMAL_MOVE_LEFT, VIM_NORMAL_MOVE_LINE_END,
    VIM_NORMAL_MOVE_LINE_START, VIM_NORMAL_MOVE_RIGHT, VIM_NORMAL_MOVE_UP,
    VIM_NORMAL_MOVE_WORD_BACKWARD, VIM_NORMAL_MOVE_WORD_END, VIM_NORMAL_MOVE_WORD_FORWARD,
    VIM_NORMAL_OPEN_LINE_ABOVE, VIM_NORMAL_OPEN_LINE_BELOW, VIM_NORMAL_PASTE_AFTER,
    VIM_NORMAL_REDO, VIM_NORMAL_REPEAT_LAST_CHANGE, VIM_NORMAL_REPLACE_CHAR,
    VIM_NORMAL_START_CHANGE_OPERATOR, VIM_NORMAL_START_DELETE_OPERATOR,
    VIM_NORMAL_START_YANK_OPERATOR, VIM_NORMAL_SUBSTITUTE_CHAR, VIM_NORMAL_TILL_BACKWARD,
    VIM_NORMAL_TILL_FORWARD, VIM_NORMAL_UNDO, VIM_NORMAL_YANK_LINE, VIM_OPERATOR_CANCEL,
    VIM_OPERATOR_DELETE_LINE, VIM_OPERATOR_MOTION_DOWN, VIM_OPERATOR_MOTION_FIND_BACKWARD,
    VIM_OPERATOR_MOTION_FIND_FORWARD, VIM_OPERATOR_MOTION_JUMP_BOTTOM,
    VIM_OPERATOR_MOTION_JUMP_TOP, VIM_OPERATOR_MOTION_LEFT, VIM_OPERATOR_MOTION_LINE_END,
    VIM_OPERATOR_MOTION_LINE_START, VIM_OPERATOR_MOTION_RIGHT, VIM_OPERATOR_MOTION_TILL_BACKWARD,
    VIM_OPERATOR_MOTION_TILL_FORWARD, VIM_OPERATOR_MOTION_UP, VIM_OPERATOR_MOTION_WORD_BACKWARD,
    VIM_OPERATOR_MOTION_WORD_END, VIM_OPERATOR_MOTION_WORD_FORWARD,
    VIM_OPERATOR_SELECT_AROUND_TEXT_OBJECT, VIM_OPERATOR_SELECT_INNER_TEXT_OBJECT,
    VIM_OPERATOR_YANK_LINE, VIM_SEARCH_BACKWARD, VIM_SEARCH_FORWARD, VIM_SEARCH_NEXT,
    VIM_SEARCH_PREVIOUS, VIM_TEXT_OBJECT_BACKTICK, VIM_TEXT_OBJECT_BIG_WORD,
    VIM_TEXT_OBJECT_BRACES, VIM_TEXT_OBJECT_BRACKETS, VIM_TEXT_OBJECT_CANCEL,
    VIM_TEXT_OBJECT_DOUBLE_QUOTE, VIM_TEXT_OBJECT_PARENTHESES, VIM_TEXT_OBJECT_SINGLE_QUOTE,
    VIM_TEXT_OBJECT_WORD,
};
use codex_tui_core::keymap::{ActionId, Binding, Keymap};
use codex_tui_core::screens::composer::vim::{
    Direction, Find, Input, Motion, Normal, Object, Operator, OperatorKey, Pending, Scope, Vim,
};

use super::ComposerMsg;
use super::editing::{self, Edit, KeyData, Resolved};
use crate::keymap::{combo_of, pressed};
use crate::widgets::composer_draft::{ComposerDraft, VimHost};

/// What the engine is doing, read once per key.
pub struct Mode {
    pub enabled: bool,
    pub normal: bool,
    pub replace: bool,
    pub pending: Pending,
    pub searching: bool,
}

pub fn mode(draft: &ComposerDraft) -> Mode {
    draft.with_vim(|v| Mode {
        enabled: v.is_enabled(),
        normal: v.is_normal(),
        replace: v.is_replace(),
        pending: v.pending(),
        searching: v.searching().is_some(),
    })
}

/// The engine run on the draft's text, its editor actions carried out as the
/// composer edits.
fn run<R>(
    data: &KeyData,
    c: &TextEditingController,
    f: impl FnOnce(&mut Vim, &mut VimHost<'_>) -> R,
) -> R {
    let editor = |action| editing::carry_out(data, c, action);
    let mut host = VimHost::with_editor(c, &data.draft, &editor);
    data.draft.with_vim(|v| f(v, &mut host))
}

/// `toggle_vim_mode`: on enters Normal mode, off Insert; what the old mode
/// recorded and any query are gone.
pub fn toggle(data: &KeyData) {
    let on = !data.draft.vim_enabled();
    data.draft.set_vim_enabled(on);
    (data.tell)(ComposerMsg::VimToggled(on));
}

// ---- reading a key ----------------------------------------------------------

const NORMAL: [(ActionId, Normal); 34] = [
    (VIM_NORMAL_ENTER_INSERT, Normal::EnterInsert),
    (VIM_NORMAL_APPEND_AFTER_CURSOR, Normal::AppendAfterCursor),
    (VIM_NORMAL_APPEND_LINE_END, Normal::AppendLineEnd),
    (VIM_NORMAL_INSERT_LINE_START, Normal::InsertLineStart),
    (VIM_NORMAL_OPEN_LINE_BELOW, Normal::OpenLineBelow),
    (VIM_NORMAL_OPEN_LINE_ABOVE, Normal::OpenLineAbove),
    (VIM_NORMAL_MOVE_LEFT, Normal::MoveLeft),
    (VIM_NORMAL_MOVE_RIGHT, Normal::MoveRight),
    (VIM_NORMAL_MOVE_DOWN, Normal::MoveDown),
    (VIM_NORMAL_MOVE_UP, Normal::MoveUp),
    (VIM_NORMAL_MOVE_WORD_FORWARD, Normal::MoveWordForward),
    (VIM_NORMAL_MOVE_WORD_BACKWARD, Normal::MoveWordBackward),
    (VIM_NORMAL_MOVE_WORD_END, Normal::MoveWordEnd),
    (VIM_NORMAL_MOVE_LINE_START, Normal::MoveLineStart),
    (VIM_NORMAL_MOVE_LINE_END, Normal::MoveLineEnd),
    (VIM_NORMAL_DELETE_CHAR, Normal::DeleteChar),
    (VIM_NORMAL_SUBSTITUTE_CHAR, Normal::SubstituteChar),
    (VIM_NORMAL_DELETE_TO_LINE_END, Normal::DeleteToLineEnd),
    (VIM_NORMAL_CHANGE_TO_LINE_END, Normal::ChangeToLineEnd),
    (VIM_NORMAL_YANK_LINE, Normal::YankLine),
    (VIM_NORMAL_PASTE_AFTER, Normal::PasteAfter),
    (
        VIM_NORMAL_START_DELETE_OPERATOR,
        Normal::StartOperator(Operator::Delete),
    ),
    (
        VIM_NORMAL_START_YANK_OPERATOR,
        Normal::StartOperator(Operator::Yank),
    ),
    (
        VIM_NORMAL_START_CHANGE_OPERATOR,
        Normal::StartOperator(Operator::Change),
    ),
    (VIM_NORMAL_CANCEL_OPERATOR, Normal::CancelOperator),
    (VIM_NORMAL_ENTER_REPLACE_MODE, Normal::EnterReplaceMode),
    (VIM_NORMAL_REPLACE_CHAR, Normal::ReplaceChar),
    (VIM_NORMAL_REPEAT_LAST_CHANGE, Normal::RepeatLastChange),
    (VIM_NORMAL_FIND_FORWARD, Normal::Find(Find::Forward)),
    (VIM_NORMAL_FIND_BACKWARD, Normal::Find(Find::Backward)),
    (VIM_NORMAL_TILL_FORWARD, Normal::Find(Find::TillForward)),
    (VIM_NORMAL_TILL_BACKWARD, Normal::Find(Find::TillBackward)),
    (VIM_NORMAL_JUMP_TOP, Normal::JumpTop),
    (VIM_NORMAL_JUMP_BOTTOM, Normal::JumpBottom),
];

const MOTIONS: [(ActionId, Motion); 9] = [
    (VIM_OPERATOR_MOTION_LEFT, Motion::Left),
    (VIM_OPERATOR_MOTION_RIGHT, Motion::Right),
    (VIM_OPERATOR_MOTION_DOWN, Motion::Down),
    (VIM_OPERATOR_MOTION_UP, Motion::Up),
    (VIM_OPERATOR_MOTION_WORD_FORWARD, Motion::WordForward),
    (VIM_OPERATOR_MOTION_WORD_BACKWARD, Motion::WordBackward),
    (VIM_OPERATOR_MOTION_WORD_END, Motion::WordEnd),
    (VIM_OPERATOR_MOTION_LINE_START, Motion::LineStart),
    (VIM_OPERATOR_MOTION_LINE_END, Motion::LineEnd),
];

/// The operator context's find and jump keys, asked after the motions.
const OPERATOR_COMMANDS: [(ActionId, OperatorKey); 6] = [
    (
        VIM_OPERATOR_MOTION_FIND_FORWARD,
        OperatorKey::Find(Find::Forward),
    ),
    (
        VIM_OPERATOR_MOTION_FIND_BACKWARD,
        OperatorKey::Find(Find::Backward),
    ),
    (
        VIM_OPERATOR_MOTION_TILL_FORWARD,
        OperatorKey::Find(Find::TillForward),
    ),
    (
        VIM_OPERATOR_MOTION_TILL_BACKWARD,
        OperatorKey::Find(Find::TillBackward),
    ),
    (VIM_OPERATOR_MOTION_JUMP_TOP, OperatorKey::JumpTop),
    (VIM_OPERATOR_MOTION_JUMP_BOTTOM, OperatorKey::JumpBottom),
];

const OBJECTS: [(ActionId, Object); 8] = [
    (VIM_TEXT_OBJECT_WORD, Object::Word),
    (VIM_TEXT_OBJECT_BIG_WORD, Object::BigWord),
    (VIM_TEXT_OBJECT_PARENTHESES, Object::Parentheses),
    (VIM_TEXT_OBJECT_BRACKETS, Object::Brackets),
    (VIM_TEXT_OBJECT_BRACES, Object::Braces),
    (VIM_TEXT_OBJECT_DOUBLE_QUOTE, Object::DoubleQuote),
    (VIM_TEXT_OBJECT_SINGLE_QUOTE, Object::SingleQuote),
    (VIM_TEXT_OBJECT_BACKTICK, Object::Backtick),
];

/// Whether `key` is the action's: one of its single keys, or the end of a
/// chord whose first key was `prefix`.
fn fires(keymap: &Keymap, id: ActionId, key: &KeyPress, prefix: Option<&KeyPress>) -> bool {
    if pressed(keymap, id, key) {
        return true;
    }
    let Some(prefix) = prefix else {
        return false;
    };
    keymap.bindings(id).iter().any(|binding| match binding {
        Binding::Chord {
            prefix: first,
            completion,
        } => {
            combo_of(first).is_some_and(|combo| combo.accepts_press(prefix))
                && combo_of(completion).is_some_and(|combo| combo.accepts_press(key))
        }
        Binding::Single(_) => false,
    })
}

/// Whether `key` is the first key of a chord of one of `ids`.
fn starts_chord(keymap: &Keymap, ids: impl IntoIterator<Item = ActionId>, key: &KeyPress) -> bool {
    ids.into_iter().any(|id| {
        keymap.bindings(id).iter().any(|binding| match binding {
            Binding::Chord { prefix, .. } => {
                combo_of(prefix).is_some_and(|combo| combo.accepts_press(key))
            }
            Binding::Single(_) => false,
        })
    })
}

/// What a key is to the engine.
enum Read {
    Input(Input),
    /// The first key of a chord: wait for the second.
    Prefix,
    /// No command is bound to it.
    Unbound,
}

/// The character a pending `r`, `f`, `t` takes (`vim_command_char`): Enter is
/// a line break, a typed key its character; no modifier but Shift.
fn command_char(key: &KeyPress) -> Option<char> {
    if key.code == KeyCode::Enter {
        return Some('\n');
    }
    if key.code != KeyCode::Char || (key.mods != Mods::NONE && key.mods != Mods::SHIFT) {
        return None;
    }
    key.ch
}

fn read(keymap: &Keymap, pending: Pending, key: &KeyPress, prefix: Option<&KeyPress>) -> Read {
    let fires = |id| fires(keymap, id, key, prefix);
    match pending {
        Pending::None => {
            for (id, command) in NORMAL {
                if fires(id) {
                    return Read::Input(Input::Normal(command));
                }
            }
            if starts_chord(keymap, NORMAL.iter().map(|(id, _)| *id), key) {
                return Read::Prefix;
            }
            Read::Unbound
        }
        Pending::Operator(operator) => {
            if operator == Operator::Delete && fires(VIM_OPERATOR_DELETE_LINE)
                || operator == Operator::Yank && fires(VIM_OPERATOR_YANK_LINE)
            {
                return Read::Input(Input::Operator(OperatorKey::Line));
            }
            if fires(VIM_OPERATOR_CANCEL) {
                return Read::Input(Input::Operator(OperatorKey::Cancel));
            }
            if fires(VIM_OPERATOR_SELECT_INNER_TEXT_OBJECT) {
                return Read::Input(Input::Operator(OperatorKey::Scope(Scope::Inner)));
            }
            if fires(VIM_OPERATOR_SELECT_AROUND_TEXT_OBJECT) {
                return Read::Input(Input::Operator(OperatorKey::Scope(Scope::Around)));
            }
            for (id, motion) in MOTIONS {
                if fires(id) {
                    return Read::Input(Input::Operator(OperatorKey::Motion(motion)));
                }
            }
            if operator == Operator::Change && fires(VIM_NORMAL_START_CHANGE_OPERATOR) {
                return Read::Input(Input::Operator(OperatorKey::Line));
            }
            for (id, command) in OPERATOR_COMMANDS {
                if fires(id) {
                    return Read::Input(Input::Operator(command));
                }
            }
            if starts_chord(keymap, OPERATOR_COMMANDS.iter().map(|(id, _)| *id), key) {
                return Read::Prefix;
            }
            Read::Unbound
        }
        Pending::TextObject { .. } => {
            if fires(VIM_TEXT_OBJECT_CANCEL) {
                return Read::Input(Input::Cancel);
            }
            for (id, object) in OBJECTS {
                if fires(id) {
                    return Read::Input(Input::Object(object));
                }
            }
            Read::Unbound
        }
        Pending::Replace | Pending::Find { .. } => Read::Input(Input::Char(command_char(key))),
    }
}

/// A command that moves the caret or yanks, and so edits nothing: the draft's
/// history does not open for it (`begin_vim_edit`).
fn is_pure(command: Normal) -> bool {
    matches!(
        command,
        Normal::MoveLeft
            | Normal::MoveRight
            | Normal::MoveUp
            | Normal::MoveDown
            | Normal::MoveWordForward
            | Normal::MoveWordBackward
            | Normal::MoveWordEnd
            | Normal::MoveLineStart
            | Normal::MoveLineEnd
            | Normal::Find(Find::Forward | Find::Backward)
            | Normal::JumpTop
            | Normal::JumpBottom
            | Normal::YankLine
            | Normal::StartOperator(Operator::Yank)
            | Normal::CancelOperator
    )
}

/// A key that is typed text: a character with no modifier but Shift.
fn typed_char(key: &KeyPress) -> Option<char> {
    if key.code == KeyCode::Char && (key.mods == Mods::NONE || key.mods == Mods::SHIFT) {
        key.ch
    } else {
        None
    }
}

// ---- Normal mode ------------------------------------------------------------

/// `/` on an empty draft is the slash command's: Normal mode goes into Insert mode and types it
/// (the search key of Vim is for a draft with text).
pub fn opens_slash(data: &KeyData, c: &TextEditingController, key: &KeyPress, mode: &Mode) -> bool {
    mode.normal
        && mode.pending == Pending::None
        && !data.suggest.open()
        && c.text().is_empty()
        && key.code == KeyCode::Char
        && key.ch == Some('/')
        && key.mods == Mods::NONE
        && pressed(&data.keymap, VIM_SEARCH_FORWARD, key)
}

/// The keys Normal mode answers before anything else of the composer: the
/// `/` query starts (and `n`, `N`), `u` and Ctrl+R go through the draft's
/// history. `None`: not one of them.
pub fn early(
    data: &KeyData,
    c: &TextEditingController,
    key: &KeyPress,
    mode: &Mode,
) -> Option<bool> {
    let keymap = &*data.keymap;
    let draft = &data.draft;
    if matches!(mode.pending, Pending::None | Pending::Operator(_)) {
        if opens_slash(data, c, key, mode) {
            draft.vim_begin_edit(c);
            run(data, c, |v, host| v.enter_insert(host));
            (data.tell)(ComposerMsg::Redraw);
            return None;
        }
        let direction = if pressed(keymap, VIM_SEARCH_FORWARD, key) {
            Some(Direction::Forward)
        } else if pressed(keymap, VIM_SEARCH_BACKWARD, key) {
            Some(Direction::Backward)
        } else {
            None
        };
        if let Some(direction) = direction {
            draft.vim_query().clear();
            draft.with_vim(|v| v.begin_search(direction));
            (data.tell)(ComposerMsg::Redraw);
            return Some(true);
        }
        for (id, reverse) in [(VIM_SEARCH_NEXT, false), (VIM_SEARCH_PREVIOUS, true)] {
            if pressed(keymap, id, key) {
                run(data, c, |v, host| v.repeat_search(host, reverse));
                finish(data, c);
                (data.tell)(ComposerMsg::Redraw);
                return Some(true);
            }
        }
    }
    if mode.pending == Pending::None && !data.suggest.open() {
        let redo = pressed(keymap, VIM_NORMAL_REDO, key);
        if redo || pressed(keymap, VIM_NORMAL_UNDO, key) {
            draft.vim_step(c, redo);
            (data.tell)(ComposerMsg::Redraw);
            return Some(true);
        }
    }
    None
}

/// The draft's history step closes when Normal mode has nothing waiting.
fn finish(data: &KeyData, c: &TextEditingController) {
    let after = mode(&data.draft);
    if after.normal && after.pending == Pending::None && !after.searching {
        data.draft.vim_finish_edit(c);
    }
}

/// Normal mode's key (the key bound to nothing is not taken unless it is a
/// typed character: nothing is typed in Normal mode). `true`: taken.
pub fn normal(data: &KeyData, c: &TextEditingController, key: &KeyPress, mode: &Mode) -> bool {
    let draft = &data.draft;
    let prefix = draft.vim_chord();
    draft.set_vim_chord(None);
    // `!` on an empty draft is shell mode (`shell_commands_enabled`)
    if mode.pending == Pending::None
        && prefix.is_none()
        && c.text().is_empty()
        && key.code == KeyCode::Char
        && key.ch == Some('!')
        && key.mods == Mods::NONE
    {
        draft.set_shell(true);
        // the command is typed next: Insert mode (`enter_vim_insert_mode`)
        run(data, c, |v, host| v.enter_insert(host));
        (data.tell)(ComposerMsg::Shell);
        return true;
    }
    // Backspace at the start of a shell draft takes the `!` away, as its own
    // undo step (the composer's key, Normal mode or not)
    if mode.pending == Pending::None
        && key.code == KeyCode::Backspace
        && key.mods == Mods::NONE
        && draft.shell()
        && c.selection().is_collapsed()
        && c.selection().extent_offset == 0
    {
        draft.vim_begin_edit(c);
        draft.set_shell(false);
        (data.tell)(ComposerMsg::Shell);
        finish(data, c);
        return true;
    }
    let input = match read(&data.keymap, mode.pending, key, prefix.as_ref()) {
        Read::Input(input) => input,
        Read::Prefix => {
            draft.set_vim_chord(Some(key.clone()));
            (data.tell)(ComposerMsg::Redraw);
            return true;
        }
        Read::Unbound => {
            // a pending command ends on a key it does not know; a key that
            // is the page's (Esc, Ctrl+T) is left to it
            if mode.pending == Pending::None {
                let ours = typed_char(key).is_some()
                    || matches!(
                        key.code,
                        KeyCode::Enter | KeyCode::Backspace | KeyCode::Delete | KeyCode::Tab
                    );
                if !ours {
                    return false;
                }
            }
            Input::Other
        }
    };
    // Up and Down at the draft's ends walk the prompt history
    if let (Pending::None, Input::Normal(command @ (Normal::MoveUp | Normal::MoveDown))) =
        (mode.pending, input)
    {
        let took = super::editing::walk_history(data, c, command == Normal::MoveUp);
        if took {
            (data.tell)(ComposerMsg::Redraw);
            return true;
        }
    }
    let opens = match (mode.pending, input) {
        (Pending::None, Input::Normal(command)) => !is_pure(command),
        (Pending::None, _) => true,
        _ => false,
    };
    if opens {
        draft.vim_begin_edit(c);
    }
    run(data, c, |v, host| v.input(host, input));
    finish(data, c);
    (data.tell)(ComposerMsg::Redraw);
    true
}

// ---- the `/` query ----------------------------------------------------------

/// A key while the query is typed: Enter searches, Escape, Ctrl+C and
/// Backspace on an empty query drop it, any other key edits it (a field of
/// its own, the editor keymap's keys).
pub fn search_key(data: &KeyData, c: &TextEditingController, key: &KeyPress) -> bool {
    let draft = &data.draft;
    let query = draft.vim_query();
    let plain = key.mods == Mods::NONE;
    let abort = key.code == KeyCode::Escape
        || (key.code == KeyCode::Char && key.ch == Some('c') && key.mods == Mods::CTRL)
        || (key.code == KeyCode::Backspace && plain && query.text().is_empty());
    if abort {
        query.clear();
        draft.with_vim(Vim::abort_search);
    } else if key.code == KeyCode::Enter && plain {
        let text = query.text().to_string();
        query.clear();
        run(data, c, |v, host| v.accept_search(host, &text));
        finish(data, c);
    } else {
        editing_key(
            &query,
            key.clone(),
            &|text| query.insert(text),
            &StoredStops,
            false,
            None,
        );
    }
    (data.tell)(ComposerMsg::Redraw);
    true
}

// ---- Insert and Replace mode ------------------------------------------------

/// Insert and Replace mode's keys the engine records or owns: Escape back to
/// Normal, Backspace restoring what Replace overwrote, the editor's actions
/// and typed text (Replace overwrites). `None`: the composer's usual keys.
pub fn insert(
    data: &KeyData,
    c: &TextEditingController,
    key: &KeyPress,
    mode: &Mode,
) -> Option<bool> {
    let draft = &data.draft;
    draft.vim_begin_edit(c);
    let plain = key.mods == Mods::NONE;
    if key.code == KeyCode::Escape && plain {
        // shell mode's Esc on an empty draft is shell mode's
        if draft.shell() && c.text().is_empty() {
            return None;
        }
        run(data, c, |v, host| v.escape(host));
        finish(data, c);
        (data.tell)(ComposerMsg::Redraw);
        return Some(true);
    }
    let resolved = editing::resolve(&data.keymap, key);
    if mode.replace
        && matches!(
            resolved,
            Resolved::Edit {
                edit: Edit::DeleteBackward,
                ..
            }
        )
        && c.selection().is_collapsed()
        && run(data, c, |v, host| v.replace_backspace(host))
    {
        return Some(true);
    }
    match resolved {
        Resolved::Edit {
            edit: Edit::Newline,
            ..
        } => {
            // Shift+Enter and Alt+Enter are the field's own line break
            if key.code == KeyCode::Enter
                && (key.mods.contains(Mods::SHIFT) || key.mods.contains(Mods::ALT))
            {
                return None;
            }
            run(data, c, |v, host| v.type_text(host, "\n"));
            Some(true)
        }
        Resolved::Edit { edit, .. } => {
            let action = edit.action()?;
            run(data, c, |v, host| v.insert_action(host, action)).then_some(true)
        }
        Resolved::None if mode.replace => {
            let ch = typed_char(key)?;
            run(data, c, |v, host| v.type_text(host, &ch.to_string()));
            Some(true)
        }
        _ => None,
    }
}

/// What the field typed on its own in Insert mode is for `.` to repeat.
pub fn typed(data: &KeyData, key: &KeyPress) {
    let text = match (key.code, typed_char(key)) {
        (_, Some(ch)) => ch.to_string(),
        (KeyCode::Enter, _) => "\n".to_string(),
        _ => return,
    };
    data.draft.with_vim(|v| v.record_text(&text));
}

/// A paste while the `/` query is typed: into the query. `false`: not typing one.
pub fn paste_into_query(draft: &ComposerDraft, text: &str) -> bool {
    if !draft.vim_enabled() || !mode(draft).searching {
        return false;
    }
    draft.vim_query().insert(text);
    true
}

/// A paste went into the draft: Insert mode records it for `.`, Normal mode's
/// edit is done.
pub fn pasted(draft: &ComposerDraft, c: &TextEditingController, text: &str) {
    if !draft.vim_enabled() {
        return;
    }
    draft.with_vim(|v| v.record_text(text));
    let now = mode(draft);
    if now.normal && now.pending == Pending::None {
        draft.vim_finish_edit(c);
    }
}
