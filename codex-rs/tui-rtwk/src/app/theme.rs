//! Styles of the skeleton, taken from the rtwk `ThemeData` of the root
//! (`ui::run::root`), never from a literal colour: a theme of the app changes
//! the look in this one place.

use std::rc::Rc;

use rtwk::Cx;
use rtwk::style::Style;
use rtwk::style::Theme;
use rtwk::style::ThemeData;
use rtwk_core::input::Brightness;
use rtwk_core::style::Attrs;
use rtwk_core::style::Color;

/// The theme of the app's root: the terminal's own colours, dark until the
/// terminal says otherwise.
pub fn data() -> Rc<ThemeData> {
    Rc::new(ThemeData::terminal(Brightness::Dark))
}

/// The style tokens of one theme.
#[derive(Clone, Debug)]
pub struct Tokens(Rc<ThemeData>);

impl Tokens {
    pub fn new(data: Rc<ThemeData>) -> Self {
        Self(data)
    }

    /// The tokens of the theme `cx` is under.
    pub fn of<S>(cx: &mut Cx<'_, S>) -> Self {
        Self(Theme::of(cx))
    }

    pub fn dim(&self) -> Style {
        self.0.dim()
    }

    pub fn bold(&self) -> Style {
        self.0.text.clone().attrs(Attrs::BOLD)
    }

    pub fn accent(&self) -> Style {
        self.0.accent.clone()
    }

    /// The theme's info colour (the oracle's cyan accents).
    pub fn info(&self) -> Style {
        self.0.info.clone()
    }

    /// The theme's success colour (the oracle's green accents).
    pub fn success(&self) -> Style {
        self.0.success.clone()
    }

    pub(crate) fn shimmer_colors(&self) -> Option<(Color, Color)> {
        let base = self.0.accent.fg.filter(|color| color.is_rgb())?;
        let highlight = self.0.text.fg.filter(|color| color.is_rgb())?;
        Some((base, highlight))
    }

    pub(crate) fn blossom_palette(&self) -> BlossomPalette {
        let channels = |color: Color| (color.r(), color.g(), color.b());
        BlossomPalette {
            background: self
                .0
                .background
                .bg
                .filter(|color| color.is_rgb())
                .map_or((15, 20, 37), channels),
            foreground: self
                .0
                .text
                .fg
                .filter(|color| color.is_rgb())
                .map_or((210, 221, 235), channels),
        }
    }

    pub fn selected(&self) -> Style {
        self.0.accent.clone().attrs(Attrs::BOLD)
    }

    pub fn error(&self) -> Style {
        self.0.error.clone()
    }
}

pub(crate) fn shimmer_style(style: &Style, base: Color, highlight: Color, strength: f64) -> Style {
    style.clone().fg(Color::lerp(base, highlight, strength))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BlossomPalette {
    pub background: (u8, u8, u8),
    pub foreground: (u8, u8, u8),
}

impl BlossomPalette {
    pub(crate) fn style(self, pixel: (u8, u8, u8), opacity: f64) -> Style {
        let (br, bg, bb) = self.background;
        let (r, g, b) = pixel;
        Style::new().fg(Color::lerp(
            Color::rgb(br, bg, bb),
            Color::rgb(r, g, b),
            opacity,
        ))
    }
}

/// The accents a task's own colour is drawn from (oracle: `thread_color.rs`,
/// `theme_accents` over the default syntax theme of `render/highlight.rs`:
/// catppuccin-mocha on a dark terminal, catppuccin-latte on a light one).
/// The values are the oracle's, computed once with its own `theme_accents`
/// (pinned by `tests/theme.rs`); no syntax-highlighting crate is linked for
/// them. Which one a task gets is core's `thread_color::slot`; a theme the
/// user chooses later brings its own list here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThreadPalette(&'static [(u8, u8, u8)]);

const MOCHA: [(u8, u8, u8); 14] = [
    (166, 227, 161),
    (245, 194, 231),
    (250, 179, 135),
    (203, 166, 247),
    (148, 226, 213),
    (137, 180, 250),
    (249, 226, 175),
    (235, 160, 172),
    (243, 139, 168),
    (137, 220, 235),
    (242, 205, 205),
    (116, 199, 236),
    (180, 190, 254),
    (245, 224, 220),
];

const LATTE: [(u8, u8, u8); 14] = [
    (64, 160, 43),
    (234, 118, 203),
    (254, 100, 11),
    (136, 57, 239),
    (23, 146, 153),
    (30, 102, 245),
    (223, 142, 29),
    (230, 69, 83),
    (210, 15, 57),
    (4, 165, 229),
    (221, 120, 120),
    (32, 159, 181),
    (114, 135, 253),
    (220, 138, 120),
];

impl ThreadPalette {
    /// The default accents of a terminal of this brightness.
    pub fn for_brightness(brightness: Brightness) -> Self {
        Self(match brightness {
            Brightness::Light => &LATTE,
            _ => &MOCHA,
        })
    }

    /// The palette of the theme the view is under.
    pub fn of(data: &ThemeData) -> Self {
        Self::for_brightness(data.brightness)
    }

    pub fn colors(&self) -> Vec<Color> {
        self.0
            .iter()
            .map(|&(r, g, b)| Color::rgb(r, g, b))
            .collect()
    }

    /// The colour of the thread `id`: the accent at its slot.
    pub fn color(&self, id: &str) -> Option<Color> {
        let (r, g, b) = *self
            .0
            .get(codex_tui_core::thread_color::slot(id, self.0.len())?)?;
        Some(Color::rgb(r, g, b))
    }
}
