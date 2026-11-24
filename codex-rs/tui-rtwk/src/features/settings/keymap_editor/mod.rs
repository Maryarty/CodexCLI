//! The `/keymap` editor (oracle `keymap_setup/`): the picker of the actions of the keymap by
//! tab, the menu of an action, the menu of the bindings to replace, the capture of a key and
//! the keypress inspector. In Flutter it is one bottom sheet over a stack of pages (the
//! oracle's view stack): each page is a widget of its own, the picker below keeps nothing of
//! the pages above it but the tab and the action it was left on. What is bound, what comes from
//! the config and what a capture does to it are the feature's (`Intent::KeymapCaptured`,
//! `KeymapCleared`, ADR-013); the sheet asks for the verb and waits for the keymap it is
//! offered to change, then returns to the picker on the action it edited
//! (`return_to_keymap_picker`). The conflict sheet of a key the keymap cannot take is the
//! feature's wish, shown by the presenter (`ConflictSheet`).

mod capture;
mod conflict;
mod debug;
mod lists;
pub mod strings;

use std::rc::Rc;

use rtwk::overlay::{Dialog, close_dialog};
use rtwk::provider::select;
use rtwk::text::Text;
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::features::FeatureStates;
use codex_tui_core::features::settings::capture::Intent as EditIntent;
use codex_tui_core::features::settings::keymap;
use codex_tui_core::features::settings::{Intent, KeymapPicker, Offer, Which};
use codex_tui_core::keymap::Keymap;

pub use self::capture::{Capture, Done, Mode, spec_of, stroke_of};
pub use self::conflict::{CaptureSheet, ConflictSheet};
pub use self::debug::Inspector;
use self::lists::Pick;
use super::strings as frame;
use crate::features::verb;
use crate::keymap::keymap as live_keymap;
use crate::widgets::picker::{Event, Picker};

/// A page of the sheet.
#[derive(Clone, Debug, PartialEq)]
enum Page {
    List,
    Menu {
        context: String,
        action: String,
    },
    Replace {
        context: String,
        action: String,
    },
    Capture {
        context: String,
        action: String,
        intent: EditIntent,
        mode: Mode,
    },
    Debug,
}

/// The sheet of the keymap editor.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeymapSheet {
    /// Opens on the keypress inspector (`/keymap debug`).
    pub inspect: bool,
}

/// The edit that was asked for: the sheet returns to the picker when the offer changes.
struct Awaiting {
    context: String,
    action: String,
    before: Option<KeymapPicker>,
}

pub struct KeymapState {
    stack: Vec<Page>,
    /// The row the picker was left on, and its tab.
    selected: Option<String>,
    tab: Option<String>,
    awaiting: Option<Awaiting>,
    /// A new picker (the keymap changed) is not the one before it.
    generation: u32,
    offer: Option<KeymapPicker>,
    keymap: Rc<Keymap>,
}

#[derive(Clone, Debug)]
pub enum Msg {
    Picked(Event),
    Captured(Done),
    Closed,
}

impl StatefulWidget for KeymapSheet {
    type State = KeymapState;

    fn create_state(&self) -> KeymapState {
        KeymapState {
            stack: vec![if self.inspect {
                Page::Debug
            } else {
                Page::List
            }],
            selected: None,
            tab: None,
            awaiting: None,
            generation: 0,
            offer: None,
            keymap: Rc::new(Keymap::defaults()),
        }
    }
}

impl_widget!(stateful eq KeymapSheet);

fn offered(cx: &mut Cx<'_, KeymapState>) -> Option<KeymapPicker> {
    select::<FeatureStates, Option<KeymapPicker>, _>(cx, |states| {
        match states.settings.offers.get(&Which::Keymap) {
            Some(Offer::Keymap(picker)) => Some(picker.clone()),
            _ => None,
        }
    })
}

impl KeymapState {
    fn pop(&mut self, cx: &mut Cx<'_, Self>) {
        self.stack.pop();
        if self.stack.is_empty() {
            close_dialog::<_, ()>(cx, None);
        }
    }

    fn top(&self) -> &Page {
        self.stack.last().unwrap_or(&Page::List)
    }

    fn custom(&self, context: &str, action: &str) -> bool {
        let id = lists::id_of(context, action);
        self.offer
            .as_ref()
            .and_then(|offer| offer.tabs.first())
            .and_then(|tab| {
                tab.items
                    .iter()
                    .find(|item| lists::id_of(&item.context, &item.action) == id)
            })
            .is_some_and(|item| item.custom)
    }

    fn menu(&self, context: &str, action: &str, cx: &mut Cx<'_, Self>) -> crate::widgets::picker::list::List {
        lists::menu(
            context,
            action,
            &self.keymap,
            self.custom(context, action),
            strings::of(cx),
        )
    }

    /// A key was captured: the verb, and back to the page below; the picker comes back when
    /// the keymap it is offered changes.
    fn captured(&mut self, key: String, cx: &mut Cx<'_, Self>) {
        if let Page::Capture {
            context,
            action,
            intent,
            ..
        } = self.top().clone()
        {
            self.awaiting = Some(Awaiting {
                context: context.clone(),
                action: action.clone(),
                before: self.offer.clone(),
            });
            verb(
                cx,
                Intent::KeymapCaptured {
                    context,
                    action,
                    key,
                    intent,
                },
            );
        }
        self.pop(cx);
    }

