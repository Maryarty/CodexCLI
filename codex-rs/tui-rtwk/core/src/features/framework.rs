//! The framework with a list of its own: two probe features registered in `probe_list`, a
//! second invocation of the generating macro, so every generated enum and function is
//! exercised while the real list is still empty. What is checked is what ADR-012 promises:
//! one registration line gives the state, the root enums and their dispatch; the hooks run in
//! list order; an answer of another feature's shape is a mismatch, not a panic; a feature
//! without a component says `Never<Self>` and two of them do not overlap.
//!
//! The outbox (`AppCx::ask`, `open`, `note`...) is typed with the real list's roots, which
//! are empty here: it is exercised by the first feature that has a payload.

use std::cell::RefCell;

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::ThreadGoalGetParams;

use super::{AppCx, AppView, Choice, Feature, Never, ThreadCx, Tone, TurnEnd};
use crate::app::AppFx;
use crate::app::facts::TuiSettings;
use crate::ids::{ThreadId, TicketMint};
use crate::rpc::RpcError;
use crate::screens::ScreenStates;
use crate::threads::{Origin, ThreadFx, ThreadSet};

/// What a hook saw.
#[derive(Debug, PartialEq, Eq)]
enum Hook {
    Resumed(ThreadId),
    TurnEnd(TurnEnd, bool),
    Interrupted,
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
/// own; beta has none of the menu, pick, prompt and note (`Never<Self>`), which is the case
/// the root `From` impls must survive twice.
macro_rules! probe {
    ($module:ident, $marker:ident, $name:literal, $($rest:tt)*) => {
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
                type State = u32;
                type ThreadPart = u32;
                type Intent = Intent;
                type ThreadIntent = ThreadIntent;
                type Command = Command;
                type Reply = Reply;
                type Ctx = Ctx;
                type Menu = Menu;
                type Pick = Pick;
                type Prompt = Prompt;
                type Note = Note;

                fn intent(state: &mut u32, intent: Intent, _cx: &mut AppCx<'_>) {
                    *state += intent.0;
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

                fn on_resumed(_state: &mut u32, thread: &ThreadId, _cx: &mut AppCx<'_>) {
                    log($name, Hook::Resumed(thread.clone()));
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
        pub struct Menu(pub &'static str);
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub struct Pick(pub &'static str);
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub struct Prompt(pub &'static str);
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub struct Note(pub &'static str);
    };
    // and of one that has none
    (@components $marker:ident, bare) => {
        pub type Menu = Never<$marker>;
        pub type Pick = Never<$marker>;
        pub type Prompt = Never<$marker>;
        pub type Note = Never<$marker>;
    };
    (@steps full) => {
        fn choose(menu: &Menu, row: &str) -> Choice<Menu, Pick> {
            match row {
                "pick" => Choice::Pick(Pick(menu.0)),
                "next" => Choice::Replace(Menu("second")),
                "close" => Choice::Dismiss,
                _ => Choice::Stay,
            }
        }

        fn picked(state: &mut u32, pick: Pick, _cx: &mut AppCx<'_>) {
            *state += pick.0.len() as u32;
        }

        fn prompt_submitted(state: &mut u32, prompt: Prompt, text: String, _cx: &mut AppCx<'_>) {
            *state += prompt.0.len() as u32 + text.len() as u32;
        }

        fn note_tone(note: &Note) -> Tone {
            if note.0 == "error" { Tone::Error } else { Tone::Info }
        }
    };
    (@steps bare) => {};
}

mod probe_list {
    use super::*;

    probe!(alpha, Alpha, "alpha", full);
    probe!(beta, Beta, "beta", bare);

    feature_dirs! {
        alpha => Alpha;
        beta => Beta;
    }
}

use probe_list::{
    FeatureCommand, FeatureCtx, FeatureIntent, FeatureMenu, FeatureNote, FeaturePick,
    FeaturePrompt, FeatureReply, FeatureStates, FeatureThreadIntent, FeatureThreadParts, alpha,
    answered, apply_intent, apply_thread_intent, apply_thread_reply, beta, choose, decode, names,
    note_tone, on_interrupted, on_resumed, on_turn_end, picked, prompt_submitted, wire,
};

/// Runs `step` with an `AppCx` over empty facts; what it asked for is nothing here.
fn app_step(step: impl FnOnce(&mut AppCx<'_>)) {
    let (threads, screens, tui) = (
        ThreadSet::default(),
        ScreenStates::default(),
        TuiSettings::default(),
    );
    let mut cx = AppCx::new(AppView::new(None, &threads, &screens, &tui, &[]));
    step(&mut cx);
    assert!(cx.into_out().is_empty(), "a probe says nothing");
}

/// Runs `step` over the model of a thread this process holds, with its effects.
fn with_thread(step: impl FnOnce(&mut ThreadCx<'_, '_>)) {
    let id = ThreadId::new("t1");
    let mut threads = ThreadSet::default();
    let model = threads.open(&id, Origin::Resumed);
    let (mut out, mut mint) = (AppFx::default(), TicketMint::default());
    let mut fx = ThreadFx::new(&id, &mut out, &mut mint);
    step(&mut ThreadCx::new(model.view(), &mut fx));
}

#[test]
fn the_real_list_starts_empty_and_a_list_names_its_features_in_order() {
    assert_eq!(super::names(), Vec::<&str>::new());
    assert_eq!(names(), ["alpha", "beta"]);
}

#[test]
fn an_intent_reaches_its_feature_and_only_its_feature() {
    let mut states = FeatureStates::default();
    app_step(|cx| apply_intent(&mut states, FeatureIntent::from(beta::Intent(2)), cx));
    app_step(|cx| apply_intent(&mut states, FeatureIntent::Alpha(alpha::Intent(5)), cx));
    app_step(|cx| apply_intent(&mut states, FeatureIntent::from(beta::Intent(3)), cx));
    assert_eq!((*states.alpha, *states.beta), (5, 5));
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

#[test]
fn a_menu_row_settles_in_the_features_own_terms_and_a_pick_and_prompt_reach_it() {
    let menu = FeatureMenu::from(alpha::Menu("first"));
    assert_eq!(choose(&menu, "other"), super::Choice::Stay);
    assert_eq!(choose(&menu, "close"), super::Choice::Dismiss);
    assert_eq!(
        choose(&menu, "next"),
        super::Choice::Replace(FeatureMenu::Alpha(alpha::Menu("second")))
    );
    let Choice::Pick(pick) = choose(&menu, "pick") else {
        panic!("the pick row picks");
    };
    assert_eq!(pick, FeaturePick::Alpha(alpha::Pick("first")));
    let mut states = FeatureStates::default();
    app_step(|cx| picked(&mut states, pick, cx));
    app_step(|cx| {
        prompt_submitted(
            &mut states,
            FeaturePrompt::from(alpha::Prompt("ab")),
            "xyz".to_owned(),
            cx,
        )
    });
    assert_eq!((*states.alpha, *states.beta), (5 + 2 + 3, 0));
    assert_eq!(
        note_tone(&FeatureNote::from(alpha::Note("error"))),
        Tone::Error
    );
    assert_eq!(
        note_tone(&FeatureNote::from(alpha::Note("fine"))),
        Tone::Info
    );
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
