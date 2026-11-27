// cell: Item::Generated(_)
//! A finished image generation call (oracle history_cell/patches.rs
//! `ImageGenerationHistoryCell`): a bold heading after a dim bullet (a red
//! cross for a failed one), the revised prompt dim under a dim branch and,
//! when the image was saved, `Saved to: <url>`. A validated local artifact
//! has a terminal preview beneath these text rows.
//!
//! Flutter: a stateless widget, a `Column` of one `Text.rich` per row.

use std::sync::Arc;

use rtwk::layout::{Column, CrossAxisAlignment, MainAxisSize};
use rtwk::style::{Attrs, Span};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, StatelessWidget, impl_widget};

use codex_tui_core::threads::GeneratedImage;

use crate::app::theme::Tokens;
use crate::features::images;

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
    pub source: Option<codex_tui_core::features::images::Source>,
    pub raw: bool,
}

impl_widget!(stateless eq GeneratedImageCell);

impl StatelessWidget for GeneratedImageCell {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let tokens = Tokens::of(cx);
        let strings = super::strings::of(cx);
        let read = self
            .source
            .as_ref()
            .map(|source| images::request(cx, source, !self.raw));
        let snapshot = read.as_ref().and_then(|read| images::snapshot(cx, read));
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
            let link = snapshot.as_ref().and_then(|image| image.link.as_deref());
            let saved = link.map_or_else(
                || Span::new(saved.clone()),
                |url| Span::styled(saved.clone(), rtwk::style::Style::default().link(url)),
            );
            column = column.child(Text::rich(vec![
                Span::styled(strings.file_branch, faint.clone()),
                Span::styled(strings.generated_saved_to, faint),
                saved,
            ]));
        }
        if let Some(preview) = snapshot.and_then(|image| image.preview) {
            column = column.child(images::preview(preview));
        }
        images::watch(read, column.into())
    }
}

/// A finished image generation.
/// The cell of `item` when it is this file's (`cell_list!`).
pub fn view(item: &Item, env: &CellEnv<'_>, place: &Place) -> AnyWidget {
    let Item::Generated(image) = item else {
        return AnyWidget::default();
    };
    below(
        place.separator,
        GeneratedImageCell {
            image: image.clone(),
            source: place.image.clone(),
            raw: env.raw,
        }
        .into(),
    )
}
