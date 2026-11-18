<!-- Written by `cargo xtask skill-references` from docs/widgets; do not edit. -->
# Focus and keys

The focus, moved by Tab and the mouse, and the keys bound where it is.

[All widgets](../widgets.md)

A Code is the body of the gallery's preview: it names widgets by the last segment of the paths under API (`use` them) and may use `cx` or values of the gallery's page.

## Focus, FocusScope

Nodes the keys go to: Tab moves the focus from one to the next, a click takes it; in a scope Tab goes round its nodes, and Tab into it focuses the one it had last.

- `rtwk::focus::Focus` (struct) in `crates/rtwk-framework/src/focus/widgets.rs:85`: Puts a FocusNode in the focus tree for its child, under the node of the nearest `Focus` or FocusScope above it: the focus_node given, or one of its own.
- `rtwk::focus::FocusScope` (struct) in `crates/rtwk-framework/src/focus/widgets.rs:610`: A Focus whose node is a FocusScopeNode (node, or one of its own): Tab and Shift+Tab stay among the nodes below it, and the one it focused last gets the focus back.

### Code

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

| option | values | starts at |
| --- | --- | --- |
| FocusScope | false, true | true |
| node two | focusable, skips Tab, unfocusable | focusable |

## ActiveItem

The item a focused list's keys act on: the terminal's hidden cursor rests on it, which iTerm2's cursor guide marks and macOS zoom, a screen reader or a braille display follows.

- `rtwk::focus::ActiveItem` (struct) in `crates/rtwk-widgets/src/active_item.rs:46`: Shows child, an item of a widget that has the focus as a whole (a list, a menu, a set of choices), which, while active, is the one its keys act on: the list's cursor row.

### Code

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

## ExcludeFocus

A subtree Tab passes by: its nodes take no focus while it excludes them, and give it up when it starts to.

- `rtwk::focus::ExcludeFocus` (struct) in `crates/rtwk-framework/src/focus/widgets.rs:858`: Shows its child, whose focus nodes cannot have the focus while excluding (twk's `ExcludeFocus`, as Flutter's).

### Code

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

| option | values | starts at |
| --- | --- | --- |
| excluding | false, true | true |

## KeyBindings, LeaderKey

Keys and sequences bound to callbacks while the focus is below them: Tab to the box, then +, -, g g or the leader , and r.

- `rtwk::focus::KeyBindings` (struct) in `crates/rtwk-framework/src/focus/shortcuts.rs:1257`: Binds keys to handlers for the nodes below it.
- `rtwk::focus::LeaderKey` (struct) in `crates/rtwk-framework/src/focus/shortcuts.rs:388`: Names the key that `leader` stands for in the sequences of the KeyBindings and Shortcuts below it, as KeyCombo::parse reads it: `LeaderKey::new(",", child)`, then `"leader w"`.

### Code

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

| option | values | starts at |
| --- | --- | --- |
| leader | ,, space, \ | , |
| sequence timeout | 1 s, none | 1 s |
| unmatched keys | drop, replay | drop |

## Shortcuts, Actions

Keys bound to intents, which the nearest action for them does: Tab to the box, then Up or Down; a key whose action is off passes on.

- `rtwk::focus::Shortcuts` (struct) in `crates/rtwk-framework/src/focus/shortcuts.rs:1448`: Binds keys to Intents for the nodes below it, which the Actions of the Actions above the focused node do.
- `rtwk::focus::Actions` (struct) in `crates/rtwk-framework/src/focus/shortcuts.rs:267`: Gives the widgets below it its actions: for a type of Intent, the Action that does the intents of that type.

### Code

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

| option | values | starts at |
| --- | --- | --- |
| actions enabled | false, true | true |
