---
name: rtwk
description: Build and test Rust terminal applications and interactive command-line tools with rtwk. Use for app views, stateful widgets, focus and forms, data and overlays, Tokio streams, CLI prompts and progress, or headless interaction tests.
---
<!-- rtwk 9f28b7fb: crates/rtwk/SKILL.md verbatim up to the line "CodexCLI"; the CodexCLI section is tools/rtwk-port/skill/skill-section.md. Written by tools/rtwk-port/skill/generate.py; do not edit. -->

# Terminal applications with rtwk

## Choose the entry point

- For a persistent widget tree, start with `rtwk::run_app(state, view)`. The application owns its domain data. A view receives `&App` and `&mut ViewCx<App>` and returns an `AnyWidget`. `cx.on` wires a zero-argument control and `cx.on_with` wires a control with a payload. Both lend handlers mutable access to the application when an event arrives.
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

## Layout, focus, and a form

Use `column!` and `row!` to compose children. `Text::new` displays text and `rtwk::editing::TextField` edits it. Use `rtwk::controls::TextFormField` inside a `Form` when the field participates in validation and submission. `on_changed` receives `Rc<str>`. Let the field own the edit buffer or keep its controller in persistent state if you need to update it from elsewhere.

Use one `autofocus(true)` field for initial input. Wrap a screen in `Focus` for bindings such as Ctrl+Q. Use a retained `FocusNode` or a stable `FocusId` with `AppCx::focus_node` or `AppCx::focus` when moving focus programmatically. A plain character such as `q` is text when an editor consumes it. Bind commands that should work during editing to a modified chord the field leaves available.

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

