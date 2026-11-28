<!-- Written by `cargo xtask skill-references` from docs/guide; do not edit. -->
# Coming from Flutter

What is the same as Flutter, what is spelled differently, and what rtwk does
not have.

rtwk keeps Flutter's widget model: `StatelessWidget`, `StatefulWidget`, `State`,
`BuildContext` (as `Cx`), `InheritedWidget`, keys, constraints going down and
sizes going up, `Navigator`, `Overlay`, `Focus`. What you know of how a tree
builds, rebuilds and lays out holds. What does not hold is the unit, the
catalog and the language: the screen is a grid of character cells, there is no
Material, and the code is Rust, so a callback is a `Handler` that belongs to the
element that made it, and a setter that rebuilds is a handler that changes the
state it is given.

<!-- not rtwk: StatelessWidget BuildContext InheritedWidget MaterialApp Scaffold Container Card Material ElevatedButton TextButton OutlinedButton IconButton InkWell RadioListTile DropdownButton InputDecoration AlertDialog SnackBar CircularProgressIndicator LinearProgressIndicator TabBarView Radio DataTable GridView SelectableText WillPopScope MediaQuery TextStyle TextSpan RichText Colors Icon AppBar FloatingActionButton Drawer Hero Dismissible PageView AnimatedContainer SafeArea ClipRRect AspectRatio FittedBox IndexedStack ReorderableListView RefreshIndicator ScaffoldMessenger FocusTraversalGroup MaterialPageRoute Offset Size TextEditingController ChangeNotifier ValueNotifier ListenableBuilder ValueListenableBuilder FutureBuilder StreamBuilder TickerProviderStateMixin Tween Localizations LayoutBuilder ThemeData EdgeInsets BoxConstraints TweenAnimationBuilder ScrollBehavior ScrollConfiguration ScrollPhysics Riverpod ProviderScope BlocBuilder -->

If you write rtwk from Flutter's names, these are the mistakes that come first.

## Cells, not pixels

Every size, offset, padding and gap is a whole number of cells: an `i32`, not a
`double`. A cell is one column wide and one row tall, and it is about twice as
tall as it is wide, so `SizedBox::fixed(20, 10)` looks about square. There are
no fractions of a cell, no device pixel ratio and no rounded corners other than
the border characters of a `BoxWidget`.

<!-- file: crates/rtwk/examples/guides/flutter_cells.rs -->
```rust
use rtwk::layout::{BoxConstraints, ConstrainedBox, EdgeInsets, Padding, SizedBox, UNBOUNDED};
use rtwk::prelude::*;

pub fn sized() -> SizedBox {
    SizedBox::fixed(20, 3)
}

// left, top, right, bottom
pub const INSETS: EdgeInsets = EdgeInsets::only(2, 0, 2, 0);
// min and max width, min and max height: at most 40 wide, any height
pub const LOOSE: BoxConstraints = BoxConstraints::new(0, 40, 0, UNBOUNDED);

pub fn padded(child: impl Into<AnyWidget>) -> Padding {
    Padding::new(INSETS, ConstrainedBox::new(LOOSE, child))
}
```

- `BoxConstraints` has `min_width`, `max_width`, `min_height` and `max_height`, as
  in Flutter, in cells: `BoxConstraints::new(min_width, max_width, min_height,
  max_height)`, with `UNBOUNDED` for no end. `tight(w, h)` and `loose(w, h)` take
  two sizes, and `tight_for`, `expand`, `loosen`, `enforce`, `tighten`, `deflate`,
  `is_tight`, `has_bounded_width`, `biggest` and `smallest` are there. The
  tester's root is given tight constraints, as Flutter's is, which a `SizedBox`
  inside it cannot override: loosen them with a `Column` or an `Align` to see a
  widget's own size.
- `EdgeInsets` has `all`, `only(left, top, right, bottom)` (four numbers, not
  named arguments), `symmetric(horizontal, vertical)`, and `x` and `y`, which are
  shorter than `symmetric` when one side is wanted.
- An `f64` is still an `f64` where it is a proportion: `Slider`'s value,
  the opacity of an `Opacity`.
- The screen's size is not in a `MediaQuery`: there is none. A widget reads the
  room it was given, in cells, from a `LayoutBuilder`'s constraints.
