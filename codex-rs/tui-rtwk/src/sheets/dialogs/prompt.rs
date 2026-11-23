//! The text prompt of a feature (`/rename`, the file name of `/export`; oracle
//! `bottom_pane/custom_prompt_view.rs` on the shared-picker layout): a title, the line that says
//! where a suggestion stands, the input behind the `›` mark and the hint. In Flutter it is a
//! `TextField` over a `TextEditingController` in a bottom sheet; the controller and the focus node
//! are the host's state (`SmallDialogState`), the model keeps only whether the user edited and the
//! suggestion's state.

use rtwk::editing::TextArea;
use rtwk::focus::{Focus, KeyPress};
use rtwk::layout::{
    BoxWidget, Column, CrossAxisAlignment, EdgeInsets, Expanded, MainAxisSize, Padding, Row,
    SizedBox,
};
use rtwk::overlay::Dialog;
use rtwk::style::Style;
use rtwk::text::{Text, TextWrap};
use rtwk::{AnyWidget, Cx};

use codex_tui_core::screens::dialogs::{Prompt, PromptKind, Suggestion};

use super::dialog::{Msg, SmallDialogState};
use super::strings::Strings;
use crate::app::theme::Tokens;

pub(super) fn build(
    prompt: &Prompt,
    s: &Strings,
    state: &SmallDialogState,
    cx: &mut Cx<'_, SmallDialogState>,
) -> Focus {
    let tokens = Tokens::of(cx);
    let PromptKind::Feature(kind) = &prompt.kind;
    let words = crate::features::prompt_words(kind, s);
    let context = match &prompt.suggestion {
        Suggestion::Idle => None,
        Suggestion::Loading => Some(s.prompt_suggestion_loading),
        Suggestion::Applied(_) => Some(s.prompt_suggestion_ready),
    };
    let gap = || SizedBox::new(None, Some(1), None);
    let sides =
        |child: AnyWidget| -> AnyWidget { Padding::new(EdgeInsets::symmetric(2, 0), child).into() };
    let input = TextArea::new()
        .controller(state.text.clone())
        .placeholder(words.placeholder)
        .placeholder_style(tokens.dim())
        .min_lines(1)
        .max_lines(8)
        .focus_node(state.node.clone())
        .autofocus(true)
        .on_changed(cx.map(|_| Msg::Typed))
        .on_submitted(cx.map(Msg::Submit));
    let mut column = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(gap())
        .child(sides(
            Text::new(words.title)
                .style(tokens.bold())
                .wrap(TextWrap::Word)
                .into(),
        ));
    if let Some(context) = context {
        column = column.child(sides(
            Text::new(context)
                .style(tokens.accent())
                .wrap(TextWrap::Word)
                .into(),
        ));
    }
    let column = column
        .child(gap())
        .child(
            Row::new()
                .child(Text::line(s.prompt_mark).style(tokens.accent()))
                .child(SizedBox::new(Some(1), None, None))
                .child(Expanded::new(1, input))
                .child(SizedBox::new(Some(2), None, None)),
        )
        .child(gap())
        .child(Padding::new(
            EdgeInsets::only(2, 0, 0, 0),
            Text::line(s.prompt_hint).style(tokens.dim()),
        ));
    // The text area leaves Esc to the nodes above it.
    Focus::new(Dialog::bare(
        BoxWidget::new().style(Style::new()).child(column),
    ))
    .can_request_focus(false)
    .bind(KeyPress::ESC, cx.send(Msg::Close))
}
