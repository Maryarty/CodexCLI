//! The composer's paste-burst keys: a terminal without bracketed paste sends a
//! paste as a stream of `Char`, `Enter` and `Tab` keys, and the composer must
//! not read them as typing (an Enter would submit, a `?` would open the
//! shortcuts). Keys that arrive within 8 ms of each other are held by the
//! draft's `PasteBurst` (core, pure) and go in as one paste once the stream
//! stops; a first char is held for 9 ms in case a burst follows. The app's
//! clock is the time (so the kit's fake clock drives it) and a timer of the
//! app flushes what is held (`ComposerMsg::BurstDue`). Oracle:
//! `bottom_pane/chat_composer.rs` (`handle_input_basic_with_time`,
//! `handle_non_ascii_char`, `handle_submission_with_time`),
//! `chat_composer/paste_input.rs` and `reconnect.rs` (`handle_paste_enter`).

use std::rc::Rc;
use std::time::Duration;

use rtwk::editing::{TextEditingController, TextSelection};
use rtwk::focus::KeyPress;
use rtwk::prelude::Mods;

use codex_tui_core::kit::paste_burst::{CharDecision, FlushResult, PasteBurst};

use super::ComposerMsg;
use super::editing::{KeyData, typed_char};
use super::vim;
use crate::keymap::fixed::{Fixed, FixedKey};
use crate::widgets::composer_draft::ComposerDraft;

/// What a held first char waits for before it goes in as typing: the burst
/// interval plus a millisecond (`PasteBurst::recommended_flush_delay`).
pub const FLUSH_AFTER: Duration = Duration::from_millis(9);

/// What the detector reads of the app.
#[derive(Clone)]
pub struct BurstEnv {
    /// The app clock's time.
    pub now: Rc<dyn Fn() -> Duration>,
    /// Calls the composer back after [`FLUSH_AFTER`] to flush what is held.
    pub arm: Rc<dyn Fn()>,
    /// The placeholder of a paste of that many chars.
    pub label: Rc<dyn Fn(usize) -> String>,
    /// The placeholder of the n-th attached image.
    pub image: Rc<dyn Fn(usize) -> String>,
}

/// Whether the burst detector takes `key`: held or buffered (`true`), or
/// the composer's own keys and the field go on with it (`false`), after
/// anything held that this key must not join has gone in.
pub fn take(
    data: &KeyData,
    env: &BurstEnv,
    c: &TextEditingController,
    key: &KeyPress,
    skill_space: Option<usize>,
) -> bool {
    let now = (env.now)();
    // a burst that stopped, or a first char nothing followed, goes in
    // before this key is looked at
    flush_due(data, env, c, now, skill_space);
    let text_key = key.mods.0 & !Mods::SHIFT.0 == 0;
    match (key.fixed(), key.ch) {
        (Fixed::Char, Some(ch)) if text_key => char_key(data, env, c, ch, now, skill_space),
        (Fixed::Enter, _) => enter_key(data, env, c, key, now),
        (Fixed::Tab, _) if key.mods == Mods::NONE => tab_key(data, env, now),
        _ => {
            // a shortcut or a navigation key: what is held goes in first
            // (`flush_before_modified_input`) and the next key starts a new
            // window (`clear_window_after_non_char`)
            flush_held(data, env, c);
            data.draft.burst(PasteBurst::clear_window_after_non_char);
            false
        }
    }
}

/// A due flush (the held first char as typing, a stopped burst as a paste);
/// `true` when something went in (`handle_paste_burst_flush`).
pub fn flush_due(
    data: &KeyData,
    env: &BurstEnv,
    c: &TextEditingController,
    now: Duration,
    skill_space: Option<usize>,
) -> bool {
    flush_due_with(&data.draft, &*data.tell, env, c, now, skill_space)
}

/// The app timer's turn (`BurstDue`): a due flush goes in (`true`); when
/// nothing was due and the draft is still in a burst (an active burst waits
/// longer than the first char, 60 ms on Windows), the timer is armed again,
/// as the oracle's `request_redraw_in` is while `is_in_paste_burst()`.
pub fn on_timer(
    draft: &ComposerDraft,
    tell: &dyn Fn(ComposerMsg),
    env: &BurstEnv,
    c: &TextEditingController,
    now: Duration,
    skill_space: Option<usize>,
) -> bool {
    if flush_due_with(draft, tell, env, c, now, skill_space) {
        return true;
    }
    if draft.in_burst() {
        (env.arm)();
    }
    false
}

/// [`flush_due`] for a caller with no key (the timer).
pub fn flush_due_with(
    draft: &ComposerDraft,
    tell: &dyn Fn(ComposerMsg),
    env: &BurstEnv,
    c: &TextEditingController,
    now: Duration,
    skill_space: Option<usize>,
) -> bool {
    match draft.burst(|b| b.flush_if_due(now)) {
        FlushResult::Paste(text) => {
            apply_paste(draft, tell, env, c, &text);
            true
        }
        FlushResult::Typed(ch) => {
            if !typed_char(draft, tell, c, ch, skill_space) {
                c.insert(&ch.to_string());
                // Insert mode records it for `.`
                vim::pasted(draft, c, &ch.to_string());
            }
            tell(ComposerMsg::Pasted);
            true
        }
        FlushResult::None => false,
    }
}

/// What is held goes in as a paste now (`flush_before_modified_input`).
pub fn flush_held(data: &KeyData, env: &BurstEnv, c: &TextEditingController) {
    flush_held_with(&data.draft, &*data.tell, env, c);
}

