---
name: rtwk
description: Write, test and debug Rust terminal UI apps and command-line tools with rtwk, which carries Flutter's widget model (StatelessWidget, StatefulWidget, Column, Row, Expanded, Focus) to Rust on a grid of character cells, with its own widgets (BoxWidget, ListView, Table, TextField, dialogs), app views whose handlers get `&mut App`, and headless tests (rtwk_test::Tester). Use when writing or changing code that uses the rtwk crates (rtwk, rtwk_test, rtwk_tokio) or before writing Flutter- or twk-style widget code in Rust: rtwk's names, units, ownership and defaults differ from Flutter's in ways memory gets wrong.
---

# Terminal applications with rtwk

<!-- not rtwk: TextStyle MediaQuery Counter Todo Slice Type -->

rtwk is Flutter's widget model on a terminal, in Rust: elements, keys,
constraints down and sizes up, `Focus`, `Overlay` and `Navigator` behave as
in Flutter (and in twk, the Dart framework rtwk follows). The unit, the
catalog, the input and the ownership do not. Do not write Flutter's API
from memory: there is no Material, no `TextStyle`, no `MediaQuery`, no
pixels, no `setState`. Look a widget up (References) before using it.

An app depends on the facade `rtwk` and imports from its modules
(`rtwk::layout`, `rtwk::controls`, `rtwk::editing`, `rtwk::overlay`,
`rtwk::style`, `rtwk::app`, `rtwk::prelude` for the common few); a command that
prints, asks and shows progress instead of drawing a screen uses
`rtwk::cli`; tests add `rtwk-test` as a dev-dependency and `rtwk-tokio` is the
companion for Tokio streams. Every Rust fence in this file is compiled
against the facade (`cargo test -p rtwk --doc skill_doctests`).

## Where rtwk differs

Each pair is the line a model writes from Flutter or twk memory (`-`, Dart) and
the Rust that is right (`+`). Every `+` line is compiled in
`crates/rtwk-gallery/tests/skill_differs.rs`, which fails where this
block and its source disagree.

```diff
# sizes are whole cells, optional i32s, and a child is an AnyWidget
- SizedBox(width: 20.5, height: 10.0, child: child)
+ SizedBox::new(Some(20), Some(10), Some(child))
# a style has colors and attributes, no font; a Text takes a Style (Color is in the prelude)
- Text('x', style: TextStyle(color: Colors.red, fontWeight: FontWeight.bold))
+ use rtwk::prelude::Color;
+ Text::new("x").style(Style::new().fg(Color::RED))
# spans are strings with a style; rich text takes a list of them
- Text.rich(TextSpan(children: [TextSpan(text: 'x')]))
+ Text::rich([Span::new("error: ").bold().fg(Color::RED), Span::new("disk full").dim()])
# the box widget is BoxWidget (Box is std's), and its border is a value
- Box(border: .all(type: .rounded), child: child)
+ BoxWidget::new().border(Border::all(Style::new()).kind(BorderType::Rounded)).child(child)
# children are macro arguments; Expanded takes its flex first
- Column(crossAxisAlignment: .stretch, children: [Expanded(child: list), footer])
+ column![Expanded::new(1, list), footer].cross_axis_alignment(CrossAxisAlignment::Stretch)
# no MediaQuery: the room is in the constraints
- MediaQuery.of(context).size.width
+ LayoutBuilder::new(|_, constraints| Text::new(constraints.max_width.to_string()).into())
# no setState: a handler changes the application and the view runs again
- Button('+', onPressed: () => setState(() => count++))
+ Button::new("+").on_pressed(cx.on(|app, _| app.count += 1))
# the entry point takes the state and a view function, and returns the exit code
- runApp(const TodoApp())
+ rtwk::run_app(Counter::default(), view)
# keys are named in strings, in lower case
- KeyBindings({'ctrl+s': save, 'g g': top}, child: child)
+ KeyBindings::new([("ctrl+s", cx.on(|_, cx| cx.quit()))], child)
# a controller is a handle, dropped with its owner: there is no dispose
- final controller = TextEditingController(text: 'a'); ... controller.dispose();
+ let controller = TextEditingController::new("a");
# tests: rtwk_test::Tester, not flutter_test; pump after input
- await tester.pumpWidget(widget); await tester.tap(find.byType(Button)); await tester.pump();
+ tester.pump_widget(widget);
+ tester.click_on(&find::by_type::<Button>());
+ tester.pump();
```

- A handler is retained and may run after the view call: it cannot borrow the
  view's `&App`. Move an owned id in (`move |app, _| ...`) and read the
  current state from the `&mut App` it gets (the `compile_fail` fence under
  Debug and pitfalls). An app handler rebuilds the view; call
  `cx.unchanged()` when nothing the view reads changed.
- `column!` and `row!` are the facade's macros (`use rtwk::column;`); a
  `Column` or `Row` built by hand takes `.child(..)` and its `main_axis_size`,
  `cross_axis_alignment`, `spacing`.
- A list (`ListView`, `Table`, `TreeView`, `SelectionList`, `Navigator`) takes
  all the room it gets: in a column put it in `Expanded`, or give it a
  height (`SizedBox::new(None, Some(8), Some(list))`).
- Dialogs, toasts, menus and popovers need an `Overlay` above their context:
  `run_app` places one at the root; a test mounts `Overlay::new(widget)`.
- Write nothing to stdout or stderr while a full-screen app runs; it owns the
  terminal. Use the log console (`rtwk::debug`).
- Only the focused widget gets a key first, then the `Focus` or
  `KeyBindings` above it. A focused `TextField` takes `q` and `ctrl+a`:
  bind `ctrl+q` to quit.

## Choose the entry point

