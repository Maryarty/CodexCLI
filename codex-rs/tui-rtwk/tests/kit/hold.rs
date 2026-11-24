//! A request held in flight: the state of a screen that waits for one answer (`Refreshing...`,
//! `Loading`) while the other requests are answered at once by the respond table.

use codex_tui_rtwk::services::backend::fake::Hold;

use super::Screen;

impl Screen {
    /// Holds every request of wire `method` (`account/rateLimits/read`) the app makes from now
    /// on: it is sent (`requests_of` counts it) and stays unanswered until `release`, then is
    /// answered as the respond table says by then. Other methods are answered as before.
    pub fn hold_request(&mut self, method: &str) -> Hold {
        let hold = Hold::new();
        self.h.fake.hold_request(method, &hold);
        hold
    }

    /// Lets the requests `hold` kept (and the later ones of its method) be answered.
    pub fn release(&mut self, hold: &Hold) -> &mut Screen {
        hold.release();
        self.h.advance(100);
        self
    }
}
