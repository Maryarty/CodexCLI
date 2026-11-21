//! The composer's own editing keys, asked before the `TextArea`'s through
//! `EditingKeys` (rtwk's door for an app's keys): the atomic elements
//! (Backspace, Delete, Left and Right take a placeholder whole), the kill
//! ring (`Ctrl+K`, `Ctrl+U`, `Ctrl+W` / `Alt+Backspace`, `Alt+D` kill,
//! `Ctrl+Y` yanks) and prompt history (Up and Down at the draft's ends).
//! Every other key is the field's. Oracle: `bottom_pane/textarea.rs`
//! (`input_with_keymap`, the default editor keymap of `keymap.rs`) and
//! `chat_composer_history.rs`.

use std::rc::Rc;

use rtwk::editing::{KeyCx, TextEditingController, TextSelection};
use rtwk::focus::KeyPress;
use rtwk::prelude::{KeyCode, Mods};

use super::ComposerMsg;
use super::draft::ComposerDraft;

/// What the handler reads besides the controller: the draft of the same
/// thread, the prompts history (newest last) and the composer to tell of a
/// key that is not an edit.
#[derive(Clone)]
pub struct KeyData {
    pub draft: ComposerDraft,
    pub history: Rc<[String]>,
    pub tell: Rc<dyn Fn(ComposerMsg)>,
}

fn only(key: &KeyPress, mods: Mods) -> bool {
    key.mods == mods
}

/// Whether the composer takes `key` for `c` before the field.
pub fn take_key(data: &KeyData, c: &TextEditingController, key: &KeyPress, kcx: KeyCx) -> bool {
    if kcx.read_only {
        return false;
    }
    let draft = &data.draft;
    let selection = c.selection();
    let caret = selection.extent_offset;
    let collapsed = selection.is_collapsed();
    let plain = only(key, Mods::NONE);
    let ctrl = only(key, Mods::CTRL);
    let alt = only(key, Mods::ALT);
    match (key.code, key.ch) {
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
        (KeyCode::Up, _) if plain => {
            let history = &data.history;
            draft.walks_history(c, history.len()) && draft.history_up(c, history)
        }
        (KeyCode::Down, _) if plain => {
            let history = &data.history;
            draft.walks_history(c, history.len()) && draft.history_down(c, history)
        }
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
