//! Compare two checkpoints captured by tools/rtwk-port/mock-model/check.py.

use std::fmt::Write as _;
use std::path::Path;

use anyhow::{Context, Result, bail};
use rtwk_test::{Screen, row_text};

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let [size, oracle, rtwk, options @ ..] = args.as_slice() else {
        bail!(
            "usage: compare-transcript WIDTHxHEIGHT ORACLE.ansi RTWK.ansi [--check] [--frames DIR]"
        );
    };
    let mut check = false;
    let mut frames = None;
    let mut options = options.iter();
    while let Some(option) = options.next() {
        match option.as_str() {
            "--check" => check = true,
            "--frames" => {
                frames = Some(Path::new(options.next().context("--frames requires DIR")?))
            }
            _ => bail!("unknown option {option}"),
        }
    }
    let (width, height) = size.split_once('x').context("expected WIDTHxHEIGHT")?;
    let (width, height) = (width.parse()?, height.parse()?);
    if width == 0 || height == 0 {
        bail!("screen dimensions must be positive");
    }
    let oracle = replay(width, height, Path::new(oracle))?;
    let rtwk = replay(width, height, Path::new(rtwk))?;
    let expected = Frame::of(&oracle, width, height)?;
    let actual = Frame::of(&rtwk, width, height)?;

    let mut different_cells = 0;
    let mut different_rows = 0;
    for (row, (expected_cells, actual_cells)) in expected.rows.iter().zip(&actual.rows).enumerate()
    {
        let mut columns = expected_cells
            .iter()
            .zip(actual_cells)
            .enumerate()
            .filter(|(_, (expected, actual))| expected != actual)
            .map(|(column, _)| column + 1);
        if let Some(first) = columns.next() {
            let count = 1 + columns.count();
            different_cells += count;
            different_rows += 1;
            println!(
                "{:>3} oracle  {}",
                row + 1,
                row_text(&oracle, row, true).trim_end()
            );
            println!(
                "{:>3} rtwk    {}",
                row + 1,
                row_text(&rtwk, row, true).trim_end()
            );
            println!("    {count} differing cells, first at column {first}");
            println!("    oracle {}", expected_cells[first - 1]);
            println!("    rtwk   {}", actual_cells[first - 1]);
        }
    }
    let cursor_differs = expected.cursor != actual.cursor;
    if cursor_differs {
        println!(
            "cursor differs (oracle {:?}, rtwk {:?})",
            expected.cursor, actual.cursor
        );
    }
    println!(
        "{different_cells}/{} cells differ across {different_rows}/{height} rows",
        width * height
    );
    let expected_bytes = expected.encode()?;
    let actual_bytes = actual.encode()?;
    if let Some(directory) = frames {
        write_frames(directory, &expected_bytes, &actual_bytes)?;
    }
    if expected_bytes == actual_bytes {
        println!("frames are byte-for-byte equal");
    } else if check {
        bail!("frames differ");
    }
    Ok(())
}

#[allow(
    clippy::disallowed_methods,
    reason = "this standalone comparison CLI writes its results"
)]
fn write_frames(directory: &Path, oracle: &str, rtwk: &str) -> Result<()> {
    std::fs::create_dir_all(directory)?;
    std::fs::write(directory.join("oracle.frame"), oracle)?;
    std::fs::write(directory.join("rtwk.frame"), rtwk)?;
    Ok(())
}

fn replay(width: usize, height: usize, path: &Path) -> Result<Screen> {
    let mut screen = Screen::new(width, height);
    let bytes = std::fs::read(path).with_context(|| format!("read {}", path.display()))?;
    screen.feed(&without_session_status(&bytes));
    if !screen.unknown().is_empty() {
        bail!(
            "unsupported terminal operations in {}: {:?}",
            path.display(),
            screen.unknown()
        );
    }
    if !screen.images().is_empty() {
        bail!(
            "{} contains an image overlay, outside the cell frame",
            path.display()
        );
    }
    Ok(screen)
}

