//! Expectations of the oracle's tests that are neither a snapshot nor a word on the screen: a
//! chart's rows at an unknown shift, the body of a page from an anchor, a widget drawn at the
//! rows a layout leaves it. A shard writes no `assert!`; each is spelled once here.

use codex_tui_rtwk::app::theme::{Tokens, data};
use codex_tui_rtwk::widgets::status_line::status_spans;
use rtwk::layout::{Expanded, Row, SizedBox};
use rtwk::style::{Span, Style};
use rtwk::text::TextOverflow;
use rtwk::widgets::Text;
use rtwk_core::terminal::BidiReordering;
use rtwk_test::{Options, Tester};

use super::{Screen, snap_rows};

/// `got` is `want`, `why` says which.
pub fn assert_same<T: PartialEq + std::fmt::Debug>(got: T, want: T, why: &str) {
    pretty_assertions::assert_eq!(got, want, "{why}");
}

/// `got` is not `other`, `why` says which.
pub fn assert_differs<T: PartialEq + std::fmt::Debug>(got: T, other: T, why: &str) {
    pretty_assertions::assert_ne!(got, other, "{why}");
}

impl Screen {
    /// The chart's rows as the oracle draws them (`chart_lines` at a graph width, trailing blanks
    /// cut), found anywhere on the screen: the first row is the one that ends with the first
    /// line, the page's inset is the shift of that row, and each following row is the next line
    /// shifted the same. `clipped` lets the last row (a caption wider than the graph:
    /// `chart_lines` does not clip, the page does at the graph's width) lose its tail.
    pub fn assert_chart(&self, want: &[&str], clipped: bool) {
        let rows = self.rows();
        let first = want[0].trim_end();
        let at = rows
            .iter()
            .position(|row| row.trim_end().ends_with(first.trim_start()))
            .unwrap_or_else(|| panic!("no row ends with `{first}` on\n{}", self.text()));
        let shift = rows[at].trim_end().chars().count() - first.chars().count();
        for (i, line) in want.iter().enumerate() {
            let expected = if line.trim().is_empty() {
                String::new()
            } else {
                format!("{}{}", " ".repeat(shift), line.trim_end())
            };
            let got = rows.get(at + i).map_or("", |row| row.trim_end());
            if clipped && i + 1 == want.len() {
                assert!(
                    expected.starts_with(got) && got.trim().chars().count() >= 15,
                    "chart caption {got:?} is not {expected:?} clipped:\n{}",
                    self.text()
                );
            } else {
                pretty_assertions::assert_eq!(got, expected, "chart row {i}:\n{}", self.text());
            }
        }
    }

    /// The `want.len()` rows from the row that starts with `from` (the page's one-column inset
    /// and trailing blanks cut) are `want`.
    pub fn assert_body_from(&self, from: &str, want: &[String], why: &str) {
        let screen = self.rows();
        let at = screen
            .iter()
            .position(|row| row.trim_start().starts_with(from))
            .unwrap_or_else(|| panic!("no `{from}` on\n{}", self.text()));
        let got: Vec<String> = screen
            .iter()
            .skip(at)
            .take(want.len())
            .map(|row| row.strip_prefix(' ').unwrap_or(row).trim_end().to_owned())
            .collect();
        let want: Vec<String> = want.iter().map(|row| row.trim_end().to_owned()).collect();
        pretty_assertions::assert_eq!(got, want, "{why}:\n{}", self.text());
    }

    /// Some row holds both `left` and `right`.
    pub fn assert_row_with_both(&self, left: &str, right: &str) {
        assert!(
            self.rows()
                .iter()
                .any(|row| row.contains(left) && row.contains(right)),
            "{left} {right}:\n{}",
            self.text()
        );
    }

    /// The last row of the screen holds `text`.
    pub fn assert_last_row_has(&self, text: &str) {
        let rows = self.rows();
        assert!(
            rows.last().is_some_and(|row| row.contains(text)),
            "the last row has no `{text}`:\n{}",
            self.text()
        );
    }

    /// The cursor, where the terminal shows one, is on the row that holds `text`.
    pub fn assert_cursor_on_row_of(&self, text: &str) {
        let Some((_, y)) = self.grid().cursor else {
            return;
        };
        let rows = self.rows();
        pretty_assertions::assert_eq!(
            Some(y),
            rows.iter().position(|row| row.contains(text)),
            "the cursor's row:\n{}",
            self.text()
        );
    }

