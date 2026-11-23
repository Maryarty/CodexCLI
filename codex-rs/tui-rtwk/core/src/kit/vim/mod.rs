//! The composer's modal (Vim) editing, pure: the state machine of
//! `bottom_pane/textarea/{vim,vim_commands,vim_search}.rs` over a text the
//! caller owns (`VimText`: the draft's text, caret, atomic elements and
//! register). No rtwk, no keys: the view resolves a key through the keymap
//! (Vim contexts) into an `Input` and hands it here; every edit goes back
//! through `VimText::replace`, so what the draft keeps beside its text
//! (placeholders, images) follows the edit.
//!
//! Counts (`3dw`) are not in the oracle's engine and not here either.

mod engine;
pub mod history;
mod motions;

use std::ops::Range;
use std::rc::Rc;

pub use engine::{Persistent, Vim};
pub use history::UndoHistory;

/// What Vim mode the draft is in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VimMode {
    /// Printable keys are movements, operators and mode changes.
    Normal,
    /// The regular editing keys until Escape.
    #[default]
    Insert,
    /// Typed characters overwrite existing graphemes.
    Replace,
}

impl VimMode {
    /// The footer label's word (`Vim: Normal`).
    pub fn label(self) -> &'static str {
        match self {
            VimMode::Normal => "Normal",
            VimMode::Insert => "Insert",
            VimMode::Replace => "Replace",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operator {
    Delete,
    Yank,
    Change,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VimScope {
    Inner,
    Around,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Object {
    Word,
    BigWord,
    Parentheses,
    Brackets,
    Braces,
    DoubleQuote,
    SingleQuote,
    Backtick,
}

/// The motions an operator takes (`d` + `w`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Motion {
    Left,
    Right,
    Up,
    Down,
    WordForward,
    WordBackward,
    WordEnd,
    LineStart,
    LineEnd,
}

/// Line-local character motions (`f`, `F`, `t`, `T`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Find {
    Forward,
    Backward,
    TillForward,
    TillBackward,
}

/// What the next key completes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum VimPending {
    #[default]
    None,
    Operator(Operator),
    TextObject {
        operator: Operator,
        scope: VimScope,
    },
    Replace,
    Find {
        motion: Find,
        operator: Option<Operator>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum VimDirection {
    #[default]
    Forward,
    Backward,
}

impl VimDirection {
    pub fn reversed(self) -> VimDirection {
        match self {
            VimDirection::Forward => VimDirection::Backward,
            VimDirection::Backward => VimDirection::Forward,
        }
    }
}

/// A literal search (`/`, `?`, `n`, `N`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SearchQuery {
    pub text: String,
    pub direction: VimDirection,
}

/// How the register's text pastes: after the character, or as lines.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RegisterKind {
    #[default]
    Characterwise,
    Linewise,
}

/// The register: the last kill or yank, shared with the editor's kill ring.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Register {
    pub text: String,
    pub kind: RegisterKind,
}

/// A Normal-mode command (the `vim_normal` keymap context, `undo` and
/// `redo` apart: the draft's whole-text history owns those).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Normal {
    EnterInsert,
    AppendAfterCursor,
    AppendLineEnd,
    InsertLineStart,
    OpenLineBelow,
    OpenLineAbove,
    EnterReplaceMode,
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    MoveWordForward,
    MoveWordBackward,
    MoveWordEnd,
    MoveLineStart,
    MoveLineEnd,
    Find(Find),
    JumpTop,
    JumpBottom,
    DeleteChar,
    ReplaceChar,
    RepeatLastChange,
    SubstituteChar,
    DeleteToLineEnd,
    ChangeToLineEnd,
    YankLine,
    PasteAfter,
    StartOperator(Operator),
    CancelOperator,
}

