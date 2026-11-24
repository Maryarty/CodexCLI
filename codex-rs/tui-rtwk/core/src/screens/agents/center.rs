//! What the command center shows, derived from the model: the rows in status
//! order, the visible ones in grouping order, the heading and task entries,
//! selection and paging. Oracle: `agents_overview.rs` (`agents_overview_view`),
//! `agents_overview_grouping.rs` (`visible_indices`, `same_group`),
//! `agent_center/navigation.rs`, `agent_center/mod.rs`.

use std::collections::HashMap;

use codex_app_server_protocol::Thread;
use codex_app_server_protocol::ThreadStatus;

use super::Grouping;
use super::History;
use super::Model;
use super::Sel;
use super::carried::AgentsOverviewGroup;
use super::carried::agents_overview_group;
use super::carried::matches_query;
use super::carried::model_name;
use super::details::Details;

use std::rc::Rc;

use crate::app::RequestQueue;
use crate::ids::ThreadId;
use crate::threads::{ThreadPreview, ThreadPreviews};

/// Read-only thread data supplied by the app when it projects the center.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CenterThreads {
    active: Option<ThreadId>,
    previews: ThreadPreviews,
    requests: Rc<RequestQueue>,
}

impl CenterThreads {
    pub fn new(active: Option<ThreadId>, previews: ThreadPreviews, requests: Rc<RequestQueue>) -> Self {
        Self { active, previews, requests }
    }

    /// The thread the session is on (the live one): the page that lists threads marks it and a
    /// command that acts on the conversation at hand reads which it is.
    pub fn live(&self) -> Option<&ThreadId> {
        self.active.as_ref()
    }

    /// What the app keeps of thread `id` for the rows: the prompt, the streamed reasoning, the
    /// last message.
    pub fn preview(&self, id: &str) -> Option<&ThreadPreview> {
        self.previews.get(id)
    }

    pub(super) fn request(&self, id: &str) -> Option<&str> {
        self.requests.preview(id).or_else(|| self.preview(id)?.seeded_request.as_deref())
    }
}

/// The status filter of the tab row (oracle: `TASK_FILTERS`): `None` is All.
pub const FILTER_GROUPS: [Option<AgentsOverviewGroup>; 5] = [
    None,
    Some(AgentsOverviewGroup::NeedsYou),
    Some(AgentsOverviewGroup::Working),
    Some(AgentsOverviewGroup::Ready),
    Some(AgentsOverviewGroup::Finished),
];

/// What the view shows that changes which rows count: the view owns it
/// (widget state) and hands it to the pure projections. Oracle: the view's
/// `filter`, `rename_target` and the search line being open.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Lens {
    /// The status tab, an index into `FILTER_GROUPS`.
    pub filter: usize,
    /// The search line is open (a closed search offers more of the list).
    pub searching: bool,
    /// The task whose name is being edited: it stays listed.
    pub renaming: Option<String>,
}

/// One root task (oracle: `AgentsOverviewRow`).
#[derive(Clone, Debug, PartialEq)]
pub struct RowData {
    pub(crate) thread: Thread,
    /// What the view shows of the task (the protocol type stays in core).
    pub name: Option<String>,
    pub preview: String,
    pub cwd: String,
    pub updated_at: i64,
    pub reasoning_effort: Option<String>,
    pub branch: Option<String>,
    pub group: AgentsOverviewGroup,
    pub unread: bool,
    pub current: bool,
    pub pinned: bool,
    /// The task's session owns the voice session (oracle: `has_voice`).
    pub has_voice: bool,
    pub details: Details,
}

impl RowData {
    /// A failed root task (shown as `Error`, not `Needs input`).
    pub fn is_error(&self) -> bool {
        self.group == AgentsOverviewGroup::NeedsYou
            && self.thread.status == ThreadStatus::SystemError
            && !super::carried::is_subagent(&self.thread)
    }

