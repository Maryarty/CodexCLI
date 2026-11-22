//! Styles of the app. The chrome follows the root rtwk theme. Markdown
//! typography uses the original's adaptive default palette.

use std::rc::Rc;

use rtwk::Cx;
use rtwk::style::Style;
use rtwk::style::TerminalBrightness;
use rtwk::style::Theme;
use rtwk::style::ThemeData;
use rtwk_core::input::Brightness;
use rtwk_core::style::Attrs;
use rtwk_core::style::Color;
use rtwk_rich::markdown::MarkdownTheme;
use rtwk_rich::style::{Style as MarkdownStyle, Theme as RichTheme};

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

    /// A shared terminal-derived surface for the draft and submitted prompts.
    pub(crate) fn prompt_surface<S>(&self, cx: &mut Cx<'_, S>, history: bool) -> Style {
        let bg = self.0.background.bg.filter(|bg| bg.is_rgb()).or_else(|| {
            let [_, r, g, b] = TerminalBrightness::background_of(cx)?.to_be_bytes();
            Some(Color::rgb(r, g, b))
        });
        let Some(bg) = bg else {
            return if history {
                self.0.surface.clone()
            } else {
                Style::new()
            };
        };
        let (toward, strength) = match (self.0.brightness == Brightness::Dark, history) {
            (true, true) => (Color::rgb(255, 255, 255), 0.16),
            (true, false) => (Color::rgb(255, 255, 255), 0.12),
            (false, true) => (Color::rgb(0, 0, 0), 0.02),
            (false, false) => (Color::rgb(0, 0, 0), 0.04),
        };
        Style::new().bg(Color::lerp(bg, toward, strength))
    }

    pub(crate) fn history_prompt_mark(&self) -> Style {
        self.0.text.clone().attrs(Attrs::BOLD | Attrs::DIM)
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

    pub(crate) fn transcript_markdown<S>(&self, cx: &mut Cx<'_, S>) -> MarkdownTheme {
        // The original's built-in Mocha and Latte Markdown roles.
        let brightness =
            TerminalBrightness::background_of(cx).map_or(self.0.brightness, Brightness::of_rgb);
        let (heading, strong, link) = match brightness {
            Brightness::Dark => (
                Color::rgb(0xf9, 0xe2, 0xaf),
                Color::rgb(0xf3, 0x8b, 0xa8),
                Color::rgb(0x89, 0xb4, 0xfa),
            ),
            Brightness::Light => (
                Color::rgb(0xa1, 0x67, 0x15),
                Color::rgb(0xd2, 0x0f, 0x39),
                Color::rgb(0x1e, 0x66, 0xf5),
            ),
        };
        let heading = MarkdownStyle::new().fg(heading);
        let italic_heading = heading.clone().attrs(Attrs::ITALIC);
        MarkdownTheme {
            heading1: heading.clone().attrs(Attrs::BOLD | Attrs::UNDERLINE),
            heading2: heading.clone().attrs(Attrs::BOLD),
            heading3: heading.attrs(Attrs::BOLD | Attrs::ITALIC),
            heading4: italic_heading.clone(),
            heading5: italic_heading.clone(),
            heading6: italic_heading,
            strong: MarkdownStyle::new().fg(strong).attrs(Attrs::BOLD),
            link: MarkdownStyle::new().fg(link).attrs(Attrs::UNDERLINE),
            // The oracle's ordered markers are ANSI light blue and a block quote,
            // bar and text, is green (markdown_render.rs `MarkdownStyles`).
            marker: MarkdownStyle::new().fg(Color::indexed(12)),
            quote: MarkdownStyle::new().fg(Color::GREEN),
            quote_bar: MarkdownStyle::new().fg(Color::GREEN),
            code_block: MarkdownStyle::new(),
            table_header: self.accent().attrs(Attrs::BOLD),
            table_border: self.dim().attrs(Attrs::DIM),
            ..MarkdownTheme::from_theme(&RichTheme::from_data(&self.0))
        }
    }

    /// The theme's info colour (the oracle's cyan accents).
    pub fn info(&self) -> Style {
        self.0.info.clone()
    }

    /// The theme's warning colour (the oracle's yellow notes).
    pub fn warning(&self) -> Style {
        self.0.warning.clone()
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

    /// The composer's shell mode: its `!` prompt and footer label (the
    /// oracle's light red).
    pub fn shell(&self) -> Style {
        Style::new().fg(Color::indexed(9))
    }

    pub fn selected(&self) -> Style {
        self.0.accent.clone().attrs(Attrs::BOLD)
    }

    pub fn error(&self) -> Style {
        self.0.error.clone()
    }

    /// The row background of an added or removed diff line (oracle diff_render.rs, the
    /// truecolor palette: dark tints, GitHub's on a light terminal; the presenter lowers it
    /// to the terminal's depth, as the oracle's 256-colour table does).
    pub fn diff_line(&self, added: bool) -> Color {
        match (self.0.brightness == Brightness::Dark, added) {
            (true, true) => Color::rgb(33, 58, 43),
            (true, false) => Color::rgb(74, 34, 29),
            (false, true) => Color::rgb(218, 251, 225),
            (false, false) => Color::rgb(255, 235, 233),
        }
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
