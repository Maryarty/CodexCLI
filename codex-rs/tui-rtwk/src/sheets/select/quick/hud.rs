//! The shortcut menu of `/presets` (oracle `chatwidget/quick_dials.rs` `quick_dial_hud`, the
//! `ShortcutHud` of `bottom_pane/shortcut_hud.rs`): a block over the composer that lists the
//! saved presets in columns and the keys that work on them (`n` new, `s` from settings, `e` edit,
//! `d` delete, a digit recalls its preset or starts one, Esc closes). Whether it is open is the
//! page's drawn state (ADR-013), what it lists is the model's `Quick`, and a key that leads on says
//! one `Intent::Quick`. In Flutter this is a `Material` banner built by a `LayoutBuilder` into a
//! grid of `Expanded` columns over a `Wrap` of key hints.

use rtwk::focus::KeyPress;
use rtwk::layout::{
    BoxWidget, Column, CrossAxisAlignment, EdgeInsets, Expanded, MainAxisSize, Row, SizedBox, Wrap,
};
use rtwk::provider::select;
use rtwk::style::Span;
use rtwk::text::TextWrap;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, LayoutBuilder};

use codex_tui_core::keymap::fixed;
use codex_tui_core::screens::ScreenStates;
use codex_tui_core::screens::select::Intent;
use codex_tui_core::screens::select::quick_dials::{
    Composer, Gate, Open, PickerMode, Quick, Settings, Target,
};

use super::strings;
use crate::app::theme::Tokens;
use crate::keymap::{fixed_presses, pressed_fixed, same_press};
use crate::widgets::dots::dots_end;

/// The narrowest a column of entries is (`MIN_COLUMN_WIDTH`).
const MIN_COLUMN_WIDTH: i32 = 32;
/// The gap between two columns (`COLUMN_GAP`).
const COLUMN_GAP: i32 = 2;
/// The gap between two keys of the last row (`COMMAND_SEPARATOR`).
const COMMAND_GAP: i32 = 3;
/// Up to this many entries stay in one column (`SINGLE_COLUMN_ENTRY_LIMIT`).
const SINGLE_COLUMN_ENTRIES: usize = 5;

/// What a press is for the menu while it is open (`handle_quick_dial_menu`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// `/presets` was run: the menu opens where presets can be used.
    Open,
    /// `n`: a new preset.
    New,
    /// `s`: a preset from the current settings.
    Settings,
    /// `e`: edit one.
    Edit,
    /// `d`: delete one.
    Delete,
    /// A digit: the slot's preset runs, or the slot gets one.
    Slot(u8),
    /// Esc: the menu closes.
    Close,
    /// Enter: nothing happens, the menu stays (the draft is not submitted behind it).
    Hold,
    /// Any other key closes the menu and goes on to the composer.
    Other,
}

impl Step {
    /// What `press` is for the open menu. Only a plain key leads on.
    pub fn of(press: &KeyPress) -> Step {
        if pressed_fixed(fixed::DISMISS, press) {
            return Step::Close;
        }
        if pressed_fixed(fixed::ENTER, press) {
            return Step::Hold;
        }
        let step = |id| pressed_fixed(id, press);
        if step(fixed::QUICK_NEW) {
            Step::New
        } else if step(fixed::QUICK_SETTINGS) {
            Step::Settings
        } else if step(fixed::QUICK_EDIT) {
            Step::Edit
        } else if step(fixed::QUICK_DELETE) {
            Step::Delete
        } else {
            // the digit keys of `QUICK_SLOT` are the slots `0` to `9` in order
            fixed_presses(fixed::QUICK_SLOT)
                .iter()
                .position(|digit| same_press(digit, press))
                .and_then(|slot| u8::try_from(slot).ok())
                .map_or(Step::Other, Step::Slot)
        }
    }

    /// Whether the menu took the key: it does not reach the composer.
    pub fn is_taken(self) -> bool {
        self != Step::Other
    }

    /// Whether the page the key opens starts from the composer's text.
    pub fn wants_composer(self) -> bool {
        matches!(self, Step::New | Step::Slot(_))
    }

    /// Whether the page the key opens starts from the current settings.
    pub fn wants_settings(self) -> bool {
        self == Step::Settings
    }
}

/// The verb a key says to the model: the page of the presets it opens. The composer's text and the
/// current settings are given when the key wants them (`wants_composer`, `wants_settings`). A digit
/// with a valid preset runs it (`recalled`) and says nothing here; one with a broken preset says
/// so (`Open::Recall`).
pub fn intent(
    key: Step,
    gate: Gate,
    quick: &Quick,
    settings: Option<Settings>,
    composer: Option<Composer>,
) -> Option<Intent> {
    let composer = composer.unwrap_or_default();
    let open = match key {
        Step::New => Open::Editor {
            target: Target::PickSlot,
            dial: None,
            composer,
        },
        Step::Settings => Open::Prefills(Box::new(settings?)),
        Step::Edit => Open::Picker(PickerMode::Edit),
        Step::Delete => Open::Picker(PickerMode::Clear),
        Step::Slot(slot) => match quick.get(slot) {
            None => Open::Editor {
                target: Target::Slot(slot),
                dial: None,
                composer,
            },
            Some(saved) if saved.dial.validate().is_err() => Open::Recall(slot),
            Some(_) => return None,
        },
        Step::Open | Step::Close | Step::Hold | Step::Other => return None,
    };
    Some(Intent::Quick { gate, open })
}

