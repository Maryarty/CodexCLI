//! What the command center shows, derived from the model: the rows in status
//! order, the visible ones in grouping order, the heading and task entries,
//! selection and paging. Oracle: `agents_overview.rs` (`agents_overview_view`),
//! `agents_overview_grouping.rs` (`visible_indices`, `same_group`),
//! `agent_center/navigation.rs`, `agent_center/mod.rs`.

use std::collections::HashMap;

use codex_app_server_protocol::Thread;
use codex_app_server_protocol::ThreadStatus;

use super::Grouping;
use super::Model;
use super::Sel;
use super::carried::AgentsOverviewGroup;
use super::carried::agents_overview_group;
use super::carried::matches_query;
use super::carried::model_name;
use super::details::Details;

/// The status filter of the tab row (oracle: `TASK_FILTERS`): `None` is All.
pub const FILTER_GROUPS: [Option<AgentsOverviewGroup>; 5] = [
    None,
    Some(AgentsOverviewGroup::NeedsYou),
    Some(AgentsOverviewGroup::Working),
    Some(AgentsOverviewGroup::Ready),
    Some(AgentsOverviewGroup::Finished),
];

/// One root task (oracle: `AgentsOverviewRow`).
#[derive(Clone, Debug, PartialEq)]
pub struct RowData {
    pub thread: Thread,
    pub group: AgentsOverviewGroup,
    pub unread: bool,
    pub current: bool,
    pub pinned: bool,
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
    /// The roots in status, newest and id order, hidden ones left out
    /// (oracle: `agents_overview_view`).
    pub(super) fn rows(&self) -> Vec<RowData> {
        let mut children: HashMap<String, Vec<&Thread>> = HashMap::new();
        for thread in &self.threads {
            if let Some(parent) = &thread.parent_thread_id {
                children.entry(parent.clone()).or_default().push(thread);
            }
        }
        let mut roots = self
            .threads
            .iter()
            .filter(|thread| thread.parent_thread_id.is_none())
            .map(|root| (root, agents_overview_group(root, &children)))
            .collect::<Vec<_>>();
        roots.sort_by(|(left, left_group), (right, right_group)| {
            left_group
                .cmp(right_group)
                .then_with(|| right.updated_at.cmp(&left.updated_at))
                .then_with(|| left.id.cmp(&right.id))
        });
        roots
            .into_iter()
            .filter(|(root, _)| !self.hidden.contains(&root.id))
            .map(|(root, group)| RowData {
                thread: root.clone(),
                group,
                unread: self.unread.contains(&root.id),
                current: self.current.as_deref() == Some(root.id.as_str()),
                pinned: self
                    .pinned
                    .as_ref()
                    .is_some_and(|pins| pins.contains(&root.id)),
                details: self.details(root, &children),
            })
            .collect()
    }

    fn pin_rank(&self, id: &str) -> Option<usize> {
        self.pinned
            .as_ref()
            .and_then(|pins| pins.iter().position(|pin| pin == id))
    }

