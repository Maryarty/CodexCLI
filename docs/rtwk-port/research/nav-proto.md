# nav-proto: Navigator + providers + dialogs inside an AppView view

Scratch crate: `$TMPDIR/codexcli-research/nav-proto/` (src/lib.rs 497 lines, tests/nav.rs 304).
Run: `cd nav-proto && ~/.cache/rtwk/acargo test -q --test nav` -> 14 passed, no network needed beyond rtwk's lockfile.
Harness: `Tester::pump_widget(overlay::App::new(AppView::new(app, view).on_start(..)))` as rtwk-weather does.
Bite check: provider fed a frozen `Slice::default()` instead of `app.slice` -> 9 of 14 tests fail (Q1, Q2, Q4).

## Shape that works (src/lib.rs:125-175)
```rust
fn view(app: &App, cx: &mut ViewCx<App>) -> AnyWidget {
    let hs = Hs { bump: cx.on(|a, _| ..), answer: cx.on_with(|a, (id, yes): (u32, bool), cx| a.answer(id, yes, cx)), .. };
    MultiProvider::new()
        .with(Provider::value_shared(app.slice.clone()))            // model: Rc<Slice>, Rc::make_mut on write
        .with(Provider::value_shared(Rc::new(hs)).update_should_notify(|_, _| false))  // handlers
        .child(Navigator::new().home(Home { label: app.label, .. }))
        .into()
}
// backend: AppView::on_start(|a, cx| a.tx = Some(cx.channel(|a, ev: Ev, _| a.apply(ev))))   (app.rs:279, :1151)
// push + pop result, in a State::update of a widget under the Navigator:
let result = Navigator::of(cx).push::<u32>(PageRoute::<u32>::new(|_| DetailSelect.into()));
cx.spawn(async move { Some(HomeMsg::Popped(result.await)) });   // later: cx.emit(&read::<Hs,_>(cx).popped, v)
// pop with value: Navigator::pop_with(cx, v);   Escape = back key, result None
```
The slice is `Rc<Slice>` in the model; `Rc::make_mut` clones only when the provider still holds the old Rc, so an
unchanged slice keeps its address and notifies nobody (`value_shared` compares by address, inherited_provider.rs:1213).

## Q1 PASS (q1_*; also with a backend channel item)
- Pushed route with `watch::<Slice>` / `select::<Slice,u32>` shows the new value after a handler change (`cx.on`) AND after an
  `AppCx::channel` item. Two channel items in one flush give one view run (asserted: `view` delta 1, last value shown).
- Route stack survives the rerun: route still on top, its local `State` (`local=2`) kept.
- Home gets new props while covered (`home:0` -> `home:1` after a handler bumps `app.label`): `Navigator.home` is not stale,
  because the home route's builder reads `config.home` at each build (navigator.rs:1335-1345; config replaced in
  `did_update_widget` navigator.rs:1992-2003).
- A popped route's State dies; a new push starts at `local=0`.

## Q2 PASS, with one placement rule (q2_*)
- Presenter = StatefulWidget under the Navigator (child of Home). `did_change_dependencies` does
  `select::<Slice,Option<u32>>(cx, |s| s.pending)`; on Some it calls `show_dialog::<bool,_>(cx, ..)` and `cx.spawn`s the await.
  Works: depending (`select`) and `show_dialog` / `close_dialog` all work inside `did_change_dependencies`
  (widget.rs:537 documents the dependency staying registered). No need to call them from `build`.
- Dialog content: `did_change_dependencies` does `select(|s| s.pending == Some(id))`, false -> `close_dialog::<_,bool>(cx, None)`.
  Resolved-elsewhere closes the dialog (marker `dialog_self_close` = 1), no answer recorded, focus back on Home
  (Home's `o` binding works), also over a pushed route (route's `+` works after the dialog).
- Answer carries the id: dialog emits `hs.answer (id, yes)` then `close_dialog(Some(yes))`; the model drops a stale answer
  (`app.stale` +1, `cx.unchanged()` -> no view run). The Probe used handlers captured at its `init_state` before many
  reruns, so handlers stay valid across reruns (they route by the root element, handler.rs:140).
- Replace: Resolve(1)+Request(2) in one flush closes dialog 1, then opens dialog 2.
- RULE (gap 1): the dialog is a route of the nearest Navigator, so its content sees the providers
  (dialog.rs:312-331). A presenter ABOVE the Navigator gets no navigator link; `show_dialog` falls back to the Overlay of
  `overlay::App` (dialog.rs:333-352), which sits above the AppView root, hence above the MultiProvider: the dialog content
  cannot find `Provider<Slice>` (`watch`/`select`/`read` panic "Could not find the correct Provider"). Test
  `q2_dialog_shown_without_a_navigator_above_is_outside_the_app_providers`. So: presenter below the Navigator.

