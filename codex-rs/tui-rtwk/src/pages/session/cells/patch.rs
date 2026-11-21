//! The patch cell (oracle `diff_render.rs` `render_changes_block`, history_cell
//! `PatchHistoryCell` and `PatchApplyFailureCell`): a summary header, one block
//! per file and its numbered lines. Flutter: a `Column` of `Row`s, each diff
//! line a fixed gutter `Text` beside an `Expanded` text that wraps between
//! graphemes (`TextWrap::Grapheme`, the oracle's hard wrap); the hunks come
//! from rtwk-rich's `Diff::parse`, the colours from the syntax highlighter.
//! The cell discloses (transcript_view/layout.rs `activity_layout`): compact,
//! three changed lines a file, until it is tapped or the transcript is
//! detailed (a `StatefulWidget` holding `expanded`, like an `ExpansionTile`).

use std::sync::Arc;

use rtwk::controls::Pressable;
use rtwk::layout::{
    Column, CrossAxisAlignment, EdgeInsets, Expanded, MainAxisSize, Padding, Row, SizedBox,
};
use rtwk::style::{Attrs, Color, Span, Style};
use rtwk::text::TextWrap;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};
use rtwk_rich::diff::{Diff, DiffLineKind};
use rtwk_rich::highlight::{Highlighter, SyntaxTheme, language_named};

use codex_tui_core::threads::{PatchActivity, PatchChange, PatchKind, ToolStatus};

use crate::app::theme::Tokens;

use super::strings::Strings;

/// oracle activity_preview.rs `DETAIL_PREVIEW_LINES`.
const PREVIEW_LINES: usize = 3;

/// One patch item of the transcript.
#[derive(Clone, PartialEq)]
pub struct PatchCell {
    pub activity: Arc<PatchActivity>,
    pub cwd: Option<String>,
    /// The detailed transcript shows every line and no control.
    pub detailed: bool,
}

#[derive(Default)]
pub struct PatchCellState {
    expanded: bool,
}

impl StatefulWidget for PatchCell {
    type State = PatchCellState;

    fn create_state(&self) -> PatchCellState {
        PatchCellState::default()
    }
}

impl State for PatchCellState {
    type Widget = PatchCell;
    type Msg = bool;

    fn update(&mut self, expanded: bool, _: &PatchCell, _: &mut Cx<'_, Self>) -> Rebuild {
        self.expanded = expanded;
        Rebuild::Yes
    }

    fn build(&self, widget: &PatchCell, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let tokens = Tokens::of(cx);
        let syntax = SyntaxTheme::of(cx);
        let strings = super::strings::of(cx);
        let styles = PatchStyles {
            tokens: &tokens,
            syntax: &syntax,
            strings,
            cwd: widget.cwd.as_deref(),
        };
        let expanded = widget.detailed || self.expanded;
        let activity = &widget.activity;
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch);
        if !activity.changes.is_empty() {
            let summary = patch(activity, &styles, (!expanded).then_some(PREVIEW_LINES));
            column = column.child(if widget.detailed {
                summary
            } else {
                let label = if expanded {
                    strings.hide_details
                } else {
                    strings.show_details
                };
                let content: AnyWidget = Column::new()
                    .main_axis_size(MainAxisSize::Min)
                    .cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .child(summary)
                    .child(Padding::new(
                        EdgeInsets::only(2, 0, 0, 0),
                        Text::new(label).style(tokens.dim().attrs(Attrs::DIM)),
                    ))
                    .into();
                let toggle = cx.send(!expanded);
                Pressable::new(move |_, _| content.clone())
                    .bindings([("enter", Some(toggle.clone())), ("space", Some(toggle))])
                    .on_tap(cx.map(move |_| !expanded))
                    .into()
            });
        }
        if activity.status == ToolStatus::Failed {
            // oracle tool_lifecycle.rs: a failed item adds a failure cell; the
            // wire item carries no stderr.
            if !activity.changes.is_empty() {
                column = column.child(Text::new(strings.gap));
            }
            column = column
                .child(
                    Text::new(strings.patch_failed)
                        .style(Style::new().fg(Color::MAGENTA).attrs(Attrs::BOLD)),
                )
                .child(Text::rich(vec![
                    Span::styled(strings.file_branch, tokens.dim()),
                    Span::styled(strings.error_unavailable, tokens.dim().attrs(Attrs::DIM)),
                ]));
        }
        column.into()
    }
}

