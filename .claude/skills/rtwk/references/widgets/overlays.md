<!-- Written by tools/rtwk-port/skill/generate.py from rtwk 9f28b7fb; do not edit. -->

# Overlays

[All widgets](../widgets.md)

## Overlay

A layer over the app that dialogs, toasts, menus and tooltips go in; an entry of its own goes in it too.

API:

- `rtwk::overlay::Overlay` (struct, `crates/rtwk-overlay-base/src/layer.rs:67`) Shows `child`, and over it the widgets its entries build, each entry over those inserted below it (`OverlayState::insert`); the widgets below find the nearest one with `Overlay::of` and put their dialogs ...
  - public fields: `child: AnyWidget`

### Code

The gallery's Code tab at the initial options (route `#/overlays/overlay`, `crates/rtwk-gallery/src/pages/mod.rs:1076`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

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

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| position | topRight, topLeft, bottomLeft | topRight |
| shadow | on, off | true |
| look | raised, bordered | raised |

### Methods

#### `Overlay`: constructors and associated functions

- `pub fn new(child: impl Into<AnyWidget>) -> Overlay` - Shows `child` under the entries inserted in it.
- `pub fn of<S>(cx: &Cx<'_, S>) -> OverlayState` - The state of the nearest `Overlay` above `cx`.
- `pub fn maybe_of<S>(cx: &Cx<'_, S>) -> Option<OverlayState>` - The state of the nearest `Overlay` above `cx`, if there is one.
- `pub fn navigator_host<S>(cx: &Cx<'_, S>, child: impl Into<AnyWidget>, link: Rc<dyn Fn(DialogRequest)>) -> AnyWidget` - What a navigator puts around its own overlay and what is above it (twk's `OverlayHost(navigator: true)`): the overlay right below is a navigator's, and the dialogs below it are pushed on it with `link`, or on the root ...

### Also used in the code

- `rtwk::overlay::OverlayEntry` (struct, `crates/rtwk-overlay-base/src/layer.rs:582`) A layer of an `Overlay`: the widget its builder builds, shown while the entry is inserted (`OverlayState::insert`) until `remove`.
- `rtwk::overlay::Panel` (struct, `crates/rtwk-overlay-base/src/panel.rs:47`) `child` on a panel that floats over what is under it: the theme's surface and text (and `style` over them), `padding` around the child and, as `look` says, a cell of room around that (`PanelLook::Raised`) or a border in ...
  - public fields: `child: AnyWidget`, `look: PanelLook`, `padding: EdgeInsets`, `shadow: bool`, `style: Option<Style>`
- `rtwk::overlay::PanelLook` (enum, `crates/rtwk-overlay-base/src/panel.rs:13`) How a floating panel stands out of what is under it.
  - variants: `Raised`, `Bordered`
- `rtwk::layout::Positioned` (struct, `crates/rtwk-widgets/src/layout/stack.rs:107`) Pins its child, in a `Stack`, to the stack's edges (twk's `Positioned`): along an axis with both edges set, exactly as long as the cells between them; with its size set, that long; otherwise as long as it wants.
  - public fields: `left: Option<i32>`, `top: Option<i32>`, `right: Option<i32>`, `bottom: Option<i32>`, `width: Option<i32>`, `height: Option<i32>`, `child: AnyWidget`
- `rtwk::controls::Button` (struct, `crates/rtwk-controls/src/button.rs:107`) A button that tells `on_pressed` when it is clicked, or when Enter or Space is pressed while it has the focus.
  - public fields: `spans: Option<Rc<[Span]>>`, `tall: bool`, `on_pressed: Option<Handler<()>>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: bool`, `style: Option<Style>`, `focused_style: Option<Style>`, `hover_style: Option<Style>`, `disabled_style: Option<Style>`

## App

The root of an app: its theme over the Overlay its dialogs show in, over the app and its own keys.

API:

- `rtwk::overlay::App` (struct, `crates/rtwk-overlay/src/app.rs:39`) `child` under `theme` and the `Overlay` that its dialogs, toasts, menus and tooltips show in, which a run loop leaves to the app to place.
  - public fields: `theme: Option<Rc<ThemeData>>`, `scroll_behavior: Option<ScrollBehavior>`, `child: AnyWidget`

### Code

The gallery's Code tab at the initial options (route `#/overlays/app`, `crates/rtwk-gallery/src/pages/mod.rs:1228`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

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

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| theme | nord, light, dark | nord |

### Methods

#### `App`: constructors and associated functions

- `pub fn new(child: impl Into<AnyWidget>) -> App` - `child` under an overlay, no theme of its own.

#### `App`: builder methods (take and return `self`)

- `pub fn theme(mut self, theme: impl Into<Rc<ThemeData>>) -> App` - Under `theme`.
- `pub fn scroll_behavior(mut self, behavior: ScrollBehavior) -> Self` - Give the app and its overlays a common wheel policy.

### Also used in the code

