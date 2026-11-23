#![allow(clippy::expect_used, clippy::unwrap_used)]

//! The copy of a transcript selection as the app sees it (oracle `transcript_view/selection.rs`
//! `copy_selected_text_with`): the view's text goes to the host as an `Intent::Effect` with an
//! `Ask::Copy`, and the host's answers are kept under the request for whoever asked, newest last
//! (`CodexApp::effect`, `latest_effect`). What the composer gap says of them is the view's
//! (ADR-013): the follower it keeps reads exactly these two.

use codex_tui_core::app::effects::{Ask, CopyDestination, CopyFormat, CopyStatus, Done};
use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Intent, apply_app};
use pretty_assertions::assert_eq;

const REQUEST: u64 = 1 << 57;

fn ask(text: &str) -> Ask {
    Ask::Copy {
        text: text.to_owned(),
        format: CopyFormat::PlainText,
        destination: CopyDestination::Clipboard,
    }
}

fn copy(app: &mut CodexApp, request: u64, text: &str) {
    let mut fx = AppFx::default();
    apply_app(
        app,
        AppEv::Intent(Intent::Effect {
            request,
            ask: ask(text),
        }),
        &mut fx,
    );
    let [
        Command::Effect {
            request: sent,
            ask: asked,
            ..
        },
    ] = fx.commands()
    else {
        panic!("one ask of the host: {:?}", fx.commands());
    };
    assert_eq!((*sent, asked), (request, &ask(text)));
}

fn answer(app: &mut CodexApp, request: u64, done: Done) {
    apply_app(app, AppEv::Effect { request, done }, &mut AppFx::default());
}

#[test]
fn nothing_is_kept_until_the_host_answers() {
    let mut app = CodexApp::new();
    copy(&mut app, REQUEST | 1, "row 03");
    assert_eq!(app.effect(REQUEST | 1), None);
    assert_eq!(app.latest_effect(), None);
}

#[test]
fn an_answer_is_kept_under_its_request() {
    let mut app = CodexApp::new();
    copy(&mut app, REQUEST | 1, "row 03");
    answer(&mut app, REQUEST | 1, Done::Copy(Ok(CopyStatus::Confirmed)));
    assert_eq!(
        app.effect(REQUEST | 1),
        Some(&Done::Copy(Ok(CopyStatus::Confirmed)))
    );
    assert_eq!(app.effect(REQUEST | 2), None);
}

#[test]
fn a_pending_copy_that_ends_replaces_its_pending_answer() {
    let mut app = CodexApp::new();
    copy(&mut app, REQUEST | 1, "abc");
    answer(
        &mut app,
        REQUEST | 1,
        Done::Copy(Ok(CopyStatus::Pending(1))),
    );
    assert_eq!(
        app.latest_effect(),
        Some((REQUEST | 1, &Done::Copy(Ok(CopyStatus::Pending(1)))))
    );
    answer(&mut app, REQUEST | 1, Done::Copy(Ok(CopyStatus::Confirmed)));
    assert_eq!(
        app.latest_effect(),
        Some((REQUEST | 1, &Done::Copy(Ok(CopyStatus::Confirmed))))
    );
}

#[test]
fn the_newest_answer_is_the_last_to_arrive_not_the_highest_request() {
    let mut app = CodexApp::new();
    copy(&mut app, REQUEST | 1, "abc");
    answer(
        &mut app,
        REQUEST | 1,
        Done::Copy(Ok(CopyStatus::Pending(1))),
    );
    copy(&mut app, REQUEST | 2, "def");
    answer(&mut app, REQUEST | 2, Done::Copy(Ok(CopyStatus::Busy)));
    assert_eq!(
        app.latest_effect(),
        Some((REQUEST | 2, &Done::Copy(Ok(CopyStatus::Busy))))
    );
    // the first one ends after the refusal of the second
    answer(&mut app, REQUEST | 1, Done::Copy(Ok(CopyStatus::Confirmed)));
    assert_eq!(
        app.latest_effect(),
        Some((REQUEST | 1, &Done::Copy(Ok(CopyStatus::Confirmed))))
    );
    assert_eq!(
        app.effect(REQUEST | 2),
        Some(&Done::Copy(Ok(CopyStatus::Busy)))
    );
}
