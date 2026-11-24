//! The app's half of the public verbs a feature has (ADR-012 "public verbs"): each verb is
//! carried out by its owner's own step, as if the owner had been told by its own surface.

use super::{AppEv, AppFx, CodexApp, Intent, apply_app, select};
use crate::features::PublicVerb;
use crate::screens::{ScreenIntent, chat};

/// One verb a feature asked for.
pub(super) fn carry(app: &mut CodexApp, verb: PublicVerb, fx: &mut AppFx) {
    match verb {
        PublicVerb::StatusOverride(set) => {
            let intent = Intent::Screen(ScreenIntent::Chat(chat::Intent::StatusOverride(set)));
            apply_app(app, AppEv::Intent(intent), fx);
        }
        PublicVerb::PickModel(selection) => select::model_pick(app, selection, fx),
        PublicVerb::PickPermissions(selection) => select::permissions_apply(app, selection, fx),
        PublicVerb::OverrideTurn { model, effort } => {
            select::turn_override(app, model, effort, fx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Command;
    use crate::ids::ThreadId;
    use crate::rpc::Reply;
    use crate::screens::ScreenCommand;
    use crate::screens::select;
    use crate::threads::SettingsPatch;
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

    /// The rate limit prompt's switch: the thread's settings change for the next turns (the
    /// model and the effort, nothing else), no line is said and no config key is written.
    #[test]
    fn an_override_turn_sets_the_model_and_effort_for_the_next_turns_only() {
        let mut app = open_app();
        let mut fx = AppFx::default();
        let verb = PublicVerb::OverrideTurn {
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
}
