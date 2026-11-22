//! The composer's own editing keys, asked before the `TextArea`'s through
//! `EditingKeys` (rtwk's door for an app's keys): the atomic elements
//! (Backspace, Delete, Left and Right take a placeholder whole), the kill
//! ring (`Ctrl+K`, `Ctrl+U`, `Ctrl+W` / `Alt+Backspace`, `Alt+D` kill,
//! `Ctrl+Y` yanks) and prompt history (Up and Down at the draft's ends).
//! Every other key is the field's. Oracle: `bottom_pane/textarea.rs`
//! (`input_with_keymap`, the default editor keymap of `keymap.rs`) and
//! `chat_composer_history.rs`.

use std::rc::Rc;

use rtwk::editing::{KeyCx, StoredStops, TextEditingController, TextSelection, editing_key};
use rtwk::focus::KeyPress;
use rtwk::prelude::{KeyCode, Mods};

use codex_tui_core::keymap::actions::{
    COMPOSER_QUEUE, COMPOSER_SUBMIT, EDITOR_DELETE_BACKWARD, EDITOR_DELETE_BACKWARD_WORD,
    EDITOR_DELETE_FORWARD, EDITOR_DELETE_FORWARD_WORD, EDITOR_INSERT_NEWLINE, EDITOR_KILL_LINE_END,
    EDITOR_KILL_LINE_START, EDITOR_KILL_WHOLE_LINE, EDITOR_MOVE_DOWN, EDITOR_MOVE_LEFT,
    EDITOR_MOVE_LINE_END, EDITOR_MOVE_LINE_START, EDITOR_MOVE_RIGHT, EDITOR_MOVE_UP,
    EDITOR_MOVE_WORD_LEFT, EDITOR_MOVE_WORD_RIGHT, EDITOR_YANK, GLOBAL_TOGGLE_VIM_MODE,
};
use codex_tui_core::keymap::{ActionId, Keymap};
use codex_tui_core::screens::Shared;
use codex_tui_core::screens::composer::history::Store;
use codex_tui_core::screens::composer::vim::{EditorAction, Pending};

use super::ComposerMsg;
use super::burst::{self, BurstEnv};
use super::search;
use super::suggest::Suggest;
use super::vim;
use crate::keymap::pressed;
use crate::widgets::composer_draft::ComposerDraft;

/// What the handler reads besides the controller: the draft of the same
/// thread, the prompts history (newest last), the composer to tell of a
/// key that is not an edit, and what the paste-burst detector needs of the
/// app (none outside an app: no burst).
#[derive(Clone)]
pub struct KeyData {
    pub draft: ComposerDraft,
    /// The message history file as far as it was read (shared, a handle).
    pub history: Shared<Store>,
    pub tell: Rc<dyn Fn(ComposerMsg)>,
    pub burst: Option<BurstEnv>,
    /// The popup's rows: it takes Up, Down, Enter and Tab while it shows.
    pub suggest: Rc<Suggest>,
    /// The user's bindings (`tui.keymap`): the Composer and Editor contexts
    /// say which key is which edit.
    pub keymap: Rc<Keymap>,
}

/// Up (`up`) or Down at the draft's ends: the history's entry replaces the
/// draft (`navigate_up`, `navigate_down`). A read the store lacks goes to
/// the app, and the draft waits for it. `false`: the caret moves instead.
pub(super) fn walk_history(data: &KeyData, c: &TextEditingController, up: bool) -> bool {
    let draft = &data.draft;
    let shell = draft.shell();
    let mut asks = Vec::new();
    let took = draft.history_step(c, &data.history, up, &mut asks);
    if !asks.is_empty() {
        (data.tell)(ComposerMsg::Lookups(asks));
    }
    // an entry's leading `!` is shell mode again
    if draft.shell() != shell {
        (data.tell)(ComposerMsg::Shell);
    }
    took
}

fn only(key: &KeyPress, mods: Mods) -> bool {
    key.mods == mods
}