/// [`flush_held`] for a caller with no key data (an explicit paste).
pub fn flush_held_with(
    draft: &ComposerDraft,
    tell: &dyn Fn(ComposerMsg),
    env: &BurstEnv,
    c: &TextEditingController,
) {
    if let Some(text) = draft.burst(PasteBurst::flush_before_modified_input) {
        apply_paste(draft, tell, env, c, &text);
    }
}

/// Puts `text` in as a paste (`apply_paste`): a large one as a placeholder,
/// a small one as text at the selection; the Enter window ends with it.
pub fn apply_paste(
    draft: &ComposerDraft,
    tell: &dyn Fn(ComposerMsg),
    env: &BurstEnv,
    c: &TextEditingController,
    text: &str,
) {
    if !draft.paste(c, text, |chars| (env.label)(chars), |n| (env.image)(n)) {
        c.insert(text);
    }
    vim::pasted(draft, c, text);
    draft.burst(PasteBurst::clear_after_explicit_paste);
    tell(ComposerMsg::Pasted);
}

fn char_key(
    data: &KeyData,
    env: &BurstEnv,
    c: &TextEditingController,
    ch: char,
    now: Duration,
    skill_space: Option<usize>,
) -> bool {
    let draft = &data.draft;
    // punctuation typed right after a `$skill` completion is classified
    // before it can take the completion's space
    let completion = skill_space.filter(|&at| {
        ",.;:!?)]}。，、！？》".contains(ch)
            && c.selection().is_collapsed()
            && c.selection().extent_offset == at + 1
            && c.text().get(at..at + 1) == Some(" ")
    });
    if !ch.is_ascii() && completion.is_none() {
        return non_ascii(data, env, c, ch, now);
    }
    let held = match draft.burst(|b| b.on_plain_char(ch, now)) {
        CharDecision::BufferAppend | CharDecision::BeginBufferFromPending => {
            draft.burst(|b| b.append_char_to_buffer(ch, now));
            true
        }
        CharDecision::BeginBuffer { retro_chars } => {
            begin_buffer(draft, c, ch, now, retro_chars as usize)
        }
        CharDecision::RetainFirstChar => {
            // the first fast char waits to see if a burst follows; the
            // completion's space waits with it
            draft.keep_skill_space(completion);
            true
        }
    };
    if held {
        (env.arm)();
        return true;
    }
    flush_held(data, env, c);
    false
}

/// A non-ASCII char (an IME's) is never held, but still counts toward a
/// burst (`handle_non_ascii_char`).
fn non_ascii(
    data: &KeyData,
    env: &BurstEnv,
    c: &TextEditingController,
    ch: char,
    now: Duration,
) -> bool {
    let draft = &data.draft;
    if draft.burst(|b| b.try_append_char_if_active(ch, now)) {
        (env.arm)();
        return true;
    }
    flush_held(data, env, c);
    let held = match draft.burst(|b| b.on_plain_char_no_hold(now)) {
        Some(CharDecision::BufferAppend) => {
            draft.burst(|b| b.append_char_to_buffer(ch, now));
            true
        }
        Some(CharDecision::BeginBuffer { retro_chars }) => {
            begin_buffer(draft, c, ch, now, retro_chars as usize)
        }
        _ => false,
    };
    if held {
        (env.arm)();
        return true;
    }
    flush_held(data, env, c);
    false
}

/// A burst begins after chars were inserted as typing: if what was typed
/// looks like a paste, it is taken back out of the text into the burst, and
/// `ch` joins it (`decide_begin_buffer`, `retract_paste_burst`).
fn begin_buffer(
    draft: &ComposerDraft,
    c: &TextEditingController,
    ch: char,
    now: Duration,
    retro_chars: usize,
) -> bool {
    let caret = c.selection().extent_offset;
    let text = c.text();
    let mut at = caret.min(text.len());
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    let grab = draft.burst(|b| b.decide_begin_buffer(now, &text[..at], retro_chars));
    let Some(grab) = grab else {
        return false;
    };
    if grab.grabbed.is_empty() {
        draft.burst(|b| b.append_char_to_buffer(ch, now));
        return true;
    }
    c.replace(grab.start_byte, at, "");
    c.set_selection(TextSelection::collapsed(grab.start_byte));
    draft.burst(|b| b.append_char_to_buffer(ch, now));
    true
}

/// Enter inside a burst is a newline of the paste, not a submit; in the
/// window after one it still is, except on a `/command` line
/// (`handle_paste_enter`).
fn enter_key(
    data: &KeyData,
    env: &BurstEnv,
    c: &TextEditingController,
    key: &KeyPress,
    now: Duration,
) -> bool {
    let draft = &data.draft;
    if draft.burst(|b| b.is_active() && b.append_control_char_if_active('\n', now)) {
        (env.arm)();
        return true;
    }
    let slash = !draft.shell() && c.text().lines().next().unwrap_or("").starts_with('/');
    if key.mods == Mods::NONE
        && !slash
        && draft.burst(|b| b.newline_should_insert_instead_of_submit(now))
    {
        c.insert("\n");
        draft.burst(|b| b.extend_window(now));
        (data.tell)(ComposerMsg::Pasted);
        return true;
    }
    false
}

/// A Tab inside a burst, or right behind a direct-inserted char, is a
/// literal tab of the paste (`append_recent_paste_separator`).
fn tab_key(data: &KeyData, env: &BurstEnv, now: Duration) -> bool {
    let draft = &data.draft;
    let took = draft.burst(|b| {
        if b.append_control_char_if_active('\t', now) {
            return true;
        }
        if b.direct_insert_newline_should_insert(now) {
            b.begin_with_retro_grabbed(String::new(), now);
            return b.append_control_char_if_active('\t', now);
        }
        false
    });
    if took {
        (env.arm)();
    }
    took
}
