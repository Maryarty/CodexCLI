//! The app's half of the public verbs a feature has (ADR-012 "public verbs"): each verb is
//! carried out by its owner's own step, as if the owner had been told by its own surface.

use super::{AppEv, AppFx, CodexApp, Intent, apply_app, select};
use crate::features::{PublicVerb, rate_offer_origin_available};
use crate::screens::{ScreenIntent, chat};

/// One verb a feature asked for.
pub(super) fn carry(app: &mut CodexApp, verb: PublicVerb, fx: &mut AppFx) {
    match verb {
        PublicVerb::StatusOverride(set) => {
            let intent = Intent::Screen(ScreenIntent::Chat(chat::Intent::StatusOverride(set)));
            apply_app(app, AppEv::Intent(intent), fx);
        }
        PublicVerb::PickModel(selection) => select::model_verb(app, selection, fx),
        PublicVerb::PickPermissions(selection) => select::permissions_apply(app, selection, fx),
        PublicVerb::OverrideTurn {
            thread,
            turn,
            address,
            model,
            effort,
        } => {
            let owner = app.screens.agents.owner_of(thread.as_str());
            if app.session.active.as_ref() == Some(&thread)
                && app.address(owner) == address
                && !super::features::navigation_pending(app)
                && app
                    .threads
                    .get(&thread)
                    .is_some_and(|model| rate_offer_origin_available(model.view(), &turn))
            {
                select::turn_override(app, model, effort, fx);
            }
        }
        PublicVerb::NotifyOwner(nudge) => {
            let backend = super::session::active_backend(app);
            super::notify_owner(app, backend, nudge, fx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{Command, Ingress, apply_thread};
    use crate::ids::{BackendId, ThreadId, TurnId};
    use crate::rpc::Reply;
    use crate::screens::ScreenCommand;
    use crate::screens::select;
    use crate::threads::{SettingsPatch, ThreadEv, TurnEnd};
    use codex_protocol::openai_models::ReasoningEffort;

    fn open_app() -> CodexApp {
        let mut app = CodexApp::new();
        let mut fx = AppFx::default();
        apply_app(&mut app, AppEv::Connected, &mut fx);
        let ticket = fx
            .commands()
            .iter()
            .find_map(|command| match command {
                Command::StartThread { ticket, .. } => Some(*ticket),
                _ => None,
            })
            .expect("a thread starts on connect");
        let reply = Reply::thread_started(ThreadId::new("thread-1"));
        apply_app(&mut app, AppEv::Reply(ticket, reply), &mut AppFx::default());
        app
    }

    fn record_completion(app: &mut CodexApp, thread: &ThreadId, turn: &TurnId) {
        apply_thread(
            app,
            thread,
            ThreadEv::TurnEnded {
                turn: turn.clone(),
                end: TurnEnd::Completed,
            },
            &mut AppFx::default(),
        );
    }

    /// The rate limit prompt's switch: the thread's settings change for the next turns (the
    /// model and the effort, nothing else), no line is said and no config key is written.
    #[test]
    fn an_override_turn_sets_the_model_and_effort_for_the_next_turns_only() {
        let mut app = open_app();
        let mut fx = AppFx::default();
        let thread = ThreadId::new("thread-1");
        let turn = TurnId::new("turn-1");
        record_completion(&mut app, &thread, &turn);
        let verb = PublicVerb::OverrideTurn {
            thread,
            turn,
            address: app.address(BackendId::default()),
            model: "gpt-5.1-codex-mini".to_owned(),
            effort: Some(ReasoningEffort::Medium),
        };
        carry(&mut app, verb, &mut fx);
        let asked: Vec<_> = fx
            .commands()
            .iter()
            .filter_map(|command| match command {
                Command::Screen {
                    command: ScreenCommand::Select(command),
                    ..
                } => Some(command),
                _ => None,
            })
            .collect();
        let [select::Command::Settings { thread, patch }] = asked.as_slice() else {
            panic!("one thread settings update, got {asked:?}");
        };
        assert_eq!(thread.as_str(), "thread-1");
        assert_eq!(
            *patch,
            SettingsPatch {
                model: Some("gpt-5.1-codex-mini".to_owned()),
                effort: Some(ReasoningEffort::Medium),
                ..SettingsPatch::default()
            }
        );
        assert!(app.screens().chat.notes.is_empty(), "no line is said");
    }

    #[test]
    fn an_override_turn_from_the_old_transport_cannot_set_the_model() {
        let mut app = open_app();
        let backend = BackendId::default();
        let thread = ThreadId::new("thread-1");
        let turn = TurnId::new("turn-1");
        record_completion(&mut app, &thread, &turn);
        let address = app.address(backend);
        app.ingest(Ingress::transport_replaced(
            backend,
            app.epoch(),
            address.epoch.next(),
        ))
        .expect("the new transport was installed");
        assert_eq!(app.session.active.as_ref(), Some(&thread));

        let mut fx = AppFx::default();
        carry(
            &mut app,
            PublicVerb::OverrideTurn {
                thread,
                turn,
                address,
                model: "gpt-6-luna".to_owned(),
                effort: None,
            },
            &mut fx,
        );
        assert!(fx.commands().is_empty());
    }

    #[test]
    fn an_override_turn_from_a_previous_turn_cannot_set_the_model() {
        let mut app = open_app();
        let thread = ThreadId::new("thread-1");
        let turn = TurnId::new("turn-1");
        record_completion(&mut app, &thread, &turn);
        apply_thread(
            &mut app,
            &thread,
            ThreadEv::TurnStarted {
                turn: TurnId::new("turn-2"),
                started_at: None,
            },
            &mut AppFx::default(),
        );
        let address = app.address(BackendId::default());
        let mut fx = AppFx::default();
        carry(
            &mut app,
            PublicVerb::OverrideTurn {
                thread,
                turn,
                address,
                model: "gpt-6-luna".to_owned(),
                effort: None,
            },
            &mut fx,
        );
        assert!(fx.commands().is_empty());
    }

    /// A model picked by a verb is no step of the select screen, so nothing else says the line it
    /// leaves: it is in the thread's transcript at once, not held on the screen for the next pick.
    #[test]
    fn a_model_picked_by_a_verb_says_its_line_at_once() {
        use crate::screens::select::models::{Apply, Selection};

        let mut app = open_app();
        let mut fx = AppFx::default();
        let selection = Selection {
            model: "gpt-5.1-codex-mini".to_owned(),
            effort: Some(ReasoningEffort::Medium),
            apply: Apply::Session,
            warning: None,
        };
        carry(&mut app, PublicVerb::PickModel(selection), &mut fx);
        assert!(
            app.screens().select.notes.is_empty(),
            "the line went into the transcript: {:?}",
            app.screens().select.notes
        );
    }

    /// The one path of the nudge: a feature's verb sends the e-mail through the same request the
    /// banner's action does, one at a time, for the thread's backend.
    #[test]
    fn a_notify_owner_verb_asks_for_one_nudge_at_a_time() {
        use crate::screens::chat::Nudge;

        let mut app = open_app();
        let sends = |app: &mut CodexApp| {
            let mut fx = AppFx::default();
            carry(app, PublicVerb::NotifyOwner(Nudge::UsageLimit), &mut fx);
            fx.commands()
                .iter()
                .filter(|command| {
                    matches!(
                        command,
                        Command::SendAddCreditsNudge {
                            nudge: Nudge::UsageLimit,
                            ..
                        }
                    )
                })
                .count()
        };
        assert_eq!(sends(&mut app), 1);
        assert_eq!(sends(&mut app), 0, "the first is still in flight");
    }
}