impl_widget!(stateful eq PatchCell);

/// The styles one patch cell paints with.
pub struct PatchStyles<'a> {
    pub tokens: &'a Tokens,
    pub syntax: &'a SyntaxTheme,
    pub strings: &'a Strings,
    /// The session's working directory: paths below it are shown relative.
    pub cwd: Option<&'a str>,
}

/// The summary block; `preview` keeps that many changed lines a file and
/// leaves the context out (oracle `create_diff_preview_with_links`).
pub fn patch(
    activity: &PatchActivity,
    styles: &PatchStyles<'_>,
    preview: Option<usize>,
) -> AnyWidget {
    let PatchStyles {
        tokens, strings, ..
    } = *styles;
    let mut column = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch);
    let counts: Vec<_> = activity.changes.iter().map(PatchChange::counts).collect();
    let mut header = vec![Span::styled(strings.bullet, tokens.dim())];
    if let [change] = activity.changes.as_slice() {
        let verb = match change.kind {
            PatchKind::Add => strings.added,
            PatchKind::Delete => strings.deleted,
            PatchKind::Update { .. } => strings.edited,
        };
        header.push(Span::styled(verb, tokens.bold()));
        header.push(Span::new(strings.gap));
        header.extend(path_spans(change, styles));
        header.push(Span::new(strings.gap));
        header.extend(count_spans(counts[0], styles));
    } else {
        let (added, removed) = counts
            .iter()
            .fold((0, 0), |(a, r), (added, removed)| (a + added, r + removed));
        let noun = if activity.changes.len() == 1 {
            strings.file
        } else {
            strings.files
        };
        header.push(Span::styled(strings.edited, tokens.bold()));
        header.push(Span::new(strings.gap));
        header.push(Span::new(activity.changes.len().to_string()));
        header.push(Span::new(strings.gap));
        header.push(Span::new(noun));
        header.push(Span::new(strings.gap));
        header.extend(count_spans((added, removed), styles));
    }
    column = column.child(Text::rich(header));
    let single = activity.changes.len() == 1;
    for (index, change) in activity.changes.iter().enumerate() {
        if index > 0 {
            column = column.child(Text::new(""));
        }
        if !single {
            let mut row = vec![Span::styled(strings.file_branch, tokens.dim())];
            row.extend(path_spans(change, styles));
            row.push(Span::new(strings.gap));
            row.extend(count_spans(counts[index], styles));
            column = column.child(Text::rich(row));
        }
        let mut lines = change_lines(change, styles, preview.is_some());
        if let Some(limit) = preview {
            lines.truncate(limit);
        }
        for line in lines {
            column = column.child(line);
        }
    }
    column.into()
}

fn path_spans(change: &PatchChange, styles: &PatchStyles<'_>) -> Vec<Span> {
    let mut spans = vec![Span::new(display_path(&change.path, styles.cwd))];
    if let PatchKind::Update {
        move_path: Some(to),
    } = &change.kind
    {
        spans.push(Span::new(styles.strings.moved_to));
        spans.push(Span::new(display_path(to, styles.cwd)));
    }
    spans
}

/// oracle `display_path_for`: a path below the working directory relative to
/// it, any other as it came.
fn display_path(path: &str, cwd: Option<&str>) -> String {
    cwd.and_then(|cwd| path.strip_prefix(cwd))
        .and_then(|rest| rest.strip_prefix(['/', '\\']))
        .filter(|rest| !rest.is_empty())
        .unwrap_or(path)
        .to_owned()
}

/// oracle `render_line_count_summary`: `(+a -r)`, the numbers green and red.
fn count_spans((added, removed): (usize, usize), styles: &PatchStyles<'_>) -> Vec<Span> {
    let strings = styles.strings;
    vec![
        Span::new(strings.count_open),
        Span::styled(format!("{}{added}", strings.plus), add_style()),
        Span::new(strings.gap),
        Span::styled(format!("{}{removed}", strings.minus), delete_style()),
        Span::new(strings.count_close),
    ]
}

fn add_style() -> Style {
    Style::new().fg(Color::GREEN)
}

