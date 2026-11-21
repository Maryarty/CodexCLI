//! A viewed image (oracle history_cell/patches.rs `ViewImageHistoryCell`).
//! Compact it is one row with the file's name; it discloses
//! (transcript_view/layout.rs `activity_layout`) the transcript lines, the
//! full path under a two-column hanging indent, and the detailed transcript
//! shows those at once. The oracle also links the name to a local image file
//! and previews the image there: both read the file system, which a view does
//! not (a services effect, left in the handover).
//!
//! Flutter: a `StatefulWidget` holding `expanded` (as an `ExpansionTile`), the
//! control from `disclosure.rs`.

use rtwk::layout::{CrossAxisAlignment, Expanded, Row, SizedBox};
use rtwk::style::{Attrs, Span};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use crate::app::theme::Tokens;

use super::one_row::one_row;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ViewedImageCell {
    pub name: String,
    /// The path as the model gave it (a Windows path keeps its backslashes).
    pub path: String,
    /// The detailed transcript shows the full path and no control.
    pub detailed: bool,
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
        let expanded = widget.detailed || self.expanded;
        let blank = |text: &str| text.replace(['\n', '\r', '\t'], strings.gap);
        let summary: AnyWidget = if expanded {
            // transcript_hyperlink_lines: the bullet, then the label and the
            // path wrapped under two blanks.
            Row::new()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .child(SizedBox::new(
                    Some(2),
                    None,
                    Some(Text::new(strings.bullet).style(tokens.dim()).into()),
                ))
                .child(Expanded::new(
                    1,
                    Text::rich(vec![
                        Span::styled(strings.viewed_image, tokens.bold()),
                        Span::styled(blank(&widget.path), tokens.dim()),
                    ]),
                ))
                .into()
        } else {
            one_row(vec![
                Span::styled(strings.bullet, tokens.dim()),
                Span::styled(strings.viewed_image, tokens.bold()),
                Span::styled(blank(&widget.name), tokens.dim()),
            ])
        };
        if widget.detailed {
            return summary;
        }
        let label = if expanded {
            strings.hide_details
        } else {
            strings.show_details
        };
        super::disclosure::disclosing(cx, summary, label, tokens.dim().attrs(Attrs::DIM), expanded)
    }
}
