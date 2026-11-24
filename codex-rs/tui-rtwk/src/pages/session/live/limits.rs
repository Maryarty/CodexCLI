//! The account's limits on the chat page: the usage notice in the row above the composer (oracle
//! `chatwidget/usage_notice.rs` `UsageNoticeState::line`, `app/composer_hints.rs`).
//!
//! What the notice says is `core`'s: `WarningLimits::notice()` names the window with the least
//! capacity left among those past their threshold, and the view picks the longest spelling the
//! row holds (`first_fitting_line`): the summary with its reset time, without it, the summary
//! alone, the compact one. The row is half the width, or as wide as the compact spelling needs;
//! a banner or a limit that stops the account takes its place (`usage_notice`).

use rtwk::layout::{Expanded, Row, SizedBox};
use rtwk::l10n::{Arg, Fmt};
use rtwk::provider::select;
use rtwk::style::Attrs;
use rtwk::text::segmenter_of;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, LayoutBuilder, StatelessWidget, impl_widget};

use codex_tui_core::app::SessionSlice;
use codex_tui_core::ids::BackendId;
use codex_tui_core::screens::ScreenStates;
use codex_tui_core::screens::cells::ClockFormat;
use codex_tui_core::screens::status::{Used, reset_time};

use crate::app::theme;
use crate::pages::session::cells::status::strings as status_strings;

/// The percent used from which the notice is bold.
const BOLD_FROM: i32 = 90;
/// The percent used from which it says `only`.
const ONLY_FROM: i32 = 95;
/// The notice's margin from the right edge, and the cells the row keeps free on the left
/// (`composer_hint`: the width less 2, the line one cell from the edge).
const MARGIN: i32 = 1;
const KEPT: i32 = 2;

/// The notice the row above the composer shows for `backend`'s account, if any: not while a
/// banner of the backend is visible for `model` or a limit that stops the account is known
/// (`usage_notice`).
pub(in crate::pages::session) fn notice_of(
    session: &SessionSlice,
    backend: BackendId,
    model: Option<&str>,
) -> Option<UsageNotice> {
    let limits = session.rate_snapshots.get(&backend)?.limits();
    if limits.blocked()
        || session
            .backend_banners
            .get(&backend)
            .is_some_and(|occurrence| occurrence.is_visible(model))
    {
        return None;
    }
    let (window, secondary) = limits.notice()?;
    Some(UsageNotice { window, secondary })
}

/// The usage notice: the window that is close to its limit, and whether it is the secondary one.
#[derive(Clone, Debug, PartialEq)]
pub(in crate::pages::session) struct UsageNotice {
    window: Used,
    secondary: bool,
}

impl_widget!(stateless eq UsageNotice);

impl StatelessWidget for UsageNotice {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let words = status_strings::of(cx);
        let tokens = theme::Tokens::of(cx);
        let (viewer, utc_offset, now) = select::<ScreenStates, _, _>(cx, |states| {
            (
                states.cells.clock,
                states.status.utc_offset_secs,
                states.status.now,
            )
        });
        let hour24 = viewer.format == ClockFormat::TwentyFourHour;
        let offset = viewer.utc_offset.unwrap_or(utc_offset);
        let mut style = tokens.warning();
        if self.window.percent >= BOLD_FROM {
            style = style.attrs(Attrs::BOLD);
        }
        let notice = self.clone();
        LayoutBuilder::new(move |cx, constraints| {
            let segmenter = segmenter_of(cx);
            let width = |text: &str| i32::try_from(segmenter.width(text)).unwrap_or(i32::MAX);
            let room = constraints.max_width - KEPT;
            let line = notice.line(words, room, now, offset, hour24, &width);
            if room <= 0 || line.is_empty() {
                return SizedBox::new(Some(0), Some(0), None).into();
            }
            SizedBox::new(
                Some(constraints.max_width),
                Some(1),
                Some(
                    Row::new()
                        .child(Expanded::new(1, SizedBox::new(Some(0), Some(0), None)))
                        .child(Text::line(line).style(style.clone()))
                        .child(SizedBox::new(Some(MARGIN), None, None))
                        .into(),
                ),
            )
            .into()
        })
        .into()
    }
}

impl UsageNotice {
    /// The longest spelling that fits `room` cells (empty when none does), `now` and `offset` the
    /// clock the reset time is told on (a `now` of 0: the host has not read it, every reset is
    /// ahead and today's).
    fn line(
        &self,
        words: &status_strings::Strings,
        room: i32,
        now: i64,
        offset: i32,
        hour24: bool,
        width: &dyn Fn(&str) -> i32,
    ) -> String {
        let window = self.window;
        let label = words.notice_window(window.window_minutes, self.secondary);
        let remaining = if window.percent >= 100 {
            words.notice_under_one.to_string()
        } else {
            format!("{}%", 100 - window.percent)
        };
        let emphasis = if window.percent >= ONLY_FROM {
            words.notice_only
        } else {
            ""
        };
        let summary = Fmt::new(
            words.notice_summary,
            [Arg::Text(label), Arg::Text(emphasis), Arg::Text(&remaining)],
            &words.num,
        )
        .to_string();
        let reset = window
            .resets_at
            .filter(|at| *at > now)
            .map(|at| {
                let read_at = if now > 0 { now } else { at };
                let when = words.when(&reset_time(at, read_at, offset), hour24);
                Fmt::new(words.notice_resets, [Arg::Text(&when)], &words.num).to_string()
            })
            .unwrap_or_default();
        let compact = Fmt::new(
            words.notice_compact,
            [Arg::Text(label), Arg::Text(&remaining)],
            &words.num,
        )
        .to_string();
        // half the row, but the compact spelling whole on a narrow one
        let own = (room / 2).max(width(&compact)).min(room);
        let status = words.notice_status;
        [
            format!("{summary}{reset}{status}"),
            format!("{summary}{status}"),
            summary,
            compact,
        ]
        .into_iter()
        .find(|candidate| width(candidate) <= own)
        .unwrap_or_default()
    }
}
