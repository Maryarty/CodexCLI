<!-- Written by `cargo xtask skill-references` from docs/widgets; do not edit. -->
# Overlays

Dialogs, toasts, tooltips, popovers, menus and the command palette, over the app.

[All widgets](../widgets.md)

A Code is the body of the gallery's preview: it names widgets by the last segment of the paths under API (`use` them) and may use `cx` or values of the gallery's page.

## Overlay

A layer over the app that dialogs, toasts, menus and tooltips go in; an entry of its own goes in it too.

- `rtwk::overlay::Overlay` (struct) in `crates/rtwk-overlay-base/src/layer.rs:67`: Shows `child`, and over it the widgets its entries build, each entry over those inserted below it (OverlayState::insert); the widgets below find the nearest one with Overlay::of and put their dialogs (`show_dialog`), toasts (show_toast) and tooltips (`Tooltip`) in it.
- `rtwk::overlay::OverlayEntry` (struct) in `crates/rtwk-overlay-base/src/layer.rs:582`: A layer of an Overlay: the widget its builder builds, shown while the entry is inserted (OverlayState::insert) until remove.

### Code

```rust
// In State::update for Insert:
let index = i32::try_from(self.entries.len()).expect("at most three entries") + 1;
let entry = OverlayEntry::new(move |cx| {
    let theme = Theme::of(cx);
    let panel = Panel::new(Text::rich([
        Span::new(format!("Entry {index}")).bold(),
        Span::styled(" of the overlay", theme.muted.clone()),
    ]))
    .look(PanelLook::Raised)
    .padding(EdgeInsets::x(1))
    .shadow(true);
    Positioned::new(panel).right(4 * index - 3).top(2 * index - 1).into()
});
Overlay::of(cx).insert(&entry);
self.entries.push(entry);

// On Remove, remove only the last entry. On dispose, remove all.
if let Some(entry) = self.entries.pop() {
    entry.remove();
}
let theme = Theme::of(cx);
let len = self.entries.len();
let mut add = Button::new("Insert an entry");
if len < 3 { add = add.on_pressed(cx.send(EntryMsg::Insert)); }
let mut remove = Button::new("Remove one");
if len != 0 { remove = remove.on_pressed(cx.send(EntryMsg::Remove)); }
column![
    row![add, remove].main_axis_size(MainAxisSize::Min).spacing(2),
    Text::rich([
        Span::styled("entries: ", theme.muted.clone()),
        Span::new(len.to_string()),
        Span::styled(" of the overlay", theme.muted.clone()),
    ]),
]
.main_axis_size(MainAxisSize::Min)
.cross_axis_alignment(CrossAxisAlignment::Start)
.spacing(1)
```

### Options

| option | values | starts at |
| --- | --- | --- |
| positioned | topRight, topLeft, bottomLeft | topRight |
| shadow | false, true | true |
| look | raised, bordered | raised |

## App

The root of an app: its theme over the Overlay its dialogs show in, over the app and its own keys.

- `rtwk::overlay::App` (struct) in `crates/rtwk-overlay/src/app.rs:39`: `child` under theme and the Overlay that its dialogs, toasts, menus and tooltips show in, which a run loop leaves to the app to place.

### Code

```rust
// The nested App owns the overlay within this 32 × 9 box.
SizedBox::new(
    Some(32),
    Some(9),
    Some(App::new(AppAsk { fade_in }).theme(ThemeData::NORD).into()),
)

// AppAsk::build, below the nested App:
Center::new(Button::new("Ask").on_pressed(cx.send(())))

// AppAskState::update: nothing waits for the result
let _result = show_dialog::<(), _>(
    cx,
    |cx| Dialog::new(
        Button::new("OK")
            .autofocus(true)
            .on_pressed(close_dialog_handler::<_, ()>(cx, None)),
    )
    .title("In the theme")
    .into(),
    DialogOptions { fade_in: w.fade_in, ..DialogOptions::default() },
);
```

### Options

| option | values | starts at |
| --- | --- | --- |
| theme | nord, light, dark | nord |

## Dialog

A box over the app, the rest dimmed behind a barrier; Tab stays in it, Escape closes it.

