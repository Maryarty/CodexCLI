<!-- Written by tools/rtwk-port/skill/generate.py from rtwk 9f28b7fb; do not edit. -->

# Scrolling

[All widgets](../widgets.md)

## SingleChildScrollView

One child scrolled in the room it is given: Tab to it, then the arrows, PageUp and PageDown, or the wheel over it.

API:

- `rtwk::scroll::SingleChildScrollView` (struct, `crates/rtwk-scroll/src/single_child_scroll_view.rs:65`) Shows its child in a window that scrolls along `scroll_direction` (a `Scrollable`: keys, the wheel, `controller`).
  - public fields: `scroll_direction: Axis`, `controller: Option<ScrollController>`, `style: Option<Style>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: Option<bool>`, `physics: Option<Rc<dyn ScrollPhysics>>`, `key_hints: bool`, `padding: Option<EdgeInsets>`, `child: Option<AnyWidget>`

### Code

The gallery's Code tab at the initial options (route `#/scrolling/singlechildscrollview`, `crates/rtwk-gallery/src/pages/mod.rs:2326`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
// State retains a ScrollController, the selected paragraph and measured heights.
// The button selects the next paragraph and jumps the least distance that reveals it.
let texts = [
    "A scroll view lays its child out as long as it likes along the axis, and shows a window of it.",
    "The arrows move it a cell, PageUp and PageDown a window, Home and End to the ends.",
    "A notch of the wheel over it scrolls three cells.",
    "Scrolled while all of it is on the screen, it has the terminal move the rows that stay.",
    "A Column in it is as tall as its children, whatever its mainAxisSize.",
    "Its controller reads the offset and moves it; the line under the box listens to it.",
];
let theme = Theme::of(cx);
let paragraphs = texts.into_iter().enumerate().map(|(i, text)| {
    Text::new(text).style(if self.shown == Some(i) { theme.primary.clone() }
        else { Style::new() })
});
let child: AnyWidget = Column::new().cross_axis_alignment(CrossAxisAlignment::Start).spacing(1)
    .children(paragraphs.zip(&self.rows).map(|(text, rows)| MeasuredParagraph {
        child: text.into(), rows: rows.clone(),
    }.into())).into();
let view = SingleChildScrollView::new(child)
    .controller(self.controller.clone()).scroll_direction(Axis::Vertical);
column![
    boxed(34, 8, view), // rounded border follows the preview's focus
    Button::new("next paragraph").on_pressed(cx.send(())),
    offset_line(&self.controller),
].main_axis_size(MainAxisSize::Min).cross_axis_alignment(CrossAxisAlignment::Start).spacing(1)
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| axis | vertical, horizontal | vertical |

### Methods

#### `SingleChildScrollView`: constructors and associated functions

- `pub fn new(child: impl Into<AnyWidget>) -> SingleChildScrollView` - Shows `child` in a window that scrolls vertically.

#### `SingleChildScrollView`: builder methods (take and return `self`)

- `pub fn scroll_direction(mut self, axis: Axis) -> Self` - Along `axis`.
- `pub fn controller(mut self, c: ScrollController) -> Self` - Read and moved by `c`.
- `pub fn style(mut self, style: Style) -> Self` - The window filled with `style`.
- `pub fn focus_node(mut self, node: FocusNode) -> Self` - With the focus node `node`.
- `pub fn autofocus(mut self, on: bool) -> Self` - Focused as it is first built.
- `pub fn can_request_focus(mut self, on: bool) -> Self` - Whether it can take the focus.
- `pub fn physics(mut self, physics: Rc<dyn ScrollPhysics>) -> Self` - With `physics`.
- `pub fn key_hints(mut self, on: bool) -> Self` - Whether its keys carry hints.
- `pub fn padding(mut self, padding: EdgeInsets) -> Self` - With `padding` round the child, which scrolls with it.
- `pub fn child(mut self, child: impl Into<AnyWidget>) -> Self` - Showing `child`.

