//! Comparing two shots at a checkpoint.

use super::normalize::Normalize;
use super::pty::Shot;

fn cursor_text(shot: &Shot) -> String {
    format!(
        "({}, {}) {}",
        shot.cursor.0,
        shot.cursor.1,
        if shot.cursor_visible { "visible" } else { "hidden" }
    )
}

/// The differences between the oracle's shot and codex-rtwk's, as lines for
/// a human; empty when they agree. Rows, the cursor and the screen in use
/// are compared; the scrollback only when `scrollback` is set.
pub fn compare(oracle: &Shot, rtwk: &Shot, n: &Normalize, scrollback: bool) -> Vec<String> {
    let (o, r) = (n.shot(oracle), n.shot(rtwk));
    let mut found = Vec::new();
    if o.rows.len() != r.rows.len() {
        found.push(format!("rows: oracle {}, rtwk {}", o.rows.len(), r.rows.len()));
    }
    for (y, (a, b)) in o.rows.iter().zip(&r.rows).enumerate() {
        if a != b {
            found.push(format!("row {}: oracle {a:?} | rtwk {b:?}", y + 1));
        }
    }
    if o.cursor != r.cursor || o.cursor_visible != r.cursor_visible {
        found.push(format!("cursor: oracle {} | rtwk {}", cursor_text(&o), cursor_text(&r)));
    }
    if o.alt_screen != r.alt_screen {
        found.push(format!("alternate screen: oracle {} | rtwk {}", o.alt_screen, r.alt_screen));
    }
    if scrollback {
        if o.scrollback.len() != r.scrollback.len() {
            found.push(format!(
                "scrollback: oracle {} lines, rtwk {} lines",
                o.scrollback.len(),
                r.scrollback.len()
            ));
        }
        for (i, (a, b)) in o.scrollback.iter().zip(&r.scrollback).enumerate() {
            if a != b {
                found.push(format!("scrollback line {}: oracle {a:?} | rtwk {b:?}", i + 1));
            }
        }
    }
    found
}

/// Both screens side by side (oracle left, codex-rtwk right), a `|` between
/// the columns and a `!` in front of rows that differ.
pub fn side_by_side(oracle: &Shot, rtwk: &Shot, n: &Normalize) -> String {
    let (o, r) = (n.shot(oracle), n.shot(rtwk));
    let width = o.rows.iter().map(|row| row.chars().count()).max().unwrap_or(0).max("oracle".len());
    let mut out = format!("  {:<width$} | rtwk\n", "oracle");
    let empty = String::new();
    for y in 0..o.rows.len().max(r.rows.len()) {
        let a = o.rows.get(y).unwrap_or(&empty);
        let b = r.rows.get(y).unwrap_or(&empty);
        let mark = if a == b { ' ' } else { '!' };
        out.push_str(&format!("{mark} {a:<width$} | {b}\n"));
    }
    out.push_str(&format!(
        "  {:<width$} | {}\n",
        format!("cursor {}", cursor_text(&o)),
        format_args!("cursor {}", cursor_text(&r)),
    ));
    out
}
