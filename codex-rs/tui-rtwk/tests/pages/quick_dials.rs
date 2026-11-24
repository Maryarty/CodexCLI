// test root: pages
//! `/presets` as a sheet of its own (oracle `chatwidget/quick_dials.rs`, `bottom_pane/custom_prompt_view.rs`,
//! snapshots `saved_quick_dials_*`): the slot pickers, the sources among the current settings, the
//! editor and the confirmation are the pages of one nested navigator; a pick pushes the next page
//! over the one it came from, Esc shows the page under it again (the editor keeps its text, a
//! picker its cursor), and a change settled on closes the whole sheet. Which row leads where is
//! core's (`core/tests/quick_dials.rs`); here is what the pages show and what the keys do on them.
//! The sheet has no door yet (the composer's shortcut menu is the HUD's), so the pages are opened
//! from the debug entries of the `select` directory, on the one preset (`coding`, slot 1) their
//! script seeds.

use crate::kit::{Owner, Screen};

const CODING: &str = "/model gpt-5.5 --reasoning=high --speed=fast";

fn entry(name: &'static str, height: u16) -> Screen {
    Screen::build(Owner::DialogsPane)
        .size(80, height)
        .entry("select", name)
        .open()
}

#[test]
fn the_save_picker_names_the_text_lists_ten_slots_and_opens_on_the_first_empty_one() {
    let s = entry("quick_save", 24);
    s.assert_in_order(&[
        "Save preset to slot",
        "Review the current changes",
        &format!(" 1. coding  {CODING}"),
        "› 2. empty",
        " 3. empty",
        " 4. empty",
        " 5. empty",
        " 6. empty",
        " 7. empty",
        " 8. empty",
        " 9. empty",
        " 0. empty",
        "enter select · esc back",
    ]);
}

#[test]
fn a_digit_on_an_empty_slot_saves_there_and_the_sheet_is_gone() {
    let mut s = entry("quick_save", 24);
    s.keys("5");
    s.assert_absent("Save preset to slot");
    s.assert_absent("Replace preset");
}

#[test]
fn the_zero_key_picks_the_tenth_slot() {
    let mut s = entry("quick_save", 24);
    s.keys("0");
    s.assert_absent("Save preset to slot");
}

#[test]
fn enter_on_the_cursor_saves_into_the_first_empty_slot() {
    let mut s = entry("quick_save", 24);
    s.keys("enter");
    s.assert_absent("Save preset to slot");
}

#[test]
fn a_digit_on_a_taken_slot_asks_to_replace_it_and_esc_shows_the_picker_again() {
    let mut s = entry("quick_save", 24);
    s.keys("1");
    s.assert_in_order(&[
        "Replace preset 1?",
        CODING,
        "→ Review the current changes",
        "› 1. Replace preset 1",
        "enter select · esc back",
    ]);
    s.keys("esc");
    s.assert_absent("Replace preset 1?");
    s.assert_in_order(&["Save preset to slot", "Review the current changes"]);
    // the sheet is still open, the cursor stays on the slot that was asked for
    s.assert_contains("› 1. coding");
    s.keys("esc");
    s.assert_absent("Save preset to slot");
}

#[test]
fn confirming_a_replacement_closes_the_whole_sheet() {
    let mut s = entry("quick_save", 24);
    s.keys("1 enter");
    s.assert_absent("Replace preset 1?");
    s.assert_absent("Save preset to slot");
}

#[test]
fn the_edit_picker_lists_only_the_saved_slots_and_enter_opens_the_editor_on_the_text() {
    let mut s = entry("quick_edit", 24);
    s.assert_in_order(&[
        "Edit a preset",
        &format!("› 1. coding  {CODING}"),
        "enter select · esc back",
    ]);
    s.assert_absent("2. empty");
    s.keys("enter");
    s.assert_in_order(&[
        "Edit preset 1",
        &format!("› {CODING}"),
        "enter review changes · esc back",
    ]);
}

