#![allow(clippy::expect_used, clippy::unwrap_used)]

//! State tests of S17 (backtrack, Esc Esc): the machine of the two Escapes,
//! the prompts a transcript offers, and the turn a chosen prompt reverts
//! before. Expected values follow the oracle's `app_backtrack.rs` unit tests
//! (:807-1238) and `app_backtrack/{browsing,legacy_input}.rs`; the revert is
//! resolved by the transcript item's server ids where the oracle counts
//! visible prompts through the persisted turns again (the module doc).

use codex_app_server_protocol::ByteRange;
use codex_app_server_protocol::TextElement;
use codex_app_server_protocol::ThreadItem;
use codex_app_server_protocol::ThreadItemPayload;
use codex_app_server_protocol::Turn;
use codex_app_server_protocol::TurnItemsView;
use codex_app_server_protocol::TurnStatus;
use codex_app_server_protocol::UserInput;
use codex_tui_core::ids::{ItemId, ThreadId, TurnId};
use codex_tui_core::screens::backtrack::{
    Backtrack, Element, EscStep, Mention, Prompt, RevertError, Selection, Targets,
    has_backtrack_target, is_visible, nth_user, nth_user_position, revert_before_turn, user_count,
};
use codex_tui_core::threads::{Item, ItemKey, Notice, Transcript, UserPrompt};
use pretty_assertions::assert_eq;

fn key(turn: &str, item: &str) -> ItemKey {
    ItemKey {
        turn: TurnId::new(turn),
        item: ItemId::new(item),
    }
}

fn text_prompt(text: &str) -> UserPrompt {
    UserPrompt {
        text: text.to_string(),
        ..UserPrompt::default()
    }
}

fn transcript(items: Vec<(ItemKey, Item)>) -> Transcript {
    let mut transcript = Transcript::default();
    for (key, item) in items {
        transcript.upsert(key, item);
    }
    transcript
}

fn thread(id: &str) -> ThreadId {
    ThreadId::new(id)
}

fn two_prompts() -> Transcript {
    transcript(vec![
        (key("t1", "u1"), Item::User(text_prompt("first"))),
        (
            key("t1", "a1"),
            Item::Agent {
                text: "answer".into(),
                phase: None,
                questions: None,
            },
        ),
        (key("t2", "u2"), Item::User(text_prompt("second"))),
    ])
}

fn targets(count: usize) -> Targets {
    Targets {
        count,
        older_history: false,
    }
}

// -- the machine ------------------------------------------------------------

#[test]
fn the_first_escape_primes_with_a_hint_only_when_a_prompt_exists() {
    let id = thread("th");
    let mut machine = Backtrack::default();
    assert_eq!(
        machine.esc(Some(&id), true, targets(2)),
        EscStep::Primed { hint: true }
    );
    assert!(machine.is_primed());

    let mut empty = Backtrack::default();
    assert_eq!(
        empty.esc(Some(&id), true, targets(0)),
        EscStep::Primed { hint: false }
    );
}

#[test]
fn a_composer_with_a_draft_keeps_the_escape() {
    let id = thread("th");
    let mut machine = Backtrack::default();
    assert_eq!(machine.esc(Some(&id), false, targets(2)), EscStep::Ignored);
    assert_eq!(machine, Backtrack::Idle);
}

#[test]
fn the_second_escape_browses_from_the_newest_prompt() {
    let id = thread("th");
    let mut machine = Backtrack::default();
    machine.esc(Some(&id), true, targets(3));
    assert_eq!(
        machine.esc(Some(&id), true, targets(3)),
        EscStep::Browse { select: Some(2) }
    );
    assert_eq!(machine.selected(), Some(2));
}

