<!-- Written by tools/rtwk-port/skill/generate.py from rtwk 9f28b7fb; do not edit. -->

# Navigation

[All widgets](../widgets.md)

## Navigator

Screens on a stack: a route pushed covers the one under it and takes the focus, Escape or Back pops it.

API:

- `rtwk::navigation::Navigator` (struct, `crates/rtwk/src/navigation/navigator.rs:1722`) Shows a stack of `Route`s, the last pushed in front: its first, which `initial_route` names (`DEFAULT_ROUTE_NAME` by default), and those pushed on it (`NavigatorState::push`, `NavigatorState::push_named`) until they are ...
  - public fields: `home: Option<AnyWidget>`, `routes: Vec<(Str`, `initial_route: Option<Str>`, `on_generate_route: Option<RouteFactory>`, `on_unknown_route: Option<RouteFactory>`, `back_key: Option<Str>`, `observers: Vec<Rc<dyn NavigatorObserver>>`
- `rtwk::navigation::PageRoute` (struct, `crates/rtwk/src/navigation/navigator.rs:716`) A screen: what its builder builds, covering the routes under it.

### Code

The gallery's Code tab at the initial options (route `#/navigation/navigator`, `crates/rtwk-gallery/src/pages/mod.rs:917`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
let navigator = Navigator::new()
    .home(DemoRoute { screen: Screen::Home, frozen })
    .back_key(Some("escape"))
    .on_generate_route(move |settings| {
        (settings.name.as_deref() == Some("/settings")).then(|| {
            page_route(Screen::Settings, frozen)
                .settings(settings.clone())
                .into_route()
        })
    });
frame(navigator, Theme::of(cx).border.clone())

// The route bodies (with their own state and event handlers).
let back = || Button::new("Back").autofocus(true).on_pressed(cx.send(RouteMsg::Back));
let body = match route.screen {
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
    _ => unreachable!(),
}
.cross_axis_alignment(CrossAxisAlignment::Start)
.spacing(1);

// In State::update
match msg {
    RouteMsg::OpenItem(item) => {
        let _result = Navigator::push::<(), _>(cx, page_route(Screen::Item(item), frozen));
    }
    RouteMsg::Named(name) => {
        let _result = Navigator::push_named::<(), _>(cx, name);
    }
    RouteMsg::Back => {
        Navigator::maybe_pop(cx);
    }
    _ => {}
}

// page_route builds PageRoute::new with 220 ms in and 180 ms out.
// Both durations are zero when the gallery's clock is frozen.
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| backKey | escape, backspace, none | escape |

### Methods

#### `Navigator`: constructors and associated functions