`run_app` supplies an overlay at the root. Mount `rtwk::overlay::Overlay` above a component that opens floating UI when building a standalone test or supplying the root yourself. From its component context use `show_toast(cx, message, ToastOptions::default())` for transient feedback. Use `show_dialog::<T, _>(cx, builder, DialogOptions::default())` for a typed modal result. Handle the returned future in `cx.spawn` so the update and task lifetime follow the caller component. Call `close_dialog` from the dialog content to deliver a result; a body built by a closure binds its buttons to `close_dialog_handler(cx, Some(value))`. From an `AppView` handler use `cx.show_dialog::<T>(builder, DialogOptions::default(), |app, result, cx| …)`: the `closed` closure gets the `Option<T>` as a change of the application, so a `ViewFn` app needs no `StatefulWidget` for dialogs. The dialog is shown from where the handler was bound: bind it in a memo node (`cx.memo(deps, f)`, rtwk's `Builder`) under the `App` or `Navigator` the view returns, since a `ViewCx` handler sees only what is above the root. For a dialog driven by state (a channel's request, a task's output) use `cx.show_dialog_from::<T>(view_root, builder, options, closed)` with a `ViewRoot` a memo node under the navigator handed on (`MemoCx::root()`, kept in an `Rc<Cell<Option<ViewRoot<A>>>>` the memo takes as input). A dialog whose origin is no longer in the tree is not shown: `closed` gets `None`, and a debug build reports a `DebugCheck`. A queue shows its next request from `closed` with `show_dialog_from` and the kept handle (not `show_dialog`, whose origin may be dead), and never shows the dialog again just because it got `None`. A `Navigator` can host dialog routes when the caller is in its route scope.

Use `Provider::value(value).child(subtree)` for a typed value shared down the tree. Read it below that provider with `provider::watch::<T, _>(cx)` in a build, `provider::select` for a projection, or `provider::read` when no rebuild dependency is desired. If a shared controller uses interior mutability, use a notifying provider such as `ChangeNotifierProvider` to propagate changes. An `Obs<T>` is a separate tree-owned reactive value. Create it with the owning context's `observable`, subscribe in build with `cx.watch(obs)`, and return `true` from the closure passed to `cx.write(obs, ...)` when the value changed. Keep handles in the tree that created them.

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

## Load the guide

The canonical file is `crates/rtwk/SKILL.md`. Load that file directly in a coding agent, or copy it into an `rtwk/SKILL.md` directory under the agent's skills directory. Claude Code can load the checkout with `claude --plugin-dir /path/to/rtwk` and invoke `/rtwk:rtwk`. The manifest points to this directory. See the [Claude Code plugin manifest reference](https://code.claude.com/docs/en/plugins-reference#fields-table).

From the RTWK checkout run `cargo test -p rtwk --doc skill_doctests` after editing this guide. Rustdoc compiles the Rust fences against the facade and companion crates. Lines prefixed by `# ` are test setup or assertions and can be omitted when using a snippet in an application.

## CodexCLI: porting codex-tui onto rtwk

Everything above is rtwk's own guide. This section is the CodexCLI part: the port
(`codex-rs/tui-rtwk`) carries `codex-rs/tui` (ratatui, the oracle, never edited) onto rtwk.
Process and the concept map: `docs/rtwk-port/process.md` Part 1; design:
`docs/rtwk-port/architecture.md`.

### Where rtwk differs from ratatui / codex-tui

Each pair: `-` what the oracle does, `+` what the port writes.

```
- fn render(&self, area: Rect, buf: &mut Buffer)           // a widget draws into a buffer
+ impl StatelessWidget { fn build(&self, cx: &mut Cx<'_>) -> AnyWidget }   // describe; rtwk lays out and paints
- Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(area)
+ Column::new().child(SizedBox::..).child(Expanded::new(1, body))   // constraints go down, sizes up
- Line::from(vec![Span::styled("x", Style::new().cyan().bold())])
+ Text::rich([Span::styled("x", style)])                    // style from theme tokens, not literal colours
- Paragraph::new(text).block(Block::bordered().title("t"))
+ BoxWidget::new().border(Border::all(style).kind(BorderType::Rounded)).child(Text::new(..))
- match key_event { KeyCode::Char('y') => .. }              // one big match in handle_key_event
+ Focus::new(child).bind('y', handler)  /  KeyBindings      // keys go to the focused widget, then up
- stateful widget struct with &mut self, mutated from the event loop
+ plain model `A` + `cx.on(|a, cx| ..)` handlers; the view reruns   // widget state (scroll, focus, edit) is rtwk's
- tx.send(AppEvent::Foo); app_event_rx.recv() in the main loop
+ cx.on_with(..) handler; work from other threads: AppCx::channel / cx.spawn_tokio + a ticket
- tokio::spawn(..) and write the result back through an AppEvent
+ cx.spawn_tokio(future, |a: &mut A, r, cx| ..)            // use rtwk_tokio::TokioApp; R1: tag the work with a ticket, drop a stale answer in the handler
- frame_requester.schedule_frame()
+ nothing: a handler that changes the model reruns the view; rtwk paces frames
- List + ListState, rendered with a computed offset
+ ListView::builder (virtualized), SelectionList for pick lists   // scroll and selection live in rtwk
- Clear + a Rect in the middle of the screen (popup, modal)
+ show_dialog / close_dialog, Popover, CommandPalette        // a dialog is a route; see recipes.md
- terminal.draw(|f| ..) in tui.rs, custom terminal and scrollback insertion
+ run_app_with(model, view, RunOptions)                      // full screen or inline; rtwk owns the terminal
- insta::assert_snapshot!(terminal.backend())
+ Tester::with(Options::new().size(w, h)); t.assert_screen(..) // compared against the oracle through the replay helper
```

Pitfalls found while compiling the recipes against the pin (each is in a `references/guides/recipes.md` snippet):

- A `Column` centres its children across; add `CrossAxisAlignment::Start` for left-aligned rows.
- A `Column` in a `Dialog` needs `MainAxisSize::Min`, else it takes the whole height.
- A `RenderFlex` that overflows fails a `Tester` test: give the test terminal room.
- `assert_line` compares the whole row; `assert_screen` takes rows from the top without trailing blanks.
- After input, `pump()`; past an animation (a dialog fades in over ~160 ms), `pump_for(..)`.
- A presenter that shows a dialog must sit below the `Navigator`, else the dialog is above every provider.
- Closure types in `update_app` need an annotation: `|a: &mut App, cx|`.

### references/ (generated, never hand-edited)

Written by `python3 -I tools/rtwk-port/skill/generate.py` from the rtwk pin; every file names
its pin. `python3 -I tools/rtwk-port/skill/generate.py --check` fails when a file differs from
what the pin produces or `docs/rtwk-port/RTWK_REF` names another pin. Read this file once, then
the one references file that matches the widget; rtwk source only for what they do not answer.

- `references/widgets.md`: the index of every widget page (148) with the group file it lives in.
- `references/widgets/<group>.md`: per widget the API (paths under `rtwk::`), the Code, the options, its methods, related types. 20 groups: text, layout, style, scrolling, data, editing, controls, overlays, animation, mouse, focus-and-keys, navigation, foundation, framework, app, screen-readers, debugging, rich, images, terminal.
- `references/guides/widgets-by-need.md`: a codex-tui need (approval dialog, composer, history list, status line, ...) to the rtwk widget.
- `references/guides/recipes.md`: seven snippets that compile and pass their tests against the pin (navigation with a result, request dialog, provider select, list, text area, channel with a ticket, screen test).
- `references/guides/state.md`, `cli.md`, `markdown.md`: rtwk's `docs/guide`, copied.
- `references/guides/app-api.md`, `testing-api.md`: the `App`/`ViewCx`/`AppCx` and `Tester` APIs from the pin's source.
