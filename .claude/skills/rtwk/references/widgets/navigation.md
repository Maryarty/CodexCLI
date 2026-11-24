<!-- Written by `cargo xtask skill-references` from docs/widgets; do not edit. -->
# Navigation

Routes, a scaffold of panes, steppers, breadcrumbs and pages.

[All widgets](../widgets.md)

A Code is the body of the gallery's preview: it names widgets by the last segment of the paths under API (`use` them) and may use `cx` or values of the gallery's page.

## Navigator

Screens on a stack: a route pushed covers the one under it and takes the focus, Escape or Back pops it.

- `rtwk::navigation::Navigator` (struct) in `crates/rtwk/src/navigation/navigator.rs:1722`: Shows a stack of Routes, the last pushed in front: its first, which initial_route names (DEFAULT_ROUTE_NAME by default), and those pushed on it (NavigatorState::push, NavigatorState::push_named) until they are popped (NavigatorState::pop).

### Code

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Screen {
    Home,
    Item(usize),
    Settings,
}

// A route fades in over 220 ms and out over 180 ms.
fn page_route(screen: Screen) -> PageRoute<()> {
    PageRoute::new(move |_| DemoRoute { screen }.into())
        .transition(Duration::from_millis(220))
        .reverse_transition(Duration::from_millis(180))
}

#[derive(Debug, PartialEq)]
struct DemoRoute {
    screen: Screen,
}

struct DemoRouteState;

#[derive(Clone, PartialEq)]
enum RouteMsg {
    OpenItem(usize),
    Named(&'static str),
    Back,
}

impl StatefulWidget for DemoRoute {
    type State = DemoRouteState;
    fn create_state(&self) -> DemoRouteState {
        DemoRouteState
    }
}

impl State for DemoRouteState {
    type Widget = DemoRoute;
    type Msg = RouteMsg;

    fn update(&mut self, msg: RouteMsg, _: &DemoRoute, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            RouteMsg::OpenItem(item) => {
                let _result = Navigator::push::<(), _>(cx, page_route(Screen::Item(item)));
            }
            RouteMsg::Named(name) => {
                let _result = Navigator::push_named::<(), _>(cx, name);
            }
            RouteMsg::Back => {
                Navigator::maybe_pop(cx);
            }
        }
        Rebuild::Yes
    }

    fn build(&self, widget: &DemoRoute, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let theme = Theme::of(cx);
        let back = || Button::new("Back").autofocus(true).on_pressed(cx.send(RouteMsg::Back));
        let body = match widget.screen {
            Screen::Home => column![
                Text::new("Home"),
                Button::new("Open item 7").on_pressed(cx.send(RouteMsg::OpenItem(7))),
                Button::new("Open /settings").on_pressed(cx.send(RouteMsg::Named("/settings"))),
            ],
            Screen::Item(item) => column![
                Text::new(format!("Item {item}")).style(theme.primary.clone()),
                row![
                    back(),
                    Button::new(format!("Open item {}", item + 1))
                        .on_pressed(cx.send(RouteMsg::OpenItem(item + 1))),
                ].spacing(2),
            ],
            Screen::Settings => column![
                Text::new("Settings").style(theme.primary.clone()),
                back(),
            ],
        };
        body.cross_axis_alignment(CrossAxisAlignment::Start).spacing(1)
        .into()
    }
}
impl_widget!(stateful eq DemoRoute);

fn navigator_demo(cx: &mut Cx<'_>) -> AnyWidget {
    let navigator = Navigator::new()
        .home(DemoRoute { screen: Screen::Home })
        .back_key(Some("escape"))
        .on_generate_route(move |settings| {
            (settings.name.as_deref() == Some("/settings")).then(|| {
                page_route(Screen::Settings)
                    .settings(settings.clone())
                    .into_route()
            })
        });
    SizedBox::new(
        Some(44),
        Some(9),
        Some(
            BoxWidget::new()
                .border(Border::all(Theme::of(cx).border.clone()).kind(BorderType::Rounded))
                .padding(EdgeInsets::x(1))
                .child(navigator)
                .into(),
        ),
    )
    .into()
}

