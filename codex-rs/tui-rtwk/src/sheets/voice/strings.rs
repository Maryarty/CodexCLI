//! The strings of `voice` (S25): a table of `&'static str` fields and typed
//! messages (`Msg<N>` behind a method) per locale, read by reference through
//! `of(cx)`. `EN` is the oracle's text byte for byte; each group of strings
//! names the oracle file it comes from. A language is one more `const` here,
//! registered with `with_ru(&RU)`-style on `TABLES`; a screen never changes
//! for it. Nothing else in this directory holds user-visible text (the rules
//! test `adr010_no_user_visible_literal_outside_the_string_tables`).

use rtwk::l10n::DependCx;

use crate::l10n::Lang;
use crate::l10n::Tables;
use crate::l10n::lang;

/// The strings of this screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Strings {
    /// The name of this screen in the debug menu and on its slot dialog.
    pub title: &'static str,
    pub voice: &'static str,
    pub voice_prefix: &'static str,
    pub active_marker: &'static str,
    pub quiet_marker: &'static str,
    pub control_gap: &'static str,
    pub connecting: &'static str,
    pub listening: &'static str,
    pub muted: &'static str,
    pub mute: &'static str,
    pub unmute: &'static str,
    pub mute_command: &'static str,
    pub stop_command: &'static str,
    pub microphone: &'static str,
    pub speaker: &'static str,
}

/// English: the oracle's text (the slice names the oracle file per group).
pub const EN: Strings = Strings {
    title: "voice and realtime strip",
    // bottom_pane/voice_strip.rs
    voice: "voice",
    voice_prefix: "voice ",
    active_marker: "●",
    quiet_marker: "◌",
    control_gap: "   ",
    connecting: " connecting",
    listening: " listen",
    muted: " muted",
    mute: " mute",
    unmute: " unmute",
    mute_command: "/voice mute",
    stop_command: "/voice stop",
    microphone: "  mic ",
    speaker: "  codex ",
};

impl Strings {
    pub fn lock_stop(&self, reason: codex_tui_core::screens::voice::LockStop) -> &'static str {
        use codex_tui_core::screens::voice::LockStop;
        match reason {
            LockStop::Locked => "Voice stopped because your session locked.",
            LockStop::MonitorFailed => {
                "Voice conversation stopped because session lock monitoring failed."
            }
        }
    }
}

static TABLES: Tables<Strings> = Tables::english(&EN);

/// The table of the locale below `cx`.
pub fn of(cx: &mut impl DependCx) -> &'static Strings {
    TABLES.pick(lang(cx))
}

/// The title of the screen in `lang`, for code with no tree (the debug
/// menu's rows).
pub fn title(lang: Lang) -> &'static str {
    TABLES.pick(lang).title
}
