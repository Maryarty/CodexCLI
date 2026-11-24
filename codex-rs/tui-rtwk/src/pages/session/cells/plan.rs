//! The proposed plan (oracle history_cell/plans.rs `ProposedPlanCell`).
//! Flutter: a `Column` of a header `Text.rich` and a `Container(color)` around
//! the `MarkdownView`.

use rtwk::AnyWidget;
use rtwk::layout::{BoxWidget, Column, CrossAxisAlignment, EdgeInsets, MainAxisSize};
use rtwk::style::{Attrs, Span, Style};
use rtwk::widgets::Text;
use rtwk_rich::markdown::MarkdownTheme;

use crate::app::theme::Tokens;

use super::dispatch::{CellEnv, Place, below};
use super::strings::Strings;
use codex_tui_core::threads::Item;

/// `• Proposed Plan`, a blank row, then the plan on the prompt surface with a
/// blank row above and below and two columns on each side.
pub fn proposed_plan(
    text: &str,
    cwd: Option<&str>,
    surface: Style,
    markdown: &MarkdownTheme,
    tokens: &Tokens,
    strings: &'static Strings,
) -> AnyWidget {
    let body: AnyWidget = if text.trim().is_empty() {
        Text::new(strings.empty_plan)
            .style(tokens.dim().attrs(Attrs::DIM | Attrs::ITALIC))
            .into()
    } else {
        super::citation::cited(text, false, cwd, strings, markdown.clone())
    };
    Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(Text::rich(vec![
            Span::styled(strings.bullet, tokens.dim()),
            Span::styled(strings.proposed_plan, tokens.bold()),
        ]))
        .child(Text::new(strings.gap))
        .child(
            BoxWidget::new()
                .style(surface)
                .padding(EdgeInsets::only(2, 1, 2, 1))
                .child(body),
        )
        .into()
}

/// A proposed plan.
/// The cell of `item` when it is this file's (`cell_list!`).
pub fn view(item: &Item, env: &CellEnv<'_>, place: &Place) -> AnyWidget {
    let Item::Plan { text } = item else {
        return AnyWidget::default();
    };
    below(
        place.separator,
        proposed_plan(
            text,
            env.cwd,
            env.prompt_surface.clone(),
            env.markdown,
            env.tokens,
            env.cells,
        ),
    )
}
