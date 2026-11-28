<!-- Written by tools/rtwk-port/skill/generate.py from rtwk 5001d2cd; do not edit. Source: tools/rtwk-port/skill/recipes.src.md. -->
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

```rust
// Navigator + PageRoute: push a page, get its result back as a handler call into the model.
// Taken from nav-proto (docs/rtwk-port/research/nav-proto.md, Q1-Q3).
use std::rc::Rc;

use rtwk::app::ViewCx;
use rtwk::focus::Focus;
use rtwk::layout::{Column, CrossAxisAlignment};
use rtwk::navigation::{Navigator, PageRoute};
use rtwk::provider::{Provider, read};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, Handler, Rebuild, State, StatefulWidget, impl_widget};

#[derive(Default)]
pub struct App {
    pub popped: Vec<Option<u32>>,
}

/// The app's handlers, provided under the Navigator: a `cx.on*` handler is never `==` to the last
/// run's, so the provider is told it never notifies and readers `read` the current one.
pub struct Hs {
    pub popped: Handler<Option<u32>>,
}

pub fn view(_app: &App, cx: &mut ViewCx<'_, '_, App>) -> AnyWidget {
    let hs = Hs { popped: cx.on_with(|a, v: Option<u32>, _| a.popped.push(v)) };
    Provider::value_shared(Rc::new(hs))
        .update_should_notify(|_, _| false)
        .child(Navigator::new().home(Home))
        .into()
}

// Home pushes a route and awaits its result; Escape inside the route is the back key (result None).
#[derive(PartialEq)]
pub struct Home;
pub struct HomeState;

#[derive(Clone, PartialEq)]
pub enum HomeMsg {
    Open,
    Popped(Option<u32>),
}

impl StatefulWidget for Home {
    type State = HomeState;
    fn create_state(&self) -> HomeState {
        HomeState
    }
}
impl_widget!(stateful eq Home);

impl State for HomeState {
    type Widget = Home;
    type Msg = HomeMsg;

    fn update(&mut self, msg: HomeMsg, _w: &Home, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            HomeMsg::Open => {
                let result = Navigator::of(cx).push::<u32>(PageRoute::<u32>::new(|_| Detail.into()));
                cx.spawn(async move { Some(HomeMsg::Popped(result.await)) });
            }
            HomeMsg::Popped(v) => {
                let hs = read::<Hs, _>(cx);
                cx.emit(&hs.popped, v);
            }
        }
        Rebuild::No
    }

    fn build(&self, _w: &Home, cx: &mut Cx<'_, Self>) -> AnyWidget {
        // a Column centres its children across: Start puts them at the left edge
        Focus::new(Column::new().cross_axis_alignment(CrossAxisAlignment::Start).child(Text::new("home")))
            .autofocus()
            .bind('o', cx.send(HomeMsg::Open))
            .into()
    }
}

#[derive(PartialEq)]
pub struct Detail;
pub struct DetailState {
    taps: u32,
}

#[derive(Clone, PartialEq)]
pub enum DetailMsg {
    Tap,
    Done,
}

impl StatefulWidget for Detail {
    type State = DetailState;
    fn create_state(&self) -> DetailState {
        DetailState { taps: 0 }
    }
}
impl_widget!(stateful eq Detail);

impl State for DetailState {
    type Widget = Detail;
    type Msg = DetailMsg;

    fn update(&mut self, msg: DetailMsg, _w: &Detail, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            DetailMsg::Tap => {
                self.taps += 1;
                Rebuild::Yes
            }
            DetailMsg::Done => {
                Navigator::pop_with(cx, self.taps); // the awaiting `push` yields Some(taps)
                Rebuild::No
            }
        }
    }

    fn build(&self, _w: &Detail, cx: &mut Cx<'_, Self>) -> AnyWidget {
        Focus::new(Text::new(format!("detail taps={}", self.taps)))
            .autofocus()
            .bind('+', cx.send(DetailMsg::Tap))
            .bind('p', cx.send(DetailMsg::Done))
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rtwk::app::AppView;
    use rtwk_test::{Options, Tester};
    use std::time::Duration;

    fn press(t: &mut Tester, key: &str) {
        t.press(key);
        t.pump_for(Duration::from_millis(100));
    }

    #[test]
    fn the_result_of_a_pushed_route_reaches_the_model() {
        let mut t = Tester::with(Options::new().size(30, 6));
        // the overlay `run_app` puts above the app, as in the tests of rtwk-weather
        t.pump_widget(rtwk::overlay::App::new(AppView::new(App::default(), view)));
        t.pump_for(Duration::from_millis(200));
        t.assert_line("home");
        press(&mut t, "o");
        press(&mut t, "+");
        press(&mut t, "+");
        t.assert_line("detail taps=2");
        press(&mut t, "p");
        t.assert_line("home");
        press(&mut t, "o");
        press(&mut t, "escape");
        t.read_app(|a: &App| assert_eq!(a.popped, [Some(2), None]));
    }
}
```