/// What a key is while an operator waits (the `vim_operator` context, in
/// the oracle's order of asking).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperatorKey {
    /// `dd` / `yy` / `cc`: the operator's own line.
    Line,
    Cancel,
    VimScope(VimScope),
    Motion(Motion),
    Find(Find),
    JumpTop,
    JumpBottom,
}

/// One key as the engine reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Input {
    Normal(Normal),
    Operator(OperatorKey),
    /// The object a `Pending::TextObject` waits for.
    Object(Object),
    /// The text-object context's cancel key.
    Cancel,
    /// The character a `Pending::Replace` / `Pending::Find` waits for
    /// (`None`: a key that is no character).
    Char(Option<char>),
    /// A key no command is bound to: a pending command ends.
    Other,
}

/// The editor's own actions (the `editor` keymap context), which the host
/// carries out and Insert mode records for `.`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorAction {
    DeleteBackward,
    DeleteForward,
    DeleteBackwardWord,
    DeleteForwardWord,
    KillLineStart,
    KillLine,
    KillLineEnd,
    Yank,
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    MoveWordLeft,
    MoveWordRight,
    MoveLineStart { move_up_at_bol: bool },
    MoveLineEnd { move_down_at_eol: bool },
}

/// The text a Vim command works on, owned by the draft. Provided methods are
/// what the engine derives (lines, atomic boundaries, words).
pub trait VimText {
    fn text(&self) -> Rc<str>;
    fn cursor(&self) -> usize;
    /// The caret to `pos`, moved off the inside of an element.
    fn set_cursor(&mut self, pos: usize);
    /// The atomic ranges (placeholders, images): never split.
    fn elements(&self) -> Vec<Range<usize>>;
    /// Replaces `range`; the caret follows as an edit moves it (inside the
    /// range: to the end of the new text), elements inside the range go.
    fn replace(&mut self, range: Range<usize>, with: &str);
    /// The start of the grapheme before `pos`.
    fn prev_grapheme(&self, pos: usize) -> usize;
    /// The end of the grapheme at `pos`.
    fn next_grapheme(&self, pos: usize) -> usize;
    /// The ends of the graphemes of `s`, in order.
    fn grapheme_ends(&self, s: &str) -> Vec<usize>;
    fn register(&self) -> Register;
    /// Stores `range` (its text and the elements in it) as the register.
    fn store_register(&mut self, range: Range<usize>, kind: RegisterKind);
    /// Sets the kind of the register as it is.
    fn set_register_kind(&mut self, kind: RegisterKind);
    /// The register's elements go back into the text it was pasted as, from `start`.
    fn paste_register_elements(&mut self, start: usize);
    /// The caret one visual line up or down (the wrapped lines and the
    /// column the caret keeps).
    fn move_vertical(&mut self, down: bool);
    /// The editor's own action; `false` when it did nothing.
    fn editor(&mut self, action: EditorAction) -> bool;
}

/// Whether the text's `pos` starts a grapheme that is no element's inside.
pub(crate) fn is_command_target(t: &dyn VimText, pos: usize) -> bool {
    let text = t.text();
    if t.elements().iter().any(|e| e.contains(&pos)) {
        return false;
    }
    pos == 0 || pos >= text.len() || t.next_grapheme(t.prev_grapheme(pos)) == pos
}

/// The ranges where `query` is in the text, each to the grapheme ends.
pub fn matching_ranges(t: &dyn VimText, query: &str) -> Vec<Range<usize>> {
    let text = t.text();
    let mut out = Vec::new();
    if query.is_empty() {
        return out;
    }
    let mut start = 0;
    while start < text.len() {
        let next = t.next_grapheme(start).max(start + 1);
        if text[start..].starts_with(query) {
            let mut end = next;
            while end < start + query.len() && end < text.len() {
                end = t.next_grapheme(end).max(end + 1);
            }
            out.push(start..end.min(text.len()));
        }
        start = next;
        while start < text.len() && !text.is_char_boundary(start) {
            start += 1;
        }
    }
    out
}
