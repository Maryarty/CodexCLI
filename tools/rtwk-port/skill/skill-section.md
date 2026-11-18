## CodexCLI: porting codex-tui onto rtwk

Everything above is rtwk's own guide. This section is the CodexCLI part: the port
(`codex-rs/tui-rtwk`) carries `codex-rs/tui` (ratatui, the oracle, never edited) onto rtwk.
Process and the concept map: `docs/rtwk-port/process.md` Part 1; design:
`docs/rtwk-port/architecture.md`.

### Where rtwk differs from ratatui / codex-tui

Each pair: `-` what the oracle does, `+` what the port writes.

```
- fn render(&self, area: Rect, buf: &mut Buffer)           // a widget draws into a buffer
+ impl StatelessWidget { fn build(&self, cx: &mut Cx<'_>) -> AnyWidget }   // describe; rtwk lays out and paints
- Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(area)
+ Column::new().child(SizedBox::..).child(Expanded::new(1, body))   // constraints go down, sizes up
- Line::from(vec![Span::styled("x", Style::new().cyan().bold())])
+ Text::rich([Span::styled("x", style)])                    // style from theme tokens, not literal colours
- Paragraph::new(text).block(Block::bordered().title("t"))
+ BoxWidget::new().border(Border::all(style).kind(BorderType::Rounded)).child(Text::new(..))
- match key_event { KeyCode::Char('y') => .. }              // one big match in handle_key_event
+ Focus::new(child).bind('y', handler)  /  KeyBindings      // keys go to the focused widget, then up
- stateful widget struct with &mut self, mutated from the event loop
+ plain model `A` + `cx.on(|a, cx| ..)` handlers; the view reruns   // widget state (scroll, focus, edit) is rtwk's
- tx.send(AppEvent::Foo); app_event_rx.recv() in the main loop
+ cx.on_with(..) handler; work from other threads: AppCx::channel / cx.spawn_tokio + a ticket
- tokio::spawn(..) and write the result back through an AppEvent
+ cx.spawn_tokio(future, |a: &mut A, r, cx| ..)            // use rtwk_tokio::TokioApp; R1: tag the work with a ticket, drop a stale answer in the handler
- frame_requester.schedule_frame()
+ nothing: a handler that changes the model reruns the view; rtwk paces frames
- List + ListState, rendered with a computed offset
+ ListView::builder (virtualized), SelectionList for pick lists   // scroll and selection live in rtwk
- Clear + a Rect in the middle of the screen (popup, modal)
+ show_dialog / close_dialog, Popover, CommandPalette        // a dialog is a route; see recipes.md
- terminal.draw(|f| ..) in tui.rs, custom terminal and scrollback insertion
+ run_app_with(model, view, RunOptions)                      // full screen or inline; rtwk owns the terminal
- insta::assert_snapshot!(terminal.backend())
+ Tester::with(Options::new().size(w, h)); t.assert_screen(..) // compared against the oracle through the replay helper
```

Pitfalls found while compiling the recipes against the pin (each is in a `references/guides/recipes.md` snippet):

- A `Column` centres its children across; add `CrossAxisAlignment::Start` for left-aligned rows.
- A `Column` in a `Dialog` needs `MainAxisSize::Min`, else it takes the whole height.
- A `RenderFlex` that overflows fails a `Tester` test: give the test terminal room.
- `assert_line` compares the whole row; `assert_screen` takes rows from the top without trailing blanks.
- After input, `pump()`; past an animation (a dialog fades in over ~160 ms), `pump_for(..)`.
- A presenter that shows a dialog must sit below the `Navigator`, else the dialog is above every provider.
- Closure types in `update_app` need an annotation: `|a: &mut App, cx|`.

### references/ (generated, never hand-edited)

Written by `python3 -I tools/rtwk-port/skill/generate.py` from the rtwk pin; every file names
its pin. `python3 -I tools/rtwk-port/skill/generate.py --check` fails when a file differs from
what the pin produces or `docs/rtwk-port/RTWK_REF` names another pin. Read this file once, then
the one references file that matches the widget; rtwk source only for what they do not answer.

- `references/widgets.md`: the index of every widget page ({{pages}}) with the group file it lives in.
- `references/widgets/<group>.md`: per widget the API (paths under `rtwk::`), the Code, the options, its methods, related types. {{n_groups}} groups: {{groups}}.
- `references/guides/widgets-by-need.md`: a codex-tui need (approval dialog, composer, history list, status line, ...) to the rtwk widget.
- `references/guides/recipes.md`: seven snippets that compile and pass their tests against the pin (navigation with a result, request dialog, provider select, list, text area, channel with a ticket, screen test).
- `references/guides/state.md`, `cli.md`, `markdown.md`: rtwk's `docs/guide`, copied.
- `references/guides/app-api.md`, `testing-api.md`: the `App`/`ViewCx`/`AppCx` and `Tester` APIs from the pin's source.
