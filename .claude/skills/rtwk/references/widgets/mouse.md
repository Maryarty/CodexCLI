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
struct GesturePad;

struct GesturePadState {
    taps: u64,
    doubles: u64,
    holds: u64,
    drags: u64,
    delta: (i32, i32),
    last: Option<String>,
    from: Option<(i32, i32)>,
    at: Option<(i32, i32)>,
}

#[derive(Clone, PartialEq)]
enum GestureMsg {
    Tap(PointerEvent),
    Double(PointerEvent),
    Hold(PointerEvent),
    Start(DragDetails),
    Move(DragDetails),
    End(DragDetails),
}

impl StatefulWidget for GesturePad {
    type State = GesturePadState;
    fn create_state(&self) -> GesturePadState {
        GesturePadState {
            taps: 0,
            doubles: 0,
            holds: 0,
            drags: 0,
            delta: (0, 0),
            last: None,
            from: None,
            at: None,
        }
    }
}

impl State for GesturePadState {
    type Widget = GesturePad;
    type Msg = GestureMsg;

    fn update(&mut self, msg: GestureMsg, _: &GesturePad, _: &mut Cx<'_, Self>) -> Rebuild {
        // A tap says nothing of where; a drag does.
        let (name, at) = match msg {
            GestureMsg::Tap(_) => {
                self.taps += 1;
                ("tap", None)
            }
            GestureMsg::Double(_) => {
                self.doubles += 1;
                ("double tap", None)
            }
            GestureMsg::Hold(_) => {
                self.holds += 1;
                ("long press", None)
            }
            GestureMsg::Start(drag) => {
                self.from = Some((drag.x(), drag.y()));
                return Rebuild::Yes;
            }
            GestureMsg::Move(drag) => {
                self.delta.0 += drag.dx;
                self.delta.1 += drag.dy;
                self.drags += 1;
                self.at = Some((drag.x(), drag.y()));
                ("drag", Some((drag.event.x, drag.event.y)))
            }
            GestureMsg::End(drag) => {
                self.from = None;
                self.at = None;
                ("drag ended", Some((drag.event.x, drag.event.y)))
            }
        };
        self.last = Some(match at {
            Some((x, y)) => format!("{name} at {x}, {y}"),
            None => name.to_owned(),
        });
        Rebuild::Yes
    }

    fn build(&self, _: &GesturePad, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let theme = Theme::of(cx);
        let double = true;
        let long = Duration::from_millis(500);
        let buttons = Buttons::of(&[0]);

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
                SizedBox::fixed(36, 7).child(
                    BoxWidget::new()
                        .border(Border::all(theme.border.clone()).kind(BorderType::Rounded))
                        .child(BoxWidget::new().style(theme.surface.clone()).child(marks)),
                ),
            ),
            column![
                row![
                    Text::rich([Span::styled("taps ", theme.muted.clone()), Span::new(self.taps.to_string())]),
                    Text::rich([Span::styled(", double ", theme.muted.clone()), Span::new(self.doubles.to_string())]),
                    Text::rich([Span::styled(", long ", theme.muted.clone()), Span::new(self.holds.to_string())]),
                ]
                .main_axis_size(MainAxisSize::Min),
                Text::rich([Span::styled("dragged ", theme.muted.clone()), Span::new(format!("{} across, {} down", self.delta.0, self.delta.1))]),
            ]
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Start),
        ]
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .spacing(1)

        .into()
    }
}
impl_widget!(stateful GesturePad);

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
use std::collections::VecDeque;

struct EventPad;

struct EventPadState {
    over: bool,
    events: VecDeque<String>,
}

#[derive(Clone, PartialEq)]
enum EventMsg {
    Enter(PointerEvent),
    Exit(PointerEvent),
    Down(PointerEvent),
    Other(&'static str, PointerEvent),
    Scroll(PointerEvent),
}

impl StatefulWidget for EventPad {
    type State = EventPadState;
    fn create_state(&self) -> EventPadState {
        EventPadState {
            over: false,
            events: VecDeque::new(),
        }
    }
}

impl State for EventPadState {
    type Widget = EventPad;
    type Msg = EventMsg;

    fn update(&mut self, msg: EventMsg, _: &EventPad, _: &mut Cx<'_, Self>) -> Rebuild {
        let (name, event) = match msg {
            EventMsg::Enter(event) => {
                self.over = true;
                ("enter".to_owned(), event)
            }
            EventMsg::Exit(event) => {
                self.over = false;
                ("exit".to_owned(), event)
            }
            EventMsg::Down(event) => (format!("press of button {}", event.button), event),
            EventMsg::Other(name, event) => (name.to_owned(), event),
            EventMsg::Scroll(event) => (event.kind.name().to_owned(), event),
        };
        self.events.push_front(format!("{name} at {}, {}", event.x, event.y));
        self.events.truncate(4);
        Rebuild::Yes
    }

