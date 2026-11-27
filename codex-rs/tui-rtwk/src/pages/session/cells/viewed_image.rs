// cell: Item::ImageView { .. }
//! A viewed image (oracle history_cell/patches.rs `ViewImageHistoryCell`).
//! Compact it is one row with the file's name; it discloses
//! (transcript_view/layout.rs `activity_layout`) the transcript lines, the
//! full path under a two-column hanging indent, and the detailed transcript
//! shows those at once. The host validates the local hyperlink and reads a
//! bounded preview for an expanded cell in a graphics-capable transcript.
//!
//! Flutter: a `StatefulWidget` holding `expanded` (as an `ExpansionTile`), the
//! control from `disclosure.rs`.

use rtwk::gestures::ExcludeSelection;
use rtwk::layout::{Column, CrossAxisAlignment, Expanded, MainAxisSize, Row, SizedBox};
use rtwk::style::{Attrs, Span};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use crate::app::theme::Tokens;
use crate::features::{artifact_preview, artifact_request, artifact_snapshot, watch_artifact};

use super::dispatch::{CellEnv, Place, below};
use super::one_row::one_row;
use super::strings::Strings;
use super::text::Part;
use codex_tui_core::threads::Item;

/// Control characters read as blanks.
fn blank(text: &str, strings: &Strings) -> String {
    text.replace(['\n', '\r', '\t'], strings.gap)
}

/// The texts of the cell. Expanded: the bullet (a gutter), the label and the
/// full path; compact: the row, whose first span is the bullet. Then the
/// disclosure's control, a gutter, unless the transcript is the detailed one.
pub fn texts(
    name: &str,
    path: &str,
    detailed: bool,
    expanded: bool,
    strings: &Strings,
) -> Vec<Part> {
    let expanded = detailed || expanded;
    let mut parts = if expanded {
        vec![
            Part::gutter(strings.bullet),
            Part::text([strings.viewed_image, &blank(path, strings)].concat()),
        ]
    } else {
        vec![Part::text(
            [strings.bullet, strings.viewed_image, &blank(name, strings)].concat(),
        )]
    };
    if !detailed {
        parts.push(Part::gutter(if expanded {
            strings.hide_details
        } else {
            strings.show_details
        }));
    }
    parts
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ViewedImageCell {
    pub name: String,
    /// The path as the model gave it (a Windows path keeps its backslashes).
    pub path: String,
    /// The detailed transcript shows the full path and no control.
    pub detailed: bool,
    pub raw: bool,
}

#[derive(Default)]
pub struct ViewedImageCellState {
    expanded: bool,
}

impl StatefulWidget for ViewedImageCell {
    type State = ViewedImageCellState;

    fn create_state(&self) -> ViewedImageCellState {
        ViewedImageCellState::default()
    }
}

impl_widget!(stateful eq ViewedImageCell);

impl State for ViewedImageCellState {
    type Widget = ViewedImageCell;
    type Msg = bool;

    fn update(&mut self, expanded: bool, _: &ViewedImageCell, _: &mut Cx<'_, Self>) -> Rebuild {
        self.expanded = expanded;
        Rebuild::Yes
    }

    fn build(&self, widget: &ViewedImageCell, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let tokens = Tokens::of(cx);
        let strings = super::strings::of(cx);
        let expanded = widget.detailed || super::disclosure::expanded(cx, self.expanded);
        let read = artifact_request(cx, expanded && !widget.raw);
        let snapshot = read.as_ref().and_then(|read| artifact_snapshot(cx, read));
        let link = snapshot.as_ref().and_then(|image| image.link.as_deref());
        let path_style = link.map_or_else(|| tokens.dim(), |url| tokens.dim().link(url));
        let summary: AnyWidget = if expanded {
            // transcript_hyperlink_lines: the bullet, then the label and the
            // path wrapped under two blanks.
            Row::new()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .child(ExcludeSelection::new(SizedBox::new(
                    Some(2),
                    None,
                    Some(Text::new(strings.bullet).style(tokens.dim()).into()),
                )))
                .child(Expanded::new(
                    1,
                    Text::rich(vec![
                        Span::styled(strings.viewed_image, tokens.bold()),
                        Span::styled(blank(&widget.path, strings), path_style),
                    ]),
                ))
                .into()
        } else {
            one_row(vec![
                Span::styled(strings.bullet, tokens.dim()),
                Span::styled(strings.viewed_image, tokens.bold()),
                Span::styled(blank(&widget.name, strings), path_style),
            ])
        };
        let text = if widget.detailed {
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
        };
        let child = match snapshot.and_then(|image| image.preview) {
            Some(preview) => Column::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .child(text)
                .child(artifact_preview(preview))
                .into(),
            None => text,
        };
        watch_artifact(read, child)
    }
}

/// A viewed image.
/// The cell of `item` when it is this file's (`cell_list!`).
pub fn view(item: &Item, env: &CellEnv<'_>, place: &Place) -> AnyWidget {
    let Item::ImageView { name, path } = item else {
        return AnyWidget::default();
    };
    below(
        place.separator,
        ViewedImageCell {
            name: name.clone(),
            path: path.clone(),
            detailed: env.detailed,
            raw: env.raw,
        }
        .into(),
    )
}