- `rtwk::overlay::Dialog` (struct) in `crates/rtwk-overlay/src/dialog.rs:751`: `child` in a box that stands out of what is around it: a raised panel (look) of the theme's surface and text, a cell of room around padding and no outline, title in bold on the first row inside, over the child; and shadow cast beside it.

### Code

```rust
// AskMsg::Open in the preview:
let dialog = show_dialog::<bool, _>(
    cx,
    move |_| DeletePrompt { shadow: false, look: PanelLook::Raised }.into(),
    DialogOptions {
        alignment: Alignment::CENTER,
        dismissible: true,
        ..DialogOptions::default()
    },
);
cx.spawn(async move { Some(AskMsg::Answer(dialog.await)) });

// AskMsg::Answer stores one of these labels and increments its serial.
let answer = match answer {
    Some(true) => "deleted",
    Some(false) => "kept",
    None => "dismissed",
};

// DeletePrompt::build and its bool event:
Dialog::new(
    column![
        Text::new("Delete 3 files?"),
        row![
            Button::new("Delete").autofocus(true).on_pressed(cx.send(true)),
            Button::new("Keep").on_pressed(cx.send(false)),
        ].main_axis_size(MainAxisSize::Min).spacing(2),
    ].main_axis_size(MainAxisSize::Min).cross_axis_alignment(CrossAxisAlignment::Start).spacing(1),
)
.title("Delete")
.look(PanelLook::Raised)

// On Delete or Keep:
close_dialog(cx, Some(delete));

// The preview:
column![
    Button::new("Ask").on_pressed(cx.send(AskMsg::Open)),
    LitLine::new(
        vec![
            ("answer: ".into(), true),
            (self.answer.unwrap_or("nothing").into(), self.answer.is_none()),
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
| alignment | center, topCenter, bottomRight | center |
| shadow | false, true | false |
| look | raised, bordered | raised |
| dismissible | false, true | true |

## Toast

A message that shows for a while and goes, over a corner of the app.

- `rtwk::overlay::show_toast` (fn) in `crates/rtwk-overlay-base/src/toast.rs:111`: Shows `message` in a toast of the Overlay above `cx` for the options' duration, three seconds by default, then takes it away.
- `rtwk::overlay::ToastOptions` (struct) in `crates/rtwk-overlay-base/src/toast.rs:41`: How show_toast shows a toast: what twk's named parameters say.

### Code

```rust
// In State::update:
let theme = Theme::of(cx);
let (message, style) = [
    ("Saved 3 files.", &theme.success),
    ("Copied the path.", &theme.info),
    ("Disk nearly full.", &theme.warning),
    ("Upload failed.", &theme.error),
][self.0];
self.0 = (self.0 + 1) % 4;
show_toast(cx, message, ToastOptions {
    alignment: Alignment::BOTTOM_RIGHT,
    duration: Duration::from_millis(3000),
    style: Some(style.clone()),
    look: PanelLook::Raised,
    ..ToastOptions::default()
});

// In State::build:
Button::new("Show a toast").on_pressed(cx.send(()))
```

### Options

| option | values | starts at |
| --- | --- | --- |
| alignment | bottomRight, topRight, bottomCenter | bottomRight |
| duration | 1 s, 3 s, 5 s | 3 s |
| look | raised, bordered | raised |

## Tooltip

A message by a widget while the mouse or the keyboard focus rests on it, or while a TooltipController says, centred under it by an AnchorFollower in the overlay; above it where there is no room below.

- `rtwk::overlay::Tooltip` (struct) in `crates/rtwk-overlay/src/tooltip.rs:197`: Shows message by child, in an entry of the Overlay above, centred under it (above it where there is no room under it), once the mouse has rested on the child for wait (half a second), or the focus has been in it as long; it goes when the mouse leaves and the focus goes, or on Escape while the focus is in the child.

### Code

```rust
// Keep TooltipController::new() for the Share button.
let row = Row::new().main_axis_size(MainAxisSize::Min).spacing(2)
    .children([("Save", "Saves the file", "Saved"), ("Open", "Opens a file", "Opened"), ("Share", "Copies a link to it", "Link copied")].into_iter().map(|(label, message, done)| {
        let mut tip = Tooltip::new(
            message,
            Button::new(label).on_pressed(cx.send(TipMsg::Toast(done))),
        )
        .wait(Duration::from_millis(500))
        .show_on_hover(true)
        .show_on_focus(true)
        .look(PanelLook::Raised);
        if label == "Share" { tip = tip.controller(&self.share); }
        tip.into()
    }));
