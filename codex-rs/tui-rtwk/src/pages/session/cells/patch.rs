// cell: Item::Patch(_)
//! The patch cell (oracle `diff_render.rs` `render_changes_block`, history_cell
//! `PatchHistoryCell` and `PatchApplyFailureCell`): a summary header, one block
//! per file and its numbered lines. Flutter: a `Column` of `Row`s, each diff
//! line a fixed gutter `Text` beside an `Expanded` text that wraps between
//! graphemes (`TextWrap::Grapheme`, the oracle's hard wrap); the hunks come
//! from rtwk-rich's `Diff::parse`, the colours from the syntax highlighter.
//! The cell discloses (transcript_view/layout.rs `activity_layout`): compact,
//! three changed lines a file, until it is tapped or the transcript is
//! detailed (a `StatefulWidget` holding `expanded`, like an `ExpansionTile`).

use std::borrow::Cow;
use std::sync::Arc;

use rtwk::layout::{
    ColoredBox, Column, ConstrainedBox, CrossAxisAlignment, EdgeInsets, Expanded, MainAxisSize,
    Padding, Row, SizedBox,
};
use rtwk::style::{Attrs, Span, Style};
use rtwk::text::{TextAlign, TextWrap};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};
use rtwk_render::{BoxConstraints, UNBOUNDED};
use rtwk_rich::diff::{Diff, DiffLineKind};
use rtwk_rich::highlight::{Highlighter, SyntaxTheme, language_named};

use codex_tui_core::threads::{PatchActivity, PatchChange, PatchKind, ToolStatus};

use crate::app::theme::Tokens;

use super::dispatch::{CellEnv, Place, below};
use super::strings::Strings;
use codex_tui_core::threads::Item;

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
                super::disclosure::disclosing(
                    cx,
                    summary,
                    label,
                    tokens.dim().attrs(Attrs::DIM),
                    expanded,
                )
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
                    // The oracle's magenta, the theme's accent (status_line.rs).
                    Text::new(strings.patch_failed).style(tokens.accent().attrs(Attrs::BOLD)),
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

/// The summary block; `preview` keeps that many screen rows a file and leaves
/// the context out (oracle `create_diff_preview_with_links`).
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
            column = column.child(SizedBox::new(None, Some(1), None));
        }
        if !single {
            let mut row = vec![Span::styled(strings.file_branch, tokens.dim())];
            row.extend(path_spans(change, styles));
            row.push(Span::new(strings.gap));
            row.extend(count_spans(counts[index], styles));
            column = column.child(Text::rich(row));
        }
        let lines = change_lines(change, styles, preview.is_some());
        match preview {
            // The oracle stops at `limit` screen rows a file, a wrapped line
            // counting for each row it takes (`lines.truncate(limit)` after
            // wrapping): a column under a maximum height whose clip is meant.
            Some(limit) => {
                let mut rows = Column::new()
                    .main_axis_size(MainAxisSize::Min)
                    .cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .overflow_intended(true);
                for line in lines {
                    rows = rows.child(line);
                }
                column = column.child(ConstrainedBox::new(
                    BoxConstraints::new(0, UNBOUNDED, 0, limit as i32),
                    rows,
                ));
            }
            None => {
                for line in lines {
                    column = column.child(line);
                }
            }
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
        Span::styled(strings.plus, styles.tokens.success()),
        Span::styled(added.to_string(), styles.tokens.success()),
        Span::new(strings.gap),
        Span::styled(strings.minus, styles.tokens.error()),
        Span::styled(removed.to_string(), styles.tokens.error()),
        Span::new(strings.count_close),
    ]
}

/// oracle diff_render.rs `TAB_REPLACEMENT`: a tab in a diff line is four blanks (the
/// content rule, so the lexer's offsets and the painted text agree).
const TAB_REPLACEMENT: &str = "    ";

fn expand_tabs(text: &str) -> Cow<'_, str> {
    if text.contains('\t') {
        Cow::Owned(text.replace('\t', TAB_REPLACEMENT))
    } else {
        Cow::Borrowed(text)
    }
}

/// One numbered line in the oracle's order: number, sign, text.
struct Numbered<'a> {
    number: usize,
    kind: DiffLineKind,
    text: Cow<'a, str>,
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
                        text: expand_tabs(text),
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
                            .map(|line| Numbered {
                                number: match line.kind {
                                    DiffLineKind::Removed => line.old_number,
                                    _ => line.new_number,
                                }
                                .unwrap_or_default(),
                                kind: line.kind,
                                text: expand_tabs(line.text.as_str()),
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
            let left = (strings.diff_indent.len() + gutter + 1) as i32;
            out.push(
                Padding::new(
                    EdgeInsets::only(left, 0, 0, 0),
                    Text::new(strings.hunk_gap).style(styles.tokens.dim()),
                )
                .into(),
            );
        }
        let source: String = hunk
            .iter()
            .map(|line| line.text.as_ref())
            .collect::<Vec<_>>()
            .join("\n");
        let highlighted = language.map(|language| {
            let lexer = Highlighter::new(language, &source);
            (0..hunk.len())
                .map(|row| {
                    let text = hunk[row].text.as_ref();
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
            // The preview leaves the context out (diff_render.rs
            // `render_change_with_preview`), after it measured and
            // highlighted the whole hunk.
            if changed_only && line.kind == DiffLineKind::Context {
                continue;
            }
            let (sign, sign_style) = match line.kind {
                DiffLineKind::Added => (strings.plus, styles.tokens.success()),
                DiffLineKind::Removed => (strings.minus, styles.tokens.error()),
                DiffLineKind::Context => (strings.gap, Style::new()),
            };
            let body = match highlighted.as_ref().and_then(|rows| rows.get(row)) {
                Some(spans) if !spans.is_empty() => spans.clone(),
                _ => vec![Span::styled(line.text.to_string(), sign_style.clone())],
            };
            // Fixed columns: the indent, the number aligned right, its blank
            // and the sign (a line's trailing blanks take no room).
            let width = |columns: usize, child: AnyWidget| {
                SizedBox::new(Some(columns as i32), None, Some(child))
            };
            let row_widget: AnyWidget = Row::new()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .child(width(
                    strings.diff_indent.len(),
                    Text::new(strings.gap).into(),
                ))
                .child(width(
                    gutter,
                    Text::new(line.number.to_string())
                        .style(styles.tokens.dim())
                        .text_align(TextAlign::Right)
                        .into(),
                ))
                .child(width(
                    2,
                    Text::rich(vec![
                        Span::new(strings.gap),
                        Span::styled(sign, sign_style.clone()),
                    ])
                    .into(),
                ))
                .child(Expanded::new(1, Text::rich(body).wrap(TextWrap::Grapheme)))
                .into();
            // oracle `style_line_bg_for`: the whole row, indent to the right edge, is tinted.
            out.push(match line.kind {
                DiffLineKind::Added => {
                    ColoredBox::new(styles.tokens.diff_line(true), row_widget).into()
                }
                DiffLineKind::Removed => {
                    ColoredBox::new(styles.tokens.diff_line(false), row_widget).into()
                }
                DiffLineKind::Context => row_widget,
            });
        }
    }
    out
}

/// A file edit.
/// The cell of `item` when it is this file's (`cell_list!`).
pub fn view(item: &Item, env: &CellEnv<'_>, place: &Place) -> AnyWidget {
    let Item::Patch(activity) = item else {
        return AnyWidget::default();
    };
    below(
        place.separator,
        PatchCell {
            activity: activity.clone(),
            cwd: env.cwd.map(str::to_owned),
            detailed: env.detailed,
        }
        .into(),
    )
}
