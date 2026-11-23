//! The sheet of the select screen: the seeded adapter between the model and
//! the `Picker` brick (`widgets/picker`). It reads the menu the model holds
//! through a provider projection, hands the picker its rows as props and turns
//! what the user did into a `ScreenIntent::Select` (ADR-003: the sheet keeps
//! no copy of the menu; it closes when the model drops it). In Flutter it is
//! the `showModalBottomSheet` builder around a presentational widget.
//! Oracle: `bottom_pane/list_selection_view.rs`.

use std::collections::BTreeMap;
use std::rc::Rc;

use rtwk::overlay::close_dialog;
use rtwk::provider::{read, select};
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::ScreenStates;
use codex_tui_core::screens::select::apps::Link;
use codex_tui_core::screens::select::models::Heading;
use codex_tui_core::screens::select::review::Menu as ReviewMenu;
use codex_tui_core::screens::select::{Intent, Menu, Page};

use super::app_link::AppLinkView;
use super::lists;
use super::prompt;
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
}

fn head(states: &ScreenStates) -> Option<Head> {
    let select = &states.select;
    Some(Head {
        menu: select.menu()?.clone(),
        page: select.page(),
        depth: select.stack.len(),
    })
}

pub struct MenuDialogState {
    shown: bool,
    /// The intents the model had applied when the dialog first looked.
    applied: Option<u64>,
    /// How deep the stack was when the dialog last looked.
    depth: usize,
    /// The row picked last: the row that opened the menu a push puts on top.
    chosen: Option<String>,
    /// The row that opened the menu above the one at each depth: Esc shows that menu again with
    /// the cursor on it (the oracle's parent view keeps its highlight). The cursor of a menu is
    /// the view's, so the sheet remembers it, not the model.
    kept: BTreeMap<usize, String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Picked(Event),
    /// What the `/review` instructions prompt answered.
    Typed(prompt::Answer),
    /// What the app link dialog asks of the app it shows.
    Link(Intent),
}

impl StatefulWidget for MenuDialog {
    type State = MenuDialogState;

    fn create_state(&self) -> MenuDialogState {
        MenuDialogState {
            shown: false,
            applied: None,
            depth: 0,
            chosen: None,
            kept: BTreeMap::new(),
        }
    }
}

impl_widget!(stateful eq MenuDialog);

impl State for MenuDialogState {
    type Widget = MenuDialog;
    type Msg = Msg;

    // Selects what the dialog's life depends on: whether the model has a menu, how many
    // intents it has applied and how deep the stack is. `build` selects the head the same way,
    // so the registration is kept whichever of the two ran last.
    fn did_change_dependencies(&mut self, _: &MenuDialog, cx: &mut Cx<'_, Self>) {
        let (present, applied, depth) =
            select::<ScreenStates, (bool, u64, usize), _>(cx, |states| {
                (
                    states.select.menu().is_some(),
                    states.select.applied(),
                    states.select.stack.len(),
                )
            });
        if depth > self.depth {
            // A menu was pushed over the one at the old depth: the row picked last opened it.
            if let Some(id) = self.chosen.take()
                && self.depth > 0
            {
                self.kept.insert(self.depth - 1, id);
            }
        } else if depth < self.depth {
            self.chosen = None;
            self.kept.retain(|at, _| *at < depth);
        }
        self.depth = depth;
        // The opener emits the intent and shows the dialog in one handler: the dialog's first
        // build comes before the model has applied the intent. An intent applied since that
        // left no menu (a note for the history instead: `/model` blocked or with nothing to
        // pick) is not one the dialog waits for: it closes instead of trapping the keys over
        // an empty sheet.
        let before = *self.applied.get_or_insert(applied);
        if !present && (self.shown || applied > before) {
            close_dialog::<_, ()>(cx, None);
        }
        self.shown = present;
    }

    fn update(&mut self, msg: Msg, _: &MenuDialog, cx: &mut Cx<'_, Self>) -> Rebuild {
        let hs = read::<Hs, _>(cx);
        let event = match msg {
            Msg::Picked(event) => event,
            Msg::Link(intent) => {
                cx.emit(&hs.screen, ScreenIntent::Select(intent));
                return Rebuild::Yes;
            }
            Msg::Typed(prompt::Answer::Submit(text)) => {
                cx.emit(&hs.screen, ScreenIntent::Select(Intent::Instructions(text)));
                return Rebuild::Yes;
            }
            Msg::Typed(prompt::Answer::Back) => Event::Cancel,
        };
        let intent = match event {
            Event::Choose(id) => {
                self.chosen = Some(id.clone());
                Intent::Choose(id)
            }
            Event::Secondary(id) => Intent::Secondary(id),
            // The typed menus have no toggles and no order to move (the generic lists, which
            // have, are `generic::ListSheet`), and only the plugin browser asks for tabs and
            // hot keys (`Picker::extras`).
            Event::Toggle(_)
            | Event::Reorder(..)
            | Event::Confirm
            | Event::Tab(_)
            | Event::Hot(_) => return Rebuild::No,
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
        let (head, link) = select::<ScreenStates, (Option<Head>, Option<Link>), _>(cx, |states| {
            (head(states), states.select.apps.link.clone())
        });
        let Some(head) = head else {
            return AnyWidget::default();
        };
        let words = strings::of(cx);
        let cursor = head
            .depth
            .checked_sub(1)
            .and_then(|at| self.kept.get(&at))
            .map(String::as_str);
        // The instructions of `/review` are a text prompt, not a list.
        if matches!(&head.menu, Menu::Review(menu) if **menu == ReviewMenu::Instructions) {
            let prompt: AnyWidget =
                review::instructions(cx.map(Msg::Typed), review::strings::of(cx)).into();
            return prompt.with_key(u32::try_from(head.depth).unwrap_or(0));
        }
        // The page of an app picked from `/apps`: a dialog of its own, not a list.
        if matches!(head.menu, Menu::Apps)
            && let Some(link) = link
        {
            let dialog: AnyWidget = AppLinkView {
                link,
                on_intent: cx.map(Msg::Link),
            }
            .into();
            return dialog.with_key(u32::try_from(head.depth).unwrap_or(0));
        }
        let list = match &head.menu {
            Menu::Review(menu) => review::list(menu, cursor, review::strings::of(cx)),
            menu => {
                let mut page = head.page.clone();
                // The row that opened the menu above this one: the cursor starts on it.
                if let (Some(Page::Model(page)), Some(id)) = (&mut page, cursor)
                    && let Some(at) = page.rows.iter().position(|row| row.id == id)
                {
                    page.initial = Some(at);
                }
                lists::of(menu, page.as_ref(), words)
            }
        };
        let Some(list) = list else {
            return AnyWidget::default();
        };
        let picker = Picker::new(Rc::new(list), cx.map(Msg::Picked));
        let picker: AnyWidget = picker.into();
        // Another page of the family (the quick menu a refresh made of the model list) is a new
        // picker with its own start; a refresh of the same page keeps cursor and scroll.
        let shape = match &head.page {
            Some(Page::Model(page)) => shape(&page.heading),
            _ => 0,
        };
        picker.with_key(
            u32::try_from(head.depth)
                .unwrap_or(0)
                .saturating_mul(8)
                .saturating_add(shape),
        )
    }
}

/// Which page of the `/model` family a heading is; the reasoning menus of two models are the
/// same page (they sit at different depths).
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