/// Whether the composer takes `key` for `c` before the field: the modal
/// (Vim) keys that precede everything, the paste-burst detector (keys that
/// arrive as fast as a paste are held), then the composer's own editing keys.
pub fn take_key(data: &KeyData, c: &TextEditingController, key: &KeyPress, kcx: KeyCx) -> bool {
    if kcx.read_only {
        return false;
    }
    if pressed(&data.keymap, GLOBAL_TOGGLE_VIM_MODE, key) {
        vim::toggle(data);
        return true;
    }
    // reverse history search owns every key while it is open
    if data.draft.searching() {
        return search::key(data, c, key);
    }
    let mode = vim::mode(&data.draft);
    if mode.searching {
        return vim::search_key(data, c, key);
    }
    if mode.normal
        && let Some(took) = vim::early(data, c, key, &mode)
    {
        return took;
    }
    // a `$skill` completion's space lasts for the next key only
    let skill_space = data.draft.take_skill_space();
    // `/` is a search in Normal mode, not a command's name being typed
    if !mode.normal && data.suggest.take_slash(c, key, &*data.tell) {
        return true;
    }
    // Normal mode's keys are commands: nothing is typed text to hold back
    if !mode.normal
        && let Some(env) = &data.burst
        && burst::take(data, env, c, key, skill_space)
    {
        return true;
    }
    // a popup with rows takes the keys of its list before the editing keys
    // (history on Up and Down, the queue key on Tab)
    if data.suggest.take_key(c, key, &*data.tell) {
        return true;
    }
    // Ctrl+R opens the search (Vim's Normal mode keeps it for redo)
    if !mode.normal && search::opens(data, key) {
        search::begin(data, c);
        return true;
    }
    // a command waiting for its next key takes Enter too (it is no submit)
    let waiting = mode.normal && mode.pending != Pending::None;
    if !waiting && let Some(took) = composer_key(data, c, key) {
        return took;
    }
    if mode.normal {
        return vim::normal(data, c, key, &mode);
    }
    if mode.enabled
        && let Some(took) = vim::insert(data, c, key, &mode)
    {
        return took;
    }
    let took = edit_key(data, c, key, skill_space);
    if !took && mode.enabled {
        vim::typed(data, key);
    }
    took
}

/// The Composer-context actions a key may be (`composer.submit`,
/// `composer.queue`): Enter and Tab are the field's and the page's own, so a
/// key bound elsewhere reaches them as a message, and an unbound Tab is
/// nothing (the oracle's `handle_key_event_without_popup`). `None`: not a
/// Composer key; `Some(false)`: the field's or the page's own key, left to
/// them (the Editor context does not see it: Composer shadows it).
fn composer_key(data: &KeyData, c: &TextEditingController, key: &KeyPress) -> Option<bool> {
    let keymap = &*data.keymap;
    let plain = |code: KeyCode| key.code == code && key.mods == Mods::NONE;
    if pressed(keymap, COMPOSER_SUBMIT, key) {
        if plain(KeyCode::Enter) {
            return Some(false);
        }
        (data.tell)(ComposerMsg::Submitted(c.text()));
        return Some(true);
    }
    if pressed(keymap, COMPOSER_QUEUE, key) {
        if plain(KeyCode::Tab) {
            return Some(false);
        }
        (data.tell)(ComposerMsg::Queue);
        return Some(true);
    }
    plain(KeyCode::Tab).then_some(true)
}

/// The Editor-context actions the composer carries out itself, each with the
/// key its editing is written for (`canonical`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Edit {
    Newline,
    Left,
    Right,
    Up,
    Down,
    WordLeft,
    WordRight,
    LineStart,
    LineEnd,
    DeleteBackward,
    DeleteForward,
    DeleteBackwardWord,
    DeleteForwardWord,
    KillLineStart,
    KillWholeLine,
    KillLineEnd,
    Yank,
}

