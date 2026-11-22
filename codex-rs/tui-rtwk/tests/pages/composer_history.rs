//! The composer's history against the history file: Up and Ctrl+R over
//! persistent entries the host reads on demand (`chat_composer_history.rs`,
//! `history_search.rs`). The host's answers are injected as the events it
//! sends (`popovers::Ev::History*`); what the composer asks of it is read
//! from the model's pending asks.

use crate::common;

use codex_tui_core::app::AppEv;
use codex_tui_core::fixtures::builders::{agent_completed, agent_delta, turn_completed};
use codex_tui_core::screens::ScreenEv;
use codex_tui_core::screens::composer::history::Lookup;
use codex_tui_core::screens::popovers::{Ask, Ev};
use codex_tui_rtwk::app::UiApp;
use codex_tui_rtwk::services::backend::fake::FAKE_THREAD;

use common::{Harness, start};

const LOG: u64 = 7;
/// The empty composer: the placeholder after the prompt.
const EMPTY: &str = "› Ask Codex to do anything";

fn prompt(h: &Harness) -> String {
    h.t.screen()
        .lines()
        .rfind(|row| row.starts_with('›') || row.starts_with('!'))
        .unwrap_or_default()
        .trim_end()
        .to_string()
}

/// A chat whose history file has `entries` entries (the metadata the host
/// reads when the thread opens).
fn chat(entries: usize) -> Harness {
    let mut h = start();
    h.wait_idle();
    reply(
        &mut h,
        Ev::HistoryMetadata {
            log_id: LOG,
            entry_count: entries,
        },
    );
    h
}

/// The host's answer.
fn reply(h: &mut Harness, ev: Ev) {
    h.inject([AppEv::Screen(ScreenEv::Popovers(ev))]);
    h.advance(100);
}

fn entry(h: &mut Harness, offset: usize, text: &str) {
    reply(
        h,
        Ev::HistoryEntry {
            log_id: LOG,
            offset,
            text: Some(text.to_string()),
        },
    );
}

/// What the composer asked of the host so far.
fn asks(h: &Harness) -> Vec<Ask> {
    h.t.read_app(|app: &UiApp| app.core.screens().popovers.pending_asks().to_vec())
}

fn lookups(h: &Harness) -> Vec<Lookup> {
    asks(h)
        .into_iter()
        .filter_map(|ask| match ask {
            Ask::Lookup(lookup) => Some(lookup),
            Ask::Append(_) => None,
        })
        .collect()
}

/// The lookups once `n` are asked: an intent reaches the model on the app's
/// next turn, which the settled screen does not promise.
fn lookups_asked(h: &mut Harness, n: usize) -> Vec<Lookup> {
    h.wait_until("the lookups asked", |h| lookups(h).len() >= n);
    lookups(h)
}

fn appended(h: &Harness) -> Vec<String> {
    asks(h)
        .into_iter()
        .filter_map(|ask| match ask {
            Ask::Append(text) => Some(text),
            Ask::Lookup(_) => None,
        })
        .collect()
}

#[test]
fn up_asks_the_host_for_the_entry_and_the_answer_fills_the_draft() {
    let mut h = chat(3);
    h.press("up");
    h.advance(100);
    // nothing to show yet: the newest entry is being read
    assert_eq!(prompt(&h), EMPTY, "{}", h.t.screen());
    assert_eq!(
        lookups_asked(&mut h, 1),
        vec![Lookup::Entry {
            log_id: LOG,
            offset: 2
        }]
    );
    entry(&mut h, 2, "newest from the file");
    assert_eq!(prompt(&h), "› newest from the file");
    // the walk goes on from where it was
    h.press("up");
    h.advance(100);
    assert_eq!(
        lookups_asked(&mut h, 2).last(),
        Some(&Lookup::Entry {
            log_id: LOG,
            offset: 1
        })
    );
    entry(&mut h, 1, "older from the file");
    assert_eq!(prompt(&h), "› older from the file");
    // Down returns to the newer one, which the store holds: no read
    let asked = lookups(&h).len();
    h.press("down");
    h.advance(100);
    assert_eq!(prompt(&h), "› newest from the file");
    assert_eq!(lookups(&h).len(), asked);
}

