<!-- Written by `cargo xtask skill-references` from docs/widgets; do not edit. -->
# Controls

Buttons, checkboxes and switches, choices, sliders, forms, tabs and status bars.

[All widgets](../widgets.md)

A Code is the body of the gallery's preview: it names widgets by the last segment of the paths under API (`use` them) and may use `cx` or values of the gallery's page.

## Button

A label pressed by a click, or by Enter or Space while it has the focus; disabled with no on_pressed.

- `rtwk::controls::Button` (struct) in `crates/rtwk-controls/src/button.rs:107`: A button that tells on_pressed when it is clicked, or when Enter or Space is pressed while it has the focus.

### Code

```rust
struct Actions;

struct ActionsState {
    last: &'static str,
}

#[derive(Clone, PartialEq)]
enum Msg {
    Save,
    Send,
}

impl StatefulWidget for Actions {
    type State = ActionsState;
    fn create_state(&self) -> ActionsState {
        ActionsState { last: "" }
    }
}

impl State for ActionsState {
    type Widget = Actions;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, _: &Actions, _: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Save => self.last = "save",
            Msg::Send => self.last = "send",
        }
        Rebuild::Yes
    }

    fn build(&self, _: &Actions, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let theme = Theme::of(cx);
        let make = |label| {
            Button::new(label)
        };

        row![
            make("Save").on_pressed(cx.send(Msg::Save)),
            make("Send").on_pressed(cx.send(Msg::Send)),
            make("Disabled"),
        ]
        .main_axis_size(MainAxisSize::Min)
        .spacing(2)
        .into()
    }
}
impl_widget!(stateful Actions);

```

### Options

| option | values | starts at |
| --- | --- | --- |
| style | primary, accent, success, warning, error, info | primary |
| look | chip, outlined, filled, plain | chip |
| tall | false, true | false |

## Pressable

What the controls are built of: a node of the focus tree that Tab passes by while it is disabled, the keys it takes while it has the focus, a press that focuses it and a tap; it builds its look by whether it has the focus.

- `rtwk::controls::Pressable` (struct) in `crates/rtwk-controls/src/pressable.rs:143`: A control's node of the focus tree: what builder builds, which takes the focus, its keys and a click of the mouse while enabled.

### Code

```rust
struct Star;

struct StarState {
    starred: bool,
}

#[derive(Clone, PartialEq)]
enum Msg {
    Toggle,
}

impl StatefulWidget for Star {
    type State = StarState;
    fn create_state(&self) -> StarState {
        StarState { starred: true }
    }
}

impl State for StarState {
    type Widget = Star;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, _: &Star, _: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Toggle => self.starred = !self.starred,
        }
        Rebuild::Yes
    }

    fn build(&self, _: &Star, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let starred = self.starred;
        Pressable::new(move |cx, focused| {
            let theme = Theme::of(cx);
            let (star, style) = if starred {
                ("★", theme.primary.merge(&Style::new().attrs(Attrs::BOLD)))
            } else {
                ("☆", theme.muted.clone())
            };
            Text::rich([Span::styled(star, style), Span::new(" rtwk")])
                .style(if focused { theme.selection.clone() } else { Style::new() })
                .into()
        })
        .keys([("space", Some(cx.send(Msg::Toggle)))], [("space", "star")])
        .on_tap(cx.map(|_| Msg::Toggle))
        .into()
    }
}
impl_widget!(stateful Star);

```

### Options

| option | values | starts at |
| --- | --- | --- |
| style | primary, accent, success, warning, error, info | primary |

## Checkbox, Switch

What is on or off: a click, or Space or Enter, flips it.

- `rtwk::controls::Checkbox` (struct) in `crates/rtwk-controls/src/toggle.rs:64`: A checkbox before label: `[x]` while value is true, `[ ]` while it is false.
- `rtwk::controls::Switch` (struct) in `crates/rtwk-controls/src/toggle.rs:193`: A switch before label, on while value is true and off while it is false.

### Code

```rust
struct Toggles;

struct TogglesState {
    wrap: bool,
    dark: bool,
}

#[derive(Clone, PartialEq)]
enum Msg {
    Wrap(bool),
    Dark(bool),
}

impl StatefulWidget for Toggles {
    type State = TogglesState;
    fn create_state(&self) -> TogglesState {
        TogglesState { wrap: true, dark: false }
    }
}

impl State for TogglesState {
    type Widget = Toggles;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, _: &Toggles, _: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Wrap(on) => self.wrap = on,
            Msg::Dark(on) => self.dark = on,
        }
        Rebuild::Yes
    }

    fn build(&self, _: &Toggles, cx: &mut Cx<'_, Self>) -> AnyWidget {
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
        .into()
    }
}
impl_widget!(stateful Toggles);

```

### Options

| option | values | starts at |
| --- | --- | --- |
| switch | pill, brackets | pill |

## RadioGroup

One of several choices: the arrows or a click choose one.

