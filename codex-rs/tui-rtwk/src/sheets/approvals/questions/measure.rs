//! What the editor of the async questions measures before it lays out: the rows the wrapped
//! question, a choice, the draft and the footer take at a width (G38: `break_lines`, the engine
//! `Text` wraps with), and the footer's tips packed into rows and cut with an ellipsis as the
//! oracle cuts them (`truncate_line_word_boundary_with_ellipsis`).

use rtwk::style::Span;
use rtwk_core::unicode::TextSegmenter;

use crate::pages::session::transcript::markdown_text;
use crate::widgets::markdown;
use crate::widgets::measure;

/// The rows `text` takes at `width` cells, one at least.
pub fn rows(seg: TextSegmenter, text: &str, width: usize) -> usize {
    measure::rows(seg, text, width.max(1)).len().max(1)
}

/// The rows of the Markdown `source` at `width` cells, as the view draws it: each block's text,
/// and the blank row between two blocks.
pub fn markdown_rows(seg: TextSegmenter, source: &str, width: usize) -> usize {
    let nodes = nodes_of(source);
    if nodes.is_empty() {
        return 0;
    }
    nodes
        .iter()
        .map(|node| usize::from(node.before == "\n\n") + rows(seg, &node.text, width))
        .sum()
}

/// What the Markdown `source` says as text (the choice rows of the oracle measure this).
pub fn plain_of(source: &str) -> String {
    markdown_text::copied(&nodes_of(source))
}

fn nodes_of(source: &str) -> Vec<markdown_text::Node> {
    let mut stream = rtwk_rich::markdown::MarkdownStream::new(source);
    let blocks = stream.blocks().to_vec();
    let definitions = stream.definitions().clone();
    let blocks = markdown::prepared(&blocks, &definitions, None);
    markdown_text::nodes(&blocks).unwrap_or_default()
}

/// The width of `spans` in cells.
pub fn width_of(seg: TextSegmenter, spans: &[Span]) -> usize {
    spans.iter().map(|span| seg.width(&span.text)).sum()
}

/// `tips` (each its spans) packed into rows as wide as `width`, `gap` between the tips of a row.
pub fn pack(seg: TextSegmenter, tips: Vec<Vec<Span>>, gap: &'static str, width: usize) -> Vec<Vec<Span>> {
    let mut lines: Vec<Vec<Span>> = Vec::new();
    let mut line: Vec<Span> = Vec::new();
    for tip in tips {
        if !line.is_empty() {
            if width_of(seg, &line) + seg.width(gap) + width_of(seg, &tip) > width {
                lines.push(std::mem::take(&mut line));
            } else {
                line.push(Span::new(gap));
            }
        }
        line.extend(tip);
    }
    lines.push(line);
    lines
}

/// `spans` cut to `max` cells at a word boundary when there is one, with an ellipsis.
pub fn truncate(seg: TextSegmenter, spans: Vec<Span>, max: usize) -> Vec<Span> {
    if max == 0 {
        return Vec::new();
    }
    if width_of(seg, &spans) <= max {
        return spans;
    }
    let ellipsis = "…";
    if seg.width(ellipsis) >= max {
        return vec![Span::new(ellipsis)];
    }
    let limit = max - seg.width(ellipsis);
    // The chars in order with the span each belongs to.
    let chars: Vec<(usize, char)> = spans
        .iter()
        .enumerate()
        .flat_map(|(at, span)| span.text.chars().map(move |c| (at, c)))
        .collect();
    let mut used = 0;
    let mut fit = 0;
    let mut word = None;
    for (at, (_, c)) in chars.iter().enumerate() {
        let w = seg.width(&c.to_string());
        if used + w > limit {
            break;
        }
        if c.is_whitespace() && at > 0 {
            word = Some(at);
        }
        used += w;
        fit = at + 1;
    }
    // A cut at a word boundary only when the text really goes on past it.
    let cut = word.unwrap_or(fit);
    let mut kept = chars[..cut].to_vec();
    while kept.last().is_some_and(|(_, c)| c.is_whitespace()) {
        kept.pop();
    }
    let mut groups: Vec<(usize, String)> = Vec::new();
    for (at, c) in &kept {
        match groups.last_mut() {
            Some((last, text)) if last == at => text.push(*c),
            _ => groups.push((*at, c.to_string())),
        }
    }
    let mark_style = groups
        .last()
        .map_or_else(Default::default, |(at, _)| spans[*at].style.clone());
    let mut out: Vec<Span> = groups
        .into_iter()
        .map(|(at, text)| Span::styled(text, spans[at].style.clone()))
        .collect();
    out.push(Span::styled(ellipsis, mark_style));
    out
}
