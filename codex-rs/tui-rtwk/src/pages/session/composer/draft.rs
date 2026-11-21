//! What the composer's draft holds beyond its text (UI state, one per thread
//! in `ThreadUi`, never seen by `apply`): the atomic elements of the text
//! (the `[Pasted Content N chars]` placeholders of large pastes and the
//! payload each stands for) and the kill ring of `Ctrl+K` / `Ctrl+Y`.
//!
//! The text itself, its caret, selection and undo are the
//! `TextEditingController`'s. As a Flutter app keeps the spans of a
//! `TextEditingController` subclass in step with its value, the draft keeps
//! its elements in step with the controller's text: `sync` maps them across
//! each change (an edit before an element moves it, an edit that cuts into
//! one breaks it and drops its payload), and the keys that would step or
//! delete into an element take it whole (`Composer`'s `EditingKeys`).
//! Oracle: `bottom_pane/chat_composer.rs` (`pending_pastes`,
//! `next_large_paste_placeholder`, `expand_pending_pastes`),
//! `chat_composer/paste_input.rs` (`apply_paste`) and `bottom_pane/textarea.rs`
//! (elements, `kill_buffer`).

use std::cell::RefCell;
use std::rc::Rc;

use rtwk::editing::{TextBuffer, TextEditingController, TextSelection};

use codex_tui_core::screens::composer;

/// One atomic element of the text: the bytes `start..end` show `label`
/// and stand for `payload` on submit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Element {
    pub start: usize,
    pub end: usize,
    pub label: Rc<str>,
    pub payload: Rc<str>,
}

#[derive(Default)]
struct Inner {
    elements: Vec<Element>,
    /// The text the elements are measured on.
    seen: Option<TextBuffer>,
    /// What the last `Ctrl+K` / `Ctrl+W` killed (`textarea.rs`
    /// `kill_buffer`), put back by `Ctrl+Y`, with the elements it held.
    kill: Option<(String, Vec<Element>)>,
    /// The history entry Up and Down last recalled, and its text
    /// (`chat_composer_history.rs`: `history_cursor`, `last_history_text`).
    recalled: Option<(usize, String)>,
    /// The drafts this composer submitted or cleared with placeholders in
    /// them, by the text they sent: Up brings an entry back with its
    /// placeholders (`HistoryEntry::pending_pastes`).
    local: Vec<Local>,
}

/// A local history entry with its elements.
#[derive(Clone, Debug)]
struct Local {
    sent: String,
    text: String,
    elements: Vec<Element>,
}

/// The draft's elements and kill ring, shared by every clone (a handle,
/// like the controller it goes with).
#[derive(Clone, Default)]
pub struct ComposerDraft(Rc<RefCell<Inner>>);

impl PartialEq for ComposerDraft {
    fn eq(&self, other: &ComposerDraft) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl std::fmt::Debug for ComposerDraft {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComposerDraft")
            .field("elements", &self.0.borrow().elements)
            .finish_non_exhaustive()
    }
}

impl ComposerDraft {
    /// The elements as they stand in `c`'s text, start first.
    pub fn elements(&self, c: &TextEditingController) -> Vec<Element> {
        self.sync(c);
        self.0.borrow().elements.clone()
    }

    /// Maps the elements across the changes of `c`'s text since they were
    /// last measured: an element after a change moves with it, one a change
    /// cut into is gone (its payload with it). Called by the controller's
    /// listener and before every use, so an edit made while no composer
    /// listened (a draft put in by the app) is mapped too.
    pub fn sync(&self, c: &TextEditingController) {
        let now = c.buffer();
        let mut inner = self.0.borrow_mut();
        let Some(seen) = inner.seen.replace(now.clone()) else {
            return;
        };
        if TextBuffer::ptr_eq(&seen, &now) || inner.elements.is_empty() {
            return;
        }
        let (start, old_end, new_end) = now
            .change_since(&seen)
            .unwrap_or_else(|| changed(&seen.text(), &now.text()));
        inner.elements.retain_mut(|e| {
            if e.end <= start {
                true
            } else if e.start >= old_end {
                e.start = e.start + new_end - old_end;
                e.end = e.end + new_end - old_end;
                true
            } else {
                false
            }
        });
        // a text that no longer holds an element's label there (a reset to
        // another draft) drops it
        let text = now.text();
        inner
            .elements
            .retain(|e| text.get(e.start..e.end) == Some(&*e.label));
    }

    /// Forgets every element and the text they were measured on (a submit
    /// or a draft the app replaced).
    pub fn clear(&self) {
        let mut inner = self.0.borrow_mut();
        inner.elements.clear();
        inner.seen = None;
    }

