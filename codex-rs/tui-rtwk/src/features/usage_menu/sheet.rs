//! The sheet of `/usage` (oracle `chatwidget/usage.rs`): the usage menu, and behind its `Redeem
//! reset` row the reset pages and the confirmation (ADR-013). In Flutter it is a
//! `showModalBottomSheet` over a nested `Navigator` whose home is the menu; the credits page
//! replaces it (the oracle dismisses the menu and shows the loading popup), and the confirmation
//! is a centred `showDialog` over the credits page, so `No, go back` and Esc leave the credits
//! under it as they were, cursor included.
//!
//! The sheet keeps nothing but what is drawn (ADR-013): it reads the count, the list's state and
//! the redemption's step from the usage screen's reset state and says verbs to it (`Menu`,
//! `Resets`, `Redeem`, `Retry`). One confirmation at a time: while one is open the picker's
//! rows do not open another (the oracle's confirmation gate).

use std::rc::Rc;

use rtwk::layout::{Alignment, EdgeInsets, Padding};
use rtwk::navigation::{Navigator, PageRoute};
use rtwk::overlay::{Dialog, DialogOptions, close_dialog, show_dialog};
use rtwk::provider::{read, select};
use rtwk::{AnyWidget, Cx, Handler, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::keymap::actions::{LIST_ACCEPT, LIST_CANCEL};
use codex_tui_core::screens::cells::ViewerClock;
use codex_tui_core::screens::usage::Intent;
use codex_tui_core::screens::usage::reset::{CreditOption, Listing, MenuEntry, Redemption};
use codex_tui_core::screens::{ScreenIntent, ScreenStates};

use super::strings::{self, Chosen};
use crate::app::view::Hs;
use crate::keymap::{keymap, label};
use crate::pages::usage;
use crate::widgets::picker::{Event, Picker};
use crate::widgets::sheet::bottom;

/// The sheet: a navigator whose home is the menu, each page at the bottom edge.
pub fn sheet() -> AnyWidget {
    Navigator::new()
        .home(bottom(Page { kind: Kind::Menu }))
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

/// The confirmation floats over the sheet, centred (the oracle's `ViewPresentation::Centered`).
fn confirm_options() -> DialogOptions {
    DialogOptions {
        use_root_navigator: false,
        alignment: Alignment::CENTER,
        ..crate::sheets::sheet_options()
    }
}

/// Says a verb to the usage screen.
pub fn say<S: 'static>(cx: &mut Cx<'_, S>, intent: Intent) {
    let hs = read::<Hs, _>(cx);
    cx.emit(&hs.screen, ScreenIntent::Usage(intent));
}

/// `enter <word> · esc back` in the keys the keymap names (`usage_hint_line`); an unbound action
/// is left out.
fn hint_of<S: 'static>(cx: &mut Cx<'_, S>) -> impl Fn(&str) -> String + use<S> {
    let keys = keymap(cx);
    move |word| {
        [(LIST_ACCEPT, word), (LIST_CANCEL, "back")]
            .into_iter()
            .filter_map(|(id, word)| label(&keys, id).map(|key| format!("{key} {word}")))
            .collect::<Vec<_>>()
            .join(" · ")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Menu,
    Resets,
}

/// One page of the sheet.
#[derive(Clone, Debug, PartialEq)]
struct Page {
    kind: Kind,
}

/// What the credits page reads of the reset state.
#[derive(Clone, Debug, PartialEq)]
struct View {
    listing: Listing,
    redemption: Option<Redemption>,
    count: i64,
    options: Vec<CreditOption>,
    clock: ViewerClock,
    zone_secs: i32,
}

fn view_of(states: &ScreenStates) -> View {
    let reset = &states.usage.reset;
    View {
        listing: reset.listing,
        redemption: reset.redemption.clone(),
        count: reset.credits.as_ref().map_or(0, |c| c.available_count),
        options: reset.options(),
        clock: states.cells.clock,
        zone_secs: states.status.utc_offset_secs,
    }
}

struct PageState {
    /// A confirmation is open over this page.
    confirming: bool,
}

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Picked(Event),
    /// The confirmation was left: the rows may open another.
    Left(()),
}

impl StatefulWidget for Page {
    type State = PageState;

    fn create_state(&self) -> PageState {
        PageState { confirming: false }
    }
}

impl_widget!(stateful eq Page);

fn credits_route() -> PageRoute<()> {
    PageRoute::new(|_| bottom(Page { kind: Kind::Resets }))
}

