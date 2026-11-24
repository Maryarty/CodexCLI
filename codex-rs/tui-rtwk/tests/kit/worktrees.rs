//! The host's answer to the worktree browser's list (`/worktree`, Browse worktrees): the oracle
//! hands `on_managed_worktrees_loaded` the entries of the repository's pool; here the sheet asks
//! the host (`Ask::Worktrees`) and a test answers the request still open.

use codex_tui_core::app::Command;
use codex_tui_core::app::effects::{Ask, Done};
use codex_tui_core::ids::ThreadId;
use codex_tui_core::screens::select::worktree::{Entry, Owner as Whose, Thread};

use super::Screen;

/// Who owns a worktree of the pool.
pub enum Held<'a> {
    /// A thread that can be resumed: its title and when it was last updated (Unix seconds).
    Resumable(&'a str, i64),
    /// An archived thread.
    Archived(&'a str, i64),
    /// An owner is recorded but its thread cannot be read.
    Missing,
    /// Nobody owns it.
    Nobody,
}

/// One worktree of the pool: its root and working directory.
pub struct Checkout<'a> {
    pub root: &'a str,
    pub cwd: &'a str,
    pub owner: Held<'a>,
}

fn thread(title: &str, updated_at: i64) -> Thread {
    Thread {
        id: ThreadId::new("00000000-0000-0000-0000-000000000001"),
        title: title.into(),
        updated_at,
    }
}

impl Screen {
    /// The host lists `pool` for the browser that waits for it.
    pub fn answer_worktrees(&mut self, pool: &[Checkout<'_>]) {
        let list = pool
            .iter()
            .map(|checkout| Entry {
                root: checkout.root.into(),
                cwd: checkout.cwd.into(),
                owner: match &checkout.owner {
                    Held::Resumable(title, at) => Whose::Resumable(thread(title, *at)),
                    Held::Archived(title, at) => Whose::Archived(thread(title, *at)),
                    Held::Missing => {
                        Whose::Unavailable(ThreadId::new("00000000-0000-0000-0000-000000000002"))
                    }
                    Held::Nobody => Whose::None,
                },
            })
            .collect();
        self.answer_listing(Ok(list));
    }

    fn answer_listing(&mut self, answer: Result<Vec<Entry>, String>) {
        let ask = self
            .commands()
            .into_iter()
            .rfind(|command| {
                matches!(
                    command,
                    Command::Effect {
                        ask: Ask::Worktrees { .. },
                        ..
                    }
                )
            })
            .expect("the browser has asked the host for the worktrees");
        self.answer_effect(&ask, Done::Worktrees(answer));
    }
}
