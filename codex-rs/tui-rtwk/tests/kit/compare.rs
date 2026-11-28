//! Comparing the screen with the oracle: the expected rows come only from an
//! oracle snapshot (`../tui/src/**/snapshots/*.snap`, found by name) or from
//! a literal of the oracle's own assertion. The only change made to the
//! screen's rows is the platform labels of keys (`labels::normalize`).

use super::grid::{self, Grid};
use super::labels;
use super::oracle_snap::{self, Frame};
use super::verdict;

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

/// The rows of each state of a snapshot cut at its bare `---` rules (`oracle_snap::cut`), in the
/// order the test wrote them.
pub fn snap_cuts(name: &str) -> Vec<Vec<String>> {
    oracle_snap::cut(&oracle_snap::find(name))
        .into_iter()
        .map(|f| f.rows)
        .collect()
}

/// The rows of each body of a snapshot the test glued of bodies of different heights with a
/// blank row between (`oracle_snap::states`, `panel(..)` bodies joined by `"\n\n"`): `heights[i]`
/// rows for body `i`, in the order the test wrote them.
pub fn snap_states(name: &str, heights: &[usize]) -> Vec<Vec<String>> {
    let rel = oracle_snap::find(name);
    (0..heights.len())
        .map(|index| oracle_snap::states(&rel, heights, index).rows)
        .collect()
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
    let frame = oracle_snap::frame(&oracle_snap::find(want.name), want.frame);
    assert_frame(grid, want, frame, want.at);
}

/// `assert_snap` (or `assert_dialog`) for a snapshot the test glued of several renders under
/// labels of its own: the frame is read by `oracle_snap::frame_labelled`.
pub fn assert_labelled(grid: &Grid, want: &Want<'_>, labels: &[&str], dialog: bool) {
    let frame = oracle_snap::frame_labelled(&oracle_snap::find(want.name), labels, want.frame);
    if dialog {
        let (frame, at) = dialog_region(grid, frame);
        assert_frame(grid, want, frame, at);
    } else {
        assert_frame(grid, want, frame, want.at);
    }
}

/// `assert_snap` for a snapshot of states of one screen glued by a blank row with no header
/// (`oracle_snap::state`): the screen equals state `index`, each state `height` rows.
pub fn assert_state(grid: &Grid, want: &Want<'_>, height: usize, index: usize) {
    let frame = oracle_snap::state(&oracle_snap::find(want.name), height, index);
    assert_frame(grid, want, frame, want.at);
}

/// `assert_snap` for a snapshot of bodies of different heights glued by a blank row with no
/// header (`oracle_snap::states`, `panel(..)` bodies joined by `"\n\n"`): the screen equals state
/// `index`, state `i` being `heights[i]` rows.
pub fn assert_states(grid: &Grid, want: &Want<'_>, heights: &[usize], index: usize) {
    let frame = oracle_snap::states(&oracle_snap::find(want.name), heights, index);
    assert_frame(grid, want, frame, want.at);
}

/// `assert_snap` for a snapshot the test joined of renders with bare `---` rules and no header
/// (`oracle_snap::cut_state`): the screen equals state `index` (from 0, in the order the test
/// wrote them); `dialog` reads it as `assert_dialog` does.
pub fn assert_cut(grid: &Grid, want: &Want<'_>, index: usize, dialog: bool) {
    let frame = oracle_snap::cut_state(&oracle_snap::find(want.name), index);
    if dialog {
        let (frame, at) = dialog_region(grid, frame);
        assert_frame(grid, want, frame, at);
    } else {
        assert_frame(grid, want, frame, want.at);
    }
}

/// `assert_snap` for a frame the oracle drew into the whole area (spare rows
/// above its footer on the last row) where ours is a dialog as tall as its
/// content (`dialog_region`).
pub fn assert_dialog(grid: &Grid, want: &Want<'_>) {
    let frame = oracle_snap::frame(&oracle_snap::find(want.name), want.frame);
    let (frame, at) = dialog_region(grid, frame);
    assert_frame(grid, want, frame, at);
}

fn assert_frame(grid: &Grid, want: &Want<'_>, frame: Frame, at: Option<(usize, usize)>) {
    assert_frame_rows(grid, want, frame, at, None);
}

