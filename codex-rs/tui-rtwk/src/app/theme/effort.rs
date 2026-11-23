//! The colours of the reasoning-effort ignition (oracle `effort_ignition.rs`,
//! `effort_ignition_styles.rs`, `effort_status_line.rs`, `color.rs`): the two
//! tiers' hues, the blend of a colour toward the terminal's own, and the
//! styles the animations paint with. The footer's animation code asks for a
//! style here and holds no colour value of its own.

use rtwk::Cx;
use rtwk::style::{Attrs, Color, Style, TerminalBrightness};
use rtwk_core::style::ColorMode;

use super::Rgb;

/// The effort tiers that play (`EffortTier`): the other efforts are plain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Grade {
    Max,
    Ultra,
}

impl Grade {
    /// `EffortTier::from_effort`, by the effort's wire name.
    pub(crate) fn from_effort(effort: Option<&str>) -> Option<Grade> {
        match effort {
            Some("max") => Some(Grade::Max),
            Some("ultra") => Some(Grade::Ultra),
            _ => None,
        }
    }

    /// `EffortTier::hues`.
    pub(crate) fn hues(self, on_light_bg: bool) -> [Rgb; 3] {
        match (self, on_light_bg) {
            (Grade::Max, false) => [(255, 178, 66), (255, 214, 120), (255, 120, 60)],
            (Grade::Max, true) => [(176, 98, 0), (150, 110, 0), (200, 70, 20)],
            (Grade::Ultra, false) => [(186, 130, 255), (255, 120, 220), (120, 170, 255)],
            (Grade::Ultra, true) => [(124, 58, 217), (190, 40, 150), (30, 100, 220)],
        }
    }

    fn accent(self, on_light_bg: bool) -> Rgb {
        self.hues(on_light_bg)[0]
    }

    /// The colour a terminal that cannot show the accent gets
    /// (`fallback_color`).
    fn fallback(self) -> Color {
        match self {
            Grade::Max => Color::YELLOW,
            Grade::Ultra => Color::MAGENTA,
        }
    }
}

/// `color::is_light`.
pub(crate) fn is_light(bg: Rgb) -> bool {
    0.299 * f32::from(bg.0) + 0.587 * f32::from(bg.1) + 0.114 * f32::from(bg.2) > 128.0
}

/// `color::blend`: `fg` over `bg` by `alpha`, each channel cut off.
pub(crate) fn blend(fg: Rgb, bg: Rgb, alpha: f32) -> Rgb {
    let channel = |f: u8, b: u8| (f32::from(f) * alpha + f32::from(b) * (1.0 - alpha)) as u8;
    (
        channel(fg.0, bg.0),
        channel(fg.1, bg.1),
        channel(fg.2, bg.2),
    )
}

fn rgb(color: Rgb) -> Color {
    Color::rgb(color.0, color.1, color.2)
}

/// The channels of a true colour; a palette colour or none is none.
pub(crate) fn channels(color: Option<Color>) -> Option<Rgb> {
    color
        .filter(|color| color.is_rgb())
        .map(|color| (color.r(), color.g(), color.b()))
}

/// The terminal's default colours, each as it told them (`default_fg()`,
/// `default_bg()`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Colors {
    pub fg: Option<Rgb>,
    pub bg: Option<Rgb>,
}

impl Colors {
    pub(crate) fn of<S>(cx: &mut Cx<'_, S>) -> Colors {
        let split = |rgb: u32| {
            let [_, r, g, b] = rgb.to_be_bytes();
            (r, g, b)
        };
        Colors {
            fg: TerminalBrightness::foreground_of(cx).map(split),
            bg: TerminalBrightness::background_of(cx).map(split),
        }
    }
}

/// `apply_fade`: a style's colour toward the tier's accent by `tint`, then
/// toward the terminal's background by `opacity` (dim below 0.7). A palette
/// colour keeps itself and only dims.
pub(crate) fn fade(style: &mut Style, grade: Grade, opacity: f32, tint: f32, colors: Colors) {
    let opacity = opacity.clamp(0.0, 1.0);
    let tint = tint.clamp(0.0, 1.0);
    let dim = |style: &mut Style| {
        if opacity < 0.7 {
            *style = style.clone().attrs(Attrs::DIM);
        }
    };
    if let Some(color) = style.fg
        && color != Color::DEFAULT
        && !color.is_rgb()
    {
        dim(style);
        return;
    }
    let Some(background) = colors.bg else {
        if tint > 0.5 {
            style.fg = Some(grade.fallback());
        }
        dim(style);
        return;
    };
    let foreground = match style.fg {
        Some(color) if color.is_rgb() => (color.r(), color.g(), color.b()),
        _ => colors.fg.unwrap_or(if is_light(background) {
            (32, 32, 32)
        } else {
            (224, 224, 224)
        }),
    };
    let accent = grade.accent(is_light(background));
    let faded = blend(blend(accent, foreground, tint), background, opacity);
    style.fg = Some(rgb(faded));
    dim(style);
}

/// A band column's background: `color` over the composer's surface.
pub(crate) fn band_style(color: Rgb) -> Style {
    Style::new().bg(rgb(color))
}

/// The Ultra wave's spark: the first hue, bold.
pub(crate) fn spark_style(color: Rgb) -> Style {
    Style::new().fg(rgb(color)).attrs(Attrs::BOLD)
}

/// The prompt glyph's accent (`accent_color_for`): the tier's colour, blended
/// from the terminal's foreground by `charge` (0 to 1 over the first 150 ms of
/// a run).
fn accent_target(grade: Grade, charge: f32, colors: Colors) -> Rgb {
    let accent = grade.accent(colors.bg.is_some_and(is_light));
    match colors.fg {
        Some(fg) => blend(accent, fg, charge.clamp(0.0, 1.0) * 0.86),
        None => accent,
    }
}

/// The prompt glyph's style on a terminal showing `mode`: the accent on true
/// colour and 256 colours, the tier's own yellow or magenta on 16, none
/// without colour.
pub(crate) fn glyph_style(grade: Grade, charge: f32, colors: Colors, mode: ColorMode) -> Style {
    let style = Style::new().attrs(Attrs::BOLD);
    match mode {
        ColorMode::TrueColor | ColorMode::Ansi256 => {
            style.fg(rgb(accent_target(grade, charge, colors)))
        }
        ColorMode::Ansi16 => style.fg(grade.fallback()),
        _ => style,
    }
}

/// The status line's model in the oracle's snapshots: cyan.
pub(crate) fn sample_model_style() -> Style {
    Style::new().fg(Color::CYAN)
}

/// Palette colours a fade must leave alone.
#[cfg(test)]
pub(crate) fn palette_styles() -> [Style; 3] {
    [Color::CYAN, Color::GREEN, Color::MAGENTA].map(|color| Style::new().fg(color))
}