```

### Options

| option | values | starts at |
| --- | --- | --- |
| back key | escape, backspace, none | escape |

## PopScope, NavigatorObserver

A route that asks before it is popped, by Escape or by the back key, a dialog too; and an observer told of every push, pop and removal.

- `rtwk::navigation::PopScope` (struct) in `crates/rtwk/src/navigation/navigator.rs:940`: Refuses or intercepts a pop of the route it is in: while can_pop is false, the back key and NavigatorState::maybe_pop leave the route, even the first one, whose back key otherwise goes on past the navigator; they take the key, and on_pop_invoked_with_result is told that it did not pop, with the result asked for.
- `rtwk::navigation::NavigatorObserver` (trait) in `crates/rtwk/src/navigation/navigator.rs:878`: Told by the Navigators it is given (Navigator::observers) of each route pushed, popped, replaced and removed, after the change and before the next build shows it.

### Code

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Screen {
    Notes,
    Edit,
    DiscardDialog,
}

// Says whether the editor asks before it pops, and whether the dialog does not.
#[derive(Debug, PartialEq)]
struct Asking {
    enabled: bool,
    keep: bool,
    child: AnyWidget,
}

impl InheritedWidget for Asking {
    fn child(&self) -> &AnyWidget {
        &self.child
    }
    fn update_should_notify(&self, old: &Self) -> bool {
        self.enabled != old.enabled || self.keep != old.keep
    }
}
impl_widget!(inherited Asking);

// Heard publishes the pushed, popped and removed route names, with a serial,
// through its ValueNotifier.
struct Heard {
    hosts: ControllerHost<NavigatorState>,
    last: Rc<ValueNotifier<(String, u64)>>,
}

impl Heard {
    fn record(&self, operation: &str, route: &Route) {
        let name = route.settings().name.as_deref().unwrap_or("a dialog");
        self.last
            .set_value((format!("{operation} {name}"), self.last.value().1 + 1));
    }
}

impl NavigatorObserver for Heard {
    fn hosts(&self) -> &ControllerHost<NavigatorState> {
        &self.hosts
    }
    fn did_push(&self, route: &Route, _: Option<&Route>) {
        self.record("pushed", route);
    }
    fn did_pop(&self, route: &Route, _: Option<&Route>) {
        self.record("popped", route);
    }
    fn did_remove(&self, route: &Route, _: Option<&Route>) {
        self.record("removed", route);
    }
}

#[derive(Debug, PartialEq)]
struct DemoRoute {
    screen: Screen,
}

struct DemoRouteState {
    changed: bool,
    asking: bool,
}

#[derive(Clone, PartialEq)]
enum RouteMsg {
    OpenEditor,
    Back,
    Changed(bool),
    PopInvoked(bool),
    CloseDialog(bool),
    ConfirmDiscard(bool),
}

impl StatefulWidget for DemoRoute {
    type State = DemoRouteState;
    fn create_state(&self) -> DemoRouteState {
        DemoRouteState {
            changed: false,
            asking: false,
        }
    }
}

impl State for DemoRouteState {
    type Widget = DemoRoute;
    type Msg = RouteMsg;

    fn update(&mut self, msg: RouteMsg, _: &DemoRoute, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            RouteMsg::Changed(changed) => self.changed = changed,
            // A refused pop asks; the editor pops only after a confirmed discard.
            RouteMsg::PopInvoked(false) if !self.asking => {
                self.asking = true;
                let keep = cx
                    .get_inherited::<Asking, _>(|scope| scope.keep)
                    .unwrap_or(false);
                let dialog = show_dialog::<bool, _>(
                    cx,
                    move |_| {
                        let content = DemoRoute { screen: Screen::DiscardDialog };
                        content.into()
                    },
                    DialogOptions::default(),
                );
                cx.spawn(async move {
                    Some(RouteMsg::ConfirmDiscard(dialog.await.unwrap_or(false)))
                });
                return Rebuild::No;
            }
            RouteMsg::PopInvoked(_) => return Rebuild::No,
            RouteMsg::CloseDialog(discard) => {
                close_dialog(cx, Some(discard));
                return Rebuild::No;
            }
            RouteMsg::ConfirmDiscard(discard) => {
                self.asking = false;
                if discard {
                    Navigator::pop(cx);
                }
                return Rebuild::No;
            }
            RouteMsg::Back => {
                Navigator::maybe_pop(cx);
                return Rebuild::No;
            }
            RouteMsg::OpenEditor => {
                let _result = Navigator::push_named::<(), _>(cx, "/edit");
                return Rebuild::No;
            }
        }
        Rebuild::Yes
    }

    fn build(&self, widget: &DemoRoute, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let theme = Theme::of(cx);
        let back = || Button::new("Back").autofocus(true).on_pressed(cx.send(RouteMsg::Back));
        let content = match widget.screen {
            Screen::Notes => column![
                Text::new("Notes"),
                Button::new("Edit the note").on_pressed(cx.send(RouteMsg::OpenEditor)),
            ],
            Screen::Edit => column![
                Text::new("Edit the note").style(theme.primary.clone()),
                Checkbox::new(self.changed)
                    .label("changed")
                    .on_changed(cx.map(RouteMsg::Changed)),
                back(),
            ],
            Screen::DiscardDialog => {
                return Dialog::new(
                    column![
                        Text::new("The note was changed."),
                        row![
                            Button::new("Discard")
                                .autofocus(true)
                                .on_pressed(cx.send(RouteMsg::CloseDialog(true))),
                            Button::new("Keep").on_pressed(cx.send(RouteMsg::CloseDialog(false))),
                        ]
                        .main_axis_size(MainAxisSize::Min)
                        .spacing(2),
                    ]
                    .main_axis_size(MainAxisSize::Min)
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .spacing(1),
                )
                .title("Discard the changes?")
                .into();
            }
        }
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .spacing(1);
        if widget.screen == Screen::Edit
            && cx.depend::<Asking, _>(|scope| scope.enabled).unwrap_or(false)
        {
            return PopScope::new(content)
                .can_pop(!self.changed)
                .on_pop_invoked_with_result(cx.map(|p: PopInvoked| RouteMsg::PopInvoked(p.did_pop)))
                .into();
        }
        content
        .into()
    }
}
impl_widget!(stateful eq DemoRoute);

struct NotesApp;

struct NotesAppState {
    heard: Rc<Heard>,
}

impl StatefulWidget for NotesApp {
    type State = NotesAppState;
    fn create_state(&self) -> NotesAppState {
        NotesAppState {
            heard: Rc::new(Heard {
                hosts: ControllerHost::new(),
                last: Rc::new(ValueNotifier::new((String::new(), 0))),
            }),
        }
    }
}

impl State for NotesAppState {
    type Widget = NotesApp;
    type Msg = ();

    fn build(&self, _: &NotesApp, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let navigator = Navigator::new()
            .home(DemoRoute { screen: Screen::Notes })
            .route("/edit", move |_| DemoRoute { screen: Screen::Edit }.into())
            .observer(self.heard.clone());
        let child = Asking { enabled: true, keep: false, child: navigator.into() };
        let last = self.heard.last.clone();
        let heard = ListenableBuilder::new(last.clone(), move |_, _| {
            let (event, _) = last.value();
            Text::new(format!("heard: {event}")).into()
        });
        let framed = SizedBox::new(
            Some(44),
            Some(9),
            Some(
                BoxWidget::new()
                    .border(Border::all(Theme::of(cx).border.clone()).kind(BorderType::Rounded))
                    .padding(EdgeInsets::x(1))
                    .child(child)
                    .into(),
            ),
        );
        column![framed, heard]
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .spacing(1)
        .into()
    }
}
impl_widget!(stateful NotesApp);

```

