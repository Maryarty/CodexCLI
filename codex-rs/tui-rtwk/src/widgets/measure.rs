//! The rows a text takes when `Text` wraps it by word (G38: not a gap, this
//! is Flutter's `TextPainter.layout().computeLineMetrics()`). The details pane puts an elision on its last visible
//! row and keeps two rows of a prompt, so the view must know how many rows
//! a text takes; no widget tells it. The breaking is rtwk-render's own
//! `break_lines` (the engine `Text` uses).

use rtwk_core::unicode::TextSegmenter;
use rtwk_render::objects::text::{TextWrap, break_lines};

/// The segmenter the tree lays text out with (`segmenter_of`), carried to the
/// measuring helpers of this directory (the gap files); the views name only
/// this alias.
pub type Measure = TextSegmenter;

/// The rows of `text` at `width` cells, an empty line for a blank one.
pub fn rows(seg: TextSegmenter, text: &str, width: usize) -> Vec<String> {
    let width = i32::try_from(width).unwrap_or(i32::MAX);
    text.split('\n')
        .flat_map(|part| {
            if part.trim().is_empty() {
                return vec![String::new()];
            }
            let breaks = break_lines(&seg, part, width, TextWrap::Word);
            breaks
                .into_iter()
                .map(|(a, b)| part[a..b].to_owned())
                .collect()
        })
        .collect()
}

/// How many rows `text` takes at `width` cells when `Text` breaks it
/// between any two graphemes (`TextWrap::Grapheme`: a path).
pub fn grapheme_rows(seg: TextSegmenter, text: &str, width: usize) -> usize {
    let width = i32::try_from(width).unwrap_or(i32::MAX);
    text.split('\n')
        .map(|part| {
            break_lines(&seg, part, width, TextWrap::Grapheme)
                .len()
                .max(1)
        })
        .sum()
}
