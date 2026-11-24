//! The view half of the security-setup reminder (ADR-013): the words of its notes. The feature is
//! `codex_tui_core::features::security_setup` (the notice the host fetched, its occurrence, the
//! action's verb). The inline banner over the composer (title, description, one action row; Esc
//! dismisses it for its occurrence, and a dismissal survives the usage banner's replacement and
//! the thread switches) is the session page's (`pages/session/banners.rs`: it shows while no
//! banner of the backend applies, `BannerId::Security`); the page keeps the dismissed occurrence
//! (`BannerGate::dismissed`, UI state, ADR-013).

pub mod strings;

use codex_tui_core::features::security_setup::{Note, SecuritySetup};

use super::FeatureView;
use crate::pages::session::strings::Strings as SessionStrings;

impl FeatureView for SecuritySetup {
    fn note_words(note: &Note, _words: &SessionStrings) -> String {
        strings::note(note)
    }
}
