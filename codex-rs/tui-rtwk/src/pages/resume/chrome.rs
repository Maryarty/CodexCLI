//! The picker's chrome by the width it gets: the first of several spellings
//! that fits, as Flutter lays out with a `LayoutBuilder` and variants: the
//! toolbar (full or compact), the footer's progress label (three spellings)
//! and each footer hint row (`hint_variants`: wide labels from 120 cells,
//! compact, then the hints of the lowest priority numbers; the row picks the
//! first that fits, `ui::page_keys::footer_fit`). Oracle: `resume_picker.rs`
//! (`toolbar_for_width`, `hint_line_for_row`, `picker_footer_progress_label`).

use rtwk::layout::{EdgeInsets, Padding, SizedBox};
use rtwk::style::Style;
use rtwk::text::{Text, segmenter_of};
use rtwk::{AnyWidget, LayoutBuilder};

use crate::widgets::measure::Measure;

use crate::widgets::page_keys::Variant;

use super::strings::Strings;

/// The three spellings of the footer's progress, widest first, the first
/// that is narrower than `w` (oracle: `picker_footer_progress_label`).
pub fn progress_label(seg: Measure, position: usize, total: &str, percent: u8, w: usize) -> String {
    [
        format!(" {position} / {total} · {percent}% "),
        format!(" {position}/{total} · {percent}% "),
        format!(" {percent}% "),
    ]
    .into_iter()
    .find(|label| seg.width(label) < w)
    .unwrap_or_default()
}

/// One footer hint (oracle: `PickerFooterHint`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hint {
    pub key: String,
    pub wide: String,
    pub compact: String,
    pub priority: u8,
}

const COMPACT_BREAKPOINT: i32 = 120;

/// The spellings of a hint row, in the order the oracle tries them
/// (`hint_line_for_row`): wide labels from 120 cells, compact, then the
/// hints of the lowest priority numbers (in their own order), compact and
/// then as keys alone.
pub fn hint_variants(hints: &[Hint]) -> Vec<Variant> {
    let entries = |picked: &[&Hint], label: fn(&Hint) -> String| -> Vec<(String, String)> {
        picked
            .iter()
            .map(|hint| (hint.key.clone(), label(hint)))
            .collect()
    };
    let all: Vec<&Hint> = hints.iter().collect();
    let mut out = vec![
        Variant {
            min_width: COMPACT_BREAKPOINT,
            entries: entries(&all, |hint| hint.wide.clone()),
        },
        Variant {
            min_width: 0,
            entries: entries(&all, |hint| hint.compact.clone()),
        },
    ];
    let mut by_priority: Vec<usize> = (0..hints.len()).collect();
    by_priority.sort_by_key(|i| hints[*i].priority);
    let label_of: [fn(&Hint) -> String; 2] = [|hint| hint.compact.clone(), |_| String::new()];
    for label in label_of {
        for keep in (1..=by_priority.len()).rev() {
            let mut picked = by_priority[..keep].to_vec();
            picked.sort_unstable();
            let picked: Vec<&Hint> = picked.iter().map(|i| &hints[*i]).collect();
            out.push(Variant {
                min_width: 0,
                entries: entries(&picked, label),
            });
        }
    }
    out
}

/// A footer row whose first word is not a key (the loading row's `loading`):
/// the first of `variants` that fits, its entries as plain text apart by
/// `separator`, `pad` cells in (`footer_fit` reads every entry's key as a key
/// the page binds; this row binds none).
pub fn plain_fit(
    variants: Vec<Variant>,
    style: Style,
    separator: &'static str,
    pad: i32,
) -> AnyWidget {
    LayoutBuilder::new(move |cx, constraints| {
        let seg = segmenter_of(cx);
        let room = constraints.max_width;
        let text = |variant: &Variant| {
            variant
                .entries
                .iter()
                .map(|(key, hint)| {
                    if hint.is_empty() {
                        key.clone()
                    } else {
                        [key.as_str(), hint.as_str()].join(" ")
                    }
                })
                .collect::<Vec<_>>()
                .join(separator)
        };
        let fits = |variant: &Variant| {
            room >= variant.min_width
                && i32::try_from(seg.width(&text(variant))).unwrap_or(i32::MAX) + pad <= room
        };
        match variants.iter().find(|variant| fits(variant)) {
            Some(variant) => Padding::new(
                EdgeInsets::only(pad, 0, 0, 0),
                Text::line(text(variant)).style(style.clone()),
            )
            .into(),
            None => SizedBox::new(None, Some(1), None).into(),
        }
    })
    .into()
}

