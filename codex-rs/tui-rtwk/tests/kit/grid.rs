//! The screen as the kit reads it: cells with their text and styles, and the
//! cursor, taken from the tester's terminal (`rtwk_test::Screen`). Styles are
//! named the way the oracle's snapshots and assertions name them (ratatui's
//! `Color` and `Modifier`), so an oracle expectation is copied, not translated.

use rtwk_core::style::{Attrs, Color};
use rtwk_test::{Cell, Screen as Vt};

/// The cells of the screen, row by row, and where the cursor is (`None` when
/// the terminal hides it).
pub struct Grid {
    pub width: usize,
    pub height: usize,
    pub(super) cells: Vec<Cell>,
    pub cursor: Option<(usize, usize)>,
}

/// A cell's style in the oracle's words: `Cyan`, `Rgb(1, 2, 3)`, `Reset`;
/// the modifier as `BOLD | DIM` or `NONE`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CellStyle {
    pub fg: String,
    pub bg: String,
    pub underline: String,
    pub modifier: String,
}

impl Grid {
    pub fn of(vt: &Vt) -> Grid {
        let (width, height) = (vt.width(), vt.height());
        let mut cells = Vec::with_capacity(width * height);
        for y in 0..height {
            for x in 0..width {
                cells.push(vt.cell(x, y));
            }
        }
        Grid {
            width,
            height,
            cells,
            cursor: vt.cursor_visible().then(|| vt.cursor()),
        }
    }

    pub fn cell(&self, x: usize, y: usize) -> &Cell {
        &self.cells[y * self.width + x]
    }

    /// Row `y` from column `from`, trailing blanks cut. A wide glyph is one
    /// char, or (`wide_space`) the glyph and a space for its second cell, as
    /// the oracle's buffer dumps write it.
    pub fn row(&self, y: usize, from: usize, to: usize, wide_space: bool) -> String {
        let mut out = String::new();
        for x in from..to.min(self.width) {
            let cell = self.cell(x, y);
            if cell.tail {
                if wide_space {
                    out.push(' ');
                }
            } else {
                out.push_str(&cell.text);
            }
        }
        out.trim_end().to_string()
    }

    pub fn rows(&self, wide_space: bool) -> Vec<String> {
        (0..self.height)
            .map(|y| self.row(y, 0, self.width, wide_space))
            .collect()
    }

    /// The screen as text, for a failure message.
    pub fn text(&self) -> String {
        let mut rows = self.rows(false);
        while rows.last().is_some_and(String::is_empty) {
            rows.pop();
        }
        rows.join("\n")
    }

    /// The first `text` on the screen: (first column, column after its last
    /// cell, row), columns counted in cells (a wide glyph is two).
    pub fn locate(&self, text: &str) -> Option<(usize, usize, usize)> {
        (0..self.height).find_map(|y| {
            self.locate_in_row(text, y)
                .map(|(first, after)| (first, after, y))
        })
    }

    /// The first `text` of row `row`: (first column, column after it).
    pub fn locate_in_row(&self, text: &str, row: usize) -> Option<(usize, usize)> {
        if row >= self.height {
            return None;
        }
        let mut line = String::new();
        let mut starts = Vec::new(); // byte offset of a cell's text -> its column
        for x in 0..self.width {
            let cell = self.cell(x, row);
            if !cell.tail {
                starts.push((line.len(), x));
                line.push_str(&cell.text);
            }
        }
        let byte = line.find(text)?;
        let first = starts.iter().find(|(b, _)| *b == byte).map(|(_, x)| *x)?;
        let end = byte + text.len();
        let after = starts
            .iter()
            .find(|(b, _)| *b >= end)
            .map_or(self.width, |(_, x)| *x);
        Some((first, after))
    }