#[test]
fn nothing_to_edit_says_so_and_goes_idle_unless_older_history_exists() {
    let id = thread("th");
    let mut machine = Backtrack::default();
    machine.esc(Some(&id), true, targets(0));
    assert_eq!(
        machine.esc(Some(&id), true, targets(0)),
        EscStep::NoPrevious
    );
    assert_eq!(machine, Backtrack::Idle);

    // Older pages may hold a prompt: browsing begins with nothing highlighted
    // and asks for history.
    let more = Targets {
        count: 0,
        older_history: true,
    };
    machine.esc(Some(&id), true, more);
    assert_eq!(
        machine.esc(Some(&id), true, more),
        EscStep::Browse { select: None }
    );
    assert!(machine.needs_history());
}

#[test]
fn left_and_right_step_and_stop_at_the_ends() {
    let id = thread("th");
    let mut machine = Backtrack::default();
    machine.esc(Some(&id), true, targets(3));
    machine.esc(Some(&id), true, targets(3));
    assert_eq!(machine.older(3), Some(1));
    assert_eq!(machine.older(3), Some(0));
    assert_eq!(machine.older(3), Some(0));
    assert!(machine.needs_history());
    assert_eq!(machine.newer(3), Some(1));
    assert_eq!(machine.newer(3), Some(2));
    assert_eq!(machine.newer(3), Some(2));
    assert!(!machine.needs_history());
    assert_eq!(machine.older(0), None);
}

#[test]
fn enter_chooses_the_highlighted_prompt_of_the_same_thread() {
    let id = thread("th");
    let mut machine = Backtrack::default();
    machine.esc(Some(&id), true, targets(2));
    machine.esc(Some(&id), true, targets(2));
    machine.older(2);
    assert_eq!(
        machine.confirm(Some(&id), 2),
        Some(Selection {
            thread: id.clone(),
            nth: 0
        })
    );
    assert_eq!(machine, Backtrack::Idle);
}

#[test]
fn enter_on_another_thread_or_a_vanished_prompt_chooses_nothing() {
    let id = thread("th");
    let mut machine = Backtrack::default();
    machine.esc(Some(&id), true, targets(2));
    machine.esc(Some(&id), true, targets(2));
    assert_eq!(machine.confirm(Some(&thread("other")), 2), None);
    assert_eq!(machine.confirm(None, 2), None);
    assert_eq!(machine.confirm(Some(&id), 1), None);
    assert!(
        machine.is_browsing(),
        "a refused choice leaves the browsing as it was"
    );
}

#[test]
fn escape_while_browsing_cancels_and_any_other_input_ends_a_primed_pair() {
    let id = thread("th");
    let mut machine = Backtrack::default();
    machine.esc(Some(&id), true, targets(2));
    machine.interrupt();
    assert_eq!(machine, Backtrack::Idle);

    machine.esc(Some(&id), true, targets(2));
    machine.esc(Some(&id), true, targets(2));
    machine.interrupt();
    assert!(
        machine.is_browsing(),
        "input while browsing is the browser's"
    );
    assert_eq!(machine.esc(Some(&id), true, targets(2)), EscStep::Cancelled);
    assert_eq!(machine, Backtrack::Idle);
}

#[test]
fn escape_in_the_detailed_transcript_browses_at_once() {
    let id = thread("th");
    let mut machine = Backtrack::default();
    assert_eq!(
        machine.begin_from_detailed(Some(&id), targets(2)),
        EscStep::Browse { select: Some(1) }
    );
    assert_eq!(
        machine.begin_from_detailed(Some(&id), targets(0)),
        EscStep::NoPrevious
    );
}

// -- the prompts of a transcript ------------------------------------------

#[test]
fn a_backtrack_target_needs_a_visible_user_prompt() {
    let mut items = vec![
        (
            key("t", "a"),
            Item::Agent {
                text: "assistant".into(),
                phase: None,
                questions: None,
            },
        ),
        (
            key("t", "n"),
            Item::Notice(Notice::ContextCompacted { took: None }),
        ),
    ];
    assert!(!has_backtrack_target(&transcript(items.clone())));
    items.push((key("t", "u"), Item::User(text_prompt("hello"))));
    let transcript = transcript(items);
    assert!(has_backtrack_target(&transcript));
    assert_eq!(user_count(&transcript), 1);
}

