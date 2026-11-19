//! The configurable status line: items (the oracle's `StatusLineItem` ids, in
//! the order the config gives) joined by ` · `, each in the colour of its
//! accent when the colours are on and in the secondary style when off. One
//! widget for the footer's line and, later, the terminal title setting.
//! Oracle: `bottom_pane/status_line_style.rs` (`status_line_from_segments`,
//! `StatusLineAccent::for_item`, `fallback_color`).

use rtwk::AnyWidget;
use rtwk::layout::Row;
use rtwk::style::Style;
use rtwk::widgets::Text;
use rtwk_core::style::Attrs;

use crate::theme::Tokens;

/// `STATUS_LINE_SEPARATOR`.
pub const SEPARATOR: &str = " \u{b7} ";

/// The colour family of an item (`StatusLineAccent`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Accent {
    Model,
    Path,
    Branch,
    State,
    Usage,
    Limit,
    Metadata,
    Mode,
    Thread,
    Progress,
}

impl Accent {
    /// `StatusLineAccent::for_item`, by the id the config writes.
    fn of(id: &str) -> Self {
        match id {
            "model"
            | "model-name"
            | "model-with-reasoning"
            | "provider"
            | "model-provider"
            | "reasoning" => Self::Model,
            "current-dir" | "project-name" | "project" | "project-root" => Self::Path,
            "git-branch" | "pull-request-number" | "branch-changes" => Self::Branch,
            "run-state" | "status" => Self::State,
            "context-remaining"
            | "context-used"
            | "context-usage"
            | "context-window-size"
            | "used-tokens"
            | "total-input-tokens"
            | "total-output-tokens"
            | "thread-credits"
            | "estimated-thread-cost" => Self::Usage,
            "five-hour-limit" | "weekly-limit" => Self::Limit,
            "codex-version" | "hostname" | "thread-id" | "session-id" | "profile" => Self::Metadata,
            "fast-mode" | "daybreak" | "raw-output" | "permissions" | "approval-mode"
            | "approval" => Self::Mode,
            "thread-name" | "thread-title" | "workspace-headline" => Self::Thread,
            "task-progress" => Self::Progress,
            _ => Self::Metadata,
        }
    }

    /// `fallback_color` (cyan, green, magenta) as the theme's own colours.
    fn style(self, tokens: &Tokens) -> Style {
        match self {
            Self::Model | Self::State | Self::Metadata | Self::Mode => tokens.info(),
            Self::Path | Self::Usage | Self::Progress => tokens.success(),
            Self::Branch | Self::Limit | Self::Thread => tokens.accent(),
        }
    }
}

/// The style of the item `id` (`status_line_from_segments`): its accent with
/// the colours on, the secondary style (`secondary_text_style`) without; the
/// pull request number is underlined as a link.
pub fn item_style(id: &str, colors: bool, tokens: &Tokens) -> Style {
    let style = if colors {
        Accent::of(id).style(tokens)
    } else {
        tokens.dim()
    };
    if id == "pull-request-number" {
        style.attrs(Attrs::UNDERLINE)
    } else {
        style
    }
}

/// The line of `items` (id and text, in order), or `None` when there are
/// none (`status_line_from_segments` returns `None`). Control characters are
/// dropped from the texts.
pub fn status_line<I, S>(items: I, colors: bool, tokens: &Tokens) -> Option<AnyWidget>
where
    I: IntoIterator<Item = (S, String)>,
    S: AsRef<str>,
{
    let mut row = Row::new();
    let mut any = false;
    for (id, text) in items {
        if any {
            row = row.child(Text::line(SEPARATOR).style(tokens.dim()));
        }
        any = true;
        let text: String = text.chars().filter(|c| !c.is_control()).collect();
        row = row.child(Text::line(text).style(item_style(id.as_ref(), colors, tokens)));
    }
    any.then(|| row.into())
}
