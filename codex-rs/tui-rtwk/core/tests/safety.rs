#![allow(clippy::expect_used, clippy::unwrap_used)]

//! State tests of the thread's safety facts (task fw1-c6, oracle `chatwidget/safety_buffering.rs`,
//! `safety_recourse.rs`, `misalignment_policy.rs` and `app/safety_*.rs`, `app/misalignment_
//! policy.rs`): the output held back during a safety check and the retry offered with it, the
//! rewrites offered after a refusal, and the stop after a misalignment failure that blocks input
//! until a new turn. Every sheet and sentence is the UI's; core keeps the facts and the verbs.

use codex_app_server_protocol::ServerNotification;
use codex_tui_core::app::{AppEv, AppFx, BackendEv, CodexApp, Command, Intent, apply_app};
use codex_tui_core::fixtures::builders::{agent_delta, turn_failed, turn_started};
use codex_tui_core::ids::{ThreadId, Ticket, TurnId};
use codex_tui_core::rpc::Reply;
use codex_tui_core::screens::{ScreenIntent, chat};
use codex_tui_core::threads::safety::{Domain, Verb};
use codex_tui_core::threads::{Item, Notice, UserMessage};
use serde_json::{Value, json};

const T: &str = "thread-1";

fn run(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    fx
}

fn ready() -> CodexApp {
    let mut app = CodexApp::new();
    let fx = run(&mut app, AppEv::Connected);
    let ticket = fx
        .commands()
        .iter()
        .find_map(|c| match c {
            Command::StartThread { ticket, .. } => Some(*ticket),
            _ => None,
        })
        .expect("a StartThread");
    run(
        &mut app,
        AppEv::Reply(ticket, Reply::thread_started(ThreadId::new(T))),
    );
    app
}

fn notify(app: &mut CodexApp, message: Value) -> AppFx {
    let notification: ServerNotification = serde_json::from_value(message).expect("notification");
    run(
        app,
        AppEv::Backend(BackendEv::Notification(Box::new(notification))),
    )
}

fn submit(app: &mut CodexApp, text: &str) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Submit {
            thread: Some(ThreadId::new(T)),
            message: UserMessage::text(text),
        }),
    )
}

fn start(fx: &AppFx) -> Option<Ticket> {
    fx.commands().iter().find_map(|c| match c {
        Command::StartTurn { ticket, .. } => Some(*ticket),
        _ => None,
    })
}

/// A turn submitted here and started by the server: the retry can be offered for it.
fn submitted_turn(app: &mut CodexApp, text: &str, turn: &str) {
    let ticket = start(&submit(app, text)).expect("a start");
    run(
        app,
        AppEv::Reply(
            ticket,
            Reply::turn_started(ThreadId::new(T), TurnId::new(turn)),
        ),
    );
}

fn verb(app: &mut CodexApp, verb: Verb) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Chat(chat::Intent::Safety {
            thread: ThreadId::new(T),
            verb,
        }))),
    )
}

fn buffering(turn: &str, show: bool, faster: Option<&str>) -> Value {
    json!({"method": "model/safetyBuffering/updated", "params": {
        "threadId": T, "turnId": turn, "model": "gpt-big", "useCases": [], "reasons": [],
        "showBufferingUi": show, "fasterModel": faster}})
}

fn rows(app: &CodexApp) -> Vec<Item> {
    let thread = app.active_thread();
    let transcript = thread.transcript();
    (0..transcript.len())
        .filter_map(|index| transcript.get(index).map(|(_, item)| item.clone()))
        .collect()
}

mod buffering_during_a_check {
    use super::*;

    fn state(app: &CodexApp) -> Option<(bool, Option<String>, u32)> {
        app.active_thread()
            .safety()
            .buffering()
            .map(|held| (held.waiting(), held.retry.clone(), held.revision))
    }

