//! Screen tests of the `dialogs` slot (S23) and the oracle helper the dialog
//! slices of `screens-dialogs` share (`oracle`): the expected text of a test
//! is the oracle's own `.snap`, read from `codex-rs/tui/src`, never copied.

pub mod oracle {
    //! The oracle's snapshot text and the comparison with an rtwk screen.
    //!
    //! codex-tui draws these views as bottom-pane panes. `assert_rows`
    //! compares the oracle's non-blank rows, trimmed, byte for byte (the
    //! cursor marker `› ` included), as an ordered subsequence of the
    //! screen's rows. No visual difference is accepted: a test that stays red
    //! shows what the view still draws differently.

    // Reading the oracle's `.snap` is the one IO of a test (R12 is about src).
    #![allow(clippy::disallowed_methods)]

    use std::path::PathBuf;

    /// The rows of the oracle snapshot `path` (relative to `codex-rs/tui/src`):
    /// header cut off, trailing blanks trimmed, blank rows dropped.
    pub fn rows(path: &str) -> Vec<String> {
        let full = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tui/src")
            .join(path);
        let text = std::fs::read_to_string(&full)
            .unwrap_or_else(|err| panic!("oracle snapshot {}: {err}", full.display()));
        let mut parts = text.splitn(3, "---\n");
        let _ = parts.next();
        let _ = parts.next();
        parts
            .next()
            .unwrap_or_default()
            .replace("\r\n", "\n")
            .lines()
            .map(|line| squeeze_wide(unquote(line).trim_end()))
            .filter(|line| !line.trim().is_empty())
            .collect()
    }

    /// A `TestBackend` snapshot writes each row as a quoted string.
    fn unquote(line: &str) -> &str {
        line.strip_prefix('"')
            .and_then(|line| line.strip_suffix('"'))
            .unwrap_or(line)
    }

    /// The oracle's snapshot writer prints the empty second cell of a wide
    /// character as a space (`日 本 語`); the text itself has none.
    fn squeeze_wide(line: &str) -> String {
        let wide = |c: char| ('\u{2e80}'..='\u{ffff}').contains(&c) && c != '\u{2022}';
        let chars: Vec<char> = line.chars().collect();
        let mut out = String::new();
        for (i, c) in chars.iter().enumerate() {
            let between = *c == ' '
                && i > 0
                && i + 1 < chars.len()
                && wide(chars[i - 1])
                && wide(chars[i + 1]);
            if !between {
                out.push(*c);
            }
        }
        out
    }

    /// Every oracle row is on the screen, in order (see the module docs).
    pub fn assert_rows(screen: &str, path: &str) {
        assert_rows_except(screen, path, &[]);
    }

    /// As `assert_rows`, for the oracle rows that are not `skip`: a row of
    /// the surrounding chat (the composer) that the dialog does not draw.
    pub fn assert_rows_except(screen: &str, path: &str, skip: &[&str]) {
        let want: Vec<String> = rows(path)
            .into_iter()
            .filter(|row| !skip.contains(&row.trim()))
            .collect();
        assert_sequence(screen, path, &want);
    }

    /// Every row of `want` is on the screen, in order; `label` names where
    /// the rows come from.
    pub fn assert_sequence(screen: &str, label: &str, want: &[String]) {
        let lines: Vec<&str> = screen.lines().collect();
        let mut at = 0;
        for row in want {
            let row = row.trim();
            let Some(found) = lines[at..].iter().position(|line| line.contains(row)) else {
                panic!(
                    "oracle row {row:?} ({label}) not found after line {at}; the screen:\n{screen}"
                );
            };
            at += found + 1;
        }
    }
}

use crate::common::{Harness, ms, start_sized};
use oracle::{assert_rows, assert_rows_except};

fn open(width: u16, height: u16, entry: &str) -> Harness {
    let mut h = start_sized(true, width, height);
    h.open_debug_row("dialogs", Some(entry));
    h.t.pump_for(ms(800));
    h
}

