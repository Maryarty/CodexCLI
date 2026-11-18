# Recipes for the tui-rtwk port

Each recipe is a whole module with its test, compiled and run against the pin by
`tools/rtwk-port/skill/compile-snippets.sh` (7 snippets, 7 tests, green at the pin). They are the manner
of the port (docs/rtwk-port/architecture.md, ADR-002, ADR-003): a plain model, handlers made in the view,
widgets that read through providers, results that carry their ids. Copy one, rename it, keep its test.

The test harness in every recipe is `rtwk_test::Tester` (`guides/testing-api.md`); the contexts are in
`guides/app-api.md`; one widget's own options are in `widgets/<group>.md`.

## Navigator and PageRoute: push a page, get its result

A screen that the oracle shows as an overlay (pager, diff, analytics) is a `PageRoute` pushed on the
`Navigator`; it comes back as a result, which a handler turns into a model change. The app's handlers are
provided under the `Navigator` with `update_should_notify(|_, _| false)`: a `cx.on*` handler is never equal
to the one of the last view run, so a notifying provider would rebuild every reader on every run. Readers
`read` the current handler. Escape inside a route is the back key and yields `None`.

{{snippet nav_push}}

## show_dialog / close_dialog for a request in the model

A bottom-pane view of the oracle (approval, elicitation, a selection popup) is a dialog opened by the
widget that asks. Here the asker is a presenter under the `Navigator` that follows the model's pending
request id: the dialog is a route of the nearest `Navigator`, so its content sees the providers (a
presenter above the `Navigator` shows it on the root overlay, above every provider: `watch` there panics).
The dialog's answer carries the id, the model drops an answer for a request that is no longer pending, and
the dialog closes itself when the model drops the request. Decide from `did_change_dependencies` values or
ids, never from a `read` in the same flush as the model change (the provider's value lags the model until
the view reruns). A `Column` in a `Dialog` takes `MainAxisSize::Min`, or it overflows its box.

{{snippet dialog_request}}

## provider select / watch / read over a model projection

The model is `Rc<Slice>` in the app struct and provided as it is: `Rc::make_mut` clones only when the
provider still holds the old `Rc`, so a slice that did not change keeps its address and notifies nobody.
`select` depends on one projection (a number, an id, an `Option`), `watch` on the whole slice, `read` on
nothing. `select` isolates a widget only if its type is `impl_widget!(stateful eq ..)`; without `eq` the
reconciler rebuilds it on every view run (the test counts the builds).

{{snippet provider_select}}

## ListView::builder over a model

Rows are built as they scroll into view. Hand the builder an `Rc` of the rows, give each row a key from its
id (state survives inserts above it), set `item_extent` when rows are one height, and give the list a height
(a list in a `Column` goes in `Expanded` or a `SizedBox`). `stick_to_bottom` follows the newest row until the
user scrolls up: the transcript.

{{snippet list_builder}}

## TextArea with a retained controller

The text of an input is widget state: keep a `TextEditingController` where the state should live (a field of
the model or of a per-thread UI struct) and hand clones to `TextArea::controller`. It survives rebuilds, screen
changes and thread switches; `text()`, `set_text`, `clear` and `insert` work on it from handlers. The area
owns the cursor and the editing keys; a plain character is text while it has the focus, so bind commands to a
modified chord.

{{snippet text_area}}

## AppCx::channel and spawn_tokio with a ticket

Anything that comes from another thread or the runtime reaches the model as a closure applied on the UI
thread at the next flush, items in order, one view for all of them. Every request gets a ticket from the
model; every result carries it; `apply` drops one that is not current. `AppView::on_start` is where the
channel is made, once. In a test, enter a multi-thread runtime before the app starts and `wait_until`
a real event while pumping.

{{snippet channel_ticket}}

## A screen test with a snapshot

`Tester::with(Options::new().size(w, h))` is the terminal; `pump_app` mounts the app; `press`, `type_text`,
`click_on` are input, followed by `pump` (or `pump_for(duration)` to move the fake clock). `assert_screen`
compares the rows from the top, without trailing blanks or empty rows; `assert_screen_file(path)` compares a
stored file with styles; `screen()` returns the text for your own comparison. The tui-rtwk replay helper that
compares a screen with a codex-tui `.snap` is built on `screen()`.

{{snippet tester_snapshot}}
