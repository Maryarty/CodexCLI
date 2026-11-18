<!-- Written by `cargo xtask skill-references` from docs/widgets; do not edit. -->
# Terminal

A program running in a widget, on a pseudo-terminal.

[All widgets](../widgets.md)

A Code is the body of the gallery's preview: it names widgets by the last segment of the paths under API (`use` them) and may use `cx` or values of the gallery's page.

## TerminalView

A program's terminal in a view: keys, pastes, the mouse and resizes sent to it, its scrollback scrolled; here a scripted shell, not a program in a pseudo-terminal.

- `rtwk_term::TerminalView` (struct) in `crates/rtwk-term/src/view/widget.rs:56`: Displays a session and gives its program terminal input, focus and size.

### Code

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

| option | values | starts at |
| --- | --- | --- |
| wheel lines | 1, 3, 10 | 3 |
