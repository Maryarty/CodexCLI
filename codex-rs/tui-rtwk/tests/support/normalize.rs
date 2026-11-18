//! What differs between two runs of the same scenario without being a
//! difference of the program: timers, ids, paths, ports, spinner frames,
//! versions. Rows are also cut of trailing blanks.

use super::pty::Shot;

#[derive(Debug, Clone, Default)]
pub struct Normalize {
    literals: Vec<(String, String)>,
}

impl Normalize {
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace `what` (a path, a url) by `with`.
    pub fn literal(mut self, what: &str, with: &str) -> Self {
        self.literals.push((what.to_owned(), with.to_owned()));
        self
    }

    /// One row, normalized.
    pub fn row(&self, row: &str) -> String {
        let _ = (row, &self.literals);
        todo!("row")
    }

    /// A shot with every row (and scrollback line) normalized.
    pub fn shot(&self, shot: &Shot) -> Shot {
        let _ = shot;
        todo!("shot")
    }
}
