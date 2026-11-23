//! The unit tests moved out of the src files by tools/rtwk-port/unit-tests/move.py
//! (one file per moved module, the path below src mirrored; this list is generated).

#![allow(
    unused_imports,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::disallowed_methods
)]

#[path = "protocol/onboarding.rs"]
mod protocol_onboarding;
#[path = "screens/onboarding/auth.rs"]
mod screens_onboarding_auth;
#[path = "screens/onboarding/provider/bedrock.rs"]
mod screens_onboarding_provider_bedrock;
#[path = "screens/onboarding/provider/gateway.rs"]
mod screens_onboarding_provider_gateway;
#[path = "screens/onboarding/provider/gcp.rs"]
mod screens_onboarding_provider_gcp;
#[path = "screens/onboarding/provider/wire.rs"]
mod screens_onboarding_provider_wire;
#[path = "screens/startup/cwd.rs"]
mod screens_startup_cwd;
#[path = "screens/usage/date.rs"]
mod screens_usage_date;
#[path = "threads/daybreak.rs"]
mod threads_daybreak;
#[path = "threads/inline_vis.rs"]
mod threads_inline_vis;
#[path = "threads/queue.rs"]
mod threads_queue;
#[path = "threads/stream_preview.rs"]
mod threads_stream_preview;
