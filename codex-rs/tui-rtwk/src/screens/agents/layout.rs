//! The command center's pure geometry and text fitting: the two-pane split,
//! the filter strip's window, word wrapping of the details.
//! Oracle: `app/agent_center/render.rs` (`center_layout`),
//! `bottom_pane/selection_tabs.rs` (`render_filled_tab_bar`) and
//! `agent_center/rows.rs` (`columns`).

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
        return Strip { start: 0, end: 0, left: false, right: false, widths: Vec::new() };
    }
    let active = active.min(count - 1);
    let widths: Vec<usize> = labels.iter().map(|l| l.chars().count() + 2).collect();
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
    Strip { start, end, left, right, widths: shown }
}

/// Greedy word wrap to `width` cells; a word longer than the row is cut.
pub fn wrap(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let mut word = word.to_string();
        while word.chars().count() > width {
            let head: String = word.chars().take(width).collect();
            word = word.chars().skip(width).collect();
            if !current.is_empty() {
                lines.push(std::mem::take(&mut current));
            }
            lines.push(head);
        }
        let need = current.chars().count() + usize::from(!current.is_empty()) + word.chars().count();
        if need > width && !current.is_empty() {
            lines.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(&word);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

/// The text cut to `width` cells with an ellipsis (oracle:
/// `truncate_line_with_ellipsis_if_overflow`).
pub fn ellipsize(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        return text.to_string();
    }
    if width == 0 {
        return String::new();
    }
    let kept: String = text.chars().take(width - 1).collect();
    format!("{kept}…")
}

/// A path cut to `max` cells, keeping its head and tail around one `…`
/// (oracle: `center_truncate_path`; the by-path carry of `text_formatting.rs`
/// replaces this once the crate root registers it).
pub fn center_truncate_path(path: &str, max: usize) -> String {
    let count = path.chars().count();
    if count <= max {
        return path.to_owned();
    }
    if max <= 1 {
        return "\u{2026}".repeat(max);
    }
    let tail = (max - 1) / 2 + (max - 1) % 2;
    let head = max - 1 - tail;
    let start: String = path.chars().take(head).collect();
    let end: String = path.chars().skip(count - tail).collect();
    format!("{start}\u{2026}{end}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_panes_from_ninety_body_cells() {
        let wide = areas(96, 25);
        assert_eq!((wide.list_width, wide.gap_x, wide.details_x), (51, Some(53), Some(56)));
        let narrow = areas(48, 25);
        assert_eq!((narrow.list_width, narrow.details_x), (44, None));
        assert_eq!((narrow.header, narrow.footer_row, narrow.body_height), (3, 24, 21));
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
}