- `rtwk::controls::RadioGroup` (struct) in `crates/rtwk-controls/src/choice.rs:193`: Radio buttons: choices, one of which is chosen, each its label after a mark, `(●)` for the one whose value is value, `( )` for the others (`Glyphs::radio_on`, `radio_off` and `radio_brackets` of the theme).

### Code

```rust
use rtwk::layout::Axis;

struct Fruit;

struct FruitState {
    chosen: Option<&'static str>,
}

#[derive(Clone, PartialEq)]
enum Msg {
    Chose(&'static str),
}

impl StatefulWidget for Fruit {
    type State = FruitState;
    fn create_state(&self) -> FruitState {
        FruitState { chosen: None }
    }
}

impl State for FruitState {
    type Widget = Fruit;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, _: &Fruit, _: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Chose(fruit) => self.chosen = Some(fruit),
        }
        Rebuild::Yes
    }

    fn build(&self, _: &Fruit, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let group = RadioGroup::new(vec![
            Choice::new("apple", "apple"),
            Choice::new("banana", "banana"),
            Choice::new("cherry", "cherry"),
            Choice::new("durian", "durian").enabled(false),
        ], self.chosen)
            .direction(Axis::Vertical)
            .on_changed(cx.map(Msg::Chose));
        group
        .into()
    }
}
impl_widget!(stateful Fruit);

```

### Options

| option | values | starts at |
| --- | --- | --- |
| direction | vertical, horizontal | vertical |
| mark | ●, *, ✓ | ● |
| theme | (●) ( ), ◉ ○ | (●) ( ) |

## SegmentedButton

A few values joined in one row, one on at a time or any of them: the arrows, a digit or a click turn one on.

- `rtwk::controls::SegmentedButton` (struct) in `crates/rtwk-controls/src/segmented_button.rs:98`: A row of segments joined into one control, of which those whose values are in selected are on: `[ All │ Errors (3) │ Warnings │ Info ]`.

### Code

```rust
struct Levels;

struct LevelsState {
    shown: Vec<&'static str>,
}

#[derive(Clone, PartialEq)]
enum Msg {
    Shown(Vec<&'static str>),
}

impl StatefulWidget for Levels {
    type State = LevelsState;
    fn create_state(&self) -> LevelsState {
        LevelsState { shown: vec!["all"] }
    }
}

impl State for LevelsState {
    type Widget = Levels;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, _: &Levels, _: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Shown(shown) => self.shown = shown,
        }
        Rebuild::Yes
    }

    fn build(&self, _: &Levels, cx: &mut Cx<'_, Self>) -> AnyWidget {
        SegmentedButton::new(vec![
            Choice::new("all", "All"),
            Choice::new("errors", "Errors (3)"),
            Choice::new("warnings", "Warnings (12)"),
            Choice::new("info", "Info").enabled(false),
        ], self.shown.clone())
            .multi_selection_enabled(false)
            .empty_selection_allowed(false)
            .show_selected_icon(false)
            .on_selection_changed(cx.map(Msg::Shown))
        .into()
    }
}
impl_widget!(stateful Levels);

```

### Options

| option | values | starts at |
| --- | --- | --- |
| multiple | false, true | false |
| allow none | false, true | false |
| selected icon | false, true | false |
| caps | square, round | square |

## Chip, ChipList

Tags on one row that wraps, with a cursor: the arrows move it, Space or a click toggles a chip, Delete or a click on its × removes it.

- `rtwk::controls::Chip` (struct) in `crates/rtwk-controls/src/chip.rs:65`: A short label on a chip of its own, on one row: a tag, a filter, a recipient.
- `rtwk::controls::ChipList` (struct) in `crates/rtwk-controls/src/chip.rs:279`: A row of Chips of chips that wraps onto more rows, with a cursor that keys and clicks move and press: `Enter` and `Space` or a click toggle the chip and tell on_selection_changed the values that are on then, as SegmentedButton does, and `Delete`, `Backspace` or a click on the `×` of a chip tell on_deleted its value.

### Code

```rust
struct Tags;

struct TagsState {
    tags: Vec<&'static str>,
    selected: Vec<&'static str>,
}

#[derive(Clone, PartialEq)]
enum Msg {
    Selection(Vec<&'static str>),
    Delete(&'static str),
}

impl StatefulWidget for Tags {
    type State = TagsState;
    fn create_state(&self) -> TagsState {
        TagsState {
            tags: vec!["rust", "tui", "terminal", "widgets", "beta", "mouse", "keyboard"],
            selected: vec!["tui"],
        }
    }
}

impl State for TagsState {
    type Widget = Tags;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, _: &Tags, _: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Selection(selected) => self.selected = selected,
            Msg::Delete(tag) => self.tags.retain(|&candidate| candidate != tag),
        }
        Rebuild::Yes
    }

    fn build(&self, _: &Tags, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let chips = ChipList::new(
            self.tags.iter()
                .map(|&tag| Choice::new(tag, tag).enabled(tag != "beta"))
                .collect::<Vec<_>>(),
        )
            .selected(self.selected.clone())
            .on_selection_changed(cx.map(Msg::Selection))
            .on_deleted(cx.map(Msg::Delete));
        SizedBox::new(Some(34), None, Some(chips.into()))
        .into()
    }
}
impl_widget!(stateful Tags);

```

