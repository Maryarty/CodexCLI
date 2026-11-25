//! The two fixed rows under the report (oracle: `hints.rs`): what the surface
//! in front does, then the keys that go to other reports. The keys are the
//! keymap's labels; a part that does not fit the width is left out and the
//! next one is tried, so a narrow terminal keeps the short ones. Which parts
//! show is the page's choice from the measured width; the bar draws them.

use std::rc::Rc;

use rtwk::foundation::ListenableBuilder;
use rtwk::l10n::NumSyms;
use rtwk::layout::{Column, CrossAxisAlignment, MainAxisSize, SizedBox};
use rtwk::scroll::ScrollController;
use rtwk::style::Style;
use rtwk::text::segmenter_of;
use rtwk::{AnyWidget, LayoutBuilder};

use codex_tui_core::keymap::Keymap;
use codex_tui_core::screens::usage::{Chosen, Section, View};

use super::keys;
use super::report;
use super::strings::Strings;
use crate::widgets::measure::Measure;

/// The separator between parts.
const SEPARATOR: &str = " \u{b7} ";

/// What the rows depend on besides the width.
pub(super) struct Facts<'a> {
    pub model: &'a View<'a>,
    pub strings: &'a Strings,
    pub num: &'a NumSyms,
    pub keymap: &'a Keymap,
    pub shown: report::Shown,
    pub help: bool,
    /// Whether the report scrolls (it is taller than its room).
    pub scrollable: bool,
    /// There is a row under the cursor to open (a period of the Plan).
    pub row_details: bool,
}

/// A hint: the keys as shown and what they do (empty: the keys alone).
type Part = (String, String);

/// The parts that fit `width`, in order, measured by the tree's own segmenter
/// (oracle: the greedy loop of `footer_lines`; a part without keys is skipped).
fn fit(seg: Measure, parts: Vec<Part>, width: i32) -> Vec<Part> {
    let cells = |text: &str| seg.width(text) as i32;
    let mut taken: Vec<Part> = Vec::new();
    let mut used = 0;
    for (key, word) in parts {
        if key.is_empty() {
            continue;
        }
        let size = (if taken.is_empty() {
            0
        } else {
            cells(SEPARATOR)
        }) + cells(&key)
            + if word.is_empty() { 0 } else { 1 + cells(&word) };
        if used + size <= width {
            used += size;
            taken.push((key, word));
        }
    }
    taken
}

/// The two rows' parts for a row `width` cells wide.
pub(super) fn rows(seg: Measure, facts: &Facts<'_>, width: i32) -> [Vec<Part>; 2] {
    let Facts {
        model,
        strings,
        num,
        keymap,
        shown,
        help,
        scrollable,
        row_details,
    } = facts;
    let hint = |id| keys::hint(keymap, id);
    let pair = |a, b| format!("{}/{}", hint(a), hint(b));
    let word = |text: &str| text.to_owned();
    let mut actions: Vec<Part> = Vec::new();
    if *help {
        actions.push((hint(keys::CANCEL), word(strings.back)));
    } else if !shown.zoomed {
        actions.push((hint(keys::ACCEPT), word(strings.hint_focus)));
        actions.push((word(strings.hint_tab_key), word(strings.hint_card)));
    } else {
        let section = model.section();
        match section {
            Section::Summary => {}
            Section::Plan => actions.push((pair(keys::UP, keys::DOWN), word(strings.hint_period))),
            Section::Chats => actions.push((pair(keys::UP, keys::DOWN), word(strings.hint_row))),
            _ => actions.push((pair(keys::LEFT, keys::RIGHT), word(strings.hint_day))),
        }
        let details = match section {
            Section::Plan | Section::Chats => *row_details,
            section => report::is_chart(section) && model.day_has_details(section, shown.cursor),
        };
        if details {
            actions.push((hint(keys::ACCEPT), word(strings.hint_details)));
        }
    }
    if *scrollable {
        actions.push((
            pair(keys::PAGE_UP, keys::PAGE_DOWN),
            word(strings.hint_scroll),
        ));
    }
    if !help {
        actions.push((hint(keys::CANCEL), word(strings.back)));
    }
    let mut navigation: Vec<Part> = if *help {
        vec![("?".into(), word(strings.hint_close_help))]
    } else if model.visible_sections().is_empty() {
        vec![("R".into(), word(strings.hint_retry))]
    } else {
        let count = model.visible_sections().len() as u64;
        vec![
            (
                if width >= 80 {
                    strings.hint_report(count, num)
                } else {
                    word(strings.hint_tab_key)
                },
                word(strings.hint_report_word),
            ),
            (
                "z".into(),
                word(if !shown.zoomed {
                    strings.hint_focus
                } else if width < 50 {
                    strings.hint_all
                } else {
                    strings.hint_dashboard
                }),
            ),
            (
                "?".into(),
                if width < 50 {
                    String::new()
                } else {
                    word(strings.hint_help)
                },
            ),
            ("R".into(), word(strings.hint_refresh)),
        ]
    };
    if !keys::help_available(keymap) {
        navigation.retain(|(key, _)| key != "?");
    }
    [fit(seg, actions, width), fit(seg, navigation, width)]
}

