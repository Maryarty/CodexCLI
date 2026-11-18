//! Comparing the screen with the oracle: the expected rows come only from an
//! oracle snapshot (`../tui/src/**/snapshots/*.snap`, found by name) or from
//! a literal of the oracle's own assertion. The only change made to the
//! screen's rows is the platform labels of keys (`normalize_labels`).

use super::oracle_snap;

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

fn actual_rows(screen: &[String], expected: &[String]) -> Vec<String> {
    trim(
        screen
            .iter()
            .map(|row| normalize_labels(row, expected))
            .collect(),
    )
}

/// The size an oracle snapshot is stored at: (widest row, rows). Rows the
/// oracle trimmed make it smaller than the terminal the oracle test used;
/// take the size from the oracle test's code.
pub fn snap_size(name: &str) -> (usize, usize) {
    let rows = oracle_snap::rows(&oracle_snap::find(name));
    let width = rows.iter().map(|r| r.chars().count()).max().unwrap_or(0);
    (width, rows.len())
}

/// The whole screen equals the snapshot's rows.
pub fn assert_snap(screen: &[String], name: &str) {
    let expected = oracle_snap::rows(&oracle_snap::find(name));
    let actual = actual_rows(screen, &expected);
    oracle_snap::assert_same(name, &actual, &expected);
}

/// The snapshot's rows equal the screen's rows from `row` on, each cut of its
/// first `col` cells: a snapshot of one region of the oracle's screen.
pub fn assert_snap_region(screen: &[String], name: &str, row: usize, col: usize) {
    let expected = oracle_snap::rows(&oracle_snap::find(name));
    let cut: Vec<String> = actual_rows(screen, &expected)
        .iter()
        .skip(row)
        .take(expected.len())
        .map(|r| r.chars().skip(col).collect::<String>())
        .collect();
    oracle_snap::assert_same(name, &trim(cut), &expected);
}

/// The screen equals one `--- section ---` of a multi-state snapshot.
pub fn assert_snap_section(screen: &[String], name: &str, section: &str) {
    let rel = oracle_snap::find(name);
    let expected = oracle_snap::section(&rel, section);
    let actual = actual_rows(screen, &expected);
    oracle_snap::assert_same(&format!("{name} / {section}"), &actual, &expected);
}
