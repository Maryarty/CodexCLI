//! The picker's chrome by the width it gets: the first of several spellings
//! that fits, as Flutter lays out with a `LayoutBuilder` and variants: the
//! toolbar (full or compact), the footer's progress label (three spellings)
//! and each footer hint row (`hint_variants`: wide labels from 120 cells,
//! compact, then the hints of the lowest priority numbers; the row picks the
//! first that fits, `ui::page_keys::footer_fit`). Oracle: `resume_picker.rs`
//! (`toolbar_for_width`, `hint_line_for_row`, `picker_footer_progress_label`).

use rtwk_core::unicode::{TextMode, string_width};

use crate::widgets::page_keys::Variant;

use super::strings::Strings;

fn width(text: &str) -> usize {
    string_width(text, TextMode::Graphemes, 1)
}

/// The three spellings of the footer's progress, widest first, the first
/// that is narrower than `w` (oracle: `picker_footer_progress_label`).
pub fn progress_label(position: usize, total: &str, percent: u8, w: usize) -> String {
    [
        format!(" {position} / {total} · {percent}% "),
        format!(" {position}/{total} · {percent}% "),
        format!(" {percent}% "),
    ]
    .into_iter()
    .find(|label| width(label) < w)
    .unwrap_or_default()
}


/// One run of the toolbar: its text and whether it is the active value.
pub type Run = (String, bool);

/// The toolbar (oracle: `toolbar_for_width`): the full strip, else the
/// compact one.
pub fn toolbar(strings: &super::strings::Strings, w: usize) -> Vec<Run> {
    let full = |compact: bool| -> Vec<Run> {
        let value = |text: &str, active: bool| (format!(" {text} "), active);
        let label = |text: &str, gap: &str| (format!("{text}{gap}"), false);
        let sep = || ("   ".to_string(), false);
        let gap = if compact { "" } else { " " };
        let mut line = vec![label(strings.filter, gap), value(strings.filter_all, true), sep()];
        line.push(label(strings.status, gap));
        line.push(value(strings.status_active, true));
        if !compact {
            line.push(value(strings.status_archived, false));
        }
        line.push(sep());
        line.push(label(strings.sort, gap));
        line.push(value(strings.sort_updated, true));
        if !compact {
            line.push(value(strings.sort_created, false));
        }
        line
    };
    let wide = full(false);
    if wide.iter().map(|(text, _)| width(text)).sum::<usize>() <= w {
        return wide;
    }
    full(true)
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
        picked.iter().map(|hint| (hint.key.clone(), label(hint))).collect()
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
            out.push(Variant { min_width: 0, entries: entries(&picked, label) });
        }
    }
    out
}

/// The footer's two hint rows' spellings (oracle: `footer_hint_lines`).
pub fn hint_rows(strings: &Strings) -> [Vec<Variant>; 2] {
    let hint = |key: &str, wide: &str, compact: &str, priority| Hint {
        key: key.to_string(),
        wide: wide.to_string(),
        compact: compact.to_string(),
        priority,
    };
    let first = [
        hint(strings.key_accept, strings.accept, strings.accept, 0),
        hint(strings.key_cancel, strings.cancel_wide, strings.cancel_compact, 1),
        hint(strings.key_quit, strings.quit, strings.quit, 2),
        hint(strings.key_focus, strings.focus_wide, strings.focus_compact, 7),
        hint(strings.key_option, strings.option_wide, strings.option_compact, 8),
    ];
    let second = [
        hint(strings.key_density, strings.density_wide, strings.density_compact, 3),
        hint(strings.key_transcript, strings.transcript_wide, strings.transcript_compact, 4),
        hint(strings.key_expand, strings.expand_wide, strings.expand_compact, 6),
        hint(strings.key_browse, strings.browse, strings.browse, 5),
    ];
    [hint_variants(&first), hint_variants(&second)]
}
