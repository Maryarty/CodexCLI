//! Two small pickers of the settings: the menu of `/skills` (`open_skills_menu`) and the prompt
//! that offers to turn a feature on for new threads (`open_feature_enable_prompt`). Each is a
//! `Picker` over the chat; the decision goes back as a verb of the feature.

use std::rc::Rc;

use rtwk::overlay::close_dialog;
use rtwk::{AnyWidget, Cx, Handler, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::features::settings::{Edit, Enable, Intent, Which};

use super::strings;
use crate::features::verb;
use crate::widgets::picker::{Event, Picker};

/// `/skills`: `List skills`, `Enable/Disable Skills`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkillsMenu;

pub struct PromptState;

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Picked(Event),
}

impl StatefulWidget for SkillsMenu {
    type State = PromptState;

    fn create_state(&self) -> PromptState {
        PromptState
    }
}

impl_widget!(stateful eq SkillsMenu);

impl State for PromptState {
    type Widget = SkillsMenu;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, _w: &SkillsMenu, cx: &mut Cx<'_, Self>) -> Rebuild {
        let Msg::Picked(event) = msg;
        match event {
            Event::Choose(id) if id == strings::SKILLS_MANAGE => {
                close_dialog::<_, ()>(cx, None);
                verb(cx, Intent::Open(Which::Skills));
            }
            // "List skills" inserts `$` into the composer (`insert_str("$")`): the page's
            // composer takes no text from a sheet yet (a Need of the task), the menu closes.
            Event::Choose(_) | Event::Cancel => close_dialog::<_, ()>(cx, None),
            _ => {}
        }
        Rebuild::No
    }

    fn build(&self, _w: &SkillsMenu, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let on: Handler<Event> = cx.map(Msg::Picked);
        Picker::new(Rc::new(strings::skills_menu()), on).into()
    }
}

/// The prompt's own state: the widget differs from the menu's, so the state is its own type.
pub struct EnableState;

impl StatefulWidget for EnableSheet {
    type State = EnableState;

    fn create_state(&self) -> EnableState {
        EnableState
    }
}

/// The sheet of an `EnablePrompt` (the widget the slot builds).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnableSheet {
    pub feature: Enable,
}

impl_widget!(stateful eq EnableSheet);

impl State for EnableState {
    type Widget = EnableSheet;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, w: &EnableSheet, cx: &mut Cx<'_, Self>) -> Rebuild {
        let Msg::Picked(event) = msg;
        match event {
            Event::Choose(id) if id == strings::ENABLE_YES => {
                close_dialog::<_, ()>(cx, None);
                verb(cx, Intent::Request(Edit::FeatureEnabled(w.feature)));
            }
            Event::Choose(_) | Event::Cancel => close_dialog::<_, ()>(cx, None),
            _ => {}
        }
        Rebuild::No
    }

    fn build(&self, w: &EnableSheet, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let on: Handler<Event> = cx.map(Msg::Picked);
        Picker::new(Rc::new(strings::enable_prompt(w.feature)), on).into()
    }
}
