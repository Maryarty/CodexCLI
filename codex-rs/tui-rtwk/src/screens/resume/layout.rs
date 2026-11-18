//! The picker's pure geometry and text packing: the chrome's rows, relative
//! ages, the meta line of a row, the footer hints and the progress label.
//! Every function is a port of the oracle's, named after it; none touches a
//! widget or a string table (the view passes the texts in).
//! Oracle: `resume_picker.rs` and `resume_picker/layout.rs`.

/// The rows of the picker's chrome (oracle: `layout::areas`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Areas {
    pub toolbar: u16,
    pub search: u16,
    pub list: u16,
    pub list_height: u16,
    pub footer: u16,
}

const FIXED_CHROME_ROWS: u16 = 7;
const MIN_LIST_ROWS: u16 = 6;
/// The list sits two cells in (oracle: `PICKER_LIST_HORIZONTAL_INSET`).
pub const LIST_INSET: u16 = 2;

/// Decorative gaps collapse before the list loses room for two comfortable
/// rows.
pub fn areas(height: u16) -> Areas {
    let gaps = height
        .saturating_sub(FIXED_CHROME_ROWS + MIN_LIST_ROWS)
        .min(3);
    let toolbar = 1 + u16::from(gaps >= 3);
    let search = toolbar + 1 + u16::from(gaps >= 2);
    let list = search + 1 + u16::from(gaps >= 1);
    let list_height = height.saturating_sub(FIXED_CHROME_ROWS + gaps);
    Areas {
        toolbar,
        search,
        list,
        list_height,
        footer: list + list_height,
    }
}

/// An age (oracle: `format_relative_time`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Age {
    Missing,
    Now,
    Seconds(i64),
    Minutes(i64),
    Hours(i64),
    Days(i64),
}

pub fn age(clock: i64, ts: Option<i64>) -> Age {
    let Some(ts) = ts else {
        return Age::Missing;
    };
    let seconds = (clock - ts).max(0);
    if seconds == 0 {
        return Age::Now;
    }
    if seconds < 60 {
        return Age::Seconds(seconds);
    }
    let minutes = seconds / 60;
    if minutes < 60 {
        return Age::Minutes(minutes);
    }
    let hours = minutes / 60;
    if hours < 24 {
        return Age::Hours(hours);
    }
    Age::Days(hours / 24)
}

fn width(text: &str) -> usize {
    text.chars().count()
}

/// Oracle: `text_formatting::truncate_text`, over chars instead of
/// graphemes (a delta only for combining text).
pub fn truncate(text: &str, max: usize) -> String {
    let count = width(text);
    if count <= max {
        return text.to_string();
    }
    if max >= 3 {
        let kept: String = text.chars().take(max - 3).collect();
        format!("{kept}...")
    } else {
        text.chars().take(max).collect()
    }
}

const META_INDENT: usize = 2;
const META_DATE_WIDTH: usize = 12;
const META_GAP: usize = 2;
const META_MIN_CWD: usize = 30;
const META_MAX_CWD: usize = 72;

/// One field of a row's meta line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Part {
    Date(String),
    Cwd(Option<String>, &'static str),
    Branch(Option<String>, &'static str),
}

/// The icons before a cwd and a branch (oracle: `SESSION_META_*_ICON`).
const CWD_ICON: &str = "⌁";
const BRANCH_ICON: &str = "\u{e0a0}";

impl Part {
    fn text(&self) -> &str {
        match self {
            Part::Date(text) => text,
            Part::Cwd(Some(text), _) | Part::Branch(Some(text), _) => text,
            Part::Cwd(None, none) | Part::Branch(None, none) => none,
        }
    }

    fn prefix(&self) -> Option<&'static str> {
        match self {
            Part::Date(_) => None,
            Part::Cwd(..) => Some(CWD_ICON),
            Part::Branch(..) => Some(BRANCH_ICON),
        }
    }

    fn is_none(&self) -> bool {
        matches!(self, Part::Cwd(None, _) | Part::Branch(None, _))
    }
}

/// A run of one meta line: its text and whether it is the italic "none".
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Seg {
    pub text: String,
    pub italic: bool,
}

fn cwd_column_width(w: usize) -> usize {
    let available = w.saturating_sub(META_INDENT + META_DATE_WIDTH + 2 * META_GAP);
    (available / 2).clamp(META_MIN_CWD, META_MAX_CWD)
}

fn part_width(part: &Part, padded: bool, cwd_width: usize) -> usize {
    let prefix = part.prefix().map_or(0, width);
    let gap = usize::from(part.prefix().is_some() && !part.text().is_empty());
    let actual = prefix + gap + width(part.text());
    match part {
        Part::Date(_) if padded => META_DATE_WIDTH.max(actual),
        Part::Cwd(..) if padded => cwd_width,
        _ => actual,
    }
}

fn parts_width(parts: &[Part], cwd_width: usize) -> usize {
    META_INDENT
        + parts
            .iter()
            .enumerate()
            .map(|(i, part)| part_width(part, i + 1 < parts.len(), cwd_width))
            .sum::<usize>()
}

