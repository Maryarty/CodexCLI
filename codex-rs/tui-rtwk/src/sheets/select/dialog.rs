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
use rtwk::provider::{read, watch};
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::ScreenStates;
use codex_tui_core::screens::select::review::Menu as ReviewMenu;
use codex_tui_core::screens::select::{Intent, Menu, Page};

use super::lists;
use super::review;
use super::strings;
use crate::app::view::Hs;
use crate::widgets::picker::{Event, Picker};

#[derive(Clone, Debug, PartialEq)]
pub struct MenuDialog;

/// What the dialog reads of the model: the menu on top of the stack, its rows
/// when it is one of the typed families and how deep it sits (a menu pushed
/// above another is a new picker, a refresh of the head is the same one).
#[derive(Clone, Debug, PartialEq)]
struct Head {
    menu: Menu,
    page: Option<Page>,
    depth: usize,
    /// The row of the menu that opened the one above this one: Esc shows this menu with the
    /// cursor on it.
    cursor: Option<String>,
}

fn head(states: &ScreenStates) -> Option<Head> {
    let select = &states.select;
    Some(Head {
        menu: select.menu()?.clone(),
        page: select.page(),
        depth: select.stack.len(),
        cursor: select.cursor_row().map(str::to_owned),
    })
}

pub struct MenuDialogState {
    shown: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Picked(Event),
    /// What the `/review` instructions prompt answered.
    Typed(review::Answer),
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

    // Watches the whole model, not a selection of it: a dependent that is
    // dirty when the provider notifies it (the `Rebuild::Yes` of `update`
    // below, a pick that empties the stack) is not told of a change it only
    // selects from, and a dialog that then missed the menu going stayed open
    // and empty over the chat. Watching tells it of every change; `build`
    // watches too, so the registration is kept whichever of the two ran last.
    fn did_change_dependencies(&mut self, _: &MenuDialog, cx: &mut Cx<'_, Self>) {
        let present = watch::<ScreenStates, _>(cx).select.menu().is_some();
        if !present && self.shown {
            close_dialog::<_, ()>(cx, None);
        }
        self.shown = present;
    }

    fn update(&mut self, msg: Msg, _: &MenuDialog, cx: &mut Cx<'_, Self>) -> Rebuild {
        let hs = read::<Hs, _>(cx);
        let event = match msg {
            Msg::Picked(event) => event,
            Msg::Typed(review::Answer::Submit(text)) => {
                cx.emit(&hs.screen, ScreenIntent::Select(Intent::Instructions(text)));
                return Rebuild::Yes;
            }
            Msg::Typed(review::Answer::Back) => Event::Cancel,
        };
        let intent = match event {
            Event::Choose(id) => Intent::Choose(id),
            Event::Secondary(id) => Intent::Secondary(id),
            Event::Toggle(id) => Intent::Toggle(id),
            Event::Reorder(id, down) => Intent::Reorder(id, down),
            Event::Confirm => Intent::Confirm,
            // Esc leaves the menu on top; one below it (the quick menu under
            // the reasoning menu) comes back, and the dialog closes when the
            // model has none left.
            Event::Cancel => {
                cx.emit(&hs.screen, ScreenIntent::Select(Intent::Close));
                return Rebuild::Yes;
            }
        };
        cx.emit(&hs.screen, ScreenIntent::Select(intent));
        // The model applies the intent at the next flush; the rebuild hands the
        // picker the rows it then holds.
        Rebuild::Yes
    }

    fn build(&self, _: &MenuDialog, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let Some(head) = head(&watch::<ScreenStates, _>(cx)) else {
            return AnyWidget::default();
        };
        // The instructions of `/review` are a text prompt, not a list.
        if matches!(&head.menu, Menu::Review(menu) if **menu == ReviewMenu::Instructions) {
            let prompt: AnyWidget = review::Instructions {
                on_answer: cx.map(Msg::Typed),
            }
            .into();
            return prompt.with_key(u32::try_from(head.depth).unwrap_or(0));
        }
        let list = match &head.menu {
            Menu::Review(menu) => {
                review::list(menu, head.cursor.as_deref(), review::strings::of(cx))
            }
            menu => lists::of(menu, head.page.as_ref(), strings::of(cx)),
        };
        let Some(list) = list else {
            return AnyWidget::default();
        };
        let picker = Picker {
            list: Rc::new(list),
            on_event: cx.map(Msg::Picked),
        };
        let picker: AnyWidget = match head.menu {
            // A confirmation floats: 72 cells wide at most and two cells in
            // from the screen (`dialog_area` of the oracle's view stack).
            Menu::Confirm(_) => Padding::new(
                EdgeInsets::symmetric(2, 0),
                Dialog::bare(picker).max_width(72),
            )
            .into(),
            _ => picker.into(),
        };
        picker.with_key(u32::try_from(head.depth).unwrap_or(0))
    }
}
