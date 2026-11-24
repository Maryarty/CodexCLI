//! The `/daemon` menu and its two confirmations (ADR-013; oracle `app/daemon_menu.rs`): sheets over
//! the chat that the presenter opens for a `Wish::Menu` of the daemon feature. In Flutter each
//! page is a `showModalBottomSheet` over a list of options; the rows and their enabled flags are
//! read from the feature's model (`State::page`), the user's decision goes back as a `Verb`, and a
//! row that leads to a confirmation closes the sheet while the feature wishes for the next one.

use std::rc::Rc;

use rtwk::layout::{Alignment, EdgeInsets, Padding};
use rtwk::overlay::{Dialog, DialogOptions, close_dialog};
use rtwk::provider::select;
use rtwk::{AnyWidget, Cx, Handler, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::features::FeatureStates;
use codex_tui_core::features::daemon::{Cli, Menu, Page, Row, Verb};

use super::strings;
use crate::features::verb;
use crate::widgets::picker::{Event, Picker};

/// One page of the menu, as the feature asked for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DaemonSheet {
    pub menu: Menu,
}

pub struct DaemonSheetState;

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Picked(Event),
}

impl StatefulWidget for DaemonSheet {
    type State = DaemonSheetState;

    fn create_state(&self) -> DaemonSheetState {
        DaemonSheetState
    }
}

impl_widget!(stateful eq DaemonSheet);

impl State for DaemonSheetState {
    type Widget = DaemonSheet;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, w: &DaemonSheet, cx: &mut Cx<'_, Self>) -> Rebuild {
        let Msg::Picked(event) = msg;
        match event {
            Event::Choose(id) => {
                let Some(said) = strings::verb_of(w.menu, &id) else {
                    return Rebuild::No;
                };
                if let Some(said) = said {
                    verb(cx, said);
                }
                close_dialog::<_, ()>(cx, None);
            }
            Event::Cancel => close_dialog::<_, ()>(cx, None),
            _ => {}
        }
        Rebuild::No
    }

    fn build(&self, w: &DaemonSheet, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let (page, cli) = select::<FeatureStates, (Page, Option<Cli>), _>(cx, |features| {
            (features.daemon.page(), features.daemon.facts.cli.clone())
        });
        let on: Handler<Event> = cx.map(Msg::Picked);
        let picker = Picker::new(Rc::new(strings::menu(w.menu, &page, cli.as_ref())), on);
        match w.menu {
            Menu::Main => picker.into(),
            // A confirmation floats: 72 cells wide at most and two cells in from the screen
            // (`ViewPresentation::Centered`).
            Menu::ConfirmRestart | Menu::ConfirmUpdate(_) => Padding::new(
                EdgeInsets::symmetric(2, 0),
                Dialog::bare(picker).max_width(72),
            )
            .into(),
        }
    }
}

/// How a page is shown: the menu at the bottom, a confirmation centered.
pub fn options(menu: Menu) -> DialogOptions {
    match menu {
        Menu::Main => crate::sheets::sheet_options(),
        Menu::ConfirmRestart | Menu::ConfirmUpdate(_) => DialogOptions {
            alignment: Alignment::CENTER,
            ..crate::sheets::sheet_options()
        },
    }
}

/// The verb a row of the main page says.
pub fn row_verb(row: Row) -> Verb {
    match row {
        Row::Update(source) => Verb::ConfirmUpdate(source),
        Row::Restart => Verb::ConfirmRestart,
    }
}
