//! What differs between two runs of the same scenario without being a
//! difference of the program: timers, ids, paths, ports, spinner frames,
//! versions. Rows are also cut of trailing blanks.

use super::pty::Shot;

#[derive(Debug, Clone, Default)]
pub struct Normalize {
    /// Literal text to replace: (what, with).
    literals: Vec<(String, String)>,
}

impl Normalize {
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace `what` (a path, a url) by `with`.
    pub fn literal(mut self, what: &str, with: &str) -> Self {
        if !what.is_empty() {
            self.literals.push((what.to_owned(), with.to_owned()));
        }
        self
    }

    /// One row, normalized.
    pub fn row(&self, row: &str) -> String {
        let mut literals: Vec<&(String, String)> = self.literals.iter().collect();
        literals.sort_by_key(|(what, _)| std::cmp::Reverse(what.len()));
        let mut text = row.to_owned();
        for (what, with) in literals {
            text = text.replace(what.as_str(), with);
        }
        tokens(text.trim_end()).trim_end().to_owned()
    }

    /// A shot with every row (and scrollback line) normalized.
    pub fn shot(&self, shot: &Shot) -> Shot {
        Shot {
            rows: shot.rows.iter().map(|r| self.row(r)).collect(),
            scrollback: shot.scrollback.iter().map(|r| self.row(r)).collect(),
            ..shot.clone()
        }
    }
}

fn is_hex(c: char) -> bool {
    c.is_ascii_hexdigit()
}

/// Whether `chars[at..]` starts a uuid (8-4-4-4-12 hex) ending at a boundary.
fn uuid_len(chars: &[char], at: usize) -> Option<usize> {
    const SHAPE: [usize; 5] = [8, 4, 4, 4, 12];
    let mut i = at;
    for (k, len) in SHAPE.iter().enumerate() {
        if k > 0 {
            if chars.get(i) != Some(&'-') {
                return None;
            }
            i += 1;
        }
        if !(0..*len).all(|j| chars.get(i + j).is_some_and(|c| is_hex(*c))) {
            return None;
        }
        i += len;
    }
    boundary_after(chars, i).then_some(i - at)
}

fn boundary_after(chars: &[char], i: usize) -> bool {
    chars.get(i).is_none_or(|c| !c.is_alphanumeric())
}

/// `v0.170.0` → its length.
fn version_len(chars: &[char], at: usize) -> Option<usize> {
    if chars[at] != 'v' {
        return None;
    }
    let mut i = at + 1;
    for part in 0..3 {
        if part > 0 {
            if chars.get(i) != Some(&'.') {
                return None;
            }
            i += 1;
        }
        let start = i;
        while chars.get(i).is_some_and(char::is_ascii_digit) {
            i += 1;
        }
        if i == start {
            return None;
        }
    }
    boundary_after(chars, i).then_some(i - at)
}

/// `12s`, `1.5s`, `3m`, `100ms`, `2h`, `4d` → its length.
fn duration_len(chars: &[char], at: usize) -> Option<usize> {
    let mut i = at;
    while chars.get(i).is_some_and(char::is_ascii_digit) {
        i += 1;
    }
    if i == at {
        return None;
    }
    if chars.get(i) == Some(&'.') && chars.get(i + 1).is_some_and(char::is_ascii_digit) {
        i += 1;
        while chars.get(i).is_some_and(char::is_ascii_digit) {
            i += 1;
        }
    }
    let unit = if chars.get(i) == Some(&'m') && chars.get(i + 1) == Some(&'s') {
        2
    } else if chars.get(i).is_some_and(|c| matches!(c, 's' | 'm' | 'h' | 'd')) {
        1
    } else {
        return None;
    };
    i += unit;
    boundary_after(chars, i).then_some(i - at)
}

/// `11:59`, `9:05` (one or two digits, a colon, two digits) → its length.
fn clock_len(chars: &[char], at: usize) -> Option<usize> {
    let digits = |from: usize| chars[from..].iter().take_while(|c| c.is_ascii_digit()).count();
    let hours = digits(at);
    if !(1..=2).contains(&hours) || chars.get(at + hours) != Some(&':') {
        return None;
    }
    let minutes = at + hours + 1;
    if digits(minutes) != 2 {
        return None;
    }
    boundary_after(chars, minutes + 2).then_some(hours + 3)
}

fn is_spinner(c: char) -> bool {
    ('\u{2800}'..='\u{28ff}').contains(&c)
}

fn tokens(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let at_boundary = i == 0 || !chars[i - 1].is_alphanumeric();
        if at_boundary {
            if let Some(len) = uuid_len(&chars, i) {
                out.push_str("<uuid>");
                i += len;
                continue;
            }
            if let Some(len) = version_len(&chars, i) {
                out.push_str("vX");
                i += len;
                continue;
            }
            if let Some(len) = clock_len(&chars, i) {
                out.push_str("<clock>");
                i += len;
                continue;
            }
            if let Some(mut len) = duration_len(&chars, i) {
                // "1m 23s" is one timer
                while chars.get(i + len) == Some(&' ')
                    && let Some(next) = duration_len(&chars, i + len + 1)
                {
                    len += 1 + next;
                }
                out.push_str("<t>");
                i += len;
                continue;
            }
        }
        if is_spinner(chars[i]) {
            out.push_str("<spin>");
            while i < chars.len() && is_spinner(chars[i]) {
                i += 1;
            }
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}
