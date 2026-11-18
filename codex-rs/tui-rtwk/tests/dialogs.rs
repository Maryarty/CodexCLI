//! Screen tests of the `dialogs` slot (S23) and the oracle helper the dialog
//! slices of `screens-dialogs` share (`oracle`): the expected text of a test
//! is the oracle's own `.snap`, read from `codex-rs/tui/src`, never copied.

pub mod oracle {
    //! The oracle's snapshot text and the comparison with an rtwk screen.
    //!
    //! codex-tui draws these views as bottom-pane panes (no frame); tui-rtwk
    //! shows them in a `Dialog` (frame, title, its own cursor marker).
    //! `assert_rows` therefore compares the oracle's non-blank rows, trimmed,
    //! as an ordered subsequence of the screen's rows, and the cursor marker
    //! `› ` of the oracle is dropped (delta N01 in the parity pack: the cursor
    //! row is highlighted by style). Rows must otherwise match byte for byte.

    use std::path::PathBuf;

    /// The rows of the oracle snapshot `path` (relative to `codex-rs/tui/src`):
    /// header cut off, trailing blanks trimmed, blank rows dropped.
    pub fn rows(path: &str) -> Vec<String> {
        let full = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tui/src")
            .join(path);
        let text = std::fs::read_to_string(&full)
            .unwrap_or_else(|err| panic!("oracle snapshot {}: {err}", full.display()));
        let mut parts = text.splitn(3, "---\n");
        let _ = parts.next();
        let _ = parts.next();
        parts
            .next()
            .unwrap_or_default()
            .replace("\r\n", "\n")
            .lines()
            .map(|line| line.trim_end().to_string())
            .filter(|line| !line.trim().is_empty())
            .collect()
    }

    /// Every oracle row is on the screen, in order (see the module docs).
    pub fn assert_rows(screen: &str, path: &str) {
        let lines: Vec<&str> = screen.lines().collect();
        let mut at = 0;
        for row in rows(path) {
            let row = row.trim().strip_prefix("› ").unwrap_or(row.trim()).trim();
            let Some(found) = lines[at..].iter().position(|line| line.contains(row)) else {
                panic!("oracle row {row:?} ({path}) not found after line {at}; the screen:\n{screen}");
            };
            at += found + 1;
        }
    }
}

use crate::common::slot;

#[test]
fn dialogs_slot_is_registered_and_empty() {
    slot("dialogs");
}
