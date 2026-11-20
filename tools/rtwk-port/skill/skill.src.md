<!-- not rtwk: MaterialApp AppBar Card TextStyle Colors FontWeight TextSpan MediaQuery BuildContext RawKeyboardListener ValueKey Layout Constraint Rect Margin Clear Block Paragraph Line ListState ScrollState ScrollbarState Tabs KeyCode KeyEvent UnicodeWidthStr FrameRequester Instant Buffer AppEvent CodexCLI RenderFlex Intent Msg Picker -->

## CodexCLI: porting codex-tui onto rtwk

Everything above is rtwk's own skill (copied from `skills/rtwk` of the pin, with
its `references/`); this part is the port's. `references/widgets.md` is the
catalog (rtwk's `docs/widgets` at the pin, one file per group under
`references/widgets/`), `references/guides/state.md`, `cli.md`, `markdown.md` its
guides. The port adds under `references/guides/`: `app-api.md` and
`testing-api.md` (every public method of `AppView`, `ViewCx`, `AppCx`, `Cx`,
`Navigator`, `Tester`, from the pin's source), `recipes.md` (snippets that
compile and pass their tests against the pin: navigation with a result, a
request dialog, provider select, a list, a text area, a channel with a ticket,
a screen test) and `widgets-by-need.md` (which widget for which codex-tui
need). Read the one that matches the widget, not all of them.

A working widget to copy the manner from: the gallery page under the pin,
`crates/rtwk-gallery/src/pages/<page>.rs` in the pinned checkout named by the Cargo manifest (search for the
widget's name there; `cargo run -p rtwk-gallery` in the pin shows every widget
live, F2 shows the source of the one under the cursor), the pin's
`crates/rtwk/examples/`, and `/Users/admin/Documents/dev_projects/rtwk-weather`.

`SKILL.md` and `references/` are written by
`python3 -I tools/rtwk-port/skill/generate.py`, never by hand; `--check` fails
when a file differs from what the pin produces, when `docs/rtwk-port/RTWK_REF`
names another pin, or when this part names an rtwk type or member the pin lacks.

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

### More port pitfalls

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
- A `Column` in a `Dialog` needs `MainAxisSize::Min`, else it takes the whole
  height; a dialog fades in over 160 ms in a test: pump past it.
- Closure types in `update_app` and `read_app` need an annotation:
  `|a: &mut App, cx|`.
