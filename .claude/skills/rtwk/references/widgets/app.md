<!-- Written by tools/rtwk-port/skill/generate.py from rtwk 7482e340; do not edit. -->

# App

[All widgets](../widgets.md)

## InputListener

The input no widget took: keys nothing is bound to, the mouse where no Listener is, pastes, focus and resizes.

API:

- `rtwk::binding::InputListener` (struct, `crates/rtwk-widgets/src/binding/app_binding.rs:1556`) Gives its callback the input events of the app it is in while it is mounted, wherever it is in the tree: the keys that no focus node took (`AppBinding::dispatch`), the mouse events that no `Listener` took, pastes, focus ...
  - public fields: `child: AnyWidget`

### Code

The gallery's Code tab at the initial options (route `#/app/inputlistener`, `crates/rtwk-gallery/src/pages/app_host.rs:57`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
use rtwk::binding::InputListener;
use rtwk::widgets::Text;
use rtwk_core::input::InputEvent;

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

### Methods

#### `InputListener`: constructors and associated functions

- `pub fn new(child: impl Into<AnyWidget>, on_event: impl Fn(&InputEvent<'_>) -> bool + 'static) -> InputListener` - Gives `on_event` the events of the app; shows `child`.
- `pub fn with_raw_key_repeats(child: impl Into<AnyWidget>, on_event: impl Fn(&InputEvent<'_>) -> bool + 'static) -> InputListener` - Like `new`, and offers this listener unmodified key repeats before the focus path.

### Also used in the code

- `rtwk_core::input::InputEvent` (enum, `crates/rtwk-input/src/events.rs:681`) One thing a terminal sent: what the user did (a key, the mouse, a paste, focus), the terminal's new size, a reply to a query, or a sequence nothing here maps (`Unknown`).
  - variants: `Key`, `Mouse`, `Paste`, `Focus`, `Resize`, `Mode`, `KeyboardFlags`, `DeviceAttributes`, `Color`, `ColorScheme`, `CursorReport`, `Unknown`
