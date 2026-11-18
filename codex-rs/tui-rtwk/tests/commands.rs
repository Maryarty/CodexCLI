//! The slash commands as the chat shows them: the descriptions (the oracle's
//! text, read from its source), and what a line that does not open a screen
//! leads to in the composer (the oracle's messages, `slash_dispatch.rs`).

use codex_tui_rtwk::l10n::Lang;
use codex_tui_rtwk::screens::chat::strings;
use codex_tui_rtwk_core::commands::SlashCommand;

use crate::chat::{assert_chat_back, chat_with_history};
use crate::common::{Harness, ms, start};

fn oracle(path: &str) -> String {
    let full = format!("{}/../tui/src/{path}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&full).unwrap_or_else(|e| panic!("{full}: {e}"))
}

/// The oracle's `description()`: every variant of an arm, with its string.
fn oracle_descriptions() -> Vec<(String, String)> {
    let text = oracle("slash_command.rs");
    let body = text
        .split("pub fn description(self)")
        .nth(1)
        .and_then(|rest| rest.split("pub fn command(self)").next())
        .expect("description()");
    let mut out = Vec::new();
    let mut pending: Vec<String> = Vec::new();
    let mut rest = body;
    while let Some(at) = rest.find(['"', 'S']) {
        rest = &rest[at..];
        if let Some(after) = rest.strip_prefix("SlashCommand::") {
            let end = after
                .find(|c: char| !c.is_alphanumeric())
                .unwrap_or(after.len());
            pending.push(after[..end].to_string());
            rest = &after[end..];
        } else if let Some(after) = rest.strip_prefix('"') {
            let end = after.find('"').expect("closing quote");
            for variant in pending.drain(..) {
                out.push((variant, after[..end].to_string()));
            }
            rest = &after[end + 1..];
        } else {
            rest = &rest[1..];
        }
    }
    out
}

#[test]
fn every_description_is_the_oracles_text() {
    let table = oracle_descriptions();
    assert_eq!(table.len(), SlashCommand::ALL.len());
    for (variant, text) in table {
        let command = SlashCommand::ALL
            .iter()
            .copied()
            .find(|c| format!("{c:?}") == variant)
            .unwrap_or_else(|| panic!("no command {variant}"));
        assert_eq!(strings::description(Lang::En, command), text, "{variant}");
    }
}

#[test]
fn the_german_table_inherits_the_descriptions() {
    // A locale that gives none falls back to English, never to a blank.
    for command in SlashCommand::ALL {
        assert!(!strings::description(Lang::Ru, command).is_empty());
    }
}

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

#[test]
fn unrecognized_slash_command_shows_the_oracles_message_and_keeps_the_draft() {
    let mut h = chat();
    send(&mut h, "/does-not-exist");
    let screen = h.t.screen();
    assert!(
        screen.contains(
            r#"Unrecognized command '/does-not-exist'. Type "/" for a list of supported commands."#
        ),
        "{screen}"
    );
    assert!(
        screen.contains("/does-not-exist"),
        "the draft is kept:\n{screen}"
    );
    assert_chat_back(&h, None);
}

#[test]
fn a_path_is_a_message_not_a_command() {
    let mut h = chat();
    h.fake.script_turn(vec![]);
    send(&mut h, "/usr/bin/env ls");
    let screen = h.t.screen();
    assert!(!screen.contains("Unrecognized command"), "{screen}");
}

#[test]
fn a_command_that_needs_the_session_says_it_is_not_available_yet() {
    for line in ["/compact", "/init", "/new", "/fork", "/copy", "/mcp"] {
        let mut h = chat();
        send(&mut h, line);
        let screen = h.t.screen();
        let name = line.trim_start_matches('/');
        assert!(
            screen.contains(&format!("'/{name}' is not available yet.")),
            "{line}:\n{screen}"
        );
    }
}

#[test]
fn usage_notices_of_the_oracle() {
    for (line, message) in [
        ("/pwd now", "Usage: /pwd"),
        ("/usage monthly", "Usage: /usage [daily|weekly|cumulative]"),
        ("/raw maybe", "Usage: /raw [on|off]"),
        ("/keymap nope", "Usage: /keymap [debug]"),
        (
            "/mcp full",
            "Usage: /mcp [verbose | login <name> [--environment <id>]]",
        ),
        (
            "/mcp login",
            "Usage: /mcp login <name> [--environment <id>]",
        ),
    ] {
        let mut h = chat();
        send(&mut h, line);
        assert!(h.t.screen().contains(message), "{line}:\n{}", h.t.screen());
    }
}

#[test]
fn slash_mention_inserts_an_at_sign() {
    let mut h = chat();
    send(&mut h, "/mention");
    let screen = h.t.screen();
    assert!(screen.contains("› @"), "{screen}");
}

#[test]
fn a_command_is_disabled_while_a_task_runs() {
    let mut h = start();
    h.wait_for("ready", |s| s.contains("ready"));
    // A turn that never completes: the task is running.
    h.fake.script_turn(vec![]);
    send(&mut h, "work");
    h.wait_for("working", |s| s.contains("working"));
    send(&mut h, "/clear");
    assert!(
        h.t.screen()
            .contains("'/clear' is disabled while a task is in progress."),
        "{}",
        h.t.screen()
    );
}
