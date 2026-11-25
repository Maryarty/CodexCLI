//! Find's footer and query, read and set by the finder's handle: the oracle's view tests ask
//! `view.footer(width, Reduced)` apart from the buffer (the bare view draws no footer), read the
//! editor's text (`search.editor.text()`) and set the query by `paste_search`.

use codex_tui_core::app::Command;
use codex_tui_core::app::effects::Ask as Effect;
use codex_tui_rtwk::pages::session::transcript::find;
use codex_tui_rtwk::pages::session::transcript::find_footer;
use codex_tui_rtwk::pages::session::transcript::strings::EN;

use super::{Screen, agent_turn, assert_same, snap_rows};

impl Screen {
    /// The status row Find says at `width` columns: the first of its spellings that fits (the
    /// oracle's `view.footer(width, ..)` while a search is open).
    pub fn find_footer(&mut self, width: usize) -> String {
        let finder = find::finder(&self.thread_ui());
        let spellings = find_footer::candidates(&EN, &finder.status());
        spellings
            .iter()
            .find(|line| line.chars().count() <= width)
            .or(spellings.last())
            .cloned()
            .unwrap_or_default()
    }

    /// The oracle asserts the status row at `width` columns reads `want`.
    pub fn assert_find_footer(&mut self, width: usize, want: &str) {
        let got = self.find_footer(width);
        assert_same(
            got.as_str(),
            want,
            &format!("Find's status row at {width} columns:\n{}", self.text()),
        );
    }

    /// The query is `text`, as typed (the finder scans for it, its match is shown).
    pub fn find_query(&mut self, text: &str) -> &mut Screen {
        let finder = find::finder(&self.thread_ui());
        finder.query().set_text(text);
        finder.changed();
        self.wait(50)
    }

    /// The oracle asserts the query the editor holds (`search.editor.text()`) is `want`.
    pub fn assert_find_query(&mut self, want: &str) {
        let got = find::finder(&self.thread_ui()).query().text().to_string();
        if got != want {
            // a long query is told by its length, its first difference and its end
            let tell = |text: &str| {
                let first = text
                    .chars()
                    .zip(want.chars())
                    .take_while(|(a, b)| a == b)
                    .count();
                let tail: String = text
                    .chars()
                    .rev()
                    .take(12)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect();
                format!(
                    "{} bytes, first difference at char {first}, ends `{tail}`",
                    text.len()
                )
            };
            assert_same(
                tell(&got),
                tell(want),
                &format!("the query Find holds:\n{}", self.text()),
            );
        }
    }

    /// The oracle's snapshot `name` is a list of `<where>: <hint>` lines; the last row of the
    /// screen that holds anything is the hint of line `at`.
    pub fn assert_exit_hint(&self, name: &str, at: usize) {
        let lines = snap_rows(name, "");
        let want = lines
            .get(at)
            .and_then(|line| line.split_once(": "))
            .map_or("", |(_, hint)| hint);
        let rows = self.rows();
        let got = rows
            .iter()
            .rev()
            .map(|row| row.trim())
            .find(|row| !row.is_empty())
            .unwrap_or("");
        assert_same(got, want, &format!("{name}, line {at}:\n{}", self.text()));
    }

    /// The oracle's `ViewAction::Copy(text)`: the latest copy the app asked of the host is `want`.
    pub fn assert_copy_asked(&self, want: &str) {
        let asked = self.commands().into_iter().rev().find_map(|c| match c {
            Command::Effect {
                ask: Effect::Copy { text, .. },
                ..
            } => Some(text),
            _ => None,
        });
        assert_same(
            asked.as_deref(),
            Some(want),
            &format!("the copy asked of the host:\n{}", self.text()),
        );
    }

    /// The oracle sets `TranscriptHistoryState::Partial` by hand over `rows` (oldest first, one
    /// turn each): the server's first page is those turns and has older ones behind a cursor.
    /// For `Open::replay(Vec::new())`.
    pub fn partial_history_of(&mut self, rows: &[&str]) -> &mut Screen {
        for _ in 0..20 {
            if self.pages_waiting() > 0 {
                break;
            }
            self.wait(100);
        }
        let turns = rows
            .iter()
            .enumerate()
            .rev()
            .map(|(n, row)| agent_turn(&format!("turn-{n:02}"), &[row]))
            .collect();
        self.history_page(turns, Some("cursor-1"))
    }
}
