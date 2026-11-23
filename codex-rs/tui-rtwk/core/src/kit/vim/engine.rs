//! The Vim state machine: modes, pending commands, the transactions that
//! `.` replays, the replace-mode recovery steps and the draft-local search
//! (`textarea.rs` `handle_vim_*`, `textarea/vim_commands.rs`, `vim_search.rs`).

use std::ops::Range;

use super::motions::{self, bol, bol_of, eol, eol_of, expand_to_elements, next_atomic, prev_atomic};
use super::{
    VimDirection, EditorAction, Find, Input, VimMode, Motion, Normal, Object, Operator, OperatorKey,
    VimPending, RegisterKind, VimScope, SearchQuery, VimText, is_command_target, matching_ranges,
};

#[derive(Clone, Copy, Debug)]
enum InsertAt {
    Cursor,
    AfterCursor,
    LineStart,
    LineEnd,
    OpenAbove,
    OpenBelow,
}

#[derive(Clone, Debug)]
enum VimTarget {
    Character,
    Line,
    LineEnd,
    Motion(Motion),
    Search(SearchQuery),
    Object { scope: VimScope, object: Object },
    Find { motion: Find, target: char },
    BufferJump { last: bool },
}

#[derive(Clone, Debug)]
enum VimAction {
    Insert(InsertAt),
    EnterReplace,
    RestoreReplaced,
    Delete(VimTarget),
    Change(VimTarget),
    Replace(char),
    PasteAfter,
    /// The register as it was when the yank key was pressed.
    Yank(String),
    Editor(EditorAction),
}

/// One step of a change `.` repeats.
#[derive(Clone, Debug)]
pub struct VimEdit(EditStep);

#[derive(Clone, Debug)]
enum EditStep {
    VimAction(VimAction),
    Text(String),
}

#[derive(Debug)]
struct ReplaceStep {
    /// Backspace also retraces attachments skipped before the replacement.
    cursor_before: usize,
    start: usize,
    inserted_len: usize,
    original: String,
}

#[derive(Debug, Default)]
struct Commands {
    pending_change: Vec<VimEdit>,
    last_change: Vec<VimEdit>,
    changed: bool,
    replaying: bool,
    replace_steps: Vec<ReplaceStep>,
}

/// What lasts when the same draft is put back (a history preview that was
/// canceled): the change `.` repeats and the last search.
#[derive(Debug, Default)]
pub struct Persistent {
    commands: Commands,
    search: SearchQuery,
}

/// The modal editing state of one draft.
#[derive(Debug, Default)]
pub struct Vim {
    enabled: bool,
    mode: VimMode,
    pending: VimPending,
    /// A `/` or `?` query is being typed (its text is the view's).
    searching: Option<VimDirection>,
    last_search: SearchQuery,
    commands: Commands,
    /// The single-line field: no line is opened and no newline is typed.
    single_line: bool,
}

impl Vim {
    pub fn new() -> Vim {
        Vim::default()
    }

    pub fn single_line(mut self) -> Vim {
        self.single_line = true;
        self
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Turning it on enters Normal mode, off Insert; a command waiting does not survive.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        self.pending = VimPending::None;
        self.searching = None;
        self.last_search = SearchQuery::default();
        self.commands = Commands::default();
        self.mode = if enabled { VimMode::Normal } else { VimMode::Insert };
    }

    pub fn mode(&self) -> VimMode {
        self.mode
    }

