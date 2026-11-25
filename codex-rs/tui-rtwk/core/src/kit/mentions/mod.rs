//! Mentions in the composer (`@` files, tasks, plugins, skills; `$` skills and apps): the
//! popup's session, filter and the codec of linked mentions, pure logic over the data the app
//! holds (`core::data`: catalog, tasks, file matches, picks). Nothing here
//! knows rtwk or the shell; the draft holds a [`session::Session`].

pub mod catalog;
pub mod codec;
pub mod dollar;
pub mod files;
pub mod filter;
pub mod popup;
pub mod rebind;
pub mod row;
pub mod search_mode;
pub mod session;
pub mod target;
pub mod task_link;
pub mod task_refs;
pub mod tools;

pub use task_refs::{TaskRef, WithContext, decode_task_links, with_task_context};
pub use tools::tool_mentions;
