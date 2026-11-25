//! The disclosure of an activity cell (oracle transcript_view/layout.rs
//! `activity_layout`, text.rs `with_control`): the cell's rows, then a
//! control (`+ Show details`, `− Hide details`) a quarter of the width in, at
//! most four columns. Flutter: the content and its `TextButton` (a `Pressable`: tap, Enter and
//! Space), the cell's `State` holding `expanded` as an `ExpansionTile` does.
//!
//! The door to F4: a row of the transcript puts its cell in a `tile`, which says which cell it is
//! and holds the thread's `Activity` above it; a cell reads `expanded` and `focused`, and its
//! control goes through the controller, so a key reaches it as a tap does (a tap puts the focus on
//! the cell, as F4 does). The cell's own flag stays for a cell outside a tile (the sheets, the
//! tests).

use rtwk::controls::Pressable;
use rtwk::foundation::ListenableBuilder;
use rtwk::gestures::{ExcludeSelection, PointerEvent};
use rtwk::layout::{Column, CrossAxisAlignment, EdgeInsets, MainAxisSize, Padding};
use rtwk::provider::Provider;
use rtwk::style::Attrs;
use rtwk::style::Style;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, LayoutBuilder, State};

use codex_tui_core::threads::ItemKey;

use crate::app::theme::Tokens;
use crate::pages::session::transcript::activity_focus::Activity;

/// What a row tells the disclosing cell in it: which item it is, whether the thread has it open
/// and has the focus on it, and the controller that opens and closes it.
#[derive(Clone, PartialEq)]
pub struct Tile {
    key: ItemKey,
    open: bool,
    focused: bool,
    activity: Activity,
}

/// `cell` as the disclosing cell of the item `key`: it builds again when the thread's controller
/// opens or closes it or moves the focus, and reads `expanded` and `focused` from the controller.
pub fn tile(activity: &Activity, key: ItemKey, cell: AnyWidget) -> AnyWidget {
    let activity = activity.clone();
    ListenableBuilder::new(activity.listenable(), move |_, _| {
        Provider::value(Tile {
            key: key.clone(),
            open: activity.disclosed().is_open(&key),
            focused: activity.is_focused(std::slice::from_ref(&key)),
            activity: activity.clone(),
        })
        .child(cell.clone())
        .into()
    })
    .into()
}

/// Whether the cell is open: the thread's answer under a `tile`, else `local`, the cell's own flag.
/// The cell builds again when the answer changes.
pub fn expanded<S>(cx: &mut Cx<'_, S>, local: bool) -> bool {
    Provider::<Tile>::maybe_of(cx, true).map_or(local, |tile| tile.open)
}

/// Whether the thread has the focus on the cell (F4, or a tap on its control): its first row is
/// underlined and its control drawn in the accent (oracle `render_disclosure`). Never outside a
/// tile.
pub fn focused<S>(cx: &mut Cx<'_, S>) -> bool {
    Provider::<Tile>::maybe_of(cx, true).is_some_and(|tile| tile.focused)
}

/// `content` with the control beneath it; a tap on the control flips `expanded` through the
/// cell state's `update` (its message is the new value). Only the control takes the tap (the
/// oracle's `toggle_disclosure_at`: the control's row): the rows above it are the selection's, a
/// click on a line of the output selects it.
pub fn disclosing<S: State<Msg = bool>>(
    cx: &mut Cx<'_, S>,
    content: AnyWidget,
    label: impl Into<String>,
    style: Style,
    expanded: bool,
) -> AnyWidget {
    // under a tile the control opens and closes the cell through the thread's controller, as a
    // key does; the cell reads the answer back with `expanded`
    let control = if let Some(tile) = Provider::<Tile>::maybe_of(cx, false) {
        tile.activity.disclosed().note(&tile.key);
        // in focus the control is the accent, bold and no longer dim
        let style = if tile.focused {
            Tokens::of(cx).accent().attrs(Attrs::BOLD)
        } else {
            style
        };
        let row = control(label.into(), style);
        // a control of the mouse: the composer keeps the keys, and the focus on a cell is the
        // transcript's own (`activity_focus`), whose Enter toggles it
        Pressable::new(move |_, _| row.clone())
            .can_request_focus(false)
            .on_tap(cx.callback(move |_, _: PointerEvent| tile.activity.toggle_cell(&tile.key)))
    } else {
        let row = control(label.into(), style);
        let toggle = cx.send(!expanded);
        Pressable::new(move |_, _| row.clone())
            .bindings([("enter", Some(toggle.clone())), ("space", Some(toggle))])
            .on_tap(cx.map(move |_| !expanded))
    };
    Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(content)
        .child(control)
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
