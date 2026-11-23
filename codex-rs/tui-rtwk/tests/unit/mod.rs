//! The unit tests moved out of the src files by tools/rtwk-port/unit-tests/move.py
//! (one file per moved module, the path below src mirrored; this list is generated).

#![allow(
    unused_imports,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::disallowed_methods
)]

#[path = "services/effects.rs"]
mod services_effects;
#[path = "services/images.rs"]
mod services_images;
#[path = "services/open_url.rs"]
mod services_open_url;
#[path = "services/remote.rs"]
mod services_remote;
#[path = "services/shell.rs"]
mod services_shell;
#[path = "services/startup.rs"]
mod services_startup;
#[path = "services/transcript_file.rs"]
mod services_transcript_file;
#[path = "sheets/settings/draft.rs"]
mod sheets_settings_draft;
#[path = "widgets/age.rs"]
mod widgets_age;
#[path = "widgets/composer_sparkle.rs"]
mod widgets_composer_sparkle;
#[path = "widgets/key_guard.rs"]
mod widgets_key_guard;
#[path = "widgets/measure.rs"]
mod widgets_measure;
#[path = "widgets/session_list.rs"]
mod widgets_session_list;
#[path = "widgets/tab_strip.rs"]
mod widgets_tab_strip;
