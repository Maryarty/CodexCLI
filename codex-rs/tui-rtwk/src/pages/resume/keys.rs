//! The picker's keys, asked before the search field's own through
//! `EditingKeys` (rtwk's door for an app's keys): the search is always the
//! typing target, so typed characters and paste are the field's (it edits the
//! controller and tells `on_changed`) and every other key is the picker's:
//! it becomes a `Msg` and the field leaves it alone. Flutter: `Shortcuts` and
//! `Actions` above a `TextField`. Oracle: `resume_picker.rs` `handle_key`
//! and `handle_transcript_loading_key`.

use std::rc::Rc;

use rtwk::editing::KeyCx;
use rtwk::focus::KeyPress;
use rtwk::prelude::{KeyCode, Mods};

use codex_tui_core::screens::resume::Dir;

use super::Msg;
use crate::widgets::session_list::Move;

/// What the handler reads: where to tell a key and whether a transcript is
/// loading (it takes the keys then).
#[derive(Clone)]
pub struct KeyData {
    pub tell: Rc<dyn Fn(Msg)>,
    pub loading: bool,
}

/// Whether the picker takes `key` before the search field.
pub fn take(data: &KeyData, key: &KeyPress, kcx: KeyCx) -> bool {
    if kcx.read_only {
        return false;
    }
    let plain = key.mods == Mods::NONE;
    let ctrl = key.mods == Mods::CTRL;
    // Ctrl+C quits, whatever the picker is doing.
    if ctrl && key.ch == Some('c') {
        (data.tell)(Msg::Quit);
        return true;
    }
    if data.loading {
        // While a transcript loads the picker takes every key; Esc cancels.
        if key.code == KeyCode::Escape {
            (data.tell)(Msg::CancelTranscript);
        }
        return true;
    }
    let msg = match key.code {
        KeyCode::Escape => Msg::Escape,
        KeyCode::Enter => Msg::Accept,
        KeyCode::Up if plain => Msg::Move(Move::Step(false)),
        KeyCode::Down if plain => Msg::Move(Move::Step(true)),
        KeyCode::PageUp if plain => Msg::Move(Move::Page(false)),
        KeyCode::PageDown if plain => Msg::Move(Move::Page(true)),
        KeyCode::Home if plain => Msg::Move(Move::Edge(false)),
        KeyCode::End if plain => Msg::Move(Move::Edge(true)),
        KeyCode::Tab if key.mods == Mods::SHIFT => Msg::Focus(false),
        KeyCode::Tab => Msg::Focus(true),
        KeyCode::Left if plain => Msg::Change(Dir::Prev),
        KeyCode::Right if plain => Msg::Change(Dir::Next),
        KeyCode::Backspace => Msg::Backspace,
        KeyCode::Char if ctrl => match key.ch {
            Some('t') => Msg::Transcript,
            Some('e') => Msg::Expand,
            Some('o') => Msg::Density,
            Some('a') => Msg::Archive,
            // the other control keys mean nothing here
            _ => return true,
        },
        // typed text, shifted or not, is the field's
        KeyCode::Char if plain || key.mods == Mods::SHIFT => return false,
        // a key the picker does not know is not the search's either
        _ => return true,
    };
    (data.tell)(msg);
    true
}
