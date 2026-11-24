// cell: Item::Generated(_)
//! A finished image generation call (oracle history_cell/patches.rs
//! `ImageGenerationHistoryCell`): a bold heading after a dim bullet (a red
//! cross for a failed one), the revised prompt dim under a dim branch and,
//! when the image was saved, `Saved to: <url>`. The terminal-image preview
//! the oracle adds is the debt task `chat-cells-debt-viewed-image`.
//!
//! Flutter: a stateless widget, a `Column` of one `Text.rich` per row.

use std::sync::Arc;

use rtwk::layout::{Column, CrossAxisAlignment, MainAxisSize};
use rtwk::style::{Attrs, Span};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, StatelessWidget, impl_widget};

use codex_tui_core::threads::GeneratedImage;

use crate::app::theme::Tokens;

use super::dispatch::{CellEnv, Place, below};
use super::strings::Strings;
use codex_tui_core::threads::Item;

/// The texts of the cell: the heading, the detail, and where the image went.
pub fn texts(image: &GeneratedImage, strings: &Strings) -> Vec<String> {
    let heading = if image.failed {
        [strings.generated_cross, strings.generated_failed].concat()
    } else {
        [strings.bullet, strings.generated_heading].concat()
    };
    let mut texts = vec![heading, [strings.file_branch, &image.detail].concat()];
    if let Some(saved) = &image.saved_to {
        texts.push([strings.file_branch, strings.generated_saved_to, saved].concat());
    }
    texts
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedImageCell {
    pub image: Arc<GeneratedImage>,
}

impl_widget!(stateless eq GeneratedImageCell);

impl StatelessWidget for GeneratedImageCell {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let tokens = Tokens::of(cx);
        let strings = super::strings::of(cx);
        let faint = tokens.dim().attrs(Attrs::DIM);
        let heading = if self.image.failed {
            vec![
                Span::styled(strings.generated_cross, tokens.error().attrs(Attrs::BOLD)),
                Span::styled(strings.generated_failed, tokens.bold()),
            ]
        } else {
            vec![
                Span::styled(strings.bullet, faint.clone()),
                Span::styled(strings.generated_heading, tokens.bold()),
            ]
        };
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(Text::rich(heading))
            .child(Text::rich(vec![
                Span::styled(strings.file_branch, faint.clone()),
                Span::styled(self.image.detail.clone(), faint.clone()),
            ]));
        if let Some(saved) = &self.image.saved_to {
            column = column.child(Text::rich(vec![
                Span::styled(strings.file_branch, faint.clone()),
                Span::styled(strings.generated_saved_to, faint),
                Span::new(saved.clone()),
            ]));
        }
        column.into()
    }
}

/// A finished image generation.
/// The cell of `item` when it is this file's (`cell_list!`).
pub fn view(item: &Item, _env: &CellEnv<'_>, place: &Place) -> AnyWidget {
    let Item::Generated(image) = item else {
        return AnyWidget::default();
    };
    below(
        place.separator,
        GeneratedImageCell {
            image: image.clone(),
        }
        .into(),
    )
}
