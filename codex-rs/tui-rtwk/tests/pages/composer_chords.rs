// test root: pages
//! The composer's editor chords (`f12 e` as `editor.move_line_end`): the first key is held, the
//! second completes it, Esc cancels it, any other key is an ordinary one, and a chord nothing
//! followed for a second is gone; a held chord draws its guide after 350 ms (oracle
//! `keymap/chords.rs`, `bottom_pane/shortcut_hud.rs`, `startup_draft.rs`). The chat's leader
//! chords (`ctrl+x p`) are the page's and win over an editor chord of the same prefix.

use crate::kit::{Owner, Screen};

const CHORDS: &[(&str, &str)] = &[
    ("editor.move_line_end", "f12 e"),
    ("editor.move_line_start", "f12 a"),
    ("editor.kill_line_end", "f12 k"),
    ("editor.delete_backward_word", "f11 d"),
];

fn chat(width: u16, height: u16) -> Screen {
    Screen::build(Owner::ChatNavPane)
        .size(width, height)
        .live()
        .keymap(CHORDS)
        .open()
}

fn starting() -> Screen {
    Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .live()
        .held_start()
        .keymap(CHORDS)
        .open()
}

/// The text now in the composer, read by typing `probe` where the caret is and taking it back.
fn caret_probe(s: &mut Screen, probe: &str) -> String {
    s.type_text(probe);
    let text = s.composer_text();
    for _ in probe.chars() {
        s.keys("backspace");
    }
    text
}

#[test]
fn the_second_key_completes_the_chord_and_carries_out_its_edit() {
    let mut s = chat(80, 24);
    s.type_text("draft");
    s.keys("f12");
    s.keys("a");
    s.assert_composer("draft");
    assert_eq!(caret_probe(&mut s, "Y"), "Ydraft");
    s.keys("f12");
    s.keys("e");
    s.assert_composer("draft");
    assert_eq!(caret_probe(&mut s, "Z"), "draftZ");
    // the first key typed nothing and opened no screen
    s.assert_absent("F12");
}

#[test]
fn an_edit_that_deletes_goes_through_the_composers_own_rules() {
    let mut s = chat(80, 24);
    s.type_text("foo bar");
    s.keys("f11");
    s.keys("d");
    s.assert_composer("foo");
    s.keys("f12");
    s.keys("a");
    s.keys("f12");
    s.keys("k");
    s.assert_composer("");
}

#[test]
fn a_key_that_completes_nothing_is_an_ordinary_key() {
    let mut s = chat(80, 24);
    s.type_text("draft");
    s.keys("f12");
    s.type_text("x");
    // the held key is spent and the letter typed
    s.assert_composer("draftx");
    // and a new prefix begins a chord of its own
    s.keys("f12");
    s.keys("f11");
    s.keys("d");
    s.assert_composer("");
}

#[test]
fn a_plain_letter_with_nothing_held_is_typed() {
    let mut s = chat(80, 24);
    s.type_text("draft");
    s.type_text("e");
    s.assert_composer("drafte");
    s.type_text("ak");
    s.assert_composer("drafteak");
}

#[test]
fn esc_cancels_the_chord_and_is_spent_on_it() {
    let mut s = chat(80, 24);
    s.type_text("draft");
    s.keys("f12");
    s.wait(400);
    s.assert_contains("Move Line End");
    s.keys("esc");
    s.assert_absent("Move Line End");
    s.assert_composer("draft");
    // nothing is held: the next letter is typed
    s.keys("e");
    s.assert_composer("drafte");
}

#[test]
fn a_chord_that_waited_a_second_is_gone_and_the_next_key_is_ordinary() {
    let mut s = chat(80, 24);
    s.type_text("draft");
    s.keys("f12");
    s.wait(1000);
    s.assert_absent("Move Line End");
    s.keys("e");
    s.assert_composer("drafte");
}