/// The footer's inputs, owned: the widget is built again when the scroll
/// controller notifies.
pub(super) struct Footer {
    pub model: Chosen,
    pub strings: Strings,
    pub num: NumSyms,
    pub keymap: Rc<Keymap>,
    pub shown: report::Shown,
    pub help: bool,
    pub row_details: bool,
    pub controller: ScrollController,
    pub key: Style,
    pub hint: Style,
}

/// The footer: two rows, laid out from the width they get and, for the
/// scroll hint, from whether the report in front scrolls.
pub(super) fn footer(footer: Footer) -> AnyWidget {
    let Footer {
        model,
        strings,
        num,
        keymap,
        shown,
        help,
        row_details,
        controller,
        key,
        hint,
    } = footer;
    let watched = Rc::new(controller);
    let listened = watched.clone();
    ListenableBuilder::new(listened, move |_, _| {
        let scrollable = watched
            .try_position()
            .is_ok_and(|position| position.max_scroll_extent() > 0);
        let (model, keymap) = (model.clone(), keymap.clone());
        let (key, hint) = (key.clone(), hint.clone());
        LayoutBuilder::new(move |cx, constraints| {
            let view = model.view();
            let facts = Facts {
                model: &view,
                strings: &strings,
                num: &num,
                keymap: &keymap,
                shown,
                help,
                scrollable,
                row_details,
            };
            let [first, second] = rows(segmenter_of(cx), &facts, constraints.max_width);
            let row = |parts: Vec<Part>| -> AnyWidget {
                SizedBox::new(
                    None,
                    Some(1),
                    Some(
                        crate::widgets::page_keys::footer_with(
                            parts,
                            key.clone(),
                            hint.clone(),
                            SEPARATOR,
                        )
                        .into(),
                    ),
                )
                .into()
            };
            Column::new()
                .overflow_intended(true)
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .child(row(first))
                .child(row(second))
                .into()
        })
        .into()
    })
    .into()
}

#[cfg(test)]
mod tests {
    use rtwk_core::unicode::{TextMode, TextSegmenter};

    use super::fit;

    fn parts() -> Vec<(String, String)> {
        vec![
            ("a".into(), "\u{ff76}\u{ff9e}".into()),
            ("b".into(), String::new()),
        ]
    }

    /// `a ｶﾞ` is 3 cells where the terminal clusters the sound mark with its
    /// letter (`Graphemes`), 4 where it counts code points (`Legacy`): in a
    /// room of 3 the first part fits only in the first, and `b` is taken
    /// alone in the second.
    #[test]
    fn the_parts_that_fit_follow_the_terminals_text_mode() {
        let keys = |mode| {
            fit(TextSegmenter::new(mode, 1), parts(), 3)
                .into_iter()
                .map(|(key, _)| key)
                .collect::<Vec<_>>()
        };
        assert_eq!(keys(TextMode::Graphemes), ["a"]);
        assert_eq!(keys(TextMode::Legacy), ["b"]);
    }
}
