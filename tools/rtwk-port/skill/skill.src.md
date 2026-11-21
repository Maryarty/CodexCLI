<!-- not rtwk: MaterialApp AppBar Card TextStyle Colors FontWeight TextSpan MediaQuery BuildContext RawKeyboardListener ValueKey Layout Constraint Ratio Fill List Rect Margin Clear Block Paragraph Line ListState ScrollState ScrollbarState Tabs KeyCode KeyEvent UnicodeWidthStr FrameRequester Instant Buffer AppEvent CodexCLI RenderFlex Intent Msg Picker Gxx -->

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
| `truncate_text`, `format!("{}…", ..)` | `Text::line(..)`, `.overflow(TextOverflow::Ellipsis)`; `TextOverflow::Ellipsis` drops the blanks next to `…` (N1), so where the oracle keeps them: `widgets/dots.rs` `dots_end` (a composition, see the cookbook) |
| `center_truncate_path`, keep-the-end cuts | `TextOverflow::EllipsisMiddle`, `TextOverflow::EllipsisStart`; the mark is the `ellipsis` field of `Glyphs`; `TextOverflow::EllipsisMiddle` cuts inside segments, so a path that keeps its first and last segments is `pages/agents/path.rs` (G37, a closed row) |
| `UnicodeWidthStr::width`, `chars().count()`, padding with spaces | none: `Row` with `SizedBox` widths and `Expanded`; a number only where unavoidable: `rtwk_core::unicode::string_width` |
| `Line::from(vec![Span::styled(..)])`, `.cyan().dim()` | `Text::rich([Span::new(..).fg(..).dim()])`, colours from `Theme::of(cx)` tokens |
| `List` + `ListState` + `ScrollState` (selected index, `scroll_top`, `ensure_visible`) | `SelectionList` + `SelectionController` (`cursor`, `set_cursor`, `selected`); whole-row scroll with ↑/↓ more: `widgets/session_list.rs` (`ListView::builder` + `ListController` + `SelectionController`, G35/G42) |
| a long or growing list drawn by offset | `ListView::builder(n, row)` + `ListController::ensure_index_visible`; `stick_to_bottom(true)` for a transcript |
| `Scrollbar` + `ScrollbarState` | `Scrollbar::new(child)`, `BoxWidget::scrollbar`; `SingleChildScrollView` for one tall child |
| `Tabs`, a tab strip sliced with `‹ ›` | `TabBar::new(controller, tabs)` / `TabBar::strip` with a `TabController` for an ordinary bar; the oracle's filled strip of whole tabs with `‹ ›` outside them is `pages/agents/tabs.rs` (G36/G45: `LayoutBuilder` + `segmenter_of(cx)` + a `Row`) |
| `Table` + column widths | a label or option column sized to its content: `Grid` with `GridTrack` (`content`, `fixed`, `fr`), in a `SingleChildScrollView` when it scrolls (`widgets/options.rs`, G39; not `Table`); `Table::new(rows, columns)`, `TableColumn::new(title, cell)`, `TableController` only for a data table with a header and sorting |
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

### When the one widget differs from the oracle: compose, do not hand-roll

Try in order: the widget alone; a composition (Flutter's way: `LayoutBuilder`,
`Row`, `Grid`, `ListView` with its controllers, `CustomPaint`); a test setting.
Only then a gap row (`docs/rtwk-port/rtwk-gaps.md`) and a `pending(Gxx)` test.
Composition is the app choosing WHICH widget or spelling to build from a width
it measured (`segmenter_of(cx)` under a `LayoutBuilder`); cutting, rewrapping or
re-measuring strings yourself is the workaround (process.md section 5). Grep
`src/widgets` and `src/pages/agents` for a landed piece first.

| oracle pattern | rtwk route |
| --- | --- |
| filled tab strip of whole tabs, `‹ ›` outside | not `TabBar`: `pages/agents/tabs.rs` (a pure `strip()` window, a `Row` under a `LayoutBuilder`, widths by `segmenter_of(cx)`) |
| a cut `…` that keeps the blanks | `TextOverflow::Ellipsis` drops them (N1): `widgets/dots.rs` `dots_end` (`LayoutBuilder` + `Row[Expanded(clipped Text), Text("…")]`) |
| list with a cursor, whole-row scroll, ↑/↓ more | `widgets/session_list.rs`: `ListView::builder` + `ListController` + `SelectionController` + `ListenableBuilder` + `NotificationListener` |
| numbered options, label column + wrapped description | `widgets/options.rs`: `Column(min)[Flexible(SingleChildScrollView(Grid[content().within(0, cap), fr(1)]))]`, not `Table` |
| label/value card | a `Grid` with content tracks (`pages/session/cells/status/card.rs`) |
| a bar of whole cells | `CustomPaint` + `CellPainter` (`pages/session/cells/status/card.rs`, G40) |
| footer hints, the first spelling that fits | `widgets/page_keys.rs` `footer_fit` |
| a frameless bottom-pane view (`›` cursor) | `show_dialog` + `Dialog::bare` (`sheets/select/dialog.rs`) |
| the rows a `Text` takes at width w | `break_lines` (rtwk-render `objects/text.rs`) via `widgets/measure.rs`; first try `Flexible` or `max_lines` |
| display width of a string | `segmenter_of(cx)` (`rtwk::text`), never `string_width(.., TextMode::Graphemes, ..)` |
| `ｶﾞ` as two cells (oracle counts code points) | a test setting: `TextMode::Legacy` in the test terminal (`Options::terminal`, G44) |
| DIM asserted on a blank cell | the presenter drops an invisible DIM (`Attrs::VISIBLE_ON_BLANK`): assert the app surface's style, `frame_loop().styles().attrs_of(surface.style(x, y))` (G46) |
| `Wrap { trim: false }` breaking at blanks only | a real gap (G48): `pending`, no `Wrap` of words |
| a path cut that keeps its first and last segments | `pages/agents/path.rs` (G37, closed as ours, held to the oracle's tests) |
| selected-row fill under blanks | `Container::color` or `ColoredBox` (G22) |
| a prompt `"› "` with its trailing blank | `Text::new("› ").wrap(TextWrap::None)` (G26) |

### Keys and focus

- A page key (Esc, `?`, `←`) must reach the page or the `Navigator` while a
  child handles the others: bind with `cx.keys(..).passing()`, a handler that
  lets the key on when it did not take it (G27); a plain `Focus::bind` consumes.
- A key that a focused field would swallow first (the composer's `?` on an
  empty field): `FocusNode::set_on_key_event` on the field's node
  (`TextArea::focus_node`), the handler tells the page through the local sender
  (G43). Landed uses: `pages/session/session_page.rs`, `sheets/select/dialog.rs`.
- A dialog or sheet brings its own key trap, Escape and focus return
  (`show_dialog`): bind nothing by hand under it.

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