    pub fn id(&self) -> &str {
        &self.thread.id
    }

    pub fn title(&self) -> &str {
        super::carried::display_title(&self.thread)
    }

    pub fn model(&self) -> &str {
        model_name(&self.thread)
    }
}

/// What a heading says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GroupLabel {
    Status(AgentsOverviewGroup),
    Pinned,
    /// A project or directory path.
    Path(String),
    Model(String),
}

/// One row of the list (oracle: `CenterRow`). `Task` indexes `Center::rows`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CenterRow {
    Group { label: GroupLabel, count: String },
    Task(usize),
    Gap,
    ShowMore,
}

/// Everything the view draws, for one model state.
#[derive(Clone, Debug, PartialEq)]
pub struct Center {
    pub rows: Vec<RowData>,
    pub entries: Vec<CenterRow>,
    /// `All`, `Needs you`, `Working`, `Ready`, `Inactive`.
    pub tab_counts: [usize; 5],
    /// The highlighted row of `rows`, if a task is.
    pub selected: Option<usize>,
    pub show_more_selected: bool,
    pub more: bool,
}

impl Eq for Center {}

impl Center {
    /// The index into `entries` of the highlighted row.
    pub fn selected_entry(&self) -> Option<usize> {
        self.entries.iter().position(|entry| match entry {
            CenterRow::Task(index) => Some(*index) == self.selected,
            CenterRow::ShowMore => self.show_more_selected,
            _ => false,
        })
    }

    pub fn selected_row(&self) -> Option<&RowData> {
        self.selected.and_then(|index| self.rows.get(index))
    }
}

impl Model {
    /// Whether tasks get their own colour (`tui.status_line_use_colors`,
    /// on unless the setting says otherwise).
    pub fn thread_colors(&self) -> bool {
        !self.thread_colors_off
    }

    /// The plan is billed directly in dollars (oracle: `PlanType::Ent26`),
    /// which hides the credits and shows the native cost.
    pub fn plan_bills_in_dollars(&self) -> bool {
        self.plan == Some(codex_protocol::account::PlanType::Ent26)
    }

    /// The server's lock or the connection holds removals, new worktrees and
    /// the like back (oracle: the gate in `handle_key_event`); the view asks
    /// before it opens a confirmation.
    pub fn held(&self) -> bool {
        matches!(
            self.notice,
            Some(
                super::Notice::Reconnecting
                    | super::Notice::ReconnectFailed
                    | super::Notice::Removing(_)
                    | super::Notice::CreatingWorktree
            )
        )
    }

    /// The roots in status, newest and id order, hidden ones left out
    /// (oracle: `agents_overview_view`).
    pub(super) fn rows(&self, state: Option<&CenterThreads>) -> Vec<RowData> {
        let mut children: HashMap<String, Vec<&Thread>> = HashMap::new();
        for thread in &self.threads {
            if let Some(parent) = &thread.parent_thread_id {
                children.entry(parent.clone()).or_default().push(thread);
            }
        }
        let mut roots = self
            .visible_roots()
            .map(|root| (root, agents_overview_group(root, &children)))
            .collect::<Vec<_>>();
        roots.sort_by(|(left, left_group), (right, right_group)| {
            left_group
                .cmp(right_group)
                .then_with(|| right.updated_at.cmp(&left.updated_at))
                .then_with(|| left.id.cmp(&right.id))
        });
        let voice_session = self.voice.as_ref().and_then(|owner| {
            self.threads
                .iter()
                .find(|thread| &thread.id == owner)
                .map(|thread| thread.session_id.as_str())
        });
        roots
            .into_iter()
            .map(|(root, group)| RowData {
                thread: root.clone(),
                name: root.name.clone(),
                preview: root.preview.clone(),
                cwd: root.cwd.display().to_string(),
                updated_at: root.updated_at,
                branch: root.git_info.as_ref().and_then(|git| git.branch.clone()),
                reasoning_effort: root.reasoning_effort.as_ref().map(|e| e.as_str().to_owned()),
                group,
                unread: self.unread.contains(&root.id),
                current: state.and_then(|state| state.active.as_ref()).map(ThreadId::as_str) == Some(root.id.as_str()),
                pinned: self.pin_rank(&root.id).is_some(),
                has_voice: voice_session.is_some_and(|session| session == root.session_id),
                details: state.map_or_else(Details::default, |state| self.details(root, &children, state)),
            })
            .collect()
    }

