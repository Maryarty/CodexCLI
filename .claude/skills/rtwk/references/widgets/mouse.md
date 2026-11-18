<!-- Written by tools/rtwk-port/skill/generate.py from rtwk 9f28b7fb; do not edit. -->

# Mouse

[All widgets](../widgets.md)

## GestureDetector

Taps, double taps, long presses and drags of the mouse over a widget.

API:

- `rtwk::gestures::GestureDetector` (struct, `crates/rtwk-widgets/src/gestures/gesture_detector.rs:91`) Tells the gestures of the presses over its child: taps, double taps, long presses and drags.
  - public fields: `on_tap: Option<Handler<PointerEvent>>`, `on_tap_up: Option<Handler<PointerEvent>>`, `on_tap_down: Option<Handler<PointerEvent>>`, `tap_at: Option<TapAt>`, `cursor: Option<PointerShape>`, `on_double_tap: Option<Handler<PointerEvent>>`, `on_long_press: Option<Handler<PointerEvent>>`, `on_long_press_end: Option<Handler<PointerEvent>>`, `on_pan_start: Option<Handler<DragDetails>>`, `on_pan_update: Option<Handler<DragDetails>>`, `on_pan_end: Option<Handler<DragDetails>>`, `buttons: Buttons`, `double_tap_interval: Duration`, `long_press_duration: Duration`, `behavior: HitTestBehavior`, `child: Option<AnyWidget>`

### Code

The gallery's Code tab at the initial options (route `#/mouse/gesturedetector`, `crates/rtwk-gallery/src/pages/mouse.rs:55`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
let double = true;
let long = Duration::from_millis(500);
let buttons = Buttons::of(&[0]);

{
    let mut marks = Stack::new().child(Center::new(self.last.as_ref().map_or_else(
        || Text::new("tap, hold or drag me").style(theme.muted.clone()),
        |text| Text::new(text.clone()),
    )));
    for (mark, point, style) in [
        ("○", self.from, &theme.muted),
        ("●", self.at, &theme.primary),
    ] {
        if let Some((x, y)) = point {
            marks = marks.child(
                Positioned::new(Text::new(mark).style(style.clone()))
                    .left(x - 1)
                    .top(y - 1),
            );
        }
    }
    let mut gestures = GestureDetector::new()
        .buttons(buttons)
        .on_tap(cx.map(GestureMsg::Tap))
        .on_long_press(cx.map(GestureMsg::Hold))
        .long_press_duration(long)
        .on_pan_start(cx.map(GestureMsg::Start))
        .on_pan_update(cx.map(GestureMsg::Move))
        .on_pan_end(cx.map(GestureMsg::End));
    if double {
        gestures = gestures.on_double_tap(cx.map(GestureMsg::Double));
    }
    column![
        gestures.child(
            SizedBox::fixed(PAD_WIDTH, PAD_HEIGHT).child(
                BoxWidget::new()
                    .border(rounded(theme.border.clone()))
                    .child(BoxWidget::new().style(theme.surface.clone()).child(marks)),
            ),
        ),
        column![
            row![
                LitLine::new(
                    vec![("taps ".into(), true), (self.taps.to_string(), false)],
                    self.taps
                ),
                LitLine::new(
                    vec![
                        (", double ".into(), true),
                        (self.doubles.to_string(), false)
                    ],
                    self.doubles
                ),
                LitLine::new(
                    vec![(", long ".into(), true), (self.holds.to_string(), false)],
                    self.holds
                ),
            ]
            .main_axis_size(MainAxisSize::Min),
            LitLine::new(
                vec![
                    ("dragged ".into(), true),
                    (
                        format!("{} across, {} down", self.delta.0, self.delta.1),
                        false
                    )
                ],
                self.drags,
            ),
        ]
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start),
    ]
    .main_axis_size(MainAxisSize::Min)
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .spacing(1)
    .into()
}
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| double | on, off | true |
| long | 500, 1000 | 500 |
| buttons | left, right, any | left |

### Methods

#### `GestureDetector`: constructors and associated functions

- `pub fn new() -> GestureDetector` - A detector of no gesture over nothing, hit where its child is, of the left button, twk's intervals (300 ms for a double tap, 500 ms for a long press).

#### `GestureDetector`: builder methods (take and return `self`)