## show_dialog / close_dialog for a request in the model

A bottom-pane view of the oracle (approval, elicitation, a selection popup) is a dialog opened by the
widget that asks. Here the asker is a presenter under the `Navigator` that follows the model's pending
request id: the dialog is a route of the nearest `Navigator`, so its content sees the providers (a
presenter above the `Navigator` shows it on the root overlay, above every provider: `watch` there panics).
The dialog's answer carries the id, the model drops an answer for a request that is no longer pending, and
the dialog closes itself when the model drops the request. Decide from `did_change_dependencies` values or
ids, never from a `read` in the same flush as the model change (the provider's value lags the model until
the view reruns). A `Column` in a `Dialog` takes `MainAxisSize::Min`, or it overflows its box.

```rust
// show_dialog / close_dialog for a request that lives in the model: a presenter under the Navigator
// opens the dialog for the model's pending id, the dialog answers with the id, and closes itself
// when the model drops the request. Taken from nav-proto (docs/rtwk-port/research/nav-proto.md, Q2, Q5).
use std::rc::Rc;
use rtwk::app::ViewCx;
use rtwk::controls::Button;
use rtwk::focus::Focus;
use rtwk::layout::{Column, MainAxisSize};
use rtwk::navigation::Navigator;
use rtwk::overlay::{Dialog, DialogOptions, close_dialog, show_dialog};
use rtwk::provider::{MultiProvider, Provider, read, select};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, Handler, Rebuild, State, StatefulWidget, impl_widget};

/// What the widgets read: a projection of the model, provided as an `Rc` so that an unchanged
/// slice keeps its address and notifies nobody (`Rc::make_mut` clones only while the provider holds it).
#[derive(Clone, Default, PartialEq)]
pub struct Slice {
    pub pending: Option<u32>, // the request being asked about
}

#[derive(Default)]
pub struct App {
    pub slice: Rc<Slice>,
    pub answers: Vec<(u32, bool)>,
}

pub struct Hs {
    pub answer: Handler<(u32, bool)>,
}

pub fn view(app: &App, cx: &mut ViewCx<'_, '_, App>) -> AnyWidget {
    let hs = Hs {
        answer: cx.on_with(|a, (id, yes): (u32, bool), _| {
            // an answer for a request that is no longer pending is dropped
            if a.slice.pending == Some(id) {
                a.answers.push((id, yes));
                Rc::make_mut(&mut a.slice).pending = None;
            }
        }),
    };
    MultiProvider::new()
        .with(Provider::value_shared(app.slice.clone()))
        .with(Provider::value_shared(Rc::new(hs)).update_should_notify(|_, _| false))
        .child(Navigator::new().home(Home))
        .into()
}

// `Home` holds the presenter: it is below the Navigator, so a dialog is a route of the Navigator
// and its content sees the providers. A presenter above the Navigator would show the dialog on the
// root overlay, above every provider.
#[derive(PartialEq)]
pub struct Home;
pub struct HomeState;
impl StatefulWidget for Home {
    type State = HomeState;
    fn create_state(&self) -> HomeState {
        HomeState
    }
}
impl_widget!(stateful eq Home);
impl State for HomeState {
    type Widget = Home;
    type Msg = ();
    fn build(&self, _w: &Home, _cx: &mut Cx<'_, Self>) -> AnyWidget {
        Column::new().child(Text::new("home")).child(Presenter).into()
    }
}

#[derive(PartialEq)]
pub struct Presenter;
pub struct PresenterState {
    shown: Option<u32>,
    last: Option<u32>, // ids are unique: a request is never shown twice
}

impl StatefulWidget for Presenter {
    type State = PresenterState;
    fn create_state(&self) -> PresenterState {
        PresenterState { shown: None, last: None }
    }
}
impl_widget!(stateful eq Presenter);

impl PresenterState {
    fn sync(&mut self, pending: Option<u32>, cx: &mut Cx<'_, Self>) {
        let (None, Some(id)) = (self.shown, pending) else { return };
        if self.last == Some(id) {
            return;
        }
        self.shown = Some(id);
        self.last = Some(id);
        let answer = show_dialog::<bool, _>(
            cx,
            move |_| Ask { id }.into(),
            DialogOptions { barrier_dismissible: false, ..DialogOptions::default() },
        );
        cx.spawn(async move {
            let _ = answer.await;
            Some(id)
        });
    }
}

impl State for PresenterState {
    type Widget = Presenter;
    type Msg = u32; // the id of the dialog that closed

    // depend with `select`, decide here: not from a `read` in the same flush as a model change
    fn did_change_dependencies(&mut self, _w: &Presenter, cx: &mut Cx<'_, Self>) {
        let pending = select::<Slice, Option<u32>, _>(cx, |s| s.pending);
        self.sync(pending, cx);
    }

    fn update(&mut self, closed: u32, _w: &Presenter, _cx: &mut Cx<'_, Self>) -> Rebuild {
        if self.shown == Some(closed) {
            self.shown = None;
        }
        Rebuild::No
    }

    fn build(&self, _w: &Presenter, _cx: &mut Cx<'_, Self>) -> AnyWidget {
        AnyWidget::default()
    }
}

#[derive(PartialEq)]
pub struct Ask {
    pub id: u32,
}
pub struct AskState;
impl StatefulWidget for Ask {
    type State = AskState;
    fn create_state(&self) -> AskState {
        AskState
    }
}
impl_widget!(stateful eq Ask);

impl State for AskState {
    type Widget = Ask;
    type Msg = bool;

    // the model dropped the request elsewhere: close without an answer
    fn did_change_dependencies(&mut self, w: &Ask, cx: &mut Cx<'_, Self>) {
        let id = w.id;
        if !select::<Slice, bool, _>(cx, move |s| s.pending == Some(id)) {
            close_dialog::<_, bool>(cx, None);
        }
    }

    fn update(&mut self, yes: bool, w: &Ask, cx: &mut Cx<'_, Self>) -> Rebuild {
        let hs = read::<Hs, _>(cx);
        cx.emit(&hs.answer, (w.id, yes)); // the answer carries the id
        close_dialog(cx, Some(yes));
        Rebuild::No
    }

    fn build(&self, w: &Ask, cx: &mut Cx<'_, Self>) -> AnyWidget {
        Focus::new(
            Dialog::new(
                Column::new()
                    .main_axis_size(MainAxisSize::Min)
                    .child(Text::new(format!("Allow request #{}?", w.id)))
                    .child(Button::new("Yes").autofocus(true).on_pressed(cx.send(true)))
                    .child(Button::new("No").on_pressed(cx.send(false))),
            )
            .title("Approval"),
        )
        .can_request_focus(false)
        .bind('y', cx.send(true))
        .bind('n', cx.send(false))
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rtwk::app::AppView;
    use std::time::Duration;
    use rtwk_test::{Options, Tester};

    #[test]
    fn a_pending_request_opens_a_dialog_and_the_answer_reaches_the_model() {
        // room enough: a Column that overflows fails a Tester test
        let mut t = Tester::with(Options::new().size(60, 20));
        t.pump_widget(rtwk::overlay::App::new(AppView::new(App::default(), view)));
        t.pump_for(Duration::from_millis(200));
        assert!(!t.screen().contains("Allow request"));
        t.update_app(|a: &mut App, _| Rc::make_mut(&mut a.slice).pending = Some(7));
        t.pump_for(Duration::from_millis(200));
        assert!(t.screen().contains("Allow request #7?"), "{}", t.screen());
        t.press("y");
        t.pump_for(Duration::from_millis(200));
        assert!(!t.screen().contains("Allow request"));
        t.read_app(|a: &App| assert_eq!(a.answers, [(7, true)]));
    }
}
```

