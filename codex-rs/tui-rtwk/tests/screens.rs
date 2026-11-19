//! The screen tests: one binary, the skeleton's own tests first, then one
//! module per screen directory (`tests/<dir>.rs`), so a slice adds tests in
//! its file and the build links one test binary, not one per directory.

mod common;

#[path = "cli.rs"]
mod cli;
#[path = "l10n.rs"]
mod l10n;
#[path = "navigation.rs"]
mod navigation;
#[path = "skeleton.rs"]
mod skeleton;

#[path = "agents.rs"]
mod agents;
#[path = "approvals.rs"]
mod approvals;
#[path = "backtrack.rs"]
mod backtrack;
#[path = "cells.rs"]
mod cells;
#[path = "chat.rs"]
mod chat;
#[path = "composer.rs"]
mod composer;
#[path = "dialogs.rs"]
mod dialogs;
#[path = "import.rs"]
mod import;
#[path = "onboarding.rs"]
mod onboarding;
#[path = "pets.rs"]
mod pets;
#[path = "popovers.rs"]
mod popovers;
#[path = "presenter.rs"]
mod presenter;
#[path = "resume.rs"]
mod resume;
#[path = "select.rs"]
mod select;
#[path = "settings.rs"]
mod settings;
#[path = "sidebar.rs"]
mod sidebar;
#[path = "startup.rs"]
mod startup;
#[path = "status.rs"]
mod status;
#[path = "status_data.rs"]
mod status_data;
#[path = "transcript.rs"]
mod transcript;
#[path = "ui_tree.rs"]
mod ui_tree;
#[path = "usage.rs"]
mod usage;
#[path = "voice.rs"]
mod voice;
