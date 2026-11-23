#![allow(clippy::expect_used, clippy::unwrap_used)]

//! `/goal` (oracle `slash_dispatch.rs`, `app/thread_goal_actions.rs`, `chatwidget/goal_menu.rs`):
//! the bare command reads the goal and says it, the control words ask the server, an objective
//! over an unfinished goal asks before it replaces it, the edit prompt (a wish of the feature, answered by a verb)
//! keeps the status and the budget, an ask that failed is the oracle's error. The words are the view's: these tests read
//! data.

use codex_app_server_protocol::{
    ClientRequest, RequestId, ThreadGoalClearParams, ThreadGoalGetParams, ThreadGoalMutationOrigin,
    ThreadGoalSetParams, ThreadGoalStatus,
};
use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Intent, apply_app};
use codex_tui_core::commands::{SessionIntent, SlashCommand};
use codex_tui_core::features::goal::{self, Ask};
use codex_tui_core::features::{FeatureCommand, FeatureIntent, FeatureNote, FeatureWish};
use codex_tui_core::fixtures::builders::{turn_completed, turn_started};
use codex_tui_core::ids::{ThreadId, Ticket};
use codex_tui_core::rpc::Reply;
use codex_tui_core::screens::{ScreenIntent, ScreenThreadIntent, chat};
use codex_tui_core::threads::queue::Dispatched;
use codex_tui_core::threads::{
    Goal, GoalStatus, Item, Note, NoteKey, Queued, Ran, Tone, UserMessage,
};
use goal::{Action as GoalAction, Failure as GoalFailure, Note as GoalNote, UsageHint as GoalHint};
use pretty_assertions::assert_eq;

const T: &str = "thread-1";

fn run(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    app.assert_consistent();
    fx
}

fn ready() -> CodexApp {
    let mut app = CodexApp::new();
    let fx = run(&mut app, AppEv::Connected);
    let ticket: Ticket = fx
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

fn goal(objective: &str, status: GoalStatus) -> Goal {
    Goal {
        objective: objective.into(),
        status,
        token_budget: Some(5_000),
        tokens_used: 120,
        time_used_seconds: 30,
        created_at: 10,
        updated_at: 20,
    }
}

/// `/goal` or `/goal <args>`, typed in the thread (`None`: before the session started).
fn command(app: &mut CodexApp, thread: bool, args: Option<&str>) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Chat(chat::Intent::Session {
            thread: thread.then(|| ThreadId::new(T)),
            command: SlashCommand::Goal,
            intent: SessionIntent::Goal {
                args: args.map(str::to_owned),
            },
        }))),
    )
}

/// The goal asks in `fx`, with their tickets.
fn asked(fx: &AppFx) -> Vec<(Ticket, String, Ask)> {
    fx.commands()
        .iter()
        .filter_map(|c| match c {
            Command::Feature {
                ticket,
                thread,
                command: FeatureCommand::Goal(ask),
                ..
            } => Some((*ticket, thread.to_string(), ask.clone())),
            _ => None,
        })
        .collect()
}

fn only(fx: &AppFx) -> (Ticket, Ask) {
    let asks = asked(fx);
    let [(ticket, thread, ask)] = asks.as_slice() else {
        panic!("expected one goal ask, got {:?}", fx.commands());
    };
    assert_eq!(thread, T);
    (*ticket, ask.clone())
}

fn answer(app: &mut CodexApp, ticket: Ticket, reply: goal::Reply) -> AppFx {
    run(
        app,
        AppEv::Reply(
            ticket,
            Reply::feature(codex_tui_core::features::FeatureReply::Goal(reply)),
        ),
    )
}

fn fail(app: &mut CodexApp, ticket: Ticket, message: &str) -> AppFx {
    run(app, AppEv::Reply(ticket, Reply::failed(message)))
}