    fn build(&self, _: &EventPad, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let theme = Theme::of(cx);
        let cursor = Some(PointerShape::Pointer);

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
                SizedBox::fixed(36, 7).child(
                    BoxWidget::new().border(Border::all(edge.clone()).kind(BorderType::Rounded)).child(
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
                events.child(Text::rich([Span::new(event.clone())]))
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
}
impl_widget!(stateful EventPad);

```

### Options

| option | values | starts at |
| --- | --- | --- |
| cursor | Pointer, Crosshair, Text, Grab, Move, NotAllowed, Wait, None | Pointer |

## IgnorePointer, AbsorbPointer

A subtree the mouse passes by: IgnorePointer lets it through to what is under it, AbsorbPointer stops it there.

- `rtwk::gestures::IgnorePointer` (struct) in `crates/rtwk-widgets/src/gestures/ignore.rs:37`: Shows its child, which the mouse does not reach while ignoring, going to what is under it instead.
- `rtwk::gestures::AbsorbPointer` (struct) in `crates/rtwk-widgets/src/gestures/ignore.rs:119`: Shows its child, which takes the mouse while absorbing and gives it to nothing in it nor under it.

### Code

```rust
struct GatePad;

struct GatePadState {
    over: bool,
    heard: Option<&'static str>,
}

#[derive(Clone, PartialEq)]
enum GateMsg {
    Hover(bool),
    Hear(&'static str),
}

impl StatefulWidget for GatePad {
    type State = GatePadState;
    fn create_state(&self) -> GatePadState {
        GatePadState {
            over: false,
            heard: None,
        }
    }
}

impl State for GatePadState {
    type Widget = GatePad;
    type Msg = GateMsg;

    fn update(&mut self, msg: GateMsg, _: &GatePad, _: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            GateMsg::Hover(over) => self.over = over,
            GateMsg::Hear(heard) => self.heard = Some(heard),
        }
        Rebuild::Yes
    }

    fn build(&self, _: &GatePad, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let theme = Theme::of(cx);
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
                            .border(Border::all(edge.clone()).kind(BorderType::Rounded))
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
                            SizedBox::fixed(36, 7).child(
                                BoxWidget::new()
                                    .border(Border::all(theme.border.clone()).kind(BorderType::Rounded))
                                    .title(BorderTitle::new(" backdrop ").style(theme.muted.clone())),
                            )
                        )
                )
                .child(Positioned::fill(Center::new(foreground))),
            Text::rich([Span::styled("heard: ", theme.muted.clone()), Span::styled(self.heard.unwrap_or("nothing"), if self.heard.is_none() { theme.muted.clone() } else { Style::new() })])
        ]
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .spacing(1)

        .into()
    }
}
impl_widget!(stateful GatePad);

```

### Options

| option | values | starts at |
| --- | --- | --- |
| around the button | none, IgnorePointer, AbsorbPointer | IgnorePointer |

## SelectionArea

Text selected with the mouse as in a terminal, and copied to the clipboard: drag over it, or click twice or three times; a painted Selectable is selected in too, and a drag past its edge scrolls it.

- `rtwk::gestures::SelectionArea` (struct) in `crates/rtwk-widgets/src/gestures/selection.rs:671`: Lets the paragraphs below it be selected with the left button, as in a terminal, and copies what is selected.

### Code

```rust
// The area's controller is a Listenable of its own selection changes.
#[derive(Clone)]
struct SelectionChanges(SelectionAreaController);

impl Listenable for SelectionChanges {
    fn add_listener(&self, listener: VoidCallback) {
        self.0.add_listener(listener);
    }

    fn remove_listener(&self, listener: &VoidCallback) {
        self.0.remove_listener(listener);
    }
}

struct Selecting;

struct SelectingState {
    copied: String,
    controller: SelectionAreaController,
    listening: Option<Listening>,
}

#[derive(Clone, PartialEq)]
enum SelectMsg {
    Copied(String),
    SelectionChanged,
    Clear,
}

impl StatefulWidget for Selecting {
    type State = SelectingState;
    fn create_state(&self) -> SelectingState {
        SelectingState {
            copied: String::new(),
            controller: SelectionAreaController::default(),
            listening: None,
        }
    }
}

impl State for SelectingState {
    type Widget = Selecting;
    type Msg = SelectMsg;

