//! The syntax theme in force (oracle `render/highlight.rs`: `current_syntax_theme`,
//! `resolve_theme_by_name`, `foreground_style_for_scopes`, `diff_scope_backgrounds`),
//! as the Flutter counterpart of a `ThemeExtension`: a value the app's `Tokens`
//! carry, read from the settings model's picked name, which the Markdown theme and
//! the diff rows ask for the colours of the scopes they draw.
//!
//! The name resolves as the oracle's does: a bundled theme by its kebab-case name;
//! no name, or one nothing resolves, is the adaptive default (catppuccin-mocha on a
//! dark terminal, catppuccin-latte on a light one); the empty name stands for the
//! oracle's `Theme::default()`, a theme with no scope in it, which the oracle's own
//! tests set. The scopes are data ([`scopes`]); no syntax-highlighting crate is linked.

use rtwk::style::color_mix::contrast_rgb;
use rtwk_core::style::Color;

mod models;
mod scopes;

/// What a theme paints a scope with (syntect's colour with bat's alpha encoding
/// read, `convert_syntect_color`; the alpha-1 terminal-default marker is in none of
/// the scopes asked for, the generator found).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Paint {
    Rgb(u8, u8, u8),
    /// A palette entry of the terminal (alpha 0: the `ansi` and `base16` themes).
    Ansi(u8),
}

/// The scopes of one theme the screens ask for.
#[derive(Clone, Copy, Debug)]
struct Scopes {
    fg: &'static [(&'static str, Paint)],
    bg: &'static [(&'static str, Paint)],
}

/// The theme with no scope: the oracle's `Theme::default()`.
static NO_SCOPES: Scopes = Scopes { fg: &[], bg: &[] };