fn closed(h: &mut Harness, title: &str) {
    h.t.pump_for(ms(800));
    assert!(!h.t.screen().contains(title), "{}", h.t.screen());
}

const SHORTCUTS: &str = "bottom_pane/snapshots/codex_tui__bottom_pane__chat_composer__tests__footer_mode_shortcut_overlay.snap";
const KEYBOARD: &str = "bottom_pane/snapshots/codex_tui__bottom_pane__shortcut_overlay__tests__shortcut_keyboard_base.snap";
const COMPOSER_ROW: &str = "› Ask Codex to do anything";

// bottom_pane/chat_composer.rs: footer_mode_shortcut_overlay (the composer row
// is the chat's, not the overlay's).
#[test]
#[ignore = "open: the oracle draws this in its bottom pane; the dialogs-pane slice builds that look (parity pack, Open)"]
fn footer_mode_shortcut_overlay() {
    let h = open(120, 24, "shortcut_overlay");
    assert_rows_except(&h.t.screen(), SHORTCUTS, &[COMPOSER_ROW]);
}

// The session page: `?` on an empty composer expands the oracle's overlay
// above the composer (footer_mode_shortcut_overlay, 100 wide), the footer
// reads `? / esc close`; `?` and Esc close it.
fn session_overlay() -> Harness {
    let mut h = start_sized(false, 100, 24);
    h.wait_idle();
    h.t.type_text("?");
    h.t.pump_for(ms(600));
    h
}

#[test]
fn session_question_mark_opens_the_footer_mode_shortcut_overlay() {
    let h = session_overlay();
    let screen = h.t.screen();
    assert!(!screen.contains("Not implemented yet"), "{screen}");
    // The oracle's rows are indented by two cells and sit above the composer.
    let at = |text: &str| screen.lines().position(|l| l.starts_with(text));
    let title = at("  Keyboard shortcuts").expect("title");
    let compose = at("  Compose       ").expect("groups");
    let custom = at("  /keymap customize").expect("customize");
    let prompt = at("› ").expect("composer");
    assert_eq!(compose, title + 2, "{screen}");
    assert_eq!(custom, compose + 10, "{screen}");
    assert!(prompt > custom, "{screen}");
    assert!(!screen.contains("? for shortcuts"), "{screen}");
}

// The rows of `footer_mode_shortcut_overlay` itself: the oracle renders them with enhanced
// keys on and the Esc hint primed (`⇧enter  New line`, `esc  Edit last message`). The overlay
// is keymap-driven now and reads as the oracle's idle default does in its status tests
// (`⌃j`, `esc esc`: `shortcut_help_above_wide`, pane_footer_1); the terminal's enhanced-keys
// fact and the Esc-backtrack hint are not in the model (parity pack chat-status, Needs). The
// snapshot's rows are the oracle shard's test, pending there with its reason:
// `tests/oracle/composer_1.rs` `footer_mode_snapshots` (`kit::pending`, owner chat-live).

// shortcut_help.rs `group_lines`: two columns when three do not fit, one
// when two do not.
#[test]
fn session_shortcut_overlay_narrows_to_two_columns_and_to_one() {
    for (width, same_row) in [(80, true), (50, false)] {
        let mut h = start_sized(false, width, 40);
        h.wait_idle();
        h.t.type_text("?");
        h.t.pump_for(ms(600));
        let screen = h.t.screen();
        let compose = screen
            .lines()
            .find(|l| l.contains("Compose"))
            .expect("Compose");
        assert_eq!(compose.contains("Session"), same_row, "{screen}");
        let session = screen.lines().position(|l| l.contains("Session")).unwrap();
        let transcript = screen
            .lines()
            .position(|l| l.contains("Transcript (open first)"))
            .unwrap();
        assert!(transcript > session, "{screen}");
        assert!(
            !screen
                .lines()
                .any(|l| l.contains("Session") && l.contains("Transcript")),
            "{screen}"
        );
    }
}

