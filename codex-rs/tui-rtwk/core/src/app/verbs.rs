//! The app's half of the public verbs a feature has (ADR-012 "public verbs"): each verb is
//! carried out by its owner's own step, as if the owner had been told by its own surface.

use super::{AppEv, AppFx, CodexApp, Intent, apply_app, select};
use crate::features::PublicVerb;
use crate::screens::{ScreenIntent, chat};

/// One verb a feature asked for.
pub(super) fn carry(app: &mut CodexApp, verb: PublicVerb, fx: &mut AppFx) {
    match verb {
        PublicVerb::StatusOverride(set) => {
            let intent = Intent::Screen(ScreenIntent::Chat(chat::Intent::StatusOverride(set)));
            apply_app(app, AppEv::Intent(intent), fx);
        }
        PublicVerb::PickModel(selection) => select::model_pick(app, selection, fx),
    }
}
