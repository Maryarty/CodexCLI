//! The worktree pickers (oracle `chatwidget/worktree_picker.rs`, `worktree_browser.rs`): where
//! `/new` and `/fork` run (the current checkout or a new worktree), what `/worktree` does
//! (continue this conversation or start a new one in a new checkout, or browse), and the browser
//! of the repository's worktrees with its actions and the delete confirmation. The menus are
//! data (rows' ids, names, what a search matches, what is disabled); the words are the view's.
//! The pages are the sheet's (`Opening::Worktree`, ADR-013 O1): every pick of the oracle
//! dismisses its view and shows the next one, so a page replaces the page before it and Esc
//! closes the sheet; the model keeps no stack of them.
//!
//! The list and the removal are the host's (`Ask::Worktrees`, `Ask::WorktreeRemove`): the model
//! names the request an answer must belong to (`Worktree::pending`), the app asks and tells the
//! answer back, and the answer stays as a fact the loading page reads (`Worktree::listing`). A
//! request is tied to the working directory it was made in and to the scope of the menus
//! (`Worktree::epoch`): an answer the sheet gave up (`Verb::Abandon`) or one for another scope is
//! dropped (`worktree_request_is_current`).

use std::path::Path;

use crate::ids::ThreadId;

pub const CURRENT: &str = "worktree.current";
pub const CREATE: &str = "worktree.create";
pub const CONTINUE: &str = "worktree.continue";
pub const START: &str = "worktree.start";
pub const BROWSE: &str = "worktree.browse";
pub const RESUME: &str = "worktree.resume";
pub const COPY: &str = "worktree.copy";
pub const DELETE: &str = "worktree.delete";
pub const CANCEL: &str = "worktree.cancel";

/// The effect requests of these menus carry this bit (the review menus' is `1 << 62`).
pub const REQUEST_BIT: u64 = 1 << 61;

/// The prefix of the ids of the browser's rows (the entry's position follows).
const ENTRY: &str = "worktree:";

/// What the checkout is for (`ManagedWorktreeMode`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Conversation {
    New,
    Fork,
}

/// The thread that owns a worktree (`ThreadSummary`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Thread {
    pub id: ThreadId,
    pub title: String,
    /// Unix seconds.
    pub updated_at: i64,
}

/// Who a worktree belongs to (`Owner`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Owner {
    None,
    /// An owner is recorded but the thread cannot be read.
    Unavailable(ThreadId),
    Archived(Thread),
    Resumable(Thread),
}

/// One worktree of the repository's pool (`Entry`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub root: String,
    pub cwd: String,
    pub owner: Owner,
}

impl Entry {
    /// The row's name: the owner's title, or the working directory of an unowned worktree.
    pub fn name(&self) -> &str {
        match &self.owner {
            Owner::Archived(thread) | Owner::Resumable(thread) => &thread.title,
            Owner::None | Owner::Unavailable(_) => &self.cwd,
        }
    }

    /// Whether the worktree may be deleted from a session working in `cwd`: not while the session
    /// runs inside it (`Switch to another checkout before deleting this one`).
    pub fn can_delete(&self, cwd: &str) -> bool {
        !Path::new(cwd).starts_with(&self.root)
    }
}

/// The `thread/read` that tells who owns a worktree (`worktree_browser::fetch` reads the thread
/// of each recorded owner): the host sends it for every entry the pool names an owner of.
pub fn owner_request(
    thread: &str,
    id: codex_app_server_protocol::RequestId,
) -> codex_app_server_protocol::ClientRequest {
    crate::threads::ThreadAsk::Read { thread: thread.to_owned() }.wire(id)
}

/// Why no worktree is made for the conversation (`start_managed_worktree`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// Not an idle primary session without queued input.
    NotIdle,
    /// A creation (or a new conversation) is already under way.
    Pending,
    /// A fork needs saved history.
    HistoryNotSaved,
}

/// How long ago a worktree's owner thread was updated (`worktree_updated_ago`); the view words it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ago {
    JustNow,
    Minutes(i64),
    Hours(i64),
    Days(i64),
}

impl Ago {
    pub fn of(updated_at: i64, now: i64) -> Ago {
        let seconds = now.saturating_sub(updated_at).max(0);
        if seconds < 60 {
            Ago::JustNow
        } else if seconds < 3_600 {
            Ago::Minutes(seconds / 60)
        } else if seconds < 86_400 {
            Ago::Hours(seconds / 3_600)
        } else {
            Ago::Days(seconds / 86_400)
        }
    }
}