## provider select / watch / read over a model projection

The model is `Rc<Slice>` in the app struct and provided as it is: `Rc::make_mut` clones only when the
provider still holds the old `Rc`, so a slice that did not change keeps its address and notifies nobody.
`select` depends on one projection (a number, an id, an `Option`), `watch` on the whole slice, `read` on
nothing. `select` isolates a widget only if its type is `impl_widget!(stateful eq ..)`; without `eq` the
reconciler rebuilds it on every view run (the test counts the builds).

```rust
// rtwk::provider over a projection of the model: `select` rebuilds a widget only when its projection
// changes, `watch` whenever the whole slice does, `read` never subscribes. A screen widget is
// `impl_widget!(stateful eq ..)`: with equal props the reconciler skips it, so `select` isolates.
// Taken from nav-proto (docs/rtwk-port/research/nav-proto.md, Q4).
use std::cell::Cell;
use std::rc::Rc;

use rtwk::app::ViewCx;
use rtwk::layout::{Column, CrossAxisAlignment};
use rtwk::provider::{Provider, read, select, watch};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

#[derive(Clone, Default, PartialEq)]
pub struct Slice {
    pub count: u32,
    pub other: u32,
}

#[derive(Default)]
pub struct App {
    pub slice: Rc<Slice>, // `Rc::make_mut` on write: an unchanged slice keeps its address
}

thread_local! {
    pub static SELECT_BUILDS: Cell<u32> = const { Cell::new(0) };
    pub static WATCH_BUILDS: Cell<u32> = const { Cell::new(0) };
}

pub fn view(app: &App, _cx: &mut ViewCx<'_, '_, App>) -> AnyWidget {
    Provider::value_shared(app.slice.clone())
        .child(
            Column::new()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .child(Selected)
                .child(Watched),
        )
        .into()
}

#[derive(PartialEq)]
pub struct Selected;
pub struct SelectedState;
impl StatefulWidget for Selected {
    type State = SelectedState;
    fn create_state(&self) -> SelectedState {
        SelectedState
    }
}
impl_widget!(stateful eq Selected);
impl State for SelectedState {
    type Widget = Selected;
    type Msg = ();
    fn build(&self, _w: &Selected, cx: &mut Cx<'_, Self>) -> AnyWidget {
        SELECT_BUILDS.with(|b| b.set(b.get() + 1));
        let count = select::<Slice, u32, _>(cx, |s| s.count); // depends on `count` only
        Text::new(format!("selected count={count}")).into()
    }
}

#[derive(PartialEq)]
pub struct Watched;
pub struct WatchedState;
impl StatefulWidget for Watched {
    type State = WatchedState;
    fn create_state(&self) -> WatchedState {
        WatchedState
    }
}
impl_widget!(stateful eq Watched);
impl State for WatchedState {
    type Widget = Watched;
    type Msg = ();
    fn build(&self, _w: &Watched, cx: &mut Cx<'_, Self>) -> AnyWidget {
        WATCH_BUILDS.with(|b| b.set(b.get() + 1));
        let slice = watch::<Slice, _>(cx); // depends on the whole slice
        let _now = read::<Slice, _>(cx); // `read`: the current value, no dependency (for handlers)
        Text::new(format!("watched count={} other={}", slice.count, slice.other)).into()
    }
    fn update(&mut self, _: (), _w: &Watched, _cx: &mut Cx<'_, Self>) -> Rebuild {
        Rebuild::No
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rtwk::app::AppView;
    use rtwk_test::{Options, Tester};
    use std::time::Duration;

    #[test]
    fn select_rebuilds_for_its_projection_only() {
        let mut t = Tester::with(Options::new().size(40, 4));
        t.pump_widget(rtwk::overlay::App::new(AppView::new(App::default(), view)));
        t.pump_for(Duration::from_millis(100));
        t.assert_line("selected count=0");
        let (s0, w0) = (SELECT_BUILDS.get(), WATCH_BUILDS.get());

        t.update_app(|a: &mut App, _| Rc::make_mut(&mut a.slice).other += 1);
        t.pump_for(Duration::from_millis(100));
        assert_eq!(SELECT_BUILDS.get(), s0, "`other` is not in its projection");
        assert_eq!(WATCH_BUILDS.get(), w0 + 1);
        t.assert_line("watched count=0 other=1");

        t.update_app(|a: &mut App, _| Rc::make_mut(&mut a.slice).count += 1);
        t.pump_for(Duration::from_millis(100));
        assert_eq!(SELECT_BUILDS.get(), s0 + 1);
        t.assert_line("selected count=1");
    }
}
```

