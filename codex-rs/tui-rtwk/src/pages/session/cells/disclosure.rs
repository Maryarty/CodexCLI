//! The disclosure of an activity cell (oracle transcript_view/layout.rs
//! `activity_layout`, text.rs `with_control`): the cell's rows, then a
//! control (`+ Show details`, `− Hide details`) a quarter of the width in, at
//! most four columns. Flutter: the content and its `TextButton` (a `Pressable`: tap, Enter and
//! Space), the cell's `State` holding `expanded` as an `ExpansionTile` does.
//!
//! The door to F4: a row of the transcript puts its cell in a `tile`, which says which cell it is
//! and holds the thread's `Disclosed` controller above it; a cell reads `expanded` and its control
//! goes through the controller, so a key reaches it as a tap does. The cell's own flag stays for a
//! cell outside a tile (the sheets, the tests).

use rtwk::controls::Pressable;
use rtwk::foundation::ListenableBuilder;
use rtwk::gestures::{ExcludeSelection, PointerEvent};
use rtwk::layout::{Column, CrossAxisAlignment, EdgeInsets, MainAxisSize, Padding};
use rtwk::provider::Provider;
use rtwk::style::Style;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, LayoutBuilder, State};

use codex_tui_core::threads::ItemKey;

use crate::widgets::thread_ui::Disclosed;

/// What a row tells the disclosing cell in it: which item it is, whether the thread has it open,
/// and the controller that opens and closes it.
#[derive(Clone, PartialEq)]
pub struct Tile {
    key: ItemKey,
    open: bool,
    disclosed: Disclosed,
}

/// `cell` as the disclosing cell of the item `key`: it builds again when the thread's controller
/// opens or closes it, and reads `expanded` from the controller.
pub fn tile(disclosed: &Disclosed, key: ItemKey, cell: AnyWidget) -> AnyWidget {
    let disclosed = disclosed.clone();
    ListenableBuilder::new(disclosed.listenable(), move |_, _| {
        Provider::value(Tile {
            key: key.clone(),
            open: disclosed.is_open(&key),
            disclosed: disclosed.clone(),
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
    let row = control(label.into(), style);
    // under a tile the control opens and closes the cell through the thread's controller, as a
    // key does; the cell reads the answer back with `expanded`
    let control = if let Some(tile) = Provider::<Tile>::maybe_of(cx, false) {
        tile.disclosed.note(&tile.key);
        let flip = move || {
            tile.disclosed.toggle(&tile.key);
        };
        let (on_key, on_tap) = (flip.clone(), flip);
        let toggle = cx.callback(move |_, ()| on_key());
        Pressable::new(move |_, _| row.clone())
            .bindings([("enter", Some(toggle.clone())), ("space", Some(toggle))])
            .on_tap(cx.callback(move |_, _: PointerEvent| on_tap()))
    } else {
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