    fn init_state(&mut self, _: &Selecting, cx: &mut Cx<'_, Self>) {
        self.listening = Some(Listening::new(
            cx,
            Rc::new(SelectionChanges(self.controller.clone())),
            SelectMsg::SelectionChanged,
        ));
    }

    fn dispose(&mut self) {
        if let Some(listening) = &self.listening {
            listening.cancel();
        }
    }

    fn update(&mut self, msg: SelectMsg, _: &Selecting, _: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            SelectMsg::Copied(text) => self.copied = text,
            SelectMsg::SelectionChanged => {
                self.listening.as_ref().expect("mounted selection").handled();
            }
            SelectMsg::Clear => self.controller.clear(),
        }
        Rebuild::Yes
    }

    fn build(&self, _: &Selecting, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let theme = Theme::of(cx);
        let copy_on_release = true;
        let reverse = false;

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
                            Text::rich([Span::styled(text, if self.copied.is_empty() { theme.muted.clone() } else { Style::new() })])
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
}
impl_widget!(stateful Selecting);

```

### Options

| option | values | starts at |
| --- | --- | --- |
| selected in | two Texts, a painted log, a numbered listing | two Texts |
| copy on release | false, true | true |
| style | the theme's, reverse | the theme's |

## Draggable, DragTarget

Data dragged from a widget with the mouse and dropped on another that accepts it.

- `rtwk::gestures::Draggable` (struct) in `crates/rtwk-widgets/src/gestures/drag_drop.rs:371`: Drags `data` from its child with the left button, to drop it on a DragTarget.
- `rtwk::gestures::DragTarget` (struct) in `crates/rtwk-widgets/src/gestures/drag_drop.rs:1098`: Takes the data of a Draggable dropped on it: shows what `builder` builds, again as a drag comes over it and leaves.

### Code

```rust
// Wrap the app content in DragLayer::new(content) for drag feedback.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Fruit {
    Apple,
    Banana,
    Cherry,
    Grape,
}

impl Fruit {
    const ALL: [Self; 4] = [Self::Apple, Self::Banana, Self::Cherry, Self::Grape];

    fn name(self) -> &'static str {
        match self {
            Self::Apple => "apple",
            Self::Banana => "banana",
            Self::Cherry => "cherry",
            Self::Grape => "grape",
        }
    }

    fn color(self) -> Color {
        match self {
            Self::Apple | Self::Cherry => Color::RED,
            Self::Banana => Color::YELLOW,
            Self::Grape => Color::MAGENTA,
        }
    }

    fn red(self) -> bool {
        matches!(self, Self::Apple | Self::Cherry)
    }
}

const BOX_NAMES: [&str; 3] = ["bowl", "basket", "red only"];
const EMPTY: [&str; 3] = ["empty", "drop here", "drop here"];

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
struct FruitBoxes;

struct FruitBoxesState {
    boxes: [Vec<Fruit>; 3],
    status: Option<String>,
}

#[derive(Clone, PartialEq)]
enum FruitMsg {
    Drop(usize, Fruit),
    Cancel(usize, Fruit),
}

impl StatefulWidget for FruitBoxes {
    type State = FruitBoxesState;
    fn create_state(&self) -> FruitBoxesState {
        FruitBoxesState {
            boxes: [Fruit::ALL.to_vec(), Vec::new(), Vec::new()],
            status: None,
        }
    }
}

impl State for FruitBoxesState {
    type Widget = FruitBoxes;
    type Msg = FruitMsg;

    fn update(&mut self, msg: FruitMsg, _: &FruitBoxes, _: &mut Cx<'_, Self>) -> Rebuild {
        self.status = Some(match msg {
            FruitMsg::Drop(to, fruit) => {
                if !self.boxes[to].contains(&fruit) {
                    for container in &mut self.boxes {
                        container.retain(|item| *item != fruit);
                    }
                    self.boxes[to].push(fruit);
                }
                format!("{} → {}", fruit.name(), BOX_NAMES[to])
            }
            FruitMsg::Cancel(from, fruit) => {
                format!("{}, back to the {}", fruit.name(), BOX_NAMES[from])
            }
        });
        Rebuild::Yes
    }

    fn build(&self, _: &FruitBoxes, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let theme = Theme::of(cx);
        let ghost = true;

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
                            .border(Border::all(edge.clone()).kind(BorderType::Rounded))
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
            Text::rich([Span::styled("dropped: ", theme.muted.clone()), Span::styled(self.status.clone().unwrap_or_else(|| "nothing".into()), if self.status.is_none() { theme.muted.clone() } else { Style::new() })]),
        ]
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .spacing(1)

        .into()
    }
}
impl_widget!(stateful FruitBoxes);

```

### Options

| option | values | starts at |
| --- | --- | --- |
| child when dragging | false, true | true |