    /// The paste rule of `apply_paste` (core's `large_paste`): a large
    /// paste goes in as a placeholder element at the selection, named by
    /// `label` from its size, and `true` says it was taken; a shorter one is
    /// left to the field.
    pub fn paste(
        &self,
        c: &TextEditingController,
        pasted: &str,
        label: impl Fn(usize) -> String,
    ) -> bool {
        let pasted = pasted.replace("\r\n", "\n").replace('\r', "\n");
        let Some(chars) = composer::large_paste(&pasted) else {
            return false;
        };
        let label = self.next_label(c, &label(chars));
        self.insert_element(c, &label, &pasted);
        true
    }

    /// Puts an element showing `label` for `payload` in place of the
    /// selection, the caret after it.
    pub fn insert_element(&self, c: &TextEditingController, label: &str, payload: &str) {
        self.sync(c);
        let selection = c.selection();
        let start = selection.start();
        c.insert(label);
        self.sync(c);
        let mut inner = self.0.borrow_mut();
        let at = inner.elements.partition_point(|e| e.start < start);
        inner.elements.insert(
            at,
            Element {
                start,
                end: start + label.len(),
                label: Rc::from(label),
                payload: Rc::from(payload),
            },
        );
    }

    /// `next_large_paste_placeholder`: `[Pasted Content N chars]`, then
    /// ` #2`, ` #3`, ... past the highest suffix of that size still in the
    /// draft.
    fn next_label(&self, c: &TextEditingController, base: &str) -> String {
        let base = base.to_owned();
        let prefix = format!("{base} #");
        let top = self
            .elements(c)
            .iter()
            .filter_map(|e| {
                if *e.label == *base {
                    Some(1)
                } else {
                    e.label.strip_prefix(&prefix)?.parse::<usize>().ok()
                }
            })
            .max()
            .unwrap_or(0);
        if top == 0 {
            base
        } else {
            format!("{base} #{}", top + 1)
        }
    }

    /// The text with every element's label replaced by its payload: what a
    /// submit sends (`expand_pending_pastes`).
    pub fn expanded(&self, c: &TextEditingController) -> String {
        let elements = self.elements(c);
        let text = c.text();
        let mut out = String::with_capacity(text.len());
        let mut at = 0;
        for e in &elements {
            out.push_str(&text[at..e.start]);
            out.push_str(&e.payload);
            at = e.end;
        }
        out.push_str(&text[at..]);
        out
    }

    /// The element that holds `offset` strictly inside it.
    fn inside(&self, c: &TextEditingController, offset: usize) -> Option<Element> {
        self.elements(c)
            .into_iter()
            .find(|e| e.start < offset && offset < e.end)
    }

    /// The element that ends at `offset`.
    pub fn ending_at(&self, c: &TextEditingController, offset: usize) -> Option<Element> {
        self.elements(c).into_iter().find(|e| e.end == offset)
    }

    /// The element that starts at `offset`.
    pub fn starting_at(&self, c: &TextEditingController, offset: usize) -> Option<Element> {
        self.elements(c).into_iter().find(|e| e.start == offset)
    }

    /// A caret the field or the mouse put inside an element moves to its
    /// nearer end (`clamp_pos_to_nearest_boundary`), the start on a tie.
    pub fn snap_caret(&self, c: &TextEditingController) {
        let selection = c.selection();
        if !selection.is_collapsed() {
            return;
        }
        let caret = selection.extent_offset;
        if let Some(e) = self.inside(c, caret) {
            let to = if caret - e.start <= e.end - caret {
                e.start
            } else {
                e.end
            };
            c.set_selection(TextSelection::collapsed(to));
        }
    }

    /// Keeps `text` (and the elements in `from..to` of the draft) as the
    /// last kill.
    pub fn kill(&self, c: &TextEditingController, from: usize, to: usize) {
        let held: Vec<Element> = self
            .elements(c)
            .into_iter()
            .filter(|e| from <= e.start && e.end <= to)
            .map(|e| Element {
                start: e.start - from,
                end: e.end - from,
                ..e
            })
            .collect();
        let text = c.text()[from..to].to_string();
        self.0.borrow_mut().kill = Some((text, held));
    }

    /// Puts the last kill back at the caret, its elements whole; `false`
    /// with nothing killed.
    pub fn yank(&self, c: &TextEditingController) -> bool {
        let Some((text, held)) = self.0.borrow().kill.clone() else {
            return false;
        };
        if text.is_empty() {
            return false;
        }
        let mut at = 0;
        for e in &held {
            if at < e.start {
                c.insert(&text[at..e.start]);
            }
            self.insert_element(c, &e.label, &e.payload);
            at = e.end;
        }
        if at < text.len() {
            c.insert(&text[at..]);
        }
        true
    }
}

