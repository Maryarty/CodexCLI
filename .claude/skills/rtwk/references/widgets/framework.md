<!-- Written by tools/rtwk-port/skill/generate.py from rtwk 7482e340; do not edit. -->

# Framework

[All widgets](../widgets.md)

## LayoutBuilder

What is built by the constraints it is laid out within: side by side when wide, one under another when narrow.

API:

- `rtwk::LayoutBuilder` (struct, `crates/rtwk-framework/src/layout_builder.rs:56`) Builds its child with the constraints it is laid out within, so that what it shows depends on the room it gets: one layout for a wide terminal and another for a narrow one.

### Code

The gallery's Code tab at the initial options (route `#/framework/layoutbuilder`, `crates/rtwk-gallery/src/pages/foundation_framework.rs:88`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
use rtwk::layout::{
    Border, BorderTitle, BorderType, BoxWidget, Center, Column, CrossAxisAlignment, Expanded, Row,
    SizedBox,
};
use rtwk::style::{Span, Theme};
use rtwk::text::Text;
use rtwk::{AnyWidget, LayoutBuilder, column};
use rtwk_render::describe_constraints;

pub(crate) fn layout(width: i32, wide_from: i32) -> AnyWidget {
    SizedBox::new(
        Some(width),
        Some(8),
        Some(
            LayoutBuilder::new(move |cx, constraints| {
                let theme = Theme::of(cx);
                let wide = constraints.max_width >= wide_from;
                let panes: [AnyWidget; 2] = ["one", "two"].map(|name| {
                    let muted = theme.muted.clone();
                    Expanded::new(
                        1,
                        BoxWidget::new()
                            .border(Border::all(theme.border.clone()).kind(BorderType::Rounded))
                            .title(BorderTitle::new(format!(" {name} ")))
                            .child(LayoutBuilder::new(move |_, constraints| {
                                Center::new(
                                    Text::new(describe_constraints(&constraints))
                                        .style(muted.clone()),
                                )
                                .into()
                            })),
                    )
                    .into()
                });
                let body: AnyWidget = if wide {
                    Row::new()
                        .cross_axis_alignment(CrossAxisAlignment::Stretch)
                        .children(panes)
                        .into()
                } else {
                    Column::new()
                        .cross_axis_alignment(CrossAxisAlignment::Stretch)
                        .children(panes)
                        .into()
                };
                column![
                    Text::rich(vec![
                        Span::new(format!("{}: ", describe_constraints(&constraints))),
                        Span::styled(if wide { "wide" } else { "narrow" }, theme.primary.clone()),
                    ]),
                    Expanded::new(1, body),
                ]
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .into()
            })
            .into(),
        ),
    )
    .into()
}

fn preview() -> AnyWidget {
    layout(40, 30)
}
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| width | 12..50 (a slider) | 40 cells |
| at | 24, 30, 36 | 30 |

### Methods

#### `LayoutBuilder`: constructors and associated functions

- `pub fn new(builder: impl Fn(&mut Cx<'_>, BoxConstraints) -> AnyWidget + 'static) -> Self` - A layout builder that builds its child with `builder`.

#### `LayoutBuilder`: methods

- `pub fn builder(&self) -> Rc<dyn Fn(&mut Cx<'_>, BoxConstraints) -> AnyWidget>`

### Also used in the code