## Q3 PASS (q3_pop_result_reaches_the_model_through_a_handler)
`push::<u32>` + `result.await` in `cx.spawn` -> `Popped(Some(3))` -> emitted handler -> `app.popped == [Some(3)]`;
Escape back key gives `None` (`[Some(3), None]`). Home screen returns.

## Q4 PASS: select isolates, with a condition (q4_*; run with --nocapture)
Counts of Detail `build` per model change (route open on top):
| widget | unrelated change (`other`) | watched change (`count`) | Home-prop change |
|---|---|---|---|
| `select(count)`, widget `eq` | 0 builds | 1 | 0 |
| `watch<Slice>`, widget `eq` | 1 per change (3 changes -> 3) | 1 | 0 |
| `select(count)`, widget WITHOUT `eq` | 1 per view run (2 -> 2) | 1 | 1 |
Each handler change = 1 view run and 1 call of the route builder closure (Navigator `did_update_widget` marks every on-stage
route's page `needs_build`, navigator.rs:1992-2003; stage = routes from the topmost opaque one up). The route's child is
skipped only if its widget type is `impl_widget!(stateful eq ..)` (PartialEq) and equal. So `select` isolates as in twk
only when screen widgets carry `eq`. Allocation counts not measured.

## Q5 what breaks / holds
- Providers re-made on each run: NOT a problem. The `Provider` element is reused (same type, same position); dependents kept.
- `cx.on` handlers are never `==` across runs (CallMaker::same = false, handler.rs:147; test q5_cx_on_handlers_are_not_equal).
  A `Provider::value(hs)` / `compare_by_value` of the handlers would notify every `watch`er on every run: use
  `.update_should_notify(|_, _| false)` (inherited_provider.rs:1265), or stable `#[route]` handlers (equal across runs).
  Readers must `read` the handlers (not hold a copy across a rerun that changes captured values).
- Provider snapshot lags the model (gap 2): the provider value changes only when the view reruns. In my first presenter,
  `PMsg::Closed` did `read::<Slice>().pending` right after the answer handler had cleared `pending` in the model, still saw
  `Some(id)` (marker `closed_read_stale`, asserted = 1) and opened the dialog again (`dialog_open` 2). Fix used: remember the
  last shown request id (ids are unique) and never reopen it. Rule: never decide from a `read` inside the same flush as a
  model change; decide from `did_change_dependencies` values or ids.
- Changing the MultiProvider list (a provider added in front) remounts an unkeyed Navigator: the route stack is lost
  (test q5_changing_the_provider_list_remounts..). `.with_key(GlobalKey)` on the Navigator keeps the stack and the
  routes' local State across adding/removing a provider (test q5_a_global_key..). Keep the provider list fixed or key the Navigator.
- `show_dialog` needs `&Cx<S>`: `AppCx` (what `cx.on` handlers get) exposes no inner `Cx` (app.rs pub fn list), so a dialog
  cannot be opened straight from a model handler; it needs a widget (the presenter) that reacts to model state.
  A `ViewCx` has none either.
- Dialog layout: a Column with two Buttons in the 60x20 test terminal logged "RenderFlex overflowed by 1 row" (cosmetic,
  dialog content needs `main_axis_size(Min)` as delete.rs does).

## rtwk gaps (3; none blocks the design)
1. Overlay fallback is above the AppView providers. Repro: `show_dialog` from a widget with no Navigator above, content does
   `watch::<T>` -> panic (nav.rs:266). Workaround: presenter under the Navigator; or give `overlay::App` content its providers.
2. No built-in way to read the current model from a widget in the same flush as a handler change (provider snapshot is
   per view run). Repro: handler clears `pending`; same-flush `read::<Slice>` still returns old value (q2 first test).
3. A change of the provider list or depth silently drops the Navigator stack (no warning). Repro: q5_changing_the_provider_list.

## Verdict
Navigator at root of an AppView view with MultiProvider above it works: the model stays a plain struct, screens are
`watch`/`select` consumers, backend events via `AppCx::channel`, dialogs via a presenter widget + `show_dialog`, results
via `push::<T>` + handlers. Conditions: widgets `eq`, presenter below the Navigator, fixed provider list (or keyed Navigator),
handlers provided with `update_should_notify(false)`, requests identified by unique ids.
