//! The text contract of the cells: what the app keeps of an item for the
//! selection over the transcript (rtwk's `ListView::item_text` gives each item
//! one line of it, ADR-029), find and copy of items that are not mounted.
//!
//! The text of an item is the strings of the `Text`s its cell builds, in the
//! order of the tree (what a `SelectionArea` over the mounted cell selects,
//! the parts joined by line breaks). A gutter that is a `Text` of its own (a
//! bullet, the prompt mark, a `└` branch, the disclosure control) is an
//! `ExcludeSelection` in the cell and not in the text; a mark that is the
//! first span of a row's own `Text` is part of that row's text. The test
//! `text_contract` in `tests/pages/cells.rs` mounts each cell and holds the
//! two equal.
//!
//! Flutter has no such thing: `SelectableRegion` selects what is built, and
//! reads nothing of what is scrolled out. twk's `ListView(itemText:)` is the
//! app's model of it (one line per item, the rows an item draws are its
//! own), and a model drifts from the widgets, so each cell keeps its `texts`
//! beside its `build` and the test is the guard.
//!
//! Markdown is read by the transcript's Markdown projection. For other
//! rows the cell supplies the text in its mounted order.

use codex_tui_core::threads::{Item, Notice, ToolActivity, ToolKind};

use crate::pages::session::strings::Strings as Chat;

use super::strings::Strings;
use super::{
    collab, deprecation, error, exec, generated_image, hook, note, notice, patch, plan_update,
    session_info, user, viewed_image, warning, web,
};

/// What the texts are read with: the tables, and the two things of the
/// transcript's view state a cell's rows depend on.
pub struct Env<'a> {
    pub cells: &'a Strings,
    pub chat: &'a Chat,
    /// The detailed transcript (Ctrl+T) draws more rows.
    pub detailed: bool,
    /// The cell's disclosure is open (a tap, Enter or Space on its control).
    pub expanded: bool,
}

/// Available columns and the same segmenter the mounted cells use.
#[derive(Clone, Copy)]
pub struct Room {
    pub width: i32,
    pub segmenter: crate::widgets::measure::Measure,
}

impl Default for Room {
    fn default() -> Self {
        Self {
            width: 80,
            segmenter: crate::widgets::measure::Measure::default(),
        }
    }
}

impl Room {
    pub(super) fn after(self, prefix: &str) -> Self {
        Self {
            width: self
                .width
                .saturating_sub(i32::try_from(self.segmenter.width(prefix)).unwrap_or(i32::MAX)),
            ..self
        }
    }

    pub(super) fn cuts(self, value: &str) -> bool {
        self.segmenter.width(value) > usize::try_from(self.width).unwrap_or_default()
    }
}

pub(crate) type CardFigure<'a> =
    &'a dyn Fn(u64) -> Option<Option<codex_tui_core::screens::status::Figure>>;

/// Context a cell's text reads from the chat and its measured room.
pub(crate) struct ReadContext<'a> {
    pub cwd: Option<&'a str>,
    pub room: Room,
    pub status: &'static super::status::strings::Strings,
    pub approvals: &'static crate::sheets::approvals::strings::Strings,
    pub usage: Option<CardFigure<'a>>,
}

impl Default for ReadContext<'_> {
    fn default() -> Self {
        Self {
            cwd: None,
            room: Room::default(),
            status: &super::status::strings::EN,
            approvals: &crate::sheets::approvals::strings::EN,
            usage: None,
        }
    }
}

/// One `Text` of a cell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Part {
    pub text: String,
    /// The text is a gutter (a bullet, the prompt mark, a `└` branch, the
    /// disclosure control): not part of what a selection copies, the cell's
    /// widget puts it in an `ExcludeSelection`.
    pub gutter: bool,
}

impl Part {
    pub fn text(text: impl Into<String>) -> Part {
        Part {
            text: text.into(),
            gutter: false,
        }
    }

    /// A blank row between two texts of the cell: the selection puts a blank
    /// line where rows lie between one text and the next.
    pub fn gap() -> Part {
        Part::text("")
    }

    pub fn gutter(text: impl Into<String>) -> Part {
        Part {
            text: text.into(),
            gutter: true,
        }
    }
}

/// The texts of one item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Texts {
    /// One part per `Text` of the cell, in the order of the tree.
    Exact(Vec<Part>),
    /// The cell's text is made by a rtwk widget the app cannot read before it
    /// is mounted.
    Gap(Gap),
}

/// Why a cell has no [`Texts::Exact`] yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gap {
    /// The body is a `MarkdownView`: a column of `Text`s whose strings rtwk
    /// gives no function of the source (rtwk-gaps G-new, "text of a Markdown
    /// document that is not mounted").
    Markdown,
    /// A tool row has no matching projected cell.
    Rows,
}