- For a persistent widget tree, start with `rtwk::run_app(state, view)`. The application owns its domain data. A view receives `&App` and `&mut ViewCx<App>` and returns an `AnyWidget`. `cx.on` wires a zero-argument control and `cx.on_with` wires a control with a payload. Both lend handlers mutable access to the application when an event arrives. `rtwk::run_app_with(state, view, RunOptions { theme, locale, mode, .. })` configures the terminal and the standard root. To keep `AppView` builders such as `on_start`, `on_error`, `exit_on_ctrl_c(false)`, `clipboard_selection` or `on_copy`, build the `AppView` and pass it to `rtwk::run_app_view(app, options)`; do not copy the runner's body.
- For a command that prints lines, asks a question, or reports progress and returns control to the shell, use `rtwk::cli::Cli`.
- Use component state for reusable UI behavior such as selection, scroll position, or a local animation. The app API and component API share the same widget tree and handlers.

Use the Rust toolchain required by the RTWK checkout. In a downstream project with a local RTWK checkout, point Cargo at the facade and add the companion crates only when needed. For a Git dependency, use the same URL and revision for all RTWK crates.

```toml
[dependencies]
rtwk = { path = "../rtwk/crates/rtwk" }
# For a Tokio-backed app:
# rtwk-tokio = { path = "../rtwk/crates/rtwk-tokio" }
# tokio = { version = "1", features = ["rt-multi-thread", "time"] }
# futures-core = "0.3"

[dev-dependencies]
rtwk-test = { path = "../rtwk/crates/rtwk-test" }
```

## App state, handlers, and testing

Keep changing values in the state. Read their current values in the handler through its `&mut App`. Capture owned row IDs or other event parameters with `move`; borrowing the view's state into a handler fails because a handler can outlive the view call. An application handler schedules a view rebuild by default. Use `cx.unchanged()` when the update leaves the view's inputs unchanged.

A counter and its headless test share the same view. In an application crate, call `rtwk::run_app(Counter::default(), view)` from `main`, returning its `ExitCode`.

```rust
use rtwk::app::{AppView, ViewCx};
use rtwk::controls::Button;
use rtwk::layout::CrossAxisAlignment;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, column};
use rtwk_test::{Options, Tester, find};

#[derive(Default)]
struct Counter {
    count: u32,
}

fn view(app: &Counter, cx: &mut ViewCx<Counter>) -> AnyWidget {
    column![
        Text::new(format!("Count: {}", app.count)),
        Button::new("Increment").on_pressed(cx.on(|app, _| app.count += 1)),
    ]
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .into()
}

let mut tester = Tester::with(Options::new().size(32, 6));
tester.pump_app(AppView::new(Counter::default(), view));
tester.assert_line("Count: 0");
tester.click_on(&find::by_type::<Button>());
tester.pump();
tester.assert_line("Count: 1");
tester.read_app(|app: &Counter| assert_eq!(app.count, 1));
```

Use `rtwk_test::Tester` and `AppView::new(state, view)` for application tests. `pump_widget` mounts an isolated widget. Drive the app with `press`, `type_text`, `paste`, or `click_on` with a finder. Pump after input and use `pump_for(Duration)` to advance fake time for animations and timers. Assert visible output with `assert_line` or `assert_screen`. Use `read_app` to inspect domain state as needed. For interactive inspection run the gallery with `cargo run -p rtwk-gallery` in the RTWK checkout and press F2 on a relevant widget to see its source.

## A complete app

A to-do list: type a task and Enter adds it, Tab goes to the list, Space marks
the task done there, Ctrl+Q quits (not `q`: the focused field would take it as
text). The controllers are handles kept in the application, so they outlive
every view. After the app comes its test, with the screen it shows; Space needs
the row that Enter's frame builds, so pump between. `main` is
`rtwk::run_app(Todo::default(), view)`. The code is
`crates/rtwk-gallery/tests/skill_todo.rs` line for line, run there, and it
fails where this fence and its source differ.

```rust
use std::rc::Rc;

use rtwk::AnyWidget;
use rtwk::app::{AppView, ViewCx};
use rtwk::controls::{KeyHintsBar, SelectionList};
use rtwk::data::SelectionController;
use rtwk::editing::{TextEditingController, TextField};
use rtwk::focus::KeyBindings;
use rtwk::layout::{
    Border, BorderTitle, BorderType, BoxWidget, Column, CrossAxisAlignment, Expanded,
};
use rtwk::style::Style;
use rtwk::widgets::Text;
use rtwk_test::{Options, Tester, find};

/// A to-do list: Enter adds the task typed, Tab goes to the list, Space
/// marks a task done there, Ctrl+Q quits.
struct Todo {
    tasks: Vec<String>,
    // controllers are shared handles: kept in the model, they outlive every view
    text: TextEditingController,
    done: SelectionController<usize>,
}

impl Default for Todo {
    fn default() -> Self {
        Todo {
            tasks: Vec::new(),
            text: TextEditingController::new(""),
            done: SelectionController::with(true, [], None),
        }
    }
}

fn view(app: &Todo, cx: &mut ViewCx<'_, '_, Todo>) -> AnyWidget {
    let tasks: Rc<[String]> = app.tasks.clone().into();
    let list = SelectionList::new(
        tasks.len(),
        |i| i,
        move |i| Text::new(tasks[i].clone()).into(),
    )
    .controller(app.done.clone());
    let add = cx.on_with(|app: &mut Todo, task: Rc<str>, _| {
        if task.trim().is_empty() {
            return;
        }
        app.tasks.push(task.trim().to_owned());
        app.text.clear();
    });
    let body = Column::new()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(Expanded::new(
            1,
            BoxWidget::new()
                .border(Border::all(Style::new()).kind(BorderType::Rounded))
                .title(BorderTitle::new(" To do "))
                .child(list),
        ))
        .child(
            TextField::new()
                .controller(app.text.clone())
                .placeholder("a task, Enter adds it")
                .on_submitted(add)
                .autofocus(true),
        )
        .child(KeyHintsBar::new());
    // not 'q': the focused field would take it as text
    KeyBindings::new([("ctrl+q", cx.on(|_, cx| cx.quit()))], body)
        .hints([("ctrl+q", "quit")])
        .into()
}

let mut t = Tester::with(Options::new().size(30, 6));
t.pump_app(AppView::new(Todo::default(), view));
t.type_text("buy milk");
t.press("enter");
// pump: Space needs the row that Enter's frame builds
t.pump();
t.press("tab space");
t.pump();
t.assert_screen(
    "╭ To do ─────────────────────╮\n\
     │▎[x] buy milk               │\n\
     │                            │\n\
     ╰────────────────────────────╯\n\
     a task, Enter adds it\n\
     space toggle   ctrl+q quit",
);
assert_eq!(find::text("buy milk").count(&t), 1);
t.read_app(|app: &Todo| assert_eq!(app.tasks, ["buy milk"]));
```

