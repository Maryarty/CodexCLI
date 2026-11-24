//! The help (`?`, oracle: `chrome.rs` `help_lines`): the keys of the page,
//! each line wrapped to the width, in the report's place. The labels are the
//! keymap's; the lines of a control show only while that control is there.

use rtwk::AnyWidget;
use rtwk::l10n::NumSyms;
use rtwk::layout::{Column, CrossAxisAlignment};
use rtwk::style::Style;
use rtwk::text::{Text, TextWrap};

use codex_tui_core::keymap::fixed::{INTERRUPT_OR_QUIT, USAGE_PREVIOUS_TAB};
use codex_tui_core::keymap::{ActionId, Keymap};
use codex_tui_core::screens::usage::{Control, View};

use super::keys;
use super::strings::Strings;

/// The label of a hardwired action's key.
fn stroke_label_of(id: ActionId) -> String {
    crate::keymap::stroke_for(id)
        .map(|stroke| crate::keymap::stroke_label(&stroke))
        .unwrap_or_default()
}

/// The help's lines before wrapping.
pub(super) fn lines(
    model: &View<'_>,
    strings: &Strings,
    num: &NumSyms,
    keymap: &Keymap,
    detail_open: bool,
) -> Vec<String> {
    let hint = |id| keys::hint(keymap, id);
    let shift_tab = stroke_label_of(USAGE_PREVIOUS_TAB);
    let interrupt = stroke_label_of(INTERRUPT_OR_QUIT);
    let count = model.visible_sections().len() as u64;
    let mut out = vec![
        strings.help_tabs(&shift_tab, num),
        strings.help_digits(count, num),
        strings.help_days(&hint(keys::LEFT), &hint(keys::RIGHT), num),
        strings.help_rows(&hint(keys::UP), &hint(keys::DOWN), num),
        strings.help_details(&hint(keys::ACCEPT), num),
        strings.help_scroll(&hint(keys::PAGE_UP), &hint(keys::PAGE_DOWN), num),
        strings.help_mouse.to_owned(),
    ];
    for (control, line) in [
        (Control::Range, strings.help_range),
        (Control::Group, strings.help_group),
        (Control::Model, strings.help_model),
        (Control::TaskMetric, strings.help_metric),
        (Control::ZeroCreditGroups, strings.help_zero),
    ] {
        if model.control_available(control, detail_open) {
            out.push(line.to_owned());
        }
    }
    out.push(strings.help_refresh.to_owned());
    out.push(strings.help_zoom(&hint(keys::ACCEPT), num));
    if keys::help_available(keymap) {
        out.push(strings.help_toggle.to_owned());
    }
    out.push(strings.help_close(&hint(keys::CANCEL), &interrupt, num));
    if let Some(updated) = model
        .history(model.section())
        .ready()
        .and_then(|history| history.updated_at)
    {
        out.push(strings.help_updated(updated, model.hour24, num));
    }
    out
}

/// The help in the report's place: one wrapped line per entry.
pub(super) fn body(lines: Vec<String>) -> AnyWidget {
    Column::new()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .children(lines.into_iter().map(|line| -> AnyWidget {
            Text::line(line)
                .style(Style::default())
                .wrap(TextWrap::Word)
                .into()
        }))
        .into()
}