## ListView::builder over a model

Rows are built as they scroll into view. Hand the builder an `Rc` of the rows, give each row a key from its
id (state survives inserts above it), set `item_extent` when rows are one height, and give the list a height
(a list in a `Column` goes in `Expanded` or a `SizedBox`). `stick_to_bottom` follows the newest row until the
user scrolls up: the transcript.

```rust
// ListView::builder over a model: rows are built as they scroll into view; a row keeps its state
// by a key made from its id, so it survives inserts above it. `stick_to_bottom` follows the newest
// row until the user scrolls up (a transcript).
// From crates/rtwk/SKILL.md ("Layout, focus, and a form") and the rtwk-gallery ListView page.
use std::rc::Rc;

use rtwk::app::ViewCx;
use rtwk::layout::SizedBox;
use rtwk::scroll::ListView;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Key};

#[derive(Clone, PartialEq)]
pub struct Row {
    pub id: u32,
    pub text: String,
}

#[derive(Default)]
pub struct App {
    pub rows: Rc<Vec<Row>>, // cheap to hand to the builder closure; `Rc::make_mut` on write
}

pub fn view(app: &App, _cx: &mut ViewCx<'_, '_, App>) -> AnyWidget {
    let rows = app.rows.clone();
    let list = ListView::builder(rows.len(), move |i| {
        let row = &rows[i];
        // the key follows the row, not its index
        AnyWidget::from(Text::new(format!("{}: {}", row.id, row.text))).with_key(Key::value(row.id))
    })
    .item_extent(1) // every row one cell high: no measuring, a true scroll extent
    .stick_to_bottom(true);
    // a list takes all the room it gets: give it a height
    SizedBox::new(None, Some(3), Some(list.into())).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rtwk::app::AppView;
    use rtwk_test::{Options, Tester};
    use std::time::Duration;

    #[test]
    fn the_list_follows_the_newest_row() {
        let mut t = Tester::with(Options::new().size(20, 3));
        t.pump_app(AppView::new(App::default(), view));
        for id in 1..=5 {
            t.update_app(move |a: &mut App, _| Rc::make_mut(&mut a.rows).push(Row { id, text: format!("line {id}") }));
            t.pump_for(Duration::from_millis(50));
        }
        t.assert_screen("3: line 3\n4: line 4\n5: line 5");
    }
}
```

