//! The disclosure of an activity cell (oracle transcript_view/layout.rs
//! `activity_layout`, text.rs `with_control`): the cell's rows, then a
//! control (`+ Show details`, `− Hide details`) a quarter of the width in, at
//! most four columns. Flutter: the content and its `TextButton` in one
//! `Pressable` (tap, Enter and Space), the cell's `State` holding `expanded`
//! as an `ExpansionTile` does.

use rtwk::controls::Pressable;
use rtwk::gestures::ExcludeSelection;
use rtwk::layout::{Column, CrossAxisAlignment, EdgeInsets, MainAxisSize, Padding};
use rtwk::style::Style;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, LayoutBuilder, State};

/// `content` with the control beneath it; a tap flips `expanded` through the
/// cell state's `update` (its message is the new value).
pub fn disclosing<S: State<Msg = bool>>(
    cx: &mut Cx<'_, S>,
    content: AnyWidget,
    label: impl Into<String>,
    style: Style,
    expanded: bool,
) -> AnyWidget {
    let content: AnyWidget = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(content)
        .child(control(label.into(), style))
        .into();
    let toggle = cx.send(!expanded);
    Pressable::new(move |_, _| content.clone())
        .bindings([("enter", Some(toggle.clone())), ("space", Some(toggle))])
        .on_tap(cx.map(move |_| !expanded))
        .into()
}

/// The control's row: indented a quarter of the width, at most four columns.
fn control(label: String, style: Style) -> AnyWidget {
    LayoutBuilder::new(move |_, room| {
        let indent = (room.max_width / 4).min(4);
        Padding::new(
            EdgeInsets::only(indent, 0, 0, 0),
            ExcludeSelection::new(Text::new(label.clone()).style(style.clone())),
        )
        .into()
    })
    .into()
}
