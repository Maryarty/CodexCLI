//! The two Windows sandbox prompts (ADR-013; oracle `chatwidget/windows_sandbox_prompts.rs`
//! `open_windows_sandbox_enable_prompt`, `open_windows_sandbox_fallback_prompt`): sheets over the
//! chat that the presenter opens for the feature's `Wish::Prompt`. In Flutter each is a
//! `showModalBottomSheet` over a list of options; the rows come from the feature's `State::page`
//! (which choices the organization allows, whether one must be made), a row says `Intent::Begin`
//! with its mode or quits the app, and Esc comes back to the prompt while a choice is required.

use std::rc::Rc;

use rtwk::overlay::close_dialog;
use rtwk::provider::{read, select};
use rtwk::{AnyWidget, Cx, Handler, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::app::SessionSlice;
use codex_tui_core::features::FeatureStates;
use codex_tui_core::features::windows_sandbox::{Intent, Page, Prompt};
use codex_tui_core::screens::select::permissions::Selection;
use codex_tui_core::threads::queue::ModalToken;

use super::strings;
use crate::app::view::Hs;
use crate::features::verb;
use crate::widgets::picker::{Event, Picker};

/// One prompt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptSheet {
    pub prompt: Prompt,
    pub owner: Option<ModalToken<Selection>>,
}

pub struct PromptSheetState;

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Picked(Event),
}

impl StatefulWidget for PromptSheet {
    type State = PromptSheetState;

    fn create_state(&self) -> PromptSheetState {
        PromptSheetState
    }
}

impl_widget!(stateful eq PromptSheet);

impl State for PromptSheetState {
    type Widget = PromptSheet;
    type Msg = Msg;

    fn did_change_dependencies(&mut self, w: &PromptSheet, cx: &mut Cx<'_, Self>) {
        if let Some(token) = &w.owner {
            let owner = token.thread().clone();
            let active = select::<SessionSlice, bool, _>(cx, move |session| {
                session.active.as_ref() == Some(&owner)
            });
            let token = token.clone();
            let current = select::<FeatureStates, bool, _>(cx, {
                let token = token.clone();
                move |states| states.windows_sandbox.owns_prompt(&token)
            });
            if !active || !current {
                if current {
                    verb(cx, Intent::DismissOwned(token));
                }
                close_dialog::<_, ()>(cx, None);
            }
        }
    }

    fn update(&mut self, msg: Msg, w: &PromptSheet, cx: &mut Cx<'_, Self>) -> Rebuild {
        let Msg::Picked(event) = msg;
        match event {
            Event::Choose(id) => {
                let Some(choice) = strings::choice_of(&id) else {
                    return Rebuild::No;
                };
                match choice.mode() {
                    Some(mode) => verb(
                        cx,
                        match &w.owner {
                            Some(token) => Intent::BeginOwned {
                                token: token.clone(),
                                mode,
                            },
                            None => Intent::Begin(mode),
                        },
                    ),
                    None => {
                        let hs = read::<Hs, _>(cx);
                        cx.emit(&hs.quit, ());
                    }
                }
                close_dialog::<_, ()>(cx, None);
            }
            Event::Cancel => {
                // Esc brings the prompt back until a choice is made: it stays.
                let page = read::<FeatureStates, _>(cx).windows_sandbox.page(w.prompt);
                if !page.choice_required {
                    if let Some(token) = &w.owner {
                        verb(cx, Intent::DismissOwned(token.clone()));
                    }
                    close_dialog::<_, ()>(cx, None);
                }
            }
            _ => {}
        }
        Rebuild::No
    }

    fn build(&self, w: &PromptSheet, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let prompt = w.prompt;
        let page = select::<FeatureStates, Page, _>(cx, move |features| {
            features.windows_sandbox.page(prompt)
        });
        let on: Handler<Event> = cx.map(Msg::Picked);
        Picker::new(Rc::new(strings::menu(&page)), on).into()
    }
}
