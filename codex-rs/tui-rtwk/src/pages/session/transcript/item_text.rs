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
use codex_tui_core::threads::{Item, Notice, ToolActivity, Transcript};
use rtwk_rich::markdown::{Block, MarkdownView};

use super::markdown_text;
use super::turn_details::{Kind, Row};
use crate::pages::session::cells::citation::blocks_of;
use crate::pages::session::cells::text::{
    CardFigure, Env, Gap, Part, ReadContext, Room, Texts, texts_at, tool_texts,
};
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
    /// Find reads what a matched cell shows, not what its room cuts (oracle `search_layout`: the
    /// full content): a shell call is its expanded form. Select mode and copy mode keep the rows
    /// as the list draws them.
    pub revealed: bool,
    /// Width of the transcript's cells.
    pub room: Room,
    pub status_words: &'static crate::pages::session::cells::status::strings::Strings,
    pub approval_words: &'static crate::sheets::approvals::strings::Strings,
    pub(crate) usage: Option<CardFigure<'a>>,
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
            revealed: false,
            room: Room::default(),
            status_words: &crate::pages::session::cells::status::strings::EN,
            approval_words: &crate::sheets::approvals::strings::EN,
            usage: None,
        }
    }
}

/// One `Text` of a mounted item: the part, and what a copy puts before it when the text before it
/// is selected too.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Piece {
    pub part: Part,
    /// A line break where the cell's texts follow each other; the nodes of a Markdown body have
    /// the blank line the view puts between its blocks (`markdown_text::Node::before`).
    pub before: &'static str,
}

impl Piece {
    fn line(part: Part) -> Piece {
        Piece { part, before: "\n" }
    }

    /// The nodes of a Markdown body, the first joined to what comes before it by a line break.
    fn body(nodes: Vec<markdown_text::Node>) -> impl Iterator<Item = Piece> {
        nodes.into_iter().enumerate().map(|(at, node)| Piece {
            part: Part::text(node.text),
            before: if at == 0 { "\n" } else { node.before },
        })
    }
}

/// The text of `item` as a `SelectionArea` over its mounted cell selects it:
/// the marks the cells draw as gutters (the assistant's bullet too) are in an
/// `ExcludeSelection` and not in it; the blank lines between the blocks of a Markdown body are
/// in it, as the area copies them.
pub fn item_text(item: &Item, reading: &Reading<'_>) -> Option<String> {
    mounted_pieces(item, reading).map(|pieces| {
        let mut out = String::new();
        let mut first = true;
        for piece in pieces.iter().filter(|piece| !piece.part.gutter) {
            if !first {
                out.push_str(piece.before);
            }
            first = false;
            out.push_str(&piece.part.text);
        }
        out
    })
}

/// The text of `item` as rtwk's `SelectableList` reads its mounted row: every `Text` below the
/// row, the gutters in their `ExcludeSelection` too (`selectable_nodes` of `selection_text.rs`
/// walks the render tree without leaving an `ExcludeSelection` out as the area does), the texts
/// joined by line breaks and nothing else: the nodes' own texts are all it knows of the row, so a
/// range of it is a node and an offset in the node's text. What the list's `ItemTexts` model has
/// to say for a range of it to land on the cells it names; `item_text` is the text that is the
/// item's own (copy).
pub fn mounted_text(item: &Item, reading: &Reading<'_>) -> Option<String> {
    mounted_parts(item, reading).map(|parts| {
        parts
            .iter()
            .map(|part| part.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    })
}

/// The same reading with every disclosure open.
fn opened_env<'a>(env: &Env<'a>) -> Env<'a> {
    Env {
        cells: env.cells,
        chat: env.chat,
        detailed: env.detailed,
        expanded: true,
    }
}

/// The parts of the list row `row` of `transcript`, what [`mounted_parts`] says of its first item
/// and, for a run of tool calls, of all of them (the row draws them together: an exploration, a
/// computer group). A row that is a fold of a turn's calls draws that fold, whose text is not the
/// item's: no parts.
pub fn row_parts(transcript: &Transcript, row: &Row, reading: &Reading<'_>) -> Option<Vec<Part>> {
    if let Kind::Tools(group) = &row.kind
        && !group.is_lone_search(transcript)
    {
        return None;
    }
    let (_, item) = transcript.get(row.range.start)?;
    if !reading.raw && matches!(item, Item::Tool(_)) {
        let calls: Vec<&ToolActivity> = row
            .range
            .clone()
            .filter_map(|at| match transcript.get(at) {
                Some((_, Item::Tool(tool))) => Some(tool.as_ref()),
                _ => None,
            })
            .collect();
        let opened;
        let env = if reading.revealed {
            opened = opened_env(reading.env);
            &opened
        } else {
            reading.env
        };
        return tool_texts(&calls, env, reading.room);
    }
    mounted_parts(item, reading)
}

/// The parts of [`mounted_text`], each with whether it is a gutter: what the select mode reads
/// (a caret never stops on a gutter, a copy leaves it out). A Markdown body is a part for each
/// of its `Text`s.
pub fn mounted_parts(item: &Item, reading: &Reading<'_>) -> Option<Vec<Part>> {
    mounted_pieces(item, reading).map(|pieces| pieces.into_iter().map(|piece| piece.part).collect())
}

