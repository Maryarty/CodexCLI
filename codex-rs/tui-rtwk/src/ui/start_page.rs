//! The page the command line starts on (`codex-rtwk agents`, `codex-rtwk
//! resume`): the session is the navigator's home as ever, and the page is
//! pushed over it at once with nowhere to go back to but the session it
//! reveals when it ends.

use rtwk::navigation::{Navigator, PageRoute};
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

/// The session `chat` with `route`'s page pushed over it at mount.
#[derive(Clone, Debug, PartialEq)]
pub struct StartPage {
    pub chat: AnyWidget,
    pub route: fn() -> PageRoute<()>,
}

pub struct StartPageState;

impl StatefulWidget for StartPage {
    type State = StartPageState;

    fn create_state(&self) -> StartPageState {
        StartPageState
    }
}

impl_widget!(stateful eq StartPage);

impl State for StartPageState {
    type Widget = StartPage;
    type Msg = ();

    fn init_state(&mut self, _: &StartPage, cx: &mut Cx<'_, Self>) {
        let push = cx.send(());
        cx.emit(&push, ());
    }

    fn update(&mut self, _: (), w: &StartPage, cx: &mut Cx<'_, Self>) -> Rebuild {
        drop(Navigator::of(cx).push::<()>((w.route)()));
        Rebuild::No
    }

    fn build(&self, w: &StartPage, _: &mut Cx<'_, Self>) -> AnyWidget {
        w.chat.clone()
    }
}