fn line(parts: Vec<Part>, w: usize, cwd_width: usize) -> Vec<Seg> {
    let plain = |text: String| Seg { text, italic: false };
    let mut segs = vec![plain("  ".to_string())];
    let mut remaining = w.saturating_sub(META_INDENT);
    let count = parts.len();
    for (i, part) in parts.into_iter().enumerate() {
        if i > 0 {
            let gap = META_GAP.min(remaining);
            if gap > 0 {
                segs.push(plain(" ".repeat(gap)));
                remaining -= gap;
            }
        }
        let padded = i + 1 < count;
        let target = match part {
            Part::Date(_) if padded => Some(META_DATE_WIDTH),
            Part::Cwd(..) if padded => Some(cwd_width),
            _ => None,
        };
        let used = push_part(&mut segs, &part, target, remaining);
        remaining = remaining.saturating_sub(used);
        if let Some(target) = target {
            let padding = target.saturating_sub(used);
            if padding > 0 {
                segs.push(plain(" ".repeat(padding)));
                remaining = remaining.saturating_sub(padding);
            }
        }
    }
    segs
}

fn push_part(segs: &mut Vec<Seg>, part: &Part, target: Option<usize>, available: usize) -> usize {
    let text = part.text().to_string();
    let Some(prefix) = part.prefix() else {
        let text = truncate(&text, available);
        let used = width(&text);
        segs.push(Seg { text, italic: false });
        return used;
    };
    let prefix_width = width(prefix);
    if available <= prefix_width {
        let prefix = truncate(prefix, available);
        let used = width(&prefix);
        segs.push(Seg { text: prefix, italic: false });
        return used;
    }
    segs.push(Seg { text: prefix.to_string(), italic: false });
    let mut used = prefix_width;
    if !text.is_empty() && used < available {
        segs.push(Seg { text: " ".to_string(), italic: false });
        used += 1;
    }
    let room = target
        .unwrap_or(available)
        .saturating_sub(used)
        .min(available.saturating_sub(used));
    let text = truncate(&text, room);
    let shown = width(&text);
    segs.push(Seg { text, italic: part.is_none() });
    used + shown
}

/// The meta lines of a row at `w` cells (oracle: `pack_footer_parts`).
pub fn pack(parts: Vec<Part>, w: usize) -> Vec<Vec<Seg>> {
    if w <= META_INDENT {
        return Vec::new();
    }
    let cwd_width = cwd_column_width(w);
    if parts_width(&parts, cwd_width) <= w {
        return vec![line(parts, w, cwd_width)];
    }
    let mut lines = Vec::new();
    let mut current: Vec<Part> = Vec::new();
    for part in parts {
        let mut candidate = std::mem::take(&mut current);
        candidate.push(part);
        if candidate.len() > 1 && parts_width(&candidate, cwd_width) > w {
            let last = candidate.split_off(candidate.len() - 1);
            lines.push(line(candidate, w, cwd_width));
            candidate = last;
        }
        current = candidate;
    }
    if !current.is_empty() {
        lines.push(line(current, w, cwd_width));
    }
    lines
}

/// One footer hint (oracle: `PickerFooterHint`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hint {
    pub key: String,
    pub wide: String,
    pub compact: String,
    pub priority: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Wide,
    Compact,
    KeyOnly,
}

const LEFT_PAD: usize = 1;
const GAP_WIDTH: usize = 3;
const COMPACT_BREAKPOINT: usize = 120;

fn fit(hints: &[&Hint], mode: Mode, w: usize) -> Option<String> {
    let total = LEFT_PAD
        + hints
            .iter()
            .enumerate()
            .map(|(i, hint)| {
                let label = match mode {
                    Mode::Wide => 1 + width(&hint.wide),
                    Mode::Compact => 1 + width(&hint.compact),
                    Mode::KeyOnly => 0,
                };
                width(&hint.key) + label + if i == 0 { 0 } else { GAP_WIDTH }
            })
            .sum::<usize>();
    if total > w {
        return None;
    }
    let mut text = " ".repeat(LEFT_PAD);
    for (i, hint) in hints.iter().enumerate() {
        if i > 0 {
            text.push_str(&" ".repeat(GAP_WIDTH));
        }
        text.push_str(&hint.key);
        match mode {
            Mode::Wide => {
                text.push(' ');
                text.push_str(&hint.wide);
            }
            Mode::Compact => {
                text.push(' ');
                text.push_str(&hint.compact);
            }
            Mode::KeyOnly => {}
        }
    }
    Some(text)
}