    pub(super) fn pin_rank(&self, id: &str) -> Option<usize> {
        self.pinned_ids().position(|pin| pin == id)
    }

    /// The rows the filters keep, pinned first, in grouping order (oracle:
    /// `visible_indices`).
    pub(super) fn visible_indices(&self, rows: &[RowData], lens: &Lens) -> Vec<usize> {
        let search = self.search.to_lowercase();
        let status_group = FILTER_GROUPS[lens.filter.min(FILTER_GROUPS.len() - 1)];
        let renaming = lens.renaming.as_deref();
        let (mut pinned, mut visible): (Vec<_>, Vec<_>) = rows
            .iter()
            .enumerate()
            .filter_map(|(index, row)| {
                (renaming == Some(row.id())
                    || ((search.is_empty() || matches_query(&row.thread, &search))
                        && status_group.is_none_or(|group| group == row.group)))
                .then_some(index)
            })
            .partition(|index| rows[*index].pinned);
        pinned.sort_by_key(|index| self.pin_rank(rows[*index].id()));
        match self.grouping {
            Grouping::Project => visible.sort_by_key(|index| {
                (
                    self.project_key(&rows[*index].thread),
                    std::cmp::Reverse(rows[*index].thread.updated_at),
                )
            }),
            Grouping::Directory => visible.sort_by_key(|index| {
                (
                    self.directory_key(&rows[*index].thread),
                    std::cmp::Reverse(rows[*index].thread.updated_at),
                )
            }),
            Grouping::Status => {
                pinned.extend(visible);
                // Stable grouping preserves pin order, then the newest-first order.
                pinned.sort_by_key(|index| rows[*index].group);
                return pinned;
            }
            Grouping::Model => visible.sort_by_key(|index| {
                (
                    model_name(&rows[*index].thread).to_owned(),
                    std::cmp::Reverse(rows[*index].thread.updated_at),
                )
            }),
        }
        pinned.extend(visible);
        pinned
    }

    /// The project of a thread: its repository and the path inside it, or
    /// its `cwd` (oracle: `AgentsOverviewProjectGroup::key`).
    fn project_key(&self, thread: &Thread) -> (String, String) {
        let cwd = thread.cwd.display().to_string();
        match self.dirs.get(&cwd).filter(|_| self.worktrees) {
            Some(keys) => keys.project.clone(),
            None => (cwd, String::new()),
        }
    }

    fn project_heading(&self, thread: &Thread) -> String {
        let cwd = thread.cwd.display().to_string();
        match self.dirs.get(&cwd).filter(|_| self.worktrees) {
            Some(keys) => keys.heading.clone(),
            None => cwd,
        }
    }

    /// The directory of a thread, canonical when the shell could tell
    /// (oracle: `directory_group_key`).
    fn directory_key(&self, thread: &Thread) -> String {
        let cwd = thread.cwd.display().to_string();
        match self.dirs.get(&cwd).filter(|_| self.worktrees) {
            Some(keys) => keys.directory.clone(),
            None => cwd,
        }
    }

    fn same_group(&self, rows: &[RowData], left: usize, right: usize) -> bool {
        let (l, r) = (&rows[left], &rows[right]);
        match self.grouping {
            Grouping::Status => l.group == r.group,
            _ if l.pinned || r.pinned => l.pinned && r.pinned,
            Grouping::Project => self.project_key(&l.thread) == self.project_key(&r.thread),
            Grouping::Directory => self.directory_key(&l.thread) == self.directory_key(&r.thread),
            Grouping::Model => model_name(&l.thread) == model_name(&r.thread),
        }
    }