let share = self.share.clone();
let toggle = cx.send(TipMsg::Toggle);
let show = ListenableBuilder::new(share.listenable(), move |_, _| {
    Button::new(if share.is_shown() { "Hide the tip" } else { "Show the tip" })
        .on_pressed(toggle.clone())
        .into()
});
column![row, show]
    .main_axis_size(MainAxisSize::Min)
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .spacing(1)

// Toggle uses the controller; the other buttons show a toast.
match msg {
    TipMsg::Toggle if self.share.is_shown() => self.share.hide(),
    TipMsg::Toggle => self.share.show(),
    TipMsg::Toast(message) => {
        let style = Theme::of(cx).success.clone();
        show_toast(cx, message, ToastOptions {
            style: Some(style),
            ..ToastOptions::default()
        });
    }
}
```

### Options

| option | values | starts at |
| --- | --- | --- |
| wait | 100 ms, 500 ms, 1 s | 500 ms |
| on hover | false, true | true |
| on focus | false, true | true |
| look | raised, bordered | raised |

## Popover

A widget by another while the app says so, placed by an AnchorTarget around it and an AnchorFollower in the overlay; Escape or a click outside closes it.

- `rtwk::overlay::Popover` (struct) in `crates/rtwk-overlay-base/src/popover.rs:75`: Shows popover by child while open, in an entry of the Overlay above: under the child from its left edge; where it does not fit there, on the side with more room, above the child, ending at its right edge; moved in from the edges of the overlay as far as it has to.

### Code

```rust
// The Toggle message flips self.open; Dismiss sets it to false.
let theme = Theme::of(cx);
let panel = Panel::new(column![
    Text::new("The popover, by the button."),
    Text::new("Escape or a click outside closes it.").style(theme.muted.clone()),
]
.main_axis_size(MainAxisSize::Min)
.cross_axis_alignment(CrossAxisAlignment::Start))
.look(PanelLook::Raised)
.padding(EdgeInsets::x(1));
Popover::new(
    self.open,
    panel,
    Button::new(if self.open { "Close" } else { "Open" })
        .on_pressed(cx.send(PopMsg::Toggle)),
    Some(cx.send(PopMsg::Dismiss)),
)
```

### Options

| option | values | starts at |
| --- | --- | --- |
| dismissible | false, true | true |
| look | raised, bordered | raised |

## MenuBar

Menus in a bar: F10 or a click opens one, Alt and a mnemonic too, or a MenuController from the app; the arrows, Enter and the mnemonics choose.

- `rtwk::overlay::MenuBar` (struct) in `crates/rtwk-overlay/src/menu/bar.rs:119`: A row of the titles of menus over child, which fills the rest of the room it gets (along an axis with no bound, it is as long as it likes): the menu bar of an app.

### Code

```rust
// Keep MenuController::new() in state.
let select = |label, action| MenuItem::new(label)
    .on_selected(cx.send(BarMsg::Chosen(action)));
let file = Menu::new("&File", [
    select("&New", "New").into(),
    select("&Open…", "Open").into(),
    Menu::new("Open &recent",
        ["main.dart", "page.dart", "overlay.dart"].map(|file| select(file, file).into())
    ).into(),
    MenuDivider.into(),
    select("&Save", "Save").into(),
    MenuItem::new("&Print").into(),
]);
let edit = Menu::new("&Edit", [
    select("&Undo", "Undo").into(),
    MenuItem::new("&Redo").into(),
    MenuDivider.into(),
    select("Cu&t", "Cut").into(),
    select("&Copy", "Copy").into(),
    select("&Paste", "Paste").into(),
]);
let view = Menu::new("&View", [
    MenuItem::new("&Wrap lines").checked(self.wrap)
        .on_selected(cx.send(BarMsg::ToggleWrap)).into(),
    MenuItem::new("Line &numbers").checked(self.numbers)
        .on_selected(cx.send(BarMsg::ToggleNumbers)).into(),
]);
let body = Padding::new(EdgeInsets::all(1), column![
    line("chosen: ", self.chosen, self.serial),
    row![
        Checkbox::new(self.wrap).label("wrap lines").on_changed(cx.map(BarMsg::SetWrap)),
        Button::new("Recent files").on_pressed(cx.send(BarMsg::Recent)),
    ].spacing(2),
].cross_axis_alignment(CrossAxisAlignment::Start).spacing(1));
let body = KeyBindings::new([
    ("ctrl+o", cx.send(BarMsg::Chosen("Open"))),
    ("ctrl+s", cx.send(BarMsg::Chosen("Save"))),
    ("ctrl+z", cx.send(BarMsg::Chosen("Undo"))),
], body);
let menu = MenuBar::new([file, edit, view], body)
    .look(PanelLook::Raised)
    .controller(&self.menu);
