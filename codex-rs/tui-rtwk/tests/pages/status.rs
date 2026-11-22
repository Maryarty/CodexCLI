//! Screen tests of S19 (the `/status` card). Expected text: the oracle's
//! snapshot (`dialogs::oracle`). Oracle tests: `status/tests.rs`
//! (`status_snapshot_includes_reasoning_details`), `status/helpers.rs`
//! (`format_tokens_compact`).

use crate::common::{Harness, ms, start_clipping, start_sized};
use crate::dialogs::oracle::assert_rows;

fn open(entry: &str) -> Harness {
    let mut h = start_sized(true, 100, 24);
    h.open_debug_row("status", Some(entry));
    h.t.pump_for(ms(800));
    h
}

#[test]
fn status_snapshot_includes_reasoning_details() {
    let h = open("card_dialog");
    assert_rows(
        &h.t.screen(),
        "status/snapshots/codex_tui__status__tests__status_snapshot_includes_reasoning_details.snap",
    );
}

#[test]
fn esc_closes_the_card() {
    let mut h = open("card_dialog");
    h.press("esc");
    h.t.pump_for(ms(800));
    assert!(!h.t.screen().contains("OpenAI Codex"), "{}", h.t.screen());
}

/// The limit bar (G40): the oracle draws `[██████░░░░░░░░░░░░░░]` for 28% left,
/// 20 whole cells of `█` and `░` between brackets (`render_limit_progress_bar`);
/// a `CustomPaint` in the card.
#[test]
fn limit_bar_is_a_custom_paint_of_whole_cells() {
    let h = open("card_dialog");
    assert!(
        h.t.screen().contains("[██████░░░░░░░░░░░░░░] 28% left"),
        "{}",
        h.t.screen()
    );
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