/// The goal notes of the thread's transcript, with their tone.
fn notes(app: &CodexApp) -> Vec<(Tone, GoalNote)> {
    app.active_thread()
        .transcript()
        .iter()
        .filter_map(|(_, item)| match item {
            Item::Note(Note {
                tone,
                key: NoteKey::Feature(FeatureNote::Goal(note)),
            }) => Some((*tone, note.clone())),
            _ => None,
        })
        .collect()
}

/// What the goal wishes the surface to show, oldest first.
fn wishes(app: &CodexApp) -> Vec<goal::Wish> {
    app.wishes()
        .pending()
        .filter_map(|wish| match &wish.wish {
            FeatureWish::Goal(wish) => Some(wish.clone()),
            _ => None,
        })
        .collect()
}

/// What the UI half says when the user answers a question or the edit prompt.
fn verb(app: &mut CodexApp, intent: goal::Intent) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Feature(FeatureIntent::Goal(intent))),
    )
}

#[test]
fn the_bare_command_reads_the_goal_and_says_it() {
    let mut app = ready();
    let fx = command(&mut app, true, None);
    let (ticket, ask) = only(&fx);
    assert_eq!(ask, Ask::Get);
    let active = goal("ship it", GoalStatus::Active);
    answer(&mut app, ticket, goal::Reply::Goal(Some(active.clone())));
    assert_eq!(notes(&app), [(Tone::Info, GoalNote::Summary(active))]);
}

#[test]
fn the_bare_command_over_no_goal_is_the_usage_and_its_hint() {
    let mut app = ready();
    let (ticket, _) = only(&command(&mut app, true, None));
    answer(&mut app, ticket, goal::Reply::Goal(None));
    assert_eq!(
        notes(&app),
        [(Tone::Info, GoalNote::Usage(GoalHint::NoGoal))]
    );
}

