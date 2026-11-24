<!-- Written by `cargo xtask skill-references` from docs/widgets; do not edit. -->
# App

The app around it all: the screen modes, a fixed viewport, output kept in the scrollback, the window title, notifications and a command-line tool's console.

[All widgets](../widgets.md)

A Code is the body of the gallery's preview: it names widgets by the last segment of the paths under API (`use` them) and may use `cx` or values of the gallery's page.

## InputListener

The input no widget took: keys nothing is bound to, the mouse where no Listener is, pastes, focus and resizes.

- `rtwk::binding::InputListener` (struct) in `crates/rtwk-widgets/src/binding/app_binding.rs:1648`: Gives its callback the input events of the app it is in while it is mounted, wherever it is in the tree: the keys that no focus node took (AppBinding::dispatch), the mouse events that no `Listener` took, pastes, focus and resizes.

### Code

```rust
use rtwk::binding::InputListener;
use rtwk::input::InputEvent;
use rtwk::widgets::Text;

pub fn build(on_input: impl Fn(String) + 'static) -> InputListener {
    InputListener::new(
        Text::new("Press a key nothing takes, such as x"),
        move |event| {
            match event {
                InputEvent::Key(_)
                | InputEvent::Mouse(_)
                | InputEvent::Paste(_)
                | InputEvent::Focus(_)
                | InputEvent::Resize { .. } => on_input(event.to_string()),
                _ => {}
            }
            false
        },
    )
}

```

## ErrorWidget

An error shown in place of what failed to build, as the app does by itself.

- `rtwk::binding::ErrorWidget` (struct) in `crates/rtwk-widgets/src/binding/error_widget.rs:46`: A red block with the message of an error (RenderErrorBox), which a widget shows for an error it caught.

### Code

```rust
use rtwk::binding::ErrorWidget;
use rtwk::text::Text;
use rtwk::{AnyWidget, column};

pub fn build() -> AnyWidget {
    let fails = true;
    let error = "Bad state: the weather failed to load";
    let weather = if fails { Err(error) } else { Ok("light rain") };
    match weather {
        Ok(forecast) => column![
            Text::new("12°C"),
            Text::new(forecast),
            Text::new("wind 14 km/h")
        ]
        .into(),
        Err(error) => ErrorWidget::new(error).into(),
    }
}

```

### Options

| option | values | starts at |
| --- | --- | --- |
| weather fails | false, true | true |
| what it throws | StateError, FormatException, Exception | StateError |

## Static

Output that is finished once shown: in place here; in the inline mode above the app, in the terminal's history.

- `rtwk::binding::Static` (struct) in `crates/rtwk-widgets/src/binding/statics.rs:59`: Shows each of its items once, as output that is finished: finished messages, the lines of a log, the results of the steps of a task.

### Code

```rust
use rtwk::app::TerminalOptions;
use rtwk::binding::Static;
use rtwk::text::Text;

pub fn terminal_options() -> TerminalOptions {
    let inline = true;
    TerminalOptions {
        alternate_screen: !inline,
        ..TerminalOptions::default()
    }
}

pub fn build(finished: Vec<&'static str>) -> Static<&'static str> {
    Static::new(finished, |_, step| Text::new(format!("✓ {step}")).into())
}

```

### Options

| option | values | starts at |
| --- | --- | --- |
| mode | inline, fullscreen | inline |

## TerminalProgress

The program's progress in the terminal's tab or taskbar, where the terminal shows one (Ghostty, kitty, Windows Terminal, ConEmu).

- `rtwk::binding::TerminalProgress` (struct) in `crates/rtwk-widgets/src/binding/terminal_status.rs:374`: Shows the program's progress in the terminal's tab or taskbar (OSC 9;4) while it is in the tree, where the terminal is known to show one (TerminalStatus): as far as value says, as state says.

### Code

```rust
use rtwk::binding::{ProgressState, TerminalProgress};
use rtwk::text::Text;

pub fn build() -> TerminalProgress {
    let value = Some(0.33);
    let mut progress = TerminalProgress::new(Text::new("Building…")).state(ProgressState::Normal);
    if let Some(value) = value {
        progress = progress.value(value);
    }
    progress
}

```

### Options

| option | values | starts at |
| --- | --- | --- |
| state | none, normal, error, indeterminate, paused | normal |
| value known | false, true | true |
| value | 0%, 1%, 2%, 3%, 4%, 5%, 6%, 7%, 8%, 9%, 10%, 11%, 12%, 13%, 14%, 15%, 16%, 17%, 18%, 19%, 20%, 21%, 22%, 23%, 24%, 25%, 26%, 27%, 28%, 29%, 30%, 31%, 32%, 33%, 34%, 35%, 36%, 37%, 38%, 39%, 40%, 41%, 42%, 43%, 44%, 45%, 46%, 47%, 48%, 49%, 50%, 51%, 52%, 53%, 54%, 55%, 56%, 57%, 58%, 59%, 60%, 61%, 62%, 63%, 64%, 65%, 66%, 67%, 68%, 69%, 70%, 71%, 72%, 73%, 74%, 75%, 76%, 77%, 78%, 79%, 80%, 81%, 82%, 83%, 84%, 85%, 86%, 87%, 88%, 89%, 90%, 91%, 92%, 93%, 94%, 95%, 96%, 97%, 98%, 99%, 100% | 33% |
| terminal shows it | false, true | true |

## Viewport

The app in a fixed rectangle of the screen (Viewport::fixed): nothing outside it is written, so what the terminal showed around it stays.

