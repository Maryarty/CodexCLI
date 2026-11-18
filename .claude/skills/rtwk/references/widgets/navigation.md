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

| option | values | starts at |
| --- | --- | --- |
| back key | escape, backspace, none | escape |

## PopScope, NavigatorObserver

A route that asks before it is popped, by Escape or by the back key, a dialog too; and an observer told of every push, pop and removal.

- `rtwk::navigation::PopScope` (struct) in `crates/rtwk/src/navigation/navigator.rs:940`: Refuses or intercepts a pop of the route it is in: while can_pop is false, the back key and NavigatorState::maybe_pop leave the route, even the first one, whose back key otherwise goes on past the navigator; they take the key, and on_pop_invoked_with_result is told that it did not pop, with the result asked for.
- `rtwk::navigation::NavigatorObserver` (trait) in `crates/rtwk/src/navigation/navigator.rs:878`: Told by the Navigators it is given (Navigator::observers) of each route pushed, popped, replaced and removed, after the change and before the next build shows it.

### Code

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

| option | values | starts at |
| --- | --- | --- |
| PopScope | false, true | true |
| esc in the dialog | closes it, is kept | closes it |

## Breadcrumbs

Where a screen is in a hierarchy, each level a link up: the arrows and Enter or a click go to one, and the middle collapses into … where the row does not fit.

- `rtwk::navigation::Breadcrumbs` (struct) in `crates/rtwk/src/navigation/breadcrumbs.rs:85`: The path of items to where a screen is, root first, on one row: `Root › Projects › my_app › lib › src › widgets`, each but the current one a link that calls on_selected with its index.

### Code

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

| option | values | starts at |
| --- | --- | --- |
| direction | horizontal, vertical | horizontal |
| linear | false, true | true |
| back and next | false, true | true |
| packages disabled | false, true | false |