/// The `Text`s of a mounted item, with what a copy puts before each.
pub fn mounted_pieces(item: &Item, reading: &Reading<'_>) -> Option<Vec<Piece>> {
    let env = reading.env;
    if reading.raw
        && let Some(lines) = raw::lines(item, env.cells)
    {
        // the raw cell is a `Text` per line, a blank line a `Text` of the gap
        return Some(
            lines
                .into_iter()
                .map(|line| {
                    Piece::line(if line.is_empty() {
                        Part::text(env.cells.gap)
                    } else {
                        Part::text(line)
                    })
                })
                .collect(),
        );
    }
    let opened;
    let env = if reading.revealed && matches!(item, Item::Tool(_) | Item::Patch(_)) {
        opened = opened_env(env);
        &opened
    } else {
        env
    };
    match texts_at(
        item,
        env,
        &ReadContext {
            cwd: reading.cwd,
            room: reading.room,
            status: reading.status_words,
            approvals: reading.approval_words,
            usage: reading.usage,
        },
    ) {
        Texts::Exact(parts) => Some(parts.into_iter().map(Piece::line).collect()),
        Texts::Gap(Gap::Markdown) => markdown_pieces(item, reading),
        Texts::Gap(Gap::Rows) => None,
    }
}

/// The `Text`s of an item whose body is Markdown: its gutters and `Text`s, the body a part for each
/// of the view's own (the blocks as the cell draws them).
fn markdown_pieces(item: &Item, reading: &Reading<'_>) -> Option<Vec<Piece>> {
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
            Some(
                parts
                    .into_iter()
                    .map(Piece::line)
                    .chain(Piece::body(body))
                    .collect(),
            )
        }
        Item::Agent { text, .. } => {
            let body = body_of(text, reading)?;
            Some(
                std::iter::once(Piece::line(Part::gutter(reading.bullet)))
                    .chain(Piece::body(body))
                    .collect(),
            )
        }
        // the reasoning cell draws nothing for a body with nothing to say
        Item::Reasoning { .. } => {
            let summary = item.reasoning_text(reading.raw_reasoning);
            if summary.trim().is_empty() {
                return None;
            }
            let body = drawn(crate::widgets::markdown::view(summary))?;
            Some(
                std::iter::once(Piece::line(Part::gutter(strings.bullet)))
                    .chain(Piece::body(body))
                    .collect(),
            )
        }
        Item::Delegated(message) => {
            let attribution = strings.label(strings.delegated_attribution, &message.source);
            let body = drawn(crate::widgets::markdown::view(&message.markdown))?;
            Some(
                [
                    Piece::line(Part::gutter(strings.bullet)),
                    Piece::line(Part::text(attribution)),
                ]
                .into_iter()
                .chain(Piece::body(body))
                .collect(),
            )
        }
        // the header is one `Text` (the bullet is a span of it), a blank `Text` and the plan on
        // its surface, a blank row of padding above it
        Item::Plan { text } => {
            let head = [
                Piece::line(Part::text([strings.bullet, strings.proposed_plan].concat())),
                Piece::line(Part::text(strings.gap)),
                Piece::line(Part::gap()),
            ];
            if text.trim().is_empty() {
                let empty = Piece::line(Part::text(strings.empty_plan));
                return Some(head.into_iter().chain([empty]).collect());
            }
            let body = body_of(text, reading)?;
            Some(head.into_iter().chain(Piece::body(body)).collect())
        }
        Item::Notice(Notice::SessionInfo(info)) if session_info::has_markdown(info) => {
            let tip = info.tip.as_deref()?;
            let tip = strings.fill(strings.tip_label, &[tip]);
            // the first node starts the card: no line break before it
            let mut pieces: Vec<Piece> =
                Piece::body(drawn(crate::widgets::markdown::view(&tip))?).collect();
            let changed = session_info::texts(info, strings);
            if !changed.is_empty() {
                // the card's parts are a blank row apart
                pieces.push(Piece::line(Part::gap()));
                pieces.extend(changed.into_iter().map(Piece::line));
            }
            Some(pieces)
        }
        _ => None,
    }
}

/// The nodes of the Markdown body of `item`, as the cell draws it: what the Markdown a copy of a
/// selection in it writes (`markdown_copy`) reads. None where the item has no Markdown body.
pub fn body_nodes(item: &Item, reading: &Reading<'_>) -> Option<Vec<markdown_text::Node>> {
    let strings = reading.env.cells;
    if reading.raw && raw::lines(item, strings).is_some() {
        return None;
    }
    match item {
        Item::User(prompt) => drawn(crate::widgets::markdown::view(
            prompt.shown().text.trim_end(),
        )),
        Item::Agent { text, .. } | Item::Plan { text } if !text.trim().is_empty() => {
            body_of(text, reading)
        }
        Item::Reasoning { .. } => {
            let summary = item.reasoning_text(reading.raw_reasoning);
            (!summary.trim().is_empty())
                .then(|| drawn(crate::widgets::markdown::view(summary)))
                .flatten()
        }
        Item::Delegated(message) => drawn(crate::widgets::markdown::view(&message.markdown)),
        Item::Notice(Notice::SessionInfo(info)) if session_info::has_markdown(info) => {
            let tip = strings.fill(strings.tip_label, &[info.tip.as_deref()?]);
            drawn(crate::widgets::markdown::view(&tip))
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

/// The nodes of the blocks the cell of a message draws (`citation::cited`).
fn body_of(source: &str, reading: &Reading<'_>) -> Option<Vec<markdown_text::Node>> {
    let blocks = blocks_of(
        source,
        false,
        reading.cwd,
        reading.env.cells,
        reading.rendering,
    );
    markdown_text::nodes(&blocks)
}

/// The nodes of a view's blocks as drawn.
fn drawn(view: MarkdownView) -> Option<Vec<markdown_text::Node>> {
    let blocks: Vec<Block> = view.blocks.map(|blocks| blocks.to_vec())?;
    markdown_text::nodes(&blocks)
}
