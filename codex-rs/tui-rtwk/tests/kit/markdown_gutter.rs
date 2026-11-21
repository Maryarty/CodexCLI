//! The agent's answer in the chat against the oracle's Markdown renderer.
//!
//! The oracle's Markdown tests render a source at a width into bare lines. The
//! chat draws the same Markdown inside the agent's cell: a two-column gutter
//! (`• ` on the first row, blanks below) and one free column at the right
//! (messages.rs `right_reserve`). `Open::markdown(width, height)` opens the
//! chat at the terminal that gives the renderer exactly `width` columns and
//! reads the screen without the gutter, so every assertion and snapshot
//! speaks of the oracle's rows. Nothing else of the screen changes.

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

impl Grid {
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
