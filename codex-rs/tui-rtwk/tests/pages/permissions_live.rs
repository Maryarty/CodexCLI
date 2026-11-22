//! `/permissions` in the chat: the typed command opens the menu behind
//! "Loading permission profiles…" until the server's catalog is read, a
//! pick asks the server to update the thread and says
//! `Permissions updated to ..` in the transcript.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::disallowed_methods)]

use codex_tui_core::app::{AppEv, Command};
use codex_tui_core::ids::Ticket;
use codex_tui_core::rpc::Reply as AppReply;
use codex_tui_core::screens::select;
use codex_tui_core::screens::{ScreenCommand, ScreenReply};

use crate::chat::chat_with_history;
use crate::common::{Harness, ms, start};

fn chat() -> Harness {
    let mut h = start();
    chat_with_history(&mut h);
    h
}

fn send(h: &mut Harness, line: &str) {
    h.t.type_text(line);
    h.press("enter");
    h.t.pump_for(ms(400));
}

fn selects(h: &Harness) -> Vec<(Ticket, select::Command)> {
    h.fake
        .commands()
        .into_iter()
        .filter_map(|command| match command {
            Command::Screen {
                ticket,
                command: ScreenCommand::Select(command),
                ..
            } => Some((ticket, command)),
            _ => None,
        })
        .collect()
}

fn reply(h: &mut Harness, ticket: Ticket, reply: select::Reply) {
    let epoch = h.fake.epoch().expect("connected");
    h.fake.inject(
        epoch,
        AppEv::Reply(ticket, AppReply::screen(ScreenReply::Select(reply))),
    );
}

fn ticket_of(h: &Harness, wanted: impl Fn(&select::Command) -> bool) -> Ticket {
    selects(h)
        .into_iter()
        .find(|(_, command)| wanted(command))
        .unwrap_or_else(|| panic!("no such ask in {:?}", h.fake.commands()))
        .0
}

/// The menu opens loading, shows the catalog once the server answered, and a
/// pick updates the thread and is said in the transcript.
#[test]
fn a_pick_updates_the_thread_and_the_transcript_says_so() {
    let mut h = chat();
    send(&mut h, "/permissions");
    h.wait_for("the loading menu", |s| {
        s.contains("Loading permission profiles")
    });
    h.settle();
    let requirements = ticket_of(&h, |c| matches!(c, select::Command::Requirements { .. }));
    reply(&mut h, requirements, select::Reply::Requirements(None));
    h.settle();
    let page = ticket_of(&h, |c| matches!(c, select::Command::Profiles { .. }));
    let profiles = [":read-only", ":workspace", ":danger-full-access"]
        .map(|id| serde_json::from_value(serde_json::json!({ "id": id })).expect("profile"));
    reply(
        &mut h,
        page,
        select::Reply::Profiles {
            profiles: profiles.to_vec(),
            next: None,
        },
    );
    h.wait_for("the menu", |s| s.contains("1. Ask for approval"));
    h.press("enter");
    h.settle();
    let update = ticket_of(&h, |c| matches!(c, select::Command::Settings { .. }));
    reply(&mut h, update, select::Reply::Updated);
    h.wait_for("the note", |s| {
        s.contains("Permissions updated to Ask for approval")
    });
    assert!(
        !h.t.screen().contains("Update Model Permissions"),
        "{}",
        h.t.screen()
    );
}