- `rtwk::layout::Center` (struct, `crates/rtwk-widgets/src/layout/basic.rs:191`) An `Align` that puts its child in the middle, the odd cell after it (twk's `Center`).
  - public fields: `width_factor: Option<i32>`, `height_factor: Option<i32>`, `child: Option<AnyWidget>`
- `rtwk::controls::Button` (struct, `crates/rtwk-controls/src/button.rs:107`) A button that tells `on_pressed` when it is clicked, or when Enter or Space is pressed while it has the focus.
  - public fields: `spans: Option<Rc<[Span]>>`, `tall: bool`, `on_pressed: Option<Handler<()>>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: bool`, `style: Option<Style>`, `focused_style: Option<Style>`, `hover_style: Option<Style>`, `disabled_style: Option<Style>`
- `rtwk::overlay::Dialog` (struct, `crates/rtwk-overlay/src/dialog.rs:751`) `child` in a box that stands out of what is around it: a raised panel (`look`) of the theme's surface and text, a cell of room around `padding` and no outline, `title` in bold on the first row inside, over the child; ...
  - public fields: `child: AnyWidget`, `title: Option<Str>`, `trailing: Option<AnyWidget>`, `scrollable: bool`, `padding: EdgeInsets`, `shadow: Option<BoxShadow>`, `look: PanelLook`, `max_width: Option<i32>`
- `rtwk::overlay::DialogOptions` (struct, `crates/rtwk-overlay/src/dialog.rs:49`) How `show_dialog` shows a dialog: what twk's named parameters say.
  - public fields: `barrier_dismissible: bool`, `alignment: Alignment`, `barrier_color: Option<Color>`, `barrier_alpha: Option<i32>`, `fade_in: Duration`, `use_root_navigator: bool`

## Dialog

A box over the app, the rest dimmed behind a barrier; Tab stays in it, Escape closes it.

API:

- `rtwk::overlay::Dialog` (struct, `crates/rtwk-overlay/src/dialog.rs:751`) `child` in a box that stands out of what is around it: a raised panel (`look`) of the theme's surface and text, a cell of room around `padding` and no outline, `title` in bold on the first row inside, over the child; ...
  - public fields: `child: AnyWidget`, `title: Option<Str>`, `trailing: Option<AnyWidget>`, `scrollable: bool`, `padding: EdgeInsets`, `shadow: Option<BoxShadow>`, `look: PanelLook`, `max_width: Option<i32>`
- `rtwk::overlay::DialogOptions` (struct, `crates/rtwk-overlay/src/dialog.rs:49`) How `show_dialog` shows a dialog: what twk's named parameters say.
  - public fields: `barrier_dismissible: bool`, `alignment: Alignment`, `barrier_color: Option<Color>`, `barrier_alpha: Option<i32>`, `fade_in: Duration`, `use_root_navigator: bool`

### Code

The gallery's Code tab at the initial options (route `#/overlays/dialog`, `crates/rtwk-gallery/src/pages/mod.rs:1105`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

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

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| alignment | center, topCenter, bottomRight | center |
| shadow | on, off | false |
| look | raised, bordered | raised |
| dismissible | on, off | true |

### Methods

#### `Dialog`: constructors and associated functions

- `pub fn new(child: impl Into<AnyWidget>) -> Dialog` - A box around `child`, a raised panel by default.

#### `Dialog`: builder methods (take and return `self`)

- `pub fn title(mut self, title: impl Into<Str>) -> Dialog` - Titled `title`.
- `pub fn trailing(mut self, w: impl Into<AnyWidget>) -> Dialog` - With `w` at the right end of the title row.
- `pub fn scrollable(mut self, on: bool) -> Dialog` - Whether the child scrolls.
- `pub fn padding(mut self, p: EdgeInsets) -> Dialog` - With `p` around the child.
- `pub fn shadow(mut self, s: BoxShadow) -> Dialog` - Casting `s`.
- `pub fn look(mut self, look: PanelLook) -> Dialog` - Raised or bordered.
- `pub fn max_width(mut self, w: impl Into<Option<i32>>) -> Dialog` - At most `w` columns; none for no limit.

### Also used in the code

- `rtwk::overlay::PanelLook` (enum, `crates/rtwk-overlay-base/src/panel.rs:13`) How a floating panel stands out of what is under it.
  - variants: `Raised`, `Bordered`
- `rtwk::layout::Alignment` (struct, `crates/rtwk-render/src/objects/align.rs:32`) Where a child goes in the cells its parent leaves around it, along each axis: at the start (-1), in the middle (0) or at the end (1).
  - public fields: `x: i32`, `y: i32`
- `rtwk::controls::Button` (struct, `crates/rtwk-controls/src/button.rs:107`) A button that tells `on_pressed` when it is clicked, or when Enter or Space is pressed while it has the focus.
  - public fields: `spans: Option<Rc<[Span]>>`, `tall: bool`, `on_pressed: Option<Handler<()>>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: bool`, `style: Option<Style>`, `focused_style: Option<Style>`, `hover_style: Option<Style>`, `disabled_style: Option<Style>`

## Toast

A message that shows for a while and goes, over a corner of the app.

API:

- `rtwk::overlay::ToastOptions` [path inferred] (struct, `crates/rtwk-overlay-base/src/toast.rs:41`) How `show_toast` shows a toast: what twk's named parameters say.
  - public fields: `duration: Duration`, `alignment: Alignment`, `style: Option<Style>`, `icon: Option<Str>`, `look: PanelLook`

### Code

The gallery's Code tab at the initial options (route `#/overlays/toast`, `crates/rtwk-gallery/src/pages/mod.rs:1141`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

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

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| alignment | bottomRight, topRight, bottomCenter | bottomRight |
| duration | 1000, 3000, 5000 | 3000 |
| look | raised, bordered | raised |

### Also used in the code

- `rtwk::layout::Alignment` (struct, `crates/rtwk-render/src/objects/align.rs:32`) Where a child goes in the cells its parent leaves around it, along each axis: at the start (-1), in the middle (0) or at the end (1).
  - public fields: `x: i32`, `y: i32`
- `rtwk::overlay::PanelLook` (enum, `crates/rtwk-overlay-base/src/panel.rs:13`) How a floating panel stands out of what is under it.
  - variants: `Raised`, `Bordered`
- `rtwk::controls::Button` (struct, `crates/rtwk-controls/src/button.rs:107`) A button that tells `on_pressed` when it is clicked, or when Enter or Space is pressed while it has the focus.
  - public fields: `spans: Option<Rc<[Span]>>`, `tall: bool`, `on_pressed: Option<Handler<()>>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: bool`, `style: Option<Style>`, `focused_style: Option<Style>`, `hover_style: Option<Style>`, `disabled_style: Option<Style>`

## Tooltip

A message by a widget while the mouse or the keyboard focus rests on it, or while a TooltipController says, centred under it by an AnchorFollower in the overlay; above it where there is no room below.

API:

- `rtwk::overlay::Tooltip` (struct, `crates/rtwk-overlay/src/tooltip.rs:197`) Shows `message` by `child`, in an entry of the `Overlay` above, centred under it (above it where there is no room under it), once the mouse has rested on the child for `wait` (half a second), or the focus has been in it ...
  - public fields: `message: Str`, `child: AnyWidget`, `show_on_hover: bool`, `show_on_focus: bool`, `controller: Option<TooltipController>`, `wait: Duration`, `look: PanelLook`, `fade_in: Duration`

### Code

The gallery's Code tab at the initial options (route `#/overlays/tooltip`, `crates/rtwk-gallery/src/pages/mod.rs:1170`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

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

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| wait | 100, 500, 1000 | 500 |
| hover | on, off | true |
| focus | on, off | true |
| look | raised, bordered | raised |

### Methods

#### `Tooltip`: constructors and associated functions

- `pub fn new(message: impl Into<Str>, child: impl Into<AnyWidget>) -> Tooltip` - `message` by `child`.
- `pub fn chip_style_of(theme: &ThemeData) -> Style` - The style of a tooltip's chip in `theme`: on a dark theme its text on its surface taken a fifth of the way to the text's color, on a light one its background on its text taken 8 % of the way to the background; the ...

#### `Tooltip`: builder methods (take and return `self`)

- `pub fn show_on_hover(mut self, on: bool) -> Tooltip` - Whether the mouse shows it.
- `pub fn show_on_focus(mut self, on: bool) -> Tooltip` - Whether the focus shows it.
- `pub fn controller(mut self, c: impl Into<TooltipController>) -> Tooltip` - Shown and hidden by `c` too.
- `pub fn wait(mut self, d: Duration) -> Tooltip` - Waiting `d` before it shows.
- `pub fn look(mut self, look: PanelLook) -> Tooltip` - Drawn as `look` says.
- `pub fn fade_in(mut self, d: Duration) -> Tooltip` - Fading in over `d`.

### Also used in the code

- `rtwk::overlay::TooltipController` (struct, `crates/rtwk-overlay/src/tooltip.rs:45`) Shows a `Tooltip`'s message from the app, and says whether it shows (twk's `TooltipController`): `show` pins it until `hide` (or Escape); it notifies its listeners as the message comes and goes.
- `rtwk::controls::Button` (struct, `crates/rtwk-controls/src/button.rs:107`) A button that tells `on_pressed` when it is clicked, or when Enter or Space is pressed while it has the focus.
  - public fields: `spans: Option<Rc<[Span]>>`, `tall: bool`, `on_pressed: Option<Handler<()>>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: bool`, `style: Option<Style>`, `focused_style: Option<Style>`, `hover_style: Option<Style>`, `disabled_style: Option<Style>`
