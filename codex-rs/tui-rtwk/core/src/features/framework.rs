//! The framework with a list of its own: two probe features registered in `probe_list`, a
//! second invocation of the generating macro, so every generated enum and function is
//! exercised while the real list is still empty. What is checked is what ADR-012 promises:
//! one registration line gives the state, the root enums and their dispatch; the hooks run in
//! list order; an answer of another feature's shape is a mismatch, not a panic; a feature
//! without a component says `Never<Self>` and two of them do not overlap.
//!
//! The outbox (`AppCx::ask`, `effect`, `open`, `note`...) is typed with the real list's
//! roots, which hold only `goal` here: what a probe's step says is nothing, and the outbox is
//! exercised with the real roots (`a_feature_asks_the_host_through_the_outbox`).

use std::cell::RefCell;

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::ServerNotification;
use codex_app_server_protocol::ServerRequest;
use codex_app_server_protocol::ThreadClosedNotification;
use codex_app_server_protocol::ThreadGoalGetParams;

use super::{
    AppCx, AppView, Connection, Feature, Never, Out, Refusal, SeedFn, Slash, StructuredFailure,
    ThreadCx, Tone, TurnEnd,
};
use crate::app::effects::{Ask, Done};
use crate::app::facts::TuiSettings;
use crate::app::{AppFx, Now};
use crate::commands::SlashCommand;
use crate::ids::{BackendId, ThreadId, TicketMint};
use crate::rpc::RpcError;
use crate::screens::status::RateRead;
use crate::screens::{ScreenStates, SeedError};
use crate::threads::{Origin, ThreadEv, ThreadFx, ThreadSet, UserMessage};

