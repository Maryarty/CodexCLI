<!-- Written by `cargo xtask skill-references` from docs/widgets; do not edit. -->
# Controls

Buttons, checkboxes and switches, choices, sliders, forms, tabs and status bars.

[All widgets](../widgets.md)

A Code is the body of the gallery's preview: it names widgets by the last segment of the paths under API (`use` them) and may use `cx` or values of the gallery's page.

## Button

A label pressed by a click, or by Enter or Space while it has the focus; disabled with no onPressed.

- `rtwk::controls::Button` (struct) in `crates/rtwk-controls/src/button.rs:107`: A button that tells on_pressed when it is clicked, or when Enter or Space is pressed while it has the focus.

### Code

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

| option | values | starts at |
| --- | --- | --- |
| style | primary, accent, success, warning, error, info | primary |

## Checkbox, Switch

What is on or off: a click, or Space or Enter, flips it.

- `rtwk::controls::Checkbox` (struct) in `crates/rtwk-controls/src/toggle.rs:64`: A checkbox before label: `[x]` while value is true, `[ ]` while it is false.
- `rtwk::controls::Switch` (struct) in `crates/rtwk-controls/src/toggle.rs:193`: A switch before label, on while value is true and off while it is false.

### Code

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

| option | values | starts at |
| --- | --- | --- |
| switch | pill, brackets | pill |

## RadioGroup

One of several choices: the arrows or a click choose one.

- `rtwk::controls::RadioGroup` (struct) in `crates/rtwk-controls/src/choice.rs:193`: Radio buttons: choices, one of which is chosen, each its label after a mark, `(●)` for the one whose value is value, `( )` for the others (`Glyphs::radio_on`, `radio_off` and `radio_brackets` of the theme).

### Code

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

| option | values | starts at |
| --- | --- | --- |
| toggle | false, true | true |
| removable | false, true | true |
| width | 20 cells, 21 cells, 22 cells, 23 cells, 24 cells, 25 cells, 26 cells, 27 cells, 28 cells, 29 cells, 30 cells, 31 cells, 32 cells, 33 cells, 34 cells, 35 cells, 36 cells, 37 cells, 38 cells, 39 cells, 40 cells, 41 cells, 42 cells, 43 cells, 44 cells, 45 cells, 46 cells, 47 cells, 48 cells, 49 cells, 50 cells | 34 cells |

## ValueStepper

One of a few values between ‹ and ›: Left and Right or a click on an arrow step to the one before or after, stopping at the ends (`wrap: true` goes round).

- `rtwk::controls::ValueStepper` (struct) in `crates/rtwk-controls/src/value_stepper.rs:66`: The value of one of values shown between arrows, `‹ dark ›`, which Left and Right and a click on an arrow step to the value before and after it.

### Code

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

| option | values | starts at |
| --- | --- | --- |
| enabled | false, true | true |

## SelectionList

Items selected by their keys: in a single selection the cursor selects, in a multiple one Space does.

- `rtwk::controls::SelectionList` (struct) in `crates/rtwk-controls/src/selection_list.rs:116`: A list of item_count items to select one of, or any number with a multiple controller, by the keys and the mouse.

### Code

```rust
// Create the controller once for this selection mode.
let controller = SelectionController::with(false, [2], None);

SelectionList::new(PLANETS.len(), |i| i, |i| Text::new(PLANETS[i]).into())
    .item_enabled(|i| i != 1)
    .bar(true)
    .controller(&controller)
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

| option | values | starts at |
| --- | --- | --- |
| default | yes, no | yes |
| answers | Yes, No, Delete, Keep | Yes, No |

## Slider

A value along a track: the arrows, a press or a drag move the thumb, which stands in eighths of a cell.

- `rtwk::controls::Slider` (struct) in `crates/rtwk-controls/src/slider.rs:88`: A value from min to max chosen along a track, by the keys or by dragging its thumb.

### Code

```rust
let slider = Slider::new(self.value)
    .semantics_label("Volume")
    .on_changed(cx.map(|value| value));

SizedBox::new(Some(30), None, Some(slider.into()))
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

## DatePicker

A date typed, stepped by Up and Down, or picked from the calendar that Alt+Down, a click on ▾ or a PopupController opens.

- `rtwk::controls::DatePicker` (struct) in `crates/rtwk-controls/src/date_picker.rs:104`: A field for a date, typed, stepped with the keys or picked from a Calendar that opens under it.

### Code

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

| option | values | starts at |
| --- | --- | --- |
| validate | disabled, always, onUserInteraction, onUnfocus | disabled |

## TabBar, TabBarView

Tabs to pick a view by: Left and Right, or a click.

- `rtwk::controls::TabBar` (struct) in `crates/rtwk-controls/src/tabs.rs:237`: A row of the labels of the controller's tabs, its selected one marked, which the keys and the mouse switch.
- `rtwk::controls::TabBarView` (struct) in `crates/rtwk-controls/src/tabs.rs:1008`: Shows the content of the controller's selected tab, and keeps the others with their state.

### Code

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

| option | values | starts at |
| --- | --- | --- |
| tabs | 3, 7 | 3 |
| look | plain, strip | plain |

## KeyHintsBar

The keys that can be pressed now, with their hints, as the focus moves: Tab to the field to see its keys.

- `rtwk::controls::KeyHintsBar` (struct) in `crates/rtwk-controls/src/key_hints.rs:104`: A row of the keys that can be pressed now, each before its hint: `ctrl+s save   q quit`.

### Code

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
    Hyperlink::new("https://dart.dev").text("The Dart language"),
    Hyperlink::new("https://pub.dev"),
    Hyperlink::new("file:///home/user/twk/README.md").text("the README"),
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
    .center(Text::new("example/widgets/main.dart")
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
