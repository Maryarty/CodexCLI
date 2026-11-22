//! The sheet of the select screen: the seeded adapter between the model and
//! the `Picker` brick (`widgets/picker`). It reads the menu the model holds
//! through a provider projection, hands the picker its rows as props and turns
//! what the user did into a `ScreenIntent::Select` (ADR-003: the sheet keeps
//! no copy of the menu; it closes when the model drops it). In Flutter it is
//! the `showModalBottomSheet` builder around a presentational widget.
//! Oracle: `bottom_pane/list_selection_view.rs`.

use std::rc::Rc;

use rtwk::layout::{EdgeInsets, Padding};
use rtwk::overlay::{Dialog, close_dialog};
use rtwk::provider::{read, select};
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::ScreenStates;
use codex_tui_core::screens::select::{Intent, Menu};

use super::lists;
use super::strings;
use crate::app::view::Hs;
use crate::widgets::picker::{Event, Picker};

#[derive(Clone, Debug, PartialEq)]
pub struct MenuDialog;

pub struct MenuDialogState {
    shown: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Picked(Event),
}

impl StatefulWidget for MenuDialog {
    type State = MenuDialogState;

    fn create_state(&self) -> MenuDialogState {
        MenuDialogState { shown: false }
    }
}

impl_widget!(stateful eq MenuDialog);

impl State for MenuDialogState {
    type Widget = MenuDialog;
    type Msg = Msg;

    fn did_change_dependencies(&mut self, _: &MenuDialog, cx: &mut Cx<'_, Self>) {
        let present = select::<ScreenStates, bool, _>(cx, |states| states.select.menu.is_some());
        if !present && self.shown {
            close_dialog::<_, ()>(cx, None);
        }
        self.shown = present;
    }

    fn update(&mut self, msg: Msg, _: &MenuDialog, cx: &mut Cx<'_, Self>) -> Rebuild {
        let Msg::Picked(event) = msg;
        let hs = read::<Hs, _>(cx);
        let is_list = select::<ScreenStates, bool, _>(cx, |states| {
            matches!(states.select.menu, Some(Menu::List(_)))
        });
        let intent = match event {
            Event::Choose(id) if is_list => Intent::Choose(id),
            Event::Choose(id) => match id.parse() {
                Ok(index) => Intent::Pick(index),
                Err(_) => return Rebuild::No,
            },
            Event::Secondary(id) => Intent::Secondary(id),
            Event::Toggle(id) => Intent::Toggle(id),
            Event::Reorder(id, down) => Intent::Reorder(id, down),
            Event::Confirm => Intent::Confirm,
            Event::Cancel => {
                cx.emit(&hs.screen, ScreenIntent::Select(Intent::Close));
                close_dialog::<_, ()>(cx, None);
                return Rebuild::No;
            }
        };
        cx.emit(&hs.screen, ScreenIntent::Select(intent));
        // The model applies the intent at the next flush; the rebuild hands the
        // picker the rows it then holds.
        Rebuild::Yes
    }

    fn build(&self, _: &MenuDialog, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let Some(menu) =
            select::<ScreenStates, Option<Menu>, _>(cx, |states| states.select.menu.clone())
        else {
            return AnyWidget::default();
        };
        let list = lists::of(&menu, strings::of(cx));
        let picker = Picker {
            list: Rc::new(list),
            on_event: cx.map(Msg::Picked),
        };
        match menu {
            // A confirmation floats: 72 cells wide at most and two cells in
            // from the screen (`dialog_area` of the oracle's view stack).
            Menu::Confirm(_) => Padding::new(
                EdgeInsets::symmetric(2, 0),
                Dialog::bare(picker).max_width(72),
            )
            .into(),
            _ => picker.into(),
        }
    }
}
