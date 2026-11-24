//! The composer's popovers in the real chat: `/` commands, `$` skills, `@`
//! mentions and files, and history search (Ctrl+R). Rows are compared with
//! the oracle's `.snap` files (`bottom_pane/snapshots/`), read at test time:
//! text and order, not the row look (delta P1 of `parity/chat-nav.md`).
//! The data (files, skills, plugins, history) is seeded by the fixture script
//! of the `composer_popups` debug entry, never handed to a widget.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::disallowed_methods)]

use codex_tui_core::commands::Target;
use codex_tui_rtwk::pages::session::open;

use crate::common::{Harness, ms, start_debug, start_driven, start_quittable};

fn open_chat() -> Harness {
    open::reset_last_opened();
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
    // /memories opens the settings: the target, not the stub's text.
    assert_eq!(open::last_opened(), Some(Target::Settings));
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
#[ignore = "pending(mount): MentionPopup is placed by session_page.rs (a Need of that file); the popup's rows are not on the page until the line is in"]
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
#[ignore = "pending(mount): MentionPopup is placed by session_page.rs (a Need of that file); the popup's rows are not on the page until the line is in"]
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
fn a_picked_file_is_inserted_as_its_path() {
    // the unified popup (`mentions_v2`) puts the path in as text, the `@` token replaced; the
    // legacy file popup's `@path` and its snapshot are not ported
    let mut h = open_chat();
    type_in(&mut h, "@parser");
    h.press("enter");
    type_in(&mut h, "x");
    assert!(
        h.t.screen()
            .contains("src/shared/long_directory/parser_alpha.rs x"),
        "{}",
        h.t.screen()
    );
    assert!(!h.t.screen().contains("@src/shared"), "{}", h.t.screen());
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

// ------------------------------------------------- chat-nav-fix (G2): the oracle's keys

/// `chat_composer.rs:2141`: only `history_search_previous` begins a search.
#[test]
fn ctrl_s_outside_a_search_does_not_open_one() {
    let mut h = open_chat();
    type_in(&mut h, "keep");
    h.press("ctrl+s");
    h.t.pump_for(ms(200));
    assert!(
        !h.t.screen().contains("reverse-i-search"),
        "{}",
        h.t.screen()
    );
    assert!(h.t.screen().contains("› keep"), "{}", h.t.screen());
}

/// `history_search.rs:172-184`: Up is older, Down is newer.
#[test]
fn up_and_down_walk_the_matches_inside_a_search() {
    let mut h = open_chat();
    h.press("ctrl+r");
    type_in(&mut h, "c");
    assert!(h.t.screen().contains("› cargo test"), "{}", h.t.screen());
    h.press("up");
    assert!(h.t.screen().contains("› cargo build"), "{}", h.t.screen());
    h.press("down");
    assert!(h.t.screen().contains("› cargo test"), "{}", h.t.screen());
}

/// `history_search.rs:213-230`: Enter accepts only a match.
#[test]
fn history_search_no_match_restores_preview_but_keeps_search_open() {
    let mut h = open_chat();
    type_in(&mut h, "keep me");
    h.press("ctrl+r");
    type_in(&mut h, "zzz");
    h.press("enter");
    h.t.pump_for(ms(200));
    let screen = h.t.screen();
    assert!(
        screen.contains("reverse-i-search: zzz  no match"),
        "{screen}"
    );
    // The draft stands where the preview would.
    assert!(screen.contains("› keep me"), "{screen}");
}

#[test]
fn enter_on_an_idle_search_keeps_it_open() {
    let mut h = open_chat();
    h.press("ctrl+r");
    h.press("enter");
    h.t.pump_for(ms(200));
    assert!(
        h.t.screen().contains("reverse-i-search"),
        "{}",
        h.t.screen()
    );
}

/// `history_search.rs:195-199`: Ctrl+C cancels the search and restores the
/// draft; it does not quit the app (parity pack N6).
#[test]
fn ctrl_c_in_a_search_restores_the_draft_and_does_not_quit() {
    let mut h = start_quittable(true);
    h.settle();
    h.open_debug_row("popovers", Some("composer_popups"));
    h.t.pump_for(ms(200));
    type_in(&mut h, "keep me");
    h.press("ctrl+r");
    type_in(&mut h, "git");
    h.press("ctrl+c");
    h.t.pump_for(ms(400));
    assert!(h.t.is_running(), "Ctrl+C in a search quit the app");
    let screen = h.t.screen();
    assert!(screen.contains("› keep me"), "{screen}");
    assert!(!screen.contains("reverse-i-search"), "{screen}");
}

/// The same key outside a search still quits.
#[test]
fn ctrl_c_outside_a_search_still_quits() {
    let mut h = start_quittable(false);
    h.wait_idle();
    h.press("ctrl+c");
    h.t.pump_for(ms(400));
    assert!(!h.t.is_running(), "Ctrl+C outside a search did not quit");
}

/// `history_search.rs`: `_ => (InputResult::None, true)`: a key the search
/// does not know is consumed, it does not reach the chat's bindings.
#[test]
fn a_key_the_search_does_not_know_is_consumed() {
    let mut h = open_chat();
    h.press("ctrl+r");
    type_in(&mut h, "git");
    open::reset_last_opened();
    h.press("ctrl+t");
    h.press("f2");
    h.t.pump_for(ms(400));
    assert_eq!(open::last_opened(), None, "{}", h.t.screen());
    assert!(
        h.t.screen().contains("reverse-i-search: git"),
        "{}",
        h.t.screen()
    );
}

/// `history_search.rs:224-240`: Ctrl+H is Backspace, Ctrl+U empties the query.
#[test]
fn ctrl_h_and_ctrl_u_edit_the_query() {
    let mut h = open_chat();
    h.press("ctrl+r");
    type_in(&mut h, "gitx");
    assert!(
        h.t.screen().contains("reverse-i-search: gitx  no match"),
        "{}",
        h.t.screen()
    );
    h.press("ctrl+h");
    assert!(
        h.t.screen().contains("reverse-i-search: git "),
        "{}",
        h.t.screen()
    );
    assert!(h.t.screen().contains("› git status"), "{}", h.t.screen());
    h.press("ctrl+u");
    let screen = h.t.screen();
    assert!(!screen.contains("git status"), "{screen}");
    assert!(screen.contains("reverse-i-search:"), "{screen}");
}

// ------------------------------------------------- chunk-composer: Ctrl+R as a mode of the composer

/// The row of the screen that starts with `text`.
fn row_of(h: &Harness, text: &str) -> usize {
    h.t.screen()
        .lines()
        .position(|r| r.trim_start().starts_with(text))
        .unwrap_or_else(|| panic!("no row starts with {text:?}:\n{}", h.t.screen()))
}

/// `history_search_paste_tests.rs:46`: a paste goes into the query whole, and
/// the match follows it; one that sanitizes to nothing changes nothing.
#[test]
fn a_paste_goes_into_the_query_and_the_match_follows() {
    let mut h = open_chat();
    type_in(&mut h, "draft");
    h.press("ctrl+r");
    h.t.paste("git");
    h.t.pump_for(ms(200));
    let screen = h.t.screen();
    assert!(screen.contains("› git status"), "{screen}");
    assert!(
        screen.contains("reverse-i-search: git  enter accept · esc cancel"),
        "{screen}"
    );
    h.t.paste("\u{1b}[31m\u{1b}[0m");
    h.t.paste("");
    h.t.pump_for(ms(200));
    assert_eq!(h.t.screen(), screen, "an empty paste changed the search");
    h.press("enter");
    h.t.pump_for(ms(200));
    assert!(h.t.screen().contains("› git status"), "{}", h.t.screen());
    assert!(!h.t.screen().contains("reverse-i-search"));
}

/// `history_search_paste_tests.rs:121`: the footer shows a newline as `↵` and
/// a tab as `⇥`, and the query stays what was pasted.
#[test]
fn the_footer_shows_pasted_newlines_and_tabs_as_marks() {
    let mut h = open_chat();
    type_in(&mut h, "draft");
    h.press("ctrl+r");
    h.t.paste("foo\nbar\tbaz");
    h.t.pump_for(ms(200));
    let screen = h.t.screen();
    assert!(
        screen.contains("reverse-i-search: foo↵bar⇥baz  no match"),
        "{screen}"
    );
    assert!(screen.contains("› draft"), "{screen}");
}

/// `history_search_paste_tests.rs:158`: Esc and Ctrl+C bring the draft back
/// with its placeholder and its caret, the preview and a miss in between.
#[test]
fn cancel_restores_the_draft_its_placeholder_and_its_caret() {
    for cancel in ["escape", "ctrl+c"] {
        let mut h = open_chat();
        type_in(&mut h, "draft");
        h.t.paste(&"x".repeat(1001));
        h.t.pump_for(ms(200));
        h.press("home");
        h.press("right right");
        h.press("ctrl+r");
        h.t.paste("git");
        h.t.pump_for(ms(200));
        assert!(h.t.screen().contains("› git status"), "{}", h.t.screen());
        h.t.paste(" missing");
        h.t.pump_for(ms(200));
        let draft = "› draft[Pasted Content 1001 chars]";
        assert!(h.t.screen().contains(draft), "{}", h.t.screen());
        h.press(cancel);
        h.t.pump_for(ms(200));
        let screen = h.t.screen();
        assert!(!screen.contains("reverse-i-search"), "{cancel}: {screen}");
        assert!(screen.contains(draft), "{cancel}: {screen}");
        // offset 2 of the draft, after the two cells of `› `
        assert_eq!(h.t.cursor(), (4, row_of(&h, "›")), "{cancel}");
    }
}

/// The footer row's look (`history_search_footer_line`): the label dim, the
/// query in the info colour, and `no match` in red.
#[test]
fn the_footer_row_is_styled_as_the_oracle_styles_it() {
    use rtwk::style::Attrs;
    let mut h = open_chat();
    h.press("ctrl+r");
    type_in(&mut h, "zzz");
    let y = row_of(&h, "reverse-i-search");
    let text = h.t.screen().lines().nth(y).unwrap().to_string();
    let at = |needle: &str| text.find(needle).unwrap();
    let label = h.t.cell(at("reverse"), y);
    assert!(label.attrs.intersects(Attrs::DIM), "label: {label:?}");
    let query = h.t.cell(at("zzz"), y);
    assert!(!query.attrs.intersects(Attrs::DIM), "query: {query:?}");
    assert_ne!(query.fg, label.fg, "the query has its own colour");
    let miss = h.t.cell(at("no match"), y);
    assert_ne!(miss.fg, query.fg, "no match is red: {miss:?}");
}

/// A click in the composer while the search is open moves the caret of the
/// preview; Esc still brings back the draft with the caret it had.
#[test]
fn a_click_in_the_preview_does_not_lose_the_draft() {
    let mut h = open_chat();
    type_in(&mut h, "keep me");
    h.press("home");
    h.press("right right");
    h.press("ctrl+r");
    type_in(&mut h, "git");
    let y = row_of(&h, "›");
    h.t.click(8, u16::try_from(y).unwrap());
    h.t.pump_for(ms(200));
    h.press("escape");
    h.t.pump_for(ms(200));
    let screen = h.t.screen();
    assert!(screen.contains("› keep me"), "{screen}");
    assert!(!screen.contains("reverse-i-search"), "{screen}");
    assert_eq!(h.t.cursor(), (4, row_of(&h, "›")));
}

// ------------------------------------------------- chat-nav-fix (G10): Ctrl+/ aliases

/// The server's answer to the fork `/side` asks for: the side conversation `SIDE`.
fn forked() -> serde_json::Value {
    serde_json::json!({
        "thread": {
            "id": SIDE, "environments": null, "extra": null, "sessionId": SIDE,
            "forkedFromId": null, "attachedToThreadId": null, "parentThreadId": null,
            "preview": SIDE, "ephemeral": true, "section": null, "sectionEnteredAt": null,
            "projectId": null, "historyMode": "legacy", "modelProvider": "openai",
            "model": null, "reasoningEffort": null, "createdAt": 1_700_000_000,
            "updatedAt": 1_700_000_000, "recencyAt": 1_700_000_000,
            "status": {"type": "idle"}, "path": null, "cwd": "/work", "cliVersion": "0.0.0",
            "originator": null, "source": "cli", "canAcceptDirectInput": true,
            "threadSource": null, "agentNickname": null, "agentRole": null, "gitInfo": null,
            "name": SIDE, "daybreakEnabled": null, "turns": []
        },
        "model": "gpt-5", "modelProvider": "openai", "serviceTier": null, "cwd": "/work",
        "instructionSources": [], "approvalPolicy": "on-request", "approvalsReviewer": "user",
        "sandbox": {"type": "dangerFullAccess"}, "reasoningEffort": null
    })
}

const SIDE: &str = "side-1";

/// `keymap.rs:730-735`: Ctrl+/ is also Ctrl+7 and, on a terminal that cannot
/// tell them apart, the raw 0x1f byte (Ctrl+_). Each of them switches from a side
/// conversation to its parent and back (`toggle_side_conversation`); the footer names the key
/// over both (`ctrl + / to switch`, `ctrl + / for side`).
#[test]
fn ctrl_slash_and_its_aliases_toggle_side_conversation() {
    for (name, bytes) in [
        ("ctrl+/ (kitty)", b"\x1b[47;5u".to_vec()),
        ("ctrl+7 (kitty)", b"\x1b[55;5u".to_vec()),
        ("ctrl+_ (raw 0x1f)", vec![0x1f]),
    ] {
        let mut h = start_driven(false, 100, 12);
        h.connect();
        h.quiesce();
        h.fake.respond(|request| match request["method"].as_str()? {
            "thread/fork" => Some(Ok(forked())),
            "thread/inject_items" => Some(Ok(serde_json::json!({}))),
            _ => None,
        });
        h.t.type_text("/side");
        h.press("enter");
        h.advance(300);
        h.wait_for(name, |s| s.contains("to switch"));
        h.t.input(&bytes);
        h.advance(300);
        h.wait_for(name, |s| s.contains("for side"));
        h.t.input(&bytes);
        h.advance(300);
        h.wait_for(name, |s| s.contains("to switch"));
    }
}