    /// The check of a turn submitted here holds the output and offers the retry with the faster
    /// model; the same news again is the prompt already shown, the retry offer going away is a
    /// prompt to show again; the first output lifts the hold.
    #[test]
    fn a_check_holds_the_output_until_the_agent_speaks() {
        let mut app = ready();
        submitted_turn(&mut app, "hello", "turn-1");
        notify(&mut app, buffering("turn-1", true, Some("gpt-fast")));
        assert_eq!(state(&app), Some((true, Some("gpt-fast".into()), 1)));
        assert!(app.active_thread().safety().is_waiting());
        notify(&mut app, buffering("turn-1", true, Some("gpt-fast")));
        assert_eq!(state(&app), Some((true, Some("gpt-fast".into()), 1)));
        notify(&mut app, buffering("turn-1", true, None));
        assert_eq!(state(&app), Some((true, None, 2)));
        run(
            &mut app,
            AppEv::Backend(agent_delta(T, "turn-1", "a1", "Sure")),
        );
        assert_eq!(state(&app).map(|s| s.0), Some(false));
        assert!(!app.active_thread().safety().is_waiting());
        // after the answer started the check changes nothing
        notify(&mut app, buffering("turn-1", true, Some("gpt-fast")));
        assert_eq!(state(&app), Some((false, None, 2)));
    }

    /// A turn this process did not submit (or a thread it only watches) is held without the
    /// offer; `show: false` ends the hold; the end of the turn clears it.
    #[test]
    fn no_retry_is_offered_for_a_turn_not_submitted_here() {
        let mut app = ready();
        run(&mut app, AppEv::Backend(turn_started(T, "turn-1")));
        notify(&mut app, buffering("turn-1", true, Some("gpt-fast")));
        assert_eq!(state(&app), Some((true, None, 1)));
        notify(&mut app, buffering("turn-1", false, None));
        assert_eq!(state(&app), None);
        notify(&mut app, buffering("turn-1", true, None));
        run(&mut app, AppEv::Backend(turn_failed(T, "turn-1")));
        assert_eq!(state(&app), None);
    }

    /// A check for another turn than the last, or with no turn running, is not held.
    #[test]
    fn a_check_of_another_turn_or_of_no_turn_is_ignored() {
        let mut app = ready();
        notify(&mut app, buffering("turn-1", true, None));
        assert_eq!(state(&app), None);
        run(&mut app, AppEv::Backend(turn_started(T, "turn-1")));
        notify(&mut app, buffering("turn-0", true, None));
        assert_eq!(state(&app), None);
    }

    /// The retry is a verb: valid while the output is held, for the model offered; core records
    /// the request (the turn, the model and the message sent again) for the app and the receipt
    /// clears it.
    #[test]
    fn the_retry_verb_is_checked_and_recorded_for_the_app() {
        let mut app = ready();
        submitted_turn(&mut app, "hello", "turn-1");
        notify(&mut app, buffering("turn-1", true, Some("gpt-fast")));
        verb(
            &mut app,
            Verb::Retry {
                turn: TurnId::new("turn-1"),
                model: "gpt-other".into(),
            },
        );
        assert_eq!(app.active_thread().safety().retry(), None);
        verb(
            &mut app,
            Verb::Retry {
                turn: TurnId::new("turn-1"),
                model: "gpt-fast".into(),
            },
        );
        let thread = app.active_thread();
        let retry = thread.safety().retry().expect("a retry request");
        assert_eq!(
            (
                retry.turn.to_string(),
                retry.model.as_str(),
                retry.message.text.as_str()
            ),
            ("turn-1".to_owned(), "gpt-fast", "hello")
        );
        verb(&mut app, Verb::Taken);
        assert_eq!(app.active_thread().safety().retry(), None);
    }
}

mod recourse {
    use super::*;

    fn loading() -> Value {
        json!({"method": "turn/moderationMetadata", "params": {"threadId": T, "turnId": "turn-1",
            "metadata": {"safety_block": {"schema_version": 1, "domain": "cyber",
                                          "status": "loading"}}}})
    }

    fn result(prompts: [&str; 3]) -> Value {
        let suggestion = |label: &str, prompt: &str| json!({"label": label, "prompt_text": prompt});
        json!({"method": "turn/moderationMetadata", "params": {"threadId": T, "turnId": "turn-1",
        "metadata": {"safety_recourse": {
            "schema_version": 1, "domain": "cyber", "source": "user_request",
            "source_message": {"id": "m1", "kind": "ordinary_message", "role": "user"},
            "topic": "high_risk_cybersecurity_tools",
            "suggestions": [
                suggestion("Explain defenses", prompts[0]),
                suggestion("Incident response", prompts[1]),
                suggestion("Hardening", prompts[2]),
            ]}}}})
    }

    fn refusal() -> Value {
        json!({"method": "error", "params": {"threadId": T, "turnId": "turn-1", "willRetry": false,
            "error": {"message": "refused", "codexErrorInfo": "cyberPolicy",
                      "additionalDetails": null}}})
    }

