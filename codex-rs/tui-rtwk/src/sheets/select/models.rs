//! The sheet of `/model` (oracle `chatwidget/model_popups.rs`): the quick menu, over it the
//! reasoning menu of a model and over that the advanced one, as the routes of a nested `Navigator`
//! (ADR-013 O1, the shape of `/review`). The home is the page the show opened on (`Loading
//! models…` while the catalog is empty, else the quick menu); a page pushed over it keeps the
//! parent's picker under it, so Esc shows it again with the cursor on the row that opened the
//! page (the oracle's parent view keeps its highlight).
//!
//! What a row leads to is the model's pure `ModelMenu::choose` (an `Onward`: a page over this one, a
//! page in place of it, a final pick, or the plan scope question alone); the stack of pages is the
//! navigator's. The facts the pages read are the catalog's: the models themselves, the provider
//! write state (a write closes the sheet: `dismiss_stale_model_picker`) and the answer of the
//! request the loading page waits for (by the request it carries). A page that cannot go on
//! (the list could not be read, a refresh left no choice) says so once (`Intent::GiveUp`, the
//! history's line) and leaves; only the sheet knows the user was still looking at it. In Flutter
//! this is the nested-navigator recipe for a flow inside `showModalBottomSheet`, each page a
//! `ConsumerStatefulWidget` that watches the catalog provider.

mod list;

use std::rc::Rc;

use rtwk::navigation::{Navigator, PageRoute, Route};
use rtwk::overlay::{DialogOptions, close_dialog};
use rtwk::provider::{read, select};
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::screens::select::models::{Catalog, Heading, ModelMenu, Next, Onward};
use codex_tui_core::screens::select::{Intent, Opening};
use codex_tui_core::screens::{ScreenIntent, ScreenStates};

use super::strings;
use crate::app::debug::seeded::Seeded;
use crate::app::view::Hs;
use crate::widgets::picker::{Event, Picker};
use crate::widgets::sheet::bottom;

/// The sheet: a navigator whose home is `menu`, each page at the bottom edge of it (where the
/// oracle's pane is).
pub fn sheet(menu: ModelMenu) -> AnyWidget {
    Navigator::new()
        .home(bottom(Page { menu }))
        .back_key(None)
        .into()
}

/// The sheet of a debug entry: the one its script's show opens (`Ev::Showing`), none when the
/// script made no show.
pub fn seeded(seeded: &Seeded<'_>) -> AnyWidget {
    seeded
        .core
        .screens()
        .select
        .shows
        .pending()
        .find_map(|show| match &show.opening {
            Opening::Models(menu) => Some(sheet(menu.clone())),
            _ => None,
        })
        .unwrap_or_default()
}

/// The options the sheet is shown with: the bottom pane, built under the providers of the page
/// that asks.
pub fn options() -> DialogOptions {
    DialogOptions {
        use_root_navigator: false,
        ..crate::sheets::sheet_options()
    }
}

fn route(menu: ModelMenu) -> PageRoute<()> {
    PageRoute::new(move |_| bottom(Page { menu: menu.clone() }))
}

/// One page of the sheet.
#[derive(Clone, Debug, PartialEq)]
struct Page {
    menu: ModelMenu,
}

struct PageState {
    /// The route this page is, to take it off the navigator when it gives up.
    route: Option<Route>,
    /// The page decided what the catalog means for it (once: it closes, goes on or leaves).
    decided: bool,
}

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Picked(Event),
    /// What the catalog made of the page (`ModelMenu::next`).
    Next(Next),
}

impl StatefulWidget for Page {
    type State = PageState;

    fn create_state(&self) -> PageState {
        PageState {
            route: None,
            decided: false,
        }
    }
}

impl_widget!(stateful eq Page);

fn catalog_of<S>(cx: &mut Cx<'_, S>) -> Catalog {
    select::<ScreenStates, Catalog, _>(cx, |states| states.select.catalog.clone())
}

/// The catalog as it is now: what a pick is judged against (not watched).
fn catalog_now<S>(cx: &mut Cx<'_, S>) -> Catalog {
    read::<ScreenStates, _>(cx).select.catalog.clone()
}

