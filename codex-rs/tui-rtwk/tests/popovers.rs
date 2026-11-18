//! The composer's popovers in the real chat: `/` commands, `$` skills, `@`
//! mentions and files, and history search (Ctrl+R). Rows are compared with
//! the oracle's `.snap` files (`bottom_pane/snapshots/`), read at test time:
//! text and order, not the row look (delta P1 of `parity/chat-nav.md`).
//! The data (files, skills, plugins, history) is seeded by the fixture script
//! of the `composer_popups` debug entry, never handed to a widget.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::disallowed_methods)]

use crate::common::{Harness, ms, start_debug};

fn open_chat() -> Harness {
    let mut h = start_debug();
    h.settle();
    h.open_debug_row("popovers", Some("composer_popups"));
    h.t.pump_for(ms(200));
    h
}

fn type_in(h: &mut Harness, text: &str) {
    h.t.type_text(text);
    h.t.pump_for(ms(200));
}

/// The oracle's snapshot text of `bottom_pane/snapshots/*<name>.snap`.
fn snap(name: &str) -> String {
    let dir = format!(
        "{}/../tui/src/bottom_pane/snapshots",
        env!("CARGO_MANIFEST_DIR")
    );
    let file = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{dir}: {e}"))
        .flatten()
        .map(|e| e.path())
        .find(|p| p.to_string_lossy().ends_with(&format!("__{name}.snap")))
        .unwrap_or_else(|| panic!("no snapshot {name}"));
    std::fs::read_to_string(file).expect("read")
}