    fn refused_with_alternatives() -> CodexApp {
        let mut app = ready();
        submitted_turn(&mut app, "hello", "turn-1");
        notify(&mut app, loading());
        assert!(app.active_thread().safety().is_preparing_alternatives());
        notify(
            &mut app,
            result([
                "Explain how to defend a network",
                "Plan an incident response",
                "Harden a server",
            ]),
        );
        notify(&mut app, refusal());
        app
    }

    /// The refusal that comes before the end of the turn waits for it: the turn still runs and
    /// there is no row yet. The end of the turn shows the refusal's cell and offers the three
    /// alternatives, the explanation worded by the contract.
    #[test]
    fn the_refusal_waits_for_the_turn_and_offers_the_alternatives() {
        let mut app = refused_with_alternatives();
        assert!(app.active_thread().is_running());
        assert_eq!(rows(&app), vec![]);
        run(&mut app, AppEv::Backend(turn_failed(T, "turn-1")));
        assert!(!app.active_thread().is_running());
        assert!(matches!(
            rows(&app).as_slice(),
            [Item::Notice(Notice::CyberPolicy(_))]
        ));
        let thread = app.active_thread();
        let offer = thread.safety().offer().expect("an offer");
        assert_eq!(offer.domain, Domain::Cyber);
        assert_eq!(
            offer.explanation,
            "We take extra caution with requests that could enable system disruption or the \
             development of high-risk cyber tools. You can ask Codex for an overview of system \
             protection and incident recovery."
        );
        assert_eq!(
            offer
                .suggestions
                .iter()
                .map(|(label, _)| label.as_str())
                .collect::<Vec<_>>(),
            ["Explain defenses", "Incident response", "Hardening"]
        );
        assert!(!thread.safety().is_preparing_alternatives());
    }

    /// The rewrite is a verb for one of the offered prompts, once the turn is over; core records
    /// the request for the app (the fork before the blocked turn is the app's).
    #[test]
    fn a_rewrite_is_recorded_only_for_an_offered_prompt() {
        let mut app = refused_with_alternatives();
        run(&mut app, AppEv::Backend(turn_failed(T, "turn-1")));
        verb(
            &mut app,
            Verb::Rewrite {
                turn: TurnId::new("turn-1"),
                prompt: "something else".into(),
            },
        );
        assert_eq!(app.active_thread().safety().rewrite(), None);
        verb(
            &mut app,
            Verb::Rewrite {
                turn: TurnId::new("turn-1"),
                prompt: "Harden a server".into(),
            },
        );
        let thread = app.active_thread();
        let rewrite = thread.safety().rewrite().expect("a rewrite request");
        assert_eq!(rewrite.prompt, "Harden a server");
        verb(&mut app, Verb::Taken);
        assert_eq!(app.active_thread().safety().rewrite(), None);
    }

    /// Without the alternatives (no recourse metadata) the refusal is the plain row at once and
    /// nothing is offered; a new turn forgets an offer.
    #[test]
    fn a_refusal_without_metadata_is_the_plain_row_and_a_new_turn_forgets_an_offer() {
        let mut app = ready();
        submitted_turn(&mut app, "hello", "turn-1");
        notify(&mut app, refusal());
        assert!(matches!(
            rows(&app).as_slice(),
            [Item::Notice(Notice::CyberPolicy(_))]
        ));
        assert_eq!(app.active_thread().safety().offer(), None);
        let mut app = refused_with_alternatives();
        run(&mut app, AppEv::Backend(turn_failed(T, "turn-1")));
        assert!(app.active_thread().safety().offer().is_some());
        run(&mut app, AppEv::Backend(turn_started(T, "turn-2")));
        assert_eq!(app.active_thread().safety().offer(), None);
    }
}

mod misalignment {
    use super::*;

    fn failure(turn: &str, explanation: Option<&str>, steer: Option<&str>) -> Value {
        json!({"method": "error", "params": {"threadId": T, "turnId": turn, "willRetry": false,
            "error": {"message": "Chat stopped", "codexErrorInfo": "misalignmentPolicyViolation",
                      "additionalDetails": null,
                      "misalignment": {"errorType": null, "detailedExplanation": explanation,
                                       "steer": steer.map(|message| json!({"message": message}))}}}})
    }

