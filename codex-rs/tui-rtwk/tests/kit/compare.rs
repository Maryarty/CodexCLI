//! Comparing the screen with the oracle: the expected rows come only from an
//! oracle snapshot (`../tui/src/**/snapshots/*.snap`, found by name) or from
//! a literal of the oracle's own assertion. The only change made to the
//! screen's rows is the platform labels of keys (`normalize_labels`).

use super::grid::{self, Grid};
use super::oracle_snap::{self, Frame};

/// The labels of key hints differ by platform in the oracle (`⇧tab` on macOS
/// and under `cfg(test)`, `shift + tab` elsewhere); the snapshots mostly hold
/// the symbols. A word form in our rows becomes its symbol form, unless the
/// oracle's expected text itself holds that word form.
const LABELS: [(&str, &str); 4] = [("ctrl", "⌃"), ("shift", "⇧"), ("alt", "⌥"), ("super", "⌘")];

pub fn normalize_labels(row: &str, expected: &[String]) -> String {
    let mut out = row.to_string();
    for (word, symbol) in LABELS {
        let literal = expected
            .iter()
            .any(|line| line.contains(&format!("{word}+")) || line.contains(&format!("{word} +")));
        if literal {
            continue;
        }
        out = replace_label(&out, word, symbol);
    }
    out
}

