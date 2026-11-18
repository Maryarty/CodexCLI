//! The command center's pure geometry and text fitting: the two-pane split,
//! the filter strip's window, word wrapping of the details.
//! Oracle: `app/agent_center/render.rs` (`center_layout`),
//! `bottom_pane/selection_tabs.rs` (`render_filled_tab_bar`) and
//! `agent_center/rows.rs` (`columns`).

use rtwk_core::unicode::TextOverflow;
use rtwk_core::unicode::TextSegmenter;
use rtwk_core::unicode::fit_text;
use rtwk_core::unicode::grapheme_boundaries;

/// The rows and panes of the screen at `width` x `height` (oracle:
/// `center_layout`): the header is three rows when the screen has six, the
/// footer one row, the body is inset two cells on each side and splits into
/// a list, a three-cell gap and a 38-cell detail pane from 90 cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Areas {
    pub header: usize,
    pub footer_row: usize,
    pub body_top: usize,
    pub body_height: usize,
    pub list_x: usize,
    pub list_width: usize,
    pub gap_x: Option<usize>,
    pub details_x: Option<usize>,
    pub details_width: usize,
}

const INSET: usize = 2;

pub fn areas(width: usize, height: usize) -> Areas {
    let footer = usize::from(height >= 2);
    let header = if height >= 6 { 3 } else { 0 };
    let body_height = height.saturating_sub(header + footer);
    let body_width = width.saturating_sub(2 * INSET);
    let (list_width, gap_x, details_x, details_width) = if body_width >= 90 {
        let details = 38;
        let list = body_width - 3 - details;
        (list, Some(INSET + list), Some(INSET + list + 3), details)
    } else {
        (body_width, None, None, 0)
    };
    Areas {
        header,
        footer_row: height.saturating_sub(footer),
        body_top: header,
        body_height,
        list_x: INSET,
        list_width,
        gap_x,
        details_x,
        details_width,
    }
}

/// The window of the filter strip and what it shows around it (oracle:
/// `render_filled_tab_bar`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Strip {
    pub start: usize,
    pub end: usize,
    pub left: bool,
    pub right: bool,
    /// The width of each shown tab, clipped to the room.
    pub widths: Vec<usize>,
}

pub fn strip(labels: &[String], active: usize, width: usize) -> Strip {
    let count = labels.len();
    if count == 0 || width == 0 {
        return Strip {
            start: 0,
            end: 0,
            left: false,
            right: false,
            widths: Vec::new(),
        };
    }
    let active = active.min(count - 1);
    let widths: Vec<usize> = labels.iter().map(|l| columns(l) + 2).collect();
    let occupied = |start: usize, end: usize| {
        widths[start..end].iter().sum::<usize>() + end - start - 1
            + usize::from(start > 0) * 2
            + usize::from(end < count) * 2
    };
    let (mut start, mut end) = (active, active + 1);
    while start > 0 && occupied(start - 1, end) <= width {
        start -= 1;
    }
    while end < count && occupied(start, end + 1) <= width {
        end += 1;
    }
    let left = start > 0 && width >= 5;
    let right = end < count && width >= 7;
    let mut x = if left { 2 } else { 0 };
    let limit = width - if right { 2 } else { 0 };
    let mut shown = Vec::new();
    for w in &widths[start..end] {
        let w = (*w).min(limit.saturating_sub(x));
        shown.push(w);
        x += w + 1;
    }
    Strip {
        start,
        end,
        left,
        right,
        widths: shown,
    }
}

/// The cells `text` takes on the screen.
pub fn columns(text: &str) -> usize {
    TextSegmenter::default().width(text)
}

/// The first part of `text` that fits `room` cells, whole graphemes only.
fn head(text: &str, room: usize) -> &str {
    let segmenter = TextSegmenter::default();
    let mut used = 0;
    let mut end = 0;
    for (start, stop) in grapheme_boundaries(text) {
        let cells = segmenter.width(&text[start..stop]);
        if used + cells > room {
            break;
        }
        used += cells;
        end = stop;
    }
    &text[..end]
}

