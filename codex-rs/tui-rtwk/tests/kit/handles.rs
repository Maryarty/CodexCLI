//! The controllers of the open chat's thread, for a test that sets a state by the handle a package
//! works through (Find's finder, the mode the transcript takes keys in) instead of by keys.

use std::cell::RefCell;
use std::rc::Rc;

use codex_tui_rtwk::app::UiApp;
use codex_tui_rtwk::widgets::thread_ui::ThreadUi;

use super::Screen;

impl Screen {
    /// The `ThreadUi` of the thread the screen shows (the blank one before a thread opens).
    pub fn thread_ui(&mut self) -> Rc<ThreadUi> {
        let slot: Rc<RefCell<Option<Rc<ThreadUi>>>> = Rc::default();
        let grab = slot.clone();
        self.h.t.update_app(move |app: &mut UiApp, _| {
            *grab.borrow_mut() = Some(app.ui.thread_ui.get(app.core.session().active.as_ref()));
        });
        slot.borrow_mut().take().expect("the app's controllers")
    }
}