    /// The cells of the first `text` on the screen, as (column, row): the cells of one row when
    /// `locate` finds it there, else of a text the screen wrapped over several rows, found with
    /// its spaces and the row breaks not counted (a wrapped text loses or gains blanks where it
    /// breaks): then only the cells that hold the text's own non-blank characters.
    pub fn locate_cells(&self, text: &str) -> Option<Vec<(usize, usize)>> {
        if let Some((x0, x1, y)) = self.locate(text) {
            return Some((x0..x1).map(|x| (x, y)).collect());
        }
        let needle: String = text.chars().filter(|c| !c.is_whitespace()).collect();
        if needle.is_empty() {
            return None;
        }
        let mut stream = String::new();
        let mut cells = Vec::new(); // (byte offset in `stream`, column, row)
        for y in 0..self.height {
            for x in 0..self.width {
                let cell = self.cell(x, y);
                if cell.tail || cell.text.chars().all(char::is_whitespace) {
                    continue;
                }
                cells.push((stream.len(), x, y));
                stream.push_str(&cell.text);
            }
        }
        let byte = stream.find(&needle)?;
        let end = byte + needle.len();
        Some(
            cells
                .into_iter()
                .filter(|(b, _, _)| (byte..end).contains(b))
                .map(|(_, x, y)| (x, y))
                .collect(),
        )
    }

    pub fn has_wide(&self) -> bool {
        self.cells.iter().any(|c| c.tail)
    }

    pub fn style(&self, x: usize, y: usize) -> CellStyle {
        let cell = self.cell(x, y);
        CellStyle {
            fg: color_name(cell.fg),
            bg: color_name(cell.bg),
            underline: color_name(cell.ul),
            modifier: modifier_name(cell.attrs),
        }
    }
}

/// ratatui's name of a color: `Reset`, `Cyan`, `LightRed`, `Indexed(n)`,
/// `Rgb(r, g, b)`.
pub fn color_name(color: Color) -> String {
    const NAMES: [&str; 16] = [
        "Black",
        "Red",
        "Green",
        "Yellow",
        "Blue",
        "Magenta",
        "Cyan",
        "Gray",
        "DarkGray",
        "LightRed",
        "LightGreen",
        "LightYellow",
        "LightBlue",
        "LightMagenta",
        "LightCyan",
        "White",
    ];
    if color.is_default() {
        "Reset".to_string()
    } else if color.is_rgb() {
        let v = color.value();
        format!(
            "Rgb({}, {}, {})",
            (v >> 16) & 0xff,
            (v >> 8) & 0xff,
            v & 0xff
        )
    } else if color.is_indexed() && color.index() < 16 {
        NAMES[usize::from(color.index())].to_string()
    } else {
        format!("Indexed({})", color.index())
    }
}

/// ratatui's `Modifier` Debug: `NONE`, `BOLD`, `BOLD | REVERSED`, in its order.
pub fn modifier_name(attrs: Attrs) -> String {
    let table: [(&str, bool); 9] = [
        ("BOLD", attrs.intersects(Attrs::BOLD)),
        ("DIM", attrs.intersects(Attrs::DIM)),
        ("ITALIC", attrs.intersects(Attrs::ITALIC)),
        ("UNDERLINED", attrs.intersects(Attrs::UNDERLINE_MASK)),
        ("SLOW_BLINK", attrs.intersects(Attrs::BLINK)),
        ("RAPID_BLINK", false),
        ("REVERSED", attrs.intersects(Attrs::REVERSE)),
        ("HIDDEN", attrs.intersects(Attrs::HIDDEN)),
        ("CROSSED_OUT", attrs.intersects(Attrs::STRIKE)),
    ];
    let on: Vec<&str> = table
        .iter()
        .filter(|(_, set)| *set)
        .map(|(n, _)| *n)
        .collect();
    if on.is_empty() {
        "NONE".to_string()
    } else {
        on.join(" | ")
    }
}

fn canon(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_ascii_lowercase()
}

/// Whether an oracle color (`Cyan`, `Rgb(1, 2, 3)`) names this color of ours.
pub fn same_color(oracle: &str, ours: &str) -> bool {
    canon(oracle) == canon(ours)
}

/// Whether the oracle's modifier text (`BOLD | DIM`) is exactly ours.
pub fn same_modifier(oracle: &str, ours: &str) -> bool {
    let set = |s: &str| {
        let mut v: Vec<String> = s
            .split('|')
            .map(canon)
            .filter(|t| !t.is_empty() && t != "none")
            .collect();
        v.sort();
        v
    };
    set(oracle) == set(ours)
}