SizedBox::new(Some(48), Some(8), Some(menu.into()))

// In State::update, Recent opens File → Open recent:
let _ = self.menu.open(&[0, 2], None);
```

### Options

| option | values | starts at |
| --- | --- | --- |
| keys bound | false, true | true |
| look | raised, bordered | raised |

## ContextMenu

A menu at the mouse on a right click, under the focus on Shift+F10, or where a MenuController opens it.

- `rtwk::overlay::ContextMenu` (struct) in `crates/rtwk-overlay/src/menu/context.rs:232`: child, with a menu of items that a right click on it opens at the mouse, and Shift+F10 under the focused widget while the focus is in it (show_context_menu); in a layer of the focus tree around it, whose Shift+F10 a key hints bar lists, `shift+f10 menu`.

### Code

```rust
let theme = Theme::of(cx);
let select = |label, action| MenuItem::new(label)
    .on_selected(cx.send(ContextMsg::Choose(action)));
let paste = if self.copied { select("&Paste", "Paste") } else { MenuItem::new("&Paste") };
let items = [
    select("Cu&t", "Cut").into(),
    select("&Copy", "Copy").into(),
    paste.into(),
    MenuDivider.into(),
    select("Select &all", "Select all").into(),
];
let panel = BoxWidget::new()
    .border(Border::all(theme.border.clone()).kind(BorderType::Rounded))
    .padding(EdgeInsets::x(1))
    .child(column![
        Text::new("Right click in here."),
        row![
            Button::new("Or focus me").on_pressed(cx.send(ContextMsg::Focus)),
            Button::new("Open the menu").on_pressed(cx.send(ContextMsg::Open)),
        ].spacing(2),
        line("chosen: ", self.chosen, self.serial),
    ].cross_axis_alignment(CrossAxisAlignment::Start));
ContextMenu::new(items, SizedBox::new(Some(40), Some(5), Some(panel.into())))
    .controller(&self.menu)
    .look(PanelLook::Raised)

// In State::update, Cut or Copy enables Paste:
self.copied |= matches!(action, "Cut" | "Copy");
// Open positions the menu within the panel:
let _ = self.menu.open(&[], Some((22, 1)));
```

### Options

| option | values | starts at |
| --- | --- | --- |
| look | raised, bordered | raised |

## CommandPalette

What can be done, found by typing: Tab to a button, then Ctrl+K, or press Dart files, whose CommandPaletteController opens it with dart typed; commandsFor adds the files that match. The gallery's own, Ctrl+P, goes to a page.

- `rtwk::overlay::CommandPalette` (struct) in `crates/rtwk-overlay/src/command_palette.rs:451`: A palette of commands over `child`, opened by a key (Ctrl+P): a list over the app, in an entry of the Overlay above, of the commands that can be done from the node that had the focus, then the keys hinted there (each key listed under its hint), each with the key that does the same at its right.

### Code

```rust
let body = KeyBindings::new([
    ("ctrl+b", cx.send(PaletteMsg::Run("Build"))),
    ("f8", cx.send(PaletteMsg::Run("Run tests"))),
    ("ctrl+s", cx.send(PaletteMsg::Run("Save"))),
], column![
    row![
        Button::new("Focus me").on_pressed(cx.send(PaletteMsg::Run("press"))),
        Button::new("Dart files").on_pressed(cx.send(PaletteMsg::Open)),
    ].main_axis_size(MainAxisSize::Min).spacing(2),
    line("ran: ", self.ran.as_deref(), self.serial),
].main_axis_size(MainAxisSize::Min).cross_axis_alignment(CrossAxisAlignment::Start).spacing(1))
.hints([("ctrl+s", "Save")]);
let palette = CommandPalette::new(body)
    .controller(&self.palette)
    .open_key("ctrl+k")
    .width(40).rows(10)
    .look(PanelLook::Raised)
    .rule(false).bar(false).thumb(false)
    .commands(["Build", "Run tests", "Format", "Deploy"].map(|label| {
        Command::new(label).on_selected(cx.send(PaletteMsg::Run(label)))
    }));

