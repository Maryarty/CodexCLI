//! The status row's reasoning header (oracle `chatwidget/streaming.rs`): the last usable line
//! of the model's reasoning summary, "Working" until there is one.
//!
//! The header is derived, not a second state machine: the running turn's reasoning items are
//! already in the transcript with their text, so the header is the latest usable line of them;
//! the only state is what the transcript has not got yet, the streamed deltas (`Live`).

use crate::ids::{ItemId, TurnId};
use crate::threads::{Item, ItemKey, Transcript};

/// Which stream a delta came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// `item/reasoning/summaryTextDelta`.
    Summary,
    /// `item/reasoning/textDelta`, the model's raw reasoning: counts only while it is shown.
    Content,
}

/// What the server streams of the model's reasoning.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ev {
    /// A delta of the reasoning item `item` (`summaryTextDelta`, `textDelta`).
    Delta {
        turn: TurnId,
        item: ItemId,
        kind: Kind,
        delta: String,
    },
    /// A new part of the summary of `item` begins: the thread is thinking.
    Part { turn: TurnId, item: ItemId },
}

impl Ev {
    pub(crate) fn key(&self) -> ItemKey {
        match self {
            Self::Delta { turn, item, .. } | Self::Part { turn, item } => ItemKey {
                turn: turn.clone(),
                item: item.clone(),
            },
        }
    }

    pub fn turn(&self) -> &TurnId {
        match self {
            Self::Delta { turn, .. } | Self::Part { turn, .. } => turn,
        }
    }
}

/// The streamed deltas of the turn's reasoning items (a completion may carry no text: the
/// deltas are all the item has), a part break standing as a blank line.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Live {
    items: Vec<Streamed>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Streamed {
    item: ItemId,
    summary: String,
    content: String,
}

impl Live {
    /// Takes `ev`; whether it says the thread is thinking (a new part).
    pub fn apply(&mut self, ev: Ev) -> bool {
        match ev {
            Ev::Delta {
                item, kind, delta, ..
            } => {
                self.delta(&item, kind, &delta);
                false
            }
            Ev::Part { item, .. } => {
                self.part_break(&item);
                true
            }
        }
    }

    /// A delta of `item`.
    fn delta(&mut self, item: &ItemId, kind: Kind, delta: &str) {
        let streamed = self.of(item);
        match kind {
            Kind::Summary => streamed.summary.push_str(delta),
            Kind::Content => streamed.content.push_str(delta),
        }
    }

    /// A section break of `item` (oracle `on_reasoning_section_break`).
    fn part_break(&mut self, item: &ItemId) {
        let streamed = self.of(item);
        for text in [&mut streamed.summary, &mut streamed.content] {
            if !text.trim().is_empty() {
                text.push_str("\n\n");
            }
        }
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }

    fn of(&mut self, item: &ItemId) -> &mut Streamed {
        let at = self
            .items
            .iter()
            .position(|streamed| &streamed.item == item)
            .unwrap_or_else(|| {
                self.items.push(Streamed {
                    item: item.clone(),
                    summary: String::new(),
                    content: String::new(),
                });
                self.items.len() - 1
            });
        &mut self.items[at]
    }

    /// What the deltas of `item` add to its own text, in the order the oracle's buffer has them.
    fn text(&self, item: &ItemId, show_raw: bool) -> String {
        let Some(streamed) = self.items.iter().find(|streamed| &streamed.item == item) else {
            return String::new();
        };
        if show_raw {
            format!("{}{}", streamed.summary, streamed.content)
        } else {
            streamed.summary.clone()
        }
    }
}

/// The last usable line of a reasoning summary (oracle `latest_summary_line`): the last line
/// that is not blank or a comment, without heading marks; a leading `**bold**` and its plain
/// suffix join; a line whose `**` is not closed yet is no line.
pub fn latest_summary_line(text: &str) -> Option<String> {
    text.lines().rev().find_map(|line| {
        let line = line.trim();
        if line.is_empty() || line.starts_with("<!--") {
            return None;
        }
        let line = line.trim_start_matches('#').trim();
        let line = if let Some(stripped) = line.strip_prefix("**") {
            let (bold, trailing) = stripped.split_once("**")?;
            format!("{bold}{trailing}")
        } else {
            line.to_string()
        };
        (!line.is_empty()).then_some(line)
    })
}

/// The header the running `turn`'s reasoning gives the status row: the latest usable line of its
/// reasoning items, newest first (the last useful summary stays through tools and empty items);
/// the deltas in `live` extend the items they belong to.
pub fn header(
    transcript: &Transcript,
    turn: &TurnId,
    live: &Live,
    show_raw: bool,
) -> Option<String> {
    // the turn's items are the transcript's tail
    let mut items = Vec::new();
    for index in (0..transcript.len()).rev() {
        let Some((key, item)) = transcript.get(index) else {
            continue;
        };
        if &key.turn != turn {
            break;
        }
        if matches!(item, Item::Reasoning { .. }) {
            items.push((key, item));
        }
    }
    items.iter().find_map(|(key, item)| {
        let text = item.reasoning_text(show_raw);
        let extra = live.text(&key.item, show_raw);
        if !extra.trim().is_empty() && !text.trim_end().ends_with(extra.trim()) {
            // an item that holds the text its deltas made adds nothing
            latest_summary_line(&format!("{text}{extra}"))
        } else {
            latest_summary_line(text)
        }
    })
}
