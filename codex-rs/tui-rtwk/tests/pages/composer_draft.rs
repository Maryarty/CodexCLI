// test root: pages
//! The composer's draft against the thread: shell mode and the history walk
//! end where the composer clears (the thread took the submit, a command took
//! the line, Ctrl+C), not where Enter was pressed, and a paste at the start
//! that opens with `!` is shell mode (`insert_str_at_target`). The oracle's
//! composer clears on its own submit; here the thread decides, so the draft
//! of a submit it did not take has to stay as it was.

use crate::common;

use codex_tui_core::app::AppEv;
use codex_tui_core::fixtures::builders::{agent_completed, agent_delta, turn_completed};
use codex_tui_rtwk::services::backend::fake::FAKE_THREAD;

use common::{Harness, start};

/// The composer's prompt row (`›` or the shell's `!`), the last one.
fn prompt(h: &Harness) -> String {
    h.t.screen()
        .lines()
        .rfind(|row| row.starts_with('›') || row.starts_with('!'))
        .unwrap_or_default()
        .trim_end()
        .to_string()
}

/// A chat whose session is up and idle, the composer empty.
fn chat() -> Harness {
    let mut h = start();
    h.wait_idle();
    h
}

#[test]
fn a_shell_draft_the_thread_did_not_take_stays_a_shell_draft() {
    let mut h = chat();
    // the connection is lost: the thread has no writer, a submit is no-op
    h.inject([AppEv::Disconnected]);
    h.t.type_text("!ls");
    h.advance(100);
    assert_eq!(prompt(&h), "! ls");
    h.press("enter");
    assert_eq!(prompt(&h), "! ls", "the draft is still the shell line");
    // and it stays one while it is edited on
    h.t.type_text("x");
    h.advance(100);
    assert_eq!(prompt(&h), "! lsx");
}

/// The `thread/shellCommand` requests that went out, their `command`.
fn shell_commands(h: &Harness) -> Vec<String> {
    h.fake
        .requests()
        .iter()
        .filter(|r| r["method"] == "thread/shellCommand")
        .filter_map(|r| r["params"]["command"].as_str().map(str::to_owned))
        .collect()
}

/// Waits until the fake has the shell command (it travels to the runtime
/// thread: an idle session does not say it arrived).
fn sent(h: &mut Harness) {
    common::wait::wait_until(h, "the shell command", |h| !shell_commands(h).is_empty());
}

#[test]
fn a_shell_draft_the_thread_took_clears_and_leaves_shell_mode() {
    let mut h = chat();
    h.fake.respond(|request| {
        (request["method"] == "thread/shellCommand").then(|| Ok(serde_json::json!({})))
    });
    h.t.type_text("!ls");
    h.press("enter");
    sent(&mut h);
    // the line went out as a user shell command, not as a turn
    assert_eq!(shell_commands(&h), vec!["ls".to_string()]);
    assert!(
        !h.fake
            .commands()
            .iter()
            .any(|c| matches!(c, codex_tui_core::app::Command::StartTurn { .. })),
        "a shell line starts no turn"
    );
    // the prompt is the plain one again: the next text is not a shell line
    h.t.type_text("x");
    h.advance(100);
    assert_eq!(prompt(&h), "› x");
}

#[test]
fn a_bang_slash_line_is_a_shell_command_not_the_slash_command() {
    let mut h = chat();
    h.fake.respond(|request| {
        (request["method"] == "thread/shellCommand").then(|| Ok(serde_json::json!({})))
    });
    h.t.type_text("!/diff");
    h.press("enter");
    sent(&mut h);
    assert_eq!(shell_commands(&h), vec!["/diff".to_string()]);
}

#[test]
fn a_bang_after_the_start_is_part_of_the_prompt() {
    let mut h = chat();
    h.fake.script_turn(vec![
        agent_delta(FAKE_THREAD, "turn-1", "msg-1", "Done"),
        agent_completed(FAKE_THREAD, "turn-1", "msg-1", "Done"),
        turn_completed(FAKE_THREAD, "turn-1"),
    ]);
    h.t.paste("hello\n!ls");
    h.advance(100);
    h.press("enter");
    h.wait_for("the answer", |s| s.contains("• Done"));
    assert!(
        shell_commands(&h).is_empty(),
        "text with a bang inside is a prompt"
    );
}

/// Sends `text` and waits for the scripted answer `turn`.
fn ask(h: &mut Harness, text: &str, turn: &str) {
    let (msg, answer) = (format!("msg-{turn}"), format!("answer {turn}"));
    h.fake.script_turn(vec![
        agent_delta(FAKE_THREAD, turn, &msg, &answer),
        agent_completed(FAKE_THREAD, turn, &msg, &answer),
        turn_completed(FAKE_THREAD, turn),
    ]);
    h.t.type_text(text);
    h.press("enter");
    h.wait_for("the answer", |s| s.contains(&format!("• {answer}")));
    h.wait_idle();
}

#[test]
fn a_pasted_bang_at_the_start_is_shell_mode() {
    let mut h = chat();
    h.t.paste("!ls -la");
    h.advance(100);
    assert_eq!(prompt(&h), "! ls -la");
}

