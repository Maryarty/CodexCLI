//! A finished turn's details behind disclosure rows (oracle `transcript_view/turn_details.rs`,
//! on when `collapse_turn_details` is set): the tool calls of a turn fold into one row per run
//! (`▸ Ran 2 commands`), and the commentary and calls before the final answer fold behind the
//! prompt (`▸ 3 previous messages`, or the turn's `Worked for 2s`).
//!
//! The planner reads the transcript's items, not their text: which rows a turn has is a function
//! of the item kinds, the message phases and whether the turn completed, so the plan is cached
//! with the rows. The words (a call's label, the counts of a group) are read when a row is drawn.
//! Flutter: an `ExpansionTile` per run, the expanded set held by the list's state
//! (`ExpansionTileController`) and keyed by item, so a group that grows keeps its state.

use std::collections::HashSet;
use std::ops::Range;
use std::path::Path;

use rtwk::Handler;
use rtwk::controls::Pressable;
use rtwk::layout::{Column, CrossAxisAlignment, EdgeInsets, MainAxisSize, Padding};
use rtwk::style::Style;
use rtwk::text::TextOverflow;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::threads::{
    ExplorationAction, Item, ItemKey, MessagePhase, ToolKind, ToolStatus, Transcript, WebActivity,
};

use super::super::cells::exec::command_text;
use super::super::cells::strings::Strings as Cells;
use super::super::cells::web;
use super::strings::Strings;

/// What a transcript index is in the folded view.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fold {
    /// Drawn as the cell it is.
    Plain,
    /// Part of another row: a call of a group (the group's first call draws it), or the
    /// separator of a turn folded behind its prompt.
    Hidden,
    /// A prompt with earlier messages behind it.
    Prompt(PromptFold),
    /// The first call of a run of calls.
    Tools(ToolGroup),
}

/// The fold of a prompt: its key (what the expanded set holds) and how many messages and calls
/// are behind it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptFold {
    pub id: ItemKey,
    pub hidden: usize,
    /// The last item of the turn: its timing names the fold when it has one.
    pub last: usize,
}

/// A run of calls: their transcript indices, and the keys that say it is open.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolGroup {
    pub members: Range<usize>,
    pub ids: Vec<ItemKey>,
}

#[derive(Clone, Debug)]
struct Entry {
    fold: Fold,
    /// The prompt this entry folds behind.
    parent: Option<usize>,
    /// The turn's timing row is part of its prompt's label now.
    folds_timing: bool,
}

/// The plan of a transcript, an entry per item.
#[derive(Clone, Debug, Default)]
pub struct Plan {
    entries: Vec<Entry>,
}

/// Which folds the reader opened: a fold is open when any of its keys is in the set (a group of
/// calls that gains a call stays open).
#[derive(Clone, Debug, Default)]
pub struct Unfolded {
    keys: HashSet<ItemKey>,
}

impl Unfolded {
    pub fn is_open(&self, ids: &[ItemKey]) -> bool {
        ids.iter().any(|id| self.keys.contains(id))
    }

    /// Opens a closed fold, closes an open one.
    pub fn toggle(&mut self, ids: &[ItemKey]) {
        if self.is_open(ids) {
            for id in ids {
                self.keys.remove(id);
            }
        } else {
            self.keys.extend(ids.iter().cloned());
        }
    }
}

/// How a row of the list is drawn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    Cell,
    Prompt(PromptFold),
    Tools(ToolGroup),
}

/// A row of the list: the transcript items it covers and how it is drawn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub range: Range<usize>,
    pub kind: Kind,
}

impl Plan {
    /// The plan of `transcript`: every prompt-bounded range of it, walked from its end so the
    /// answer and the completion are known before an earlier row folds.
    pub fn of(transcript: &Transcript) -> Plan {
        let len = transcript.len();
        let mut entries: Vec<Entry> = (0..len)
            .map(|_| Entry {
                fold: Fold::Plain,
                parent: None,
                folds_timing: false,
            })
            .collect();
        let mut start = 0;
        for end in (1..len)
            .filter(|&at| is_prompt(transcript, at))
            .chain([len])
        {
            if start < end {
                index_turn(transcript, start..end, &mut entries);
            }
            start = end;
        }
        Plan { entries }
    }

    pub fn fold(&self, index: usize) -> &Fold {
        self.entries
            .get(index)
            .map_or(&Fold::Plain, |entry| &entry.fold)
    }