#[test]
fn before_the_session_starts_the_command_only_says_how_to_use_it() {
    let mut app = CodexApp::new();
    let fx = command(&mut app, false, None);
    assert_eq!(asked(&fx), []);
    for (args, hint) in [
        (None, GoalHint::Example),
        (Some("clear"), GoalHint::StartToChange),
        (Some("Pause"), GoalHint::StartToChange),
        (Some("resume"), GoalHint::StartToChange),
        (Some("improve coverage"), GoalHint::StartToSet),
    ] {
        let mut app = CodexApp::new();
        let fx = command(&mut app, false, args);
        assert_eq!(asked(&fx), [], "{args:?} asks nothing");
        let said: Vec<_> = app
            .screens()
            .chat
            .notes
            .iter()
            .filter_map(|note| match &note.key {
                NoteKey::Feature(FeatureNote::Goal(note)) => Some(note.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(said, [GoalNote::Usage(hint)], "{args:?}");
    }
}

#[test]
fn clear_pause_and_resume_ask_the_server_and_say_the_answer() {
    let mut app = ready();

    let (ticket, ask) = only(&command(&mut app, true, Some("clear")));
    assert_eq!(ask, Ask::Clear);
    answer(&mut app, ticket, goal::Reply::Cleared(true));
    let (ticket, _) = only(&command(&mut app, true, Some("CLEAR")));
    answer(&mut app, ticket, goal::Reply::Cleared(false));

    let (ticket, ask) = only(&command(&mut app, true, Some("pause")));
    assert_eq!(
        ask,
        Ask::Set {
            objective: None,
            status: Some(GoalStatus::Paused),
            token_budget: None
        }
    );
    let paused = goal("ship it", GoalStatus::Paused);
    answer(&mut app, ticket, goal::Reply::Goal(Some(paused.clone())));

    let (_, ask) = only(&command(&mut app, true, Some("resume")));
    assert_eq!(
        ask,
        Ask::Set {
            objective: None,
            status: Some(GoalStatus::Active),
            token_budget: None
        }
    );

    assert_eq!(
        notes(&app),
        [
            (Tone::Info, GoalNote::Cleared),
            (Tone::Info, GoalNote::NothingToClear),
            (Tone::Info, GoalNote::Status(paused)),
        ]
    );
}

#[test]
fn an_objective_over_no_goal_sets_it_active() {
    let mut app = ready();
    let (ticket, ask) = only(&command(&mut app, true, Some("  improve coverage ")));
    assert_eq!(ask, Ask::Get, "the goal is read first");
    let (ticket, ask) = only(&answer(&mut app, ticket, goal::Reply::Goal(None)));
    assert_eq!(
        ask,
        Ask::Set {
            objective: Some("improve coverage".into()),
            status: Some(GoalStatus::Active),
            token_budget: None
        }
    );
    let set = goal("improve coverage", GoalStatus::Active);
    answer(&mut app, ticket, goal::Reply::Goal(Some(set.clone())));
    assert_eq!(notes(&app), [(Tone::Info, GoalNote::Status(set))]);
}

#[test]
fn an_objective_over_an_unfinished_goal_asks_before_it_replaces_it() {
    let mut app = ready();
    let (ticket, _) = only(&command(&mut app, true, Some("new objective")));
    let fx = answer(
        &mut app,
        ticket,
        goal::Reply::Goal(Some(goal("old", GoalStatus::Paused))),
    );
    assert_eq!(asked(&fx), [], "nothing is replaced yet");
    let question = goal::Wish::Replace {
        thread: ThreadId::new(T),
        objective: "new objective".into(),
    };
    assert_eq!(wishes(&app), [question]);

    // Cancel: nothing is said to the feature, the goal stays (the wish is shown and gone).
    let seq = app.wishes().pending().next().expect("a wish").seq;
    run(&mut app, AppEv::Intent(Intent::WishShown(seq)));
    assert_eq!(wishes(&app), []);

    // Replace: the old goal is cleared, then the new one set.
    let (ticket, _) = only(&command(&mut app, true, Some("new objective")));
    answer(
        &mut app,
        ticket,
        goal::Reply::Goal(Some(goal("old", GoalStatus::Active))),
    );
    let fx = verb(
        &mut app,
        goal::Intent::Replace {
            thread: ThreadId::new(T),
            objective: "new objective".into(),
        },
    );
    let (ticket, ask) = only(&fx);
    assert_eq!(ask, Ask::Clear);
    let (ticket, ask) = only(&answer(&mut app, ticket, goal::Reply::Cleared(true)));
    assert_eq!(
        ask,
        Ask::Set {
            objective: Some("new objective".into()),
            status: Some(GoalStatus::Active),
            token_budget: None
        }
    );
    let set = goal("new objective", GoalStatus::Active);
    answer(&mut app, ticket, goal::Reply::Goal(Some(set.clone())));
    assert_eq!(notes(&app), [(Tone::Info, GoalNote::Status(set))]);
}

#[test]
fn an_objective_over_a_complete_goal_replaces_it_without_asking() {
    let mut app = ready();
    let (ticket, _) = only(&command(&mut app, true, Some("next")));
    let fx = answer(
        &mut app,
        ticket,
        goal::Reply::Goal(Some(goal("done", GoalStatus::Complete))),
    );
    assert_eq!(only(&fx).1, Ask::Clear);
    assert_eq!(wishes(&app), []);
}

#[test]
fn the_edit_prompt_keeps_the_status_and_the_budget_of_the_goal() {
    let mut app = ready();
    let (ticket, _) = only(&command(&mut app, true, Some("edit")));
    answer(
        &mut app,
        ticket,
        goal::Reply::Goal(Some(goal("old", GoalStatus::Paused))),
    );
    let edit = goal::Wish::Edit {
        thread: ThreadId::new(T),
        objective: "old".into(),
        status: GoalStatus::Paused,
        token_budget: Some(5_000),
    };
    assert_eq!(wishes(&app), [edit]);
    // The prompt's text is the UI's; the verb carries the submitted objective.
    let fx = verb(
        &mut app,
        goal::Intent::Edit {
            thread: ThreadId::new(T),
            objective: "  better \n".into(),
            status: GoalStatus::Paused,
            token_budget: Some(5_000),
        },
    );
    assert_eq!(
        only(&fx).1,
        Ask::Set {
            objective: Some("better".into()),
            status: Some(GoalStatus::Paused),
            token_budget: Some(Some(5_000)),
        }
    );
}

#[test]
fn a_finished_goal_is_edited_back_to_active() {
    let mut app = ready();
    let (ticket, _) = only(&command(&mut app, true, Some("edit")));
    answer(
        &mut app,
        ticket,
        goal::Reply::Goal(Some(goal("old", GoalStatus::BudgetLimited))),
    );
    let wished = wishes(&app);
    let [goal::Wish::Edit { status, .. }] = wished.as_slice() else {
        panic!("the edit prompt is wished: {wished:?}");
    };
    assert_eq!(*status, GoalStatus::Active);
}

#[test]
fn edit_over_no_goal_is_an_error_then_the_usage() {
    let mut app = ready();
    let (ticket, _) = only(&command(&mut app, true, Some("edit")));
    answer(&mut app, ticket, goal::Reply::Goal(None));
    assert_eq!(
        notes(&app),
        [
            (Tone::Error, GoalNote::NoGoalToEdit),
            (Tone::Info, GoalNote::Usage(GoalHint::CreateFirst)),
        ]
    );
    assert_eq!(wishes(&app), []);

    // Before the session starts the same two lines come without an ask.
    let mut app = CodexApp::new();
    assert_eq!(asked(&command(&mut app, false, Some("edit"))), []);
}

#[test]
fn a_failed_ask_is_the_error_of_its_step_and_a_temporary_session_is_explained() {
    let mut app = ready();
    let (ticket, _) = only(&command(&mut app, true, Some("clear")));
    fail(&mut app, ticket, "boom");
    let (ticket, _) = only(&command(&mut app, true, None));
    fail(
        &mut app,
        ticket,
        "thread/goal/get failed: ephemeral thread does not support goals: thread-1",
    );
    let (ticket, _) = only(&command(&mut app, true, Some("pause")));
    fail(&mut app, ticket, "denied");
    assert_eq!(
        notes(&app),
        [
            (
                Tone::Error,
                GoalNote::Failed {
                    action: GoalAction::Clear,
                    failure: GoalFailure::Other("boom".into())
                }
            ),
            (
                Tone::Error,
                GoalNote::Failed {
                    action: GoalAction::Read,
                    failure: GoalFailure::Ephemeral
                }
            ),
            (
                Tone::Error,
                GoalNote::Failed {
                    action: GoalAction::Update,
                    failure: GoalFailure::Other("denied".into())
                }
            ),
        ]
    );
}

#[test]
fn a_failed_replace_names_the_step_that_failed() {
    let mut app = ready();
    let (ticket, _) = only(&command(&mut app, true, Some("next")));
    let (ticket, _) = only(&answer(
        &mut app,
        ticket,
        goal::Reply::Goal(Some(goal("done", GoalStatus::Complete))),
    ));
    fail(&mut app, ticket, "no clear");
    assert_eq!(
        notes(&app),
        [(
            Tone::Error,
            GoalNote::Failed {
                action: GoalAction::Replace,
                failure: GoalFailure::Other("no clear".into())
            }
        )]
    );
}

#[test]
fn the_resume_verb_sets_the_goal_active() {
    let mut app = ready();
    // `Leave paused` says nothing to the feature; `Resume goal` is this verb.
    let fx = verb(
        &mut app,
        goal::Intent::Resume {
            thread: ThreadId::new(T),
        },
    );
    assert_eq!(
        only(&fx).1,
        Ask::Set {
            objective: None,
            status: Some(GoalStatus::Active),
            token_budget: None
        }
    );
}

/// A goal is set and cleared as the user (the server takes a goal change without that
/// provenance as the model's); a token budget of `Some(None)` removes it. Expected values are
/// the oracle's (`app_server_session.rs`: `thread_goal_set|clear|get`).
#[test]
fn goals_are_set_cleared_and_read_as_the_user() {
    let wire = |ask: Ask| {
        codex_tui_core::features::wire(
            &FeatureCommand::Goal(ask),
            &ThreadId::new(T),
            RequestId::Integer(1),
        )
    };
    assert_eq!(
        wire(Ask::Set {
            objective: Some("ship it".into()),
            status: Some(GoalStatus::Paused),
            token_budget: Some(None),
        }),
        ClientRequest::ThreadGoalSet {
            request_id: RequestId::Integer(1),
            params: ThreadGoalSetParams {
                origin: Some(ThreadGoalMutationOrigin::User),
                thread_id: T.into(),
                objective: Some("ship it".into()),
                status: Some(ThreadGoalStatus::Paused),
                token_budget: Some(None),
            },
        }
    );
    assert_eq!(
        wire(Ask::Clear),
        ClientRequest::ThreadGoalClear {
            request_id: RequestId::Integer(1),
            params: ThreadGoalClearParams {
                origin: Some(ThreadGoalMutationOrigin::User),
                thread_id: T.into(),
            },
        }
    );
    assert_eq!(
        wire(Ask::Get),
        ClientRequest::ThreadGoalGet {
            request_id: RequestId::Integer(1),
            params: ThreadGoalGetParams {
                thread_id: T.into(),
            },
        }
    );
}

/// What the thread started, as texts.
fn started_texts(fx: &AppFx) -> Vec<String> {
    fx.commands()
        .iter()
        .filter_map(|c| match c {
            Command::StartTurn { message, .. } => Some(message.text.clone()),
            _ => None,
        })
        .collect()
}

/// `maybe_send_next_queued_input` after the goal is set (oracle
/// `app/thread_goal_actions.rs`): `/goal` stops the queue while the screen runs it, and the set
/// goal lets it go on.
#[test]
fn a_goal_set_lets_the_queue_go_on() {
    let mut app = ready();
    run(&mut app, AppEv::Backend(turn_started(T, "turn-1")));
    let tab = |app: &mut CodexApp, text: &str| {
        run(
            app,
            AppEv::Intent(Intent::ThreadScreen {
                thread: ThreadId::new(T),
                intent: ScreenThreadIntent::Chat(chat::ThreadIntent::Queue(Queued::of(
                    UserMessage::text(text),
                    false,
                ))),
            }),
        )
    };
    tab(&mut app, "/goal ship it");
    tab(&mut app, "hello");
    let fx = run(&mut app, AppEv::Backend(turn_completed(T, "turn-1")));
    assert!(
        started_texts(&fx).is_empty(),
        "the command is handed to the screen first"
    );
    // the screen ran the command it was handed: `/goal` stops the queue
    let fx = run(
        &mut app,
        AppEv::Intent(Intent::ThreadScreen {
            thread: ThreadId::new(T),
            intent: ScreenThreadIntent::Chat(chat::ThreadIntent::QueueRan(Ran::Dispatched(
                Dispatched {
                    command: Some(SlashCommand::Goal),
                    modal: false,
                    can_change_directory: true,
                    managed_worktree: false,
                },
            ))),
        }),
    );
    assert!(started_texts(&fx).is_empty(), "the goal stops the queue");
    let (ticket, ask) = only(&command(&mut app, true, Some("ship it")));
    assert_eq!(ask, Ask::Get);
    let (ticket, ask) = only(&answer(&mut app, ticket, goal::Reply::Goal(None)));
    assert!(matches!(ask, Ask::Set { .. }));
    let set = goal("ship it", GoalStatus::Active);
    let fx = answer(&mut app, ticket, goal::Reply::Goal(Some(set)));
    assert_eq!(started_texts(&fx), ["hello"]);
}