- `pub fn on_tap(mut self, h: Handler<PointerEvent>) -> Self` - Telling `h` of a tap.
- `pub fn on_tap_up(mut self, h: Handler<PointerEvent>) -> Self` - Telling `h` of the release that ends a tap, before `on_tap` is told.
- `pub fn on_tap_down(mut self, h: Handler<PointerEvent>) -> Self` - Telling `h` of the left button going down on a cell of a tap.
- `pub fn tap_at(mut self, f: impl Fn(i32, i32) -> bool + 'static) -> Self` - With the cells of a tap those `f` says yes to.
- `pub fn cursor(mut self, shape: PointerShape) -> Self` - With the pointer in `shape`.
- `pub fn on_double_tap(mut self, h: Handler<PointerEvent>) -> Self` - Telling `h` of a double tap.
- `pub fn on_long_press(mut self, h: Handler<PointerEvent>) -> Self` - Telling `h` of a long press.
- `pub fn on_long_press_end(mut self, h: Handler<PointerEvent>) -> Self` - Telling `h` of the release of a long press.
- `pub fn on_pan_start(mut self, h: Handler<DragDetails>) -> Self` - Telling `h` of the start of a drag.
- `pub fn on_pan_update(mut self, h: Handler<DragDetails>) -> Self` - Telling `h` of each cell a drag moves to.
- `pub fn on_pan_end(mut self, h: Handler<DragDetails>) -> Self` - Telling `h` of the end of a drag.
- `pub fn buttons(mut self, buttons: Buttons) -> Self` - Taking the presses of `buttons`.
- `pub fn double_tap_interval(mut self, d: Duration) -> Self` - Waiting `d` for the second tap of a double tap.
- `pub fn long_press_duration(mut self, d: Duration) -> Self` - Holding `d` for a long press.
- `pub fn behavior(mut self, behavior: HitTestBehavior) -> Self` - Hit as `behavior` says.
- `pub fn child(mut self, child: impl Into<AnyWidget>) -> Self` - Over `child`.

### Also used in the code