#[test]
fn a_prompt_of_control_characters_is_hidden_unless_it_has_elements() {
    assert!(!is_visible(&text_prompt("\u{0007}")));
    assert!(!is_visible(&text_prompt("  \n ")));
    assert!(is_visible(&text_prompt("hi")));
    #[allow(clippy::single_range_in_vec_init)]
    let with_element = UserPrompt {
        elements: vec![0..1],
        ..text_prompt("\u{0007}")
    };
    assert!(is_visible(&with_element));
}

#[test]
fn nth_counts_visible_prompts_from_the_oldest() {
    let transcript = transcript(vec![
        (key("t0", "hidden"), Item::User(text_prompt("\u{0007}"))),
        (key("t1", "u1"), Item::User(text_prompt("first"))),
        (
            key("t1", "a1"),
            Item::Agent {
                text: "answer".into(),
                phase: None,
                questions: None,
            },
        ),
        (key("t2", "u2"), Item::User(text_prompt("second"))),
    ]);
    assert_eq!(nth_user_position(&transcript, 0), Some(1));
    assert_eq!(nth_user_position(&transcript, 1), Some(3));
    assert_eq!(nth_user_position(&transcript, 2), None);
    let (id, prompt) = nth_user(&transcript, 1).expect("the second prompt");
    assert_eq!((id, prompt.text.as_str()), (&key("t2", "u2"), "second"));
}

#[test]
fn the_machine_reads_the_count_of_a_transcript() {
    let transcript = two_prompts();
    assert_eq!(
        Targets::of(&transcript, true),
        Targets {
            count: 2,
            older_history: true
        }
    );
}

// -- which turn a prompt reverts before -------------------------------------

fn turn(id: &str, status: TurnStatus, prompts: usize) -> Turn {
    Turn {
        id: id.to_string(),
        root_turn_id: None,
        items: (0..prompts)
            .map(|index| ThreadItemPayload::UserMessage {
                id: format!("{id}-user-{index}"),
                client_id: None,
                content: vec![UserInput::Text {
                    text: format!("{id}-prompt-{index}"),
                    text_elements: Vec::new(),
                }],
            })
            .map(ThreadItem::untimed)
            .collect(),
        items_view: TurnItemsView::Full,
        status,
        error: None,
        started_at: None,
        completed_at: None,
        duration_ms: None,
    }
}

fn first_of(turn: &str) -> ItemKey {
    key(turn, &format!("{turn}-user-0"))
}

#[test]
fn the_first_and_later_prompts_resolve_to_their_turns() {
    let turns = vec![
        turn("turn-1", TurnStatus::Completed, 1),
        turn("turn-compaction", TurnStatus::Completed, 0),
        turn("turn-2", TurnStatus::Completed, 1),
    ];
    let (before, prompt) = revert_before_turn(&turns, &first_of("turn-1")).expect("first");
    assert_eq!(
        (before.as_str(), prompt.text.as_str()),
        ("turn-1", "turn-1-prompt-0")
    );
    let (before, prompt) = revert_before_turn(&turns, &first_of("turn-2")).expect("later");
    assert_eq!(
        (before.as_str(), prompt.text.as_str()),
        ("turn-2", "turn-2-prompt-0")
    );
}

#[test]
fn a_mid_turn_steer_cannot_be_edited_even_when_the_page_starts_after_its_turn_start() {
    // Two prompts in one turn: the second is a steer. The persisted turn is
    // read whole, so how much of it the transcript loaded does not matter.
    let turns = vec![turn("turn-1", TurnStatus::Completed, 2)];
    let error = revert_before_turn(&turns, &key("turn-1", "turn-1-user-1")).expect_err("steer");
    assert_eq!(error, RevertError::Steer);
    assert_eq!(
        error.to_string(),
        "the selected prompt is a steer and cannot be edited independently"
    );
    assert!(revert_before_turn(&turns, &first_of("turn-1")).is_ok());
}

