//! The platform labels of keys, the one normalization the kit makes to our
//! rows before it compares them with the oracle's (never to the oracle's
//! expected text). This file stands alone (no kit items): a test file outside
//! the kit includes it with `#[path = "kit/labels.rs"] mod labels;`.

/// Our rows as the oracle's snapshots hold them: the modifier labels
/// (`normalize_labels`) and the key names (`normalize_key_names`).
pub fn normalize(rows: Vec<String>, expected: &[String]) -> Vec<String> {
    let rows = rows
        .iter()
        .map(|row| normalize_labels(row, expected))
        .collect();
    normalize_key_names(rows, expected)
}

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

/// Key names the shipped oracle prints by platform, against the name the
/// oracle's `cfg(test)` snapshots hold: (ours, the snapshot's). The oracle's
/// key hint prints Backspace as `backspace` under `cfg(test)`, and as
/// crossterm's `Display` of the key as shipped, which is `Delete` on macOS
/// (`crossterm/src/event.rs`: `KeyCode::Backspace`), lower-cased `delete`; the
/// view prints the shipped name.
const KEY_NAMES: [(&str, &str); 1] = [("delete", "backspace")];

/// A key name in our rows becomes the snapshot's name, with the cells of
/// padding the help lays its key column out by the longest key: the name is
/// longer, so every row of that column moves its text right by the
/// difference. A name is changed only where the oracle's rows hold the
/// snapshot's name at the same row and column, followed by a description (a
/// key cell of a column), and only in the rows of that column; the oracle's
/// rows are read, never changed.
pub fn normalize_key_names(rows: Vec<String>, expected: &[String]) -> Vec<String> {
    let mut cells: Vec<Vec<char>> = rows.iter().map(|row| row.chars().collect()).collect();
    let wanted: Vec<Vec<char>> = expected.iter().map(|row| row.chars().collect()).collect();
    for (ours, theirs) in KEY_NAMES {
        let ours: Vec<char> = ours.chars().collect();
        let theirs: Vec<char> = theirs.chars().collect();
        let extra = theirs.len() - ours.len();
        // The column: a row where the oracle's key cell holds the snapshot's name
        // and ours holds ours; or, when the screen clips that row (the column is
        // still as wide as its longest key), a row whose key cell is the same
        // in both and whose description starts `extra` cells further right in
        // the oracle's.
        let Some((anchor, col)) = (0..wanted.len().min(cells.len())).find_map(|y| {
            let (want, got) = (&wanted[y], &cells[y]);
            if let Some(x) = key_cell(want, &theirs) {
                return (got.get(x..x + ours.len()) == Some(ours.as_slice())
                    && got.get(x + ours.len()) == Some(&' ')
                    && got.get(x + ours.len() + 1) == Some(&' '))
                .then_some((y, x));
            }
            let x = want.iter().position(|c| *c != ' ')?;
            let key: Vec<char> = want[x..]
                .iter()
                .take_while(|c| **c != ' ')
                .copied()
                .collect();
            let after = |row: &[char], width: usize| {
                let from = x + width + 2;
                (row.len() > from
                    && row[x + key.len()..from].iter().all(|c| *c == ' ')
                    && row[from] != ' ')
                    .then(|| {
                        row[from..]
                            .iter()
                            .take_while(|c| **c != ' ')
                            .collect::<String>()
                    })
            };
            (key.len() <= ours.len()
                && got.get(x..x + key.len()) == Some(key.as_slice())
                && after(got, ours.len()).is_some()
                && after(got, ours.len()) == after(want, theirs.len()))
            .then_some((y, x))
        }) else {
            continue;
        };
        // The rows of the column: from the anchor up and down while the oracle's
        // rows have a cell at its column.
        let filled = |y: usize| {
            wanted
                .get(y)
                .and_then(|r| r.get(col))
                .is_some_and(|c| *c != ' ')
        };
        let mut top = anchor;
        while top > 0 && filled(top - 1) {
            top -= 1;
        }
        let mut y = top;
        while y < cells.len() && filled(y) {
            let padded = wanted[y]
                .get(col + ours.len()..)
                .is_none_or(|tail| tail.iter().take(extra).all(|c| *c == ' '));
            let row = &mut cells[y];
            if row.get(col..col + ours.len()) == Some(ours.as_slice())
                && row.get(col + ours.len()) == Some(&' ')
            {
                row.splice(col..col + ours.len(), theirs.iter().copied());
            } else if padded
                && row.len() > col + ours.len() + 2
                && row[col + ours.len()..col + ours.len() + 2]
                    .iter()
                    .all(|c| *c == ' ')
            {
                row.splice(col + ours.len()..col + ours.len(), vec![' '; extra]);
            }
            y += 1;
        }
    }
    cells
        .into_iter()
        .map(|row| row.into_iter().collect())
        .collect()
}

/// Where `name` starts in `row` as a key cell: at a word start, followed by
/// two spaces and a description.
fn key_cell(row: &[char], name: &[char]) -> Option<usize> {
    let x = row.windows(name.len()).position(|w| w == name)?;
    let starts = x == 0 || !row[x - 1].is_alphanumeric();
    let after = row.get(x + name.len()..)?;
    (starts && after.starts_with(&[' ', ' ']) && after.iter().any(|c| *c != ' ')).then_some(x)
}