/// The syntax theme in force, as the scopes it paints.
#[derive(Clone, Copy, Debug)]
pub struct SyntaxPalette(&'static Scopes);

impl SyntaxPalette {
    /// The theme `name` (the settings model's `syntax_theme`) is, on a terminal
    /// whose default background is `terminal_bg` (`0xRRGGBB`, if it told).
    pub fn named(name: Option<&str>, terminal_bg: Option<u32>) -> Self {
        match name {
            Some("") => Self(&NO_SCOPES),
            Some(name) => scopes::THEMES
                .iter()
                .chain(models::THEMES)
                .find(|(theme, _)| *theme == name)
                .map_or_else(|| Self::adaptive(terminal_bg), |(_, scopes)| Self(scopes)),
            None => Self::adaptive(terminal_bg),
        }
    }

    /// The default theme of a terminal (`adaptive_default_theme_selection`).
    pub fn adaptive(terminal_bg: Option<u32>) -> Self {
        let name = if terminal_bg.is_some_and(is_light) {
            "catppuccin-latte"
        } else {
            "catppuccin-mocha"
        };
        Self::named(Some(name), None)
    }

    /// The colour of the first of `scopes` the theme gives a foreground
    /// (`foreground_color_for_scopes`).
    pub fn foreground(&self, scopes: &[&str]) -> Option<Color> {
        scopes.iter().find_map(
            |scope| match self.0.fg.iter().find(|(name, _)| name == scope)?.1 {
                Paint::Rgb(r, g, b) => Some(Color::rgb(r, g, b)),
                Paint::Ansi(index) => Some(Color::indexed(index)),
            },
        )
    }

    /// As [`foreground`](Self::foreground), kept readable on the terminal's
    /// background (`MarkdownStyles::for_theme`'s `readable_color_on`).
    pub fn readable_foreground(&self, scopes: &[&str], terminal_bg: Option<u32>) -> Option<Color> {
        self.foreground(scopes)
            .map(|color| readable_on(color, terminal_bg))
    }

    /// The theme's fill of an added or removed diff line, `markup.*` before
    /// `diff.*` (`diff_scope_backgrounds`).
    pub fn diff_background(&self, inserted: bool) -> Option<Color> {
        let scopes = if inserted {
            ["markup.inserted", "diff.inserted"]
        } else {
            ["markup.deleted", "diff.deleted"]
        };
        scopes.iter().find_map(
            |scope| match self.0.bg.iter().find(|(name, _)| name == scope)?.1 {
                Paint::Rgb(r, g, b) => Some(Color::rgb(r, g, b)),
                Paint::Ansi(_) => None,
            },
        )
    }
}

/// The oracle's `color::is_light`: luma above 128.
pub(crate) fn is_light(rgb: u32) -> bool {
    let [_, r, g, b] = rgb.to_be_bytes();
    0.299 * f32::from(r) + 0.587 * f32::from(g) + 0.114 * f32::from(b) > 128.0
}

/// Text colour `preferred` on the terminal's background (`style::readable_color_on` at
/// truecolor, `contrast::foreground`): kept when it reads (WCAG 4.5), else the nearest
/// step from black or white (whichever contrasts more) that does. A palette entry stays
/// the terminal's; the presenter lowers the rest to the terminal's depth.
pub(crate) fn readable_on(preferred: Color, background: Option<u32>) -> Color {
    const MIN_TEXT_CONTRAST: f64 = 4.5;
    let Some(background) = background else {
        return preferred;
    };
    if !preferred.is_rgb() {
        return preferred;
    }
    let want = rgb_u32(preferred.r(), preferred.g(), preferred.b());
    if contrast_rgb(want, background) >= MIN_TEXT_CONTRAST {
        return preferred;
    }
    let end = if contrast_rgb(0x00_0000, background) >= contrast_rgb(0xff_ffff, background) {
        0u8
    } else {
        255u8
    };
    let blend = |step: u32, pref: u8| {
        let alpha = step as f32 / 255.0;
        (f32::from(end) * alpha + f32::from(pref) * (1.0 - alpha)) as u8
    };
    let (r, g, b) = (1..=255)
        .map(|step| {
            (
                blend(step, preferred.r()),
                blend(step, preferred.g()),
                blend(step, preferred.b()),
            )
        })
        .find(|&(r, g, b)| contrast_rgb(rgb_u32(r, g, b), background) >= MIN_TEXT_CONTRAST)
        .unwrap_or((end, end, end));
    Color::rgb(r, g, b)
}

fn rgb_u32(r: u8, g: u8, b: u8) -> u32 {
    u32::from_be_bytes([0, r, g, b])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_adaptive_default_follows_the_terminal() {
        let dark = SyntaxPalette::named(None, Some(0x12_14_1e));
        let light = SyntaxPalette::named(Some("no-such-theme"), Some(0xff_ff_ff));
        assert_ne!(
            dark.foreground(&["markup.heading.markdown"]),
            light.foreground(&["markup.heading.markdown"])
        );
        assert_eq!(
            SyntaxPalette::named(Some("catppuccin-latte"), None).foreground(&["variable"]),
            light.foreground(&["variable"])
        );
    }

    #[test]
    fn the_empty_name_is_a_theme_without_scopes() {
        let empty = SyntaxPalette::named(Some(""), Some(0xff_ff_ff));
        assert_eq!(empty.foreground(&["markup.quote.markdown"]), None);
        assert_eq!(empty.diff_background(true), None);
    }

    #[test]
    fn a_bundled_theme_gives_diff_backgrounds_and_ansi_entries() {
        let github = SyntaxPalette::named(Some("github"), None);
        assert_eq!(
            github.diff_background(true),
            Some(Color::rgb(221, 255, 221))
        );
        assert_eq!(
            SyntaxPalette::named(Some("ansi"), None).foreground(&["markup.heading.markdown"]),
            Some(Color::indexed(4))
        );
    }

    #[test]
    fn a_colour_that_does_not_read_is_taken_toward_black_or_white() {
        let pale = Color::rgb(250, 250, 200);
        let on_white = readable_on(pale, Some(0xff_ff_ff));
        assert_ne!(on_white, pale);
        let rgb = rgb_u32(on_white.r(), on_white.g(), on_white.b());
        assert!(contrast_rgb(rgb, 0xff_ff_ff) >= 4.5);
        assert_eq!(readable_on(pale, Some(0x12_14_1e)), pale);
        assert_eq!(readable_on(pale, None), pale);
        assert_eq!(
            readable_on(Color::indexed(4), Some(0xff_ff_ff)),
            Color::indexed(4)
        );
    }
}
