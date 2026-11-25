//! The mention popups' UI state of one composer, as Flutter keeps an
//! autocomplete's: the options (core's [`Session`], which popup the text at
//! the caret calls for and its rows), the highlighted option (a
//! `SelectionController`, the cursor of the rows) and the scroll window (a
//! `ListController`), shared by the key hook (`EditingKeys`) that moves them
//! and the options view that draws them, and a notifier the view listens to.
//! Oracle: `chat_composer.rs` (`popups`, `sync_popups`), `scroll_state.rs`
//! (the selection is clamped, never reset, when the rows change).

use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;

use rtwk::controls::SelectionController;
use rtwk::editing::{TextEditingController, TextSelection};
use rtwk::foundation::ChangeNotifier;
use rtwk::scroll::ListController;

use codex_tui_core::data::files::FileSearchAsk;
use codex_tui_core::kit::mentions::search_mode::SearchMode;
use codex_tui_core::kit::mentions::session::{
    Active, MentionPick, MentionSession, PickAction, Separator, Sources, completion_separator,
    starts_after_element,
};
use codex_tui_core::kit::mentions::target::Doc;

use super::ComposerDraft;
use crate::services::images;

/// The state of the popups of one draft, shared by every clone (a handle).
#[derive(Clone)]
pub struct Mentions(Rc<Shared>);

struct Shared {
    session: RefCell<MentionSession>,
    /// The highlighted row, by its position among the popup's rows.
    cursor: SelectionController<usize>,
    /// The scroll of the rows: the `↑` and `↓` marks follow it.
    list: ListController,
    /// Told when the popup has to be drawn again.
    changed: Rc<ChangeNotifier>,
    /// What a click on a row does: the composer's (it owns the text the
    /// pick goes into), as the `onSelected` of Flutter's options view.
    chooser: RefCell<Option<Chooser>>,
}

impl Default for Mentions {
    fn default() -> Self {
        Mentions(Rc::new(Shared {
            session: RefCell::default(),
            cursor: SelectionController::new(),
            list: ListController::new(),
            changed: Rc::new(ChangeNotifier::new()),
            chooser: RefCell::new(None),
        }))
    }
}

/// What a click on a row of the popup asks for.
type Chooser = Rc<dyn Fn(usize)>;

/// What a `sync` found.
#[derive(Debug, Default)]
pub struct Synced {
    /// A popup is or was up: the view draws it again.
    pub changed: bool,
    /// What the file search is asked (the shell runs it).
    pub asks: Vec<FileSearchAsk>,
}

impl Mentions {
    /// Whether a popup is up.
    pub fn is_open(&self) -> bool {
        self.0.session.borrow().is_open()
    }

    /// Reads the session (the popup that is up and its rows).
    pub fn read<R>(&self, f: impl FnOnce(&MentionSession) -> R) -> R {
        f(&self.0.session.borrow())
    }

    /// The highlighted row of the popup, `None` when it has no rows.
    pub fn cursor(&self) -> Option<usize> {
        self.0.cursor.cursor()
    }

    pub fn cursor_controller(&self) -> &SelectionController<usize> {
        &self.0.cursor
    }

    pub fn list(&self) -> &ListController {
        &self.0.list
    }

    /// What the popup view listens to.
    pub fn listenable(&self) -> Rc<ChangeNotifier> {
        self.0.changed.clone()
    }

    /// What a click on a row of the popup asks for (`choose`), set by the
    /// composer.
    pub fn on_choose(&self, chooser: Chooser) {
        *self.0.chooser.borrow_mut() = Some(chooser);
    }

    /// The row `at` was clicked: the composer picks it.
    pub fn choose(&self, at: usize) {
        let chooser = self.0.chooser.borrow().clone();
        if let Some(chooser) = chooser {
            chooser(at);
        }
    }

    /// The popup is drawn again.
    pub fn notify(&self) {
        self.0.changed.notify_listeners();
    }

    /// How many rows the popup that is up lists.
    pub fn rows(&self) -> usize {
        match self.0.session.borrow().active() {
            Active::At(popup) => popup.rows().len(),
            Active::Dollar(popup) => popup.rows().len(),
            Active::None => 0,
        }
    }

    /// Brings the popup in line with the draft (`sync_popups`) and the
    /// highlight with the rows: a popup that opens highlights its first
    /// row, one that stays keeps its row (clamped to the rows left).
    pub fn sync(&self, doc: &Doc, src: &Sources) -> Synced {
        let was_open = self.is_open();
        let asks = self.0.session.borrow_mut().sync(doc, src);
        self.settle(was_open);
        Synced {
            changed: was_open || self.is_open(),
            asks,
        }
    }