/// `assert_snap` for a snapshot the oracle drew as the text alone, of which ours is the rows
/// from `want.at` to the end of `grid`: ours may be shorter or longer than the snapshot (a
/// blank row between blocks, a wrap), and all of its rows are compared.
pub fn assert_snap_rows_to_end(grid: &Grid, want: &Want<'_>) {
    let frame = oracle_snap::frame(&oracle_snap::find(want.name), want.frame);
    let row0 = want.at.map_or(0, |(row, _)| row);
    let rows = grid.height.saturating_sub(row0);
    assert_frame_rows(grid, want, frame, want.at, Some(rows));
}

fn assert_frame_rows(
    grid: &Grid,
    want: &Want<'_>,
    mut frame: Frame,
    at: Option<(usize, usize)>,
    rows: Option<usize>,
) {
    // The oracle's tests replace its own version with `<VERSION>` before they snapshot; the
    // port's heading says the version of this build.
    if let Some(version) = codex_tui_rtwk::services::cli_help::VERSION
        .split_whitespace()
        .last()
    {
        for row in &mut frame.rows {
            *row = row.replace("<VERSION>", version);
        }
    }
    let label = if want.frame.is_empty() {
        want.name.to_string()
    } else {
        format!("{} / {}", want.name, want.frame)
    };
    let (row0, col0) = at.unwrap_or((0, 0));
    // A locator that found no row (`position(..)` unwrapped to MAX) is a wrong region, not a diff.
    assert!(
        row0 <= grid.rows(false).len() && col0 <= usize::from(u16::MAX),
        "{label}: the region starts at row {row0}, the screen has {} rows (the row the test looked for is not on the screen):\n{}",
        grid.rows(false).len(),
        grid.text()
    );
    // A wide glyph is one char in the oracle's Debug rows and a glyph and a
    // space in its buffer-to-text rows: one compare, either convention.
    let mut tries = vec![compare_rows(grid, &frame, at, rows, false)];
    if grid.has_wide() && tries[0].is_err() {
        tries.push(compare_rows(grid, &frame, at, rows, true));
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
            verdict::screen(&actual, &expected, || {
                oracle_snap::assert_same(&label, &actual, &expected);
            });
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
        verdict::screen_styles(|| {
            assert!(
                bad.is_empty(),
                "{label}: cell styles differ from the oracle's (ours vs oracle):\n{}",
                bad.join("\n")
            );
        });
    }
}

/// The part of a full-area frame that a dialog as tall as its content shows: the
/// rows from the blank ones above the first text to the last text, one blank
/// row (the spacer above the hint), the footer on the frame's last row; placed
/// where our first text row sits. Without that text on our screen the region
/// starts at row `usize::MAX`, which `assert_snap` reports as a wrong region.
fn dialog_region(grid: &Grid, mut frame: Frame) -> (Frame, Option<(usize, usize)>) {
    let Some(footer) = frame.rows.pop() else {
        return (frame, None);
    };
    while frame.rows.last().is_some_and(String::is_empty) {
        frame.rows.pop();
    }
    let lead = frame.rows.iter().take_while(|r| r.is_empty()).count();
    let first = frame.rows.get(lead).cloned().unwrap_or_default();
    let top = (0..grid.height)
        .find(|&y| grid.row(y, 0, grid.width, false).trim_end() == first)
        .map_or(usize::MAX, |y| y.saturating_sub(lead));
    frame.rows.push(String::new());
    frame.rows.push(footer);
    (frame, Some((top, 0)))
}

type Rows = (Vec<String>, Vec<String>);

/// Ours (left) and the oracle's rows (right) when they differ.
fn compare_rows(
    grid: &Grid,
    frame: &Frame,
    at: Option<(usize, usize)>,
    rows: Option<usize>,
    wide_space: bool,
) -> Result<(), Rows> {
    let expected = frame.rows.clone();
    let (row0, col0) = at.unwrap_or((0, 0));
    let width = frame.area.map_or(grid.width, |(w, _)| col0 + w);
    let count = rows.unwrap_or(match (at, frame.area) {
        (Some(_), Some((_, h))) => h,
        (Some(_), None) => expected.len(),
        (None, _) => grid.height,
    });
    let actual = trim(labels::normalize(
        (row0..(row0 + count).min(grid.height))
            .map(|y| grid.row(y, col0, width, wide_space))
            .collect(),
        &expected,
    ));
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
