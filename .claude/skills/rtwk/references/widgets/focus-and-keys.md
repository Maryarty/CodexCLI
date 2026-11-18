<!-- Written by tools/rtwk-port/skill/generate.py from rtwk 7482e340; do not edit. -->

# Focus and keys

[All widgets](../widgets.md)

## Focus, FocusScope

Nodes the keys go to: Tab moves the focus from one to the next, a click takes it; in a scope Tab goes round its nodes, and Tab into it focuses the one it had last.

API:

- `rtwk::focus::Focus` (struct, `crates/rtwk-framework/src/focus/widgets.rs:85`) Puts a `FocusNode` in the focus tree for its child, under the node of the nearest `Focus` or `FocusScope` above it: the `focus_node` given, or one of its own.
  - public fields: `id: Option<FocusId>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `on_focus_change: Option<Handler<bool>>`, `on_key_event: Option<FocusOnKeyEventCallback>`, `can_request_focus: Option<bool>`, `skip_traversal: Option<bool>`, `descendants_are_focusable: Option<bool>`, `takes_text: Option<bool>`, `debug_label: Option<Str>`, `on_key: Option<KeyHandler>`, `bindings: Vec<(KeyPress`, `reads_layout: bool`, `child: AnyWidget`
- `rtwk::focus::FocusScope` (struct, `crates/rtwk-framework/src/focus/widgets.rs:610`) A `Focus` whose node is a `FocusScopeNode` (`node`, or one of its own): Tab and Shift+Tab stay among the nodes below it, and the one it focused last gets the focus back.
  - public fields: `child: AnyWidget`, `node: Option<FocusScopeNode>`, `autofocus: bool`, `on_focus_change: Option<Handler<bool>>`, `on_key_event: Option<FocusOnKeyEventCallback>`, `can_request_focus: Option<bool>`, `skip_traversal: Option<bool>`, `descendants_are_focusable: Option<bool>`, `debug_label: Option<Str>`

### Code

The gallery's Code tab at the initial options (route `#/focus-and-keys/focus-focusscope`, `crates/rtwk-gallery/src/pages/focus_and_keys.rs:31`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NodeMode {
    Normal,
    SkipTab,
    Disabled,
    Muted,
}

fn focusable(node: &FocusNode, mode: NodeMode, tap: Handler<PointerEvent>) -> AnyWidget {
    let observed = node.clone();
    let tile = ListenableBuilder::new(Rc::new(node.clone()), move |cx, _| {
        let theme = Theme::of(cx);
        let focused = observed.has_focus();
        let (border, kind) = if focused {
            (theme.primary.clone(), BorderType::Heavy)
        } else {
            (theme.border.clone(), BorderType::Rounded)
        };
        let style = if matches!(mode, NodeMode::Disabled | NodeMode::Muted) {
            theme.muted.clone()
        } else if focused {
            Style::new().attrs(Attrs::BOLD)
        } else {
            Style::new()
        };
        BoxWidget::new()
            .border(Border::all(border).kind(kind))
            .padding(EdgeInsets::x(1))
            .child(Text::new(observed.debug_label().expect("demo label")).style(style))
            .into()
    });
    GestureDetector::new()
        .on_tap(tap)
        .child(
            Focus::new(tile)
                .focus_node(node.clone())
                .skip_traversal(mode == NodeMode::SkipTab)
                .can_request_focus(mode != NodeMode::Disabled),
        )
        .into()
}