#[test]
fn esc_goes_back_page_by_page_from_the_confirmation_through_the_editor_to_the_picker() {
    let mut s = entry("quick_edit", 24);
    s.keys("enter");
    s.keys("enter");
    // the text is unchanged, and still replaces what the slot holds
    s.assert_in_order(&["Replace preset 1?", "› 1. Replace preset 1"]);
    s.keys("esc");
    s.assert_absent("Replace preset 1?");
    s.assert_contains("Edit preset 1");
    s.keys("esc");
    s.assert_absent("Edit preset 1");
    s.assert_contains("Edit a preset");
    s.assert_contains("› 1. coding");
    s.keys("esc");
    s.assert_absent("Edit a preset");
}

#[test]
fn the_editor_keeps_what_was_typed_when_esc_comes_back_from_the_next_page() {
    let mut s = entry("quick_edit", 24);
    s.keys("enter");
    s.type_text(" --extra");
    s.keys("enter");
    s.assert_contains("Replace preset 1?");
    s.assert_contains("--extra");
    s.keys("esc");
    s.assert_contains("Edit preset 1");
    s.assert_contains(&format!("{CODING} --extra"));
}

#[test]
fn the_delete_picker_lists_the_saved_slots_and_enter_asks_to_delete() {
    let mut s = entry("quick_clear", 24);
    s.assert_in_order(&[
        "Delete a preset",
        &format!("› 1. coding  {CODING}"),
        "enter select · esc back",
    ]);
    s.keys("enter");
    s.assert_in_order(&[
        "Delete preset 1?",
        "coding",
        "› 1. Delete preset 1",
        "enter select · esc back",
    ]);
    s.keys("esc");
    s.assert_absent("Delete preset 1?");
    s.assert_contains("Delete a preset");
    s.keys("1");
    s.assert_contains("Delete preset 1?");
    s.keys("enter");
    s.assert_absent("Delete preset 1?");
    s.assert_absent("Delete a preset");
}

#[test]
fn the_delete_picker_with_nothing_saved_has_no_row_to_pick() {
    let mut s = entry("quick_clear_empty", 24);
    s.assert_contains("Delete a preset");
    // no row is selected: the oracle's `accept` with nothing under the cursor cancels the view
    s.keys("enter");
    s.assert_absent("Delete a preset");
    s.assert_absent("Delete preset");
}

#[test]
fn a_new_preset_opens_the_editor_on_the_text_and_says_where_enter_goes() {
    let s = entry("quick_editor", 24);
    s.assert_in_order(&[
        "New preset",
        "› Review the current changes",
        "for safety",
        "enter choose slot · esc back",
    ]);
}

#[test]
fn a_blank_text_submits_nothing_and_shows_the_placeholder() {
    let mut s = entry("quick_editor", 24);
    s.keys(&"backspace ".repeat(40));
    s.assert_contains("Write a prompt or slash command");
    s.keys("enter");
    s.assert_contains("New preset");
    s.assert_absent("Save preset to slot");
    // whitespace is not a text either
    s.type_text("   ");
    s.keys("enter");
    s.assert_contains("New preset");
    s.assert_absent("Save preset to slot");
}

#[test]
fn a_text_that_is_not_a_preset_leaves_the_editor_where_it_is() {
    let mut s = entry("quick_editor", 24);
    s.keys(&"backspace ".repeat(40));
    s.type_text("!ls");
    s.keys("enter");
    s.assert_contains("New preset");
    s.assert_contains("› !ls");
    s.assert_absent("Save preset to slot");
}

#[test]
fn a_text_goes_to_the_slot_picker_and_esc_brings_the_editor_back_with_its_text() {
    let mut s = entry("quick_editor", 24);
    s.type_text(" now");
    s.keys("enter");
    s.assert_in_order(&["Save preset to slot", "› 2. empty"]);
    s.keys("esc");
    s.assert_absent("Save preset to slot");
    s.assert_in_order(&[
        "New preset",
        "for safety now",
        "enter choose slot · esc back",
    ]);
    s.keys("esc");
    s.assert_absent("New preset");
}

#[test]
fn the_editor_of_a_saved_slot_is_titled_edit_and_goes_to_the_confirmation() {
    let mut s = entry("quick_editor_slot", 24);
    s.assert_in_order(&[
        "Edit preset 1",
        &format!("› {CODING}"),
        "enter review changes · esc back",
    ]);
    s.keys("enter");
    // the confirmation shows the saved text, not its label (oracle `confirm_quick_dial`)
    s.assert_in_order(&[
        "Replace preset 1?",
        CODING,
        "→ ",
        CODING,
        "› 1. Replace preset 1",
    ]);
}