### Options

| option | values | starts at |
| --- | --- | --- |
| PopScope | false, true | true |
| esc in the dialog | closes it, is kept | closes it |

## Breadcrumbs

Where a screen is in a hierarchy, each level a link up: the arrows and Enter or a click go to one, and the middle collapses into … where the row does not fit.

- `rtwk::navigation::Breadcrumbs` (struct) in `crates/rtwk/src/navigation/breadcrumbs.rs:85`: The path of items to where a screen is, root first, on one row: `Root › Projects › my_app › lib › src › widgets`, each but the current one a link that calls on_selected with its index.

### Code

```rust
const PATH: [&str; 6] = ["rtwk", "crates", "rtwk-gallery", "src", "pages", "menus"];

struct Crumbs;

struct CrumbsState {
    at: usize,
}

impl StatefulWidget for Crumbs {
    type State = CrumbsState;
    fn create_state(&self) -> CrumbsState {
        CrumbsState { at: PATH.len() - 1, }
    }
}

impl State for CrumbsState {
    type Widget = Crumbs;
    type Msg = usize;

    fn update(&mut self, msg: usize, _: &Crumbs, _: &mut Cx<'_, Self>) -> Rebuild {
        // A selected crumb becomes the current one.
        self.at = msg;
        Rebuild::Yes
    }

    fn build(&self, _: &Crumbs, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let crumbs = Breadcrumbs::new(PATH)
            .current(self.at)
            .keep_leading(1)
            .separator(" › ")
            .on_selected(cx.map(|index| index));
        column![
            SizedBox::new(Some(36), None, Some(crumbs.into())),
            Text::new(format!("at: {}", PATH[..=self.at].join("/"))),
        ]
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .spacing(1)
        .into()
    }
}
impl_widget!(stateful Crumbs);

```