- `rtwk::focus::Focus` (struct, `crates/rtwk-framework/src/focus/widgets.rs:85`) Puts a `FocusNode` in the focus tree for its child, under the node of the nearest `Focus` or `FocusScope` above it: the `focus_node` given, or one of its own.
  - public fields: `id: Option<FocusId>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `on_focus_change: Option<Handler<bool>>`, `on_key_event: Option<FocusOnKeyEventCallback>`, `can_request_focus: Option<bool>`, `skip_traversal: Option<bool>`, `descendants_are_focusable: Option<bool>`, `takes_text: Option<bool>`, `debug_label: Option<Str>`, `on_key: Option<KeyHandler>`, `bindings: Vec<(KeyPress`, `reads_layout: bool`, `child: AnyWidget`

## ErrorWidget

An error shown in place of what failed to build, as the app does by itself.

API:

- `rtwk::binding::ErrorWidget` (struct, `crates/rtwk-widgets/src/binding/error_widget.rs:46`) A red block with the message of an error (`RenderErrorBox`), which a widget shows for an error it caught.
  - public fields: `message: String`

### Code

The gallery's Code tab at the initial options (route `#/app/errorwidget`, `crates/rtwk-gallery/src/pages/app_host.rs:70`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

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

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| fails | on, off | true |
| error | StateError, FormatException, Exception | StateError |

### Methods

#### `ErrorWidget`: constructors and associated functions

- `pub fn new(error: impl fmt::Display) -> ErrorWidget` - Shows `error`'s message.

#### `ErrorWidget`: methods

- `pub fn message(&self) -> &str` - What the block says.

## Static

Output that is finished once shown: in place here; in the inline mode above the app, in the terminal's history.

API:

- `rtwk_core::terminal::TerminalOptions` (struct, `crates/rtwk-core/src/terminal/info.rs:333`) How `Terminal` opens a terminal: the screen it draws on, the input it asks for, how long it waits for answers.
  - public fields: `alternate_screen: bool`, `mouse: bool`, `mouse_motion: bool`, `normal_screen_mouse: bool`, `bracketed_paste: bool`, `focus_events: bool`, `kitty_keyboard: bool`, `kitty_keyboard_flags: i64`, `probe_timeout: Duration`, `escape_timeout: Duration`, `paste_timeout: Duration`, `graphics: Option<GraphicsProtocol>`, `plain: Option<bool>`, `bidi: Option<BidiReordering>`
- `rtwk::binding::Static` (struct, `crates/rtwk-widgets/src/binding/statics.rs:59`) Shows each of its items once, as output that is finished: finished messages, the lines of a log, the results of the steps of a task.
  - public fields: `items: Rc<[T]>`

### Code

The gallery's Code tab at the initial options (route `#/app/static`, `crates/rtwk-gallery/src/pages/app_host.rs:94`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
use rtwk::binding::Static;
use rtwk::text::Text;
use rtwk_core::terminal::TerminalOptions;

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

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| mode | inline, fullscreen | inline |

### Methods

#### `TerminalOptions`: methods

- `pub const fn kitty_flags(&self) -> i64` - The kitty keyboard flags pushed: those asked for, always with disambiguation.

#### `Static`: constructors and associated functions

- `pub fn new(items: impl Into<Rc<[T]>>, builder: impl Fn(&mut Cx<'_>, &T) -> AnyWidget + 'static) -> Static<T>` - Shows each of `items` once, as `builder` builds it.

## TerminalProgress

The program's progress in the terminal's tab or taskbar, where the terminal shows one (Ghostty, kitty, Windows Terminal, ConEmu).

API:

- `rtwk::binding::TerminalProgress` (struct, `crates/rtwk-widgets/src/binding/terminal_status.rs:374`) Shows the program's progress in the terminal's tab or taskbar (OSC 9;4) while it is in the tree, where the terminal is known to show one (`TerminalStatus`): as far as `value` says, as `state` says.
  - public fields: `state: ProgressState`, `value: Option<f64>`, `child: AnyWidget`

### Code

The gallery's Code tab at the initial options (route `#/app/terminalprogress`, `crates/rtwk-gallery/src/pages/app_host.rs:107`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
use rtwk::binding::TerminalProgress;
use rtwk::text::Text;
use rtwk_core::terminal::ProgressState;

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

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| state | none, normal, error, indeterminate, paused | normal |
| known | on, off | true |
| value | 0..100 (a slider) | 33% |
| shows | on, off | true |

### Methods

#### `TerminalProgress`: constructors and associated functions

- `pub fn new(child: impl Into<AnyWidget>) -> TerminalProgress` - Shows `ProgressState::Normal` progress while `child` is in the tree.

#### `TerminalProgress`: builder methods (take and return `self`)

- `pub fn state(mut self, state: ProgressState) -> TerminalProgress` - This with `state`.
- `pub fn value(mut self, value: f64) -> TerminalProgress` - This with `value`, from 0 to 1.

### Also used in the code

- `rtwk_core::terminal::ProgressState` (enum, `crates/rtwk-core/src/terminal/sequences.rs:579`) What a program's progress in the terminal's tab or taskbar shows (`progress_sequence`).
  - variants: `None`, `Normal`, `Error`, `Indeterminate`, `Paused`

## Viewport

The app in a fixed rectangle of the screen (Viewport.fixed): nothing outside it is written, so what the terminal showed around it stays.

API:

- `rtwk::app::Viewport` (struct, `crates/rtwk-core/src/presenter/modes.rs:22`) A rectangle of the terminal's screen that an app draws in and owns: nothing outside it is written, erased or moved (`ScreenMode::Fixed`).
  - public fields: `left: usize`, `top: usize`, `width: usize`, `height: usize`
- `rtwk_core::terminal::TerminalOptions` (struct, `crates/rtwk-core/src/terminal/info.rs:333`) How `Terminal` opens a terminal: the screen it draws on, the input it asks for, how long it waits for answers.
  - public fields: `alternate_screen: bool`, `mouse: bool`, `mouse_motion: bool`, `normal_screen_mouse: bool`, `bracketed_paste: bool`, `focus_events: bool`, `kitty_keyboard: bool`, `kitty_keyboard_flags: i64`, `probe_timeout: Duration`, `escape_timeout: Duration`, `paste_timeout: Duration`, `graphics: Option<GraphicsProtocol>`, `plain: Option<bool>`, `bidi: Option<BidiReordering>`
- `rtwk::controls::Button` (struct, `crates/rtwk-controls/src/button.rs:107`) A button that tells `on_pressed` when it is clicked, or when Enter or Space is pressed while it has the focus.
  - public fields: `spans: Option<Rc<[Span]>>`, `tall: bool`, `on_pressed: Option<Handler<()>>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: bool`, `style: Option<Style>`, `focused_style: Option<Style>`, `hover_style: Option<Style>`, `disabled_style: Option<Style>`

### Code

The gallery's Code tab at the initial options (route `#/app/viewport`, `crates/rtwk-gallery/src/pages/app_terminal.rs:67`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

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

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| left | 0..20 (a slider) | 20 |
| top | 0..4 (a slider) | 0 |
| mode | inline, fullscreen | inline |

### Methods

#### `Viewport`: constructors and associated functions

- `pub const fn fixed(left: usize, top: usize, width: usize, height: usize) -> Self` - The `width` × `height` cells from column `left` of row `top`, counted from 0 at the screen's top left.

#### `TerminalOptions`: methods

- `pub const fn kitty_flags(&self) -> i64` - The kitty keyboard flags pushed: those asked for, always with disambiguation.

#### `Button`: constructors and associated functions

- `pub fn new(label: impl Into<Str>) -> Button` - A button showing `label`; disabled until `on_pressed` is set.
- `pub fn outlined(label: impl Into<Str>) -> Button` - A button of `label` between two edges, or with `tall` in a box drawn around it: in `ThemeData::border`, or `style`'s text color, at rest.
- `pub fn filled(label: impl Into<Str>) -> Button` - A button of `label` on a fill of `ThemeData::primary`, or of `style`'s text color, at rest; with `tall` three rows of it.
- `pub fn rich(spans: impl Into<Rc<[Span]>>) -> Button` - A chip of `spans`, disabled until `on_pressed` is set: the key and its caption of a button, each in its own `Span` style, on one chip with the hover, the focus and the flash of `Button`.

#### `Button`: builder methods (take and return `self`)

- `pub fn on_pressed(mut self, h: Handler<()>) -> Button` - Told when it is pressed.
- `pub fn focus_node(mut self, node: Option<FocusNode>) -> Button` - With `node` as its node of the focus tree.
- `pub fn autofocus(mut self, on: bool) -> Button` - Taking the focus when mounted.
- `pub fn can_request_focus(mut self, on: bool) -> Button` - Whether it can take the focus.
- `pub fn tall(mut self, on: bool) -> Button` - A box or a fill around the label, three rows.
- `pub fn style(mut self, style: Style) -> Button` - At rest in `style`.
- `pub fn focused_style(mut self, style: Style) -> Button` - While focused in `style` over the one at rest.
- `pub fn hover_style(mut self, style: Style) -> Button` - While the mouse is over it in `style` over the one at rest.
- `pub fn disabled_style(mut self, style: Style) -> Button` - While disabled in `style` over the one at rest.

#### `Button`: methods

- `pub fn label(&self) -> Str` - The text it shows, with a blank on either side; what a screen reader reads.

## Window title

The title of the terminal's window or tab while the app runs; the user's own comes back when it ends.

API:

- `rtwk::binding::TerminalTitle` (struct, `crates/rtwk-widgets/src/binding/terminal_status.rs:494`) Sets the title of the terminal's window or tab to its title while it is in the tree, as Flutter's `Title` does the app's in the system, and again whenever the title changes (`TerminalStatus::title`): through the ...
  - public fields: `title: String`, `child: AnyWidget`

### Code

The gallery's Code tab at the initial options (route `#/app/window-title`, `crates/rtwk-gallery/src/pages/app_terminal.rs:89`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
use rtwk::binding::TerminalTitle;
use rtwk::prelude::*;

fn view(_: &(), _: &mut ViewCx<()>) -> AnyWidget {
    TerminalTitle::new("twk gallery", Text::new("In the title bar: twk gallery")).into()
}
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| title | twk gallery, build: 3 of 5, chat · ~/project | twk gallery |
| running | on, off | true |

### Methods

#### `TerminalTitle`: constructors and associated functions

- `pub fn new(title: impl Into<String>, child: impl Into<AnyWidget>) -> TerminalTitle` - Sets the title to `title` while `child` is in the tree.

## Notifications

A desktop notification of a title and a body, where the terminal shows one (OSC 9, 777 or 99); nothing where it does not.

API:

- `rtwk::controls::Button` (struct, `crates/rtwk-controls/src/button.rs:107`) A button that tells `on_pressed` when it is clicked, or when Enter or Space is pressed while it has the focus.
  - public fields: `spans: Option<Rc<[Span]>>`, `tall: bool`, `on_pressed: Option<Handler<()>>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: bool`, `style: Option<Style>`, `focused_style: Option<Style>`, `hover_style: Option<Style>`, `disabled_style: Option<Style>`

### Code

The gallery's Code tab at the initial options (route `#/app/notifications`, `crates/rtwk-gallery/src/pages/app_terminal.rs:100`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

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

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| terminal | kitty, Ghostty, iTerm2, xterm | kitty |

### Methods

#### `Button`: constructors and associated functions

- `pub fn new(label: impl Into<Str>) -> Button` - A button showing `label`; disabled until `on_pressed` is set.
- `pub fn outlined(label: impl Into<Str>) -> Button` - A button of `label` between two edges, or with `tall` in a box drawn around it: in `ThemeData::border`, or `style`'s text color, at rest.
- `pub fn filled(label: impl Into<Str>) -> Button` - A button of `label` on a fill of `ThemeData::primary`, or of `style`'s text color, at rest; with `tall` three rows of it.
- `pub fn rich(spans: impl Into<Rc<[Span]>>) -> Button` - A chip of `spans`, disabled until `on_pressed` is set: the key and its caption of a button, each in its own `Span` style, on one chip with the hover, the focus and the flash of `Button`.

#### `Button`: builder methods (take and return `self`)

- `pub fn on_pressed(mut self, h: Handler<()>) -> Button` - Told when it is pressed.
- `pub fn focus_node(mut self, node: Option<FocusNode>) -> Button` - With `node` as its node of the focus tree.
- `pub fn autofocus(mut self, on: bool) -> Button` - Taking the focus when mounted.
- `pub fn can_request_focus(mut self, on: bool) -> Button` - Whether it can take the focus.
- `pub fn tall(mut self, on: bool) -> Button` - A box or a fill around the label, three rows.
- `pub fn style(mut self, style: Style) -> Button` - At rest in `style`.
- `pub fn focused_style(mut self, style: Style) -> Button` - While focused in `style` over the one at rest.
- `pub fn hover_style(mut self, style: Style) -> Button` - While the mouse is over it in `style` over the one at rest.
- `pub fn disabled_style(mut self, style: Style) -> Button` - While disabled in `style` over the one at rest.

#### `Button`: methods

- `pub fn label(&self) -> Str` - The text it shows, with a blank on either side; what a screen reader reads.

## Cli

A command-line tool's console (twk's cli.dart) as it runs: questions that leave their answer as a line, a spinner, tasks with bars, a traceback.

