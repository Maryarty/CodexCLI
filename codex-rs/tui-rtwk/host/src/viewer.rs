//! What the host finds out about the viewer's terminal and clock, once, and sends the cells
//! screen: how a web link is written (oracle `markdown_render/web_links.rs`
//! `WebLinkDisplay::for_terminal`) and the 12/24-hour clock (`clock_format.rs`, detected in
//! `analytics::clock`).

use std::io::IsTerminal;

use codex_core::config::Config;
use codex_terminal_detection::{TerminalInfo, TerminalName, terminal_info};
use codex_tui_core::app::AppEv;
use codex_tui_core::screens::ScreenEv;
use codex_tui_core::screens::cells::{ClockFormat, Ev, LinkDisplay, Startup, ViewerClock};

/// A terminal that shows a hyperlink as its label alone writes the label; one that does not
/// (a multiplexer, `TERM=dumb`/`screen*`/`tmux*`, an unknown one) keeps the destination in
/// parentheses so a label never hides the only usable URL.
fn link_display(info: &TerminalInfo, term: Option<&str>) -> LinkDisplay {
    if info.multiplexer.is_some()
        || term.is_some_and(|term| {
            term == "dumb" || term.starts_with("screen") || term.starts_with("tmux")
        })
    {
        return LinkDisplay::WithDestination;
    }
    match info.name {
        TerminalName::Ghostty
        | TerminalName::Iterm2
        | TerminalName::WezTerm
        | TerminalName::Rex
        | TerminalName::Kitty
        | TerminalName::VsCode
        | TerminalName::Alacritty
        | TerminalName::WindowsTerminal
        | TerminalName::Konsole
        | TerminalName::GnomeTerminal
        | TerminalName::Vte => LinkDisplay::LabelOnly,
        TerminalName::AppleTerminal
        | TerminalName::WarpTerminal
        | TerminalName::Dumb
        | TerminalName::Unknown => LinkDisplay::WithDestination,
    }
}

/// The facts as events of the cells screen: the terminal's links, the clock, the config's
/// `show_raw_agent_reasoning` and how the run started (the project's trust was never set: the
/// user's first run, oracle `is_first_run`; the model-availability notice is not found yet).
pub fn events(config: &Config) -> [AppEv; 4] {
    let display = if !std::io::stdout().is_terminal() || std::env::var_os("STY").is_some() {
        LinkDisplay::WithDestination
    } else {
        link_display(&terminal_info(), std::env::var("TERM").ok().as_deref())
    };
    let clock = ViewerClock {
        format: if crate::analytics::clock::hour24() {
            ClockFormat::TwentyFourHour
        } else {
            ClockFormat::TwelveHour
        },
        utc_offset: None,
    };
    [
        AppEv::Screen(ScreenEv::Cells(Ev::LinkDisplay(display))),
        AppEv::Screen(ScreenEv::Cells(Ev::Viewer(clock))),
        AppEv::Screen(ScreenEv::Cells(Ev::RawReasoning(
            config.show_raw_agent_reasoning,
        ))),
        AppEv::Screen(ScreenEv::Cells(Ev::Startup(Startup {
            first_run: config.active_project.trust_level.is_none(),
            tooltip: None,
        }))),
    ]
}

#[cfg(test)]
mod tests {
    use codex_terminal_detection::Multiplexer;

    use super::*;

    fn info(name: TerminalName, multiplexer: Option<Multiplexer>) -> TerminalInfo {
        TerminalInfo {
            name,
            term_program: None,
            version: None,
            term: None,
            multiplexer,
            backend: None,
        }
    }

    #[test]
    fn a_terminal_with_hyperlinks_hides_the_destination() {
        for name in [
            TerminalName::Ghostty,
            TerminalName::Iterm2,
            TerminalName::Kitty,
        ] {
            assert_eq!(
                link_display(&info(name, None), Some("xterm-256color")),
                LinkDisplay::LabelOnly
            );
        }
        for name in [
            TerminalName::AppleTerminal,
            TerminalName::Unknown,
            TerminalName::Dumb,
        ] {
            assert_eq!(
                link_display(&info(name, None), None),
                LinkDisplay::WithDestination
            );
        }
    }

    #[test]
    fn a_multiplexer_or_a_screen_term_keeps_the_destination() {
        let tmux = Some(Multiplexer::Tmux { version: None });
        assert_eq!(
            link_display(&info(TerminalName::Ghostty, tmux), None),
            LinkDisplay::WithDestination
        );
        for term in ["dumb", "screen-256color", "tmux-256color"] {
            assert_eq!(
                link_display(&info(TerminalName::Ghostty, None), Some(term)),
                LinkDisplay::WithDestination,
                "{term}"
            );
        }
    }
}
