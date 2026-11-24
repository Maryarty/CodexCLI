//! The text of an item of the transcript as it is mounted, written from the
//! item: what find, the select mode and copy read of an item that is scrolled
//! out of the list (what the list's `ListView::item_text` is given).
//!
//! The cells keep their own texts (`cells::text`); a Markdown body is the
//! `Gap` they declare, and this file fills it with [`markdown_text`] over the
//! blocks the cell draws (`citation::blocks_of`, `widgets::markdown::view`):
//! the assistant's message, a prompt with markup, the reasoning summary, the proposed plan, a
//! delegated message and the session card's tip. What is still a gap (a tool
//! call, a patch, a status or a decision, which draw by room) answers `None`: such an
//! item has no text here, and a reader of this function treats it as it does
//! an item that is not selectable.

use codex_tui_core::app::facts::Rendering;
use codex_tui_core::threads::{Item, Notice};
use rtwk_rich::markdown::{Block, MarkdownView};

use super::markdown_text;
use crate::pages::session::cells::citation::blocks_of;
use crate::pages::session::cells::text::{Env, Gap, Part, Texts, texts};
use crate::pages::session::cells::{raw, session_info};

/// What the texts are read with: the cells' own `Env`, and what the cells that hold Markdown draw
/// it with (the session's directory, the user's `tui.rendering`, raw output mode, raw reasoning).
pub struct Reading<'a> {
    pub env: &'a Env<'a>,
    /// The assistant's bullet: the first part of its message.
    pub bullet: &'a str,
    pub cwd: Option<&'a str>,
    pub rendering: Rendering,
    /// Raw output mode: the items that have a raw form draw as their source (`cells::raw`).
    pub raw: bool,
    /// `show_raw_agent_reasoning`: a reasoning block also draws the raw reasoning.
    pub raw_reasoning: bool,
}