/// A row of hints fitted to `w` (oracle: `hint_line_for_row`): wide labels
/// from 120 cells, else compact, else the hints of the lowest priority
/// numbers that fit.
pub fn hint_row(hints: &[Hint], w: usize) -> String {
    let all: Vec<&Hint> = hints.iter().collect();
    if w >= COMPACT_BREAKPOINT
        && let Some(text) = fit(&all, Mode::Wide, w)
    {
        return text;
    }
    if let Some(text) = fit(&all, Mode::Compact, w) {
        return text;
    }
    let mut retained: Vec<usize> = (0..hints.len()).collect();
    retained.sort_by_key(|i| hints[*i].priority);
    for mode in [Mode::Compact, Mode::KeyOnly] {
        for keep in (1..=retained.len()).rev() {
            let mut picked = retained[..keep].to_vec();
            picked.sort_unstable();
            let candidate: Vec<&Hint> = picked.iter().map(|i| &hints[*i]).collect();
            if let Some(text) = fit(&candidate, mode, w) {
                return text;
            }
        }
    }
    String::new()
}

/// The three spellings of the footer's progress, widest first, the first
/// that is narrower than `w` (oracle: `picker_footer_progress_label`).
pub fn progress_label(position: usize, total: &str, percent: u8, w: usize) -> String {
    [
        format!(" {position} / {total} · {percent}% "),
        format!(" {position}/{total} · {percent}% "),
        format!(" {percent}% "),
    ]
    .into_iter()
    .find(|label| width(label) < w)
    .unwrap_or_default()
}

/// The footer rule with the label near its right end.
pub fn rule(w: usize, label: &str) -> String {
    let label_width = width(label);
    if label_width < w {
        format!("{}{label}─", "─".repeat(w - label_width - 1))
    } else {
        "─".repeat(w)
    }
}

/// How many screen rows a row takes: its title line and meta lines.
pub fn row_height(meta_lines: usize) -> usize {
    1 + meta_lines
}

/// The rows the list can fill: the viewport less the "more" lines (oracle:
/// `available_content_rows`).
fn capacity(top: usize, selected: usize, len: usize, viewport: usize) -> usize {
    viewport
        .saturating_sub(usize::from(top > 0))
        .saturating_sub(usize::from(selected + 1 < len))
        .max(1)
}

/// Oracle: `ensure_selected_visible`, with the heights of the rows in
/// `heights` (title and meta lines) and one separator row between rows.
pub fn ensure_visible(scroll_top: usize, selected: usize, heights: &[usize], viewport: usize) -> usize {
    if heights.is_empty() {
        return 0;
    }
    let mut top = scroll_top.min(heights.len() - 1);
    if selected < top {
        top = selected;
    }
    while top < selected
        && between(heights, top, selected) > capacity(top, selected, heights.len(), viewport)
    {
        top += 1;
    }
    top
}

/// Oracle: `has_more_below`.
pub fn more_below(top: usize, selected: usize, heights: &[usize], viewport: usize) -> bool {
    if heights.is_empty() {
        return false;
    }
    let capacity = capacity(top, selected, heights.len(), viewport);
    let mut used = 0;
    for (offset, height) in heights[top..].iter().enumerate() {
        let separator = usize::from(offset > 0);
        if used + separator + height > capacity {
            return true;
        }
        used += separator + height;
    }
    false
}

/// The rendered height from `start` to `end` inclusive with separators.
pub fn between(heights: &[usize], start: usize, end: usize) -> usize {
    heights[start..=end].iter().sum::<usize>() + (end - start)
}

/// Oracle: `picker_footer_scroll_percent`.
pub fn scroll_percent(scroll_top: usize, heights: &[usize], viewport: usize, selected: usize) -> u8 {
    if heights.is_empty() {
        return 100;
    }
    let content = capacity(scroll_top, selected, heights.len(), viewport);
    let last = heights.len() - 1;
    let total = between(heights, 0, last);
    let max_scroll = total.saturating_sub(content);
    if max_scroll == 0 {
        return 100;
    }
    if between(heights, scroll_top, last) <= content {
        return 100;
    }
    let skipped = if scroll_top == 0 {
        0
    } else {
        between(heights, 0, scroll_top - 1)
    };
    (((skipped.min(max_scroll)) as f32 / max_scroll as f32) * 100.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chrome_rows_collapse_with_the_height() {
        let tall = areas(22);
        assert_eq!((tall.toolbar, tall.search, tall.list, tall.list_height), (2, 4, 6, 12));
        assert_eq!(tall.footer, 18);
        let short = areas(13);
        assert_eq!((short.toolbar, short.search, short.list, short.list_height), (1, 2, 3, 6));
        assert_eq!(short.footer, 9);
    }

    #[test]
    fn ages_follow_the_oracle() {
        assert_eq!(age(100, None), Age::Missing);
        assert_eq!(age(100, Some(100)), Age::Now);
        assert_eq!(age(100, Some(58)), Age::Seconds(42));
        assert_eq!(age(10_000, Some(10_000 - 35 * 60)), Age::Minutes(35));
        assert_eq!(age(100_000, Some(100_000 - 7200)), Age::Hours(2));
        assert_eq!(age(1_000_000, Some(0)), Age::Days(11));
    }
}
