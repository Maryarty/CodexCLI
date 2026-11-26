//! The slash commands as the chat shows them: the descriptions (the oracle's
//! text, read from its source), and what a line that does not open a screen
//! leads to in the composer (the oracle's messages, `slash_dispatch.rs`).

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::disallowed_methods)]

use codex_tui_core::app::{AppEv, Command};
use codex_tui_core::commands::SlashCommand;
use codex_tui_core::rpc::Reply as AppReply;
use codex_tui_core::screens::{ScreenCommand, ScreenReply, agents};
use codex_tui_rtwk::l10n::Lang;
use codex_tui_rtwk::pages::session::strings;

use crate::chat::{assert_transcript_kept, chat_with_history};
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
    // The notice goes through the model (an intent), which answers on the
    // runtime thread: the screen is waited for, not pumped.
    h.wait_for("the notice", |s| s.contains("Unrecognized command"));
    let screen = h.t.screen();
    // The oracle's line wraps at the width: the words are what is compared.
    let words = screen.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        words.contains(
            r#"Unrecognized command '/does-not-exist'. Type "/" for a list of supported commands."#
        ),
        "{screen}"
    );
    assert!(
        screen.contains("/does-not-exist"),
        "the draft is kept:\n{screen}"
    );
    assert_transcript_kept(&h, None);
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
    // `/copy`, `/recap` and `/mcp verbose` are features now; `/mcp login <name>` (the sign-in is
    // not built) is still answered by `Notice::NotAvailable`.
    let mut h = chat();
    send(&mut h, "/mcp login github");
    let message = "'/mcp' is not available yet.";
    h.wait_for(message, |s| s.contains(message));
}