    fn picked(&mut self, event: Event, cx: &mut Cx<'_, Self>) -> Rebuild {
        match (self.top().clone(), event) {
            (_, Event::Cancel) => self.pop(cx),
            (Page::List, Event::Tab(id)) => {
                self.tab = Some(id);
                return Rebuild::No;
            }
            (Page::List, Event::Choose(id)) if id == lists::DEBUG_ID => {
                self.stack.push(Page::Debug);
            }
            (Page::List, Event::Choose(id)) => {
                if let Some((context, action)) = lists::action_of(&id) {
                    let page = Page::Menu {
                        context: context.to_owned(),
                        action: action.to_owned(),
                    };
                    self.selected = Some(id);
                    self.stack.push(page);
                }
            }
            (Page::Menu { context, action }, Event::Choose(id)) => {
                let list = self.menu(&context, &action, cx);
                let enabled = list
                    .items
                    .iter()
                    .any(|item| item.id == id && !item.is_disabled());
                if let (true, Some(pick)) = (enabled, Pick::of(&id)) {
                    self.menu_pick(pick, context, action, cx);
                }
            }
            (Page::Replace { context, action }, Event::Choose(id)) => {
                if let Some((binding, chord)) = lists::replace_of(&id) {
                    self.stack.pop();
                    self.stack.push(Page::Capture {
                        context,
                        action,
                        intent: EditIntent::ReplaceOne {
                            old_key: binding.to_owned(),
                        },
                        mode: if chord { Mode::Chord } else { Mode::Single },
                    });
                }
            }
            _ => return Rebuild::No,
        }
        Rebuild::Yes
    }

    fn menu_pick(&mut self, pick: Pick, context: String, action: String, cx: &mut Cx<'_, Self>) {
        let capture = |intent: EditIntent, mode: Mode| Page::Capture {
            context: context.clone(),
            action: action.clone(),
            intent,
            mode,
        };
        let page = match pick {
            Pick::Set | Pick::Replace | Pick::ReplaceAll => {
                capture(EditIntent::ReplaceAll, Mode::Single)
            }
            Pick::Add => capture(EditIntent::AddAlternate, Mode::Single),
            Pick::ReplaceOne => Page::Replace {
                context: context.clone(),
                action: action.clone(),
            },
            Pick::Leader => capture(EditIntent::ReplaceAll, Mode::Leader),
            Pick::ConfigureLeader => Page::Menu {
                context: "global".to_owned(),
                action: "leader".to_owned(),
            },
            Pick::Chord | Pick::ReplaceChord => capture(EditIntent::ReplaceAll, Mode::Chord),
            Pick::AddChord => capture(EditIntent::AddAlternate, Mode::Chord),
            Pick::Remove => {
                self.awaiting = Some(Awaiting {
                    context: context.clone(),
                    action: action.clone(),
                    before: self.offer.clone(),
                });
                verb(cx, Intent::KeymapCleared { context, action });
                return;
            }
            Pick::Back => {
                self.pop(cx);
                return;
            }
        };
        self.stack.push(page);
    }
}

impl State for KeymapState {
    type Widget = KeymapSheet;
    type Msg = Msg;

    fn did_change_dependencies(&mut self, _: &KeymapSheet, cx: &mut Cx<'_, Self>) {
        let offer = offered(cx);
        self.keymap = live_keymap(cx);
        // The keymap the edit made came back: the picker, on the action that was edited.
        if let Some(awaiting) = &self.awaiting
            && offer != awaiting.before
        {
            self.selected = Some(lists::id_of(&awaiting.context, &awaiting.action));
            self.tab = None;
            self.stack = vec![Page::List];
            self.generation += 1;
            self.awaiting = None;
        }
        self.offer = offer;
    }

    fn update(&mut self, msg: Msg, _: &KeymapSheet, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Picked(event) => return self.picked(event, cx),
            Msg::Captured(Done::Key(key)) => self.captured(key, cx),
            Msg::Captured(Done::Cancel) | Msg::Closed => self.pop(cx),
        }
        Rebuild::Yes
    }

    fn build(&self, _: &KeymapSheet, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let s = strings::of(cx);
        let words = frame::of(cx);
        let Some(offer) = &self.offer else {
            return Dialog::bare(Text::new(String::new())).into();
        };
        let key = (u32::try_from(self.stack.len()).unwrap_or(0) << 16) | (self.generation & 0xffff);
        let picked = cx.map(Msg::Picked);
        let page: AnyWidget = match self.top() {
            Page::List => {
                let (list, extras) =
                    lists::picker(offer, s, words, self.tab.as_deref(), self.selected.as_deref());
                Picker::new(Rc::new(list), picked).extras(extras).into()
            }
            Page::Menu { context, action } => {
                Picker::new(Rc::new(self.menu(context, action, cx)), picked).into()
            }
            Page::Replace { context, action } => Picker::new(
                Rc::new(lists::replace_menu(context, action, &self.keymap, s)),
                picked,
            )
            .into(),
            Page::Capture {
                context,
                action,
                mode,
                ..
            } => Capture {
                context: context.clone(),
                action: action.clone(),
                current: keymap::summary(&self.keymap, context, action),
                mode: *mode,
                on_done: cx.map(Msg::Captured),
            }
            .into(),
            Page::Debug => Inspector {
                on_close: cx.map(|(): ()| Msg::Closed),
            }
            .into(),
        };
        page.with_key(key)
    }
}