const EDITS: [(Edit, ActionId); 17] = [
    (Edit::Newline, EDITOR_INSERT_NEWLINE),
    (Edit::Left, EDITOR_MOVE_LEFT),
    (Edit::Right, EDITOR_MOVE_RIGHT),
    (Edit::Up, EDITOR_MOVE_UP),
    (Edit::Down, EDITOR_MOVE_DOWN),
    (Edit::WordLeft, EDITOR_MOVE_WORD_LEFT),
    (Edit::WordRight, EDITOR_MOVE_WORD_RIGHT),
    (Edit::LineStart, EDITOR_MOVE_LINE_START),
    (Edit::LineEnd, EDITOR_MOVE_LINE_END),
    (Edit::DeleteBackward, EDITOR_DELETE_BACKWARD),
    (Edit::DeleteForward, EDITOR_DELETE_FORWARD),
    (Edit::DeleteBackwardWord, EDITOR_DELETE_BACKWARD_WORD),
    (Edit::DeleteForwardWord, EDITOR_DELETE_FORWARD_WORD),
    (Edit::KillLineStart, EDITOR_KILL_LINE_START),
    (Edit::KillWholeLine, EDITOR_KILL_WHOLE_LINE),
    (Edit::KillLineEnd, EDITOR_KILL_LINE_END),
    (Edit::Yank, EDITOR_YANK),
];

impl Edit {
    /// The key the field's own editing answers to for this action: a key
    /// the user bound to it is carried out as this one.
    fn canonical(self) -> Option<KeyPress> {
        let ctrl = |ch: char| KeyPress::char(ch).with_mods(Mods::CTRL);
        let alt = |code: KeyCode| KeyPress::code(code).with_mods(Mods::ALT);
        Some(match self {
            Edit::Newline | Edit::KillWholeLine => return None,
            Edit::Left => KeyPress::code(KeyCode::Left),
            Edit::Right => KeyPress::code(KeyCode::Right),
            Edit::Up => KeyPress::code(KeyCode::Up),
            Edit::Down => KeyPress::code(KeyCode::Down),
            Edit::WordLeft => alt(KeyCode::Left),
            Edit::WordRight => alt(KeyCode::Right),
            Edit::LineStart => KeyPress::code(KeyCode::Home),
            Edit::LineEnd => KeyPress::code(KeyCode::End),
            Edit::DeleteBackward => KeyPress::code(KeyCode::Backspace),
            Edit::DeleteForward => KeyPress::code(KeyCode::Delete),
            Edit::DeleteBackwardWord => ctrl('w'),
            Edit::DeleteForwardWord => KeyPress::char('d').with_mods(Mods::ALT),
            Edit::KillLineStart => ctrl('u'),
            Edit::KillLineEnd => ctrl('k'),
            Edit::Yank => ctrl('y'),
        })
    }
}

/// What a key is to the composer's editing under the user's bindings.
pub(super) enum Resolved {
    /// Not an editing key: the field's.
    None,
    /// The key of an action the user took away from it: nothing happens
    /// (`left` once `move_left` is bound elsewhere does not fall back).
    Nothing,
    /// An editing action; `via` is the key it is carried out as.
    Edit { edit: Edit, via: Option<KeyPress> },
}

pub(super) fn resolve(keymap: &Keymap, key: &KeyPress) -> Resolved {
    for (edit, id) in EDITS {
        if pressed(keymap, id, key) {
            let via = edit.canonical().filter(|k| k != key);
            return Resolved::Edit { edit, via };
        }
    }
    thread_local! {
        static DEFAULTS: Keymap = Keymap::defaults();
    }
    let was_default = DEFAULTS.with(|d| EDITS.iter().any(|(_, id)| pressed(d, *id, key)));
    if was_default {
        Resolved::Nothing
    } else {
        Resolved::None
    }
}

