//! Styles of the app. The chrome follows the root rtwk theme. Markdown
//! typography uses the original's adaptive default palette.

use std::rc::Rc;

use codex_tui_core::data::links::LinkDisplay;
use codex_tui_core::screens::ScreenStates;
use rtwk::Cx;
use rtwk::provider::{Provider, select};
use rtwk::style::Style;
use rtwk::style::TerminalBrightness;
use rtwk::style::Theme;
use rtwk::style::ThemeData;
use rtwk_core::input::Brightness;
use rtwk_core::style::Attrs;
use rtwk_core::style::Color;
use rtwk_rich::markdown::MarkdownTheme;
use rtwk_rich::style::{Style as MarkdownStyle, Theme as RichTheme};

pub(crate) mod effort;
mod syntax;

pub use syntax::SyntaxPalette;

/// The theme of the app's root: the terminal's own colours, dark until the
/// terminal says otherwise.
pub fn data() -> Rc<ThemeData> {
    Rc::new(ThemeData::terminal(Brightness::Dark))
}

/// The scope of each heading level, before the one all levels share.
const HEADING_SCOPES: [&str; 6] = [
    "markup.heading.1.markdown",
    "markup.heading.2.markdown",
    "markup.heading.3.markdown",
    "markup.heading.4.markdown",
    "markup.heading.5.markdown",
    "markup.heading.6.markdown",
];

/// The style tokens of one theme, with the syntax theme in force (the settings
/// model's pick, which the Markdown roles and the diff rows are drawn from).
#[derive(Clone, Debug)]
pub struct Tokens {
    data: Rc<ThemeData>,
    syntax: SyntaxPalette,
    /// The terminal's default background (`0xRRGGBB`), if it told.
    terminal_bg: Option<u32>,
}

impl Tokens {
    pub fn new(data: Rc<ThemeData>) -> Self {
        let syntax = SyntaxPalette::adaptive(None);
        Self {
            data,
            syntax,
            terminal_bg: None,
        }
    }

    /// The tokens of the theme `cx` is under, with the syntax theme the user
    /// picked (`settings::Model::syntax_theme`) where the app's models are
    /// provided.
    pub fn of<S>(cx: &mut Cx<'_, S>) -> Self {
        let data = Theme::of(cx);
        let terminal_bg = TerminalBrightness::background_of(cx);
        let picked = if Provider::<ScreenStates>::maybe_of(cx, false).is_some() {
            select::<ScreenStates, _, _>(cx, |states| states.settings.syntax_theme.clone())
        } else {
            None
        };
        Self {
            syntax: SyntaxPalette::named(picked.as_deref(), terminal_bg),
            data,
            terminal_bg,
        }
    }