- `rtwk::gestures::Buttons` (struct, `crates/rtwk-widgets/src/gestures/detector.rs:103`) A set of mouse buttons: 0 left, 1 middle, 2 right, 8 to 11 the side buttons.
- `rtwk::layout::Stack` (struct, `crates/rtwk-widgets/src/layout/stack.rs:36`) Its children on top of each other, a later one over an earlier one (twk's `Stack`, over a `RenderStack`).
  - public fields: `alignment: Alignment`, `fit: StackFit`
- `rtwk::layout::Center` (struct, `crates/rtwk-widgets/src/layout/basic.rs:191`) An `Align` that puts its child in the middle, the odd cell after it (twk's `Center`).
  - public fields: `width_factor: Option<i32>`, `height_factor: Option<i32>`, `child: Option<AnyWidget>`
- `rtwk::layout::Positioned` (struct, `crates/rtwk-widgets/src/layout/stack.rs:107`) Pins its child, in a `Stack`, to the stack's edges (twk's `Positioned`): along an axis with both edges set, exactly as long as the cells between them; with its size set, that long; otherwise as long as it wants.
  - public fields: `left: Option<i32>`, `top: Option<i32>`, `right: Option<i32>`, `bottom: Option<i32>`, `width: Option<i32>`, `height: Option<i32>`, `child: AnyWidget`

## Listener, MouseRegion

The mouse events over a widget as they come: presses, releases, motion and the wheel; the mouse coming in and going out, the pointer a hand over it where the terminal shows that.

API:

- `rtwk::gestures::Listener` (struct, `crates/rtwk-widgets/src/gestures/mouse.rs:74`) Gives the mouse events over its child to its callbacks, each with the cell in the coordinates of this widget.
  - public fields: `on_pointer_down: PointerHandler`, `on_pointer_up: PointerHandler`, `on_pointer_hover: PointerHandler`, `on_pointer_move: PointerHandler`, `on_scroll: PointerHandler`, `scroll_axis: Option<Axis>`, `behavior: HitTestBehavior`, `child: Option<AnyWidget>`
- `rtwk::gestures::MouseRegion` (struct, `crates/rtwk-widgets/src/gestures/mouse.rs:270`) Tells when the mouse comes in over its child (`on_enter`), moves over it with no button held (`on_hover`) and goes out (`on_exit`), and sets the pointer's shape over it (`cursor`).
  - public fields: `on_enter: PointerHandler`, `on_exit: PointerHandler`, `on_hover: PointerHandler`, `cursor: Option<PointerShape>`, `cursor_at: Option<CursorAt>`, `cursor_asks_motion: bool`, `behavior: HitTestBehavior`, `child: Option<AnyWidget>`

### Code

The gallery's Code tab at the initial options (route `#/mouse/listener-mouseregion`, `crates/rtwk-gallery/src/pages/mouse.rs:95`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
let cursor = Some(PointerShape::Pointer);

{
    let mut region = MouseRegion::new()
        .on_enter(cx.map(EventMsg::Enter))
        .on_exit(cx.map(EventMsg::Exit));
    if let Some(cursor) = cursor {
        region = region.cursor(cursor);
    }
    let (surface, edge, label) = if self.over {
        (&theme.selection, &theme.primary, "the mouse is over me")
    } else {
        (&theme.surface, &theme.border, "move over me")
    };
    let pad = Listener::new()
        .on_pointer_down(cx.map(EventMsg::Down))
        .on_pointer_up(cx.map(|event| EventMsg::Other("release", event)))
        .on_pointer_hover(cx.map(|event| EventMsg::Other("move", event)))
        .on_pointer_move(cx.map(|event| EventMsg::Other("drag", event)))
        .on_scroll(cx.map(EventMsg::Scroll))
        .behavior(HitTestBehavior::Opaque)
        .child(
            SizedBox::fixed(PAD_WIDTH, PAD_HEIGHT).child(
                BoxWidget::new().border(rounded(edge.clone())).child(
                    BoxWidget::new()
                        .style(surface.clone())
                        .child(Center::new(Text::new(label))),
                ),
            ),
        );
    let mut events = column![]
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start);
    if self.events.is_empty() {
        events = events.child(Text::new("nothing yet").style(theme.muted.clone()));
    }
    for (index, event) in self.events.iter().enumerate() {
        events = if index == 0 {
            events.child(LitLine::new(vec![(event.clone(), false)], self.serial))
        } else {
            events.child(Text::new(event.clone()).style(theme.muted.clone()))
        };
    }
    column![region.child(pad), events]
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .spacing(1)
        .into()
}
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| cursor | pointer, crosshair, text, grab, move, notAllowed, wait, none | pointer |

### Methods

#### `Listener`: constructors and associated functions

- `pub fn new() -> Listener` - A listener with no callbacks, hit where its child is.

#### `Listener`: builder methods (take and return `self`)

- `pub fn on_pointer_down(mut self, h: Handler<PointerEvent>) -> Listener` - Telling `h` of a press.
- `pub fn on_pointer_up(mut self, h: Handler<PointerEvent>) -> Listener` - Telling `h` of a release.
- `pub fn on_pointer_hover(mut self, h: Handler<PointerEvent>) -> Listener` - Telling `h` of motion with no button held.
- `pub fn on_pointer_move(mut self, h: Handler<PointerEvent>) -> Listener` - Telling `h` of motion with a button held.
- `pub fn on_scroll(mut self, h: Handler<PointerEvent>) -> Listener` - Telling `h` of the wheel.
- `pub fn scroll_axis(mut self, axis: Axis) -> Listener` - For the wheel along `axis` only.
- `pub fn behavior(mut self, behavior: HitTestBehavior) -> Listener` - Hit as `behavior` says.
- `pub fn child(mut self, child: impl Into<AnyWidget>) -> Listener` - Over `child`.

#### `MouseRegion`: constructors and associated functions

- `pub fn new() -> MouseRegion` - A region with no callbacks and no cursor, hit anywhere in its bounds.

#### `MouseRegion`: builder methods (take and return `self`)