impl<'a> Reading<'a> {
    /// The reading of a transcript in the rich mode, the defaults of the config.
    pub fn new(env: &'a Env<'a>, bullet: &'a str) -> Reading<'a> {
        Reading {
            env,
            bullet,
            cwd: None,
            rendering: Rendering::default(),
            raw: false,
            raw_reasoning: false,
        }
    }
}

/// The text of `item` as a `SelectionArea` over its mounted cell selects it:
/// the marks the cells draw as gutters (the assistant's bullet too) are in an
/// `ExcludeSelection` and not in it.
pub fn item_text(item: &Item, reading: &Reading<'_>) -> Option<String> {
    mounted_parts(item, reading).map(|parts| {
        parts
            .iter()
            .filter(|part| !part.gutter)
            .map(|part| part.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    })
}

/// The text of `item` as rtwk's `SelectableList` reads its mounted row: every `Text` below the
/// row, the gutters in their `ExcludeSelection` too (`selectable_nodes` of `selection_text.rs`
/// walks the render tree without leaving an `ExcludeSelection` out as the area does), the parts
/// joined by line breaks. What the list's `ItemTexts` model has to say for a range of it to land
/// on the cells it names; `item_text` is the text that is the item's own (copy).
pub fn mounted_text(item: &Item, reading: &Reading<'_>) -> Option<String> {
    mounted_parts(item, reading).map(|parts| {
        parts
            .iter()
            .map(|part| part.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    })
}

/// The parts of [`mounted_text`], each with whether it is a gutter: what the select mode reads
/// (a caret never stops on a gutter, a copy leaves it out).
pub fn mounted_parts(item: &Item, reading: &Reading<'_>) -> Option<Vec<Part>> {
    let env = reading.env;
    if reading.raw
        && let Some(lines) = raw::lines(item, env.cells)
    {
        // the raw cell is a `Text` per line, a blank line a `Text` of the gap
        return Some(
            lines
                .into_iter()
                .map(|line| {
                    if line.is_empty() {
                        Part::text(env.cells.gap)
                    } else {
                        Part::text(line)
                    }
                })
                .collect(),
        );
    }
    match texts(item, env) {
        Texts::Exact(parts) => Some(parts),
        Texts::Gap(Gap::Markdown) => markdown_parts(item, reading),
        Texts::Gap(Gap::Rows) => None,
    }
}

/// The parts of an item whose body is Markdown: its gutters and `Text`s, the body a part of its
/// own (the blocks as the cell draws them).
fn markdown_parts(item: &Item, reading: &Reading<'_>) -> Option<Vec<Part>> {
    let env = reading.env;
    let strings = env.cells;
    match item {
        // a prompt with markup: the labels of its images, a blank row, the prompt mark (a gutter)
        // and the body as the Markdown view draws it (`cells::user::texts` leaves it a gap)
        Item::User(prompt) => {
            let shown = prompt.shown();
            let body = drawn(crate::widgets::markdown::view(shown.text.trim_end()))?;
            let mut parts: Vec<Part> = prompt
                .images
                .iter()
                .map(|number| Part::text(strings.label(strings.image_label, &number.to_string())))
                .collect();
            if !parts.is_empty() {
                parts.push(Part::gap());
            }
            parts.push(Part::gutter(env.chat.prompt.trim_end()));
            parts.push(Part::text(body));
            Some(parts)
        }
        Item::Agent { text, .. } => {
            let body = body_of(text, reading)?;
            Some(vec![Part::gutter(reading.bullet), Part::text(body)])
        }
        // the reasoning cell draws nothing for a body with nothing to say
        Item::Reasoning { .. } => {
            let summary = item.reasoning_text(reading.raw_reasoning);
            if summary.trim().is_empty() {
                return None;
            }
            let body = drawn(crate::widgets::markdown::view(summary))?;
            Some(vec![Part::gutter(strings.bullet), Part::text(body)])
        }
        Item::Delegated(message) => {
            let attribution = strings.label(strings.delegated_attribution, &message.source);
            let body = drawn(crate::widgets::markdown::view(&message.markdown))?;
            Some(vec![
                Part::gutter(strings.bullet),
                Part::text(attribution),
                Part::text(body),
            ])
        }
        // the header is one `Text` (the bullet is a span of it), a blank `Text` and the plan on
        // its surface, a blank row of padding above it
        Item::Plan { text } => {
            let body = if text.trim().is_empty() {
                strings.empty_plan.to_owned()
            } else {
                body_of(text, reading)?
            };
            Some(vec![
                Part::text([strings.bullet, strings.proposed_plan].concat()),
                Part::text(strings.gap),
                Part::gap(),
                Part::text(body),
            ])
        }
        Item::Notice(Notice::SessionInfo(info)) if session_info::has_markdown(info) => {
            let tip = info.tip.as_deref()?;
            let tip = strings.fill(strings.tip_label, &[tip]);
            let mut parts = vec![Part::text(drawn(crate::widgets::markdown::view(&tip))?)];
            let changed = session_info::texts(info, strings);
            if !changed.is_empty() {
                // the card's parts are a blank row apart
                parts.push(Part::gap());
                parts.extend(changed);
            }
            Some(parts)
        }
        _ => None,
    }
}

/// The blocks the cell of a message or a plan draws, whose code and quotes the copy mode selects.
pub fn drawn_blocks(item: &Item, reading: &Reading<'_>) -> Vec<Block> {
    match item {
        Item::Agent { text, .. } | Item::Plan { text } => blocks_of(
            text,
            false,
            reading.cwd,
            reading.env.cells,
            reading.rendering,
        ),
        _ => Vec::new(),
    }
}

/// The text of the blocks the cell of a message draws (`citation::cited`).
fn body_of(source: &str, reading: &Reading<'_>) -> Option<String> {
    let blocks = blocks_of(
        source,
        false,
        reading.cwd,
        reading.env.cells,
        reading.rendering,
    );
    markdown_text::drawn(&blocks)
}

/// The text of a view's blocks as drawn.
fn drawn(view: MarkdownView) -> Option<String> {
    let blocks: Vec<Block> = view.blocks.map(|blocks| blocks.to_vec())?;
    markdown_text::drawn(&blocks)
}
