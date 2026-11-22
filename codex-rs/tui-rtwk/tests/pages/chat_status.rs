//! Tests of the chat's footer and status surfaces (S10; oracle
//! `bottom_pane/footer.rs`, `chat_composer/passive_hints.rs`,
//! `chat_composer/status_surface.rs`): what the shards do not say (state
//! and mouse behaviour), and the footer entries one by one while the shard's
//! `footer_snapshots` waits for the entries that need the status line.

use crate::kit::{Owner, Screen};

const IDLE: &str = include_str!("../../src/pages/session/footer/fixtures/footer_idle.jsonl");
const RUNNING: &str = include_str!("../../src/pages/session/footer/fixtures/footer_running.jsonl");

/// The idle session with three retained start-up warnings.
fn warned() -> &'static str {
    static SCRIPT: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    SCRIPT.get_or_init(|| {
        let entries: Vec<String> = (1..=3)
            .map(|i| format!(r#"{{"source":"Startup","details":"warning {i}"}}"#))
            .collect();
        format!(
            "{IDLE}{}\n",
            format_args!(
                r#"{{"step":"screen","dir":"dialogs","ev":{{"kind":"retained","entries":[{}]}}}}"#,
                entries.join(",")
            )
        )
    })
}

fn page(script: &'static str, width: u16, height: u16) -> Screen {
    Screen::build(Owner::UiPane)
        .size(width, height)
        .script(script)
        .open()
}

/// One footer snapshot of the oracle's `footer_snapshots`, drawn by the real
/// footer over the model its fixture script makes.
fn footer_matches(name: &'static str, width: u16) {
    let s = Screen::build(Owner::UiPane)
        .size(width, 1)
        .entry("chat", name)
        .open();
    s.assert_snap(name);
}

/// The footers whose facts the model has: hints, the queue hint, the context
/// meter in both spellings, and the transient lines.
#[test]
fn the_footer_entries_equal_the_oracle_snapshots() {
    for name in [
        "footer_shortcuts_default",
        "footer_shortcuts_context_running",
        "footer_context_tokens_used",
        "footer_composer_has_draft_queue_hint_enabled",
        "footer_ctrl_c_quit_idle",
        "footer_esc_hint_idle",
        "footer_esc_hint_primed",
    ] {
        footer_matches(name, 80);
    }
}

/// The footer's labels are the page's keys (ADR-011): a click on the hint
/// presses the key it names.
#[test]
fn a_click_on_the_shortcuts_hint_opens_the_shortcuts() {
    let mut s = page(IDLE, 80, 24);
    s.assert_contains("? for shortcuts");
    s.click_text("? for shortcuts");
    s.assert_contains("Keyboard shortcuts");
}

#[test]
fn a_click_on_the_queue_hint_queues_the_draft() {
    let mut s = page(RUNNING, 80, 24);
    s.type_text("later");
    s.assert_contains("tab to queue message");
    s.click_text("tab to queue message");
    s.assert_contains("(queued) later");
    s.assert_absent("tab to queue message");
}

#[test]
fn a_click_on_the_warning_notice_opens_the_warnings() {
    let mut s = page(warned(), 80, 24);
    s.assert_contains("⚠ 3 warnings · f2 to view");
    s.click_text("3 warnings");
    s.assert_contains("warning 1");
}

/// With a draft in the composer the footer carries no shortcut hints (the
/// oracle's `ComposerHasDraft`), and keeps its row.
#[test]
fn a_draft_takes_the_shortcut_hints_away_and_the_row_stays() {
    let mut s = page(IDLE, 80, 12);
    s.assert_contains("? for shortcuts");
    s.type_text("hi");
    s.assert_absent("? for shortcuts");
    s.assert_absent("for agents");
}
