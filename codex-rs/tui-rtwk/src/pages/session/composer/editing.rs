//! The composer's own editing keys, asked before the `TextArea`'s through
//! `EditingKeys` (rtwk's door for an app's keys): the atomic elements
//! (Backspace, Delete, Left and Right take a placeholder whole), the kill
//! ring (`Ctrl+K`, `Ctrl+U`, `Ctrl+W` / `Alt+Backspace`, `Alt+D` kill,
//! `Ctrl+Y` yanks) and prompt history (Up and Down at the draft's ends).
//! Every other key is the field's. Oracle: `bottom_pane/textarea.rs`
//! (`input_with_keymap`, the default editor keymap of `keymap.rs`) and
//! `chat_composer_history.rs`.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use rtwk::editing::{KeyCx, StoredStops, TextEditingController, TextSelection, editing_key};
use rtwk::focus::{KeyCombo, KeyPress};

use codex_tui_core::data::history::Store;
use codex_tui_core::keymap::actions::{
    COMPOSER_QUEUE, COMPOSER_SUBMIT, EDITOR_DELETE_BACKWARD, EDITOR_DELETE_BACKWARD_WORD,
    EDITOR_DELETE_FORWARD, EDITOR_DELETE_FORWARD_WORD, EDITOR_INSERT_NEWLINE, EDITOR_KILL_LINE_END,
    EDITOR_KILL_LINE_START, EDITOR_KILL_WHOLE_LINE, EDITOR_MOVE_DOWN, EDITOR_MOVE_LEFT,
    EDITOR_MOVE_LINE_END, EDITOR_MOVE_LINE_START, EDITOR_MOVE_RIGHT, EDITOR_MOVE_UP,
    EDITOR_MOVE_WORD_LEFT, EDITOR_MOVE_WORD_RIGHT, EDITOR_YANK, GLOBAL_COPY,
    GLOBAL_TOGGLE_VIM_MODE,
};
use codex_tui_core::keymap::fixed::{
    CLEAR_FOR_CTRL_C, DISMISS, ENTER, FIELD_DELETE_BACKWARD, FIELD_DELETE_BACKWARD_WORD,
    FIELD_DELETE_FORWARD, FIELD_DELETE_FORWARD_WORD, FIELD_DOWN, FIELD_KILL_LINE_END,
    FIELD_KILL_LINE_START, FIELD_LEFT, FIELD_LINE_END, FIELD_LINE_START, FIELD_RIGHT, FIELD_UP,
    FIELD_WORD_LEFT, FIELD_WORD_RIGHT, FIELD_YANK, HISTORY_NEXT, HISTORY_PREVIOUS,
    INTERRUPT_OR_QUIT, PASTE_IMAGE, TAB,
};
use codex_tui_core::keymap::{ActionId, Binding, Keymap};
use codex_tui_core::kit::vim::{EditorAction, VimPending};
use codex_tui_core::screens::composer::restricted::Restricted;
use codex_tui_core::screens::{Shared, popovers};

use super::ComposerMsg;
use super::burst::{self, BurstEnv};
use super::mentions;
use super::search;
use super::suggest::Suggest;
use super::vim;
use crate::keymap::{
    c0_ctrl_h, combo_of, holds_command_mod, holds_ctrl_or_alt, holds_shift_or_alt, only_shift,
    press_for, pressed, stroke_label, unmodified,
};
use crate::widgets::composer_draft::{ComposerDraft, KEY_CHORD_TIMEOUT};
use crate::widgets::composer_sparkle::{Key as SparkleKey, SparkleController};

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
    /// What the app holds for the popups: their sources, the clipboard's answers.
    pub popovers: Rc<popovers::Model>,
    /// Why the composer may only edit and run a few commands (`None`: it is
    /// whole).
    pub restricted: Option<Restricted>,
    /// The thread's sparkle: told each key before the composer handles it.
    pub sparkle: SparkleController,
}

/// Up (`up`) or Down at the draft's ends: the history's entry replaces the
/// draft (`navigate_up`, `navigate_down`). A read the store lacks goes to
/// the app, and the draft waits for it. `false`: the caret moves instead.
pub(super) fn walk_history(data: &KeyData, c: &TextEditingController, up: bool) -> bool {
    let draft = &data.draft;
    // an offline draft is a text only: Up and Down move the caret
    if data.restricted.is_some() {
        return false;
    }
    let shell = draft.shell();
    let remote = draft.remote_images();
    let mut asks = Vec::new();
    let took = draft.history_step(c, &data.history, up, &mut asks);
    if !asks.is_empty() {
        (data.tell)(ComposerMsg::Lookups(asks));
    }
    // an entry's remote images stand above the draft: the rows change with
    // no change of the text
    if took && draft.remote_images() != remote {
        (data.tell)(ComposerMsg::Remote);
    }
    // an entry's leading `!` is shell mode again
    if draft.shell() != shell {
        (data.tell)(ComposerMsg::Shell);
    }
    took
}

/// Whether the composer takes `key` for `c` before the field: the modal
/// (Vim) keys that precede everything, the paste-burst detector (keys that
/// arrive as fast as a paste are held), then the composer's own editing keys.
pub fn take_key(data: &KeyData, c: &TextEditingController, key: &KeyPress, kcx: KeyCx) -> bool {
    if kcx.read_only {
        return false;
    }
    // the sparkle hears of the key first (`before_sparkle_key`) and of what a
    // held character became after it (`after_sparkle_key`: a character held
    // back that the key dropped changed the draft with no change of its text)
    note_keyboard_activity(&data.draft, key, || {
        data.burst.as_ref().map(|env| (env.now)())
    });
    let held = data.draft.in_burst();
    let mode = data.draft.vim_label();
    data.sparkle.before_key(&sparkle_key(data, c, key));
    let took = take_key_inner(data, c, key);
    // the footer's `Vim: <mode>` is the page's: it hears of a mode the key changed
    if data.draft.vim_label() != mode {
        (data.tell)(ComposerMsg::VimMode);
    }
    if held && !data.draft.in_burst() && !data.draft.searching() {
        data.sparkle.replaced(&c.text());
    }
    took
}