/// The input of the preset a digit recalls, when it is a valid one (`recall_quick_dial`).
pub fn recalled(key: Step, quick: &Quick) -> Option<String> {
    let Step::Slot(slot) = key else {
        return None;
    };
    quick
        .get(slot)
        .filter(|saved| saved.dial.validate().is_ok())
        .map(|saved| saved.dial.input.clone())
}

/// The menu as a block over the composer (`ShortcutHud::render`): the menu surface with two cells
/// at the sides and a row above and below, the title, a blank row, the entries in columns, a
/// blank row and the keys.
pub fn view<S: 'static>(cx: &mut Cx<'_, S>) -> AnyWidget {
    let words = strings::of(cx);
    let tokens = Tokens::of(cx);
    let quick = select::<ScreenStates, Quick, _>(cx, |states| states.select.quick.clone());
    let (accent, dim, bold) = (tokens.accent(), tokens.dim(), tokens.bold());
    let item = |key: &str, label: &str| -> AnyWidget {
        Row::new()
            .child(
                Text::rich(vec![
                    Span::styled(key.to_owned(), accent.clone()),
                    Span::styled(words.hud_separator, dim.clone()),
                ])
                // the separator ends in a space that is the label's gap: a word-wrapped text
                // lets it hang
                .wrap(TextWrap::None),
            )
            .child(Expanded::new(
                1,
                dots_end(label.to_owned(), Default::default()),
            ))
            .into()
    };
    let entries: Vec<AnyWidget> = quick
        .saved()
        .iter()
        .map(|saved| item(&saved.slot.to_string(), &saved.dial.label_line()))
        .collect();
    let mut keys = Wrap::new().spacing(COMMAND_GAP).run_spacing(0);
    for (key, label) in words.hud_commands {
        keys = keys.child(
            Row::new()
                .main_axis_size(MainAxisSize::Min)
                .child(Text::rich(vec![
                    Span::styled(key, accent.clone()),
                    Span::styled(words.hud_separator, dim.clone()),
                    Span::new(label),
                ])),
        );
    }
    let empty = Text::line(words.hud_empty).style(dim.clone());
    let content = LayoutBuilder::new(move |_, constraints| {
        let columns = column_count(constraints.max_width, entries.len());
        let rows = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(Text::line(words.hud_title).style(bold.clone()))
            .child(Text::new(String::new()));
        let rows = if entries.is_empty() {
            rows.child(empty.clone())
        } else {
            rows.child(grid(&entries, columns))
        };
        rows.child(Text::new(String::new()))
            .child(keys.clone())
            .into()
    });
    BoxWidget::new()
        .style(tokens.prompt_surface(cx, false))
        .padding(EdgeInsets::only(2, 1, 2, 1))
        .child(content)
        .into()
}

/// How many columns `width` holds for `count` entries (`column_count`: one up to
/// `SINGLE_COLUMN_ENTRIES`, two above).
fn column_count(width: i32, count: usize) -> usize {
    let limit = if count > SINGLE_COLUMN_ENTRIES { 2 } else { 1 };
    let fits = (width + COLUMN_GAP) / (MIN_COLUMN_WIDTH + COLUMN_GAP);
    usize::try_from(fits).unwrap_or(1).clamp(1, limit)
}

/// The entries down `columns` columns of equal width (`grid_lines`).
fn grid(entries: &[AnyWidget], columns: usize) -> AnyWidget {
    let rows = entries.len().div_ceil(columns).max(1);
    let mut row = Row::new().cross_axis_alignment(CrossAxisAlignment::Start);
    for (index, chunk) in entries.chunks(rows).enumerate() {
        if index > 0 {
            row = row.child(SizedBox::new(Some(COLUMN_GAP), None, None));
        }
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch);
        for entry in chunk {
            column = column.child(entry.clone());
        }
        row = row.child(Expanded::new(1, column));
    }
    row.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keymap::typed;

    #[test]
    fn a_narrow_room_or_few_entries_keep_one_column() {
        assert_eq!(column_count(96, 6), 2);
        assert_eq!(column_count(96, 5), 1);
        assert_eq!(column_count(44, 6), 1);
        assert_eq!(column_count(96, 0), 1);
    }

    #[test]
    fn a_plain_letter_or_digit_leads_on_and_the_rest_goes_to_the_composer() {
        assert_eq!(Step::of(&typed('n')), Step::New);
        assert_eq!(Step::of(&typed('7')), Step::Slot(7));
        assert_eq!(Step::of(&typed('x')), Step::Other);
        assert!(!Step::Other.is_taken());
        assert!(Step::Hold.is_taken());
    }
}