    /// The rows of the list: an item is hidden when it is a later call of a group or sits
    /// behind a prompt the reader has not opened; `skip` holds back what the view never shows.
    pub fn rows(
        &self,
        transcript: &Transcript,
        open: &Unfolded,
        skip: impl Fn(usize) -> bool,
    ) -> Vec<Row> {
        let mut rows = Vec::new();
        for index in 0..transcript.len() {
            let Some(entry) = self.entries.get(index) else {
                continue;
            };
            if skip(index) || matches!(entry.fold, Fold::Hidden) {
                continue;
            }
            if let Some(parent) = entry.parent
                && let Some(Fold::Prompt(prompt)) = self.entries.get(parent).map(|e| &e.fold)
                && !open.is_open(std::slice::from_ref(&prompt.id))
            {
                continue;
            }
            rows.push(match &entry.fold {
                Fold::Prompt(prompt) => Row {
                    range: index..index + 1,
                    kind: Kind::Prompt(prompt.clone()),
                },
                Fold::Tools(group) => Row {
                    range: group.members.clone(),
                    kind: Kind::Tools(group.clone()),
                },
                _ => Row {
                    range: index..index + 1,
                    kind: Kind::Cell,
                },
            });
        }
        rows
    }

    /// The turn's timing is the label of its prompt's fold, not a row of its own.
    pub fn folds_timing(&self, index: usize) -> bool {
        self.entries
            .get(index)
            .is_some_and(|entry| entry.folds_timing)
    }
}

fn is_prompt(transcript: &Transcript, index: usize) -> bool {
    matches!(transcript.get(index), Some((_, Item::User(_))))
}

/// Whether the item is a tool call a group summarizes (oracle: `tool_call_labels` is not empty).
fn is_call(item: &Item) -> bool {
    match item {
        Item::Tool(tool) => !matches!(
            tool.kind,
            ToolKind::Shell {
                user_shell: true,
                ..
            }
        ),
        Item::Patch(_)
        | Item::PlanUpdate(_)
        | Item::Web(_)
        | Item::ImageView { .. }
        | Item::Generated(_) => true,
        _ => false,
    }
}

/// A run of calls met from its end: it becomes one row when a row that is no call (or the start
/// of the turn) ends it.
#[derive(Default)]
struct Run {
    /// Transcript indices, last first.
    members: Vec<usize>,
}

impl Run {
    fn push(&mut self, index: usize) {
        self.members.push(index);
    }

    fn finish(&mut self, transcript: &Transcript, entries: &mut [Entry]) {
        let mut members = std::mem::take(&mut self.members);
        members.reverse();
        let (Some(&first), Some(&last)) = (members.first(), members.last()) else {
            return;
        };
        for &member in &members[1..] {
            entries[member].fold = Fold::Hidden;
        }
        let ids = members
            .iter()
            .filter_map(|&at| transcript.get(at).map(|(key, _)| key.clone()))
            .collect();
        entries[first].fold = Fold::Tools(ToolGroup {
            members: first..last + 1,
            ids,
        });
    }
}

/// One prompt-bounded range of items (oracle `index_turn`).
fn index_turn(transcript: &Transcript, range: Range<usize>, entries: &mut [Entry]) {
    let start = range.start;
    let last = range.end - 1;
    let has_prompt = is_prompt(transcript, start);
    let completed = transcript
        .get(last)
        .is_some_and(|(key, _)| transcript.is_completed(&key.turn));
    // The turn's separator is after its last row: `separator` is whether it still waits to be
    // folded into the prompt's label.
    let mut separator = completed;
    let mut fold = false;
    let mut hidden = 0;
    let mut run = Run::default();
    for index in range.rev() {
        let Some((_, item)) = transcript.get(index) else {
            continue;
        };
        let phase = match item {
            Item::Agent { phase, .. } => phase.as_ref(),
            _ => None,
        };
        let message = matches!(item, Item::Agent { .. });
        let call = is_call(item);
        if phase == Some(&MessagePhase::FinalAnswer)
            || (!fold
                && separator
                && ((message && phase.is_none()) || matches!(item, Item::Plan { .. })))
        {
            // An answer item can finish before its turn succeeds.
            fold |= separator;
        } else if fold
            && phase != Some(&MessagePhase::PartialAnswer)
            && has_prompt
            && (call || message)
        {
            entries[index].parent = Some(start);
            hidden += 1;
            if separator {
                separator = false;
                entries[last].folds_timing = true;
            }
        }
        if call {
            run.push(index);
        } else {
            run.finish(transcript, entries);
        }
    }
    run.finish(transcript, entries);
    if hidden > 0
        && let Some((key, _)) = transcript.get(start)
    {
        entries[start].fold = Fold::Prompt(PromptFold {
            id: key.clone(),
            hidden,
            last,
        });
    }
}