    /// How a web link's destination is written (`cells::Model::link_display`, the host's fact about
    /// the terminal): the slice a message draws, the oracle's default where no app is above.
    pub fn link_display<S: 'static>(cx: &mut Cx<'_, S>) -> LinkDisplay {
        if Provider::<ScreenStates>::maybe_of(cx, false).is_some() {
            select::<ScreenStates, _, _>(cx, |states| states.cells.link_display)
        } else {
            LinkDisplay::default()
        }
    }

    /// A shared terminal-derived surface for the draft and submitted prompts (oracle
    /// `style.rs`: `user_message_bg_rgb` for the composer, `history_prompt_style` for a
    /// submitted prompt): the terminal's background shaded toward black on a light
    /// terminal and toward white on a dark one, a light touch for the history.
    pub(crate) fn prompt_surface<S>(&self, cx: &mut Cx<'_, S>, history: bool) -> Style {
        let terminal = self
            .terminal_bg
            .or_else(|| TerminalBrightness::background_of(cx))
            .or_else(|| {
                let bg = self.data.background.bg.filter(|bg| bg.is_rgb())?;
                Some(u32::from_be_bytes([0, bg.r(), bg.g(), bg.b()]))
            });
        let Some(terminal) = terminal else {
            return if history {
                self.data.surface.clone()
            } else {
                Style::new()
            };
        };
        let [_, r, g, b] = terminal.to_be_bytes();
        let (toward, alpha) = match (syntax::is_light(terminal), history) {
            (true, true) => (0u8, 0.02_f32),
            (true, false) => (0, 0.04),
            (false, true) => (255, 0.16),
            (false, false) => (255, 0.12),
        };
        // the oracle's `blend`: each channel truncated
        let shade =
            |channel: u8| (f32::from(toward) * alpha + f32::from(channel) * (1.0 - alpha)) as u8;
        Style::new().bg(Color::rgb(shade(r), shade(g), shade(b)))
    }

    /// A tint of the terminal's own background: `dark_by` toward white on a
    /// dark terminal, `light_by` toward black on a light one (the resume
    /// picker's zebra rows and loading overlay; oracle `color::blend` of the
    /// default background). No fill when the terminal's background is unknown.
    pub(crate) fn tint<S>(&self, cx: &mut Cx<'_, S>, dark_by: f64, light_by: f64) -> Style {
        let Some(bg) = TerminalBrightness::background_of(cx) else {
            return Style::new();
        };
        let [_, r, g, b] = bg.to_be_bytes();
        let (toward, strength) = if self.data.brightness == Brightness::Light {
            (Color::rgb(0, 0, 0), light_by)
        } else {
            (Color::rgb(255, 255, 255), dark_by)
        };
        Style::new().bg(Color::lerp(Color::rgb(r, g, b), toward, strength))
    }

    /// The background a content keeps when it names a colour of its own (the
    /// side preview of a picker, `preserve_side_content_bg`): ratatui's names of
    /// the sixteen ANSI colours (`blue`, `light_red`, `dark_gray`, ...), the
    /// terminal's own palette. A name that is none of them keeps no fill.
    pub(crate) fn tone(&self, name: &str) -> Style {
        const NAMES: [&str; 16] = [
            "black",
            "red",
            "green",
            "yellow",
            "blue",
            "magenta",
            "cyan",
            "gray",
            "dark_gray",
            "light_red",
            "light_green",
            "light_yellow",
            "light_blue",
            "light_magenta",
            "light_cyan",
            "white",
        ];
        let name = name.to_ascii_lowercase().replace(['-', ' '], "_");
        NAMES
            .iter()
            .position(|known| *known == name)
            .map_or_else(Style::new, |index| {
                Style::new().bg(Color::indexed(index as u8))
            })
    }

    /// The colour a thing that needs attention is marked with (the unread dot
    /// of a saved session): the oracle's fixed blue (`resume_picker.rs`
    /// `rgb_color((0x3b, 0x82, 0xf6))`), the same on every theme.
    pub fn attention(&self) -> Style {
        Style::new().fg(Color::rgb(0x3b, 0x82, 0xf6))
    }

    pub(crate) fn history_prompt_mark(&self) -> Style {
        self.data.text.clone().attrs(Attrs::BOLD | Attrs::DIM)
    }

    pub fn dim(&self) -> Style {
        self.data.dim()
    }

    pub fn bold(&self) -> Style {
        self.data.text.clone().attrs(Attrs::BOLD)
    }

    pub fn accent(&self) -> Style {
        self.data.accent.clone()
    }

    /// The Markdown roles of the transcript (oracle `MarkdownStyles::for_theme`): the
    /// colours of the syntax theme in force for headings, strong, emphasis, links,
    /// quotes, inline code and the ordered markers, kept readable on the terminal's
    /// background; the oracle's own accent and ANSI colours where the theme has none.
    pub(crate) fn transcript_markdown<S>(&self, _cx: &mut Cx<'_, S>) -> MarkdownTheme {
        let readable = |scopes: &[&str]| {
            let fg = self.syntax.readable_foreground(scopes, self.terminal_bg);
            fg.map_or_else(MarkdownStyle::new, |fg| MarkdownStyle::new().fg(fg))
        };
        let heading =
            |level: usize| readable(&[HEADING_SCOPES[level - 1], "markup.heading.markdown"]);
        let accent = self.oracle_accent();
        let link = MarkdownStyle::new()
            .fg(accent)
            .merge(&readable(&["markup.underline.link.markdown"]))
            .attrs(Attrs::UNDERLINE);
        let quote = MarkdownStyle::new()
            .fg(Color::GREEN)
            .merge(&readable(&["markup.quote.markdown"]));
        let code = self
            .syntax
            .foreground(&[
                "markup.inline.raw.string.markdown",
                "markup.raw.inline.markdown",
            ])
            .unwrap_or(accent);
        MarkdownTheme {
            heading1: heading(1).attrs(Attrs::BOLD | Attrs::UNDERLINE),
            heading2: heading(2).attrs(Attrs::BOLD),
            heading3: heading(3).attrs(Attrs::BOLD | Attrs::ITALIC),
            heading4: heading(4).attrs(Attrs::ITALIC),
            heading5: heading(5).attrs(Attrs::ITALIC),
            heading6: heading(6).attrs(Attrs::ITALIC),
            emphasis: readable(&["markup.italic.markdown"]).attrs(Attrs::ITALIC),
            strong: readable(&["markup.bold.markdown"]).attrs(Attrs::BOLD),
            code: MarkdownStyle::new().fg(code),
            link,
            // The ordered markers are ANSI light blue unless the theme says otherwise
            // (the bullets' own scope is the gap G73 item 4).
            marker: MarkdownStyle::new()
                .fg(Color::indexed(12))
                .merge(&readable(&["markup.list.numbered.markdown"])),
            quote: quote.clone(),
            quote_bar: quote,
            code_block: MarkdownStyle::new(),
            table_header: self.accent().attrs(Attrs::BOLD),
            table_border: self.dim().attrs(Attrs::DIM),
            ..MarkdownTheme::from_theme(&RichTheme::from_data(&self.data))
        }
    }

    /// The oracle's accent (`style::accent_color`).
    fn oracle_accent(&self) -> Color {
        self.accent_color_on(None)
    }

    /// The oracle's accent on the fill `surface` paints (`style::accent_color_on`): its
    /// blue on a dark surface, a deeper one on a light surface, kept readable on it;
    /// the terminal's background where `surface` is not a colour of its own.
    pub(crate) fn accent_on(&self, surface: &Style) -> Style {
        Style::new().fg(self.accent_color_on(surface.bg))
    }

    fn accent_color_on(&self, surface: Option<Color>) -> Color {
        let background = surface
            .filter(|fill| fill.is_rgb())
            .map(|fill| u32::from_be_bytes([0, fill.r(), fill.g(), fill.b()]))
            .or(self.terminal_bg);
        let preferred = if background.is_some_and(syntax::is_light) {
            Color::rgb(28, 100, 200)
        } else {
            Color::rgb(99, 168, 248)
        };
        syntax::readable_on(preferred, background)
    }

    /// The theme's info colour (the oracle's cyan accents).
    pub fn info(&self) -> Style {
        self.data.info.clone()
    }

    /// The theme's warning colour (the oracle's yellow notes).
    pub fn warning(&self) -> Style {
        self.data.warning.clone()
    }

    /// The theme's success colour (the oracle's green accents).
    pub fn success(&self) -> Style {
        self.data.success.clone()
    }

    /// The style of a chart series (the oracle's `plot/painting.rs` order:
    /// the terminal's cyan, magenta and green, then the terminal's own ink;
    /// an index past the last repeats it).
    pub fn series(&self, index: usize) -> Style {
        match index {
            0 => self.info(),
            1 => self.accent(),
            2 => self.success(),
            _ => Style::new(),
        }
    }

    pub(crate) fn blossom_palette(&self) -> BlossomPalette {
        let channels = |color: Color| (color.r(), color.g(), color.b());
        BlossomPalette {
            background: self
                .data
                .background
                .bg
                .filter(|color| color.is_rgb())
                .map_or((15, 20, 37), channels),
            foreground: self
                .data
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
        self.data.accent.clone().attrs(Attrs::BOLD)
    }

    pub fn error(&self) -> Style {
        self.data.error.clone()
    }

    /// The row background of an added or removed diff line (oracle diff_render.rs
    /// `resolve_diff_backgrounds_for`): the syntax theme's own `markup.inserted` /
    /// `markup.deleted` fill when it has one, else the truecolor palette: dark tints, GitHub's on a light terminal; the
    /// presenter lowers it to the terminal's depth, as the oracle's 256-colour table does.
    pub fn diff_line(&self, added: bool) -> Color {
        if let Some(fill) = self.syntax.diff_background(added) {
            return fill;
        }
        match (self.data.brightness == Brightness::Dark, added) {
            (true, true) => Color::rgb(33, 58, 43),
            (true, false) => Color::rgb(74, 34, 29),
            (false, true) => Color::rgb(218, 251, 225),
            (false, false) => Color::rgb(255, 235, 233),
        }
    }
}