### Options

| option | values | starts at |
| --- | --- | --- |
| toggle | false, true | true |
| removable | false, true | true |
| width | 20 cells, 21 cells, 22 cells, 23 cells, 24 cells, 25 cells, 26 cells, 27 cells, 28 cells, 29 cells, 30 cells, 31 cells, 32 cells, 33 cells, 34 cells, 35 cells, 36 cells, 37 cells, 38 cells, 39 cells, 40 cells, 41 cells, 42 cells, 43 cells, 44 cells, 45 cells, 46 cells, 47 cells, 48 cells, 49 cells, 50 cells | 34 cells |

## ValueStepper

One of a few values between ‹ and ›: Left and Right or a click on an arrow step to the one before or after, stopping at the ends (`wrap(true)` goes round).

- `rtwk::controls::ValueStepper` (struct) in `crates/rtwk-controls/src/value_stepper.rs:66`: The value of one of values shown between arrows, `‹ dark ›`, which Left and Right and a click on an arrow step to the value before and after it.

### Code

```rust
struct Appearance;

struct AppearanceState {
    mode: &'static str,
}

#[derive(Clone, PartialEq)]
enum Msg {
    Mode(&'static str),
}

impl StatefulWidget for Appearance {
    type State = AppearanceState;
    fn create_state(&self) -> AppearanceState {
        AppearanceState { mode: "dark" }
    }
}

impl State for AppearanceState {
    type Widget = Appearance;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, _: &Appearance, _: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Mode(mode) => self.mode = mode,
        }
        Rebuild::Yes
    }

    fn build(&self, _: &Appearance, cx: &mut Cx<'_, Self>) -> AnyWidget {
        ValueStepper::new(vec![
            Choice::new("dark", "dark"),
            Choice::new("light", "light"),
            Choice::new("sepia", "sepia").enabled(false),
            Choice::new("auto", "auto-switching"),
        ], self.mode)
            .on_changed(cx.map(Msg::Mode))
        .into()
    }
}
impl_widget!(stateful Appearance);

```

### Options

| option | values | starts at |
| --- | --- | --- |
| enabled | false, true | true |

## SelectionList

Items selected by their keys: in a single selection the cursor selects, in a multiple one Space does.

- `rtwk::controls::SelectionList` (struct) in `crates/rtwk-controls/src/selection_list.rs:118`: A list of item_count items to select one of, or any number with a multiple controller, by the keys and the mouse.

### Code

```rust
const PLANETS: [&str; 8] = [
    "Mercury", "Venus", "Earth", "Mars", "Jupiter", "Saturn", "Uranus", "Neptune",
];

// Create the controller once for this selection mode.
fn planets() -> impl Into<AnyWidget> {
    let controller = SelectionController::with(false, [2], None);

    SelectionList::new(PLANETS.len(), |i| i, |i| Text::new(PLANETS[i]).into())
        .item_enabled(|i| i != 1)
        .bar(true)
        .controller(&controller)
}
```

### Options

| option | values | starts at |
| --- | --- | --- |
| multiple | false, true | false |
| cursor bar | false, true | true |

## Select, MultiSelect

A field that opens its choices to pick one, or several, from, or a PopupController from the app; typing filters them.

- `rtwk::controls::Select` (struct) in `crates/rtwk-controls/src/select.rs:330`: A drop-down: a field that shows the label of the choice whose value is value, and opens a list of the choices to choose another from.
- `rtwk::controls::MultiSelect` (struct) in `crates/rtwk-controls/src/select.rs:393`: A MultiSelect of any number of the choices: those whose values are in values are selected.

### Code

```rust
use std::rc::Rc;

use rtwk::overlay::PanelLook;

const FRUIT: [&str; 9] = [
    "apple",
    "apricot",
    "banana",
    "blueberry",
    "cherry",
    "durian",
    "fig",
    "grape",
    "lemon",
];

struct Fruits;

struct FruitsState {
    popup: PopupController,
    choices: Vec<Choice<&'static str>>,
    one: Option<&'static str>,
    some: Rc<[&'static str]>,
}

#[derive(Clone, PartialEq)]
enum Msg {
    Open,
    One(&'static str),
    Some(Vec<&'static str>),
}

impl StatefulWidget for Fruits {
    type State = FruitsState;
    fn create_state(&self) -> FruitsState {
        FruitsState {
            popup: PopupController::new(),
            choices: FRUIT
                .into_iter()
                .map(|fruit| Choice::new(fruit, fruit).enabled(fruit != "durian"))
                .collect(),
            one: None,
            some: Rc::from([]),
        }
    }
}

impl State for FruitsState {
    type Widget = Fruits;
    type Msg = Msg;

    fn dispose(&mut self) {
        self.popup.dispose();
    }

    fn update(&mut self, msg: Msg, _: &Fruits, _: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Open => {
                self.popup.open();
                return Rebuild::No;
            }
            Msg::One(one) => self.one = Some(one),
            Msg::Some(some) => self.some = some.into(),
        }
        Rebuild::Yes
    }

    fn build(&self, _: &Fruits, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let one = Select::new(self.choices.clone(), self.one)
            .hint("a fruit")
            .inline(false)
            .rows(4)
            .controller(&self.popup)
            .on_changed(cx.map(Msg::One));
        let some = MultiSelect::new(self.choices.clone(), self.some.clone())
            .hint("some fruit")
            .inline(false)
            .rows(4)
            .on_changed(cx.map(Msg::Some));

        column![
            row![one, some].spacing(4),
            Button::new("Choose a fruit").on_pressed(cx.send(Msg::Open)),
        ]
        .spacing(1)
        .into()
    }
}
impl_widget!(stateful Fruits);

```

