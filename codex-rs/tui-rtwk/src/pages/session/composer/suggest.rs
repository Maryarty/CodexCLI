//! The composer's suggestions: what the popup lists for the text before the
//! caret, which row is highlighted, and the keys that move through it. In
//! Flutter terms this is the `RawAutocomplete` controller the field and its
//! options view share: the model lives in the composer's state and the key
//! hook (`EditingKeys`) reads it, the popup (`Popover` over an `OptionList`)
//! draws it, a click on a row picks through the same door as Enter.
//!
//! Oracle: `bottom_pane/chat_composer.rs` (`handle_key_event_with_slash_popup`,
//! `handle_key_event_with_skill_popup`): Up and Down move the highlight,
//! Enter picks it (a command runs, a mention is put in), Tab completes it as
//! text (`/compact `; Tab on `/skills` runs it), Esc closes the popup until the
//! text changes.

use std::cell::RefCell;
use std::rc::Rc;

use rtwk::editing::{Suggestion, TextEditingController};
use rtwk::focus::KeyPress;

use super::ComposerMsg;
use crate::keymap::{only_ctrl, pressed, pressed_fixed, unmodified};
use codex_tui_core::keymap::Keymap;
use codex_tui_core::keymap::fixed::{
    POPUP_ACCEPT, POPUP_COMPLETE, POPUP_NEXT, POPUP_PREVIOUS, SLASH_COMPLETE,
};

/// The suggestions shown, shared by the key hook and the popup's builder.
#[derive(Default)]
pub struct Suggest(RefCell<Inner>);

#[derive(Default)]
struct Inner {
    items: Rc<[Suggestion]>,
    /// The text before the caret the items answer.
    query: Rc<str>,
    highlighted: usize,
    /// The text the popup was closed in: it stays closed until the text
    /// changes (`hide`).
    hidden_in: Option<Rc<str>>,
    /// A turn runs, so Tab on a command typed in full queues the line (`set_queueing`).
    queueing: bool,
}

/// What a popup shows: the rows, the query they answer and the highlight.
pub struct Shown {
    pub items: Rc<[Suggestion]>,
    pub query: Rc<str>,
    pub highlighted: usize,
}

impl Suggest {
    /// Tells whether a turn runs: Tab on a command already typed in full has nothing to complete,
    /// and queues the whole line then (the page's Tab) instead of completing it.
    pub fn set_queueing(&self, on: bool) {
        self.0.borrow_mut().queueing = on;
    }

    /// Whether the popup has rows to show.
    pub fn open(&self) -> bool {
        !self.0.borrow().items.is_empty()
    }

    pub fn shown(&self) -> Shown {
        let inner = self.0.borrow();
        Shown {
            items: inner.items.clone(),
            query: inner.query.clone(),
            highlighted: inner.highlighted,
        }
    }

    /// Asks `find` for the rows of the text before the caret of `c` (none
    /// for a selection or a caret at the start, none while the popup is
    /// closed in this very text). Whether the rows or the query changed:
    /// the popup then draws again, its highlight back on the first row.
    pub fn refresh(
        &self,
        c: &TextEditingController,
        find: impl FnOnce(&str) -> Vec<Suggestion>,
    ) -> bool {
        let text = c.text();
        let selection = c.selection();
        let caret = selection.extent_offset;
        let before = (selection.is_collapsed() && caret > 0)
            .then(|| text.get(..caret))
            .flatten();
        let mut inner = self.0.borrow_mut();
        if inner.hidden_in.as_deref() == Some(&*text) {
            return false;
        }
        inner.hidden_in = None;
        let (items, query): (Vec<Suggestion>, &str) = match before {
            Some(before) => (find(before), before),
            None => (Vec::new(), ""),
        };
        if items.is_empty() && inner.items.is_empty() {
            return false;
        }
        if *inner.items == *items && *inner.query == *query {
            return false;
        }
        inner.items = items.into();
        inner.query = Rc::from(query);
        inner.highlighted = 0;
        true
    }

    /// Closes the popup until the text changes from `text`.
    pub fn hide(&self, text: Rc<str>) {
        let mut inner = self.0.borrow_mut();
        inner.hidden_in = Some(text);
        inner.items = Rc::from([]);
        inner.highlighted = 0;
    }

    /// Highlights row `at` (the pointer is over it, the wheel moved to it).
    pub fn highlight(&self, at: usize) {
        let mut inner = self.0.borrow_mut();
        if at < inner.items.len() {
            inner.highlighted = at;
        }
    }

    /// The highlight `by` rows on, wrapping round (`n - 1` is one back).
    fn step(&self, by: usize) {
        let mut inner = self.0.borrow_mut();
        let n = inner.items.len();
        // a row a side conversation cannot run is listed, not selectable
        for k in 1..=n {
            let at = (inner.highlighted + by * k) % n;
            if inner.items[at].enabled {
                inner.highlighted = at;
                return;
            }
        }
    }