impl Edit {
    /// The engine's name for the edit (Insert mode records it for `.`); the
    /// line break is the engine's own typed text.
    pub(super) fn action(self) -> Option<EditorAction> {
        Some(match self {
            Edit::Newline => return None,
            Edit::Left => EditorAction::MoveLeft,
            Edit::Right => EditorAction::MoveRight,
            Edit::Up => EditorAction::MoveUp,
            Edit::Down => EditorAction::MoveDown,
            Edit::WordLeft => EditorAction::MoveWordLeft,
            Edit::WordRight => EditorAction::MoveWordRight,
            Edit::LineStart => EditorAction::MoveLineStart {
                move_up_at_bol: false,
            },
            Edit::LineEnd => EditorAction::MoveLineEnd {
                move_down_at_eol: false,
            },
            Edit::DeleteBackward => EditorAction::DeleteBackward,
            Edit::DeleteForward => EditorAction::DeleteForward,
            Edit::DeleteBackwardWord => EditorAction::DeleteBackwardWord,
            Edit::DeleteForwardWord => EditorAction::DeleteForwardWord,
            Edit::KillLineStart => EditorAction::KillLineStart,
            Edit::KillWholeLine => EditorAction::KillLine,
            Edit::KillLineEnd => EditorAction::KillLineEnd,
            Edit::Yank => EditorAction::Yank,
        })
    }

    fn of(action: EditorAction) -> Edit {
        match action {
            EditorAction::MoveLeft => Edit::Left,
            EditorAction::MoveRight => Edit::Right,
            EditorAction::MoveUp => Edit::Up,
            EditorAction::MoveDown => Edit::Down,
            EditorAction::MoveWordLeft => Edit::WordLeft,
            EditorAction::MoveWordRight => Edit::WordRight,
            EditorAction::MoveLineStart { .. } => Edit::LineStart,
            EditorAction::MoveLineEnd { .. } => Edit::LineEnd,
            EditorAction::DeleteBackward => Edit::DeleteBackward,
            EditorAction::DeleteForward => Edit::DeleteForward,
            EditorAction::DeleteBackwardWord => Edit::DeleteBackwardWord,
            EditorAction::DeleteForwardWord => Edit::DeleteForwardWord,
            EditorAction::KillLineStart => Edit::KillLineStart,
            EditorAction::KillLine => Edit::KillWholeLine,
            EditorAction::KillLineEnd => Edit::KillLineEnd,
            EditorAction::Yank => Edit::Yank,
        }
    }
}

/// An editor action the Vim engine hands back (Insert mode, `.`): carried out
/// as the composer's own editing does, the elements, shell mode, history and
/// kill ring included.
pub(super) fn carry_out(data: &KeyData, c: &TextEditingController, action: EditorAction) -> bool {
    match Edit::of(action) {
        Edit::KillWholeLine => {
            kill_whole_line(&data.draft, c);
            true
        }
        edit => {
            let Some(key) = edit.canonical() else {
                return false;
            };
            edit_arms(data, c, &key, None) || {
                field_edit(c, &key);
                true
            }
        }
    }
}

/// The caret one logical line up or down, in the column it is in
/// (`move_up` / `move_down` bound to a key the field does not know).
fn move_line(c: &TextEditingController, down: bool) {
    let text = c.buffer();
    let caret = c.selection().extent_offset;
    let line = text.line_of(caret);
    let column = caret - text.line_start(line);
    let to = if down {
        let end = text.line_end(line);
        if end >= text.len() {
            text.len()
        } else {
            let next = end + 1;
            (next + column).min(text.line_end(text.line_of(next)))
        }
    } else if line == 0 {
        0
    } else {
        let prev_end = text.line_start(line) - 1;
        let prev = text.line_start(text.line_of(prev_end));
        (prev + column).min(prev_end)
    };
    c.move_caret(to, false);
}

/// Carries out `key` as the field's own editing would (the caret moves and
/// the character deletions, as `rtwk` writes them).
fn field_edit(c: &TextEditingController, key: &KeyPress) {
    match key.code {
        KeyCode::Up => move_line(c, false),
        KeyCode::Down => move_line(c, true),
        KeyCode::Backspace if c.selection().is_collapsed() => {
            let caret = c.selection().extent_offset;
            let from = c.buffer().character_start_before(caret);
            c.replace(from, caret, "");
        }
        KeyCode::Delete if c.selection().is_collapsed() => {
            let caret = c.selection().extent_offset;
            let to = c.buffer().character_end_after(caret);
            c.replace(caret, to, "");
        }
        KeyCode::Backspace | KeyCode::Delete => {
            let s = c.selection();
            c.replace(s.start(), s.end(), "");
        }
        _ => {
            editing_key(c, key.clone(), &|t| c.insert(t), &StoredStops, false, None);
        }
    }
}

