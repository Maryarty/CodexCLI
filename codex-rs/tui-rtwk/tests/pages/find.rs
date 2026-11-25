// test root: pages
//! Find on the chat's transcript (F3; oracle `transcript_view/search.rs`, `search_footer.rs`,
//! `app/owned_transcript.rs`): the query line under the composer, the newest match first, the
//! match drawn reversed, `Ctrl+P`/`Ctrl+N` stepping without wrapping, Enter reading a match,
//! Esc and `Ctrl+C` closing it. The transcript is two agent answers, `Needle one` and `Needle two`.
//!
//! The footer's place is the page's footer slot, which calls `find_footer::slot` once.

use codex_tui_rtwk::pages::session::transcript::find;

use crate::kit::{Owner, Screen};

const NEEDLES: &str = include_str!("../oracle/fixtures/transcript_view_4/find_jump.jsonl");

fn chat() -> Screen {
    Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(NEEDLES)
        .open()
}

#[test]
fn f3_opens_the_query_line_and_the_newest_match_is_drawn_reversed() {
    let mut s = chat();
    s.keys("f3");
    s.assert_contains("Find:");
    s.assert_contains("Type to find");
    s.type_text("needle two");
    s.assert_contains("Find: needle two");
    s.assert_contains("enter accept");
    s.assert_text_style("Needle two", "reversed");
    s.assert_text_style("Needle one", "!reversed");
}

#[test]
fn the_query_is_found_whatever_its_case() {
    let mut s = chat();
    s.keys("f3");
    s.type_text("NEEDLE TWO");
    s.assert_text_style("Needle two", "reversed");
}

/// The match is the query's range (`needle`), not its whole line: the word `Needle` of the answer
/// on `row` is drawn reversed, or not.
fn matched(s: &Screen, row: usize, reversed: bool) {
    let spec = if reversed { "reversed" } else { "!reversed" };
    for x in 2..8 {
        s.assert_style(x, row, spec);
    }
}

const ONE: usize = 11;
const TWO: usize = 15;

#[test]
fn ctrl_p_and_ctrl_n_step_older_and_newer_and_stop_at_the_ends() {
    let mut s = chat();
    s.keys("f3");
    s.type_text("needle");
    matched(&s, TWO, true);
    s.keys("ctrl+p");
    matched(&s, ONE, true);
    matched(&s, TWO, false);
    s.keys("ctrl+p");
    s.assert_contains("No more matches");
    matched(&s, ONE, true);
    s.keys("ctrl+n");
    matched(&s, TWO, true);
    matched(&s, ONE, false);
    s.keys("ctrl+n");
    s.assert_contains("No more matches");
}

#[test]
fn a_query_nothing_matches_says_so() {
    let mut s = chat();
    s.keys("f3");
    s.type_text("absent");
    s.assert_contains("No matches");
    s.keys("enter");
    // an empty result is not accepted: the query stays open
    s.assert_contains("Find: absent");
}

#[test]
fn esc_closes_find_and_the_composer_has_its_draft_back() {
    let mut s = chat();
    s.type_text("my draft");
    s.keys("f3");
    s.type_text("needle");
    s.assert_contains("Find: needle");
    s.keys("esc");
    s.assert_absent("Find: ");
    s.assert_composer("my draft");
    s.type_text("!");
    s.assert_composer("my draft!");
}

#[test]
fn enter_reads_the_match_and_esc_then_goes_back_to_the_latest() {
    let mut s = chat();
    s.type_text("my draft");
    s.keys("f3");
    s.type_text("needle");
    s.keys("enter");
    s.assert_contains("Find · ctrl+p older · ctrl+n newer · esc latest");
    s.assert_absent("Find: ");
    s.assert_composer("my draft");
    // read: the keys of the match are Find's, the draft is not touched
    s.keys("ctrl+p");
    // reading takes one row less of footer than the query does: the answers are a row lower
    matched(&s, ONE + 1, true);
    s.keys("enter");
    s.assert_composer("my draft");
    s.keys("esc");
    s.assert_absent("Find · ");
    s.assert_composer("my draft");
}

