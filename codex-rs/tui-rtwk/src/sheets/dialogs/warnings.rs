//! The warnings viewer: one warning at a time, its full text in a scrolling
//! body, the viewer's own footer. Oracle: `bottom_pane/warnings_view.rs` (keys,
//! decisions) and `warnings_view_render.rs` (layout, footer).

use std::collections::BTreeSet;

use rtwk::focus::Focus;
use rtwk::layout::{
    BoxConstraints, BoxWidget, Column, ConstrainedBox, CrossAxisAlignment, EdgeInsets, Flexible,
    MainAxisSize, SizedBox, UNBOUNDED,
};
use rtwk::overlay::Dialog;
use rtwk::scroll::SingleChildScrollView;
use rtwk::style::Theme;
use rtwk::text::segmenter_of;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, LayoutBuilder};

use codex_tui_core::keymap::fixed;
use codex_tui_core::screens::dialogs::{Decision, Warning};

use super::dialog::{Msg, SmallDialogState};
use super::strings::Strings;
use crate::app::theme::Tokens;
use crate::keymap::bind_fixed;

/// `WARNING_PANEL_CONTENT_ROWS` less the header and its blank row.
const BODY_ROWS: i32 = 8;

/// The viewer's actions (`WarningsView::handle_key`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Turn {
    Previous,
    Next,
    /// `k`: keep this one and show the next; on the last page, close.
    KeepAndNext,
    Copy,
}

/// What a `Turn` asks of the dialog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Stay,
    /// The text of the page shown goes to the clipboard.
    Copy(String),
    /// The viewer closes.
    Finish,
}

/// The warnings shown one at a time (`warnings_view.rs`: `WarningsView`): a frozen copy of the
/// retained warnings, the page on screen, the pages that were shown (the oracle: drawn) and the
/// ones the user saved with `k`. The viewer's own state; only its `decision` reaches the model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Pages {
    entries: Vec<Warning>,
    current: usize,
    visited: BTreeSet<usize>,
    saved: BTreeSet<usize>,
}

impl Pages {
    pub(super) fn new(entries: Vec<Warning>) -> Self {
        let visited = if entries.is_empty() {
            BTreeSet::new()
        } else {
            BTreeSet::from([0])
        };
        Self {
            entries,
            current: 0,
            visited,
            saved: BTreeSet::new(),
        }
    }

    /// The warning on the page shown.
    fn page(&self) -> Option<&Warning> {
        self.entries.get(self.current)
    }

    pub(super) fn turn(&mut self, turn: Turn) -> Outcome {
        let last = self.entries.len().saturating_sub(1);
        match turn {
            Turn::Previous => self.current = self.current.saturating_sub(1),
            Turn::Next => self.current = (self.current + 1).min(last),
            Turn::KeepAndNext => {
                self.saved.insert(self.current);
                if self.current >= last {
                    return Outcome::Finish;
                }
                self.current += 1;
            }
            Turn::Copy => {
                return self
                    .page()
                    .map_or(Outcome::Stay, |entry| Outcome::Copy(entry.details.clone()));
            }
        }
        self.visited.insert(self.current);
        Outcome::Stay
    }

    /// What the user decided when the viewer closed (`WarningsView::close`): saved pages are
    /// saved, the other shown pages are dismissed, pages never shown are left alone. `None`
    /// when no page was shown or saved.
    pub(super) fn decision(&self) -> Option<Decision> {
        let mut dismissed = Vec::new();
        let mut saved = Vec::new();
        for (index, entry) in self.entries.iter().enumerate() {
            if self.saved.contains(&index) {
                saved.push(entry.clone());
            } else if self.visited.contains(&index) {
                dismissed.push(entry.clone());
            }
        }
        (!dismissed.is_empty() || !saved.is_empty()).then_some(Decision { dismissed, saved })
    }
}