/// What the hint rows depend on (oracle: `footer_hint_lines`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HintFacts {
    /// A transcript is loading: the rows say so and take nothing else.
    pub loading: bool,
    pub fork: bool,
    pub archived: bool,
    /// The search has text: esc clears it.
    pub searching: bool,
    /// Opened from a running session: esc and ctrl+c leave the picker.
    pub in_session: bool,
    pub dense: bool,
    /// The archive key is offered: rows to act on and active sessions.
    pub archive: bool,
}

/// The footer's two hint rows' spellings (oracle: `footer_hint_lines`).
pub fn hint_rows(strings: &Strings, facts: HintFacts) -> [Vec<Variant>; 2] {
    let hint = |key: &str, wide: &str, compact: &str, priority| Hint {
        key: key.to_string(),
        wide: wide.to_string(),
        compact: compact.to_string(),
        priority,
    };
    if facts.loading {
        let first = [
            hint(
                strings.key_loading,
                strings.loading_transcript,
                strings.loading_transcript,
                0,
            ),
            hint(strings.key_quit, strings.quit, strings.quit, 1),
        ];
        return [hint_variants(&first), Vec::new()];
    }
    let accept = if facts.archived {
        strings.accept_restore
    } else if facts.fork {
        strings.accept_fork
    } else {
        strings.accept
    };
    let (cancel_wide, cancel_compact) = if facts.searching {
        (strings.cancel_clear, strings.cancel_clear_compact)
    } else if facts.in_session {
        (strings.exit, strings.exit)
    } else {
        (strings.cancel_wide, strings.cancel_compact)
    };
    let quit = if facts.in_session {
        strings.exit
    } else {
        strings.quit
    };
    let (density_wide, density_compact) = if facts.dense {
        (strings.density_comfy_wide, strings.density_comfy_compact)
    } else {
        (strings.density_wide, strings.density_compact)
    };
    let mut first = vec![hint(strings.key_accept, accept, accept, 0)];
    if facts.archive {
        first.push(hint(
            strings.key_archive,
            strings.archive,
            strings.archive,
            2,
        ));
    }
    first.extend([
        hint(strings.key_cancel, cancel_wide, cancel_compact, 1),
        hint(strings.key_quit, quit, quit, 2),
        hint(
            strings.key_focus,
            strings.focus_wide,
            strings.focus_compact,
            7,
        ),
        hint(
            strings.key_option,
            strings.option_wide,
            strings.option_compact,
            8,
        ),
    ]);
    let second = [
        hint(strings.key_density, density_wide, density_compact, 3),
        hint(
            strings.key_transcript,
            strings.transcript_wide,
            strings.transcript_compact,
            4,
        ),
        hint(
            strings.key_expand,
            strings.expand_wide,
            strings.expand_compact,
            6,
        ),
        hint(strings.key_browse, strings.browse, strings.browse, 5),
    ];
    [hint_variants(&first), hint_variants(&second)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use rtwk_core::unicode::TextSegmenter;

    /// A halfwidth katakana and its sound mark: one cell clustered, two in legacy.
    const MARKED: &str = "\u{ff76}\u{ff9e}";

    #[test]
    fn the_progress_label_counts_the_terminals_cells() {
        // ` 1 / ｶﾞ · 5% ` takes 12 cells clustered, 13 in legacy; it must be narrower than 13
        let graphemes = progress_label(TextSegmenter::default(), 1, MARKED, 5, 13);
        let legacy = progress_label(TextSegmenter::legacy(), 1, MARKED, 5, 13);
        assert_eq!(graphemes, format!(" 1 / {MARKED} \u{b7} 5% "));
        assert_eq!(legacy, format!(" 1/{MARKED} \u{b7} 5% "));
    }
}
