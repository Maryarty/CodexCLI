//! G39 (`docs/rtwk-port/rtwk-gaps.md`): the picker's chrome picks, by the width it
//! gets, the first of several spellings that fits: the toolbar (full or
//! compact), each footer hint row (wide labels, compact, or the hints of the
//! lowest priority numbers) and the footer's progress label. No rtwk widget
//! takes candidates and shows the first that fits, so the candidates are
//! measured here. Oracle: `resume_picker.rs` (`toolbar_for_width`,
//! `hint_line_for_row`, `picker_footer_progress_label`).

use super::strings::Strings;

fn width(text: &str) -> usize {
    text.chars().count()
}

/// One footer hint (oracle: `PickerFooterHint`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hint {
    pub key: String,
    pub wide: String,
    pub compact: String,
    pub priority: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Wide,
    Compact,
    KeyOnly,
}

const LEFT_PAD: usize = 1;
const GAP_WIDTH: usize = 3;
const COMPACT_BREAKPOINT: usize = 120;

fn fit(hints: &[&Hint], mode: Mode, w: usize) -> Option<String> {
    let total = LEFT_PAD
        + hints
            .iter()
            .enumerate()
            .map(|(i, hint)| {
                let label = match mode {
                    Mode::Wide => 1 + width(&hint.wide),
                    Mode::Compact => 1 + width(&hint.compact),
                    Mode::KeyOnly => 0,
                };
                width(&hint.key) + label + if i == 0 { 0 } else { GAP_WIDTH }
            })
            .sum::<usize>();
    if total > w {
        return None;
    }
    let mut text = " ".repeat(LEFT_PAD);
    for (i, hint) in hints.iter().enumerate() {
        if i > 0 {
            text.push_str(&" ".repeat(GAP_WIDTH));
        }
        text.push_str(&hint.key);
        match mode {
            Mode::Wide => {
                text.push(' ');
                text.push_str(&hint.wide);
            }
            Mode::Compact => {
                text.push(' ');
                text.push_str(&hint.compact);
            }
            Mode::KeyOnly => {}
        }
    }
    Some(text)
}

/// A row of hints fitted to `w` (oracle: `hint_line_for_row`): wide labels
/// from 120 cells, else compact, else the hints of the lowest priority
/// numbers that fit.
pub fn hint_row(hints: &[Hint], w: usize) -> String {
    let all: Vec<&Hint> = hints.iter().collect();
    if w >= COMPACT_BREAKPOINT
        && let Some(text) = fit(&all, Mode::Wide, w)
    {
        return text;
    }
    if let Some(text) = fit(&all, Mode::Compact, w) {
        return text;
    }
    let mut retained: Vec<usize> = (0..hints.len()).collect();
    retained.sort_by_key(|i| hints[*i].priority);
    for mode in [Mode::Compact, Mode::KeyOnly] {
        for keep in (1..=retained.len()).rev() {
            let mut picked = retained[..keep].to_vec();
            picked.sort_unstable();
            let candidate: Vec<&Hint> = picked.iter().map(|i| &hints[*i]).collect();
            if let Some(text) = fit(&candidate, mode, w) {
                return text;
            }
        }
    }
    String::new()
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

/// The footer's two hint rows (oracle: `footer_hint_lines`).
pub fn hint_rows(strings: &Strings, w: usize) -> [String; 2] {
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
    [hint_row(&first, w), hint_row(&second, w)]
}