/// The words of a style spec: apart by spaces, except inside `( )`.
fn spec_tokens(spec: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut depth = 0usize;
    for c in spec.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ => {}
        }
        if c.is_whitespace() && depth == 0 {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
        } else {
            cur.push(c);
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// An oracle style assertion, written as it reads: `fg=Cyan bg=Reset bold
/// !dim reversed plain`: `fg=`/`bg=` the color named as ratatui does (`Rgb(1,2,3)`
/// without spaces is fine; `!fg=Red` asks the color is not that one); a modifier name (`bold dim italic underlined
/// reversed hidden crossed_out`) asks the modifier is on, `!name` that it is
/// off, `plain` that none is. Returns what differs, empty when all hold.
pub fn check_style(spec: &str, style: &CellStyle) -> Vec<String> {
    let mut bad = Vec::new();
    for token in spec_tokens(spec) {
        let (negate, tok) = match token.strip_prefix('!') {
            Some(t) => (true, t),
            None => (false, token.as_str()),
        };
        if let Some(want) = tok.strip_prefix("fg=") {
            if same_color(want, &style.fg) == negate {
                bad.push(if negate {
                    format!("fg is {}, the oracle's is not {want}", style.fg)
                } else {
                    format!("fg is {}, the oracle's is {want}", style.fg)
                });
            }
        } else if let Some(want) = tok.strip_prefix("bg=") {
            if same_color(want, &style.bg) == negate {
                bad.push(if negate {
                    format!("bg is {}, the oracle's is not {want}", style.bg)
                } else {
                    format!("bg is {}, the oracle's is {want}", style.bg)
                });
            }
        } else if tok == "plain" {
            if style.modifier != "NONE" {
                bad.push(format!(
                    "modifier is {}, the oracle's is none",
                    style.modifier
                ));
            }
        } else {
            let name = tok.to_ascii_uppercase();
            assert!(
                [
                    "BOLD",
                    "DIM",
                    "ITALIC",
                    "UNDERLINED",
                    "REVERSED",
                    "HIDDEN",
                    "CROSSED_OUT"
                ]
                .contains(&name.as_str()),
                "`{token}` in a style spec: use fg=<Color> bg=<Color> bold dim italic underlined reversed hidden crossed_out plain, `!` to negate"
            );
            let on = style.modifier.split(" | ").any(|m| m == name);
            if on == negate {
                bad.push(format!(
                    "modifier is {}, the oracle says {}{}",
                    style.modifier,
                    if negate { "no " } else { "" },
                    name
                ));
            }
        }
    }
    bad
}

// ---- the composer's text, read from its rows ----------------------------------------

impl Grid {
    /// The composer's text, read black-box from its rows: the prompt row
    /// (`› `, the row of the cursor or the nearest one above it that starts
    /// so) and the continuation rows under it (two blanks of indent), down to
    /// the first blank row (and never above the cursor's row). Rows are
    /// joined with `\n`. The placeholder of an empty composer (drawn dim) reads
    /// as `""`.
    ///
    /// Limits, by what the screen shows: the cursor must be visible (the
    /// composer has the focus; no popup or dialog took it), a soft-wrapped
    /// line reads as several rows, and a draft with an empty line ends at that
    /// line unless the cursor is below it. Drafts of the oracle tests fit one
    /// row each.
    pub fn composer_text(&self) -> String {
        let (_, cy) = self.cursor.unwrap_or_else(|| {
            panic!(
                "composer_text: the cursor is hidden, the composer has no focus:\n{}",
                self.text()
            )
        });
        let is_prompt = |y: usize| self.cell(0, y).text == "›" && self.cell(1, y).text == " ";
        let top = (0..=cy).rev().find(|&y| is_prompt(y)).unwrap_or_else(|| {
            panic!(
                "composer_text: no prompt row `› ` at or above the cursor row {cy}:\n{}",
                self.text()
            )
        });
        let mut lines = vec![self.row(top, 2, self.width, false)];
        let mut y = top + 1;
        while y < self.height {
            let row = self.row(y, 0, self.width, false);
            if row.is_empty() && y > cy {
                break;
            }
            if !row.is_empty() && !row.starts_with("  ") {
                break;
            }
            lines.push(self.row(y, 2, self.width, false));
            y += 1;
        }
        if lines.len() == 1 && self.style(2, top).modifier.contains("DIM") {
            return String::new();
        }
        lines.join("\n")
    }

    pub fn assert_composer(&self, text: &str) {
        pretty_assertions::assert_eq!(
            text,
            self.composer_text(),
            "the composer's text:\n{}",
            self.text()
        );
    }
}

// ---- an oracle assertion's literal, on the screen's cells ----------------------------

impl Grid {
    pub fn assert_contains(&self, text: &str) {
        assert!(
            self.text().contains(text),
            "the screen has no `{text}`:\n{}",
            self.text()
        );
    }

    pub fn assert_absent(&self, text: &str) {
        assert!(
            !self.text().contains(text),
            "the screen still has `{text}`:\n{}",
            self.text()
        );
    }

    fn row_text(&self, row: usize) -> String {
        if row < self.height {
            self.row(row, 0, self.width, false)
        } else {
            String::new()
        }
    }

    pub fn assert_row(&self, row: usize, text: &str) {
        pretty_assertions::assert_eq!(
            text.trim_end(),
            self.row_text(row),
            "row {row} of the screen:\n{}",
            self.text()
        );
    }

    pub fn assert_has_row(&self, text: &str) {
        assert!(
            self.rows(false).iter().any(|r| r == text.trim_end()),
            "no row reads `{text}`:\n{}",
            self.text()
        );
    }

    pub fn assert_row_starts_with(&self, row: usize, text: &str) {
        assert!(
            self.row_text(row).starts_with(text),
            "row {row} does not start with `{text}`:\n{}",
            self.text()
        );
    }

    pub fn assert_row_ends_with(&self, row: usize, text: &str) {
        assert!(
            self.row_text(row).ends_with(text.trim_end()),
            "row {row} does not end with `{text}`:\n{}",
            self.text()
        );
    }

    pub fn assert_in_order(&self, fragments: &[&str]) {
        let text = self.text();
        let mut from = 0;
        for fragment in fragments {
            match text[from..].find(fragment) {
                Some(at) => from += at + fragment.len(),
                None => panic!(
                    "`{fragment}` is not on the screen after the fragments before it ({fragments:?}):\n{text}"
                ),
            }
        }
    }

    pub fn assert_text_at(&self, x: usize, y: usize, text: &str) {
        let at = self.locate_in_row(text, y).map(|(first, _)| first);
        assert!(
            at == Some(x),
            "`{text}` is at column {at:?} of row {y}, the oracle's is {x}:\n{}",
            self.text()
        );
    }

    pub fn assert_style(&self, x: usize, y: usize, spec: &str) {
        assert!(
            x < self.width && y < self.height,
            "cell ({x}, {y}) is off the screen"
        );
        let bad = check_style(spec, &self.style(x, y));
        assert!(
            bad.is_empty(),
            "cell ({x}, {y}) `{}` against `{spec}`: {}\n{}",
            self.cell(x, y).text,
            bad.join("; "),
            self.text()
        );
    }

    pub fn assert_text_style(&self, text: &str, spec: &str) {
        let Some((x0, x1, y)) = self.locate(text) else {
            panic!("no `{text}` on the screen:\n{}", self.text());
        };
        for x in x0..x1 {
            if self.cell(x, y).tail {
                continue;
            }
            let bad = check_style(spec, &self.style(x, y));
            assert!(
                bad.is_empty(),
                "`{text}` at ({x}, {y}) `{}` against `{spec}`: {}\n{}",
                self.cell(x, y).text,
                bad.join("; "),
                self.text()
            );
        }
    }

    pub fn assert_cursor(&self, x: usize, y: usize) {
        assert!(
            self.cursor == Some((x, y)),
            "the cursor is {:?}, the oracle's is shown at {:?}:\n{}",
            self.cursor,
            (x, y),
            self.text()
        );
    }

    pub fn assert_cursor_hidden(&self) {
        assert!(
            self.cursor.is_none(),
            "the cursor is shown at {:?}, the oracle's is hidden:\n{}",
            self.cursor,
            self.text()
        );
    }

    pub fn assert_link(&self, text: &str, url: &str) {
        let Some(cells) = self.locate_cells(text) else {
            panic!("no `{text}` on the screen:\n{}", self.text());
        };
        for (x, y) in cells {
            let link = self.cell(x, y).link.as_deref();
            assert!(
                self.cell(x, y).tail || link == Some(url),
                "`{text}`: cell ({x}, {y}) links to {link:?}, the oracle's is {url}"
            );
        }
    }

    pub fn assert_no_link(&self, text: &str) {
        let Some(cells) = self.locate_cells(text) else {
            panic!("no `{text}` on the screen:\n{}", self.text());
        };
        for (x, y) in cells {
            let link = self.cell(x, y).link.as_deref();
            assert!(
                link.is_none(),
                "`{text}`: cell ({x}, {y}) links to {link:?}, the oracle has none"
            );
        }
    }
}