- `pub fn on_enter(mut self, h: Handler<PointerEvent>) -> MouseRegion` - Telling `h` the mouse came in.
- `pub fn on_exit(mut self, h: Handler<PointerEvent>) -> MouseRegion` - Telling `h` the mouse went out.
- `pub fn on_hover(mut self, h: Handler<PointerEvent>) -> MouseRegion` - Telling `h` the mouse moved inside.
- `pub fn cursor(mut self, shape: PointerShape) -> MouseRegion` - With the pointer in `shape` while the mouse is in it.
- `pub fn cursor_at(mut self, f: impl Fn(i32, i32) -> Option<PointerShape> + 'static) -> MouseRegion` - With the pointer in the shape `f` gives at each cell of it.
- `pub fn cursor_asks_motion(mut self, asks: bool) -> MouseRegion` - Whether a cursor has the terminal report all motion.
- `pub fn behavior(mut self, behavior: HitTestBehavior) -> MouseRegion` - Hit as `behavior` says.
- `pub fn child(mut self, child: impl Into<AnyWidget>) -> MouseRegion` - Over `child`.

### Also used in the code

- `rtwk::gestures::PointerShape` (enum, `crates/rtwk-core/src/terminal/sequences.rs:334`) The shapes of the mouse pointer OSC 22 sets (`pointer_shape_sequence`), by the names of CSS's `cursor` (`css`), as kitty's protocol names them, or by the X cursor font's (`x11`), as xterm and iTerm2 do.
  - variants: `Alias`, `Cell`, `Copy`, `Crosshair`, `Arrow`, `EResize`, `EwResize`, `Grab`, `Grabbing`, `Help`, `Move`, `NResize`, `NeResize`, `NeswResize`, `NoDrop`, `NotAllowed`, `NsResize`, `NwResize`, `NwseResize`, `Pointer`, `Progress`, `SResize`, `SeResize`, `SwResize`, `Text`, `VerticalText`, `WResize`, `Wait`, `ZoomIn`, `ZoomOut`
- `rtwk_render::HitTestBehavior` (enum, `crates/rtwk-render/src/object.rs:49`) What a hit test does with a render object in whose bounds the cell is (`RenderTree::hit_test`).
  - variants: `DeferToChild`, `Opaque`, `Translucent`