- rtwk has a `Size`, a width and a height in cells, as Flutter does, and no
  `Offset`: a position is two numbers of cells.

## Spacing is a parameter, as in Flutter

`Row`, `Column` and `Flex` take `spacing`, and `Wrap` a `run_spacing` besides, as
in Flutter, so the `SizedBox(height: 8)` between children is not written; here it
is an `i32` of cells. `Spacer` and `Expanded` are as in Flutter, with the flex
factor first: `Expanded::new(1, child)`.

## Text is a string with a `Style`

There is no `TextStyle`, `TextSpan`, `RichText`, `Colors` or `Icon`.

- `Text::new("hi").style(..)` takes a `Style`: `fg` and `bg` colors and attributes
  (`Attrs::BOLD`, `Attrs::DIM`, `Attrs::ITALIC`, underlines, `Attrs::REVERSE`,
  `Attrs::STRIKE`). A `Style` has no font, size or weight: the terminal draws one
  font.
- A `Color` is one of the terminal's sixteen (`Color::RED`, `Color::BLUE`), an
  indexed one or an RGB one. There is no `Colors.red[500]`.
- Styled runs are `Span`s: `Span::new("error: ").bold().fg(Color::RED)`, and
  `Text::rich` takes a list of them where Flutter's takes a `TextSpan` tree.
- `DefaultTextStyle::new(style, child)` merges `style` onto the one above, field by
  field, where Flutter's plain constructor replaces it: a field the style leaves
  unset keeps the value from above. `Style::merge` does the same for two styles,
  and a `Style` can switch an attribute off (`attrs_off`) where merging would
  keep it. To start from nothing, set every field, or put the style on the `Text`.
- An icon is a character, `Text::new("✉")`, or a mark of the theme's `Glyphs`.
- `Text` wraps at words by default; `Text::line` is one row, cut with an
  ellipsis. A width is counted in cells, so a wide character (CJK, most emoji) is
  two.

<!-- file: crates/rtwk/examples/guides/flutter_text.rs -->
```rust
use rtwk::prelude::*;
use rtwk::style::{Attrs, Span};

pub const WARN: Style = Style::new().fg(Color::YELLOW).attrs(Attrs::BOLD);

pub fn rich() -> Text {
    Text::rich([
        Span::new("error: ").bold().fg(Color::RED),
        Span::new("disk full").dim(),
    ])
}
```

The colors an app draws with come from a `Theme`: `Theme::of(cx)` gives `text`,
`muted`, `primary`, `border`, `error` and the other `Style`s of
[Theming](theming.md), which follow the terminal's own colors by default. Writing
a `Style` with a fixed color is the exception, not the way a Flutter app writes
`Theme.of(context).colorScheme`.

## No Material, and what is there

rtwk has no `MaterialApp`, `AppBar`, `Card`, `FloatingActionButton`, `Drawer`,
`SnackBar`, `Hero`, `InkWell`, `Dismissible`, `GridView`, `PageView`,
`AnimatedContainer`, `SafeArea`, `ClipRRect`, `AspectRatio`, `FittedBox`,
`IndexedStack`, `ReorderableListView`, `RefreshIndicator`, `ScaffoldMessenger`,
`FocusTraversalGroup` or `showModalBottomSheet`. Each has a counterpart or none:

| Flutter | rtwk |
| --- | --- |
| `runApp(MaterialApp(...))` | `run_app(state, view)`: it places an `App` root with an `Overlay` for dialogs and toasts |
| `Scaffold(appBar, body, drawer)` | `Scaffold::new(body)` with `.header(..)`, `.sidebar(..)` and `.status_bar(..)` |
| `Container(decoration: BoxDecoration(border: ...))` | `BoxWidget::new().border(..).title(..).padding(..).style(..)`; `Container` exists too |
| `Card`, `Material` | `Panel` |
| `ElevatedButton`, `TextButton`, `OutlinedButton` | `Button::new`, `Button::filled`, `Button::outlined` (a label and `on_pressed`, no `child`) |
| `IconButton`, `InkWell` | `Pressable`, `GestureDetector` |
| `Checkbox`, `Switch`, `Slider` | the same names; `Checkbox` and `Switch` take a `label`, and no `on_changed` disables them |
| `Radio`, `RadioListTile` | `RadioGroup` |
| `DropdownButton` | `Select` |
| `TextField(decoration: InputDecoration(...))` | `TextField`, with a `placeholder` and an `InputDecoration` |
| `Form`, `TextFormField` | the same names |
| `AlertDialog`, `showDialog` | `Dialog`, `show_dialog` |
| `SnackBar` | `show_toast` |
| `Tooltip` | `Tooltip` |
| `CircularProgressIndicator`, `LinearProgressIndicator` | `Spinner`, `ProgressBar` |
| `TabBar`, `TabBarView`, `TabController` | the same names |
| `DataTable` | `Table` (rows and columns, selection, sorting) |
| `GridView` | `Grid` with column and row tracks |
| `SelectableText` | `SelectionArea` |
| `WillPopScope` | `PopScope` |
| `Image` | `Image` in `rtwk::rich`, in the terminal's graphics or block characters |

[The widget catalog](../widgets.md) lists what rtwk has that Flutter does
not: `Sparkline`, `Chart`, `Gauge`, `Calendar`, `TreeView`, `SplitView`,
`CommandPalette`, `Stepper`, `Breadcrumbs`, `Console`, markdown and diff views and
a terminal in a widget.

## The keyboard is first

A terminal has a keyboard before it has a mouse, and a widget that can be acted
on is reached with Tab. A `Button` does nothing until it has the focus (Tab, or
the mouse) and Enter or Space is pressed; a `TextField` takes characters only when
focused, so a single-letter binding such as `q` above a focused field is typed
into the field.

- Keys are named by strings: `KeyBindings::new([("ctrl+s", save), ("g g", top)],
  child)`. `Shortcuts`, `Actions` and `Intent` are Flutter's and are there as well;
  the string form is how an app usually binds. See [Input and
  focus](input-and-focus.md).
- The mouse is a `GestureDetector`, `MouseRegion`, `Draggable` and `Listener`, with
  Flutter's callbacks by name (`on_tap`, `on_double_tap`, `on_long_press`,
  `on_pan_start`, `on_pan_update`, `on_pan_end`; a `Listener` has
  `on_pointer_down`, `on_pointer_up`, `on_pointer_hover` and `on_pointer_move`),
  and a different argument: a tap is told the `PointerEvent`, which has the cell
  (`x` and `y` in the widget, `screen_x` and `screen_y` on the screen), the
  `button` and the `modifiers`, and a pan the `DragDetails`: that event and the
  `dx` and `dy` moved. A terminal reports no velocity or timestamp. Not every
  terminal reports the mouse, so an action reachable only by the mouse is a bug
  here.
- `KeyHintsBar` shows the keys that the bindings and the widgets hint, on the bar at
  the bottom.

## A screen, ported

A Flutter form, as it is written here:

<!-- file: crates/rtwk/examples/guides/flutter_form.rs -->
```rust
use std::process::ExitCode;

use rtwk::controls::{Button, Checkbox};
use rtwk::editing::{TextEditingController, TextField};
use rtwk::layout::{CrossAxisAlignment, EdgeInsets, Padding, SizedBox};
use rtwk::overlay::{Dialog, DialogOptions, ToastOptions, show_dialog, show_toast};
use rtwk::prelude::*;
use rtwk::{Cx, impl_widget};

pub fn main() -> ExitCode {
    run_app((), |_, _| Profile.into())
}

pub struct Profile;

pub struct ProfileState {
    name: TextEditingController,
    news: bool,
}

impl StatefulWidget for Profile {
    type State = ProfileState;

    fn create_state(&self) -> ProfileState {
        ProfileState {
            name: TextEditingController::new(""),
            news: false,
        }
    }
}

impl State for ProfileState {
    type Widget = Profile;
    type Msg = ();

    fn build(&self, _: &Profile, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let theme = Theme::of(cx);
        let save = cx.callback_with(|_, (): (), cx| {
            show_toast(cx, "saved", ToastOptions::default());
            Rebuild::No
        });
        let delete = cx.callback_with(|_, (): (), cx| {
            drop(show_dialog::<(), _>(
                cx,
                |_| {
                    Dialog::new(Text::new("This cannot be undone."))
                        .title("Delete?")
                        .into()
                },
                DialogOptions::default(),
            ));
            Rebuild::No
        });
        Padding::new(
            EdgeInsets::all(1),
            Column::new()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .spacing(1)
                .child(Text::new("Profile").style(theme.primary.clone()))
                .child(SizedBox::new(
                    Some(30),
                    None,
                    Some(
                        TextField::new()
                            .controller(self.name.clone())
                            .semantics_label("Name")
                            .into(),
                    ),
                ))
                .child(
                    Checkbox::new(self.news)
                        .label("Send me the news")
                        .on_changed(
                            cx.callback(|state: &mut ProfileState, value| state.news = value),
                        ),
                )
                .child(
                    Row::new()
                        .spacing(2)
                        .child(Button::filled("Save").on_pressed(save))
                        .child(Button::outlined("Delete").on_pressed(delete)),
                ),
        )
        .into()
    }
}

impl_widget!(stateful Profile);
```

