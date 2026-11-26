//! Exploration groups (oracle `exec_cell/model.rs` `ExecCell::is_exploring_cell`, `add_call`,
//! `append_completed`, `mark_failed`; `chatwidget/activity_groups.rs`): the read, list and search
//! commands of a turn that follow each other are one group, `Exploring` while a call runs and
//! `Explored` after. The group is data over the transcript, found by [`Transcript::exploration_groups`]
//! as the cells are, never stored: a late `item/completed`, a replayed turn and an older page
//! that joins all give the groups the live turn had.
//!
//! The rules, the oracle's:
//! - a call is a command of the model whose every parsed action reads, lists or searches
//!   ([`Item::group`]); a command of the user's own shell or with an action of no known kind is
//!   none, and ends the run;
//! - the run is of one turn; any entry that is not a call or a detail (a message, another tool,
//!   a patch) ends it;
//! - the reasoning summary and the writes to a background terminal are details: they stay
//!   inside the group so they do not split it, and keep their place between the calls
//!   ([`Detail::after`]) for the detailed transcript;
//! - a call that failed (non-zero exit, or the command did not run) stays in the group and stays
//!   visible, so does one still running while another ended;
//! - an interrupted or failed turn ends its running commands as failed ([`Transcript::fail_running`],
//!   oracle `finalize_active_cell_as_failed`), but never a background terminal's own command.

use std::sync::Arc;

use super::{Item, ItemKey, Notice, ToolKind, ToolStatus, Transcript, TurnId};
use std::rc::Rc;

/// How one call of a group stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The command runs.
    Running,
    /// It ended with exit code 0.
    Ok,
    /// It ended with a non-zero exit code, or did not run (oracle `CommandOutput::from_execution`:
    /// a status other than completed is exit code 1 unless the server said another).
    Failed { exit_code: i32 },
}

/// An entry inside a group that is no call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Detail {
    /// The transcript index.
    pub index: usize,
    /// How many calls of the group came before it (oracle `ActivityDetails::push(calls.len())`).
    pub after: usize,
}

/// One call of a group.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Call {
    /// The transcript index.
    pub index: usize,
    pub outcome: Outcome,
}

/// A run of exploration calls that is one cell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExplorationGroup {
    /// The first and the one after the last transcript index the group holds (a trailing
    /// detail is inside).
    pub start: usize,
    pub end: usize,
    pub calls: Vec<Call>,
    pub details: Vec<Detail>,
}

impl ExplorationGroup {
    /// A call still runs: the cell reads `Exploring`, else `Explored` (oracle `is_active`).
    pub fn active(&self) -> bool {
        self.calls
            .iter()
            .any(|call| call.outcome == Outcome::Running)
    }

    /// The calls that did not end with exit code 0 and are not running.
    pub fn failures(&self) -> impl Iterator<Item = &Call> {
        self.calls
            .iter()
            .filter(|call| matches!(call.outcome, Outcome::Failed { .. }))
    }
}

/// An entry the group keeps inside without it being a call (oracle `absorb_activity_detail`:
/// the hidden reasoning summary and the interaction of a background terminal).
fn is_detail(item: &Item) -> bool {
    matches!(item, Item::Reasoning { .. })
        || matches!(item, Item::Notice(Notice::TerminalInteraction { .. }))
}

fn outcome(item: &Item) -> Option<Outcome> {
    let Item::Tool(tool) = item else {
        return None;
    };
    let ToolKind::Shell {
        actions,
        exit_code,
        user_shell: false,
        ..
    } = &tool.kind
    else {
        return None;
    };
    if actions.is_empty() {
        return None;
    }
    Some(match tool.status {
        ToolStatus::Running => Outcome::Running,
        ToolStatus::Completed => match exit_code.unwrap_or_default() {
            0 => Outcome::Ok,
            exit_code => Outcome::Failed { exit_code },
        },
        ToolStatus::Failed | ToolStatus::Declined => Outcome::Failed {
            exit_code: exit_code.filter(|code| *code != 0).unwrap_or(1),
        },
    })
}

impl Transcript {
    /// The exploration groups, in order. A run of one call is a group too: the view draws
    /// `Exploring`/`Explored` with the one call (oracle: the cell of a first call).
    pub fn exploration_groups(&self) -> Vec<ExplorationGroup> {
        let mut groups: Vec<ExplorationGroup> = Vec::new();
        // the open group, and the turn it is of
        let mut open: Option<(ExplorationGroup, &TurnId)> = None;
        for (index, key) in self.order.iter().enumerate() {
            let Some(item) = self.item(key) else {
                continue;
            };
            match (outcome(item), &mut open) {
                (Some(outcome), Some((group, turn))) if **turn == key.turn => {
                    group.calls.push(Call { index, outcome });
                    group.end = index + 1;
                }
                (Some(outcome), _) => {
                    groups.extend(open.take().map(|(group, _)| group));
                    open = Some((
                        ExplorationGroup {
                            start: index,
                            end: index + 1,
                            calls: vec![Call { index, outcome }],
                            details: Vec::new(),
                        },
                        &key.turn,
                    ));
                }
                (None, Some((group, turn))) if is_detail(item) && **turn == key.turn => {
                    group.details.push(Detail {
                        index,
                        after: group.calls.len(),
                    });
                    group.end = index + 1;
                }
                (None, _) => groups.extend(open.take().map(|(group, _)| group)),
            }
        }
        groups.extend(open.map(|(group, _)| group));
        groups
    }

    /// The turn ended without its commands: each that still runs is a failed one, exit code 1 if
    /// the server said none (oracle `ExecCell::mark_failed`). A background terminal's own
    /// command is left running: the terminal outlives the turn.
    pub(super) fn fail_running(&mut self, turn: &TurnId) {
        let running: Vec<ItemKey> = self
            .order
            .iter()
            .filter(|key| &key.turn == turn)
            .filter(|key| {
                matches!(self.item(key), Some(Item::Tool(tool))
                    if tool.status == ToolStatus::Running
                        && !matches!(&tool.kind, ToolKind::Shell { unified_exec: true, .. }))
            })
            .cloned()
            .collect();
        for key in running {
            let Some(item) = Rc::make_mut(&mut self.items).get_mut(&key) else {
                continue;
            };
            let Item::Tool(tool) = Rc::make_mut(&mut item.item) else {
                continue;
            };
            let tool = Arc::make_mut(tool);
            if tool.interrupt() {
                continue;
            }
            tool.status = ToolStatus::Failed;
            if let ToolKind::Shell { exit_code, .. } = &mut tool.kind {
                *exit_code = Some(exit_code.filter(|code| *code != 0).unwrap_or(1));
            }
            self.item_revision += 1;
            self.activity_revision += 1;
            item.revision = self.item_revision;
        }
    }
}
