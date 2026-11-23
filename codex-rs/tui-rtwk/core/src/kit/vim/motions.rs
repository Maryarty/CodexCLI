//! Where a motion lands and what a text object spans: lines, atomic
//! boundaries, words and objects over a `VimText`
//! (`textarea.rs` `beginning_of_*`/`end_of_*`, `textarea/vim.rs`).

use std::ops::Range;

use super::{Object, VimScope, VimText};

const WORD_SEPARATORS: &str = "`~!@#$%^&*()-=+[{]}\\|;:'\",.<>/?";

fn is_separator(ch: char) -> bool {
    WORD_SEPARATORS.contains(ch)
}

/// The pieces of a run without blanks: stretches of separators and stretches
/// of the rest (`split_word_pieces`; the oracle cuts at Unicode word
/// boundaries first, which only differs for scripts written without spaces).
fn word_pieces(run: &str) -> Vec<(usize, &str)> {
    let mut pieces = Vec::new();
    let mut start = 0;
    let mut in_separator = None;
    for (idx, ch) in run.char_indices() {
        let sep = is_separator(ch);
        match in_separator {
            Some(prev) if prev != sep => {
                pieces.push((start, &run[start..idx]));
                start = idx;
            }
            _ => {}
        }
        in_separator = Some(sep);
    }
    if start < run.len() {
        pieces.push((start, &run[start..]));
    }
    pieces
}

pub(super) fn bol_of(text: &str, pos: usize) -> usize {
    text[..pos].rfind('\n').map_or(0, |i| i + 1)
}

pub(super) fn eol_of(text: &str, pos: usize) -> usize {
    text[pos..].find('\n').map_or(text.len(), |i| i + pos)
}

pub(super) fn bol(t: &dyn VimText) -> usize {
    bol_of(&t.text(), t.cursor())
}

pub(super) fn eol(t: &dyn VimText) -> usize {
    eol_of(&t.text(), t.cursor())
}

pub(super) fn first_non_blank(t: &dyn VimText) -> usize {
    let text = t.text();
    let (b, e) = (bol_of(&text, t.cursor()), eol_of(&text, t.cursor()));
    text[b..e]
        .char_indices()
        .find_map(|(offset, ch)| (!ch.is_whitespace()).then_some(b + offset))
        .unwrap_or(e)
}

/// The line the caret is on, with its newline.
pub(super) fn current_line_with_newline(t: &dyn VimText) -> Range<usize> {
    let text = t.text();
    let (b, e) = (bol_of(&text, t.cursor()), eol_of(&text, t.cursor()));
    b..if e < text.len() { e + 1 } else { e }
}

fn element_containing(t: &dyn VimText, pos: usize) -> Option<Range<usize>> {
    t.elements()
        .into_iter()
        .find(|e| pos > e.start && pos < e.end)
}

pub(super) fn prev_atomic(t: &dyn VimText, pos: usize) -> usize {
    if pos == 0 {
        return 0;
    }
    if let Some(e) = t
        .elements()
        .into_iter()
        .find(|e| pos > e.start && pos <= e.end)
    {
        return e.start;
    }
    let b = t.prev_grapheme(pos);
    element_containing(t, b).map_or(b, |e| e.start)
}

pub(super) fn next_atomic(t: &dyn VimText, pos: usize) -> usize {
    let len = t.text().len();
    if pos >= len {
        return len;
    }
    if let Some(e) = t
        .elements()
        .into_iter()
        .find(|e| pos >= e.start && pos < e.end)
    {
        return e.end;
    }
    let b = t.next_grapheme(pos);
    element_containing(t, b).map_or(b, |e| e.end)
}

/// `range` grown over every element it cuts.
pub(super) fn expand_to_elements(t: &dyn VimText, mut range: Range<usize>) -> Range<usize> {
    let elements = t.elements();
    loop {
        let mut changed = false;
        for e in &elements {
            if e.start < range.end && e.end > range.start {
                let (start, end) = (range.start.min(e.start), range.end.max(e.end));
                if (start, end) != (range.start, range.end) {
                    range = start..end;
                    changed = true;
                }
            }
        }
        if !changed {
            return range;
        }
    }
}

fn out_of_elements(t: &dyn VimText, pos: usize, prefer_start: bool) -> usize {
    element_containing(t, pos).map_or(pos, |e| if prefer_start { e.start } else { e.end })
}

