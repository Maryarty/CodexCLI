//! State tests of S20 (the `/usage` dashboard): the remote panels of the
//! oracle's `Remote<T>` (`analytics/data.rs`: loading, unavailable, error,
//! ready) as events, and the report intent.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use codex_tui_rtwk_core::app::{AppEv, Intent};
use codex_tui_rtwk_core::fixtures::sandbox::Sandbox;
use codex_tui_rtwk_core::screens::ScreenIntent;
use codex_tui_rtwk_core::screens::usage::{
    Failure, Intent as UsageIntent, Overview, Panel, Report,
};
use pretty_assertions::assert_eq;

const SESSION: &str = r#"{"step":"seed","seed":"session","thread":"thread-1"}"#;

fn play(steps: &[&str]) -> Sandbox {
    Sandbox::play(&format!("{SESSION}\n{}\n", steps.join("\n"))).expect("plays")
}

#[test]
fn a_fresh_model_is_loading_everywhere() {
    let sandbox = play(&[]);
    let model = &sandbox.app().screens.usage;
    assert_eq!(model.overview, Panel::<Overview>::Loading);
    assert_eq!(model.history, Panel::Loading);
    assert_eq!(model.credits, Panel::Loading);
    assert_eq!(model.report, Report::Overview);
}

#[test]
fn panels_take_their_own_events_and_stay_independent() {
    let sandbox = play(&[
        r#"{"step":"screen","dir":"usage","ev":{"history":"unavailable"}}"#,
        r#"{"step":"screen","dir":"usage","ev":{"credits":{"failed":"credits"}}}"#,
        r#"{"step":"screen","dir":"usage","ev":{"usage":{"ready":[]}}}"#,
    ]);
    let model = &sandbox.app().screens.usage;
    assert_eq!(model.history, Panel::Unavailable);
    assert_eq!(model.credits, Panel::Failed(Failure::Credits));
    assert_eq!(model.usage, Panel::Ready(Vec::new()));
    assert_eq!(model.overview, Panel::Loading);
}

#[test]
fn an_overview_and_an_account_seed() {
    let sandbox = play(&[
        r#"{"step":"screen","dir":"usage","ev":{"account":{"name":"Example User","handle":"@example.user","plan":"Plus"}}}"#,
        r#"{"step":"screen","dir":"usage","ev":{"overview":{"ready":{"lifetime_tokens":142300000000,"peak_tokens":7000000000,"longest_chat_secs":158820,"current_streak_days":158,"longest_streak_days":160}}}}"#,
    ]);
    let model = &sandbox.app().screens.usage;
    assert_eq!(model.account.as_ref().expect("account").plan, "Plus");
    let Panel::Ready(overview) = &model.overview else {
        panic!("{:?}", model.overview)
    };
    assert_eq!(overview.longest_streak_days, 160);
}

#[test]
fn the_report_intent_switches_the_tab() {
    let mut sandbox = play(&[]);
    sandbox.send(AppEv::Intent(Intent::Screen(ScreenIntent::Usage(
        UsageIntent::Report(Report::Usage),
    ))));
    assert_eq!(sandbox.app().screens.usage.report, Report::Usage);
}