/// A menu of the worktree family.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Menu {
    /// `Where should the new (forked) conversation run?`: the current checkout, or a new one.
    Checkout { mode: Conversation, name: Option<String> },
    /// `Worktrees`: continue this conversation in a new checkout, start a new one, or browse.
    Worktrees,
    /// `Managed worktrees` while the list loads (one disabled row).
    Loading,
    /// `Managed worktrees`: the pool, searchable.
    Browser(Vec<Entry>),
    /// `Worktree: {title}`: what to do with one worktree; `can_delete` is false while the
    /// session runs inside it (`Switch to another checkout before deleting this one`).
    Actions { entry: Entry, can_delete: bool },
    /// `Delete this worktree?`: Cancel, then Delete.
    Confirm { root: String },
}

/// A row: the fixed ones carry no words here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub id: String,
    /// The browser's rows: the owner's title, or the directory.
    pub name: Option<String>,
    /// What a search matches: `{name} {cwd}`.
    pub search: Option<String>,
    /// A disabled row cannot be picked.
    pub disabled: bool,
}

impl Menu {
    /// The rows, in order.
    pub fn rows(&self) -> Vec<Row> {
        let fixed = |id: &str, disabled: bool| Row {
            id: id.to_string(),
            name: None,
            search: None,
            disabled,
        };
        match self {
            Menu::Checkout { .. } => vec![fixed(CURRENT, false), fixed(CREATE, false)],
            Menu::Worktrees => vec![fixed(CONTINUE, false), fixed(START, false), fixed(BROWSE, false)],
            Menu::Loading => vec![fixed("loading", true)],
            Menu::Browser(entries) => entries
                .iter()
                .enumerate()
                .map(|(at, entry)| Row {
                    id: [ENTRY, &at.to_string()].concat(),
                    name: Some(entry.name().to_string()),
                    search: Some([entry.name(), " ", &entry.cwd].concat()),
                    disabled: false,
                })
                .collect(),
            Menu::Actions { entry, can_delete } => {
                let mut rows = Vec::new();
                if matches!(entry.owner, Owner::Resumable(_)) {
                    rows.push(fixed(RESUME, false));
                }
                rows.push(fixed(COPY, false));
                rows.push(fixed(DELETE, !can_delete));
                rows
            }
            Menu::Confirm { .. } => vec![fixed(CANCEL, false), fixed(DELETE, false)],
        }
    }

    /// What the pick of the row `id` of this page does: the pure half of the sheet's `update`.
    pub fn choose(&self, id: &str) -> Option<Chosen> {
        match (self, id) {
            (Menu::Checkout { mode, name }, CURRENT) => Some(Chosen::Pick(Pick::Current {
                mode: *mode,
                name: name.clone(),
            })),
            (Menu::Checkout { mode, name }, CREATE) => Some(Chosen::Pick(Pick::Start {
                mode: *mode,
                name: name.clone(),
            })),
            (Menu::Worktrees, CONTINUE) => Some(Chosen::Pick(Pick::Start {
                mode: Conversation::Fork,
                name: None,
            })),
            (Menu::Worktrees, START) => Some(Chosen::Pick(Pick::Start {
                mode: Conversation::New,
                name: None,
            })),
            (Menu::Worktrees, BROWSE) => Some(Chosen::Browse),
            (Menu::Browser(entries), id) => {
                let at: usize = id.strip_prefix(ENTRY)?.parse().ok()?;
                entries.get(at).cloned().map(Chosen::Actions)
            }
            (Menu::Actions { entry, .. }, RESUME) => match &entry.owner {
                Owner::Resumable(thread) => Some(Chosen::Pick(Pick::Resume(thread.id.clone()))),
                _ => None,
            },
            (Menu::Actions { entry, .. }, COPY) => {
                Some(Chosen::Pick(Pick::Copy(entry.cwd.clone())))
            }
            (Menu::Actions { entry, can_delete }, DELETE) => can_delete.then(|| {
                Chosen::Confirm {
                    root: entry.root.clone(),
                }
            }),
            (Menu::Confirm { .. }, CANCEL) => Some(Chosen::Close),
            (Menu::Confirm { root }, DELETE) => {
                Some(Chosen::Pick(Pick::Remove { root: root.clone() }))
            }
            _ => None,
        }
    }
}

/// What a pick of a row does to the pages.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Chosen {
    /// The sheet closes and the app acts.
    Pick(Pick),
    /// The loading page replaces this one and the list is asked for (`Verb::List`).
    Browse,
    /// The actions of an entry replace the browser.
    Actions(Entry),
    /// The delete confirmation replaces the actions.
    Confirm { root: String },
    /// The sheet closes, nothing is done.
    Close,
}

/// What a page of the sheet says to the model (`Intent::Worktree`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verb {
    /// The pages settled on this: the app acts (a removal is first asked of the host).
    Pick(Pick),
    /// The browser's list is asked of the host.
    List,
    /// The list asked for is not wanted any more (the loading page was left): its answer is
    /// dropped, and so is its failure.
    Abandon,
}

