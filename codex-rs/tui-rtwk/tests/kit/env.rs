//! Env seeds: what a test sets in the world around the app, before it starts.
//!
//! The oracle's tests change the terminal (`set_default_colors`, the palette
//! a probe found), the process (`FORCE_COLOR`, the highlight theme), and the
//! disk and cwd of the session. A kit screen gets each of them from one call
//! on the builder (`Open`), through what `rtwk_test` offers (`Options`: the
//! terminal's default colours and colour depth) and the app's own events:
//!
//! - `terminal_colors(fg, bg)`: the terminal answers the colour queries with
//!   these defaults (`0xRRGGBB`); without it no palette is known, as in a
//!   terminal that never answers (the Tester's default).
//! - `light()` / `dark()`: the same with the oracle's two typical terminals.
//! - `palette(&[(index, rgb)])`: the terminal's ANSI palette entries.
//! - `color_depth(depth)`: the colours the terminal shows (`FORCE_COLOR=0`
//!   is `ColorDepth::None`, `1` is `Ansi16`, `2` is `Ansi256`, `3` is
//!   `TrueColor`); the app's colours come down to them as they are written.
//! - `syntax_theme(name)`: the config's `[tui] theme` (the highlight theme in
//!   force), delivered as the host does, the settings screen's facts.
//! - `env(name, value)`: a variable of the app's environment (empty by
//!   default: what the shell exports changes nothing).
//!
//! Files of the session: `kit::image_file(name)` writes a tiny PNG, `Open::cwd`
//! (markdown seeds) names the session's directory.

use codex_tui_core::app::AppEv;
use codex_tui_core::screens::ScreenEv;
use codex_tui_core::screens::settings::{Ev as SettingsEv, Facts};
use rtwk_core::style::ColorMode;
use rtwk_test::Options;

use super::seed::Open;
use crate::common::Harness;

/// The colours a terminal shows (`FORCE_COLOR` of the oracle's tests).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorDepth {
    /// No colours, only attributes (`FORCE_COLOR=0`).
    None,
    /// The 16 ANSI colours (`FORCE_COLOR=1`).
    Ansi16,
    /// The 256 palette entries (`FORCE_COLOR=2`).
    Ansi256,
    /// 24-bit colour (`FORCE_COLOR=3`, the default).
    TrueColor,
}

impl ColorDepth {
    fn mode(self) -> ColorMode {
        match self {
            ColorDepth::None => ColorMode::None,
            ColorDepth::Ansi16 => ColorMode::Ansi16,
            ColorDepth::Ansi256 => ColorMode::Ansi256,
            ColorDepth::TrueColor => ColorMode::TrueColor,
        }
    }
}

/// The seeds of the world around the app (`Open::env` and the rest).
#[derive(Clone, Default)]
pub(super) struct Env {
    foreground: Option<u32>,
    background: Option<u32>,
    palette: Vec<(u8, u32)>,
    depth: Option<ColorDepth>,
    vars: Vec<(&'static str, &'static str)>,
    syntax_theme: Option<&'static str>,
}

impl Env {
    /// The terminal as `rtwk_test` makes it for these seeds.
    pub(super) fn options(&self, width: u16, height: u16) -> Options {
        let mut options = Options::new().size(width, height);
        if let Some(bg) = self.background {
            options = options.background_rgb(bg);
        }
        for (name, value) in &self.vars {
            options = options.env(name, value);
        }
        let (fg, palette, depth) = (self.foreground, self.palette.clone(), self.depth);
        options.terminal(move |terminal| {
            terminal.foreground_rgb = fg;
            terminal.palette = palette;
            if let Some(depth) = depth {
                terminal.color_mode = depth.mode();
            }
        })
    }

    /// An app event is to be delivered (the app has to be connected first).
    pub(super) fn needs_app(&self) -> bool {
        self.syntax_theme.is_some()
    }

    /// Delivers what is an app event rather than a terminal's property, once
    /// the app is connected.
    pub(super) fn deliver(&self, h: &mut Harness) {
        let Some(name) = self.syntax_theme else {
            return;
        };
        let facts = Facts {
            syntax_theme: Some(name.to_string()),
            ..Facts::default()
        };
        h.connect();
        h.inject([AppEv::Screen(ScreenEv::Settings(SettingsEv::Facts(facts)))]);
    }
}

impl Open {
    /// The terminal's default colours (`0xRRGGBB`), as its answer to the
    /// colour queries gives them: the oracle's `set_default_colors`.
    pub fn terminal_colors(mut self, foreground: u32, background: u32) -> Open {
        self.env.foreground = Some(foreground);
        self.env.background = Some(background);
        self
    }

    /// A light terminal as the oracle's light-palette tests set one:
    /// foreground `555753`, background `ffffff`.
    pub fn light(self) -> Open {
        self.terminal_colors(0x55_57_53, 0xff_ff_ff)
    }

    /// A dark terminal: foreground `eeeeee`, background `1e1e1e`.
    pub fn dark(self) -> Open {
        self.terminal_colors(0xee_ee_ee, 0x1e_1e_1e)
    }

    /// The terminal's ANSI palette entries: `(index, 0xRRGGBB)`.
    pub fn palette(mut self, entries: &[(u8, u32)]) -> Open {
        self.env.palette = entries.to_vec();
        self
    }

    /// The colours the terminal shows (`FORCE_COLOR`).
    pub fn color_depth(mut self, depth: ColorDepth) -> Open {
        self.env.depth = Some(depth);
        self
    }

    /// The syntax theme of the config (`[tui] theme`), by name.
    pub fn syntax_theme(mut self, name: &'static str) -> Open {
        self.env.syntax_theme = Some(name);
        self
    }

    /// A variable of the app's environment.
    pub fn env(mut self, name: &'static str, value: &'static str) -> Open {
        self.env.vars.push((name, value));
        self
    }
}