    /// The footer's word for the mode; none while Vim is off.
    pub fn label(&self) -> Option<&'static str> {
        self.enabled.then(|| self.mode.label())
    }

    pub fn pending(&self) -> VimPending {
        self.pending
    }

    pub fn is_normal(&self) -> bool {
        self.enabled && self.mode == VimMode::Normal
    }

    pub fn is_replace(&self) -> bool {
        self.enabled && self.mode == VimMode::Replace
    }

    /// Whether typing goes in as characters at the caret (paste-burst
    /// detection is off in Normal mode).
    pub fn types_text(&self) -> bool {
        !self.enabled || matches!(self.mode, VimMode::Insert | VimMode::Replace)
    }

    /// Whether the caret is the block-less insert one (Insert mode).
    pub fn uses_insert_cursor(&self) -> bool {
        self.enabled && self.mode == VimMode::Insert
    }

    /// `.` repeats nothing until the next change: an edit outside the
    /// keys (an image from the clipboard) is not one to repeat
    /// (`begin_direct_vim_edit` clears `last_change`).
    pub fn forget_repeat(&mut self) {
        self.commands.last_change.clear();
    }

    /// A command, a query or a text object waits for its next key.
    pub fn is_operator_pending(&self) -> bool {
        self.searching.is_some() || self.pending != VimPending::None
    }

    /// Escape is the editor's, not the turn's: Insert/Replace mode or a command waiting.
    pub fn owns_escape(&self) -> bool {
        self.enabled && (self.mode != VimMode::Normal || self.is_operator_pending())
    }

    pub fn searching(&self) -> Option<VimDirection> {
        self.searching
    }

    pub fn last_search(&self) -> &SearchQuery {
        &self.last_search
    }

    /// The text was replaced as a whole: what waited and what `.` recorded is gone.
    pub fn text_replaced(&mut self) {
        self.pending = VimPending::None;
        self.searching = None;
        self.last_search = SearchQuery::default();
        self.commands = Commands::default();
    }

    pub fn swap_persistent(&mut self, state: &mut Persistent) {
        std::mem::swap(&mut self.commands, &mut state.commands);
        std::mem::swap(&mut self.last_search, &mut state.search);
    }

    /// Normal mode's key: the command it is, given what waits.
    pub fn input(&mut self, t: &mut dyn VimText, input: Input) {
        if !self.enabled || self.mode != VimMode::Normal {
            return;
        }
        match std::mem::take(&mut self.pending) {
            VimPending::None => {
                if let Input::Normal(command) = input {
                    self.normal(t, command);
                }
            }
            VimPending::Operator(op) => {
                if let Input::Operator(key) = input {
                    self.operator(t, op, key);
                }
            }
            VimPending::TextObject { operator, scope } => {
                if let Input::Object(object) = input {
                    self.object(t, operator, scope, object);
                }
            }
            pending @ (VimPending::Replace | VimPending::Find { .. }) => {
                if let Input::Char(ch) = input {
                    self.pending_char(t, pending, ch);
                }
            }
        }
    }

    // ---- modes --------------------------------------------------------------

    /// Escape in Insert or Replace mode: back to Normal, one grapheme left.
    pub fn escape(&mut self, t: &mut dyn VimText) {
        if !self.enabled || self.mode == VimMode::Normal {
            return;
        }
        self.leave_insert(t);
        self.finish_pending_change();
    }

    fn leave_insert(&mut self, t: &mut dyn VimText) {
        let b = bol(t);
        if t.cursor() > b {
            let to = prev_atomic(t, t.cursor()).max(b);
            t.set_cursor(to);
        }
        self.enter_normal(t);
    }

    /// Insert mode (a submit, a new draft): a change starts recording.
    pub fn enter_insert(&mut self, t: &mut dyn VimText) {
        if !self.enabled {
            return;
        }
        self.mode = VimMode::Insert;
        self.pending = VimPending::None;
        self.commands.replace_steps.clear();
        self.searching = None;
        if self.commands.pending_change.is_empty() && !self.commands.replaying {
            self.start_edit(t, VimAction::Insert(InsertAt::Cursor));
        }
    }

    pub fn enter_normal(&mut self, _t: &mut dyn VimText) {
        if self.enabled {
            self.mode = VimMode::Normal;
            self.pending = VimPending::None;
            self.searching = None;
            self.commands.replace_steps.clear();
        }
    }

    // ---- recording ----------------------------------------------------------

    /// Text typed or pasted in Insert or Replace mode, for `.`.
    pub fn record_text(&mut self, text: &str) {
        let c = &mut self.commands;
        if !self.enabled
            || self.mode == VimMode::Normal
            || c.replaying
            || c.pending_change.is_empty()
            || text.is_empty()
        {
            return;
        }
        if self.mode == VimMode::Insert
            && let Some(VimEdit(EditStep::Text(pending))) = c.pending_change.last_mut()
        {
            pending.push_str(text);
        } else {
            c.pending_change.push(VimEdit(EditStep::Text(text.to_owned())));
        }
        c.changed = true;
    }

    /// An editor action (delete, kill, move) in Insert or Replace mode: the
    /// host carries it out and `.` records it when it changed something.
    pub fn insert_action(&mut self, t: &mut dyn VimText, action: EditorAction) -> bool {
        if matches!(action, EditorAction::Yank) && t.register().text.is_empty() {
            return false;
        }
        let deletion = matches!(
            action,
            EditorAction::DeleteBackward
                | EditorAction::DeleteForward
                | EditorAction::DeleteBackwardWord
                | EditorAction::DeleteForwardWord
                | EditorAction::KillLineStart
                | EditorAction::KillLine
                | EditorAction::KillLineEnd
        );
        let recording = self.enabled
            && self.mode != VimMode::Normal
            && !self.commands.replaying
            && !self.commands.pending_change.is_empty();
        let prior_len = t.text().len();
        let recorded = if matches!(action, EditorAction::Yank) {
            VimAction::Yank(t.register().text)
        } else {
            VimAction::Editor(action)
        };
        if !self.apply_action(t, recorded.clone()) {
            return false;
        }
        let changed =
            t.text().len() != prior_len || matches!(action, EditorAction::Yank);
        if recording && (changed || !deletion) {
            self.commands.pending_change.push(VimEdit(EditStep::VimAction(recorded)));
        }
        self.commands.changed |= recording && changed;
        true
    }

    /// Typed text in Replace mode overwrites; Insert mode inserts it.
    pub fn type_text(&mut self, t: &mut dyn VimText, text: &str) {
        self.insert_str(t, text);
        self.record_text(text);
    }

    fn insert_str(&mut self, t: &mut dyn VimText, text: &str) {
        if self.is_replace() {
            self.replace_text(t, text);
        } else {
            let at = t.cursor();
            t.replace(at..at, text);
        }
    }

    /// Backspace in Replace mode puts the overwritten character back.
    pub fn replace_backspace(&mut self, t: &mut dyn VimText) -> bool {
        self.is_replace() && self.insert_action_restore(t)
    }

    fn insert_action_restore(&mut self, t: &mut dyn VimText) -> bool {
        let recording = !self.commands.replaying && !self.commands.pending_change.is_empty();
        if !self.apply_action(t, VimAction::RestoreReplaced) {
            return false;
        }
        if recording {
            self.commands
                .pending_change
                .push(VimEdit(EditStep::VimAction(VimAction::RestoreReplaced)));
            self.commands.changed = true;
        }
        true
    }

    fn replace_text(&mut self, t: &mut dyn VimText, text: &str) {
        let mut from = 0;
        for end in t.grapheme_ends(text) {
            let grapheme = &text[from..end];
            from = end;
            let cursor_before = t.cursor();
            while let Some(e) = t.elements().into_iter().find(|e| e.start == t.cursor()) {
                t.set_cursor(e.end);
            }
            let start = t.cursor();
            let doc = t.text();
            let end = if grapheme == "\n" || start >= eol_of(&doc, start) {
                start
            } else {
                next_atomic(t, start)
            };
            let original = doc[start..end].to_string();
            t.replace(start..end, grapheme);
            self.commands.replace_steps.push(ReplaceStep {
                cursor_before,
                start,
                inserted_len: grapheme.len(),
                original,
            });
        }
    }

    fn restore_replaced(&mut self, t: &mut dyn VimText) -> bool {
        let steps = &mut self.commands.replace_steps;
        let Some(step) = steps.pop().filter(|s| t.cursor() == s.start + s.inserted_len) else {
            steps.clear();
            return false;
        };
        t.replace(step.start..step.start + step.inserted_len, &step.original);
        t.set_cursor(step.cursor_before);
        true
    }

    fn start_edit(&mut self, t: &mut dyn VimText, action: VimAction) -> bool {
        let prior_len = t.text().len();
        self.commands.pending_change = vec![VimEdit(EditStep::VimAction(action.clone()))];
        self.commands.changed = false;
        if !self.apply_action(t, action.clone()) {
            self.commands.pending_change.clear();
            return false;
        }
        self.commands.changed = t.text().len() != prior_len || matches!(action, VimAction::Replace(_));
        if self.mode == VimMode::Normal {
            self.finish_pending_change();
        }
        true
    }

    fn finish_pending_change(&mut self) {
        let c = &mut self.commands;
        if c.changed {
            c.last_change = std::mem::take(&mut c.pending_change);
        } else {
            c.pending_change.clear();
        }
        c.changed = false;
    }

    fn repeat(&mut self, t: &mut dyn VimText) {
        if self.commands.last_change.is_empty() {
            return;
        }
        let edits = self.commands.last_change.clone();
        self.commands.replaying = true;
        for VimEdit(step) in edits {
            let done = match step {
                EditStep::VimAction(action) => self.apply_action(t, action),
                EditStep::Text(text) => {
                    if self.mode == VimMode::Normal {
                        false
                    } else {
                        self.insert_str(t, &text);
                        true
                    }
                }
            };
            if !done {
                break;
            }
        }
        if self.mode != VimMode::Normal {
            self.leave_insert(t);
        }
        self.pending = VimPending::None;
        self.commands.replaying = false;
    }

    // ---- the actions ----------------------------------------------------------

    fn apply_action(&mut self, t: &mut dyn VimText, action: VimAction) -> bool {
        if !matches!(action, VimAction::RestoreReplaced | VimAction::Yank(_)) {
            self.commands.replace_steps.clear();
        }
        let prior_len = t.text().len();
        match action {
            VimAction::EnterReplace => self.mode = VimMode::Replace,
            VimAction::RestoreReplaced => return self.restore_replaced(t),
            VimAction::Insert(at) => {
                match at {
                    InsertAt::Cursor => {}
                    InsertAt::AfterCursor => t.set_cursor(next_atomic(t, t.cursor())),
                    InsertAt::LineStart => t.set_cursor(motions::first_non_blank(t)),
                    InsertAt::LineEnd => t.set_cursor(eol(t)),
                    InsertAt::OpenAbove => {
                        let b = bol(t);
                        t.replace(b..b, "\n");
                        t.set_cursor(b);
                    }
                    InsertAt::OpenBelow => {
                        let e = eol(t);
                        let insert_at = if e < prior_len { e + 1 } else { e };
                        t.replace(insert_at..insert_at, "\n");
                        t.set_cursor(if e < prior_len { insert_at } else { insert_at + 1 });
                    }
                }
                self.mode = VimMode::Insert;
            }
            VimAction::Delete(target) => return self.operate(t, Operator::Delete, target, prior_len),
            VimAction::Change(target) => return self.operate(t, Operator::Change, target, prior_len),
            VimAction::Replace(ch) => {
                if t.cursor() >= eol(t) {
                    return false;
                }
                let start = t.cursor();
                let range = expand_to_elements(t, start..next_atomic(t, start));
                t.replace(range, &ch.to_string());
                t.set_cursor(start + usize::from(ch == '\n'));
            }
            VimAction::PasteAfter => {
                self.paste_after(t);
                return t.text().len() != prior_len;
            }
            VimAction::Yank(text) => {
                if text.is_empty() {
                    return true;
                }
                let replaying = std::mem::replace(&mut self.commands.replaying, true);
                self.insert_str(t, &text);
                self.commands.replaying = replaying;
            }
            VimAction::Editor(action) => {
                t.editor(action);
            }
        }
        true
    }

    fn operate(
        &mut self,
        t: &mut dyn VimText,
        op: Operator,
        target: VimTarget,
        prior_len: usize,
    ) -> bool {
        match target {
            VimTarget::Character => {
                if t.cursor() < eol(t) {
                    let cursor = t.cursor();
                    let end = next_atomic(t, cursor);
                    self.kill(t, cursor..end, RegisterKind::Characterwise);
                }
                if op == Operator::Change {
                    self.mode = VimMode::Insert;
                }
            }
            VimTarget::Line => {
                if op == Operator::Delete {
                    let range = motions::current_line_with_newline(t);
                    self.kill(t, range, RegisterKind::Linewise);
                } else {
                    let line = bol(t)..eol(t);
                    self.kill(t, line, RegisterKind::Linewise);
                    self.mode = VimMode::Insert;
                }
            }
            VimTarget::LineEnd => {
                let line_end = eol(t);
                if t.cursor() < line_end {
                    let cursor = t.cursor();
                    self.kill(t, cursor..line_end, RegisterKind::Characterwise);
                }
                if op == Operator::Change {
                    self.mode = VimMode::Insert;
                }
            }
            VimTarget::Motion(motion) => self.apply_operator(t, op, motion),
            VimTarget::Search(query) => {
                if !self.search(t, &query, Some(op)) {
                    return false;
                }
            }
            VimTarget::Object { scope, object } => {
                let Some(range) = motions::object_range(t, object, scope) else {
                    return false;
                };
                self.operate_range(t, op, range);
            }
            VimTarget::Find { motion, target } => {
                if !self.find(t, motion, Some(op), target) {
                    return false;
                }
            }
            VimTarget::BufferJump { last } => self.jump_to_line(t, last, Some(op)),
        }
        if op == Operator::Change {
            return self.mode == VimMode::Insert;
        }
        t.text().len() != prior_len
    }

    // ---- Normal-mode commands -----------------------------------------------------

    fn normal(&mut self, t: &mut dyn VimText, command: Normal) {
        let edit = |s: &mut Vim, t: &mut dyn VimText, at| {
            s.start_edit(t, VimAction::Insert(at));
        };
        match command {
            Normal::EnterInsert => edit(self, t, InsertAt::Cursor),
            Normal::AppendAfterCursor => edit(self, t, InsertAt::AfterCursor),
            Normal::AppendLineEnd => edit(self, t, InsertAt::LineEnd),
            Normal::InsertLineStart => edit(self, t, InsertAt::LineStart),
            Normal::OpenLineBelow if !self.single_line => edit(self, t, InsertAt::OpenBelow),
            Normal::OpenLineAbove if !self.single_line => edit(self, t, InsertAt::OpenAbove),
            Normal::OpenLineBelow | Normal::OpenLineAbove => {}
            Normal::EnterReplaceMode => {
                self.start_edit(t, VimAction::EnterReplace);
            }
            Normal::MoveLeft => t.set_cursor(prev_atomic(t, t.cursor())),
            Normal::MoveRight => t.set_cursor(next_atomic(t, t.cursor())),
            Normal::MoveUp => t.move_vertical(false),
            Normal::MoveDown => t.move_vertical(true),
            Normal::MoveWordForward => t.set_cursor(motions::beginning_of_next_word(t)),
            Normal::MoveWordBackward => t.set_cursor(motions::beginning_of_previous_word(t)),
            Normal::MoveWordEnd => t.set_cursor(motions::word_end_cursor(t)),
            Normal::MoveLineStart => t.set_cursor(bol(t)),
            Normal::MoveLineEnd => t.set_cursor(motions::line_end_cursor(t)),
            Normal::DeleteChar => {
                self.start_edit(t, VimAction::Delete(VimTarget::Character));
            }
            Normal::SubstituteChar => {
                self.start_edit(t, VimAction::Change(VimTarget::Character));
            }
            Normal::DeleteToLineEnd => {
                self.start_edit(t, VimAction::Delete(VimTarget::LineEnd));
            }
            Normal::ChangeToLineEnd => {
                self.start_edit(t, VimAction::Change(VimTarget::LineEnd));
            }
            Normal::YankLine => self.yank_current_line(t),
            Normal::PasteAfter => {
                self.start_edit(t, VimAction::PasteAfter);
            }
            Normal::StartOperator(op) => self.pending = VimPending::Operator(op),
            Normal::CancelOperator => {
                self.pending = VimPending::None;
                self.searching = None;
            }
            Normal::ReplaceChar => {
                if t.cursor() < eol(t) {
                    self.pending = VimPending::Replace;
                }
            }
            Normal::RepeatLastChange => self.repeat(t),
            Normal::Find(motion) => {
                self.pending = VimPending::Find {
                    motion,
                    operator: None,
                };
            }
            Normal::JumpTop => self.jump_to_line(t, false, None),
            Normal::JumpBottom => self.jump_to_line(t, true, None),
        }
    }

    fn operator(&mut self, t: &mut dyn VimText, op: Operator, key: OperatorKey) {
        match key {
            OperatorKey::Line => match op {
                Operator::Delete => {
                    self.start_edit(t, VimAction::Delete(VimTarget::Line));
                }
                Operator::Yank => self.yank_current_line(t),
                Operator::Change => {
                    self.start_edit(t, VimAction::Change(VimTarget::Line));
                }
            },
            OperatorKey::Cancel => {}
            OperatorKey::VimScope(scope) => {
                self.pending = VimPending::TextObject {
                    operator: op,
                    scope,
                };
            }
            OperatorKey::Motion(motion) => match op {
                Operator::Delete => {
                    self.start_edit(t, VimAction::Delete(VimTarget::Motion(motion)));
                }
                Operator::Change => {
                    self.start_edit(t, VimAction::Change(VimTarget::Motion(motion)));
                }
                Operator::Yank => self.apply_operator(t, op, motion),
            },
            OperatorKey::Find(motion) => {
                self.pending = VimPending::Find {
                    motion,
                    operator: Some(op),
                };
            }
            OperatorKey::JumpTop | OperatorKey::JumpBottom => {
                let last = key == OperatorKey::JumpBottom;
                match op {
                    Operator::Delete => {
                        self.start_edit(t, VimAction::Delete(VimTarget::BufferJump { last }));
                    }
                    Operator::Change => {
                        self.start_edit(t, VimAction::Change(VimTarget::BufferJump { last }));
                    }
                    Operator::Yank => self.jump_to_line(t, last, Some(op)),
                }
            }
        }
    }

    fn object(&mut self, t: &mut dyn VimText, op: Operator, scope: VimScope, object: Object) {
        match op {
            Operator::Delete => {
                self.start_edit(t, VimAction::Delete(VimTarget::Object { scope, object }));
            }
            Operator::Change => {
                self.start_edit(t, VimAction::Change(VimTarget::Object { scope, object }));
            }
            Operator::Yank => {
                if let Some(range) = motions::object_range(t, object, scope) {
                    self.operate_range(t, op, range);
                }
            }
        }
    }

    fn pending_char(&mut self, t: &mut dyn VimText, pending: VimPending, ch: Option<char>) {
        let Some(ch) = ch else { return };
        match pending {
            VimPending::Replace => {
                if !(self.single_line && matches!(ch, '\r' | '\n')) {
                    self.start_edit(t, VimAction::Replace(ch));
                }
            }
            VimPending::Find { motion, operator } => match operator {
                Some(Operator::Delete) => {
                    self.start_edit(t, VimAction::Delete(VimTarget::Find { motion, target: ch }));
                }
                Some(Operator::Change) => {
                    self.start_edit(t, VimAction::Change(VimTarget::Find { motion, target: ch }));
                }
                Some(Operator::Yank) | None => {
                    self.find(t, motion, operator, ch);
                }
            },
            _ => {}
        }
    }

    // ---- operators -----------------------------------------------------------------

    fn apply_operator(&mut self, t: &mut dyn VimText, op: Operator, motion: Motion) {
        if op == Operator::Change && motion == Motion::WordForward {
            let doc = t.text();
            let target = if doc[t.cursor()..].chars().next().is_some_and(|ch| !ch.is_whitespace()) {
                motions::end_of_next_word(t)
            } else {
                motions::beginning_of_next_word(t).min(eol(t))
            };
            if target > t.cursor() {
                let cursor = t.cursor();
                self.operate_range(t, op, cursor..target);
            } else {
                self.mode = VimMode::Insert;
            }
            return;
        }
        let Some(range) = self.range_for_motion(t, motion) else {
            if op == Operator::Change && motion == Motion::LineEnd {
                self.mode = VimMode::Insert;
            }
            return;
        };
        if op == Operator::Change && matches!(motion, Motion::Up | Motion::Down) {
            let doc = t.text();
            if motion == Motion::Up && bol(t) == 0
                || motion == Motion::Down && eol(t) == doc.len()
            {
                return;
            }
            let retain_newline = range.end < doc.len() && doc[range.clone()].ends_with('\n');
            let start = range.start;
            self.kill(t, range, RegisterKind::Linewise);
            if retain_newline {
                t.replace(start..start, "\n");
                t.set_cursor(start);
            }
            self.mode = VimMode::Insert;
            return;
        }
        self.operate_range(t, op, range);
    }

    fn operate_range(&mut self, t: &mut dyn VimText, op: Operator, range: Range<usize>) {
        match op {
            Operator::Delete => self.kill(t, range, RegisterKind::Characterwise),
            Operator::Yank => self.yank(t, range, RegisterKind::Characterwise),
            Operator::Change => {
                self.kill(t, range, RegisterKind::Characterwise);
                self.mode = VimMode::Insert;
            }
        }
    }

    fn range_for_motion(&mut self, t: &mut dyn VimText, motion: Motion) -> Option<Range<usize>> {
        if matches!(motion, Motion::Up | Motion::Down) {
            return linewise_range(t, motion);
        }
        let start = t.cursor();
        let target = match motion {
            Motion::Left => prev_atomic(t, start),
            Motion::Right => next_atomic(t, start),
            Motion::WordForward => motions::beginning_of_next_word(t),
            Motion::WordBackward => motions::beginning_of_previous_word(t),
            Motion::WordEnd => motions::word_end_exclusive(t),
            Motion::LineStart => bol(t),
            Motion::LineEnd => eol(t),
            Motion::Up | Motion::Down => start,
        };
        (start != target).then(|| start.min(target)..start.max(target))
    }

    // ---- register ----------------------------------------------------------------

    fn kill(&mut self, t: &mut dyn VimText, range: Range<usize>, kind: RegisterKind) {
        let range = expand_to_elements(t, range);
        if range.start >= range.end {
            return;
        }
        t.store_register(range.clone(), kind);
        t.replace(range, "");
    }

    fn yank(&mut self, t: &mut dyn VimText, range: Range<usize>, kind: RegisterKind) {
        let range = expand_to_elements(t, range);
        if range.start >= range.end {
            return;
        }
        t.store_register(range, kind);
    }

    fn yank_current_line(&mut self, t: &mut dyn VimText) {
        let range = motions::current_line_with_newline(t);
        self.yank(t, range, RegisterKind::Linewise);
    }

    fn paste_after(&mut self, t: &mut dyn VimText) {
        let register = t.register();
        if register.text.is_empty() {
            return;
        }
        if register.kind == RegisterKind::Linewise && !self.single_line {
            self.paste_line_after(t, &register.text);
            return;
        }
        let insert_at = next_atomic(t, t.cursor());
        t.set_cursor(insert_at);
        let start = t.cursor();
        let replaying = std::mem::replace(&mut self.commands.replaying, true);
        self.insert_str(t, &register.text);
        self.commands.replaying = replaying;
        t.paste_register_elements(start);
    }

    fn paste_line_after(&mut self, t: &mut dyn VimText, register: &str) {
        let len = t.text().len();
        let e = eol(t);
        let insert_at = if e < len { e + 1 } else { e };
        let cursor = if e < len { insert_at } else { insert_at + 1 };
        let text = if e < len {
            if register.ends_with('\n') {
                register.to_owned()
            } else {
                format!("{register}\n")
            }
        } else {
            format!("\n{}", register.trim_end_matches('\n'))
        };
        t.replace(insert_at..insert_at, &text);
        t.paste_register_elements(cursor);
        t.set_cursor(cursor.min(t.text().len()));
    }

    // ---- find, jumps, search ---------------------------------------------------------

    fn find(&mut self, t: &mut dyn VimText, motion: Find, op: Option<Operator>, target: char) -> bool {
        let origin = t.cursor();
        let doc = t.text();
        let clusters = |from: usize, to: usize| -> Vec<Range<usize>> {
            let mut start = from;
            t.grapheme_ends(&doc[from..to])
                .into_iter()
                .map(|end| {
                    let range = start..from + end;
                    start = from + end;
                    range
                })
                .collect()
        };
        let hit = |g: &Range<usize>| doc[g.start..].starts_with(target) && is_command_target(t, g.start);
        let found = match motion {
            Find::Forward | Find::TillForward => {
                let line_end = eol_of(&doc, origin);
                if origin >= line_end {
                    return false;
                }
                let start = next_atomic(t, origin);
                clusters(start, line_end).into_iter().find(|g| hit(g))
            }
            Find::Backward | Find::TillBackward => {
                let line_start = bol_of(&doc, origin);
                clusters(line_start, origin).into_iter().rev().find(|g| hit(g))
            }
        };
        let Some(position) = found else { return false };
        if let Some(op) = op {
            let range = match motion {
                Find::Forward => origin..position.end,
                Find::Backward => position.start..origin,
                Find::TillForward => origin..position.start,
                Find::TillBackward => position.end..origin,
            };
            if op == Operator::Yank {
                t.set_cursor(range.start);
            }
            self.operate_range(t, op, range);
        } else {
            let destination = match motion {
                Find::Forward | Find::Backward => position.start,
                Find::TillForward => {
                    let previous = t.prev_grapheme(position.start);
                    t.elements()
                        .into_iter()
                        .find(|e| previous > e.start && previous < e.end)
                        .map_or(previous, |e| e.start)
                }
                Find::TillBackward => position.end,
            };
            t.set_cursor(destination);
        }
        true
    }

    fn jump_to_line(&mut self, t: &mut dyn VimText, last: bool, op: Option<Operator>) {
        let doc = t.text();
        if let Some(op) = op {
            let current = motions::current_line_with_newline(t);
            let range = if last { current.start..doc.len() } else { 0..current.end };
            match op {
                Operator::Delete => self.kill(t, range, RegisterKind::Linewise),
                Operator::Yank => self.yank(t, range, RegisterKind::Linewise),
                Operator::Change => {
                    self.kill(t, range, RegisterKind::Linewise);
                    self.mode = VimMode::Insert;
                }
            }
            return;
        }
        t.set_cursor(if last { bol_of(&doc, doc.len()) } else { 0 });
        t.set_cursor(motions::first_non_blank(t));
    }

    /// A `/` or `?` query starts (its text is the view's).
    pub fn begin_search(&mut self, direction: VimDirection) {
        if self.is_normal() && matches!(self.pending, VimPending::None | VimPending::Operator(_)) {
            self.searching = Some(direction);
        }
    }

    /// The query is dropped (Escape, Ctrl+C, Backspace on an empty query).
    pub fn abort_search(&mut self) {
        if self.searching.take().is_some() {
            self.pending = VimPending::None;
        }
    }

    /// The query is accepted; an empty one searches the last text again.
    pub fn accept_search(&mut self, t: &mut dyn VimText, text: &str) {
        let Some(direction) = self.searching.take() else { return };
        let text = if text.is_empty() {
            self.last_search.text.clone()
        } else {
            text.to_owned()
        };
        let query = SearchQuery { text, direction };
        self.last_search = query.clone();
        self.run_search(t, query);
    }

    /// `n` (the last search again) or `N` (the other way).
    pub fn repeat_search(&mut self, t: &mut dyn VimText, reverse: bool) {
        let mut query = self.last_search.clone();
        if reverse {
            query.direction = query.direction.reversed();
        }
        self.run_search(t, query);
    }

    fn run_search(&mut self, t: &mut dyn VimText, query: SearchQuery) {
        match std::mem::take(&mut self.pending) {
            VimPending::Operator(Operator::Delete) => {
                self.start_edit(t, VimAction::Delete(VimTarget::Search(query)));
            }
            VimPending::Operator(Operator::Change) => {
                self.start_edit(t, VimAction::Change(VimTarget::Search(query)));
            }
            VimPending::Operator(Operator::Yank) => {
                self.search(t, &query, Some(Operator::Yank));
            }
            _ => {
                self.search(t, &query, None);
            }
        }
    }

    fn search(&mut self, t: &mut dyn VimText, query: &SearchQuery, op: Option<Operator>) -> bool {
        let origin = t.cursor();
        let target = matching_ranges(t, &query.text)
            .into_iter()
            .map(|r| r.start)
            .filter(|&start| is_command_target(t, start))
            .min_by_key(|&pos| match query.direction {
                VimDirection::Forward => (pos <= origin, pos),
                VimDirection::Backward => (pos >= origin, usize::MAX - pos),
            });
        let Some(target) = target else { return false };
        let Some(op) = op else {
            t.set_cursor(target);
            return true;
        };
        if origin == target {
            return false;
        }
        let doc = t.text();
        let mut range = origin.min(target)..origin.max(target);
        let line_start = bol_of(&doc, range.start);
        let ends_at_line_start = range.end == bol_of(&doc, range.end);
        // Exclusive motions ending at column zero may become linewise.
        let linewise = ends_at_line_start && doc[line_start..range.start].trim().is_empty();
        if linewise {
            range.start = line_start;
        } else if ends_at_line_start {
            range.end -= 1;
        }
        let start = range.start;
        if op == Operator::Yank {
            t.set_cursor(start);
        }
        self.operate_range(t, op, range);
        if linewise {
            t.set_register_kind(RegisterKind::Linewise);
            if op == Operator::Change {
                t.replace(start..start, "\n");
                t.set_cursor(start);
            }
        }
        true
    }

    /// The ranges the draft highlights for the query being typed or the last one.
    pub fn search_highlights(&self, t: &dyn VimText, typing: Option<&str>) -> Vec<Range<usize>> {
        let query = typing.unwrap_or(&self.last_search.text);
        matching_ranges(t, query)
            .into_iter()
            .filter(|r| is_command_target(t, r.start))
            .collect()
    }
}

/// `dj` / `dk` and the like: whole lines.
fn linewise_range(t: &dyn VimText, motion: Motion) -> Option<Range<usize>> {
    let doc = t.text();
    let current = motions::current_line_with_newline(t);
    let range = match motion {
        Motion::Up => {
            let start = if current.start == 0 {
                0
            } else {
                bol_of(&doc, current.start - 1)
            };
            start..current.end
        }
        Motion::Down => {
            let end = if current.end >= doc.len() {
                current.end
            } else {
                let next_eol = eol_of(&doc, current.end);
                if next_eol < doc.len() { next_eol + 1 } else { next_eol }
            };
            current.start..end
        }
        _ => return None,
    };
    (range.start < range.end).then_some(range)
}
