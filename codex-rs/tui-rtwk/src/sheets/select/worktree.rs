//! The sheet of the worktree pickers (oracle `chatwidget/worktree_picker.rs`): the checkout choice
//! of `/new` and `/fork` or `/worktree`'s three rows, then the browser of the repository's
//! worktrees (its loading row, the list, a worktree's actions, the delete confirmation), as the
//! routes of a nested `Navigator` (ADR-013 O1, the shape of the `/plugins` pilot and of
//! `review_sheet.rs`). The oracle dismisses every view it leaves (`dismiss_on_select`) and shows the
//! next one, so each page *replaces* the one before it and Esc closes the sheet, whichever page it
//! is on; the loading page that is left gives its list up (`Verb::Abandon`), so an answer that
//! comes late, or fails, is dropped without a word.
//!
//! The list is the host's: the browse row asks for it (`Verb::List`) and the loading page reads
//! the answer from the model's fact (`Worktree::listing`), replacing itself with the browser or,
//! on a failure the model has said in the history, closing. Which row leads where is the model's
//! pure `Menu::choose`; the pages are the navigator's. The delete confirmation floats centred as
//! the oracle's `confirmation()` does; every other page sits at the bottom edge. In Flutter this
//! is the nested-navigator recipe for a flow inside `showModalBottomSheet`, each page a
//! `ConsumerStatefulWidget` and a replacement a `pushReplacement`.

mod list;
pub mod strings;

use std::rc::Rc;

use rtwk::layout::{Align, Alignment, EdgeInsets, Padding};
use rtwk::navigation::{Navigator, PageRoute};
use rtwk::overlay::{Dialog, DialogOptions, close_dialog};
use rtwk::provider::{read, select};
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::screens::select::Intent;
use codex_tui_core::screens::select::worktree::{Chosen, Entry, Menu, Verb};
use codex_tui_core::screens::{ScreenIntent, ScreenStates};

use crate::app::view::Hs;
use crate::widgets::picker::{Event, Picker};

/// The sheet that opens on `home`, the checkout choice or `/worktree`'s page.
pub fn sheet(home: Menu) -> AnyWidget {
    Sheet { home }.into()
}

/// The options the sheet is shown with: the bottom pane, built under the providers of the page
/// that asks.
pub fn options() -> DialogOptions {
    DialogOptions {
        use_root_navigator: false,
        ..crate::sheets::sheet_options()
    }
}

/// The sheet: a navigator as tall as the sheet's cap, closed when the model's scope moves on.
#[derive(Clone, Debug, PartialEq)]
struct Sheet {
    home: Menu,
}

struct SheetState {
    /// The scope of the menus the sheet was opened for.
    epoch: Option<u64>,
}

impl StatefulWidget for Sheet {
    type State = SheetState;

    fn create_state(&self) -> SheetState {
        SheetState { epoch: None }
    }
}

impl_widget!(stateful eq Sheet);

impl State for SheetState {
    type Widget = Sheet;
    type Msg = ();

    // A change of the session's scope (the account, the directory) leaves the sheet's requests
    // meaningless (`worktree_request_is_current`): it closes.
    fn did_change_dependencies(&mut self, _: &Sheet, cx: &mut Cx<'_, Self>) {
        let epoch = select::<ScreenStates, u64, _>(cx, |states| states.select.worktree.epoch());
        match self.epoch {
            None => self.epoch = Some(epoch),
            Some(opened) if opened != epoch => close_dialog::<_, ()>(cx, None),
            Some(_) => {}
        }
    }

    fn build(&self, w: &Sheet, _: &mut Cx<'_, Self>) -> AnyWidget {
        Navigator::new()
            .home(placed(w.home.clone()))
            .back_key(None)
            .into()
    }
}

/// A page where the oracle shows its view: the confirmation centred, the rest at the bottom edge.
fn placed(menu: Menu) -> AnyWidget {
    let alignment = if matches!(menu, Menu::Confirm { .. }) {
        Alignment::CENTER
    } else {
        Alignment::BOTTOM_CENTER
    };
    Align::new(alignment, Page { menu }).into()
}

fn route(menu: Menu) -> PageRoute<()> {
    PageRoute::new(move |_| placed(menu.clone()))
}

