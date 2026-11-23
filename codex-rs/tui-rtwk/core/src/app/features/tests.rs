//! The door of a feature to the host (`AppCx::effect`, ADR-012), end to end through the app. The
//! ctx of a real feature is built here because `Out` and `Effects` are typed by the list's roots
//! (the rules let this file and the framework's probes name one: `ROOT_TESTS`).

use super::*;
use crate::app::effects::{Ask, CopyStatus, is_feature_request};
use crate::app::{AppEv, apply_app};
use crate::features::goal;

fn ctx() -> FeatureCtx {
    FeatureCtx::Goal(goal::Ctx {
        thread: ThreadId::new("t1"),
        flow: goal::Flow::Show,
    })
}

/// What `AppCx::effect` makes of an ask: the command the host runs, with its request number.
fn ask_host(app: &mut CodexApp, ask: Ask) -> (u64, Ask) {
    let mut fx = AppFx::default();
    carry(app, Out::Effect { ask, ctx: ctx() }, &mut fx);
    let [Command::Effect { request, ask }] = fx.commands() else {
        panic!("one ask of the host: {:?}", fx.commands());
    };
    (*request, ask.clone())
}

fn answer(app: &mut CodexApp, request: u64, done: Done) {
    apply_app(app, AppEv::Effect { request, done }, &mut AppFx::default());
}

/// The app chooses the request number of a feature's ask (never a screen's), the host runs the
/// ask as it is, and the answers are the feature's: a copy's `Pending` does not end the ask, the
/// last answer does, and a late one is dropped.
#[test]
fn a_features_host_ask_goes_out_under_the_apps_number_and_its_answers_are_the_features() {
    let mut app = CodexApp::new();
    let (first, ask) = ask_host(&mut app, Ask::ClipboardText);
    let (second, _) = ask_host(&mut app, Ask::ClipboardText);
    assert_eq!(
        ask,
        Ask::ClipboardText,
        "the host's own capability, unchanged"
    );
    assert!(is_feature_request(first) && is_feature_request(second));
    assert_ne!(first, second);
    assert_eq!(app.effects.feature_asks_waiting(), 2);

    answer(&mut app, first, Done::Copy(Ok(CopyStatus::Pending(7))));
    assert_eq!(
        app.effects.feature_asks_waiting(),
        2,
        "a pending copy waits"
    );
    answer(&mut app, first, Done::Copy(Ok(CopyStatus::Confirmed)));
    assert_eq!(
        app.effects.feature_asks_waiting(),
        1,
        "its last answer ends it"
    );
    answer(&mut app, first, Done::ClipboardText(Ok("late".to_owned())));
    assert_eq!(
        app.effects.feature_asks_waiting(),
        1,
        "a late one is dropped"
    );
    assert!(
        app.effect(first).is_none()
            && app.effect(second).is_none()
            && app.latest_effect().is_none(),
        "no screen reads a feature's answer"
    );
}

/// The asks of a screen keep their own path: the answer is kept for the screen to read.
#[test]
fn a_screens_host_ask_is_still_kept_for_the_screen() {
    let mut app = CodexApp::new();
    answer(&mut app, 5, Done::ClipboardText(Ok("text".to_owned())));
    assert_eq!(
        app.effect(5),
        Some(&Done::ClipboardText(Ok("text".to_owned())))
    );
}

fn open_prompt(app: &CodexApp) -> Prompt {
    match &app.screens.dialogs.open {
        Some(Open::Prompt(prompt)) => prompt.clone(),
        other => panic!("not a prompt: {other:?}"),
    }
}

fn goal_prompt(thread: &str) -> FeaturePrompt {
    FeaturePrompt::Goal(goal::Prompt::Edit {
        thread: ThreadId::new(thread),
        status: crate::threads::GoalStatus::Active,
        token_budget: None,
    })
}

/// The door of a generated suggestion for an open prompt (`AppCx::prompt_suggesting`, `suggest`):
/// the prompt says it waits; the suggestion fills the field once, and only the prompt it is for;
/// what the user typed first wins; a prompt that was closed takes nothing.
#[test]
fn a_suggestion_fills_the_prompt_it_is_for_once_and_never_over_the_users_typing() {
    use crate::screens::dialogs::Suggestion;
    let prompt = |suggesting| Out::Prompt {
        prompt: goal_prompt("t1"),
        initial: "keep".to_owned(),
        suggesting,
    };
    let suggest = |thread: &str, text: &str| Out::Suggest {
        prompt: goal_prompt(thread),
        text: Some(text.to_owned()),
    };
    let mut fx = AppFx::default();

    let mut app = CodexApp::new();
    carry(&mut app, prompt(false), &mut fx);
    assert_eq!(open_prompt(&app).suggestion, Suggestion::Idle);
    carry(&mut app, suggest("t1", "no one asked"), &mut fx);
    assert_eq!(open_prompt(&app).suggestion, Suggestion::Idle);

    carry(&mut app, prompt(true), &mut fx);
    assert_eq!(open_prompt(&app).suggestion, Suggestion::Loading);
    carry(&mut app, suggest("t2", "another prompt's"), &mut fx);
    assert_eq!(open_prompt(&app).suggestion, Suggestion::Loading);
    carry(&mut app, suggest("t1", "Fix the build"), &mut fx);
    assert_eq!(
        open_prompt(&app).suggestion,
        Suggestion::Applied("Fix the build".to_owned())
    );
    carry(&mut app, suggest("t1", "a second answer"), &mut fx);
    assert_eq!(
        open_prompt(&app).suggestion,
        Suggestion::Applied("Fix the build".to_owned())
    );
    assert_eq!(open_prompt(&app).initial, "keep");

    carry(&mut app, prompt(true), &mut fx);
    if let Some(Open::Prompt(open)) = &mut Rc::make_mut(&mut app.screens).dialogs.open {
        open.typed();
    }
    carry(&mut app, suggest("t1", "too late"), &mut fx);
    assert_eq!(open_prompt(&app).suggestion, Suggestion::Idle);

    Rc::make_mut(&mut app.screens).dialogs.open = None;
    carry(&mut app, suggest("t1", "to nobody"), &mut fx);
    assert_eq!(app.screens.dialogs.open, None);
}
