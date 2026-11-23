//! `/review` (oracle `chatwidget/review_popups.rs`): the presets menu, the base-branch picker, the
//! commit picker and the custom instructions, as typed menus of the select screen. The menus are
//! data (their rows' ids, names and what a search matches); the words (`Select a review preset`,
//! `Current branch: ...`, the placeholders) are the view's. A child menu is pushed over the
//! presets, so Esc shows the presets again with the cursor on the row that opened it; a pick of a
//! target dismisses all of them (`dismiss_parent_on_child_accept`) and is answered as
//! `Picked::Review`.
//!
//! The branch list and the commit log are the host's (`Ask::GitBranches`, `Ask::GitLog`): the
//! model names the request an answer must belong to (`Review::pending`, as `Catalog::pending`
//! does), the app asks and tells the answer back.

use crate::threads::ReviewTarget;

/// The presets (the ids of the first menu's rows, in the oracle's order).
pub const BASE: &str = "review.base";
pub const UNCOMMITTED: &str = "review.uncommitted";
pub const COMMIT: &str = "review.commit";
pub const CUSTOM: &str = "review.custom";

/// The commits the picker lists (`recent_commits(cwd, 100)`).
pub const RECENT_COMMITS: usize = 100;

/// The effect requests of this menu carry this bit: the UI's own request numbers never do.
pub const REQUEST_BIT: u64 = 1 << 62;

/// One commit of the log (`CommitLogEntry`, without its time).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Commit {
    pub sha: String,
    pub subject: String,
}

/// A menu of the review family.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Menu {
    /// `Select a review preset`: base branch, uncommitted changes, a commit, custom instructions.
    Presets,
    /// `Select a base branch`; `current` is none on a detached HEAD (the view says
    /// `(detached HEAD)`).
    Branches {
        current: Option<String>,
        branches: Vec<String>,
    },
    /// `Select a commit to review`.
    Commits(Vec<Commit>),
    /// `Custom review instructions`: a text prompt; its answer is `Intent::Instructions`.
    Instructions,
}

/// A row of a menu: the presets have no name here (their words are the view's).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub id: String,
    pub name: Option<String>,
    /// What a search matches: the branch, or `subject sha` of a commit.
    pub search: Option<String>,
}

impl Menu {
    /// The rows of the menu, in order; the instructions prompt has none.
    pub fn rows(&self) -> Vec<Row> {
        let preset = |id: &str| Row {
            id: id.to_string(),
            name: None,
            search: None,
        };
        match self {
            Menu::Presets => vec![
                preset(BASE),
                preset(UNCOMMITTED),
                preset(COMMIT),
                preset(CUSTOM),
            ],
            Menu::Branches { branches, .. } => branches
                .iter()
                .map(|branch| Row {
                    id: branch_id(branch),
                    name: Some(branch.clone()),
                    search: Some(branch.clone()),
                })
                .collect(),
            Menu::Commits(commits) => commits
                .iter()
                .map(|commit| Row {
                    id: commit_id(&commit.sha),
                    name: Some(commit.subject.clone()),
                    search: Some(format!("{} {}", commit.subject, commit.sha)),
                })
                .collect(),
            Menu::Instructions => Vec::new(),
        }
    }

    /// What the pick of the row `id` of this menu starts: a review of a target, or the
    /// next step.
    pub fn choose(&self, id: &str) -> Option<Chosen> {
        match self {
            Menu::Presets => match id {
                BASE => Some(Chosen::Ask(Git::Branches)),
                COMMIT => Some(Chosen::Ask(Git::Log)),
                CUSTOM => Some(Chosen::Prompt),
                UNCOMMITTED => Some(Chosen::Target(ReviewTarget::UncommittedChanges)),
                _ => None,
            },
            Menu::Branches { branches, .. } => {
                let branch = id.strip_prefix(BRANCH)?;
                branches
                    .iter()
                    .find(|name| name.as_str() == branch)
                    .map(|branch| {
                        Chosen::Target(ReviewTarget::BaseBranch {
                            branch: branch.clone(),
                        })
                    })
            }
            Menu::Commits(commits) => {
                let sha = id.strip_prefix(COMMIT_ROW)?;
                commits.iter().find(|commit| commit.sha == sha).map(|commit| {
                    Chosen::Target(ReviewTarget::Commit {
                        sha: commit.sha.clone(),
                        title: Some(commit.subject.clone()),
                    })
                })
            }
            Menu::Instructions => None,
        }
    }
}

