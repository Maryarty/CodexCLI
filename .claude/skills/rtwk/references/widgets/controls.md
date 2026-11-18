<!-- Written by tools/rtwk-port/skill/generate.py from rtwk 7482e340; do not edit. -->

# Controls

[All widgets](../widgets.md)

## Button

A label pressed by a click, or by Enter or Space while it has the focus; disabled with no onPressed.

API:

- `rtwk::controls::Button` (struct, `crates/rtwk-controls/src/button.rs:107`) A button that tells `on_pressed` when it is clicked, or when Enter or Space is pressed while it has the focus.
  - public fields: `spans: Option<Rc<[Span]>>`, `tall: bool`, `on_pressed: Option<Handler<()>>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: bool`, `style: Option<Style>`, `focused_style: Option<Style>`, `hover_style: Option<Style>`, `disabled_style: Option<Style>`

### Code

The gallery's Code tab at the initial options (route `#/controls/button`, `crates/rtwk-gallery/src/pages/mod.rs:230`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
let theme = Theme::of(cx);
let make = |label| {
    Button::new(label)
};

row![
    make("Save").on_pressed(cx.send(Message::Save)),
    make("Send").on_pressed(cx.send(Message::Send)),
    make("Disabled"),
]
.main_axis_size(MainAxisSize::Min)
.spacing(2)
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| color | primary, accent, success, warning, error, info | primary |
| look | chip, outlined, filled, plain | chip |
| tall | on, off | false |

### Methods

#### `Button`: constructors and associated functions

- `pub fn new(label: impl Into<Str>) -> Button` - A button showing `label`; disabled until `on_pressed` is set.
- `pub fn outlined(label: impl Into<Str>) -> Button` - A button of `label` between two edges, or with `tall` in a box drawn around it: in `ThemeData::border`, or `style`'s text color, at rest.
- `pub fn filled(label: impl Into<Str>) -> Button` - A button of `label` on a fill of `ThemeData::primary`, or of `style`'s text color, at rest; with `tall` three rows of it.
- `pub fn rich(spans: impl Into<Rc<[Span]>>) -> Button` - A chip of `spans`, disabled until `on_pressed` is set: the key and its caption of a button, each in its own `Span` style, on one chip with the hover, the focus and the flash of `Button`.

#### `Button`: builder methods (take and return `self`)

- `pub fn on_pressed(mut self, h: Handler<()>) -> Button` - Told when it is pressed.
- `pub fn focus_node(mut self, node: Option<FocusNode>) -> Button` - With `node` as its node of the focus tree.
- `pub fn autofocus(mut self, on: bool) -> Button` - Taking the focus when mounted.
- `pub fn can_request_focus(mut self, on: bool) -> Button` - Whether it can take the focus.
- `pub fn tall(mut self, on: bool) -> Button` - A box or a fill around the label, three rows.
- `pub fn style(mut self, style: Style) -> Button` - At rest in `style`.
- `pub fn focused_style(mut self, style: Style) -> Button` - While focused in `style` over the one at rest.
- `pub fn hover_style(mut self, style: Style) -> Button` - While the mouse is over it in `style` over the one at rest.
- `pub fn disabled_style(mut self, style: Style) -> Button` - While disabled in `style` over the one at rest.

#### `Button`: methods

- `pub fn label(&self) -> Str` - The text it shows, with a blank on either side; what a screen reader reads.

## Pressable

What the controls are built of: a node of the focus tree that Tab passes by while it is disabled, the keys it takes while it has the focus, a press that focuses it and a tap; it builds its look by whether it has the focus.

API:

- `rtwk::controls::Pressable` (struct, `crates/rtwk-controls/src/pressable.rs:143`) A control's node of the focus tree: what `builder` builds, which takes the focus, its keys and a click of the mouse while `enabled`.
  - public fields: `builder: PressBuilder`, `enabled: bool`, `can_request_focus: bool`, `bindings: Rc<[(Str`, `hints: Rc<[(Str`, `on_key_event: Option<FocusOnKeyEventCallback>`, `on_tap: Option<Handler<PointerEvent>>`, `on_tap_up: Option<Handler<PointerEvent>>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `behavior: HitTestBehavior`

### Code

The gallery's Code tab at the initial options (route `#/controls/pressable`, `crates/rtwk-gallery/src/pages/mod.rs:259`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
Pressable::new(move |cx, focused| {
    let theme = Theme::of(cx);
    let (star, style) = if starred {
        ("★", theme.primary.merge(&Style::new().attrs(Attrs::BOLD)))
    } else {
        ("☆", theme.muted.clone())
    };
    Text::rich([Span::styled(star, style), Span::new(" twk")])
        .style(if focused { theme.selection.clone() } else { Style::new() })
        .into()
})
.keys([("space", Some(toggle_by_key))], [("space", "star")])
.on_tap(toggle_by_mouse)
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| color | primary, accent, success, warning, error, info | primary |

### Methods

#### `Pressable`: constructors and associated functions

- `pub fn new(builder: impl Fn(&mut BuildCx<'_>, bool) -> AnyWidget + 'static) -> Pressable` - A control built by `builder`, enabled.
- `pub fn with_builder(builder: PressBuilder) -> Pressable` - A control built by the builder `builder`, enabled.

#### `Pressable`: builder methods (take and return `self`)

- `pub fn enabled(mut self, enabled: bool) -> Pressable` - Whether it takes the focus and the presses.
- `pub fn can_request_focus(mut self, on: bool) -> Pressable` - Whether it can take the focus while enabled.
- `pub fn bindings<N, H>(mut self, bindings: impl IntoIterator<Item = (N, H)>) -> Pressable where N: Into<Str>, H: Into<Option<Handler<()>>>,` - With the keys `bindings` name taking the handlers.
- `pub fn keys<const B: usize, const H: usize>(mut self, bindings: [(&'static str, Option<Handler<()>>); B], hints: [(&'static str, &'static str); H]) -> Pressable` - With the keys of `bindings` and their `hints`, all static names (one allocation for each list, which an array lets `Rc` make at once).
- `pub fn shared_keys(mut self, bindings: Rc<[(Str, Option<Handler<()>>)]>, hints: Rc<[(Str, Str)]>) -> Pressable` - With the keys and hints a control keeps from build to build (one made once, shared: the widget copies nothing).
- `pub fn hints<N: Into<Str>, T: Into<Str>>(mut self, hints: impl IntoIterator<Item = (N, T)>) -> Pressable` - With `hints` for the keys by name.
- `pub fn on_key_event(mut self, f: impl Fn(&FocusNode, rtwk_framework::focus::KeyPress) -> rtwk_framework::focus::KeyEventResult + 'static) -> Pressable` - Giving the keys the bindings leave to `f`.
- `pub fn on_tap(mut self, h: Handler<PointerEvent>) -> Pressable` - Telling `h` a tap.
- `pub fn on_tap_up(mut self, h: Handler<PointerEvent>) -> Pressable` - Telling `h` the release of a tap, before `on_tap` is told it.
- `pub fn focus_node(mut self, node: Option<FocusNode>) -> Pressable` - With `node` as its node, in place of one of its own.
- `pub fn autofocus(mut self, on: bool) -> Pressable` - Taking the focus when mounted, when its scope has none.
- `pub fn behavior(mut self, behavior: HitTestBehavior) -> Pressable` - Hit where `behavior` says.

## Checkbox, Switch

What is on or off: a click, or Space or Enter, flips it.

API:

- `rtwk::controls::Checkbox` (struct, `crates/rtwk-controls/src/toggle.rs:64`) A checkbox before `label`: `[x]` while `value` is true, `[ ]` while it is false.
  - public fields: `value: bool`, `on_changed: Option<Handler<bool>>`, `label: Str`, `semantics_label: Option<Str>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `style: Option<Style>`, `bracket_style: Option<Style>`, `focused_style: Option<Style>`, `disabled_style: Option<Style>`
- `rtwk::controls::Switch` (struct, `crates/rtwk-controls/src/toggle.rs:193`) A switch before `label`, on while `value` is true and off while it is false.
  - public fields: `value: bool`, `on_changed: Option<Handler<bool>>`, `label: Str`, `semantics_label: Option<Str>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `pill: bool`, `style: Option<Style>`, `focused_style: Option<Style>`, `disabled_style: Option<Style>`

### Code

The gallery's Code tab at the initial options (route `#/controls/checkbox-switch`, `crates/rtwk-gallery/src/pages/mod.rs:217`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
column![
    Checkbox::new(self.wrap)
        .label("wrap lines")
        .on_changed(cx.map(Msg::Wrap)),
    Switch::new(self.dark)
        .label("dark mode")
        .pill(true)
        .on_changed(cx.map(Msg::Dark)),
    Checkbox::new(true).label("disabled"),
]
.main_axis_size(MainAxisSize::Min)
.cross_axis_alignment(CrossAxisAlignment::Start)
.spacing(1)
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| switch | pill, brackets | pill |

### Methods

#### `Checkbox`: constructors and associated functions

- `pub fn new(value: bool) -> Checkbox` - A box checked while `value` is; disabled until `on_changed` is set.

#### `Checkbox`: builder methods (take and return `self`)

- `pub fn on_changed(mut self, h: Handler<bool>) -> Checkbox` - Told the value the box is to take.
- `pub fn label(mut self, label: impl Into<Str>) -> Checkbox` - With `label` after the box.
- `pub fn semantics_label(mut self, label: impl Into<Str>) -> Checkbox` - Read as `label` by a screen reader where it shows none.
- `pub fn focus_node(mut self, node: Option<FocusNode>) -> Checkbox` - With `node` as its node of the focus tree.
- `pub fn autofocus(mut self, on: bool) -> Checkbox` - Taking the focus when mounted.
- `pub fn style(mut self, style: Style) -> Checkbox` - The row in `style`.
- `pub fn bracket_style(mut self, style: Style) -> Checkbox` - The brackets in `style`.
- `pub fn focused_style(mut self, style: Style) -> Checkbox` - The row while focused in `style`.
- `pub fn disabled_style(mut self, style: Style) -> Checkbox` - The row while disabled in `style`.

#### `Switch`: constructors and associated functions

- `pub fn new(value: bool) -> Switch` - A switch on while `value` is; disabled until `on_changed` is set.
- `pub fn pill_spans(theme: &ThemeData, at: f64, enabled: bool) -> Vec<Span>` - The spans of the pill a `Switch` draws in `theme`, five cells, its knob `at` of the way to the right end (0 off, 1 on, between them as it moves), muted unless `enabled`: for a painter of one's own that shows a switch as ...

#### `Switch`: builder methods (take and return `self`)