/// What a hook saw.
#[derive(Debug, PartialEq, Eq)]
enum Hook {
    Resumed(ThreadId),
    TurnEnd(TurnEnd, bool),
    Interrupted,
    Settled,
    Shown(ThreadId),
    Connection(Connection),
    Opened(&'static str, Result<ThreadId, Refusal>),
    Structured(&'static str, Result<String, StructuredFailure>),
    Message(ThreadId, String),
    Rates(BackendId),
}

thread_local! {
    /// The probes' hooks, with the probe's name, in the order they ran.
    static LOG: RefCell<Vec<(&'static str, Hook)>> = const { RefCell::new(Vec::new()) };
}

fn log(probe: &'static str, hook: Hook) {
    LOG.with(|log| log.borrow_mut().push((probe, hook)));
}

fn logged() -> Vec<(&'static str, Hook)> {
    LOG.with(|log| std::mem::take(&mut *log.borrow_mut()))
}

/// A probe: counts what reaches it, and logs its hooks. Alpha has every component of its
/// own; beta has none of the note, fact, wish and interest (`Never<Self>`), which is the case
/// the root `From` impls must survive twice.
macro_rules! probe {
    ($module:ident, $marker:ident, $name:literal, $runs:expr, $($rest:tt)*) => {
        pub mod $module {
            use super::*;

            pub struct $marker;

            #[derive(Clone, Debug, PartialEq, Eq)]
            pub struct Intent(pub u32);
            #[derive(Clone, Debug, PartialEq, Eq)]
            pub struct ThreadIntent(pub u32);
            #[derive(Clone, Debug, PartialEq, Eq)]
            pub struct Command(pub String);
            #[derive(Clone, Debug, PartialEq, Eq)]
            pub struct Reply(pub u32);
            #[derive(Clone, Debug, PartialEq, Eq)]
            pub struct Ctx(pub &'static str);

            probe!(@components $marker, $($rest)*);

            impl Feature for $marker {
                const NAME: &'static str = $name;
                const RUNS: &'static [SlashCommand] = $runs;
                type State = u32;
                type ThreadPart = u32;
                type Target = Never<Self>;
                type Intent = Intent;
                type ThreadIntent = ThreadIntent;
                type Command = Command;
                type Reply = Reply;
                type Ctx = Ctx;
                type Note = Note;
                type Fact = Fact;
                type Wish = Wish;
                type Interest = Interest;
                type Event = Event;

                fn intent(state: &mut u32, intent: Intent, _cx: &mut AppCx<'_>) {
                    *state += intent.0;
                }

                fn slash(state: &mut u32, slash: Slash, _cx: &mut AppCx<'_>) {
                    *state += slash.args.len() as u32 + slash.source.map_or(0, |_| 100);
                }

                fn answered(
                    state: &mut u32,
                    _ctx: Ctx,
                    result: Result<Reply, RpcError>,
                    _cx: &mut AppCx<'_>,
                ) {
                    match result {
                        Ok(reply) => *state += reply.0,
                        Err(RpcError::Mismatch) => *state += 1000,
                        Err(_) => *state += 1,
                    }
                }

                fn effect_done(state: &mut u32, ctx: Ctx, done: Done, _cx: &mut AppCx<'_>) {
                    *state += ctx.0.len() as u32
                        + match done {
                            Done::Export(Ok(path)) => path.len() as u32,
                            Done::Export(Err(_)) => 1,
                            _ => 1000,
                        };
                }

                // A state (or part) of a thousand or more is a timer: that many seconds after
                // the epoch. Its intent adds nothing, so it stays asked for.
                fn wake_at(state: &u32) -> Option<(Now, Intent)> {
                    (*state >= 1000).then(|| (Now::at_secs(i64::from(*state)), Intent(0)))
                }

                fn thread_wake_at(part: &u32) -> Option<(Now, ThreadIntent)> {
                    (*part >= 1000).then(|| (Now::at_secs(i64::from(*part)), ThreadIntent(0)))
                }

                fn on_resumed(_state: &mut u32, thread: &ThreadId, _cx: &mut AppCx<'_>) {
                    log($name, Hook::Resumed(thread.clone()));
                }

                fn thread_opened(
                    _state: &mut u32,
                    ctx: Ctx,
                    result: Result<ThreadId, Refusal>,
                    _cx: &mut AppCx<'_>,
                ) {
                    log($name, Hook::Opened(ctx.0, result));
                }

                fn structured_done(
                    _state: &mut u32,
                    ctx: Ctx,
                    result: Result<String, StructuredFailure>,
                    _cx: &mut AppCx<'_>,
                ) {
                    log($name, Hook::Structured(ctx.0, result));
                }

                fn on_user_message(
                    _state: &mut u32,
                    thread: &ThreadId,
                    text: &str,
                    _cx: &mut AppCx<'_>,
                ) {
                    log($name, Hook::Message(thread.clone(), text.to_owned()));
                }

                fn on_rate_limits_read(
                    _state: &mut u32,
                    backend: BackendId,
                    _read: &RateRead,
                    _cx: &mut AppCx<'_>,
                ) {
                    log($name, Hook::Rates(backend));
                }

                fn fault_words(
                    _state: &u32,
                    _thread: &ThreadId,
                    message: &str,
                    _view: &AppView<'_>,
                ) -> Option<String> {
                    (message == $name).then(|| format!("{} says", $name))
                }

                fn structured_wanted(_state: &u32, ctx: &Ctx, _view: &AppView<'_>) -> bool {
                    ctx.0 != "stale"
                }

                fn on_shown(_state: &mut u32, thread: &ThreadId, _cx: &mut AppCx<'_>) {
                    log($name, Hook::Shown(thread.clone()));
                }

                fn on_connection(_state: &mut u32, connection: Connection, _cx: &mut AppCx<'_>) {
                    log($name, Hook::Connection(connection));
                }

                fn thread_intent(part: &mut u32, intent: ThreadIntent, _cx: &mut ThreadCx<'_, '_>) {
                    *part += intent.0;
                }

                fn thread_answered(
                    part: &mut u32,
                    _ctx: Ctx,
                    result: Result<Reply, RpcError>,
                    _cx: &mut ThreadCx<'_, '_>,
                ) {
                    *part += result.map_or(1000, |reply| reply.0);
                }

                fn on_turn_end(part: &mut u32, end: TurnEnd, cx: &mut ThreadCx<'_, '_>) {
                    *part += 1;
                    log($name, Hook::TurnEnd(end, cx.view.busy()));
                }

                fn on_interrupted(_part: &mut u32, _cx: &mut ThreadCx<'_, '_>) {
                    log($name, Hook::Interrupted);
                }

                fn on_settled(_part: &mut u32, _cx: &mut ThreadCx<'_, '_>) {
                    log($name, Hook::Settled);
                }

                fn wire(command: &Command, thread: &ThreadId, id: RequestId) -> ClientRequest {
                    ClientRequest::ThreadGoalGet {
                        request_id: id,
                        params: ThreadGoalGetParams {
                            thread_id: format!("{thread}:{}", command.0),
                        },
                    }
                }

                fn decode(command: &Command, result: serde_json::Value) -> Result<Reply, RpcError> {
                    result
                        .as_u64()
                        .map(|n| Reply(n as u32 + command.0.len() as u32))
                        .ok_or_else(|| RpcError::Malformed(command.0.clone()))
                }

                probe!(@steps $($rest)*);
            }
        }
    };
    // the components of a feature that has them
    (@components $marker:ident, full) => {
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub struct Note(pub &'static str);
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub struct Fact(pub u32);
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub struct Wish(pub &'static str);
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub struct Interest(pub u32);
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub struct Event(pub u32);
    };
    // and of one that has none
    (@components $marker:ident, bare) => {
        pub type Note = Never<$marker>;
        pub type Fact = Never<$marker>;
        pub type Wish = Never<$marker>;
        pub type Interest = Never<$marker>;
        pub type Event = Never<$marker>;
    };
    (@steps full) => {
        const SEED_NOTE: Option<SeedFn<Note>> = Some(|value| match value.as_str() {
            Some("error") => Ok(Note("error")),
            Some("fine") => Ok(Note("fine")),
            _ => Err(SeedError::Bad("bad note".to_owned())),
        });

        fn note_tone(note: &Note) -> Tone {
            if note.0 == "error" { Tone::Error } else { Tone::Info }
        }

        fn on_fact(state: &mut u32, fact: Fact, _cx: &mut AppCx<'_>) {
            *state += fact.0;
        }

        // the stream: a closed thread is an event worth the length of its id
        fn project(notification: &ServerNotification) -> Option<Event> {
            match notification {
                ServerNotification::ThreadClosed(closed) => {
                    Some(Event(closed.thread_id.len() as u32))
                }
                _ => None,
            }
        }

        // a request is observed, never answered
        fn project_request(_request: &ServerRequest) -> Option<Event> {
            Some(Event(100))
        }

        fn on_event(state: &mut u32, event: Event, _cx: &mut AppCx<'_>) {
            *state += event.0;
        }

        fn on_interest(state: &mut u32, interest: &Interest, open: bool, _cx: &mut AppCx<'_>) {
            *state += if open { interest.0 } else { 100 * interest.0 };
        }
    };
    (@steps bare) => {};
}

mod probe_list {
    use super::*;

    probe!(alpha, Alpha, "alpha", &[SlashCommand::Rename], full);
    probe!(
        beta,
        Beta,
        "beta",
        &[SlashCommand::Goal, SlashCommand::Copy],
        bare
    );

    feature_dirs! {
        alpha => Alpha;
        beta => Beta;
    }
}

use probe_list::{
    FeatureCommand, FeatureCtx, FeatureEvent, FeatureFact, FeatureIntent, FeatureInterest,
    FeatureNote, FeatureReply, FeatureStates, FeatureThreadIntent, FeatureThreadParts, alpha,
    answered, apply_intent, apply_slash, apply_thread_intent, apply_thread_reply, beta, decode,
    effect_done, fault_words, names, next_thread_wake, next_wake, note_tone, on_connection,
    on_event, on_fact, on_interest, on_interrupted, on_rate_limits_read, on_resumed, on_shown,
    on_turn_end, on_user_message, project, project_request, runs_of, seed_note, structured_done,
    structured_wanted, thread_opened, thread_wakes_due, wakes_due, wire,
};

/// Runs `step` with an `AppCx` over empty facts; what it asked for is nothing here.
fn app_step(step: impl FnOnce(&mut AppCx<'_>)) {
    let (threads, screens, tui) = (
        ThreadSet::default(),
        ScreenStates::default(),
        TuiSettings::default(),
    );
    let mut cx = AppCx::new(AppView::new(None, &threads, &screens, &tui, &[], None));
    step(&mut cx);
    assert!(cx.into_out().is_empty(), "a probe says nothing");
}

/// Runs `step` over the model of a thread this process holds, with its effects.
fn with_thread(step: impl FnOnce(&mut ThreadCx<'_, '_>)) {
    with_thread_after(|_| {}, step);
}

/// Like `with_thread`, after `said` made the thread's own effects (what `end_turn`'s
/// `send_next` does before the features' hooks run).
fn with_thread_after(
    said: impl FnOnce(&mut ThreadFx<'_>),
    step: impl FnOnce(&mut ThreadCx<'_, '_>),
) {
    let id = ThreadId::new("t1");
    let mut threads = ThreadSet::default();
    let model = threads.open(&id, Origin::Resumed);
    let (mut out, mut mint) = (AppFx::default(), TicketMint::default());
    let mut fx = ThreadFx::new(&id, &mut out, &mut mint);
    said(&mut fx);
    let mut cx = ThreadCx::new(model.view(), &mut fx);
    step(&mut cx);
}

#[test]
fn a_list_names_its_features_in_order_and_the_real_one_names_each_once() {
    let real = super::names();
    let mut sorted = real.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), real.len(), "{real:?}");
    assert_eq!(names(), ["alpha", "beta"]);
}

/// An interest goes to its feature as the feature's own type; a feature without interests has
/// none to receive (`Never<Self>`), so the beta probe's state stays what it was.
#[test]
fn an_interest_reaches_its_feature_opened_and_closed() {
    let mut states = FeatureStates::default();
    app_step(|cx| {
        on_interest(
            &mut states,
            &FeatureInterest::from(alpha::Interest(2)),
            true,
            cx,
        );
        on_interest(
            &mut states,
            &FeatureInterest::from(alpha::Interest(1)),
            false,
            cx,
        );
    });
    assert_eq!((*states.alpha, *states.beta), (2 + 100, 0));
}

/// A feature says when it wants the clock told (`Feature::wake_at`); the generated functions
/// give the nearest moment of all (what the app hands the shell's one timer) and the intents
/// whose moment the clock has reached, in list order, and nothing before. The clock is a value
/// the test makes up: nothing here waits for a second.
#[test]
fn a_feature_declares_its_wake_and_it_is_due_only_when_the_clock_reaches_it() {
    let mut states = FeatureStates::default();
    assert_eq!(next_wake(&states), None, "no feature asked");
    assert!(wakes_due(&states, Now::at_secs(1_000_000)).is_empty());
    *states.alpha = 2_000;
    *states.beta = 1_500;
    assert_eq!(
        next_wake(&states),
        Some(Now::at_secs(1_500)),
        "the nearer of the two"
    );
    assert!(wakes_due(&states, Now::at_secs(1_499)).is_empty());
    assert_eq!(
        wakes_due(&states, Now::at_secs(1_500)),
        [FeatureIntent::from(beta::Intent(0))]
    );
    assert_eq!(
        wakes_due(&states, Now::at_secs(2_500)),
        [
            FeatureIntent::from(alpha::Intent(0)),
            FeatureIntent::from(beta::Intent(0))
        ],
        "both, in list order"
    );
}

#[test]
fn a_thread_part_declares_its_wake_the_same_way() {
    let mut parts = FeatureThreadParts::default();
    assert_eq!(next_thread_wake(&parts), None);
    parts.beta = 3_000;
    assert_eq!(next_thread_wake(&parts), Some(Now::at_secs(3_000)));
    assert!(thread_wakes_due(&parts, Now::at_secs(2_999)).is_empty());
    assert_eq!(
        thread_wakes_due(&parts, Now::at_secs(3_000)),
        [FeatureThreadIntent::from(beta::ThreadIntent(0))]
    );
}

#[test]
fn an_intent_reaches_its_feature_and_only_its_feature() {
    let mut states = FeatureStates::default();
    app_step(|cx| apply_intent(&mut states, FeatureIntent::from(beta::Intent(2)), cx));
    app_step(|cx| apply_intent(&mut states, FeatureIntent::Alpha(alpha::Intent(5)), cx));
    app_step(|cx| apply_intent(&mut states, FeatureIntent::from(beta::Intent(3)), cx));
    assert_eq!((*states.alpha, *states.beta), (5, 5));
}

/// A slash command goes to the feature whose `RUNS` holds it, with the text after its name and
/// the thread it was typed in; one nobody runs is not taken (the app says it is not available).
#[test]
fn a_slash_command_reaches_its_owner_with_its_arguments() {
    let mut states = FeatureStates::default();
    let slash = |command, args: &str, thread: Option<&str>| Slash {
        command,
        args: args.to_owned(),
        source: thread.map(ThreadId::new),
    };
    let mut taken = Vec::new();
    app_step(|cx| {
        for line in [
            slash(SlashCommand::Rename, "abc", None),
            slash(SlashCommand::Copy, "xy", Some("t1")),
            slash(SlashCommand::Goal, "", None),
            slash(SlashCommand::Logout, "", None),
        ] {
            taken.push(apply_slash(&mut states, line, cx));
        }
    });
    assert_eq!(taken, [true, true, true, false]);
    assert_eq!((*states.alpha, *states.beta), (3, 2 + 100));
    assert!(runs_of(SlashCommand::Goal) && !runs_of(SlashCommand::Logout));
}

#[test]
fn an_answer_goes_to_its_feature_and_one_of_another_shape_is_a_mismatch() {
    let mut states = FeatureStates::default();
    app_step(|cx| {
        answered(
            &mut states,
            FeatureCtx::from(alpha::Ctx("a")),
            Ok(FeatureReply::from(alpha::Reply(5))),
            cx,
        );
        answered(
            &mut states,
            FeatureCtx::from(alpha::Ctx("a")),
            Ok(FeatureReply::from(beta::Reply(5))),
            cx,
        );
        answered(
            &mut states,
            FeatureCtx::from(beta::Ctx("b")),
            Err(RpcError::Remote("down".into())),
            cx,
        );
    });
    assert_eq!(
        (*states.alpha, *states.beta),
        (1005, 1),
        "right reply applied; wrong shape is a Mismatch to the ctx's feature; error reaches beta"
    );
}

fn export() -> Ask {
    Ask::Export {
        thread: ThreadId::new("t1"),
        path: "out.md".to_owned(),
        cwd: "/work".to_owned(),
        markdown: String::new(),
    }
}

/// The host's answer goes to the feature whose ctx the ask carried, as the feature's own
/// `Ctx`, and to no other.
#[test]
fn a_host_answer_reaches_the_feature_that_asked() {
    let mut states = FeatureStates::default();
    app_step(|cx| {
        effect_done(
            &mut states,
            FeatureCtx::from(beta::Ctx("bb")),
            Done::Export(Ok("abc".to_owned())),
            cx,
        );
        effect_done(
            &mut states,
            FeatureCtx::from(alpha::Ctx("a")),
            Done::Export(Err("none".to_owned())),
            cx,
        );
        effect_done(
            &mut states,
            FeatureCtx::from(beta::Ctx("b")),
            Done::ClipboardText(Err("no".to_owned())),
            cx,
        );
    });
    assert_eq!(
        (*states.alpha, *states.beta),
        (2, (2 + 3) + (1 + 1000)),
        "each answer is its asker's, with the asker's own ctx"
    );
}

/// A feature has no handle on the host: it says what it wants, with the ctx that comes back
/// with the answer, and the app carries it out after the step (`app/features.rs`). The outbox
/// is typed with the real list's roots, so the ask is a `goal` one.
#[test]
fn a_feature_asks_the_host_through_the_outbox() {
    let ctx = crate::features::goal::Ctx {
        thread: ThreadId::new("t1"),
        flow: crate::features::goal::Flow::Show,
    };
    let (threads, screens, tui) = (
        ThreadSet::default(),
        ScreenStates::default(),
        TuiSettings::default(),
    );
    let mut cx = AppCx::new(AppView::new(None, &threads, &screens, &tui, &[], None));
    cx.effect(Ask::ClipboardText, ctx.clone());
    cx.effect(export(), ctx.clone());
    assert_eq!(
        cx.into_out(),
        [
            Out::Effect {
                ask: Ask::ClipboardText,
                ctx: super::FeatureCtx::Goal(ctx.clone()),
            },
            Out::Effect {
                ask: export(),
                ctx: super::FeatureCtx::Goal(ctx),
            },
        ],
        "in the order they were said"
    );
}

/// A script's `note` step: the feature reads its own note from the JSON; a feature with none to
/// seed, and a name no feature has, are told apart.
#[test]
fn a_note_is_seeded_by_the_feature_that_says_it() {
    use crate::screens::SeedError;
    use serde_json::json;
    assert_eq!(
        seed_note("alpha", json!("error")),
        Ok(FeatureNote::from(alpha::Note("error")))
    );
    assert!(matches!(
        seed_note("alpha", json!(3)),
        Err(SeedError::Bad(_))
    ));
    assert_eq!(seed_note("beta", json!("error")), Err(SeedError::NoSeeds));
    assert_eq!(
        seed_note("gamma", json!("error")),
        Err(SeedError::UnknownDir)
    );
}

#[test]
fn a_notes_tone_is_the_features_own() {
    assert_eq!(
        note_tone(&FeatureNote::from(alpha::Note("error"))),
        Tone::Error
    );
    assert_eq!(
        note_tone(&FeatureNote::from(alpha::Note("fine"))),
        Tone::Info
    );
}

/// A fact the host sends unasked goes to the feature it is for, as that feature's own `Fact`
/// (the host builds it with `From`), and to no other: the probes' door for the config feed
/// and the presets.
#[test]
fn a_host_fact_reaches_its_feature_and_only_its_feature() {
    let mut states = FeatureStates::default();
    app_step(|cx| on_fact(&mut states, FeatureFact::from(alpha::Fact(4)), cx));
    app_step(|cx| on_fact(&mut states, FeatureFact::Alpha(alpha::Fact(3)), cx));
    assert_eq!((*states.alpha, *states.beta), (7, 0));
}

#[test]
fn app_hooks_run_in_list_order() {
    let mut states = FeatureStates::default();
    let thread = ThreadId::new("t9");
    logged();
    app_step(|cx| on_resumed(&mut states, &thread, cx));
    assert_eq!(
        logged(),
        [
            ("alpha", Hook::Resumed(thread.clone())),
            ("beta", Hook::Resumed(thread)),
        ]
    );
}

#[test]
fn a_feature_follows_the_stream_through_its_own_event() {
    let mut states = FeatureStates::default();
    let closed = ServerNotification::ThreadClosed(ThreadClosedNotification {
        thread_id: "t-closed".to_owned(),
    });
    let events = project(&closed);
    assert_eq!(
        events,
        [FeatureEvent::from(alpha::Event(8))],
        "alpha projects the notification, beta follows nothing"
    );
    app_step(|cx| {
        for event in events {
            on_event(&mut states, event, cx);
        }
    });
    assert_eq!((*states.alpha, *states.beta), (8, 0));
    // a server request is observed the same way, and nothing answers it
    let request: ServerRequest = serde_json::from_value(serde_json::json!({
        "method": "item/commandExecution/requestApproval",
        "id": 1,
        "params": {
            "threadId": "t", "turnId": "u", "itemId": "i", "startedAtMs": 0,
            "approvalId": "i", "environmentId": null, "command": "echo hi", "cwd": "/tmp"
        }
    }))
    .expect("a request");
    let events = project_request(&request);
    assert_eq!(events, [FeatureEvent::from(alpha::Event(100))]);
    app_step(|cx| {
        for event in events {
            on_event(&mut states, event, cx);
        }
    });
    assert_eq!(*states.alpha, 108);
}

#[test]
fn the_shown_connection_and_opened_hooks_reach_their_features() {
    let mut states = FeatureStates::default();
    let thread = ThreadId::new("t9");
    logged();
    app_step(|cx| {
        on_shown(&mut states, &thread, cx);
        on_connection(&mut states, Connection::Lost, cx);
        thread_opened(
            &mut states,
            FeatureCtx::from(beta::Ctx("b")),
            Err(Refusal::Busy),
            cx,
        );
    });
    assert_eq!(
        logged(),
        [
            ("alpha", Hook::Shown(thread.clone())),
            ("beta", Hook::Shown(thread)),
            ("alpha", Hook::Connection(Connection::Lost)),
            ("beta", Hook::Connection(Connection::Lost)),
            ("beta", Hook::Opened("b", Err(Refusal::Busy))),
        ],
        "the app-wide hooks run in list order, the answer reaches the feature that asked"
    );
}

/// The answer of a structured turn goes to the feature whose ctx the request carried, as its own
/// `Ctx`, with the text or the reason there is none.
#[test]
fn a_structured_answer_reaches_the_feature_that_asked() {
    let mut states = FeatureStates::default();
    logged();
    app_step(|cx| {
        structured_done(
            &mut states,
            FeatureCtx::from(alpha::Ctx("a")),
            Ok("{}".to_owned()),
            cx,
        );
        structured_done(
            &mut states,
            FeatureCtx::from(beta::Ctx("b")),
            Err(StructuredFailure::TimedOut),
            cx,
        );
    });
    assert_eq!(
        logged(),
        [
            ("alpha", Hook::Structured("a", Ok("{}".to_owned()))),
            (
                "beta",
                Hook::Structured("b", Err(StructuredFailure::TimedOut))
            ),
        ]
    );
}

/// The feature that asked is asked once more before the turn starts: a feature says no by its
/// own ctx, and one that says nothing wants it.
#[test]
fn a_structured_turn_is_asked_whether_it_is_still_wanted() {
    let states = FeatureStates::default();
    let (threads, screens, tui) = (
        ThreadSet::default(),
        ScreenStates::default(),
        TuiSettings::default(),
    );
    let view = AppView::new(None, &threads, &screens, &tui, &[], None);
    let wanted = |ctx: FeatureCtx| structured_wanted(&states, &ctx, &view);
    assert!(wanted(FeatureCtx::from(alpha::Ctx("fresh"))));
    assert!(!wanted(FeatureCtx::from(alpha::Ctx("stale"))));
    assert!(!wanted(FeatureCtx::from(beta::Ctx("stale"))));
}

/// What a read of the account's limits said reaches every feature, in list order, with the
/// backend that was read.
#[test]
fn a_rate_limits_read_reaches_every_feature_in_list_order() {
    let mut states = FeatureStates::default();
    logged();
    let backend = BackendId::default();
    app_step(|cx| on_rate_limits_read(&mut states, backend, &RateRead::default(), cx));
    assert_eq!(
        logged(),
        [
            ("alpha", Hook::Rates(backend)),
            ("beta", Hook::Rates(backend))
        ]
    );
}

/// A turn error is reworded by the first feature in list order that has words for it.
#[test]
fn a_turn_error_is_reworded_by_the_first_feature_that_has_words() {
    let states = FeatureStates::default();
    let (threads, screens, tui) = (
        ThreadSet::default(),
        ScreenStates::default(),
        TuiSettings::default(),
    );
    let view = AppView::new(None, &threads, &screens, &tui, &[], None);
    let thread = ThreadId::new("t1");
    let words = |message: &str| fault_words(&states, &thread, message, &view);
    assert_eq!(words("beta").as_deref(), Some("beta says"));
    assert_eq!(words("alpha").as_deref(), Some("alpha says"));
    assert_eq!(words("the server's own"), None);
}

/// A feature says its own thread is quiet about interruptions (a side conversation): the door
/// is one lifecycle step the app carries out (`ThreadEv::QuietInterrupts`).
#[test]
fn a_feature_makes_its_thread_quiet_about_interruptions() {
    let (threads, screens, tui) = (
        ThreadSet::default(),
        ScreenStates::default(),
        TuiSettings::default(),
    );
    let mut cx = AppCx::new(AppView::new(None, &threads, &screens, &tui, &[], None));
    let thread = ThreadId::new("side-1");
    cx.quiet_interrupts(&thread, true);
    assert_eq!(
        cx.into_out(),
        [Out::Lifecycle(super::Lifecycle::QuietInterrupts {
            thread,
            quiet: true
        })]
    );
}

/// The backend banner of the thread in use is a fact a feature reads, shown or not (the limit
/// prompts wait while one shows); where the view has none, there is none.
#[test]
fn the_backend_banner_of_the_thread_in_use_is_a_fact_a_feature_reads() {
    use crate::ids::Ticket;
    use crate::screens::chat::{Banner, BannerOccurrence};

    let banner: Banner = serde_json::from_value(serde_json::json!({
        "banner_type": "selected_model_limit", "model_slug": "test-model-a",
        "title": "Selected model usage exhausted", "description": "Switch to another model.",
        "ctas": [], "presentation": "inline",
    }))
    .expect("a banner");
    let occurrence = BannerOccurrence::new(banner, Ticket::new(3));
    let (threads, screens, tui) = (
        ThreadSet::default(),
        ScreenStates::default(),
        TuiSettings::default(),
    );
    let view = |banner| AppView::new(None, &threads, &screens, &tui, &[], None).with_banner(banner);
    let with = view(Some(&occurrence));
    let seen = with
        .facts()
        .backend_banner()
        .expect("the banner of the view");
    assert_eq!(seen.ticket, Ticket::new(3));
    assert!(seen.is_visible(Some("test-model-a")));
    assert!(!seen.is_visible(Some("another-model")));
    assert!(view(None).facts().backend_banner().is_none());
}

/// A user message of an unnamed thread reaches every feature, in list order.
#[test]
fn a_user_message_reaches_every_feature_in_list_order() {
    let mut states = FeatureStates::default();
    logged();
    let thread = ThreadId::new("t1");
    app_step(|cx| on_user_message(&mut states, &thread, "fix the build", cx));
    assert_eq!(
        logged(),
        [
            (
                "alpha",
                Hook::Message(thread.clone(), "fix the build".into())
            ),
            ("beta", Hook::Message(thread, "fix the build".into())),
        ]
    );
}

#[test]
fn thread_hooks_run_in_list_order_with_the_thread_read_only() {
    let mut parts = FeatureThreadParts::default();
    logged();
    with_thread(|cx| {
        on_turn_end(&mut parts, TurnEnd::Interrupted, cx);
        on_interrupted(&mut parts, cx);
    });
    assert_eq!(
        logged(),
        [
            ("alpha", Hook::TurnEnd(TurnEnd::Interrupted, false)),
            ("beta", Hook::TurnEnd(TurnEnd::Interrupted, false)),
            ("alpha", Hook::Interrupted),
            ("beta", Hook::Interrupted),
        ]
    );
    assert_eq!((parts.alpha, parts.beta), (1, 1));
}

/// ADR-012: "a feature after it sees the thread already busy". The thread's own `send_next`
/// says a start before the hooks run, and its ticket is the model's only after the whole
/// `apply`: the view of the step still has to know.
#[test]
fn every_hook_sees_a_turn_start_that_was_said_before_it() {
    let mut parts = FeatureThreadParts::default();
    logged();
    with_thread(|cx| on_turn_end(&mut parts, TurnEnd::Completed, cx));
    assert_eq!(
        logged(),
        [
            ("alpha", Hook::TurnEnd(TurnEnd::Completed, false)),
            ("beta", Hook::TurnEnd(TurnEnd::Completed, false)),
        ],
        "an idle thread is not busy"
    );
    with_thread_after(
        |fx| {
            fx.start_turn(UserMessage::text("next"))
                .expect("the first start");
        },
        |cx| on_turn_end(&mut parts, TurnEnd::Completed, cx),
    );
    assert_eq!(
        logged(),
        [
            ("alpha", Hook::TurnEnd(TurnEnd::Completed, true)),
            ("beta", Hook::TurnEnd(TurnEnd::Completed, true)),
        ]
    );
}

/// ADR-003: the effects of a feature's step are the writer's; the news is everyone's. A thread
/// this process does not write (another process writes it, the connection was lost, the server
/// refused it for its provider) still tells its features what happened, so a part that waits
/// for a reply or a turn's end does not stay stuck, but it opens no RPC there and a feature's
/// ask (`thread_intent`) is not delivered.
#[test]
fn a_thread_this_process_does_not_write_hears_the_news_and_has_no_effects() {
    let id = ThreadId::new("t1");
    let refused = ThreadEv::Screen(crate::screens::ScreenThreadEv::Chat(
        crate::screens::chat::ThreadEv::ProviderError(
            codex_app_server_protocol::ModelProviderError::ModelProviderNotConfigured {
                model_provider: "old-provider".into(),
            },
        ),
    ));
    let (mut out, mut mint) = (AppFx::default(), TicketMint::default());
    for (what, origin, then, writes) in [
        ("a thread this process writes", Origin::Resumed, None, true),
        (
            "another process writes it",
            Origin::ForeignWriter,
            None,
            false,
        ),
        (
            "the connection was lost",
            Origin::Resumed,
            Some(ThreadEv::ConnectionLost),
            false,
        ),
        (
            "the server refused it for its provider",
            Origin::Resumed,
            Some(refused),
            false,
        ),
    ] {
        let mut threads = ThreadSet::default();
        let model = threads.open(&id, origin);
        let mut fx = ThreadFx::new(&id, &mut out, &mut mint);
        if let Some(ev) = then {
            crate::threads::apply(model, ev, &mut fx);
        }
        let mut parts = FeatureThreadParts::default();
        logged();
        let mut cx = ThreadCx::new(model.view(), &mut fx);
        assert_eq!(cx.view.has_writer(), writes, "{what}");
        // the effect door is the writer's (typed with the real list's roots: a goal ask)
        let ticket = cx.request(
            crate::features::goal::Ctx {
                thread: id.clone(),
                flow: crate::features::goal::Flow::Show,
            },
            crate::features::goal::Ask::Get,
        );
        assert_eq!(ticket.is_some(), writes, "{what}: request");
        // the news is everyone's: a reply, a turn's end, what held the input back
        apply_thread_reply(
            &mut parts,
            FeatureCtx::from(alpha::Ctx("x")),
            Ok(FeatureReply::from(alpha::Reply(4))),
            &mut cx,
        );
        on_turn_end(&mut parts, TurnEnd::Completed, &mut cx);
        apply_thread_intent(&mut parts, FeatureThreadIntent::Settled, &mut cx);
        assert_eq!(
            logged(),
            [
                ("alpha", Hook::TurnEnd(TurnEnd::Completed, false)),
                ("beta", Hook::TurnEnd(TurnEnd::Completed, false)),
                ("alpha", Hook::Settled),
                ("beta", Hook::Settled),
            ],
            "{what}: the hooks run"
        );
        assert_eq!(
            (parts.alpha, parts.beta),
            (4 + 1, 1),
            "{what}: the reply reached the part, and both turn-end hooks counted"
        );
        // an ask is the writer's
        apply_thread_intent(
            &mut parts,
            FeatureThreadIntent::from(beta::ThreadIntent(10)),
            &mut cx,
        );
        assert_eq!(
            parts.beta,
            if writes { 11 } else { 1 },
            "{what}: a feature's ask"
        );
    }
}

/// One turn start per step (ADR-012, `ThreadFx::start_turn`): A starts, B's start in the same
/// step is refused and its message given back, and only one `StartTurn` goes out.
#[test]
fn a_second_turn_start_in_one_step_is_refused() {
    let id = ThreadId::new("t1");
    let (mut out, mut mint) = (AppFx::default(), TicketMint::default());
    let mut fx = ThreadFx::new(&id, &mut out, &mut mint);
    assert!(!fx.start_said());
    let first = fx.start_turn(UserMessage::text("A"));
    assert!(first.is_ok(), "the first start goes out");
    assert!(fx.start_said());
    assert_eq!(
        fx.start_turn(UserMessage::text("B")),
        Err(UserMessage::text("B")),
        "the second is refused, and the caller keeps its message"
    );
    let starts: Vec<_> = out
        .commands()
        .iter()
        .filter_map(|command| match command {
            crate::app::Command::StartTurn { message, .. } => Some(message.text.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(starts, ["A"]);
}

#[test]
fn a_settled_thread_runs_every_features_hook_in_list_order() {
    let mut parts = FeatureThreadParts::default();
    logged();
    with_thread(|cx| apply_thread_intent(&mut parts, FeatureThreadIntent::Settled, cx));
    assert_eq!(
        logged(),
        [("alpha", Hook::Settled), ("beta", Hook::Settled)]
    );
}

#[test]
fn a_thread_intent_and_a_thread_answer_reach_their_features_part() {
    let mut parts = FeatureThreadParts::default();
    with_thread(|cx| {
        apply_thread_intent(
            &mut parts,
            FeatureThreadIntent::from(beta::ThreadIntent(3)),
            cx,
        );
        apply_thread_reply(
            &mut parts,
            FeatureCtx::from(alpha::Ctx("x")),
            Ok(FeatureReply::from(alpha::Reply(4))),
            cx,
        );
        apply_thread_reply(
            &mut parts,
            FeatureCtx::from(alpha::Ctx("x")),
            Ok(FeatureReply::from(beta::Reply(4))),
            cx,
        );
    });
    assert_eq!((parts.alpha, parts.beta), (1004, 3));
}

#[test]
fn a_features_rpc_is_wired_and_decoded_by_its_own_feature() {
    let command = FeatureCommand::from(beta::Command("rpc".to_owned()));
    let ClientRequest::ThreadGoalGet { params, .. } =
        wire(&command, &ThreadId::new("t1"), RequestId::Integer(3))
    else {
        panic!("the probe wires a goal get");
    };
    assert_eq!(params.thread_id, "t1:rpc");
    assert_eq!(
        decode(&command, serde_json::json!(2)),
        Ok(FeatureReply::Beta(beta::Reply(5)))
    );
    assert_eq!(
        decode(&command, serde_json::json!("x")),
        Err(RpcError::Malformed("rpc".into()))
    );
}
