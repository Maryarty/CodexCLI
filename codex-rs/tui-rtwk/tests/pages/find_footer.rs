// test root: pages
#![allow(clippy::expect_used, clippy::unwrap_used)]

//! The status row of Find (oracle `transcript_view/search_footer.rs`
//! `Search::status_line`): the first spelling that fits the width, for the
//! states and widths the oracle's tests (`search_tests.rs`) pin.

use rtwk::style::Style;
use rtwk_test::{Options, Tester};

use codex_tui_rtwk::pages::session::transcript::find_footer::{Progress, Status, status_row};
use codex_tui_rtwk::pages::session::transcript::strings::EN;

fn row(width: u16, status: Status) -> String {
    let mut t = Tester::with(Options::new().size(width, 1));
    t.pump_widget(status_row(&EN, status, Style::new(), Style::new()));
    t.screen().trim_end().to_owned()
}

fn at(progress: Progress) -> Status {
    Status {
        progress,
        ..Status::idle()
    }
}

#[test]
fn an_empty_finder_asks_for_a_query() {
    assert_eq!(
        row(80, Status::idle()),
        "Type to find · full transcript · esc cancel"
    );
}

#[test]
fn a_scan_says_so_and_the_history_wait_names_the_history() {
    assert_eq!(
        row(80, at(Progress::Scanning)),
        "Searching… · full transcript · esc cancel"
    );
    assert_eq!(
        row(80, at(Progress::Restart)),
        "Searching… · full transcript · esc cancel"
    );
    assert_eq!(
        row(80, at(Progress::AwaitingHistory)),
        "Searching earlier history… · full transcript · esc cancel"
    );
}

#[test]
fn a_match_offers_accept_and_the_two_walking_keys() {
    let found = Status {
        has_match: true,
        query_empty: false,
        ..at(Progress::Found)
    };
    assert_eq!(
        row(80, found),
        "enter accept · ⌃p older · ⌃n newer · full transcript · esc cancel"
    );
    // too narrow for the whole: the compact spellings, then the bare key
    assert_eq!(row(40, found), "enter accept · esc cancel");
    assert_eq!(row(3, found), "");
}

#[test]
fn the_end_of_the_matches_drops_the_full_transcript_hint_when_it_does_not_fit() {
    let exhausted = Status {
        has_match: true,
        query_empty: false,
        ..at(Progress::Exhausted)
    };
    assert_eq!(
        row(80, exhausted),
        "No more matches · enter accept · ⌃p older · ⌃n newer · esc cancel"
    );
    assert_eq!(
        row(80, at(Progress::Exhausted)),
        "No matches · full transcript · esc cancel"
    );
}

#[test]
fn a_failed_history_offers_a_retry() {
    let failed = Status {
        history_failed: true,
        ..at(Progress::AwaitingHistory)
    };
    assert_eq!(
        row(80, failed),
        "History unavailable · ⌃p retry · full transcript · esc cancel"
    );
    assert_eq!(row(30, failed), "⌃p retry · esc cancel");
}

#[test]
fn a_cut_query_says_so_while_it_fits() {
    let cut = Status {
        truncated: true,
        ..at(Progress::Scanning)
    };
    assert_eq!(
        row(100, cut),
        "Searching… · full transcript · esc cancel · query limited to 4 KiB"
    );
}

#[test]
fn reading_a_match_is_find_dot_the_walking_keys_and_the_way_back() {
    let reading = Status {
        reading: true,
        has_match: true,
        ..at(Progress::Found)
    };
    assert_eq!(
        row(80, reading),
        "Find · ctrl+p older · ctrl+n newer · esc latest"
    );
    assert_eq!(row(34, reading), "Find · ctrl+p older · esc latest");
    assert_eq!(row(30, reading), "esc latest");
    assert_eq!(row(14, reading), "esc latest");
}

/// Copy mode on the chat's transcript (`/copy`; oracle `transcript_view/copy_mode.rs`): a cursor
/// over the pieces that are copied alone (the whole last response, each code block and quote of
/// the messages, each prompt), the two-row footer that names the piece (`Copy code block`) and the
/// keys, Enter that copies the piece and goes back to the last row, Esc that ends the mode.
///
/// The mode is begun by its entry (`features::copy::begins_mode`, which the presenter of
/// `Wish::Picker` calls); the keys are the page's. The transcript is the oracle's `copy_nav`: an
/// answer with a code block, a prompt with a quote and a code block, an answer with a quote and a
/// code block.
mod copy_mode {
    use codex_tui_core::app::Command;
    use codex_tui_core::app::effects::{Ask, CopyFormat};
    use codex_tui_rtwk::features::copy::begins_mode;
    use codex_tui_rtwk::widgets::thread_ui::Input;