### Options

| option | values | starts at |
| --- | --- | --- |
| width | 12 cells, 13 cells, 14 cells, 15 cells, 16 cells, 17 cells, 18 cells, 19 cells, 20 cells, 21 cells, 22 cells, 23 cells, 24 cells, 25 cells, 26 cells, 27 cells, 28 cells, 29 cells, 30 cells, 31 cells, 32 cells, 33 cells, 34 cells, 35 cells, 36 cells, 37 cells, 38 cells, 39 cells, 40 cells, 41 cells, 42 cells, 43 cells, 44 cells, 45 cells, 46 cells, 47 cells, 48 cells, 49 cells, 50 cells | 36 cells |
| leading kept | 1, 0, 2 | 1 |
| separator | ›, / | › |

## Pagination

The pages of a long list on one row, the current one among its siblings: the arrows, PageUp and PageDown, a page number typed or a click go to another.

- `rtwk::navigation::Pagination` (struct) in `crates/rtwk/src/navigation/pagination.rs:89`: The page_count pages of a list loaded a page at a time, from an API or a database, on one row, the page shown among them: `« First ‹ Prev 1 [2] 3 4 5 … 42 Next › Last »  (21–40 of 840)`.

### Code

```rust
struct Pages;

struct PagesState {
    // Initially page = 1, the second page; the index is zero-based.
    page: usize,
}

impl StatefulWidget for Pages {
    type State = PagesState;
    fn create_state(&self) -> PagesState {
        PagesState { page: 1, }
    }
}

impl State for PagesState {
    type Widget = Pages;
    type Msg = usize;

    fn update(&mut self, msg: usize, _: &Pages, _: &mut Cx<'_, Self>) -> Rebuild {
        self.page = msg;
        Rebuild::Yes
    }

    fn build(&self, _: &Pages, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let pages = Pagination::new(self.page, 42)
            .siblings(1)
            .show_first_last(true)
            .show_prev_next(true)
            .on_changed(cx.map(|page| page));
        column![
            SizedBox::new(Some(56), None, Some(pages.into())),
            Text::new(format!("page: {} of 42", self.page + 1)),
        ]
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .spacing(1)
        .into()
    }
}
impl_widget!(stateful Pages);

```

### Options

| option | values | starts at |
| --- | --- | --- |
| width | 16 cells, 17 cells, 18 cells, 19 cells, 20 cells, 21 cells, 22 cells, 23 cells, 24 cells, 25 cells, 26 cells, 27 cells, 28 cells, 29 cells, 30 cells, 31 cells, 32 cells, 33 cells, 34 cells, 35 cells, 36 cells, 37 cells, 38 cells, 39 cells, 40 cells, 41 cells, 42 cells, 43 cells, 44 cells, 45 cells, 46 cells, 47 cells, 48 cells, 49 cells, 50 cells, 51 cells, 52 cells, 53 cells, 54 cells, 55 cells, 56 cells | 56 cells |
| siblings | 1, 0, 2 | 1 |
| first and last | false, true | true |
| previous and next | false, true | true |
| range | false, true | false |

## Stepper

A wizard of steps under their headers, each left forward once its validation takes it: Next or PageDown validates, Back or PageUp goes back, a header clicked goes to it.

- `rtwk::navigation::Stepper` (struct) in `crates/rtwk/src/navigation/stepper.rs:979`: A wizard of steps, one shown at a time under the headers of all of them, with Back and Next under it: installers, `init`-style setups, deploy flows.

### Code