/// Whether the composer takes `key` for the editable field of `c`: the key
/// is read through the user's Editor bindings first, then the editing that
/// is Codex's (elements, history, kills) runs on the key it means.
fn edit_key(
    data: &KeyData,
    c: &TextEditingController,
    key: &KeyPress,
    skill_space: Option<usize>,
) -> bool {
    match resolve(&data.keymap, key) {
        Resolved::None => edit_arms(data, c, key, skill_space),
        Resolved::Nothing => true,
        Resolved::Edit {
            edit: Edit::Newline,
            ..
        } => {
            // Shift+Enter and Alt+Enter are the field's own line break;
            // any other key of the action (Enter, once submit is
            // elsewhere) breaks the line here
            if key.code == KeyCode::Enter
                && (key.mods.contains(Mods::SHIFT) || key.mods.contains(Mods::ALT))
            {
                return false;
            }
            c.insert("\n");
            true
        }
        Resolved::Edit {
            edit: Edit::KillWholeLine,
            ..
        } => {
            kill_whole_line(&data.draft, c);
            true
        }
        Resolved::Edit { via: None, .. } => edit_arms(data, c, key, skill_space),
        Resolved::Edit { via: Some(via), .. } => {
            edit_arms(data, c, &via, skill_space) || {
                field_edit(c, &via);
                true
            }
        }
    }
}

/// `kill_current_line`: the line the caret is on goes, with the newline
/// that ends it (the one before it for the last line), into the kill ring.
fn kill_whole_line(draft: &ComposerDraft, c: &TextEditingController) {
    let text = c.buffer();
    let caret = c.selection().extent_offset;
    let line = text.line_of(caret);
    let (start, end) = (text.line_start(line), text.line_end(line));
    let (from, to) = if end < text.len() {
        (start, end + 1)
    } else if start > 0 {
        (start - 1, end)
    } else {
        (start, end)
    };
    kill(draft, c, from, to);
}

/// A typed character the draft's rules take: punctuation right after a
/// `$skill` completion takes the completion's space
/// (`skill_completion_space_for_char`); a `!` typed at the draft's start
/// becomes shell mode (`sync_bash_mode_from_text`). `false` leaves the
/// character to the field.
pub fn typed_char(
    draft: &ComposerDraft,
    tell: &dyn Fn(ComposerMsg),
    c: &TextEditingController,
    ch: char,
    skill_space: Option<usize>,
) -> bool {
    let selection = c.selection();
    let caret = selection.extent_offset;
    if !selection.is_collapsed() {
        return false;
    }
    if ",.;:!?)]}。，、！？》".contains(ch)
        && skill_space.is_some_and(|at| caret == at + 1 && c.text().get(at..at + 1) == Some(" "))
    {
        c.replace(caret - 1, caret, &ch.to_string());
        return true;
    }
    if ch == '!' && caret == 0 && !draft.shell() {
        draft.set_shell(true);
        tell(ComposerMsg::Shell);
        return true;
    }
    false
}