pub(super) fn beginning_of_previous_word(t: &dyn VimText) -> usize {
    let text = t.text();
    let prefix = &text[..t.cursor()];
    let Some((first_non_ws, ch)) = prefix
        .char_indices()
        .rev()
        .find(|&(_, ch)| !ch.is_whitespace())
    else {
        return 0;
    };
    let run_start = prefix[..first_non_ws]
        .char_indices()
        .rev()
        .find(|&(_, ch)| ch.is_whitespace())
        .map_or(0, |(idx, ch)| idx + ch.len_utf8());
    let run_end = first_non_ws + ch.len_utf8();
    let pieces = word_pieces(&prefix[run_start..run_end]);
    let mut pieces = pieces.into_iter().rev().peekable();
    let Some((piece_start, piece)) = pieces.next() else {
        return run_start;
    };
    let mut start = run_start + piece_start;
    if piece.chars().all(is_separator) {
        while let Some((idx, piece)) = pieces.peek() {
            if !piece.chars().all(is_separator) {
                break;
            }
            start = run_start + *idx;
            pieces.next();
        }
    }
    out_of_elements(t, start, true)
}

pub(super) fn end_of_next_word(t: &dyn VimText) -> usize {
    end_of_next_word_from(t, t.cursor())
}

fn end_of_next_word_from(t: &dyn VimText, from: usize) -> usize {
    let text = t.text();
    let suffix = &text[from..];
    let Some(first_non_ws) = suffix.find(|ch: char| !ch.is_whitespace()) else {
        return text.len();
    };
    let run = &suffix[first_non_ws..];
    let run = &run[..run.find(char::is_whitespace).unwrap_or(run.len())];
    let mut pieces = word_pieces(run).into_iter().peekable();
    let Some((start, piece)) = pieces.next() else {
        return from + first_non_ws;
    };
    let word_start = from + first_non_ws + start;
    let mut end = word_start + piece.len();
    if piece.chars().all(is_separator) {
        while let Some((idx, piece)) = pieces.peek() {
            if !piece.chars().all(is_separator) {
                break;
            }
            end = from + first_non_ws + *idx + piece.len();
            pieces.next();
        }
    }
    out_of_elements(t, end, false)
}

pub(super) fn beginning_of_next_word(t: &dyn VimText) -> usize {
    let text = t.text();
    let cursor = t.cursor();
    let Some(first_non_ws) = text[cursor..].find(|c: char| !c.is_whitespace()) else {
        return text.len();
    };
    let word_start = cursor + first_non_ws;
    if word_start != cursor {
        return out_of_elements(t, word_start, true);
    }
    let end = end_of_next_word(t);
    if end >= text.len() {
        return text.len();
    }
    let Some(next_non_ws) = text[end..].find(|c: char| !c.is_whitespace()) else {
        return text.len();
    };
    out_of_elements(t, end + next_non_ws, true)
}

/// The end an operator's `e` reaches (the last character included).
pub(super) fn word_end_exclusive(t: &dyn VimText) -> usize {
    let end = end_of_next_word(t);
    let target = if end > t.cursor() {
        prev_atomic(t, end)
    } else {
        end
    };
    if target == t.cursor() && end < t.text().len() {
        end_of_next_word_from(t, end)
    } else {
        end
    }
}

/// Where `e` puts the caret: on the word's last character.
pub(super) fn word_end_cursor(t: &dyn VimText) -> usize {
    let end = word_end_exclusive(t);
    if end > t.cursor() {
        prev_atomic(t, end)
    } else {
        end
    }
}

/// Where `$` puts the caret: on the line's last character.
pub(super) fn line_end_cursor(t: &dyn VimText) -> usize {
    let (b, e) = (bol(t), eol(t));
    if e > b { prev_atomic(t, e).max(b) } else { e }
}

// ---- text objects ----------------------------------------------------------

pub(super) fn object_range(
    t: &dyn VimText,
    object: Object,
    scope: VimScope,
) -> Option<Range<usize>> {
    match object {
        Object::Word => word_object(t, scope, false),
        Object::BigWord => word_object(t, scope, true),
        Object::Parentheses => paired(t, scope, '(', ')'),
        Object::Brackets => paired(t, scope, '[', ']'),
        Object::Braces => paired(t, scope, '{', '}'),
        Object::DoubleQuote => quoted(t, scope, '"'),
        Object::SingleQuote => quoted(t, scope, '\''),
        Object::Backtick => quoted(t, scope, '`'),
    }
}