let files = ["lib/main.dart", "lib/src/app.dart", "lib/src/theme.dart", "test/app_test.dart", "README.md", "pubspec.yaml"].map(|file| {
    Command::new(file).on_selected(cx.send(PaletteMsg::File(file)))
});
let palette = palette.commands_for(move |query| {
    CommandsAnswer::Now(files.iter().filter(|item| {
        !query.is_empty() && fuzzy_match(query, &item.label).is_some()
    }).cloned().collect())
});

palette

// In State::update, Open uses a prefilled query:
let _ = self.palette.open("dart");
```

### Options

| option | values | starts at |
| --- | --- | --- |
| width | 40, 60 | 40 |
| rows | 3, 10 | 10 |
| files | false, true | true |
| rule | false, true | false |
| bar | false, true | false |
| thumb | false, true | false |
| placeholder | false, true | false |
| look | raised, bordered | raised |

## OptionList

Options in rows, one highlighted, the characters a query matched standing out and a detail at the right edge: what an autocomplete, a Select and a command palette list.

- `rtwk::overlay::OptionList` (struct) in `crates/rtwk-overlay-base/src/options.rs:143`: options one a row in the theme's surface and text, a column of room either side, each label cut with the theme's ellipsis where the row is too narrow; the highlighted one in the theme's selection, with a bar of the theme's cursor mark at its left (bar); at most rows rows, scrolled to keep the highlighted one in view, a slim thumb at the right edge saying where (thumb).

### Code

```rust
let commands = [("Open file", "ctrl+o"), ("Save", "ctrl+s"), ("Save as…", "ctrl+shift+s"), ("Close tab", "ctrl+w"), ("Find", "ctrl+f"), ("Replace", "ctrl+h"), ("Go to line", "ctrl+g"), ("Toggle theme", "ctrl+t"), ("Quit", "ctrl+q")];
let items: Vec<_> = commands.into_iter().filter_map(|(label, detail)| {
    let matched = fuzzy_match("", label)?;
    let option = OptionItem::new(label).marks(matched.positions);
    let option = option.detail(detail);
    Some(option)
}).collect();

let highlighted = self.highlighted.clamp(0, i32::try_from(items.len()).unwrap() - 1);
let list = OptionList::new(items)
    .highlighted(highlighted)
    .rows(5).bar(true).thumb(true)
    .on_highlight(cx.map(OptionsMsg::Highlight))
    .on_selected(cx.map(OptionsMsg::Run));
