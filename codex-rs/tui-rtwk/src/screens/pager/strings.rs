//! The strings of `pager` (S16): a table of `&'static str` fields per locale,
//! read by reference through `of(cx)`. `EN` is the oracle's text byte for
//! byte; each group names the oracle file it comes from. A language is one
//! more `const` here, registered with `with_ru(&RU)`-style on `TABLES`; a
//! screen never changes for it. Nothing else in this directory holds
//! user-visible text (the rules test
//! `adr010_no_user_visible_literal_outside_the_string_tables`).

use rtwk::l10n::DependCx;

use crate::l10n::Lang;
use crate::l10n::Tables;
use crate::l10n::lang;

/// The strings of this screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Strings {
    /// The name of this screen in the debug menu and on its slot dialog.
    pub title: &'static str,
    /// The slot's heading (the empty form).
    /// Oracle: `pager_overlay/transcript.rs` (the transcript overlay's title).
    pub transcript_title: &'static str,
    /// The navigation hints (oracle: `pager_overlay.rs` `render_navigation_hints`
    /// with the default keymap's primary keys).
    pub navigation: &'static str,
    /// Oracle: `StaticOverlay::render_hints`, the second row.
    pub close_static: &'static str,
    /// Oracle: `TranscriptOverlay::render_hints`, the second row.
    pub close_transcript: &'static str,
    /// Oracle: `transcript_view/footer.rs` `status_line_with_navigation`.
    pub select: &'static str,
    pub loading_marker: &'static str,
    pub loading: &'static str,
    pub earlier: &'static str,
    pub retry: &'static str,
    /// Oracle: `render_header`: `"/ "` over the row's width, the title over
    /// its start (here wider than any terminal, cut by the row).
    pub header_fill: &'static str,
}

/// English: the oracle's text.
pub const EN: Strings = Strings {
    title: "transcript pager",
    transcript_title: "T R A N S C R I P T",
    navigation: " ↑/↓ to scroll · pgup/pgdn to page · home/end to jump",
    close_static: " q close",
    close_transcript: " q close · f3 find · esc browse prompts",
    select: "⌃space select",
    loading_marker: "↑ ",
    loading: "Loading earlier messages…",
    earlier: "Earlier messages available.  ",
    retry: "Retry history: ⌥</⌃home.",
    header_fill: "/ / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / ",
};

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
