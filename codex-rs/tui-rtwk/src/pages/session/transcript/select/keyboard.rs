//! Keyboard movement of an existing native mouse selection. Its active end lies between
//! graphemes, unlike the inclusive caret of explicit Select mode.

use rtwk::focus::{KeyCombo, KeyPress};

use codex_tui_core::threads::ItemKey;

use super::{Motion, Pos, Rc, RowTexts, Select, TextSelection, lines, mouse_range, moved};

/// A byte in the text of a transcript row. A prepend can change the row's index.
pub(super) struct Endpoint {
    item: ItemKey,
    offset: usize,
}

impl Endpoint {
    fn new(rows: &RowTexts, pos: Pos) -> Option<Self> {
        Some(Self {
            item: rows.item_key(pos.row)?.clone(),
            offset: pos.offset,
        })
    }

    fn resolve(&self, rows: &RowTexts) -> Option<Pos> {
        Some(Pos {
            row: rows.row_of_item(&self.item)?,
            offset: self.offset,
        })
    }
}

pub(super) enum Range {
    Starting {
        text: String,
        offset: Option<usize>,
        motion: Vec<Motion>,
    },
    Extending {
        anchor: Endpoint,
        end: Endpoint,
        column: Option<usize>,
        motion: Vec<Motion>,
    },
}

fn motion(key: &KeyPress) -> Option<Motion> {
    [
        (["left", "shift+left"], Motion::Left),
        (["right", "shift+right"], Motion::Right),
        (["up", "shift+up"], Motion::Rows(-1)),
        (["down", "shift+down"], Motion::Rows(1)),
    ]
    .into_iter()
    .find(|(specs, _)| {
        specs
            .iter()
            .any(|spec| KeyCombo::parse(spec).is_ok_and(|combo| combo.accepts_press(key)))
    })
    .map(|(_, step)| step)
}

impl Select {
    /// The reader has a keyboard selection, including a range collapsed back to its anchor.
    pub fn extending(&self) -> bool {
        self.0.data.borrow().keyboard.is_some()
    }

    /// Accept a plain or shifted arrow while a native mouse selection is held.
    pub fn extend(&self, key: &KeyPress, selection: &TextSelection) -> bool {
        let Some(step) = motion(key) else {
            return false;
        };
        let mut data = self.0.data.borrow_mut();
        if data.active {
            return false;
        }
        match &mut data.keyboard {
            Some(Range::Starting { motion, .. } | Range::Extending { motion, .. }) => {
                motion.push(step);
            }
            None => {
                let controller = selection.controller();
                let Some((from, to)) = controller.start().zip(controller.end()) else {
                    return false;
                };
                data.keyboard = Some(Range::Starting {
                    text: controller.text(),
                    offset: (from.object == to.object).then_some(from.offset),
                    motion: vec![step],
                });
                data.list = None;
                data.shown = None;
            }
        }
        drop(data);
        self.notify();
        true
    }

    /// Return control to the native area when another key or a pointer takes its selection.
    pub fn end_extension(&self) {
        if self.0.data.borrow_mut().keyboard.take().is_some() {
            self.notify();
        }
    }

    /// Paint the keyboard range through the same selection area as the native mouse.
    pub fn show_extension(&self, rows: &Rc<RowTexts>, selection: &TextSelection) {
        let Some(held) = self.0.data.borrow_mut().keyboard.take() else {
            return;
        };
        let first = matches!(held, Range::Starting { .. });
        let (anchor, mut end, mut column, motion) = match held {
            Range::Starting {
                text,
                offset,
                motion,
            } => {
                let Some((from, to, _)) = mouse_range(rows, &text, offset) else {
                    self.notify();
                    return;
                };
                (from, to, None, motion)
            }
            Range::Extending {
                anchor,
                end,
                column,
                motion,
            } => {
                let Some((anchor, end)) = anchor.resolve(rows).zip(end.resolve(rows)) else {
                    selection.controller().clear();
                    self.notify();
                    return;
                };
                (anchor, end, column, motion)
            }
        };
        let lines = lines(rows);
        let in_rows = |point: Pos| {
            lines.iter().any(|line| {
                point.row == line.row
                    && (line.start..=line.end).contains(&point.offset)
                    && rows.row_text(point.row).is_char_boundary(point.offset)
            })
        };
        if !in_rows(anchor) || !in_rows(end) {
            self.notify();
            return;
        }
        if !motion.is_empty() {
            for step in motion {
                (end, column) = moved(rows, &lines, end, column, step, true);
            }
            let mut data = self.0.data.borrow_mut();
            data.cursor = end;
            data.reveal = true;
        }
        let Some((saved_anchor, saved_end)) =
            Endpoint::new(rows, anchor).zip(Endpoint::new(rows, end))
        else {
            self.notify();
            return;
        };
        self.0.data.borrow_mut().keyboard = Some(Range::Extending {
            anchor: saved_anchor,
            end: saved_end,
            column,
            motion: Vec::new(),
        });
        let wanted = (anchor != end).then_some((anchor.min(end), anchor.max(end)));
        self.show_range(rows, selection, wanted, !first);
    }
}