#[test]
fn an_in_progress_turn_and_a_missing_prompt_are_refused_with_the_oracles_words() {
    let turns = vec![turn("turn-1", TurnStatus::InProgress, 1)];
    let error = revert_before_turn(&turns, &first_of("turn-1")).expect_err("in progress");
    assert_eq!(
        error.to_string(),
        "the selected prompt belongs to a turn that is still in progress"
    );
    let missing = revert_before_turn(&turns, &key("turn-1", "nope")).expect_err("missing item");
    assert_eq!(
        missing.to_string(),
        "the selected prompt was not found in the persisted thread"
    );
    assert_eq!(
        revert_before_turn(&turns, &key("turn-9", "x")).expect_err("missing turn"),
        RevertError::NotFound
    );
}

#[test]
fn review_markers_in_other_turns_do_not_move_a_prompt() {
    // The oracle counts visible prompts past review markers; an id does not move.
    let mut review = turn("turn-review", TurnStatus::Completed, 1);
    review.items.insert(
        0,
        ThreadItem::untimed(ThreadItemPayload::EnteredReviewMode {
            id: "review-start".to_string(),
            review: "changes against main".to_string(),
        }),
    );
    review
        .items
        .push(ThreadItem::untimed(ThreadItemPayload::ExitedReviewMode {
            id: "review-end".to_string(),
            review: "review complete".to_string(),
        }));
    let turns = vec![
        turn("turn-1", TurnStatus::Completed, 1),
        review,
        turn("turn-2", TurnStatus::Completed, 1),
    ];
    let (before, _) = revert_before_turn(&turns, &first_of("turn-2")).expect("after review");
    assert_eq!(before.as_str(), "turn-2");
}

#[test]
fn the_persisted_prompt_keeps_elements_images_skills_and_mentions() {
    let mut selected = turn("turn-2", TurnStatus::Completed, 1);
    selected.items = vec![ThreadItem::untimed(ThreadItemPayload::UserMessage {
        id: "selected".to_string(),
        client_id: None,
        content: vec![
            UserInput::Text {
                text: "use $skill @sample [Image #1]".to_string(),
                text_elements: vec![TextElement::new(
                    ByteRange { start: 19, end: 29 },
                    Some("[Image #1]".to_string()),
                )],
            },
            UserInput::LocalImage {
                detail: None,
                path: "/tmp/a.png".into(),
            },
            UserInput::Skill {
                name: "skill".to_string(),
                path: "/tmp/skills/skill/SKILL.md".into(),
            },
            UserInput::Mention {
                name: "Sample Plugin".to_string(),
                path: "plugin://sample@test".to_string(),
            },
        ],
    })];
    let turns = vec![turn("turn-1", TurnStatus::Completed, 1), selected];
    let (_, prompt) = revert_before_turn(&turns, &key("turn-2", "selected")).expect("resolves");
    assert_eq!(
        prompt,
        Prompt {
            text: "use $skill @sample [Image #1]".to_string(),
            elements: vec![Element {
                range: 19..29,
                placeholder: Some("[Image #1]".to_string())
            }],
            local_images: vec!["/tmp/a.png".into()],
            remote_images: Vec::new(),
            mentions: vec![
                Mention {
                    skill: true,
                    name: "skill".to_string(),
                    path: "/tmp/skills/skill/SKILL.md".to_string()
                },
                Mention {
                    skill: false,
                    name: "Sample Plugin".to_string(),
                    path: "plugin://sample@test".to_string()
                },
            ],
        }
    );
}

// -- the edit: read, revert, wait for the server -----------------------------

/// The edit of a past prompt through the app, as the shell drives it: the intent, the
/// read, the revert, the server's `ThreadReverted`; the wait for it has a limit the view
/// times (`ThreadPart::awaiting`, `ThreadIntent::Expired`).
mod edit {
    use super::*;
    use codex_app_server_protocol::ServerNotification;
    use codex_tui_core::app::{AppEv, AppFx, BackendEv, CodexApp, Command, Intent, apply_app};
    use codex_tui_core::rpc::Reply;
    use codex_tui_core::screens::backtrack::{
        Command as BacktrackCommand, Draft, EditError, Reply as BacktrackReply, ThreadIntent,
    };
    use codex_tui_core::screens::{ScreenCommand, ScreenReply, ScreenThreadIntent};
    use pretty_assertions::assert_eq;
    use serde_json::json;