/// Says a verb to the model.
fn say(cx: &mut Cx<'_, PageState>, intent: Intent) {
    let hs = read::<Hs, _>(cx);
    cx.emit(&hs.screen, ScreenIntent::Select(intent));
}

fn close(cx: &mut Cx<'_, PageState>) {
    close_dialog::<_, ()>(cx, None);
}

/// Esc: the page under this one shows again, the first page closes the sheet.
fn back(cx: &mut Cx<'_, PageState>) {
    let nav = Navigator::of(cx);
    if nav.can_pop() {
        nav.pop(cx);
    } else {
        close(cx);
    }
}

impl PageState {
    /// What a choice does to the pages.
    fn step(&self, cx: &mut Cx<'_, Self>, step: Onward) {
        match step {
            Onward::None => {}
            Onward::Push(menu) => drop(Navigator::of(cx).push::<()>(route(menu))),
            Onward::Replace(menu) => drop(Navigator::of(cx).push_replacement::<()>(route(menu))),
            Onward::Done(selection) => {
                say(cx, Intent::Settle(selection));
                close(cx);
            }
            // The plan scope question stands alone: every page under it goes.
            Onward::Prompt(menu) => {
                drop(Navigator::of(cx).push_and_remove_until::<()>(route(menu), |_| false));
            }
        }
    }

    /// The page leaves the sheet, the pages over it stay; the last one closes it.
    fn leave(&self, cx: &mut Cx<'_, Self>) {
        let nav = Navigator::of(cx);
        match &self.route {
            Some(route) if nav.can_pop() => {
                if nav.routes().contains(route) {
                    nav.remove_route(route);
                }
            }
            _ => close(cx),
        }
    }
}

impl State for PageState {
    type Widget = Page;
    type Msg = Msg;

    // The model's facts, between builds: what a change of them asks of the navigator is never
    // done inside one. `build` selects the catalog the same way, so the registration is kept
    // whichever of the two ran last.
    fn did_change_dependencies(&mut self, w: &Page, cx: &mut Cx<'_, Self>) {
        self.route = Route::maybe_of(cx);
        let catalog = catalog_of(cx);
        if !self.decided
            && let Some(next) = w.menu.next(&catalog)
        {
            self.decided = true;
            let go = cx.send(Msg::Next(next));
            cx.emit(&go, ());
        }
    }

    fn update(&mut self, msg: Msg, w: &Page, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Picked(Event::Choose(id)) => {
                let step = w.menu.choose(&catalog_now(cx), &id, false);
                self.step(cx, step);
            }
            // The `s` key: this session only.
            Msg::Picked(Event::Secondary(id)) => {
                let step = w.menu.choose(&catalog_now(cx), &id, true);
                self.step(cx, step);
            }
            Msg::Picked(Event::Cancel) => back(cx),
            // These lists have no toggles, order, tabs or hot keys.
            Msg::Picked(_) => {}
            Msg::Next(Next::Close) => close(cx),
            Msg::Next(Next::GiveUp(why)) => {
                say(cx, Intent::GiveUp(why));
                self.leave(cx);
            }
            Msg::Next(Next::Replace(menu)) => {
                drop(Navigator::of(cx).push_replacement::<()>(route(*menu)));
            }
        }
        Rebuild::No
    }

    fn build(&self, w: &Page, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let catalog = catalog_of(cx);
        let page = w.menu.page(&catalog);
        let words = strings::of(cx);
        let picker: AnyWidget =
            Picker::new(Rc::new(list::of(&page, words)), cx.map(Msg::Picked)).into();
        // Another page of the family (the quick menu a refresh made the model list of) is a new
        // picker with its own start; a refresh of the same page keeps cursor and scroll.
        picker.with_key(shape(&page.heading))
    }
}

/// Which page of the family a heading is.
fn shape(heading: &Heading) -> u32 {
    match heading {
        Heading::Loading => 1,
        Heading::Quick => 2,
        Heading::All => 3,
        Heading::Reasoning { .. } => 4,
        Heading::Advanced => 5,
        Heading::PlanScope { .. } => 6,
        Heading::Reserve => 7,
    }
}
