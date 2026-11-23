use rtwk_render::objects::text::TextWrap;
use rtwk_render::objects::text::break_lines;

use codex_tui_rtwk::widgets::measure::rows;
use rtwk_core::unicode::{TextMode, TextSegmenter};

/// The rows follow the tree's segmenter (`segmenter_of`), not the default:
/// ambiguous-width glyphs take two cells on a CJK terminal.
#[test]
fn rows_follow_the_segmenter_given() {
    let narrow = TextSegmenter::default();
    let wide = TextSegmenter::new(TextMode::Graphemes, 2);
    assert_eq!(rows(narrow, "\u{25cf}\u{25cf}\u{25cf}\u{25cf}", 4).len(), 1);
    assert!(rows(wide, "\u{25cf}\u{25cf}\u{25cf}\u{25cf}", 4).len() > 1);
}
