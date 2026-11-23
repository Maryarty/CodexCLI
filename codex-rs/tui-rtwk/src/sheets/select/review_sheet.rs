//! The sheet of `/review` (oracle `chatwidget/review_popups.rs`): the presets, over them the
//! base-branch picker, the commit picker and the custom instructions prompt, as the routes of a
//! nested `Navigator` (ADR-013 O1, the shape of the `/plugins` pilot). The home is the presets; a
//! page pushed over it keeps the presets' picker under it, so Esc shows them again with the cursor
//! on the row that opened the page (the oracle's parent view keeps its highlight), and a pick of a
//! target says the verb once and closes the sheet (`dismiss_parent_on_child_accept`).
//!
//! The lists are the host's: the presets page asks for one (`Intent::ReviewList`) and reads the
//! answer from the model's fact (`Review::listing`, by the request it carries); the page of a list
//! is pushed once per request, between builds. Which row leads where is the model's pure
//! `Menu::choose`; the stack of pages is the navigator's. In Flutter this is the nested-navigator
//! recipe for a flow inside `showModalBottomSheet`, each page a `ConsumerStatefulWidget`.

use std::rc::Rc;

use rtwk::layout::{Align, Alignment};
use rtwk::navigation::{Navigator, PageRoute};
use rtwk::overlay::{DialogOptions, close_dialog};
use rtwk::provider::{read, select};
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::screens::select::Intent;
use codex_tui_core::screens::select::review::{Chosen, Listing, Menu};
use codex_tui_core::screens::{ScreenIntent, ScreenStates};
use codex_tui_core::threads::ReviewTarget;

use super::prompt::Answer;
use super::review::{self, strings};
use crate::app::view::Hs;
use crate::widgets::picker::{Event, Picker};

/// The sheet: a navigator as tall as the sheet's cap whose home is the presets, each page at the
/// bottom edge of it (where the oracle's pane is; `features/plugins/sheet.rs`).
pub fn sheet() -> AnyWidget {
    Navigator::new()
        .home(bottom(Page {
            menu: Menu::Presets,
        }))
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

fn bottom(page: impl Into<AnyWidget>) -> AnyWidget {
    Align::new(Alignment::BOTTOM_CENTER, page).into()
}

fn route(menu: Menu) -> PageRoute<()> {
    PageRoute::new(move |_| bottom(Page { menu: menu.clone() }))
}

/// One page of the sheet.
#[derive(Clone, Debug, PartialEq)]
struct Page {
    menu: Menu,
}

struct PageState {
    /// The request whose list this page (the presets) has pushed the page of.
    pushed: Option<u64>,
}

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Picked(Event),
    Typed(Answer),
    /// A list of the host arrived: its page goes over the presets.
    Listed(Listing),
}

impl StatefulWidget for Page {
    type State = PageState;

    fn create_state(&self) -> PageState {
        PageState { pushed: None }
    }
}

impl_widget!(stateful eq Page);

/// Says a verb to the model.
fn say(cx: &mut Cx<'_, PageState>, intent: Intent) {
    let hs = read::<Hs, _>(cx);
    cx.emit(&hs.screen, ScreenIntent::Select(intent));
}

/// The review starts and the sheet is gone.
fn start(cx: &mut Cx<'_, PageState>, target: ReviewTarget) {
    say(cx, Intent::Review(target));
    close_dialog::<_, ()>(cx, None);
}

fn back(cx: &mut Cx<'_, PageState>, menu: &Menu) {
    if *menu == Menu::Presets {
        close_dialog::<_, ()>(cx, None);
    } else {
        Navigator::of(cx).pop(cx);
    }
}

impl State for PageState {
    type Widget = Page;
    type Msg = Msg;

    // The presets wait for the host's list: the page of it is pushed when the answer to a request
    // arrives, once. Between builds: what a change of the model asks of the navigator is never
    // done inside one.
    fn did_change_dependencies(&mut self, w: &Page, cx: &mut Cx<'_, Self>) {
        if w.menu != Menu::Presets {
            return;
        }
        let listed = select::<ScreenStates, Option<(u64, Listing)>, _>(cx, |states| {
            states
                .select
                .review
                .listing()
                .map(|(request, listing)| (request, listing.clone()))
        });
        if let Some((request, listing)) = listed
            && self.pushed != Some(request)
        {
            self.pushed = Some(request);
            let go = cx.send(Msg::Listed(listing));
            cx.emit(&go, ());
        }
    }

    fn update(&mut self, msg: Msg, w: &Page, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Listed(listing) => {
                drop(Navigator::of(cx).push::<()>(route(listing.menu())));
            }
            Msg::Picked(Event::Choose(id)) => match w.menu.choose(&id) {
                Some(Chosen::Ask(git)) => say(cx, Intent::ReviewList(git)),
                Some(Chosen::Prompt) => {
                    drop(Navigator::of(cx).push::<()>(route(Menu::Instructions)));
                }
                Some(Chosen::Target(target)) => start(cx, target),
                None => {}
            },
            Msg::Picked(Event::Cancel) => back(cx, &w.menu),
            // The review lists have no secondary action, toggles, order or tabs.
            Msg::Picked(_) => {}
            Msg::Typed(Answer::Submit(text)) => {
                let text = text.trim();
                if !text.is_empty() {
                    start(
                        cx,
                        ReviewTarget::Custom {
                            instructions: text.to_string(),
                        },
                    );
                }
            }
            Msg::Typed(Answer::Back) => back(cx, &w.menu),
        }
        Rebuild::No
    }

    fn build(&self, w: &Page, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let words = strings::of(cx);
        // The instructions are a text prompt, not a list.
        let Some(list) = review::list(&w.menu, None, words) else {
            return review::instructions(cx.map(Msg::Typed), words).into();
        };
        Picker::new(Rc::new(list), cx.map(Msg::Picked)).into()
    }
}
