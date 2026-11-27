//! The portion of command stdout/stderr still available after the live byte limit.
//! Final command items carry the server's own output instead.

use std::borrow::Cow;
use std::collections::VecDeque;
use std::sync::Arc;

pub(super) const MAX_BYTES: usize = 1024 * 1024;
const END_LINES: usize = 50;
// Leave space for the partial line and the representations of omissions.
const LINE_BYTES: usize = MAX_BYTES / (2 * END_LINES + 2);
const PREFIX_BYTES: usize = LINE_BYTES / 2;
const SUFFIX_BYTES: usize = LINE_BYTES - PREFIX_BYTES;

// The compact shell reads its last three visible lines. These ranges avoid scanning
// the accumulated bytes again, including a long line with no newline yet.
const PREVIEW_LINES: usize = 3;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct OutputLines {
    pub(super) initial: usize,
    pub(super) visible: usize,
    breaks: usize,
    bytes: usize,
    starts: [usize; PREVIEW_LINES],
    visible_starts: [usize; PREVIEW_LINES],
    visible_end: usize,
}

impl OutputLines {
    pub(super) fn of(text: &str) -> Self {
        let mut lines = Self::default();
        lines.append(text);
        lines.initial = lines.visible;
        lines
    }

    /// CR and LF after the last visible byte do not turn into compact rows
    /// until a later delta puts visible text after them.
    pub(super) fn append(&mut self, delta: &str) {
        for (at, byte) in delta.bytes().enumerate() {
            let end = self.bytes + at + 1;
            match byte {
                b'\n' => {
                    self.breaks += 1;
                    self.starts.rotate_left(1);
                    self.starts[PREVIEW_LINES - 1] = end;
                }
                b'\r' => {}
                _ => {
                    if self.visible <= self.breaks {
                        self.visible_starts = self.starts;
                        self.visible = self.breaks + 1;
                    }
                    self.visible_end = end;
                }
            }
        }
        self.bytes += delta.len();
    }

    pub(super) fn tail<'a>(&self, text: &'a str) -> impl DoubleEndedIterator<Item = &'a str> {
        let count = self.visible.min(PREVIEW_LINES);
        let starts = self.visible_starts;
        let visible_end = self.visible_end;
        (PREVIEW_LINES - count..PREVIEW_LINES).map(move |index| {
            let line = if let Some(next) = starts.get(index + 1) {
                &text[starts[index]..next - 1]
            } else {
                &text[starts[index]..visible_end]
            };
            line.strip_suffix('\r').unwrap_or(line)
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RetainedOutput {
    head: Vec<Arc<OutputLine>>,
    tail: VecDeque<Arc<OutputLine>>,
    partial: OutputLine,
    completed: usize,
}

impl RetainedOutput {
    /// Parse only new bytes. A CR at the end of one delta may be the first half of a CRLF.
    pub(super) fn append(&mut self, delta: &str) {
        for part in delta.split_inclusive('\n') {
            let complete = part.ends_with('\n');
            let part = part.strip_suffix('\n').unwrap_or(part);
            let cr = part.ends_with('\r');
            let part = part.strip_suffix('\r').unwrap_or(part);
            if self.partial.pending_cr && (!complete || cr || !part.is_empty()) {
                self.partial.append("\r");
            }
            self.partial.pending_cr = !complete && cr;
            self.partial.append(part);
            if complete {
                self.completed = self.completed.saturating_add(1);
                let line = Arc::new(std::mem::take(&mut self.partial));
                if self.head.len() < END_LINES {
                    self.head.push(line);
                } else {
                    if self.tail.len() == END_LINES {
                        self.tail.pop_front();
                    }
                    self.tail.push_back(line);
                }
            }
        }
    }

    pub fn lines(&self) -> impl DoubleEndedIterator<Item = &OutputLine> {
        self.head
            .iter()
            .chain(self.tail.iter())
            .map(Arc::as_ref)
            .chain((!self.partial.is_empty()).then_some(&self.partial))
    }

    pub fn retained_lines(&self) -> usize {
        self.head.len() + self.tail.len() + usize::from(!self.partial.is_empty())
    }

    pub fn omitted_lines(&self) -> usize {
        self.completed - self.head.len() - self.tail.len()
    }

    /// Where the omission belongs, between the retained head and tail.
    pub fn head_lines(&self) -> usize {
        self.head.len()
    }
}

/// A retained line. If it exceeded the line budget, the number of missing bytes is exact.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OutputLine {
    pub prefix: String,
    suffix: VecDeque<u8>,
    pub omitted_bytes: usize,
    pub pending_cr: bool,
}

impl OutputLine {
    pub fn suffix(&self) -> Cow<'_, str> {
        let (first, last) = self.suffix.as_slices();
        if last.is_empty() {
            String::from_utf8_lossy(first)
        } else {
            let bytes = [first, last].concat();
            Cow::Owned(
                String::from_utf8(bytes)
                    .unwrap_or_else(|error| String::from_utf8_lossy(error.as_bytes()).into_owned()),
            )
        }
    }

    fn is_empty(&self) -> bool {
        self.prefix.is_empty() && self.suffix.is_empty() && !self.pending_cr
    }

    fn append(&mut self, text: &str) {
        let room = if self.suffix.is_empty() && self.omitted_bytes == 0 {
            PREFIX_BYTES - self.prefix.len()
        } else {
            0
        };
        let at = text.floor_char_boundary(room.min(text.len()));
        self.prefix.push_str(&text[..at]);
        let text = &text[at..];
        if text.is_empty() {
            return;
        }

        if text.len() >= SUFFIX_BYTES {
            let first = text.ceil_char_boundary(text.len() - SUFFIX_BYTES);
            self.omitted_bytes = self
                .omitted_bytes
                .saturating_add(self.suffix.len())
                .saturating_add(first);
            self.suffix.clear();
            self.suffix.extend(text[first..].bytes());
        } else {
            let mut first = (self.suffix.len() + text.len()).saturating_sub(SUFFIX_BYTES);
            while self
                .suffix
                .get(first)
                .is_some_and(|byte| byte & 0xc0 == 0x80)
            {
                first += 1;
            }
            if first > 0 {
                self.suffix.drain(..first);
                self.omitted_bytes = self.omitted_bytes.saturating_add(first);
            }
            self.suffix.extend(text.bytes());
        }
    }
}