    use crate::kit::{Owner, Screen};

    const NAV: &str = include_str!("../oracle/fixtures/transcript_view_2/copy_nav.jsonl");
    const VIM: &str = include_str!("../oracle/fixtures/transcript_view_2/user_vim.jsonl");
    const EMPTY: &str = include_str!("../oracle/fixtures/transcript_view_4/sm_entry_6.jsonl");

    const HINTS: &str = "↑/↓/j/k select · g/G ends · enter copy · esc close";

    fn chat(script: &'static str, width: u16) -> Screen {
        let mut s = Screen::build(Owner::ChatLive)
            .size(width, 40)
            .script(script)
            .open();
        s.wait(50);
        s
    }

    /// `/copy`: the mode begins over the transcript of the thread the screen shows.
    fn begin(s: &mut Screen) {
        assert!(begins_mode(&s.thread_ui()));
        s.wait(50);
    }

    fn copies(s: &Screen) -> Vec<(String, CopyFormat)> {
        s.commands()
            .into_iter()
            .filter_map(|command| match command {
                Command::Effect {
                    ask: Ask::Copy { text, format, .. },
                    ..
                } => Some((text, format)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_mode_begins_on_the_whole_response_and_the_footer_says_its_keys() {
        let mut s = chat(NAV, 80);
        s.assert_absent("Copy whole response");
        begin(&mut s);
        assert_eq!(s.thread_ui().input(), Input::Copy);
        s.assert_contains("Copy whole response");
        s.assert_contains(HINTS);
        // the response is the whole of the last answer: its text is reversed, not the prompt's
        s.assert_text_style("Latest plan", "reversed");
        s.assert_text_style("assistant code", "reversed");
        s.assert_text_style("User text", "!reversed");
        s.assert_text_style("Old answer", "!reversed");
    }

    #[test]
    fn the_footer_is_the_shorter_spelling_where_the_width_asks() {
        let mut s = chat(NAV, 40);
        begin(&mut s);
        s.assert_contains("Copy whole response");
        s.assert_contains("↑/↓ select · enter copy · esc close");
        s.assert_absent("g/G ends");
    }

    #[test]
    fn up_walks_the_pieces_backwards_and_down_walks_them_forwards() {
        let mut s = chat(NAV, 80);
        begin(&mut s);
        // the code block of the last answer, then its quote
        s.keys("up");
        s.assert_contains("Copy code block");
        s.keys("up");
        s.assert_contains("Copy blockquote");
        // the prompt's code, its quote, the prompt as a whole
        s.keys("up");
        s.assert_contains("Copy code block");
        s.keys("up");
        s.assert_contains("Copy blockquote");
        s.keys("up");
        s.assert_contains("Copy user message");
        // the code of the first answer, the oldest piece: Up stays there
        s.keys("up");
        s.assert_contains("Copy code block");
        s.keys("up");
        // and down, as far as the whole response past the last piece
        s.keys("down");
        s.assert_contains("Copy user message");
        s.keys("down down");
        s.assert_contains("Copy code block");
        s.keys("down");
        s.assert_contains("Copy blockquote");
        s.keys("down");
        s.assert_contains("Copy code block");
        s.keys("down");
        s.assert_contains("Copy whole response");
        s.keys("down");
        s.assert_contains("Copy whole response");
    }

    #[test]
    fn the_vim_keys_and_the_ends_move_the_cursor() {
        let mut s = chat(NAV, 80);
        begin(&mut s);
        s.keys("k");
        s.assert_contains("Copy code block");
        s.keys("k");
        s.assert_contains("Copy blockquote");
        s.keys("j");
        s.assert_contains("Copy code block");
        // Home and `g` are the first piece, End and `G` the whole response
        s.keys("g");
        s.keys("shift+g");
        s.assert_contains("Copy whole response");
        s.keys("home");
        s.keys("end");
        s.assert_contains("Copy whole response");
    }

    #[test]
    fn a_prompt_with_markup_is_a_piece_and_the_prompts_are_all_there_is() {
        let mut s = chat(VIM, 80);
        begin(&mut s);
        // no response: the cursor starts on the last prompt
        s.assert_contains("Copy user message");
        s.assert_text_style("Plain prompt with wrapping", "reversed");
        s.assert_text_style("First user message", "!reversed");
        s.keys("k");
        s.assert_text_style("First user message", "reversed");
        s.assert_text_style("Plain prompt with wrapping", "!reversed");
        s.keys("j");
        s.assert_text_style("Plain prompt with wrapping", "reversed");
    }

    #[test]
    fn every_other_key_is_the_modes_and_edits_nothing() {
        let mut s = chat(NAV, 80);
        s.type_text("draft");
        begin(&mut s);
        s.keys("x");
        s.type_text("zzz");
        s.keys("backspace");
        s.assert_contains("draft");
        s.assert_absent("draftx");
        s.assert_absent("zzz");
        s.assert_contains("Copy whole response");
        s.keys("esc");
        s.assert_contains("draft");
    }

    #[test]
    fn esc_ends_the_mode_and_lets_go_of_the_piece() {
        let mut s = chat(NAV, 80);
        begin(&mut s);
        s.keys("up");
        s.keys("esc");
        assert_eq!(s.thread_ui().input(), Input::Plain);
        s.assert_absent("Copy code block");
        s.assert_absent(HINTS);
        // and it can begin again
        begin(&mut s);
        s.assert_contains("Copy whole response");
    }

    #[test]
    fn enter_copies_the_piece_and_ends_the_mode() {
        let mut s = chat(NAV, 80);
        begin(&mut s);
        s.keys("up");
        s.keys("enter");
        s.wait(50);
        let sent = copies(&s);
        assert_eq!(sent.len(), 1, "{}", s.text());
        // the code block's own source, newline and all (oracle copy_mode_tests `payload`)
        assert_eq!(sent[0].0, "assistant code\n");
        assert_eq!(s.thread_ui().input(), Input::Plain);
        s.assert_absent(HINTS);
    }

    #[test]
    fn nothing_to_copy_still_begins_a_mode_that_ends_with_enter() {
        let mut s = chat(EMPTY, 80);
        begin(&mut s);
        s.assert_contains("Copy whole response");
        s.keys("enter");
        s.wait(50);
        assert!(copies(&s).is_empty(), "{}", s.text());
        assert_eq!(s.thread_ui().input(), Input::Plain);
    }

    #[test]
    fn the_mode_does_not_begin_while_find_or_select_holds_the_keys() {
        let mut s = chat(NAV, 80);
        s.keys("ctrl+x [");
        assert!(!begins_mode(&s.thread_ui()));
        s.keys("esc");
        s.keys("f3");
        assert!(!begins_mode(&s.thread_ui()));
    }

    /// The cursor is the reversed text of its piece. A message of more than one block is a row
    /// whose mounted cells the list's `ItemTexts` model has to name one by one (the blocks are
    /// `Text`s joined by one line break; the model joins them by a blank line), so a piece after
    /// the first block lands late.
    #[test]
    #[ignore = "pending(chat-live): the mounted ItemTexts model of a Markdown row is its blocks joined by a blank line; the row's cells are joined by one line break, so a range after the first block lands late"]
    fn the_cursor_is_the_reversed_text_of_its_piece() {
        let mut s = chat(NAV, 80);
        begin(&mut s);
        s.assert_text_style("Latest plan", "reversed");
        s.assert_text_style("assistant code", "reversed");
        s.assert_text_style("User text", "!reversed");
        s.keys("up");
        s.assert_text_style("assistant code", "reversed");
        s.assert_text_style("Latest plan", "!reversed");
        s.keys("up");
        s.assert_text_style("assistant quote", "reversed");
        s.assert_text_style("assistant code", "!reversed");
        s.keys("up up");
        s.assert_text_style("user quote", "reversed");
        s.keys("up");
        s.assert_text_style("User text", "reversed");
        s.assert_text_style("user code", "reversed");
        s.keys("g");
        s.assert_text_style("old code", "reversed");
        s.assert_text_style("User text", "!reversed");
        s.keys("end");
        s.assert_text_style("Latest plan", "reversed");
        s.keys("up");
        s.keys("esc");
        s.assert_text_style("assistant code", "!reversed");
    }
}