### Options

| option | values | starts at |
| --- | --- | --- |
| inline | false, true | false |
| rows | 4, 8 | 4 |
| popup look | raised, bordered | raised |

## ConfirmInput

A question answered by y, n or Enter.

- `rtwk::controls::ConfirmInput` (struct) in `crates/rtwk-controls/src/confirm.rs:63`: A yes-or-no question: prompt, then the answers yes and no to press, which tell on_confirmed.

### Code

```rust
struct Delete;

struct DeleteState {
    answer: Option<bool>,
}

#[derive(Clone, PartialEq)]
enum Msg {
    Answer(bool),
}

impl StatefulWidget for Delete {
    type State = DeleteState;
    fn create_state(&self) -> DeleteState {
        DeleteState { answer: None }
    }
}

impl State for DeleteState {
    type Widget = Delete;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, _: &Delete, _: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Answer(yes) => self.answer = Some(yes),
        }
        Rebuild::Yes
    }

    fn build(&self, _: &Delete, cx: &mut Cx<'_, Self>) -> AnyWidget {
        AnyWidget::from(
            ConfirmInput::new("Delete 3 files?")
                .default_value(true)
                .yes("Yes")
                .no("No")
                .on_confirmed(cx.map(Msg::Answer)),
        )
        .with_key(Key::value(true))
        .into()
    }
}
impl_widget!(stateful Delete);

```

### Options

| option | values | starts at |
| --- | --- | --- |
| default | yes, no | yes |
| answers | Yes, No, Delete, Keep | Yes, No |

## Slider

A value along a track: the arrows, a press or a drag move the thumb, which stands in eighths of a cell.

- `rtwk::controls::Slider` (struct) in `crates/rtwk-controls/src/slider.rs:90`: A value from min to max chosen along a track, by the keys or by dragging its thumb.

### Code

```rust
struct Volume;

struct VolumeState {
    value: f64,
}

#[derive(Clone, PartialEq)]
enum Msg {
    Moved(f64),
}

impl StatefulWidget for Volume {
    type State = VolumeState;
    fn create_state(&self) -> VolumeState {
        VolumeState { value: 0.4 }
    }
}

impl State for VolumeState {
    type Widget = Volume;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, _: &Volume, _: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Moved(value) => self.value = value,
        }
        Rebuild::Yes
    }

    fn build(&self, _: &Volume, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let slider = Slider::new(self.value)
            .semantics_label("Volume")
            .on_changed(cx.map(Msg::Moved));

        SizedBox::new(Some(30), None, Some(slider.into()))
        .into()
    }
}
impl_widget!(stateful Volume);

```

### Options

| option | values | starts at |
| --- | --- | --- |
| divisions | none, 10, 4 | none |
| width | 10 cells, 11 cells, 12 cells, 13 cells, 14 cells, 15 cells, 16 cells, 17 cells, 18 cells, 19 cells, 20 cells, 21 cells, 22 cells, 23 cells, 24 cells, 25 cells, 26 cells, 27 cells, 28 cells, 29 cells, 30 cells, 31 cells, 32 cells, 33 cells, 34 cells, 35 cells, 36 cells, 37 cells, 38 cells, 39 cells, 40 cells | 30 cells |

## NumberInput

A number typed, or stepped by Up and Down, the wheel or a click on ▴▾; kept within its bounds.

- `rtwk::controls::NumberInput` (struct) in `crates/rtwk-controls/src/number_input.rs:93`: A field for a number from min to max, typed, or stepped by step with the keys, the wheel and the arrows `▴▾` after it.

### Code