/// The prefixes of the ids of a branch row and of a commit row.
const BRANCH: &str = "branch:";
const COMMIT_ROW: &str = "commit:";

fn branch_id(branch: &str) -> String {
    [BRANCH, branch].concat()
}

fn commit_id(sha: &str) -> String {
    [COMMIT_ROW, sha].concat()
}

/// What a pick of a row does.
pub enum Chosen {
    /// The host is asked for a list; the answer opens the child menu.
    Ask(Git),
    /// The instructions prompt opens.
    Prompt,
    /// The review starts, and every review menu closes.
    Target(ReviewTarget),
}

/// What the host lists for a child menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Git {
    /// `local_git_branches`, and `current_branch_name`.
    Branches,
    /// `recent_commits(cwd, RECENT_COMMITS)`.
    Log,
}

/// The host's list, as the app tells it (`Done::GitBranches`, `Done::GitLog`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Listing {
    Branches {
        current: Option<String>,
        branches: Vec<String>,
    },
    Log(Vec<Commit>),
}

impl Listing {
    pub fn menu(self) -> Menu {
        match self {
            Listing::Branches { current, branches } => Menu::Branches { current, branches },
            Listing::Log(commits) => Menu::Commits(commits),
        }
    }
}

/// The state of the review menus that outlives a menu: where the repository is, and the list
/// asked for.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Review {
    /// The working directory the git lists are asked in (the session's).
    cwd: Option<String>,
    stamp: u64,
    /// The list asked for, under the request its answer must carry.
    pending: Option<(u64, Git)>,
    /// A request the app has not yet sent.
    unsent: bool,
    /// The host's last answer, under the request it answered: a fact the sheet over the presets
    /// reads (it pushes the page of a list it has not shown yet, by the request).
    listing: Option<(u64, Listing)>,
}

/// An ask the app sends for the review menus.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Wanted {
    pub request: u64,
    pub git: Git,
    pub cwd: Option<String>,
}

impl Review {
    pub(super) fn open(&mut self, cwd: String) {
        self.cwd = Some(cwd);
        self.pending = None;
        self.unsent = false;
        self.listing = None;
    }

    pub(super) fn want(&mut self, git: Git) {
        self.stamp += 1;
        self.pending = Some((REQUEST_BIT | self.stamp, git));
        self.unsent = true;
    }

    /// The request the app has still to send, once.
    pub fn take_wanted(&mut self) -> Option<Wanted> {
        let (request, git) = self.pending.filter(|_| self.unsent)?;
        self.unsent = false;
        Some(Wanted {
            request,
            git,
            cwd: self.cwd.clone(),
        })
    }

    /// Whether the answer `request` is the one asked for.
    pub fn waits_for(&self, request: u64) -> bool {
        self.pending.is_some_and(|(wanted, _)| wanted == request)
    }

    /// The host's answer: kept when it is the one asked for, else dropped.
    pub(super) fn answered(&mut self, request: u64, listing: Listing) {
        if !self.waits_for(request) {
            return;
        }
        self.pending = None;
        self.listing = Some((request, listing));
    }

    /// The host's last answer and the request it carries.
    pub fn listing(&self) -> Option<(u64, &Listing)> {
        self.listing.as_ref().map(|(request, listing)| (*request, listing))
    }
}