    pub(super) fn more_available(&self, lens: &Lens) -> bool {
        self.history
            .as_ref()
            .map_or(self.paging.has_more && !lens.searching, History::has_more)
    }

    /// Visible rows plus the "Show more" slot (`usize::MAX`).
    fn selectable(&self, rows: &[RowData], lens: &Lens) -> Vec<usize> {
        let mut indices = self.visible_indices(rows, lens);
        if self.more_available(lens) {
            indices.push(usize::MAX);
        }
        indices
    }

    fn entries(&self, rows: &[RowData], indices: &[usize]) -> Vec<CenterRow> {
        let mut entries = Vec::new();
        let mut previous = None;
        for index in indices.iter().copied() {
            if index == usize::MAX {
                entries.push(CenterRow::Gap);
                entries.push(CenterRow::ShowMore);
                continue;
            }
            if previous.is_none_or(|previous| !self.same_group(rows, previous, index)) {
                if previous.is_some() {
                    entries.push(CenterRow::Gap);
                }
                entries.push(self.heading(rows, indices, index));
            }
            entries.push(CenterRow::Task(index));
            previous = Some(index);
        }
        entries
    }

    fn heading(&self, rows: &[RowData], indices: &[usize], index: usize) -> CenterRow {
        let row = &rows[index];
        let label = match self.grouping {
            Grouping::Status => GroupLabel::Status(row.group),
            _ if row.pinned => GroupLabel::Pinned,
            Grouping::Project => GroupLabel::Path(self.project_heading(&row.thread)),
            Grouping::Directory => GroupLabel::Path(row.thread.cwd.display().to_string()),
            Grouping::Model => GroupLabel::Model(model_name(&row.thread).to_owned()),
        };
        let count = indices
            .iter()
            .filter(|&&other| other != usize::MAX && self.same_group(rows, other, index))
            .count();
        let total = (0..rows.len())
            .filter(|&other| self.same_group(rows, other, index))
            .count();
        let count = if count == total {
            count.to_string()
        } else {
            format!("{count} of {total}")
        };
        CenterRow::Group { label, count }
    }

    fn task_candidates(&self) -> impl Iterator<Item = &Thread> {
        let matches = self.history.iter().flat_map(|_| self.search_matches.iter());
        self.threads.iter().chain(matches)
    }

    fn visible_roots(&self) -> impl Iterator<Item = &Thread> {
        self.task_candidates()
            .filter(|thread| thread.parent_thread_id.is_none() && !self.hidden.contains(&thread.id))
    }

    pub(super) fn task(&self, id: &str) -> Option<&Thread> {
        if self.hidden.contains(id) {
            return None;
        }
        self.task_candidates()
            .find(|thread| thread.id == id && thread.parent_thread_id.is_none())
    }

    pub fn has_task(&self, id: &str) -> bool {
        self.task(id).is_some()
    }

    /// The text a confirmation uses for the task's name.
    pub fn task_name(&self, id: &str) -> Option<&str> {
        self.task(id).map(|thread| thread.name.as_deref().unwrap_or(&thread.preview))
    }

    /// Everything the view draws, with the highlight at the default row (the
    /// current task's, else the first).
    pub fn center(&self, state: &CenterThreads) -> Center {
        self.center_at(state, None, &Lens::default())
    }