### Also used in the code

- `rtwk::layout::Axis` (enum, `crates/rtwk-render/src/objects/decoration.rs:39`) Along which axis something runs: across or down.
  - variants: `Horizontal`, `Vertical`
- `rtwk::controls::Button` (struct, `crates/rtwk-controls/src/button.rs:107`) A button that tells `on_pressed` when it is clicked, or when Enter or Space is pressed while it has the focus.
  - public fields: `spans: Option<Rc<[Span]>>`, `tall: bool`, `on_pressed: Option<Handler<()>>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: bool`, `style: Option<Style>`, `focused_style: Option<Style>`, `hover_style: Option<Style>`, `disabled_style: Option<Style>`

## Scrollable

What scrolls a viewport: here one of the page's own, a ruler that tells the scrollable how long it is as it is laid out.

API:

- `rtwk::scroll::Scrollable` (struct, `crates/rtwk-scroll/src/scrollable.rs:1170`) Scrolls the viewport `viewport_builder` builds along `scroll_direction`: the keys while it has the focus, the wheel over it, its `controller` and `Scrollable::ensure_visible`.
  - public fields: `viewport_builder: ViewportBuilder`, `scroll_direction: Axis`, `reverse: bool`, `controller: Option<ScrollController>`, `physics: Option<Rc<dyn ScrollPhysics>>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: Option<bool>`, `key_hints: bool`

### Code

The gallery's Code tab at the initial options (route `#/scrolling/scrollable`, `crates/rtwk-gallery/src/pages/mod.rs:2339`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
// RenderRuler::perform_layout publishes the actual viewport and 100 content rows.
let size = Size::new(constraints.constrain_width(32), constraints.constrain_height(6));
self.position.apply_extents(i64::from(size.h), 100, None, false);

// RenderRuler::paint draws from the controller's visible offset.
for y in 0..height {
    let row = self.offset + i64::from(y) + 1;
    if row > 100 { break; }
    canvas.text(0, y, &format!("{row:>3} "), normal);
    let tick = if row % 10 == 0 { "├─────────" }
        else if row % 5 == 0 { "├────" } else { "├─" };
    canvas.text(4, y, tick, dim);
}

// The viewport builder keeps the native position and redraws when it changes.
let ruler = Scrollable::new(|_, position| {
    let read = position.clone();
    ListenableBuilder::new(Rc::new(position.clone()), move |_, _| Ruler {
        position: read.clone(), offset: read.offset(),
    }.into()).into()
}).controller(self.controller.clone());
column![boxed(34, 8, ruler), offset_line(&self.controller)]
    .main_axis_size(MainAxisSize::Min).cross_axis_alignment(CrossAxisAlignment::Start).spacing(1)