/// The terminal's default foreground and background as it told them (the
/// oracle's `default_fg()` and `default_bg()`): what the status shimmer blends.
pub(crate) fn terminal_colors<S>(cx: &mut Cx<'_, S>) -> Option<(Rgb, Rgb)> {
    let channels = |rgb: u32| {
        let [_, r, g, b] = rgb.to_be_bytes();
        (r, g, b)
    };
    Some((
        channels(TerminalBrightness::foreground_of(cx)?),
        channels(TerminalBrightness::background_of(cx)?),
    ))
}

/// A colour as channels.
pub(crate) type Rgb = (u8, u8, u8);

/// The oracle's `color::blend`: `fg` over `bg` by `alpha` (channels cut off),
/// as a foreground.
pub(crate) fn blended(fg: Rgb, bg: Rgb, alpha: f64) -> Style {
    let channel = |f: u8, b: u8| (f64::from(f) * alpha + f64::from(b) * (1.0 - alpha)) as u8;
    Style::new().fg(Color::rgb(
        channel(fg.0, bg.0),
        channel(fg.1, bg.1),
        channel(fg.2, bg.2),
    ))
}

/// The terminal's own foreground (`key_hint`'s key labels: bold in it).
pub(crate) fn terminal_foreground(fg: Rgb) -> Style {
    Style::new().fg(Color::rgb(fg.0, fg.1, fg.2))
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
