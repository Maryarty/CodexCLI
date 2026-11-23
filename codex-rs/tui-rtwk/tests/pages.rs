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

#[path = "pages/agents_focus.rs"]
mod agents_focus;
#[path = "pages/agents_mouse.rs"]
mod agents_mouse;
#[path = "pages/chat_provider_error.rs"]
mod chat_provider_error;
#[path = "pages/chat_status.rs"]
mod chat_status;
#[path = "pages/composer_burst.rs"]
mod composer_burst;
#[path = "pages/composer_draft.rs"]
mod composer_draft;
#[path = "pages/composer_history.rs"]
mod composer_history;
#[path = "pages/composer_images.rs"]
mod composer_images;
#[path = "pages/composer_mentions.rs"]
mod composer_mentions;
#[path = "pages/composer_mouse.rs"]
mod composer_mouse;
#[path = "pages/composer_plain.rs"]
mod composer_plain;
#[path = "pages/composer_startup.rs"]
mod composer_startup;
#[path = "pages/composer_suggest.rs"]
mod composer_suggest;
#[path = "pages/composer_vim.rs"]
mod composer_vim;
#[path = "pages/history_pages.rs"]
mod history_pages;
#[path = "pages/onboarding_focus.rs"]
mod onboarding_focus;
#[path = "pages/plugins.rs"]
mod plugins;
#[path = "pages/resume_focus.rs"]
mod resume_focus;
#[path = "pages/review.rs"]
mod review;
#[path = "pages/selection_copy.rs"]
mod selection_copy;
#[path = "pages/startup_mouse.rs"]
mod startup_mouse;
#[path = "pages/text_mode.rs"]
mod text_mode;
#[path = "pages/transcript_follow.rs"]
mod transcript_follow;
#[path = "pages/transcript_pager.rs"]
mod transcript_pager;
#[path = "pages/transcript_sheet.rs"]
mod transcript_sheet;
#[path = "pages/worktree.rs"]
mod worktree;

mod rtwk_g46;
mod rtwk_g49_g51;
mod rtwk_g9;
mod rtwk_o1;
