//! The local pet selection's outcome in scrollback.

pub(crate) use codex_tui_core::features::pets::{
    Intent as PetIntent, Phase as PetPhase, Visit as PetVisit, is_disable as is_pet_disable,
};
use codex_tui_core::features::pets::{Note, PetSelection};

use super::FeatureView;
use crate::pages::session::strings::Strings;

mod strings;

impl FeatureView for PetSelection {
    fn note_words(note: &Note, _: &Strings) -> String {
        strings::note(note)
    }
}