```rust
use std::future::poll_fn;
use std::task::{Poll, Waker};

struct ValidationTimer {
    timer: Timer,
    completion: Rc<Completion>,
}

#[derive(Default)]
struct Completion {
    ready: Cell<bool>,
    waker: RefCell<Option<Waker>>,
}

impl Completion {
    fn finish(&self) {
        self.ready.set(true);
        let waker = self.waker.take();
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

// Starts an 800 ms app timer and returns Validation::later: the Checked
// message wakes it, and it refuses when no package is selected.
fn check_packages(
    binding: &AppBinding,
    checked: &Handler<()>,
    pending: &RefCell<Option<ValidationTimer>>,
    selected: Rc<Cell<[bool; 3]>>,
) -> Validation {
    let completion = Rc::new(Completion::default());
    let timer = binding.start_timer(Duration::from_millis(800), checked.clone());
    let old = pending.replace(Some(ValidationTimer {
        timer,
        completion: completion.clone(),
    }));
    if let Some(old) = old {
        old.timer.cancel();
        old.completion.finish();
    }
    Validation::later(poll_fn(move |cx| {
        if completion.ready.get() {
            Poll::Ready(
                (!selected.get().into_iter().any(|on| on)).then(|| "Pick one at least".to_owned()),
            )
        } else {
            *completion.waker.borrow_mut() = Some(cx.waker().clone());
            Poll::Pending
        }
    }))
}

struct Wizard;

struct WizardState {
    // Keep the controllers and the validation's state across builds.
    stepper: StepperController,
    name: TextEditingController,
    selected: Rc<Cell<[bool; 3]>>,
    pending: Rc<RefCell<Option<ValidationTimer>>>,
    created: Rc<str>,
}

#[derive(Clone, PartialEq)]
enum Msg {
    NameChanged,
    Package(usize, bool),
    Checked,
    Finished,
}

impl StatefulWidget for Wizard {
    type State = WizardState;
    fn create_state(&self) -> WizardState {
        WizardState {
            stepper: StepperController::new(),
            name: TextEditingController::new("my_app"),
            selected: Rc::new(Cell::new([true, false, false])),
            pending: Rc::default(),
            created: "".into(),
        }
    }
}

impl State for WizardState {
    type Widget = Wizard;
    type Msg = Msg;

    fn dispose(&mut self) {
        if let Some(pending) = self.pending.take() {
            pending.timer.cancel();
        }
    }

    fn update(&mut self, msg: Msg, _: &Wizard, _: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::NameChanged => {}
            Msg::Package(index, on) => {
                let mut selected = self.selected.get();
                selected[index] = on;
                self.selected.set(selected);
            }
            Msg::Checked => {
                if let Some(pending) = self.pending.take() {
                    pending.completion.finish();
                }
                return Rebuild::No;
            }
            Msg::Finished => self.created = self.name.text().into(),
        }
        Rebuild::Yes
    }

    fn build(&self, _: &Wizard, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let name = self.name.clone();
        let name_step = Step::new(
            "Name",
            TextField::new()
                .controller(&self.name)
                .semantics_label("Name")
                .on_changed(cx.map(|_| Msg::NameChanged)),
        )
        .subtitle("What the package is called")
        .validate(move || {
            if name.text().is_empty() {
                Validation::refuse("The package needs a name")
            } else {
                Validation::accept()
            }
        });
        let choices = self.selected.get();
        let offered = ["args", "http", "path"];
        let content = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .children(offered.into_iter().enumerate().map(|(index, package)| {
                Checkbox::new(choices[index])
                    .label(package)
                    .on_changed(cx.map(move |on| Msg::Package(index, on)))
                    .into()
            }));
        let selected = self.selected.clone();
        let pending = self.pending.clone();
        let checked = cx.send(Msg::Checked);
        let binding = AppBinding::of(cx);
        let packages = Step::new("Packages", content)
            .subtitle("What it depends on")
            .validate(move || check_packages(&binding, &checked, &pending, selected.clone()));
        let package_list = offered.into_iter().zip(choices)
            .filter_map(|(package, chosen)| chosen.then_some(package))
            .collect::<Vec<_>>().join(", ");
        let confirm = Step::new(
            "Confirm",
            Text::new(format!("Create {} with {package_list}?", self.name.text())),
        );
        let stepper = Stepper::new([name_step, packages, confirm])
            .controller(&self.stepper)
            .direction(Axis::Horizontal)
            .linear(true)
            .show_controls(true)
            .on_finished(cx.send(Msg::Finished));
        column![
            SizedBox::new(Some(50), None, Some(stepper.into())),
            Text::new(format!(
                "created: {}",
                if self.created.is_empty() { "nothing" } else { &self.created },
            )),
        ]
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .spacing(1)
        .into()
    }
}
impl_widget!(stateful Wizard);

```

### Options

| option | values | starts at |
| --- | --- | --- |
| direction | horizontal, vertical | horizontal |
| linear | false, true | true |
| back and next | false, true | true |
| packages disabled | false, true | false |
