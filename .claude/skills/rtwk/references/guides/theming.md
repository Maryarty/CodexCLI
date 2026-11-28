<!-- Written by `cargo xtask skill-references` from docs/guide; do not edit. -->
# Theming

Colors and styles from a theme, dark or light as the terminal is.

A `Theme` gives the widgets below it a `ThemeData`: a background, text, `muted`,
`primary`, `accent`, `border`, `selection`, and `success`, `warning`, `error`
and `info`, each a `Style`. Every widget of rtwk draws in them, and
`Theme::of(cx)` gives them to an app's own, where a context is at hand (a
component's `build`, or `Builder::new(|cx| ..)` in an app's view).

<!-- file: crates/rtwk/examples/guides/theming.rs -->
```rust
use std::process::ExitCode;

use rtwk::app::ViewCx;
use rtwk::focus::KeyBindings;
use rtwk::layout::{Border, BorderTitle, BorderType, BoxWidget, CrossAxisAlignment, EdgeInsets};
use rtwk::prelude::*;
use rtwk::style::{Span, ThemeData};
use rtwk::widgets::Builder;

pub fn main() -> ExitCode {
    run_app((), view)
}

pub fn view(_: &(), cx: &mut ViewCx<()>) -> AnyWidget {
    let quit = cx.on(|_, cx| cx.quit());
    // Catppuccin's dark flavor on a dark terminal, its light one on a light one, switched when
    // the terminal reports another color scheme; without a Theme, the terminal's own colors
    Theme::adaptive(Builder::new(move |cx| status(cx, quit.clone())))
        .dark(ThemeData::CATPPUCCIN_MOCHA)
        .light(ThemeData::CATPPUCCIN_LATTE)
        .into()
}

fn status(cx: &mut rtwk::BuildCx<'_>, quit: Handler<()>) -> AnyWidget {
    let theme = Theme::of(cx);
    KeyBindings::new(
        [("q", quit)],
        BoxWidget::new()
            .border(Border::all(theme.border.clone()).kind(BorderType::Rounded))
            .title(BorderTitle::new(" Services ").style(theme.primary.clone()))
            .padding(EdgeInsets::all(1))
            .child(
                Column::new()
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .child(Text::rich([
                        Span::styled("● ", theme.success.clone()),
                        Span::new("api      up"),
                    ]))
                    .child(Text::rich([
                        Span::styled("● ", theme.warning.clone()),
                        Span::new("worker   slow"),
                    ]))
                    .child(Text::rich([
                        Span::styled("● ", theme.error.clone()),
                        Span::new("mail     down"),
                    ]))
                    .child(Text::new("q quits").style(theme.muted.clone())),
            ),
    )
    .into()
}
```

On a dark terminal, in Catppuccin's dark flavor:

```text
╭ Services ────────────╮
│                      │
│ ● api      up        │
│ ● worker   slow      │
│ ● mail     down      │
│ q quits              │
│                      │
╰──────────────────────╯
```

## Which theme

- `Theme::new(ThemeData::NORD, child)` sets one theme. Built in:
  `ThemeData::DARK`, `LIGHT`, `NORD`, `CATPPUCCIN_MOCHA` and `CATPPUCCIN_LATTE`.
  Give a `Theme` the `Rc<ThemeData>` the app keeps and a rebuild compares it by
  address; a `ThemeData` given by value is compared field by field.
- `Theme::adaptive(child)` chooses the dark and the light theme (`DARK` and
  `LIGHT`, unless `.dark(data)` and `.light(data)` say otherwise), as the app
  above does, switched when the terminal reports another color scheme.
- Without a `Theme`, `Theme::of(cx)` is the same adaptive choice of the
  defaults: `DARK` on a dark terminal, `LIGHT` on a light one; on a terminal
  that does not say which it is, its own colors.
- `Theme::terminal(child)` uses the terminal's own colors
  (`ThemeData::terminal_of(cx)`): the user's palette, `primary` blue, and
  `surface` (toasts, dialogs, menus), `muted`, `border` and the selections greys
  mixed from the background and text color the terminal told, a step from its
  background whatever it is, and a color of the palette its bright entry where
  only that one reads on the background; where it told none, greys of the 256
  colors, which no color scheme changes. For an app that should look like the
  rest of the user's terminal, and keep their scheme (Solarized, a light one).