#[test]
fn the_guide_shows_at_350_ms_and_goes_with_the_chord() {
    let mut s = chat(80, 24);
    s.type_text("draft");
    s.keys("f12");
    s.assert_absent("Move Line End");
    // the key and the pause after it are 110 ms of the 350
    s.wait(100);
    s.assert_absent("Move Line End");
    s.wait(300);
    s.assert_contains("\u{2500} f12 ");
    s.assert_contains("a \u{b7} Move Line Start");
    s.assert_contains("e \u{b7} Move Line End");
    s.assert_contains("k \u{b7} Kill Line End");
    s.assert_contains("Esc cancel");
    // only the actions of the prefix held are listed
    s.assert_absent("Delete Backward Word");
    // completed: the guide goes at once
    s.keys("e");
    s.assert_absent("Move Line End");
    s.assert_absent("Esc cancel");
    // a chord that outlives its second takes the guide with it
    s.keys("f12");
    s.wait(350);
    s.assert_contains("Move Line End");
    s.wait(900);
    s.assert_absent("Move Line End");
}

#[test]
fn the_guide_is_a_block_at_the_bottom_right_of_the_window() {
    let mut s = chat(80, 24);
    s.keys("f12");
    s.wait(350);
    let rows = s.rows();
    let title = rows
        .iter()
        .position(|row| row.contains("\u{2500} f12 "))
        .expect("the title row");
    // two cells in from the right edge, half the window wide, one row above the bottom
    let row = &rows[title];
    assert!(row.trim_end().ends_with('\u{2500}'), "{row:?}");
    assert_eq!(row.chars().count(), 78, "{row:?}");
    assert_eq!(row.chars().position(|c| c == '\u{2500}'), Some(38));
    // title, blank, three entries, blank, Esc cancel
    let block = |at: usize| rows[at].chars().skip(38).collect::<String>();
    assert!(block(title + 1).trim().is_empty());
    assert!(block(title + 2).contains("a \u{b7} Move Line Start"));
    assert!(block(title + 5).trim().is_empty());
    assert!(block(title + 6).contains("Esc cancel"));
    assert_eq!(title + 6, 22, "the last row is one above the bottom");
}

#[test]
fn a_narrow_window_clips_the_guide_and_a_short_one_draws_a_compact_one_or_none() {
    let mut s = chat(30, 24);
    s.keys("f12");
    s.wait(350);
    s.assert_contains("\u{2500} f12 ");
    s.keys("esc");
    // under seven rows the block covers the window: the title with the cancel key beside it
    let mut short = chat(80, 4);
    short.keys("f12");
    short.wait(350);
    short.assert_row_starts_with(0, "f12   Esc cancel");
    short.assert_row_starts_with(1, "a \u{b7} Move Line Start");
    short.keys("e");
    short.assert_absent("Move Line");
    // under four the page draws nothing at all
    let mut tiny = chat(80, 3);
    tiny.keys("f12");
    tiny.wait(350);
    tiny.assert_absent("f12");
    tiny.assert_absent("Move Line End");
}

#[test]
fn a_chord_edit_on_the_startup_draft_follows_the_confirmation() {
    let mut s = starting();
    s.type_text("draft");
    s.keys("enter");
    s.assert_contains("Waiting for startup");
    // the caret moves and the confirmation stays
    s.keys("f12");
    s.keys("a");
    s.assert_contains("Waiting for startup");
    // Esc with a chord held cancels the chord only
    s.keys("f12");
    s.keys("esc");
    s.assert_contains("Waiting for startup");
    // the edit that changes the text cancels it
    s.keys("f12");
    s.keys("k");
    s.assert_composer("");
    s.assert_absent("Waiting for startup");
}

#[test]
fn the_leaders_chords_stay_the_pages_when_no_editor_chord_shares_their_prefix() {
    // `ctrl+x p` opens the presets (the page takes the prefix before any composer chord); an
    // editor chord on `ctrl+x` is a conflict the keymap settles (it drops the leader's action)
    let mut s = chat(80, 24);
    s.type_text("draft");
    s.keys("ctrl+x p");
    s.assert_composer("draft");
    s.keys("ctrl+x");
    s.wait(400);
    s.assert_absent("Move Line");
}

#[test]
fn normal_mode_keeps_its_own_keys() {
    let mut s = Screen::build(Owner::ChatNavPane)
        .size(80, 24)
        .live()
        .keymap(&[
            ("global.toggle_vim_mode", "f9"),
            ("editor.move_line_end", "alt+x e"),
        ])
        .open();
    s.type_text("foo bar");
    s.keys("f9");
    s.keys("0");
    // the chord's keys are Normal mode's own: `e` moves to the end of the word
    s.keys("alt+x");
    s.keys("e");
    s.type_text("i");
    s.type_text("Q");
    s.assert_composer("foQo bar");
}
