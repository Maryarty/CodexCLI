//! Mentions in the composer (`@` files, tasks, plugins, skills; `$` skills and
//! apps): pure carry of the oracle's mention machinery over core values.
//! Nothing here knows rtwk or the shell: the view draws [`popup::Popup`] and
//! [`dollar`]'s rows, the shell runs [`files::FileSearchBackend`].

pub mod candidate;
pub mod catalog;
pub mod codec;
pub mod dollar;
pub mod files;
pub mod filter;
pub mod popup;
pub mod rebind;
pub mod search_mode;
pub mod session;
pub mod sources;
pub mod target;
pub mod task;
pub mod usage;