#[test]
fn session_shortcut_overlay_closes_on_question_mark_and_on_escape() {
    for key in ["?", "escape"] {
        let mut h = session_overlay();
        h.press(key);
        h.t.pump_for(ms(600));
        let screen = h.t.screen();
        assert!(!screen.contains("Keyboard shortcuts"), "{key}:\n{screen}");
        assert!(screen.contains("? for shortcuts"), "{key}:\n{screen}");
        assert!(!screen.contains("› ?"), "{key}:\n{screen}");
    }
}

// shortcut_overlay_tests.rs: shortcut_overlay_draws_base_keyboard_and_falls_back_to_the_list
#[test]
#[ignore = "not in oracle 0.170.0: bottom_pane/shortcut_overlay/keyboard.rs, snapshot shortcut_keyboard_base; open: the oracle draws this in its bottom pane"]
fn shortcut_overlay_draws_base_keyboard_and_falls_back_to_the_list() {
    let mut h = open(120, 30, "shortcut_overlay");
    assert!(
        h.t.screen()
            .lines()
            .any(|row| row.contains("Paste image") && row.contains("⌃v")),
        "{}",
        h.t.screen()
    );
    h.press("tab");
    h.t.pump_for(ms(200));
    assert_rows(&h.t.screen(), KEYBOARD);
    h.press("tab");
    h.t.pump_for(ms(200));
    assert!(h.t.screen().contains("Paste image"), "{}", h.t.screen());
}

// Tab swaps the overlay's two pages: the page is the dialog's own state.
#[test]
fn tab_swaps_the_shortcut_overlay_between_its_list_and_the_keyboard() {
    let mut h = open(120, 30, "shortcut_overlay");
    assert!(h.t.screen().contains("Tab: Keyboard"), "{}", h.t.screen());
    h.press("tab");
    h.t.pump_for(ms(200));
    assert!(h.t.screen().contains("Tab: List"), "{}", h.t.screen());
    h.press("tab");
    h.t.pump_for(ms(200));
    assert!(h.t.screen().contains("Tab: Keyboard"), "{}", h.t.screen());
}

#[test]
fn shortcut_overlay_closes_on_question_mark_and_on_escape() {
    for key in ["?", "escape"] {
        let mut h = open(120, 24, "shortcut_overlay");
        h.press(key);
        closed(&mut h, "Keyboard shortcuts");
    }
}

// bottom_pane/warnings_view_tests.rs: warning_pages_wrap_and_scroll_long_diagnostics
// (snapshot warnings_narrow; the oracle's pane is 40 wide, the dialog 44).
#[test]
fn warnings_narrow() {
    let h = open(44, 14, "warnings_long");
    assert_rows(
        &h.t.screen(),
        "bottom_pane/snapshots/codex_tui__bottom_pane__warnings_view__tests__warnings_narrow.snap",
    );
}

#[test]
fn warning_pages_wrap_and_scroll_long_diagnostics() {
    let mut h = open(44, 14, "warnings_long");
    h.press("end");
    h.t.pump_for(ms(200));
    assert!(
        h.t.screen().contains("END OF DIAGNOSTIC"),
        "{}",
        h.t.screen()
    );
    h.press("right");
    h.t.pump_for(ms(200));
    assert!(h.t.screen().contains("Unknown setting"), "{}", h.t.screen());
    h.press("left");
    h.t.pump_for(ms(200));
    assert!(h.t.screen().contains("Long diagnostic"), "{}", h.t.screen());
}

// warning_pages_copy_only_the_current_diagnostic: the pages' text and title.
#[test]
fn warning_pages_show_the_current_diagnostic_and_its_source() {
    let mut h = open(100, 16, "warnings");
    let screen = h.t.screen();
    for row in [
        "Warnings · 1 of 2 · MCP · example",
        "MCP example could not connect",
        "Sign in again using codex mcp login example",
        "k keep & next · esc dismiss & close · ⌃o copy · ←/→ warning",
    ] {
        assert!(screen.contains(row), "{row}\n{screen}");
    }
    h.press("right");
    h.t.pump_for(ms(200));
    let screen = h.t.screen();
    for row in [
        "Warnings · 2 of 2 · Startup",
        "Unknown setting `old_option`",
        "Remove it from config.toml",
    ] {
        assert!(screen.contains(row), "{row}\n{screen}");
    }
}

