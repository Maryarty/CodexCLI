//! The configurable status line: items (the oracle's `StatusLineItem` ids, in
//! the order the config gives) joined by ` · `, each in the colour of its
//! accent when the colours are on and in the secondary style when off. One
//! widget for the footer's line and, later, the terminal title setting.
//! Oracle: `bottom_pane/status_line_style.rs` (`status_line_from_segments`,
//! `StatusLineAccent::for_item`, `fallback_color`).

use crate::widgets::measure::Measure;
use rtwk::layout::{Expanded, Row, SizedBox};
use rtwk::style::{Span, Style};
use rtwk::text::{TextOverflow, segmenter_of};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, LayoutBuilder};
use rtwk_core::style::Attrs;

use crate::app::theme::Tokens;

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

/// The spans of the line of `items` (id and text, in order), empty when there
/// are none (`status_line_from_segments` returns `None`). Control characters
/// are dropped from the texts.
pub fn status_spans<I, S>(
    items: I,
    colors: bool,
    tokens: &Tokens,
    link: impl Fn(&str) -> Option<String>,
) -> Vec<Span>
where
    I: IntoIterator<Item = (S, String)>,
    S: AsRef<str>,
{
    let mut spans = Vec::new();
    for (id, text) in items {
        if !spans.is_empty() {
            spans.push(Span::styled(SEPARATOR, tokens.dim()));
        }
        let text: String = text.chars().filter(|c| !c.is_control()).collect();
        let style = item_style(id.as_ref(), colors, tokens);
        // the pull request number opens its page (`status_line_hyperlink`)
        let style = match link(id.as_ref()) {
            Some(url) => style.link(url),
            None => style,
        };
        spans.push(Span::styled(text, style));
    }
    spans
}

/// The line of `items`, or `None` when there are none, three rows at most
/// (the title setup's preview; the footer's own rows follow the room left,
/// `footer::StatusRows`).
pub fn status_line<I, S>(items: I, colors: bool, tokens: &Tokens) -> Option<AnyWidget>
where
    I: IntoIterator<Item = (S, String)>,
    S: AsRef<str>,
{
    let spans = status_spans(items, colors, tokens, |_| None);
    (!spans.is_empty()).then(|| {
        Text::rich(spans)
            .max_lines(Some(3))
            .overflow(TextOverflow::Ellipsis)
            .into()
    })
}

/// What `warning_notice` is made of: the texts (the screen's string table)
/// and the styles.
pub struct Notice<'a> {
    pub badge: &'a str,
    pub singular: &'a str,
    pub plural: &'a str,
    pub key: &'a str,
    pub view: &'a str,
    pub accent: Style,
    pub secondary: Style,
    pub key_style: Style,
}

impl Notice<'_> {
    /// The three spellings of `warning_labels`: full, compact, count.
    fn labels(&self, count: usize) -> [Vec<(String, Style)>; 3] {
        let word = if count == 1 {
            self.singular
        } else {
            self.plural
        };
        let sep = || (SEPARATOR.to_string(), self.secondary.clone());
        let key = || (self.key.to_string(), self.key_style.clone());
        let badge = || (self.badge.to_string(), self.secondary.clone());
        [
            vec![
                badge(),
                (format!("{count} {word}"), self.accent.clone()),
                sep(),
                key(),
                (self.view.to_string(), self.secondary.clone()),
            ],
            vec![
                badge(),
                (count.to_string(), self.accent.clone()),
                sep(),
                key(),
            ],
            vec![badge(), (count.to_string(), self.accent.clone())],
        ]
    }
}

fn labels_width(seg: Measure, parts: &[(String, Style)]) -> i32 {
    parts.iter().map(|(text, _)| seg.width(text) as i32).sum()
}

/// `footer` with the oracle's warning notice at its right (`warning_notice_layout`):
/// half the width (at least 14) of what is left after the clear cell at the
/// edge is its budget; the full label, else the compact one, else the count;
/// none when not even the count fits.
pub fn with_warning_notice(footer: AnyWidget, count: usize, notice: Notice<'static>) -> AnyWidget {
    if count == 0 {
        return footer;
    }
    LayoutBuilder::new(move |cx, constraints| {
        let seg = segmenter_of(cx);
        let footer = footer.clone();
        let available = (constraints.max_width - 1).max(0);
        let budget = (available / 2).max(14).min(available);
        let labels = notice.labels(count);
        let [full, compact, only] = labels;
        let chosen = [full, compact, only]
            .into_iter()
            .find(|parts| labels_width(seg, parts) <= budget);
        match chosen {
            Some(parts) => {
                // One line of spans: the notice is the secondary style with
                // the count in the accent and the key bold.
                let spans: Vec<Span> = parts
                    .into_iter()
                    .map(|(text, style)| Span::styled(text, style))
                    .collect();
                Row::new()
                    .child(Expanded::new(1, footer))
                    .child(Text::rich_line(spans))
                    .child(SizedBox::new(Some(1), None, None))
                    .into()
            }
            None => footer,
        }
    })
    .into()
}
