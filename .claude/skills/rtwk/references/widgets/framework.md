<!-- Written by `cargo xtask skill-references` from docs/widgets; do not edit. -->
# Framework

Stateful and inherited widgets, builders, keys and the element tree.

[All widgets](../widgets.md)

A Code is the body of the gallery's preview: it names widgets by the last segment of the paths under API (`use` them) and may use `cx` or values of the gallery's page.

## LayoutBuilder

What is built by the constraints it is laid out within: side by side when wide, one under another when narrow.

- `rtwk::LayoutBuilder` (struct) in `crates/rtwk-framework/src/layout_builder.rs:56`: Builds its child with the constraints it is laid out within, so that what it shows depends on the room it gets: one layout for a wide terminal and another for a narrow one.

### Code

```rust
use rtwk::layout::{
    Border, BorderTitle, BorderType, BoxWidget, Center, Column, CrossAxisAlignment, Expanded, Row,
    SizedBox,
};
use rtwk::render::describe_constraints;
use rtwk::style::{Span, Theme};
use rtwk::text::Text;
use rtwk::{AnyWidget, LayoutBuilder, column};

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

| option | values | starts at |
| --- | --- | --- |
| width | 12 cells, 13 cells, 14 cells, 15 cells, 16 cells, 17 cells, 18 cells, 19 cells, 20 cells, 21 cells, 22 cells, 23 cells, 24 cells, 25 cells, 26 cells, 27 cells, 28 cells, 29 cells, 30 cells, 31 cells, 32 cells, 33 cells, 34 cells, 35 cells, 36 cells, 37 cells, 38 cells, 39 cells, 40 cells, 41 cells, 42 cells, 43 cells, 44 cells, 45 cells, 46 cells, 47 cells, 48 cells, 49 cells, 50 cells | 40 cells |
| wide from | 24, 30, 36 | 30 |

## NotificationListener

The notifications that bubble up to it from below, such as those of a list as it scrolls: the wheel over it, or Tab to it and the arrows.

- `rtwk::NotificationListener` (struct) in `crates/rtwk-framework/src/notification.rs:88`: A widget that hears the notifications of type `N` dispatched from the contexts below it (twk's `NotificationListener`): its callback answers `true` to stop one there, or `false` to let it bubble on up.

### Code

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

| option | values | starts at |
| --- | --- | --- |
| outer hears | Scroll, ScrollUpdate, Overscroll | Scroll |
| inner stops them | false, true | false |

## Slot, SlotScope

Places the app names that plugins fill with the entries of a SlotRegistry: Tab to a plugin, then Space puts its entry in the status bar or takes it out.

- `rtwk::slots::Slot` (struct) in `crates/rtwk-widgets/src/slots.rs:305`: Shows the contributions for one name in the nearest registry.
- `rtwk::slots::SlotScope` (struct) in `crates/rtwk-widgets/src/slots.rs:249`: Makes a registry available to the nearest descendant outlets.
- `rtwk::slots::SlotRegistry` (struct) in `crates/rtwk-widgets/src/slots.rs:93`: A thread-local registry shared by independently mounted outlets.

### Code

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

| option | values | starts at |
| --- | --- | --- |
| lint order | -1, first, 1, between, 3, last | 1, between |
| placeholder | false, true | true |
