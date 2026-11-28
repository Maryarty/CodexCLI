<!-- Written by `cargo xtask skill-references` from docs/guide; do not edit. -->
# Screen modes

The same widgets run in three places on the terminal. `RunOptions::mode`
(`rtwk::app::RunOptions`, passed to `run_app_with` or `run_app_view`) picks one;
`run_app` is the first.

| mode | how | for |
| --- | --- | --- |
| full screen | `run_app(state, view)` | an app that is its own screen: an editor, a monitor, a file manager |
| inline | `RunOptions { mode: ScreenMode::Inline { at_line_start: false }, .. }` | a prompt, a chat, a progress view that stays in the shell's history |
| fixed viewport | `RunOptions { mode: ScreenMode::Fixed(Viewport::fixed(left, top, width, height)), .. }` | a panel in a rectangle of the screen, the rest left alone |

## Full screen

The app takes the terminal's alternate screen, all of it, at its size; it
follows resizes, and what the terminal showed before comes back when the app
ends. Full screen and a fixed viewport get the mouse; inline leaves it to the
terminal, whose selection and scrollback keep working.

## Inline

The app takes rows of the normal screen from the prompt down, as many as it
asks for (its height when unconstrained), and grows and shrinks with them.
`at_line_start: false` starts the region on the line after the cursor's, `true`
on the cursor's own line when the cursor is at the start of one. When the app
ends, its rows stay in the terminal's history with the shell's output
(the `clear_inline_on_exit` of `RunOptions` erases them instead).

<!-- file: crates/rtwk/examples/guides/inline.rs -->
```rust
use std::process::ExitCode;
use std::rc::Rc;

use rtwk::app::{RunOptions, ScreenMode, ViewCx, run_app_with};
use rtwk::binding::Static;
use rtwk::editing::TextEditingController;
use rtwk::layout::CrossAxisAlignment;
use rtwk::prelude::*;

pub fn main() -> ExitCode {
    let options = RunOptions {
        mode: ScreenMode::Inline {
            at_line_start: false,
        },
        ..RunOptions::default()
    };
    run_app_with(Prompt::default(), view, options)
}

pub struct Prompt {
    pub sent: Vec<String>,
    pub text: TextEditingController,
}

impl Default for Prompt {
    fn default() -> Prompt {
        Prompt {
            sent: Vec::new(),
            text: TextEditingController::new(""),
        }
    }
}

pub fn view(app: &Prompt, cx: &mut ViewCx<Prompt>) -> AnyWidget {
    let send = cx.on_with(|app, line: Rc<str>, _| {
        app.sent.push(line.to_string());
        app.text.clear();
    });
    Column::new()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        // written once each, into the scrollback; only ever appended to
        .child(Static::new(app.sent.clone(), |_, line: &String| {
            Text::new(format!("> {line}")).into()
        }))
        .child(
            TextField::new()
                .controller(app.text.clone())
                .placeholder("say something, Enter sends")
                .on_submitted(send)
                .autofocus(true),
        )
        .into()
}
```

`Static::new(items, builder)` (`rtwk::binding::Static`) is output that is
finished once shown: inline, each new item is written once into the terminal's
scrollback above the app's rows and takes no rows of them. Here only the field
stays in the app's rows, however much was said. Items are told apart by their
index, so a `Static`'s list is only appended to. Full screen, the same items
show in place, one below another.

Inline, a list asks for no rows: give it a height
(`SizedBox::new(None, Some(16), Some(list))`).

A test runs an inline app with `Options::new().mode(ScreenMode::Inline)`
(`rtwk::binding::ScreenMode`, the tester's own two modes) and reads what went to
the scrollback with `Tester::scrollback()`; [Testing](testing.md) has it.

## Where the output is no terminal

In a pipe or a CI log, the app writes no escape sequences, at a fixed size
(`COLUMNS` by `LINES`, else 80 by 24); [Running an app](running.md) says when
that is and what else ends the run.

`Static` and a fixed `Viewport` have their pages in the catalog's
[App](../widgets/app.md) group. For a command-line tool that prints lines, asks a
question or two and shows progress, and must work just as well with nobody at
the keyboard, there is `rtwk::cli` and the
[recipe](recipes.md#a-command-line-tool-rtwkcli-not-run_app) that uses it.
