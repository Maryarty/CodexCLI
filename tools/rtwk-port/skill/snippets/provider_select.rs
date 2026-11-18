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
