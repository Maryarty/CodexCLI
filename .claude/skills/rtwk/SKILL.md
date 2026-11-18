---
name: rtwk
description: Write, test and debug Rust terminal UI apps with rtwk, which has Flutter's widget model (StatelessWidget, StatefulWidget, State, Column, Row, Expanded) on a grid of character cells, with its own widgets (BoxWidget, ListView, SelectionList, Table, TextField, KeyBindings, dialogs) and headless tests (rtwk_test::Tester with press, type_text, pump, assert_screen). Use when writing or changing code that uses the rtwk crates (rtwk::…, rtwk_test, rtwk_tokio), in this repository codex-rs/tui-rtwk, before writing any Flutter-style or ratatui-style widget code for a terminal: rtwk's names, units and defaults differ from Flutter's and ratatui's in ways memory gets wrong.
---
<!-- rtwk 9f28b7fb. Written by tools/rtwk-port/skill/generate.py from tools/rtwk-port/skill/skill.src.md and snippets/; do not edit. -->

# Terminal apps with rtwk

rtwk is Flutter's widget model on a terminal, in Rust (a port of twk): immutable
widgets describe the UI, elements keep `State` across rebuilds, render objects
lay out by constraints down and sizes up and paint into cells; keys,
`InheritedWidget`, `Navigator`, `Overlay` and `Focus` behave as in Flutter.
The unit, the catalog, the input and the API's shape do not. Do not write
Flutter's (or ratatui's) API from memory: there is no Material, no
`TextStyle`, no `MediaQuery`, no pixels, no `Rect` to split and no `render`
into a buffer. Look a widget up (References) before using it.

An app depends on `rtwk` (the facade: `rtwk::layout`, `rtwk::widgets`,
`rtwk::text`, `rtwk::style`, `rtwk::focus`, `rtwk::controls`, `rtwk::scroll`,
`rtwk::editing`, `rtwk::overlay`, `rtwk::data`, `rtwk::navigation`,
`rtwk::animation`, `rtwk::provider`, `rtwk::app`), and besides it on
`rtwk_tokio` (Tokio work), `rtwk_rich` (Markdown, diff, highlighting) and
`rtwk_core` where it uses them; its tests on `rtwk_test`. A command-line tool
that prints, asks and shows progress, instead of drawing a screen, uses
`rtwk::cli::Cli` and no `run_app`. Edition 2024, the toolchain of rtwk's
`rust-toolchain.toml`.

## Where rtwk differs from Flutter

Each pair is the line a model writes from memory (`-`) and the one that is
right (`+`).

```diff
# a widget is a struct with builder methods; sizes are whole cells, i32
- SizedBox(width: 20.5, height: 10.0, child: Text('x'))
+ SizedBox::new(Some(20), Some(10), Some(Text::new("x").into()))
- Padding(padding: EdgeInsets.all(1), child: child)
+ Padding::new(EdgeInsets::all(1), child)
# setState: an app is a plain model and a view; a handler changes the model and the view reruns
- onPressed: () => setState(() => count++)
+ Button::new("+").on_pressed(cx.on(|app, _| app.count += 1))
# a State's handlers send it messages (State::update answers Rebuild::Yes) or are lent it
+ cx.send(Msg::Add)    cx.callback(|state: &mut Picker, i: usize| state.pick = i)
# a style has colors and attributes, no font; spans carry their own
- Text('x', style: TextStyle(color: Colors.red, fontWeight: FontWeight.bold))
- Text.rich(TextSpan(children: [TextSpan(text: 'x')]))
+ Text::new("x").style(Style::new().fg(Color::RED).attrs(Attrs::BOLD))
+ Text::rich(["error: ".bold().fg(Color::RED), Span::new("disk full")])
# Text::new wraps by words and clips; Text::line is one line with the ellipsis
- Text(s, maxLines: 1, overflow: TextOverflow.ellipsis)
+ Text::line(s)    Text::new(s).max_lines(2).overflow(TextOverflow::EllipsisMiddle)
# no Material: run_app is the root (overlay, theme, locale); a screen is a BoxWidget or a Scaffold
- MaterialApp(theme: ThemeData.dark(), home: Scaffold(appBar: AppBar(), body: Card()))
+ run_app_with(model, view, RunOptions { theme: Some(ThemeData::NORD), ..Default::default() })
# the room is in the constraints, there is no MediaQuery
- MediaQuery.of(context).size.width
+ LayoutBuilder::new(|_, c| Text::new(format!("{}", c.max_width)).into())
# a Column centres across; a list takes all the room it gets
- Column(children: [ListView(children: rows)])
+ Column::new().cross_axis_alignment(CrossAxisAlignment::Stretch).child(Expanded::new(1, list))
# keys are strings, in lower case; Focus binds a KeyPress
- RawKeyboardListener(onKey: onKey, child: child)
+ KeyBindings::new([("ctrl+s", save)], child).hints([("ctrl+s", "save")])
+ Focus::new(child).bind(KeyPress::char('y'), cx.on(|app, _| app.accept()))
# a child made in a loop takes a Key
- ListTile(key: ValueKey(row.id))
+ AnyWidget::from(row_widget).with_key(Key::value(row.id))
# Theme::of answers an Rc<ThemeData>; marks come from its glyphs, so an ASCII theme switches the app
- Text('✓ done')
+ Text::new(format!("{} done", Theme::of(cx).glyphs.check))
# tests: rtwk_test, not flutter_test
- await tester.tap(find.byType(Button)); await tester.pumpAndSettle();
+ t.press("enter"); t.pump();
```