// warnings_keep_current_and_next_without_dismissing_unvisited_pages
#[test]
fn warnings_keep_and_next_moves_on_and_the_last_keep_closes() {
    let mut h = open(100, 16, "warnings");
    h.press("k");
    h.t.pump_for(ms(200));
    assert!(h.t.screen().contains("2 of 2"), "{}", h.t.screen());
    h.press("k");
    closed(&mut h, "Warnings");
}

#[test]
fn warnings_close_on_escape_and_on_f2() {
    for key in ["escape", "f2"] {
        let mut h = open(100, 16, "warnings");
        h.press(key);
        closed(&mut h, "Warnings ·");
    }
}

// chatwidget/tests/popups_and_settings.rs: feedback_selection_popup_snapshot
#[test]
#[ignore = "open: the oracle draws this in its bottom pane; the dialogs-pane slice builds that look (parity pack, Open)"]
fn feedback_selection_popup_snapshot() {
    let h = open(100, 24, "feedback_categories");
    assert_rows(
        &h.t.screen(),
        "chatwidget/snapshots/codex_tui__chatwidget__tests__feedback_selection_popup.snap",
    );
}

#[test]
fn a_feedback_category_leads_to_the_consent_step() {
    let mut h = open(100, 24, "feedback_categories");
    h.press("down");
    h.press("enter");
    h.t.pump_for(ms(800));
    let screen = h.t.screen();
    assert!(
        screen.contains("Include logs and session history?"),
        "{screen}"
    );
    assert!(!screen.contains("How was this?"), "{screen}");
}

// bottom_pane/feedback_view.rs: feedback_upload_consent_lists_doctor_report
// and ..._lists_windows_sandbox_log_when_included (the header), and the
// options of feedback_consent_params.
#[test]
fn feedback_upload_consent_lists_doctor_report() {
    let h = open(100, 30, "feedback_consent");
    assert_rows(
        &h.t.screen(),
        "bottom_pane/snapshots/codex_tui__bottom_pane__feedback_view__tests__feedback_upload_consent_lists_doctor_report.snap",
    );
    let screen = h.t.screen();
    for row in [
        "1. Yes  Include session logs and diagnostics in the report",
        "2. No   Keep only your note and report metadata",
        "enter select · esc back",
    ] {
        assert!(screen.contains(row), "{row}\n{screen}");
    }
}

#[test]
fn feedback_upload_consent_lists_windows_sandbox_log_when_included() {
    let h = open(100, 30, "feedback_consent_windows");
    assert_rows(
        &h.t.screen(),
        "bottom_pane/snapshots/codex_tui__bottom_pane__feedback_view__tests__feedback_upload_consent_lists_windows_sandbox_log_when_included.snap",
    );
}

#[test]
fn the_consent_answer_closes_the_flow_and_escape_goes_back() {
    let mut h = open(100, 30, "feedback_consent");
    h.press("2");
    closed(&mut h, "Include logs and session history?");
    let mut h = open(100, 30, "feedback_consent");
    h.press("escape");
    closed(&mut h, "Include logs and session history?");
}

/// ADR-011 (beyond the oracle): a tap on a row puts the cursor there (`SelectionList` selects on
/// a tap; Enter submits), so Enter answers for the row tapped.
#[test]
fn a_tap_on_a_feedback_row_moves_the_cursor_to_it() {
    let mut h = open(100, 24, "feedback_categories");
    let screen = h.t.screen();
    let (y, line) = screen
        .lines()
        .enumerate()
        .find(|(_, line)| line.contains("3. good result"))
        .expect("the third category");
    let x = u16::try_from(line.find("3. good result").unwrap()).unwrap();
    h.t.click(x, u16::try_from(y).unwrap());
    h.settle();
    h.press("enter");
    h.t.pump_for(ms(800));
    let screen = h.t.screen();
    assert!(
        screen.contains("Include logs and session history?"),
        "{screen}"
    );
}