/// `word+key` and `word + key` at a word start, as `symbolkey`.
fn replace_label(row: &str, word: &str, symbol: &str) -> String {
    let chars: Vec<char> = row.chars().collect();
    let w: Vec<char> = word.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let at_word_start = i == 0 || !chars[i - 1].is_alphanumeric();
        if at_word_start && chars[i..].starts_with(&w) {
            let mut j = i + w.len();
            let mut k = j;
            while chars.get(k) == Some(&' ') {
                k += 1;
            }
            if chars.get(k) == Some(&'+') {
                k += 1;
                while chars.get(k) == Some(&' ') {
                    k += 1;
                }
                if chars.get(k).is_some_and(|c| !c.is_whitespace()) {
                    j = k;
                    out.push_str(symbol);
                    i = j;
                    continue;
                }
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn trim(rows: Vec<String>) -> Vec<String> {
    let mut rows: Vec<String> = rows.into_iter().map(|r| r.trim_end().to_string()).collect();
    while rows.last().is_some_and(String::is_empty) {
        rows.pop();
    }
    rows
}

/// The size an oracle snapshot is stored at: (widest row, rows). Rows the
/// oracle trimmed make it smaller than the terminal the oracle test used;
/// take the size from the oracle test's code.
pub fn snap_size(name: &str) -> (usize, usize) {
    let rows = oracle_snap::rows(&oracle_snap::find(name));
    let width = rows.iter().map(|r| r.chars().count()).max().unwrap_or(0);
    (width, rows.len())
}

/// The names of the frames of a snapshot (`oracle_snap::frames`).
pub fn snap_frames(name: &str) -> Vec<String> {
    oracle_snap::frames(&oracle_snap::find(name))
        .into_iter()
        .map(|f| f.name)
        .collect()
}

/// The rows of one frame of a snapshot (`""` for the only frame).
pub fn snap_rows(name: &str, frame: &str) -> Vec<String> {
    oracle_snap::frame(&oracle_snap::find(name), frame).rows
}

/// What to compare the screen with: a snapshot, one frame of it, placed at a
/// row and column of the screen.
pub struct Want<'a> {
    pub name: &'a str,
    pub frame: &'a str,
    pub at: Option<(usize, usize)>,
}

/// The screen equals the oracle's snapshot: its rows (a wide glyph written as
/// the oracle wrote it), and, where the oracle's snapshot says them, the cell
/// styles of its `Buffer` dump and the cursor of its `Cursor:` line.
pub fn assert_snap(grid: &Grid, want: &Want<'_>) {
    let rel = oracle_snap::find(want.name);
    let frame = oracle_snap::frame(&rel, want.frame);
    let label = if want.frame.is_empty() {
        want.name.to_string()
    } else {
        format!("{} / {}", want.name, want.frame)
    };
    let (row0, col0) = want.at.unwrap_or((0, 0));
    // A wide glyph is one char in the oracle's Debug rows and a glyph and a
    // space in its buffer-to-text rows: one compare, either convention.
    let mut tries = vec![compare_rows(grid, &frame, want.at, false)];
    if grid.has_wide() && tries[0].is_err() {
        tries.push(compare_rows(grid, &frame, want.at, true));
    }
    if tries.iter().all(Result::is_err) {
        let nearest = tries
            .into_iter()
            .filter_map(Result::err)
            .min_by_key(|(actual, expected)| {
                let n = actual.len().max(expected.len());
                (0..n).filter(|&i| actual.get(i) != expected.get(i)).count()
            });
        if let Some((actual, expected)) = nearest {
            oracle_snap::assert_same(&label, &actual, &expected);
        }
    }
    if let Some(cursor) = frame.cursor {
        let ours = grid
            .cursor
            .map(|(x, y)| (x.saturating_sub(col0), y.saturating_sub(row0)));
        assert!(
            ours == cursor,
            "{label}: the cursor is at {ours:?}, the oracle's is {cursor:?}"
        );
    }
    if frame.styles.is_some() {
        let bad = style_diffs(grid, &frame, row0, col0);
        assert!(
            bad.is_empty(),
            "{label}: cell styles differ from the oracle's (ours vs oracle):\n{}",
            bad.join("\n")
        );
    }
}

type Rows = (Vec<String>, Vec<String>);

/// Ours (left) and the oracle's rows (right) when they differ.
fn compare_rows(
    grid: &Grid,
    frame: &Frame,
    at: Option<(usize, usize)>,
    wide_space: bool,
) -> Result<(), Rows> {
    let expected = frame.rows.clone();
    let (row0, col0) = at.unwrap_or((0, 0));
    let width = frame.area.map_or(grid.width, |(w, _)| col0 + w);
    let count = match (at, frame.area) {
        (Some(_), Some((_, h))) => h,
        (Some(_), None) => expected.len(),
        (None, _) => grid.height,
    };
    let actual = trim(
        (row0..(row0 + count).min(grid.height))
            .map(|y| normalize_labels(&grid.row(y, col0, width, wide_space), &expected))
            .collect(),
    );
    if actual == expected {
        Ok(())
    } else {
        Err((actual, expected))
    }
}

/// The cells the oracle's style listing covers, against ours: (what, ours,
/// the oracle's) for the first few that differ. A cell the oracle hid under a
/// wide glyph, and the second cell of ours, are not compared.
fn style_diffs(grid: &Grid, frame: &Frame, row0: usize, col0: usize) -> Vec<String> {
    let Some((w, h)) = frame.area else {
        return vec!["the oracle's dump has no area".to_string()];
    };
    let Some(runs) = &frame.styles else {
        return Vec::new();
    };
    let mut bad = Vec::new();
    let mut current = 0;
    for y in 0..h {
        for x in 0..w {
            while runs.get(current + 1).is_some_and(|r| (r.y, r.x) <= (y, x)) {
                current += 1;
            }
            let Some(run) = runs.get(current) else {
                return bad;
            };
            if (run.y, run.x) > (y, x) {
                continue;
            }
            let (gx, gy) = (col0 + x, row0 + y);
            if gx >= grid.width || gy >= grid.height {
                bad.push(format!("({x}, {y}): off our screen"));
                continue;
            }
            let hidden = frame.hidden.get(y).is_some_and(|c| c.contains(&x));
            if grid.cell(gx, gy).tail || hidden {
                continue;
            }
            let ours = grid.style(gx, gy);
            let same = grid::same_color(&run.fg, &ours.fg)
                && grid::same_color(&run.bg, &ours.bg)
                && grid::same_color(&run.underline, &ours.underline)
                && grid::same_modifier(&run.modifier, &ours.modifier);
            if !same && bad.len() < 8 {
                bad.push(format!(
                    "({x}, {y}) `{}`: ours fg {} bg {} underline {} modifier {}; oracle fg {} bg {} underline {} modifier {}",
                    grid.cell(gx, gy).text,
                    ours.fg, ours.bg, ours.underline, ours.modifier,
                    run.fg, run.bg, run.underline, run.modifier
                ));
            }
        }
    }
    bad
}
