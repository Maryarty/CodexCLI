//! `kit` (ADR-013 O3): pure logic without app state, held by whoever uses it (the composer's
//! draft widget): Vim's engine, the paste-burst detector, the walk over the prompt history and
//! the mention popups' session and codec. A kit type is never a field of a domain type, and no
//! kit file names a root of the domain (`tests/rules/adr013.rs`, `adr013_kit_is_not_app_state`).
//! The data the app holds that a kit function reads (`Store`, `Mention`, `FileSearchAsk`) is
//! `core::data`: the kit and the widgets name it there, and the kit names no screen or feature.

pub mod backtrack;
pub mod fuzzy;
pub mod history;
pub mod mentions;
pub mod paste;
pub mod paste_burst;
pub mod vim;
