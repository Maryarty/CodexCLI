//! `kit` (ADR-013 O3): pure logic without app state, held by whoever uses it (the composer's
//! draft widget): Vim's engine, the paste-burst detector, the walk over the prompt history and
//! the mention popups' session and codec. A kit type is never a field of a domain type, and no
//! kit file names a root of the domain (`tests/rules/adr013.rs`, `adr013_kit_is_not_app_state`).
//! The data the app holds that a kit function reads (`Store`, `Mention`, `FileSearchAsk`) stays
//! with its screen; the kit imports it and the widgets reach it through the kit's re-exports.

pub mod history;
pub mod mentions;
pub mod paste;
pub mod paste_burst;
pub mod vim;