#[test]
fn ctrl_c_cancels_find_instead_of_quitting() {
    let mut s = chat();
    s.keys("f3");
    s.type_text("needle");
    s.keys("ctrl+c");
    s.assert_absent("Find: ");
    s.assert_contains("Needle two");
}

#[test]
fn a_paste_past_the_limit_is_cut_whole_and_says_so() {
    let mut s = chat();
    s.keys("f3");
    s.paste(&"é".repeat(3000));
    s.assert_contains("query limited to 4 KiB");
}

/// The pane reads the finder, not the footer: a query set on the finder is scanned for, the match
/// scrolled to and shown (reversed) by the transcript's selection area, whoever edits the query.
#[test]
fn the_pane_shows_the_match_its_finder_found_and_closing_takes_it_away() {
    let mut s = chat();
    s.keys("f3");
    let finder = find::finder(&s.thread_ui());
    finder.query().set_text("needle two");
    finder.changed();
    s.wait(50);
    s.assert_text_style("Needle two", "reversed");
    s.assert_text_style("Needle one", "!reversed");
    finder.cancel();
    s.wait(50);
    s.assert_text_style("Needle two", "!reversed");
}

/// Find on the transcript sheet (oracle `pager_overlay/transcript.rs`: `find` and `find_transcript`
/// begin the search, `footer_with_navigation` draws its query row and status row in the place of
/// the status row and the hints, from the first column): the sheet's `sheet_basic` entry, a
/// prompt, a reasoning summary and an answer of forty rows, at 60x12.
mod on_the_sheet {
    use crate::kit::{Owner, Screen};

    fn sheet() -> Screen {
        Screen::build(Owner::ChatLive)
            .size(60, 12)
            .entry("transcript", "sheet_basic")
            .open()
    }

    const HINTS: &str = "q close · f3 find · esc browse prompts";

    #[test]
    fn f3_puts_the_query_row_and_the_status_row_where_the_status_and_hints_were() {
        let mut s = sheet();
        s.assert_contains(HINTS);
        s.keys("f3");
        let rows = s.rows();
        // from the first column, the footer is the last two rows
        assert!(rows[10].starts_with("Find:"), "{}", s.text());
        assert!(rows[11].starts_with("Type to find"), "{}", s.text());
        s.assert_absent(HINTS);
        s.assert_absent("to scroll");
    }

    #[test]
    fn the_slash_key_of_the_pager_opens_it_too() {
        let mut s = sheet();
        s.keys("/");
        s.assert_contains("Type to find · full transcript · esc cancel");
    }

    #[test]
    fn typing_finds_the_newest_match_and_scrolls_to_it() {
        let mut s = sheet();
        s.keys("home");
        s.assert_absent("row 39");
        s.keys("f3");
        s.type_text("row 39");
        s.assert_contains("Find: row 39");
        s.assert_text_style("row 39", "reversed");
        s.assert_contains("enter accept");
    }

    #[test]
    fn enter_reads_the_match_and_esc_gives_the_hints_back() {
        let mut s = sheet();
        s.keys("f3");
        s.type_text("row 30");
        s.keys("enter");
        // reading: the status row alone, the keys of the match are Find's
        s.assert_contains("Find · ctrl+p older · ctrl+n newer · esc latest");
        s.assert_absent("Find: ");
        s.keys("ctrl+p");
        s.assert_contains("row 30");
        s.keys("esc");
        s.assert_absent("Find · ");
        s.assert_contains(HINTS);
        // the sheet is open still
        s.assert_contains("T R A N S C R I P T");
    }

    #[test]
    fn esc_while_editing_closes_find_and_not_the_sheet() {
        let mut s = sheet();
        s.keys("f3");
        s.type_text("row");
        s.keys("esc");
        s.assert_absent("Find: ");
        s.assert_contains("T R A N S C R I P T");
        s.assert_contains(HINTS);
    }
}