/// Codex's own editing of the keys it means (the placeholders whole, shell
/// mode, history at the draft's ends, the kill ring).
fn edit_arms(
    data: &KeyData,
    c: &TextEditingController,
    key: &KeyPress,
    skill_space: Option<usize>,
) -> bool {
    let draft = &data.draft;
    let selection = c.selection();
    let caret = selection.extent_offset;
    let collapsed = selection.is_collapsed();
    let plain = only(key, Mods::NONE);
    let ctrl = only(key, Mods::CTRL);
    let alt = only(key, Mods::ALT);
    match (key.code, key.ch) {
        (KeyCode::Char, Some(ch))
            if (plain || only(key, Mods::SHIFT))
                && typed_char(draft, &*data.tell, c, ch, skill_space) =>
        {
            true
        }
        // Shell mode: Backspace at the start and Esc on an empty draft
        // leave it.
        (KeyCode::Backspace, _) if plain && collapsed && caret == 0 && draft.shell() => {
            draft.set_shell(false);
            (data.tell)(ComposerMsg::Shell);
            true
        }
        (KeyCode::Escape, _) if draft.shell() && c.text().is_empty() => {
            draft.set_shell(false);
            (data.tell)(ComposerMsg::Shell);
            true
        }
        (KeyCode::Backspace, _) if plain && collapsed => {
            let Some(e) = draft.ending_at(c, caret) else {
                return false;
            };
            c.replace(e.start, e.end, "");
            true
        }
        (KeyCode::Delete, _) if plain && collapsed => {
            let Some(e) = draft.starting_at(c, caret) else {
                return false;
            };
            c.replace(e.start, e.end, "");
            true
        }
        (KeyCode::Left, _) if plain || only(key, Mods::SHIFT) => {
            if plain && !collapsed {
                return false;
            }
            let Some(e) = draft.ending_at(c, caret) else {
                return false;
            };
            c.move_caret(e.start, !plain);
            true
        }
        (KeyCode::Right, _) if plain || only(key, Mods::SHIFT) => {
            if plain && !collapsed {
                return false;
            }
            let Some(e) = draft.starting_at(c, caret) else {
                return false;
            };
            c.move_caret(e.end, !plain);
            true
        }
        (KeyCode::Up | KeyCode::Down, _) if plain => walk_history(data, c, key.code == KeyCode::Up),
        (KeyCode::Char, Some('k')) if ctrl && collapsed => {
            let text = c.buffer();
            let end = text.line_end(text.line_of(caret));
            let to = if end == caret {
                if end < text.len() {
                    end + 1
                } else {
                    return true;
                }
            } else {
                end
            };
            kill(draft, c, caret, to);
            true
        }
        (KeyCode::Char, Some('u')) if ctrl && collapsed => {
            let text = c.buffer();
            let start = text.line_start(text.line_of(caret));
            let from = if start == caret {
                if start > 0 { start - 1 } else { return true }
            } else {
                start
            };
            kill(draft, c, from, caret);
            true
        }
        (KeyCode::Char, Some('w')) if ctrl && collapsed => {
            let from = c.buffer().word_start_before(caret, false);
            kill(draft, c, from, caret);
            true
        }
        (KeyCode::Backspace, _) if (alt || ctrl) && collapsed => {
            let from = c.buffer().word_start_before(caret, false);
            kill(draft, c, from, caret);
            true
        }
        (KeyCode::Char, Some('d')) if alt && collapsed => {
            let to = c.buffer().word_end_after(caret, false);
            kill(draft, c, caret, to);
            true
        }
        (KeyCode::Delete, _) if (alt || ctrl) && collapsed => {
            let to = c.buffer().word_end_after(caret, false);
            kill(draft, c, caret, to);
            true
        }
        // `clear_for_ctrl_c`: a draft is cleared (and kept in history)
        // before Ctrl+C interrupts or quits.
        (KeyCode::Char, Some('c')) if ctrl && !c.text().is_empty() => {
            (data.tell)(ComposerMsg::ClearForCtrlC);
            true
        }
        (KeyCode::Char, Some('y')) if ctrl => {
            draft.yank(c);
            // a yank at the start is a prefix insertion: a leading `!`
            // becomes shell mode (`sync_bash_mode_from_text`)
            if caret == 0 && collapsed && draft.absorb_shell_prefix(c) {
                (data.tell)(ComposerMsg::Shell);
            }
            true
        }
        _ => false,
    }
}