API:

- `rtwk::cli::Cli` (struct, `crates/rtwk-cli/src/cli.rs:127`) The console of a command-line tool: messages by channel and level, styled as the terminal takes them, and any rtwk widget shown in the output (`show`).

### Code

The gallery's Code tab at the initial options (route `#/app/cli`, `crates/rtwk-gallery/src/pages/app_terminal.rs:555`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

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

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| program | create, install, check | create |

### Methods

#### `Cli`: constructors and associated functions

- `pub fn new(terminal: Rc<dyn CliTerminal>) -> Cli` - A console on `terminal` (see `SystemTerminal` for the process's stdio, `FakeCliTerminal` for a test's), showing what `LogLevel::Info` lets through, in the terminal's own theme, its built-in strings the environment's ...
- `pub fn run<T, E: Into<Box<dyn Error + Send + Sync>>>(options: RunOptions, body: impl FnOnce(&Cli) -> Result<T, E>) -> Result<T, RunError>` - Runs `body` with a console set up from a command line's flags, which is `close`d when it ends, by an error too.
- `pub fn run_with_args<T, E: Into<Box<dyn Error + Send + Sync>>>(args: impl IntoIterator<Item = impl Into<String>>, mut options: RunOptions, body: impl FnOnce(&Cli, Vec<String>) -> Result<T, E>) -> Result<T, RunError>` - `run` with its flags taken off `args`: `-v`/`--verbose`, `-q`/`--quiet`, `--no-color` and `--no-input`, wherever they are before a `--`; the rest, in order and with what follows `--` as it is, goes to `body`.