/// An approval waits until the draft's user stops typing, including in its history search.
pub(super) fn note_keyboard_activity(
    draft: &ComposerDraft,
    key: &KeyPress,
    now: impl FnOnce() -> Option<Duration>,
) {
    if counts_as_typing(key)
        && let Some(now) = now()
    {
        draft.keyed(now);
    }
}

/// Whether the oracle counts the key as typing (characters, Backspace, Delete, Enter and Tab
/// without Ctrl or Alt).
fn counts_as_typing(key: &KeyPress) -> bool {
    if holds_ctrl_or_alt(key) {
        return false;
    }
    key.text().is_some()
        || ["backspace", "delete", "enter", "tab"].iter().any(|name| {
            rtwk::focus::KeyCombo::parse(name).is_ok_and(|combo| combo.accepts_press(key))
        })
}

/// What the sparkle asks of a key: whether it is a plain character, whether
/// the composer holds nothing, where the caret is, and whether the key is
/// the history search's (it opens or is inside it: no sparkle's business).
fn sparkle_key(data: &KeyData, c: &TextEditingController, key: &KeyPress) -> SparkleKey {
    let plain = !holds_command_mod(key);
    let plain_char = if plain && key.ch.is_some() {
        key.text().and_then(|text| text.chars().next())
    } else {
        None
    };
    let mode = vim::mode(&data.draft);
    SparkleKey {
        plain_char,
        empty: c.text().is_empty() && !data.draft.shell(),
        cursor: c.selection().extent_offset,
        text: c.text().to_string(),
        aside: data.draft.searching() || search::opens(data, key),
        vim_query: mode.searching,
        // (`/` on an empty draft goes into Insert mode first: the sparkle hears it as typed)
        vim_normal: mode.normal && !vim::opens_slash(data, c, key, &mode),
    }
}

