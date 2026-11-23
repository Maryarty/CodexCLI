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

use codex_tui_core::features::FeatureStates;
use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::ScreenStates;
use codex_tui_core::screens::select::plugins as plugin_model;
use codex_tui_core::screens::select::models::Heading;
use codex_tui_core::screens::select::review::Menu as ReviewMenu;
use codex_tui_core::screens::select::{Intent, Menu, Page};

use super::app_link::AppLinkView;
use super::lists;
use super::plugins;
use super::prompt;
use super::review;
use super::strings;
use crate::app::view::Hs;
use crate::keymap::keymap;
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
    /// The intents the model had applied when the dialog first looked.
    applied: Option<u64>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Picked(Event),
    /// What the `/review` instructions prompt answered.
    Typed(prompt::Answer),
    /// What the marketplace source prompt of `/plugins` answered.
    Source(prompt::Answer),
    /// What the app link dialog asks of the app it shows.
    Link(Intent),
}

impl StatefulWidget for MenuDialog {
    type State = MenuDialogState;

    fn create_state(&self) -> MenuDialogState {
        MenuDialogState {
            shown: false,
            applied: None,
        }
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
        let states = watch::<ScreenStates, _>(cx);
        let present = states.select.menu().is_some();
        // The opener emits the intent and shows the dialog in one handler: the dialog's first
        // build comes before the model has applied the intent. An intent applied since that
        // left no menu (a note for the history instead: `/model` blocked or with nothing to
        // pick) is not one the dialog waits for: it closes instead of trapping the keys over
        // an empty sheet.
        let applied = states.select.applied();
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
            Msg::Typed(prompt::Answer::Back) | Msg::Source(prompt::Answer::Back) => Event::Cancel,
            Msg::Source(prompt::Answer::Submit(text)) => {
                let intent = plugin_model::Intent::Source(text);
                cx.emit(&hs.screen, ScreenIntent::Select(Intent::Plugins(intent)));
                return Rebuild::Yes;
            }
        };
        let intent = match event {
            Event::Choose(id) => Intent::Choose(id),
            Event::Secondary(id) => Intent::Secondary(id),
            Event::Toggle(id) => Intent::Toggle(id),
            Event::Reorder(id, down) => Intent::Reorder(id, down),
            Event::Confirm => Intent::Confirm,
            // Only the plugin browser asks for these (`Picker::extras`).
            Event::Tab(id) => Intent::Plugins(plugin_model::Intent::Tab(id)),
            Event::Hot(plugins::REMOVE_KEY) => Intent::Plugins(plugin_model::Intent::Remove),
            Event::Hot(plugins::UPGRADE_KEY) => Intent::Plugins(plugin_model::Intent::Upgrade),
            Event::Hot(_) => return Rebuild::No,
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
        let states = watch::<ScreenStates, _>(cx);
        // A feature's menu reads its rows from the feature's state: watched, so a change of the
        // state while the menu is open rebuilds its rows under the same picker (same depth
        // key: cursor, tab and query stay).
        let features = watch::<FeatureStates, _>(cx);
        let Some(head) = head(&states) else {
            return AnyWidget::default();
        };
        // The plugin browser reads its own part of the model.
        if matches!(head.menu, Menu::Plugins) {
            return Self::plugins(&states, head.depth, cx);
        }
        // The instructions of `/review` are a text prompt, not a list.
        if matches!(&head.menu, Menu::Review(menu) if **menu == ReviewMenu::Instructions) {
            let prompt: AnyWidget =
                review::instructions(cx.map(Msg::Typed), review::strings::of(cx)).into();
            return prompt.with_key(u32::try_from(head.depth).unwrap_or(0));
        }
        // The page of an app picked from `/apps`: a dialog of its own, not a list.
        if matches!(head.menu, Menu::Apps)
            && let Some(link) = &states.select.apps.link
        {
            let dialog: AnyWidget = AppLinkView {
                link: link.clone(),
                on_intent: cx.map(Msg::Link),
            }
            .into();
            return dialog.with_key(u32::try_from(head.depth).unwrap_or(0));
        }
        let list = match &head.menu {
            Menu::Review(menu) => {
                review::list(menu, head.cursor.as_deref(), review::strings::of(cx))
            }
            menu => lists::of(menu, head.page.as_ref(), &features, strings::of(cx)),
        };
        let Some(list) = list else {
            return AnyWidget::default();
        };
        let picker = Picker::new(Rc::new(list), cx.map(Msg::Picked));
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

impl MenuDialogState {
    /// `/plugins`: the page the plugin model is at. A refresh of the same page keeps its picker
    /// (cursor, query, tab); another page is a new one.
    fn plugins(states: &ScreenStates, depth: usize, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let select = &states.select;
        let words = plugins::strings::of(cx);
        let keys = keymap(cx);
        let auth = select.plugin_auth();
        let Some(page) = plugins::page(&select.plugins, auth.as_ref(), &keys, words) else {
            return AnyWidget::default();
        };
        let key = u32::try_from(depth)
            .unwrap_or(0)
            .saturating_mul(32)
            .saturating_add(plugins::shape(select.plugins.menu()));
        let on_event = cx.map(Msg::Picked);
        let widget: AnyWidget = match page {
            plugins::Page::Add => plugins::add_prompt(cx.map(Msg::Source), words).into(),
            plugins::Page::Browser(list, extras) => {
                Picker::new(Rc::new(list), on_event).extras(extras).into()
            }
            plugins::Page::List(list) => Picker::new(Rc::new(list), on_event).into(),
            // A confirmation floats like the others.
            plugins::Page::Confirm(list) => Padding::new(
                EdgeInsets::symmetric(2, 0),
                Dialog::bare(Picker::new(Rc::new(list), on_event)).max_width(72),
            )
            .into(),
        };
        widget.with_key(key)
    }
}