- `pub fn on_changed(mut self, h: Handler<bool>) -> Switch` - Told the value the switch is to take.
- `pub fn label(mut self, label: impl Into<Str>) -> Switch` - With `label` after the switch.
- `pub fn semantics_label(mut self, label: impl Into<Str>) -> Switch` - Read as `label` by a screen reader where it shows none.
- `pub fn focus_node(mut self, node: Option<FocusNode>) -> Switch` - With `node` as its node of the focus tree.
- `pub fn autofocus(mut self, on: bool) -> Switch` - Taking the focus when mounted.
- `pub fn pill(mut self, pill: bool) -> Switch` - The pill, or `[on ]` and `[off]`.
- `pub fn style(mut self, style: Style) -> Switch` - The row in `style`.
- `pub fn focused_style(mut self, style: Style) -> Switch` - The row while focused in `style`.
- `pub fn disabled_style(mut self, style: Style) -> Switch` - The row while disabled in `style`.

## RadioGroup

One of several choices: the arrows or a click choose one.

API:

- `rtwk::controls::RadioGroup` (struct, `crates/rtwk-controls/src/choice.rs:193`) Radio buttons: `choices`, one of which is chosen, each its label after a mark, `(●)` for the one whose value is `value`, `( )` for the others (`Glyphs::radio_on`, `radio_off` and `radio_brackets` of the theme).
  - public fields: `choices: Rc<[Choice<T>]>`, `value: Option<T>`, `on_changed: Option<Handler<T>>`, `direction: Axis`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: bool`, `mark: Option<Str>`, `style: Option<Style>`, `bracket_style: Option<Style>`, `focused_style: Option<Style>`, `disabled_style: Option<Style>`

### Code

The gallery's Code tab at the initial options (route `#/controls/radiogroup`, `crates/rtwk-gallery/src/pages/mod.rs:515`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
let group = RadioGroup::new(vec![
    Choice::new("apple", "apple"),
    Choice::new("banana", "banana"),
    Choice::new("cherry", "cherry"),
    Choice::new("durian", "durian").enabled(false),
], self.chosen)
    .direction(Axis::Vertical)
    .on_changed(on_chosen);
group
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| direction | vertical, horizontal | vertical |
| mark | ●, *, ✓ | ● |
| look | (●) ( ), ◉ ○ | (●) ( ) |

### Methods

#### `RadioGroup`: constructors and associated functions

- `pub fn new(choices: impl Into<Rc<[Choice<T>]>>, value: Option<T>) -> RadioGroup<T>` - Radio buttons of `choices`, the one of `value` chosen; disabled until `on_changed` is set.

#### `RadioGroup`: builder methods (take and return `self`)

- `pub fn on_changed(mut self, h: Handler<T>) -> RadioGroup<T>` - Told the value of the choice chosen.
- `pub fn direction(mut self, direction: Axis) -> RadioGroup<T>` - With the choices going along `direction`.
- `pub fn focus_node(mut self, node: Option<FocusNode>) -> RadioGroup<T>` - With `node` as its node of the focus tree.
- `pub fn autofocus(mut self, on: bool) -> RadioGroup<T>` - Taking the focus when mounted.
- `pub fn can_request_focus(mut self, on: bool) -> RadioGroup<T>` - Whether it can take the focus.
- `pub fn mark(mut self, mark: impl Into<Str>) -> RadioGroup<T>` - With `mark` between the brackets of the choice chosen.
- `pub fn style(mut self, style: Style) -> RadioGroup<T>` - With `style` under a choice's.
- `pub fn bracket_style(mut self, style: Style) -> RadioGroup<T>` - With `style` for the brackets.
- `pub fn focused_style(mut self, style: Style) -> RadioGroup<T>` - With `style` for the choice that shows the focus.
- `pub fn disabled_style(mut self, style: Style) -> RadioGroup<T>` - With `style` for a choice that cannot be chosen.

#### `RadioGroup`: methods

- `pub fn chosen(&self) -> Option<usize>` - The choice chosen: the enabled one whose value is `value`.

### Also used in the code

- `rtwk::controls::Choice` (struct, `crates/rtwk-controls/src/choice.rs:68`) One of the values a `RadioGroup`, a `Select` or a `MultiSelect` offers: `value`, shown as `label`.
  - public fields: `value: T`, `label: Str`, `enabled: bool`
- `rtwk::layout::Axis` (enum, `crates/rtwk-render/src/objects/decoration.rs:39`) Along which axis something runs: across or down.
  - variants: `Horizontal`, `Vertical`

## SegmentedButton

A few values joined in one row, one on at a time or any of them: the arrows, a digit or a click turn one on.

API:

- `rtwk::controls::SegmentedButton` (struct, `crates/rtwk-controls/src/segmented_button.rs:98`) A row of `segments` joined into one control, of which those whose values are in `selected` are on: `[ All │ Errors (3) │ Warnings │ Info ]`.
  - public fields: `segments: Rc<[Choice<T>]>`, `selected: Rc<[T]>`, `on_selection_changed: Option<Handler<Vec<T>>>`, `multi_selection_enabled: bool`, `empty_selection_allowed: bool`, `show_selected_icon: bool`, `caps: (Str`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `style: Option<Style>`

### Code

The gallery's Code tab at the initial options (route `#/controls/segmentedbutton`, `crates/rtwk-gallery/src/pages/mod.rs:544`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
SegmentedButton::new(vec![
    Choice::new("all", "All"),
    Choice::new("errors", "Errors (3)"),
    Choice::new("warnings", "Warnings (12)"),
    Choice::new("info", "Info").enabled(false),
], self.shown[0].clone())
    .multi_selection_enabled(false)
    .empty_selection_allowed(false)
    .show_selected_icon(false)
    .on_selection_changed(on_shown)
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| multiple | on, off | false |
| empty | on, off | false |
| icon | on, off | false |
| caps | square, round | square |

### Methods

#### `SegmentedButton`: constructors and associated functions

- `pub fn new(segments: impl Into<Rc<[Choice<T>]>>, selected: impl Into<Rc<[T]>>) -> SegmentedButton<T>` - A row of `segments`, those whose values are in `selected` on; disabled until `on_selection_changed` is set.

#### `SegmentedButton`: builder methods (take and return `self`)

- `pub fn on_selection_changed(mut self, h: Handler<Vec<T>>) -> SegmentedButton<T>` - Told the values that are on after a press.
- `pub fn multi_selection_enabled(mut self, on: bool) -> SegmentedButton<T>` - Whether any number of segments can be on.
- `pub fn empty_selection_allowed(mut self, on: bool) -> SegmentedButton<T>` - Whether the last segment on can be turned off.
- `pub fn show_selected_icon(mut self, on: bool) -> SegmentedButton<T>` - Whether a segment that is on shows `✓ ` before its label.
- `pub fn caps(mut self, open: impl Into<Str>, close: impl Into<Str>) -> SegmentedButton<T>` - With `open` and `close` at either end.
- `pub fn focus_node(mut self, node: Option<FocusNode>) -> SegmentedButton<T>` - With `node` as its node of the focus tree.
- `pub fn autofocus(mut self, on: bool) -> SegmentedButton<T>` - Taking the focus when mounted.
- `pub fn style(mut self, style: Style) -> SegmentedButton<T>` - The row in `style`.

### Also used in the code

- `rtwk::controls::Choice` (struct, `crates/rtwk-controls/src/choice.rs:68`) One of the values a `RadioGroup`, a `Select` or a `MultiSelect` offers: `value`, shown as `label`.
  - public fields: `value: T`, `label: Str`, `enabled: bool`

## Chip, ChipList

Tags on one row that wraps, with a cursor: the arrows move it, Space or a click toggles a chip, Delete or a click on its × removes it.

API:

- `rtwk::controls::Chip` [path inferred] (struct, `crates/rtwk-controls/src/chip.rs:65`) A short `label` on a chip of its own, on one row: a tag, a filter, a recipient.
  - public fields: `label: Str`, `selected: bool`, `style: Option<Style>`
- `rtwk::controls::ChipList` (struct, `crates/rtwk-controls/src/chip.rs:279`) A row of `Chip`s of `chips` that wraps onto more rows, with a cursor that keys and clicks move and press: `Enter` and `Space` or a click toggle the chip and tell `on_selection_changed` the values that are on then, as ...
  - public fields: `chips: Rc<[Choice<T>]>`, `selected: Rc<[T]>`, `on_selection_changed: Option<Handler<Vec<T>>>`, `on_deleted: Option<Handler<T>>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `style: Option<Style>`

### Code

The gallery's Code tab at the initial options (route `#/controls/chip-chiplist`, `crates/rtwk-gallery/src/pages/mod.rs:580`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
let chips = ChipList::new(
    self.tags.iter()
        .map(|&tag| Choice::new(tag, tag).enabled(tag != "beta"))
        .collect::<Vec<_>>(),
)
    .selected(self.selected.clone())
    .on_selection_changed(on_selected)
    .on_deleted(on_deleted);
SizedBox::new(Some(34), None, Some(chips.into()))
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| toggle | on, off | true |
| remove | on, off | true |
| width | 20..50 (a slider) | 34 cells |

### Methods

#### `Chip`: constructors and associated functions

- `pub fn new(label: impl Into<Str>) -> Chip` - A chip of `label`.

#### `Chip`: builder methods (take and return `self`)

- `pub fn selected(mut self, selected: bool) -> Chip` - Whether it is on.
- `pub fn style(mut self, style: Style) -> Chip` - Over the theme's text.

#### `ChipList`: constructors and associated functions

- `pub fn new(chips: impl Into<Rc<[Choice<T>]>>) -> ChipList<T>` - A list of `chips`, none on; disabled until a callback is set.

#### `ChipList`: builder methods (take and return `self`)

- `pub fn selected(mut self, selected: impl Into<Rc<[T]>>) -> ChipList<T>` - With the chips whose values are in `selected` on.
- `pub fn on_selection_changed(mut self, h: Handler<Vec<T>>) -> ChipList<T>` - Told the values that are on after a chip was toggled.
- `pub fn on_deleted(mut self, h: Handler<T>) -> ChipList<T>` - Told the value of a chip to take away.
- `pub fn focus_node(mut self, node: Option<FocusNode>) -> ChipList<T>` - With `node` as its node of the focus tree.
- `pub fn autofocus(mut self, on: bool) -> ChipList<T>` - Taking the focus when mounted.
- `pub fn style(mut self, style: Style) -> ChipList<T>` - Over the theme's text on each chip.

### Also used in the code

- `rtwk::controls::Choice` (struct, `crates/rtwk-controls/src/choice.rs:68`) One of the values a `RadioGroup`, a `Select` or a `MultiSelect` offers: `value`, shown as `label`.
  - public fields: `value: T`, `label: Str`, `enabled: bool`

## ValueStepper

One of a few values between ‹ and ›: Left and Right or a click on an arrow step to the one before or after, stopping at the ends (`wrap: true` goes round).