- `ThemeData::copy_with(|t| t.primary = ..)` makes a theme of another: the
  slots derived from those it did not change stay as they were.
- A test chooses the color scheme the terminal reports by mounting the app in
  a `TerminalBrightness::new(Brightness::Light, app)` with `Tester::pump_app_in`.

## Glyphs

The marks the widgets draw (a check, a cross, a cursor bar, the arrows of a
tree, a scrollbar's thumb and track, the line of a gauge, the frames of a
spinner) are the theme's too, the `glyphs` field of `ThemeData`, a `Glyphs` set named by what
a mark means and not by the widget that shows it. `Glyphs::UNICODE` is the
default; `Glyphs::ASCII` is for a terminal or a font without them (a `#` thumb
on a `|` track, `=` for a gauge's filled part, `v` for a check, `>` for what is
closed); `copy_with` changes some marks of a set. For the whole app a
`ThemeData::copy_with(|t| t.glyphs = ..)` does it, for one part of it a `Theme`
around that part. A widget's own parameter for a mark it has stays above the
set. Borders are in it: a `Border`, a `Divider`, a `SplitView` or a `Callout`
given no `BorderType` draws `glyphs.lines`, and in `Glyphs::ASCII` every type is
ASCII, a `BorderType::Rounded` or a `BorderType::Heavy` too, so one `copy_with` makes the whole app ASCII
but its images. Charts are in it too, and so is the ellipsis a `Text` or a box
title is cut with. One rule holds for every mark, in a cell or inside text
alike: it takes one column and one code point; one that is wider, or a letter
with a combining accent, is the `Glyphs::ASCII` mark in its place, with one
warning in the log for each. The rule is kept in one place, `ThemeMarks`, which a
widget of your own asks (`ThemeMarks::of(cx).text(Marks::CLOSE)`, a render
object `ThemeMarks::of_object(segmenter)`); `Marks` has a descriptor for each
mark.

<!-- file: crates/rtwk/examples/guides/glyphs.rs -->
```rust
use std::process::ExitCode;

use rtwk::app::ViewCx;
use rtwk::controls::{Checkbox, StatusKind, StatusMessage};
use rtwk::data::LineGauge;
use rtwk::focus::KeyBindings;
use rtwk::layout::CrossAxisAlignment;
use rtwk::prelude::*;
use rtwk::style::{Glyphs, ThemeData};

pub fn main() -> ExitCode {
    run_app((), view)
}

pub fn view(_: &(), cx: &mut ViewCx<()>) -> AnyWidget {
    // ASCII all through, but a plus for a check and for a checked box
    let data = ThemeData::DARK.copy_with(|t| {
        t.glyphs = Glyphs::ASCII.copy_with(|g| {
            g.check = "+".into();
            g.checkbox_on = "+".into();
        });
    });
    Theme::new(
        data,
        KeyBindings::new(
            [("q", cx.on(|_, cx| cx.quit()))],
            Column::new()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .child(StatusMessage::new("Built in 2 s").kind(StatusKind::Success))
                .child(StatusMessage::new("3 files skipped").kind(StatusKind::Warning))
                .child(LineGauge::new(0.6).label("upload "))
                .child(Checkbox::new(true).label("Publish")),
        ),
    )
    .into()
}
```

Which shows:

```text
+ Built in 2 s
! 3 files skipped
upload  =============---------
[+] Publish
```

## Styles and spans

- `Style::new().fg(..).bg(..)` and the attributes `.bold()`, `.dim()`,
  `.italic()`, `.underline()` (and curly, dotted, dashed and double),
  `.reverse()`, `.strike()`. A `Color` is one of the 16 named ones, an index of
  the 256, or `Color::rgb(r, g, b)`; the terminal's own depth decides what is
  written.
- `Text::new("Ready").style(theme.success.clone())` styles a whole text;
  `Text::rich([..])` a row of spans, `Span::styled("● ", theme.success.clone())`
  or `Span::new("q").bold()`.
- `DefaultTextStyle` sets the style of the text below it, which a `Text` merges
  its own over.

Gradients across a background or text, and how the brightness is read, are in
the catalog's [Style](../widgets/style.md) group.