```

### Methods

#### `Scrollable`: constructors and associated functions

- `pub fn new(viewport_builder: impl Fn(&mut BuildCx<'_>, &ScrollPosition) -> AnyWidget + 'static) -> Scrollable` - Scrolls what `viewport_builder` builds, vertically.
- `pub fn of<S>(cx: &mut Cx<'_, S>) -> ScrollPosition` - The position of the nearest scrollable above `cx`; `cx` depends on it.
- `pub fn maybe_of<S>(cx: &mut Cx<'_, S>) -> Option<ScrollPosition>` - The position `of` finds, if there is one.
- `pub fn ensure_visible(tree: &mut WidgetTree, element: Eid)` - Scrolls each scrollable above `element` of `tree`, the nearest first, the least that shows its widget (`ScrollPosition::ensure_visible`).
- `pub fn ensure_visible_rect(tree: &mut WidgetTree, element: Eid, origin: (i32, i32), size: Size)` - Shows a local rectangle in the existing render object of `element`.
- `pub fn ensure_visible_rect_in(cx: &mut ViewportCx<'_>, element: Eid, origin: (i32, i32), size: Size)` - Shows a settled local rectangle during a native viewport operation.
- `pub fn try_ensure_visible_rect_in(cx: &mut ViewportCx<'_>, element: Eid, origin: (i32, i32), size: Size) -> bool` - Requests an immediate nearest-outward visibility operation, after validating the whole chain of viewports.

#### `Scrollable`: builder methods (take and return `self`)

- `pub fn scroll_direction(mut self, axis: Axis) -> Scrollable` - Along `axis`.
- `pub fn reverse(mut self, on: bool) -> Scrollable` - Reversed, or not.
- `pub fn controller(mut self, c: ScrollController) -> Scrollable` - Read and moved by `c`.
- `pub fn physics(mut self, physics: Rc<dyn ScrollPhysics>) -> Scrollable` - With `physics`.
- `pub fn focus_node(mut self, node: FocusNode) -> Scrollable` - With the focus node `node`.
- `pub fn autofocus(mut self, on: bool) -> Scrollable` - Focused as it is first built.
- `pub fn can_request_focus(mut self, on: bool) -> Scrollable` - Whether it can take the focus.
- `pub fn key_hints(mut self, on: bool) -> Scrollable` - Whether its keys carry hints.

### Also used in the code

- `rtwk_render::Size` (struct, `crates/rtwk-render/src/constraints.rs:21`) The size of a render object, in cells.
  - public fields: `w: i32`, `h: i32`
- `rtwk::foundation::ListenableBuilder` (struct, `crates/rtwk-framework/src/foundation/listenable_builder.rs:131`) Shows what its builder returns, built again whenever its listenable notifies; the widgets around it are not (twk's `ListenableBuilder`).
  - public fields: `listenable: Rc<dyn Listenable>`, `builder: TransitionBuilder`, `child: Option<AnyWidget>`

## ListView

Items built as they come into view, however many there are; the wheel and the keys scroll it. Separated, reversed as a chat, or kept on what it shows as items come before it.

API:

- `rtwk::scroll::ListView` (struct, `crates/rtwk-scroll/src/list_view.rs:279`) A list of `item_count` items one below another, each as wide as the list and as tall as it likes, or `item_extent` rows, in a window that scrolls (a `Scrollable`: keys, the wheel, `controller`).
  - public fields: `item_count: usize`, `item_builder: IndexedWidgetBuilder`, `separator_builder: Option<IndexedWidgetBuilder>`, `reverse: bool`, `padding: Option<EdgeInsets>`, `find_item_index_callback: Option<ChildIndexGetter>`, `item_extent: Option<i32>`, `cache_extent: i32`, `max_kept: usize`, `stick_to_bottom: bool`, `controller: Option<ScrollController>`, `style: Option<Style>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: Option<bool>`, `physics: Option<Rc<dyn ScrollPhysics>>`, `key_hints: bool`

### Code

The gallery's Code tab at the initial options (route `#/scrolling/listview`, `crates/rtwk-gallery/src/pages/mod.rs:2346`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
// State owns ListController::with(0, false).
// Start, Ensure(index) and Jump(index) call jump_to(0),
// ensure_index_visible(index) and jump_to_index(index).
let count = 100;
let theme = Theme::of(cx);
let muted = theme.muted.clone();
let border = theme.border.clone();
let item = |i| Text::new(format!("item {}", i + 1)).into();
let list = ListView::builder(count, item)
    .controller(self.controller.clone())
    .reverse(false).stick_to_bottom(false);
let buttons = row![
    Button::new("top").on_pressed(cx.send(ListMsg::Start)),
    Button::new("item 40").on_pressed(cx.send(ListMsg::Ensure(39))),
    Button::new("middle").on_pressed(cx.send(ListMsg::Jump(count / 2))),
].main_axis_size(MainAxisSize::Min).spacing(1);
column![boxed(34, 8, list), buttons, offset_line(self.controller.as_scroll_controller())]
    .main_axis_size(MainAxisSize::Min).cross_axis_alignment(CrossAxisAlignment::Start).spacing(1)
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| items | 100, 10000, 1000000, growing, chat, feed | 100 |
| heights | one row, different | one row |
| separated | on, off | false |
| stick | stickToBottom, stickToEnd | stickToBottom |

### Methods

#### `ListView`: constructors and associated functions

- `pub fn new(children: Vec<AnyWidget>) -> ListView` - A list of `children`, for a short list: the widgets are made at once, by the caller, and built as they come into view.
- `pub fn builder(item_count: usize, item_builder: impl Fn(usize) -> AnyWidget + 'static) -> Self` - A list of `item_count` items that `item_builder` builds as they come into view.
- `pub fn separated(item_count: usize, item_builder: impl Fn(usize) -> AnyWidget + 'static, separator_builder: impl Fn(usize) -> AnyWidget + 'static) -> Self` - A list whose items `item_builder` builds, with a separator that `separator_builder` builds between each two: separator `i` between item `i` and item `i + 1`, built as it comes into view, as the items are.
- `pub fn list_object(tree: &WidgetTree, element: Eid) -> Option<RenderId>` - The `RenderList` of the list view whose element is `element` in `tree` (the first list below the element's render object): what a test or a tool reads a list's window from.

#### `ListView`: builder methods (take and return `self`)

- `pub fn find_item_index_callback(mut self, f: impl Fn(&Key) -> Option<usize> + 'static) -> ListView` - Finding where an item's key went with `f` (twk's `findChildIndexCallback`).
- `pub fn find_index(self, f: impl Fn(&Key) -> Option<usize> + 'static) -> ListView` - The same as `find_item_index_callback`.
- `pub fn item_extent(mut self, rows: i32) -> ListView` - Each item `rows` tall.
- `pub fn cache_extent(mut self, rows: i32) -> ListView` - Building `rows` past each edge of the window.
- `pub fn max_kept(mut self, n: usize) -> ListView` - Keeping at most `n` keyed items out of view.
- `pub fn stick_to_bottom(mut self, on: bool) -> ListView` - Sticking to its end, or not.
- `pub fn reverse(mut self, on: bool) -> ListView` - Item 0 at the bottom, or not.
- `pub fn padding(mut self, padding: EdgeInsets) -> ListView` - With `padding` around the items, which scrolls with them.
- `pub fn controller(mut self, c: impl Into<ScrollController>) -> ListView` - Read and moved by `c` (a `ListController` too, `.into()` or as itself).
- `pub fn style(mut self, style: Style) -> ListView` - The window filled with `style`.
- `pub fn focus_node(mut self, node: FocusNode) -> ListView` - With the focus node `node`.
- `pub fn autofocus(mut self, on: bool) -> ListView` - Focused as it is first built.
- `pub fn can_request_focus(mut self, on: bool) -> ListView` - Whether it can take the focus.
- `pub fn physics(mut self, physics: Rc<dyn ScrollPhysics>) -> ListView` - With `physics`.
- `pub fn key_hints(mut self, on: bool) -> ListView` - Whether its keys carry hints.

### Also used in the code

- `rtwk::scroll::ListController` (struct, `crates/rtwk-scroll/src/list_view.rs:89`) A `ScrollController` that also moves its `ListView`s to an item, built or not.
- `rtwk::controls::Button` (struct, `crates/rtwk-controls/src/button.rs:107`) A button that tells `on_pressed` when it is clicked, or when Enter or Space is pressed while it has the focus.
  - public fields: `spans: Option<Rc<[Span]>>`, `tall: bool`, `on_pressed: Option<Handler<()>>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: bool`, `style: Option<Style>`, `focused_style: Option<Style>`, `hover_style: Option<Style>`, `disabled_style: Option<Style>`

## LogView

Lines that keep coming, followed at the bottom: levels, a filter and a search, a count of the new lines while it is scrolled up.

API:

- `rtwk::scroll::LogView` (struct, `crates/rtwk-scroll/src/log_view/view.rs:93`) The lines of a `LogController` in a window that scrolls, the newest at the bottom: `tail -f` for an app (twk's `LogView`).
  - public fields: `controller: LogController`, `wrap: bool`, `show_time: bool`, `show_level: bool`, `selectable: bool`, `style: Option<Style>`, `match_style: Option<Style>`, `current_match_style: Option<Style>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `scrollbar: ScrollbarVisibility`

### Code

The gallery's Code tab at the initial options (route `#/scrolling/logview`, `crates/rtwk-gallery/src/pages/mod.rs:2389`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
// State owns LogController and appends dated entries with add_with.
// The 400 ms ticker appends the next log_entry(i) while the preview is visible.
self.controller.set_level(LogLevel::Debug);
self.controller.set_filter("");
self.controller.set_query("");

let controller = self.controller.clone();
LayoutBuilder::new(move |_, constraints| {
    let view = LogView::new(&controller)
        .show_time(true).show_level(true)
        .scrollbar(ScrollbarVisibility::Always);
    boxed(50, constraints.max_height.clamp(6, 12), view).into()
})
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| level | debug, info, warning, error | debug |
| filter | none, GET, slow | none |
| query | none, items, timeout | none |
| time | on, off | true |
| showLevel | on, off | true |
| wrap | on, off | true |
| live | on, off | true |
| scrollbar | always, onScroll, never | always |

### Methods

#### `LogView`: constructors and associated functions

- `pub fn new(controller: impl Into<LogController>) -> LogView` - Shows the lines of `controller`, each with its time and level, wrapped unless `wrap` is false, selectable with the mouse, with a scrollbar always shown.
- `pub fn render_visits() -> usize` - How many render objects the views of this thread looked at to find their list and the lines in their windows (twk's `LogView.renderVisits`).
- `pub fn set_render_visits(n: usize)` - Sets `render_visits`.
- `pub fn selection_lines() -> usize` - How many lines the selections of this thread wrote into the text they select in; counted in a debug build only (twk's `LogView.selectionLines`, counted under `assert`).
- `pub fn set_selection_lines(n: usize)` - Sets `selection_lines`.

#### `LogView`: builder methods (take and return `self`)

- `pub fn wrap(mut self, on: bool) -> LogView` - Whether a line too long for the window goes on below (`true` by default) or is cut off, Left and Right scrolling the lines across.
- `pub fn show_time(mut self, on: bool) -> LogView` - A line shows its time, when it has one.
- `pub fn show_level(mut self, on: bool) -> LogView` - A line shows its level.
- `pub fn selectable(mut self, on: bool) -> LogView` - The lines can be selected with the mouse and copied.
- `pub fn style(mut self, style: Style) -> LogView` - Filling the window under the lines with `style`.
- `pub fn match_style(mut self, style: Style) -> LogView` - What the query finds drawn in `style` merged onto the line.
- `pub fn current_match_style(mut self, style: Style) -> LogView` - What the query finds on the current match's line drawn in `style`.
- `pub fn focus_node(mut self, node: FocusNode) -> LogView` - Focused by `node`.
- `pub fn autofocus(mut self, on: bool) -> LogView` - Focused as it is built.
- `pub fn scrollbar(mut self, v: ScrollbarVisibility) -> LogView` - The scrollbar shown as `v` says.

### Also used in the code

- `rtwk_core::log::LogLevel` (enum, `crates/rtwk-core/src/log.rs:28`) How much a `LogRecord` matters, from the least to the most.
  - variants: `Debug`, `Info`, `Warning`, `Error`
- `rtwk::LayoutBuilder` (struct, `crates/rtwk-framework/src/layout_builder.rs:56`) Builds its child with the constraints it is laid out within, so that what it shows depends on the room it gets: one layout for a wide terminal and another for a narrow one.
- `rtwk::scroll::ScrollbarVisibility` (enum, `crates/rtwk-scroll/src/scrollbar.rs:43`) When a `Scrollbar` shows its thumb.
  - variants: `Always`, `OnScroll`, `Never`

## Scrollbar

A bar by the scrollable below it, its thumb where the window is; the thumb dragged and the track clicked scroll it.

API:

- `rtwk::scroll::Scrollbar` (struct, `crates/rtwk-scroll/src/scrollbar.rs:145`) Shows how far the outermost `Scrollable` inside its child is scrolled, as a thumb on a track beside it, and scrolls it with the mouse.
  - public fields: `child: AnyWidget`, `axis: Option<Axis>`, `visibility: ScrollbarVisibility`, `time_to_fade: Duration`, `style: Option<Style>`, `slim: bool`, `track_style: Option<Style>`

### Code

The gallery's Code tab at the initial options (route `#/scrolling/scrollbar`, `crates/rtwk-gallery/src/pages/mod.rs:2460`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
let list = ListView::builder(200, |i| Text::new(format!("row {}", i + 1)).wrap(TextWrap::None).into());
let bar = |axis, child: AnyWidget| -> AnyWidget {
    Scrollbar::new(child)
        .axis(axis).visibility(ScrollbarVisibility::Always)
        .slim(false).into()
};
let child = list.into();
boxed(34, 10, bar(Axis::Vertical, child))
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| visibility | always, onScroll, never | always |
| axes | vertical, horizontal, both | vertical |
| color | muted, border, primary, accent | muted |
| slim | on, off | false |
| track | faint, border, muted | faint |

### Methods

#### `Scrollbar`: constructors and associated functions

- `pub fn new(child: impl Into<AnyWidget>) -> Scrollbar` - A scrollbar for the scrollable in `child`, along its axis; its thumb always shows.

#### `Scrollbar`: builder methods (take and return `self`)

- `pub fn axis(mut self, axis: Axis) -> Scrollbar` - For the scrollable along `axis`.
- `pub fn visibility(mut self, v: ScrollbarVisibility) -> Scrollbar` - Showing its thumb as `v` says.
- `pub fn time_to_fade(mut self, d: Duration) -> Scrollbar` - Hiding the thumb `d` after the last move.
- `pub fn style(mut self, style: Style) -> Scrollbar` - In `style`.
- `pub fn slim(mut self, on: bool) -> Scrollbar` - Slim, or not.
- `pub fn track_style(mut self, style: Style) -> Scrollbar` - The slim track's line in `style`.

### Also used in the code

- `rtwk::scroll::ListView` (struct, `crates/rtwk-scroll/src/list_view.rs:279`) A list of `item_count` items one below another, each as wide as the list and as tall as it likes, or `item_extent` rows, in a window that scrolls (a `Scrollable`: keys, the wheel, `controller`).
  - public fields: `item_count: usize`, `item_builder: IndexedWidgetBuilder`, `separator_builder: Option<IndexedWidgetBuilder>`, `reverse: bool`, `padding: Option<EdgeInsets>`, `find_item_index_callback: Option<ChildIndexGetter>`, `item_extent: Option<i32>`, `cache_extent: i32`, `max_kept: usize`, `stick_to_bottom: bool`, `controller: Option<ScrollController>`, `style: Option<Style>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: Option<bool>`, `physics: Option<Rc<dyn ScrollPhysics>>`, `key_hints: bool`
- `rtwk::text::TextWrap` (enum, `crates/rtwk-render/src/objects/text.rs:55`) Where a `RenderParagraph` may break a line too long for it.
  - variants: `Word`, `Grapheme`, `None`
- `rtwk::scroll::ScrollbarVisibility` (enum, `crates/rtwk-scroll/src/scrollbar.rs:43`) When a `Scrollbar` shows its thumb.
  - variants: `Always`, `OnScroll`, `Never`
- `rtwk::layout::Axis` (enum, `crates/rtwk-render/src/objects/decoration.rs:39`) Along which axis something runs: across or down.
  - variants: `Horizontal`, `Vertical`