API:

- `rtwk::controls::ValueStepper` (struct, `crates/rtwk-controls/src/value_stepper.rs:66`) The value of one of `values` shown between arrows, `‹ dark ›`, which Left and Right and a click on an arrow step to the value before and after it.
  - public fields: `values: Rc<[Choice<T>]>`, `value: T`, `on_changed: Option<Handler<T>>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: bool`, `style: Option<Style>`, `wrap: bool`

### Code

The gallery's Code tab at the initial options (route `#/controls/valuestepper`, `crates/rtwk-gallery/src/pages/mod.rs:616`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
ValueStepper::new(vec![
    Choice::new("dark", "dark"),
    Choice::new("light", "light"),
    Choice::new("sepia", "sepia").enabled(false),
    Choice::new("auto", "auto-switching"),
], self.mode)
    .on_changed(on_mode)
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| enabled | on, off | true |

### Methods

#### `ValueStepper`: constructors and associated functions

- `pub fn new(values: impl Into<Rc<[Choice<T>]>>, value: T) -> ValueStepper<T>` - A stepper of `values` showing `value`; disabled until `on_changed` is set.

#### `ValueStepper`: builder methods (take and return `self`)

- `pub fn on_changed(mut self, h: Handler<T>) -> ValueStepper<T>` - Told the value stepped to.
- `pub fn focus_node(mut self, node: Option<FocusNode>) -> ValueStepper<T>` - With `node` as its node of the focus tree.
- `pub fn autofocus(mut self, on: bool) -> ValueStepper<T>` - Taking the focus when mounted.
- `pub fn can_request_focus(mut self, on: bool) -> ValueStepper<T>` - Whether it can take the focus.
- `pub fn style(mut self, style: Style) -> ValueStepper<T>` - The row in `style`.
- `pub fn wrap(mut self, wrap: bool) -> ValueStepper<T>` - Whether stepping goes round.

#### `ValueStepper`: methods

- `pub fn index(&self) -> Option<usize>` - The place of the value shown among the values; none when it is none of them.

### Also used in the code

- `rtwk::controls::Choice` (struct, `crates/rtwk-controls/src/choice.rs:68`) One of the values a `RadioGroup`, a `Select` or a `MultiSelect` offers: `value`, shown as `label`.
  - public fields: `value: T`, `label: Str`, `enabled: bool`

## SelectionList

Items selected by their keys: in a single selection the cursor selects, in a multiple one Space does.

API:

- `rtwk::controls::SelectionList` (struct, `crates/rtwk-controls/src/selection_list.rs:116`) A list of `item_count` items to select one of, or any number with a multiple `controller`, by the keys and the mouse.
  - public fields: `item_count: usize`, `key_of: Rc<dyn Fn(usize) -> K>`, `item_builder: IndexedWidgetBuilder`, `controller: Option<SelectionController<K>>`, `on_submitted: Option<Handler<K>>`, `item_enabled: Option<Rc<dyn Fn(usize) -> bool>>`, `enabled: bool`, `item_extent: Option<i32>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `style: Option<Style>`, `selected_style: Option<Style>`, `cursor_style: Option<Style>`, `idle_style: Option<Style>`, `disabled_style: Option<Style>`, `bracket_style: Option<Style>`, `bar: bool`

### Code

The gallery's Code tab at the initial options (route `#/controls/selectionlist`, `crates/rtwk-gallery/src/pages/mod.rs:416`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
// Create the controller once for this selection mode.
let controller = SelectionController::with(false, [2], None);

SelectionList::new(PLANETS.len(), |i| i, |i| Text::new(PLANETS[i]).into())
    .item_enabled(|i| i != 1)
    .bar(true)
    .controller(&controller)
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| multiple | on, off | false |
| bar | on, off | true |

### Methods

#### `SelectionList`: constructors and associated functions

- `pub fn new(item_count: usize, key_of: impl Fn(usize) -> K + 'static, item_builder: impl Fn(usize) -> AnyWidget + 'static) -> SelectionList<K>` - A list of `item_count` items (at least 0), of the keys `key_of` gives and the widgets `item_builder` builds.
- `pub fn with_builders(item_count: usize, key_of: Rc<dyn Fn(usize) -> K>, item_builder: IndexedWidgetBuilder) -> SelectionList<K>` - As `new`, with the closures behind `Rc`s the app keeps: built again with the same ones (and the same other arguments), the list is an equal widget and is skipped.

#### `SelectionList`: builder methods (take and return `self`)

- `pub fn controller(mut self, controller: impl Into<SelectionController<K>>) -> SelectionList<K>` - Holding what is selected and the cursor in `controller`.
- `pub fn on_submitted(mut self, h: Handler<K>) -> SelectionList<K>` - Told the key of the cursor's item on Enter.
- `pub fn item_enabled(mut self, f: impl Fn(usize) -> bool + 'static) -> SelectionList<K>` - Whether an item, by its index, can be selected.
- `pub fn item_enabled_rc(mut self, f: Rc<dyn Fn(usize) -> bool>) -> SelectionList<K>` - As `item_enabled`, behind an `Rc` the app keeps.
- `pub fn enabled(mut self, on: bool) -> SelectionList<K>` - Whether the list takes the focus and the mouse.
- `pub fn item_extent(mut self, rows: i32) -> SelectionList<K>` - Each item `rows` tall.
- `pub fn focus_node(mut self, node: FocusNode) -> SelectionList<K>` - With `node` as its node of the focus tree.
- `pub fn autofocus(mut self, on: bool) -> SelectionList<K>` - Taking the focus when mounted.
- `pub fn style(mut self, style: Style) -> SelectionList<K>` - An item over the theme's text in `style`.
- `pub fn selected_style(mut self, style: Style) -> SelectionList<K>` - An item selected in `style`.
- `pub fn cursor_style(mut self, style: Style) -> SelectionList<K>` - The cursor's item while the list has the focus in `style`.
- `pub fn idle_style(mut self, style: Style) -> SelectionList<K>` - The item selected while the list has not the focus in `style`.
- `pub fn disabled_style(mut self, style: Style) -> SelectionList<K>` - An item that cannot be selected in `style`.
- `pub fn bracket_style(mut self, style: Style) -> SelectionList<K>` - The brackets of the marks in `style`.
- `pub fn bar(mut self, on: bool) -> SelectionList<K>` - Whether a column before the items shows the bar.

### Also used in the code

- `rtwk::controls::SelectionController` (struct, `crates/rtwk-data/src/selection_controller.rs:72`) Holds which items are `selected`, and the one its `cursor` is on, by their keys.

## Select, MultiSelect

A field that opens its choices to pick one, or several, from, or a PopupController from the app; typing filters them.

API:

- `rtwk::controls::Select` (struct, `crates/rtwk-controls/src/select.rs:330`) A drop-down: a field that shows the label of the choice whose value is `value`, and opens a list of the `choices` to choose another from.
  - public fields: `choices: Rc<[Choice<T>]>`, `value: Option<T>`, `on_changed: Option<Handler<T>>`, `controller: Option<PopupController>`, `hint: Str`, `inline: bool`, `rows: usize`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `style: Option<Style>`, `focused_style: Option<Style>`, `disabled_style: Option<Style>`, `arrow_style: Option<Style>`, `popup_look: PanelLook`, `popup_fade_in: Duration`, `decoration: Option<InputDecoration>`
- `rtwk::controls::MultiSelect` (struct, `crates/rtwk-controls/src/select.rs:393`) A `MultiSelect` of any number of the choices: those whose values are in `values` are selected.
  - public fields: `choices: Rc<[Choice<T>]>`, `values: Rc<[T]>`, `on_changed: Option<Handler<Vec<T>>>`, `controller: Option<PopupController>`, `hint: Str`, `inline: bool`, `rows: usize`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `style: Option<Style>`, `focused_style: Option<Style>`, `disabled_style: Option<Style>`, `arrow_style: Option<Style>`, `popup_look: PanelLook`, `popup_fade_in: Duration`, `decoration: Option<InputDecoration>`

### Code

The gallery's Code tab at the initial options (route `#/controls/select-multiselect`, `crates/rtwk-gallery/src/pages/mod.rs:272`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
let one = Select::new(choices.clone(), selected_fruit)
    .hint("a fruit")
    .inline(false)
    .rows(4)
    .controller(&popup)
    .on_changed(choose_one);
let some = MultiSelect::new(choices, selected_fruits)
    .hint("some fruit")
    .inline(false)
    .rows(4)
    .on_changed(choose_some);

column![
    row![one, some].spacing(4),
    Button::new("Choose a fruit").on_pressed(open_popup),
]
.spacing(1)
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| inline | on, off | false |
| rows | 4, 8 | 4 |
| popup | raised, bordered | raised |

### Methods

#### `Select`: constructors and associated functions

- `pub fn new(choices: impl Into<Rc<[Choice<T>]>>, value: Option<T>) -> Select<T>` - A select of `choices` showing the one whose value is `value`; disabled until `on_changed` is set.

#### `Select`: builder methods (take and return `self`)

- `pub fn controller(mut self, controller: impl Into<PopupController>) -> Select<T>` - Opened and closed by `controller`.
- `pub fn hint(mut self, hint: impl Into<Str>) -> Select<T>` - Showing `hint` while nothing is chosen.
- `pub fn inline(mut self, inline: bool) -> Select<T>` - Whether the list is always below the field.
- `pub fn rows(mut self, rows: usize) -> Select<T>` - At most `rows` choices shown at once, above 0.
- `pub fn focus_node(mut self, node: Option<FocusNode>) -> Select<T>` - With `node` as its node of the focus tree.
- `pub fn autofocus(mut self, on: bool) -> Select<T>` - Taking the focus when mounted.
- `pub fn style(mut self, style: Style) -> Select<T>` - The field's chip in `style`.
- `pub fn focused_style(mut self, style: Style) -> Select<T>` - The field in `style` while it has the focus.
- `pub fn disabled_style(mut self, style: Style) -> Select<T>` - The field in `style` while it is disabled.
- `pub fn arrow_style(mut self, style: Style) -> Select<T>` - `▾` in `style` while the field has not the focus.
- `pub fn popup_look(mut self, look: PanelLook) -> Select<T>` - The popup as `look` says.
- `pub fn popup_fade_in(mut self, d: Duration) -> Select<T>` - The popup fading in over `d`.
- `pub fn decoration(mut self, decoration: InputDecoration) -> Select<T>` - The field drawn in `decoration`.
- `pub fn on_changed(mut self, h: Handler<T>) -> Select<T>` - Told the value of the choice chosen.

#### `MultiSelect`: constructors and associated functions