/// One row as text: the markers (the selection's and the scrollbar's) gone, runs of blanks as one.
fn norm(line: &str) -> String {
    let line = line
        .trim()
        .trim_start_matches(['›', '▸', '▎'])
        .trim_end_matches('▐');
    line.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The rows of the first Debug-quoted buffer of a snapshot, normalized, blank
/// rows dropped.
fn oracle_rows(name: &str) -> Vec<String> {
    snap(name)
        .lines()
        .filter_map(|l| {
            let l = l.trim();
            let inner = l.strip_prefix('"')?;
            let inner = inner
                .strip_suffix("\",")
                .or_else(|| inner.strip_suffix('"'))?;
            Some(norm(inner))
        })
        .filter(|l| !l.is_empty())
        .collect()
}

fn screen_rows(h: &Harness) -> Vec<String> {
    h.t.screen().lines().map(norm).collect()
}

/// The rows, in order, are among the screen's rows.
fn assert_rows(h: &Harness, rows: &[String]) {
    let screen = screen_rows(h);
    let mut from = 0;
    for row in rows {
        let at = screen[from..]
            .iter()
            .position(|line| line.contains(row.as_str()))
            .unwrap_or_else(|| panic!("row {row:?} not in order on:\n{}", h.t.screen()));
        from += at + 1;
    }
}

// ----------------------------------------------------------------- command_popup

#[test]
fn command_popup_app() {
    let mut h = open_chat();
    type_in(&mut h, "/app");
    let rows: Vec<String> = oracle_rows("command_popup__tests__command_popup_app");
    assert_eq!(rows.len(), 2, "{rows:?}");
    assert_rows(&h, &rows);
}

#[test]
fn command_popup_voice() {
    let mut h = open_chat();
    type_in(&mut h, "/voice");
    let rows = oracle_rows("command_popup__tests__command_popup_voice");
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_rows(&h, &rows);
}

#[test]
fn command_popup_wrap_boundary_shows_both_rows() {
    // The fixture's model has the two service tiers of the oracle's test.
    let mut h = open_chat();
    type_in(&mut h, "/tier");
    assert_rows(
        &h,
        &[
            "/tier-one Use faster inference".to_string(),
            "/tier-two Keep default speed".to_string(),
        ],
    );
}

#[test]
fn the_slash_popup_lists_commands_in_enum_order() {
    let mut h = open_chat();
    type_in(&mut h, "/");
    let screen = screen_rows(&h);
    let at = |name: &str| {
        screen
            .iter()
            .position(|l| l.starts_with(name))
            .unwrap_or_else(|| panic!("{name} missing:\n{}", h.t.screen()))
    };
    assert!(at("/model") < at("/permissions"), "{}", h.t.screen());
    // Quit and Btw are not listed without a filter (`slash_commands.rs`).
    assert!(
        !screen.iter().any(|l| l.starts_with("/quit")),
        "{}",
        h.t.screen()
    );
    assert!(
        !screen.iter().any(|l| l.starts_with("/btw")),
        "{}",
        h.t.screen()
    );
}

#[test]
fn the_slash_popup_filters_as_typed_and_closes_after_the_name() {
    let mut h = open_chat();
    type_in(&mut h, "/mo");
    assert!(h.t.screen().contains("/model"), "{}", h.t.screen());
    assert!(!h.t.screen().contains("/permissions"), "{}", h.t.screen());
    type_in(&mut h, "del now");
    assert!(
        !h.t.screen().contains("choose what model"),
        "{}",
        h.t.screen()
    );
}

#[test]
fn up_and_down_move_the_selection_and_enter_runs_the_selected_command() {
    let mut h = open_chat();
    type_in(&mut h, "/m");
    // model, memories, mention, mcp (presentation order): down once = memories.
    h.press("down");
    h.press("enter");
    h.t.pump_for(ms(800));
    assert!(
        h.t.screen().contains("settings (S15): empty slot"),
        "{}",
        h.t.screen()
    );
}

#[test]
fn esc_closes_the_popup_and_keeps_the_text() {
    let mut h = open_chat();
    type_in(&mut h, "/mo");
    h.press("escape");
    h.t.pump_for(ms(200));
    assert!(
        !h.t.screen().contains("choose what model"),
        "{}",
        h.t.screen()
    );
    assert!(h.t.screen().contains("/mo"), "{}", h.t.screen());
}

// ------------------------------------------------------------------- skill popup

#[test]
fn skill_popup_category_tags() {
    let mut h = open_chat();
    type_in(&mut h, "$sha");
    let rows: Vec<String> = oracle_rows("skill_popup__tests__skill_popup_category_tags")
        .into_iter()
        .filter(|r| r.contains("Secondary details") && !r.contains("mention"))
        .take(2)
        .collect();
    assert_eq!(
        rows,
        [
            "Shared [App] Secondary details",
            "Shared [Skill] Secondary details"
        ]
    );
    assert_rows(&h, &rows);
}

#[test]
fn a_picked_skill_is_inserted_with_its_sigil_and_a_space() {
    let mut h = open_chat();
    type_in(&mut h, "$sha");
    h.press("down");
    h.press("enter");
    // A blank follows the pick: what is typed next stands apart from it.
    type_in(&mut h, "x");
    assert!(h.t.screen().contains("$shared x"), "{}", h.t.screen());
}

// ------------------------------------------------------------ mentions and files

#[test]
fn default_unified_mention_popup() {
    let mut h = open_chat();
    type_in(&mut h, "@sa");
    let row = oracle_rows("chat_composer__tests__default_unified_mention_popup")
        .into_iter()
        .find(|r| r.starts_with("Sample Plugin"))
        .expect("the plugin row");
    assert_eq!(
        row,
        "Sample Plugin Plugin with skills and an MCP server Plugin"
    );
    assert_rows(&h, &[row]);
}

#[test]
fn long_paths_keep_distinguishing_filenames_and_original_selection() {
    // The width-80 snapshot of the oracle's file popup test: both rows in full.
    let text = std::fs::read_to_string(format!(
        "{}/../tui/src/bottom_pane/snapshots/codex_tui__bottom_pane__file_search_popup__tests__long_paths_keep_distinguishing_filenames_and_original_selection.snap",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("snap");
    let first_80 = text
        .split("width 80")
        .nth(1)
        .and_then(|rest| rest.split("\n\n").nth(1))
        .expect("the first width-80 block");
    let rows: Vec<String> = first_80
        .lines()
        .map(norm)
        .filter(|l| !l.is_empty())
        .collect();
    assert_eq!(
        rows,
        [
            "src/shared/long_directory/parser_alpha.rs",
            "src/shared/long_directory/parser_beta.rs"
        ]
    );
    let mut h = open_chat();
    type_in(&mut h, "@parser");
    assert_rows(&h, &rows);
}

#[test]
fn a_picked_file_is_inserted_as_an_at_path() {
    let mut h = open_chat();
    type_in(&mut h, "@parser_a");
    h.press("enter");
    type_in(&mut h, "x");
    assert!(
        h.t.screen()
            .contains("@src/shared/long_directory/parser_alpha.rs x"),
        "{}",
        h.t.screen()
    );
}

#[test]
fn the_at_popup_closes_after_a_space() {
    let mut h = open_chat();
    type_in(&mut h, "@parser ");
    assert!(!h.t.screen().contains("parser_beta"), "{}", h.t.screen());
}

// ---------------------------------------------------------------- history search

#[test]
fn footer_mode_history_search() {
    let mut h = open_chat();
    h.press("ctrl+r");
    type_in(&mut h, "c");
    let footer = oracle_rows("chat_composer__tests__footer_mode_history_search")
        .into_iter()
        .find(|r| r.starts_with("reverse-i-search"))
        .expect("footer");
    assert_eq!(footer, "reverse-i-search: c enter accept · esc cancel");
    assert_rows(&h, &[footer]);
    assert!(h.t.screen().contains("› cargo test"), "{}", h.t.screen());
}

#[test]
fn ctrl_r_and_ctrl_s_walk_older_and_newer_matches() {
    let mut h = open_chat();
    h.press("ctrl+r");
    type_in(&mut h, "c");
    h.press("ctrl+r");
    assert!(h.t.screen().contains("› cargo build"), "{}", h.t.screen());
    h.press("ctrl+s");
    assert!(h.t.screen().contains("› cargo test"), "{}", h.t.screen());
}

#[test]
fn enter_accepts_the_match_into_the_composer() {
    let mut h = open_chat();
    h.press("ctrl+r");
    type_in(&mut h, "git");
    h.press("enter");
    h.t.pump_for(ms(200));
    assert!(h.t.screen().contains("› git status"), "{}", h.t.screen());
    assert!(
        !h.t.screen().contains("reverse-i-search"),
        "{}",
        h.t.screen()
    );
}

#[test]
fn esc_cancels_and_restores_the_draft() {
    let mut h = open_chat();
    type_in(&mut h, "keep me");
    h.press("ctrl+r");
    type_in(&mut h, "git");
    h.press("escape");
    h.t.pump_for(ms(200));
    assert!(h.t.screen().contains("› keep me"), "{}", h.t.screen());
    assert!(
        !h.t.screen().contains("reverse-i-search"),
        "{}",
        h.t.screen()
    );
}