#### `Cli`: builder methods (take and return `self`)

- `pub fn with_source_lines(self, source_lines: SourceLines) -> Cli` - Formats source excerpts in errors that carry a traceback.
- `pub fn with_level(self, level: LogLevel) -> Cli` - The same, showing what `level` lets through.
- `pub fn with_theme(self, theme: CliTheme) -> Cli` - The same, in `theme`.
- `pub fn with_strings(self, strings: &'static WidgetStrings) -> Cli` - The same, its prompts and progress speaking `strings` (one of `tables`).
- `pub fn with_overflow_marker(self, on: bool) -> Cli` - The same, its live region drawing the marker of an overflow (`true`) or none (`false`); a debug build's draws it without this call, a release build's never (`SessionOptions::overflow_marker`).

#### `Cli`: methods

- `pub fn source_lines(&self) -> Option<SourceLines>` - The console's custom source excerpt formatter, if configured.
- `pub fn terminal(&self) -> &Rc<dyn CliTerminal>` - The terminal output goes to and input comes from.
- `pub fn mode(&self) -> CliMode` - How the console talks to the terminal (`CliTerminal::mode`).
- `pub fn level(&self) -> LogLevel` - The threshold a channel is written at or above.
- `pub fn set_level(&self, level: LogLevel)` - Sets the threshold, for a `--verbose` that comes late.
- `pub fn theme(&self) -> CliTheme` - The style of each channel, and what widgets are shown in.
- `pub fn set_theme(&self, theme: CliTheme)` - Sets the theme.
- `pub fn strings(&self) -> &'static WidgetStrings` - The table the console's prompts and progress read their words from.
- `pub fn styled_underlines(&self) -> bool` - Whether the lines written show the underline styles: as the inline session's probe found once one was opened, the environment's guess (`CliCapabilities::styled_underlines`) before.
- `pub fn detail(&self, message: impl Into<Message>)` - Logs `message` at `LogLevel::Debug`, to the output, in the theme's muted `detail`.
- `pub fn info(&self, message: impl Into<Message>)` - Logs `message` at `LogLevel::Info`, to the output, unstyled.
- `pub fn success(&self, message: impl Into<Message>)` - Logs `message` at `LogLevel::Info`, to the output, in the theme's `success`.
- `pub fn warn(&self, message: impl Into<Message>)` - Logs `message` at `LogLevel::Warning`, to the error output, in the theme's `warning`: a warning is a diagnostic, not a result.
- `pub fn err(&self, message: impl Into<Message>)` - Logs `message` at `LogLevel::Error`, to the error output, in the theme's `error`.
- `pub fn alert(&self, message: impl Into<Message>)` - Logs `message` as an alert, to the error output, in the theme's `alert` (bold white on red): critical, shown wherever `err` is.
- `pub fn write(&self, text: &str)` - Writes `text` to the output as it is: no level, no style, no line break.
- `pub fn delayed(&self, message: impl Into<Message>)` - Keeps `message` until `flush`: for what should come after the rest of the output, such as a summary of warnings.
- `pub fn flush(&self)` - Writes the messages `delayed` kept, in order, through `info`, and forgets them.
- `pub fn link(&self, uri: &str, text: Option<&str>) -> String` - `text`, or `uri` itself, as a hyperlink to `uri` (OSC 8) where the terminal takes them, else the label alone; a message reads it back as a span with that link.
- `pub fn show(&self, widget: impl Into<AnyWidget>)` - Shows `widget` once, laid out at the terminal's width, as tall as it is there; returns once it is written.
- `pub fn show_at(&self, widget: impl Into<AnyWidget>, width: Option<usize>)` - `show` `width` columns wide where given.
- `pub fn write_line(&self, line: &[Span])` - Writes `line` to the output, with escapes where the terminal takes them, whatever the level: what prompts and progress write outside the inline session.
- `pub fn session(&self) -> io::Result<CliSession>` - The inline session of the interactive mode, opened the first time it is asked for and kept until `close`: what the prompts and progress of this package mount their widgets in, and where a program mounts a live widget ...
- `pub fn is_closed(&self) -> bool` - Whether the console is closed (`close`): no session opens again.
- `pub fn suspend<T>(&self, run: impl FnOnce() -> T) -> T` - Hands the terminal to another program while `run` runs, one that reads it itself (`sudo` asking for a password, a package manager, a pager, an editor, started with inherited stdio), and takes it back after.
- `pub fn pump(&self, time: Duration)` - Runs the console for `time` on the terminal's clock: the inline session's frames due drawn (a spinner's steps), the keys typed handled, the prompts answered committed.
- `pub fn close(&self)` - Ends the inline session where one is open, what `write` left without a line break written first, so the terminal is restored with the cursor below the output; then flushes the terminal and closes it ...
- `pub fn error(&self, error: &dyn Error)` - Shows `error` as `Cli::run` shows what its body returns, for an error the program caught: `✗` and its message on the error output at `LogLevel::Error`, committed above what is live while an inline session holds the ...
- `pub fn error_with_traceback(&self, error: &dyn Error, traceback: &str)` - Reports an error with a traceback captured where it occurred.
- `pub fn progress(&self, message: impl Into<String>) -> CliProgress` - Shows `message` while an operation goes on, until the progress is closed (`CliProgress::complete`, `CliProgress::fail`, `CliProgress::cancel`), `CliProgress::run` closing it on the outcome of a body.
- `pub fn tasks<T, E>(&self, body: impl FnOnce(&ProgressTracker) -> Result<T, E>) -> Result<T, E>` - Runs `body` with a `ProgressTracker` whose tasks are shown as the output allows, and returns what the body does, its error too; the tracker is disposed once it has ended.
- `pub fn tasks_with<T, E>(&self, options: TasksOptions, body: impl FnOnce(&ProgressTracker) -> Result<T, E>) -> Result<T, E>` - `tasks` as `options` say: the columns, whether the finished leave the list, the bars' shape and width, the bytes' units.
- `pub fn prompt(&self, question: impl Into<String>) -> PromptBuilder<'_>` - Asks `question` and answers the line typed, the default for an empty one (the empty string without one).
- `pub fn prompt_any(&self, question: impl Into<String>) -> PromptAnyBuilder<'_>` - Asks `question` for many values on one line, split on the `separator` and trimmed, the blank ones dropped; an empty line answers the `default_values`, or none without them.
- `pub fn confirm(&self, question: impl Into<String>) -> ConfirmBuilder<'_>` - Asks `question`, answered yes (true) or no (false); the `default_value` is what Enter, or an empty line, answers.
- `pub fn choose_one<T: fmt::Display + Clone + PartialEq + 'static>(&self, question: impl Into<String>, choices: Vec<T>) -> ChooseOneBuilder<'_, T>` - Asks `question` for one of `choices`, which must not be empty, each shown by its `Display`.
- `pub fn choose_one_by<T: Clone + PartialEq + 'static>(&self, question: impl Into<String>, choices: Vec<T>, display: impl Fn(&T) -> String + 'static) -> ChooseOneBuilder<'_, T>` - `choose_one` of choices shown as `display` makes them.
- `pub fn choose_any<T: fmt::Display + Clone + PartialEq + 'static>(&self, question: impl Into<String>, choices: Vec<T>) -> ChooseAnyBuilder<'_, T>` - Asks `question` for any of `choices`, which must not be empty, answered in the order of `choices`: at least `min` of them, at most `max` (no limit when unset); a choice of fewer or more is refused with a message and ...
- `pub fn choose_any_by<T: Clone + PartialEq + 'static>(&self, question: impl Into<String>, choices: Vec<T>, display: impl Fn(&T) -> String + 'static) -> ChooseAnyBuilder<'_, T>` - `choose_any` of choices shown as `display` makes them.
