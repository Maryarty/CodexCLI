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
