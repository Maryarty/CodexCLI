//! The agent's answer in the chat against the oracle's Markdown renderer.
//!
//! The oracle's Markdown tests render a source at a width into bare lines. The
//! chat draws the same Markdown inside the agent's cell: a two-column gutter
//! (`• ` on the first row, blanks below) and one free column at the right
//! (messages.rs `right_reserve`). `Open::markdown(width, height)` opens the
//! chat at the terminal that gives the renderer exactly `width` columns and
//! reads the screen without the gutter, so every assertion and snapshot
//! speaks of the oracle's rows. Nothing else of the screen changes.

use super::Screen;
use super::compare::{self, Want};
use super::grid::Grid;
use super::seed::{Open, Seed};

/// The agent cell's bullet column (history_cell/messages.rs).
pub const GUTTER: u16 = 2;
/// The column the cell keeps free at its right (messages.rs `right_reserve`).
pub const RESERVE: u16 = 1;

impl Open {
    /// The oracle rendered Markdown at `width` columns; the terminal is as
    /// wide as the agent's cell needs for that, `height` rows high.
    pub fn markdown(mut self, width: u16, height: u16) -> Open {
        self.width = width + GUTTER + RESERVE;
        self.height = height;
        self.gutter = usize::from(GUTTER);
        self
    }

    /// A session whose turn answers with `text` (a streamed agent message), for
    /// the oracle tests that build their Markdown source at run time.
    pub fn agent_says(mut self, text: &str) -> Open {
        let delta = serde_json::json!({"step": "notify", "message": {
            "method": "item/agentMessage/delta",
            "params": {"threadId": "thread-1", "turnId": "turn-1", "itemId": "msg-1", "delta": text}
        }});
        self.seed = Seed::Script(Self::turn(&delta));
        self
    }

    /// A session whose turn holds a completed reasoning summary of `text`
    /// (shown by the detailed transcript, ctrl+t).
    pub fn reasoning_says(mut self, text: &str) -> Open {
        let item = serde_json::json!({"step": "notify", "message": {
            "method": "item/completed",
            "params": {"item": {"type": "reasoning", "id": "r1", "summary": [text], "content": []},
                "threadId": "thread-1", "turnId": "turn-1", "completedAtMs": 0}
        }});
        self.seed = Seed::Script(Self::turn(&item));
        self
    }

    /// The script of one turn with `item` in it. Leaked: a test's script
    /// lives for the process, as the `include_str!` ones do.
    fn turn(item: &serde_json::Value) -> &'static str {
        let started = r#"{"step":"notify","message":{"method":"turn/started","params":{"threadId":"thread-1","turn":{"id":"turn-1","rootTurnId":null,"items":[],"itemsView":"full","status":"inProgress","error":null,"startedAt":null,"completedAt":null,"durationMs":null}}}}"#;
        let completed = r#"{"step":"notify","message":{"method":"turn/completed","params":{"threadId":"thread-1","turn":{"id":"turn-1","rootTurnId":null,"items":[],"itemsView":"summary","status":"completed","error":null,"startedAt":null,"completedAt":null,"durationMs":1000}}}}"#;
        let seed = r#"{"step":"seed","seed":"session","thread":"thread-1"}"#;
        Box::leak(format!("{seed}\n{started}\n{item}\n{completed}\n").into_boxed_str())
    }

    /// The session's working directory, as the app learns it: a
    /// `thread/started` of `thread-1` after the script. The oracle's render
    /// tests run in a directory of their own (`/tmp`), which a citation or a
    /// local link is shown relative to.
    pub fn cwd(mut self, cwd: &str) -> Open {
        let started = serde_json::json!({
            "step": "notify",
            "message": {"method": "thread/started", "params": {"thread": {
                "id": "thread-1", "environments": null, "extra": null, "sessionId": "thread-1",
                "forkedFromId": null, "attachedToThreadId": null, "parentThreadId": null,
                "preview": "", "ephemeral": false, "section": null, "sectionEnteredAt": null,
                "projectId": null, "historyMode": "legacy", "modelProvider": "openai",
                "model": null, "reasoningEffort": null, "createdAt": 0, "updatedAt": 0,
                "recencyAt": 0, "status": {"type": "idle"}, "path": null, "cwd": cwd,
                "cliVersion": "0.0.0", "originator": null, "source": "cli",
                "canAcceptDirectInput": true, "threadSource": null, "agentNickname": null,
                "agentRole": null, "gitInfo": null, "name": null, "daybreakEnabled": null,
                "turns": []
            }}}
        });
        let script = match self.seed {
            Seed::Script(script) => script,
            _ => "",
        };
        // A test's script lives for the process, as the `include_str!` it extends does.
        self.seed = Seed::Script(Box::leak(format!("{script}\n{started}\n").into_boxed_str()));
        self
    }
}