    const T: &str = "thread-1";

    fn run(app: &mut CodexApp, ev: AppEv) -> AppFx {
        let mut fx = AppFx::default();
        apply_app(app, ev, &mut fx);
        fx
    }

    fn notified(app: &mut CodexApp, notification: serde_json::Value) {
        let notification: ServerNotification =
            serde_json::from_value(notification).expect("notification");
        run(
            app,
            AppEv::Backend(BackendEv::Notification(Box::new(notification))),
        );
    }

    /// A thread with one finished turn that holds the prompt `first`.
    fn app_with_a_prompt() -> CodexApp {
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
            AppEv::Reply(ticket, Reply::thread_started(thread(T))),
        );
        notified(
            &mut app,
            json!({"method": "turn/started", "params": {"threadId": T, "turn": {
                "id": "turn-1", "items": [], "itemsView": "full", "status": "inProgress",
                "error": null}}}),
        );
        notified(
            &mut app,
            json!({"method": "item/completed", "params": {"threadId": T, "turnId": "turn-1",
                "completedAtMs": 0, "startedAtMs": 0,
                "item": {"type": "userMessage", "id": "u1", "clientId": null,
                    "content": [{"type": "text", "text": "first", "text_elements": []}]}}}),
        );
        notified(
            &mut app,
            json!({"method": "turn/completed", "params": {"threadId": T, "turn": {
                "id": "turn-1", "items": [], "itemsView": "notLoaded", "status": "completed",
                "error": null}}}),
        );
        app
    }

    fn part(app: &CodexApp) -> codex_tui_core::screens::backtrack::ThreadPart {
        app.active_thread().screen_slots().backtrack.clone()
    }

    /// The edit of the prompt `u1` of `turn-1`, as the view sends it on Enter.
    fn edit(app: &mut CodexApp) -> AppFx {
        let transcript = app.active_thread().transcript().clone();
        let chosen = Selection {
            thread: thread(T),
            nth: 0,
        };
        let edit = chosen.edit(&transcript).expect("the prompt is offered");
        run(
            app,
            AppEv::Intent(Intent::ThreadScreen {
                thread: thread(T),
                intent: ScreenThreadIntent::Backtrack(ThreadIntent::EditPrompt(edit)),
            }),
        )
    }

    fn asked(fx: &AppFx) -> (codex_tui_core::ids::Ticket, BacktrackCommand) {
        fx.commands()
            .iter()
            .find_map(|command| match command {
                Command::Screen {
                    ticket,
                    command: ScreenCommand::Backtrack(command),
                    ..
                } => Some((*ticket, command.clone())),
                _ => None,
            })
            .expect("a request of the backtrack")
    }

    fn answer(
        app: &mut CodexApp,
        ticket: codex_tui_core::ids::Ticket,
        reply: BacktrackReply,
    ) -> AppFx {
        run(
            app,
            AppEv::Reply(ticket, Reply::screen(ScreenReply::Backtrack(reply))),
        )
    }

    /// Edit, read answered, revert answered: the edit now waits on the server.
    fn reverted_and_waiting(app: &mut CodexApp) {
        let fx = edit(app);
        let (ticket, command) = asked(&fx);
        assert_eq!(
            command,
            BacktrackCommand::Read {
                key: key("turn-1", "u1")
            }
        );
        let fx = answer(
            app,
            ticket,
            BacktrackReply::Resolved(Ok((
                TurnId::new("turn-1"),
                Prompt {
                    text: "first".into(),
                    ..Prompt::default()
                },
            ))),
        );
        let (ticket, command) = asked(&fx);
        assert_eq!(
            command,
            BacktrackCommand::Revert {
                before_turn: TurnId::new("turn-1")
            }
        );
        answer(app, ticket, BacktrackReply::Reverted);
    }

    #[test]
    fn the_prompt_goes_to_the_composer_when_the_server_reverted_and_the_edit_waits_for_the_notice()
    {
        let mut app = app_with_a_prompt();
        reverted_and_waiting(&mut app);
        let part = part(&app);
        let restore = part.restore.as_ref().expect("the prompt goes back");
        assert_eq!(restore.draft.message().text, "first");
        assert!(part.awaiting().is_some(), "waiting for ThreadReverted");
        assert!(part.editing());

        notified(
            &mut app,
            json!({"method": "thread/reverted", "params": {"threadId": T}}),
        );
        let part = self::part(&app);
        assert!(!part.editing());
        assert_eq!(part.error, None);
    }

    #[test]
    fn ten_seconds_without_the_notice_say_so_and_the_composer_keeps_the_text() {
        assert_eq!(codex_tui_core::screens::backtrack::CONFIRM_SECS, 10);
        let mut app = app_with_a_prompt();
        reverted_and_waiting(&mut app);
        let ticket = part(&app).awaiting().expect("waiting");

        run(
            &mut app,
            AppEv::Intent(Intent::ThreadScreen {
                thread: thread(T),
                intent: ScreenThreadIntent::Backtrack(ThreadIntent::Expired(ticket)),
            }),
        );
        let part = part(&app);
        assert_eq!(part.error, Some(EditError::NotConfirmed));
        assert!(!part.editing());
        // the text was given when the revert answered, and is still the composer's to hold
        let restore = part.restore.as_ref().expect("kept");
        assert_eq!(restore.seq, 1);
        assert!(matches!(restore.draft, Draft::Persisted(_)));
    }

    #[test]
    fn a_notice_that_came_in_time_makes_a_late_timer_harmless() {
        let mut app = app_with_a_prompt();
        reverted_and_waiting(&mut app);
        let ticket = part(&app).awaiting().expect("waiting");
        notified(
            &mut app,
            json!({"method": "thread/reverted", "params": {"threadId": T}}),
        );
        run(
            &mut app,
            AppEv::Intent(Intent::ThreadScreen {
                thread: thread(T),
                intent: ScreenThreadIntent::Backtrack(ThreadIntent::Expired(ticket)),
            }),
        );
        assert_eq!(part(&app).error, None);
    }

    #[test]
    fn a_notice_before_the_answer_needs_no_wait() {
        let mut app = app_with_a_prompt();
        let fx = edit(&mut app);
        let (ticket, _) = asked(&fx);
        let fx = answer(
            &mut app,
            ticket,
            BacktrackReply::Resolved(Ok((TurnId::new("turn-1"), Prompt::default()))),
        );
        let (ticket, _) = asked(&fx);
        notified(
            &mut app,
            json!({"method": "thread/reverted", "params": {"threadId": T}}),
        );
        answer(&mut app, ticket, BacktrackReply::Reverted);
        let part = part(&app);
        assert!(part.restore.is_some());
        assert_eq!(part.awaiting(), None);
        assert!(!part.editing());
    }

    #[test]
    fn a_failed_read_puts_the_shown_prompt_back_with_the_servers_words() {
        let mut app = app_with_a_prompt();
        let fx = edit(&mut app);
        let (ticket, _) = asked(&fx);
        run(&mut app, AppEv::Reply(ticket, Reply::failed("boom")));
        let part = part(&app);
        assert_eq!(part.error, Some(EditError::Failed("boom".into())));
        let restore = part.restore.as_ref().expect("the prompt goes back");
        assert!(matches!(restore.draft, Draft::Shown(_)));
        assert_eq!(restore.draft.message().text, "first");
        assert!(!part.editing());
    }

    #[test]
    fn a_side_conversation_refuses_the_edit() {
        let mut app = app_with_a_prompt();
        run(
            &mut app,
            AppEv::Intent(Intent::ThreadScreen {
                thread: thread(T),
                intent: ScreenThreadIntent::Backtrack(ThreadIntent::Unavailable),
            }),
        );
        assert_eq!(part(&app).error, Some(EditError::Side));
    }
}
