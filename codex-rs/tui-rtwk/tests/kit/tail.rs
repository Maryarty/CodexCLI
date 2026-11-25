//! The two ends of a transcript the oracle's view tests move: its live tail (the answer a turn is
//! still streaming, committed when the turn ends) and its older end (the history page that joins
//! above what is shown). Each step is what the server does; the transcript feature under test
//! reacts to it.
//!
//! The steps work on the thread the screen shows, `thread-1` of a script or the replayed thread of
//! `Open::replay`.

use codex_tui_core::app::{AppEv, Intent};
use codex_tui_core::features::recap::Note as Recap;
use codex_tui_core::ids::ThreadId;
use codex_tui_core::screens::{ScreenIntent, chat};
use codex_tui_core::threads::{Note, NoteKey};
use codex_tui_rtwk::app::UiApp;

use super::Screen;

/// The turn the first live tail of [`Screen::live_tail`] is in; a tail after a committed one is in
/// `live-tail-1`, `live-tail-2`, and so on.
const TAIL_TURN: &str = "live-tail";

/// The turn of the live tail that is the `n`th this thread has had.
fn tail_turn(n: usize) -> String {
    if n == 0 {
        TAIL_TURN.to_owned()
    } else {
        format!("{TAIL_TURN}-{n}")
    }
}

/// The item of the live answer in `turn`.
fn tail_item(turn: &str) -> String {
    format!("{turn}-answer")
}

impl Screen {
    /// The thread in use, and the turn running on it (none when no turn runs).
    fn thread_and_turn(&mut self) -> (String, Option<String>) {
        let (thread, turn, _) = self.thread_turn_and_tails();
        (thread, turn)
    }

    /// The same, and how many live tails the thread has had before (the turns named for them).
    fn thread_turn_and_tails(&mut self) -> (String, Option<String>, usize) {
        let slot = std::rc::Rc::new(std::cell::RefCell::new(None));
        let grab = slot.clone();
        self.h.t.update_app(move |app: &mut UiApp, _| {
            let thread = app
                .core
                .session()
                .active
                .as_ref()
                .map(|id| id.as_str().to_owned());
            let turn = match app.core.active_thread().turn() {
                codex_tui_core::threads::TurnState::Running(running) => {
                    Some(running.turn.as_str().to_owned())
                }
                _ => None,
            };
            let mut tails: Vec<String> = Vec::new();
            for (key, _) in app.core.active_thread().transcript().iter() {
                let turn = key.turn.as_str();
                if turn.starts_with(TAIL_TURN) && !tails.iter().any(|seen| seen == turn) {
                    tails.push(turn.to_owned());
                }
            }
            *grab.borrow_mut() = thread.map(|thread| (thread, turn, tails.len()));
        });
        slot.borrow_mut().take().expect("the screen has a thread")
    }

    /// `/recap` says its recap in the transcript of the thread in use (oracle
    /// `ThreadRecapHistoryCell`: what happened, and what to do next). A note of the thread's own
    /// transcript, not the chat's: the transcript sheet draws it too.
    pub fn recap(&mut self, summary: &str, next: Option<&str>) -> &mut Screen {
        let (thread, _) = self.thread_and_turn();
        let note = Recap::Recap {
            summary: summary.to_owned(),
            next_action: next.map(str::to_owned),
        };
        let intent = chat::Intent::Note {
            thread: Some(ThreadId::new(&thread)),
            note: Note::from(NoteKey::Feature(note.into())),
        };
        let epoch = self.h.fake.epoch().expect("attached");
        let ev = AppEv::Intent(Intent::Screen(ScreenIntent::Chat(intent)));
        self.h.fake.inject(epoch, ev);
        self.h.advance(100);
        self
    }

    /// The model streams `text` into its live answer (the transcript's live tail: the item of a
    /// turn that has not ended): the first call starts the turn, each next one adds to the same
    /// answer.
    pub fn live_tail(&mut self, text: &str) -> &mut Screen {
        let (thread, turn, tails) = self.thread_turn_and_tails();
        let start = turn.is_none();
        assert!(
            start
                || turn
                    .as_deref()
                    .is_some_and(|turn| turn.starts_with(TAIL_TURN)),
            "a turn other than the live tail's is running: {turn:?}"
        );
        let turn = turn.unwrap_or_else(|| tail_turn(tails));
        let line = serde_json::json!({
            "step": "entry", "kind": "agent", "thread": thread, "turn": turn,
            "item": tail_item(&turn), "text": text, "start": start, "end": false,
        });
        self.feed(&line.to_string())
    }

    /// The live answer's text is now `text`, not what it was with more added (the oracle's
    /// `sync_live_tail` replaces the active cell: a live view of the answer is the whole of it each
    /// time). The answer is the live tail's item; the turn goes on running.
    pub fn replace_live_tail(&mut self, text: &str) -> &mut Screen {
        let (thread, turn) = self.thread_and_turn();
        let turn = turn
            .filter(|turn| turn.starts_with(TAIL_TURN))
            .expect("no live tail to replace (call `live_tail` first)");
        let line = serde_json::json!({
            "step": "entry", "kind": "agent", "thread": thread, "turn": turn,
            "item": tail_item(&turn), "text": text, "start": false, "end": false,
            "replace": true,
        });
        self.feed(&line.to_string())
    }

    /// The turn that is running ends: its live tail is committed to the transcript (the oracle's
    /// commit of the active cell; `turn/completed`).
    pub fn commit_live_tail(&mut self) -> &mut Screen {
        let (thread, turn) = self.thread_and_turn();
        let turn = turn.expect("a running turn to end: no live tail");
        let line = serde_json::json!({"step": "commit", "thread": thread, "turn": turn});
        self.feed(&line.to_string())
    }

    /// The first page of a replayed thread's history is `turns` one-row turns (`row 00` is the
    /// oldest) and the server has older ones behind a cursor: the state the oracle's tests set by
    /// hand as `TranscriptHistoryState::Partial`. For `Open::replay(Vec::new())`; the transcript
    /// is not scrolled and nothing is typed, so the view alone can take it.
    pub fn partial_history(&mut self, turns: usize) -> &mut Screen {
        self.wait_for_page_request();
        let first = (0..turns)
            .rev()
            .map(|n| super::agent_turn(&format!("turn-{n:02}"), &[&format!("row {n:02}")]))
            .collect();
        self.history_page(first, Some("cursor-1"))
    }

    /// The older page joins the history: the server answers the request the reader made at the
    /// top of the transcript with one turn per row of `rows` (oldest first), and `more` says it
    /// has still older pages. When the app has not asked yet the reader goes to the top first.
    pub fn join_older_page(&mut self, rows: &[&str], more: bool) -> &mut Screen {
        for _ in 0..200 {
            if self.pages_waiting() > 0 {
                break;
            }
            self.wheel_up(0, 0, 5);
        }
        assert!(
            self.pages_waiting() > 0,
            "the transcript was scrolled to its top and the app asked for no older page"
        );
        let asked = self.page_requests().len();
        let turns = rows
            .iter()
            .enumerate()
            .rev()
            .map(|(n, row)| super::agent_turn(&format!("older-{asked}-{n:02}"), &[row]))
            .collect();
        let next = format!("cursor-{}", asked + 1);
        self.history_page(turns, more.then_some(next.as_str()))
    }

    fn wait_for_page_request(&mut self) {
        for _ in 0..20 {
            if self.pages_waiting() > 0 {
                return;
            }
            self.h.advance(100);
        }
        panic!("the app asked for no page of the thread's history");
    }
}