#[test]
fn usage_notices_of_the_oracle() {
    for (line, message) in [
        ("/pwd now", "Usage: /pwd"),
        ("/usage monthly", "Usage: /usage [daily|weekly|cumulative]"),
        ("/raw maybe", "Usage: /raw [on|off]"),
        ("/keymap nope", "Usage: /keymap [debug]"),
        ("/mcp full", "Usage: /mcp [verbose | login <name>]"),
        ("/mcp login", "Usage: /mcp [verbose | login <name>]"),
    ] {
        let mut h = chat();
        send(&mut h, line);
        h.wait_for(message, |s| s.contains(message));
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
    h.wait_idle();
    // A turn that never completes: the task is running.
    h.fake.script_turn(vec![]);
    send(&mut h, "work");
    h.wait_for("working", |s| s.contains("Working"));
    send(&mut h, "/clear");
    let message = "'/clear' is disabled while a task is in progress.";
    h.wait_for(message, |s| s.contains(message));
}

/// `model_popups.rs:19-24`: the words of `/model` before startup completes
/// (the session is always ready by the time the harness types, so the words
/// are the string table's; the decision is core's test).
#[test]
fn slash_model_before_startup_shows_the_oracles_info_line() {
    use codex_tui_core::commands::Notice;
    use codex_tui_rtwk::pages::session::strings::EN;
    assert_eq!(
        EN.notice(&Notice::ModelStartup),
        "Model selection is disabled until startup completes."
    );
}

// ---------------------------------------------------------------- notes
// What a command line answered with is a row of the transcript (oracle
// `add_info_message`, `add_error_message`): it keeps its place when the
// conversation goes on, it is not a line under the composer.

fn at(screen: &str, text: &str) -> usize {
    screen
        .lines()
        .position(|row| row.contains(text))
        .unwrap_or_else(|| panic!("no row with `{text}`:\n{screen}"))
}

#[test]
fn a_command_answer_is_a_row_of_the_transcript_and_stays_above_what_follows() {
    use codex_tui_core::fixtures::builders::{agent_completed, agent_delta, turn_completed};
    use codex_tui_rtwk::services::backend::fake::FAKE_THREAD;
    let mut h = chat();
    send(&mut h, "/raw maybe");
    h.wait_for("the usage note", |s| s.contains("■ Usage: /raw [on|off]"));
    h.fake.script_turn(vec![
        agent_delta(FAKE_THREAD, "turn-2", "msg-2", "Later"),
        agent_completed(FAKE_THREAD, "turn-2", "msg-2", "Later"),
        turn_completed(FAKE_THREAD, "turn-2"),
    ]);
    send(&mut h, "next");
    h.wait_for("the later answer", |s| s.contains("• Later"));
    let screen = h.t.screen();
    assert!(
        at(&screen, "• Hello") < at(&screen, "■ Usage: /raw [on|off]")
            && at(&screen, "■ Usage: /raw [on|off]") < at(&screen, "› next"),
        "{screen}"
    );
}

#[test]
fn an_information_note_is_a_bullet_row_and_not_an_error() {
    let mut h = chat();
    send(&mut h, "!");
    h.wait_for("the shell help", |s| {
        s.contains("Prefix a command with ! to run it locally")
    });
    let screen = h.t.screen();
    assert!(
        !screen.contains("■ Prefix a command"),
        "an information note has no error mark:\n{screen}"
    );
    assert!(
        screen.contains("• Prefix a command with ! to run it locally"),
        "{screen}"
    );
}

#[test]
fn a_command_refused_while_a_task_runs_runs_once_and_keeps_its_line_for_recall() {
    let mut h = start();
    h.wait_idle();
    h.fake.script_turn(vec![]);
    send(&mut h, "work");
    h.wait_for("working", |s| s.contains("Working"));
    send(&mut h, "/clear");
    let message = "'/clear' is disabled while a task is in progress.";
    h.wait_for(message, |s| s.contains(message));
    let screen = h.t.screen();
    assert_eq!(
        screen.matches(message).count(),
        1,
        "the line ran more than once:\n{screen}"
    );
    // the composer keeps the line, to be corrected, and recall has it too
    assert!(screen.contains("› /clear"), "the draft is kept:\n{screen}");
    h.press("ctrl+u");
    h.press("up");
    h.t.pump_for(ms(100));
    assert!(
        h.t.screen().contains("› /clear"),
        "recall lost the line:\n{}",
        h.t.screen()
    );
}

#[test]
fn raw_toggles_the_mode_and_says_which_one_it_is_now() {
    let mut h = chat();
    send(&mut h, "/raw");
    h.wait_for("raw on", |s| s.contains("• Raw output mode on:"));
    send(&mut h, "/raw");
    h.wait_for("raw off", |s| s.contains("• Raw output mode off:"));
    send(&mut h, "/raw on");
    h.wait_for("raw on again", |s| {
        s.matches("• Raw output mode on:").count() == 2
    });
}

#[test]
fn init_sends_the_prompt_as_a_message_and_clears_the_composer() {
    let mut h = chat();
    h.fake.script_turn(vec![]);
    send(&mut h, "/init");
    // The transcript follows the tail of the long prompt.
    h.wait_for("the prompt", |s| {
        s.contains("State test naming conventions")
    });
    assert!(!h.t.screen().contains("› /init"), "{}", h.t.screen());
}

#[test]
fn cd_starts_the_session_afresh_in_the_directory() {
    let mut h = start();
    h.wait_idle();
    send(&mut h, "/cd /tmp");
    h.wait_until("thread/start in /tmp", |h| {
        h.fake.commands().iter().any(|c| {
            matches!(
                c,
                codex_tui_core::app::Command::Screen {
                    command: codex_tui_core::screens::ScreenCommand::Agents(
                        codex_tui_core::screens::agents::Command::Thread(
                            codex_tui_core::threads::ThreadAsk::Start { cwd: Some(cwd) }
                        )
                    ),
                    ..
                } if cwd == "/tmp"
            )
        })
    });
}

#[test]
fn cd_in_a_conversation_with_history_says_what_it_cannot_carry_over() {
    let mut h = chat();
    send(&mut h, "/cd /tmp");
    h.wait_for("the refusal", |s| {
        s.contains("■ Conversation history is not saved.")
    });
    assert!(
        !h.fake
            .requests()
            .iter()
            .any(|r| r["method"] == "thread/start" && r["params"]["cwd"] == "/tmp"),
        "{:?}",
        h.fake.requests()
    );
}

/// The tickets of the removals the app asked of the server (`/archive`, `/delete`).
fn removals(h: &Harness) -> Vec<(codex_tui_core::ids::Ticket, agents::Command)> {
    h.fake
        .commands()
        .into_iter()
        .filter_map(|command| match command {
            Command::Screen {
                ticket,
                command: ScreenCommand::Agents(command),
                ..
            } if matches!(
                command,
                agents::Command::Archive { .. } | agents::Command::Delete { .. }
            ) =>
            {
                Some((ticket, command))
            }
            _ => None,
        })
        .collect()
}

fn answer(h: &mut Harness, ticket: codex_tui_core::ids::Ticket, reply: AppReply) {
    let epoch = h.fake.epoch().expect("connected");
    h.fake.inject(epoch, AppEv::Reply(ticket, reply));
}

/// The cell of `needle` on the screen (x, y).
fn find(h: &Harness, needle: &str) -> (u16, u16) {
    let screen = h.t.screen();
    for (y, line) in screen.lines().enumerate() {
        if let Some(byte) = line.find(needle) {
            return (line[..byte].chars().count() as u16, y as u16);
        }
    }
    panic!("`{needle}` is not on the screen:\n{screen}");
}

#[test]
fn a_click_on_yes_in_the_confirmation_asks_the_server_to_archive() {
    let mut h = chat();
    send(&mut h, "/archive");
    h.wait_for("the confirmation", |s| s.contains("Archive this session?"));
    h.settle();
    assert!(removals(&h).is_empty(), "the question alone asks nothing");
    let (x, y) = find(&h, "Yes, archive");
    h.t.click(x, y);
    h.settle();
    let asked = removals(&h);
    assert_eq!(asked.len(), 1, "{:?}", h.fake.commands());
    assert!(
        matches!(asked[0].1, agents::Command::Archive { .. }),
        "{:?}",
        asked[0]
    );
    assert!(
        !h.t.screen().contains("Archive this session?"),
        "{}",
        h.t.screen()
    );
}

#[test]
fn a_click_on_no_in_the_confirmation_asks_nothing() {
    let mut h = chat();
    send(&mut h, "/delete");
    h.wait_for("the confirmation", |s| s.contains("Delete this session?"));
    h.settle();
    let (x, y) = find(&h, "No, keep this session");
    h.t.click(x, y);
    h.settle();
    assert!(removals(&h).is_empty(), "{:?}", h.fake.commands());
    assert!(
        !h.t.screen().contains("Delete this session?"),
        "{}",
        h.t.screen()
    );
}

#[test]
fn the_server_removing_the_thread_opens_the_command_center() {
    let mut h = chat();
    send(&mut h, "/delete");
    h.wait_for("the confirmation", |s| s.contains("Delete this session?"));
    h.settle();
    let (x, y) = find(&h, "Yes, delete and return to command center");
    h.t.click(x, y);
    h.settle();
    let (ticket, command) = removals(&h).remove(0);
    assert!(matches!(command, agents::Command::Delete { .. }));
    answer(
        &mut h,
        ticket,
        AppReply::screen(ScreenReply::Agents(agents::Reply::Done)),
    );
    h.wait_for("the command center", |s| s.contains("Agent command center"));
}

#[test]
fn a_refusal_of_the_server_is_said_in_the_chat_and_the_chat_stays() {
    let mut h = chat();
    send(&mut h, "/archive");
    h.wait_for("the confirmation", |s| s.contains("Archive this session?"));
    h.settle();
    let (x, y) = find(&h, "Yes, archive");
    h.t.click(x, y);
    h.settle();
    let (ticket, _) = removals(&h).remove(0);
    answer(&mut h, ticket, AppReply::failed("denied"));
    h.wait_for("the refusal", |s| {
        s.contains("Failed to archive current thread: failed to archive session")
    });
    assert!(
        !h.t.screen().contains("Agent command center"),
        "{}",
        h.t.screen()
    );
}

/// `/cd` into a folder whose trust is undecided: the oracle's question over the chat
/// (`app/working_directory.rs:262-292`), asked of the host and answered by hand here (the
/// scripted server reads no trust).
mod consent {
    use codex_tui_core::app::{AppEv, Command};
    use codex_tui_core::screens::onboarding::consent::{Checked, Target};
    use codex_tui_core::screens::onboarding::{Ask, Ev, TrustAsk, TrustCancel};
    use codex_tui_core::screens::{ScreenCommand, ScreenEv, agents};
    use codex_tui_core::threads::ThreadAsk;

    use crate::common::{Harness, ms, start_app};

    fn armed() -> Harness {
        let mut h = start_app(|app| app.with_folder_consent(true));
        h.wait_idle();
        h
    }

    fn type_cd(h: &mut Harness, line: &str) {
        h.t.type_text(line);
        h.press("enter");
        h.t.pump_for(ms(400));
    }

    fn inject(h: &Harness, ev: Ev) {
        let Some(epoch) = h.fake.epoch() else {
            panic!("the app is not attached to the scripted server");
        };
        h.fake
            .inject(epoch, AppEv::Screen(ScreenEv::Onboarding(ev)));
    }

    fn startup_asks(h: &Harness) -> Vec<Ask> {
        h.fake
            .commands()
            .into_iter()
            .filter_map(|command| match command {
                Command::Startup { ask, .. } => Some(ask),
                _ => None,
            })
            .collect()
    }

    /// The request number of the trust read the app asked of the host.
    fn read_request(h: &mut Harness) -> u64 {
        h.wait_until("the trust read", |h| {
            startup_asks(h)
                .iter()
                .any(|ask| matches!(ask, Ask::CheckConsent { .. }))
        });
        startup_asks(h)
            .into_iter()
            .find_map(|ask| match ask {
                Ask::CheckConsent { request, .. } => Some(request),
                _ => None,
            })
            .expect("a trust read")
    }

    fn question() -> TrustAsk {
        TrustAsk {
            cwd: "/tmp".into(),
            target: "/tmp".into(),
            cancel: TrustCancel::CurrentTask,
            ..TrustAsk::default()
        }
    }

    /// The question on screen: `/cd /tmp` typed, the host's answer injected.
    fn asked() -> Harness {
        let mut h = armed();
        type_cd(&mut h, "/cd /tmp");
        let request = read_request(&mut h);
        let ask = Some(question());
        inject(&h, Ev::Consent { request, ask });
        h.wait_for("the question", |s| s.contains("Folder access"));
        h
    }

    /// The session was asked to start in /tmp (the command, or the request it became).
    fn started_in_tmp(h: &Harness) -> bool {
        let asked = h.fake.commands().iter().any(|c| {
            matches!(
                c,
                Command::Screen {
                    command: ScreenCommand::Agents(agents::Command::Thread(
                        ThreadAsk::Start { cwd: Some(cwd) }
                    )),
                    ..
                } if cwd == "/tmp"
            )
        });
        asked
            || h.fake
                .requests()
                .iter()
                .any(|r| r["method"] == "thread/start" && r["params"]["cwd"] == "/tmp")
    }

    #[test]
    fn cd_into_an_undecided_folder_shows_the_trust_page_over_the_chat() {
        let h = asked();
        let screen = h.t.screen();
        assert!(screen.contains("› 1. Trust and continue"), "{screen}");
        assert!(screen.contains("2. Keep current directory"), "{screen}");
        assert!(!started_in_tmp(&h), "nothing starts before the answer");
    }

    #[test]
    fn trusting_the_folder_starts_the_session_there() {
        let mut h = asked();
        h.press("enter");
        h.wait_until("the trust write", |h| {
            startup_asks(h)
                .iter()
                .any(|ask| matches!(ask, Ask::WriteTrust { .. }))
        });
        assert!(!started_in_tmp(&h), "the thread waits for the write");
        let write = startup_asks(&h)
            .into_iter()
            .find_map(|ask| match ask {
                Ask::WriteTrust { request, target } => Some((request, target)),
                _ => None,
            })
            .expect("a write");
        assert_eq!(write.1, "/tmp");
        inject(&h, Ev::TrustSaved { request: write.0 });
        h.wait_until("thread/start in /tmp", |h| started_in_tmp(h));
        h.wait_for("the page gone", |s| !s.contains("Folder access"));
    }

    #[test]
    fn cancelling_a_new_tasks_trust_prompt_returns_to_the_center() {
        let mut h = start_app(|app| app.with_start_agents(true).with_folder_consent(true));
        h.wait_for("the command center", |s| s.contains("Agent command center"));
        h.press("n");
        let request = read_request(&mut h);
        inject(
            &h,
            Ev::Consent {
                request,
                ask: Some(TrustAsk {
                    cancel: TrustCancel::AgentsOverview,
                    ..question()
                }),
            },
        );
        h.wait_for("the trust prompt", |s| {
            s.contains("Back to Agent Command Center")
        });
        h.press("esc");
        h.wait_for("the command center again", |s| {
            s.contains("Agent command center") && !s.contains("Folder access")
        });
        h.settle();
        assert!(
            h.fake
                .requests()
                .iter()
                .all(|r| r["method"] != "thread/start")
        );
    }

    fn opening_saved_task() -> (Harness, u64) {
        let mut h = armed();
        let id = "00000000-0000-0000-0000-000000000001";
        let saved = serde_json::json!({
            "id": id, "environments": null, "extra": null, "sessionId": id,
            "forkedFromId": null, "attachedToThreadId": null, "parentThreadId": null,
            "preview": "Saved task to open", "ephemeral": false, "section": null,
            "sectionEnteredAt": null, "projectId": null, "historyMode": "legacy",
            "modelProvider": "openai", "model": null, "reasoningEffort": null,
            "createdAt": 1_777_392_000, "updatedAt": 1_777_395_600, "recencyAt": 1_777_395_600,
            "status": {"type": "idle"}, "path": null, "cwd": "/workspace/first",
            "cliVersion": "0.0.0", "originator": null, "source": "cli",
            "canAcceptDirectInput": true, "threadSource": null, "agentNickname": null,
            "agentRole": null, "gitInfo": null, "name": "Saved task to open",
            "daybreakEnabled": null, "turns": []
        });
        h.fake.respond(move |request| {
            if request["method"] != "thread/list" {
                return None;
            }
            let rows = if request["params"]["sectionId"].is_null() {
                vec![saved.clone()]
            } else {
                Vec::new()
            };
            Some(Ok(serde_json::json!({
                "data": rows, "nextCursor": null, "backwardsCursor": null
            })))
        });
        h.press("left");
        h.wait_for("the command center", |s| s.contains("Agent command center"));
        h.wait_for("the saved task", |s| s.contains("Saved task to open"));
        h.press("enter");
        let request = read_request(&mut h);
        (h, request)
    }

    fn resumes(h: &Harness) -> bool {
        h.fake
            .requests()
            .iter()
            .any(|request| request["method"] == "thread/resume")
    }

    #[test]
    fn leaving_the_center_during_the_initial_trust_read_cancels_the_open() {
        let (mut h, request) = opening_saved_task();
        h.press("esc");
        h.wait_for("the chat", |s| !s.contains("Agent command center"));
        inject(&h, Ev::Consent { request, ask: None });
        h.settle();
        assert!(!resumes(&h));
        assert!(!h.t.screen().contains("Folder access"));
    }

    #[test]
    fn leaving_the_center_during_a_new_tasks_trust_read_starts_nothing() {
        let mut h = armed();
        h.press("left");
        h.wait_for("the command center", |s| s.contains("Agent command center"));
        let starts = |h: &Harness| {
            h.fake
                .requests()
                .iter()
                .filter(|request| request["method"] == "thread/start")
                .count()
        };
        let before = starts(&h);
        h.press("n");
        let request = read_request(&mut h);
        h.press("esc");
        h.wait_for("the chat", |s| !s.contains("Agent command center"));
        inject(&h, Ev::Consent { request, ask: None });
        h.settle();
        assert_eq!(starts(&h), before);
    }

    fn daemon_prompt(request: u64, cwd: &str) -> Ev {
        Ev::ConsentAt {
            request,
            ask: Some(TrustAsk {
                cwd: cwd.into(),
                target: cwd.into(),
                existing_task: true,
                cancel: TrustCancel::AgentsOverview,
                ..TrustAsk::default()
            }),
            checked: Checked::Task(cwd.into()),
        }
    }

    #[test]
    fn a_changed_daemon_folder_updates_the_same_consent_route() {
        let (mut h, first) = opening_saved_task();
        inject(&h, daemon_prompt(first, "/workspace/first"));
        h.wait_for("the first folder", |s| {
            s.contains("Folder access") && s.contains("/workspace/first")
        });
        h.press("enter");
        h.wait_until("the trust write", |h| {
            startup_asks(h)
                .iter()
                .any(|ask| matches!(ask, Ask::WriteTrust { .. }))
        });
        let write = startup_asks(&h)
            .into_iter()
            .find_map(|ask| match ask {
                Ask::WriteTrust { request, .. } => Some(request),
                _ => None,
            })
            .expect("the write");
        inject(&h, Ev::TrustSaved { request: write });
        h.wait_until("the cwd recheck", |h| {
            startup_asks(h).iter().any(|ask| {
                matches!(
                    ask,
                    Ask::CheckConsent {
                        target: Target::TaskRecheck { .. },
                        ..
                    }
                )
            })
        });
        let route =
            rtwk_test::find::by_type::<codex_tui_rtwk::pages::startup::onboarding::StartupPage>();
        assert_eq!(route.count(&h.t), 1);
        let second = startup_asks(&h)
            .into_iter()
            .find_map(|ask| match ask {
                Ask::CheckConsent {
                    request,
                    target: Target::TaskRecheck { .. },
                    ..
                } => Some(request),
                _ => None,
            })
            .expect("the recheck");
        inject(&h, daemon_prompt(second, "/workspace/second"));
        h.wait_for("the changed folder", |s| {
            s.contains("Folder access") && s.contains("/workspace/second")
        });
        assert_eq!(route.count(&h.t), 1);
        h.press("esc");
        h.wait_for("the command center again", |s| {
            s.contains("Agent command center")
        });
        assert!(!resumes(&h));
    }

    #[test]
    fn cancelling_the_fresh_session_prompt_opens_the_center() {
        let mut h = start_app(|app| app.with_start_resume(true).with_folder_consent(true));
        h.wait_for("the resume picker", |s| {
            s.contains("Resume a previous session")
        });
        h.press("esc");
        let request = read_request(&mut h);
        inject(
            &h,
            Ev::Consent {
                request,
                ask: Some(TrustAsk {
                    cancel: TrustCancel::AgentsOverview,
                    ..question()
                }),
            },
        );
        h.wait_for("the trust prompt", |s| s.contains("Folder access"));
        h.press("esc");
        h.wait_for("the command center", |s| {
            s.contains("Agent command center") && !s.contains("Folder access")
        });
        h.settle();
        assert!(
            h.fake
                .requests()
                .iter()
                .all(|r| r["method"] != "thread/start")
        );
    }

    #[test]
    fn keep_current_directory_returns_to_the_chat_and_starts_nothing() {
        for keys in [&["down", "enter"][..], &["esc"][..]] {
            let mut h = asked();
            for key in keys {
                h.press(key);
            }
            h.wait_for("the chat again", |s| !s.contains("Folder access"));
            h.t.pump_for(ms(300));
            assert!(!started_in_tmp(&h), "{keys:?}");
            assert!(
                !startup_asks(&h)
                    .iter()
                    .any(|ask| matches!(ask, Ask::WriteTrust { .. })),
                "{keys:?}"
            );
            h.t.type_text("hello");
            h.t.pump_for(ms(200));
            assert!(h.t.screen().contains("› hello"), "{}", h.t.screen());
        }
    }
}