```rust
struct Order;

struct OrderState {
    copies: f64,
    price: f64,
}

#[derive(Clone, PartialEq)]
enum Msg {
    Copies(f64),
    Price(f64),
}

impl StatefulWidget for Order {
    type State = OrderState;
    fn create_state(&self) -> OrderState {
        OrderState { copies: 1.0, price: 9.5 }
    }
}

impl State for OrderState {
    type Widget = Order;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, _: &Order, _: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Copies(value) => self.copies = value,
            Msg::Price(value) => self.price = value,
        }
        Rebuild::Yes
    }

    fn build(&self, _: &Order, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let copies = NumberInput::new(self.copies)
            .min(1.0)
            .max(99.0)
            .on_changed(cx.map(Msg::Copies));

        let price = NumberInput::new(self.price)
            .min(0.0)
            .step(0.25)
            .decimals(2)
            .on_changed(cx.map(Msg::Price));

        column![copies, price].spacing(1)
        .into()
    }
}
impl_widget!(stateful Order);

```

## DatePicker

A date typed, stepped by Up and Down, or picked from the calendar that Alt+Down, a click on ▾ or a PopupController opens.

- `rtwk::controls::DatePicker` (struct) in `crates/rtwk-controls/src/date_picker.rs:104`: A field for a date, typed, stepped with the keys or picked from a Calendar that opens under it.

### Code

```rust
use rtwk::overlay::PanelLook;

struct Appointment;

struct AppointmentState {
    popup: PopupController,
    date: Option<DateTime>,
}

#[derive(Clone, PartialEq)]
enum Msg {
    Open,
    Picked(Option<DateTime>),
}

impl StatefulWidget for Appointment {
    type State = AppointmentState;
    fn create_state(&self) -> AppointmentState {
        AppointmentState { popup: PopupController::new(), date: None }
    }
}

impl State for AppointmentState {
    type Widget = Appointment;
    type Msg = Msg;

    fn dispose(&mut self) {
        self.popup.dispose();
    }

    fn update(&mut self, msg: Msg, _: &Appointment, _: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Open => {
                self.popup.open();
                return Rebuild::No;
            }
            Msg::Picked(date) => self.date = date,
        }
        Rebuild::Yes
    }

    fn build(&self, _: &Appointment, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let picker = DatePicker::new(self.date, Some(cx.map(Msg::Picked)))
            .controller(&self.popup)
            .first_date(DateTime::new(2026, 1, 1))
            .last_date(DateTime::new(2026, 12, 31))
            .today(DateTime::new(2026, 10, 19))
            .show_time(false)
            .first_weekday(DateTime::MONDAY)
            .popup_look(PanelLook::Raised);

        row![
            SizedBox::new(Some(22), None, Some(picker.into())),
            Button::new("Calendar").on_pressed(cx.send(Msg::Open)),
        ]
        .spacing(2)
        .into()
    }
}
impl_widget!(stateful Appointment);

```

### Options

| option | values | starts at |
| --- | --- | --- |
| time of day | false, true | false |
| weeks start | monday, sunday | monday |
| popup look | raised, bordered | raised |

## FilePicker

A file picked from the directory listed under a field that names its path: the arrows and Enter go through it, Tab completes a name; a FilePickerController goes to a directory.

- `rtwk::controls::FilePicker` (struct) in `crates/rtwk-controls/src/file_picker.rs:1122`: A file browser: a directory's entries to pick from, under a field that names a path, which calls on_picked with the paths picked.

### Code

```rust
use std::rc::Rc;

fn filters() -> Vec<FileFilter> {
    vec![
        FileFilter::new("Rust").extensions(["rs"]),
        FileFilter::new("All files"),
    ]
}

fn options(fs: SharedFileSystem) -> FilePickerOptions {
    FilePickerOptions::new()
        .mode(FilePickerMode::Open)
        .multiple(false)
        .show_hidden(false)
        .file_system(fs)
        .filters(filters())
}

struct Files;

struct FilesState {
    fs: SharedFileSystem,
    picker: FilePickerController,
    picked: Option<String>,
}

#[derive(Clone, PartialEq)]
enum Msg {
    Dialog,
    Picked(Vec<String>),
}

impl StatefulWidget for Files {
    type State = FilesState;
    fn create_state(&self) -> FilesState {
        FilesState {
            fs: Rc::new(
                InMemoryFileSystem::new(["/home/user/src/main.rs", "/home/user/Cargo.toml"])
                    .with_current_directory("/home/user"),
            ),
            picker: FilePickerController::new(),
            picked: None,
        }
    }
}

impl State for FilesState {
    type Widget = Files;
    type Msg = Msg;

    fn dispose(&mut self) {
        self.picker.dispose();
    }

    fn update(&mut self, msg: Msg, _: &Files, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            // Open from a handler, then await the result in a task.
            Msg::Dialog => {
                let result = show_file_picker(cx, options(self.fs.clone()));
                let sender = cx.channel();
                cx.spawn(async move {
                    if let Some(paths) = result.await {
                        let _ = sender.send(Msg::Picked(paths));
                    }
                    None
                });
                return Rebuild::No;
            }
            Msg::Picked(paths) => self.picked = Some(paths.join(", ")),
        }
        Rebuild::Yes
    }

    fn build(&self, _: &Files, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let picker = FilePicker::new(cx.map(Msg::Picked))
            .controller(self.picker.clone())
            .file_system(self.fs.clone())
            .filters(filters())
            .mode(FilePickerMode::Open)
            .multiple(false)
            .show_hidden(false);

        column![
            SizedBox::new(None, Some(8), Some(picker.into())),
            row![
                Button::new("In a dialog").on_pressed(cx.send(Msg::Dialog)),
                Text::new(self.picked.clone().unwrap_or_default()),
            ]
            .spacing(2),
        ]
        .spacing(1)
        .into()
    }
}
impl_widget!(stateful Files);

```