- A handler is retained (`Handler`): move owned ids into it and read the
  current state from its `&mut App` argument. A closure that borrows the
  view's `app` does not compile (E0521). `cx.unchanged()` in a handler that
  leaves the view's inputs as they were skips the rebuild.
- Controllers (`TextEditingController`, `SelectionController`,
  `ListController`, `TabController`, `TableController`, `ScrollController`)
  are shared handles: keep them in the model or a `State`, never make them in
  a view, or the cursor and scroll reset on every rebuild.
- Work off the UI thread: `AppCx::spawn(future, then)`, `AppCx::channel` for
  items from any thread (applied in order, one view for all), and
  `rtwk_tokio`'s `spawn_tokio(future, then)`. A handler that changes the model
  is the redraw: there is no frame to request.
- `run_app(model, view)` is full screen; `RunOptions { mode:
  ScreenMode::Inline { at_line_start: true }, .. }` runs in rows below the
  prompt, where `Static` writes items into the terminal's history. Dialogs,
  menus, popovers and toasts need an `Overlay` above their context:
  `run_app`'s root places it; in a test, mount
  `rtwk::overlay::App::new(AppView::new(model, view))`.
- Only the focused widget gets a key first, then `Focus::bind` and
  `KeyBindings` above it. A focused `TextField` takes `q`, so bind `ctrl+q`
  to quit.
- Nothing may print to the terminal while the app owns it; `rtwk::debug` has
  the inspector and the frame timing overlay.

## An app

Depend on the crates by path; the toolchain and lockfile come from the rtwk
checkout. `cargo run`: type a task and Enter adds it, Tab goes to the list,
Space marks the task done there.

```toml
# Cargo.toml; <rtwk> is where the rtwk repository is checked out
[package]
name = "my_app"
version = "0.1.0"
edition = "2024"

[dependencies]
rtwk = { path = "<rtwk>/crates/rtwk" }

[dev-dependencies]
rtwk-test = { path = "<rtwk>/crates/rtwk-test" }
```

```rust
// src/todo.rs
use std::rc::Rc;

use rtwk::AnyWidget;
use rtwk::app::ViewCx;
use rtwk::controls::{KeyHintsBar, SelectionList};
use rtwk::data::SelectionController;
use rtwk::editing::{TextEditingController, TextField};
use rtwk::focus::KeyBindings;
use rtwk::layout::{Border, BorderTitle, BorderType, BoxWidget, Column, CrossAxisAlignment, Expanded};
use rtwk::style::Style;
use rtwk::widgets::Text;

/// A to-do list: Enter adds the task typed, Tab goes to the list, Space
/// marks a task done there, Ctrl+Q quits.
pub struct Todo {
    pub tasks: Vec<String>,
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

pub fn view(app: &Todo, cx: &mut ViewCx<'_, '_, Todo>) -> AnyWidget {
    let tasks: Rc<[String]> = app.tasks.clone().into();
    let list = SelectionList::new(tasks.len(), |i| i, move |i| Text::new(tasks[i].clone()).into())
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
```

```rust
// src/main.rs
fn main() -> std::process::ExitCode {
    rtwk::run_app(my_app::todo::Todo::default(), my_app::todo::view)
}
```

## Test

`Tester` runs the real app on a terminal in memory under a fake clock: input
goes in as the bytes a terminal sends, and the screen is read back from what
the app wrote. `cargo test` runs it.

```rust
// src/todo.rs, at its end
#[cfg(test)]
mod tests {
    use super::*;
    use rtwk::app::AppView;
    use rtwk_test::{Options, Tester, find};

    #[test]
    fn enter_adds_the_task_typed_space_in_the_list_marks_it_done() {
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
    }
}
```

- Input draws nothing by itself: `t.pump()` after `type_text`, `press` or a
  click, before reading the screen. `t.pump_for(duration)` moves the fake
  clock; a dialog fades in over 160 ms: pump past it.