fn non_ws_runs(text: &str) -> Vec<Range<usize>> {
    let mut runs = Vec::new();
    let mut start = None;
    for (idx, ch) in text.char_indices() {
        if ch.is_whitespace() {
            if let Some(run_start) = start.take() {
                runs.push(run_start..idx);
            }
        } else if start.is_none() {
            start = Some(idx);
        }
    }
    if let Some(run_start) = start {
        runs.push(run_start..text.len());
    }
    runs
}

fn overlaps(cursor: usize, range: &Range<usize>) -> bool {
    range.start <= cursor && cursor < range.end
}

fn at_end(cursor: usize, range: &Range<usize>) -> bool {
    range.start < range.end && cursor == range.end
}

fn word_object(t: &dyn VimText, scope: VimScope, big: bool) -> Option<Range<usize>> {
    let text = t.text();
    let cursor = t.cursor();
    let inner = if big {
        non_ws_runs(&text)
            .into_iter()
            .find(|r| overlaps(cursor, r) || at_end(cursor, r))
    } else {
        small_word(&text, cursor)
    }?;
    Some(match scope {
        VimScope::Inner => inner,
        VimScope::Around => {
            let following = text[inner.end..]
                .char_indices()
                .take_while(|(_, ch)| ch.is_whitespace())
                .last()
                .map_or(inner.end, |(i, ch)| inner.end + i + ch.len_utf8());
            if following > inner.end {
                inner.start..following
            } else {
                let start = text[..inner.start]
                    .char_indices()
                    .rev()
                    .take_while(|(_, ch)| ch.is_whitespace())
                    .last()
                    .map_or(inner.start, |(i, _)| i);
                start..inner.end
            }
        }
    })
}

fn small_word(text: &str, cursor: usize) -> Option<Range<usize>> {
    for run in non_ws_runs(text) {
        if !overlaps(cursor, &run) && !at_end(cursor, &run) {
            continue;
        }
        let mut last = None;
        for (piece_start, piece) in word_pieces(&text[run.clone()]) {
            let piece = run.start + piece_start..run.start + piece_start + piece.len();
            if overlaps(cursor, &piece) {
                return Some(piece);
            }
            last = Some(piece);
        }
        if at_end(cursor, &run) {
            return last.or(Some(run));
        }
        return Some(run);
    }
    None
}

fn paired(t: &dyn VimText, scope: VimScope, open: char, close: char) -> Option<Range<usize>> {
    let text = t.text();
    let cursor = t.cursor();
    let elements = t.elements();
    let mut stack: Vec<usize> = Vec::new();
    let mut best: Option<Range<usize>> = None;
    for (idx, ch) in text.char_indices() {
        if elements.iter().any(|e| e.contains(&idx)) {
            continue;
        }
        if ch == open {
            stack.push(idx);
        } else if ch == close {
            let Some(open_idx) = stack.pop() else {
                continue;
            };
            if open_idx <= cursor && cursor <= idx {
                let candidate = match scope {
                    VimScope::Inner => open_idx + open.len_utf8()..idx,
                    VimScope::Around => open_idx..idx + ch.len_utf8(),
                };
                if candidate.start <= candidate.end
                    && best.as_ref().is_none_or(|b| candidate.len() < b.len())
                {
                    best = Some(candidate);
                }
            }
        }
    }
    best
}

fn quoted(t: &dyn VimText, scope: VimScope, quote: char) -> Option<Range<usize>> {
    let text = t.text();
    let cursor = t.cursor();
    let elements = t.elements();
    let line = bol_of(&text, cursor)..eol_of(&text, cursor);
    let mut open = None;
    let mut best: Option<Range<usize>> = None;
    for (offset, ch) in text[line.clone()].char_indices() {
        let idx = line.start + offset;
        if elements.iter().any(|e| e.contains(&idx)) || ch != quote || escaped(&text, idx) {
            continue;
        }
        if let Some(open_idx) = open.take() {
            if open_idx <= cursor && cursor <= idx {
                let candidate = match scope {
                    VimScope::Inner => open_idx + quote.len_utf8()..idx,
                    VimScope::Around => open_idx..idx + quote.len_utf8(),
                };
                if candidate.start <= candidate.end
                    && best.as_ref().is_none_or(|b| candidate.len() < b.len())
                {
                    best = Some(candidate);
                }
            }
        } else {
            open = Some(idx);
        }
    }
    best
}

fn escaped(text: &str, pos: usize) -> bool {
    text[..pos]
        .chars()
        .rev()
        .take_while(|&ch| ch == '\\')
        .count()
        % 2
        == 1
}
