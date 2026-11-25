//! The server's answers to the history page requests of a replayed thread (`Open::replay`):
//! `thread/turns/list`, newest page first. The app asks for the first page when it opens the
//! thread and for each older one when the reader reaches the top; a request stays open until a
//! test answers it, so a test can look at the screen while a page is loading (the oracle sets
//! `TranscriptHistoryState` by hand; here the state is the app's own).

use codex_tui_core::app::{AppEv, Command};
use codex_tui_core::screens::ScreenCommand;
use codex_tui_core::screens::agents;
use codex_tui_core::threads::ThreadAsk;

use super::Screen;

impl Screen {
    /// The cursors of the page requests the app has made, oldest first (none for the first page).
    pub fn page_requests(&self) -> Vec<Option<String>> {
        pages(&self.h.fake.commands())
            .into_iter()
            .map(|(_, cursor)| cursor)
            .collect()
    }

    /// How many page requests have no answer yet.
    pub fn pages_waiting(&self) -> usize {
        self.page_requests().len() - self.answered_pages
    }

    /// The server answers the oldest open page request with `turns` (their order as the server
    /// sends the page: newest first) and the cursor of the page after it (`next`; none on the
    /// last page).
    pub fn history_page(
        &mut self,
        turns: Vec<serde_json::Value>,
        next: Option<&str>,
    ) -> &mut Screen {
        let page = serde_json::json!({
            "data": turns, "nextCursor": next, "backwardsCursor": null
        });
        self.answer_page(Ok(page))
    }

    /// The server refuses the oldest open page request with `message`.
    pub fn history_failed(&mut self, message: &str) -> &mut Screen {
        self.answer_page(Err(message.to_string()))
    }

    fn answer_page(&mut self, result: Result<serde_json::Value, String>) -> &mut Screen {
        let asked = pages(&self.h.fake.commands());
        let nth = self.answered_pages;
        let (command, _) = asked
            .get(nth)
            .unwrap_or_else(|| panic!("no open page request ({} made)", asked.len()))
            .clone();
        self.answered_pages = nth + 1;
        let (ticket, reply) =
            codex_tui_core::protocol::reply(&command, result).expect("a command with a reply");
        let epoch = self.h.fake.epoch().expect("attached");
        self.h.fake.inject(epoch, AppEv::Reply(ticket, reply));
        self.h.advance(100);
        self
    }
}

/// What the server has done with the page the reader asked for at the top of the transcript.
#[derive(Clone, Copy, Debug)]
pub enum Page {
    /// Not answered yet: the page is loading.
    Loading,
    /// Refused: the retry is offered.
    Failed,
    /// The last page: nothing older is left.
    Complete,
}

impl Screen {
    /// The reader of a replayed thread of `turns` one-row turns (`row 00`, `row 01`..., the server
    /// has older ones behind a cursor) goes up to its top, where the older page is asked for;
    /// `page` is what the server did with it. `draft` typed first leaves the footer to the
    /// transcript.
    pub fn at_the_top_of_a_partial_history(&mut self, turns: usize, page: Page) -> &mut Screen {
        self.wait_for("Ask Codex to do anything");
        let first: Vec<_> = (0..turns)
            .rev()
            .map(|n| agent_turn(&format!("turn-{n:02}"), &[&format!("row {n:02}")]))
            .collect();
        self.history_page(first, Some("cursor-1"));
        self.type_text("draft");
        for _ in 0..turns * 2 {
            self.wheel_up(0, 0, 10);
        }
        match page {
            Page::Loading => {}
            Page::Failed => {
                self.history_failed("the server is gone");
            }
            Page::Complete => {
                self.history_page(vec![agent_turn("turn-older", &["row older"])], None);
            }
        }
        self
    }
}

/// A finished turn of the server with one agent message each of `rows` (a page of the history
/// is a list of these, newest first).
pub fn agent_turn(id: &str, rows: &[&str]) -> serde_json::Value {
    let items: Vec<serde_json::Value> = rows
        .iter()
        .enumerate()
        .map(|(index, text)| {
            serde_json::json!({
                "type": "agentMessage", "id": format!("{id}-{index}"), "text": text,
                "phase": "final_answer", "memoryCitation": null, "moderation": null,
                "delivery": null, "questions": null
            })
        })
        .collect();
    super::replay_turn(id, serde_json::Value::Array(items))
}

/// A page of one-row turns `{word} {n}` for each `n` of `range`, newest first as the server
/// sends it.
pub fn turns_of(word: &str, range: std::ops::Range<usize>) -> Vec<serde_json::Value> {
    range
        .rev()
        .map(|n| agent_turn(&format!("turn-{word}-{n}"), &[&format!("{word} {n}")]))
        .collect()
}

/// The page requests among `commands`, with the cursor each asks for.
fn pages(commands: &[Command]) -> Vec<(Command, Option<String>)> {
    commands
        .iter()
        .filter_map(|command| match command {
            Command::Screen {
                command:
                    ScreenCommand::Agents(agents::Command::Thread(ThreadAsk::History {
                        cursor, ..
                    })),
                ..
            } => Some((command.clone(), cursor.clone())),
            _ => None,
        })
        .collect()
}