/// Kills `from..to` (widened to whole elements), keeping it for `Ctrl+Y`
/// (`kill_range`).
fn kill(draft: &ComposerDraft, c: &TextEditingController, from: usize, to: usize) {
    let (mut from, mut to) = (from.min(to), from.max(to));
    for e in draft.elements(c) {
        if e.start < from && from < e.end {
            from = e.start;
        }
        if e.start < to && to < e.end {
            to = e.end;
        }
    }
    if from == to {
        return;
    }
    draft.kill(c, from, to);
    c.replace(from, to, "");
    c.set_selection(TextSelection::collapsed(from));
}

#[cfg(test)]
mod tests {
    //! The oracle's `textarea.rs` state tests of the kill keys
    //! (`delete_backward_word_and_kill_line_variants`, `yank_restores_last_kill`,
    //! `kill_buffer_persists_across_set_text`), through the composer's keys.

    use super::*;

    fn data() -> KeyData {
        KeyData {
            draft: ComposerDraft::default(),
            history: Shared::default(),
            tell: Rc::new(|_| {}),
            burst: None,
            suggest: Rc::default(),
            keymap: Rc::new(Keymap::defaults()),
        }
    }

    fn at(text: &str, caret: usize) -> TextEditingController {
        let c = TextEditingController::new(text);
        c.set_selection(TextSelection::collapsed(caret));
        c
    }

    fn press(d: &KeyData, c: &TextEditingController, key: KeyPress) {
        assert!(edit_key(d, c, &key, None), "the composer takes {key:?}");
    }

    fn ctrl(ch: char) -> KeyPress {
        KeyPress::char(ch).with_mods(Mods::CTRL)
    }

    fn state(c: &TextEditingController) -> (String, usize) {
        (c.text().to_string(), c.selection().extent_offset)
    }

    #[test]
    fn delete_backward_word_and_kill_line_variants() {
        let d = data();
        let c = at("hello   world  ", 15);
        press(&d, &c, ctrl('w'));
        assert_eq!(state(&c), ("hello   ".into(), 8));
        let c = at("foo bar", 6);
        press(
            &d,
            &c,
            KeyPress::code(KeyCode::Backspace).with_mods(Mods::ALT),
        );
        assert_eq!(state(&c), ("foo r".into(), 4));
        let c = at("foo bar", 7);
        press(&d, &c, ctrl('w'));
        assert_eq!(state(&c), ("foo ".into(), 4));
        let c = at("abc\ndef", 1);
        press(&d, &c, ctrl('k'));
        assert_eq!(state(&c), ("a\ndef".into(), 1));
        let c = at("abc\ndef", 3);
        press(&d, &c, ctrl('k'));
        assert_eq!(state(&c), ("abcdef".into(), 3));
        let c = at("abc\ndef", 5);
        press(&d, &c, ctrl('u'));
        assert_eq!(c.text().as_ref(), "abc\nef");
        let c = at("abc\ndef", 4);
        press(&d, &c, ctrl('u'));
        assert_eq!(state(&c), ("abcdef".into(), 3));
    }

    #[test]
    fn yank_restores_last_kill() {
        let d = data();
        let c = at("hello", 0);
        press(&d, &c, ctrl('k'));
        assert_eq!(state(&c), (String::new(), 0));
        press(&d, &c, ctrl('y'));
        assert_eq!(state(&c), ("hello".into(), 5));

        let d = data();
        let c = at("hello world", 11);
        press(&d, &c, ctrl('w'));
        assert_eq!(state(&c), ("hello ".into(), 6));
        press(&d, &c, ctrl('y'));
        assert_eq!(state(&c), ("hello world".into(), 11));

        let d = data();
        let c = at("hello", 5);
        press(&d, &c, ctrl('u'));
        assert_eq!(state(&c), (String::new(), 0));
        press(&d, &c, ctrl('y'));
        assert_eq!(state(&c), ("hello".into(), 5));
    }

    #[test]
    fn kill_buffer_persists_across_set_text() {
        let d = data();
        let c = at("restore me", 0);
        press(&d, &c, ctrl('k'));
        assert!(c.text().is_empty());
        c.set_text("/diff");
        c.set_text("");
        press(&d, &c, ctrl('y'));
        assert_eq!(state(&c), ("restore me".into(), "restore me".len()));
    }

