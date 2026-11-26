//! The shortcut menu of `/presets` on the page (oracle `chatwidget/quick_dials.rs`:
//! `open_quick_dial_menu`, `handle_quick_dial_menu`, `recall_quick_dial`): the command opens it
//! over the composer where presets can be used, a key of it either says a verb of the model (a
//! page of the presets opens: `Intent::Quick`) or recalls the preset of its digit, and any other key
//! closes it and goes on to the composer. Whether it is open is `BannerGate::quick_menu`, the
//! page's drawn state; the words and the drawing are the menu's (`sheets::select::hud`).

use rtwk::provider::read;
use rtwk::{Cx, Rebuild};

use codex_tui_core::app::{Session, SessionSlice};
use codex_tui_core::screens::select::Intent;
use codex_tui_core::screens::select::models;
use codex_tui_core::screens::select::quick_dials::{Composer, Gate, Open, Settings};
use codex_tui_core::screens::{ScreenIntent, ScreenStates, ScreenThreadIntent, chat};
use codex_tui_core::threads::{Queued, Ran, ThreadModel, UserMessage};

use crate::app::view::Hs;
use crate::pages::session::actions::{reserve_only, run};
use crate::pages::session::composer::queue::DeferredLine;
use crate::pages::session::composer::queue::{Delivery, message};
use crate::pages::session::composer::submission;
use crate::sheets::select::hud::{self, Step};

use super::{Chat, ChatState};

/// `key` for the menu: `Step::Open` is the command (it opens the menu where `quick_dials_available`),
/// the others are the presses of an open menu.
pub(super) fn key(
    key: Step,
    state: &ChatState,
    w: &Chat,
    cx: &mut Cx<'_, ChatState>,
    hs: &Hs,
) -> Rebuild {
    let menu = &state.banner.quick_menu;
    let gate = gate_of(cx);
    if key == Step::Open {
        // the model says why presets are unavailable; the menu opens only where they are
        let intent = Intent::Quick {
            gate,
            open: Open::Menu,
        };
        cx.emit(&hs.screen, ScreenIntent::Select(intent));
        menu.set(gate == Gate::Ready);
        return if menu.get() {
            Rebuild::Yes
        } else {
            Rebuild::No
        };
    }
    if !menu.replace(false) {
        return Rebuild::No;
    }
    let states = read::<ScreenStates, _>(cx);
    let quick = &states.select.quick;
    // a digit with a preset runs it as typed, behind the draft (`recall_quick_dial`)
    if let Some(input) = hud::recalled(key, quick) {
        if gate == Gate::Ready {
            w.ui.follow.jump_to_latest();
            recall(&input, w, cx, hs);
        } else {
            let intent = Intent::Quick {
                gate,
                open: Open::Menu,
            };
            cx.emit(&hs.screen, ScreenIntent::Select(intent));
        }
        return Rebuild::Yes;
    }
    let settings = key.wants_settings().then(|| settings_of(cx, &states));
    let composer = key.wants_composer().then(|| composer_of(w, cx, &states));
    let intent = hud::intent(key, gate, quick, settings, composer);
    if let Some(intent) = intent {
        cx.emit(&hs.screen, ScreenIntent::Select(intent));
    }
    Rebuild::Yes
}

/// `quick_dials_available`: a conversation that takes no direct input, or one not configured yet,
/// has no presets.
fn gate_of(cx: &mut Cx<'_, ChatState>) -> Gate {
    let states = read::<ScreenStates, _>(cx);
    let thread = read::<ThreadModel, _>(cx);
    if states.chat.flags.side_conversation_active || thread.screen_slots().chat.parent_owned {
        Gate::Blocked
    } else if !matches!(read::<SessionSlice, _>(cx).status, Session::Ready) {
        Gate::Starting
    } else {
        Gate::Ready
    }
}

/// The settings the sources of `s` read (`current_model_quick_dial` and the others).
fn settings_of(cx: &mut Cx<'_, ChatState>, states: &ScreenStates) -> Settings {
    let (session, thread) = (read::<SessionSlice, _>(cx), read::<ThreadModel, _>(cx));
    let facts = &states.chat.status;
    let current = models::Current {
        reserve: reserve_only(&session, states, &thread),
        ..models::Current::from_facts(facts)
    };
    Settings::from_facts(facts, current)
}

/// What the composer holds when an editor opens on it (`composer_quick_dial_text`): the text, and
/// whether images or linked mentions came with it (the model drops the text and says so).
fn composer_of(w: &Chat, cx: &mut Cx<'_, ChatState>, states: &ScreenStates) -> Composer {
    let draft = message(&submission(&w.ui), w, cx, states);
    Composer {
        images: !draft.images.is_empty() || !draft.remote_images.is_empty(),
        mentions: !draft.mentions.is_empty(),
        text: draft.text,
    }
}

/// The preset's input as typed (`queue_user_message_with_options`, `ParseSlash`): behind a running
/// turn it waits in the queue and is parsed when due, else it runs now through the dispatch a
/// typed line goes through. The draft, its history and its mode stay the user's.
fn recall(input: &str, w: &Chat, cx: &mut Cx<'_, ChatState>, hs: &Hs) {
    let thread = read::<ThreadModel, _>(cx);
    let message = UserMessage::text(input);
    if thread.is_running() {
        let queued =
            ScreenThreadIntent::Chat(chat::ThreadIntent::Queue(Queued::of(message, false)));
        cx.emit(&hs.thread_screen, (thread.id().clone(), queued));
    } else {
        run(
            input,
            Some(DeferredLine::Preset(&message)),
            w,
            cx,
            hs,
            Delivery::Drain,
            &mut Some(Ran::Submitted),
        );
    }
}