## TextArea with a retained controller

The text of an input is widget state: keep a `TextEditingController` where the state should live (a field of
the model or of a per-thread UI struct) and hand clones to `TextArea::controller`. It survives rebuilds, screen
changes and thread switches; `text()`, `set_text`, `clear` and `insert` work on it from handlers. The area
owns the cursor and the editing keys; a plain character is text while it has the focus, so bind commands to a
modified chord.

```rust
// TextArea with a retained controller: the text lives in a `TextEditingController` the model keeps
// (a cheap clone is the handle), so it survives rebuilds, a screen change and a thread switch.
// Read it in a handler with `controller.text()`; write it with `set_text` / `clear`.
// From the rtwk-gallery TextArea page (crates/rtwk-gallery/tests/code/editing_area_submit.rs).
use std::rc::Rc;

use rtwk::app::ViewCx;
use rtwk::editing::{TextArea, TextEditingController};
use rtwk::layout::{Column, CrossAxisAlignment, SizedBox};
use rtwk::widgets::Text;
use rtwk::AnyWidget;

pub struct App {
    pub composer: TextEditingController, // retained: created once, with the model
    pub sent: Vec<String>,
}

impl Default for App {
    fn default() -> Self {
        App { composer: TextEditingController::new(""), sent: Vec::new() }
    }
}

pub fn view(app: &App, cx: &mut ViewCx<'_, '_, App>) -> AnyWidget {
    let area = TextArea::new()
        .controller(app.composer.clone())
        .placeholder("message")
        .autofocus(true)
        .min_lines(1)
        .max_lines(3)
        // Enter submits: the handler gets the text; clear the retained controller there
        .on_submitted(cx.on_with(|a, text: Rc<str>, _| {
            a.sent.push(text.to_string());
            a.composer.clear();
        }));
    Column::new()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .child(SizedBox::new(Some(30), None, Some(area.into())))
        .child(Text::new(format!("sent: {}", app.sent.len())))
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rtwk::app::AppView;
    use rtwk_test::{Options, Tester};
    use std::time::Duration;

    #[test]
    fn enter_submits_the_retained_text() {
        let mut t = Tester::with(Options::new().size(40, 6));
        t.pump_app(AppView::new(App::default(), view));
        t.type_text("hello");
        t.pump_for(Duration::from_millis(50));
        t.read_app(|a: &App| assert_eq!(&*a.composer.text(), "hello"));
        t.press("enter");
        t.pump_for(Duration::from_millis(50));
        t.read_app(|a: &App| {
            assert_eq!(a.sent, ["hello"]);
            assert_eq!(&*a.composer.text(), "");
        });
        t.assert_line("sent: 1");
    }
}
```

