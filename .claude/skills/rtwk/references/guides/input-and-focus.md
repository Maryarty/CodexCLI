<!-- Written by `cargo xtask skill-references` from docs/guide; do not edit. -->
# Input and focus

Keys, the focus that Tab moves, the keys hinted where it is, and the mouse.

A key goes first to the widget that has the focus, then up through the layers
above it (`KeyBindings`, `Shortcuts`, a `Focus` with an `on_key`), nearest first,
until one takes it. Tab and Shift+Tab that no widget takes move the focus to
the next or the previous widget that can have it.

<!-- file: crates/rtwk/examples/guides/focus.rs -->
```rust
use std::process::ExitCode;

use rtwk::AnyWidget;
use rtwk::app::ViewCx;
use rtwk::controls::{Button, KeyHintsBar};
use rtwk::editing::{TextEditingController, TextField};
use rtwk::focus::KeyBindings;
use rtwk::layout::{
    Border, BorderTitle, BorderType, BoxWidget, CrossAxisAlignment, EdgeInsets, MainAxisSize,
    SizedBox, Spacer,
};
use rtwk::prelude::*;
use rtwk::widgets::Builder;

pub fn main() -> ExitCode {
    run_app(SignIn::default(), view)
}

pub struct SignIn {
    // controllers are handles: kept in the app, they outlive every view
    pub name: TextEditingController,
    pub password: TextEditingController,
    pub status: String,
}

impl Default for SignIn {
    fn default() -> SignIn {
        SignIn {
            name: TextEditingController::new(""),
            password: TextEditingController::new(""),
            status: String::new(),
        }
    }
}

impl SignIn {
    fn save(&mut self) {
        self.status = if self.name.text().is_empty() {
            "a name, please".to_owned()
        } else {
            format!("hi, {}", self.name.text())
        };
    }
}

pub fn view(app: &SignIn, cx: &mut ViewCx<SignIn>) -> AnyWidget {
    let save = cx.on(|app: &mut SignIn, _| app.save());
    let (name, password, status) = (app.name.clone(), app.password.clone(), app.status.clone());
    KeyBindings::new(
        // not 'q' or 's': a focused field takes the characters typed
        [
            ("ctrl+s", save.clone()),
            ("ctrl+q", cx.on(|_, cx| cx.quit())),
        ],
        Builder::new(move |cx| {
            let theme = Theme::of(cx);
            BoxWidget::new()
                .style(theme.background.merge(&theme.text))
                .child(
                    Column::new()
                        .cross_axis_alignment(CrossAxisAlignment::Stretch)
                        .child(
                            BoxWidget::new()
                                .border(Border::all(theme.border.clone()).kind(BorderType::Rounded))
                                .title(BorderTitle::new(" Sign in ").style(theme.primary.clone()))
                                .padding(EdgeInsets::all(1))
                                .child(
                                    Column::new()
                                        .main_axis_size(MainAxisSize::Min)
                                        .cross_axis_alignment(CrossAxisAlignment::Stretch)
                                        .spacing(1)
                                        .child(labeled(
                                            "Name",
                                            TextField::new()
                                                .controller(name.clone())
                                                .placeholder("who you are")
                                                .autofocus(true),
                                        ))
                                        .child(labeled(
                                            "Password",
                                            TextField::new()
                                                .controller(password.clone())
                                                .obscure_text(true)
                                                .semantics_label("Password"),
                                        ))
                                        .child(labeled(
                                            "",
                                            Row::new()
                                                .spacing(2)
                                                .child(
                                                    Button::new("Sign in").on_pressed(save.clone()),
                                                )
                                                .child(
                                                    Text::new(status.clone())
                                                        .style(theme.muted.clone()),
                                                ),
                                        )),
                                ),
                        )
                        .child(Spacer::new(1))
                        // the keys the focused widget and the bindings above it take
                        .child(KeyHintsBar::new()),
                )
                .into()
        }),
    )
    .hints([("ctrl+s", "save"), ("ctrl+q", "quit")])
    .into()
}

/// `child` after `label`, in a column of labels as wide as the longest.
fn labeled(label: &str, child: impl Into<AnyWidget>) -> Row {
    Row::new()
        .spacing(1)
        .child(SizedBox::new(
            Some(8),
            None,
            Some(Text::new(label.to_owned()).into()),
        ))
        .child(Expanded::new(1, child))
}
```

After typing `ann`, Tab, a password and Ctrl+S:

```text
╭ Sign in ─────────────────────────────╮
│                                      │
│ Name     ann                         │
│                                      │
│ Password ••••••                      │
│                                      │
│           Sign in   hi, ann          │
│                                      │
╰──────────────────────────────────────╯
ctrl+z undo   ctrl+s save   ctrl+q quit
```

## Keys

- `KeyBindings::new([("ctrl+s", save), ("g g", top)], child)` binds keys by name
  for what is below it; a space between names makes a sequence. `.hints(..)`
  names what each does for a `KeyHintsBar`, which shows the keys that work where
  the focus is now. A handler is made where the view is, with `cx.on(..)`: it
  gets `&mut` of the app.
- A focused `TextField` takes the characters typed, so a binding above it for
  `q` never sees `q`. It lets Tab, Escape, Up, Down, Enter without
  `on_submitted` and the Ctrl and Alt keys it does not edit with go up: bind
  `ctrl+q` rather than `q` in an app with fields.
- A binding works on any keyboard layout in a terminal with the kitty keyboard
  protocol (rtwk asks for its alternate keys): Ctrl and the key that types `с`
  on a Russian layout is `ctrl+c`, as Ctrl and `c` is. A field still gets `с`
  for plain typing, and key hints show the name the binding is written with.
  Without the protocol, a terminal sends Ctrl and a letter as a control
  character whatever the layout, so it works there already.
