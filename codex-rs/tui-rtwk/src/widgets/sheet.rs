//! What a sheet declares (ADR-013 Ж): when it still has a reason to be on screen. A sheet is
//! shown for a request of a feature (the plugins of a directory, the setup of an installed
//! plugin); when the request is gone, the scope is no longer the current one or the thread it was
//! for changed, it closes by itself instead of staying over a screen it no longer belongs to. The
//! rules test `adr013_sheets_declare_alive` has every `impl Sheet` say it.

/// A sheet over a page.
pub trait Sheet {
    /// What the sheet reads of the app to decide: a selection, compared for equality.
    type Seen: Clone + PartialEq + 'static;

    /// Whether the sheet is still wanted, given what it sees now.
    fn alive(&self, seen: &Self::Seen) -> bool;
}