impl Screen {
    /// The oracle's snapshot is the Markdown text alone (`plain_lines` of its renderer, from
    /// the first row on); the chat draws the answer under its header and above the composer.
    /// The rows of the snapshot equal ours from the row where its first text row is, up to the
    /// end of the answer (the turn's `Worked for` row, the `Working` row or the composer): an
    /// answer shorter or longer than the snapshot (fewer blank rows between blocks, a wrap)
    /// is compared whole, and not with what the chat draws below it. The chat's own rows are read
    /// from the bottom of the screen (the composer is the last `›` row, the status the rows above
    /// the blank rows over it), so an answer row that starts with `Working`, `Worked for` or `›`
    /// is the answer's and compared.
    /// Where our first row says other things than the oracle's (a formula drawn another way),
    /// the answer starts at the first row with text.
    pub fn assert_markdown_snap(&self, name: &str) {
        let rows = super::snap_rows(name, "");
        let Some((lead, first)) = rows
            .iter()
            .enumerate()
            .find(|(_, row)| !row.trim().is_empty())
        else {
            panic!("the snapshot `{name}` has no text row");
        };
        let grid = self.grid();
        let text_row = |y: usize| grid.row(y, 0, grid.width, false);
        let start = match grid.locate(first.trim()) {
            Some((_, _, row)) => row,
            None => (0..grid.height)
                .find(|&y| !text_row(y).trim().is_empty())
                .unwrap_or_else(|| panic!("no answer on the screen:\n{}", self.text())),
        };
        // the chat's own rows are read from the bottom of the whole screen (the gutter is part
        // of it): the composer is the last row that starts with `›`; the status (`Working`,
        // `Worked for`, which may wrap) is the block of rows right above the blank rows over the
        // composer. Neither is an answer row, one that looks like them is.
        let screen = self.h.t.with_screen(Grid::of);
        let composer = (start + 1..screen.height)
            .rev()
            .find(|&y| screen.row(y, 0, screen.width, false).starts_with('›'));
        let status = composer.and_then(|composer| {
            let last = (start + 1..composer)
                .rev()
                .find(|&y| !text_row(y).trim().is_empty())?;
            let top = (start + 1..=last)
                .rev()
                .take_while(|&y| !text_row(y).trim().is_empty())
                .last()?;
            let row = text_row(top);
            ["Worked for", "Working"]
                .iter()
                .any(|mark| row.trim_start().starts_with(mark))
                .then_some(top)
        });
        let end = status.or(composer).unwrap_or(grid.height);
        let want = Want {
            name,
            frame: "",
            at: Some((start.saturating_sub(lead), 0)),
        };
        compare::assert_snap_rows_to_end(&grid.first_rows(end), &want);
    }
}

impl Grid {
    /// The grid's first `rows` rows.
    pub(super) fn first_rows(&self, rows: usize) -> Grid {
        let height = rows.min(self.height);
        Grid {
            width: self.width,
            height,
            cells: (0..height)
                .flat_map(|y| (0..self.width).map(move |x| (x, y)))
                .map(|(x, y)| self.cell(x, y).clone())
                .collect(),
            cursor: self.cursor.filter(|&(_, y)| y < height),
        }
    }

    /// The grid without its `left` first columns.
    pub(super) fn without_left(&self, left: usize) -> Grid {
        let width = self.width.saturating_sub(left);
        let mut cells = Vec::with_capacity(width * self.height);
        for y in 0..self.height {
            for x in left..self.width {
                cells.push(self.cell(x, y).clone());
            }
        }
        Grid {
            width,
            height: self.height,
            cells,
            cursor: self
                .cursor
                .and_then(|(x, y)| x.checked_sub(left).map(|x| (x, y))),
        }
    }
}