What changed from the Flutter version: a `State` has a `Msg` and a `build` that
makes handlers from its `Cx` (`cx.callback` changes the state and rebuilds, which
is what `setState(() => ..)` is), the sizes and the `spacing` are numbers of
cells, the label belongs to the `Checkbox`, the buttons take a string, and
`show_dialog` and `show_toast` take the `Cx` where Flutter's take the `context`.
The controller is a handle: cloned into the widget, it is shared, and there is
nothing to `dispose`, since it is dropped with its last handle.

## Spelled another way, on purpose

A few names are not Flutter's, and are meant not to be:

- A `Style` has `fg` and `bg`, not `color` and `backgroundColor`; there is no
  `TextStyle`.
- The theme presets are constants, `ThemeData::DARK`, `ThemeData::LIGHT` and
  `ThemeData::NORD`, to take `copy_with` of, where Flutter calls `ThemeData.dark()`.
- `Scaffold` has a `header` and a `status_bar` where Flutter's has an `appBar` and
  a `bottomNavigationBar`, and `Select` has `choices` where a `DropdownButton` has
  items.
- A callback is a `Handler<E>` made from a `Cx` (`cx.callback`, `cx.send`,
  `cx.map`), or, in an `AppView`, from the `ViewCx` (`cx.on`, `cx.on_with`). A
  handler is equal to its clones, which is what lets a rebuild with the same
  handlers keep the widgets under it ([State](state.md)).
- `TweenAnimationBuilder` takes a `Tween` as Flutter's does.
- Flutter's `Localizations`, for a `Locale`, is not here; the words rtwk writes
  itself are in `TwkLocalizations` ([Words in another language](localization.md)).

## Lists and navigation

`SingleChildScrollView`, `ListView` and `Table` take padding, which scrolls with the
content, and a `Table`'s header stays where it is.

`ListView::new` takes children; `ListView::builder(count, |index| ..)` builds only
the rows in view. A `ListView` is focusable and scrolls with the arrow keys, Page
keys and the wheel, and a `ListTile`'s `on_tap` runs on Enter, Space or a click.

`Navigator::new().home(..).route(name, builder)` takes the first screen and a table of
named routes, and `Navigator::push`, `push_named`, `pop`, `pop_until`,
`push_and_remove_until` and `pop_and_push_named` are associated functions that take
the `Cx`, as Flutter's take the `context`. A route is a `PageRoute::new(builder)`, not
a `MaterialPageRoute`. Escape pops by default (`back_key`), as a back button does.

The builder of a list gets no `Cx`, so a row that pushes a route is a widget of its
own, where Flutter's `itemBuilder` has the `context` at hand; it makes its handler
in its own `build`:

<!-- file: crates/rtwk/examples/guides/flutter_list.rs -->
```rust
use rtwk::controls::ListTile;
use rtwk::navigation::{Navigator, PageRoute};
use rtwk::prelude::*;
use rtwk::scroll::ListView;
use rtwk::{Cx, impl_widget};

/// The home of a navigator: the rows are built as they come into view.
pub fn items() -> AnyWidget {
    ListView::builder(100, |i| Item(i).into()).into()
}

/// One row. A handler belongs to the element that made it, so a row that pushes a route is a
/// widget of its own, where Flutter's `itemBuilder` has the `context` at hand.
#[derive(Clone, Debug, PartialEq)]
pub struct Item(pub usize);

pub struct ItemState;

impl StatefulWidget for Item {
    type State = ItemState;

    fn create_state(&self) -> ItemState {
        ItemState
    }
}

impl State for ItemState {
    type Widget = Item;
    type Msg = ();

    fn build(&self, item: &Item, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let i = item.0;
        let open = cx.callback_with(move |_, (): (), cx| {
            drop(Navigator::push::<(), _>(
                cx,
                PageRoute::<()>::new(move |_| Text::new(format!("Item {i}")).into()),
            ));
            Rebuild::No
        });
        ListTile::new(Text::new(format!("Item {i}")))
            .on_tap(open)
            .into()
    }
}

impl_widget!(stateful Item);
```

A list that is long and has a handler for each row can also make one handler for
the whole list and bind each row's index to it (`ViewCx::action_route_shared`, and
`ActionRoute::bind` in the builder); the API reference of `action_route_shared` has an
example.

Scrolling has `ScrollPhysics` for its bounds and, as in Flutter, `ScrollConfiguration`
for its input: what the wheel and a trackpad do.

## State

`ChangeNotifier`, `ValueNotifier`, `ListenableBuilder`, `ValueListenableBuilder`,
`FutureBuilder`, `StreamBuilder` and `AnimationController` are as in Flutter, in
`rtwk::foundation` and `rtwk::animation`. `rtwk::provider` is Provider's API
(`ChangeNotifierProvider`, `Consumer`, `Selector`, `MultiProvider`), and
`rtwk::hooks` is flutter_hooks'. An app that keeps its state in a model, with
`AppView`, or in an external store, with `rtwk-state`, has no counterpart in
Flutter's own library, and [State](state.md) has all of them.

<!-- file: crates/rtwk/examples/guides/flutter_state.rs -->
```rust
use std::rc::Rc;

use rtwk::foundation::{ValueListenableBuilder, ValueNotifier};
use rtwk::prelude::*;

pub fn counter() -> Rc<ValueNotifier<i32>> {
    Rc::new(ValueNotifier::new(0))
}

pub fn built(counter: Rc<ValueNotifier<i32>>) -> ValueListenableBuilder<i32> {
    ValueListenableBuilder::new(counter, |_, n: &i32, _| Text::new(n.to_string()).into())
}
```

There is no `AnimatedContainer`; `AnimatedBox`, `AnimatedPadding`, `AnimatedAlign`,
`AnimatedPositioned`, `AnimatedOpacity`, `AnimatedSize`, `AnimatedSwitcher`,
`AnimatedDefaultTextStyle` and `TweenAnimationBuilder` are there. Animation is cell
by cell: a size animates in whole steps, at most `AppOptions::max_fps` (60) frames a
second.

## Runs differently

- `run_app` runs full screen on the alternate screen, or inline below the prompt
  ([Screen modes](screen-modes.md)). Nothing is a window.
- `println!` writes into the screen: log through the debug console instead
  ([Getting started](getting-started.md)). The `rtwk` command runs the app with the
  debug aids and an inspector; there is no hot reload yet.
- Tests are `rtwk_test`, with a `Tester` (`pump_widget`, `pump`), finders and a
  terminal in memory, whose screen a test reads as text: no golden pictures of
  pixels ([Testing](testing.md)).
- `rtwk::prelude::*` imports the names an app needs, and some names are words an
  app defines too (`Table`, `Step`, `Chip`, `Window`, `Gauge`): import the widgets by
  their module (`rtwk::data::Table`) where an app has its own.

## Where to go next

- [Layout](layout.md) for `BoxWidget`, `Grid`, `Stack` and the constraints in cells.
- [Input and focus](input-and-focus.md) for keys, focus and the mouse.
- [Theming](theming.md) for `Theme` and the `Style`s it gives.
- [State](state.md) and [Testing](testing.md).
- [Details and pitfalls](details.md).
- [The widget catalog](../widgets.md).
