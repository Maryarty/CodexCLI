//! Palette fixtures: the terminal's default colours a test gives (`Open::terminal_colors`) and
//! the contrast the screen shows against them, as the oracle's palette tests measure it
//! (`analytics/styles_tests.rs`: WCAG luminance and contrast ratio, `Reset` read as the
//! terminal's own colour, `White` / `Black` as the named colours, any other colour left out).
//!
//! - `LIGHT` / `DARK`: the two terminals of that test, `(foreground, background)` as
//!   `terminal_colors` takes them.
//! - `Grid::contrast_at(x, y, terminal)`: the contrast of one cell.
//! - `Grid::low_contrast(terminal, min)`: the cells of text under `min`, a key hint's white
//!   (`fg` `White`: its own style, asserted apart) aside.

use super::grid::Grid;

/// The terminal of the oracle's light palette test: black on white.
pub const LIGHT: (u32, u32) = (0x00_00_00, 0xff_ff_ff);

/// The terminal of the oracle's dark palette test: `e6e6e6` on `101010`.
pub const DARK: (u32, u32) = (0xe6_e6_e6, 0x10_10_10);

type Rgb = (u8, u8, u8);

fn rgb(value: u32) -> Rgb {
    ((value >> 16) as u8, (value >> 8) as u8, value as u8)
}

/// The relative luminance of a colour (WCAG: sRGB channels linearised, 0.2126 / 0.7152 / 0.0722).
pub fn luminance(color: (u8, u8, u8)) -> f64 {
    let [r, g, b] = [color.0, color.1, color.2].map(|channel| {
        let value = f64::from(channel) / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    });
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// The contrast ratio of two colours, 1 to 21.
pub fn contrast(foreground: (u8, u8, u8), background: (u8, u8, u8)) -> f64 {
    let (fg, bg) = (luminance(foreground), luminance(background));
    (fg.max(bg) + 0.05) / (fg.min(bg) + 0.05)
}

/// What a colour of the screen is as the oracle's test reads it: `Rgb(r, g, b)` as it is, `White`
/// and `Black`, `Reset` the terminal's own `default`; nothing for the rest.
fn resolve(name: &str, default: Rgb) -> Option<Rgb> {
    match name {
        "Reset" => Some(default),
        "White" => Some((255, 255, 255)),
        "Black" => Some((0, 0, 0)),
        _ => {
            let inner = name.strip_prefix("Rgb(")?.strip_suffix(')')?;
            let mut channels = inner.split(", ").map(|c| c.parse::<u8>().ok());
            Some((channels.next()??, channels.next()??, channels.next()??))
        }
    }
}

impl Grid {
    /// The contrast of the cell at (x, y) between its text colour and its background, on a
    /// terminal whose default colours are `terminal` (`(foreground, background)`, as
    /// `Open::terminal_colors` took them); `None` when a colour is not one the oracle measures.
    pub fn contrast_at(&self, x: usize, y: usize, terminal: (u32, u32)) -> Option<f64> {
        let style = self.style(x, y);
        let foreground = resolve(&style.fg, rgb(terminal.0))?;
        let background = resolve(&style.bg, rgb(terminal.1))?;
        Some(contrast(foreground, background))
    }

    /// The cells with text whose contrast is under `min`: (column, row, text, contrast). A cell
    /// whose text colour is `White` is a key hint's own style and is not measured.
    pub fn low_contrast(&self, terminal: (u32, u32), min: f64) -> Vec<(usize, usize, String, f64)> {
        let mut found = Vec::new();
        for y in 0..self.height {
            for x in 0..self.width {
                let cell = self.cell(x, y);
                if cell.text.trim().is_empty() || self.style(x, y).fg == "White" {
                    continue;
                }
                if let Some(ratio) = self.contrast_at(x, y, terminal)
                    && ratio < min
                {
                    found.push((x, y, cell.text.clone(), ratio));
                }
            }
        }
        found
    }
}