fn take_key_inner(data: &KeyData, c: &TextEditingController, key: &KeyPress) -> bool {
    if let Some(mode) = data.restricted
        && let Some(took) = restricted_key(data, c, key, mode)
    {
        return took;
    }
    if before_search(data, key) {
        return true;
    }
    // reverse history search owns every key while it is open
    if data.restricted.is_none() && data.draft.searching() {
        return search::key(data, c, key);
    }
    let mut mode = vim::mode(&data.draft);
    if mode.searching {
        return vim::search_key(data, c, key);
    }
    if mode.normal {
        if let Some(took) = vim::early(data, c, key, &mode) {
            return took;
        }
        // `/` on an empty draft went into Insert mode and is typed there: the key is read in the
        // mode it left behind
        mode = vim::mode(&data.draft);
    }
    // a `$skill` completion's space lasts for the next key only
    let skill_space = data.draft.take_skill_space();
    // an editor chord's keys are no typing, whatever the burst detector or a popup would make of
    // the second (Normal mode's own chords are Vim's)
    if !mode.normal
        && let Some(took) = chord_key(data, c, key, skill_space)
    {
        return took;
    }
    // a restricted composer only edits: no popup, burst, history, search or
    // submit of its own (`handle_input_basic`)
    let free = data.restricted.is_none();
    // `/` is a search in Normal mode, not a command's name being typed
    if free && !mode.normal && data.suggest.take_slash(c, key, &*data.tell) {
        return true;
    }
    // Normal mode's keys are commands: nothing is typed text to hold back
    if free
        && !mode.normal
        && let Some(env) = &data.burst
        && burst::take(data, env, c, key, skill_space)
    {
        return true;
    }
    // a popup with rows takes the keys of its list before the editing keys
    // (history on Up and Down, the queue key on Tab)
    if free && mentions::take_key(data, c, key) {
        return true;
    }
    if free && data.suggest.take_key(&data.keymap, c, key, &*data.tell) {
        return true;
    }
    // the rows of remote images above the draft take Up, Down and Delete
    if free && remote_key(data, c, key) {
        return true;
    }
    // Ctrl+R opens the search (Vim's Normal mode took it for redo above, if
    // redo is bound to it)
    if free && search::opens(data, key) {
        search::begin(data, c);
        return true;
    }
    // a command waiting for its next key takes Enter too (it is no submit)
    let waiting = mode.normal && mode.pending != VimPending::None;
    if free
        && !waiting
        && let Some(took) = composer_key(data, c, key)
    {
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

/// The keys of a restricted composer (`handle_restricted_key`): the history
/// search ends, no remote image row is selected, the popups close and the
/// pastes are expanded where they stand (a key that expanded one runs no
/// command: it must be on the screen before Enter sends it); then Enter and
/// Tab, whatever they are bound to, are not edits, and Enter runs the draft
/// when it is a command the mode lets pass. `None`: an edit, the rest of the
/// composer's own.
fn restricted_key(
    data: &KeyData,
    c: &TextEditingController,
    key: &KeyPress,
    mode: Restricted,
) -> Option<bool> {
    if mode == Restricted::Disconnected && pressed(&data.keymap, INTERRUPT_OR_QUIT, key) {
        return Some(false);
    }
    let expanded = data.draft.restrict(c);
    data.suggest.hide(c.text());
    let submit = pressed(&data.keymap, COMPOSER_SUBMIT, key);
    let queue = pressed(&data.keymap, COMPOSER_QUEUE, key);
    let plain =
        unmodified(key) && (pressed(&data.keymap, ENTER, key) || pressed(&data.keymap, TAB, key));
    if !(submit || queue || plain) {
        return None;
    }
    if submit && !expanded && mode.dispatches(&c.text()) {
        (data.tell)(ComposerMsg::Submitted(c.text()));
    }
    Some(true)
}

/// Global editing actions that precede both the draft's and the query field's keys.
#[derive(Clone, Copy, Debug)]
pub enum BeforeSearch {
    VimToggled(bool),
    PasteImage,
}

fn before_search(data: &KeyData, key: &KeyPress) -> bool {
    before_search_key(
        &data.draft,
        &data.keymap,
        key,
        || {
            data.burst
                .as_ref()
                .map(|env| ((env.now)(), data.popovers.pasted.answers()))
        },
        |action| (data.tell)(ComposerMsg::BeforeSearch(action)),
    )
}

/// The Vim toggle and the clipboard key keep their precedence during history search.
/// A plain-text composer cannot attach images, and a user can bind the key to Copy.
pub(super) fn before_search_key(
    draft: &ComposerDraft,
    keymap: &Keymap,
    key: &KeyPress,
    clipboard: impl FnOnce() -> Option<(Duration, u64)>,
    emit: impl FnOnce(BeforeSearch),
) -> bool {
    if pressed(keymap, GLOBAL_TOGGLE_VIM_MODE, key) {
        emit(BeforeSearch::VimToggled(vim::toggle(draft)));
        return true;
    }
    if draft.is_plain() || !pressed(keymap, PASTE_IMAGE, key) || pressed(keymap, GLOBAL_COPY, key) {
        return false;
    }
    let Some((now, answers)) = clipboard() else {
        return false;
    };
    if draft.ask_for_image(now, answers) {
        emit(BeforeSearch::PasteImage);
    }
    true
}

/// Up at the draft's start selects the last remote image row, Up and Down
/// move the selection, Delete and Backspace remove the selected row; any
/// other key ends the selection and goes on
/// (`handle_remote_image_selection_key`). Removing a row is an edit of its
/// own in Vim.
fn remote_key(data: &KeyData, c: &TextEditingController, key: &KeyPress) -> bool {
    let draft = &data.draft;
    let Some(env) = &data.burst else {
        return false;
    };
    let took = unmodified(key)
        && if pressed(&data.keymap, HISTORY_PREVIOUS, key) {
            draft.remote_up(c)
        } else if pressed(&data.keymap, HISTORY_NEXT, key) {
            draft.remote_down(c)
        } else if pressed(&data.keymap, FIELD_DELETE_FORWARD, key)
            || pressed(&data.keymap, FIELD_DELETE_BACKWARD, key)
        {
            draft.remote_removable(c) && {
                draft.vim_direct(c, || draft.remote_remove(c, &*env.image));
                true
            }
        } else {
            false
        };
    if took || draft.clear_remote_selection() {
        (data.tell)(ComposerMsg::Remote);
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
    let plain = |id: ActionId| pressed(keymap, id, key) && unmodified(key);
    if pressed(keymap, COMPOSER_SUBMIT, key) {
        if plain(ENTER) {
            return Some(false);
        }
        (data.tell)(ComposerMsg::Submitted(c.text()));
        return Some(true);
    }
    if pressed(keymap, COMPOSER_QUEUE, key) {
        // a plain-text composer has no queue and no shell command: the queue
        // key submits (`handle_submission(queue_submissions)`)
        if data.draft.is_plain() {
            (data.tell)(ComposerMsg::Submitted(c.text()));
            return Some(true);
        }
        if plain(TAB) {
            return Some(false);
        }
        (data.tell)(ComposerMsg::Queue);
        return Some(true);
    }
    plain(TAB).then_some(true)
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
        press_for(match self {
            Edit::Newline | Edit::KillWholeLine => return None,
            Edit::Left => FIELD_LEFT,
            Edit::Right => FIELD_RIGHT,
            Edit::Up => FIELD_UP,
            Edit::Down => FIELD_DOWN,
            Edit::WordLeft => FIELD_WORD_LEFT,
            Edit::WordRight => FIELD_WORD_RIGHT,
            Edit::LineStart => FIELD_LINE_START,
            Edit::LineEnd => FIELD_LINE_END,
            Edit::DeleteBackward => FIELD_DELETE_BACKWARD,
            Edit::DeleteForward => FIELD_DELETE_FORWARD,
            Edit::DeleteBackwardWord => FIELD_DELETE_BACKWARD_WORD,
            Edit::DeleteForwardWord => FIELD_DELETE_FORWARD_WORD,
            Edit::KillLineStart => FIELD_KILL_LINE_START,
            Edit::KillLineEnd => FIELD_KILL_LINE_END,
            Edit::Yank => FIELD_YANK,
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
    // a terminal without the kitty protocol sends Ctrl+H as the C0 byte, which arrives as
    // Backspace holding Ctrl: the oracle's backend reads it as `Ctrl+h`, the key `ctrl-h` names
    let c0 = c0_ctrl_h(key);
    for (edit, id) in EDITS {
        if pressed(keymap, id, key) || c0.as_ref().is_some_and(|k| pressed(keymap, id, k)) {
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
                field_edit(&data.keymap, c, &key);
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
fn field_edit(keymap: &Keymap, c: &TextEditingController, key: &KeyPress) {
    let is = |id: ActionId| pressed(keymap, id, key);
    match () {
        () if is(FIELD_UP) => move_line(c, false),
        () if is(FIELD_DOWN) => move_line(c, true),
        () if is(FIELD_DELETE_BACKWARD) && c.selection().is_collapsed() => {
            let caret = c.selection().extent_offset;
            let from = c.buffer().character_start_before(caret);
            c.replace(from, caret, "");
        }
        () if is(FIELD_DELETE_FORWARD) && c.selection().is_collapsed() => {
            let caret = c.selection().extent_offset;
            let to = c.buffer().character_end_after(caret);
            c.replace(caret, to, "");
        }
        () if is(FIELD_DELETE_BACKWARD) || is(FIELD_DELETE_FORWARD) => {
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
        Resolved::Edit { edit, via } => {
            // Shift+Enter and Alt+Enter are the field's own line break;
            // any other key of the action (Enter, once submit is
            // elsewhere) breaks the line here
            if edit == Edit::Newline && pressed(&data.keymap, ENTER, key) && holds_shift_or_alt(key)
            {
                return false;
            }
            carry_edit(data, c, edit, via, key, skill_space)
        }
    }
}

/// Carries `edit` out: the line break and the kill of a line are the composer's, the rest run as
/// the key `via` (the field's own for the action) or, with none, as `key`.
fn carry_edit(
    data: &KeyData,
    c: &TextEditingController,
    edit: Edit,
    via: Option<KeyPress>,
    key: &KeyPress,
    skill_space: Option<usize>,
) -> bool {
    match edit {
        Edit::Newline => {
            c.insert("\n");
            true
        }
        Edit::KillWholeLine => {
            kill_whole_line(&data.draft, c);
            true
        }
        _ => match via {
            None => edit_arms(data, c, key, skill_space),
            Some(via) => {
                edit_arms(data, c, &via, skill_space) || {
                    field_edit(&data.keymap, c, &via);
                    true
                }
            }
        },
    }
}

/// What a key is to the editor chords of the user's bindings (`KeyChordMatcher::advance`).
enum ChordStep {
    /// The key begins a chord: it is held and nothing else happens.
    Hold,
    /// The key completes the chord the held key began.
    Done(Edit),
    /// Esc with a chord held: the chord is gone and Esc is spent on it.
    Cancel,
    /// Not part of a chord: an ordinary key (a held key that nothing completed is dropped).
    Fresh,
}

/// The chords `(prefix, completion)` of an editing action as the keymap resolved them.
fn chords_of(keymap: &Keymap, id: ActionId) -> Vec<(KeyCombo, KeyCombo)> {
    keymap
        .bindings(id)
        .into_iter()
        .filter_map(|binding| match binding {
            Binding::Chord { prefix, completion } => {
                Some((combo_of(&prefix)?, combo_of(&completion)?))
            }
            Binding::Single(_) => None,
        })
        .collect()
}

/// What `key` is to the editor chords, when `first` is the key held (a chord's first key the user
/// pressed and nothing has ended yet). The held key's age is the caller's: an expired one is
/// not passed.
fn chord_step(keymap: &Keymap, first: Option<&KeyPress>, key: &KeyPress) -> ChordStep {
    if let Some(first) = first {
        if pressed(keymap, DISMISS, key) && unmodified(key) {
            return ChordStep::Cancel;
        }
        if let Some((edit, _)) = EDITS.iter().find(|(_, id)| {
            chords_of(keymap, *id)
                .iter()
                .any(|(prefix, done)| prefix.accepts_press(first) && done.accepts_press(key))
        }) {
            return ChordStep::Done(*edit);
        }
    }
    let begins = EDITS.iter().any(|(_, id)| {
        chords_of(keymap, *id)
            .iter()
            .any(|(prefix, _)| prefix.accepts_press(key))
    });
    if begins {
        ChordStep::Hold
    } else {
        ChordStep::Fresh
    }
}

/// Whether `key` completes an editor chord the held `first` began: the page asks before it
/// lets a key go on to the composer with a leader's prefix held.
pub(crate) fn completes_editor_chord(keymap: &Keymap, first: &KeyPress, key: &KeyPress) -> bool {
    matches!(chord_step(keymap, Some(first), key), ChordStep::Done(_))
}

/// One guide row per action: the action and the labels of the keys that complete it.
pub(crate) type ChordRows = Vec<(ActionId, Vec<String>)>;

/// The completions of the editor chords `first` begins, as the guide lists them: the prefix as
/// the keymap spells it, and each action with the keys that complete it (`PendingChord::actions`).
/// `None`: `first` begins none.
pub(crate) fn chord_rows(keymap: &Keymap, first: &KeyPress) -> Option<(String, ChordRows)> {
    let mut title = None;
    let mut rows: ChordRows = Vec::new();
    for (_, id) in EDITS {
        for binding in keymap.bindings(id) {
            let Binding::Chord { prefix, completion } = binding else {
                continue;
            };
            if !combo_of(&prefix).is_some_and(|combo| combo.accepts_press(first)) {
                continue;
            }
            title.get_or_insert_with(|| stroke_label(&prefix));
            let label = stroke_label(&completion);
            match rows.iter_mut().find(|(row, _)| *row == id) {
                Some((_, keys)) => keys.push(label),
                None => rows.push((id, vec![label])),
            }
        }
    }
    Some((title?, rows))
}

/// The editor chords of the composer: the first key is held (`ComposerDraft::chord`) and the
/// second completes it, Esc cancels it, anything else is an ordinary key; one that nothing
/// followed for a second is gone (`KEY_CHORD_TIMEOUT`). `None`: the key goes on as an ordinary one.
fn chord_key(
    data: &KeyData,
    c: &TextEditingController,
    key: &KeyPress,
    skill_space: Option<usize>,
) -> Option<bool> {
    let draft = &data.draft;
    let now = data
        .burst
        .as_ref()
        .map(|env| (env.now)())
        .unwrap_or_default();
    let before = draft.chord();
    // a leader's prefix waits for ever (`expire`: only a chord that is not persistent ends)
    let leader = draft.chord_is_leader();
    let held = before
        .clone()
        .filter(|(_, since)| leader || now.saturating_sub(*since) < KEY_CHORD_TIMEOUT);
    let step = chord_step(&data.keymap, held.as_ref().map(|(first, _)| first), key);
    match step {
        ChordStep::Hold => draft.set_chord(Some((key.clone(), now))),
        _ if before.is_some() => draft.set_chord(None),
        _ => {}
    }
    match step {
        ChordStep::Hold | ChordStep::Cancel => Some(true),
        ChordStep::Done(edit) => Some(carry_edit(
            data,
            c,
            edit,
            edit.canonical(),
            key,
            skill_space,
        )),
        ChordStep::Fresh => None,
    }
}

/// What a one-line field (Find's query: the oracle's own `TextArea`, not the composer's) takes of
/// the user's Editor bindings: an action's key is carried out as the field's own editing would,
/// and a chord's second key completes the chord whose first `held` kept. The composer's elements,
/// kill ring and history are the composer's. `false`: the key is the field's.
pub(crate) fn edit_query(
    keymap: &Keymap,
    c: &TextEditingController,
    key: &KeyPress,
    held: &RefCell<Option<KeyPress>>,
) -> bool {
    let carried = |edit: Edit| {
        if let Some(via) = edit.canonical() {
            field_edit(keymap, c, &via);
        }
    };
    // (Esc with a chord held is the field's: Find closes on it)
    let first = held.borrow_mut().take();
    match chord_step(keymap, first.as_ref(), key) {
        ChordStep::Done(edit) => {
            carried(edit);
            return true;
        }
        ChordStep::Hold => {
            *held.borrow_mut() = Some(key.clone());
            return true;
        }
        ChordStep::Cancel | ChordStep::Fresh => {}
    }
    match resolve(keymap, key) {
        Resolved::Nothing => true,
        Resolved::Edit {
            edit,
            via: Some(via),
        } if !matches!(edit, Edit::Newline | Edit::KillWholeLine) => {
            field_edit(keymap, c, &via);
            true
        }
        _ => false,
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
    // (a plain-text draft cannot enter shell mode: the `!` is text)
    if ch == '!' && caret == 0 && draft.set_shell(true) {
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
    let plain = unmodified(key);
    let is = |id: ActionId| pressed(&data.keymap, id, key);
    if let Some(ch) = key.ch
        && (plain || only_shift(key))
        && typed_char(draft, &*data.tell, c, ch, skill_space)
    {
        return true;
    }
    // Shell mode: Backspace at the start and Esc on an empty draft
    // leave it.
    let deleting_backward = matches!(
        resolve(&data.keymap, key),
        Resolved::Edit {
            edit: Edit::DeleteBackward,
            ..
        }
    );
    if deleting_backward && collapsed && caret == 0 && draft.shell() {
        draft.set_shell(false);
        (data.tell)(ComposerMsg::Shell);
        return true;
    }
    if is(DISMISS) && draft.shell() && c.text().is_empty() {
        draft.set_shell(false);
        (data.tell)(ComposerMsg::Shell);
        return true;
    }
    if is(FIELD_DELETE_BACKWARD) && plain && collapsed {
        let Some(e) = draft.ending_at(c, caret) else {
            return false;
        };
        c.replace(e.start, e.end, "");
        return true;
    }
    if is(FIELD_DELETE_FORWARD) && plain && collapsed {
        let Some(e) = draft.starting_at(c, caret) else {
            return false;
        };
        c.replace(e.start, e.end, "");
        return true;
    }
    if is(FIELD_LEFT) && (plain || only_shift(key)) {
        if plain && !collapsed {
            return false;
        }
        let Some(e) = draft.ending_at(c, caret) else {
            return false;
        };
        c.move_caret(e.start, !plain);
        return true;
    }
    if is(FIELD_RIGHT) && (plain || only_shift(key)) {
        if plain && !collapsed {
            return false;
        }
        let Some(e) = draft.starting_at(c, caret) else {
            return false;
        };
        c.move_caret(e.end, !plain);
        return true;
    }
    if (is(HISTORY_PREVIOUS) || is(HISTORY_NEXT)) && plain {
        return walk_history(data, c, is(HISTORY_PREVIOUS));
    }
    if is(FIELD_KILL_LINE_END) && collapsed {
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
        return true;
    }
    if is(FIELD_KILL_LINE_START) && collapsed {
        let text = c.buffer();
        let start = text.line_start(text.line_of(caret));
        let from = if start == caret {
            if start > 0 { start - 1 } else { return true }
        } else {
            start
        };
        kill(draft, c, from, caret);
        return true;
    }
    if is(FIELD_DELETE_BACKWARD_WORD) && collapsed {
        let from = c.buffer().word_start_before(caret, false);
        kill(draft, c, from, caret);
        return true;
    }
    if is(FIELD_DELETE_FORWARD_WORD) && collapsed {
        let to = c.buffer().word_end_after(caret, false);
        kill(draft, c, caret, to);
        return true;
    }
    // `clear_for_ctrl_c`: a draft is cleared (and kept in history)
    // before Ctrl+C interrupts or quits.
    if is(CLEAR_FOR_CTRL_C) && !c.text().is_empty() {
        (data.tell)(ComposerMsg::ClearForCtrlC);
        return true;
    }
    if is(FIELD_YANK) {
        draft.yank(c);
        // a yank at the start is a prefix insertion: a leading `!`
        // becomes shell mode (`sync_bash_mode_from_text`)
        if caret == 0 && collapsed && draft.absorb_shell_prefix(c) {
            (data.tell)(ComposerMsg::Shell);
        }
        return true;
    }
    false
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
    use crate::keymap::press_of;
    use codex_tui_core::keymap::Stroke;

    fn data() -> KeyData {
        KeyData {
            draft: ComposerDraft::default(),
            history: Shared::default(),
            tell: Rc::new(|_| {}),
            burst: None,
            suggest: Rc::default(),
            keymap: Rc::new(Keymap::defaults()),
            popovers: Rc::default(),
            restricted: None,
            sparkle: SparkleController::default(),
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

    /// The press of a hardwired action's first key (the defaults' for a configurable one).
    fn of(id: ActionId) -> KeyPress {
        nth(id, 0)
    }

    fn nth(id: ActionId, n: usize) -> KeyPress {
        crate::keymap::presses(&Keymap::defaults(), id)
            .into_iter()
            .nth(n)
            .expect("the action has the key")
    }

    /// The press of the first key the test's own keymap gives an action.
    fn of_keyed(d: &KeyData, id: ActionId) -> KeyPress {
        crate::keymap::presses(&d.keymap, id)
            .into_iter()
            .next()
            .expect("the keymap binds the action")
    }

    fn state(c: &TextEditingController) -> (String, usize) {
        (c.text().to_string(), c.selection().extent_offset)
    }

    #[test]
    fn delete_backward_word_and_kill_line_variants() {
        let d = data();
        let c = at("hello   world  ", 15);
        press(&d, &c, of(FIELD_DELETE_BACKWARD_WORD));
        assert_eq!(state(&c), ("hello   ".into(), 8));
        let c = at("foo bar", 6);
        press(&d, &c, nth(FIELD_DELETE_BACKWARD_WORD, 1));
        assert_eq!(state(&c), ("foo r".into(), 4));
        let c = at("foo bar", 7);
        press(&d, &c, of(FIELD_DELETE_BACKWARD_WORD));
        assert_eq!(state(&c), ("foo ".into(), 4));
        let c = at("abc\ndef", 1);
        press(&d, &c, of(FIELD_KILL_LINE_END));
        assert_eq!(state(&c), ("a\ndef".into(), 1));
        let c = at("abc\ndef", 3);
        press(&d, &c, of(FIELD_KILL_LINE_END));
        assert_eq!(state(&c), ("abcdef".into(), 3));
        let c = at("abc\ndef", 5);
        press(&d, &c, of(FIELD_KILL_LINE_START));
        assert_eq!(c.text().as_ref(), "abc\nef");
        let c = at("abc\ndef", 4);
        press(&d, &c, of(FIELD_KILL_LINE_START));
        assert_eq!(state(&c), ("abcdef".into(), 3));
    }

    #[test]
    fn yank_restores_last_kill() {
        let d = data();
        let c = at("hello", 0);
        press(&d, &c, of(FIELD_KILL_LINE_END));
        assert_eq!(state(&c), (String::new(), 0));
        press(&d, &c, of(FIELD_YANK));
        assert_eq!(state(&c), ("hello".into(), 5));

        let d = data();
        let c = at("hello world", 11);
        press(&d, &c, of(FIELD_DELETE_BACKWARD_WORD));
        assert_eq!(state(&c), ("hello ".into(), 6));
        press(&d, &c, of(FIELD_YANK));
        assert_eq!(state(&c), ("hello world".into(), 11));

        let d = data();
        let c = at("hello", 5);
        press(&d, &c, of(FIELD_KILL_LINE_START));
        assert_eq!(state(&c), (String::new(), 0));
        press(&d, &c, of(FIELD_YANK));
        assert_eq!(state(&c), ("hello".into(), 5));
    }

    #[test]
    fn kill_buffer_persists_across_set_text() {
        let d = data();
        let c = at("restore me", 0);
        press(&d, &c, of(FIELD_KILL_LINE_END));
        assert!(c.text().is_empty());
        c.set_text("/diff");
        c.set_text("");
        press(&d, &c, of(FIELD_YANK));
        assert_eq!(state(&c), ("restore me".into(), "restore me".len()));
    }

    #[test]
    fn a_placeholder_goes_whole_under_left_right_and_backspace() {
        let d = data();
        let c = at("ab", 1);
        d.draft.insert_element(&c, "[P]", "payload");
        assert_eq!(state(&c), ("a[P]b".into(), 4));
        assert_eq!(d.draft.expanded(&c), "apayloadb");
        press(&d, &c, of(FIELD_LEFT));
        assert_eq!(c.selection().extent_offset, 1);
        press(&d, &c, of(FIELD_RIGHT));
        assert_eq!(c.selection().extent_offset, 4);
        press(&d, &c, of(FIELD_DELETE_BACKWARD));
        assert_eq!(state(&c), ("ab".into(), 1));
        assert_eq!(d.draft.expanded(&c), "ab");
    }

    /// The messages a composer's key hook is told, as `KeyData::tell` records them.
    fn told(draft: ComposerDraft) -> (KeyData, Rc<std::cell::RefCell<Vec<ComposerMsg>>>) {
        let sent = Rc::new(std::cell::RefCell::new(Vec::new()));
        let record = sent.clone();
        let d = KeyData {
            draft,
            tell: Rc::new(move |msg| record.borrow_mut().push(msg)),
            ..data()
        };
        (d, sent)
    }

    // ORACLE chat_composer/embedded_input_tests.rs plain_text_composer_keeps_command_prefixes_literal
    #[test]
    fn a_plain_text_draft_keeps_its_command_prefixes_literal() {
        let (d, sent) = told(ComposerDraft::plain());
        for ch in ['!', '/', '?'] {
            let c = at("", 0);
            assert!(
                !typed_char(&d.draft, &*d.tell, &c, ch, None),
                "`{ch}` is the field's text"
            );
        }
        // a paste at the start that begins with `!` is text too, no shell mode
        let c = at("", 0);
        assert!(
            !d.draft
                .paste(&c, "!keep", |n| n.to_string(), |n| n.to_string())
        );
        c.insert("!keep");
        assert!(!d.draft.absorb_shell_prefix(&c));
        assert!(!d.draft.shell());
        assert_eq!(state(&c).0, "!keep");
        assert!(
            sent.borrow().is_empty(),
            "no Shell message: {:?}",
            sent.borrow()
        );
    }

    #[test]
    fn tab_submits_a_plain_text_draft_and_is_the_pages_in_the_chat() {
        let key = of(TAB);
        let (d, sent) = told(ComposerDraft::plain());
        let c = at("!keep this literal", 18);
        assert_eq!(composer_key(&d, &c, &key), Some(true));
        assert!(
            matches!(&sent.borrow()[..], [ComposerMsg::Submitted(t)] if &**t == "!keep this literal"),
            "{:?}",
            sent.borrow()
        );
        // the chat's Tab is the page's queue key: the composer tells nothing
        let (d, sent) = told(ComposerDraft::default());
        assert_eq!(composer_key(&d, &c, &key), Some(false));
        assert!(sent.borrow().is_empty());
    }

    fn keyed(table: &str, action: &str, keys: &[&str]) -> KeyData {
        let spec = codex_tui_core::keymap::KeymapSpec::default().with(table, action, keys);
        KeyData {
            keymap: Rc::new(Keymap::resolve_unchecked(&spec)),
            ..data()
        }
    }

    /// The restricted composer (`reconnect_tests.rs`): Enter and Tab are not
    /// edits, Enter tells the composer only for a command the mode passes and
    /// a key that expanded a paste ran none; every other key is an edit.
    mod restricted {
        use std::cell::RefCell;

        use super::*;

        fn told_by(mode: Restricted) -> (KeyData, Rc<RefCell<Vec<String>>>) {
            let told: Rc<RefCell<Vec<String>>> = Rc::default();
            let log = told.clone();
            let d = KeyData {
                tell: Rc::new(move |msg| log.borrow_mut().push(format!("{msg:?}"))),
                restricted: Some(mode),
                ..data()
            };
            (d, told)
        }

        /// Whether the restricted door took `key` (an edit is not its).
        fn key(d: &KeyData, c: &TextEditingController, key: KeyPress) -> bool {
            let mode = d.restricted.expect("a restricted composer");
            restricted_key(d, c, &key, mode) == Some(true)
        }

        fn enter() -> KeyPress {
            of(ENTER)
        }

        #[test]
        fn enter_runs_a_command_the_mode_passes_and_swallows_the_rest() {
            let (d, told) = told_by(Restricted::Disconnected);
            let c = at("/quit", 5);
            assert!(key(&d, &c, enter()));
            assert_eq!(told.borrow().len(), 1);
            let c = at("/status", 7);
            assert!(key(&d, &c, enter()));
            assert!(key(&d, &c, of(TAB)));
            assert_eq!(told.borrow().len(), 1, "nothing else was told");
            assert_eq!(c.text().as_ref(), "/status");
            let c = at("hello", 5);
            assert!(key(&d, &c, enter()));
            assert_eq!(told.borrow().len(), 1);
        }

        #[test]
        fn a_handoff_runs_nothing() {
            let (d, told) = told_by(Restricted::DraftHandoff);
            let c = at("/quit", 5);
            assert!(key(&d, &c, enter()));
            assert!(told.borrow().is_empty());
        }

        #[test]
        fn an_unavailable_thread_runs_its_recovery_commands() {
            let (d, told) = told_by(Restricted::UnavailableThread);
            let c = at("/new", 4);
            assert!(key(&d, &c, enter()));
            assert_eq!(told.borrow().len(), 1);
        }

        #[test]
        fn the_key_that_expands_a_paste_runs_no_command() {
            let (d, told) = told_by(Restricted::UnavailableThread);
            let c = at("", 0);
            let big = ["x"; 1001].concat();
            assert!(d.draft.paste(
                &c,
                &["/clear ", big.as_str()].concat(),
                |n| format!("[{n}]"),
                |n| format!("<{n}>")
            ));
            assert!(key(&d, &c, enter()));
            assert!(
                told.borrow().is_empty(),
                "the command is on the screen first"
            );
            assert!(c.text().starts_with("/clear xxx"));
            // the next Enter runs it
            assert!(key(&d, &c, enter()));
            assert_eq!(told.borrow().len(), 1);
        }

        /// `reconnect_edit_removes_deleted_attachment_and_paste_metadata`:
        /// Ctrl+U on an offline draft is the edit it is.
        #[test]
        fn other_keys_are_edits() {
            let (d, told) = told_by(Restricted::Disconnected);
            let c = at("abc def", 7);
            // the door lets it by, and the composer's own edit takes it
            assert!(!key(&d, &c, of(FIELD_KILL_LINE_START)));
            assert!(edit_key(&d, &c, &of(FIELD_KILL_LINE_START), None));
            assert_eq!(c.text().as_ref(), "");
            assert!(told.borrow().is_empty());
        }

        #[test]
        fn up_is_the_carets_not_the_historys() {
            let (d, _) = told_by(Restricted::Disconnected);
            let c = at("abc", 3);
            assert!(!walk_history(&d, &c, true));
        }
    }

    #[test]
    fn enter_and_tab_are_left_to_the_field_and_the_page_by_default() {
        let d = data();
        let c = at("abc", 3);
        // Enter is `editor.insert_newline` too, but Composer shadows it: it submits
        assert_eq!(composer_key(&d, &c, &of(ENTER)), Some(false));
        // the page's own binding queues
        assert_eq!(composer_key(&d, &c, &of(TAB)), Some(false));
        assert_eq!(composer_key(&d, &c, &crate::keymap::typed('a')), None);
    }

    #[test]
    fn a_remapped_submit_frees_enter_for_the_line_break_and_swallows_tab_when_unbound() {
        let d = keyed("composer", "submit", &["ctrl-s"]);
        let c = at("ab", 2);
        assert_eq!(composer_key(&d, &c, &of(ENTER)), None);
        press(&d, &c, of(ENTER));
        assert_eq!(c.text().as_ref(), "ab\n");
        let d = keyed("composer", "queue", &["f5"]);
        assert_eq!(composer_key(&d, &c, &of(TAB)), Some(true));
    }

    #[test]
    fn a_key_rebound_to_an_editor_action_is_carried_out_as_it_and_the_old_key_does_nothing() {
        let d = keyed("editor", "move_left", &["f2"]);
        let c = at("abc", 2);
        press(&d, &c, press_of(&Stroke::parse("f2").unwrap()).unwrap());
        assert_eq!(state(&c), ("abc".into(), 1));
        // `left` is no longer bound to anything: it does not fall back
        press(&d, &c, of(FIELD_LEFT));
        assert_eq!(state(&c), ("abc".into(), 1));
    }

    #[test]
    fn kill_whole_line_is_unbound_until_the_user_binds_it() {
        let d = data();
        let c = at("abc\ndef\nghi", 5);
        press(&d, &c, of(FIELD_KILL_LINE_START));
        assert_eq!(
            c.text().as_ref(),
            "abc\nef\nghi",
            "ctrl+u is kill_line_start"
        );
        let d = keyed("editor", "kill_whole_line", &["ctrl-o"]);
        let c = at("abc\ndef\nghi", 5);
        press(&d, &c, of_keyed(&d, EDITOR_KILL_WHOLE_LINE));
        assert_eq!(c.text().as_ref(), "abc\nghi");
        let c = at("abc\ndef", 5);
        press(&d, &c, of_keyed(&d, EDITOR_KILL_WHOLE_LINE));
        assert_eq!(c.text().as_ref(), "abc");
        // the killed line is in the ring
        press(&d, &c, of(FIELD_YANK));
        assert_eq!(c.text().as_ref(), "abc\ndef");
    }
    /// What the keys of the clipboard image and of the remote image rows ask
    /// of the composer: the messages it is told, in order.
    mod images {
        use std::cell::{Cell, RefCell};
        use std::time::Duration;

        use super::*;

        struct Rig {
            data: KeyData,
            told: Rc<RefCell<Vec<String>>>,
            now: Rc<Cell<Duration>>,
        }

        fn image(n: usize) -> String {
            ["[Image #", &n.to_string(), "]"].concat()
        }

        fn rig() -> Rig {
            let told: Rc<RefCell<Vec<String>>> = Rc::default();
            let now: Rc<Cell<Duration>> = Rc::default();
            let (log, clock) = (told.clone(), now.clone());
            let data = KeyData {
                tell: Rc::new(move |msg| log.borrow_mut().push(format!("{msg:?}"))),
                burst: Some(BurstEnv {
                    now: Rc::new(move || clock.get()),
                    arm: Rc::new(|| {}),
                    label: Rc::new(|n| ["[Pasted Content ", &n.to_string(), " chars]"].concat()),
                    image: Rc::new(image),
                }),
                ..data()
            };
            Rig { data, told, now }
        }

        fn urls(n: usize) -> Vec<String> {
            (1..=n)
                .map(|i| ["https://example.com/", &i.to_string(), ".png"].concat())
                .collect()
        }

        fn key(code: KeyPress) -> KeyPress {
            code
        }

        #[test]
        fn ctrl_v_and_alt_v_ask_for_the_clipboard_image_once_per_quarter_second() {
            let r = rig();
            assert!(before_search(&r.data, &of(PASTE_IMAGE)));
            assert_eq!(*r.told.borrow(), ["BeforeSearch(PasteImage)"]);
            // the repeat of a held key, 100 ms later, is swallowed
            r.now.set(Duration::from_millis(100));
            assert!(before_search(&r.data, &of(PASTE_IMAGE)));
            assert_eq!(r.told.borrow().len(), 1);
            // a press 250 ms after the last asks again, whichever its modifier
            r.now.set(Duration::from_millis(350));
            let alt = nth(PASTE_IMAGE, 1);
            assert!(before_search(&r.data, &alt));
            assert_eq!(r.told.borrow().len(), 2);
        }

        #[test]
        fn a_plain_v_is_typed_and_ctrl_v_outside_an_app_is_the_fields() {
            let r = rig();
            assert!(!before_search(&r.data, &crate::keymap::typed('v')));
            assert!(r.told.borrow().is_empty());
            let outside = KeyData {
                burst: None,
                ..data()
            };
            assert!(!before_search(&outside, &of(PASTE_IMAGE)));
        }

        #[test]
        fn up_at_the_start_selects_the_last_row_and_up_down_move_it() {
            let r = rig();
            let c = at("hello", 0);
            r.data.draft.set_remote_images(&c, urls(3), image);
            assert!(remote_key(&r.data, &c, &key(of(FIELD_UP))));
            assert_eq!(r.data.draft.remote_selected(), Some(2));
            assert!(remote_key(&r.data, &c, &key(of(FIELD_UP))));
            assert!(remote_key(&r.data, &c, &key(of(FIELD_UP))));
            assert!(remote_key(&r.data, &c, &key(of(FIELD_UP))));
            assert_eq!(
                r.data.draft.remote_selected(),
                Some(0),
                "the first row stays"
            );
            assert!(remote_key(&r.data, &c, &key(of(FIELD_DOWN))));
            assert_eq!(r.data.draft.remote_selected(), Some(1));
            assert!(remote_key(&r.data, &c, &key(of(FIELD_DOWN))));
            assert!(remote_key(&r.data, &c, &key(of(FIELD_DOWN))));
            assert_eq!(r.data.draft.remote_selected(), None, "past the last row");
            assert!(r.told.borrow().iter().all(|m| m == "Remote"));
        }

        #[test]
        fn up_elsewhere_than_the_start_and_down_with_no_row_selected_are_not_its() {
            let r = rig();
            let c = at("hello", 2);
            r.data.draft.set_remote_images(&c, urls(2), image);
            assert!(!remote_key(&r.data, &c, &key(of(FIELD_UP))));
            assert!(!remote_key(&r.data, &c, &key(of(FIELD_DOWN))));
            let c = at("hello", 0);
            assert!(!remote_key(&r.data, &c, &key(of(FIELD_DOWN))));
            assert_eq!(r.data.draft.remote_selected(), None);
            // a selected stretch of text keeps Up for the text
            c.set_selection(TextSelection {
                base_offset: 0,
                extent_offset: 3,
            });
            assert!(!remote_key(&r.data, &c, &key(of(FIELD_UP))));
            assert!(r.told.borrow().is_empty());
        }

        #[test]
        fn delete_removes_the_selected_row_and_renumbers_the_local_images() {
            let r = rig();
            let c = at("", 0);
            r.data.draft.set_remote_images(&c, urls(2), image);
            r.data
                .draft
                .attach_image(&c, std::path::PathBuf::from("/tmp/a.png"), image);
            assert_eq!(&*c.text(), "[Image #3]");
            c.set_selection(TextSelection::collapsed(0));
            assert!(remote_key(&r.data, &c, &key(of(FIELD_UP))));
            assert!(remote_key(&r.data, &c, &key(of(FIELD_UP))));
            assert!(remote_key(&r.data, &c, &key(of(FIELD_DELETE_FORWARD))));
            assert_eq!(r.data.draft.remote_images(), urls(2)[1..].to_vec());
            assert_eq!(
                &*c.text(),
                "[Image #2]",
                "the local image follows the one row"
            );
            assert_eq!(r.data.draft.remote_selected(), Some(0));
            // Backspace removes the last one; no row is left to select
            assert!(remote_key(&r.data, &c, &key(of(FIELD_DELETE_BACKWARD))));
            assert_eq!(r.data.draft.remote_count(), 0);
            assert_eq!(r.data.draft.remote_selected(), None);
            assert_eq!(&*c.text(), "[Image #1]");
        }

        #[test]
        fn any_other_key_ends_the_selection_and_goes_on() {
            let r = rig();
            let c = at("", 0);
            r.data.draft.set_remote_images(&c, urls(1), image);
            assert!(remote_key(&r.data, &c, &key(of(FIELD_UP))));
            assert!(!remote_key(&r.data, &c, &crate::keymap::typed('a')));
            assert_eq!(r.data.draft.remote_selected(), None);
        }
    }
}