    /// Closes the popup with no dismissal (history search, the Vim `/`
    /// query, a history walk, a selection): a search that runs is ended.
    pub fn close(&self) -> Synced {
        let was_open = self.is_open();
        let asks = self.0.session.borrow_mut().close();
        self.settle(was_open);
        Synced {
            changed: was_open,
            asks,
        }
    }

    /// Closes the popup; a dismissal stays.
    pub fn hide(&self) {
        self.0.session.borrow_mut().hide();
        self.settle(true);
    }

    /// Esc: closed, and closed for this very token.
    pub fn dismiss(&self, doc: &Doc, src: &Sources) {
        self.0.session.borrow_mut().dismiss(doc, src);
        self.settle(true);
    }

    /// The highlight on row `at` (the pointer is over it).
    pub fn highlight(&self, at: usize) {
        if at < self.rows() {
            self.0.cursor.set_cursor(Some(at));
            self.0.list.ensure_index_visible(at);
        }
    }

    /// The highlight one row on, round the ends (Up and Down).
    pub fn step(&self, forward: bool) {
        let n = self.rows();
        if n == 0 {
            return;
        }
        let at = match (self.cursor(), forward) {
            (None, true) => 0,
            (None, false) => n - 1,
            (Some(at), true) => (at + 1) % n,
            (Some(at), false) => (at + n - 1) % n,
        };
        self.highlight(at);
    }

    /// Whether Left and Right switch the `@` popup's tab at this caret.
    pub fn can_switch_search_mode(&self, doc: &Doc) -> bool {
        self.0.session.borrow().can_switch_search_mode(doc)
    }

    /// The `@` popup's tab one on or back; the highlight stays on its row
    /// number, clamped to the rows the tab leaves.
    pub fn switch_search_mode(&self, forward: bool) {
        {
            let mut session = self.0.session.borrow_mut();
            if forward {
                session.next_search_mode();
            } else {
                session.previous_search_mode();
            }
        }
        self.settle(true);
    }

    /// The `@` popup's tab `mode` (a tap on it); the highlight stays on its
    /// row number, clamped to the rows the tab leaves.
    pub fn set_search_mode(&self, mode: SearchMode) {
        self.0.session.borrow_mut().set_search_mode(mode);
        self.settle(true);
    }

    /// What picking row `position` puts in the draft.
    pub fn pick(&self, position: usize, doc: &Doc, src: &Sources) -> Option<MentionPick> {
        self.0.session.borrow().pick(position, doc, src)
    }

    /// The pick was applied: the popup closes, the completed token stays
    /// dismissed (`doc` is the draft after it).
    pub fn finish_pick(&self, pick: &MentionPick, inserted: std::ops::Range<usize>, doc: &Doc) {
        self.0.session.borrow_mut().finish_pick(pick, inserted, doc);
        self.settle(true);
    }

    /// Whether `@` or `$` calls for a popup at the caret (the slash popup
    /// then stays down).
    pub fn claims_caret(&self, doc: &Doc, src: &Sources) -> bool {
        self.0.session.borrow().claims_caret(doc, src)
    }

    /// Keeps the highlight valid after the rows changed (`clamp_selection`,
    /// `ensure_visible`): none without rows, the first row for a popup that
    /// just opened.
    fn settle(&self, was_open: bool) {
        let n = self.rows();
        let cursor = match (n, self.cursor()) {
            (0, _) => None,
            (_, Some(at)) if was_open => Some(at.min(n - 1)),
            _ => Some(0),
        };
        if cursor != self.cursor() {
            self.0.cursor.set_cursor(cursor);
        }
        match cursor {
            Some(at) if self.is_open() => {
                if !was_open {
                    self.0.list.jump_to_index(0);
                }
                self.0.list.ensure_index_visible(at);
            }
            _ => self.0.list.jump_to_index(0),
        }
    }
}

/// The draft as the mention targets read it: its text, the caret and the
/// ranges of its elements.
struct DocOwned {
    text: Rc<str>,
    cursor: usize,
    elements: Vec<Range<usize>>,
}

impl DocOwned {
    fn doc(&self) -> Doc<'_> {
        Doc {
            text: &self.text,
            cursor: self.cursor,
            elements: &self.elements,
        }
    }
}

impl ComposerDraft {
    fn mention_doc(&self, c: &TextEditingController) -> DocOwned {
        DocOwned {
            text: c.text(),
            cursor: c.selection().extent_offset,
            elements: self.elements(c).iter().map(|e| e.start..e.end).collect(),
        }
    }

    /// Brings the popups in line with the text and the caret (`sync_popups`):
    /// none for a selection.
    pub fn sync_mentions(&self, c: &TextEditingController, src: &Sources) -> Synced {
        let mentions = self.mentions();
        if !c.selection().is_collapsed() {
            return mentions.close();
        }
        let now = self.mention_doc(c);
        mentions.sync(&now.doc(), src)
    }

