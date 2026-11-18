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
