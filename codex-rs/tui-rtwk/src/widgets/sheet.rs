//! What a sheet declares (ADR-013 Ж): when it still has a reason to be on screen. A sheet is
//! shown for a request of a feature (the plugins of a directory, the setup of an installed
//! plugin); when the request is gone, the scope is no longer the current one or the thread it was
//! for changed, it closes by itself instead of staying over a screen it no longer belongs to. The
//! rules test `adr013_sheets_declare_alive` has every `impl Sheet` say it.

use std::cell::RefCell;
use std::rc::Rc;

use rtwk::navigation::NavigatorState;
use rtwk::provider::{InheritedProvider, Provider};

/// A sheet over a page.
pub trait Sheet {
    /// What the sheet reads of the app to decide: a selection, compared for equality.
    type Seen: Clone + PartialEq + 'static;

    /// Whether the sheet is still wanted, given what it sees now.
    fn alive(&self, seen: &Self::Seen) -> bool;
}

/// What an open sheet leaves for whoever wants a page shown in it: which sheet it is, the scope
/// it was opened for and how to reach its navigator. Written by the sheet's first route, cleared
/// when that goes.
#[derive(Clone)]
pub struct Opened {
    pub kind: &'static str,
    pub scope: String,
    pub nav: NavigatorState,
}

/// The place the open sheets leave their `Opened` in, one per kind of sheet. The presenter keeps
/// the door and the view provides it (never notifying: a reader looks when it needs to).
#[derive(Default)]
pub struct Door(RefCell<Vec<Opened>>);

impl Door {
    /// The sheet of `kind` is open for `scope`.
    pub fn set(&self, opened: Opened) {
        let mut all = self.0.borrow_mut();
        all.retain(|one| one.kind != opened.kind);
        all.push(opened);
    }

    /// The sheet of `kind` is gone.
    pub fn clear(&self, kind: &str) {
        self.0.borrow_mut().retain(|one| one.kind != kind);
    }

    /// The navigator of the sheet of `kind` that is open for `scope`.
    pub fn nav(&self, kind: &str, scope: &str) -> Option<NavigatorState> {
        self.0
            .borrow()
            .iter()
            .find(|one| one.kind == kind && one.scope == scope)
            .map(|one| one.nav.clone())
    }

    /// The provider of the door, for the view's `MultiProvider`.
    pub fn provider(door: Rc<Door>) -> InheritedProvider<Door> {
        Provider::value_shared(door).update_should_notify(|_, _| false)
    }
}