- `pub fn new(choices: impl Into<Rc<[Choice<T>]>>, values: impl Into<Rc<[T]>>) -> MultiSelect<T>` - A select of `choices` with those whose values are in `values` selected; disabled until `on_changed` is set.

#### `MultiSelect`: builder methods (take and return `self`)

- `pub fn controller(mut self, controller: impl Into<PopupController>) -> MultiSelect<T>` - Opened and closed by `controller`.
- `pub fn hint(mut self, hint: impl Into<Str>) -> MultiSelect<T>` - Showing `hint` while nothing is chosen.
- `pub fn inline(mut self, inline: bool) -> MultiSelect<T>` - Whether the list is always below the field.
- `pub fn rows(mut self, rows: usize) -> MultiSelect<T>` - At most `rows` choices shown at once, above 0.
- `pub fn focus_node(mut self, node: Option<FocusNode>) -> MultiSelect<T>` - With `node` as its node of the focus tree.
- `pub fn autofocus(mut self, on: bool) -> MultiSelect<T>` - Taking the focus when mounted.
- `pub fn style(mut self, style: Style) -> MultiSelect<T>` - The field's chip in `style`.
- `pub fn focused_style(mut self, style: Style) -> MultiSelect<T>` - The field in `style` while it has the focus.
- `pub fn disabled_style(mut self, style: Style) -> MultiSelect<T>` - The field in `style` while it is disabled.
- `pub fn arrow_style(mut self, style: Style) -> MultiSelect<T>` - `▾` in `style` while the field has not the focus.
- `pub fn popup_look(mut self, look: PanelLook) -> MultiSelect<T>` - The popup as `look` says.
- `pub fn popup_fade_in(mut self, d: Duration) -> MultiSelect<T>` - The popup fading in over `d`.
- `pub fn decoration(mut self, decoration: InputDecoration) -> MultiSelect<T>` - The field drawn in `decoration`.
- `pub fn on_changed(mut self, h: Handler<Vec<T>>) -> MultiSelect<T>` - Told the values selected.

### Also used in the code

- `rtwk::controls::Button` (struct, `crates/rtwk-controls/src/button.rs:107`) A button that tells `on_pressed` when it is clicked, or when Enter or Space is pressed while it has the focus.
  - public fields: `spans: Option<Rc<[Span]>>`, `tall: bool`, `on_pressed: Option<Handler<()>>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: bool`, `style: Option<Style>`, `focused_style: Option<Style>`, `hover_style: Option<Style>`, `disabled_style: Option<Style>`

## ConfirmInput

A question answered by y, n or Enter.

API:

- `rtwk::controls::ConfirmInput` (struct, `crates/rtwk-controls/src/confirm.rs:63`) A yes-or-no question: `prompt`, then the answers `yes` and `no` to press, which tell `on_confirmed`.
  - public fields: `prompt: Str`, `on_confirmed: Option<Handler<bool>>`, `default_value: bool`, `yes: Option<Str>`, `no: Option<Str>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `style: Option<Style>`, `answer_style: Option<Style>`, `highlighted_style: Option<Style>`, `focused_style: Option<Style>`, `disabled_style: Option<Style>`

### Code

The gallery's Code tab at the initial options (route `#/controls/confirminput`, `crates/rtwk-gallery/src/pages/mod.rs:301`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
AnyWidget::from(
    ConfirmInput::new("Delete 3 files?")
        .default_value(true)
        .yes("Yes")
        .no("No")
        .on_confirmed(record_answer),
)
.with_key(Key::value(true))
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| default | yes, no | yes |
| labels | Yes, No, Delete, Keep | Yes, No |

### Methods

#### `ConfirmInput`: constructors and associated functions

- `pub fn new(prompt: impl Into<Str>) -> ConfirmInput` - Asks `prompt`; disabled until `on_confirmed` is set.

#### `ConfirmInput`: builder methods (take and return `self`)

- `pub fn on_confirmed(mut self, h: Handler<bool>) -> ConfirmInput` - Told the answer.
- `pub fn default_value(mut self, value: bool) -> ConfirmInput` - The answer highlighted at first.
- `pub fn yes(mut self, label: impl Into<Str>) -> ConfirmInput` - With `label` for yes.
- `pub fn no(mut self, label: impl Into<Str>) -> ConfirmInput` - With `label` for no.
- `pub fn focus_node(mut self, node: Option<FocusNode>) -> ConfirmInput` - With `node` as its node of the focus tree.
- `pub fn autofocus(mut self, on: bool) -> ConfirmInput` - Taking the focus when mounted.
- `pub fn style(mut self, style: Style) -> ConfirmInput` - The row in `style`.
- `pub fn answer_style(mut self, style: Style) -> ConfirmInput` - The answer not highlighted in `style`.
- `pub fn highlighted_style(mut self, style: Style) -> ConfirmInput` - The answer highlighted in `style`.
- `pub fn focused_style(mut self, style: Style) -> ConfirmInput` - The answer highlighted and focused in `style`.
- `pub fn disabled_style(mut self, style: Style) -> ConfirmInput` - The row while disabled in `style`.

## Slider

A value along a track: the arrows, a press or a drag move the thumb, which stands in eighths of a cell.

API:

- `rtwk::controls::Slider` (struct, `crates/rtwk-controls/src/slider.rs:88`) A value from `min` to `max` chosen along a track, by the keys or by dragging its thumb.
  - public fields: `value: f64`, `on_changed: Option<Handler<f64>>`, `on_change_start: Option<Handler<f64>>`, `on_change_end: Option<Handler<f64>>`, `min: f64`, `max: f64`, `divisions: Option<u32>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `style: Option<Style>`, `focused_style: Option<Style>`, `track_style: Option<Style>`, `disabled_style: Option<Style>`, `semantics_label: Option<Str>`

### Code

The gallery's Code tab at the initial options (route `#/controls/slider`, `crates/rtwk-gallery/src/pages/mod.rs:387`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
let slider = Slider::new(self.value)
    .semantics_label("Volume")
    .on_changed(cx.map(|value| value));

SizedBox::new(Some(30), None, Some(slider.into()))
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| divisions | none, 10, 4 | none |
| width | 10..40 (a slider) | 30 cells |

### Methods

#### `Slider`: constructors and associated functions

- `pub fn new(value: f64) -> Slider` - A slider of `value` from 0 to 1, disabled until `on_changed` is set.

#### `Slider`: builder methods (take and return `self`)

- `pub fn on_changed(mut self, h: Handler<f64>) -> Slider` - Told the value the slider is to take.
- `pub fn on_change_start(mut self, h: Handler<f64>) -> Slider` - Told the value the slider has when a press of the mouse starts a change.
- `pub fn on_change_end(mut self, h: Handler<f64>) -> Slider` - Told the value the slider has when the press that changed it ends: a value to save once, not on every step of the drag.
- `pub fn min(mut self, min: f64) -> Slider` - The value at the start of the track.
- `pub fn max(mut self, max: f64) -> Slider` - The value at the end of the track.
- `pub fn divisions(mut self, divisions: u32) -> Slider` - The values on `divisions` even steps from `min` to `max`.
- `pub fn focus_node(mut self, node: Option<FocusNode>) -> Slider` - With `node` as its node of the focus tree.
- `pub fn autofocus(mut self, on: bool) -> Slider` - Taking the focus when mounted.
- `pub fn style(mut self, style: Style) -> Slider` - The thumb and the line before it in `style`.
- `pub fn focused_style(mut self, style: Style) -> Slider` - The thumb and the line before it in `style` while it has the focus.
- `pub fn track_style(mut self, style: Style) -> Slider` - The line after the thumb in `style`.
- `pub fn disabled_style(mut self, style: Style) -> Slider` - All of it in `style` while it is disabled.
- `pub fn semantics_label(mut self, label: impl Into<Str>) -> Slider` - What a screen reader calls the slider.

## NumberInput

A number typed, or stepped by Up and Down, the wheel or a click on ▴▾; kept within its bounds.

API:

- `rtwk::controls::NumberInput` (struct, `crates/rtwk-controls/src/number_input.rs:93`) A field for a number from `min` to `max`, typed, or stepped by `step` with the keys, the wheel and the arrows `▴▾` after it.
  - public fields: `value: f64`, `on_changed: Option<Handler<f64>>`, `min: Option<f64>`, `max: Option<f64>`, `step: f64`, `decimals: usize`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `style: Option<Style>`, `focused_style: Option<Style>`, `arrow_style: Option<Style>`, `error_style: Option<Style>`

### Code

The gallery's Code tab at the initial options (route `#/controls/numberinput`, `crates/rtwk-gallery/src/pages/mod.rs:629`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
let copies = NumberInput::new(copies)
    .min(1.0)
    .max(99.0)
    .on_changed(copies_changed);

let price = NumberInput::new(price)
    .min(0.0)
    .step(0.25)
    .decimals(2)
    .on_changed(price_changed);
```

### Methods

#### `NumberInput`: constructors and associated functions

- `pub fn new(value: f64) -> NumberInput` - A field for `value`, disabled until `on_changed` is set; steps of 1, no digits after the point, no bounds.

#### `NumberInput`: builder methods (take and return `self`)

- `pub fn on_changed(mut self, h: Handler<f64>) -> NumberInput` - Told the number the field is to take.
- `pub fn min(mut self, min: f64) -> NumberInput` - With `min` the least number.
- `pub fn max(mut self, max: f64) -> NumberInput` - With `max` the greatest number.
- `pub fn step(mut self, step: f64) -> NumberInput` - Stepping by `step`, above 0.
- `pub fn decimals(mut self, decimals: usize) -> NumberInput` - Showing `decimals` digits after the point, from 0 to 20.
- `pub fn focus_node(mut self, node: Option<FocusNode>) -> NumberInput` - With `node` as its node of the focus tree.
- `pub fn autofocus(mut self, on: bool) -> NumberInput` - Taking the focus when mounted.
- `pub fn style(mut self, style: Style) -> NumberInput` - The field in `style`.
- `pub fn focused_style(mut self, style: Style) -> NumberInput` - The field in `style` while it has the focus.
- `pub fn arrow_style(mut self, style: Style) -> NumberInput` - `▴▾` in `style` while the field has not the focus.
- `pub fn error_style(mut self, style: Style) -> NumberInput` - The field in `style` while its text is wrong.

## DatePicker

A date typed, stepped by Up and Down, or picked from the calendar that Alt+Down, a click on ▾ or a PopupController opens.

API:

- `rtwk::controls::DatePicker` (struct, `crates/rtwk-controls/src/date_picker.rs:104`) A field for a date, typed, stepped with the keys or picked from a `Calendar` that opens under it.
  - public fields: `value: Option<DateTime>`, `on_changed: Option<Handler<Option<DateTime>>>`, `controller: Option<PopupController>`, `first_date: Option<DateTime>`, `last_date: Option<DateTime>`, `show_time: bool`, `today: Option<DateTime>`, `first_weekday: u8`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `style: Option<Style>`, `focused_style: Option<Style>`, `arrow_style: Option<Style>`, `error_style: Option<Style>`, `popup_look: PanelLook`, `popup_fade_in: Duration`, `cursor_style: Option<Style>`

### Code

The gallery's Code tab at the initial options (route `#/controls/datepicker`, `crates/rtwk-gallery/src/pages/mod.rs:636`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
let picker = DatePicker::new(date, Some(date_changed))
    .controller(calendar.clone())
    .first_date(DateTime::new(2026, 1, 1))
    .last_date(DateTime::new(2026, 12, 31))
    .today(DateTime::new(2026, 10, 19))
    .show_time(false)
    .first_weekday(DateTime::MONDAY)
    .popup_look(PanelLook::Raised);

