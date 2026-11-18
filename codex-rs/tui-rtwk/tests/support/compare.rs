//! Comparing two shots at a checkpoint.

use super::normalize::Normalize;
use super::pty::Shot;

/// The differences between the oracle's shot and codex-rtwk's, as lines for
/// a human; empty when they agree. Rows and the cursor are compared;
/// the scrollback only when `scrollback` is set.
pub fn compare(oracle: &Shot, rtwk: &Shot, n: &Normalize, scrollback: bool) -> Vec<String> {
    let _ = (oracle, rtwk, n, scrollback);
    todo!("compare")
}

/// Both screens side by side (oracle left, codex-rtwk right), a `|` between
/// the columns and a `!` in front of rows that differ.
pub fn side_by_side(oracle: &Shot, rtwk: &Shot, n: &Normalize) -> String {
    let _ = (oracle, rtwk, n);
    todo!("side_by_side")
}