### Options

| option | values | starts at |
| --- | --- | --- |
| mode | open, open several, save, directory | open |
| show hidden | false, true | false |

## Form, FormField

Fields validated and submitted together: Enter goes on to the next field, and from the last submits the form.

- `rtwk::controls::Form` (struct) in `crates/rtwk-controls/src/form.rs:110`: Keeps the FormFields below child together, to validate, save, reset and submit them as one.
- `rtwk::controls::FormField` (struct) in `crates/rtwk-controls/src/form.rs:526`: A field of the nearest Form above it, which holds a value that its builder shows and edits.

### Code

```rust
struct Signup;

struct SignupState {
    name: Option<String>,
    submitted: Option<String>,
}

#[derive(Clone, PartialEq)]
enum Msg {
    Name(Option<String>),
    Submitted,
}

impl StatefulWidget for Signup {
    type State = SignupState;
    fn create_state(&self) -> SignupState {
        SignupState { name: None, submitted: None }
    }
}

impl State for SignupState {
    type Widget = Signup;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, _: &Signup, _: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Name(name) => self.name = name,
            Msg::Submitted => self.submitted.clone_from(&self.name),
        }
        Rebuild::Yes
    }

    fn build(&self, _: &Signup, cx: &mut Cx<'_, Self>) -> AnyWidget {
        // Build SignupFields below Form so its buttons can find Form::of(cx).
        Form::new(SignupFields {
            on_saved: cx.map(Msg::Name),
            result: Text::new(format!(" submitted: {}", self.submitted.as_deref().unwrap_or("nothing")))
                .wrap(TextWrap::None)
                .into(),
        })
        .autovalidate_mode(AutovalidateMode::Disabled)
        .on_submitted(cx.send(Msg::Submitted))
        .into()
    }
}
impl_widget!(stateful Signup);

#[derive(Debug, PartialEq)]
struct SignupFields {
    on_saved: Handler<Option<String>>,
    result: AnyWidget,
}

struct SignupFieldsState;

#[derive(Clone, PartialEq)]
enum FormCommand {
    Submit,
    Reset,
}

impl StatefulWidget for SignupFields {
    type State = SignupFieldsState;
    fn create_state(&self) -> SignupFieldsState {
        SignupFieldsState
    }
}

impl State for SignupFieldsState {
    type Widget = SignupFields;
    type Msg = FormCommand;

    fn update(&mut self, msg: FormCommand, _: &SignupFields, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            FormCommand::Submit => {
                Form::of(cx).submit(cx);
            }
            FormCommand::Reset => Form::of(cx).reset(cx),
        }
        Rebuild::Yes
    }

    fn build(&self, widget: &SignupFields, cx: &mut Cx<'_, Self>) -> AnyWidget {
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
        .into()
    }
}
impl_widget!(stateful eq SignupFields);

```

### Options

| option | values | starts at |
| --- | --- | --- |
| validate | Disabled, Always, OnUserInteraction, OnUnfocus | Disabled |

## TabBar, TabBarView

Tabs to pick a view by: Left and Right, or a click.

- `rtwk::controls::TabBar` (struct) in `crates/rtwk-controls/src/tabs.rs:237`: A row of the labels of the controller's tabs, its selected one marked, which the keys and the mouse switch.
- `rtwk::controls::TabBarView` (struct) in `crates/rtwk-controls/src/tabs.rs:1008`: Shows the content of the controller's selected tab, and keeps the others with their state.

### Code

```rust
struct Settings;

struct SettingsState {
    tabs: TabController,
    wrap: bool,
    hidden: bool,
    indent: &'static str,
}

#[derive(Clone, PartialEq)]
enum Msg {
    Wrap(bool),
    Hidden(bool),
    Indent(&'static str),
}

impl StatefulWidget for Settings {
    type State = SettingsState;
    fn create_state(&self) -> SettingsState {
        SettingsState {
            tabs: TabController::new(3),
            wrap: true,
            hidden: false,
            indent: "spaces",
        }
    }
}

impl State for SettingsState {
    type Widget = Settings;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, _: &Settings, _: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Wrap(value) => self.wrap = value,
            Msg::Hidden(value) => self.hidden = value,
            Msg::Indent(value) => self.indent = value,
        }
        Rebuild::Yes
    }

    fn build(&self, _: &Settings, cx: &mut Cx<'_, Self>) -> AnyWidget {
        // Recreate the TabController when the tab count changes.
        let bar = TabBar::new(&self.tabs, ["General", "Editor", "Keys"]);
        let general = column![
            Checkbox::new(self.wrap).label("wrap lines").on_changed(cx.map(Msg::Wrap)),
            Checkbox::new(self.hidden).label("show hidden files").on_changed(cx.map(Msg::Hidden)),
        ]
        .cross_axis_alignment(CrossAxisAlignment::Start);
        let editor = column![
            Text::new("Indent with"),
            RadioGroup::new([Choice::new("spaces", "spaces"), Choice::new("tabs", "tabs")],
                Some(self.indent))
                .direction(Axis::Horizontal)
                .on_changed(cx.map(Msg::Indent)),
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
        .into()
    }
}
impl_widget!(stateful Settings);

```