- `rtwk::layout::Center` (struct, `crates/rtwk-widgets/src/layout/basic.rs:191`) An `Align` that puts its child in the middle, the odd cell after it (twk's `Center`).
  - public fields: `width_factor: Option<i32>`, `height_factor: Option<i32>`, `child: Option<AnyWidget>`

## IgnorePointer, AbsorbPointer

A subtree the mouse passes by: IgnorePointer lets it through to what is under it, AbsorbPointer stops it there.

API:

- `rtwk::gestures::IgnorePointer` (struct, `crates/rtwk-widgets/src/gestures/ignore.rs:37`) Shows its child, which the mouse does not reach while `ignoring`, going to what is under it instead.
  - public fields: `ignoring: bool`, `child: Option<AnyWidget>`
- `rtwk::gestures::AbsorbPointer` (struct, `crates/rtwk-widgets/src/gestures/ignore.rs:119`) Shows its child, which takes the mouse while `absorbing` and gives it to nothing in it nor under it.
  - public fields: `absorbing: bool`, `child: Option<AnyWidget>`

### Code

The gallery's Code tab at the initial options (route `#/mouse/ignorepointer-absorbpointer`, `crates/rtwk-gallery/src/pages/mouse.rs:110`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
{
    let edge = if self.over {
        &theme.primary
    } else {
        &theme.border
    };
    let button = MouseRegion::new()
        .on_enter(cx.map(|_| GateMsg::Hover(true)))
        .on_exit(cx.map(|_| GateMsg::Hover(false)))
        .child(
            GestureDetector::new()
                .on_tap(cx.map(|_| GateMsg::Hear("the button")))
                .child(
                    BoxWidget::new()
                        .border(rounded(edge.clone()))
                        .padding(EdgeInsets::x(2))
                        .child(Text::new("click me")),
                ),
        );
    let foreground: AnyWidget = IgnorePointer::new().child(button).into();
    column![
        Stack::new()
            .child(
                GestureDetector::new()
                    .behavior(HitTestBehavior::Opaque)
                    .on_tap(cx.map(|_| GateMsg::Hear("the backdrop")))
                    .child(
                        SizedBox::fixed(PAD_WIDTH, PAD_HEIGHT).child(
                            BoxWidget::new()
                                .border(rounded(theme.border.clone()))
                                .title(BorderTitle::new(" backdrop ").style(theme.muted.clone())),
                        )
                    )
            )
            .child(Positioned::fill(Center::new(foreground))),
        LitLine::new(
            vec![
                ("heard: ".into(), true),
                (self.heard.unwrap_or("nothing").into(), self.heard.is_none()),
            ],
            self.serial,
        )
    ]
    .main_axis_size(MainAxisSize::Min)
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .spacing(1)
    .into()
}
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| gate | none, ignore, absorb | ignore |

### Methods

#### `IgnorePointer`: constructors and associated functions

- `pub fn new() -> IgnorePointer` - Keeps the mouse from its child, by default.

#### `IgnorePointer`: builder methods (take and return `self`)

- `pub fn ignoring(mut self, ignoring: bool) -> IgnorePointer` - Whether the mouse passes the child by.
- `pub fn child(mut self, child: impl Into<AnyWidget>) -> IgnorePointer` - Over `child`.

#### `AbsorbPointer`: constructors and associated functions

- `pub fn new() -> AbsorbPointer` - Stops the mouse at its child, by default.

#### `AbsorbPointer`: builder methods (take and return `self`)

- `pub fn absorbing(mut self, absorbing: bool) -> AbsorbPointer` - Whether the mouse stops at it.
- `pub fn child(mut self, child: impl Into<AnyWidget>) -> AbsorbPointer` - Over `child`.

### Also used in the code

- `rtwk::gestures::MouseRegion` (struct, `crates/rtwk-widgets/src/gestures/mouse.rs:270`) Tells when the mouse comes in over its child (`on_enter`), moves over it with no button held (`on_hover`) and goes out (`on_exit`), and sets the pointer's shape over it (`cursor`).
  - public fields: `on_enter: PointerHandler`, `on_exit: PointerHandler`, `on_hover: PointerHandler`, `cursor: Option<PointerShape>`, `cursor_at: Option<CursorAt>`, `cursor_asks_motion: bool`, `behavior: HitTestBehavior`, `child: Option<AnyWidget>`
- `rtwk::gestures::GestureDetector` (struct, `crates/rtwk-widgets/src/gestures/gesture_detector.rs:91`) Tells the gestures of the presses over its child: taps, double taps, long presses and drags.
  - public fields: `on_tap: Option<Handler<PointerEvent>>`, `on_tap_up: Option<Handler<PointerEvent>>`, `on_tap_down: Option<Handler<PointerEvent>>`, `tap_at: Option<TapAt>`, `cursor: Option<PointerShape>`, `on_double_tap: Option<Handler<PointerEvent>>`, `on_long_press: Option<Handler<PointerEvent>>`, `on_long_press_end: Option<Handler<PointerEvent>>`, `on_pan_start: Option<Handler<DragDetails>>`, `on_pan_update: Option<Handler<DragDetails>>`, `on_pan_end: Option<Handler<DragDetails>>`, `buttons: Buttons`, `double_tap_interval: Duration`, `long_press_duration: Duration`, `behavior: HitTestBehavior`, `child: Option<AnyWidget>`
- `rtwk::layout::Stack` (struct, `crates/rtwk-widgets/src/layout/stack.rs:36`) Its children on top of each other, a later one over an earlier one (twk's `Stack`, over a `RenderStack`).
  - public fields: `alignment: Alignment`, `fit: StackFit`
- `rtwk_render::HitTestBehavior` (enum, `crates/rtwk-render/src/object.rs:49`) What a hit test does with a render object in whose bounds the cell is (`RenderTree::hit_test`).
  - variants: `DeferToChild`, `Opaque`, `Translucent`
- `rtwk::layout::BorderTitle` (struct, `crates/rtwk-widgets/src/layout/box_.rs:153`) Text on the top or the bottom edge of a `BoxWidget`, in `style`: the y of `alignment` picks the edge (-1 the top, 1 the bottom), its x the place along it (twk's `BorderTitle`).
  - public fields: `text: Str`, `style: Style`, `alignment: Alignment`
- `rtwk::layout::Positioned` (struct, `crates/rtwk-widgets/src/layout/stack.rs:107`) Pins its child, in a `Stack`, to the stack's edges (twk's `Positioned`): along an axis with both edges set, exactly as long as the cells between them; with its size set, that long; otherwise as long as it wants.
  - public fields: `left: Option<i32>`, `top: Option<i32>`, `right: Option<i32>`, `bottom: Option<i32>`, `width: Option<i32>`, `height: Option<i32>`, `child: AnyWidget`
- `rtwk::layout::Center` (struct, `crates/rtwk-widgets/src/layout/basic.rs:191`) An `Align` that puts its child in the middle, the odd cell after it (twk's `Center`).
  - public fields: `width_factor: Option<i32>`, `height_factor: Option<i32>`, `child: Option<AnyWidget>`

## SelectionArea

Text selected with the mouse as in a terminal, and copied to the clipboard: drag over it, or click twice or three times; a painted Selectable is selected in too, and a drag past its edge scrolls it.

API:

- `rtwk::gestures::SelectionArea` (struct, `crates/rtwk-widgets/src/gestures/selection.rs:423`) Lets the paragraphs below it be selected with the left button, as in a terminal, and copies what is selected.
  - public fields: `style: Option<Pen>`, `copy_on_release: bool`, `copy_key: Option<KeyPress>`, `clear_key: Option<KeyPress>`, `trim_trailing_blanks: bool`, `on_copy: Option<Handler<String>>`, `copy_transform: Option<CopyTransform>`, `controller: Option<SelectionAreaController>`, `multi_click_interval: Duration`, `child: AnyWidget`

### Code

The gallery's Code tab at the initial options (route `#/mouse/selectionarea`, `crates/rtwk-gallery/src/pages/mouse.rs:365`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
let copy_on_release = true;
let reverse = false;

{
    let content: AnyWidget =
        column![
            Text::new("An app that reads the mouse keeps the terminal from selecting its text, so it selects the text itself."),
            Text::new("A drag goes on from one paragraph to the next."),
        ]
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .spacing(1)
        .into();
    let mut selection = SelectionArea::new(content)
        .controller(self.controller.clone())
        .clear_key(None)
        .copy_on_release(copy_on_release)
        .on_copy(cx.map(SelectMsg::Copied));
    if reverse {
        selection = selection.style(Style::new().attrs(Attrs::REVERSE).pen());
    }
    let selected = self.controller.has_selection();
    let selection = KeyBindings::new(
        [("escape", selected.then(|| cx.send(SelectMsg::Clear)))],
        selection,
    )
    .hints(selected.then_some(("escape", "clear")));
    let text = if self.copied.is_empty() {
        "nothing".to_owned()
    } else {
        self.copied.replace('\n', " ")
    };
    SizedBox::new(Some(44), None, None)
        .child(
            column![
                selection,
                row![
                    Text::new("copied:").style(theme.muted.clone()),
                    Expanded::new(
                        1,
                        LitLine::new(vec![(text, self.copied.is_empty())], self.serial)
                            .max_lines(Some(2))
                    ),
                ]
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .spacing(1),
            ]
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .spacing(1),
        )
        .into()
}
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| view | texts, log | texts |
| copy | on, off | true |
| style | selection, reverse | selection |

### Methods

#### `SelectionArea`: constructors and associated functions

- `pub fn new(child: impl Into<AnyWidget>) -> SelectionArea` - Lets the texts below `child` be selected.

#### `SelectionArea`: builder methods (take and return `self`)

- `pub fn style(mut self, style: Pen) -> Self` - Merging `style` onto the text selected.
- `pub fn copy_on_release(mut self, on: bool) -> Self` - Whether a release copies what its press selected.
- `pub fn copy_key(mut self, key: impl Into<KeyPress>) -> Self` - Copying with `key`.
- `pub fn clear_key(mut self, key: Option<KeyPress>) -> Self` - Clearing with `key`, or with none.
- `pub fn trim_trailing_blanks(mut self, on: bool) -> Self` - Whether the blanks that end a line are left out of what is copied.
- `pub fn on_copy(mut self, h: Handler<String>) -> Self` - Telling `h` the text copied.
- `pub fn copy_transform(mut self, f: impl Fn(&str) -> String + 'static) -> Self` - Making what is copied of the text selected with `f`: called when the text is asked for (not for an empty one), and again after the text of a view it reads changed.
- `pub fn controller(mut self, controller: SelectionAreaController) -> Self` - Served by `controller`.
- `pub fn multi_click_interval(mut self, d: Duration) -> Self` - Waiting `d` for the next click of a double or triple click.

### Also used in the code

- `rtwk::focus::KeyBindings` (struct, `crates/rtwk-framework/src/focus/shortcuts.rs:1257`) Binds keys to handlers for the nodes below it.
  - public fields: `bindings: Vec<(Str`, `combos: Vec<(KeyCombo`, `hints: Vec<(Str`, `combo_hints: Vec<(KeyCombo`, `sequence_timeout: Option<Duration>`, `unmatched: UnmatchedKeys`, `child: AnyWidget`, `debug_label: Option<Str>`
- `rtwk::layout::Expanded` (struct, `crates/rtwk-widgets/src/layout/flex.rs:270`) `Flexible` with `FlexFit::Tight`: the child fills its part (twk's `Expanded`).

## Draggable, DragTarget

Data dragged from a widget with the mouse and dropped on another that accepts it.

API:

- `rtwk::gestures::Draggable` (struct, `crates/rtwk-widgets/src/gestures/drag_drop.rs:371`) Drags `data` from its child with the left button, to drop it on a `DragTarget`.
  - public fields: `data: T`, `feedback: AnyWidget`, `child: AnyWidget`, `child_when_dragging: Option<AnyWidget>`, `on_drag_started: Option<Handler<()>>`, `on_drag_completed: Option<Handler<()>>`, `on_drag_canceled: Option<Handler<()>>`, `behavior: HitTestBehavior`
- `rtwk::gestures::DragTarget` (struct, `crates/rtwk-widgets/src/gestures/drag_drop.rs:1088`) Takes the data of a `Draggable` dropped on it: shows what `builder` builds, again as a drag comes over it and leaves.
  - public fields: `builder: DragTargetBuilder<T>`, `on_will_accept_with_details: Option<WillAccept<T>>`, `on_accept_with_details: Option<Handler<DragTargetDetails<T>>>`, `on_leave: Option<Handler<T>>`, `on_move: Option<Handler<DragTargetDetails<T>>>`, `behavior: HitTestBehavior`

### Code

The gallery's Code tab at the initial options (route `#/mouse/draggable-dragtarget`, `crates/rtwk-gallery/src/pages/mouse.rs:699`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
// Wrap the app content in DragLayer::new(content) for drag feedback.
fn fruit_label(fruit: Fruit, style: Style) -> Text {
    Text::rich([
        Span::styled("● ", Style::new().fg(fruit.color())),
        Span::new(fruit.name()),
    ])
    .style(style)
}

fn fruit_source(
    fruit: Fruit,
    ghost: bool,
    theme: &ThemeData,
    canceled: rtwk::Handler<()>,
) -> AnyWidget {
    let feedback = Shadow::new(
        BoxWidget::new()
            .style(theme.surface.clone())
            .padding(EdgeInsets::x(1))
            .child(fruit_label(fruit, theme.text.clone().attrs(Attrs::BOLD))),
    )
    .shadow(BoxShadow {
        dx: 1,
        ..BoxShadow::default()
    });
    let source = Draggable::new(fruit, feedback, fruit_label(fruit, Style::new()))
        .on_drag_canceled(canceled);
    let source = if ghost {
        source.child_when_dragging(
            Text::new(format!("● {}", fruit.name())).style(theme.muted.clone()),
        )
    } else {
        source
    };
    AnyWidget::from(source).with_key(fruit.name())
}

let ghost = true;

{
    let targets = self.boxes.iter().enumerate().map(|(index, fruits)| {
        let fruits = fruits.clone();
        let theme = theme.clone();
        let sources: Vec<_> = fruits
            .iter()
            .map(|&fruit| {
                fruit_source(
                    fruit,
                    ghost,
                    &theme,
                    cx.map(move |()| FruitMsg::Cancel(index, fruit)),
                )
            })
            .collect();
        DragTarget::new(move |candidate: Option<&Fruit>, rejected| {
            let over = candidate.is_some() || rejected.is_some();
            let edge = if candidate.is_some() {
                &theme.primary
            } else if rejected.is_some() {
                &theme.error
            } else {
                &theme.border
            };
            let title_style = if over {
                edge.clone().attrs(Attrs::BOLD)
            } else {
                theme.muted.clone()
            };
            let mut contents = column![]
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .children(sources.clone());
            if fruits.is_empty() && !over {
                contents = contents.child(Text::new(EMPTY[index]).style(theme.muted.clone()));
            }
            if let Some(fruit) = candidate.filter(|fruit| !fruits.contains(fruit)) {
                contents = contents
                    .child(Text::new(format!("+ {}", fruit.name())).style(theme.primary.clone()));
            }
            if rejected.is_some() {
                contents = contents.child(Text::new("✗ red only").style(theme.error.clone()));
            }
            SizedBox::fixed(15, 6)
                .child(
                    BoxWidget::new()
                        .border(rounded(edge.clone()))
                        .title(
                            BorderTitle::new(format!(" {} ", BOX_NAMES[index])).style(title_style),
                        )
                        .padding(EdgeInsets::x(1))
                        .child(contents),
                )
                .into()
        })
        .on_will_accept_with_details(move |drag| index != 2 || drag.data.red())
        .on_accept_with_details(cx.map(move |drop: DragTargetDetails<Fruit>| FruitMsg::Drop(index, drop.data)))
        .into()
    });
    column![
        row![]
            .main_axis_size(MainAxisSize::Min)
            .spacing(2)
            .children(targets),
        LitLine::new(
            vec![
                ("dropped: ".into(), true),
                (
                    self.status.clone().unwrap_or_else(|| "nothing".into()),
                    self.status.is_none()
                ),
            ],
            self.serial,
        ),
    ]
    .main_axis_size(MainAxisSize::Min)
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .spacing(1)
    .into()
}
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| ghost | on, off | true |

### Methods

#### `Draggable`: constructors and associated functions

- `pub fn new(data: T, feedback: impl Into<AnyWidget>, child: impl Into<AnyWidget>) -> Self` - Drags `data` from `child`, `feedback` following the mouse.

#### `Draggable`: builder methods (take and return `self`)

- `pub fn child_when_dragging(mut self, w: impl Into<AnyWidget>) -> Self` - Showing `w` in place of the child while it is dragged.
- `pub fn on_drag_started(mut self, h: Handler<()>) -> Self` - Telling `h` the drag started.
- `pub fn on_drag_completed(mut self, h: Handler<()>) -> Self` - Telling `h` the data was dropped on a target that accepted it.
- `pub fn on_drag_canceled(mut self, h: Handler<()>) -> Self` - Telling `h` the drag ended with no target accepting it.
- `pub fn behavior(mut self, behavior: HitTestBehavior) -> Self` - Hit as `behavior` says.

#### `DragTarget`: constructors and associated functions

- `pub fn new(builder: impl Fn(Option<&T>, Option<&dyn Any>) -> AnyWidget + 'static) -> Self` - A target of the data of type `T` that shows what `builder` builds.

#### `DragTarget`: builder methods (take and return `self`)

- `pub fn on_will_accept_with_details(mut self, f: impl Fn(&DragTargetDetails<T>) -> bool + 'static) -> Self` - Asking `f` whether it accepts the data of a drag that came over it.
- `pub fn on_accept_with_details(mut self, h: Handler<DragTargetDetails<T>>) -> Self` - Telling `h` the data was dropped on it.
- `pub fn on_leave(mut self, h: Handler<T>) -> Self` - Telling `h` the drag left it.
- `pub fn on_move(mut self, h: Handler<DragTargetDetails<T>>) -> Self` - Telling `h` each cell the drag goes to over it.
- `pub fn behavior(mut self, behavior: HitTestBehavior) -> Self` - Hit as `behavior` says.

### Also used in the code

- `rtwk::layout::Shadow` (struct, `crates/rtwk-widgets/src/layout/shadow.rs:117`) Casts `shadow` from its child on the cells around it (twk's `Shadow`): the box is as big as the child and the room the shadow takes beside it.
  - public fields: `shadow: BoxShadow`, `child: Option<AnyWidget>`
- `rtwk::layout::BoxShadow` (struct, `crates/rtwk-widgets/src/layout/shadow.rs:38`) A shadow: how far it is cast, in which color, how strongly (twk's `BoxShadow`).
  - public fields: `dx: i32`, `dy: i32`, `color: Color`, `alpha: i32`, `opacity: Option<f64>`
- `rtwk::layout::BorderTitle` (struct, `crates/rtwk-widgets/src/layout/box_.rs:153`) Text on the top or the bottom edge of a `BoxWidget`, in `style`: the y of `alignment` picks the edge (-1 the top, 1 the bottom), its x the place along it (twk's `BorderTitle`).
  - public fields: `text: Str`, `style: Style`, `alignment: Alignment`