- `pub fn new() -> Navigator` - A navigator with no route made yet: give it a `home` or `routes`.
- `pub fn of<S>(cx: &Cx<'_, S>) -> NavigatorState` - The nearest navigator above `cx`.
- `pub fn maybe_of<S>(cx: &Cx<'_, S>) -> Option<NavigatorState>` - The navigator `of` finds, if there is one.
- `pub fn root_of<S>(cx: &Cx<'_, S>) -> NavigatorState` - The furthest navigator above `cx`, the root one.
- `pub fn maybe_root_of<S>(cx: &Cx<'_, S>) -> Option<NavigatorState>` - The navigator `root_of` finds, if there is one.
- `pub fn push<T: 'static, S>(cx: &Cx<'_, S>, route: impl Into<Route>) -> DialogResult<T>` - `NavigatorState::push` on the navigator above `cx`.
- `pub fn push_named<T: 'static, S>(cx: &Cx<'_, S>, name: &str) -> DialogResult<T>` - `NavigatorState::push_named` on the navigator above `cx`.
- `pub fn push_named_with<T: 'static, S>(cx: &Cx<'_, S>, name: &str, arguments: impl Any + fmt::Debug) -> DialogResult<T>` - `NavigatorState::push_named_with` on the navigator above `cx`.
- `pub fn push_replacement<T: 'static, S>(cx: &Cx<'_, S>, route: impl Into<Route>) -> DialogResult<T>` - `NavigatorState::push_replacement` on the navigator above `cx`.
- `pub fn push_replacement_with<T: 'static, R: Any + Clone, S>(cx: &Cx<'_, S>, route: impl Into<Route>, result: R) -> DialogResult<T>` - `NavigatorState::push_replacement_with` on the navigator above `cx`.
- `pub fn push_replacement_named_with<T: 'static, R: Any + Clone, S>(cx: &Cx<'_, S>, name: &str, arguments: Option<Arguments>, result: Option<R>) -> DialogResult<T>` - `NavigatorState::push_replacement_named_with` on the navigator above `cx`.
- `pub fn push_and_remove_until<T: 'static, S>(cx: &Cx<'_, S>, route: impl Into<Route>, predicate: impl Fn(&Route) -> bool) -> DialogResult<T>` - `NavigatorState::push_and_remove_until` on the navigator above `cx`.
- `pub fn push_named_and_remove_until<T: 'static, S>(cx: &Cx<'_, S>, name: &str, predicate: impl Fn(&Route) -> bool) -> DialogResult<T>` - `NavigatorState::push_named_and_remove_until` on the navigator above `cx`.
- `pub fn push_named_and_remove_until_with<T: 'static, S>(cx: &Cx<'_, S>, name: &str, arguments: impl Any + fmt::Debug, predicate: impl Fn(&Route) -> bool) -> DialogResult<T>` - `NavigatorState::push_named_and_remove_until_with` on the navigator above `cx`.
- `pub fn pop_and_push_named<T: 'static, S>(cx: &mut Cx<'_, S>, name: &str) -> DialogResult<T>` - `NavigatorState::pop_and_push_named` on the navigator above `cx`.
- `pub fn pop_and_push_named_with<T: 'static, R: Any + Clone, S>(cx: &mut Cx<'_, S>, name: &str, arguments: Option<Arguments>, result: Option<R>) -> DialogResult<T>` - `NavigatorState::pop_and_push_named_with` on the navigator above `cx`.
- `pub fn pop<S>(cx: &mut Cx<'_, S>)` - `NavigatorState::pop` on the navigator above `cx`.
- `pub fn pop_with<S, R: Any + Clone>(cx: &mut Cx<'_, S>, result: R)` - `NavigatorState::pop_with` on the navigator above `cx`.
- `pub fn pop_until<S>(cx: &mut Cx<'_, S>, predicate: impl Fn(&Route) -> bool)` - `NavigatorState::pop_until` on the navigator above `cx`.
- `pub fn maybe_pop<S>(cx: &mut Cx<'_, S>) -> bool` - `NavigatorState::maybe_pop` on the navigator above `cx`.
- `pub fn maybe_pop_with<S, R: Any + Clone>(cx: &mut Cx<'_, S>, result: R) -> bool` - `NavigatorState::maybe_pop_with` on the navigator above `cx`.
- `pub fn can_pop<S>(cx: &Cx<'_, S>) -> bool` - `NavigatorState::can_pop` on the navigator above `cx`; false when there is none.

#### `Navigator`: builder methods (take and return `self`)

- `pub fn home(mut self, home: impl Into<AnyWidget>) -> Navigator` - With `home` as the route named `/`.
- `pub fn route(mut self, name: impl Into<Str>, builder: impl Fn(&mut BuildCx<'_>) -> AnyWidget + 'static) -> Navigator` - With the route `name` showing what `builder` builds.
- `pub fn initial_route(mut self, name: impl Into<Str>) -> Navigator` - Starting at the route `name`.
- `pub fn on_generate_route(mut self, f: impl Fn(&RouteSettings) -> Option<Route> + 'static) -> Navigator` - Making the routes `home` and `routes` do not make with `f`.
- `pub fn on_unknown_route(mut self, f: impl Fn(&RouteSettings) -> Option<Route> + 'static) -> Navigator` - Making the routes nothing else makes with `f`.
- `pub fn back_key(mut self, key: Option<&str>) -> Navigator` - Popping the route in front on `key`, none for no key.
- `pub fn observer<O: NavigatorObserver>(mut self, observer: Rc<O>) -> Navigator` - Telling `observer` of each change.

#### `PageRoute`: constructors and associated functions

