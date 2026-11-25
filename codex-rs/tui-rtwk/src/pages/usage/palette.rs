//! The colours of the dashboard on the terminal's own (oracle: `analytics/styles.rs`,
//! `activity_chart/palette.rs`): the accent and the active tab as they read on the
//! terminal's background, and the activity chart's five-step gradient with the glyph
//! policy that goes with it. The terminal's colours are what its colour queries told
//! (`Colors`); the colour level is only known where cells are painted, so the chart is
//! painted by [`ActivityPainter`] and the palette is made there.

use rtwk::layout::CellPainter;
use rtwk::style::{Attrs, Color, Style};
use rtwk_core::canvas::Canvas;
use rtwk_core::style::ColorMode;

use codex_tui_core::screens::usage::TokenActivityView;
use codex_tui_core::screens::usage::activity::Square;

use crate::app::theme::effort::{Colors, blend, is_light};
use crate::app::theme::{self, Rgb, readable_on};

use super::Look;

/// The scopes the oracle's activity chart takes its anchor colour from.
pub const ANCHOR_SCOPES: [&str; 3] = ["entity.name.type", "support.type", "variable"];

fn packed(rgb: Rgb) -> u32 {
    u32::from_be_bytes([0, rgb.0, rgb.1, rgb.2])
}

/// `style::accent_style`: the syntax theme's `codex.accent` where it has one (bold), else on
/// a terminal that told its background the accent (ChatGPT blue 200, a darker blue on a
/// light terminal) kept readable on it, bold. The theme's own accent (`base`) where the
/// terminal told nothing. (The colour level is the presenter's: it lowers the colour.)
pub fn accent(base: Style, colors: Colors, theme: Option<Color>) -> Style {
    if let Some(color) = theme {
        return theme::bold(color);
    }
    let Some(bg) = colors.bg else { return base };
    let preferred = if is_light(bg) {
        (28, 100, 200)
    } else {
        (99, 168, 248)
    };
    Style::new()
        .fg(readable_on(theme::color(preferred), Some(packed(bg))))
        .attrs(Attrs::BOLD)
}

/// `bottom_pane::active_tab_style`: a grey fill (lighter on a light terminal) with the
/// terminal's text on it kept readable, bold. `base` where the terminal told no
/// background.
pub fn active_tab(base: Style, colors: Colors) -> Style {
    let Some(bg) = colors.bg else { return base };
    let fill = if is_light(bg) {
        (220, 220, 220)
    } else {
        (76, 76, 76)
    };
    let mut style = Style::new().bg(theme::color(fill)).attrs(Attrs::BOLD);
    if let Some(fg) = colors.fg {
        style = style.fg(readable_on(theme::color(fg), Some(packed(fill))));
    }
    style
}

/// `style::selection_style` on a terminal that told its background: ChatGPT blue 100 on a
/// light one and 200 on a dark one (the other where the first hardly shows against the
/// terminal), with dark blue text kept readable on it, bold. `base` (the reverse video of
/// the oracle's fallback) where the terminal told no background.
pub fn selection(base: Style, colors: Colors) -> Style {
    let Some(bg) = colors.bg else { return base };
    const BLUE_100: Rgb = (164, 205, 251);
    const BLUE_200: Rgb = (99, 168, 248);
    let (preferred, alternate) = if is_light(bg) {
        (BLUE_100, BLUE_200)
    } else {
        (BLUE_200, BLUE_100)
    };
    let fill = if ratio(preferred, bg) < 1.25 && ratio(alternate, bg) > ratio(preferred, bg) {
        alternate
    } else {
        preferred
    };
    Style::new()
        .fg(readable_on(theme::color((0, 0, 46)), Some(packed(fill))))
        .bg(theme::color(fill))
        .attrs(Attrs::BOLD)
}