let pressable = Pressable::new(move |cx, focused| {
    let theme = Theme::of(cx);
    let style = if focused { &theme.primary } else { &theme.border };
    BoxWidget::new()
        .border(Border::all(style.clone()).kind(BorderType::Rounded))
        .child(list.clone())
        .into()
}).keys(
    [
        ("up", Some(cx.send(OptionsMsg::Move(-1)))),
        ("down", Some(cx.send(OptionsMsg::Move(1)))),
        ("enter", Some(cx.send(OptionsMsg::Run(highlighted)))),
    ],
    [("up", "select"), ("down", "select"), ("enter", "run")],
);
// The update stores the highlight, clamps keyboard moves and records the selected label.
column![
    pressable,
    LitLine::new(
        vec![
            ("ran: ".into(), true),
            (self.ran.as_deref().unwrap_or("nothing").into(), self.ran.is_none()),
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
| marks of | none, o, sa | none |
| rows | 3, 5, 8 | 5 |
| details | false, true | true |
| bar | false, true | true |
| thumb | false, true | true |

## ModalBarrier

What dims the app under a dialog and keeps the mouse from it.

- `rtwk::overlay::ModalBarrier` (struct) in `crates/rtwk-overlay/src/barrier.rs:72`: Covers the room it gets and dims what is under it: each cell keeps its glyph and its attributes, its background, its text color and the color of its underline taken opacity of the way to color, channel by channel and rounded once (`a + (color - a) * opacity`; the terminal's default colors where a cell has them); where they are none, the theme's shade and its alpha (ThemeData::shade, ThemeData::shade_alpha: black at half on a dark theme, at 28 % on a light one), not rounded to 255ths on the way.

### Code

```rust
let theme = Theme::of(cx);
let body = column![
    Text::new("The text under the barrier, dimmed by it: the mouse does not reach it."),
    Button::new("Press me").on_pressed(cx.send(BarrierMsg::Press)),
    Text::rich([
        Span::styled("pressed ", theme.muted.clone()),
        Span::new(self.pressed.to_string()),
        Span::styled(", the barrier ", theme.muted.clone()),
        Span::new(self.dismissed.to_string()),
    ]),
].cross_axis_alignment(CrossAxisAlignment::Start).spacing(1);
let stack = Stack::new().fit(StackFit::Expand).child(body);
let stack = stack.child(ModalBarrier::new()
    .on_dismiss(cx.send(BarrierMsg::Dismiss)));
let front = Center::new(Panel::new(Text::new("over it"))
    .padding(EdgeInsets::x(1))
    .shadow(false));
SizedBox::new(Some(40), Some(8), Some(stack.child(front).into()))
```

### Options

| option | values | starts at |
| --- | --- | --- |
| barrier | false, true | true |
| alpha | ¼, ½, ¾ | ½ |
| fade | false, true | false |

## WindowLayer

Windows over the app: a title bar drags one, an edge resizes it, its buttons minimise, maximise and close it; Tab to a title bar, then the arrows move it.

- `rtwk::overlay::WindowLayer` (struct) in `crates/rtwk-overlay/src/window/mod.rs:569`: Shows windows over child, each in a frame (frame) with its title on top, the title bar; laid out as `child` is, as big as it is.

### Code

```rust
// Keep the two controllers in state, even while a window is closed.
let notes = WindowController::new().placed(2, 3, 30, 6);
let counter = WindowController::new().placed(24, 5, 26, 6);

// In State::build:
let theme = Theme::of(cx);
let window = |title: &'static str, index, controller, child: AnyWidget| {
    Window::new(title, Padding::new(EdgeInsets::x(1), child))
        .key(title)
        .controller(controller)
        .on_close(cx.send(WindowsMsg::Close(index)))
        .resizable(true)
        .minimisable(true)
        .maximisable(true)
        .shadow(Panel::shadow_of(&theme))
};
let mut windows = Vec::new();
if !self.closed[0] {
    windows.push(window("Notes", 0, self.notes.clone(), Text::new("Drag me by the title bar, or resize me by an edge.").into()));
}
if !self.closed[1] {
    windows.push(window("Counter", 1, self.counter.clone(), column![
        Text::new(format!("count: {}", self.count)),
        Button::new("Add one").on_pressed(cx.send(WindowsMsg::Count)),
    ].cross_axis_alignment(CrossAxisAlignment::Start).into()));
}
let mut reopen = Button::new("Open the windows closed");
if self.closed.iter().any(|closed| *closed) {
    reopen = reopen.on_pressed(cx.send(WindowsMsg::Reopen));
}
let layer = WindowLayer::new(windows, column![
    Text::new("The app, under its windows."),
    reopen,
].cross_axis_alignment(CrossAxisAlignment::Start)).frame(WindowFrame::Raised);
SizedBox::new(Some(50), Some(12), Some(layer.into()))

// Close marks a window closed, Count increments the counter,
// and Reopen clears the closed flags without resetting either controller.
```

### Options

| option | values | starts at |
| --- | --- | --- |
| frame | raised, bordered | raised |
| shadow | false, true | true |
| resizable | false, true | true |
| minimisable | false, true | true |
| maximisable | false, true | true |
