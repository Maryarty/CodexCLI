//! Screen tests of S19 (the `/status` card): the card is a cell of the thread's
//! transcript. Oracle tests: `status/tests.rs`, `status/helpers.rs`
//! (`format_tokens_compact`).

use crate::common::{Harness, ms, start_clipping, start_sized};

/// `/status` is a command of the session (oracle: `slash_dispatch.rs`,
/// `SlashCommand::Status`): the card is a cell of the transcript, put there
/// through the thread's `apply` (ADR-003), never a dialog over the chat.
fn chat_with_status() -> Harness {
    let mut h = start_sized(false, 100, 40);
    crate::chat::chat_with_history(&mut h);
    h.t.type_text("/status");
    h.press("enter");
    h.wait_for("the status card", |s| s.contains(">_ OpenAI Codex"));
    h.wait_idle();
    h
}

#[test]
fn slash_status_puts_the_card_into_the_transcript_and_opens_no_dialog() {
    let h = chat_with_status();
    let screen = h.t.screen();
    assert!(
        screen.contains("• Hello"),
        "the transcript above it:\n{screen}"
    );
    assert!(
        screen.contains("/status"),
        "the command that made it:\n{screen}"
    );
    assert!(screen.contains("Model:"), "{screen}");
    assert!(screen.contains("Directory:"), "{screen}");
    // The chat is still the chat: the composer takes the next line.
    assert!(
        !screen.contains("esc to close") && !screen.contains("Esc to close"),
        "no dialog over the chat:\n{screen}"
    );
}

/// The card scrolls away with the transcript (it is a cell, not an overlay):
/// a later turn is below it and Escape closes nothing.
#[test]
fn the_card_stays_in_the_transcript_when_the_chat_goes_on() {
    let mut h = chat_with_status();
    h.press("esc");
    h.t.pump_for(ms(400));
    assert!(h.t.screen().contains(">_ OpenAI Codex"), "{}", h.t.screen());
}

// ---- the cell (oracle `status/card.rs`): what the width decides ----------------------------

fn open_cell(entry: &str, width: u16, height: u16) -> Harness {
    let mut h = start_clipping(true, width, height);
    h.open_debug_row("status", Some(entry));
    h.t.pump_for(ms(800));
    h
}

/// A value track with room shows the bar, the share left and the reset on one
/// row (oracle `rate_limit_row_lines`).
#[test]
fn a_limit_shows_its_bar_when_the_value_track_has_room() {
    let h = open_cell("enterprise_monthly_credit", 92, 40);
    let screen = h.t.screen();
    assert!(
        screen.contains(
            "Monthly credit limit:  [██████████████░░░░░░] 68% left (resets 07:08 on 7 May)"
        ),
        "{screen}"
    );
}

/// On a narrow value track the share left stays and the bar gives way; the
/// reset goes under it (the oracle keeps the percentage visible rather than
/// let the fixed-width bar crowd out the reset time).
#[test]
fn a_narrow_value_track_drops_the_bar_and_keeps_the_percentage() {
    let h = open_cell("enterprise_monthly_credit", 46, 40);
    let screen = h.t.screen();
    assert!(
        screen.contains("Monthly credit limit:  68% left"),
        "{screen}"
    );
    assert!(!screen.contains('█'), "{screen}");
    assert!(screen.contains("(resets 07:08 on"), "{screen}");
}

/// A terminal narrower than the label column has no room for a value beside
/// its label: each row flows as one paragraph under the indent.
#[test]
fn a_terminal_narrower_than_the_labels_flows_the_rows() {
    let h = open_cell("narrow_terminal", 12, 80);
    let screen = h.t.screen();
    assert!(screen.contains("Model:"), "{screen}");
    assert!(screen.contains("Directory:"), "{screen}");
}

/// The cell shows the command that made it, the title and the usage link.
#[test]
fn the_cell_shows_the_command_the_title_and_the_usage_link() {
    let h = open_cell("server_auth_required", 120, 24);
    let screen = h.t.screen();
    assert!(screen.contains("/status"), "{screen}");
    assert!(screen.contains(">_ OpenAI Codex (v0.0.0)"), "{screen}");
    assert!(
        screen.contains("Visit https://chatgpt.com/settings/usage for up-to-date"),
        "{screen}"
    );
    assert!(
        screen.contains("information on rate limits and credits"),
        "{screen}"
    );
}
