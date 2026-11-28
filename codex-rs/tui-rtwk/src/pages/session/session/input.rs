//! Input routing of the page: the composer node's key handler (what the page sees before the
//! field: browsing the prompts, the selection's keys, the `change_host` chord, the banner's
//! numbers, `?` and `←`) and the bindings the page's focus and its footer's hints carry.

use std::cell::Cell;
use std::rc::Rc;

use rtwk::binding::{AppBinding, InputListener};
use rtwk::focus::{Focus, KeyBindings, KeyEventResult, KeyPress};
use rtwk::gestures::MouseTracker;
use rtwk::layout::Stack;
use rtwk::provider::Provider;
use rtwk::{AnyWidget, Cx, Key};
use rtwk_core::input::{InputEvent, KeyKind};

use codex_tui_core::keymap::actions::{
    CHAT_INTERRUPT_TURN, CHAT_OPEN_PRESETS, COMPOSER_SUBMIT, GLOBAL_CHANGE_HOST,
    GLOBAL_SELECT_TRANSCRIPT, PAGER_CLOSE_TRANSCRIPT, VOICE_TOGGLE_VOICE_MUTE,
};
use codex_tui_core::keymap::actions::{
    CHAT_NEXT_PERMISSION_MODE, CHAT_PREVIOUS_PERMISSION_MODE, CHAT_TOGGLE_VOICE,
};
use codex_tui_core::keymap::fixed;

use crate::keymap::press::PaneActivationGuard;
use crate::keymap::{chords_of, holds_ctrl_or_alt, pressed_fixed, unmodified};
use crate::pages::session::composer::editing::completes_editor_chord;
use crate::pages::session::composer::vim;
use crate::pages::session::keys;
use crate::pages::session::live::browse::{self, Browse};
use crate::pages::session::live::provider_error;
use crate::pages::session::transcript::activity_focus::{self, Step as ActivityStep};
use crate::pages::session::transcript::copy::{self as copying, Step as CopyStep};
use crate::pages::session::transcript::select::{self as caret, Step};
use crate::pages::session::transcript::selection::Key as SelectionKey;
use crate::pages::session::transcript::{self, find};
use crate::sheets::select::hud::Step as PresetStep;
use crate::widgets::finder::{Finder, Mode as FindMode};
use crate::widgets::thread_ui::{Input, ThreadUi};

use super::frame::Frame;
use super::{ChatMsg, ChatState, Presentation};

#[derive(Clone, Default)]
struct VoiceKeys(Rc<Cell<bool>>);

fn voice_keys(state: &ChatState) -> Rc<Cell<bool>> {
    state.find.borrow().held(VoiceKeys::default).0
}

pub(super) fn may_close_transcript(ui: &ThreadUi) -> bool {
    ui.modes.detailed()
        && !ui.modes.browsing()
        && ui.input() == Input::Plain
        && !ui.selection.held()
        && !caret::of(&ui.find).extending()
        && !activity_focus::of(&ui.find).focused()
}