impl PageState {
    fn choose(&mut self, w: &Page, id: &str, cx: &mut Cx<'_, Self>) {
        match (w.kind, id) {
            (Kind::Menu, strings::ANALYTICS) => {
                // The analytics page opens over the chat the sheet leaves.
                let root = Navigator::root_of(cx);
                close_dialog::<_, ()>(cx, None);
                drop(root.push::<()>(usage::route()));
            }
            (Kind::Menu, strings::REDEEM) => {
                say(cx, Intent::Resets);
                drop(Navigator::of(cx).push_replacement::<()>(credits_route()));
            }
            (Kind::Resets, strings::CANCEL | strings::CLOSE) => close_dialog::<_, ()>(cx, None),
            (Kind::Resets, strings::RELOAD) => say(cx, Intent::Resets),
            (Kind::Resets, strings::RETRY) => say(cx, Intent::Retry),
            (Kind::Resets, id) => {
                let Some(n) = id
                    .strip_prefix(strings::CREDIT)
                    .and_then(|n| n.parse::<usize>().ok())
                else {
                    return;
                };
                if self.confirming {
                    return;
                }
                let view = view_of(&read::<ScreenStates, _>(cx));
                let Some(option) = view.options.get(n) else {
                    return;
                };
                self.confirming = true;
                let chosen = strings::chosen(option, view.clock, view.zone_secs);
                let on_leave: Handler<()> = cx.map(Msg::Left);
                drop(show_dialog::<(), Self>(
                    cx,
                    move |_| {
                        Confirmation {
                            chosen: chosen.clone(),
                            on_leave: on_leave.clone(),
                        }
                        .into()
                    },
                    confirm_options(),
                ));
            }
            _ => {}
        }
    }
}

impl State for PageState {
    type Widget = Page;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, w: &Page, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Picked(Event::Choose(id)) => self.choose(w, &id, cx),
            // A redemption under way cannot be dismissed (`allow_cancel: false`).
            Msg::Picked(Event::Cancel) => {
                if !read::<ScreenStates, _>(cx).usage.reset.redeeming() {
                    close_dialog::<_, ()>(cx, None);
                }
            }
            // The menu and the credits have no secondary action, toggles, order or tabs.
            Msg::Picked(_) => {}
            Msg::Left(()) => self.confirming = false,
        }
        Rebuild::No
    }

    fn build(&self, w: &Page, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let hint = hint_of(cx);
        let list = match w.kind {
            Kind::Menu => {
                let entry = select::<ScreenStates, MenuEntry, _>(cx, |states| {
                    states
                        .usage
                        .reset
                        .menu_entry(states.status.has_chatgpt_account())
                });
                strings::menu(entry, hint("open"))
            }
            Kind::Resets => {
                let view = select::<ScreenStates, View, _>(cx, view_of);
                strings::resets(
                    view.listing,
                    view.redemption.as_ref(),
                    view.count,
                    &view.options,
                    view.clock,
                    view.zone_secs,
                    &hint,
                )
            }
        };
        Picker::new(Rc::new(list), cx.map(Msg::Picked)).into()
    }
}

/// `Use this reset?` over one credit: `Yes, use reset` redeems it, `No, go back` and Esc leave.
#[derive(Clone, Debug, PartialEq)]
struct Confirmation {
    chosen: Chosen,
    /// Told when the confirmation is left, however.
    on_leave: Handler<()>,
}

struct ConfirmationState;

impl StatefulWidget for Confirmation {
    type State = ConfirmationState;

    fn create_state(&self) -> ConfirmationState {
        ConfirmationState
    }
}

impl_widget!(stateful eq Confirmation);

#[derive(Clone, Debug, PartialEq)]
enum ConfirmMsg {
    Picked(Event),
}

impl State for ConfirmationState {
    type Widget = Confirmation;
    type Msg = ConfirmMsg;

    fn update(&mut self, msg: ConfirmMsg, w: &Confirmation, cx: &mut Cx<'_, Self>) -> Rebuild {
        let ConfirmMsg::Picked(event) = msg;
        match event {
            Event::Choose(id) if id == strings::YES => {
                say(
                    cx,
                    Intent::Redeem {
                        credit_id: w.chosen.credit_id.clone(),
                    },
                );
            }
            Event::Choose(id) if id == strings::NO => {}
            Event::Cancel => {}
            _ => return Rebuild::No,
        }
        cx.emit(&w.on_leave, ());
        close_dialog::<_, ()>(cx, None);
        Rebuild::No
    }

    fn build(&self, w: &Confirmation, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let hint = hint_of(cx);
        let list = strings::confirmation(&w.chosen, hint("confirm"));
        let picker = Picker::new(Rc::new(list), cx.map(ConfirmMsg::Picked));
        Padding::new(
            EdgeInsets::symmetric(2, 0),
            Dialog::bare(picker).max_width(72),
        )
        .into()
    }
}