    #[test]
    fn a_placeholder_goes_whole_under_left_right_and_backspace() {
        let d = data();
        let c = at("ab", 1);
        d.draft.insert_element(&c, "[P]", "payload");
        assert_eq!(state(&c), ("a[P]b".into(), 4));
        assert_eq!(d.draft.expanded(&c), "apayloadb");
        press(&d, &c, KeyPress::code(KeyCode::Left));
        assert_eq!(c.selection().extent_offset, 1);
        press(&d, &c, KeyPress::code(KeyCode::Right));
        assert_eq!(c.selection().extent_offset, 4);
        press(&d, &c, KeyPress::code(KeyCode::Backspace));
        assert_eq!(state(&c), ("ab".into(), 1));
        assert_eq!(d.draft.expanded(&c), "ab");
    }

    fn keyed(table: &str, action: &str, keys: &[&str]) -> KeyData {
        let spec = codex_tui_core::keymap::KeymapSpec::default().with(table, action, keys);
        KeyData {
            keymap: Rc::new(Keymap::resolve_unchecked(&spec)),
            ..data()
        }
    }

    #[test]
    fn enter_and_tab_are_left_to_the_field_and_the_page_by_default() {
        let d = data();
        let c = at("abc", 3);
        // Enter is `editor.insert_newline` too, but Composer shadows it: it submits
        assert_eq!(
            composer_key(&d, &c, &KeyPress::code(KeyCode::Enter)),
            Some(false)
        );
        // the page's own binding queues
        assert_eq!(
            composer_key(&d, &c, &KeyPress::code(KeyCode::Tab)),
            Some(false)
        );
        assert_eq!(composer_key(&d, &c, &KeyPress::char('a')), None);
    }

    #[test]
    fn a_remapped_submit_frees_enter_for_the_line_break_and_swallows_tab_when_unbound() {
        let d = keyed("composer", "submit", &["ctrl-s"]);
        let c = at("ab", 2);
        assert_eq!(composer_key(&d, &c, &KeyPress::code(KeyCode::Enter)), None);
        press(&d, &c, KeyPress::code(KeyCode::Enter));
        assert_eq!(c.text().as_ref(), "ab\n");
        let d = keyed("composer", "queue", &["f5"]);
        assert_eq!(
            composer_key(&d, &c, &KeyPress::code(KeyCode::Tab)),
            Some(true)
        );
    }

    #[test]
    fn a_key_rebound_to_an_editor_action_is_carried_out_as_it_and_the_old_key_does_nothing() {
        let d = keyed("editor", "move_left", &["f2"]);
        let c = at("abc", 2);
        press(&d, &c, KeyPress::code(KeyCode::F2));
        assert_eq!(state(&c), ("abc".into(), 1));
        // `left` is no longer bound to anything: it does not fall back
        press(&d, &c, KeyPress::code(KeyCode::Left));
        assert_eq!(state(&c), ("abc".into(), 1));
    }

    #[test]
    fn kill_whole_line_is_unbound_until_the_user_binds_it() {
        let d = data();
        let c = at("abc\ndef\nghi", 5);
        press(&d, &c, ctrl('u'));
        assert_eq!(
            c.text().as_ref(),
            "abc\nef\nghi",
            "ctrl+u is kill_line_start"
        );
        let d = keyed("editor", "kill_whole_line", &["ctrl-o"]);
        let c = at("abc\ndef\nghi", 5);
        press(&d, &c, ctrl('o'));
        assert_eq!(c.text().as_ref(), "abc\nghi");
        let c = at("abc\ndef", 5);
        press(&d, &c, ctrl('o'));
        assert_eq!(c.text().as_ref(), "abc");
        // the killed line is in the ring
        press(&d, &c, ctrl('y'));
        assert_eq!(c.text().as_ref(), "abc\ndef");
    }
}
