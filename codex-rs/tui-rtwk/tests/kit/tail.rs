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

/// The turn the live tail of [`Screen::live_tail`] is in.
const TAIL_TURN: &str = "live-tail";

impl Screen {
    /// The thread in use, and the turn running on it (none when no turn runs).
    fn thread_and_turn(&mut self) -> (String, Option<String>) {
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
            *grab.borrow_mut() = thread.map(|thread| (thread, turn));
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
        let (thread, turn) = self.thread_and_turn();
        let start = turn.is_none();
        assert!(
            start || turn.as_deref() == Some(TAIL_TURN),
            "a turn other than the live tail's is running: {turn:?}"
        );
        let line = serde_json::json!({
            "step": "entry", "kind": "agent", "thread": thread, "turn": TAIL_TURN,
            "item": "live-tail-answer", "text": text, "start": start, "end": false,
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