- `Focus::bind(key, handler)` takes its key: Escape bound on a page never
  reaches the `Navigator`'s back key. To run a handler and let the key go on up,
  make the handler a passing one (`.passing()`); the
  [recipe](recipes.md#escape-that-passes) has it.
- `Shortcuts`, `Actions` and `Intent` are for keys that map to actions a widget
  below decides on.
- `InputListener` gets what nobody else took: keys, pastes, resizes.
- A text field asks the `EditingKeys` above it before it handles a key: the
  one door for an application's editing keys, such as a kill ring (see the
  [recipe](recipes.md#a-composer-with-a-kill-ring)).

## Focus

- `autofocus(true)` on a field or `Focus::autofocus()` gives a widget the focus
  when it is first shown.
- A `FocusNode` (`request_focus()`, `has_focus()`) given to a widget with its
  `focus_node(..)` moves the focus from code; in a handler,
  `AppCx::focus_node(&node)` does. `FocusScope` keeps Tab within a part of the
  app, `ExcludeFocus` keeps the focus out of one, and `cx.blur()` takes it from
  whatever has it.
- A widget that shows whether the focus is in it is a `FocusBuilder`: its
  builder runs again when the focus enters or leaves the widget, and the widgets
  around it do not. In a view, `cx.has_focus(id)` is the same for a `Focus` with
  an id (`Focus::new(..).id(FocusId(1))`), watched: the view runs again when the
  focus moves to or from that very `Focus`.
- Several keys read at once (fast typing, a paste of keys) are handled one after
  another with no build between them. A handler gets the app (or the component's
  state in `update`), so a key sees what the key before it changed: `+++` counts
  three. Keep the count in the state the handler changes, not in a value copied
  from the last build.

<!-- file: crates/rtwk/examples/guides/batch.rs -->
```rust
use std::process::ExitCode;

use rtwk::AnyWidget;
use rtwk::app::ViewCx;
use rtwk::focus::{Focus, KeyBindings};
use rtwk::prelude::*;

pub fn main() -> ExitCode {
    run_app(Counter::default(), view)
}

#[derive(Default)]
pub struct Counter {
    pub count: u32,
}

pub fn view(app: &Counter, cx: &mut ViewCx<Counter>) -> AnyWidget {
    KeyBindings::new(
        [
            // the handler gets the app itself: the next key of the batch sees this change
            ("+", cx.on(|app, _| app.count += 1)),
            ("q", cx.on(|_, cx| cx.quit())),
        ],
        Focus::new(Text::new(format!("count: {}", app.count))).autofocus(),
    )
    .into()
}
```

## The mouse

Full screen, the terminal reports the mouse while a widget that needs it is
mounted: `GestureDetector` (`on_tap`, `on_double_tap`, `on_pan_update`),
`Listener`, `MouseRegion` for hovers and the pointer's shape, `Draggable` and
`DragTarget`. A click on a focusable widget focuses it. Inline, the mouse stays
the terminal's: its own selection keeps working.

Detectors one inside another compete for a press: each gesture goes to the
deepest one with a callback for it, and a press is one gesture, so when the
inner one takes the tap the outer one gets nothing of it. A press of a button
the inner one has no callback for goes to the outer one. A tap on the outer
card's padding says nothing: a `BoxWidget` without a background style is hit on
its border and its child alone, so `GestureDetector::behavior(HitTestBehavior::Opaque)`
takes the cells between. Below, a tap on the inner card says `inner`, one on the
outer card's border `outer`, and F2 takes the focus from the search field, whose
`›` goes with it.

To see a press the inner detector takes, put a `Listener` (`on_pointer_down`,
`on_pointer_up`) around it: a `Listener` is not in the competition and gets
every press hit in it, the deepest first. It is the answer for a parent that
counts clicks itself, or that has to know of a tap anyway.

The terminal reports the mouse by mode for the whole app, not by area: presses
and the wheel (1000) for taps and scrolling, motion with a button held (1002) for
drags, all motion (1003) for hovers, `on_enter`, `on_exit`, a `MouseRegion`'s
`cursor` where the terminal shows the pointer's shape, and a `Button`'s hover
look. One of them anywhere in the app turns 1003 on everywhere. What a click
presses shows the hand by the `pointer_mode` of the theme: `PointerMode::Auto`, the
default, shows it everywhere and asks for the motion in a local session, but not
in a remote one, where each report crosses the network, so there it shows only
where the app has the terminal report motion already; `PointerMode::On` asks for
motion in a remote session too, `PointerMode::Off` leaves the pointer to the
terminal.

The pointer follows the press: a `GestureDetector` with `on_tap`, `on_tap_up` or
`on_tap_down` shows the hand by that rule, with nothing else to write. `cursor`
puts a `PointerShape` of its own in its place (`PointerShape::Arrow` for a click
that presses nothing), and `tap_at(|x, y| ..)` says which cells of a row a tap is
on, for the hand and the tap at once.

A `ListView` or `Scrollable` that should leave the wheel (and the keys) to the
view around takes `.physics(Rc::new(NeverScrollableScrollPhysics))`; its
controller still moves it.

<!-- file: crates/rtwk/examples/guides/who.rs -->
```rust
use std::process::ExitCode;

use rtwk::AnyWidget;
use rtwk::app::ViewCx;
use rtwk::editing::TextField;
use rtwk::focus::{FocusBuilder, KeyBindings};
use rtwk::gestures::{GestureDetector, PointerEvent};
use rtwk::layout::{Border, BorderType, BoxWidget, CrossAxisAlignment, EdgeInsets, SizedBox};
use rtwk::prelude::*;

pub fn main() -> ExitCode {
    run_app(Who::default(), view)
}

pub struct Who {
    pub said: String,
}

impl Default for Who {
    fn default() -> Who {
        Who {
            said: "nothing yet".to_owned(),
        }
    }
}

pub fn view(app: &Who, cx: &mut ViewCx<Who>) -> AnyWidget {
    // `on_with` takes the event: a tap's position and button, here left alone
    let tapped = |what: &'static str| {
        cx.on_with(move |app: &mut Who, _: PointerEvent, _| app.said = what.to_owned())
    };
    let (outer, inner) = (tapped("outer"), tapped("inner"));
    KeyBindings::new(
        [
            // takes the focus from whatever has it
            ("f2", cx.on(|_, cx| cx.blur())),
            ("ctrl+q", cx.on(|_, cx| cx.quit())),
        ],
        Column::new()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .child(Text::new(app.said.clone()))
            .child(
                GestureDetector::new().on_tap(outer).child(
                    BoxWidget::new()
                        .border(Border::all(Style::new()).kind(BorderType::Rounded))
                        .padding(EdgeInsets::all(1))
                        .child(
                            // the deepest detector with an on_tap takes the tap
                            GestureDetector::new().on_tap(inner).child(
                                BoxWidget::new()
                                    .border(Border::all(Style::new()).kind(BorderType::Single))
                                    .child(Text::new("inner")),
                            ),
                        ),
                ),
            )
            .child(SizedBox::new(
                Some(22),
                None,
                Some(
                    // built again when the focus enters or leaves the field, and nothing else is
                    FocusBuilder::new(|_, focused, field| {
                        Row::new()
                            .child(SizedBox::new(
                                Some(2),
                                None,
                                Some(Text::new(if *focused { "›" } else { " " }).into()),
                            ))
                            .child(Expanded::new(1, field.cloned().expect("the field")))
                            .into()
                    })
                    .child(TextField::new().placeholder("search").autofocus(true))
                    .into(),
                ),
            )),
    )
    .into()
}
```

More in the catalog's [Focus and keys](../widgets/focus-and-keys.md) and
[Mouse](../widgets/mouse.md) groups.