fn watch_nodes<const N: usize>(
    nodes: [FocusNode; N],
    build: impl Fn(&mut rtwk::BuildCx<'_>, &[FocusNode; N]) -> AnyWidget + 'static,
) -> ListenableBuilder {
    let listenable = <dyn Listenable>::merge(
        nodes
            .iter()
            .cloned()
            .map(|node| Some(Rc::new(node) as Rc<dyn Listenable>)),
    );
    ListenableBuilder {
        listenable,
        builder: Rc::new(move |cx, _| build(cx, &nodes)),
        child: None,
    }
}

fn focus_status(nodes: &[FocusNode; 4]) -> ListenableBuilder {
    watch_nodes((*nodes).clone(), |cx, nodes| {
        let theme = Theme::of(cx);
        let current = nodes.iter().find(|node| node.has_primary_focus());
        let value = current.map_or_else(
            || Span::styled("none", theme.muted.clone()),
            |node| {
                Span::styled(
                    node.debug_label().expect("demo label"),
                    theme.primary.merge(&Style::new().attrs(Attrs::BOLD)),
                )
            },
        );
        Text::rich([Span::styled("focus: ", theme.muted.clone()), value]).into()
    })
}

// Retain these nodes in state and dispose them when the view unmounts.
let nodes = &self.nodes;
let tile = |i, mode| focusable(&nodes[i], mode, cx.map(move |_: PointerEvent| i));
// Handle the message by calling self.nodes[i].request_focus().
let scope_node = &self.scope_node;
let scope = true;
let two = NodeMode::Normal;

{
    let mut tiles = row![tile(0, NodeMode::Normal), tile(1, two)]
        .main_axis_size(MainAxisSize::Min)
        .spacing(1);
    let three = tile(2, NodeMode::Normal);
    let four = tile(3, NodeMode::Normal);
    if scope {
        let node = scope_node.clone();
        let child = FocusScope::new(row![three, four].main_axis_size(MainAxisSize::Min))
            .node(node.clone());
        let framed = watch_nodes([node.node().clone()], move |cx, [node]| {
            let theme = Theme::of(cx);
            let (border, title) = if node.has_focus() {
                (theme.primary.clone(), theme.primary.clone())
            } else {
                (theme.border.clone(), theme.muted.clone())
            };
            BoxWidget::new()
                .border(Border::all(border).kind(BorderType::Rounded))
                .title(BorderTitle::new(" FocusScope ").style(title))
                .child(child.clone())
                .into()
        });
        tiles = tiles.child(framed);
    } else {
        tiles = tiles.child(three).child(four);
    }
    column![tiles, focus_status(nodes)]
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
| scope | on, off | true |
| two | focusable, skips Tab, unfocusable | focusable |

### Methods

#### `Focus`: constructors and associated functions

- `pub fn new(child: impl Into<AnyWidget>) -> Focus` - A region around `child`, unnamed, taking no key.
- `pub fn of<S>(cx: &mut Cx<'_, S>) -> FocusNode` - The node of the nearest `Focus` or `FocusScope` above `cx` (the tree's root scope when there is none, as twk's app puts one above it).
- `pub fn maybe_of<S>(cx: &mut Cx<'_, S>, create_dependency: bool) -> Option<FocusNode>` - The node `of` finds; without `create_dependency`, `cx` does not depend on it.

#### `Focus`: builder methods (take and return `self`)

- `pub fn id(mut self, id: FocusId) -> Focus` - Named `id`: it can take the focus.
- `pub fn focus_node(mut self, node: impl Into<FocusNode>) -> Focus` - With `node` as its node, in place of one of its own.
- `pub fn autofocus(mut self) -> Focus` - Takes the focus when mounted, when its scope has none.
- `pub fn on_focus_change(mut self, h: Handler<bool>) -> Focus` - Told whether its node or one below it has the focus, when that changes.
- `pub fn on_key_event(mut self, f: impl Fn(&FocusNode, KeyPress) -> KeyEventResult + 'static) -> Focus` - Calls `f` with each key that reaches its node, first.
- `pub fn can_request_focus(mut self, on: bool) -> Focus` - Whether its node can take the focus.
- `pub fn skip_traversal(mut self, on: bool) -> Focus` - Whether Tab passes its node by.
- `pub fn descendants_are_focusable(mut self, on: bool) -> Focus` - Whether the nodes below its own can take the focus.
- `pub fn takes_text(mut self, on: bool) -> Focus` - Whether its node takes the keys that are characters as text.
- `pub fn debug_label(mut self, label: impl Into<Str>) -> Focus` - Names its node.
- `pub fn on_key(mut self, h: KeyHandler) -> Focus` - Hands the keys its node and bindings did not take to `h`.
- `pub fn bind(mut self, k: impl Into<KeyPress>, h: Handler<()>) -> Focus` - Key `k` calls `h` (the key-binding combinator).
- `pub fn reads_layout(mut self) -> Focus` - Says its handlers (or those of a `Focus` inside it) read what the layout recorded — a layout builder's constraints, a viewport's height: a key routed through it is dispatched once the tree is laid out, as a pointer ...

#### `FocusScope`: constructors and associated functions

- `pub fn new(child: impl Into<AnyWidget>) -> FocusScope` - A scope node of its own around `child`.
- `pub fn of<S>(cx: &mut Cx<'_, S>) -> FocusScopeNode` - The scope of the node `Focus::of` finds (`FocusNode::nearest_scope`).

#### `FocusScope`: builder methods (take and return `self`)

- `pub fn node(mut self, node: FocusScopeNode) -> FocusScope` - With `node` as its node.
- `pub fn autofocus(mut self) -> FocusScope` - Takes the focus when mounted, when its scope has none.
- `pub fn on_focus_change(mut self, h: Handler<bool>) -> FocusScope` - Told whether its node or one below it has the focus, when that changes.
- `pub fn on_key_event(mut self, f: impl Fn(&FocusNode, KeyPress) -> KeyEventResult + 'static) -> FocusScope` - Calls `f` with each key that reaches its node.
- `pub fn can_request_focus(mut self, on: bool) -> FocusScope` - Whether its node can take the focus.
- `pub fn skip_traversal(mut self, on: bool) -> FocusScope` - Whether Tab passes its node by.
- `pub fn descendants_are_focusable(mut self, on: bool) -> FocusScope` - Whether the nodes below its own can take the focus.
- `pub fn debug_label(mut self, label: impl Into<Str>) -> FocusScope` - Names its node.

### Also used in the code

- `rtwk::foundation::ListenableBuilder` (struct, `crates/rtwk-framework/src/foundation/listenable_builder.rs:131`) Shows what its builder returns, built again whenever its listenable notifies; the widgets around it are not (twk's `ListenableBuilder`).
  - public fields: `listenable: Rc<dyn Listenable>`, `builder: TransitionBuilder`, `child: Option<AnyWidget>`
- `rtwk::gestures::GestureDetector` (struct, `crates/rtwk-widgets/src/gestures/gesture_detector.rs:91`) Tells the gestures of the presses over its child: taps, double taps, long presses and drags.
  - public fields: `on_tap: Option<Handler<PointerEvent>>`, `on_tap_up: Option<Handler<PointerEvent>>`, `on_tap_down: Option<Handler<PointerEvent>>`, `tap_at: Option<TapAt>`, `cursor: Option<PointerShape>`, `on_double_tap: Option<Handler<PointerEvent>>`, `on_long_press: Option<Handler<PointerEvent>>`, `on_long_press_end: Option<Handler<PointerEvent>>`, `on_pan_start: Option<Handler<DragDetails>>`, `on_pan_update: Option<Handler<DragDetails>>`, `on_pan_end: Option<Handler<DragDetails>>`, `buttons: Buttons`, `double_tap_interval: Duration`, `long_press_duration: Duration`, `behavior: HitTestBehavior`, `child: Option<AnyWidget>`
- `rtwk::layout::BorderTitle` (struct, `crates/rtwk-widgets/src/layout/box_.rs:153`) Text on the top or the bottom edge of a `BoxWidget`, in `style`: the y of `alignment` picks the edge (-1 the top, 1 the bottom), its x the place along it (twk's `BorderTitle`).
  - public fields: `text: Str`, `style: Style`, `alignment: Alignment`

## ActiveItem

The item a focused list's keys act on: the terminal's hidden cursor rests on it, which iTerm2's cursor guide marks and macOS zoom, a screen reader or a braille display follows.

API:

- `rtwk::focus::ActiveItem` (struct, `crates/rtwk-widgets/src/active_item.rs:46`) Shows `child`, an item of a widget that has the focus as a whole (a list, a menu, a set of choices), which, while `active`, is the one its keys act on: the list's cursor row.
  - public fields: `active: bool`, `column: i32`, `text: Option<(usize`, `child: Option<AnyWidget>`

### Code

The gallery's Code tab at the initial options (route `#/focus-and-keys/activeitem`, `crates/rtwk-gallery/src/pages/focus_and_keys.rs:64`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
{
    let theme = Theme::of(cx);
    let mut fruits = column![]
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start);
    for (index, fruit) in ["apple", "banana", "cherry", "damson", "elder"]
        .into_iter()
        .enumerate()
    {
        let active = index == self.cursor;
        let text = if active {
            Text::new(format!("› {fruit}")).style(theme.selection.clone())
        } else {
            Text::new(format!("  {fruit}"))
        };
        fruits = fruits.child(ActiveItem::new(active, text));
    }
    KeyBindings::new(
        [
            ("up", cx.send(Message::Cursor(-1))),
            ("down", cx.send(Message::Cursor(1))),
        ],
        Focus::new(fruits).focus_node(self.node.clone()),
    )
    .into()
}
```

### Methods

#### `ActiveItem`: constructors and associated functions

- `pub fn new(active: bool, child: impl Into<AnyWidget>) -> ActiveItem` - `child`, the item its widget's keys act on while `active`, the cursor on the first cell of its first row.

#### `ActiveItem`: builder methods (take and return `self`)

- `pub fn column(mut self, column: i32) -> ActiveItem` - The cursor on `column` of the first row.
- `pub fn text(mut self, text: Option<(usize, usize)>) -> ActiveItem` - The cursor where the text `text` (byte offsets into the spans joined) is drawn.

### Also used in the code

- `rtwk::focus::KeyBindings` (struct, `crates/rtwk-framework/src/focus/shortcuts.rs:1257`) Binds keys to handlers for the nodes below it.
  - public fields: `bindings: Vec<(Str`, `combos: Vec<(KeyCombo`, `hints: Vec<(Str`, `combo_hints: Vec<(KeyCombo`, `sequence_timeout: Option<Duration>`, `unmatched: UnmatchedKeys`, `child: AnyWidget`, `debug_label: Option<Str>`
- `rtwk::focus::Focus` (struct, `crates/rtwk-framework/src/focus/widgets.rs:85`) Puts a `FocusNode` in the focus tree for its child, under the node of the nearest `Focus` or `FocusScope` above it: the `focus_node` given, or one of its own.
  - public fields: `id: Option<FocusId>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `on_focus_change: Option<Handler<bool>>`, `on_key_event: Option<FocusOnKeyEventCallback>`, `can_request_focus: Option<bool>`, `skip_traversal: Option<bool>`, `descendants_are_focusable: Option<bool>`, `takes_text: Option<bool>`, `debug_label: Option<Str>`, `on_key: Option<KeyHandler>`, `bindings: Vec<(KeyPress`, `reads_layout: bool`, `child: AnyWidget`

## ExcludeFocus

A subtree Tab passes by: its nodes take no focus while it excludes them, and give it up when it starts to.

API:

- `rtwk::focus::ExcludeFocus` (struct, `crates/rtwk-framework/src/focus/widgets.rs:858`) Shows its child, whose focus nodes cannot have the focus while `excluding` (twk's `ExcludeFocus`, as Flutter's).
  - public fields: `child: AnyWidget`, `excluding: bool`

### Code

The gallery's Code tab at the initial options (route `#/focus-and-keys/excludefocus`, `crates/rtwk-gallery/src/pages/focus_and_keys.rs:57`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NodeMode {
    Normal,
    SkipTab,
    Disabled,
    Muted,
}

fn focusable(node: &FocusNode, mode: NodeMode, tap: Handler<PointerEvent>) -> AnyWidget {
    let observed = node.clone();
    let tile = ListenableBuilder::new(Rc::new(node.clone()), move |cx, _| {
        let theme = Theme::of(cx);
        let focused = observed.has_focus();
        let (border, kind) = if focused {
            (theme.primary.clone(), BorderType::Heavy)
        } else {
            (theme.border.clone(), BorderType::Rounded)
        };
        let style = if matches!(mode, NodeMode::Disabled | NodeMode::Muted) {
            theme.muted.clone()
        } else if focused {
            Style::new().attrs(Attrs::BOLD)
        } else {
            Style::new()
        };
        BoxWidget::new()
            .border(Border::all(border).kind(kind))
            .padding(EdgeInsets::x(1))
            .child(Text::new(observed.debug_label().expect("demo label")).style(style))
            .into()
    });
    GestureDetector::new()
        .on_tap(tap)
        .child(
            Focus::new(tile)
                .focus_node(node.clone())
                .skip_traversal(mode == NodeMode::SkipTab)
                .can_request_focus(mode != NodeMode::Disabled),
        )
        .into()
}

fn watch_nodes<const N: usize>(
    nodes: [FocusNode; N],
    build: impl Fn(&mut rtwk::BuildCx<'_>, &[FocusNode; N]) -> AnyWidget + 'static,
) -> ListenableBuilder {
    let listenable = <dyn Listenable>::merge(
        nodes
            .iter()
            .cloned()
            .map(|node| Some(Rc::new(node) as Rc<dyn Listenable>)),
    );
    ListenableBuilder {
        listenable,
        builder: Rc::new(move |cx, _| build(cx, &nodes)),
        child: None,
    }
}

fn focus_status(nodes: &[FocusNode; 4]) -> ListenableBuilder {
    watch_nodes((*nodes).clone(), |cx, nodes| {
        let theme = Theme::of(cx);
        let current = nodes.iter().find(|node| node.has_primary_focus());
        let value = current.map_or_else(
            || Span::styled("none", theme.muted.clone()),
            |node| {
                Span::styled(
                    node.debug_label().expect("demo label"),
                    theme.primary.merge(&Style::new().attrs(Attrs::BOLD)),
                )
            },
        );
        Text::rich([Span::styled("focus: ", theme.muted.clone()), value]).into()
    })
}

// Retain these nodes in state and dispose them when the view unmounts.
let nodes = &self.nodes;
let tile = |i, mode| focusable(&nodes[i], mode, cx.map(move |_: PointerEvent| i));
// Handle the message by calling self.nodes[i].request_focus().
let excluding = true;

{
    let mode = if excluding {
        NodeMode::Muted
    } else {
        NodeMode::Normal
    };
    let middle =
        ExcludeFocus::new(row![tile(1, mode), tile(2, mode)].main_axis_size(MainAxisSize::Min))
            .excluding(excluding);
    let framed = watch_nodes([nodes[1].clone(), nodes[2].clone()], move |cx, nodes| {
        let theme = Theme::of(cx);
        let (border, title) = if nodes.iter().any(FocusNode::has_focus) {
            (theme.primary.clone(), theme.primary.clone())
        } else {
            (theme.border.clone(), theme.muted.clone())
        };
        BoxWidget::new()
            .border(Border::all(border).kind(BorderType::Rounded))
            .title(BorderTitle::new(" ExcludeFocus ").style(title))
            .child(middle.clone())
            .into()
    });
    column![
        row![tile(0, NodeMode::Normal), framed, tile(3, NodeMode::Normal)]
            .main_axis_size(MainAxisSize::Min)
            .spacing(1),
        focus_status(nodes),
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
| excluding | on, off | true |

### Methods

#### `ExcludeFocus`: constructors and associated functions

- `pub fn new(child: impl Into<AnyWidget>) -> ExcludeFocus` - Keeps the nodes of `child` from the focus.

#### `ExcludeFocus`: builder methods (take and return `self`)

- `pub fn excluding(mut self, on: bool) -> ExcludeFocus` - Whether it keeps them from the focus.

### Also used in the code

- `rtwk::foundation::ListenableBuilder` (struct, `crates/rtwk-framework/src/foundation/listenable_builder.rs:131`) Shows what its builder returns, built again whenever its listenable notifies; the widgets around it are not (twk's `ListenableBuilder`).
  - public fields: `listenable: Rc<dyn Listenable>`, `builder: TransitionBuilder`, `child: Option<AnyWidget>`
- `rtwk::gestures::GestureDetector` (struct, `crates/rtwk-widgets/src/gestures/gesture_detector.rs:91`) Tells the gestures of the presses over its child: taps, double taps, long presses and drags.
  - public fields: `on_tap: Option<Handler<PointerEvent>>`, `on_tap_up: Option<Handler<PointerEvent>>`, `on_tap_down: Option<Handler<PointerEvent>>`, `tap_at: Option<TapAt>`, `cursor: Option<PointerShape>`, `on_double_tap: Option<Handler<PointerEvent>>`, `on_long_press: Option<Handler<PointerEvent>>`, `on_long_press_end: Option<Handler<PointerEvent>>`, `on_pan_start: Option<Handler<DragDetails>>`, `on_pan_update: Option<Handler<DragDetails>>`, `on_pan_end: Option<Handler<DragDetails>>`, `buttons: Buttons`, `double_tap_interval: Duration`, `long_press_duration: Duration`, `behavior: HitTestBehavior`, `child: Option<AnyWidget>`
- `rtwk::focus::Focus` (struct, `crates/rtwk-framework/src/focus/widgets.rs:85`) Puts a `FocusNode` in the focus tree for its child, under the node of the nearest `Focus` or `FocusScope` above it: the `focus_node` given, or one of its own.
  - public fields: `id: Option<FocusId>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `on_focus_change: Option<Handler<bool>>`, `on_key_event: Option<FocusOnKeyEventCallback>`, `can_request_focus: Option<bool>`, `skip_traversal: Option<bool>`, `descendants_are_focusable: Option<bool>`, `takes_text: Option<bool>`, `debug_label: Option<Str>`, `on_key: Option<KeyHandler>`, `bindings: Vec<(KeyPress`, `reads_layout: bool`, `child: AnyWidget`
- `rtwk::focus::FocusNode` (struct, `crates/rtwk-framework/src/focus/node.rs:153`) A place in a tree's focus tree: the node of a `Focus` widget, which can have the focus and gets the keys (twk's `FocusNode`).
- `rtwk::layout::BorderTitle` (struct, `crates/rtwk-widgets/src/layout/box_.rs:153`) Text on the top or the bottom edge of a `BoxWidget`, in `style`: the y of `alignment` picks the edge (-1 the top, 1 the bottom), its x the place along it (twk's `BorderTitle`).
  - public fields: `text: Str`, `style: Style`, `alignment: Alignment`

## KeyBindings, LeaderKey

Keys and sequences bound to callbacks while the focus is below them: Tab to the box, then +, -, g g or the leader , and r.

API:

- `rtwk::focus::KeyBindings` (struct, `crates/rtwk-framework/src/focus/shortcuts.rs:1257`) Binds keys to handlers for the nodes below it.
  - public fields: `bindings: Vec<(Str`, `combos: Vec<(KeyCombo`, `hints: Vec<(Str`, `combo_hints: Vec<(KeyCombo`, `sequence_timeout: Option<Duration>`, `unmatched: UnmatchedKeys`, `child: AnyWidget`, `debug_label: Option<Str>`
- `rtwk::focus::LeaderKey` (struct, `crates/rtwk-framework/src/focus/shortcuts.rs:388`) Names the key that `leader` stands for in the sequences of the `KeyBindings` and `Shortcuts` below it, as `KeyCombo::parse` reads it: `LeaderKey::new(",", child)`, then `"leader w"`.
  - public fields: `leader: Str`, `child: AnyWidget`

### Code

The gallery's Code tab at the initial options (route `#/focus-and-keys/keybindings-leaderkey`, `crates/rtwk-gallery/src/pages/focus_and_keys.rs:71`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
fn with_bar(
    counter: impl Into<AnyWidget>,
    bar: KeyHintsBar,
    did: Option<AnyWidget>,
) -> FocusBuilder {
    let counter = counter.into();
    FocusBuilder::new(move |_, focused, _| {
        let hints = if *focused {
            AnyWidget::from(bar.clone())
        } else {
            SizedBox::new(None, Some(1), None).into()
        };
        let mut children = column![counter.clone(), hints]
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .spacing(1);
        if let Some(did) = &did {
            children = children.child(did.clone());
        }
        children.into()
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NodeMode {
    Normal,
    SkipTab,
    Disabled,
    Muted,
}

fn focusable(node: &FocusNode, mode: NodeMode, tap: Handler<PointerEvent>) -> AnyWidget {
    let observed = node.clone();
    let tile = ListenableBuilder::new(Rc::new(node.clone()), move |cx, _| {
        let theme = Theme::of(cx);
        let focused = observed.has_focus();
        let (border, kind) = if focused {
            (theme.primary.clone(), BorderType::Heavy)
        } else {
            (theme.border.clone(), BorderType::Rounded)
        };
        let style = if matches!(mode, NodeMode::Disabled | NodeMode::Muted) {
            theme.muted.clone()
        } else if focused {
            Style::new().attrs(Attrs::BOLD)
        } else {
            Style::new()
        };
        BoxWidget::new()
            .border(Border::all(border).kind(kind))
            .padding(EdgeInsets::x(1))
            .child(Text::new(observed.debug_label().expect("demo label")).style(style))
            .into()
    });
    GestureDetector::new()
        .on_tap(tap)
        .child(
            Focus::new(tile)
                .focus_node(node.clone())
                .skip_traversal(mode == NodeMode::SkipTab)
                .can_request_focus(mode != NodeMode::Disabled),
        )
        .into()
}

fn watch_nodes<const N: usize>(
    nodes: [FocusNode; N],
    build: impl Fn(&mut rtwk::BuildCx<'_>, &[FocusNode; N]) -> AnyWidget + 'static,
) -> ListenableBuilder {
    let listenable = <dyn Listenable>::merge(
        nodes
            .iter()
            .cloned()
            .map(|node| Some(Rc::new(node) as Rc<dyn Listenable>)),
    );
    ListenableBuilder {
        listenable,
        builder: Rc::new(move |cx, _| build(cx, &nodes)),
        child: None,
    }
}

fn focus_status(nodes: &[FocusNode; 4]) -> ListenableBuilder {
    watch_nodes((*nodes).clone(), |cx, nodes| {
        let theme = Theme::of(cx);
        let current = nodes.iter().find(|node| node.has_primary_focus());
        let value = current.map_or_else(
            || Span::styled("none", theme.muted.clone()),
            |node| {
                Span::styled(
                    node.debug_label().expect("demo label"),
                    theme.primary.merge(&Style::new().attrs(Attrs::BOLD)),
                )
            },
        );
        Text::rich([Span::styled("focus: ", theme.muted.clone()), value]).into()
    })
}

// Keep the FocusNode and count in state.
let theme = Theme::of(cx);
let count = Text::rich([
    Span::styled("count: ", theme.muted.clone()),
    Span::new(self.count.to_string()),
]);
let counter = row![
    focusable(&self.node, NodeMode::Normal, cx.map(|_: PointerEvent| Message::Focus)),
    count,
].main_axis_size(MainAxisSize::Min).spacing(2);
let leader = ",";
let timeout = Some(Duration::from_secs(1));
let unmatched = UnmatchedKeys::Drop;
{
    let reset = cx.send(Message::Reset);
    let bindings = KeyBindings::new(
        [
            ("+", cx.send(Message::Count(1))),
            ("-", cx.send(Message::Count(-1))),
            ("g g", reset.clone()),
            ("leader r", reset),
        ],
        with_bar(counter, KeyHintsBar::new(), None),
    )
    .hints([
        ("+", "more"),
        ("-", "less"),
        ("g g", "reset"),
        ("leader r", "reset"),
    ])
    .sequence_timeout(timeout)
    .unmatched(unmatched);
    if leader == "space" {
        bindings.into()
    } else {
        LeaderKey::new(leader, bindings).into()
    }
}
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| leader | ,, space, \ | , |
| timeout | 1 s, none | 1 s |
| unmatched | drop, replay | drop |

### Methods

#### `KeyBindings`: constructors and associated functions

- `pub fn new<N, H>(bindings: impl IntoIterator<Item = (N, H)>, child: impl Into<AnyWidget>) -> KeyBindings where N: Into<Str>, H: Into<Option<Handler<()>>>,` - Binds the keys and sequences named in `bindings` for the nodes below `child`.
- `pub fn combos<H>(combos: impl IntoIterator<Item = (KeyCombo, H)>, child: impl Into<AnyWidget>) -> KeyBindings where H: Into<Option<Handler<()>>>,` - Binds the keys of `combos`, each alone.

#### `KeyBindings`: builder methods (take and return `self`)

- `pub fn hints<N: Into<Str>, T: Into<Str>>(mut self, hints: impl IntoIterator<Item = (N, T)>) -> KeyBindings` - With `hints` for the keys by name.
- `pub fn combo_hints<T: Into<Str>>(mut self, hints: impl IntoIterator<Item = (KeyCombo, T)>) -> KeyBindings` - With `hints` for the keys of `combos`.
- `pub fn sequence_timeout(mut self, timeout: Option<Duration>) -> KeyBindings` - With `timeout` for the first keys of a sequence.
- `pub fn unmatched(mut self, u: UnmatchedKeys) -> KeyBindings` - With what becomes of keys held that no binding takes.
- `pub fn debug_label(mut self, label: impl Into<Str>) -> KeyBindings` - Names the node of the layer.

#### `LeaderKey`: constructors and associated functions

- `pub fn new(leader: impl Into<Str>, child: impl Into<AnyWidget>) -> LeaderKey` - Names `leader` the leader key below it.

### Also used in the code

- `rtwk::focus::FocusBuilder` (struct, `crates/rtwk-framework/src/focus/widgets.rs:905`) Shows what its builder returns for whether the focus is in this widget's subtree, built again when the focus enters or leaves it; the widgets around it are not (twk's `FocusBuilder`).
  - public fields: `builder: ValueWidgetBuilder<bool>`, `child: Option<AnyWidget>`, `debug_label: Option<Str>`
- `rtwk::foundation::ListenableBuilder` (struct, `crates/rtwk-framework/src/foundation/listenable_builder.rs:131`) Shows what its builder returns, built again whenever its listenable notifies; the widgets around it are not (twk's `ListenableBuilder`).
  - public fields: `listenable: Rc<dyn Listenable>`, `builder: TransitionBuilder`, `child: Option<AnyWidget>`
- `rtwk::gestures::GestureDetector` (struct, `crates/rtwk-widgets/src/gestures/gesture_detector.rs:91`) Tells the gestures of the presses over its child: taps, double taps, long presses and drags.
  - public fields: `on_tap: Option<Handler<PointerEvent>>`, `on_tap_up: Option<Handler<PointerEvent>>`, `on_tap_down: Option<Handler<PointerEvent>>`, `tap_at: Option<TapAt>`, `cursor: Option<PointerShape>`, `on_double_tap: Option<Handler<PointerEvent>>`, `on_long_press: Option<Handler<PointerEvent>>`, `on_long_press_end: Option<Handler<PointerEvent>>`, `on_pan_start: Option<Handler<DragDetails>>`, `on_pan_update: Option<Handler<DragDetails>>`, `on_pan_end: Option<Handler<DragDetails>>`, `buttons: Buttons`, `double_tap_interval: Duration`, `long_press_duration: Duration`, `behavior: HitTestBehavior`, `child: Option<AnyWidget>`
- `rtwk::focus::Focus` (struct, `crates/rtwk-framework/src/focus/widgets.rs:85`) Puts a `FocusNode` in the focus tree for its child, under the node of the nearest `Focus` or `FocusScope` above it: the `focus_node` given, or one of its own.
  - public fields: `id: Option<FocusId>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `on_focus_change: Option<Handler<bool>>`, `on_key_event: Option<FocusOnKeyEventCallback>`, `can_request_focus: Option<bool>`, `skip_traversal: Option<bool>`, `descendants_are_focusable: Option<bool>`, `takes_text: Option<bool>`, `debug_label: Option<Str>`, `on_key: Option<KeyHandler>`, `bindings: Vec<(KeyPress`, `reads_layout: bool`, `child: AnyWidget`
- `rtwk::focus::UnmatchedKeys` (enum, `crates/rtwk-framework/src/focus/shortcuts.rs:419`) What a layer does with the first keys of a sequence it holds when the next key does not go on with them, or the sequence timeout passes, and no binding of their first keys alone takes them.
  - variants: `Drop`, `Replay`
- `rtwk::controls::KeyHintsBar` (struct, `crates/rtwk-controls/src/key_hints.rs:104`) A row of the keys that can be pressed now, each before its hint: `ctrl+s save q quit`.
  - public fields: `show_unhinted: bool`, `key_label: Option<KeyLabel>`, `join_keys: bool`, `separator: Str`, `style: Option<Style>`, `key_style: Option<Style>`, `hint_style: Option<Style>`, `pressable: bool`

## Shortcuts, Actions

Keys bound to intents, which the nearest action for them does: Tab to the box, then Up or Down; a key whose action is off passes on.

API:

- `rtwk::focus::Shortcuts` (struct, `crates/rtwk-framework/src/focus/shortcuts.rs:1448`) Binds keys to `Intent`s for the nodes below it, which the `Action`s of the `Actions` above the focused node do.
  - public fields: `shortcuts: Vec<(Str`, `combos: Vec<(KeyCombo`, `hints: Vec<(Str`, `combo_hints: Vec<(KeyCombo`, `sequence_timeout: Option<Duration>`, `unmatched: UnmatchedKeys`, `child: AnyWidget`, `debug_label: Option<Str>`
- `rtwk::focus::Actions` (struct, `crates/rtwk-framework/src/focus/shortcuts.rs:267`) Gives the widgets below it its actions: for a type of `Intent`, the `Action` that does the intents of that type.
  - public fields: `actions: Vec<(TypeId`, `child: AnyWidget`

### Code

The gallery's Code tab at the initial options (route `#/focus-and-keys/shortcuts-actions`, `crates/rtwk-gallery/src/pages/focus_and_keys.rs:82`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
fn with_bar(
    counter: impl Into<AnyWidget>,
    bar: KeyHintsBar,
    did: Option<AnyWidget>,
) -> FocusBuilder {
    let counter = counter.into();
    FocusBuilder::new(move |_, focused, _| {
        let hints = if *focused {
            AnyWidget::from(bar.clone())
        } else {
            SizedBox::new(None, Some(1), None).into()
        };
        let mut children = column![counter.clone(), hints]
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .spacing(1);
        if let Some(did) = &did {
            children = children.child(did.clone());
        }
        children.into()
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NodeMode {
    Normal,
    SkipTab,
    Disabled,
    Muted,
}

fn focusable(node: &FocusNode, mode: NodeMode, tap: Handler<PointerEvent>) -> AnyWidget {
    let observed = node.clone();
    let tile = ListenableBuilder::new(Rc::new(node.clone()), move |cx, _| {
        let theme = Theme::of(cx);
        let focused = observed.has_focus();
        let (border, kind) = if focused {
            (theme.primary.clone(), BorderType::Heavy)
        } else {
            (theme.border.clone(), BorderType::Rounded)
        };
        let style = if matches!(mode, NodeMode::Disabled | NodeMode::Muted) {
            theme.muted.clone()
        } else if focused {
            Style::new().attrs(Attrs::BOLD)
        } else {
            Style::new()
        };
        BoxWidget::new()
            .border(Border::all(border).kind(kind))
            .padding(EdgeInsets::x(1))
            .child(Text::new(observed.debug_label().expect("demo label")).style(style))
            .into()
    });
    GestureDetector::new()
        .on_tap(tap)
        .child(
            Focus::new(tile)
                .focus_node(node.clone())
                .skip_traversal(mode == NodeMode::SkipTab)
                .can_request_focus(mode != NodeMode::Disabled),
        )
        .into()
}

fn watch_nodes<const N: usize>(
    nodes: [FocusNode; N],
    build: impl Fn(&mut rtwk::BuildCx<'_>, &[FocusNode; N]) -> AnyWidget + 'static,
) -> ListenableBuilder {
    let listenable = <dyn Listenable>::merge(
        nodes
            .iter()
            .cloned()
            .map(|node| Some(Rc::new(node) as Rc<dyn Listenable>)),
    );
    ListenableBuilder {
        listenable,
        builder: Rc::new(move |cx, _| build(cx, &nodes)),
        child: None,
    }
}

fn focus_status(nodes: &[FocusNode; 4]) -> ListenableBuilder {
    watch_nodes((*nodes).clone(), |cx, nodes| {
        let theme = Theme::of(cx);
        let current = nodes.iter().find(|node| node.has_primary_focus());
        let value = current.map_or_else(
            || Span::styled("none", theme.muted.clone()),
            |node| {
                Span::styled(
                    node.debug_label().expect("demo label"),
                    theme.primary.merge(&Style::new().attrs(Attrs::BOLD)),
                )
            },
        );
        Text::rich([Span::styled("focus: ", theme.muted.clone()), value]).into()
    })
}

// Keep the FocusNode and count in state.
let theme = Theme::of(cx);
let count = Text::rich([
    Span::styled("count: ", theme.muted.clone()),
    Span::new(self.count.to_string()),
]);
let counter = row![
    focusable(&self.node, NodeMode::Normal, cx.map(|_: PointerEvent| Message::Focus)),
    count,
].main_axis_size(MainAxisSize::Min).spacing(2);
let enabled = true;
let did = if self.did.is_empty() { Span::styled("nothing", theme.muted.clone()) } else { Span::new(self.did) };
let did_line = Text::rich([Span::styled("did: ", theme.muted.clone()), did]);
{
    let bar = KeyHintsBar::new().key_label(|key| match key.to_string().as_str() {
        "up" => "↑".to_owned(),
        "down" => "↓".to_owned(),
        _ => key.to_string(),
    });
    let shortcuts = Shortcuts::new(
        [("up", AnyIntent::new(Up)), ("down", AnyIntent::new(Down))],
        with_bar(counter, bar, Some(did_line.into())),
    )
    .hints([("up", "step up"), ("down", "step down")]);
    let actions = Actions::new(shortcuts)
        .action(
            CallbackAction::new(
                cx.map(|_: Up| Message::Step(1, "↑ _Up → its action stepped up")),
            )
            .enabled(enabled),
        )
        .action(
            CallbackAction::new(
                cx.map(|_: Down| Message::Step(-1, "↓ _Down → its action stepped down")),
            )
            .enabled(enabled),
        );
    KeyBindings::new(
        [
            (
                "up",
                cx.send(Message::Pass("↑ _Up → passed on to the KeyBindings")),
            ),
            (
                "down",
                cx.send(Message::Pass("↓ _Down → passed on to the KeyBindings")),
            ),
        ],
        actions,
    )
    .hints([("up", "passed on"), ("down", "passed on")])
    .into()
}
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| actions | on, off | true |

### Methods

#### `Shortcuts`: constructors and associated functions

- `pub fn new<N, I>(shortcuts: impl IntoIterator<Item = (N, I)>, child: impl Into<AnyWidget>) -> Shortcuts where N: Into<Str>, I: Into<AnyIntent>,` - Binds the keys and sequences named in `shortcuts` for the nodes below `child`.
- `pub fn combos<I: Into<AnyIntent>>(combos: impl IntoIterator<Item = (KeyCombo, I)>, child: impl Into<AnyWidget>) -> Shortcuts` - Binds the keys of `combos`, each alone.

#### `Shortcuts`: builder methods (take and return `self`)

- `pub fn hints<N: Into<Str>, T: Into<Str>>(mut self, hints: impl IntoIterator<Item = (N, T)>) -> Shortcuts` - With `hints` for the keys by name.
- `pub fn combo_hints<T: Into<Str>>(mut self, hints: impl IntoIterator<Item = (KeyCombo, T)>) -> Shortcuts` - With `hints` for the keys of `combos`.
- `pub fn sequence_timeout(mut self, timeout: Option<Duration>) -> Shortcuts` - With `timeout` for the first keys of a sequence.
- `pub fn unmatched(mut self, u: UnmatchedKeys) -> Shortcuts` - With what becomes of keys held that no binding takes.
- `pub fn debug_label(mut self, label: impl Into<Str>) -> Shortcuts` - Names the node of the layer.

#### `Actions`: constructors and associated functions

- `pub fn new(child: impl Into<AnyWidget>) -> Actions` - No action yet, above `child`.
- `pub fn maybe_find<S>(cx: &Cx<'_, S>, intent: &AnyIntent) -> Option<AnyAction>` - The action that does `intent` from `cx`: of the `Actions` above it, the nearest one's action for the type of `intent` that is enabled; none if there is none.
- `pub fn maybe_invoke<S>(cx: &mut Cx<'_, S>, intent: &AnyIntent) -> bool` - Invokes the action `maybe_find` finds for `intent`; answers whether there is one.

#### `Actions`: builder methods (take and return `self`)

- `pub fn action<T: Intent, A: Action<T>>(mut self, action: A) -> Actions` - With `action` for the intents of type `T` (in place of another).

### Also used in the code

- `rtwk::focus::FocusBuilder` (struct, `crates/rtwk-framework/src/focus/widgets.rs:905`) Shows what its builder returns for whether the focus is in this widget's subtree, built again when the focus enters or leaves it; the widgets around it are not (twk's `FocusBuilder`).
  - public fields: `builder: ValueWidgetBuilder<bool>`, `child: Option<AnyWidget>`, `debug_label: Option<Str>`
- `rtwk::foundation::ListenableBuilder` (struct, `crates/rtwk-framework/src/foundation/listenable_builder.rs:131`) Shows what its builder returns, built again whenever its listenable notifies; the widgets around it are not (twk's `ListenableBuilder`).
  - public fields: `listenable: Rc<dyn Listenable>`, `builder: TransitionBuilder`, `child: Option<AnyWidget>`
- `rtwk::gestures::GestureDetector` (struct, `crates/rtwk-widgets/src/gestures/gesture_detector.rs:91`) Tells the gestures of the presses over its child: taps, double taps, long presses and drags.
  - public fields: `on_tap: Option<Handler<PointerEvent>>`, `on_tap_up: Option<Handler<PointerEvent>>`, `on_tap_down: Option<Handler<PointerEvent>>`, `tap_at: Option<TapAt>`, `cursor: Option<PointerShape>`, `on_double_tap: Option<Handler<PointerEvent>>`, `on_long_press: Option<Handler<PointerEvent>>`, `on_long_press_end: Option<Handler<PointerEvent>>`, `on_pan_start: Option<Handler<DragDetails>>`, `on_pan_update: Option<Handler<DragDetails>>`, `on_pan_end: Option<Handler<DragDetails>>`, `buttons: Buttons`, `double_tap_interval: Duration`, `long_press_duration: Duration`, `behavior: HitTestBehavior`, `child: Option<AnyWidget>`
- `rtwk::focus::Focus` (struct, `crates/rtwk-framework/src/focus/widgets.rs:85`) Puts a `FocusNode` in the focus tree for its child, under the node of the nearest `Focus` or `FocusScope` above it: the `focus_node` given, or one of its own.
  - public fields: `id: Option<FocusId>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `on_focus_change: Option<Handler<bool>>`, `on_key_event: Option<FocusOnKeyEventCallback>`, `can_request_focus: Option<bool>`, `skip_traversal: Option<bool>`, `descendants_are_focusable: Option<bool>`, `takes_text: Option<bool>`, `debug_label: Option<Str>`, `on_key: Option<KeyHandler>`, `bindings: Vec<(KeyPress`, `reads_layout: bool`, `child: AnyWidget`
- `rtwk::controls::KeyHintsBar` (struct, `crates/rtwk-controls/src/key_hints.rs:104`) A row of the keys that can be pressed now, each before its hint: `ctrl+s save q quit`.
  - public fields: `show_unhinted: bool`, `key_label: Option<KeyLabel>`, `join_keys: bool`, `separator: Str`, `style: Option<Style>`, `key_style: Option<Style>`, `hint_style: Option<Style>`, `pressable: bool`
- `rtwk::focus::AnyIntent` (struct, `crates/rtwk-framework/src/focus/shortcuts.rs:64`) An `Intent` of any type, shared: what a `Shortcuts` binds a key to.
- `rtwk::focus::CallbackAction` (struct, `crates/rtwk-framework/src/focus/shortcuts.rs:207`) An `Action` that sends its handler the intent, while `enabled`.
  - public fields: `on_invoke: Handler<T>`, `enabled: bool`
- `rtwk::navigation::Step` (struct, `crates/rtwk/src/navigation/stepper.rs:176`) A step of a `Stepper`: its `title`, what it shows while it is the current one (`content`), and what it takes to leave it forward (`validate`).
  - public fields: `title: Str`, `content: AnyWidget`, `subtitle: Option<Str>`, `state: Option<StepState>`, `validate: Option<Validate>`
- `rtwk::focus::KeyBindings` (struct, `crates/rtwk-framework/src/focus/shortcuts.rs:1257`) Binds keys to handlers for the nodes below it.
  - public fields: `bindings: Vec<(Str`, `combos: Vec<(KeyCombo`, `hints: Vec<(Str`, `combo_hints: Vec<(KeyCombo`, `sequence_timeout: Option<Duration>`, `unmatched: UnmatchedKeys`, `child: AnyWidget`, `debug_label: Option<Str>`
