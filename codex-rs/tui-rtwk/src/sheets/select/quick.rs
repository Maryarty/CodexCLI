//! The sheet of `/presets` (oracle `chatwidget/quick_dials.rs`): the page the model asked for (the
//! slot picker, the sources among the current settings or the editor), and over it the pages a pick
//! leads to (the editor, the slot picker, the confirmation), as the routes of a nested `Navigator`
//! (ADR-013 O1, the shape of the `/review` sheet). A page pushed over another keeps it under it, so
//! Esc shows it again where it was (the oracle's parent view keeps its highlight and the editor
//! its text), and a change settled on says the verb once and closes the whole sheet
//! (`dismiss_parent_on_child_accept`).
//!
//! Which row leads where is the model's pure `Menu::choose`, where the text of the editor leads is
//! `Menu::submit`; the stack of pages is the navigator's. What the model keeps is what IO needs:
//! the slots the config holds (read for the rows) and the write. A write the config refuses
//! brings the editor back as a sheet of its own (a new show). In Flutter this is the
//! nested-navigator recipe for a flow inside `showModalBottomSheet`.

mod editor;
mod entries;
pub mod hud;
mod lists;
pub mod strings;

pub use self::entries::debug_entries;

use std::rc::Rc;

use rtwk::navigation::{Navigator, PageRoute};
use rtwk::overlay::{DialogOptions, close_dialog};
use rtwk::provider::{read, select};
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::screens::select::Intent;
use codex_tui_core::screens::select::quick_dials::{Ask, Chosen, Menu, Quick, Target};
use codex_tui_core::screens::{ScreenIntent, ScreenStates};

use self::editor::{Answer, Editor};
use crate::app::view::Hs;
use crate::widgets::picker::{Event, Picker};
use crate::widgets::sheet::bottom;

/// The sheet: a navigator as tall as the sheet's cap whose home is `menu`, each page at the bottom
/// edge of it (where the oracle's pane is; `review_sheet.rs`).
pub fn sheet(menu: &Menu) -> AnyWidget {
    Navigator::new()
        .home(bottom(Page { menu: menu.clone() }))
        .back_key(None)
        .into()
}

/// The options the sheet is shown with: the bottom pane, built under the providers of the page
/// that asks.
pub fn options() -> DialogOptions {
    DialogOptions {
        use_root_navigator: false,
        ..crate::sheets::sheet_options()
    }
}

fn route(menu: Menu) -> PageRoute<()> {
    PageRoute::new(move |_| bottom(Page { menu: menu.clone() }))
}

/// One page of the sheet.
#[derive(Clone, Debug, PartialEq)]
struct Page {
    menu: Menu,
}

struct PageState;

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Picked(Event),
    Typed(Answer),
}

impl StatefulWidget for Page {
    type State = PageState;

    fn create_state(&self) -> PageState {
        PageState
    }
}

impl_widget!(stateful eq Page);

/// What the model holds of the saved presets, as the pages read it.
fn slots(cx: &mut Cx<'_, PageState>) -> Quick {
    read::<ScreenStates, _>(cx).select.quick.clone()
}

/// Where a pick or the text led: a page opens over this one, or the model is told (and a write
/// closes the sheet: the whole chain is dismissed once a change is settled).
fn go(cx: &mut Cx<'_, PageState>, chosen: Option<Chosen>) {
    match chosen {
        Some(Chosen::Push(next)) => drop(Navigator::of(cx).push::<()>(route(next))),
        Some(Chosen::Ask(ask)) => {
            let settled = matches!(ask, Ask::Write(_));
            let hs = read::<Hs, _>(cx);
            cx.emit(&hs.screen, ScreenIntent::Select(Intent::Preset(ask)));
            if settled {
                close_dialog::<_, ()>(cx, None);
            }
        }
        None => {}
    }
}

/// Esc: the page under this one shows again, or the sheet is gone.
fn back(cx: &mut Cx<'_, PageState>) {
    if Navigator::can_pop(cx) {
        Navigator::pop(cx);
    } else {
        close_dialog::<_, ()>(cx, None);
    }
}

impl State for PageState {
    type Widget = Page;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, w: &Page, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Picked(Event::Choose(id)) => {
                let chosen = w.menu.choose(&id, &slots(cx));
                go(cx, chosen);
            }
            Msg::Picked(Event::Cancel) | Msg::Typed(Answer::Back) => back(cx),
            // The slot lists and the sources have no secondary action, toggles, order or tabs.
            Msg::Picked(_) => {}
            Msg::Typed(Answer::Submit(text)) => {
                let chosen = w.menu.submit(&text, &slots(cx));
                go(cx, chosen);
            }
        }
        Rebuild::No
    }

    fn build(&self, w: &Page, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let words = strings::of(cx);
        let quick = select::<ScreenStates, Quick, _>(cx, |states| states.select.quick.clone());
        // The editor is a text prompt, not a list.
        let Some(list) = lists::list(&w.menu, &quick, words) else {
            let Menu::Editor { target, dial } = &w.menu else {
                return AnyWidget::default();
            };
            // The title says Edit when the slot holds a preset (`open_quick_dial_editor`), the
            // hint where the text goes next.
            let (title, hint) = match target {
                Target::Slot(slot) if quick.get(*slot).is_some() => {
                    (words.edit_preset(*slot), words.hint_review)
                }
                Target::Slot(_) => (words.new_title.to_string(), words.hint_review),
                Target::PickSlot => (words.new_title.to_string(), words.hint_slot),
            };
            return Editor {
                title,
                initial: dial.input.clone(),
                placeholder: words.placeholder,
                hint,
                mark: words.prompt_mark,
                on_answer: cx.map(Msg::Typed),
            }
            .into();
        };
        Picker::new(Rc::new(list), cx.map(Msg::Picked)).into()
    }
}