- `pub fn new(builder: impl Fn(&mut BuildCx<'_>) -> AnyWidget + 'static) -> PageRoute<T>` - A screen of what `builder` builds.

#### `PageRoute`: builder methods (take and return `self`)

- `pub fn settings(mut self, settings: RouteSettings) -> PageRoute<T>` - Known by `settings`.
- `pub fn transition(mut self, d: Duration) -> PageRoute<T>` - Coming in over `d`.
- `pub fn reverse_transition(mut self, d: Duration) -> PageRoute<T>` - Going out over `d` (the time to come in where none is given).
- `pub fn into_route(self) -> Route` - The route to push; a result it is popped with has to be a `T`.
- `pub fn into_route_any(self) -> Route` - The route to push, popped with a result of any type (twk's `Route<Object?>`): the type of what `push` returns is the push's alone.

#### `PageRoute`: methods

- `pub fn transition_duration(&self) -> Duration` - How long it takes to come in.
- `pub fn reverse_transition_duration(&self) -> Duration` - How long it takes to go out.

### Also used in the code

- `rtwk::controls::Button` (struct, `crates/rtwk-controls/src/button.rs:107`) A button that tells `on_pressed` when it is clicked, or when Enter or Space is pressed while it has the focus.
  - public fields: `spans: Option<Rc<[Span]>>`, `tall: bool`, `on_pressed: Option<Handler<()>>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: bool`, `style: Option<Style>`, `focused_style: Option<Style>`, `hover_style: Option<Style>`, `disabled_style: Option<Style>`

## PopScope, NavigatorObserver

A route that asks before it is popped, by Escape or by the back key, a dialog too; and an observer told of every push, pop and removal.

API:

- `rtwk::navigation::PopScope` (struct, `crates/rtwk/src/navigation/navigator.rs:940`) Refuses or intercepts a pop of the route it is in: while `can_pop` is false, the back key and `NavigatorState::maybe_pop` leave the route, even the first one, whose back key otherwise goes on past the navigator; they ...
  - public fields: `child: AnyWidget`, `can_pop: bool`, `on_pop_invoked_with_result: Option<Handler<PopInvoked>>`
- `rtwk::navigation::NavigatorObserver` (trait, `crates/rtwk/src/navigation/navigator.rs:878`) Told by the `Navigator`s it is given (`Navigator::observers`) of each route pushed, popped, replaced and removed, after the change and before the next build shows it.

### Code

The gallery's Code tab at the initial options (route `#/navigation/popscope-navigatorobserver`, `crates/rtwk-gallery/src/pages/mod.rs:930`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
let navigator = Navigator::new()
    .home(DemoRoute { screen: Screen::Notes, frozen })
    .route("/edit", move |_| DemoRoute { screen: Screen::Edit, frozen }.into())
    .observer(self.heard.clone());
let child = Asking { enabled: true, keep: false, child: navigator.into() };
// Heard implements NavigatorObserver and publishes pushed, popped,
// and removed route names with a serial through its ValueNotifier.
let last = self.heard.last.clone();
let heard = ListenableBuilder::new(last.clone(), move |_, _| {
    let (event, serial) = last.value();
    result("heard: ", &event, false, serial).into()
});
column![frame(child, Theme::of(cx).border.clone()), heard]
    .main_axis_size(MainAxisSize::Min)
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .spacing(1)

// In the editor:
let content = column![
    Text::new("Edit the note").style(theme.primary.clone()),
    Checkbox::new(self.changed).label("changed").on_changed(cx.map(RouteMsg::Changed)),
    Button::new("Back").autofocus(true).on_pressed(cx.send(RouteMsg::Back)),
]
.cross_axis_alignment(CrossAxisAlignment::Start)
.spacing(1);
if cx.depend::<Asking, _>(|scope| scope.enabled).unwrap_or(false) {
    PopScope::new(content)
        .can_pop(!self.changed)
        .on_pop_invoked_with_result(cx.map(|p: PopInvoked| RouteMsg::PopInvoked(p.did_pop)))
        .into()
} else {
    content.into()
}

// Ask on a refused pop and pop the editor only after a confirmed discard.
let dialog = show_dialog::<bool, _>(cx, move |_| {
    let content = DemoRoute { screen: Screen::DiscardDialog, frozen };
    content.into()
}, options);
cx.spawn(async move {
    Some(RouteMsg::ConfirmDiscard(dialog.await.unwrap_or(false)))
});

// Screen::DiscardDialog
Dialog::new(
    column![
        Text::new("The note was changed."),
        row![
            Button::new("Discard").autofocus(true)
                .on_pressed(cx.send(RouteMsg::CloseDialog(true))),
            Button::new("Keep").on_pressed(cx.send(RouteMsg::CloseDialog(false))),
        ].main_axis_size(MainAxisSize::Min).spacing(2),
    ].main_axis_size(MainAxisSize::Min).cross_axis_alignment(CrossAxisAlignment::Start).spacing(1),
).title("Discard the changes?")

// State::update closes the dialog with a result, then pops the editor on true.
RouteMsg::CloseDialog(discard) => close_dialog(cx, Some(discard)),
RouteMsg::ConfirmDiscard(discard) => {
    self.asking = false;
    if discard { Navigator::pop(cx); }
}
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| scope | on, off | true |
| dialog | close, keep | close |

### Methods

#### `PopScope`: constructors and associated functions

- `pub fn new(child: impl Into<AnyWidget>) -> PopScope` - Keeps nothing yet: `child` in a route that pops.

#### `PopScope`: builder methods (take and return `self`)

- `pub fn can_pop(mut self, can_pop: bool) -> PopScope` - Whether the back key pops the route.
- `pub fn on_pop_invoked_with_result(mut self, h: Handler<PopInvoked>) -> PopScope` - Told of each pop, with its result.

### Also used in the code

- `rtwk::navigation::Navigator` (struct, `crates/rtwk/src/navigation/navigator.rs:1722`) Shows a stack of `Route`s, the last pushed in front: its first, which `initial_route` names (`DEFAULT_ROUTE_NAME` by default), and those pushed on it (`NavigatorState::push`, `NavigatorState::push_named`) until they are ...
  - public fields: `home: Option<AnyWidget>`, `routes: Vec<(Str`, `initial_route: Option<Str>`, `on_generate_route: Option<RouteFactory>`, `on_unknown_route: Option<RouteFactory>`, `back_key: Option<Str>`, `observers: Vec<Rc<dyn NavigatorObserver>>`
- `rtwk::foundation::ListenableBuilder` (struct, `crates/rtwk-framework/src/foundation/listenable_builder.rs:131`) Shows what its builder returns, built again whenever its listenable notifies; the widgets around it are not (twk's `ListenableBuilder`).
  - public fields: `listenable: Rc<dyn Listenable>`, `builder: TransitionBuilder`, `child: Option<AnyWidget>`
- `rtwk::controls::Checkbox` (struct, `crates/rtwk-controls/src/toggle.rs:64`) A checkbox before `label`: `[x]` while `value` is true, `[ ]` while it is false.
  - public fields: `value: bool`, `on_changed: Option<Handler<bool>>`, `label: Str`, `semantics_label: Option<Str>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `style: Option<Style>`, `bracket_style: Option<Style>`, `focused_style: Option<Style>`, `disabled_style: Option<Style>`
- `rtwk::controls::Button` (struct, `crates/rtwk-controls/src/button.rs:107`) A button that tells `on_pressed` when it is clicked, or when Enter or Space is pressed while it has the focus.
  - public fields: `spans: Option<Rc<[Span]>>`, `tall: bool`, `on_pressed: Option<Handler<()>>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: bool`, `style: Option<Style>`, `focused_style: Option<Style>`, `hover_style: Option<Style>`, `disabled_style: Option<Style>`
- `rtwk::navigation::PopInvoked` (struct, `crates/rtwk/src/navigation/navigator.rs:249`) What a `PopScope` is told of a pop of its route: whether it was popped (`did_pop`) and the result it was popped with, or asked to be.
  - public fields: `did_pop: bool`
- `rtwk::overlay::Dialog` (struct, `crates/rtwk-overlay/src/dialog.rs:751`) `child` in a box that stands out of what is around it: a raised panel (`look`) of the theme's surface and text, a cell of room around `padding` and no outline, `title` in bold on the first row inside, over the child; ...
  - public fields: `child: AnyWidget`, `title: Option<Str>`, `trailing: Option<AnyWidget>`, `scrollable: bool`, `padding: EdgeInsets`, `shadow: Option<BoxShadow>`, `look: PanelLook`, `max_width: Option<i32>`

## Breadcrumbs

Where a screen is in a hierarchy, each level a link up: the arrows and Enter or a click go to one, and the middle collapses into … where the row does not fit.

API:

- `rtwk::navigation::Breadcrumbs` (struct, `crates/rtwk/src/navigation/breadcrumbs.rs:85`) The path of `items` to where a screen is, root first, on one row: `Root › Projects › my_app › lib › src › widgets`, each but the `current` one a link that calls `on_selected` with its index.
  - public fields: `items: Rc<[Str]>`, `on_selected: Option<Handler<usize>>`, `current: Option<usize>`, `separator: Option<Str>`, `keep_leading: usize`, `focus_node: Option<FocusNode>`, `style: Option<Style>`, `current_style: Option<Style>`

### Code

The gallery's Code tab at the initial options (route `#/navigation/breadcrumbs`, `crates/rtwk-gallery/src/pages/mod.rs:952`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
// Initially, at = PATH.len() - 1. Update it on selection.
const PATH: [&str; 6] = ["twk", "packages", "gallery", "lib", "src", "pages"];
let crumbs = Breadcrumbs::new(PATH)
    .current(self.at)
    .keep_leading(1)
    .separator(" › ")
    .on_selected(cx.map(|index| index));
column![
    SizedBox::new(Some(36), None, Some(crumbs.into())),
    result("at: ", &PATH[..=self.at].join("/"), false, self.serial),
]
.main_axis_size(MainAxisSize::Min)
.cross_axis_alignment(CrossAxisAlignment::Start)
.spacing(1)
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| width | 12..50 (a slider) | 36 cells |
| keep | 1, 0, 2 | 1 |
| separator |  › ,  /  |  ›  |

### Methods

#### `Breadcrumbs`: constructors and associated functions

- `pub fn new<I, S>(items: I) -> Breadcrumbs where I: IntoIterator<Item = S>, S: Into<Str>,` - The path of `items`, root first; shown only until `on_selected` is set.

#### `Breadcrumbs`: builder methods (take and return `self`)

- `pub fn on_selected(mut self, h: Handler<usize>) -> Breadcrumbs` - Told the index of the item selected.
- `pub fn current(mut self, index: usize) -> Breadcrumbs` - The item where the screen is, which is no link.
- `pub fn separator(mut self, separator: impl Into<Str>) -> Breadcrumbs` - `separator` between two crumbs.
- `pub fn keep_leading(mut self, n: usize) -> Breadcrumbs` - `n` of the first items kept before the `…`.
- `pub fn focus_node(mut self, node: Option<FocusNode>) -> Breadcrumbs` - With `node` as its node of the focus tree.
- `pub fn style(mut self, style: Style) -> Breadcrumbs` - The links and the `…` in `style`.
- `pub fn current_style(mut self, style: Style) -> Breadcrumbs` - The current item in `style`.

## Pagination

The pages of a long list on one row, the current one among its siblings: the arrows, PageUp and PageDown, a page number typed or a click go to another.

API:

- `rtwk::navigation::Pagination` (struct, `crates/rtwk/src/navigation/pagination.rs:89`) The `page_count` pages of a list loaded a page at a time, from an API or a database, on one row, the `page` shown among them: `« First ‹ Prev 1 [2] 3 4 5 … 42 Next › Last » (21–40 of 840)`.
  - public fields: `page: usize`, `page_count: usize`, `on_changed: Option<Handler<usize>>`, `item_count: Option<usize>`, `page_size: Option<usize>`, `siblings: usize`, `show_first_last: bool`, `show_prev_next: bool`, `focus_node: Option<FocusNode>`, `style: Option<Style>`, `current_style: Option<Style>`

### Code

The gallery's Code tab at the initial options (route `#/navigation/pagination`, `crates/rtwk-gallery/src/pages/mod.rs:989`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
// Initially, page = 1 (the second page). Store the zero-based index on change.
let pages = Pagination::new(self.page, 42)
    .siblings(1)
    .show_first_last(true)
    .show_prev_next(true)
    .on_changed(cx.map(|page| page));
column![
    SizedBox::new(Some(56), None, Some(pages.into())),
    result("page: ", &format!("{} of 42", self.page + 1), false, self.serial),
]
.main_axis_size(MainAxisSize::Min)
.cross_axis_alignment(CrossAxisAlignment::Start)
.spacing(1)
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| width | 16..56 (a slider) | 56 cells |
| siblings | 1, 0, 2 | 1 |
| firstLast | on, off | true |
| prevNext | on, off | true |
| range | on, off | false |

### Methods

#### `Pagination`: constructors and associated functions

- `pub fn new(page: usize, page_count: usize) -> Pagination` - The `page_count` pages, `page` the one shown; disabled until `on_changed` is set.

#### `Pagination`: builder methods (take and return `self`)

- `pub fn on_changed(mut self, h: Handler<usize>) -> Pagination` - Told the page to go to, from 0.
- `pub fn item_count(mut self, n: usize) -> Pagination` - `n` items in all.
- `pub fn page_size(mut self, n: usize) -> Pagination` - `n` items a page.
- `pub fn siblings(mut self, n: usize) -> Pagination` - `n` pages on either side of the current one.
- `pub fn show_first_last(mut self, on: bool) -> Pagination` - Whether `« First` and `Last »` are shown.
- `pub fn show_prev_next(mut self, on: bool) -> Pagination` - Whether `‹ Prev` and `Next ›` are shown.
- `pub fn focus_node(mut self, node: Option<FocusNode>) -> Pagination` - With `node` as its node of the focus tree.
- `pub fn style(mut self, style: Style) -> Pagination` - The row in `style`.
- `pub fn current_style(mut self, style: Style) -> Pagination` - The current page in `style`.

## Stepper

A wizard of steps under their headers, each left forward once its validation takes it: Next or PageDown validates, Back or PageUp goes back, a header clicked goes to it.

API:

- `rtwk::navigation::Stepper` (struct, `crates/rtwk/src/navigation/stepper.rs:979`) A wizard of `steps`, one shown at a time under the headers of all of them, with Back and Next under it: installers, `init`-style setups, deploy flows.
  - public fields: `steps: Rc<[Step]>`, `controller: Option<StepperController>`, `direction: Axis`, `linear: bool`, `on_finished: Option<Handler<()>>`, `on_step_changed: Option<Handler<usize>>`, `back_label: Option<Str>`, `next_label: Option<Str>`, `finish_label: Option<Str>`, `show_controls: bool`

### Code

The gallery's Code tab at the initial options (route `#/navigation/stepper`, `crates/rtwk-gallery/src/pages/mod.rs:1040`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
// Keep TextEditingController::new("my_app") in state.
let name = self.name.clone();
let name_step = Step::new(
    "Name",
    TextField::new()
        .controller(&self.name)
        .semantics_label("Name")
        .on_changed(cx.map(|_| WizardMsg::NameChanged)),
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
            .on_changed(cx.map(move |on| WizardMsg::Package(index, on)))
            .into()
    }));