impl ComposerDraft {
    /// Whether Up and Down walk the history now (`should_handle_navigation`):
    /// on an empty draft, or on the text they recalled last with the caret
    /// at one of its ends.
    pub fn walks_history(&self, c: &TextEditingController, entries: usize) -> bool {
        if entries == 0 {
            return false;
        }
        let text = c.text();
        if text.is_empty() {
            return true;
        }
        let selection = c.selection();
        if !selection.is_collapsed() {
            return false;
        }
        let caret = selection.extent_offset;
        if caret != 0 && caret != text.len() {
            return false;
        }
        matches!(&self.0.borrow().recalled, Some((_, last)) if **last == *text)
    }

    /// Up: the entry before the one recalled (the newest first), its text
    /// in the draft with the caret at its end; `false` at the oldest
    /// (`navigate_up`).
    pub fn history_up(&self, c: &TextEditingController, history: &[String]) -> bool {
        let at = match self.0.borrow().recalled {
            None => history.len().checked_sub(1),
            Some((0, _)) => None,
            Some((i, _)) => Some(i - 1),
        };
        let Some(at) = at else {
            return false;
        };
        self.recall(c, at, &history[at]);
        true
    }

    /// Down: the entry after the one recalled, or past the newest an empty
    /// draft and the walk over; `false` when not walking (`navigate_down`).
    pub fn history_down(&self, c: &TextEditingController, history: &[String]) -> bool {
        let Some((i, _)) = self.0.borrow().recalled.clone() else {
            return false;
        };
        if i + 1 < history.len() {
            self.recall(c, i + 1, &history[i + 1]);
        } else {
            self.0.borrow_mut().recalled = None;
            self.clear();
            c.set_text("");
        }
        true
    }

    fn recall(&self, c: &TextEditingController, at: usize, entry: &str) {
        let local = {
            let inner = self.0.borrow();
            inner.local.iter().rev().find(|l| l.sent == entry).cloned()
        };
        let (text, elements) = match local {
            Some(l) => (l.text, l.elements),
            None => (entry.to_string(), Vec::new()),
        };
        self.clear();
        c.set_text(&text);
        {
            let mut inner = self.0.borrow_mut();
            inner.recalled = Some((at, text.clone()));
            inner.elements = elements;
            inner.seen = Some(c.buffer());
        }
        c.set_selection(TextSelection::collapsed(text.len()));
    }

    /// A submit (or `Ctrl+C`'s clear) ends the walk and keeps the draft's
    /// placeholders for the entry it sent (`record_local_submission`).
    pub fn record_local(&self, c: &TextEditingController) {
        let elements = self.elements(c);
        let sent = self.expanded(c);
        let mut inner = self.0.borrow_mut();
        inner.recalled = None;
        if !elements.is_empty() {
            inner.local.push(Local {
                sent,
                text: c.text().to_string(),
                elements,
            });
        }
    }
}

/// The changed part of `b` against `a`: `(start, old_end, new_end)` after
/// the common head and tail, on character boundaries.
fn changed(a: &str, b: &str) -> (usize, usize, usize) {
    let head = a
        .char_indices()
        .zip(b.chars())
        .find(|((_, x), y)| x != y)
        .map_or(a.len().min(b.len()), |((i, _), _)| i);
    let tail = a[head..]
        .chars()
        .rev()
        .zip(b[head..].chars().rev())
        .take_while(|(x, y)| x == y)
        .map(|(x, _)| x.len_utf8())
        .sum::<usize>();
    (head, a.len() - tail, b.len() - tail)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pages::session::composer::strings;

    #[test]
    fn changed_finds_the_edited_middle() {
        assert_eq!(changed("abcdef", "abXdef"), (2, 3, 3));
        assert_eq!(changed("abc", "abc!"), (3, 3, 4));
        assert_eq!(changed("abc", "bc"), (0, 1, 0));
    }

    #[test]
    fn an_element_moves_with_an_edit_before_it_and_breaks_when_cut() {
        let c = TextEditingController::new("");
        let d = ComposerDraft::default();
        assert!(d.paste(&c, &"x".repeat(1001), |n| strings::EN.pasted(n)));
        assert_eq!(&*c.text(), "[Pasted Content 1001 chars]");
        c.set_selection(TextSelection::collapsed(0));
        c.insert("hi ");
        let e = d.elements(&c);
        assert_eq!((e[0].start, e[0].end), (3, 30));
        assert_eq!(d.expanded(&c), format!("hi {}", "x".repeat(1001)));
        c.replace(10, 11, "");
        assert!(d.elements(&c).is_empty());
    }
}
