//! Input routing of the page: the composer node's key handler (what the page sees before the
//! field: browsing the prompts, the selection's keys, the `change_host` chord, the banner's
//! numbers, `?` and `←`) and the bindings the page's focus and its footer's hints carry.

use std::rc::Rc;

use rtwk::focus::{Focus, KeyBindings, KeyCombo, KeyEventResult};
use rtwk::layout::Stack;
use rtwk::{AnyWidget, Cx, Key};

use codex_tui_core::keymap::actions::{CHAT_OPEN_PRESETS, GLOBAL_CHANGE_HOST};
use codex_tui_core::keymap::{ActionId, Binding, Keymap, fixed};

use crate::keymap::{combo_of, holds_ctrl_or_alt, pressed_fixed, unmodified};
use crate::pages::session::keys;
use crate::pages::session::live::browse::{self, Browse};
use crate::pages::session::live::provider_error;
use crate::pages::session::transcript::{self, find};
use crate::sheets::select::hud::Step as PresetStep;

use super::frame::Frame;
use super::{ChatMsg, ChatState};

/// The chords of an action (the leader's prefix and its completion: `Ctrl+X b` for `change_host`,
/// `Ctrl+X p` for `open_presets` by default), as the keymap resolved them.
fn chords_of(keymap: &Keymap, action: ActionId) -> Vec<(KeyCombo, KeyCombo)> {
    keymap
        .bindings(action)
        .into_iter()
        .filter_map(|binding| match binding {
            Binding::Chord { prefix, completion } => {
                Some((combo_of(&prefix)?, combo_of(&completion)?))
            }
            Binding::Single(_) => None,
        })
        .collect()
}

/// `?` (the shortcut overlay, `footer_shortcuts_default`) and `←` (the agents page,
/// `handle_agents_navigation`) on an EMPTY composer belong to the page; with text they go to
/// the field.
pub(super) fn install(state: &mut ChatState, cx: &mut Cx<'_, ChatState>) {
    let send = cx.local_sender();
    let composer = state.composer.clone();
    let draft = state.draft.clone();
    let modes = state.modes.clone();
    let selection = state.selection.clone();
    let (leader, control) = (state.leader.clone(), state.control.clone());
    let (banner, blocked) = (state.banner.clone(), state.banner_blocked.clone());
    let keymap = state.keymap.clone();
    state.node.set_on_key_event(Some(Rc::new(move |_, key| {
        // the menu of `/presets` is open: its keys come first, any other key closes it and goes on
        // to the field (`handle_quick_dial_menu`)
        if banner.quick_menu.get() {
            let pressed = PresetStep::of(&key);
            if pressed != PresetStep::Hold {
                send(ChatMsg::Presets(pressed));
            }
            return if pressed.is_taken() {
                KeyEventResult::Handled
            } else {
                KeyEventResult::Ignored
            };
        }
        // a match being read (Find accepted): `Ctrl+P`, `Ctrl+N`, Enter (swallowed), Esc and
        // `Ctrl+C` are Find's (`handle_search_key`), whatever the composer holds
        if let Some(finder) = find::reading(&selection.borrow())
            && let Some(taken) = find::key(finder.mode(), &key)
            && finder.take(taken)
        {
            return KeyEventResult::Handled;
        }
        // browsing the prompts: its keys come first, any other key ends it
        // and goes on to the field
        if modes.borrow().browsing() {
            match browse::key(&keymap.borrow(), &key) {
                Some(step) => {
                    send(ChatMsg::Browse(step));
                    return KeyEventResult::Handled;
                }
                None => send(ChatMsg::Browse(Browse::Leave)),
            }
        }
        // the copy keys and Esc are the selection's while something is selected
        // (`handle_selection_key`): they come before the field's
        if selection.borrow().controller().has_selection()
            && let Some(step) = transcript::selection::key(&key)
        {
            send(ChatMsg::Selection(step));
            return KeyEventResult::Handled;
        }
        // the leader chords: `change_host` (`Ctrl+X b`) is live while the backend control shows
        // (`visible_new_backend_control`), `open_presets` (`Ctrl+X p`) opens the menu of `/presets`
        // where the page is free (`no_modal_or_popup_active`); both work with a draft in the
        // composer.
        let (hosts, presets) = {
            let keymap = keymap.borrow();
            let hosts = if control.get() {
                chords_of(&keymap, GLOBAL_CHANGE_HOST)
            } else {
                Vec::new()
            };
            (hosts, chords_of(&keymap, CHAT_OPEN_PRESETS))
        };
        if leader.replace(false) {
            if hosts.iter().any(|(_, done)| done.accepts_press(&key)) {
                send(ChatMsg::Cycle);
                return KeyEventResult::Handled;
            }
            if presets.iter().any(|(_, done)| done.accepts_press(&key)) {
                send(ChatMsg::Presets(PresetStep::Open));
                return KeyEventResult::Handled;
            }
        } else if hosts
            .iter()
            .chain(&presets)
            .any(|(prefix, _)| prefix.accepts_press(&key))
        {
            leader.set(true);
            return KeyEventResult::Handled;
        }
        if let Some(pages) = transcript::follow::pager_key(&key, &composer.borrow()) {
            send(ChatMsg::Page(pages));
            return KeyEventResult::Handled;
        }
        // The banner's numbers and Esc: only on an empty composer, off a
        // paste, with no turn running and no modifier
        // (`handle_inline_banner_key`; a popup needs text in the field).
        if composer.borrow().text().is_empty()
            && !draft.borrow().in_burst()
            && unmodified(&key)
            && let Some(shown) = banner.keys.get().backend
            && !blocked.get().running
        {
            let keys = banner.keys.get();
            if pressed_fixed(fixed::DISMISS, &key) && keys.dismissible && !blocked.get().overlay {
                send(ChatMsg::BannerDismiss(shown));
                return KeyEventResult::Handled;
            }
            if let Some(row) = key
                .text()
                .and_then(|digit| digit.parse::<usize>().ok())
                .filter(|n| (1..=keys.actions).contains(n))
            {
                banner.options.set(row - 1);
                send(ChatMsg::BannerPick(shown, row - 1));
                return KeyEventResult::Handled;
            }
        }
        // Ctrl+D on an empty composer quits (`on_ctrl_d`); the field would take it for
        // delete-forward, which is nothing here
        if composer.borrow().text().is_empty()
            && !draft.borrow().in_burst()
            && pressed_fixed(fixed::QUIT_ON_EMPTY, &key)
        {
            send(ChatMsg::CtrlD);
            return KeyEventResult::Handled;
        }
        // keys held as a paste are the paste's, not the shortcuts'
        // (`is_in_paste_burst`)
        if !composer.borrow().text().is_empty()
            || draft.borrow().in_burst()
            || holds_ctrl_or_alt(&key)
        {
            return KeyEventResult::Ignored;
        }
        if pressed_fixed(fixed::OPEN_SHORTCUTS, &key) {
            send(ChatMsg::Shortcuts);
        } else if pressed_fixed(fixed::OPEN_AGENTS_FROM_LEFT, &key) {
            send(ChatMsg::Agents);
        } else {
            return KeyEventResult::Ignored;
        }
        KeyEventResult::Handled
    })));
}