/// `?` (the shortcut overlay, `footer_shortcuts_default`) and `←` (the agents page,
/// `handle_agents_navigation`) on an EMPTY composer belong to the page; with text they go to
/// the field.
pub(super) fn install(state: &mut ChatState, workspace: bool, cx: &mut Cx<'_, ChatState>) {
    let send = cx.local_sender();
    let composer = state.composer.clone();
    let draft = state.draft.clone();
    let modes = state.modes.clone();
    let selection = state.selection.clone();
    let mouse = MouseTracker::of(cx);
    let finder = state.find.clone();
    let control = state.control.clone();
    let voice_controls = voice_keys(state);
    let clock = AppBinding::maybe_of(cx).map(|binding| binding.clock());
    let (banner, blocked) = (state.banner.clone(), state.banner_blocked.clone());
    let keymap = state.keymap.clone();
    let activation = Provider::<PaneActivationGuard>::maybe_of(cx, false);
    state.node.set_on_key_event(Some(Rc::new(move |_, key| {
        if let Some(activation) = &activation {
            activation.fresh(&key);
        }
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
        let finder = finder.borrow().clone();
        let activity = activity_focus::of(&finder);
        // a fold or a cell of the transcript in focus (F4): the keys of a list, any other key lets
        // go of it and goes on to the field (`handle_disclosure_key`). They come before Find's
        // while a match is read, and not while its query is edited.
        if finder.mode() != FindMode::Editing
            && !modes.borrow().detailed()
            && let Some(step) = activity.key(&keymap.borrow(), &key)
        {
            return match step {
                ActivityStep::Taken => KeyEventResult::Handled,
                ActivityStep::Left => KeyEventResult::Ignored,
            };
        }
        if let Some(step) = find_selection_key(&finder, &key) {
            send(ChatMsg::Selection(step));
            return KeyEventResult::Handled;
        }
        if finder.reading()
            && let Some(taken) = find::key(finder.mode(), &key)
            && finder.take(taken)
        {
            return KeyEventResult::Handled;
        }
        // copy mode (`/copy`): its keys are the only ones there are, whatever the composer holds
        // (`handle_copy_mode_key`: a key that is none of its own does nothing)
        let copier = copying::mode_of(&finder);
        if copier.active()
            && !finder.active()
            && let Some(step) = copier.key(&key)
        {
            match step {
                CopyStep::Moved => {}
                CopyStep::Copy => {
                    // the target is what the area selects: the copy sends it, and the target's
                    // own source where it has one (`copying` keeps it for that copy)
                    copier.copying();
                    send(ChatMsg::Selection(SelectionKey::CopyAndFollow));
                    modes.borrow().hold(Input::Plain);
                }
                CopyStep::Exit => {
                    copier.end(&selection.borrow());
                    if let Some(follow) = copier.reader() {
                        follow.jump_to_latest();
                    }
                    modes.borrow().hold(Input::Plain);
                }
                CopyStep::Page(pages) => send(ChatMsg::Page(pages)),
            }
            return KeyEventResult::Handled;
        }
        // select mode: the caret's keys come before the field's, while Find is not asking
        // (`handle_select_mode_key`); a key of the mode ends here, one that is not goes on
        let caret = caret::of(&finder);
        if caret.take(
            &keymap.borrow(),
            &key,
            &selection.borrow(),
            &finder,
            &|step| match step {
                Step::Copy => send(ChatMsg::Selection(SelectionKey::Copy)),
                Step::CopyAndFollow => {
                    send(ChatMsg::Selection(SelectionKey::CopyAndFollow));
                    modes.borrow().hold(Input::Plain);
                }
                Step::Exit => modes.borrow().hold(Input::Plain),
                Step::Moved | Step::Find => {}
            },
        ) {
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
        // The mouse range owns arrows as well as its copy and clear keys while it is held.
        if !finder.active() && caret.extend(&key, &selection.borrow()) {
            return KeyEventResult::Handled;
        }
        if !caret.active()
            && (selection.borrow().held() || caret.extending())
            && let Some(step) = transcript::selection::key(&key)
        {
            send(ChatMsg::Selection(step));
            return KeyEventResult::Handled;
        }
        if let Some(mouse) = &mouse
            && selection.borrow().owns_pointer_point(mouse.presses())
        {
            let step = transcript::selection::key(&key);
            if step != Some(SelectionKey::CopyAndFollow) {
                selection.borrow().clear_pointer_point();
            }
            if matches!(
                step,
                Some(SelectionKey::CopyAndFollow | SelectionKey::Clear)
            ) {
                return KeyEventResult::Handled;
            }
        }
        // A detailed reader's close belongs to it before ordinary chat shortcuts. Plain Esc
        // still goes through the editor and the page's Escape ladder.
        let may_close = {
            let modes = modes.borrow();
            modes.detailed()
                && !modes.browsing()
                && !finder.active()
                && !copier.active()
                && !caret.active()
                && !caret.extending()
                && !selection.borrow().held()
                && !activity.focused()
        };
        let close_action = ChatMsg::Action(keys::Action::CloseTranscript);
        if may_close
            && !pressed_fixed(fixed::ESCAPE, &key)
            && crate::keymap::pressed(&keymap.borrow(), PAGER_CLOSE_TRANSCRIPT, &key)
        {
            send(close_action);
            return KeyEventResult::Handled;
        }
        // the leader chords: `change_host` (`Ctrl+X b`) is live while the backend control shows
        // (`visible_new_backend_control`), `open_presets` (`Ctrl+X p`) opens the menu of `/presets`
        // where the page is free (`no_modal_or_popup_active`), `select_transcript` (`Ctrl+X [`)
        // begins select mode; all work with a draft in the composer.
        let (hosts, presets, selects, voice, close, panes) = {
            let keymap = keymap.borrow();
            if workspace && let Some(action) = super::super::side_pane::single(&keymap, &key) {
                send(ChatMsg::Workspace(action));
                return KeyEventResult::Handled;
            }
            let hosts = if control.get() {
                chords_of(&keymap, GLOBAL_CHANGE_HOST)
            } else {
                Vec::new()
            };
            (
                hosts,
                chords_of(&keymap, CHAT_OPEN_PRESETS),
                chords_of(&keymap, GLOBAL_SELECT_TRANSCRIPT),
                if voice_controls.get() {
                    chords_of(&keymap, VOICE_TOGGLE_VOICE_MUTE)
                } else {
                    Vec::new()
                },
                if may_close {
                    chords_of(&keymap, PAGER_CLOSE_TRANSCRIPT)
                } else {
                    Vec::new()
                },
                if workspace {
                    super::super::side_pane::chords(&keymap)
                } else {
                    Vec::new()
                },
            )
        };
        // The prefix of a leader chord is held in the draft, as the editor chords' first key is
        // (one `KeyChordMatcher::pending`), and never expires (`persistent`). What comes with it
        // held (`KeyChordMatcher::advance`): a completion of the leader's, Esc or the prefix again
        // cancels, a completion of an editor chord of that prefix is the field's, any other key
        // is an ordinary one.
        let now = clock.as_ref().map(|clock| clock.now()).unwrap_or_default();
        let draft_now = draft.borrow().clone();
        let held = if draft_now.chord_is_leader() {
            draft_now.chord().map(|(first, _)| first)
        } else {
            None
        };
        if let Some(first) = held {
            if let Some((_, _, action)) = panes
                .iter()
                .find(|(prefix, done, _)| prefix.accepts_press(&first) && done.accepts_press(&key))
            {
                draft_now.set_chord(None);
                send(ChatMsg::Workspace(action.clone()));
                return KeyEventResult::Handled;
            }
            if close
                .iter()
                .any(|(prefix, done)| prefix.accepts_press(&first) && done.accepts_press(&key))
            {
                draft_now.set_chord(None);
                send(close_action);
                return KeyEventResult::Handled;
            }
            if voice
                .iter()
                .any(|(prefix, done)| prefix.accepts_press(&first) && done.accepts_press(&key))
            {
                draft_now.set_chord(None);
                send(ChatMsg::Action(keys::Action::VoiceMute));
                return KeyEventResult::Handled;
            }
            if hosts.iter().any(|(_, done)| done.accepts_press(&key)) {
                draft_now.set_chord(None);
                send(ChatMsg::Cycle);
                return KeyEventResult::Handled;
            }
            if presets.iter().any(|(_, done)| done.accepts_press(&key)) {
                draft_now.set_chord(None);
                send(ChatMsg::Presets(PresetStep::Open));
                return KeyEventResult::Handled;
            }
            if selects.iter().any(|(_, done)| done.accepts_press(&key)) {
                draft_now.set_chord(None);
                send(ChatMsg::Action(keys::Action::Select));
                return KeyEventResult::Handled;
            }
            let again = hosts
                .iter()
                .chain(&presets)
                .chain(&selects)
                .chain(&voice)
                .chain(&close)
                .any(|(prefix, _)| prefix.accepts_press(&first) && prefix.accepts_press(&key));
            if again
                || panes.iter().any(|(prefix, _, _)| {
                    prefix.accepts_press(&first) && prefix.accepts_press(&key)
                })
                || (unmodified(&key) && pressed_fixed(fixed::DISMISS, &key))
            {
                draft_now.set_chord(None);
                return KeyEventResult::Handled;
            }
            // Normal mode and a search in the field read their own keys, not the editor chords
            let own = vim::mode(&draft_now);
            if !own.normal
                && !own.searching
                && completes_editor_chord(&keymap.borrow(), &first, &key)
            {
                return KeyEventResult::Ignored;
            }
            draft_now.set_chord(None);
        }
        if hosts
            .iter()
            .chain(&presets)
            .chain(&selects)
            .chain(&voice)
            .chain(&close)
            .any(|(prefix, _)| prefix.accepts_press(&key))
            || panes
                .iter()
                .any(|(prefix, _, _)| prefix.accepts_press(&key))
        {
            draft_now.set_leader_chord(Some((key, now)));
            return KeyEventResult::Handled;
        }
        if let Some(pages) = transcript::follow::pager_key(&key, &composer.borrow()) {
            send(ChatMsg::Page(pages));
            return KeyEventResult::Handled;
        }
        // Ctrl+Home, Ctrl+End and their Alt twins jump the transcript wherever the caret is
        // (`JumpTarget::from_key`; the field would move its caret to the text's ends)
        if let Some(jump) = jump_of(&key) {
            send(jump);
            return KeyEventResult::Handled;
        }
        if !finder.active() && !caret.active() && (selection.borrow().held() || caret.extending()) {
            caret.end_extension();
            selection.borrow().controller().clear();
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
        // the edit-queued key goes to the questions or the queue before the field's own keys
        // (`handle_question_key`, the queue's pop)
        if blocked.get().edit
            && !draft.borrow().in_burst()
            && crate::keymap::pressed(
                &keymap.borrow(),
                codex_tui_core::keymap::actions::CHAT_EDIT_QUEUED_MESSAGE,
                &key,
            )
        {
            send(ChatMsg::EditQueued);
            return KeyEventResult::Handled;
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
        // Normal mode's search keys come first (`wants_vim_search_key`: `?` searches backward)
        if vim::wants_search_key(&draft.borrow(), &keymap.borrow(), &key) {
            return KeyEventResult::Ignored;
        }
        if pressed_fixed(fixed::OPEN_SHORTCUTS, &key) {
            send(ChatMsg::Shortcuts);
        } else if pressed_fixed(fixed::OPEN_AGENTS_FROM_LEFT, &key)
            // the plain Left only (`key_hint::plain(KeyCode::Left)`): Shift+Left is the edit-queued
            // key when it is bound, and nothing when it is not
            && unmodified(&key)
            && vim::agents_key_available(&draft.borrow(), &keymap.borrow())
            && !crate::keymap::pressed(
                &keymap.borrow(),
                codex_tui_core::keymap::actions::CHAT_EDIT_QUEUED_MESSAGE,
                &key,
            )
        {
            send(ChatMsg::Agents);
        } else {
            return KeyEventResult::Ignored;
        }
        KeyEventResult::Handled
    })));
}

fn find_selection_key(finder: &Finder, key: &KeyPress) -> Option<SelectionKey> {
    if finder.paused() && !caret::of(finder).active() {
        transcript::selection::key(key)
    } else {
        None
    }
}

/// The jump a key asks of the transcript: its first row (Ctrl+Home, Alt+<) or its last (Ctrl+End,
/// Alt+>).
fn jump_of(key: &rtwk::focus::KeyPress) -> Option<ChatMsg> {
    if pressed_fixed(fixed::TRANSCRIPT_TOP, key) || pressed_fixed(fixed::TRANSCRIPT_TOP_ALT, key) {
        Some(ChatMsg::Beginning)
    } else if pressed_fixed(fixed::TRANSCRIPT_LATEST, key)
        || pressed_fixed(fixed::TRANSCRIPT_LATEST_ALT, key)
    {
        Some(ChatMsg::Latest)
    } else {
        None
    }
}

/// The page's focus over `page`: the keys the page binds (interrupt, quit, Esc, the debug key,
/// the transcript's pages, Tab's queue, the read-only thread's and the provider lock's keys, the
/// chat's actions), keyed by the two states that change which of them are bound.
pub(super) fn focus(
    state: &ChatState,
    cx: &mut Cx<'_, ChatState>,
    frame: &Frame,
    ui: &ThreadUi,
    presentation: Presentation,
    page: Stack,
) -> AnyWidget {
    let (thread, keymap) = (&frame.thread, &frame.keymap);
    let finder = state.find.clone();
    let find_keymap = keymap.clone();
    let send = cx.local_sender();
    // The Find field is a sibling of the composer. Its selection keys reach this common ancestor.
    let mut focus = Focus::new(page).on_key_event(move |_, key| {
        let finder = finder.borrow();
        if let Some(step) = find_selection_key(&finder, &key) {
            send(ChatMsg::Selection(step));
            KeyEventResult::Handled
        } else if finder.mode() == FindMode::Editing
            && crate::keymap::pressed(&find_keymap, PAGER_CLOSE_TRANSCRIPT, &key)
        {
            // The query has first use of the key, including an editor binding or typed text.
            // A key it did not use must not close the reader or reach a colliding chat action.
            KeyEventResult::Handled
        } else {
            KeyEventResult::Ignored
        }
    });
    for (id, msg) in [
        (fixed::INTERRUPT_OR_QUIT, ChatMsg::CtrlC),
        (fixed::QUIT_ON_EMPTY, ChatMsg::CtrlD),
        (fixed::ESCAPE, ChatMsg::Escape),
        (fixed::DEBUG_KEY, ChatMsg::DebugMenu),
        (fixed::TRANSCRIPT_PAGE_UP, ChatMsg::Page(-1)),
        (fixed::TRANSCRIPT_PAGE_DOWN, ChatMsg::Page(1)),
        (fixed::TRANSCRIPT_TOP, ChatMsg::Beginning),
        (fixed::TRANSCRIPT_TOP_ALT, ChatMsg::Beginning),
        (fixed::TRANSCRIPT_LATEST, ChatMsg::Latest),
        (fixed::TRANSCRIPT_LATEST_ALT, ChatMsg::Latest),
    ] {
        for key in crate::keymap::presses(keymap, id) {
            if presentation != Presentation::Solo
                && super::super::side_pane::claims_key(keymap, &key)
            {
                continue;
            }
            // A configured turn binding wins over page-level shortcuts such as the debug key.
            // Physical Escape still runs the overlay and editor precedence in ChatState::escape.
            if id != fixed::ESCAPE
                && thread.can_interrupt()
                && crate::keymap::pressed(keymap, CHAT_INTERRUPT_TURN, &key)
            {
                continue;
            }
            focus = focus.bind(key, cx.send(msg.clone()));
        }
    }
    // Tab queues while a task runs and submits otherwise, but not a
    // shell command (`queue_keys` in `handle_key_event_without_popup`).
    if (thread.task_running() || !state.draft_shell)
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
    let voice_controls =
        rtwk::provider::select::<codex_tui_core::threads::ThreadModel, bool, _>(cx, |thread| {
            thread.screen_slots().voice.can_mute()
        });
    voice_keys(state).set(voice_controls);
    for (action, key) in keys::presses(keymap) {
        if presentation != Presentation::Solo && super::super::side_pane::claims_key(keymap, &key) {
            continue;
        }
        if action == keys::Action::CloseTranscript {
            continue;
        }
        if action.id() == CHAT_TOGGLE_VOICE
            && [CHAT_NEXT_PERMISSION_MODE, CHAT_PREVIOUS_PERMISSION_MODE]
                .into_iter()
                .any(|id| crate::keymap::pressed(keymap, id, &key))
        {
            continue;
        }
        if action.id() == VOICE_TOGGLE_VOICE_MUTE {
            if !voice_controls {
                continue;
            }
        } else if voice_controls && crate::keymap::pressed(keymap, VOICE_TOGGLE_VOICE_MUTE, &key) {
            continue;
        }
        focus = focus.bind(key, cx.send(ChatMsg::Action(action)));
    }
    let focus =
        AnyWidget::from(focus).with_key(Key::value((thread.read_only(), frame.provider.is_some())));
    let ui = ui.clone();
    let node = state.node.clone();
    let menu = state.banner.quick_menu.clone();
    let submit_on_enter = crate::keymap::press_for(fixed::ENTER)
        .is_some_and(|key| crate::keymap::pressed(keymap, COMPOSER_SUBMIT, &key));
    let activation = Provider::<PaneActivationGuard>::maybe_of(cx, false);
    // Ordinary Enter returns an empty composer to the latest row. The native
    // focus path treats repeats as presses, so filter this gesture before it.
    InputListener::with_raw_key_repeats(focus, move |event| {
        let InputEvent::Key(key) = event else {
            return false;
        };
        if activation
            .as_ref()
            .is_some_and(|guard| guard.suppresses(key))
            && node.has_primary_focus()
        {
            return true;
        }
        key.kind == KeyKind::Repeat
            && submit_on_enter
            && crate::keymap::event_is_fixed(fixed::ENTER, key)
            && node.has_primary_focus()
            && ui.follow.away()
            && ui.input() == Input::Plain
            && !ui.modes.browsing()
            && !ui.selection.held()
            && !activity_focus::of(&ui.find).focused()
            && !menu.get()
            && ui.composer.text().is_empty()
            && !ui.draft.shell()
            && ui.draft.remote_count() == 0
            && !ui.draft.in_burst()
            && !ui.draft.searching()
    })
    .into()
}

/// The footer's entries are these bindings' hints: every key it shows is bound here.
pub(super) fn hints(cx: &mut Cx<'_, ChatState>, frame: &Frame, focus: AnyWidget) -> AnyWidget {
    use codex_tui_core::screens::select::reasoning::Direction;
    let (keymap, pages) = (&frame.keymap, frame.strings.pages);
    // Shift+Left is the edit-queued key before it is Left's agents (the chat's own keys first)
    let mut keys = crate::keymap::bindings(
        keymap,
        codex_tui_core::keymap::actions::CHAT_EDIT_QUEUED_MESSAGE,
        &cx.send(ChatMsg::EditQueued),
    );
    keys.push(("left".to_string(), cx.send(ChatMsg::Agents)));
    keys.push((pages.key_shortcuts.to_string(), cx.send(ChatMsg::Shortcuts)));
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
    if frame.thread.can_interrupt() {
        let escape = crate::keymap::names(keymap, fixed::ESCAPE);
        keys.extend(
            crate::keymap::bindings(keymap, CHAT_INTERRUPT_TURN, &cx.send(ChatMsg::Interrupt))
                .into_iter()
                .filter(|(key, _)| !escape.contains(key)),
        );
    }
    KeyBindings::new(crate::keymap::first_of_each(keys), focus)
        .hints([
            ("left", pages.hint_agents),
            (pages.key_shortcuts, pages.hint_shortcuts),
        ])
        .into()
}
