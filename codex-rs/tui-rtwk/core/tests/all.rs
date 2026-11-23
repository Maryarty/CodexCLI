//! The core's integration tests, one binary: every `tests/<name>.rs` is a module here.
//!
//! One binary instead of one per file: each test binary is a rustc run and a link of the
//! whole dependency set, and every edit of the core relinked all 25 of them (the
//! build-speed task, docs/rtwk-port/build-speed.md). A new test file is a new line in
//! this list (the rules test `every_test_file_is_registered` fails without it).

mod adr003;
mod agents;
mod agents_backends;
mod agents_details;
mod agents_followups;
mod agents_live;
mod agents_reconnect;
mod agents_shared;
mod app;
mod app_wiring;
mod approvals;
mod apps_directory;
mod backend_switch;
mod backtrack;
mod banners;
mod cells;
mod citations;
mod commands;
mod composer_history;
mod composer_shell;
mod composer_vim;
mod dialogs;
mod directives;
mod export;
mod goal;
mod import;
mod keymap;
mod model;
mod models;
mod onboarding;
mod permission_args;
mod permissions;
mod pets;
mod plan_mode;
mod plugins;
mod popovers;
mod provider_error;
mod queue;
mod quick_dials;
mod rename_prompt;
mod resume;
mod review_popup;
mod script;
mod select;
mod select_live;
mod selection;
mod settings;
mod sparkle;
mod status;
mod status_data;
mod status_facts;
mod submission;
mod tips;
mod transcript_diff;
mod unit;
mod usage;
mod worked_for;
mod worktree_popups;
