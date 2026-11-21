//! Page tests that bring their own harness setup, one binary: the mouse tests of the pages,
//! the composer draft, and the probes of the open rtwk gaps (`pending(Gxx)`). The page tests
//! of `tests/screens.rs` stay there; this binary holds what once had a binary each
//! (docs/rtwk-port/build-speed.md). A new file under `tests/pages/` is registered in
//! `tests/screens.rs`, unless it needs a module root of its own: then it is a line here.

// the kit and the harness it drives are crate-level modules (`crate::common`), as in `screens`
#[path = "common/mod.rs"]
mod common;
#[path = "kit/mod.rs"]
mod kit;

#[path = "pages/agents_mouse.rs"]
mod agents_mouse;
#[path = "pages/composer_draft.rs"]
mod composer_draft;
#[path = "pages/composer_mouse.rs"]
mod composer_mouse;
#[path = "pages/startup_mouse.rs"]
mod startup_mouse;

mod rtwk_g46;
mod rtwk_g49_g51;
mod rtwk_g9;
