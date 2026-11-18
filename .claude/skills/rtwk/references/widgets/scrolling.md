<!-- Written by `cargo xtask skill-references` from docs/widgets; do not edit. -->
# Scrolling

Lists, scroll views and scrollbars, built as they scroll into view.

[All widgets](../widgets.md)

A Code is the body of the gallery's preview: it names widgets by the last segment of the paths under API (`use` them) and may use `cx` or values of the gallery's page.

## SingleChildScrollView

One child scrolled in the room it is given: Tab to it, then the arrows, PageUp and PageDown, or the wheel over it.

- `rtwk::scroll::SingleChildScrollView` (struct) in `crates/rtwk-scroll/src/single_child_scroll_view.rs:65`: Shows its child in a window that scrolls along scroll_direction (a Scrollable: keys, the wheel, controller).

### Code

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

| option | values | starts at |
| --- | --- | --- |
| direction | vertical, horizontal | vertical |

## Scrollable

What scrolls a viewport: here one of the page's own, a ruler that tells the scrollable how long it is as it is laid out.

- `rtwk::scroll::Scrollable` (struct) in `crates/rtwk-scroll/src/scrollable.rs:1170`: Scrolls the viewport viewport_builder builds along scroll_direction: the keys while it has the focus, the wheel over it, its controller and Scrollable::ensure_visible.

### Code

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

## ListView

Items built as they come into view, however many there are; the wheel and the keys scroll it. Separated, reversed as a chat, or kept on what it shows as items come before it.

- `rtwk::scroll::ListView` (struct) in `crates/rtwk-scroll/src/list_view.rs:279`: A list of item_count items one below another, each as wide as the list and as tall as it likes, or item_extent rows, in a window that scrolls (a Scrollable: keys, the wheel, controller).

### Code

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

| option | values | starts at |
| --- | --- | --- |
| items | 100, 10 000, 1 000 000, growing, chat, feed | 100 |
| heights | one row, different | one row |
| separated | false, true | false |
| stuck by | stickToBottom, stickToEnd | stickToBottom |

## LogView

Lines that keep coming, followed at the bottom: levels, a filter and a search, a count of the new lines while it is scrolled up.

- `rtwk::scroll::LogView` (struct) in `crates/rtwk-scroll/src/log_view/view.rs:93`: The lines of a LogController in a window that scrolls, the newest at the bottom: `tail -f` for an app (twk's `LogView`).

### Code

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

| option | values | starts at |
| --- | --- | --- |
| level | debug, info, warning, error | debug |
| filter | none, GET, slow | none |
| search | none, items, timeout | none |
| time | false, true | true |
| show level | false, true | true |
| wrap | false, true | true |
| live | false, true | true |
| scrollbar | always, onScroll, never | always |

## Scrollbar

A bar by the scrollable below it, its thumb where the window is; the thumb dragged and the track clicked scroll it.

- `rtwk::scroll::Scrollbar` (struct) in `crates/rtwk-scroll/src/scrollbar.rs:145`: Shows how far the outermost Scrollable inside its child is scrolled, as a thumb on a track beside it, and scrolls it with the mouse.

### Code

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

| option | values | starts at |
| --- | --- | --- |
| visibility | always, onScroll, never | always |
| bars | vertical, horizontal, both | vertical |
| style | muted, border, primary, accent | muted |
| slim | false, true | false |
| track | faint, border, muted | faint |
