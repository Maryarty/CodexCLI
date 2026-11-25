//! What the app asked the backend to do: the oracle's `Op::UserTurn` and
//! `Op::Interrupt` read off `op_rx`, as the black-box kit sees them.
//!
//! The fake backend records every `Command` the app sends. A test of the
//! oracle that reads the op channel (`next_submit_op`, `op_rx.try_recv()`)
//! asks here instead: the texts of the turns started, in order, and how many
//! interrupts went out. The asserts live in the kit (a shard has none).
//!
//! `AppEvent::FollowTranscript` has no counterpart: following the latest row
//! is the page's scroll state, not a command, so a test that counts it looks
//! at the rows instead.

use codex_tui_core::app::Command;

use super::Screen;

impl Screen {
    /// The texts of the turns the app has started, oldest first (the oracle's
    /// `UserInput::Text` of each `Op::UserTurn`).
    pub fn turn_texts(&self) -> Vec<String> {
        self.turns().into_iter().map(|m| m.text).collect()
    }

    /// How many `InterruptTurn` requests the app has sent.
    pub fn interrupts(&self) -> usize {
        self.h
            .fake
            .commands()
            .iter()
            .filter(|c| matches!(c, Command::InterruptTurn { .. }))
            .count()
    }

    /// Every command the app sent, as its `Debug` text, oldest first (for a
    /// test that looks for one kind of ask: `commands_named("ResumeThread")`).
    pub fn commands_named(&self, name: &str) -> Vec<String> {
        self.h
            .fake
            .commands()
            .iter()
            .map(|c| format!("{c:?}"))
            .filter(|text| text.starts_with(name))
            .collect()
    }

    /// How many wire requests of `method` (`thread/resume`, `turn/start`) went
    /// out to the server.
    pub fn requests_of(&self, method: &str) -> usize {
        self.h
            .fake
            .requests()
            .iter()
            .filter(|r| r["method"] == method)
            .count()
    }

    /// The params of every wire request of `method`, oldest first.
    pub fn request_params(&self, method: &str) -> Vec<serde_json::Value> {
        self.h
            .fake
            .requests()
            .iter()
            .filter(|r| r["method"] == method)
            .map(|r| r["params"].clone())
            .collect()
    }

    /// The turns started so far carry exactly `texts`, in order.
    pub fn assert_turn_texts(&self, texts: &[&str]) {
        let got = self.turn_texts();
        assert_eq!(
            got,
            texts,
            "the turns the app started differ\n{}",
            self.text()
        );
    }

    /// No turn has been started yet (nothing sent).
    pub fn assert_no_turn(&self) {
        self.assert_turn_texts(&[]);
    }

    /// The text the last turn goes out with holds every part of `holds` and ends with `ends`
    /// (the oracle's `UserInput::Text` of a task-mention submission): the message the app
    /// started the turn with, its task mentions given their context as `turn_input` does.
    pub fn assert_last_turn_text(&self, holds: &[&str], ends: &str) {
        let (thread, message) = self
            .h
            .fake
            .commands()
            .into_iter()
            .filter_map(|c| match c {
                Command::StartTurn {
                    thread, message, ..
                } => Some((thread, message)),
                _ => None,
            })
            .next_back()
            .expect("no turn was started");
        let images: Vec<_> = message.images.iter().map(|i| i.range.clone()).collect();
        let text = codex_tui_core::kit::mentions::with_task_context(
            &message.text,
            &message.tasks,
            Some(thread.as_str()),
            &images,
        )
        .map_or(message.text.clone(), |context| context.text);
        for part in holds {
            assert!(text.contains(part), "the turn text lacks {part:?}: {text}");
        }
        assert!(
            text.ends_with(ends),
            "the turn text does not end with {ends:?}: {text}"
        );
    }

    /// `n` interrupts have gone out.
    pub fn assert_interrupts(&self, n: usize) {
        assert_eq!(self.interrupts(), n, "interrupts sent\n{}", self.text());
    }
}
