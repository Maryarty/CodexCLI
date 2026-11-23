//! Mentions in the composer (`@` files, tasks, plugins, skills; `$` skills and
//! apps): pure carry of the oracle's mention machinery over core values.
//! What stays here is what the app builds from the backend (the catalog's sources) and the
//! shell's file search ([`files::FileSearchBackend`]); the values are `core::data`, the popup's
//! logic (the session, the filter, the codec, the catalog) is `kit::mentions`, held by the draft.

pub mod files;
pub mod sources;