    /// Esc on a popup: closed for this very token.
    pub fn dismiss_mentions(&self, c: &TextEditingController, src: &Sources) {
        let now = self.mention_doc(c);
        self.mentions().dismiss(&now.doc(), src);
    }

    /// Whether `@` or `$` at the caret calls for a popup (the slash popup
    /// then stays down).
    pub fn mentions_claim_caret(&self, c: &TextEditingController, src: &Sources) -> bool {
        let now = self.mention_doc(c);
        self.mentions().claims_caret(&now.doc(), src)
    }

    /// Whether Left and Right switch the `@` popup's tab at the caret.
    pub fn can_switch_search_mode(&self, c: &TextEditingController) -> bool {
        let now = self.mention_doc(c);
        self.mentions().can_switch_search_mode(&now.doc())
    }

    /// What picking row `position` of the popup would put in.
    pub fn mention_pick(
        &self,
        c: &TextEditingController,
        position: usize,
        src: &Sources,
    ) -> Option<MentionPick> {
        let now = self.mention_doc(c);
        self.mentions().pick(position, &now.doc(), src)
    }

    /// Puts what `pick` stands for in place of the token it names
    /// (`insert_selected_path`, `insert_selected_file_path`,
    /// `insert_selected_mention`): a path as text, an image file as the next
    /// `[Image #N]`, a skill, plugin or task as an atomic element with its
    /// binding; a space first when the token follows an element, one
    /// separator after it. Only the token is replaced, so an image or a
    /// paste before it keeps its element. `skill`: the pick is a skill, whose
    /// separator the next punctuation key may replace.
    pub fn apply_pick(
        &self,
        c: &TextEditingController,
        pick: &MentionPick,
        image: impl Fn(usize) -> String,
        skill: bool,
    ) {
        // a completion is an edit no key made: a step of undo of its own in Vim, and
        // what Replace mode overwrote is not what Backspace could put back
        self.with_vim(codex_tui_core::kit::vim::Vim::forget_recovery);
        self.vim_direct(c, || self.pick_into(c, pick, image, skill));
    }

    fn pick_into(
        &self,
        c: &TextEditingController,
        pick: &MentionPick,
        image: impl Fn(usize) -> String,
        skill: bool,
    ) {
        let mut range = pick.range.clone();
        let before = self.mention_doc(c);
        if starts_after_element(&before.doc(), &range) {
            c.replace(range.start, range.start, " ");
            range = range.start + 1..range.end + 1;
        }
        let start = range.start;
        let inserted = match &pick.action {
            PickAction::Path(text) => self.put_text(c, range, text),
            PickAction::Image { path, text } => {
                // the file as a terminal names a dropped one: quoted
                let dropped = shlex::try_quote(path)
                    .ok()
                    .and_then(|quoted| images::pasted_image(&quoted));
                if let Some(file) = dropped {
                    c.replace(range.start, range.end, "");
                    c.set_selection(TextSelection::collapsed(start));
                    self.attach_image(c, file, image);
                    self.advance_past_separator(c);
                    start..start
                } else {
                    self.put_text(c, range, text)
                }
            }
            PickAction::Element { text, binding } => {
                c.replace(range.start, range.end, "");
                c.set_selection(TextSelection::collapsed(start));
                self.insert_mention(c, text, binding.clone());
                let added = self.advance_past_separator(c);
                self.keep_skill_space(added.filter(|_| skill));
                start..start + text.len()
            }
        };
        let after = self.mention_doc(c);
        self.mentions().finish_pick(pick, inserted, &after.doc());
    }

    /// `text` in place of `range`, the caret and a separator after it; the
    /// range it took.
    fn put_text(&self, c: &TextEditingController, range: Range<usize>, text: &str) -> Range<usize> {
        let start = range.start;
        c.replace(range.start, range.end, text);
        c.set_selection(TextSelection::collapsed(start + text.len()));
        self.advance_past_separator(c);
        start..start + text.len()
    }

    /// The caret after one horizontal separator following a completion
    /// (`advance_past_completion_separator`); the position of the space when
    /// the completion put one in.
    fn advance_past_separator(&self, c: &TextEditingController) -> Option<usize> {
        let text = c.text();
        let caret = c.selection().extent_offset;
        match completion_separator(&text, caret) {
            Separator::MoveCaretTo(at) => {
                c.set_selection(TextSelection::collapsed(at));
                None
            }
            Separator::InsertSpaceAt(at) => {
                c.replace(at, at, " ");
                c.set_selection(TextSelection::collapsed(at + 1));
                Some(at)
            }
        }
    }
}
