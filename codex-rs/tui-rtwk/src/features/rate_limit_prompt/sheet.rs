//! The sheets of the limit prompts (ADR-013): a list of rows over the chat that the presenter
//! opens for a wish of the feature. In Flutter this is `showModalBottomSheet` over a list of
//! options; what the user decides goes back to the feature as a verb, and Esc says only that the
//! prompt is closed. The switch prompt watches the feature's state and closes itself when the
//! prompt is no longer wanted (credits turned up: `dismiss_view_by_id`).

use std::rc::Rc;

use rtwk::overlay::close_dialog;
use rtwk::provider::select;
use rtwk::{AnyWidget, Cx, Handler, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::features::FeatureStates;
use codex_tui_core::features::rate_limit_prompt::{Candidate, Intent, OfferId, Standing};
use codex_tui_core::screens::chat::Nudge;

use super::strings;
use crate::features::{hide_rate_limit_prompt, verb};
use crate::widgets::picker::{Event, Picker};

/// What the sheet asks.
#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    /// `Approaching rate limits`, offering the model.
    Switch {
        offer: OfferId,
        candidate: Candidate,
    },
    /// The workspace owner is asked for credits or a higher limit.
    Nudge(Nudge),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Prompt {
    pub kind: Kind,
}

pub struct PromptState;

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Picked(Event),
}

impl StatefulWidget for Prompt {
    type State = PromptState;

    fn create_state(&self) -> PromptState {
        PromptState
    }
}

impl_widget!(stateful eq Prompt);

impl PromptState {
    /// What the switch prompt shows: asked in `did_change_dependencies` and again in `build`,
    /// because a build forgets the selects of the build before.
    fn standing(cx: &mut Cx<'_, Self>, offer: &OfferId) -> Standing {
        let offer = offer.clone();
        select::<FeatureStates, Standing, _>(cx, move |features| {
            features.rate_limit_prompt.standing(&offer)
        })
    }
}

impl State for PromptState {
    type Widget = Prompt;
    type Msg = Msg;

    fn did_change_dependencies(&mut self, w: &Prompt, cx: &mut Cx<'_, Self>) {
        if let Kind::Switch { offer, .. } = &w.kind
            && Self::standing(cx, offer) == Standing::Gone
        {
            close_dialog::<_, ()>(cx, None);
        }
    }

    fn update(&mut self, msg: Msg, w: &Prompt, cx: &mut Cx<'_, Self>) -> Rebuild {
        let Msg::Picked(event) = msg;
        match (&w.kind, &event) {
            (Kind::Switch { offer, .. }, _) if Self::standing(cx, offer) == Standing::Gone => {}
            (Kind::Switch { offer, candidate }, Event::Choose(id)) => match id.as_str() {
                strings::SWITCH => verb(
                    cx,
                    Intent::Switch {
                        offer: offer.clone(),
                        model: candidate.model.clone(),
                        effort: candidate.effort.clone(),
                    },
                ),
                strings::NEVER => {
                    hide_rate_limit_prompt(cx);
                    verb(cx, Intent::Closed(offer.clone()));
                }
                strings::KEEP | strings::DISMISS => verb(cx, Intent::Closed(offer.clone())),
                _ => return Rebuild::No,
            },
            (Kind::Switch { offer, .. }, Event::Cancel) => {
                verb(cx, Intent::Closed(offer.clone()));
            }
            (Kind::Nudge(nudge), Event::Choose(id)) => match id.as_str() {
                strings::YES => verb(cx, Intent::Nudge(*nudge)),
                strings::NO => {}
                _ => return Rebuild::No,
            },
            (Kind::Nudge(_), Event::Cancel) => {}
            _ => return Rebuild::No,
        }
        close_dialog::<_, ()>(cx, None);
        Rebuild::No
    }

    fn build(&self, w: &Prompt, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let on: Handler<Event> = cx.map(Msg::Picked);
        let list = match &w.kind {
            Kind::Switch { offer, candidate } => match Self::standing(cx, offer) {
                Standing::Recovery(seconds) => strings::recovery_menu(seconds),
                Standing::Offer | Standing::Gone => strings::switch_menu(candidate),
            },
            Kind::Nudge(nudge) => strings::nudge_menu(*nudge),
        };
        Picker::new(Rc::new(list), on).into()
    }
}