## AppCx::channel and spawn_tokio with a ticket

Anything that comes from another thread or the runtime reaches the model as a closure applied on the UI
thread at the next flush, items in order, one view for all of them. Every request gets a ticket from the
model; every result carries it; `apply` drops one that is not current. `AppView::on_start` is where the
channel is made, once. In a test, enter a multi-thread runtime before the app starts and `wait_until`
a real event while pumping.

```rust
// Results from elsewhere, each with its ticket: work started on the runtime (`spawn_tokio`) and items
// pushed from any thread (`AppCx::channel`) both land on the UI thread as a closure over the model.
// The model keeps the current ticket; a result carrying an older one is dropped (R1: stale results).
// Taken from crates/rtwk/SKILL.md ("Tokio and streaming") and nav-proto (`AppView::on_start` + `cx.channel`).
use std::time::Duration;

use rtwk::app::{AppCx, AppView, ViewCx};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Sender};
use rtwk_tokio::TokioApp;

#[derive(Default)]
pub struct App {
    pub ticket: u64,           // the request that is current
    pub answer: Option<String>,
    pub chunks: Vec<String>,   // streamed pieces of the current request
    pub tx: Option<Sender<Event>>, // handed to a backend thread; set once, in `on_start`
}

/// What a backend thread sends: always with the ticket of the request it answers.
pub enum Event {
    Chunk { ticket: u64, text: String },
}

impl App {
    pub fn apply(&mut self, ev: Event) {
        match ev {
            Event::Chunk { ticket, text } if ticket == self.ticket => self.chunks.push(text),
            Event::Chunk { .. } => {} // stale: a newer request replaced it
        }
    }

    /// A new request: bump the ticket, drop what the last one left, start the work.
    pub fn ask(&mut self, cx: &mut AppCx<'_, '_, App>, question: &'static str) {
        self.ticket += 1;
        let ticket = self.ticket;
        self.chunks.clear();
        self.answer = None;
        cx.spawn_tokio(
            async move {
                tokio::time::sleep(Duration::from_millis(5)).await;
                format!("answer to {question}")
            },
            move |app, answer, _| {
                if app.ticket == ticket {
                    app.answer = Some(answer);
                } // else: stale, dropped
            },
        );
    }
}

pub fn app_view() -> AppView<App> {
    AppView::new(App::default(), view).on_start(|a, cx| {
        a.tx = Some(cx.channel(|a, ev: Event, _| a.apply(ev)));
    })
}

pub fn view(app: &App, _cx: &mut ViewCx<'_, '_, App>) -> AnyWidget {
    let text = match &app.answer {
        Some(answer) => format!("done: {answer}"),
        None => format!("waiting ({} chunks)", app.chunks.len()),
    };
    Text::new(text).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rtwk_test::{Options, Tester, wait_until};

    #[test]
    fn a_stale_result_is_dropped_and_the_current_one_lands() {
        let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
        let _entered = rtwk_tokio::enter(&runtime);
        let mut t = Tester::with(Options::new().size(40, 3));
        t.pump_app(app_view());
        t.update_app(|a: &mut App, cx| a.ask(cx, "first"));
        t.update_app(|a: &mut App, cx| a.ask(cx, "second")); // ticket 2 replaces ticket 1
        let tx = t.read_app(|a: &App| a.tx.clone().unwrap());
        assert!(tx.send(Event::Chunk { ticket: 1, text: "old".into() }).is_ok());
        assert!(tx.send(Event::Chunk { ticket: 2, text: "new".into() }).is_ok());
        wait_until("the second answer", || {
            t.pump();
            t.read_app(|a: &App| a.answer.is_some())
        });
        t.read_app(|a: &App| {
            assert_eq!(a.answer.as_deref(), Some("answer to second"));
            assert_eq!(a.chunks, ["new"]);
        });
        t.assert_line("done: answer to second");
    }
}
```

