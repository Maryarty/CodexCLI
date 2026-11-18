//! Styles of the skeleton. One place, so the later ThemeData mapping (the
//! app's `RunOptions::theme`) replaces constants, not call sites.

use rtwk::prelude::Color;
use rtwk::style::Style;
use rtwk_core::style::Attrs;

pub fn dim() -> Style {
    Style::new().attrs(Attrs::DIM)
}

pub fn bold() -> Style {
    Style::new().attrs(Attrs::BOLD)
}

pub fn accent() -> Style {
    Style::new().fg(Color::CYAN)
}

pub fn selected() -> Style {
    Style::new().fg(Color::CYAN).attrs(Attrs::BOLD)
}

pub fn error() -> Style {
    Style::new().fg(Color::RED)
}
