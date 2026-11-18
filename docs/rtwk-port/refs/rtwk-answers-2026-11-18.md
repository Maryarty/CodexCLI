# rtwk: how to do it right — answers to tui-rtwk's rtwk-gaps.md (2026-11-18)

Checked against rtwk `claude/app-dialogs` (main d47446d1 + memo-b + dialogs, not yet landed).
"compiled+run" = a scratch test against that branch passed; "read" = from the source only.

Tester gotcha: after `t.press(..)` call `t.pump()`; `pump_until_idle()` right after a press can return before the key's effect.

**G13 (compiled+run, needs app-dialogs).** A dialog shown from the root lands in `overlay::App` above your providers. Bind the handler in a memo node under providers + Navigator; `AppCx::show_dialog` shows from that node, as a route of that Navigator, under the providers. For state-driven dialogs (approval queue on a channel), keep the memo node's `ViewRoot` (an `Rc<Cell<Option<ViewRoot<A>>>>` passed as the memo's input) and call `AppCx::show_dialog_from(root, ..)`; in `closed` show the next queued request with `show_dialog_from(stored root)` too, and never retry on `None`.
```rust
fn page(done: &Option<bool>, cx: &mut MemoCx<Shell>) -> AnyWidget {
    let ask = cx.on(|_, cx| { cx.show_dialog::<bool>(
        |cx| Dialog::new(Text::new(format!("provided {}", read::<i32, _>(cx)))).into(),
        DialogOptions::default(), |app, a, _| app.done = a.or(Some(false))); });
    Button::new("ask").on_pressed(ask).into()
}
// view: App::new(MultiProvider::new().with(Provider::value(2_i32))
//   .child(Navigator::new().home(cx.memo(app.done, page))))
```

**G14 (read).** A provider projection is a per-run snapshot (as an InheritedWidget). Don't read a provider back in the same flush: decide on the model in `closed` (it gets `&mut A`).
```rust
cx.show_dialog::<bool>(build, opts, |app, answer, _| {
    app.pending.take();
    app.answers.push(answer == Some(true));
});
```

**G15 (compiled+run).** Changing the provider list remounts an unkeyed Navigator (Flutter semantics). Keep a `GlobalKey` in app state (not per build): `AnyWidget::from(Navigator::new().home(page)).with_key(app.nav_key.clone())`. Your current workaround is the intended fix.

**G16.** Fixed by app-dialogs: `AppCx::show_dialog(builder, options, closed)` and `show_dialog_from`; `shell::dialog_closed`, the `CloseWatcher` and the shell's `DialogResult` can go after the repin.

**G17 (theme part compiled).** `run_app_with` already places `overlay::App`, localizations and the graphics scope, and takes `RunOptions { theme, locale, .. }` — don't copy `run_process`. `run_app_view(AppView, RunOptions)` (for `on_start`/handle) comes in the gap batch; until then do start-up work in `main` before `run_app_with`.
```rust
let rt = tokio::runtime::Runtime::new()?; let _g = rtwk_tokio::enter(&rt);
let code = rtwk::run_app_with(state, view,
    RunOptions { theme: Some(ThemeData::terminal(Brightness::Dark)), ..Default::default() });
```