### Options

| option | values | starts at |
| --- | --- | --- |
| tabs | 3, 7 | 3 |
| look | plain, strip | plain |

## KeyHintsBar

The keys that can be pressed now, with their hints, as the focus moves: Tab to the field to see its keys.

- `rtwk::controls::KeyHintsBar` (struct) in `crates/rtwk-controls/src/key_hints.rs:104`: A row of the keys that can be pressed now, each before its hint: `ctrl+s save   q quit`.

### Code

```rust
struct Hinted;

struct HintedState {
    autosave: bool,
    did: Option<&'static str>,
}

#[derive(Clone, PartialEq)]
enum Msg {
    Act(&'static str),
    Autosave(bool),
}

impl StatefulWidget for Hinted {
    type State = HintedState;
    fn create_state(&self) -> HintedState {
        HintedState { autosave: false, did: None }
    }
}

impl State for HintedState {
    type Widget = Hinted;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, _: &Hinted, _: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Act(action) => self.did = Some(action),
            Msg::Autosave(on) => self.autosave = on,
        }
        Rebuild::Yes
    }

    fn build(&self, _: &Hinted, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let theme = Theme::of(cx);
        let hints = KeyHintsBar::new();
        let body = column![
            row![
                Expanded::new(1, TextField::new().placeholder("a field")
                    .focused_style(theme.selection.clone())),
                Checkbox::new(self.autosave).label("autosave")
                    .on_changed(cx.map(Msg::Autosave)),
            ].spacing(2),
            Text::new(format!("did: {}", self.did.unwrap_or("nothing"))),
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
                ("ctrl+s", cx.send(Msg::Act("save"))),
                ("ctrl+x ctrl+o", cx.send(Msg::Act("open"))),
                ("f2", cx.send(Msg::Act("rename"))),
            ],
            SizedBox::new(Some(44), None, Some(body.into())),
        )
        .hints([("ctrl+s", "save"), ("ctrl+x ctrl+o", "open")])
        .into()
    }
}
impl_widget!(stateful Hinted);

```

### Options

| option | values | starts at |
| --- | --- | --- |
| unhinted keys | false, true | false |
| join keys | false, true | true |
| keys named by | signs, names | signs |
| separator | blanks, dot, line | blanks |

## StatusMessage, Badge

A message of a kind after its icon, in its color; a short label on its color.

- `rtwk::controls::StatusMessage` (struct) in `crates/rtwk-controls/src/status.rs:110`: A message of a kind after the kind's icon in its color: `✓ Saved 3 files`.
- `rtwk::controls::Badge` (struct) in `crates/rtwk-controls/src/status.rs:231`: A short label set off by its color: a count, a state; ` 3 `, ` new `.

### Code

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

| option | values | starts at |
| --- | --- | --- |
| kind | info, success, warning, error | info |

## Callout

A notice in a box lined in the color of its kind, its title on the top edge; [×] or Escape dismisses it, and buttons below act on it.

- `rtwk::controls::Callout` (struct) in `crates/rtwk-controls/src/callout.rs:87`: A notice in a box lined in the color of its kind: what a command-line tool prints for "a newer version is available", a hint, a fatal error.

### Code

```rust
struct Notice;

struct NoticeState {
    shown: bool,
    did: Option<&'static str>,
}

#[derive(Clone, PartialEq)]
enum Msg {
    Dismiss,
    Restore,
    Act(&'static str),
}

impl StatefulWidget for Notice {
    type State = NoticeState;
    fn create_state(&self) -> NoticeState {
        NoticeState { shown: true, did: None }
    }
}

impl State for NoticeState {
    type Widget = Notice;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, _: &Notice, _: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Dismiss => {
                self.shown = false;
                self.did = Some("dismissed");
            }
            Msg::Restore => {
                self.shown = true;
                self.did = Some("shown again");
            }
            Msg::Act(name) => self.did = Some(name),
        }
        Rebuild::Yes
    }

    fn build(&self, _: &Notice, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let notice: AnyWidget = if self.shown {
            let notice = Callout::text("A newer version of rtwk is out: 0.3.0.")
                .kind(StatusKind::Info)
                .border_type(BorderType::Single)
                .on_dismissed(cx.send(Msg::Dismiss))
                .actions([Button::new("Update")
                    .on_pressed(cx.send(Msg::Act("update")))
                    .into()]);
            SizedBox::new(Some(44), None, Some(notice.into())).into()
        } else {
            Button::new("Show it again")
                .on_pressed(cx.send(Msg::Restore))
                .into()
        };

        column![
            notice,
            Text::new(format!("did: {}", self.did.unwrap_or("nothing"))),
        ]
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .spacing(1)
        .into()
    }
}
impl_widget!(stateful Notice);

```

