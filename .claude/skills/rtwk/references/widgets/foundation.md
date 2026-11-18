<!-- Written by `cargo xtask skill-references` from docs/widgets; do not edit. -->
# Foundation

Notifiers and listenables that widgets build on.

[All widgets](../widgets.md)

A Code is the body of the gallery's preview: it names widgets by the last segment of the paths under API (`use` them) and may use `cx` or values of the gallery's page.

## ListenableBuilder

What is built again each time a listenable notifies; ValueListenableBuilder is given its value.

- `rtwk::foundation::ListenableBuilder` (struct) in `crates/rtwk-framework/src/foundation/listenable_builder.rs:131`: Shows what its builder returns, built again whenever its listenable notifies; the widgets around it are not (twk's `ListenableBuilder`).

### Code

```rust
use std::rc::Rc;
use std::time::Duration;

use rtwk::binding::AppBinding;
use rtwk::controls::Button;
use rtwk::foundation::{ListenableBuilder, ValueListenableBuilder, ValueNotifier};
use rtwk::layout::{CrossAxisAlignment, MainAxisSize};
use rtwk::text::Text;
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, column, impl_widget, row};

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Counter {
    pub timer: bool,
    pub visible: bool,
    pub frozen_ms: Option<i64>,
}

pub(crate) struct CounterState {
    count: Rc<ValueNotifier<i64>>,
    wake: Option<(AppBinding, u64)>,
    since: Option<i64>,
}

#[derive(Clone, PartialEq)]
pub(crate) enum CounterMsg {
    Count,
    Reset,
    Tick,
}

impl StatefulWidget for Counter {
    type State = CounterState;

    fn create_state(&self) -> CounterState {
        CounterState {
            count: Rc::new(ValueNotifier::new(0)),
            wake: None,
            since: None,
        }
    }
}

impl Counter {
    fn now(&self, cx: &Cx<'_, CounterState>) -> i64 {
        self.frozen_ms.unwrap_or_else(|| {
            AppBinding::maybe_of(cx).map_or(0, |binding| {
                i64::try_from(binding.frame_time().as_millis()).unwrap_or(i64::MAX)
            })
        })
    }
}

impl CounterState {
    fn tick(&mut self, now: i64) {
        if let Some(since) = &mut self.since {
            let ticks = now.saturating_sub(*since) / 1_000;
            if ticks != 0 {
                *since = since.saturating_add(ticks.saturating_mul(1_000));
                self.count
                    .set_value(self.count.value().saturating_add(ticks));
            }
        }
    }

    fn schedule(&mut self, widget: &Counter, cx: &Cx<'_, Self>) {
        self.cancel_wake();
        if widget.visible
            && widget.frozen_ms.is_none()
            && let (Some(since), Some(binding)) = (self.since, AppBinding::maybe_of(cx))
        {
            let id = binding.schedule_frame_callback(
                cx.map(|_: Duration| CounterMsg::Tick),
                Some(Duration::from_millis(
                    u64::try_from(since.saturating_add(1_000)).unwrap_or(0),
                )),
            );
            self.wake = Some((binding, id));
        }
    }

    fn cancel_wake(&mut self) {
        if let Some((binding, id)) = self.wake.take() {
            binding.cancel_frame_callback(id);
        }
    }
}

impl State for CounterState {
    type Widget = Counter;
    type Msg = CounterMsg;

    fn init_state(&mut self, widget: &Counter, cx: &mut Cx<'_, Self>) {
        if widget.timer && widget.visible {
            self.since = Some(widget.frozen_ms.map_or_else(|| widget.now(cx), |_| 0));
            self.tick(widget.now(cx));
        }
        self.schedule(widget, cx);
    }

    fn did_update_widget(&mut self, old: &Counter, widget: &Counter, cx: &mut Cx<'_, Self>) {
        if old.timer != widget.timer
            || old.visible != widget.visible
            || old.frozen_ms != widget.frozen_ms
        {
            let now = widget.now(cx);
            self.tick(now);
            if !widget.timer {
                self.since = None;
            } else if self.since.is_none() && widget.visible {
                self.since = Some(now);
            }
            self.schedule(widget, cx);
        }
    }

    fn update(&mut self, msg: CounterMsg, widget: &Counter, cx: &mut Cx<'_, Self>) -> Rebuild {
        self.tick(widget.now(cx));
        let value = match msg {
            CounterMsg::Count => self.count.value().saturating_add(1),
            CounterMsg::Reset => 0,
            CounterMsg::Tick => self.count.value(),
        };
        self.count.set_value(value);
        self.schedule(widget, cx);
        Rebuild::No
    }

    fn build(&self, _: &Counter, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let count = self.count.clone();
        column![
            row![
                Button::new("Count").on_pressed(cx.send(CounterMsg::Count)),
                Button::new("Reset").on_pressed(cx.send(CounterMsg::Reset)),
            ]
            .main_axis_size(MainAxisSize::Min)
            .spacing(2),
            ListenableBuilder::new(self.count.clone(), move |_, _| {
                Text::new(format!("ListenableBuilder: {}", count.value())).into()
            }),
            ValueListenableBuilder::new(self.count.clone(), |_, value, _| {
                Text::new(format!("ValueListenableBuilder: {value}")).into()
            }),
        ]
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .spacing(1)
        .into()
    }

    fn dispose(&mut self) {
        self.cancel_wake();
        self.count.dispose();
    }
}

impl_widget!(stateful eq Counter);

fn preview() -> AnyWidget {
    Counter { timer: true, visible: true, frozen_ms: None }.into()
}

```

### Options

| option | values | starts at |
| --- | --- | --- |
| timer | false, true | true |

## FutureBuilder, StreamBuilder

What is built from the latest snapshot of a future or a stream: waiting, done, or each event.

- `rtwk::foundation::FutureBuilder` (struct) in `crates/rtwk-framework/src/foundation/async_builders.rs:239`: Shows what its builder returns for the latest snapshot of its future (twk's `FutureBuilder`): waiting, then done with the value or the error, built in the flush the result comes in.
- `rtwk::foundation::StreamBuilder` (struct) in `crates/rtwk-framework/src/foundation/async_builders.rs:684`: Shows what its builder returns for the latest snapshot of its stream (twk's `StreamBuilder`): waiting once subscribed, active with each item or error, done when it ends; a new stream from the parent is listened to in place of the old, the snapshot `None`-connected in between.

### Code

```rust
use std::cell::RefCell;
use std::future::poll_fn;
use std::rc::Rc;
use std::task::{Poll, Waker};
use std::time::Duration;

use rtwk::animation::Spinner;
use rtwk::binding::AppBinding;
use rtwk::controls::Button;
use rtwk::foundation::{
    AsyncError, AsyncFuture, AsyncSnapshot, AsyncStream, ConnectionState, FutureBuilder,
    StreamBuilder, StreamSender,
};
use rtwk::layout::{CrossAxisAlignment, MainAxisSize, SizedBox};
use rtwk::style::Theme;
use rtwk::text::Text;
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, column, impl_widget, row};

const EVERY: Duration = Duration::from_secs(1);
const WAIT: Duration = Duration::from_secs(2);

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct AsyncDemo {
    pub error: bool,
    pub take: bool,
    pub visible: bool,
    pub frozen_ms: Option<u64>,
}

#[derive(Default)]
struct Completion {
    ready: bool,
    waker: Option<Waker>,
}

impl Completion {
    fn finish(&mut self) {
        self.ready = true;
        if let Some(waker) = self.waker.take() {
            waker.wake();
        }
    }
}

pub(crate) struct AsyncState {
    future: Option<AsyncFuture<&'static str>>,
    load_wake: Option<(AppBinding, u64)>,
    load_deadline: Option<Duration>,
    stream: AsyncStream<u64>,
    sender: StreamSender<u64>,
    stream_wake: Option<(AppBinding, u64)>,
    listened: Option<Duration>,
    completion: Option<Rc<RefCell<Completion>>>,
    generation: u64,
    sent: u64,
    take: bool,
}

impl StatefulWidget for AsyncDemo {
    type State = AsyncState;

    fn create_state(&self) -> AsyncState {
        let (sender, stream) = AsyncStream::channel();
        AsyncState {
            future: None,
            load_wake: None,
            load_deadline: None,
            stream,
            sender,
            stream_wake: None,
            listened: None,
            completion: None,
            generation: 0,
            sent: 0,
            take: self.take,
        }
    }
}

fn frame_time(cx: &Cx<'_, AsyncState>) -> Duration {
    AppBinding::maybe_of(cx).map_or(Duration::ZERO, |b| b.frame_time())
}

impl AsyncState {
    fn tick(&mut self, now: Duration) {
        let Some(since) = self.listened else {
            return;
        };
        let elapsed = now.saturating_sub(since).as_secs();
        let count = if self.take { elapsed.min(5) } else { elapsed };
        if self.sent < count {
            self.sent = count;
            self.sender.send(count);
        }
        if self.take && self.sent == 5 {
            self.sender.close();
        }
    }

    fn sync_stream(&mut self, widget: &AsyncDemo, cx: &Cx<'_, Self>) {
        let now = widget
            .frozen_ms
            .map_or_else(|| frame_time(cx), Duration::from_millis);
        if self.take != widget.take {
            self.sender.close();
            (self.sender, self.stream) = AsyncStream::channel();
            self.take = widget.take;
            self.sent = 0;
            self.listened = Some(now);
        } else if self.listened.is_none() && widget.visible {
            self.listened = Some(widget.frozen_ms.map_or(now, |_| Duration::ZERO));
        }
        cancel(&mut self.stream_wake);
        if !widget.visible {
            return;
        }
        self.tick(now);
        if widget.frozen_ms.is_none()
            && (!self.take || self.sent < 5)
            && let (Some(since), Some(binding)) = (self.listened, AppBinding::maybe_of(cx))
        {
            let next = since.saturating_add(EVERY + Duration::from_secs(self.sent));
            let id =
                binding.schedule_frame_callback(cx.map(|_: Duration| AsyncMsg::Tick), Some(next));
            self.stream_wake = Some((binding, id));
        }
    }

    fn load(&mut self, widget: &AsyncDemo, cx: &mut Cx<'_, Self>) {
        cancel(&mut self.load_wake);
        self.completion = None;
        self.generation += 1;
        let completion = Rc::new(RefCell::new(Completion::default()));
        let done = completion.clone();
        let error = widget.error;
        self.future = Some(AsyncFuture::try_new(async move {
            poll_fn(move |cx| {
                let mut done = done.borrow_mut();
                if done.ready {
                    if error {
                        Poll::Ready(Err(
                            Rc::new(std::io::Error::other("Exception: offline")) as AsyncError
                        ))
                    } else {
                        Poll::Ready(Ok("loaded"))
                    }
                } else {
                    done.waker = Some(cx.waker().clone());
                    Poll::Pending
                }
            })
            .await
        }));

        self.completion = Some(completion);
        self.load_deadline = Some(frame_time(cx).saturating_add(WAIT));
        self.sync_load(widget, cx);
    }

    fn sync_load(&mut self, widget: &AsyncDemo, cx: &Cx<'_, Self>) {
        cancel(&mut self.load_wake);
        let Some(deadline) = self.load_deadline.filter(|_| widget.visible) else {
            return;
        };
        if widget.frozen_ms.is_some() || frame_time(cx) >= deadline {
            self.load_deadline = None;
            if let Some(completion) = self.completion.take() {
                completion.borrow_mut().finish();
            }
        } else if let Some(binding) = AppBinding::maybe_of(cx) {
            let generation = self.generation;
            let id = binding.schedule_frame_callback(
                cx.map(move |_: Duration| AsyncMsg::Loaded(generation)),
                Some(deadline),
            );
            self.load_wake = Some((binding, id));
        }
    }
}

fn cancel(wake: &mut Option<(AppBinding, u64)>) {
    if let Some((binding, id)) = wake.take() {
        binding.cancel_frame_callback(id);
    }
}

#[derive(Clone, PartialEq)]
pub(crate) enum AsyncMsg {
    Load,
    Tick,
    Loaded(u64),
}

impl State for AsyncState {
    type Widget = AsyncDemo;
    type Msg = AsyncMsg;

    fn init_state(&mut self, widget: &AsyncDemo, cx: &mut Cx<'_, Self>) {
        self.sync_stream(widget, cx);
    }

    fn did_update_widget(&mut self, _: &AsyncDemo, widget: &AsyncDemo, cx: &mut Cx<'_, Self>) {
        self.sync_stream(widget, cx);
        self.sync_load(widget, cx);
    }

    fn update(&mut self, msg: AsyncMsg, widget: &AsyncDemo, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            AsyncMsg::Load => {
                self.load(widget, cx);
                Rebuild::Yes
            }
            AsyncMsg::Tick => {
                self.sync_stream(widget, cx);
                Rebuild::No
            }
            AsyncMsg::Loaded(generation) if generation == self.generation => {
                self.sync_load(widget, cx);
                Rebuild::No
            }
            AsyncMsg::Loaded(_) => Rebuild::No,
        }
    }

    fn build(&self, _: &AsyncDemo, cx: &mut Cx<'_, Self>) -> AnyWidget {
        column![
            Button::new("Load").on_pressed(cx.send(AsyncMsg::Load)),
            FutureBuilder::new(self.future.clone(), |cx, snapshot| {
                let theme = Theme::of(cx);
                let content: AnyWidget = match snapshot.connection_state {
                    ConnectionState::None => Text::new("future: not started").into(),
                    ConnectionState::Waiting => {
                        row![Text::new("future:"), Spinner::new()].spacing(1).into()
                    }
                    _ => match &snapshot.error {
                        Some(error) => Text::new(format!("future: {error}"))
                            .style(theme.error.clone())
                            .into(),
                        None => Text::new(format!("future: {}", snapshot.data.unwrap_or_default()))
                            .style(theme.success.clone())
                            .into(),
                    },
                };
                shown(content, snapshot, cx)
            }),
            StreamBuilder::new(Some(self.stream.clone()), |cx, snapshot| {
                shown(
                    Text::new(format!("stream: {} s", snapshot.data.unwrap_or(0))),
                    snapshot,
                    cx,
                )
            }),
        ]
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .spacing(1)
        .into()
    }

    fn dispose(&mut self) {
        self.sender.close();
        cancel(&mut self.stream_wake);
        cancel(&mut self.load_wake);
    }
}

impl_widget!(stateful eq AsyncDemo);

fn shown<T>(
    content: impl Into<AnyWidget>,
    snapshot: &AsyncSnapshot<T>,
    cx: &mut Cx<'_>,
) -> AnyWidget {
    let theme = Theme::of(cx);
    let state = match snapshot.connection_state {
        ConnectionState::None => "none",
        ConnectionState::Waiting => "waiting",
        ConnectionState::Active => "active",
        ConnectionState::Done => "done",
    };
    row![
        SizedBox::new(Some(26), None, Some(content.into())),
        Text::new(state).style(theme.muted.clone()),
    ]
    .main_axis_size(MainAxisSize::Min)
    .spacing(1)
    .into()
}

fn preview() -> AnyWidget {
    AsyncDemo { error: false, take: false, visible: true, frozen_ms: None }.into()
}

```

### Options

| option | values | starts at |
| --- | --- | --- |
| future | value, error | value |
| stream | endless, take(5) | endless |
