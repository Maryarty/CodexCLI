//! The screen tests: one binary, the skeleton's own tests first, then one
//! module per page, sheet and app part (`tests/{pages,sheets,app}/<name>.rs`), so a slice adds tests in
//! its file and the build links one test binary, not one per directory.

mod common;
mod unit;

#[path = "app/cli.rs"]
mod cli;
#[path = "app/l10n.rs"]
mod l10n;
#[path = "app/navigation.rs"]
mod navigation;
#[path = "app/skeleton.rs"]
mod skeleton;

#[path = "pages/agents.rs"]
mod agents;
#[path = "sheets/approvals.rs"]
mod approvals;
#[path = "pages/backtrack.rs"]
mod backtrack;
#[path = "pages/cells.rs"]
mod cells;
#[path = "pages/chat.rs"]
mod chat;
#[path = "pages/composer.rs"]
mod composer;
#[path = "sheets/dialogs.rs"]
mod dialogs;
#[path = "pages/import.rs"]
mod import;
#[path = "pages/onboarding.rs"]
mod onboarding;
#[path = "sheets/pets.rs"]
mod pets;
#[path = "pages/popovers.rs"]
mod popovers;
#[path = "sheets/presenter.rs"]
mod presenter;
#[path = "pages/resume.rs"]
mod resume;
#[path = "sheets/select.rs"]
mod select;
#[path = "sheets/settings.rs"]
mod settings;
#[path = "pages/signin.rs"]
mod signin;
#[path = "pages/signin_wizards.rs"]
mod signin_wizards;
#[path = "pages/startup.rs"]
mod startup;
#[path = "pages/status.rs"]
mod status;
#[path = "pages/status_data.rs"]
mod status_data;
#[path = "pages/transcript.rs"]
mod transcript;
#[path = "pages/usage.rs"]
mod usage;
#[path = "sheets/voice.rs"]
mod voice;