- `rtwk::app::Viewport` (struct) in `crates/rtwk-core/src/presenter/modes.rs:23`: A rectangle of the terminal's screen that an app draws in and owns: nothing outside it is written, erased or moved (ScreenMode::Fixed).
- `rtwk::app::ScreenMode` (enum) in `crates/rtwk-core/src/presenter/modes.rs:63`: Which part of the terminal's screen a Presenter draws on.

### Code

```rust
use rtwk::app::Viewport;
use rtwk::column;
use rtwk::prelude::*;

fn main() -> std::process::ExitCode {
    let options = RunOptions {
        mode: ScreenMode::Fixed(Viewport::fixed(20, 0, 20, 4)),
        terminal: TerminalOptions {
            alternate_screen: false,
            ..TerminalOptions::default()
        },
        ..RunOptions::default()
    };
    run_app_with(
        (),
        |(), cx| {
            column![
                Text::new("Build viewport"),
                Button::new("Done").on_pressed(cx.on(|(), cx| cx.quit()))
            ]
            .into()
        },
        options,
    )
}

```

### Options

| option | values | starts at |
| --- | --- | --- |
| left | 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20 | 20 |
| top | 0, 1, 2, 3, 4 | 0 |
| mode | inline, fullscreen | inline |

## Window title

The title of the terminal's window or tab while the app runs; the user's own comes back when it ends.

- `rtwk::binding::TerminalTitle` (struct) in `crates/rtwk-widgets/src/binding/terminal_status.rs:494`: Sets the title of the terminal's window or tab to its title while it is in the tree, as Flutter's `Title` does the app's in the system, and again whenever the title changes (TerminalStatus::title): through the backend's WindowBackend::set_title (a real terminal saves the user's title the first time and gives it back when the app ends).

### Code

```rust
use rtwk::binding::TerminalTitle;
use rtwk::prelude::*;

fn view(_: &(), _: &mut ViewCx<()>) -> AnyWidget {
    TerminalTitle::new("rtwk gallery", Text::new("In the title bar: rtwk gallery")).into()
}

```

### Options

| option | values | starts at |
| --- | --- | --- |
| title | rtwk gallery, build: 3 of 5, chat · ~/project | rtwk gallery |
| app running | false, true | true |

## Notifications

A desktop notification of a title and a body, where the terminal shows one (OSC 9, 777 or 99); nothing where it does not.

- `rtwk::binding::AppBinding` (struct) in `crates/rtwk-widgets/src/binding/app_binding.rs:344`: The app a widget runs in (twk's `AppBinding`): one per running app, found from a build by maybe_of.

### Code

```rust
use rtwk::binding::BindingScope;
use rtwk::column;
use rtwk::layout::{CrossAxisAlignment, MainAxisSize};
use rtwk::prelude::*;

fn view(count: &usize, cx: &mut ViewCx<usize>) -> AnyWidget {
    let binding = cx.depend::<BindingScope, _>(|scope| scope.binding.clone());
    let notify = cx.on(move |count, _| {
        if let Some(binding) = &binding {
            binding.notify("Build finished", "ran 214 tests in 3.2 s");
        }
        *count += 1;
    });
    let status = if *count == 0 {
        "nothing".to_owned()
    } else {
        count.to_string()
    };
    column![
        Button::new("Notify").on_pressed(notify),
        Text::new(format!("notified: {status}")),
    ]
    .main_axis_size(MainAxisSize::Min)
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .spacing(1)
    .into()
}

```

### Options

| option | values | starts at |
| --- | --- | --- |
| terminal | kitty, Ghostty, iTerm2, xterm | kitty |

## Cli

A command-line tool's console (the Cli widget) as it runs: questions that leave their answer as a line, a spinner, tasks with bars, a traceback.

- `rtwk::cli::Cli` (struct) in `crates/rtwk-cli/src/cli.rs:127`: The console of a command-line tool: messages by channel and level, styled as the terminal takes them, and any rtwk widget shown in the output (show).

### Code

```rust
use rtwk::cli::{Cli, PromptException};
use rtwk::style::Span;

pub(crate) fn create(
    cli: &Cli,
    mut type_at_prompt: impl FnMut(usize),
) -> Result<(), PromptException> {
    let name = cli
        .prompt("Project name?")
        .default_value("my_app")
        .validate(|name| {
            let mut bytes = name.bytes();
            let valid = bytes.next().is_some_and(|byte| byte.is_ascii_lowercase())
                && bytes
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_');
            (!valid).then(|| "lowercase letters, digits and _".to_owned())
        })
        .start();
    type_at_prompt(0);
    let name = name.wait()?;
    let template = cli
        .choose_one("Template?", vec!["console", "package", "server", "web"])
        .start();
    type_at_prompt(1);
    let template = template.wait()?;
    let features = cli
        .choose_any("Features?", vec!["Linter", "Docker", "Tests", "CI"])
        .default_values(["Linter"])
        .start();
    type_at_prompt(2);
    let features = features.wait()?;
    let confirm = cli.confirm(format!("Create {name}?")).start();
    type_at_prompt(3);
    if confirm.wait()? {
        cli.success(vec![
            Span::new("Created ").bold(),
            Span::new(name),
            Span::new(" from "),
            Span::new(template).italic(),
            Span::new(format!(" with {}", features.join(", "))),
        ]);
    } else {
        cli.info("Nothing created.");
    }
    Ok(())
}

```

### Options

| option | values | starts at |
| --- | --- | --- |
| program | create, install, check | create |
