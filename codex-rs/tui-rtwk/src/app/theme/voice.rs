//! Terminal palette roles for the voice controls and native audio meters.

use rtwk::style::{Attrs, Color, Style};

pub(crate) fn connecting() -> Style {
    mic()
}

pub(crate) fn recording() -> Style {
    Style::new().fg(Color::RED).attrs(Attrs::BOLD)
}

pub(crate) fn quiet() -> Style {
    Style::new().fg(Color::BRIGHT_BLACK)
}

pub(crate) fn mic() -> Style {
    Style::new().fg(Color::CYAN)
}

pub(crate) fn speaker() -> Style {
    Style::new().fg(Color::MAGENTA)
}
