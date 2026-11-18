//! G38 (`docs/rtwk-port/rtwk-gaps.md`): the rows a text takes when `Text`
//! wraps it by word. The details pane puts an elision on its last visible
//! row and keeps two rows of a prompt, so the view must know how many rows
//! a text takes; no widget tells it. The breaking is rtwk-render's own
//! `break_lines` (the engine `Text` uses); the facade does not export it.

use rtwk_core::unicode::TextSegmenter;
use rtwk_render::objects::text::{TextWrap, break_lines};

/// The rows of `text` at `width` cells, an empty line for a blank one.
pub fn rows(text: &str, width: usize) -> Vec<String> {
    let width = i32::try_from(width).unwrap_or(i32::MAX);
    let seg = TextSegmenter::default();
    text.split('\n')
        .flat_map(|part| {
            if part.trim().is_empty() {
                return vec![String::new()];
            }
            let breaks = break_lines(&seg, part, width, TextWrap::Word);
            breaks.into_iter().map(|(a, b)| part[a..b].to_owned()).collect()
        })
        .collect()
}