- `type_text("text")` sends text as one read; `press("tab space")` sends each
  key as a read of its own, named as `KeyBindings` names them. Read the screen
  with `lines`, `assert_line` (the whole row), `assert_screen` (rows from the
  top, no trailing blanks), `assert_screen_styled`; find widgets with
  `find::text`, `find::by_type`, `find::by_key`; read the model with
  `read_app`, change it with `update_app`. A test never sleeps: it pumps the
  clock, or `wait_until`s a real event.

## Pitfalls

- A list (`ListView`, `SelectionList`, `Table`, `TreeView`, `Navigator`)
  takes all the room it gets. In a `Column` put it in `Expanded`, or it takes
  the whole height and pushes the rest off the screen; inline, give it a
  height (`SizedBox::new(None, Some(16), Some(list.into()))`).
- A `Column` centres its children across: `CrossAxisAlignment::Start` for
  left-aligned rows, `CrossAxisAlignment::Stretch` for full-width ones. A `Column` in a `Dialog`
  needs `MainAxisSize::Min`, else it takes the whole height. A flex that
  overflows fails a `Tester` test: give the test terminal room.
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
- Closure types in `update_app` and `read_app` need an annotation:
  `|a: &mut App, cx|`.

## References

Read the one that matches the widget, not all of them. `references/`, next to
this file, is generated from the rtwk pin (every file names it):

- `references/widgets.md`: every widget (148 pages), one line each, by group.
- `references/widgets/<group>.md`: the group's widgets with their API paths
  under `rtwk::`, the Rust that builds them (the gallery's Code tab), the
  options, the builder methods and the types they use. 20 groups:
  text, layout, style, scrolling, data, editing, controls, overlays, animation, mouse, focus-and-keys, navigation, foundation, framework, app, screen-readers, debugging, rich, images, terminal.
- `references/guides/`: `getting-started.md` (rtwk's own guide: entry
  points, handlers, a form, data and overlays, components, Tokio streams, CLI
  prompts, debugging), `state.md`, `cli.md`, `markdown.md` (rtwk's
  `docs/guide`), `app-api.md` and `testing-api.md` (every public method of
  `ViewCx`, `AppCx`, `Cx`, `Navigator`, `Tester`, from the source),
  `recipes.md` (snippets that compile and pass their tests against the pin:
  navigation with a result, a request dialog, provider select, a list, a text
  area, a channel with a ticket, a screen test) and `widgets-by-need.md`
  (which widget for which codex-tui need).

Before writing a widget, look at a working one in the gallery: its Code-tab
snippets are under each widget's `### Code` in `references/widgets/<group>.md`
(extracted into `tools/rtwk-port/skill/gallery-codes.jsonl`), and the whole
page, with its state and controllers, is
`~/.cache/codexcli/rtwk-pin/crates/rtwk-gallery/src/pages/<page>.rs` (grep the
widget's name there). `cargo run -p rtwk-gallery` in the pin shows every
widget live; F2 shows the source of the one under the cursor. rtwk's own
apps: `crates/rtwk/examples/` in the pin and
`/Users/admin/Documents/dev_projects/rtwk-weather`.

`references/` is written by `python3 -I tools/rtwk-port/skill/generate.py`,
never by hand; `--check` fails when a file differs from what the pin
produces, when `docs/rtwk-port/RTWK_REF` names another pin, or when this
file names an rtwk type or member the pin lacks.

## CodexCLI: porting codex-tui onto rtwk

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
| `truncate_text`, `format!("{}…", ..)` | `Text::line(..)`, `.overflow(TextOverflow::Ellipsis)` |
| `center_truncate_path`, keep-the-end cuts | `TextOverflow::EllipsisMiddle`, `TextOverflow::EllipsisStart`; the mark is `Glyphs::ellipsis` |
| `UnicodeWidthStr::width`, `chars().count()`, padding with spaces | none: `Row` with `SizedBox` widths and `Expanded`; a number only where unavoidable: `rtwk_core::unicode::string_width` |
| `Line::from(vec![Span::styled(..)])`, `.cyan().dim()` | `Text::rich([Span::new(..).fg(..).dim()])`, colours from `Theme::of(cx)` tokens |
| `List` + `ListState` + `ScrollState` (selected index, `scroll_top`, `ensure_visible`) | `SelectionList` + `SelectionController` (`cursor`, `set_cursor`, `selected`) |
| a long or growing list drawn by offset | `ListView::builder(n, row)` + `ListController::ensure_index_visible`; `stick_to_bottom(true)` for a transcript |
| `Scrollbar` + `ScrollbarState` | `Scrollbar::new(child)`, `BoxWidget::scrollbar`; `SingleChildScrollView` for one tall child |
| `Tabs`, a tab strip sliced with `‹ ›` | `TabBar::new(controller, tabs)` / `TabBar::strip` with a `TabController` |
| `Table` + column widths | `Table::new(rows, columns)`, `TableColumn::new(title, cell)`, `TableCell`, `TableController` |
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
