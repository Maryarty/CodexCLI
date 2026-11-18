<!-- Written by `cargo xtask skill-references` from docs/widgets; do not edit. -->
# Mouse

Clicks, drags, hovers and the wheel, and where the mouse is let through.

[All widgets](../widgets.md)

A Code is the body of the gallery's preview: it names widgets by the last segment of the paths under API (`use` them) and may use `cx` or values of the gallery's page.

## GestureDetector

Taps, double taps, long presses and drags of the mouse over a widget.

- `rtwk::gestures::GestureDetector` (struct) in `crates/rtwk-widgets/src/gestures/gesture_detector.rs:91`: Tells the gestures of the presses over its child: taps, double taps, long presses and drags.

### Code

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

| option | values | starts at |
| --- | --- | --- |
| double tap | false, true | true |
| long press after | 500 ms, 1 s | 500 ms |
| buttons | left, right, left, middle, right | left |

## Listener, MouseRegion

The mouse events over a widget as they come: presses, releases, motion and the wheel; the mouse coming in and going out, the pointer a hand over it where the terminal shows that.

- `rtwk::gestures::Listener` (struct) in `crates/rtwk-widgets/src/gestures/mouse.rs:74`: Gives the mouse events over its child to its callbacks, each with the cell in the coordinates of this widget.
- `rtwk::gestures::MouseRegion` (struct) in `crates/rtwk-widgets/src/gestures/mouse.rs:270`: Tells when the mouse comes in over its child (on_enter), moves over it with no button held (on_hover) and goes out (on_exit), and sets the pointer's shape over it (cursor).

### Code

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

| option | values | starts at |
| --- | --- | --- |
| cursor | pointer, crosshair, text, grab, move, notAllowed, wait, none | pointer |

## IgnorePointer, AbsorbPointer

A subtree the mouse passes by: IgnorePointer lets it through to what is under it, AbsorbPointer stops it there.

- `rtwk::gestures::IgnorePointer` (struct) in `crates/rtwk-widgets/src/gestures/ignore.rs:37`: Shows its child, which the mouse does not reach while ignoring, going to what is under it instead.
- `rtwk::gestures::AbsorbPointer` (struct) in `crates/rtwk-widgets/src/gestures/ignore.rs:119`: Shows its child, which takes the mouse while absorbing and gives it to nothing in it nor under it.

### Code

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

| option | values | starts at |
| --- | --- | --- |
| around the button | none, IgnorePointer, AbsorbPointer | IgnorePointer |

## SelectionArea

Text selected with the mouse as in a terminal, and copied to the clipboard: drag over it, or click twice or three times; a painted Selectable is selected in too, and a drag past its edge scrolls it.

- `rtwk::gestures::SelectionArea` (struct) in `crates/rtwk-widgets/src/gestures/selection.rs:423`: Lets the paragraphs below it be selected with the left button, as in a terminal, and copies what is selected.

### Code

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

| option | values | starts at |
| --- | --- | --- |
| selected in | two Texts, a painted log | two Texts |
| copy on release | false, true | true |
| style | the theme's, reverse | the theme's |

## Draggable, DragTarget

Data dragged from a widget with the mouse and dropped on another that accepts it.

- `rtwk::gestures::Draggable` (struct) in `crates/rtwk-widgets/src/gestures/drag_drop.rs:371`: Drags `data` from its child with the left button, to drop it on a DragTarget.
- `rtwk::gestures::DragTarget` (struct) in `crates/rtwk-widgets/src/gestures/drag_drop.rs:1088`: Takes the data of a Draggable dropped on it: shows what `builder` builds, again as a drag comes over it and leaves.

### Code

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

| option | values | starts at |
| --- | --- | --- |
| child when dragging | false, true | true |