row![
    SizedBox::new(Some(22), None, Some(picker.into())),
    Button::new("Calendar").on_pressed(open_calendar),
]
.spacing(2)
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| time | on, off | false |
| week | monday, sunday | monday |
| popup | raised, bordered | raised |

### Methods

#### `DatePicker`: constructors and associated functions

- `pub fn new(value: Option<DateTime>, on_changed: Option<Handler<Option<DateTime>>>) -> DatePicker` - A field for `value`, which `on_changed` changes; disabled when that is none.

#### `DatePicker`: builder methods (take and return `self`)

- `pub fn controller(mut self, controller: impl Into<PopupController>) -> Self` - Opens and closes the calendar, and says whether it is open.
- `pub fn first_date(mut self, date: DateTime) -> Self` - The earliest day it takes.
- `pub fn last_date(mut self, date: DateTime) -> Self` - The latest day it takes.
- `pub fn show_time(mut self, on: bool) -> Self` - Whether the field shows and takes the time of day.
- `pub fn today(mut self, date: DateTime) -> Self` - The date the calendar shows as today.
- `pub fn first_weekday(mut self, weekday: u8) -> Self` - The weekday the calendar's rows start with.
- `pub fn focus_node(mut self, node: FocusNode) -> Self` - The node the field takes the focus with.
- `pub fn autofocus(mut self, on: bool) -> Self` - Whether the field takes the focus as it is first built.
- `pub fn style(mut self, style: Style) -> Self` - The field, over the theme's surface and text.
- `pub fn focused_style(mut self, style: Style) -> Self` - The field while it has the focus.
- `pub fn arrow_style(mut self, style: Style) -> Self` - `▾` while the field has not the focus and the calendar is closed.
- `pub fn error_style(mut self, style: Style) -> Self` - The field while its text is no date, or one beyond the bounds.
- `pub fn popup_look(mut self, look: PanelLook) -> Self` - How the calendar's popup stands out of what is under it.
- `pub fn popup_fade_in(mut self, d: Duration) -> Self` - How long the calendar's popup takes to fade in.
- `pub fn cursor_style(mut self, style: Style) -> Self` - The calendar's cursor.

### Also used in the code

- `rtwk::data::DateTime` (struct, `crates/rtwk-data/src/date_time.rs:35`) A civil local date and time to the microsecond, the constructors normalising as Dart's `DateTime(year, month, day)` does: a month past 12 carries into the year, a day past the month's length (or below 1) into the months ...
- `rtwk::overlay::PanelLook` (enum, `crates/rtwk-overlay-base/src/panel.rs:13`) How a floating panel stands out of what is under it.
  - variants: `Raised`, `Bordered`
- `rtwk::controls::Button` (struct, `crates/rtwk-controls/src/button.rs:107`) A button that tells `on_pressed` when it is clicked, or when Enter or Space is pressed while it has the focus.
  - public fields: `spans: Option<Rc<[Span]>>`, `tall: bool`, `on_pressed: Option<Handler<()>>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: bool`, `style: Option<Style>`, `focused_style: Option<Style>`, `hover_style: Option<Style>`, `disabled_style: Option<Style>`

## FilePicker

A file picked from the directory listed under a field that names its path: the arrows and Enter go through it, Tab completes a name; a FilePickerController goes to a directory.

API:

- `rtwk::controls::FilePicker` (struct, `crates/rtwk-controls/src/file_picker.rs:1122`) A file browser: a directory's entries to pick from, under a field that names a path, which calls `on_picked` with the paths picked.
  - public fields: `on_picked: Handler<Vec<String>>`, `on_cancelled: Option<Handler<()>>`, `controller: Option<FilePickerController>`, `mode: FilePickerMode`, `directory: Option<String>`, `file_system: SharedFileSystem`, `name: String`, `filters: Vec<FileFilter>`, `show_hidden: bool`, `multiple: bool`, `autofocus: bool`, `bar: bool`, `cursor_style: Option<Style>`, `idle_style: Option<Style>`, `field_style: Option<Style>`, `field_focused_style: Option<Style>`

### Code

The gallery's Code tab at the initial options (route `#/controls/filepicker`, `crates/rtwk-gallery/src/pages/mod.rs:665`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
let filters = vec![
    FileFilter::new("Dart").extensions(["dart"]),
    FileFilter::new("All files"),
];
let picker = FilePicker::new(on_picked)
    .controller(files.clone())
    .file_system(demo_files.clone())
    .mode(FilePickerMode::Open)
    .multiple(false)
    .show_hidden(false)
    .filters(filters.clone());

// Open from a handler, then await the result in a task.
let result = show_file_picker(
    cx,
    FilePickerOptions::new()
        .file_system(demo_files.clone())
        .mode(FilePickerMode::Open)
        .multiple(false)
        .show_hidden(false)
        .filters(filters),
);
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| mode | open, several, save, directory | open |
| hidden | on, off | false |

### Methods

#### `FilePicker`: constructors and associated functions

- `pub fn new(on_picked: Handler<Vec<String>>) -> FilePicker` - A picker of the machine's file system, which calls `on_picked` with what is picked.

#### `FilePicker`: builder methods (take and return `self`)

- `pub fn on_cancelled(mut self, h: Handler<()>) -> Self` - Called by Cancel; none for no Cancel.
- `pub fn controller(mut self, controller: FilePickerController) -> Self` - Reads what it shows and moves it from outside.
- `pub fn mode(mut self, mode: FilePickerMode) -> Self` - What it picks.
- `pub fn directory(mut self, directory: impl Into<String>) -> Self` - The directory shown at first, relative to the current one of the file system.
- `pub fn file_system(mut self, file_system: SharedFileSystem) -> Self` - What the directories are listed from.
- `pub fn name(mut self, name: impl Into<String>) -> Self` - The name in the field at first, after the directory.
- `pub fn filters(mut self, filters: Vec<FileFilter>) -> Self` - The filters to choose from.
- `pub fn show_hidden(mut self, on: bool) -> Self` - Whether the entries whose names start with a dot show at first.
- `pub fn multiple(mut self, on: bool) -> Self` - Whether Space marks files to pick together; the open mode only, which the picker asserts as it starts.
- `pub fn autofocus(mut self, on: bool) -> Self` - Whether the field takes the focus as it is first built.
- `pub fn bar(mut self, on: bool) -> Self` - Whether a column before the entries shows a bar on the cursor's.
- `pub fn cursor_style(mut self, style: Style) -> Self` - The cursor's entry while the list has the focus.
- `pub fn idle_style(mut self, style: Style) -> Self` - The cursor's entry while the list has not the focus.
- `pub fn field_style(mut self, style: Style) -> Self` - The field, over the theme's surface and text.
- `pub fn field_focused_style(mut self, style: Style) -> Self` - The field while it has the focus, over `field_style`.

### Also used in the code

- `rtwk::data::FileFilter` (struct, `crates/rtwk-data/src/picker_files.rs:46`) A choice of which files a file picker lists: those whose names end in a dot and one of `extensions`, in any case.
  - public fields: `label: String`, `extensions: Vec<String>`
- `rtwk::controls::FilePickerMode` (enum, `crates/rtwk-controls/src/file_picker.rs:61`) What a `FilePicker` picks.
  - variants: `Open`, `Save`, `Directory`
- `rtwk::controls::FilePickerOptions` (struct, `crates/rtwk-controls/src/file_picker.rs:770`) What `show_file_picker` shows a picker with: the parameters of the `FilePicker` it shows, and the dialog's own.
  - public fields: `mode: FilePickerMode`, `directory: Option<String>`, `file_system: SharedFileSystem`, `name: String`, `filters: Vec<FileFilter>`, `show_hidden: bool`, `multiple: bool`, `title: Option<Str>`, `width: i32`, `height: i32`

## Form, FormField

Fields validated and submitted together: Enter goes on to the next field, and from the last submits the form.

API:

- `rtwk::controls::Form` (struct, `crates/rtwk-controls/src/form.rs:110`) Keeps the `FormField`s below `child` together, to validate, save, reset and submit them as one.
  - public fields: `child: AnyWidget`, `on_submitted: Option<Handler<()>>`, `on_changed: Option<Handler<()>>`, `autovalidate_mode: AutovalidateMode`
- `rtwk::controls::FormField` (struct, `crates/rtwk-controls/src/form.rs:526`) A field of the nearest `Form` above it, which holds a value that its builder shows and edits.
  - public fields: `builder: FieldBuilder<T>`, `initial_value: Option<T>`, `validator: Option<FieldValidator<T>>`, `on_saved: Option<Handler<Option<T>>>`, `autovalidate_mode: Option<AutovalidateMode>`, `error_style: Option<Style>`

### Code

The gallery's Code tab at the initial options (route `#/controls/form-formfield`, `crates/rtwk-gallery/src/pages/mod.rs:816`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
// Build SignupFields below Form so its buttons can find Form::of(cx).
Form::new(SignupFields {
    on_saved: cx.map(SignupMsg::Name),
    result: result_line(" submitted: ", self.submitted.as_deref(), self.serial)
        .wrap(TextWrap::None)
        .into(),
})
.autovalidate_mode(AutovalidateMode::Disabled)
.on_submitted(cx.send(SignupMsg::Submitted))

// SignupFieldsState::update
let form = Form::of(cx);
match msg {
    FormCommand::Submit => { form.submit(cx); }
    FormCommand::Reset => form.reset(cx),
}