fn delete_style() -> Style {
    Style::new().fg(Color::RED)
}

/// One numbered line in the oracle's order: number, sign, text.
struct Numbered<'a> {
    number: usize,
    kind: DiffLineKind,
    text: &'a str,
}

fn change_lines(
    change: &PatchChange,
    styles: &PatchStyles<'_>,
    changed_only: bool,
) -> Vec<AnyWidget> {
    let path = match &change.kind {
        PatchKind::Update {
            move_path: Some(to),
        } => to.as_str(),
        _ => change.path.as_str(),
    };
    let language = path
        .rsplit_once('.')
        .map(|(_, extension)| language_named(extension));
    let mut out = Vec::new();
    let mut hunks: Vec<Vec<Numbered<'_>>> = Vec::new();
    let parsed;
    match change.kind {
        PatchKind::Add | PatchKind::Delete => {
            let kind = if change.kind == PatchKind::Add {
                DiffLineKind::Added
            } else {
                DiffLineKind::Removed
            };
            hunks.push(
                change
                    .diff
                    .lines()
                    .enumerate()
                    .map(|(index, text)| Numbered {
                        number: index + 1,
                        kind,
                        text,
                    })
                    .collect(),
            );
        }
        PatchKind::Update { .. } => {
            parsed = Diff::parse(&change.diff);
            for file in &parsed.files {
                for hunk in &file.hunks {
                    hunks.push(
                        hunk.lines
                            .iter()
                            .filter(|line| !changed_only || line.kind != DiffLineKind::Context)
                            .map(|line| Numbered {
                                number: match line.kind {
                                    DiffLineKind::Removed => line.old_number,
                                    _ => line.new_number,
                                }
                                .unwrap_or_default(),
                                kind: line.kind,
                                text: line.text.as_str(),
                            })
                            .collect(),
                    );
                }
            }
        }
    }
    let widest = hunks
        .iter()
        .flatten()
        .map(|line| line.number)
        .max()
        .unwrap_or_default();
    let gutter = widest.max(1).to_string().len();
    let strings = styles.strings;
    for (index, hunk) in hunks.iter().enumerate() {
        if index > 0 {
            out.push(
                Text::rich(vec![
                    Span::new(strings.diff_indent),
                    Span::new(strings.gap.repeat(gutter + 1)),
                    Span::styled(strings.hunk_gap, styles.tokens.dim()),
                ])
                .into(),
            );
        }
        let source: String = hunk
            .iter()
            .map(|line| line.text)
            .collect::<Vec<_>>()
            .join("\n");
        let highlighted = language.map(|language| {
            let lexer = Highlighter::new(language, &source);
            (0..hunk.len())
                .map(|row| {
                    let text = hunk[row].text;
                    styles
                        .syntax
                        .spans(text, lexer.tokens(row))
                        .into_iter()
                        .map(|span| Span::styled(span.text, span.style))
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        });
        for (row, line) in hunk.iter().enumerate() {
            let (sign, sign_style) = match line.kind {
                DiffLineKind::Added => (strings.plus, add_style()),
                DiffLineKind::Removed => (strings.minus, delete_style()),
                DiffLineKind::Context => (strings.gap, Style::new()),
            };
            let body = match highlighted.as_ref().and_then(|rows| rows.get(row)) {
                Some(spans) if !spans.is_empty() => spans.clone(),
                _ => vec![Span::styled(line.text.to_owned(), sign_style.clone())],
            };
            // The number, its blank and the sign keep their columns even where
            // the sign is a blank (a line's trailing blanks take no room).
            let gutter_text = Text::rich(vec![
                Span::new(strings.diff_indent),
                Span::styled(format!("{:>gutter$}", line.number), styles.tokens.dim()),
                Span::new(strings.gap),
                Span::styled(sign, sign_style.clone()),
            ]);
            let gutter_width = strings.diff_indent.len() + gutter + 2;
            out.push(
                Row::new()
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .child(SizedBox::new(
                        Some(gutter_width as i32),
                        None,
                        Some(gutter_text.into()),
                    ))
                    .child(Expanded::new(1, Text::rich(body).wrap(TextWrap::Grapheme)))
                    .into(),
            );
        }
    }
    out
}
