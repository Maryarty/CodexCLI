//! Mentions in the composer (`@` files, tasks, plugins, skills; `$` skills and
//! apps): pure carry of the oracle's mention machinery over core values.
//! What stays here is the data the app holds (the catalog, the tasks, the file matches, the
//! recent picks) and the shell's file search ([`files::FileSearchBackend`]); the popup's
//! logic (the session, the filter, the codec) is `kit::mentions`, held by the draft.

pub mod candidate;
pub mod catalog;
pub mod files;
pub mod sources;
pub mod task;
pub mod usage;
