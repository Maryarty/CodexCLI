<!-- Written by tools/rtwk-port/skill/generate.py from rtwk 9f28b7fb; do not edit. -->

# Terminal

[All widgets](../widgets.md)

## TerminalView

A program's terminal in a view: keys, pastes, the mouse and resizes sent to it, its scrollback scrolled; here a scripted shell, not a program in a pseudo-terminal.

API:

- `rtwk_term::TerminalView` (struct, `crates/rtwk-term/src/view/widget.rs:56`) Displays a session and gives its program terminal input, focus and size.

### Code

The gallery's Code tab at the initial options (route `#/terminal/terminalview`, `crates/rtwk-gallery/src/pages/media_terminal.rs:178`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
use rtwk::AnyWidget;
use rtwk::focus::{FocusNode, KeyCombo};
use rtwk::layout::{Border, BorderTitle, BorderType, BoxWidget, SizedBox};
use rtwk::style::ThemeData;
use rtwk_term::{SharedSession, TerminalView};

// Supply a TerminalSession connected to an in-memory program or a PTY.
// Keep the session and focus node in state while rebuilding the view.
fn preview(session: SharedSession, focus: FocusNode, theme: &ThemeData) -> AnyWidget {
    let ignored = ["tab", "shift+tab", "escape", "ctrl+p"]
        .map(|key| KeyCombo::parse(key).expect("gallery key"));
    let terminal = TerminalView::new(session)
        .focus_node(focus)
        .ignored_keys(ignored)
        .wheel_lines(3);
    SizedBox::new(
        Some(82),
        Some(26),
        Some(
            BoxWidget::new()
                .border(Border::all(theme.border.clone()).kind(BorderType::Rounded))
                .title(BorderTitle::new(" shell "))
                .child(terminal)
                .into(),
        ),
    )
    .into()
}
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| wheel | 1, 3, 10 | 3 |

### Methods

#### `TerminalView`: constructors and associated functions

- `pub fn new(session: SharedSession) -> Self` - Creates a view of a shared session.

#### `TerminalView`: builder methods (take and return `self`)

- `pub fn focus_node(mut self, node: FocusNode) -> Self` - Uses an application-owned focus node.
- `pub fn autofocus(mut self, on: bool) -> Self` - Requests focus when first mounted.
- `pub fn can_request_focus(mut self, on: impl Into<Option<bool>>) -> Self` - Whether the view's focus node can take the focus; false makes a view to read only, as of a log a program writes (`tail -F`): a press does not focus it, and no key or paste reaches the program.
- `pub fn ignored_keys(mut self, keys: impl Into<Rc<[KeyCombo]>>) -> Self` - Leaves these key combinations to ancestors of the view.
- `pub fn wheel_lines(mut self, lines: u16) -> Self` - Lines scrolled per wheel turn, or arrow presses on the alternate screen.
- `pub fn default_style(mut self, style: Style) -> Self` - Overrides the theme foreground and background used for default cells.
- `pub fn on_event(mut self, handler: Handler<TerminalEvent>) -> Self` - Reports each event before the corresponding dedicated callback.
- `pub fn on_completion(mut self, handler: Handler<SessionCompletion>) -> Self` - Reports each completion milestone once for this mounted view and its session.
- `pub fn on_title_changed(mut self, handler: Handler<String>) -> Self` - Reports title changes.
- `pub fn on_bell(mut self, handler: Handler<()>) -> Self` - Reports a bell.
- `pub fn on_clipboard_write(mut self, handler: Handler<(String, String)>) -> Self` - Reports a clipboard write request as (selection, text).
- `pub fn on_clipboard_read<F: Future<Output = Option<String>> + 'static>(mut self, handler: impl Fn(String) -> F + 'static) -> Self` - Answers clipboard reads from an application callback.

#### `TerminalView`: methods

- `pub fn session(&self) -> &SharedSession` - The program and its terminal.

### Also used in the code

- `rtwk::focus::KeyCombo` (struct, `crates/rtwk-framework/src/focus/key_combo.rs:60`) A key and the modifiers held with it, as a key binding names it; it `accepts` the key events a terminal sends for that key.
  - public fields: `code: KeyCode`, `char: Option<char>`, `modifiers: Mods`
- `rtwk::layout::BorderTitle` (struct, `crates/rtwk-widgets/src/layout/box_.rs:153`) Text on the top or the bottom edge of a `BoxWidget`, in `style`: the y of `alignment` picks the edge (-1 the top, 1 the bottom), its x the place along it (twk's `BorderTitle`).
  - public fields: `text: Str`, `style: Style`, `alignment: Alignment`