    /// The rows the filters keep, pinned first, in grouping order (oracle:
    /// `visible_indices`).
    fn visible_indices(&self, rows: &[RowData]) -> Vec<usize> {
        let search = self.search.to_lowercase();
        let status_group = FILTER_GROUPS[self.filter.min(FILTER_GROUPS.len() - 1)];
        let renaming = self.rename.as_ref().map(|rename| rename.target.as_str());
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
            Grouping::Project | Grouping::Directory => visible.sort_by_key(|index| {
                (
                    rows[*index].thread.cwd.display().to_string(),
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

    fn same_group(&self, rows: &[RowData], left: usize, right: usize) -> bool {
        let (l, r) = (&rows[left], &rows[right]);
        match self.grouping {
            Grouping::Status => l.group == r.group,
            _ if l.pinned || r.pinned => l.pinned && r.pinned,
            Grouping::Project | Grouping::Directory => l.thread.cwd == r.thread.cwd,
            Grouping::Model => model_name(&l.thread) == model_name(&r.thread),
        }
    }

    pub(super) fn more_available(&self) -> bool {
        self.paging.has_more && !self.searching
    }

    /// Visible rows plus the "Show more" slot (`usize::MAX`).
    fn selectable(&self, rows: &[RowData]) -> Vec<usize> {
        let mut indices = self.visible_indices(rows);
        if self.more_available() {
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
            Grouping::Project | Grouping::Directory => GroupLabel::Path(row.thread.cwd.display().to_string()),
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

    /// Everything the view draws.
    pub fn center(&self) -> Center {
        let rows = self.rows();
        let indices = self.selectable(&rows);
        let entries = self.entries(&rows, &indices);
        let mut tab_counts = [0; 5];
        for (slot, group) in FILTER_GROUPS.iter().enumerate() {
            tab_counts[slot] = rows
                .iter()
                .filter(|row| group.is_none_or(|group| group == row.group))
                .count();
        }
        let selected = self.selected_index(&rows, &indices);
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
    fn selected_index(&self, rows: &[RowData], indices: &[usize]) -> Option<usize> {
        let wanted = match &self.selected {
            Some(Sel::Task(id)) => rows.iter().position(|row| row.id() == id),
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

    /// Pins `selected` to what the view shows, so that a later change of rows
    /// moves from there.
    pub(super) fn reconcile(&mut self) {
        let rows = self.rows();
        let indices = self.selectable(&rows);
        self.selected = self
            .selected_index(&rows, &indices)
            .map(|index| match rows.get(index) {
                Some(row) => Sel::Task(row.id().to_owned()),
                None => Sel::ShowMore,
            });
    }

    pub(super) fn move_selection(&mut self, forward: bool) {
        if self.rename.is_some() {
            return;
        }
        let rows = self.rows();
        let indices = self.selectable(&rows);
        if indices.is_empty() {
            return;
        }
        let current = self.selected_index(&rows, &indices);
        let position = indices
            .iter()
            .position(|index| Some(*index) == current)
            .unwrap_or(0);
        let next = if forward {
            indices[(position + 1) % indices.len()]
        } else {
            indices[position.checked_sub(1).unwrap_or(indices.len() - 1)]
        };
        self.set_slot(&rows, next);
    }

    fn set_slot(&mut self, rows: &[RowData], slot: usize) {
        self.selected = Some(match rows.get(slot) {
            Some(row) => Sel::Task(row.id().to_owned()),
            None => Sel::ShowMore,
        });
    }

    pub(super) fn jump(&mut self, top: bool) {
        if self.rename.is_some() {
            return;
        }
        let rows = self.rows();
        let visible = self.visible_indices(&rows);
        let slot = if top { visible.first() } else { visible.last() };
        if let Some(slot) = slot.copied() {
            self.set_slot(&rows, slot);
        }
    }

    /// Page navigation shares the rendered layout, headings included, and
    /// does not wrap (oracle: `page_selection`).
    pub(super) fn page(&mut self, forward: bool, height: usize) {
        let rows = self.rows();
        let indices = self.selectable(&rows);
        if self.rename.is_some() || indices.is_empty() {
            return;
        }
        let entries = self.entries(&rows, &indices);
        let tasks = entries
            .iter()
            .enumerate()
            .filter_map(|(position, row)| match row {
                CenterRow::Task(index) => Some((position, *index)),
                CenterRow::ShowMore => Some((position, usize::MAX)),
                _ => None,
            })
            .collect::<Vec<_>>();
        let selected = self.selected_index(&rows, &indices);
        let current = tasks
            .iter()
            .find(|(_, index)| Some(*index) == selected)
            .map(|(position, _)| *position)
            .unwrap_or(tasks[0].0);
        let height = height.max(1);
        let target = if forward {
            current.saturating_add(height)
        } else {
            current.saturating_sub(height)
        };
        let picked = if forward {
            tasks
                .partition_point(|(position, _)| *position < target)
                .min(tasks.len() - 1)
        } else {
            tasks
                .partition_point(|(position, _)| *position <= target)
                .saturating_sub(1)
        };
        self.set_slot(&rows, tasks[picked].1);
    }

    /// The task after `removed` rows leave: the next displayed one, else the
    /// one before (oracle: `selection_after_removal`).
    pub fn selection_after_removal(&self, removed: &[String]) -> Option<String> {
        let rows = self.rows();
        let visible = self.visible_indices(&rows);
        let selected = self.selected_index(&rows, &visible)?;
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