- `rtwk::overlay::PanelLook` (enum, `crates/rtwk-overlay-base/src/panel.rs:13`) How a floating panel stands out of what is under it.
  - variants: `Raised`, `Bordered`
- `rtwk::foundation::ListenableBuilder` (struct, `crates/rtwk-framework/src/foundation/listenable_builder.rs:131`) Shows what its builder returns, built again whenever its listenable notifies; the widgets around it are not (twk's `ListenableBuilder`).
  - public fields: `listenable: Rc<dyn Listenable>`, `builder: TransitionBuilder`, `child: Option<AnyWidget>`

## Popover

A widget by another while the app says so, placed by an AnchorTarget around it and an AnchorFollower in the overlay; Escape or a click outside closes it.

API:

- `rtwk::overlay::Popover` (struct, `crates/rtwk-overlay-base/src/popover.rs:75`) Shows `popover` by `child` while `open`, in an entry of the `Overlay` above: under the child from its left edge; where it does not fit there, on the side with more room, above the child, ending at its right edge; moved ...
  - public fields: `open: bool`, `popover: AnyWidget`, `child: AnyWidget`, `on_dismiss: Option<Handler<()>>`, `fade_in: Duration`, `fade_out: Duration`, `match_anchor_width: bool`

### Code

The gallery's Code tab at the initial options (route `#/overlays/popover`, `crates/rtwk-gallery/src/pages/mod.rs:1206`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

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

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| dismissible | on, off | true |
| look | raised, bordered | raised |

### Methods

#### `Popover`: constructors and associated functions

- `pub fn new(open: bool, popover: impl Into<AnyWidget>, child: impl Into<AnyWidget>, on_dismiss: Option<Handler<()>>) -> Popover` - Shows `popover` by `child` while `open`, calling `on_dismiss` when it is to close; `None` for one the app never closes that way (twk's required `onDismiss`, null stated rather than left out).

#### `Popover`: builder methods (take and return `self`)

- `pub fn fade_in(mut self, d: Duration) -> Popover` - Fading in over `d`.
- `pub fn fade_out(mut self, d: Duration) -> Popover` - Fading out over `d`.
- `pub fn match_anchor_width(mut self, on: bool) -> Popover` - At least as wide as the child.

### Also used in the code

- `rtwk::overlay::Panel` (struct, `crates/rtwk-overlay-base/src/panel.rs:47`) `child` on a panel that floats over what is under it: the theme's surface and text (and `style` over them), `padding` around the child and, as `look` says, a cell of room around that (`PanelLook::Raised`) or a border in ...
  - public fields: `child: AnyWidget`, `look: PanelLook`, `padding: EdgeInsets`, `shadow: bool`, `style: Option<Style>`
- `rtwk::overlay::PanelLook` (enum, `crates/rtwk-overlay-base/src/panel.rs:13`) How a floating panel stands out of what is under it.
  - variants: `Raised`, `Bordered`
- `rtwk::controls::Button` (struct, `crates/rtwk-controls/src/button.rs:107`) A button that tells `on_pressed` when it is clicked, or when Enter or Space is pressed while it has the focus.
  - public fields: `spans: Option<Rc<[Span]>>`, `tall: bool`, `on_pressed: Option<Handler<()>>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: bool`, `style: Option<Style>`, `focused_style: Option<Style>`, `hover_style: Option<Style>`, `disabled_style: Option<Style>`

## MenuBar

Menus in a bar: F10 or a click opens one, Alt and a mnemonic too, or a MenuController from the app; the arrows, Enter and the mnemonics choose.

API:

- `rtwk::overlay::MenuBar` (struct, `crates/rtwk-overlay/src/menu/bar.rs:119`) A row of the titles of `menus` over `child`, which fills the rest of the room it gets (along an axis with no bound, it is as long as it likes): the menu bar of an app.
  - public fields: `menus: Rc<[Menu]>`, `child: AnyWidget`, `key_label: Option<KeyLabel>`, `style: Option<Style>`, `open_style: Option<Style>`, `look: PanelLook`, `controller: Option<MenuController>`

### Code

The gallery's Code tab at the initial options (route `#/overlays/menubar`, `crates/rtwk-gallery/src/pages/mod.rs:1241`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

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

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| keys | on, off | true |
| look | raised, bordered | raised |

### Methods

#### `MenuBar`: constructors and associated functions

- `pub fn new(menus: impl Into<Rc<[Menu]>>, child: impl Into<AnyWidget>) -> MenuBar` - A bar of the titles of `menus` over `child`.

#### `MenuBar`: builder methods (take and return `self`)

- `pub fn key_label(mut self, f: impl Fn(&KeyCombo) -> String + 'static) -> MenuBar` - Naming keys with `f`.
- `pub fn style(mut self, style: Style) -> MenuBar` - With the bar in `style` over the theme's.
- `pub fn open_style(mut self, style: Style) -> MenuBar` - With the title of the menu open in `style`.
- `pub fn look(mut self, look: PanelLook) -> MenuBar` - The menus raised or bordered.
- `pub fn controller(mut self, c: impl Into<MenuController>) -> MenuBar` - Opened and closed by `c` too.

### Also used in the code

- `rtwk::overlay::MenuController` (struct, `crates/rtwk-overlay/src/menu/mod.rs:481`) What opens and closes the menus of the `MenuBar` or the `ContextMenu` it is given to as its `controller`, and says which are open, however they opened: by it, the keys or the mouse.
- `rtwk::overlay::MenuItem` (struct, `crates/rtwk-overlay/src/menu/mod.rs:128`) An entry that does something when chosen, once its menus are closed: calls `on_selected`, or does `intent` as a key that `Shortcuts` binds to it would, from the node the keys went to first when the menu opened (the ...
  - public fields: `label: Str`, `on_selected: Option<Handler<()>>`, `intent: Option<AnyIntent>`, `checked: Option<bool>`, `enabled: bool`, `shortcut: Option<Str>`
- `rtwk::overlay::Menu` (struct, `crates/rtwk-overlay/src/menu/mod.rs:243`) An entry that opens a menu of `items` beside the menu it is in, its first entry level with this one: on its right, or, where it does not fit there, on the side with more room, and higher where it does not fit below, its ...
  - public fields: `label: Str`, `items: Rc<[MenuEntry]>`, `enabled: bool`
- `rtwk::layout::Padding` (struct, `crates/rtwk-widgets/src/layout/basic.rs:42`) Insets its child by `padding`: as big as the child and the padding.
  - public fields: `padding: EdgeInsets`, `child: Option<AnyWidget>`
- `rtwk::controls::Checkbox` (struct, `crates/rtwk-controls/src/toggle.rs:64`) A checkbox before `label`: `[x]` while `value` is true, `[ ]` while it is false.
  - public fields: `value: bool`, `on_changed: Option<Handler<bool>>`, `label: Str`, `semantics_label: Option<Str>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `style: Option<Style>`, `bracket_style: Option<Style>`, `focused_style: Option<Style>`, `disabled_style: Option<Style>`
- `rtwk::controls::Button` (struct, `crates/rtwk-controls/src/button.rs:107`) A button that tells `on_pressed` when it is clicked, or when Enter or Space is pressed while it has the focus.
  - public fields: `spans: Option<Rc<[Span]>>`, `tall: bool`, `on_pressed: Option<Handler<()>>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: bool`, `style: Option<Style>`, `focused_style: Option<Style>`, `hover_style: Option<Style>`, `disabled_style: Option<Style>`
- `rtwk::focus::KeyBindings` (struct, `crates/rtwk-framework/src/focus/shortcuts.rs:1257`) Binds keys to handlers for the nodes below it.
  - public fields: `bindings: Vec<(Str`, `combos: Vec<(KeyCombo`, `hints: Vec<(Str`, `combo_hints: Vec<(KeyCombo`, `sequence_timeout: Option<Duration>`, `unmatched: UnmatchedKeys`, `child: AnyWidget`, `debug_label: Option<Str>`
- `rtwk::overlay::PanelLook` (enum, `crates/rtwk-overlay-base/src/panel.rs:13`) How a floating panel stands out of what is under it.
  - variants: `Raised`, `Bordered`

## ContextMenu

A menu at the mouse on a right click, under the focus on Shift+F10, or where a MenuController opens it.

API:

- `rtwk::overlay::ContextMenu` (struct, `crates/rtwk-overlay/src/menu/context.rs:232`) `child`, with a menu of `items` that a right click on it opens at the mouse, and Shift+F10 under the focused widget while the focus is in it (`show_context_menu`); in a layer of the focus tree around it, whose Shift+F10 ...
  - public fields: `items: Rc<[MenuEntry]>`, `child: AnyWidget`, `key_label: Option<KeyLabel>`, `look: PanelLook`, `controller: Option<MenuController>`

### Code

The gallery's Code tab at the initial options (route `#/overlays/contextmenu`, `crates/rtwk-gallery/src/pages/mod.rs:1263`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

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

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| look | raised, bordered | raised |

### Methods

#### `ContextMenu`: constructors and associated functions

- `pub fn new(items: impl Into<Rc<[MenuEntry]>>, child: impl Into<AnyWidget>) -> ContextMenu` - `child`, with a menu of `items` that a right click or Shift+F10 opens.

#### `ContextMenu`: builder methods (take and return `self`)

- `pub fn key_label(mut self, f: impl Fn(&KeyCombo) -> String + 'static) -> ContextMenu` - Naming keys with `f`.
- `pub fn look(mut self, look: PanelLook) -> ContextMenu` - The menus raised or bordered.
- `pub fn controller(mut self, c: impl Into<MenuController>) -> ContextMenu` - Opened and closed by `c` too.

### Also used in the code

- `rtwk::overlay::MenuItem` (struct, `crates/rtwk-overlay/src/menu/mod.rs:128`) An entry that does something when chosen, once its menus are closed: calls `on_selected`, or does `intent` as a key that `Shortcuts` binds to it would, from the node the keys went to first when the menu opened (the ...
  - public fields: `label: Str`, `on_selected: Option<Handler<()>>`, `intent: Option<AnyIntent>`, `checked: Option<bool>`, `enabled: bool`, `shortcut: Option<Str>`
- `rtwk::controls::Button` (struct, `crates/rtwk-controls/src/button.rs:107`) A button that tells `on_pressed` when it is clicked, or when Enter or Space is pressed while it has the focus.
  - public fields: `spans: Option<Rc<[Span]>>`, `tall: bool`, `on_pressed: Option<Handler<()>>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: bool`, `style: Option<Style>`, `focused_style: Option<Style>`, `hover_style: Option<Style>`, `disabled_style: Option<Style>`
- `rtwk::overlay::PanelLook` (enum, `crates/rtwk-overlay-base/src/panel.rs:13`) How a floating panel stands out of what is under it.
  - variants: `Raised`, `Bordered`

## CommandPalette

What can be done, found by typing: Tab to a button, then Ctrl+K, or press Dart files, whose CommandPaletteController opens it with dart typed; commandsFor adds the files that match. The gallery's own, Ctrl+P, goes to a page.

API:

- `rtwk::overlay::CommandPalette` (struct, `crates/rtwk-overlay/src/command_palette.rs:451`) A palette of commands over `child`, opened by a key (Ctrl+P): a list over the app, in an entry of the `Overlay` above, of the `commands` that can be done from the node that had the focus, then the keys hinted there ...
  - public fields: `child: AnyWidget`, `commands: Rc<[Command]>`, `commands_for: Option<CommandsFor>`, `controller: Option<CommandPaletteController>`, `open_key: Str`, `key_label: Option<KeyLabel>`, `rows: i32`, `width: i32`, `look: PanelLook`, `prompt: Str`, `placeholder: Option<Str>`, `rule: bool`, `bar: bool`, `thumb: bool`, `barrier_alpha: Option<i32>`, `hinted_keys: bool`

### Code

The gallery's Code tab at the initial options (route `#/overlays/commandpalette`, `crates/rtwk-gallery/src/pages/mod.rs:1276`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

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

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| width | 40, 60 | 40 |
| rows | 3, 10 | 10 |
| files | on, off | true |
| rule | on, off | false |
| bar | on, off | false |
| thumb | on, off | false |
| placeholder | on, off | false |
| look | raised, bordered | raised |

### Methods

#### `CommandPalette`: constructors and associated functions

- `pub fn new(child: impl Into<AnyWidget>) -> CommandPalette` - A palette over `child`, opened by Ctrl+P, of no commands yet.

#### `CommandPalette`: builder methods (take and return `self`)

- `pub fn commands(mut self, commands: impl Into<Rc<[Command]>>) -> CommandPalette` - Listing `commands`.
- `pub fn commands_for(mut self, f: impl Fn(&str) -> CommandsAnswer + 'static) -> CommandPalette` - Asking `f` for more commands as the query changes.
- `pub fn controller(mut self, c: impl Into<CommandPaletteController>) -> CommandPalette` - Opened and closed by `c`.
- `pub fn open_key(mut self, key: impl Into<Str>) -> CommandPalette` - Opened by `key`.
- `pub fn key_label(mut self, f: impl Fn(&rtwk_framework::focus::KeyCombo) -> String + 'static) -> CommandPalette` - Naming keys with `f`.
- `pub fn rows(mut self, rows: i32) -> CommandPalette` - At most `rows` commands shown.
- `pub fn width(mut self, width: i32) -> CommandPalette` - `width` columns wide.
- `pub fn look(mut self, look: PanelLook) -> CommandPalette` - Raised or bordered.
- `pub fn prompt(mut self, prompt: impl Into<Str>) -> CommandPalette` - With `prompt` before the field.
- `pub fn placeholder(mut self, placeholder: impl Into<Str>) -> CommandPalette` - With `placeholder` in the empty field.
- `pub fn rule(mut self, on: bool) -> CommandPalette` - With a rule under the field.
- `pub fn bar(mut self, on: bool) -> CommandPalette` - With a bar at the highlighted command.
- `pub fn thumb(mut self, on: bool) -> CommandPalette` - With a thumb.
- `pub fn barrier_alpha(mut self, alpha: impl Into<Option<i32>>) -> CommandPalette` - Dimming what is under it by `alpha` of 255; none for the theme's shade.
- `pub fn hinted_keys(mut self, on: bool) -> CommandPalette` - Whether the keys hinted show among the commands.

### Also used in the code

- `rtwk::focus::KeyBindings` (struct, `crates/rtwk-framework/src/focus/shortcuts.rs:1257`) Binds keys to handlers for the nodes below it.
  - public fields: `bindings: Vec<(Str`, `combos: Vec<(KeyCombo`, `hints: Vec<(Str`, `combo_hints: Vec<(KeyCombo`, `sequence_timeout: Option<Duration>`, `unmatched: UnmatchedKeys`, `child: AnyWidget`, `debug_label: Option<Str>`
- `rtwk::controls::Button` (struct, `crates/rtwk-controls/src/button.rs:107`) A button that tells `on_pressed` when it is clicked, or when Enter or Space is pressed while it has the focus.
  - public fields: `spans: Option<Rc<[Span]>>`, `tall: bool`, `on_pressed: Option<Handler<()>>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: bool`, `style: Option<Style>`, `focused_style: Option<Style>`, `hover_style: Option<Style>`, `disabled_style: Option<Style>`
- `rtwk::overlay::PanelLook` (enum, `crates/rtwk-overlay-base/src/panel.rs:13`) How a floating panel stands out of what is under it.
  - variants: `Raised`, `Bordered`
- `rtwk::overlay::Command` (struct, `crates/rtwk-overlay/src/command_palette.rs:59`) What the palette lists to run: a label, and what it does when run once the palette is closed: calls `on_selected`, or does `intent` from the node the keys went to first as the palette opened (the enabled action for it ...
  - public fields: `label: Str`, `on_selected: Option<Handler<()>>`, `intent: Option<AnyIntent>`, `detail: Option<Str>`
- `rtwk::overlay::CommandsAnswer` (enum, `crates/rtwk-overlay/src/command_palette.rs:115`) What a `CommandPalette::commands_for` gives: the commands at once, or in a future that comes later.
  - variants: `Now`, `Later`

## OptionList

Options in rows, one highlighted, the characters a query matched standing out and a detail at the right edge: what an autocomplete, a Select and a command palette list.

API:

- `rtwk::overlay::OptionList` (struct, `crates/rtwk-overlay-base/src/options.rs:96`) `options` one a row in the theme's surface and text, a column of room either side, each label cut with the theme's ellipsis where the row is too narrow; the `highlighted` one in the theme's selection, with a bar of the ...
  - public fields: `options: Rc<[OptionItem]>`, `highlighted: i32`, `rows: i32`, `bar: bool`, `thumb: bool`, `on_highlight: std::option::Option<Handler<i32>>`, `on_selected: std::option::Option<Handler<i32>>`

### Code

The gallery's Code tab at the initial options (route `#/overlays/optionlist`, `crates/rtwk-gallery/src/pages/mod.rs:1340`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

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

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| query | , o, sa |  |
| rows | 3, 5, 8 | 5 |
| details | on, off | true |
| bar | on, off | true |
| thumb | on, off | true |

### Methods

#### `OptionList`: constructors and associated functions

- `pub fn new(options: impl Into<Rc<[OptionItem]>>) -> OptionList` - A list of `options`, none highlighted.

#### `OptionList`: builder methods (take and return `self`)

- `pub fn highlighted(mut self, i: i32) -> OptionList` - With row `i` highlighted (-1 for none).
- `pub fn rows(mut self, rows: i32) -> OptionList` - At most `rows` rows shown.
- `pub fn bar(mut self, on: bool) -> OptionList` - With a bar at the highlighted row or not.
- `pub fn thumb(mut self, on: bool) -> OptionList` - With a thumb or not.
- `pub fn on_highlight(mut self, h: Handler<i32>) -> OptionList` - Calling `h` with a row highlighted by the mouse.
- `pub fn on_selected(mut self, h: Handler<i32>) -> OptionList` - Calling `h` with a row clicked.

### Also used in the code

- `rtwk::overlay::OptionItem` (struct, `crates/rtwk-overlay-base/src/options.rs:37`) One row of an `OptionList`: its label, with the characters at `marks` in the list's mark style (what a query matched), and a `detail` at the row's right end (twk's `Option`, named so as not to hide Rust's own).
  - public fields: `label: Str`, `marks: Vec<usize>`, `detail: std::option::Option<Str>`
- `rtwk::controls::Pressable` (struct, `crates/rtwk-controls/src/pressable.rs:143`) A control's node of the focus tree: what `builder` builds, which takes the focus, its keys and a click of the mouse while `enabled`.
  - public fields: `builder: PressBuilder`, `enabled: bool`, `can_request_focus: bool`, `bindings: Rc<[(Str`, `hints: Rc<[(Str`, `on_key_event: Option<FocusOnKeyEventCallback>`, `on_tap: Option<Handler<PointerEvent>>`, `on_tap_up: Option<Handler<PointerEvent>>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `behavior: HitTestBehavior`

## ModalBarrier

What dims the app under a dialog and keeps the mouse from it.

API:

- `rtwk::overlay::ModalBarrier` (struct, `crates/rtwk-overlay/src/barrier.rs:72`) Covers the room it gets and dims what is under it: each cell keeps its glyph and its attributes, its background, its text color and the color of its underline taken `opacity` of the way to `color`, channel by channel ...
  - public fields: `color: Option<Color>`, `alpha: Option<i32>`, `opacity: Option<f64>`, `on_dismiss: Option<Handler<()>>`

### Code

The gallery's Code tab at the initial options (route `#/overlays/modalbarrier`, `crates/rtwk-gallery/src/pages/mod.rs:1383`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

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

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| barrier | on, off | true |
| alpha | 64, 128, 192 | 128 |
| fade | on, off | false |

### Methods

#### `ModalBarrier`: constructors and associated functions

- `pub fn new() -> ModalBarrier` - A barrier of the theme's shade, a click not dismissing it.

#### `ModalBarrier`: builder methods (take and return `self`)

- `pub fn color(mut self, color: Color) -> ModalBarrier` - Blending with `color`.
- `pub fn alpha(mut self, alpha: i32) -> ModalBarrier` - Taking `alpha` of 255 of the color.
- `pub fn opacity(mut self, opacity: f64) -> ModalBarrier` - Taking `opacity` of the color.
- `pub fn on_dismiss(mut self, h: Handler<()>) -> ModalBarrier` - Calling `h` when clicked.

### Also used in the code

- `rtwk::controls::Button` (struct, `crates/rtwk-controls/src/button.rs:107`) A button that tells `on_pressed` when it is clicked, or when Enter or Space is pressed while it has the focus.
  - public fields: `spans: Option<Rc<[Span]>>`, `tall: bool`, `on_pressed: Option<Handler<()>>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: bool`, `style: Option<Style>`, `focused_style: Option<Style>`, `hover_style: Option<Style>`, `disabled_style: Option<Style>`
- `rtwk::layout::Stack` (struct, `crates/rtwk-widgets/src/layout/stack.rs:36`) Its children on top of each other, a later one over an earlier one (twk's `Stack`, over a `RenderStack`).
  - public fields: `alignment: Alignment`, `fit: StackFit`
- `rtwk::layout::StackFit` (enum, `crates/rtwk-render/src/objects/stack.rs:23`) How a `RenderStack` lays out the children it places by its alignment.
  - variants: `Loose`, `Expand`, `Passthrough`
- `rtwk::layout::Center` (struct, `crates/rtwk-widgets/src/layout/basic.rs:191`) An `Align` that puts its child in the middle, the odd cell after it (twk's `Center`).
  - public fields: `width_factor: Option<i32>`, `height_factor: Option<i32>`, `child: Option<AnyWidget>`
- `rtwk::overlay::Panel` (struct, `crates/rtwk-overlay-base/src/panel.rs:47`) `child` on a panel that floats over what is under it: the theme's surface and text (and `style` over them), `padding` around the child and, as `look` says, a cell of room around that (`PanelLook::Raised`) or a border in ...
  - public fields: `child: AnyWidget`, `look: PanelLook`, `padding: EdgeInsets`, `shadow: bool`, `style: Option<Style>`

## WindowLayer

Windows over the app: a title bar drags one, an edge resizes it, its buttons minimise, maximise and close it; Tab to a title bar, then the arrows move it.

API:

- `rtwk::overlay::WindowLayer` (struct, `crates/rtwk-overlay/src/window/mod.rs:569`) Shows `windows` over `child`, each in a frame (`frame`) with its title on top, the title bar; laid out as `child` is, as big as it is.
  - public fields: `windows: Vec<Window>`, `child: AnyWidget`, `focused_style: Option<Style>`, `frame: Option<WindowFrame>`, `duration: Duration`, `margin: EdgeInsets`

### Code

The gallery's Code tab at the initial options (route `#/overlays/windowlayer`, `crates/rtwk-gallery/src/pages/mod.rs:1412`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

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

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| frame | raised, bordered | raised |
| shadow | on, off | true |
| resizable | on, off | true |
| minimisable | on, off | true |
| maximisable | on, off | true |

### Methods

#### `WindowLayer`: constructors and associated functions

- `pub fn new(windows: Vec<Window>, child: impl Into<AnyWidget>) -> WindowLayer` - Shows `windows` over `child`, each maximised, restored and minimised over 160 ms.

#### `WindowLayer`: builder methods (take and return `self`)

- `pub fn focused_style(mut self, style: Style) -> WindowLayer` - The window the focus is in merged with `style`.
- `pub fn frame(mut self, frame: WindowFrame) -> WindowLayer` - Drawn in `frame`.
- `pub fn duration(mut self, d: Duration) -> WindowLayer` - Maximised, restored and minimised over `d`; zero for at once.
- `pub fn decorate(mut self, f: impl Fn(&mut BuildCx<'_>, AnyWidget) -> AnyWidget + 'static) -> WindowLayer` - Each window, in its frame, shown in what `f` builds of it.
- `pub fn margin(mut self, margin: EdgeInsets) -> WindowLayer` - `margin` cells around each window for what `decorate` builds.

### Also used in the code

- `rtwk::overlay::WindowController` (struct, `crates/rtwk-overlay/src/window/mod.rs:52`) Where a `Window` is and how it shows: the cells it asks for, `left` columns and `top` rows from the top left of its `WindowLayer`, `width` by `height` with its frame, and whether it is minimised or maximised.
- `rtwk::overlay::Window` (struct, `crates/rtwk-overlay/src/window/mod.rs:300`) A window of a `WindowLayer`: `child` in a frame, `title` on its top edge, the title bar, with the buttons that minimise and maximise it (while `minimisable` and `maximisable`) and close it (while there is an ...
  - public fields: `title: Str`, `child: AnyWidget`, `key: Option<Key>`, `controller: Option<WindowController>`, `on_close: Option<Handler<()>>`, `resizable: bool`, `minimisable: bool`, `maximisable: bool`, `min_width: i32`, `min_height: i32`, `shadow: Option<BoxShadow>`
- `rtwk::layout::Padding` (struct, `crates/rtwk-widgets/src/layout/basic.rs:42`) Insets its child by `padding`: as big as the child and the padding.
  - public fields: `padding: EdgeInsets`, `child: Option<AnyWidget>`
- `rtwk::overlay::Panel` (struct, `crates/rtwk-overlay-base/src/panel.rs:47`) `child` on a panel that floats over what is under it: the theme's surface and text (and `style` over them), `padding` around the child and, as `look` says, a cell of room around that (`PanelLook::Raised`) or a border in ...
  - public fields: `child: AnyWidget`, `look: PanelLook`, `padding: EdgeInsets`, `shadow: bool`, `style: Option<Style>`
- `rtwk::controls::Button` (struct, `crates/rtwk-controls/src/button.rs:107`) A button that tells `on_pressed` when it is clicked, or when Enter or Space is pressed while it has the focus.
  - public fields: `spans: Option<Rc<[Span]>>`, `tall: bool`, `on_pressed: Option<Handler<()>>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: bool`, `style: Option<Style>`, `focused_style: Option<Style>`, `hover_style: Option<Style>`, `disabled_style: Option<Style>`
- `rtwk::overlay::WindowFrame` (enum, `crates/rtwk-overlay/src/window/mod.rs:435`) How a `WindowLayer` draws its windows.
  - variants: `Raised`, `Bordered`