- `rtwk::layout::Expanded` (struct, `crates/rtwk-widgets/src/layout/flex.rs:270`) `Flexible` with `FlexFit::Tight`: the child fills its part (twk's `Expanded`).
- `rtwk::layout::BorderTitle` (struct, `crates/rtwk-widgets/src/layout/box_.rs:153`) Text on the top or the bottom edge of a `BoxWidget`, in `style`: the y of `alignment` picks the edge (-1 the top, 1 the bottom), its x the place along it (twk's `BorderTitle`).
  - public fields: `text: Str`, `style: Style`, `alignment: Alignment`
- `rtwk::layout::Center` (struct, `crates/rtwk-widgets/src/layout/basic.rs:191`) An `Align` that puts its child in the middle, the odd cell after it (twk's `Center`).
  - public fields: `width_factor: Option<i32>`, `height_factor: Option<i32>`, `child: Option<AnyWidget>`

## NotificationListener

The notifications that bubble up to it from below, such as those of a list as it scrolls: the wheel over it, or Tab to it and the arrows.

API:

- `rtwk::NotificationListener` (struct, `crates/rtwk-framework/src/notification.rs:88`) A widget that hears the notifications of type `N` dispatched from the contexts below it (twk's `NotificationListener`): its callback answers `true` to stop one there, or `false` to let it bubble on up.

### Code

The gallery's Code tab at the initial options (route `#/framework/notificationlistener`, `crates/rtwk-gallery/src/pages/foundation_framework.rs:123`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
use std::collections::VecDeque;
use std::rc::Rc;

use rtwk::focus::FocusNode;
use rtwk::foundation::{ListenableBuilder, ValueNotifier};
use rtwk::layout::{
    Border, BorderType, BoxWidget, CrossAxisAlignment, EdgeInsets, Expanded, MainAxisSize,
    SizedBox, Spacer,
};
use rtwk::scroll::{
    ListView, OverscrollNotification, ScrollMetrics, ScrollNotification, ScrollUpdateNotification,
};
use rtwk::style::{Span, Theme, ThemeData};
use rtwk::text::Text;
use rtwk::{
    AnyWidget, Cx, Notification, NotificationListener, State, StatefulWidget, column, impl_widget,
    row,
};

pub(crate) const TYPES: [&str; 3] = [
    "ScrollNotification",
    "ScrollUpdateNotification",
    "OverscrollNotification",
];

type StatusLine = fn(Option<&str>, u64, &ThemeData) -> AnyWidget;

#[derive(Debug)]
pub(crate) struct Bubbling {
    pub outer: usize,
    pub stops: bool,
    pub status: Option<StatusLine>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mark {
    Past,
    Heard,
    Stops,
    Blocked,
}

impl Mark {
    fn text(self) -> &'static str {
        match self {
            Self::Past => "past",
            Self::Heard => "heard",
            Self::Stops => "stops",
            Self::Blocked => "–",
        }
    }
}

struct Sent {
    update: bool,
    delta: i64,
    metrics: ScrollMetrics,
    inner: Mark,
    outer: Mark,
}

#[derive(Default)]
struct Trace {
    sent: VecDeque<Sent>,
    heard: Option<String>,
    serial: u64,
}

pub(crate) struct BubblingState {
    trace: Rc<ValueNotifier<Trace>>,
    focus: Rc<FocusNode>,
}

impl StatefulWidget for Bubbling {
    type State = BubblingState;

    fn create_state(&self) -> BubblingState {
        BubblingState {
            trace: Rc::new(ValueNotifier::new(Trace::default())),
            focus: Rc::new(FocusNode::new()),
        }
    }
}

fn record(trace: &ValueNotifier<Trace>, event: &dyn Notification) -> bool {
    let (update, delta, metrics) = if let Some(n) = event.get::<ScrollUpdateNotification>() {
        (true, n.scroll_delta, n.metrics)
    } else if let Some(n) = event.get::<OverscrollNotification>() {
        (false, n.overscroll, n.metrics)
    } else {
        return false;
    };
    trace.update(|trace| {
        if trace.sent.len() == 20 {
            trace.sent.pop_front();
        }
        trace.sent.push_back(Sent {
            update,
            delta,
            metrics,
            inner: Mark::Past,
            outer: Mark::Past,
        });
    });
    false
}

fn heard(trace: &ValueNotifier<Trace>) -> bool {
    trace.update(|trace| {
        if let Some(last) = trace.sent.back_mut() {
            last.outer = Mark::Heard;
            let ScrollMetrics {
                pixels: offset,
                max_scroll_extent: max_offset,
                ..
            } = last.metrics;
            trace.heard = Some(if last.update {
                format!("scrolled {} to {offset} of {max_offset}", last.delta)
            } else {
                format!("overscrolled {} at {offset} of {max_offset}", last.delta)
            });
            trace.serial += 1;
        }
    });
    false
}

impl State for BubblingState {
    type Widget = Bubbling;
    type Msg = ();

    fn build(&self, widget: &Bubbling, _: &mut Cx<'_, Self>) -> AnyWidget {
        let trace = self.trace.clone();
        let leaf = NotificationListener::<dyn Notification>::new(
            ListView::builder(100, |i| Text::new(format!("item {}", i + 1)).into())
                .focus_node(self.focus.as_ref().clone()),
        )
        .on_notification(move |event| record(&trace, event));

        let focus = self.focus.clone();
        let border = ListenableBuilder::new(self.focus.clone(), move |cx, child| {
            let theme = Theme::of(cx);
            let style = if focus.has_focus() {
                &theme.primary
            } else {
                &theme.border
            };
            BoxWidget::new()
                .border(Border::all(style.clone()).kind(BorderType::Rounded))
                .padding(EdgeInsets::x(1))
                .child(child.expect("list child").clone())
                .into()
        })
        .child(leaf);

        let trace = self.trace.clone();
        let stops = widget.stops;
        let inner = NotificationListener::<ScrollUpdateNotification>::new(border).on_notification(
            move |_| {
                trace.update(|trace| {
                    if let Some(last) = trace.sent.back_mut() {
                        last.inner = if stops { Mark::Stops } else { Mark::Heard };
                        if stops {
                            last.outer = Mark::Blocked;
                        }
                    }
                });
                stops
            },
        );

        let trace = self.trace.clone();
        let outer: AnyWidget = match widget.outer {
            1 => NotificationListener::<ScrollUpdateNotification>::new(inner)
                .on_notification(move |_| heard(&trace))
                .into(),
            2 => NotificationListener::<OverscrollNotification>::new(inner)
                .on_notification(move |_| heard(&trace))
                .into(),
            _ => NotificationListener::<ScrollNotification>::new(inner)
                .on_notification(move |_| heard(&trace))
                .into(),
        };

        let outer_name = TYPES[widget.outer];
        let status = widget.status.unwrap_or(plain_status);
        let trace = self.trace.clone();
        ListenableBuilder::new(self.trace.clone(), move |cx, child| {
            let theme = Theme::of(cx);
            trace.with(|trace| {
                SizedBox::new(
                    Some(50),
                    None,
                    Some(
                        column![
                            row![
                                SizedBox::new(Some(16), Some(8), child.cloned()),
                                Expanded::new(1, trace_rows(trace, outer_name, &theme)),
                            ]
                            .cross_axis_alignment(CrossAxisAlignment::Start)
                            .spacing(2),
                            status(trace.heard.as_deref(), trace.serial, &theme),
                        ]
                        .main_axis_size(MainAxisSize::Min)
                        .cross_axis_alignment(CrossAxisAlignment::Start)
                        .spacing(1)
                        .into(),
                    ),
                )
                .into()
            })
        })
        .child(outer)
        .into()
    }
}

impl_widget!(stateful Bubbling);

fn plain_status(message: Option<&str>, _: u64, theme: &ThemeData) -> AnyWidget {
    Text::rich(vec![
        Span::styled("heard: ", theme.muted.clone()),
        if let Some(message) = message {
            Span::new(message.to_owned())
        } else {
            Span::styled("nothing", theme.muted.clone())
        },
    ])
    .into()
}

fn trace_rows(trace: &Trace, outer: &'static str, theme: &ThemeData) -> AnyWidget {
    let mut lines = column![Text::new("             Δ  inner  outer").style(theme.muted.clone())]
        .cross_axis_alignment(CrossAxisAlignment::Start);
    if trace.sent.is_empty() {
        lines = lines.child(Text::new("nothing yet").style(theme.muted.clone()));
    }
    for event in trace.sent.iter().rev().take(4).rev() {
        let delta = if event.delta > 0 {
            format!("+{}", event.delta)
        } else {
            event.delta.to_string()
        };
        let label = if event.update {
            "update     "
        } else {
            "overscroll "
        };
        let style = |mark| match mark {
            Mark::Heard => theme.success.clone(),
            Mark::Stops => theme.warning.clone(),
            _ => theme.muted.clone(),
        };
        lines = lines.child(Text::rich(vec![
            Span::new(format!("{label}{delta:>3}")),
            Span::styled(format!("  {:<7}", event.inner.text()), style(event.inner)),
            Span::styled(event.outer.text(), style(event.outer)),
        ]));
    }
    SizedBox::new(
        None,
        Some(8),
        Some(
            lines
                .child(Spacer::new(1))
                .child(Text::rich(vec![
                    Span::styled("inner ", theme.muted.clone()),
                    Span::new(TYPES[1]),
                ]))
                .child(Text::rich(vec![
                    Span::styled("outer ", theme.muted.clone()),
                    Span::new(outer),
                ]))
                .into(),
        ),
    )
    .into()
}

fn preview() -> AnyWidget {
    Bubbling { outer: 0, stops: false, status: None }.into()
}
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| outer | ScrollNotification, ScrollUpdateNotification, OverscrollNotification | ScrollNotification |
| stops | on, off | false |

### Methods

#### `NotificationListener`: constructors and associated functions

- `pub fn new(child: impl Into<AnyWidget>) -> Self` - A listener over `child`, without a callback: notifications go past.

#### `NotificationListener`: builder methods (take and return `self`)

- `pub fn on_notification(mut self, f: impl Fn(&N) -> bool + 'static) -> Self` - With `f` called for each notification of type `N` that reaches it.

### Also used in the code

- `rtwk::foundation::ValueNotifier` (struct, `crates/rtwk-framework/src/foundation/change_notifier.rs:247`) A `ChangeNotifier` that holds a value and notifies its listeners when the value is set to one not equal to it (twk's `ValueNotifier`).
- `rtwk::focus::FocusNode` (struct, `crates/rtwk-framework/src/focus/node.rs:153`) A place in a tree's focus tree: the node of a `Focus` widget, which can have the focus and gets the keys (twk's `FocusNode`).
- `rtwk::scroll::ScrollMetrics` (struct, `crates/rtwk-scroll/src/scrollable.rs:59`) Where a scrollable view is along its `axis`: `pixels` cells of its content scrolled off its start, out of `max_scroll_extent`, with `viewport_dimension` cells of it in its window.
  - public fields: `pixels: i64`, `max_scroll_extent: i64`, `viewport_dimension: i64`, `axis: Axis`
- `rtwk::scroll::ListView` (struct, `crates/rtwk-scroll/src/list_view.rs:257`) A list of `item_count` items one below another, each as wide as the list and as tall as it likes, or `item_extent` rows, in a window that scrolls (a `Scrollable`: keys, the wheel, `controller`).
  - public fields: `item_count: usize`, `item_builder: IndexedWidgetBuilder`, `separator_builder: Option<IndexedWidgetBuilder>`, `reverse: bool`, `padding: Option<EdgeInsets>`, `find_item_index_callback: Option<ChildIndexGetter>`, `item_extent: Option<i32>`, `cache_extent: i32`, `max_kept: usize`, `stick_to_bottom: bool`, `controller: Option<ScrollController>`, `style: Option<Style>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `can_request_focus: Option<bool>`, `physics: Option<Rc<dyn ScrollPhysics>>`, `key_hints: bool`
- `rtwk::foundation::ListenableBuilder` (struct, `crates/rtwk-framework/src/foundation/listenable_builder.rs:131`) Shows what its builder returns, built again whenever its listenable notifies; the widgets around it are not (twk's `ListenableBuilder`).
  - public fields: `listenable: Rc<dyn Listenable>`, `builder: TransitionBuilder`, `child: Option<AnyWidget>`
- `rtwk::layout::Expanded` (struct, `crates/rtwk-widgets/src/layout/flex.rs:270`) `Flexible` with `FlexFit::Tight`: the child fills its part (twk's `Expanded`).
- `rtwk::layout::Spacer` (struct, `crates/rtwk-widgets/src/layout/flex.rs:348`) An empty `Expanded`: a gap that takes `flex` parts of the free room, to push its siblings apart (twk's `Spacer`).
  - public fields: `flex: i32`

## Slot, SlotScope

Places the app names that plugins fill with the entries of a SlotRegistry: Tab to a plugin, then Space puts its entry in the status bar or takes it out.

API:

- `rtwk::slots::Slot` (struct, `crates/rtwk-widgets/src/slots.rs:305`) Shows the contributions for one name in the nearest registry.
- `rtwk::slots::SlotScope` (struct, `crates/rtwk-widgets/src/slots.rs:249`) Makes a registry available to the nearest descendant outlets.

### Code

The gallery's Code tab at the initial options (route `#/framework/slot-slotscope`, `crates/rtwk-gallery/src/pages/foundation_framework.rs:153`): a body for a widget's `build` with `cx` in scope, or a whole module where it starts with `use`.

```rust
use rtwk::controls::Checkbox;
use rtwk::layout::{
    Border, BorderTitle, BorderType, BoxWidget, CrossAxisAlignment, EdgeInsets, MainAxisSize, Row,
    SizedBox,
};
use rtwk::slots::{Slot, SlotEntry, SlotRegistry, SlotScope};
use rtwk::style::{Span, Theme};
use rtwk::text::Text;
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, column, impl_widget};

const PLUGINS: [(&str, &str); 3] = [("git", "main"), ("lint", "0 problems"), ("words", "120")];

const STATUS: &str = "status";
const LINT: usize = 1;

fn plugin(index: usize, lint_order: i32) -> SlotEntry {
    let (name, value) = PLUGINS[index];
    let order = [0, lint_order, 2][index];
    SlotEntry::new(STATUS, name, i64::from(order), move |cx| {
        Text::rich(vec![
            Span::styled(format!("{name} "), Theme::of(cx).muted.clone()),
            Span::new(value),
        ])
        .into()
    })
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct StatusBar {
    pub lint_order: i32,
    pub placeholder: bool,
}

pub(crate) struct StatusBarState {
    registry: SlotRegistry,
}

impl StatefulWidget for StatusBar {
    type State = StatusBarState;

    fn create_state(&self) -> StatusBarState {
        let registry = SlotRegistry::new();
        registry.register(plugin(0, self.lint_order));
        registry.register(plugin(2, self.lint_order));
        StatusBarState { registry }
    }
}

impl State for StatusBarState {
    type Widget = StatusBar;
    type Msg = (usize, bool);

    fn update(
        &mut self,
        (index, on): Self::Msg,
        widget: &StatusBar,
        _: &mut Cx<'_, Self>,
    ) -> Rebuild {
        if on {
            self.registry.register(plugin(index, widget.lint_order));
        } else {
            self.registry.unregister(STATUS, PLUGINS[index].0);
        }
        Rebuild::Yes
    }

    fn did_update_widget(&mut self, old: &StatusBar, widget: &StatusBar, _: &mut Cx<'_, Self>) {
        if old.lint_order != widget.lint_order
            && self
                .registry
                .entries_of(STATUS)
                .iter()
                .any(|entry| entry.id() == PLUGINS[LINT].0)
        {
            self.registry.register(plugin(LINT, widget.lint_order));
        }
    }

    fn build(&self, widget: &StatusBar, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let theme = Theme::of(cx);
        let entries = self.registry.entries_of(STATUS);
        let toggles = Row::new()
            .main_axis_size(MainAxisSize::Min)
            .spacing(2)
            .children(PLUGINS.iter().enumerate().map(|(index, (name, _))| {
                let enabled = entries.iter().any(|entry| entry.id() == *name);
                Checkbox::new(enabled)
                    .label(*name)
                    .on_changed(cx.map(move |on| (index, on)))
                    .into()
            }));
        let mut outlet = Slot::new(STATUS, |_, children| {
            Row::new().spacing(2).children(children).into()
        });
        if widget.placeholder {
            outlet = outlet.placeholder(Text::new("no plugins").style(theme.muted.clone()));
        }
        let content = SlotScope::new(self.registry.clone(), outlet);
        column![
            toggles,
            SizedBox::new(
                Some(44),
                None,
                Some(
                    BoxWidget::new()
                        .border(Border::all(theme.border.clone()).kind(BorderType::Rounded))
                        .title(BorderTitle::new(" status "))
                        .padding(EdgeInsets::x(1))
                        .child(content)
                        .into()
                ),
            ),
        ]
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .spacing(1)
        .into()
    }
}

impl_widget!(stateful eq StatusBar);

fn preview() -> AnyWidget {
    StatusBar { lint_order: 1, placeholder: true }.into()
}
```

### Options

The gallery's inspector; each value is one setting of the example above.

| option | values | starts at |
| --- | --- | --- |
| lint | -1, 1, 3 | 1 |
| placeholder | on, off | true |

### Methods

#### `Slot`: constructors and associated functions

- `pub fn new(name: impl Into<Rc<str>>, layout: impl Fn(&mut BuildCx<'_>, Vec<AnyWidget>) -> AnyWidget + 'static) -> Self` - Creates an outlet using the given layout for its ordered children.

#### `Slot`: builder methods (take and return `self`)

- `pub fn placeholder(mut self, widget: impl Into<AnyWidget>) -> Self` - Shows a widget instead of calling the layout for an empty slot.

#### `Slot`: methods

- `pub fn name(&self) -> &str` - Borrows the outlet name.
- `pub fn placeholder_widget(&self) -> Option<&AnyWidget>` - Borrows the configured fallback, if any.
- `pub fn layout(&self) -> SlotLayout` - Clones the stored layout handle, which can outlive this outlet.

#### `SlotScope`: constructors and associated functions

- `pub fn new(registry: SlotRegistry, child: impl Into<AnyWidget>) -> Self` - Supplies this registry under the scope.
- `pub fn maybe_of(cx: &mut BuildCx<'_>) -> Option<SlotRegistry>` - Reads the closest scope and depends on its registry identity.
- `pub fn of(cx: &mut BuildCx<'_>) -> SlotRegistry` - Reads the nearest scope's registry and depends on its identity.

#### `SlotScope`: methods

- `pub fn registry(&self) -> &SlotRegistry` - Borrows the registry supplied to this scope.

### Also used in the code

- `rtwk::slots::SlotEntry` (struct, `crates/rtwk-widgets/src/slots.rs:21`) A contribution to one named slot.
- `rtwk::controls::StatusBar` (struct, `crates/rtwk-controls/src/status_bar.rs:60`) A status bar: one row of three sections, `left` at its left edge, `right` at its right edge and `center` in its middle.
  - public fields: `left: Option<AnyWidget>`, `center: Option<AnyWidget>`, `right: Option<AnyWidget>`, `spacing: i32`, `style: Option<Style>`
- `rtwk::slots::SlotRegistry` (struct, `crates/rtwk-widgets/src/slots.rs:93`) A thread-local registry shared by independently mounted outlets.
- `rtwk::controls::Checkbox` (struct, `crates/rtwk-controls/src/toggle.rs:64`) A checkbox before `label`: `[x]` while `value` is true, `[ ]` while it is false.
  - public fields: `value: bool`, `on_changed: Option<Handler<bool>>`, `label: Str`, `semantics_label: Option<Str>`, `focus_node: Option<FocusNode>`, `autofocus: bool`, `style: Option<Style>`, `bracket_style: Option<Style>`, `focused_style: Option<Style>`, `disabled_style: Option<Style>`
- `rtwk::layout::BorderTitle` (struct, `crates/rtwk-widgets/src/layout/box_.rs:153`) Text on the top or the bottom edge of a `BoxWidget`, in `style`: the y of `alignment` picks the edge (-1 the top, 1 the bottom), its x the place along it (twk's `BorderTitle`).
  - public fields: `text: Str`, `style: Style`, `alignment: Alignment`
