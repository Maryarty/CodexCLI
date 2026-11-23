//! The text contract of the cells: what the app keeps of an item for rtwk's
//! selection over a list (`ItemTexts`, `ListView::selection_text`), find
//! and copy of items that are not mounted.
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
//! reads nothing of what is scrolled out. rtwk's `ItemTexts` is the app's
//! model of it, and a model drifts from the widgets, so each cell keeps its
//! `texts` beside its `build` and the test is the guard.
//!
//! A cell whose text is made by a rtwk widget (Markdown, a diff) has no text
//! here yet, and says [`Gap`]: the rows of the gap are in the pack.

use codex_tui_core::threads::{Item, Notice};

use crate::pages::session::strings::Strings as Chat;

use super::strings::Strings;
use super::{
    deprecation, error, generated_image, hook, note, notice, session_info, user, viewed_image,
    warning, web,
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

/// One `Text` of a cell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Part {
    pub text: String,
    /// The text is a gutter (a bullet, the prompt mark, a `└` branch, the
    /// disclosure control): not part of what a selection copies, once the
    /// cell's rtwk widget leaves it out ([`GUTTERS_SELECTED`]).
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

/// Whether a `SelectionArea` over the mounted cell selects the gutters. It
/// does while the rtwk pin has no `ExcludeSelection` (rtwk `d21d7a13`, after
/// the pin 5583): the cells wrap their gutters in it at the repin, this turns
/// `false`, and `text_contract` then holds `Texts::joined(false)` to the
/// selection. What the app keeps as the text of a mounted item is
/// `joined(GUTTERS_SELECTED)`.
pub const GUTTERS_SELECTED: bool = true;

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
    /// The cell's rows are not yet written down (the tool, patch, plan and
    /// collab cells of the chunk's next steps).
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

    /// What `ItemTexts::text` answers for the item as it is mounted.
    pub fn as_mounted(&self) -> Option<String> {
        self.joined(GUTTERS_SELECTED)
    }
}

fn one(text: String) -> Texts {
    Texts::Exact(vec![Part::text(text)])
}

/// The texts of `item`.
pub fn texts(item: &Item, env: &Env<'_>) -> Texts {
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
        Item::Notice(notice) => Texts::Exact(notice::texts(notice, cells)),
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
        Item::Tool(_)
        | Item::Patch(_)
        | Item::PlanUpdate(_)
        | Item::Collab(_)
        | Item::Status { .. }
        | Item::Decision(_)
        | Item::Review(_) => Texts::Gap(Gap::Rows),
    }
}
