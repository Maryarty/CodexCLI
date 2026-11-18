#![allow(clippy::expect_used, clippy::unwrap_used)]

//! State tests of S19: the card a script's `screen` step shows holds the raw
//! values; the blended total is the oracle's (`TokenUsage::blended_total`).

use codex_tui_core::fixtures::sandbox::Sandbox;
use codex_tui_core::screens::status::{Approval, Profile};
use pretty_assertions::assert_eq;

const CARD: &str = r#"{"step":"screen","dir":"status","ev":{"kind":"card","version":"0.0.0","model":"gpt-5.1-codex-max","reasoning_effort":"high","reasoning_summary":"detailed","directory":"/w","profile":"workspace","approval":"on_request","tokens":{"input":1200,"cached_input":200,"output":900,"total":2250},"context":null,"limits":[]}}"#;

#[test]
fn the_card_keeps_the_values_it_was_shown_with() {
    let sandbox = Sandbox::play(CARD).expect("plays");
    let card = sandbox.app().screens.status.card.clone().expect("a card");
    assert_eq!(card.model, "gpt-5.1-codex-max");
    assert_eq!(card.profile, Profile::Workspace);
    assert_eq!(card.approval, Approval::OnRequest);
}

#[test]
fn the_blended_total_leaves_out_cached_input() {
    let sandbox = Sandbox::play(CARD).expect("plays");
    let card = sandbox.app().screens.status.card.clone().expect("a card");
    assert_eq!(card.tokens.blended_total(), 1_900);
}

#[test]
fn a_card_missing_a_field_is_a_bad_seed() {
    let err = Sandbox::play(r#"{"step":"screen","dir":"status","ev":{"kind":"card"}}"#)
        .err()
        .expect("fails");
    assert_eq!(err.line, 1);
}
