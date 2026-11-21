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
    !kcx.read_only && edit_key(data, c, key)
}

/// Whether the composer takes `key` for the editable field of `c`.
fn edit_key(data: &KeyData, c: &TextEditingController, key: &KeyPress) -> bool {
    let draft = &data.draft;
    let selection = c.selection();
    let caret = selection.extent_offset;
    let collapsed = selection.is_collapsed();
    let plain = only(key, Mods::NONE);
    let ctrl = only(key, Mods::CTRL);
    let alt = only(key, Mods::ALT);
    // a `$skill` completion's space lasts for the next key only
    let skill_space = draft.take_skill_space();
    match (key.code, key.ch) {
        // typed punctuation right after a skill completion takes the
        // completion's space (`skill_completion_space_for_char`)
        (KeyCode::Char, Some(ch))
            if (plain || only(key, Mods::SHIFT))
                && collapsed
                && ",.;:!?)]}。，、！？》".contains(ch)
                && skill_space
                    .is_some_and(|at| caret == at + 1 && c.text().get(at..at + 1) == Some(" ")) =>
        {
            let at = caret - 1;
            c.replace(at, caret, &ch.to_string());
            true
        }
        // Shell mode (`sync_bash_mode_from_text`): a `!` typed at the
        // draft's start becomes the prompt; Backspace there and Esc on an
        // empty draft leave it.
        (KeyCode::Char, Some('!'))
            if (plain || only(key, Mods::SHIFT)) && collapsed && caret == 0 && !draft.shell() =>
        {
            draft.set_shell(true);
            (data.tell)(ComposerMsg::Shell);
            true
        }
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
        (KeyCode::Up | KeyCode::Down, _) if plain => {
            let history = &data.history;
            let shell = draft.shell();
            let took = draft.walks_history(c, history.len())
                && if key.code == KeyCode::Up {
                    draft.history_up(c, history)
                } else {
                    draft.history_down(c, history)
                };
            // an entry's leading `!` is shell mode again
            if draft.shell() != shell {
                (data.tell)(ComposerMsg::Shell);
            }
            took
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
            history: Rc::from(Vec::new()),
            tell: Rc::new(|_| {}),
        }
    }

    fn at(text: &str, caret: usize) -> TextEditingController {
        let c = TextEditingController::new(text);
        c.set_selection(TextSelection::collapsed(caret));
        c
    }

    fn press(d: &KeyData, c: &TextEditingController, key: KeyPress) {
        assert!(edit_key(d, c, &key), "the composer takes {key:?}");
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
}