// SignupFieldsState::build
let fields = column![
    TextFormField::new()
        .placeholder("name")
        .validator(|name| name.is_none_or(String::is_empty)
            .then(|| "a name, please".into()))
        .on_saved(widget.on_saved.clone()),
    TextFormField::new()
        .placeholder("email")
        .validator(|email| (!email.is_some_and(|s| s.contains('@')))
            .then(|| "an email has an @ in it".into())),
    FormField::new(|field| {
        Checkbox::new(field.value().unwrap_or(false))
            .label("I accept the terms")
            .on_changed(field.on_changed())
            .into()
    })
    .initial_value(false)
    .validator(|accepted| (accepted != Some(&true))
        .then(|| "the terms are to be accepted".into())),
    row![
        Button::new("Submit").on_pressed(cx.send(FormCommand::Submit)),
        Button::new("Reset").on_pressed(cx.send(FormCommand::Reset)),
        Expanded::new(1, widget.result.clone()),
    ].spacing(2),
]
.main_axis_size(MainAxisSize::Min)
.cross_axis_alignment(CrossAxisAlignment::Stretch)
.spacing(1);
SizedBox::new(Some(40), None, Some(fields.into()))
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| validate | disabled, always, onUserInteraction, onUnfocus | disabled |

### Methods

#### `Form`: constructors and associated functions

- `pub fn new(child: impl Into<AnyWidget>) -> Form` - A form of the fields below `child`.
- `pub fn of<S>(cx: &Cx<'_, S>) -> FormState` - The state of the nearest `Form` above `cx`, without depending on it.
- `pub fn maybe_of<S>(cx: &Cx<'_, S>) -> Option<FormState>` - The state `of` finds, if there is one.

#### `Form`: builder methods (take and return `self`)

- `pub fn on_submitted(mut self, h: Handler<()>) -> Form` - Told by a submit that found every field valid.
- `pub fn on_changed(mut self, h: Handler<()>) -> Form` - Told when the value of a field changed and after a reset.
- `pub fn autovalidate_mode(mut self, mode: AutovalidateMode) -> Form` - With the fields validating as `mode` says, those that do not say.

#### `FormField`: constructors and associated functions

- `pub fn new(builder: impl Fn(&FormFieldState<T>) -> AnyWidget + 'static) -> FormField<T>` - A field of the value `builder` shows.
- `pub fn with_context(builder: impl Fn(&mut BuildCx<'_>, &FormFieldState<T>) -> AnyWidget + 'static) -> FormField<T>` - As `new`, the builder given the context of the field's element too, to read the theme and the like from.

#### `FormField`: builder methods (take and return `self`)

- `pub fn initial_value(mut self, value: T) -> FormField<T>` - The value the field starts with and a reset gives back.
- `pub fn validator(mut self, f: impl Fn(Option<&T>) -> Option<Str> + 'static) -> FormField<T>` - The error for a value, `None` for a valid one.
- `pub fn on_saved(mut self, h: Handler<Option<T>>) -> FormField<T>` - Told the value when the form saves.
- `pub fn autovalidate_mode(mut self, mode: AutovalidateMode) -> FormField<T>` - When the field validates on its own.
- `pub fn error_style(mut self, style: Style) -> FormField<T>` - The error in `style` over the theme's text.

### Also used in the code

- `rtwk::text::TextWrap` (enum, `crates/rtwk-render/src/objects/text.rs:55`) Where a `RenderParagraph` may break a line too long for it.
  - variants: `Word`, `Grapheme`, `None`
- `rtwk::controls::AutovalidateMode` (enum, `crates/rtwk-controls/src/form.rs:44`) When a `FormField` validates its value on its own, besides when its `Form` validates (`FormState::validate`, `FormState::submit`) or it is told to (`FormFieldState::validate`).
  - variants: `Disabled`, `Always`, `OnUserInteraction`, `OnUnfocus`
- `rtwk::controls::TextFormField` (struct, `crates/rtwk-controls/src/form.rs:1189`) A `FormField` of the text a `TextField` edits.
  - public fields: `controller: Option<TextEditingController>`, `initial_value: Option<String>`, `validator: Option<FieldValidator<String>>`, `on_saved: Option<Handler<Option<String>>>`, `autovalidate_mode: Option<AutovalidateMode>`, `error_style: Option<Style>`, `placeholder: Option<Str>`, `semantics_label: Option<Str>`, `obscure_text: bool`, `obscuring_character: Option<Str>`, `max_length: Option<usize>`, `on_changed: Option<Handler<Rc<str>>>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: Option<bool>`, `style: Option<Style>`, `focused_style: Option<Style>`, `placeholder_style: Option<Style>`, `selection_style: Option<Style>`, `decoration: Option<InputDecoration>`
- `rtwk::controls::Checkbox` (struct, `crates/rtwk-controls/src/toggle.rs:64`) A checkbox before `label`: `[x]` while `value` is true, `[ ]` while it is false.
  - public fields: `value: bool`, `on_changed: Option<Handler<bool>>`, `label: Str`, `semantics_label: Option<Str>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `style: Option<Style>`, `bracket_style: Option<Style>`, `focused_style: Option<Style>`, `disabled_style: Option<Style>`
- `rtwk::controls::Button` (struct, `crates/rtwk-controls/src/button.rs:107`) A button that tells `on_pressed` when it is clicked, or when Enter or Space is pressed while it has the focus.
  - public fields: `spans: Option<Rc<[Span]>>`, `tall: bool`, `on_pressed: Option<Handler<()>>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: bool`, `style: Option<Style>`, `focused_style: Option<Style>`, `hover_style: Option<Style>`, `disabled_style: Option<Style>`
- `rtwk::layout::Expanded` (struct, `crates/rtwk-widgets/src/layout/flex.rs:270`) `Flexible` with `FlexFit::Tight`: the child fills its part (twk's `Expanded`).

## TabBar, TabBarView

Tabs to pick a view by: Left and Right, or a click.

API:

- `rtwk::controls::TabBar` (struct, `crates/rtwk-controls/src/tabs.rs:237`) A row of the labels of the `controller`'s tabs, its selected one marked, which the keys and the mouse switch.
  - public fields: `controller: TabController`, `tabs: Rc<[Str]>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `style: Option<Style>`, `tab_style: Option<Style>`, `selected_style: Option<Style>`, `focused_style: Option<Style>`
- `rtwk::controls::TabBarView` (struct, `crates/rtwk-controls/src/tabs.rs:1008`) Shows the content of the `controller`'s selected tab, and keeps the others with their state.
  - public fields: `controller: TabController`, `children: Vec<AnyWidget>`

### Code

The gallery's Code tab at the initial options (route `#/controls/tabbar-tabbarview`, `crates/rtwk-gallery/src/pages/mod.rs:829`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
// Keep TabController::new(3) in state.
// Recreate it when the tab count changes.
let bar = TabBar::new(&self.tabs, ["General", "Editor", "Keys"]);
let general = column![
    Checkbox::new(self.wrap).label("wrap lines").on_changed(cx.map(SettingsMsg::Wrap)),
    Checkbox::new(self.hidden).label("show hidden files").on_changed(cx.map(SettingsMsg::Hidden)),
]
.cross_axis_alignment(CrossAxisAlignment::Start);
let editor = column![
    Text::new("Indent with"),
    RadioGroup::new([Choice::new("spaces", "spaces"), Choice::new("tabs", "tabs")],
        Some(self.indent))
        .direction(Axis::Horizontal)
        .on_changed(cx.map(SettingsMsg::Indent)),
]
.cross_axis_alignment(CrossAxisAlignment::Start);
let view = TabBarView::new(&self.tabs, [
    general.into(),
    editor.into(),
    Text::new("ctrl+s saves the file.\nctrl+q quits.").into(),
]);
let view = FocusBuilder::new(|cx, focused, child| {
    let theme = Theme::of(cx);
    let style = if *focused { &theme.primary } else { &theme.border };
    BoxWidget::new()
        .border(Border::all(style.clone()).kind(BorderType::Rounded))
        .padding(EdgeInsets::x(1))
        .child(child.expect("tab view child").clone())
        .into()
}).child(view);

SizedBox::new(
    Some(40),
    Some(7),
    Some(column![bar, Expanded::new(1, view)]
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .into()),
)
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| tabs | 3, 7 | 3 |
| look | plain, strip | plain |

### Methods

#### `TabBar`: constructors and associated functions

- `pub fn new<S: Into<Str>>(controller: impl Into<TabController>, tabs: impl IntoIterator<Item = S>) -> TabBar` - A tab bar of `tabs`, marked in the theme's primary color.
- `pub fn strip<S: Into<Str>>(controller: impl Into<TabController>, tabs: impl IntoIterator<Item = S>) -> TabBar` - A tab bar on a strip: the styles left none are the theme's surface for the row, `ThemeData::muted` for a tab, `ThemeData::primary` in bold on the theme's background for the selected tab, and `ThemeData::on_primary` in ...

#### `TabBar`: builder methods (take and return `self`)

- `pub fn focus_node(mut self, node: Option<FocusNode>) -> TabBar` - With `node` as its node of the focus tree.
- `pub fn autofocus(mut self, on: bool) -> TabBar` - Taking the focus when mounted.
- `pub fn style(mut self, style: Style) -> TabBar` - The row in `style`.
- `pub fn tab_style(mut self, style: Style) -> TabBar` - A tab, and the marks of the edges, in `style`.
- `pub fn selected_style(mut self, style: Style) -> TabBar` - The selected tab in `style`.
- `pub fn focused_style(mut self, style: Style) -> TabBar` - The selected tab in `style` while the bar has the focus.

#### `TabBarView`: constructors and associated functions

- `pub fn new(controller: impl Into<TabController>, children: impl IntoIterator<Item = AnyWidget>) -> TabBarView` - A view of the `children` of the `controller`'s tabs.

### Also used in the code

- `rtwk::controls::TabController` (struct, `crates/rtwk-controls/src/tabs.rs:63`) Holds which of `length` tabs is selected: `index`, which a `TabBar` sets and a `TabBarView` shows.
- `rtwk::controls::Checkbox` (struct, `crates/rtwk-controls/src/toggle.rs:64`) A checkbox before `label`: `[x]` while `value` is true, `[ ]` while it is false.
  - public fields: `value: bool`, `on_changed: Option<Handler<bool>>`, `label: Str`, `semantics_label: Option<Str>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `style: Option<Style>`, `bracket_style: Option<Style>`, `focused_style: Option<Style>`, `disabled_style: Option<Style>`
- `rtwk::controls::RadioGroup` (struct, `crates/rtwk-controls/src/choice.rs:193`) Radio buttons: `choices`, one of which is chosen, each its label after a mark, `(●)` for the one whose value is `value`, `( )` for the others (`Glyphs::radio_on`, `radio_off` and `radio_brackets` of the theme).
  - public fields: `choices: Rc<[Choice<T>]>`, `value: Option<T>`, `on_changed: Option<Handler<T>>`, `direction: Axis`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: bool`, `mark: Option<Str>`, `style: Option<Style>`, `bracket_style: Option<Style>`, `focused_style: Option<Style>`, `disabled_style: Option<Style>`