#[test]
fn the_sources_are_the_current_settings_and_a_row_opens_the_editor_on_it() {
    let mut s = entry("quick_source", 24);
    s.assert_in_order(&[
        "Start preset from current settings",
        "› 1. Current model",
        "/model gpt-5.5 --speed=standard",
        " 2. Current permissions",
        // the oracle wraps the value at the hyphen (snapshot `saved_quick_dials_source`)
        "/permissions :workspace --approval-policy=on-",
        "request --reviewer=user",
        " 3. Current terminal title",
        "/title model project-name model",
        " 4. Current status line",
        "/statusline model project-name model --no-colors",
        "enter select · esc back",
    ]);
    s.keys("enter");
    s.assert_in_order(&[
        "New preset",
        "› /model gpt-5.5 --speed=standard",
        "enter choose slot · esc back",
    ]);
    s.keys("esc");
    s.assert_absent("New preset");
    s.assert_contains("Start preset from current settings");
    s.assert_contains("› 1. Current model");
}

#[test]
fn a_source_is_picked_by_its_number() {
    let mut s = entry("quick_source", 24);
    s.keys("4");
    s.assert_in_order(&[
        "New preset",
        "› /statusline model project-name model --no-colors",
    ]);
}

#[test]
fn a_source_that_cannot_be_recalled_says_why_and_leads_nowhere() {
    let mut s = entry("quick_source_unavailable", 24);
    s.assert_in_order(&[
        "Start preset from current settings",
        "Current model",
        "Choose an available reasoning effort with /model",
        "› 1. Current permissions",
        "Current terminal title",
        "Choose available terminal-title items with /title",
        " 2. Current status line",
    ]);
    // the disabled rows are not numbered: 1 is the permissions, 2 the status line
    s.keys("3");
    s.assert_contains("Start preset from current settings");
    s.assert_absent("New preset");
    s.keys("2");
    s.assert_in_order(&[
        "New preset",
        "› /statusline model project-name model --no-colors",
    ]);
}

#[test]
fn the_confirmation_of_a_replacement_shows_the_old_input_and_the_new_one() {
    let s = entry("quick_confirm_replace", 24);
    s.assert_in_order(&[
        "Replace preset 1?",
        CODING,
        "→ /model gpt-5.6-terra --reasoning=high --speed=fast",
        "› 1. Replace preset 1",
        "enter select · esc back",
    ]);
}

#[test]
fn the_confirmation_of_a_new_preset_shows_the_input() {
    let s = entry("quick_confirm_save", 24);
    s.assert_in_order(&[
        "Save preset 2?",
        "/model gpt-5.6-terra --reasoning=high --speed=fast",
        "› 1. Save preset 2",
        "enter select · esc back",
    ]);
}

#[test]
fn the_confirmation_of_a_delete_names_the_preset() {
    let s = entry("quick_confirm_delete", 24);
    s.assert_in_order(&[
        "Delete preset 1?",
        "coding",
        "› 1. Delete preset 1",
        "enter select · esc back",
    ]);
}

#[test]
fn the_one_row_of_a_confirmation_needs_enter_and_its_digit_does_not_confirm() {
    let mut s = entry("quick_confirm_delete", 24);
    // the row is numbered 1 but its digit only moves the cursor (`require_explicit_confirmation`)
    s.keys("1");
    s.assert_contains("Delete preset 1?");
    s.keys("enter");
    s.assert_absent("Delete preset 1?");
}

#[test]
fn esc_on_a_confirmation_that_is_the_only_page_closes_the_sheet() {
    let mut s = entry("quick_confirm_replace", 24);
    s.keys("esc");
    s.assert_absent("Replace preset 1?");
}

#[test]
fn a_tap_on_a_slot_picks_it_and_a_tap_on_the_confirmation_row_confirms() {
    let mut s = entry("quick_save", 24);
    s.click_text("1. coding");
    s.assert_contains("Replace preset 1?");
    s.click_text("1. Replace preset 1");
    s.assert_absent("Replace preset 1?");
    s.assert_absent("Save preset to slot");
}