    /// Everything the view draws, the highlight at `cursor` (the list
    /// widget's cursor, the only copy of it) when it is selectable.
    pub fn center_at(&self, state: &CenterThreads, cursor: Option<&Sel>, lens: &Lens) -> Center {
        let rows = self.rows(Some(state));
        let indices = self.selectable(&rows, lens);
        let entries = self.entries(&rows, &indices);
        let mut tab_counts = [0; 5];
        for (slot, group) in FILTER_GROUPS.iter().enumerate() {
            tab_counts[slot] = rows
                .iter()
                .filter(|row| group.is_none_or(|group| group == row.group))
                .count();
        }
        let selected = self.selected_index(&rows, &indices, cursor);
        Center {
            show_more_selected: selected == Some(usize::MAX),
            selected: selected.filter(|index| *index != usize::MAX),
            more: indices.contains(&usize::MAX),
            rows,
            entries,
            tab_counts,
        }
    }

    /// The highlighted slot: the selection if it is selectable, else the
    /// first (oracle: `reconcile_command_center_selection`).
    fn selected_index(
        &self,
        rows: &[RowData],
        indices: &[usize],
        cursor: Option<&Sel>,
    ) -> Option<usize> {
        let wanted = match cursor {
            Some(Sel::Task(id)) => rows.iter().position(|row| row.id() == id.as_str()),
            Some(Sel::ShowMore) => Some(usize::MAX),
            None => None,
        };
        // Oracle (`AgentsOverviewView::new`): the highlighted id, else the
        // current thread's row, else the first.
        wanted
            .or_else(|| rows.iter().position(|row| row.current))
            .filter(|index| indices.contains(index))
            .or_else(|| indices.first().copied())
    }

    /// Drops a thread and everything below it, however deep (oracle:
    /// `remove_agents_overview_thread` with its descendants).
    pub(super) fn remove_subtree(&mut self, id: &str) {
        self.remove_tree(id, true);
    }

    /// Drops a thread, and everything below it when `whole` (a cloud archive
    /// keeps the children). Each shown root that leaves is a vacancy for
    /// the refill (oracle: `remove_agents_overview_thread`).
    pub(super) fn remove_tree(&mut self, id: &str, whole: bool) {
        let mut gone = vec![id.to_owned()];
        let mut next = 0;
        while whole && next < gone.len() {
            let parent = gone[next].clone();
            gone.extend(
                self.threads
                    .iter()
                    .filter(|held| held.parent_thread_id.as_deref() == Some(parent.as_str()))
                    .map(|held| held.id.clone()),
            );
            next += 1;
        }
        for held in &self.threads {
            if gone.contains(&held.id)
                && held.parent_thread_id.is_none()
                && !held.ephemeral
                && !self.hidden.contains(&held.id)
            {
                self.paging.refill += 1;
            }
        }
        self.removed.extend(gone.iter().cloned());
        self.search_matches.retain(|held| !gone.contains(&held.id));
        self.threads.retain(|held| !gone.contains(&held.id));
    }

    /// The task after `removed` rows leave: the next displayed one, else the
    /// one before (oracle: `selection_after_removal`).
    /// A pure function: the view applies it to its cursor (`from`: the id of
    /// the task that is highlighted).
    pub fn selection_after_removal(
        &self,
        from: &str,
        removed: &[String],
        lens: &Lens,
    ) -> Option<String> {
        let rows = self.rows(None);
        let visible = self.visible_indices(&rows, lens);
        let from = Sel::Task(from.to_owned());
        let selected = self.selected_index(&rows, &visible, Some(&from))?;
        let position = visible.iter().position(|index| *index == selected)?;
        visible[position..]
            .iter()
            .chain(visible[..position].iter().rev())
            .map(|index| rows[*index].id().to_owned())
            .find(|id| !removed.contains(id))
    }
}

/// A directory as the list shows it (oracle: `format_directory_display`
/// with no width): below `home` it starts with `~`.
pub fn display_directory(path: &str, home: Option<&str>) -> String {
    let Some(home) = home.map(|home| home.trim_end_matches('/')) else {
        return path.to_owned();
    };
    match path.strip_prefix(home) {
        Some("") => "~".to_owned(),
        Some(rest) if rest.starts_with('/') => format!("~{rest}"),
        _ => path.to_owned(),
    }
}