/// The oracle's OSC 7501 and iTerm2's OSC 21337 publish session
/// metadata to the host. They leave the terminal grid and cursor untouched.
fn without_session_status(mut bytes: &[u8]) -> Vec<u8> {
    let mut terminal = Vec::with_capacity(bytes.len());
    while let Some(start) = bytes.windows(2).position(|pair| pair == b"\x1b]") {
        let (before, osc) = bytes.split_at(start + 2);
        let status = osc.starts_with(b"7501;") || osc.starts_with(b"21337;");
        if status
            && let Some(end) = osc.iter().enumerate().find_map(|(i, byte)| match byte {
                b'\x07' => Some(i + 1),
                b'\x1b' if osc.get(i + 1) == Some(&b'\\') => Some(i + 2),
                _ => None,
            })
        {
            terminal.extend_from_slice(&before[..start]);
            bytes = &osc[end..];
        } else {
            terminal.extend_from_slice(before);
            bytes = osc;
        }
    }
    terminal.extend_from_slice(bytes);
    terminal
}

/// An exact terminal-cell frame. Each line is JSON with one record per cell:
/// text, wide-glyph tail, foreground, background, underline color, attribute
/// bits and resolved hyperlink. Retaining styles on spaces makes background
/// and underline regressions measurable even on otherwise empty rows.
struct Frame {
    width: usize,
    cursor: Option<((usize, usize), String)>,
    rows: Vec<Vec<String>>,
}

impl Frame {
    fn of(screen: &Screen, width: usize, height: usize) -> Result<Self> {
        let cursor = screen
            .cursor_visible()
            .then(|| (screen.cursor(), format!("{:?}", screen.cursor_shape())));
        let rows = (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| {
                        let cell = screen.cell(x, y);
                        serde_json::to_string(&(
                            cell.text,
                            cell.tail,
                            cell.fg.0,
                            cell.bg.0,
                            cell.ul.0,
                            cell.attrs.bits(),
                            cell.link,
                        ))
                    })
                    .collect::<serde_json::Result<Vec<_>>>()
            })
            .collect::<serde_json::Result<Vec<_>>>()?;
        Ok(Self {
            width,
            cursor,
            rows,
        })
    }

    fn encode(&self) -> Result<String> {
        let mut out = format!("rtwk-cell-frame-v1 {}x{}\n", self.width, self.rows.len());
        writeln!(out, "{}", serde_json::to_string(&self.cursor)?)?;
        for row in &self.rows {
            writeln!(out, "[{}]", row.join(","))?;
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn different_terminal_writes_can_leave_identical_screen_bytes() -> Result<()> {
        let mut first = Screen::new(4, 2);
        first.feed(b"\x1b[?25l\x1b[1;31mhi");
        let mut second = Screen::new(4, 2);
        second.feed(b"abc\x1b[2J\x1b[H\x1b[31;1m");
        second.feed(b"h");
        second.feed(b"i\x1b[?25l");
        assert_eq!(
            Frame::of(&first, 4, 2)?.encode()?,
            Frame::of(&second, 4, 2)?.encode()?
        );
        Ok(())
    }

    #[test]
    fn only_out_of_band_session_status_is_removed() {
        assert_eq!(
            without_session_status(
                b"a\x1b]7501;state=idle\x1b\\b\x1b]21337;status=Idle\x07\x1b]8;;https://example.org\x07c"
            ),
            b"ab\x1b]8;;https://example.org\x07c"
        );
    }

    #[test]
    fn changed_background_on_a_blank_cell_changes_the_frame() -> Result<()> {
        let mut screen = Screen::new(4, 2);
        let before = Frame::of(&screen, 4, 2)?;
        screen.feed(b"\x1b[42m ");
        screen.feed(b"\x1b[H");
        let after = Frame::of(&screen, 4, 2)?;
        assert_eq!(before.cursor, after.cursor);
        assert_ne!(before.rows[0][0], after.rows[0][0]);
        assert_eq!(before.rows[0][1..], after.rows[0][1..]);
        assert_eq!(before.rows[1], after.rows[1]);
        assert_ne!(before.encode()?, after.encode()?);
        Ok(())
    }
}