/// What the user settled on; the app acts (`Picked::Worktree`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pick {
    /// Run in the current checkout: `/new`, `/fork` as they are.
    Current { mode: Conversation, name: Option<String> },
    /// Create a worktree and run there (`StartManagedWorktree`).
    Start { mode: Conversation, name: Option<String> },
    /// Resume the owner thread.
    Resume(ThreadId),
    /// Copy the working directory.
    Copy(String),
    /// Remove the worktree at this root (`RemoveManagedWorktree`).
    Remove { root: String },
}

/// What the host is asked for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Want {
    List,
    Remove { root: String },
}

/// An ask the app sends for the worktree menus.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Wanted {
    pub request: u64,
    pub want: Want,
    /// The working directory of the session the request is for.
    pub cwd: String,
}

/// The state of the worktree menus that outlives a page: the scope of the requests, the one asked
/// for and its answer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Worktree {
    cwd: String,
    thread: Option<ThreadId>,
    stamp: u64,
    /// How many scopes the menus have had: a sheet of an older one closes.
    epoch: u64,
    pending: Option<u64>,
    /// The host's last list, or why there is none: a fact the loading page reads.
    listing: Option<Result<Vec<Entry>, String>>,
    outbox: Vec<(u64, Want)>,
    /// The removals asked for: the request and the root.
    removing: Vec<(u64, String)>,
}

impl Worktree {
    /// The session the menus are opened for.
    pub(super) fn open(&mut self, cwd: String, thread: Option<ThreadId>) {
        self.cwd = cwd;
        self.thread = thread;
        self.rescope();
    }

    /// A new scope: what was asked for is not wanted, what was answered is gone, and the sheets of
    /// the old scope close.
    pub(super) fn rescope(&mut self) {
        self.epoch += 1;
        self.pending = None;
        self.listing = None;
    }

    /// What a page said: the pick the app acts on, if it is one (`Verb`).
    pub(super) fn say(&mut self, verb: Verb) -> Option<Pick> {
        match verb {
            Verb::Pick(Pick::Remove { root }) => {
                if self.removable(&root) {
                    self.want(Want::Remove { root });
                }
                None
            }
            Verb::Pick(pick) => Some(pick),
            Verb::List => {
                self.want(Want::List);
                None
            }
            Verb::Abandon => {
                self.pending = None;
                None
            }
        }
    }

    fn want(&mut self, want: Want) {
        self.stamp += 1;
        let request = REQUEST_BIT | self.stamp;
        match &want {
            Want::List => {
                self.pending = Some(request);
                self.listing = None;
            }
            Want::Remove { root } => self.removing.push((request, root.clone())),
        }
        self.outbox.push((request, want));
    }

    /// The requests the app has still to send.
    pub fn take_wanted(&mut self) -> Vec<Wanted> {
        std::mem::take(&mut self.outbox)
            .into_iter()
            .map(|(request, want)| Wanted {
                request,
                want,
                cwd: self.cwd.clone(),
            })
            .collect()
    }

    /// The working directory the menus were opened in.
    pub fn cwd(&self) -> &str {
        &self.cwd
    }

    pub fn thread(&self) -> Option<&ThreadId> {
        self.thread.as_ref()
    }

    /// The scope the menus are in: a sheet closes when it is not the one it was opened for.
    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    /// The host's answer to the list asked for: the worktrees, or why they could not be listed.
    pub fn listing(&self) -> Option<&Result<Vec<Entry>, String>> {
        self.listing.as_ref()
    }

    /// Whether `request` is the list the menus wait for, or a removal asked for.
    pub fn waits_for(&self, request: u64) -> bool {
        self.pending == Some(request) || self.removing.iter().any(|(asked, _)| *asked == request)
    }

    /// The root of the removal `request` asked for, once.
    pub(super) fn removed(&mut self, request: u64) -> Option<String> {
        let at = self.removing.iter().position(|(asked, _)| *asked == request)?;
        Some(self.removing.remove(at).1)
    }

    /// Whether a worktree at `root` may be removed from this session (`!cwd.starts_with(root)`).
    fn removable(&self, root: &str) -> bool {
        !Path::new(&self.cwd).starts_with(root)
    }

    /// The host's answer to the list: kept as the fact when it is the one asked for, else dropped.
    /// A failure is given back, for the history to say.
    pub(super) fn answered(
        &mut self,
        request: u64,
        result: Result<Vec<Entry>, String>,
    ) -> Option<String> {
        if self.pending != Some(request) {
            return None;
        }
        self.pending = None;
        let failure = result.as_ref().err().cloned();
        self.listing = Some(result);
        failure
    }
}