## A screen test with a snapshot

`Tester::with(Options::new().size(w, h))` is the terminal; `pump_app` mounts the app; `press`, `type_text`,
`click_on` are input, followed by `pump` (or `pump_for(duration)` to move the fake clock). `assert_screen`
compares the rows from the top, without trailing blanks or empty rows; `assert_screen_file(path)` compares a
stored file with styles; `screen()` returns the text for your own comparison. The tui-rtwk replay helper that
compares a screen with a codex-tui `.snap` is built on `screen()`.

```rust
// A screen test with a snapshot: the app on an in-memory terminal at a given size, input as keys,
// the screen compared with the expected text. `assert_screen` takes the rows from the top without
// trailing blanks or empty rows (what `insta::assert_snapshot!` of `screen()` would store); for a
// stored expected file use `assert_screen_file`. Pump after input, and move the fake clock with
// `pump_for` past animations (a dialog fades in over ~160 ms).
// From crates/rtwk/SKILL.md ("App state, handlers, and testing") and rtwk-weather's tests.
use rtwk::app::ViewCx;
use rtwk::controls::Button;
use rtwk::layout::{Border, BorderType, BoxWidget, Column, CrossAxisAlignment};
use rtwk::widgets::Text;
use rtwk::AnyWidget;

#[derive(Default)]
pub struct Counter {
    pub count: u32,
}

pub fn view(app: &Counter, cx: &mut ViewCx<'_, '_, Counter>) -> AnyWidget {
    let body = Column::new()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .child(Text::new(format!("Count: {}", app.count)))
        .child(Button::new("Increment").autofocus(true).on_pressed(cx.on(|a, _| a.count += 1)));
    BoxWidget::new()
        .border(Border::all(rtwk::style::Style::new()).kind(BorderType::Rounded))
        .child(body)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rtwk::app::AppView;
    use rtwk_test::{Options, Tester};

    #[test]
    fn enter_on_the_focused_button_counts() {
        let mut t = Tester::with(Options::new().size(18, 4));
        t.pump_app(AppView::new(Counter::default(), view));
        t.press("enter");
        t.pump();
        t.assert_screen(
            "╭────────────────╮\n\
             │Count: 1        │\n\
             │ Increment      │\n\
             ╰────────────────╯",
        );
        t.read_app(|a: &Counter| assert_eq!(a.count, 1));
    }
}
```