    /// Puts row `at` in place of the text before the caret (`pick`) and
    /// closes the popup; the text it put in.
    pub fn pick(&self, c: &TextEditingController, at: usize) -> Option<Rc<str>> {
        let item = self.0.borrow().items.get(at)?.clone();
        if !item.enabled {
            return None;
        }
        let text = c.text();
        let caret = c.selection().extent_offset;
        // Only the word being completed is rewritten, from where the pick
        // and the text before the caret part: what stands before it (an
        // image, a pasted placeholder) keeps its element.
        let before = text.get(..caret).unwrap_or_default();
        let shared = before
            .char_indices()
            .zip(item.text.chars())
            .take_while(|((_, a), b)| a == b)
            .last()
            .map_or(0, |((at, a), _)| at + a.len_utf8());
        c.replace(shared, caret, &item.text[shared..]);
        self.hide(c.text());
        Some(Rc::from(&*item.text))
    }

    /// As `pick`, completing a command as the text `/name ` (Tab).
    fn complete(&self, c: &TextEditingController, at: usize) -> Option<Rc<str>> {
        let text = self.pick(c, at)?;
        if !text.starts_with('/') || text.ends_with(' ') {
            return Some(text);
        }
        let caret = c.selection().extent_offset;
        c.replace(caret, caret, " ");
        self.hide(c.text());
        Some(Rc::from(format!("{text} ")))
    }

    /// A typed `/` while a command's name is being typed completes the
    /// highlighted command as text (`/model `). Asked before the paste-burst
    /// detector, which would hold the character.
    pub fn take_slash(
        &self,
        c: &TextEditingController,
        key: &KeyPress,
        tell: &dyn Fn(ComposerMsg),
    ) -> bool {
        if !pressed_fixed(SLASH_COMPLETE, key) {
            return false;
        }
        let (at, naming) = {
            let inner = self.0.borrow();
            // a muted row (a command a side conversation cannot run) is no completion: the `/`
            // is typed
            let naming = inner
                .items
                .get(inner.highlighted)
                .is_some_and(|i| i.enabled)
                && inner.query.starts_with('/')
                && !inner.query.contains(char::is_whitespace);
            (inner.highlighted, naming)
        };
        if !naming {
            return false;
        }
        if let Some(text) = self.complete(c, at) {
            tell(ComposerMsg::Completed(text));
        }
        true
    }

    /// Whether the popup takes `key`: Up and Down (Ctrl+P, Ctrl+N) move
    /// through the rows, Enter picks the highlighted one, Tab completes it
    /// as text. Only a popup with rows takes keys.
    pub fn take_key(
        &self,
        keymap: &Keymap,
        c: &TextEditingController,
        key: &KeyPress,
        tell: &dyn Fn(ComposerMsg),
    ) -> bool {
        if !self.open() {
            return false;
        }
        let (n, at) = {
            let inner = self.0.borrow();
            (inner.items.len(), inner.highlighted)
        };
        let plain = unmodified(key);
        let ctrl = only_ctrl(key);
        let usable = self.0.borrow().items.get(at).is_some_and(|i| i.enabled);
        // Up and Down are the plain keys, Ctrl+P and Ctrl+N the control characters
        let is = |id| pressed(keymap, id, key) && (plain || (ctrl && key.ch.is_some()));
        match () {
            () if is(POPUP_PREVIOUS) => self.step(n - 1),
            () if is(POPUP_NEXT) => self.step(1),
            () if is(POPUP_ACCEPT) && plain && usable => {
                if let Some(text) = self.pick(c, at) {
                    tell(ComposerMsg::Picked(text));
                }
                return true;
            }
            () if is(POPUP_COMPLETE) && plain && usable => {
                // `/skills` is the one command Tab runs, as Enter does
                let (runs, typed, queueing) = {
                    let inner = self.0.borrow();
                    let item = inner.items.get(at);
                    (
                        item.is_some_and(|i| &*i.text == "/skills"),
                        // the command's name stands in the text already: nothing to complete
                        // (`selected_command_completion`)
                        item.is_some_and(|i| {
                            i.text.starts_with('/')
                                && inner.query.trim_start().starts_with(&*i.text)
                        }),
                        inner.queueing,
                    )
                };
                if runs {
                    if let Some(text) = self.pick(c, at) {
                        tell(ComposerMsg::Picked(text));
                    }
                    return true;
                }
                if typed {
                    // a turn running: the page's Tab queues the line; none: Tab does nothing
                    return !queueing;
                }
                if let Some(text) = self.complete(c, at) {
                    tell(ComposerMsg::Completed(text));
                }
                return true;
            }
            _ => return false,
        }
        tell(ComposerMsg::Redraw);
        true
    }
}