**G18 (compiled).** `spawn_tokio` aborts on unmount by design (element-scoped, like a State's subscription). App-lifetime work goes on the runtime directly; `main` owns shutdown:
```rust
let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
let backend = rt.spawn(async move { let _ = stopped.await; /* client.shutdown().await */ });
let code = rtwk::run_app_with(state, view, opts);
let _ = stop.send(());
let _ = rt.block_on(backend);
```

**G19 (compiled+run).** `Tester::pump_app(AppView)` gives the app its `AppHandle` (`cx.quit()` works); `pump_widget(App(AppView))` does not. For tokio: enter a multi-thread runtime on the test thread and loop pump/check/sleep (worker runs in real time, Tester clock is fake). Theme/localizations: wrap them in the view fn. (`Tester::pump_app_in` + docs come in the gap batch.)
```rust
let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(1).enable_time().build()?;
let _g = rtwk_tokio::enter(&rt);
let mut t = Tester::with(Options::new().size(40, 3));
t.pump_app(AppView::new(Back::default(), view));
t.update_app::<Back>(|_, cx| { cx.spawn_tokio(async { 7 }, |b, v, _| b.got = Some(v)); });
for _ in 0..2000 { t.pump(); if t.read_app::<Back, _>(|b| b.got.is_some()) { break; } std::thread::sleep(Duration::from_millis(1)); }
```

**G20.** Real gap; `rtwk::style::{Attrs, Color}` re-export comes in the gap batch.

**G21 (compiled+run).** No initial `ScrollNotification` (twk has none either). A `ScrollController` notifies on the first layout — listen to it. Add the pager to your R14 allowlist.
```rust
let c = ScrollController::new();
let body = ListenableBuilder::new(Rc::new(c.clone()), { let c = c.clone(); move |_, child| {
    if let Ok(p) = c.try_position() { /* p.pixels(), p.max_scroll_extent(), p.viewport_dimension() */ }
    child.cloned().unwrap() }})
  .child(SingleChildScrollView::new(Text::new(text)).controller(c.clone()));
```

**G22 (compiled+run).** Fill the row with a styled `BoxWidget` in a stretch column (the pattern `Autocomplete` uses):
```rust
let theme = Theme::of(cx);
let style = if sel { theme.text.merge(&theme.selection) } else { theme.text.clone() };
Column::new().main_axis_size(MainAxisSize::Min).cross_axis_alignment(CrossAxisAlignment::Stretch)
    .child(BoxWidget::new().style(style).child(Text::new("› two").wrap(TextWrap::None)))
```
Or `SelectionList` with `selected_style`/`cursor_style` carrying a background.

**G23 (compiled+run).** `~` filler is app work:
```rust
LayoutBuilder::new(move |_, c| if n >= c.max_height {
    SingleChildScrollView::new(Text::new(text.clone())).into()
} else {
    Column::new().cross_axis_alignment(CrossAxisAlignment::Start)
        .child(Text::new(text.clone()))
        .child(Expanded::new(1, Text::new(vec!["~"; (c.max_height - n) as usize].join("\n")))).into()
})
```

**G24 (compiled).** Same order as twk (replace, hide, then `on_selected`). Act on `Suggestion.text` from the message, not on the composer text, then clear the field. Unavailable commands: filter them out in the suggestions closure; `Suggestion::enabled(false)` (muted, skipped) comes in the gap batch.
```rust
Autocomplete::new(draft.clone(), |q: &str| suggest(q), TextField::new().controller(draft.clone()))
    .trigger("/")
    .on_selected(cx.map(|s: Suggestion| M::Picked(s.text.to_string())))
// update(M::Picked(cmd)) => { run(&cmd); w.draft.reset(""); Rebuild::Yes }
```

**G25.** Popup growing as a bottom pane: `Autocomplete::inline_suggestions()` puts rows in the field's own column. Marker `›`, own match ranges: draw your own list (`SelectionList`/`ListView` with `Text::rich` spans) and keep `Autocomplete` for trigger + query. `enabled` comes in the gap batch.

**G26 (compiled+run).** By design, identical to twk (`_widestLine` drops trailing blanks under word wrap):
```rust
Row::new().child(Text::new("› ").wrap(TextWrap::None)).child(Text::new("X"))  // "› X"
```

**G27 (compiled+run).** `Focus::bind` consumes the key. Use a passing handler: it runs, and the Navigator's back key still pops.
```rust
Focus::new(child).autofocus()
    .on_key(cx.keys(|k| (k == KeyPress::ESC).then_some(Msg::ClearDraft)).passing())
```

**G28 (compiled+run, needs app-dialogs).** No `Dialog` widget needed: an unframed builder, bottom-aligned, no barrier, no fade. For a pane that *replaces* the composer, render it in the page instead: `Column[transcript, if pending { ApprovalPane } else { Composer }]` with the pane's `Focus` autofocused.
```rust
cx.show_dialog::<bool>(|cx| { let th = Theme::of(cx);
    FractionallySizedBox::new(BoxWidget::new().style(th.surface.merge(&th.text))
        .child(Column::new().main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Start).child(Text::new("› Yes, run it"))))
      .width_factor(1.0).alignment(Alignment::BOTTOM_CENTER).into() },
  DialogOptions { alignment: Alignment::BOTTOM_CENTER, barrier_alpha: Some(0),
                  fade_in: Duration::ZERO, ..Default::default() },
  |app, a, _| app.answer = a);
```

**G8.** Codex itself has no typed-ahead guard (approval views take keys at once; `tcflush` only at exit), so parity = nothing; rtwk adds no primitive.