### Options

| option | values | starts at |
| --- | --- | --- |
| kind | info, success, warning, error | info |
| dismissible | false, true | true |
| actions | false, true | true |
| title | kind, own | kind |
| icon | kind, own, none | kind |
| border | single, rounded, heavy | single |
| width | 24 cells, 25 cells, 26 cells, 27 cells, 28 cells, 29 cells, 30 cells, 31 cells, 32 cells, 33 cells, 34 cells, 35 cells, 36 cells, 37 cells, 38 cells, 39 cells, 40 cells, 41 cells, 42 cells, 43 cells, 44 cells, 45 cells, 46 cells, 47 cells, 48 cells, 49 cells, 50 cells, 51 cells, 52 cells, 53 cells, 54 cells, 55 cells, 56 cells, 57 cells, 58 cells, 59 cells, 60 cells | 44 cells |

## Hyperlink

Text linked to a URL, which a terminal that knows hyperlinks opens.

- `rtwk::controls::Hyperlink` (struct) in `crates/rtwk-controls/src/link.rs:45`: Text linked to url as a hyperlink (OSC 8), which a terminal that knows them opens when it is clicked.

### Code

```rust
column![
    Hyperlink::new("https://www.rust-lang.org").text("The Rust language"),
    Hyperlink::new("https://crates.io"),
    Hyperlink::new("file:///home/user/rtwk/README.md").text("the README"),
]
.main_axis_size(MainAxisSize::Min)
.cross_axis_alignment(CrossAxisAlignment::Start)
.spacing(1)
```

### Options

| option | values | starts at |
| --- | --- | --- |
| style | primary, accent, info | primary |
| shows | text, url | text |

## ListTile

A row of a list: a title over a subtitle, something before and after them; tapped by a click or Enter, lit while focused or under the mouse.

- `rtwk::controls::ListTile` (struct) in `crates/rtwk-controls/src/list_tile.rs:63`: A row of a list: title, under it subtitle, leading before them and trailing after them, a column apart; tapped, by a click or by Enter or Space while it has the focus, it tells on_tap.

### Code

```rust
struct Files;

struct FilesState {
    open: usize,
}

#[derive(Clone, PartialEq)]
enum Msg {
    Open(usize),
}

impl StatefulWidget for Files {
    type State = FilesState;
    fn create_state(&self) -> FilesState {
        FilesState { open: 0 }
    }
}

impl State for FilesState {
    type Widget = Files;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, _: &Files, _: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Open(i) => self.open = i,
        }
        Rebuild::Yes
    }

    fn build(&self, _: &Files, cx: &mut Cx<'_, Self>) -> AnyWidget {
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
                .on_tap(cx.send(Msg::Open(i)))
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
        .into()
    }
}
impl_widget!(stateful Files);

```

### Options

| option | values | starts at |
| --- | --- | --- |
| leading | false, true | true |
| subtitle | false, true | true |
| trailing | false, true | true |

## StatusBar

A row of three sections, left, centre and right; those that do not fit give way, the centre first.

- `rtwk::controls::StatusBar` (struct) in `crates/rtwk-controls/src/status_bar.rs:60`: A status bar: one row of three sections, left at its left edge, right at its right edge and center in its middle.

### Code

```rust
let theme = Theme::of(cx);
let bar = StatusBar::new()
    .spacing(1)
    .left(Badge::new("NORMAL"))
    .center(Text::new("examples/widgets/main.rs")
        .wrap(TextWrap::None)
        .overflow(TextOverflow::Ellipsis))
    .right(Text::new("Ln 12, Col 4").style(theme.muted.clone()));
SizedBox::new(Some(50), None, Some(bar.into()))
```

### Options

| option | values | starts at |
| --- | --- | --- |
| width | 12 cells, 13 cells, 14 cells, 15 cells, 16 cells, 17 cells, 18 cells, 19 cells, 20 cells, 21 cells, 22 cells, 23 cells, 24 cells, 25 cells, 26 cells, 27 cells, 28 cells, 29 cells, 30 cells, 31 cells, 32 cells, 33 cells, 34 cells, 35 cells, 36 cells, 37 cells, 38 cells, 39 cells, 40 cells, 41 cells, 42 cells, 43 cells, 44 cells, 45 cells, 46 cells, 47 cells, 48 cells, 49 cells, 50 cells | 50 cells |
| spacing | 0, 1, 3 | 1 |