/// Greedy word wrap to `width` cells; a word longer than the row is cut.
pub fn wrap(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let mut word = word;
        while columns(word) > width {
            let first = head(word, width);
            if first.is_empty() {
                break;
            }
            word = &word[first.len()..];
            if !current.is_empty() {
                lines.push(std::mem::take(&mut current));
            }
            lines.push(first.to_owned());
        }
        let need = columns(&current) + usize::from(!current.is_empty()) + columns(word);
        if need > width && !current.is_empty() {
            lines.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

/// The text cut to `width` cells with an ellipsis (oracle:
/// `truncate_line_with_ellipsis_if_overflow`).
pub fn ellipsize(text: &str, width: usize) -> String {
    let room = isize::try_from(width).unwrap_or(isize::MAX);
    fit_text(
        &TextSegmenter::default(),
        text,
        room,
        TextOverflow::Ellipsis,
        "\u{2026}",
    )
    .into_owned()
}

/// The end of `original` that fits `allowed` cells behind a `…`.
fn front_truncate(original: &str, allowed: usize) -> String {
    if allowed == 0 {
        return String::new();
    }
    if columns(original) <= allowed {
        return original.to_owned();
    }
    if allowed == 1 {
        return "\u{2026}".to_owned();
    }
    let room = isize::try_from(allowed).unwrap_or(isize::MAX);
    fit_text(
        &TextSegmenter::default(),
        original,
        room,
        TextOverflow::EllipsisStart,
        "\u{2026}",
    )
    .into_owned()
}

/// One segment of a path while it is cut to fit.
struct Piece<'a> {
    original: &'a str,
    text: String,
    cuttable: bool,
    tail: bool,
}

fn assemble(leading: bool, pieces: &[Piece<'_>]) -> String {
    let mut path = String::new();
    if leading {
        path.push('/');
    }
    for piece in pieces {
        if !path.is_empty() && !path.ends_with('/') {
            path.push('/');
        }
        path.push_str(&piece.text);
    }
    path
}

/// Cuts `pieces` until the path fits `max` cells: the segment that may be
/// cut, tail ones first, loses its start behind a `…`.
fn fit_pieces(
    leading: bool,
    pieces: &mut [Piece<'_>],
    max: usize,
    cut: bool,
    count: usize,
) -> Option<String> {
    loop {
        let candidate = assemble(leading, pieces);
        let width = columns(&candidate);
        if width <= max {
            return Some(candidate);
        }
        if !cut {
            return None;
        }
        let order: Vec<usize> = (0..pieces.len())
            .rev()
            .filter(|at| pieces[*at].cuttable && pieces[*at].tail)
            .chain(
                (0..pieces.len())
                    .rev()
                    .filter(|at| pieces[*at].cuttable && !pieces[*at].tail),
            )
            .collect();
        let mut changed = false;
        for at in order {
            if columns(pieces[at].original) <= max && count > 2 {
                continue;
            }
            let others = width.saturating_sub(columns(&pieces[at].text));
            let allowed = max.saturating_sub(others).max(1);
            let text = front_truncate(pieces[at].original, allowed);
            if text != pieces[at].text {
                pieces[at].text = text;
                changed = true;
                break;
            }
        }
        if !changed {
            return None;
        }
    }
}

/// A path cut to `max` cells with a `…` in place of its middle segments,
/// its first segments and its last two kept as the room allows (oracle:
/// `text_formatting.rs` `center_truncate_path`, held to its four tests).
pub fn center_truncate_path(path: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    if columns(path) <= max {
        return path.to_owned();
    }
    let leading = path.starts_with('/');
    let mut segments: Vec<&str> = path.split('/').collect();
    if leading && segments.first().is_some_and(|first| first.is_empty()) {
        segments.remove(0);
    }
    if path.ends_with('/') && segments.last().is_some_and(|last| last.is_empty()) {
        segments.pop();
    }
    let count = segments.len();
    if count == 0 {
        return if leading && max >= 1 {
            "/".to_owned()
        } else {
            "\u{2026}".to_owned()
        };
    }
    let suffix = if count > 1 { count.min(3) - 1 } else { 0 };
    let mut combos: Vec<(usize, usize)> = (1..=count)
        .flat_map(|left| {
            let least = usize::from(left != count);
            (least..=count - left).map(move |right| (left, right))
        })
        .collect();
    let by_room = |a: &(usize, usize), b: &(usize, usize)| {
        b.0.cmp(&a.0)
            .then(b.1.cmp(&a.1))
            .then((b.0 + b.1).cmp(&(a.0 + a.1)))
    };
    combos.sort_by(by_room);
    let (preferred, rest): (Vec<_>, Vec<_>) =
        combos.into_iter().partition(|(_, right)| *right >= suffix);
    for (left, right) in preferred.into_iter().chain(rest) {
        let mut pieces: Vec<Piece<'_>> = segments[..left]
            .iter()
            .map(|seg| Piece {
                original: seg,
                text: (*seg).to_owned(),
                cuttable: true,
                tail: false,
            })
            .collect();
        let ellipsis = left + right < count;
        if ellipsis {
            pieces.push(Piece {
                original: "\u{2026}",
                text: "\u{2026}".to_owned(),
                cuttable: false,
                tail: false,
            });
        }
        pieces.extend(segments[count - right..].iter().map(|seg| Piece {
            original: seg,
            text: (*seg).to_owned(),
            cuttable: true,
            tail: true,
        }));
        if let Some(fit) = fit_pieces(leading, &mut pieces, max, ellipsis || count <= 2, count) {
            return fit;
        }
    }
    front_truncate(path, max)
}

/// What a span of a help line is for: the title of a group, a key, or text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ink {
    Title,
    Key,
    Plain,
}

/// One row of the shortcut help, as spans.
pub type HelpLine = Vec<(Ink, String)>;

/// A titled group of `(key, action)` entries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HelpGroup {
    pub title: String,
    pub entries: Vec<(String, String)>,
}

fn line_width(line: &HelpLine) -> usize {
    line.iter().map(|(_, text)| columns(text)).sum()
}

fn group_rows(group: &HelpGroup) -> Vec<HelpLine> {
    let key_width = group
        .entries
        .iter()
        .map(|(key, _)| columns(key))
        .max()
        .unwrap_or(0);
    let mut rows = vec![vec![(Ink::Title, group.title.clone())]];
    for (key, action) in &group.entries {
        let pad = " ".repeat(key_width.saturating_sub(columns(key)) + 2);
        rows.push(vec![
            (Ink::Key, key.clone()),
            (Ink::Plain, pad),
            (Ink::Plain, action.clone()),
        ]);
    }
    rows
}

const COLUMN_GAP: usize = 4;

fn join_columns(groups: &[Vec<HelpLine>], widths: &[usize]) -> Vec<HelpLine> {
    let height = groups.iter().map(Vec::len).max().unwrap_or(0);
    (0..height)
        .map(|row| {
            let mut line = HelpLine::new();
            for (column, group) in groups.iter().enumerate() {
                let entry = group.get(row).cloned().unwrap_or_default();
                let pad = widths[column].saturating_sub(line_width(&entry)) + COLUMN_GAP;
                line.extend(entry);
                if column + 1 < groups.len() {
                    line.push((Ink::Plain, " ".repeat(pad)));
                }
            }
            line
        })
        .collect()
}

/// The three groups flowed into three, two or one column by `width`
/// (oracle: `shortcut_help.rs` `group_lines`).
pub fn help_rows(groups: &[HelpGroup; 3], width: usize) -> Vec<HelpLine> {
    let groups: Vec<Vec<HelpLine>> = groups.iter().map(group_rows).collect();
    let widths: Vec<usize> = groups
        .iter()
        .map(|rows| rows.iter().map(line_width).max().unwrap_or(0))
        .collect();
    let width = width.max(1);
    if widths.iter().sum::<usize>() + COLUMN_GAP * 2 <= width {
        return join_columns(&groups, &widths);
    }
    if widths[0] + COLUMN_GAP + widths[1].max(widths[2]) <= width {
        let mut right = groups[1].clone();
        right.push(HelpLine::new());
        right.extend(groups[2].clone());
        return join_columns(
            &[groups[0].clone(), right],
            &[widths[0], widths[1].max(widths[2])],
        );
    }
    let mut rows = Vec::new();
    for (index, group) in groups.into_iter().enumerate() {
        if index > 0 {
            rows.push(HelpLine::new());
        }
        rows.extend(group);
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_panes_from_ninety_body_cells() {
        let wide = areas(96, 25);
        assert_eq!(
            (wide.list_width, wide.gap_x, wide.details_x),
            (51, Some(53), Some(56))
        );
        let narrow = areas(48, 25);
        assert_eq!((narrow.list_width, narrow.details_x), (44, None));
        assert_eq!(
            (narrow.header, narrow.footer_row, narrow.body_height),
            (3, 24, 21)
        );
    }

    #[test]
    fn the_strip_scrolls_around_the_active_tab() {
        let labels: Vec<String> = ["All 0", "Needs you 0", "Working 0", "Ready 0", "Inactive 0"]
            .iter()
            .map(ToString::to_string)
            .collect();
        let narrow = strip(&labels, 0, 44);
        assert_eq!((narrow.start, narrow.end, narrow.right), (0, 3, true));
        let wide = strip(&labels, 0, 76);
        assert_eq!((wide.end, wide.right), (5, false));
    }

    #[test]
    fn test_center_truncate_doesnt_truncate_short_path() {
        assert_eq!(
            center_truncate_path("/Users/codex/Public", 40),
            "/Users/codex/Public"
        );
    }

    #[test]
    fn test_center_truncate_truncates_long_path() {
        assert_eq!(
            center_truncate_path("~/hello/the/fox/is/very/fast", 24),
            "~/hello/the/\u{2026}/very/fast"
        );
    }

    #[test]
    fn test_center_truncate_truncates_long_windows_path() {
        let path = "C:/Users/codex/Projects/super/long/windows/path/file.txt";
        assert_eq!(
            center_truncate_path(path, 36),
            "C:/Users/codex/\u{2026}/path/file.txt"
        );
    }

    #[test]
    fn test_center_truncate_handles_long_segment() {
        assert_eq!(
            center_truncate_path("~/supercalifragilisticexpialidocious", 18),
            "~/\u{2026}cexpialidocious"
        );
    }
}