/// The WCAG contrast ratio of two colours.
fn ratio(a: Rgb, b: Rgb) -> f64 {
    fn luminance(rgb: Rgb) -> f64 {
        let [r, g, b] = [rgb.0, rgb.1, rgb.2].map(|channel| {
            let value = f64::from(channel) / 255.0;
            if value <= 0.04045 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        });
        0.2126 * r + 0.7152 * g + 0.0722 * b
    }
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

/// The styles and glyph strategy of the token activity cells.
#[derive(Clone, Debug, PartialEq)]
pub struct ActivityPalette {
    styles: [Style; 5],
    bar: Style,
    /// A terminal that shows the gradient: every cell has the one glyph and the colour
    /// carries the level. Else the hollow glyph marks an empty cell, the active style
    /// the others.
    uses_color: bool,
}

impl ActivityPalette {
    /// The palette of a terminal with `colors`, showing `mode`; `active` is the style
    /// of an active cell (the anchor of the gradient when its colour is a true colour),
    /// `empty` the style of an empty one without the gradient.
    pub fn current(colors: Colors, mode: ColorMode, active: &Style, empty: &Style) -> Self {
        let fallback = || Self::fallback(active, empty);
        let Some(anchor) = theme::rgb_of(active) else {
            return fallback();
        };
        let (Some(fg), Some(bg)) = (colors.fg, colors.bg) else {
            return fallback();
        };
        if !matches!(mode, ColorMode::TrueColor | ColorMode::Ansi256) {
            return fallback();
        }
        let empty_alpha = if is_light(bg) { 0.18 } else { 0.14 };
        let alphas = [empty_alpha, 0.22, 0.42, 0.68, 1.00];
        let styles = std::array::from_fn(|level| {
            let source = if level == 0 { fg } else { anchor };
            theme::fg(theme::color(blend(source, bg, alphas[level])))
        });
        Self {
            styles,
            bar: theme::fg(theme::color(blend(anchor, bg, 0.78))),
            uses_color: true,
        }
    }

    fn fallback(active: &Style, empty: &Style) -> Self {
        Self {
            styles: [
                empty.clone(),
                active.clone(),
                active.clone(),
                active.clone(),
                active.clone(),
            ],
            bar: active.clone(),
            uses_color: false,
        }
    }

    pub fn for_level(&self, level: usize) -> Style {
        self.styles[level.min(4)].clone()
    }

    pub fn for_bar_level(&self, level: usize) -> Style {
        if level == 0 {
            self.for_level(0)
        } else {
            self.bar.clone()
        }
    }

    /// The glyph of a cell at `level`: the daily view's square (hollow for an empty
    /// cell without the gradient), the bar views' full block or a blank.
    pub fn glyph(&self, view: TokenActivityView, level: usize, glyphs: &Glyphs) -> &'static str {
        if view != TokenActivityView::Daily {
            return if level == 0 { glyphs.gap } else { glyphs.bar };
        }
        if self.uses_color || level > 0 {
            glyphs.active
        } else {
            glyphs.empty
        }
    }
}

/// The glyphs a language's strings give the cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Glyphs {
    pub empty: &'static str,
    pub active: &'static str,
    pub bar: &'static str,
    pub gap: &'static str,
}

/// What the palette is made from at paint time.
#[derive(Clone, Debug, PartialEq)]
pub struct Source {
    pub colors: Colors,
    pub active: Style,
    pub empty: Style,
    pub glyphs: Glyphs,
    pub view: TokenActivityView,
}

impl Source {
    pub fn of(look: &Look, glyphs: Glyphs, view: TokenActivityView) -> Self {
        Source {
            colors: look.colors,
            active: look.activity.clone(),
            empty: look.dim.clone(),
            glyphs,
            view,
        }
    }
}

/// A piece of a chart row.
#[derive(Clone, Debug, PartialEq)]
pub enum Seg {
    Text(String, Style),
    Cell(Square),
}

/// The chart's rows: text and cells, the cells in the palette of the colours the
/// terminal shows.
#[derive(Debug, PartialEq)]
pub struct ActivityPainter {
    pub source: Source,
    pub rows: Vec<Vec<Seg>>,
}

impl CellPainter for ActivityPainter {
    fn paint(&self, canvas: &mut Canvas<'_>, width: i32, height: i32) {
        let Source {
            colors,
            active,
            empty,
            glyphs,
            view,
        } = &self.source;
        let mode = canvas.styles_mut().color_mode();
        let palette = ActivityPalette::current(*colors, mode, active, empty);
        for (y, row) in self.rows.iter().enumerate() {
            if y as i32 >= height {
                break;
            }
            let mut x = 0;
            for seg in row {
                let (shown, style) = match seg {
                    Seg::Text(text, style) => (text.as_str(), style.clone()),
                    Seg::Cell(Square::Blank) => (glyphs.gap, Style::new()),
                    Seg::Cell(Square::Level(level)) => (
                        palette.glyph(*view, *level, glyphs),
                        if *view == TokenActivityView::Daily {
                            palette.for_level(*level)
                        } else {
                            palette.for_bar_level(*level)
                        },
                    ),
                };
                if x >= width {
                    break;
                }
                let pen = style.pen_in(canvas.styles_mut());
                x += canvas.text(x, y as i32, shown, pen) as i32;
            }
        }
    }

    fn should_repaint(&self, old: &dyn CellPainter) -> bool {
        old.downcast_ref::<Self>() != Some(self)
    }
}