#[test]
fn a_pasted_bang_after_text_stays_text() {
    let mut h = chat();
    h.t.type_text("echo ");
    h.advance(100);
    h.t.paste("!ls");
    h.advance(100);
    assert_eq!(prompt(&h), "› echo !ls");
}

/// The composer of an offline chat (`reconnect.offline`): the draft stays
/// editable, Enter runs only what needs no connection.
mod offline {
    use super::*;
    use codex_tui_core::screens::ScreenEv;
    use codex_tui_core::screens::agents::Ev;

    fn offline() -> Harness {
        let mut h = chat();
        h.inject([AppEv::Screen(ScreenEv::Agents(Ev::Reconnecting))]);
        h
    }

    fn big() -> String {
        "x".repeat(1001)
    }

    /// `reconnect_expands_pastes_preserving_images_and_cursor` through the
    /// page: the paste shows what Enter would run at once.
    #[test]
    fn a_paste_is_expanded_where_it_lands() {
        let mut h = offline();
        h.t.paste(&big());
        h.advance(100);
        assert!(h.t.screen().contains(&"x".repeat(60)));
        assert!(!h.t.screen().contains("[Pasted Content"));
        // the caret is after the text: the key lands there
        h.t.type_text("!");
        h.advance(100);
        assert!(h.t.screen().lines().any(|row| row.trim() == "!"));
    }

    /// The same paste before the connection was lost is a placeholder, and
    /// the loss expands it.
    #[test]
    fn the_loss_of_the_connection_expands_a_placeholder_already_there() {
        let mut h = chat();
        h.t.paste(&big());
        h.advance(100);
        assert!(h.t.screen().contains("[Pasted Content"));
        h.inject([AppEv::Screen(ScreenEv::Agents(Ev::Reconnecting))]);
        h.advance(100);
        assert!(!h.t.screen().contains("[Pasted Content"));
    }

    /// `reconnect_edit_removes_deleted_attachment_and_paste_metadata`.
    #[test]
    fn ctrl_u_removes_the_draft_and_its_paste() {
        let mut h = offline();
        h.t.paste(&big());
        h.press("ctrl+u");
        h.advance(100);
        assert_eq!(prompt(&h), "› Ask Codex to do anything");
        assert!(!h.t.screen().contains("xxxxxxxx"));
    }

    #[test]
    fn enter_and_tab_edit_nothing_and_a_prompt_stays_in_the_draft() {
        let mut h = offline();
        h.t.type_text("hello");
        h.press("enter");
        h.press("tab");
        h.advance(100);
        assert_eq!(prompt(&h), "› hello");
        assert!(
            h.fake
                .commands()
                .iter()
                .all(|c| !matches!(c, codex_tui_core::app::Command::StartTurn { .. })),
            "no turn went out"
        );
    }

    #[test]
    fn delete_edits_the_offline_row_without_moving_the_caret() {
        use codex_tui_rtwk::app::UiApp;
        let mut h = offline();
        h.t.paste("line 19 abc\nline 20 abc");
        h.advance(100);
        h.press("left");
        h.press("left");
        let draft = |h: &Harness| {
            h.t.read_app(|app: &UiApp| {
                let editor = app.ui.thread_ui.get(app.core.session().active.as_ref());
                (
                    editor.composer.text(),
                    editor.composer.selection().extent_offset,
                )
            })
        };
        let (text, caret) = draft(&h);
        for key in ["enter", "tab"] {
            h.press(key);
            assert_eq!(draft(&h), (text.clone(), caret));
        }
        h.press("delete");
        let (edited, position) = draft(&h);
        assert_eq!(edited.as_ref(), "line 19 abc\nline 20 ac");
        assert_eq!(position, caret);
    }

    /// `restricted_input_preserves_blocked_drafts`: a command that needs the
    /// connection stays in the draft, with no popup over it.
    #[test]
    fn a_command_that_needs_the_connection_stays_in_the_draft() {
        let mut h = offline();
        h.t.type_text("/status");
        h.advance(100);
        h.press("enter");
        h.advance(100);
        assert_eq!(prompt(&h), "› /status");
    }

    /// Up and Down are the caret's: no history walk.
    #[test]
    fn up_is_no_history_walk() {
        let mut h = chat();
        ask(&mut h, "one", "turn-1");
        h.inject([AppEv::Screen(ScreenEv::Agents(Ev::Reconnecting))]);
        h.press("up");
        assert_eq!(prompt(&h), "› Ask Codex to do anything");
    }

    /// Back online the composer is whole again: Up walks, Enter submits.
    #[test]
    fn the_composer_is_whole_again_when_the_connection_is_back() {
        let mut h = chat();
        ask(&mut h, "one", "turn-1");
        h.inject([AppEv::Screen(ScreenEv::Agents(Ev::Reconnecting))]);
        h.inject([AppEv::Screen(ScreenEv::Agents(Ev::Reconnected))]);
        h.press("up");
        assert_eq!(prompt(&h), "› one");
    }

    /// `disconnected_input_dispatches_local_config_command`: `/warnings`
    /// runs offline and takes the line.
    #[test]
    fn a_local_command_runs_and_takes_the_line() {
        let mut h = offline();
        h.t.type_text("/warnings");
        h.advance(100);
        h.press("enter");
        h.advance(200);
        assert!(!h.t.screen().contains("› /warnings"), "the line went");
    }
}
