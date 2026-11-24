#![allow(clippy::expect_used, clippy::unwrap_used)]

//! State tests of the guardian's reviews as thread facts (task fw1-c6, oracle
//! `chatwidget/guardian.rs`, `app/guardian.rs`): a review under way is on the thread's slot until
//! it completes, a denial is kept after it, and both reach the thread their notification names
//! through the app's router. The "Reviewing approval request" row and the denial warning are the
//! UI's (they read `ThreadPart.reviews` and `recent_denials`).

use codex_tui_core::fixtures::sandbox::Sandbox;
use codex_tui_core::ids::ThreadId;
use pretty_assertions::assert_eq;

const SESSION: &str = r#"{"step":"seed","seed":"session","thread":"thread-1"}"#;

fn notify(method: &str, params: &str) -> String {
    format!(r#"{{"step":"notify","message":{{"method":"{method}","params":{params}}}}}"#)
}

fn review(id: &str, status: &str, rationale: &str) -> String {
    let (method, done) = if status == "inProgress" {
        ("item/autoApprovalReview/started", "")
    } else {
        (
            "item/autoApprovalReview/completed",
            r#","completedAtMs":1,"decisionSource":"agent""#,
        )
    };
    notify(
        method,
        &format!(
            r#"{{"threadId":"thread-1","turnId":"turn-1","startedAtMs":0{done},"reviewId":"{id}","targetItemId":null,"review":{{"status":"{status}","riskLevel":null,"userAuthorization":null,"rationale":{rationale}}},"action":{{"type":"command","source":"shell","command":"rm -rf x","cwd":"/tmp"}}}}"#
        ),
    )
}

fn state(lines: &[String]) -> (Vec<String>, Vec<String>) {
    let script = format!("{SESSION}\n{}\n", lines.join("\n"));
    let sandbox = Sandbox::play(&script).expect("plays");
    let thread = sandbox
        .app()
        .thread(&ThreadId::new("thread-1"))
        .expect("the thread");
    let part = &thread.screen_slots().approvals;
    let running = part.reviews.iter().map(|r| r.id.clone()).collect();
    let kept = part.recent_denials.iter().map(|d| d.id.clone()).collect();
    (running, kept)
}

#[test]
fn a_review_under_way_is_on_the_thread_until_it_completes() {
    let (running, kept) = state(&[review("g1", "inProgress", "null")]);
    assert_eq!((running, kept), (vec!["g1".to_owned()], vec![]));
    let (running, kept) = state(&[
        review("g1", "inProgress", "null"),
        review("g1", "approved", "null"),
    ]);
    assert_eq!((running, kept), (vec![], vec![]));
}

#[test]
fn a_denial_is_kept_after_its_review_and_a_second_review_runs_beside_it() {
    let (running, kept) = state(&[
        review("g1", "inProgress", "null"),
        review("g2", "denied", r#""Destructive.""#),
    ]);
    assert_eq!(
        (running, kept),
        (vec!["g1".to_owned()], vec!["g2".to_owned()])
    );
}