- `rtwk::controls::Choice` (struct, `crates/rtwk-controls/src/choice.rs:68`) One of the values a `RadioGroup`, a `Select` or a `MultiSelect` offers: `value`, shown as `label`.
  - public fields: `value: T`, `label: Str`, `enabled: bool`
- `rtwk::layout::Axis` (enum, `crates/rtwk-render/src/objects/decoration.rs:39`) Along which axis something runs: across or down.
  - variants: `Horizontal`, `Vertical`
- `rtwk::focus::FocusBuilder` (struct, `crates/rtwk-framework/src/focus/widgets.rs:905`) Shows what its builder returns for whether the focus is in this widget's subtree, built again when the focus enters or leaves it; the widgets around it are not (twk's `FocusBuilder`).
  - public fields: `builder: ValueWidgetBuilder<bool>`, `child: Option<AnyWidget>`, `debug_label: Option<Str>`
- `rtwk::layout::Expanded` (struct, `crates/rtwk-widgets/src/layout/flex.rs:270`) `Flexible` with `FlexFit::Tight`: the child fills its part (twk's `Expanded`).

## KeyHintsBar

The keys that can be pressed now, with their hints, as the focus moves: Tab to the field to see its keys.

API:

- `rtwk::controls::KeyHintsBar` (struct, `crates/rtwk-controls/src/key_hints.rs:104`) A row of the keys that can be pressed now, each before its hint: `ctrl+s save q quit`.
  - public fields: `show_unhinted: bool`, `key_label: Option<KeyLabel>`, `join_keys: bool`, `separator: Str`, `style: Option<Style>`, `key_style: Option<Style>`, `hint_style: Option<Style>`, `pressable: bool`

### Code

The gallery's Code tab at the initial options (route `#/controls/keyhintsbar`, `crates/rtwk-gallery/src/pages/mod.rs:851`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
let theme = Theme::of(cx);
let hints = KeyHintsBar::new();
let body = column![
    row![
        Expanded::new(1, TextField::new().placeholder("a field")
            .focused_style(theme.selection.clone())),
        Checkbox::new(self.autosave).label("autosave")
            .on_changed(cx.map(HintedMsg::Autosave)),
    ].spacing(2),
    result_line("did: ", self.did, self.serial),
    BoxWidget::new()
        .border(Border::all(theme.border.clone()).kind(BorderType::Rounded))
        .padding(EdgeInsets::x(1))
        .child(hints),
]
.main_axis_size(MainAxisSize::Min)
.cross_axis_alignment(CrossAxisAlignment::Stretch)
.spacing(1);
KeyBindings::new(
    [
        ("ctrl+s", cx.send(HintedMsg::Act("save"))),
        ("ctrl+x ctrl+o", cx.send(HintedMsg::Act("open"))),
        ("f2", cx.send(HintedMsg::Act("rename"))),
    ],
    SizedBox::new(Some(44), None, Some(body.into())),
)
.hints([("ctrl+s", "save"), ("ctrl+x ctrl+o", "open")])
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| unhinted | on, off | false |
| join | on, off | true |
| keys | signs, names | signs |
| separator |    ,  · ,  │  |     |

### Methods

#### `KeyHintsBar`: constructors and associated functions

- `pub fn new() -> KeyHintsBar` - A row of the keys of the key path and their hints.
- `pub fn from_pairs<K, H>(pairs: impl IntoIterator<Item = (K, H)>) -> KeyHintsBar where K: Into<Str>, H: Into<Str>,` - A row of the keys `pairs` name, each before its hint, whatever keys are in force: for hints the app words itself, as they change with its state (`enter open d delete`).
- `pub fn signs(combo: &KeyCombo, glyphs: &Glyphs) -> String` - `combo` as a hint names it: the arrows and Escape by their signs, Page Up and Down short, after the modifiers too (`alt+↓`, `esc`, `pgdn`); any other key as `KeyCombo`'s `Display` does (`ctrl+s`).
- `pub fn entries(keys: &[ActiveKey], key_label: Option<&dyn Fn(&KeyCombo) -> String>, join_keys: bool, show_unhinted: bool, glyphs: &Glyphs) -> Vec<(String, Option<Str>)>` - The entries a bar shows for `keys`, in their order: each the keys it names, as `ActiveKey::label` names them by `key_label` (`KeyHintsBar::signs` when none), and their hint.

#### `KeyHintsBar`: builder methods (take and return `self`)

- `pub fn show_unhinted(mut self, on: bool) -> KeyHintsBar` - Whether keys without a hint are shown.
- `pub fn key_label(mut self, label: impl Fn(&KeyCombo) -> String + 'static) -> KeyHintsBar` - With each key named by `label`.
- `pub fn key_label_rc(mut self, label: KeyLabel) -> KeyHintsBar` - With each key named by `label` (shared).
- `pub fn join_keys(mut self, on: bool) -> KeyHintsBar` - Whether keys of one family that share a hint show as one.
- `pub fn separator(mut self, separator: impl Into<Str>) -> KeyHintsBar` - With `separator` between two entries.
- `pub fn style(mut self, style: Style) -> KeyHintsBar` - The row in `style`, over the theme's text.
- `pub fn key_style(mut self, style: Style) -> KeyHintsBar` - A key in `style`.
- `pub fn hint_style(mut self, style: Style) -> KeyHintsBar` - A hint in `style`.
- `pub fn pressable(mut self, on: bool) -> KeyHintsBar` - Whether a click presses a key.

### Also used in the code

- `rtwk::layout::Expanded` (struct, `crates/rtwk-widgets/src/layout/flex.rs:270`) `Flexible` with `FlexFit::Tight`: the child fills its part (twk's `Expanded`).
- `rtwk::editing::TextField` (struct, `crates/rtwk-editing/src/text_field.rs:160`) A field of a line of text the user edits, which its `controller` holds (twk's `TextField`).
  - public fields: `controller: Option<TextEditingController>`, `placeholder: Option<Str>`, `semantics_label: Option<Str>`, `obscure_text: bool`, `obscuring_character: Option<Str>`, `max_length: Option<usize>`, `on_changed: Option<Handler<Rc<str>>>`, `on_submitted: Option<Handler<Rc<str>>>`, `on_editing_complete: Option<Handler<()>>`, `on_paste: Option<OnPaste>`, `read_only: bool`, `enabled: bool`, `text_align: TextAlign`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: Option<bool>`, `style: Option<Style>`, `focused_style: Option<Style>`, `placeholder_style: Option<Style>`, `selection_style: Option<Style>`
- `rtwk::controls::Checkbox` (struct, `crates/rtwk-controls/src/toggle.rs:64`) A checkbox before `label`: `[x]` while `value` is true, `[ ]` while it is false.
  - public fields: `value: bool`, `on_changed: Option<Handler<bool>>`, `label: Str`, `semantics_label: Option<Str>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `style: Option<Style>`, `bracket_style: Option<Style>`, `focused_style: Option<Style>`, `disabled_style: Option<Style>`
- `rtwk::focus::KeyBindings` (struct, `crates/rtwk-framework/src/focus/shortcuts.rs:1257`) Binds keys to handlers for the nodes below it.
  - public fields: `bindings: Vec<(Str`, `combos: Vec<(KeyCombo`, `hints: Vec<(Str`, `combo_hints: Vec<(KeyCombo`, `sequence_timeout: Option<Duration>`, `unmatched: UnmatchedKeys`, `child: AnyWidget`, `debug_label: Option<Str>`

## StatusMessage, Badge

A message of a kind after its icon, in its color; a short label on its color.

API:

- `rtwk::controls::StatusMessage` (struct, `crates/rtwk-controls/src/status.rs:110`) A `message` of a `kind` after the kind's `icon` in its color: `✓ Saved 3 files`.
  - public fields: `message: Str`, `kind: StatusKind`, `title: Option<Str>`, `icon: Option<Str>`, `style: Option<Style>`, `icon_style: Option<Style>`
- `rtwk::controls::Badge` (struct, `crates/rtwk-controls/src/status.rs:231`) A short `label` set off by its color: a count, a state; ` 3 `, ` new `.
  - public fields: `label: Str`, `kind: Option<StatusKind>`, `style: Option<Style>`

### Code

The gallery's Code tab at the initial options (route `#/controls/statusmessage-badge`, `crates/rtwk-gallery/src/pages/mod.rs:687`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
column![
    StatusMessage::new("Saving runs in the background.").kind(StatusKind::Info),
    StatusMessage::new("The disk has 120 GB free.").kind(StatusKind::Info).title("Disk"),
    row![
        Badge::new("info").kind(StatusKind::Info),
        Badge::new("success").kind(StatusKind::Success),
        Badge::new("warning").kind(StatusKind::Warning),
        Badge::new("error").kind(StatusKind::Error),
        Badge::new("3"),
    ]
    .main_axis_size(MainAxisSize::Min)
    .spacing(1),
]
.main_axis_size(MainAxisSize::Min)
.cross_axis_alignment(CrossAxisAlignment::Start)
.spacing(1)
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| kind | info, success, warning, error | info |

### Methods

#### `StatusMessage`: constructors and associated functions

- `pub fn new(message: impl Into<Str>) -> StatusMessage` - A `message` of `Info`.

#### `StatusMessage`: builder methods (take and return `self`)

- `pub fn kind(mut self, kind: StatusKind) -> StatusMessage` - Of `kind`.
- `pub fn title(mut self, title: impl Into<Str>) -> StatusMessage` - With a `title` line above the message.
- `pub fn icon(mut self, icon: impl Into<Str>) -> StatusMessage` - With `icon` in place of the kind's.
- `pub fn style(mut self, style: Style) -> StatusMessage` - With the message in `style` over the theme's text.
- `pub fn icon_style(mut self, style: Style) -> StatusMessage` - With the icon and title in `style` over the kind's color.

#### `Badge`: constructors and associated functions

- `pub fn new(label: impl Into<Str>) -> Badge` - A badge of `label` in the primary color.

#### `Badge`: builder methods (take and return `self`)

- `pub fn kind(mut self, kind: StatusKind) -> Badge` - In the color of `kind`.
- `pub fn style(mut self, style: Style) -> Badge` - In `style` over the color filled and bold.

### Also used in the code

- `rtwk::controls::StatusKind` (enum, `crates/rtwk-controls/src/status.rs:21`) What a `StatusMessage` or a `Badge` tells: each kind has its `icon` (from the theme's `Glyphs`) and its color in the theme (`style_in`).
  - variants: `Info`, `Success`, `Warning`, `Error`

## Callout

A notice in a box lined in the color of its kind, its title on the top edge; [×] or Escape dismisses it, and buttons below act on it.

API:

- `rtwk::controls::Callout` (struct, `crates/rtwk-controls/src/callout.rs:87`) A notice in a box lined in the color of its `kind`: what a command-line tool prints for "a newer version is available", a hint, a fatal error.
  - public fields: `kind: StatusKind`, `title: Option<Str>`, `icon: Option<Str>`, `on_dismissed: Option<Handler<()>>`, `actions: Vec<AnyWidget>`, `style: Option<Style>`, `border_type: Option<BorderType>`, `title_style: Option<Style>`

### Code

The gallery's Code tab at the initial options (route `#/controls/callout`, `crates/rtwk-gallery/src/pages/mod.rs:700`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
let notice: AnyWidget = if self.shown {
    let notice = Callout::text("A newer version of twk is out: 0.3.0.")
        .kind(StatusKind::Info)
        .border_type(BorderType::Single)
        .on_dismissed(cx.send(NoticeMsg::Dismiss))
        .actions([Button::new("Update")
            .on_pressed(cx.send(NoticeMsg::Act("update")))
            .into()]);
    SizedBox::new(Some(44), None, Some(notice.into())).into()
} else {
    Button::new("Show it again")
        .on_pressed(cx.send(NoticeMsg::Restore))
        .into()
};

column![
    notice,
    LitLine::new(
        vec![
            ("did: ".into(), true),
            (self.did.unwrap_or("nothing").into(), self.did.is_none()),
        ],
        self.serial,
    ),
]
.main_axis_size(MainAxisSize::Min)
.cross_axis_alignment(CrossAxisAlignment::Start)
.spacing(1)
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| kind | info, success, warning, error | info |
| dismiss | on, off | true |
| actions | on, off | true |
| title | kind, own | kind |
| icon | kind, own, none | kind |
| border | single, rounded, heavy | single |
| width | 24..60 (a slider) | 44 cells |

### Methods

#### `Callout`: constructors and associated functions

- `pub fn new(child: impl Into<AnyWidget>) -> Callout` - A notice of `child` in a box of `Info`.
- `pub fn text(message: impl Into<Str>) -> Callout` - A notice of a message, a `Text` that wraps, in a box of `Info`.

#### `Callout`: builder methods (take and return `self`)

- `pub fn kind(mut self, kind: StatusKind) -> Callout` - Of `kind`.
- `pub fn title(mut self, title: impl Into<Str>) -> Callout` - With `title` on the top edge.
- `pub fn icon(mut self, icon: impl Into<Str>) -> Callout` - With `icon` in place of the kind's.
- `pub fn on_dismissed(mut self, h: Handler<()>) -> Callout` - Told when it is dismissed.
- `pub fn actions(mut self, actions: impl IntoIterator<Item = AnyWidget>) -> Callout` - With `actions` below the child.
- `pub fn style(mut self, style: Style) -> Callout` - The box and the child's text in `style`.
- `pub fn border_type(mut self, ty: BorderType) -> Callout` - The lines of the box.
- `pub fn title_style(mut self, style: Style) -> Callout` - The title in `style` over the kind's color in bold.

#### `Callout`: methods

- `pub fn child(&self) -> AnyWidget` - What it says: the child given, or a `Text` of the message given to `Callout::text`.

### Also used in the code

- `rtwk::controls::StatusKind` (enum, `crates/rtwk-controls/src/status.rs:21`) What a `StatusMessage` or a `Badge` tells: each kind has its `icon` (from the theme's `Glyphs`) and its color in the theme (`style_in`).
  - variants: `Info`, `Success`, `Warning`, `Error`
- `rtwk::controls::Button` (struct, `crates/rtwk-controls/src/button.rs:107`) A button that tells `on_pressed` when it is clicked, or when Enter or Space is pressed while it has the focus.
  - public fields: `spans: Option<Rc<[Span]>>`, `tall: bool`, `on_pressed: Option<Handler<()>>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: bool`, `style: Option<Style>`, `focused_style: Option<Style>`, `hover_style: Option<Style>`, `disabled_style: Option<Style>`

## Hyperlink

Text linked to a URL, which a terminal that knows hyperlinks opens.

API:

- `rtwk::controls::Hyperlink` (struct, `crates/rtwk-controls/src/link.rs:45`) Text linked to `url` as a hyperlink (OSC 8), which a terminal that knows them opens when it is clicked.
  - public fields: `url: Str`, `text: Option<Str>`, `style: Option<Style>`, `hover_style: Option<Style>`

### Code

The gallery's Code tab at the initial options (route `#/controls/hyperlink`, `crates/rtwk-gallery/src/pages/mod.rs:765`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
column![
    Hyperlink::new("https://dart.dev").text("The Dart language"),
    Hyperlink::new("https://pub.dev"),
    Hyperlink::new("file:///home/user/twk/README.md").text("the README"),
]
.main_axis_size(MainAxisSize::Min)
.cross_axis_alignment(CrossAxisAlignment::Start)
.spacing(1)
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| color | primary, accent, info | primary |
| text | label, url | label |

### Methods

#### `Hyperlink`: constructors and associated functions

- `pub fn new(url: impl Into<Str>) -> Hyperlink` - A link to `url` that shows the URL.

#### `Hyperlink`: builder methods (take and return `self`)

- `pub fn text(mut self, text: impl Into<Str>) -> Hyperlink` - Showing `text`.
- `pub fn style(mut self, style: Style) -> Hyperlink` - Drawn in `style`.
- `pub fn hover_style(mut self, style: Style) -> Hyperlink` - Over the style while the mouse is over it.

## ListTile

A row of a list: a title over a subtitle, something before and after them; tapped by a click or Enter, lit while focused or under the mouse.

API:

- `rtwk::controls::ListTile` (struct, `crates/rtwk-controls/src/list_tile.rs:63`) A row of a list: `title`, under it `subtitle`, `leading` before them and `trailing` after them, a column apart; tapped, by a click or by Enter or Space while it has the focus, it tells `on_tap`.
  - public fields: `title: AnyWidget`, `leading: Option<AnyWidget>`, `subtitle: Option<AnyWidget>`, `trailing: Option<AnyWidget>`, `on_tap: Option<Handler<()>>`, `selected: bool`, `enabled: bool`, `focus_node: Option<FocusNode>`, `autofocus: bool`

### Code

The gallery's Code tab at the initial options (route `#/controls/listtile`, `crates/rtwk-gallery/src/pages/mod.rs:787`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
let files = [
    ("notes.md", "2 KB, an hour ago"),
    ("todo.md", "640 B, today"),
    ("photo.png", "1.2 MB, last week"),
    ("locked.txt", "no access"),
];
let tiles = files.into_iter().enumerate().map(|(i, (name, about))| {
    ListTile::new(Text::new(name))
        .leading(Text::new("▤"))
        .subtitle(Text::new(about))
        .trailing(Text::new("›"))
        .selected(i == self.open)
        .enabled(i != 3)
        .on_tap(cx.send(i))
        .into()
});

SizedBox::new(
    Some(32),
    None,
    Some(Column::new()
        .main_axis_size(MainAxisSize::Min)
        .children(tiles)
        .into()),
)
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| leading | on, off | true |
| subtitle | on, off | true |
| trailing | on, off | true |

### Methods

#### `ListTile`: constructors and associated functions

- `pub fn new(title: impl Into<AnyWidget>) -> ListTile` - A row of `title`.

#### `ListTile`: builder methods (take and return `self`)

- `pub fn leading(mut self, w: impl Into<AnyWidget>) -> ListTile` - With `w` before the title.
- `pub fn subtitle(mut self, w: impl Into<AnyWidget>) -> ListTile` - With `w` under the title.
- `pub fn trailing(mut self, w: impl Into<AnyWidget>) -> ListTile` - With `w` at the right end.
- `pub fn on_tap(mut self, h: Handler<()>) -> ListTile` - Told when it is tapped.
- `pub fn selected(mut self, selected: bool) -> ListTile` - Whether it is the one chosen.
- `pub fn enabled(mut self, enabled: bool) -> ListTile` - Whether it can be tapped.
- `pub fn focus_node(mut self, node: Option<FocusNode>) -> ListTile` - With `node` as its node of the focus tree.
- `pub fn autofocus(mut self, on: bool) -> ListTile` - Taking the focus when mounted.

## StatusBar

A row of three sections, left, centre and right; those that do not fit give way, the centre first.

API:

- `rtwk::controls::StatusBar` (struct, `crates/rtwk-controls/src/status_bar.rs:60`) A status bar: one row of three sections, `left` at its left edge, `right` at its right edge and `center` in its middle.
  - public fields: `left: Option<AnyWidget>`, `center: Option<AnyWidget>`, `right: Option<AnyWidget>`, `spacing: i32`, `style: Option<Style>`

### Code

The gallery's Code tab at the initial options (route `#/controls/statusbar`, `crates/rtwk-gallery/src/pages/mod.rs:887`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
let theme = Theme::of(cx);
let bar = StatusBar::new()
    .spacing(1)
    .left(Badge::new("NORMAL"))
    .center(Text::new("example/widgets/main.dart")
        .wrap(TextWrap::None)
        .overflow(TextOverflow::Ellipsis))
    .right(Text::new("Ln 12, Col 4").style(theme.muted.clone()));
SizedBox::new(Some(50), None, Some(bar.into()))
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| width | 12..50 (a slider) | 50 cells |
| spacing | 0, 1, 3 | 1 |

### Methods

#### `StatusBar`: constructors and associated functions

- `pub fn new() -> StatusBar` - A bar of no sections yet, a gap of 1.

#### `StatusBar`: builder methods (take and return `self`)

- `pub fn left(mut self, w: impl Into<AnyWidget>) -> StatusBar` - With `w` at the left edge.
- `pub fn center(mut self, w: impl Into<AnyWidget>) -> StatusBar` - With `w` in the middle.
- `pub fn right(mut self, w: impl Into<AnyWidget>) -> StatusBar` - With `w` at the right edge.
- `pub fn spacing(mut self, spacing: i32) -> StatusBar` - With at least `spacing` columns between two sections.
- `pub fn style(mut self, style: Style) -> StatusBar` - The row in `style`, over the theme's surface and text.

### Also used in the code

- `rtwk::controls::Badge` (struct, `crates/rtwk-controls/src/status.rs:231`) A short `label` set off by its color: a count, a state; ` 3 `, ` new `.
  - public fields: `label: Str`, `kind: Option<StatusKind>`, `style: Option<Style>`
- `rtwk::text::TextWrap` (enum, `crates/rtwk-render/src/objects/text.rs:55`) Where a `RenderParagraph` may break a line too long for it.
  - variants: `Word`, `Grapheme`, `None`
- `rtwk::text::TextOverflow` (enum, `crates/rtwk-unicode/src/fit.rs:30`) How a paragraph (or `fit_text`) shows that it cut text off.
  - variants: `Clip`, `Ellipsis`, `EllipsisStart`, `EllipsisMiddle`