    fn stopped() -> CodexApp {
        let mut app = ready();
        submitted_turn(&mut app, "first", "turn-1");
        // a second message waits behind the turn
        submit(&mut app, "second");
        notify(
            &mut app,
            failure(
                "turn-1",
                Some("The agent drifted."),
                Some("Please continue carefully"),
            ),
        );
        app
    }

    fn review_seq(app: &CodexApp) -> u64 {
        let thread = app.active_thread();
        thread
            .safety()
            .violation()
            .and_then(|violation| violation.review.as_ref())
            .map(|review| review.seq)
            .expect("a review")
    }

    /// The failure ends the turn with no row, drops what waited, keeps the findings as a review
    /// and blocks input; the turn's own completion carrying the same failure changes nothing.
    #[test]
    fn the_failure_stops_the_chat_and_keeps_the_findings() {
        let mut app = stopped();
        let thread = app.active_thread();
        assert!(thread.safety().blocks_input());
        assert!(!thread.is_running());
        assert!(thread.queue().is_empty());
        assert_eq!(rows(&app), vec![]);
        let seq = review_seq(&app);
        let thread = app.active_thread();
        let review = thread
            .safety()
            .violation()
            .and_then(|violation| violation.review.as_ref())
            .unwrap();
        assert_eq!(review.findings(), "The agent drifted.");
        assert_eq!(review.continuation(), Some("Please continue carefully"));
        run(&mut app, AppEv::Backend(turn_failed(T, "turn-1")));
        assert_eq!(review_seq(&app), seq);
    }

    /// While stopped, ordinary input starts nothing and is not queued.
    #[test]
    fn input_is_rejected_while_stopped() {
        let mut app = stopped();
        let fx = submit(&mut app, "more");
        assert_eq!(start(&fx), None);
        assert!(app.active_thread().queue().is_empty());
        assert!(app.active_thread().safety().blocks_input());
    }

    /// A failure of a turn that is not the last one is not the thread's.
    #[test]
    fn a_failure_of_another_turn_is_ignored() {
        let mut app = ready();
        submitted_turn(&mut app, "first", "turn-1");
        notify(&mut app, failure("turn-0", Some("Old."), None));
        assert_eq!(app.active_thread().safety().violation(), None);
    }

    /// Findings that are blank carry no review (nothing to continue from) but still block input.
    #[test]
    fn blank_findings_block_input_without_a_review() {
        let mut app = ready();
        submitted_turn(&mut app, "first", "turn-1");
        notify(&mut app, failure("turn-1", Some("  "), Some("Go on")));
        let thread = app.active_thread();
        let violation = thread.safety().violation().expect("a violation");
        assert!(violation.review.is_none());
        assert!(thread.safety().blocks_input());
    }

    /// The continuation is a verb naming the review on screen. A stale number does nothing; the
    /// current one starts a turn with the server's instruction and the acknowledgement; the new
    /// turn lifts the block.
    #[test]
    fn continuing_needs_the_current_review_and_lifts_the_block_with_the_new_turn() {
        let mut app = stopped();
        let seq = review_seq(&app);
        let fx = verb(&mut app, Verb::Continue { review: seq + 1 });
        assert_eq!(start(&fx), None);
        let fx = verb(&mut app, Verb::Continue { review: seq });
        let (ticket, text, stamp) = fx
            .commands()
            .iter()
            .find_map(|c| match c {
                Command::StartTurn {
                    ticket,
                    message,
                    misalignment_override,
                    ..
                } => Some((*ticket, message.text.clone(), *misalignment_override)),
                _ => None,
            })
            .expect("a start");
        assert_eq!(text, "Please continue carefully");
        assert!(stamp.is_some());
        assert!(app.active_thread().safety().blocks_input());
        run(
            &mut app,
            AppEv::Reply(
                ticket,
                Reply::turn_started(ThreadId::new(T), TurnId::new("turn-2")),
            ),
        );
        assert!(!app.active_thread().safety().blocks_input());
        assert_eq!(app.active_thread().safety().violation(), None);
    }

    /// A turn the server starts on its own lifts the block too (`clear_misalignment_for_new_turn`).
    #[test]
    fn a_new_turn_from_the_server_lifts_the_block() {
        let mut app = stopped();
        run(&mut app, AppEv::Backend(turn_started(T, "turn-2")));
        assert!(!app.active_thread().safety().blocks_input());
    }
}