let selected = self.selected.clone();
let pending = self.pending.clone();
let checked = cx.send(WizardMsg::Checked);
let binding = AppBinding::of(cx);
let packages = Step::new("Packages", content)
    .subtitle("What it depends on")
    .validate(move || check_packages(&binding, &checked, &pending, selected.clone()));
// check_packages starts an 800 ms app timer and returns Validation::later.
// WizardMsg::Checked wakes it; it refuses when no package is selected.
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
    .on_finished(cx.send(WizardMsg::Finished));
column![
    SizedBox::new(Some(50), None, Some(stepper.into())),
    result(
        "created: ",
        if self.created.is_empty() { "nothing" } else { &self.created },
        self.created.is_empty(),
        self.serial,
    ),
]
.main_axis_size(MainAxisSize::Min)
.cross_axis_alignment(CrossAxisAlignment::Start)
.spacing(1)
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| direction | horizontal, vertical | horizontal |
| linear | on, off | true |
| controls | on, off | true |
| disabled | on, off | false |

### Methods

#### `Stepper`: constructors and associated functions

- `pub fn new(steps: impl Into<Rc<[Step]>>) -> Stepper` - A wizard of `steps`, the first shown unless the controller says.

#### `Stepper`: builder methods (take and return `self`)

- `pub fn controller(mut self, controller: impl Into<StepperController>) -> Stepper` - Moved by `controller`.
- `pub fn direction(mut self, direction: Axis) -> Stepper` - With the headers along `direction`.
- `pub fn linear(mut self, linear: bool) -> Stepper` - Whether a step is reached only past those before it.
- `pub fn on_finished(mut self, h: Handler<()>) -> Stepper` - Told once the last step is taken.
- `pub fn on_step_changed(mut self, h: Handler<usize>) -> Stepper` - Told the step it moved to, from 0.
- `pub fn back_label(mut self, label: impl Into<Str>) -> Stepper` - With `label` on the button that goes back.
- `pub fn next_label(mut self, label: impl Into<Str>) -> Stepper` - With `label` on the button that goes on.
- `pub fn finish_label(mut self, label: impl Into<Str>) -> Stepper` - With `label` on the button that goes on from the last step.
- `pub fn show_controls(mut self, on: bool) -> Stepper` - Whether Back and Next are shown.