pub(super) fn build(warnings: &Pages, s: &Strings, cx: &mut Cx<'_, SmallDialogState>) -> Focus {
    let tokens = Tokens::of(cx);
    let page = warnings.page();
    let title = match page {
        Some(entry) => s
            .warnings_title(warnings.current + 1, warnings.entries.len(), &entry.source)
            .to_string(),
        None => s.warnings_title_empty.to_string(),
    };
    let details = page.map_or(s.no_warnings, |entry| entry.details.as_str());
    // Keyed by the page: each page's body scrolls from its top.
    let body =
        AnyWidget::from(SingleChildScrollView::new(Text::new(details.to_string())).autofocus(true))
            .with_key(warnings.current);
    let hints = [s.hint_keep, s.hint_dismiss, s.hint_copy, s.hint_pages];
    let separator = s.hint_separator;
    let dim = tokens.dim();
    let seg = segmenter_of(cx);
    // The footer shows the hints that fit the width, in order; one that does not fit is
    // left out and the next is tried (`warnings_view_render.rs`).
    let hint = LayoutBuilder::new(move |_, constraints| {
        let mut shown: Vec<&str> = Vec::new();
        for hint in hints {
            shown.push(hint);
            if seg.width(&shown.join(separator)) as i32 > constraints.max_width {
                shown.pop();
            }
        }
        Text::line(shown.join(separator)).style(dim.clone()).into()
    });
    // The panel (`render_menu_surface`): a blank row on top, the title, a blank row, the
    // body, a blank row, all inset two cells on the panel colour; the hint row is under it.
    // The body is at most the oracle's body rows, fewer where the room is short (its body
    // is `Constraint::Min(0)` above the footer row).
    let surface = tokens.prompt_surface(cx, false);
    let content = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(Text::line(title).style(tokens.bold()))
        .child(Text::new(String::new()))
        .child(Flexible::new(
            1,
            ConstrainedBox::new(
                BoxConstraints::new(0, UNBOUNDED, 0, BODY_ROWS),
                SizedBox::new(None, Some(BODY_ROWS), Some(body)),
            ),
        ));
    let panel = BoxWidget::new()
        .style(surface)
        .padding(EdgeInsets::only(2, 1, 2, 1))
        .child(content);
    let column = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(Flexible::new(1, panel))
        // The footer row is opaque: what the composer's footer drew there is covered.
        .child(
            BoxWidget::new()
                .style(Theme::of(cx).background.clone())
                .padding(EdgeInsets::only(2, 0, 0, 0))
                .child(hint),
        );
    let focus = Focus::new(Dialog::bare(column)).can_request_focus(false);
    let focus = bind_fixed(focus, fixed::DISMISS, cx.send(Msg::Close));
    let focus = bind_fixed(
        focus.bind('k', cx.send(Msg::Turn(Turn::KeepAndNext))),
        fixed::TURN_PREVIOUS,
        cx.send(Msg::Turn(Turn::Previous)),
    );
    bind_fixed(focus, fixed::TURN_NEXT, cx.send(Msg::Turn(Turn::Next)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn warning(source: &str, details: &str) -> Warning {
        Warning {
            source: source.into(),
            details: details.into(),
        }
    }

    /// `warnings_view_tests.rs::entries()` plus the third page of
    /// `warnings_dismiss_only_drawn_pages_after_navigation_skips_a_frame`.
    fn entries() -> Vec<Warning> {
        vec![
            warning(
                "MCP · example",
                "MCP example could not connect\nSign in again using codex mcp login example",
            ),
            warning(
                "Startup",
                "Unknown setting `old_option`\nRemove it from config.toml",
            ),
            warning("Startup", "Not yet viewed"),
        ]
    }

    #[test]
    fn warning_pages_copy_only_the_current_diagnostic() {
        let mut pages = Pages::new(entries());
        let want = entries();
        for (turn, index) in [(Turn::Next, 1), (Turn::Previous, 0)] {
            assert_eq!(pages.turn(turn), Outcome::Stay);
            assert_eq!(pages.current, index);
            assert_eq!(
                pages.turn(Turn::Copy),
                Outcome::Copy(want[index].details.clone())
            );
        }
    }

    #[test]
    fn navigation_stops_at_the_first_and_the_last_page() {
        let mut pages = Pages::new(entries());
        pages.turn(Turn::Previous);
        assert_eq!(pages.current, 0);
        for _ in 0..5 {
            pages.turn(Turn::Next);
        }
        assert_eq!(pages.current, 2);
    }

    #[test]
    fn the_pages_that_were_shown_are_dismissed_on_close() {
        let mut pages = Pages::new(entries());
        pages.turn(Turn::Next);
        pages.turn(Turn::Next);
        pages.turn(Turn::Previous);
        let decision = pages.decision().expect("a decision");
        assert_eq!(decision.dismissed, entries());
        assert!(decision.saved.is_empty());
    }

    #[test]
    fn a_page_never_shown_is_not_dismissed() {
        let decision = Pages::new(entries()).decision().expect("a decision");
        assert_eq!(decision.dismissed, vec![entries()[0].clone()]);
    }

    #[test]
    fn keep_and_next_saves_the_page_without_dismissing_unvisited_ones() {
        let mut pages = Pages::new(entries());
        pages.turn(Turn::Next);
        assert_eq!(pages.turn(Turn::KeepAndNext), Outcome::Stay);
        assert_eq!(pages.current, 2);
        pages.turn(Turn::Previous);
        let want = entries();
        let decision = pages.decision().expect("a decision");
        // The oracle test keeps page 2 of 3 and dismisses page 1 only: here every page
        // reached was shown, so page 3 is dismissed too (delta N03).
        assert_eq!(decision.saved, vec![want[1].clone()]);
        assert_eq!(decision.dismissed, vec![want[0].clone(), want[2].clone()]);
    }

    #[test]
    fn keep_on_the_last_page_closes_the_viewer() {
        let mut pages = Pages::new(entries());
        pages.turn(Turn::Next);
        pages.turn(Turn::Next);
        assert_eq!(pages.turn(Turn::KeepAndNext), Outcome::Finish);
        assert_eq!(
            pages.decision().expect("a decision").saved,
            vec![entries()[2].clone()]
        );
    }

    #[test]
    fn no_decision_when_nothing_was_shown_or_saved() {
        assert_eq!(Pages::new(Vec::new()).decision(), None);
    }
}