/// The words a call's label is made of.
#[derive(Clone, Copy)]
pub struct Words<'a> {
    pub table: &'a Strings,
    pub cells: &'a Cells,
}

impl Words<'_> {
    /// `template` with its `{}` read as `text`.
    fn fill(&self, template: &str, text: &str) -> String {
        template.replace("{}", text)
    }
}

/// The label of one call (oracle `tool_call_labels`): what it did or is doing, without output.
pub fn call_label(item: &Item, words: Words<'_>) -> Option<String> {
    if !is_call(item) {
        return None;
    }
    let Words { table, cells } = words;
    Some(match item {
        Item::Tool(tool) => match &tool.kind {
            ToolKind::Shell {
                command,
                actions,
                exit_code,
                ..
            } => {
                let (running, finished, target) = match actions.as_slice() {
                    [ExplorationAction::Read(names)] if names.len() == 1 => {
                        (table.reading, table.read, names[0].clone())
                    }
                    [ExplorationAction::Search { query, path }] => (
                        table.searching_for,
                        table.searched_for,
                        match path {
                            Some(path) => [query, table.search_in, path].concat(),
                            None => query.clone(),
                        },
                    ),
                    [ExplorationAction::List(path)] => {
                        (table.listing_files_in, table.listed_files_in, path.clone())
                    }
                    _ => (table.running, table.ran, command_text(command).into_owned()),
                };
                let verb = if tool.status == ToolStatus::Running {
                    running
                } else {
                    finished
                };
                let mut label = format!("{verb} {target}");
                if let Some(code) = exit_code.filter(|&code| code != 0) {
                    label.push_str(&words.fill(table.exit_code, &code.to_string()));
                }
                label
            }
            ToolKind::Mcp(call) => {
                let verb = match call.success() {
                    Some(true) => cells.called_tool,
                    Some(false) => cells.failed_tool,
                    None => cells.calling_tool,
                };
                format!("{verb} {}", call.title())
            }
            ToolKind::Dynamic(call) => {
                let verb = match call.status {
                    ToolStatus::Running => cells.calling_tool,
                    ToolStatus::Completed => cells.called_tool,
                    ToolStatus::Failed | ToolStatus::Declined => cells.failed_tool,
                };
                let mut label = format!("{verb} {}", call.name);
                if let Some(millis) = call.duration_ms.filter(|millis| *millis >= 0) {
                    label.push_str(&cells.label(cells.tool_duration, &millis.to_string()));
                }
                label
            }
        },
        Item::Patch(patch) => {
            if patch.status == ToolStatus::Failed {
                table.patch_failed.to_owned()
            } else {
                match patch.changes.as_slice() {
                    [change] => {
                        let name = Path::new(&change.path)
                            .file_name()
                            .map_or(change.path.as_str(), |name| {
                                name.to_str().unwrap_or(change.path.as_str())
                            });
                        words.fill(table.edited_file, name)
                    }
                    _ => table.edited_files.to_owned(),
                }
            }
        }
        Item::PlanUpdate(_) => table.updated_plan.to_owned(),
        Item::Web(activity) => web::summary(activity, cells),
        Item::ImageView { path, .. } => words.fill(table.viewed_image, path),
        Item::Generated(image) => if image.failed {
            table.image_failed
        } else {
            table.image_generated
        }
        .to_owned(),
        _ => return None,
    })
}

impl ToolGroup {
    /// The labels of the calls, in order.
    pub fn calls(&self, transcript: &Transcript, words: Words<'_>) -> Vec<String> {
        self.members
            .clone()
            .filter_map(|index| transcript.get(index))
            .filter_map(|(_, item)| call_label(item, words))
            .collect()
    }

    /// A lone completed web search reads as the row it is, not as a group.
    pub fn is_lone_search(&self, transcript: &Transcript) -> bool {
        self.members.len() == 1
            && matches!(
                transcript.get(self.members.start),
                Some((
                    _,
                    Item::Web(WebActivity {
                        completed: true,
                        ..
                    })
                ))
            )
    }