impl Texts {
    /// The parts joined by line breaks, the gutters in it or not.
    pub fn joined(&self, gutters: bool) -> Option<String> {
        match self {
            Texts::Exact(parts) => Some(
                parts
                    .iter()
                    .filter(|part| gutters || !part.gutter)
                    .map(|part| part.text.as_str())
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
            Texts::Gap(_) => None,
        }
    }

    /// What a `SelectionArea` over the item as it is mounted selects: the
    /// gutters are in an `ExcludeSelection` (rtwk `d21d7a13`), so not in it.
    pub fn as_mounted(&self) -> Option<String> {
        self.joined(false)
    }
}

fn one(text: String) -> Texts {
    Texts::Exact(vec![Part::text(text)])
}

/// The texts of `item`.
pub fn texts(item: &Item, env: &Env<'_>) -> Texts {
    texts_at(item, env, &ReadContext::default())
}

pub(crate) fn texts_at(item: &Item, env: &Env<'_>, cx: &ReadContext<'_>) -> Texts {
    let cells = env.cells;
    match item {
        Item::User(prompt) => user::texts(prompt, env.chat, cells),
        Item::Error { message } => one(error::line(message, cells)),
        Item::HighLoad => one(error::line(env.chat.high_load, cells)),
        Item::Warning { message } => Texts::Exact(warning::texts(message, cells)),
        Item::LocalWarning(warning) => {
            Texts::Exact(warning::texts(env.chat.local_warning(*warning), cells))
        }
        Item::StartupWarning(warning) => {
            Texts::Exact(warning::texts(&env.chat.mcp_warning(warning), cells))
        }
        Item::Deprecation { summary, details } => Texts::Exact(
            deprecation::texts(summary, details.as_deref(), cells)
                .into_iter()
                .map(Part::text)
                .collect(),
        ),
        Item::Notice(Notice::SessionInfo(info)) if session_info::has_markdown(info) => {
            Texts::Gap(Gap::Markdown)
        }
        Item::Notice(notice) => Texts::Exact(notice::texts(notice, cells, env.detailed)),
        Item::Note(note) => Texts::Exact(note::texts(note, env.chat, cells)),
        Item::Hook(run) => Texts::Exact(
            hook::texts(run, cells)
                .into_iter()
                .map(Part::text)
                .collect(),
        ),
        Item::Generated(image) => Texts::Exact(
            generated_image::texts(image, cells)
                .into_iter()
                .map(Part::text)
                .collect(),
        ),
        Item::Web(activity) => Texts::Exact(web::texts(activity, cells, env.detailed)),
        Item::ImageView { path, name } => Texts::Exact(viewed_image::texts(
            name,
            path,
            env.detailed,
            env.expanded,
            cells,
        )),
        Item::Agent { .. } | Item::Reasoning { .. } | Item::Plan { .. } | Item::Delegated(_) => {
            Texts::Gap(Gap::Markdown)
        }
        Item::PlanUpdate(update) => Texts::Exact(plan_update::texts(
            update,
            env.detailed,
            env.expanded,
            cells,
        )),
        Item::Collab(event) => Texts::Exact(collab::texts(event, cells)),
        Item::Tool(tool) => {
            tool_texts(&[tool.as_ref()], env, cx.room).map_or(Texts::Gap(Gap::Rows), Texts::Exact)
        }
        Item::Patch(activity) => Texts::Exact(patch::texts(activity, cx.cwd, env)),
        Item::Status { card, .. } => {
            let held = cx.usage.and_then(|figure| figure(card.number));
            Texts::Exact(super::status::StatusCell::texts(
                card,
                held.as_ref().map(Option::as_ref),
                cx.status,
                cx.room,
            ))
        }
        Item::Decision(decided) => Texts::Exact(
            crate::sheets::approvals::DecisionCell::texts(decided, cx.approvals)
                .into_iter()
                .map(Part::text)
                .collect(),
        ),
        Item::Review(line) => one(crate::sheets::approvals::ReviewLineCell::text(
            line,
            cx.approvals,
        )),
    }
}

/// The text of the first call and the tool calls its row draws with it.
pub(crate) fn tool_texts(calls: &[&ToolActivity], env: &Env<'_>, room: Room) -> Option<Vec<Part>> {
    let first = *calls.first()?;
    match &first.kind {
        ToolKind::Shell { .. } => exec::texts_at(calls, env, room),
        ToolKind::Mcp(_) if first.is_computer_activity() => {
            let group: Vec<_> = calls
                .iter()
                .filter_map(|tool| match &tool.kind {
                    ToolKind::Mcp(call) if call.is_computer_activity() => Some(call),
                    _ => None,
                })
                .collect();
            Some(super::computer_activity::texts(&group, env, room))
        }
        ToolKind::Mcp(call) => Some(super::tool_call::mcp_texts(call, env, room)),
        ToolKind::Dynamic(call) => Some(super::tool_call::dynamic_texts(call, env, room)),
    }
}