## Layout, focus, and a form

Use `column!` and `row!` to compose children. `Text::new` displays text and `rtwk::editing::TextField` edits it. Use `rtwk::controls::TextFormField` inside a `Form` when the field participates in validation and submission. `on_changed` receives `Rc<str>`. Let the field own the edit buffer or keep its controller in persistent state if you need to update it from elsewhere.

Use one `autofocus(true)` field for initial input. Wrap a screen in `Focus` for bindings such as Ctrl+Q. Use a retained `FocusNode` or a stable `FocusId` with `AppCx::focus_node` or `AppCx::focus` when moving focus programmatically. A plain character such as `q` is text when an editor consumes it. Bind commands that should work during editing to a modified chord the field leaves available. `Focus::bind(key, handler)` consumes its key: Escape bound on a page never reaches the `Navigator`'s back key. To run a handler and let the key continue up (the page clears its draft and the navigator still pops it), use a passing key handler: `.on_key(cx.on_keys(|k| (k == KeyPress::ESC).then_some(()), |app, (), _| app.draft.clear()).passing())` in an application, `.on_key(cx.keys(|k| …).passing())` in a component.

What the app copies (a `SelectionArea` selection, a text field's Ctrl+C, `rtwk::editing::copy_to_clipboard`) goes to the terminal as OSC 52, selection `c` by default. `AppView::clipboard_selection(ClipboardSelection::Primary)` (or `ClipboardSelection::Both`) changes it. `AppView::on_copy(|text| …)` is also told each copied text so the app can copy natively where the terminal drops OSC 52, for example with the `arboard` crate or `pbcopy` on a thread. rtwk itself spawns no process. `Color` and `Attrs` for a `Style` come from `rtwk::style`.

Give a scrolling child a bounded viewport. Use `ListView::builder(count, builder)` for lazy rows, and use a stable identity for rows with retained state if their position can change.

```rust
use std::rc::Rc;

use rtwk::app::ViewCx;
use rtwk::controls::{Form, TextFormField};
use rtwk::focus::{Focus, KeyPress};
use rtwk::layout::{CrossAxisAlignment, MainAxisSize, SizedBox};
use rtwk::prelude::Mods;
use rtwk::scroll::ListView;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, column};

#[derive(Default)]
struct Notes {
    draft: String,
    saved: Vec<String>,
}

fn view(app: &Notes, cx: &mut ViewCx<Notes>) -> AnyWidget {
    let save = cx.on(|app, _| app.saved.push(app.draft.trim().to_owned()));
    let rows: Rc<[String]> = app.saved.clone().into();
    let list =
        ListView::builder(rows.len(), move |i| Text::new(rows[i].clone()).into()).item_extent(1);
    let form = Form::new(
        column![
            TextFormField::new()
                .placeholder("Note")
                .autofocus(true)
                .validator(|value| {
                    value
                        .is_none_or(|note| note.trim().is_empty())
                        .then(|| "Enter a note".into())
                })
                .on_changed(cx.on_with(|app, text: Rc<str>, _| app.draft = text.to_string())),
            Text::new("Enter to save"),
        ]
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start),
    )
    .on_submitted(save);

    Focus::new(column![form, SizedBox::new(None, Some(4), Some(list.into()))])
        .bind(
            KeyPress::char('q').with_mods(Mods::CTRL),
            cx.on(|_, cx| cx.quit()),
        )
        .into()
}
# let mut tester = rtwk_test::Tester::with(rtwk_test::Options::new().size(30, 10));
# tester.pump_app(rtwk::app::AppView::new(Notes::default(), view));
# tester.press("enter");
# tester.pump();
# tester.read_app(|app: &Notes| assert!(app.saved.is_empty()));
# tester.type_text("quiet");
# tester.pump();
# tester.press("enter");
# tester.pump();
# tester.read_app(|app: &Notes| assert_eq!(app.saved, ["quiet"]));
# assert!(tester.lines().iter().any(|line| line.trim() == "quiet"));
```

## Data, overlays, and shared state

For small typed data sets use `rtwk::data::Table`. Annotate the row parameter when inference needs help, give each sortable column a comparator, and keep a `TableController` across view calls. Stable `row_key` values keep selection attached to the underlying row after sorting. For large or lazy data use `Table::source` with a `TableSource`.

```rust
use rtwk::Key;
use rtwk::data::{Table, TableCell, TableColumn, TableController};
use rtwk::layout::SizedBox;

let rows = ["Gamma", "Alpha", "Beta"];
let columns = [
    TableColumn::new("Name", |name: &&str| TableCell::new(*name))
        .compare(|a, b| a.cmp(b)),
];
let controller = TableController::new();
controller.sort_by(Some(0), true);
let table = Table::new(rows, columns)
    .row_key(|name| Key::value(*name))
    .controller(controller)
    .show_header(true);
let mut tester = rtwk_test::Tester::with(rtwk_test::Options::new().size(24, 8));
tester.pump_widget(SizedBox::new(None, Some(6), Some(table.into())));
let screen = tester.screen();
assert!(screen.find("Alpha").unwrap() < screen.find("Gamma").unwrap());
```

`run_app` supplies an overlay at the root. Mount `rtwk::overlay::Overlay` above a component that opens floating UI when building a standalone test or supplying the root yourself. From its component context use `show_toast(cx, message, ToastOptions::default())` for transient feedback. Use `show_dialog::<T, _>(cx, builder, DialogOptions::default())` for a typed modal result. Handle the returned future in `cx.spawn` so the update and task lifetime follow the caller component. Call `close_dialog` from the dialog content to deliver a result; a body built by a closure binds its buttons to `close_dialog_handler(cx, Some(value))`. From an `AppView` handler use `cx.show_dialog::<T>(builder, DialogOptions::default(), |app, result, cx| …)`: the `closed` closure gets the `Option<T>` as a change of the application, so a `ViewFn` app needs no `StatefulWidget` for dialogs. The dialog is shown from where the handler was bound: bind it in a memo node (`cx.memo(deps, f)`, rtwk's `Builder`) under the `App` or `Navigator` the view returns, since a `ViewCx` handler sees only what is above the root. For a dialog driven by state (a channel's request, a task's output) use `cx.show_dialog_from::<T>(view_root, builder, options, closed)` with a `ViewRoot` a memo node under the navigator handed on (`MemoCx::root()`, kept in an `Rc<Cell<Option<ViewRoot<A>>>>` the memo takes as input). A dialog whose origin is no longer in the tree is not shown: `closed` gets `None`, and a debug build reports an `ErrorKind::DebugCheck`. A queue shows its next request from `closed` with `show_dialog_from` and the kept handle (not `show_dialog`, whose origin may be dead), and never shows the dialog again just because it got `None`. A `Navigator` can host dialog routes when the caller is in its route scope.

Use `Provider::value(value).child(subtree)` for a typed value shared down the tree. Read it below that provider with `provider::watch::<T, _>(cx)` in a build, `provider::select` for a projection, or `provider::read` when no rebuild dependency is desired. If a shared controller uses interior mutability, use a notifying provider such as `ChangeNotifierProvider` to propagate changes. An `Obs<T>` is a separate tree-owned reactive value. Create it with the owning context's `observable`, subscribe in build with `cx.watch(obs)`, and return `true` from the closure passed to `cx.write(obs, ...)` when the value changed. Keep handles in the tree that created them. A provider built from the application (`Provider::value(Slice { pending: app.pending })`) is a snapshot of the view's last run: a read in the same flush as the change returns the old value until the view runs again. Decide on the model, not on the projection. In a dialog's `closed`, which gets `&mut App`, take the request it answers and move the next one in with the same change.

```rust
use rtwk::app::ViewCx;
use rtwk::controls::Button;
use rtwk::layout::CrossAxisAlignment;
use rtwk::provider::{Consumer, Provider};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Obs, column};

#[derive(Default)]
struct Status {
    count: Option<Obs<u32>>,
}

fn view(app: &Status, cx: &mut ViewCx<Status>) -> AnyWidget {
    let count = app.count.map_or(0, |obs| cx.watch(obs));
    Provider::value(String::from("Sync"))
        .child(
            column![
                Consumer::<String>::new(|_, label, _| Text::new(label.to_string()).into()),
                Text::new(format!("Processed: {count}")),
                Button::new("Add").on_pressed(cx.on(|app, cx| {
                    if let Some(count) = app.count {
                        cx.write(count, |value| {
                            *value += 1;
                            true
                        });
                        cx.unchanged();
                    } else {
                        app.count = Some(cx.observable(1));
                    }
                })),
            ]
            .cross_axis_alignment(CrossAxisAlignment::Start),
        )
        .into()
}
# let mut tester = rtwk_test::Tester::with(rtwk_test::Options::new().size(28, 6));
# tester.pump_app(rtwk::app::AppView::new(Status::default(), view));
# tester.assert_line("Sync");
# for _ in 0..2 {
#     tester.click_on(&rtwk_test::find::by_type::<Button>());
#     tester.pump();
# }
# tester.assert_line("Processed: 2");
```

## Reusable components and overlays

Use `StatelessWidget` with `BuildCx` for a reusable type that builds from its inputs. Use `StatefulWidget` and `State` to retain state across builds. A build maps control events to messages with `cx.send` or `cx.map`. A message passed to `cx.send` implements `Clone + PartialEq`. `State::update` changes state and returns `Rebuild::Yes` or `Rebuild::No`. Use `impl_widget!(stateless Type)` or `impl_widget!(stateful Type)` to integrate with the widget tree. Widgets of the same type and key at a position can reuse the existing element. Give reorderable siblings keys derived from their identity, for example with `AnyWidget::with_key(Key::value(id))`.

```rust
use std::time::Duration;

use rtwk::controls::Button;
use rtwk::overlay::{
    Dialog, DialogOptions, Overlay, ToastOptions, close_dialog, show_dialog, show_toast,
};
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

struct DeleteButton;
struct DeleteState;

#[derive(Clone, PartialEq)]
enum DeleteMessage {
    Open,
    Answer(bool),
}

impl StatefulWidget for DeleteButton {
    type State = DeleteState;

    fn create_state(&self) -> DeleteState {
        DeleteState
    }
}

impl State for DeleteState {
    type Widget = DeleteButton;
    type Msg = DeleteMessage;

    fn build(&self, _: &DeleteButton, cx: &mut Cx<'_, Self>) -> AnyWidget {
        Button::new("Delete")
            .on_pressed(cx.send(DeleteMessage::Open))
            .into()
    }

    fn update(&mut self, msg: DeleteMessage, _: &DeleteButton, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            DeleteMessage::Open => {
                let result = show_dialog::<bool, _>(
                    cx,
                    |_| Dialog::new(Confirm).title("Delete the file?").into(),
                    DialogOptions::default(),
                );
                cx.spawn(async move { Some(DeleteMessage::Answer(result.await == Some(true))) });
            }
            DeleteMessage::Answer(true) => show_toast(cx, "Deleted", ToastOptions::default()),
            DeleteMessage::Answer(false) => {}
        }
        Rebuild::No
    }
}

impl_widget!(stateful DeleteButton);

struct Confirm;
struct ConfirmState;

impl StatefulWidget for Confirm {
    type State = ConfirmState;

    fn create_state(&self) -> ConfirmState {
        ConfirmState
    }
}

impl State for ConfirmState {
    type Widget = Confirm;
    type Msg = ();

    fn build(&self, _: &Confirm, cx: &mut Cx<'_, Self>) -> AnyWidget {
        Button::new("Confirm").on_pressed(cx.send(())).into()
    }

    fn update(&mut self, (): (), _: &Confirm, cx: &mut Cx<'_, Self>) -> Rebuild {
        close_dialog(cx, Some(true));
        Rebuild::No
    }
}

impl_widget!(stateful Confirm);

let mut tester = rtwk_test::Tester::with(rtwk_test::Options::new().size(40, 10));
tester.pump_widget(Overlay::new(DeleteButton));
tester.click_on(&rtwk_test::find::by_type::<DeleteButton>());
tester.pump_for(Duration::from_millis(200));
assert!(tester.screen().contains("Delete the file?"));
tester.click_on(&rtwk_test::find::by_type::<Confirm>());
tester.pump_for(Duration::from_millis(200));
assert!(tester.screen().contains("Deleted"));
```

## Tokio and streaming

Enter a multi-thread Tokio runtime on the UI thread before running the app. `rtwk_tokio::TokioApp` adds `spawn_tokio`, `subscribe`, and `try_subscribe` to `AppCx`. Keep terminal rendering and UI mutations on the UI thread. The producer is `Send` and delivers owned values to short UI handlers. Start work in a handler or an app start callback so a rebuild does not start a duplicate task.

Use `subscribe` for a stream without errors, and `try_subscribe` for one that yields `Result`. The default queue is bounded and applies backpressure, keeping ordered chunks before completion. Preserve every transcript chunk. Use latest-value coalescing only for data such as gauges where intermediate samples are disposable. Cancelling the returned task or unmounting its element aborts the producer.

```rust
# use std::pin::Pin;
# use std::task::{Context, Poll};
#
use futures_core::Stream;
use rtwk::AnyWidget;
use rtwk::app::{AppCx, ViewCx};
use rtwk::widgets::Text;
use rtwk_tokio::TokioApp;

#[derive(Default)]
struct Transcript {
    text: String,
    finished: bool,
}

fn view(app: &Transcript, _: &mut ViewCx<Transcript>) -> AnyWidget {
    Text::new(app.text.clone()).into()
}

fn start(cx: &mut AppCx<'_, '_, Transcript>, chunks: impl Stream<Item = String> + Send + 'static) {
    cx.subscribe(
        chunks,
        |app, chunk, _| app.text.push_str(&chunk),
        |app, _| app.finished = true,
    );
}

let runtime = tokio::runtime::Builder::new_multi_thread()
    .enable_all()
    .build()
    .unwrap();
let _entered = rtwk_tokio::enter(&runtime);
// Run the app here. Pass the stream to start from its request handler.
# struct Chunks(std::vec::IntoIter<String>);
#
# impl Stream for Chunks {
#     type Item = String;
#
#     fn poll_next(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<String>> {
#         Poll::Ready(self.0.next())
#     }
# }
#
# let mut tester = rtwk_test::Tester::with(rtwk_test::Options::new().size(32, 6));
# tester.pump_app(rtwk::app::AppView::new(Transcript::default(), view));
# tester.update_app(|_, cx| start(cx, Chunks(vec!["first ".into(), "second".into()].into_iter())));
# rtwk_test::wait_until("transcript stream to finish", || {
#     tester.pump();
#     tester.read_app(|app: &Transcript| app.finished)
# });
# tester.assert_line("first second");
```

## CLI prompts and progress

`Cli::default()` uses the process's terminal. Use `Cli::new` with a `FakeCliTerminal` for deterministic prompt and output tests. Prompts return a result. Choose an explicit default when it is appropriate for noninteractive use, or propagate the error. Progress reporting adapts to the output: piped output gets the final line, and interactive terminals can redraw. Close progress with `complete`, `fail`, or `cancel` according to the operation's outcome. Keep machine-readable output separate from human progress when the command has a structured-output mode.

```rust
use std::rc::Rc;
use rtwk::cli::{Cli, FakeCliTerminal};

let terminal = Rc::new(FakeCliTerminal::new());
let cli = Cli::new(terminal.clone());
if cli.confirm("Build the index?").default_value(true).ask()? {
    let progress = cli.progress("Indexing");
    progress.update("Reading files");
    progress.complete(Some("Index built"));
}
assert!(terminal.out_text().contains("Index built"));
assert!(!terminal.out_text().contains("Reading files"));
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Debug and pitfalls

- A handler is retained. Move an owned identifier into it and get the current state from the callback argument. An owned copy of the current displayed value will be stale if the callback fires after that value changes.
- Retain text, table, scroll and focus controllers when their state should survive rebuilds. Establish a stable widget key when a child can change position.
- Match the test driver to the behavior. Pump fake time for UI timers. Tokio work has its own runtime and may require a synchronization signal from the producer. Test input and focus together when diagnosing a shortcut that does not fire.
- Inspect the rendered screen and failure diagnostics from `Tester`. For live development, see `rtwk::debug` for the inspector and frame timing overlay. Avoid printing into the terminal while a full-screen widget app owns it.

```compile_fail,E0521
use rtwk::AnyWidget;
use rtwk::app::ViewCx;
use rtwk::controls::Button;

struct Counter {
    count: u32,
}

fn view(app: &Counter, cx: &mut ViewCx<Counter>) -> AnyWidget {
    Button::new("Increment")
        .on_pressed(cx.on(move |state, _| state.count = app.count + 1))
        .into()
}
```

## References

Read the one that matches the widget, not all of them. `references/`, next to
this file, is written by `cargo xtask skill-references` from the widget
catalog (`docs/widgets/`, written by `cargo xtask catalog` from the gallery
and the sources) and the guides (`docs/guide/`); do not edit it:

- `references/widgets.md`: every widget, one line each with its facade path
  (`rtwk::layout::Row`), by group.
- `references/widgets/<group>.md`: the group's widgets, each with the paths
  it is public at, the file and line it is defined at, the first line of its
  rustdoc, the Rust that builds it as the gallery's Code tab shows it, and the
  options.
- `references/guides/`: the guides, each a page of `docs/guide/`: `getting-started.md`,
  `layout.md`, `input-and-focus.md`, `theming.md`, `screen-modes.md`, `state.md`,
  `testing.md`, `running.md`, `recipes.md`, `widgets-by-need.md`, `cli.md`,
  `markdown.md`, `localization.md` (the words rtwk writes itself, in another
  language), `performance.md` (what to narrow, what to measure),
  `coming-from-flutter.md` (the names and units that differ) and `details.md` (how
  an app runs, the tester, the debug aids and the pitfalls).

Before writing a widget, look at a working one in the gallery. Its Code-tab
snippet is under the widget's `### Code` in `references/widgets/<group>.md`; the
whole page, with its state and controllers, is
`crates/rtwk-gallery/src/pages/<page>.rs` in the rtwk checkout (grep the
widget's name there); the Code-tab bodies, compiled, are in
`crates/rtwk-gallery/tests/code/`. `cargo run -p rtwk-gallery` in the checkout
shows every widget live: F2 shows the source of the one under the cursor, `c`
its Code tab. rtwk's own apps are in `crates/rtwk/examples/`.

If `references/` is not next to this file, the skill was copied short: copy
`skills/rtwk/` of the checkout whole, `SKILL.md` and `references/`, into the
agent's skills directory (`.claude/skills/rtwk`, `.agents/skills/rtwk`), or
load the checkout with `claude --plugin-dir /path/to/rtwk` and invoke
`/rtwk:rtwk`; the manifest points to `skills/rtwk`. See the
[Claude Code plugin manifest reference](https://code.claude.com/docs/en/plugins-reference#fields-table).

After editing this file run `cargo test -p rtwk --doc skill_doctests` and
`cargo nextest run -p rtwk-gallery skill_differs`: rustdoc compiles the Rust
fences against the facade and companion crates (lines prefixed by `# ` are
test setup or assertions and can be omitted in an application), the second
compiles the pairs above. `cargo xtask skill-references --check` holds
`references/` to the catalog and the guides, and fails where this file or a
guide names an rtwk type or `Type::member` that the sources do not define
(`<!-- not rtwk: Name Type::member -->` lists a name written on purpose: a
Flutter or application name, or a method a macro writes). The to-do app above
is `crates/rtwk-gallery/tests/skill_todo.rs`: edit the fence and that file
together.

<!-- The part below is CodexCLI's: written by tools/rtwk-port/skill/generate.py from tools/rtwk-port/skill/skill.src.md (rtwk c953f062); do not edit. -->

<!-- not rtwk: MaterialApp AppBar Card TextStyle Colors FontWeight TextSpan MediaQuery BuildContext RawKeyboardListener ValueKey Layout Constraint Ratio Fill List Rect Margin Clear Block Paragraph Line ListState ScrollState ScrollbarState Tabs KeyCode KeyEvent UnicodeWidthStr FrameRequester Instant Buffer AppEvent CodexCLI RenderFlex Intent Msg Picker Gxx -->

## CodexCLI: porting codex-tui onto rtwk

Everything above is rtwk's own skill (copied from `skills/rtwk` of the pin, with
its `references/`); this part is the port's. `references/widgets.md` is the
catalog (rtwk's `docs/widgets` at the pin, one file per group under
`references/widgets/`), `references/guides/state.md`, `cli.md`, `markdown.md` its
guides. The port adds under `references/guides/`: `app-api.md` and
`testing-api.md` (every public method of `AppView`, `ViewCx`, `AppCx`, `Cx`,
`Navigator`, `Tester`, from the pin's source), `recipes.md` (snippets that
compile and pass their tests against the pin: navigation with a result, a
request dialog, provider select, a list, a text area, a channel with a ticket,
a screen test) and `widgets-by-need.md` (which widget for which codex-tui
need). Read the one that matches the widget, not all of them.

A working widget to copy the manner from: the gallery page under the pin,
`crates/rtwk-gallery/src/pages/<page>.rs` in the pinned checkout named by the Cargo manifest (search for the
widget's name there; `cargo run -p rtwk-gallery` in the pin shows every widget
live, F2 shows the source of the one under the cursor), the pin's
`crates/rtwk/examples/`, and `/Users/admin/Documents/dev_projects/rtwk-weather`.

`SKILL.md` and `references/` are written by
`python3 -I tools/rtwk-port/skill/generate.py`, never by hand; `--check` fails
when a file differs from what the pin produces, when `docs/rtwk-port/RTWK_REF`
names another pin, or when this part names an rtwk type or member the pin lacks.

`codex-rs/tui-rtwk` carries `codex-rs/tui` (ratatui, the oracle, never edited)
onto rtwk: the oracle's look and behaviour are the spec, its mechanics are
not. Process and the concept map: `docs/rtwk-port/process.md` Part 1;
design: `docs/rtwk-port/architecture.md`.

### Reading ratatui, writing rtwk

The oracle does by hand what an rtwk widget owns: never re-implement it. Each
row is what the oracle's code does, and what the port writes instead.

| the oracle (ratatui) | rtwk |
| --- | --- |
| `fn render(&self, area: Rect, buf: &mut Buffer)` | `StatelessWidget::build` / `State::build` returns widgets; rtwk lays out and paints |
| `Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(area)` | `Column` of `SizedBox::new(None, Some(1), ..)` and `Expanded::new(1, ..)`; `Row` across |
| `Constraint::Percentage`, `Ratio`, `Fill` | `Expanded::new(flex, ..)`, `Flexible`, `Spacer` |
| `area.inner(Margin { .. })`, `x: area.x + 2` | `Padding::new(EdgeInsets::x(2), ..)` (`all`, `symmetric`, `only`) |
| `if area.width >= 90 { two panes }` | one `LayoutBuilder` read of `max_width` for the breakpoint; Row/Expanded inside |
| a centred `Rect` + `Clear` (popup, modal), keys routed by hand while it shows | `show_dialog(cx, builder, DialogOptions { alignment, .. })` + `close_dialog`; `Popover` anchored |
| `Block::bordered().title(..)` | `BoxWidget::new().border(Border::all(style).kind(BorderType::Rounded)).title(BorderTitle::new(..))` |
| `"─".repeat(width)`, a `│` per row | `Divider`, `VerticalDivider`, or `Border::new().bottom(style)` / `left(style)` on a `BoxWidget` |
| `Paragraph::new(..).wrap(..)`, `word_wrap_lines` | `Text::new(..)` (`TextWrap::Word` by default), `max_lines` |
| `truncate_text`, `format!("{}…", ..)` | `Text::line(..)`, `.overflow(TextOverflow::Ellipsis)`; `TextOverflow::Ellipsis` drops the blanks next to `…` (N1), so where the oracle keeps them: `widgets/dots.rs` `dots_end` (a composition, see the cookbook) |
| `center_truncate_path`, keep-the-end cuts | `TextOverflow::EllipsisMiddle`, `TextOverflow::EllipsisStart`; the mark is the `ellipsis` field of `Glyphs`; `TextOverflow::EllipsisMiddle` cuts inside segments, so a path that keeps its first and last segments is `pages/agents/path.rs` (G37, a closed row: carried oracle logic, not a model to copy) |
| `UnicodeWidthStr::width`, `chars().count()`, padding with spaces | none: `Row` with `SizedBox` widths and `Expanded`; a number only where unavoidable: `rtwk_core::unicode::string_width` |
| `Line::from(vec![Span::styled(..)])`, `.cyan().dim()` | `Text::rich([Span::new(..).fg(..).dim()])`, colours from `Theme::of(cx)` tokens |
| `List` + `ListState` + `ScrollState` (selected index, `scroll_top`, `ensure_visible`) | `SelectionList` + `SelectionController` (`cursor`, `set_cursor`, `selected`); whole-row scroll with ↑/↓ more: `widgets/session_list.rs` (`ListView::builder` + `ListController` + `SelectionController`, G35/G42) |
| a long or growing list drawn by offset | `ListView::builder(n, row)` + `ListController::ensure_index_visible`; `stick_to_bottom(true)` for a transcript |
| `Scrollbar` + `ScrollbarState` | `Scrollbar::new(child)`, `BoxWidget::scrollbar`; `SingleChildScrollView` for one tall child |
| `Tabs`, a tab strip sliced with `‹ ›` | `TabBar::new(controller, tabs)` / `TabBar::strip` with a `TabController` for an ordinary bar; the oracle's filled strip of whole tabs with `‹ ›` outside them is `pages/agents/tabs.rs` (G36/G45: `LayoutBuilder` + `segmenter_of(cx)` + a `Row`) |
| `Table` + column widths | a label or option column sized to its content: `Grid` with `GridTrack` (`content`, `fixed`, `fr`), in a `SingleChildScrollView` when it scrolls (`widgets/options.rs`, G39; not `Table`); `Table::new(rows, columns)`, `TableColumn::new(title, cell)`, `TableController` only for a data table with a header and sorting |
| `Gauge`, `LineGauge`, `Sparkline` | `Gauge`, `LineGauge`, `Sparkline` (`rtwk::data`) |
| `match key.code { KeyCode::Char('y') => .. }` in `handle_key_event` | `Focus::bind`, `KeyBindings`, `Shortcuts` / `Actions`; the footer's hints `KeyHintsBar` |
| the composer's `TextArea` (cursor, kill ring, wrap) | `TextArea` + `TextEditingController`; one line: `TextField` |
| a mouse hit test against row rects | `GestureDetector::on_tap`; lists, tabs and buttons take clicks themselves |
| `FrameRequester::schedule_frame`, shimmer or spinner frames by `Instant` | nothing to redraw by hand; `AnimationController`, `AnimatedBuilder`, `Spinner` |
| `tx.send(AppEvent::..)`, `tokio::spawn` writing back | `cx.on_with(..)` handlers; `AppCx::channel`, `spawn_tokio` with a ticket (R1) |
| `markdown_render`, `diff_render`, `highlight` | `MarkdownView`, `DiffView`, `CodeView` (`rtwk_rich`) |
| bottom-pane view stack, `pager_overlay` | `Navigator` + `PageRoute`; asking views: `show_dialog` |
| `insert_history_lines`, the custom terminal | `ScreenMode::Inline` + `Static`; rtwk owns the terminal |
| `insta::assert_snapshot!(terminal.backend())` | `Tester::with(Options::new().size(w, h))`, `assert_screen`, compared with the oracle through the replay helper |

### When the one widget differs from the oracle: compose, do not hand-roll

Try in order: the widget alone; a composition (Flutter's way: `LayoutBuilder`,
`Row`, `Grid`, `ListView` with its controllers, `CustomPaint`); a test setting.
Only then a gap row (`docs/rtwk-port/rtwk-gaps.md`) and a `pending(Gxx)` test.
Composition is the app choosing WHICH widget or spelling to build from a width
it measured (`segmenter_of(cx)` under a `LayoutBuilder`); cutting, rewrapping or
re-measuring strings yourself is the workaround (process.md section 5). Grep
`src/widgets` and `src/pages/agents` for a landed piece first.

| oracle pattern | rtwk route |
| --- | --- |
| filled tab strip of whole tabs, `‹ ›` outside | not `TabBar`: `pages/agents/tabs.rs` (a pure `strip()` window, a `Row` under a `LayoutBuilder`, widths by `segmenter_of(cx)`) |
| a cut `…` that keeps the blanks | `TextOverflow::Ellipsis` drops them (N1): `widgets/dots.rs` `dots_end` (`LayoutBuilder` + `Row[Expanded(clipped Text), Text("…")]`) |
| list with a cursor, whole-row scroll, ↑/↓ more | `widgets/session_list.rs`: `ListView::builder` + `ListController` + `SelectionController` + `ListenableBuilder` + `NotificationListener` |
| numbered options, label column + wrapped description | `widgets/options.rs`: `Column(min)[Flexible(SingleChildScrollView(Grid[content().within(0, cap), fr(1)]))]`, not `Table` |
| label/value card | a `Grid` with content tracks (`pages/session/cells/status/card.rs`) |
| a bar of whole cells | `CustomPaint` + `CellPainter` (`pages/session/cells/status/card.rs`, G40) |
| footer hints, the first spelling that fits | `widgets/page_keys.rs` `footer_fit` |
| a frameless bottom-pane view (`›` cursor) | `show_dialog` + `Dialog::bare` (`sheets/select/dialog.rs`) |
| the rows a `Text` takes at width w | `break_lines` (rtwk-render `objects/text.rs`) via `widgets/measure.rs`; first try `Flexible` or `max_lines` |
| display width of a string | `segmenter_of(cx)` (`rtwk::text`), never `string_width(.., TextMode::Graphemes, ..)` |
| `ｶﾞ` as two cells (oracle counts code points) | a test setting: `TextMode::Legacy` in the test terminal (`Options::terminal`, G44) |
| DIM asserted on a blank cell | the presenter drops an invisible DIM (`Attrs::VISIBLE_ON_BLANK`): assert the app surface's style, `frame_loop().styles().attrs_of(surface.style(x, y))` (G46) |
| `Wrap { trim: false }` breaking at blanks only | a real gap (G48): `pending`, no `Wrap` of words |
| a path cut that keeps its first and last segments | `pages/agents/path.rs` (G37, closed as ours, held to the oracle's tests; carried oracle logic, not a model to copy) |
| selected-row fill under blanks | `Container::color` or `ColoredBox` (G22) |
| a prompt `"› "` with its trailing blank | `Text::new("› ").wrap(TextWrap::None)` (G26) |

### Keys and focus

- A page key (Esc, `?`, `←`) must reach the page or the `Navigator` while a
  child handles the others: bind with `cx.keys(..).passing()`, a handler that
  lets the key on when it did not take it (G27); a plain `Focus::bind` consumes.
- A key that a focused field would swallow first (the composer's `?` on an
  empty field): `FocusNode::set_on_key_event` on the field's node
  (`TextArea::focus_node`), the handler tells the page through the local sender
  (G43). Landed uses: `pages/session/session_page.rs`, `sheets/select/dialog.rs`.
- A dialog or sheet brings its own key trap, Escape and focus return
  (`show_dialog`): bind nothing by hand under it.

### Port pitfalls

From the VS1 review and the rtwk-first sweep
(`docs/rtwk-port/reviews/vs1-review.md`, `rtwk-first-sweep.md`):

- A cursor, scroll top, tab index or query kept in core or in a `Cell` beside
  the widget is a shadow of the controller's: two sources synced both ways
  moved the wrong row (D2, D3). The controller owns it; intents carry ids.
- A decision from the model in a key or click handler, the same flush as the
  widget's own move (`jj` moved one row, a click opened the row not clicked):
  read the controller, send `Intent::Open(id)`.
- Cell arithmetic under a `LayoutBuilder` that hands widths to every builder
  (`inner - 4 - x` on a `usize` panicked at width 8): Row/Column/Expanded,
  and only the oracle's breakpoints read the width.
- `chars().count()` as a width, labels padded with spaces: wrong for CJK and
  emoji; a `Row` with `SizedBox` widths lines columns up.
- Hand `wrap` and `ellipsize` helpers: `Text` wraps and cuts. A cut that
  differs from the oracle is an `rtwk-gaps.md` row with both renderings, not
  a local helper.
- Modal panels drawn row by row in a `Stack` with keys routed by hand:
  `show_dialog` brings the key trap, Escape, the barrier and focus return.
- A `GestureDetector` per row to select: `SelectionList`'s tap already moves
  the cursor and selects.
- A carried oracle helper (`scroll_state.rs`, `selection_tabs.rs`,
  `text_formatting.rs`, `wrapping.rs`) is UI mechanics rtwk owns: check
  `references/guides/widgets-by-need.md` first.
- A delta written off as "accept": no visual delta is accepted. Compose rtwk
  primitives until rows and styles equal the oracle's; only what rtwk truly
  cannot do is a gap, reported with the route tried.

### More port pitfalls

- Keys typed together (a burst, `t.type_text("jj")`) are handled one after
  another before the next flush: a handler that steps from the model reads
  the value before the last step. Read the controller (`cursor()`), which
  changes at once.
- A view or `build` describes; it never writes state (no `Cell` set during
  build, no controller moved there). Changes go in handlers, `init_state`,
  `did_update_widget` or a controller's listener.
- Closures made anew in every view (`SelectionList::new(n, |i| ..)`) never
  compare equal, so the subtree rebuilds every frame: keep them as `Rc`s
  (`SelectionList::with_builders`, `ViewCx::memo`) and give a stateful widget
  `impl_widget!(stateful eq ...)`.
- A presenter that shows a dialog sits below the `Navigator`, else the
  dialog is above every provider.
- A `Column` in a `Dialog` needs `MainAxisSize::Min`, else it takes the whole
  height; a dialog fades in over 160 ms in a test: pump past it.
- Closure types in `update_app` and `read_app` need an annotation:
  `|a: &mut App, cx|`.
