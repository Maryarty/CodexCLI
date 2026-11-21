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
mod approvals;
mod backend_switch;
mod cells;
mod commands;
mod dialogs;
mod import;
mod model;
mod onboarding;
mod pets;
mod popovers;
mod resume;
mod script;
mod select;
mod settings;
mod status;
mod status_data;
mod usage;
