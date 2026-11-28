<!-- Written by `cargo xtask skill-references` from docs/guide; do not edit. -->
# Getting started

rtwk is a widget model for the terminal in Rust: widgets built from values,
layout by constraints, focus and keys, all measured in whole cells. Under the
facade crate `rtwk`, `rtwk-core` keeps the screen as a buffer of cells and
writes only the ones that changed. An application owns its model in a value of
its own and describes the screen as a function of it.

## Set up

rtwk needs Rust 1.90 or later. Start a project from a template, or add the
crates to one you have:

```sh
rtwk new my_app                 # the developer tool, docs/guide/cli.md
# or, in an existing package:
cargo add rtwk --git https://github.com/Maryarty/rtwk.git
cargo add rtwk-test --git https://github.com/Maryarty/rtwk.git --dev
```

With a checkout of rtwk next to the project, point the dependency at it
(`rtwk = { path = "../rtwk/crates/rtwk" }`). Every rtwk crate of one build
must come from the same source and revision. `rtwk doctor` checks this.

## A first app

A counter: Up counts, q quits. Put it in `src/main.rs`:

<!-- file: crates/rtwk/examples/guides/counter.rs -->
```rust
use std::process::ExitCode;

use rtwk::app::ViewCx;
use rtwk::focus::KeyBindings;
use rtwk::layout::{Alignment, Border, BorderTitle, BorderType, BoxWidget, EdgeInsets};
use rtwk::prelude::*;
use rtwk::style::{Span, Style};

pub fn main() -> ExitCode {
    run_app(Counter::default(), view)
}

#[derive(Default)]
pub struct Counter {
    pub n: u32,
}

pub fn view(app: &Counter, cx: &mut ViewCx<Counter>) -> AnyWidget {
    KeyBindings::new(
        [
            ("up", cx.on(|app, _| app.n += 1)),
            ("q", cx.on(|_, cx| cx.quit())),
        ],
        BoxWidget::new()
            .border(Border::all(Style::new()).kind(BorderType::Rounded))
            .title(BorderTitle::new(" up: count  q: quit ").alignment(Alignment::BOTTOM_RIGHT))
            .padding(EdgeInsets::all(1))
            .child(Text::rich([
                Span::new("Count: ").dim(),
                Span::new(app.n.to_string()).bold().fg(Color::GREEN),
            ])),
    )
    .into()
}
```

```text
╭────────────────────────────╮
│                            │
│ Count: 2                   │
│                            │
╰──────── up: count  q: quit ╯
```

- `run_app(state, view)` runs the application until it quits and returns the
  `ExitCode` `main` returns. It is full screen by default, on the terminal's
  alternate screen, which gives back what the terminal showed when the app
  ends.
- The view is a plain function of `&Counter` and a `ViewCx`. It runs again after
  each handler. `cx.on(|app, cx| ..)` makes the handler of a control: it gets
  `&mut Counter` and an `AppCx` (`cx.quit()`), and may run after the view call
  returned, so it borrows nothing from the view.
- `KeyBindings` binds keys by name (`"up"`, `"ctrl+s"`, `"g g"`) for the
  widgets below it.
- `BoxWidget` draws a border, titles on it and padding around its child (`Box`
  is std's).
- `Span::new("Count: ").dim()` and `.bold().fg(Color::GREEN)` are spans of
  styled text, which `Text::rich` puts in a row.

`rtwk::prelude::*` brings the few names almost every file uses (`Text`,
`Button`, `Column`, `Row`, `Expanded`, `Focus`, `Style`, `Color`, `run_app`,
`ViewCx`, `AnyWidget`); the rest come from the module that owns them
(`rtwk::layout`, `rtwk::controls`, `rtwk::editing`, `rtwk::overlay`,
`rtwk::style`, `rtwk::app`). The names a program of its own may have too (a
`Table`, a `Gauge`) are in modules, not in the prelude: import what you use.

## Run it

```sh
cargo run                  # full screen
rtwk run --bin my_app      # the same, with the debug aids below
```

The app runs inline, in rows below the shell's prompt, with
`RunOptions { mode: ScreenMode::Inline { at_line_start: false }, ..Default::default() }`
passed to `run_app_with`: [Screen modes](screen-modes.md) says more. Ctrl+C that
no widget takes quits.

A frame rebuilds only what a change marked, at most 60 a second, and writes
only the cells that changed: an app that changes nothing writes nothing.

## Debug it

The app owns the terminal, so `println!` would write into its screen. Use the
debug console instead (`rtwk::debug`).

```sh
rtwk run --bin my_app --debug overlay,console
```

turns the debug aids on from the start (the variable they set is `TWK_DEBUG`,
`rtwk run --help` lists them): `overlay` shows each frame's times and bytes,
`console` the log, `rainbow` tints what repaints. With any of them on, Ctrl+G and
then `o`, `c` or `r` turns each on or off while the app runs.

An error the framework reports (a panic in a handler, a failed task, a check a
debug build makes of a widget) is told to `AppView::on_error`, goes to the log
and, where it is recoverable, lets the app go on; a broken contract of the
tree ends the run with it. [Running an app](running.md) says what ends a run.

## Next

- [Layout](layout.md): rows, columns, boxes and the room each child gets.
- [Input and focus](input-and-focus.md): keys, the focus and the mouse.
- [Theming](theming.md): colors and styles from a theme.
- [Screen modes](screen-modes.md): full screen, inline and a fixed viewport.
- [State](state.md): application state, observables, providers and memo nodes.
- [Testing](testing.md): an app tested on a terminal in memory.
- [Recipes](recipes.md): a dialog, a table, a log, a bottom pane and a
  command-line tool.
- [Words in another language](localization.md): the words rtwk writes itself.
- [Coming from Flutter](coming-from-flutter.md): what is the same, what is
  spelled differently and what rtwk does not have.
- [Performance](performance.md): what costs nothing, what to narrow, and how to
  measure.
- [Details and pitfalls](details.md): how an app runs, what the tester does, the
  debug aids and the mistakes that bite.
- [The widget catalog](../widgets.md): every widget, with the Rust that
  builds it.

<!-- not rtwk: Counter -->