    /// The rows where a row starts with one of the card edges (`╭`, `╰`).
    pub fn card_edge_rows(&self) -> Vec<usize> {
        self.rows()
            .iter()
            .enumerate()
            .filter(|(_, row)| row.trim_start().starts_with(['\u{256d}', '\u{2570}']))
            .map(|(at, _)| at)
            .collect()
    }
}

/// A status line handed to the host as a widget: `prefix` (no link) and the status text under
/// `url`, drawn at 18 columns with the rows each layout leaves the status (4, 3, 2, 1), against
/// the oracle's frames of `snap` (`---` between the four).
pub struct NativeStatus {
    frames: Vec<Vec<String>>,
    drawn: Vec<Vec<String>>,
    ellipsis_links: Vec<Option<String>>,
}

const INDENT: usize = 2;

pub fn native_status(snap: &str, url: &str, prefix: &str, status: &str) -> NativeStatus {
    let tokens = Tokens::new(data());
    let mut spans = vec![Span::styled(prefix.to_string(), Style::default())];
    spans.extend(status_spans(
        [("pull-request-number", status.to_string())],
        false,
        &tokens,
        None,
        |_| Some(url.to_string()),
    ));
    let mut frames: Vec<Vec<String>> = vec![Vec::new()];
    for row in snap_rows(snap, "") {
        if row.trim() == "---" {
            frames.push(Vec::new());
        } else if let Some(frame) = frames.last_mut() {
            frame.push(row.trim().to_owned());
        }
    }
    assert_eq!(frames.len(), 4, "the oracle's four layouts");
    let mut drawn: Vec<Vec<String>> = Vec::new();
    let mut ellipsis_links: Vec<Option<String>> = Vec::new();
    for (rows, want) in [4_u16, 3, 2, 1].into_iter().zip(&frames) {
        let mut t = Tester::with(
            Options::new()
                .size(18, rows)
                .terminal(|o| o.bidi = BidiReordering::Terminal),
        );
        t.pump_widget(
            Row::new()
                .child(SizedBox::new(Some(INDENT as i32), None, None))
                .child(Expanded::new(
                    1,
                    Text::rich(spans.clone())
                        .max_lines(Some(usize::from(rows)))
                        .overflow(TextOverflow::Ellipsis),
                )),
        );
        let lines = t.lines();
        let got: Vec<String> = lines.iter().map(|row| row.trim().to_owned()).collect();
        let (last, rest) = got.split_last().expect("a row");
        let (want_last, want_rest) = want.split_last().expect("a row");
        assert_eq!(rest, want_rest, "{rows} rows: the rows above the last");
        assert!(
            last.starts_with(want_last.trim_end_matches('\u{2026}'))
                && last.ends_with('\u{2026}') == want_last.ends_with('\u{2026}'),
            "{rows} rows: the last row `{last}` begins as the oracle's `{want_last}`"
        );
        for (y, row) in lines.iter().enumerate() {
            let cells: Vec<char> = row.trim_end().chars().collect();
            assert!(
                cells[..INDENT].iter().all(|ch| *ch == ' '),
                "{rows} rows, row {y}: indented"
            );
            for (x, ch) in cells.iter().enumerate().skip(INDENT) {
                let link = t.cell(x, y).link;
                if *ch == '\u{2026}' && x + 1 == cells.len() {
                    ellipsis_links.push(link);
                } else if y == 0 && x < INDENT + prefix.chars().count() {
                    assert_eq!(link, None, "{rows} rows, ({x}, {y}) the prefix");
                } else {
                    assert_eq!(link.as_deref(), Some(url), "{rows} rows, ({x}, {y}) `{ch}`");
                }
            }
        }
        drawn.push(got);
    }
    NativeStatus {
        frames,
        drawn,
        ellipsis_links,
    }
}

impl NativeStatus {
    /// What the widget does the oracle's way: the wrap, so every row but the last equals the
    /// oracle's, each frame begins as the oracle's, every cell of the link's text carries the
    /// link and the prefix none (asserted while drawing, in `native_status`).
    pub fn assert_wrap_holds(&self) {
        assert_eq!(self.drawn.len(), self.frames.len());
    }

    /// The frames are the oracle's, and its ellipsis carries no link.
    pub fn assert_oracle_frames(&self) {
        pretty_assertions::assert_eq!(self.drawn, self.frames, "the oracle's frames");
        assert!(
            self.ellipsis_links.len() == 4 && self.ellipsis_links.iter().all(Option::is_none),
            "the ellipsis carries no link: {:?}",
            self.ellipsis_links
        );
    }
}