### Also used in the code

- `rtwk::editing::TextEditingController` (struct, `crates/rtwk-editing/src/text_editing.rs:418`) The `TextEditingValue` a field edits, which notifies its listeners of each change.
- `rtwk::navigation::Step` (struct, `crates/rtwk/src/navigation/stepper.rs:176`) A step of a `Stepper`: its `title`, what it shows while it is the current one (`content`), and what it takes to leave it forward (`validate`).
  - public fields: `title: Str`, `content: AnyWidget`, `subtitle: Option<Str>`, `state: Option<StepState>`, `validate: Option<Validate>`
- `rtwk::editing::TextField` (struct, `crates/rtwk-editing/src/text_field.rs:160`) A field of a line of text the user edits, which its `controller` holds (twk's `TextField`).
  - public fields: `controller: Option<TextEditingController>`, `placeholder: Option<Str>`, `semantics_label: Option<Str>`, `obscure_text: bool`, `obscuring_character: Option<Str>`, `max_length: Option<usize>`, `on_changed: Option<Handler<Rc<str>>>`, `on_submitted: Option<Handler<Rc<str>>>`, `on_editing_complete: Option<Handler<()>>`, `on_paste: Option<OnPaste>`, `read_only: bool`, `enabled: bool`, `text_align: TextAlign`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: Option<bool>`, `style: Option<Style>`, `focused_style: Option<Style>`, `placeholder_style: Option<Style>`, `selection_style: Option<Style>`
- `rtwk::navigation::Validation` (enum, `crates/rtwk/src/navigation/stepper.rs:107`) What a `Step::validate` answers: the message that refuses the step, none to take it, at once or after a wait (twk's `FutureOr<String?>`).
  - variants: `Done`, `Later`
- `rtwk::controls::Checkbox` (struct, `crates/rtwk-controls/src/toggle.rs:64`) A checkbox before `label`: `[x]` while `value` is true, `[ ]` while it is false.
  - public fields: `value: bool`, `on_changed: Option<Handler<bool>>`, `label: Str`, `semantics_label: Option<Str>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `style: Option<Style>`, `bracket_style: Option<Style>`, `focused_style: Option<Style>`, `disabled_style: Option<Style>`
- `rtwk::binding::AppBinding` (struct, `crates/rtwk-widgets/src/binding/app_binding.rs:327`) The app a widget runs in (twk's `AppBinding`): one per running app, found from a build by `maybe_of`.
- `rtwk::layout::Axis` (enum, `crates/rtwk-render/src/objects/decoration.rs:39`) Along which axis something runs: across or down.
  - variants: `Horizontal`, `Vertical`
