//! The few places a screen turns a computed colour into a style: the dashboard's accent and
//! chart colours are worked out from the terminal's own (`palette.rs` of the usage page) and
//! only the theme directory builds a colour or a foreground style from values.

use rtwk::style::Style;
use rtwk_core::style::{Attrs, Color};

use super::Rgb;

/// The true colour of `rgb`.
pub(crate) fn color(rgb: Rgb) -> Color {
    Color::rgb(rgb.0, rgb.1, rgb.2)
}

/// The three bytes of the true colour of `style`'s foreground, none for any other.
pub(crate) fn rgb_of(style: &Style) -> Option<Rgb> {
    match style.fg {
        Some(c) if c.is_rgb() => Some((c.r(), c.g(), c.b())),
        _ => None,
    }
}

/// `color` as the foreground.
pub(crate) fn fg(color: Color) -> Style {
    Style::new().fg(color)
}

/// `color` as a bold foreground.
pub(crate) fn bold(color: Color) -> Style {
    fg(color).attrs(Attrs::BOLD)
}