#[test]
fn an_answer_for_another_log_is_ignored() {
    let mut h = chat(2);
    h.press("up");
    h.advance(100);
    reply(
        &mut h,
        Ev::HistoryEntry {
            log_id: LOG + 1,
            offset: 1,
            text: Some("stale".into()),
        },
    );
    assert_eq!(prompt(&h), EMPTY, "{}", h.t.screen());
    entry(&mut h, 1, "current");
    assert_eq!(prompt(&h), "› current");
}

#[test]
fn ctrl_r_says_searching_until_the_host_answers_then_previews_the_match() {
    let mut h = chat(3);
    h.press("ctrl+r");
    h.t.type_text("needle");
    h.advance(100);
    let screen = h.t.screen();
    assert!(
        screen.contains("reverse-i-search: needle  searching"),
        "{screen}"
    );
    // the draft is the original while nothing is found
    assert_eq!(prompt(&h), EMPTY, "{screen}");
    entry(&mut h, 2, "a needle in the file");
    let screen = h.t.screen();
    assert!(
        screen.contains("reverse-i-search: needle  enter accept · esc cancel"),
        "{screen}"
    );
    assert_eq!(prompt(&h), "› a needle in the file");
}

#[test]
fn a_failed_read_restores_the_draft_and_says_nothing() {
    let mut h = chat(3);
    h.t.type_text("draft");
    h.advance(100);
    h.press("ctrl+r");
    h.t.type_text("x");
    h.advance(100);
    assert!(h.t.screen().contains("  searching"), "{}", h.t.screen());
    // the probe finds nothing, then the batch read fails for good
    entry(&mut h, 2, "no match here");
    for _ in 0..3 {
        reply(
            &mut h,
            Ev::HistoryBatchFailed {
                log_id: LOG,
                end_offset: 1,
            },
        );
    }
    let screen = h.t.screen();
    assert!(
        screen.contains("reverse-i-search: x")
            && !screen.contains("searching")
            && !screen.contains("no match"),
        "{screen}"
    );
    assert_eq!(prompt(&h), "› draft");
}

#[test]
fn ctrl_c_keeps_the_draft_for_up_but_does_not_write_the_file() {
    let mut h = chat(0);
    h.t.type_text("cancelled draft");
    h.advance(100);
    h.press("ctrl+c");
    h.advance(100);
    assert_eq!(prompt(&h), EMPTY);
    h.press("up");
    h.advance(100);
    assert_eq!(prompt(&h), "› cancelled draft");
    assert_eq!(appended(&h), Vec::<String>::new(), "{:?}", asks(&h));
}

#[test]
fn a_submit_is_appended_to_the_file_and_found_by_up() {
    let mut h = chat(0);
    h.fake.script_turn(vec![
        agent_delta(FAKE_THREAD, "turn-1", "msg-1", "ok"),
        agent_completed(FAKE_THREAD, "turn-1", "msg-1", "ok"),
        turn_completed(FAKE_THREAD, "turn-1"),
    ]);
    h.t.type_text("sent prompt");
    h.press("enter");
    h.wait_for("the answer", |s| s.contains("• ok"));
    h.wait_idle();
    assert_eq!(appended(&h), vec!["sent prompt".to_string()]);
    h.press("up");
    h.advance(100);
    assert_eq!(prompt(&h), "› sent prompt");
}

/// `history_navigation_cursor`: Vim's Normal mode rests on the last
/// character, which is the end for the walk.
#[test]
fn vim_normal_mode_walks_on_from_the_last_character() {
    let mut h = chat(2);
    h.set_keymap(&[("global.toggle_vim_mode", "f9")]);
    h.press("f9");
    h.press("up");
    h.advance(100);
    entry(&mut h, 1, "newest");
    assert_eq!(prompt(&h), "› newest");
    // `$` rests the caret on the last character, not after it
    h.press("$");
    h.press("up");
    h.advance(100);
    assert_eq!(
        lookups_asked(&mut h, 2).last(),
        Some(&Lookup::Entry {
            log_id: LOG,
            offset: 0
        })
    );
    entry(&mut h, 0, "older");
    assert_eq!(prompt(&h), "› older");
}
