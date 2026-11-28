<!-- Written by `cargo xtask skill-references` from docs/guide; do not edit. -->
# Running an app

`run_app(state, view)` opens the process's terminal, runs the app on it and
puts the terminal back. `run_app_with` takes `RunOptions` (theme, locale, screen
mode, frame rate), and `run_app_view` an `AppView` with the builders of the
host (`on_start`, `on_error`, `on_copy`, `exit_on_ctrl_c`, `stop_on_ctrl_z`).
This page is the contract: what ends the run, and what the process does in the
cases that are not a user at a terminal.

## What ends the run

- The call returns when the app quits (`cx.quit()` of an `AppCx`, or Ctrl+C
  below). It returns the `ExitCode` `main` returns: `SUCCESS`, or `FAILURE`
  after an error printed to stderr. The terminal is restored by then: raw mode,
  the alternate screen, the mouse and the cursor are as they were. The
  widgets' states are dropped.
- An error that ends the app is the `Err` of the run, after the terminal is
  restored: `run_app_view` prints it and returns `FAILURE`. A fatal
  `ErrorKind` (a broken contract of the element tree, a duplicate key) is one;
  the recoverable kinds (a message loop, a failed task, a check of a debug
  build) are told to `AppView::on_error`, go to the log, and the app goes on.
  The policy of each kind is `ErrorKind::policy()`.
- A panic in a handler or a build is a panic: the terminal is restored before
  its message is printed, so the message is readable, and the program ends as
  any Rust program does.
- Ctrl+C quits, when no widget takes the key, unless
  `AppView::exit_on_ctrl_c(false)`. A SIGINT sent to the process is the same
  key.
- Ctrl+Z stops the app for the shell until `fg`, when no widget takes the key,
  unless `stop_on_ctrl_z(false)`. The terminal is given back to the shell
  while it is stopped and taken again on `fg`. POSIX only; the shell has to do
  job control, or nothing stops. A SIGTSTP from outside (`kill -TSTP`) does the
  same.
- SIGTERM restores the terminal and exits the process with code 143, and
  SIGHUP (on Windows the console being closed) does the same with 129. The
  call does not return: the process is gone.

## When there is no terminal

An app needs both stdin and stdout to be a terminal, and falls back where it
is not.

- stdout is not a terminal (piped, or redirected to a file), or the
  environment says CI, or `TWK_PLAIN=1`: the app runs on plain output. No
  capability probe is sent and no mode or escape sequence is written; the app
  draws at a fixed size (`COLUMNS` by `LINES`, else 80 by 24). Stdin is read
  only where it is a terminal. Ctrl+C and SIGTERM end the app as above.
  `TWK_PLAIN=0` turns the fallback off, and the refusal below is what follows.
- stdout is a terminal and stdin is not (a file, a pipe, `/dev/null`, a tool
  that gives the program none): there is no input to read, and the run ends
  with an error saying so and what to do. Run the app in a terminal; or set
  `TWK_PLAIN=1` to have it write plain text; or start `rtwk shell` in the
  project, or point `TWK_SHELL` at one, and the app takes that shell's
  terminal.

An app started from an editor or an IDE that gives it no terminal is the last
case, and the shell is what the editor starts for it.

## Work that outlives a view

Work started from a handler with `AppCx::spawn_tokio` or `subscribe`
(`rtwk-tokio`) belongs to the element that started it: unmounting aborts it.
Work for the life of the program goes on the runtime directly and `main` owns
its shutdown, once the run has returned:

<!-- file: crates/rtwk/examples/guides/backend.rs -->
```rust
use std::process::ExitCode;

use rtwk::app::{AppView, RunOptions, ViewCx, run_app_view};
use rtwk::prelude::*;
use rtwk::style::Brightness;

#[derive(Default)]
pub struct Session {
    pub answer: Option<u32>,
}

pub fn view(app: &Session, cx: &mut ViewCx<Session>) -> AnyWidget {
    let text = app
        .answer
        .map_or_else(|| "waiting".to_owned(), |n| format!("answer: {n}"));
    Focus::new(Text::new(text))
        .autofocus()
        .bind('q', cx.on(|_, cx| cx.quit()))
        .into()
}

pub fn main() -> Result<ExitCode, Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Runtime::new()?;
    // the runtime a handler's `spawn_tokio` and `subscribe` use
    let _entered = rtwk_tokio::enter(&runtime);
    // the backend lives until main says so, not until a view unmounts
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let backend = runtime.spawn(async move {
        let _ = stopped.await;
        // client.shutdown().await
    });
    let app = AppView::new(Session::default(), view).on_start(|app, _| app.answer = Some(42));
    let options = RunOptions {
        theme: Some(ThemeData::terminal(Brightness::Dark)),
        ..RunOptions::default()
    };
    let code = run_app_view(app, options);
    let _ = stop.send(());
    let _ = runtime.block_on(backend);
    Ok(code)
}
```

`run_app_view` places the standard root itself (the overlay, the localizations
and the graphics scope), takes a theme and a locale in its `RunOptions`, and
keeps the builders of the `AppView` it is given. Do not copy the body of the
runner to get them.

## Testing the run

A test does not open a terminal: `Tester` runs the same `AppView` on one in
memory, as [Testing](testing.md) shows, and wraps it in the root a process
gives it with `pump_app_in`. To see an app in a real terminal without a test,
or to record a session as a regression test, there are `rtwk record` and `rtwk
tape` ([the command line](cli.md)). What the process does with real signals
and a real tty is covered by running a built app in a pseudo-terminal, which
the repository's own end-to-end checks do.
