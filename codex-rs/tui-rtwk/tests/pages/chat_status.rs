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
/// oracle's `ComposerHasDraft`), and a footer with no label is no row
/// (`footer_hint_height` is 0 when no label fits): the composer sits one row
/// lower.
#[test]
fn a_draft_takes_the_shortcut_hints_and_their_row_away() {
    // the status line is on: the context meter is in it, not in the hints
    let mut s = page(status_script("model"), 80, 12);
    s.assert_contains("? for shortcuts");
    let prompt = |s: &Screen| s.rows().iter().position(|r| r.starts_with("› "));
    let before = prompt(&s).expect("composer row");
    s.type_text("hi");
    s.assert_absent("? for shortcuts");
    s.assert_absent("for agents");
    assert_eq!(prompt(&s), Some(before + 1), "{}", s.text());
}

fn status_script(model: &str) -> &'static str {
    let script = format!(
        "{{\"step\":\"screen\",\"dir\":\"chat\",\"ev\":{{\"status\":{{\"model\":\"{model}\",\"items\":[\"model-name\"]}}}}}}\n"
    );
    Box::leak(script.into_boxed_str())
}

/// The status line yields to the draft as the draft grows (`status_budget`:
/// the composer first), and takes its rows back as it shrinks.
#[test]
fn the_status_line_yields_rows_to_a_growing_draft() {
    let mut s = page(status_script("one two three four five"), 7, 10);
    s.paste("a");
    s.assert_in_order(&["one", "two…"]);
    s.paste("\nb\nc\nd\ne");
    s.assert_contains("one…");
    s.assert_absent("two");
    // a key line of the draft taken back: the line has its second row again
    for _ in 0..3 {
        s.keys("backspace");
        s.keys("backspace");
    }
    s.assert_in_order(&["one", "two…"]);
}

/// The status line stays under the composer, at its two cells, while the
/// command popup is open above it (`slash_popup_preserves_footer_and_selection_style`:
/// the popup's bottom is the composer's top, the composer's bottom the status).
#[test]
fn the_status_line_stays_under_the_composer_while_the_popup_is_open() {
    let script: &'static str = Box::leak(
        r#"{"step":"screen","dir":"chat","ev":{"status":{"model":"model","effort":"high","project_root":"fast","items":["model-name","reasoning","project-root"],"colors":false}}}"#
            .to_owned()
            .into_boxed_str(),
    );
    let mut s = page(script, 80, 12);
    s.type_text("/m");
    s.keys("down");
    let row = |s: &Screen, text: &str| s.rows().iter().position(|r| r.contains(text));
    let composer = row(&s, "› /m").expect("composer row");
    let status = row(&s, "model · high · fast").expect("status row");
    assert_eq!(status, composer + 2, "{}", s.text());
    s.assert_row(status, "  model · high · fast");
    s.resize(32, 12);
    let composer = row(&s, "› /m").expect("composer row");
    let status = row(&s, "model · high · fast").expect("status row");
    assert_eq!(status, composer + 2, "{}", s.text());
}
