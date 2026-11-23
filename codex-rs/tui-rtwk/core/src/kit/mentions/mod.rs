//! Mentions in the composer (`@` files, tasks, plugins, skills; `$` skills and apps): the
//! popup's session, filter and the codec of linked mentions, pure logic over the data the app
//! holds (`screens::composer::mentions`: catalog, tasks, file matches, picks). Nothing here
//! knows rtwk or the shell; the draft holds a [`session::Session`].

pub mod codec;
pub mod dollar;
pub mod filter;
pub mod popup;
pub mod rebind;
pub mod search_mode;
pub mod session;
pub mod target;

/// The data of the app a mention popup reads or answers with: the widgets name it here.
pub use crate::screens::composer::mentions::catalog::{Mention, MentionKind, tool_mentions};
pub use crate::screens::composer::mentions::files::FileSearchAsk;
pub use crate::screens::composer::mentions::task::valid_thread_path;