    /// The row's label (oracle `ToolGroup::label`): the count of commands for a group that
    /// ran or explored only, else the last call's first line, with how many calls came before.
    pub fn label(&self, transcript: &Transcript, calls: &[String], words: Words<'_>) -> String {
        let table = words.table;
        let (mut explored, mut ran) = (0, 0);
        for (_, item) in self
            .members
            .clone()
            .filter_map(|index| transcript.get(index))
        {
            if let Item::Tool(tool) = item
                && let ToolKind::Shell {
                    actions,
                    exit_code: Some(0),
                    ..
                } = &tool.kind
                && is_call(item)
            {
                if actions.is_empty() {
                    ran += 1;
                } else {
                    explored += 1;
                }
            }
        }
        if calls.len() > 1 && explored + ran == calls.len() {
            let count = ran.to_string();
            return match (explored, ran) {
                (0, _) => words.fill(
                    if ran == 1 {
                        table.ran_command
                    } else {
                        table.ran_commands
                    },
                    &count,
                ),
                (_, 0) => table.explored_files.to_owned(),
                _ => words.fill(
                    if ran == 1 {
                        table.explored_ran_command
                    } else {
                        table.explored_ran_commands
                    },
                    &count,
                ),
            };
        }
        let mut lines = calls.last().into_iter().flat_map(|last| last.lines());
        let mut label = blank_controls(lines.next().unwrap_or_default());
        if lines.next().is_some() {
            label.push_str(table.label_more);
        }
        if calls.len() > 1 {
            return [
                words
                    .fill(table.tool_calls, &calls.len().to_string())
                    .as_str(),
                label.as_str(),
            ]
            .concat();
        }
        label
    }
}

/// Control characters read as blanks.
pub fn blank_controls(text: &str) -> String {
    text.replace(char::is_control, " ")
}

/// The label of a prompt's fold: the turn's own words when it has them, else how many messages
/// are behind it.
pub fn prompt_label(hidden: usize, timing: Option<String>, words: Words<'_>) -> String {
    timing.unwrap_or_else(|| match hidden {
        1 => words.table.previous_message.to_owned(),
        count => words.fill(words.table.previous_messages, &count.to_string()),
    })
}

/// Where a fold's control sits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Anchor {
    /// Under its prompt, after a blank row, at the left edge.
    Turn,
    /// First row of a run of calls, a quarter of the width in, at most four columns.
    Calls,
}

/// The arrow of a fold: open, closed, or a search group's.
pub fn arrow(open: bool, search: bool, words: Words<'_>) -> &'static str {
    let table = words.table;
    if open {
        table.fold_open
    } else if search {
        table.fold_search
    } else {
        table.fold_closed
    }
}

/// The columns an open run's calls are indented by (oracle: six spaces).
const CALL_INDENT: i32 = 6;

/// A disclosure row: the control, and below it the calls of an open run (Flutter: an
/// `ExpansionTile` header and children in one `Pressable`; a tap asks `toggled` for
/// the fold's keys).
#[derive(Clone, PartialEq)]
pub struct FoldRow {
    pub anchor: Anchor,
    pub control: String,
    /// The calls under an open run, one row each, six columns in.
    pub calls: Vec<String>,
    pub ids: Vec<ItemKey>,
    pub style: Style,
    /// How the control is drawn while the fold has the focus (F4).
    pub focus: Option<Style>,
    pub toggled: Handler<Vec<ItemKey>>,
}

impl_widget!(stateful eq FoldRow);

pub struct FoldState;

impl StatefulWidget for FoldRow {
    type State = FoldState;

    fn create_state(&self) -> FoldState {
        FoldState
    }
}

impl State for FoldState {
    type Widget = FoldRow;
    type Msg = ();

    fn update(&mut self, (): (), widget: &FoldRow, cx: &mut Cx<'_, Self>) -> Rebuild {
        cx.emit(&widget.toggled, widget.ids.clone());
        Rebuild::No
    }

    fn build(&self, widget: &FoldRow, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let control = widget.control.clone();
        let calls = widget.calls.clone();
        let style = widget.focus.clone().unwrap_or_else(|| widget.style.clone());
        let anchor = widget.anchor;
        let body: AnyWidget = LayoutBuilder::new(move |_, room| {
            let line = |text: &str, left: i32, top: i32, overflow: TextOverflow| -> AnyWidget {
                Padding::new(
                    EdgeInsets::only(left, top, 0, 0),
                    Text::new(blank_controls(text))
                        .style(style.clone())
                        .max_lines(Some(1))
                        .overflow(overflow),
                )
                .into()
            };
            let indent = (room.max_width / 4).min(4);
            let mut column = Column::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Stretch);
            column = match anchor {
                Anchor::Turn => column.child(line(&control, 0, 1, TextOverflow::Clip)),
                Anchor::Calls => column.child(line(&control, indent, 0, TextOverflow::Clip)),
            };
            for call in &calls {
                column = column.child(line(call, CALL_INDENT, 0, TextOverflow::Ellipsis));
            }
            column.into()
        })
        .into();
        // a control of the mouse: the composer keeps the keys, and the focus on a fold is the
        // transcript's own (`activity_focus`), whose Enter toggles it
        Pressable::new(move |_, _| body.clone())
            .can_request_focus(false)
            .on_tap(cx.map(|_| ()))
            .into()
    }
}