/// One page of the sheet.
#[derive(Clone, Debug, PartialEq)]
struct Page {
    menu: Menu,
}

struct PageState {
    /// The directory the session works in: a worktree it runs inside cannot be deleted.
    cwd: String,
    /// The clock the browser's ages are counted from (a minute's resolution: the list is not
    /// rebuilt every second).
    now: i64,
    /// The loading page has given way to the browser or closed.
    left: bool,
}

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Picked(Event),
    /// The host's list arrived (or why it did not): the loading page is done.
    Listed(Result<Vec<Entry>, String>),
}

impl StatefulWidget for Page {
    type State = PageState;

    fn create_state(&self) -> PageState {
        PageState {
            cwd: String::new(),
            now: 0,
            left: false,
        }
    }
}

impl_widget!(stateful eq Page);

/// Says a verb to the model.
fn say(cx: &mut Cx<'_, PageState>, verb: Verb) {
    let hs = read::<Hs, _>(cx);
    cx.emit(&hs.screen, ScreenIntent::Select(Intent::Worktree(verb)));
}

/// The page gives way to the next one: the oracle dismissed its view to show it.
fn replace(cx: &mut Cx<'_, PageState>, menu: Menu) {
    drop(Navigator::of(cx).push_replacement::<()>(route(menu)));
}

fn close(cx: &mut Cx<'_, PageState>) {
    close_dialog::<_, ()>(cx, None);
}

impl State for PageState {
    type Widget = Page;
    type Msg = Msg;

    // The page reads the model's facts it draws: where the session works and the clock; the
    // loading page also waits for the host's list and tells itself when it arrives (between
    // builds: what a change of the model asks of the navigator is never done inside one).
    fn did_change_dependencies(&mut self, w: &Page, cx: &mut Cx<'_, Self>) {
        let (cwd, minute) = select::<ScreenStates, (String, i64), _>(cx, |states| {
            (
                states.select.worktree.cwd().to_owned(),
                states.status.now / 60,
            )
        });
        self.cwd = cwd;
        self.now = minute * 60;
        if w.menu != Menu::Loading || self.left {
            return;
        }
        let listed = select::<ScreenStates, Option<Result<Vec<Entry>, String>>, _>(cx, |states| {
            states.select.worktree.listing().cloned()
        });
        if let Some(listed) = listed {
            self.left = true;
            let go = cx.send(Msg::Listed(listed));
            cx.emit(&go, ());
        }
    }

    fn update(&mut self, msg: Msg, w: &Page, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Listed(Ok(entries)) => replace(cx, Menu::Browser(entries)),
            // The model said why in the history; there is nothing to show.
            Msg::Listed(Err(_)) => close(cx),
            Msg::Picked(Event::Choose(id)) => match w.menu.choose(&id) {
                Some(Chosen::Pick(pick)) => {
                    say(cx, Verb::Pick(pick));
                    close(cx);
                }
                Some(Chosen::Browse) => {
                    say(cx, Verb::List);
                    replace(cx, Menu::Loading);
                }
                Some(Chosen::Actions(entry)) => {
                    let can_delete = entry.can_delete(&self.cwd);
                    replace(cx, Menu::Actions { entry, can_delete });
                }
                Some(Chosen::Confirm { root }) => replace(cx, Menu::Confirm { root }),
                Some(Chosen::Close) => close(cx),
                None => {}
            },
            // Esc closes the sheet from every page; a list still asked for is given up.
            Msg::Picked(Event::Cancel) => {
                if w.menu == Menu::Loading {
                    say(cx, Verb::Abandon);
                }
                close(cx);
            }
            // The worktree lists have no secondary action, toggles, order or tabs.
            Msg::Picked(_) => {}
        }
        Rebuild::No
    }

    fn build(&self, w: &Page, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let list = list::of(&w.menu, self.now, strings::of(cx));
        let picker = Picker::new(Rc::new(list), cx.map(Msg::Picked));
        if matches!(w.menu, Menu::Confirm { .. }) {
            // The confirmation floats: 72 cells wide at most and two cells in from the screen
            // (`dialog_area` of the oracle's view stack).
            return Padding::new(
                EdgeInsets::symmetric(2, 0),
                Dialog::bare(picker).max_width(72),
            )
            .into();
        }
        picker.into()
    }
}