/// The page's focus over `page`: the keys the page binds (interrupt, quit, Esc, the debug key,
/// the transcript's pages, Tab's queue, the read-only thread's and the provider lock's keys, the
/// chat's actions), keyed by the two states that change which of them are bound.
pub(super) fn focus(
    state: &ChatState,
    cx: &mut Cx<'_, ChatState>,
    frame: &Frame,
    page: Stack,
) -> AnyWidget {
    let (thread, keymap) = (&frame.thread, &frame.keymap);
    let mut focus = Focus::new(page);
    for (id, msg) in [
        (fixed::INTERRUPT_OR_QUIT, ChatMsg::CtrlC),
        (fixed::QUIT_ON_EMPTY, ChatMsg::CtrlD),
        (fixed::ESCAPE, ChatMsg::Escape),
        (fixed::DEBUG_KEY, ChatMsg::DebugMenu),
        (fixed::TRANSCRIPT_PAGE_UP, ChatMsg::Page(-1)),
        (fixed::TRANSCRIPT_PAGE_DOWN, ChatMsg::Page(1)),
    ] {
        for key in crate::keymap::presses(keymap, id) {
            focus = focus.bind(key, cx.send(msg.clone()));
        }
    }
    // Tab queues while a turn runs and submits otherwise, but not a
    // shell command (`queue_keys` in `handle_key_event_without_popup`).
    if (frame.run.running() || !state.draft_shell)
        && !state.draft_empty
        && frame.searching.is_none()
    {
        for key in crate::keymap::presses(keymap, fixed::TAB) {
            focus = focus.bind(key, cx.send(ChatMsg::QueueDraft));
        }
    }
    if thread.read_only() {
        focus = focus.can_request_focus(true).autofocus();
        for (id, msg) in [
            (fixed::RETRY_WRITER, ChatMsg::RetryWriter),
            (fixed::FORK_WRITER, ChatMsg::ForkWriter),
            (fixed::QUIT_READ_ONLY, ChatMsg::QuitReadOnly),
        ] {
            for key in crate::keymap::presses(keymap, id) {
                focus = focus.bind(key, cx.send(msg.clone()));
            }
        }
    }
    if frame.provider.is_some() {
        focus = focus.can_request_focus(true).autofocus();
        for (key, msg) in provider_error::keys(keymap, frame.can_continue) {
            focus = focus.bind(key, cx.send(msg));
        }
    }
    for (action, key) in keys::presses(keymap) {
        focus = focus.bind(key, cx.send(ChatMsg::Action(action)));
    }
    AnyWidget::from(focus).with_key(Key::value((thread.read_only(), frame.provider.is_some())))
}

/// The footer's entries are these bindings' hints: every key it shows is bound here.
pub(super) fn hints(cx: &mut Cx<'_, ChatState>, frame: &Frame, focus: AnyWidget) -> AnyWidget {
    use codex_tui_core::screens::select::reasoning::Direction;
    let (keymap, pages) = (&frame.keymap, frame.strings.pages);
    let mut keys = vec![
        ("left".to_string(), cx.send(ChatMsg::Agents)),
        (pages.key_shortcuts.to_string(), cx.send(ChatMsg::Shortcuts)),
    ];
    keys.extend(crate::keymap::bindings(
        keymap,
        codex_tui_core::keymap::actions::CHAT_EDIT_QUEUED_MESSAGE,
        &cx.send(ChatMsg::EditQueued),
    ));
    keys.extend(crate::keymap::bindings(
        keymap,
        codex_tui_core::keymap::actions::CHAT_DECREASE_REASONING_EFFORT,
        &cx.send(ChatMsg::Reasoning(Direction::Lower)),
    ));
    keys.extend(crate::keymap::bindings(
        keymap,
        codex_tui_core::keymap::actions::CHAT_INCREASE_REASONING_EFFORT,
        &cx.send(ChatMsg::Reasoning(Direction::Raise)),
    ));
    KeyBindings::new(crate::keymap::first_of_each(keys), focus)
        .hints([
            ("left", pages.hint_agents),
            (pages.key_shortcuts, pages.hint_shortcuts),
        ])
        .into()
}
