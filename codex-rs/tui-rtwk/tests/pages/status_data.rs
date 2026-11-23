#![allow(clippy::expect_used, clippy::unwrap_used)]

//! Screen tests of the session's status line and footer data (`status-data`).
//! Oracle: `chatwidget/status_surfaces.rs` (the items), `bottom_pane/
//! chat_composer/warning_notice.rs` and `warning_notice_tests.rs` (the
//! footer's `⚠ N warnings · f2 to view`, snapshot
//! `warning_notice_styles`: the notice's row is the hints at the left, the
//! notice at the right with one clear cell at the edge).

use codex_tui_core::app::AppEv;
use codex_tui_core::screens::seed_ev;

use crate::common::{Harness, start_sized};

fn feed(h: &mut Harness, jsonl: &str) {
    h.wait_idle();
    let events: Vec<AppEv> = jsonl
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            let v: serde_json::Value = serde_json::from_str(line).expect("json");
            let dir = v["dir"].as_str().expect("dir");
            AppEv::Screen(seed_ev(dir, v["ev"].clone()).expect("seed"))
        })
        .collect();
    // Applied by the app, not merely sent: the screen below is read at once.
    h.inject(events);
    h.advance(300);
}

fn session(width: u16, jsonl: &str) -> Harness {
    let mut h = start_sized(false, width, 24);
    feed(&mut h, jsonl);
    h
}

fn rows(h: &Harness) -> Vec<String> {
    h.t.lines()
        .iter()
        .map(|r| r.trim_end().to_string())
        .collect()
}

fn status(model: &str, items: &str, colors: bool) -> String {
    format!(
        r#"{{"step":"screen","dir":"chat","ev":{{"status":{{"model":"{model}","effort":"high","project_root":"rtwk","branch":"main","approval_policy":"on-request","auto_review":true,"colors":{colors},"items":{items}}}}}}}"#
    )
}

const SCREENSHOT_ITEMS: &str = r#"["model-with-reasoning","project-root","git-branch","approval-mode","context-remaining","context-used","five-hour-limit","weekly-limit"]"#;

fn retained(count: usize) -> String {
    let entries: Vec<String> = (1..=count)
        .map(|i| format!(r#"{{"source":"Startup","details":"warning {i}"}}"#))
        .collect();
    format!(
        r#"{{"step":"screen","dir":"dialogs","ev":{{"kind":"retained","entries":[{}]}}}}"#,
        entries.join(",")
    )
}

fn status_row(h: &Harness) -> String {
    rows(h)
        .into_iter()
        .find(|r| r.contains(" · ") && !r.contains("for agents"))
        .unwrap_or_default()
}

// The person's screenshot: the row sits two cells in, the items joined by ` · ` in the
// configured order. A session that has counted no tokens still says `Context 100% left` and
// `0% used` (`status_line_value_for_item`: no window and no count read as a full window);
// the rate-limit items wait for rate-limit data and stay out.
#[test]
fn status_line_shows_the_configured_items() {
    let h = session(80, &status("GPT-6.2-Sol", SCREENSHOT_ITEMS, true));
    assert!(
        status_row(&h)
            .starts_with("  GPT-6.2-Sol high · rtwk · main · Approve for me · Context 100% left"),
        "{}",
        h.t.screen()
    );
    assert!(h.t.screen().contains("0% used"), "{}", h.t.screen());
}

// Reordering the config's list reorders the line (`configured_status_line_items`).
#[test]
fn reordering_the_items_reorders_the_line() {
    let items = r#"["git-branch","approval-mode","project-root","model-name"]"#;
    let h = session(80, &status("GPT-6.2-Sol", items, true));
    assert_eq!(
        status_row(&h),
        "  main · Approve for me · rtwk · GPT-6.2-Sol",
        "{}",
        h.t.screen()
    );
}

// No list configured: the oracle's default (model with reasoning, current dir,
// thread name); the dir and the name have no value here, so the model alone.
#[test]
fn without_a_configured_list_the_default_items_show() {
    let line = r#"{"step":"screen","dir":"chat","ev":{"status":{"model":"gpt-5.4","effort":"xhigh","cwd":"/tmp/project"}}}"#;
    let h = session(80, line);
    assert!(
        rows(&h)
            .iter()
            .any(|r| r == "  gpt-5.4 xhigh · /tmp/project"),
        "{}",
        h.t.screen()
    );
}

// tui.status_line_use_colors: on, an item has its own colour; off, every item
// is the secondary style of the separators (`status_line_segments_can_disable_theme_colors`).
#[test]
fn colours_off_make_the_line_plain() {
    let fg = |h: &Harness| {
        let y = rows(h)
            .iter()
            .position(|r| r.starts_with("  GPT-6.2-Sol"))
            .expect("status row");
        (
            h.t.cell(2, y).fg,
            h.t.cell(2 + "GPT-6.2-Sol high".len() + 1, y).fg,
        )
    };
    let on = session(
        80,
        &status(
            "GPT-6.2-Sol",
            r#"["model-with-reasoning","git-branch"]"#,
            true,
        ),
    );
    let (model, separator) = fg(&on);
    assert_ne!(model, separator, "{}", on.t.screen());
    let off = session(
        80,
        &status(
            "GPT-6.2-Sol",
            r#"["model-with-reasoning","git-branch"]"#,
            false,
        ),
    );
    let (model, separator) = fg(&off);
    assert_eq!(model, separator, "{}", off.t.screen());
}

// Nothing read yet: no status row, the footer alone.
#[test]
fn no_facts_no_status_row() {
    let h = session(
        80,
        r#"{"step":"screen","dir":"chat","ev":{"status":{"items":["model-name"]}}}"#,
    );
    assert_eq!(status_row(&h), "", "{}", h.t.screen());
}

// warning_notice_styles (80 columns): the hints at the left, `⚠ 3 warnings ·
// f2 to view` at the right with one clear cell at the edge.
#[test]
fn warning_notice_is_at_the_footer_right() {
    let h = session(80, &retained(3));
    let rows = rows(&h);
    let last = rows.last().expect("rows");
    assert!(
        last.starts_with("  ← for agents · ? for shortcuts"),
        "{last}"
    );
    assert!(last.ends_with("⚠ 3 warnings · f2 to view"), "{last}");
    assert_eq!(
        last.chars().count(),
        79,
        "one clear cell at the edge:\n{last}"
    );
    // The count is the warning accent, the key bold, the rest secondary.
    let y = rows.len() - 1;
    let at = |col: usize| h.t.cell(col, y);
    assert_eq!(at(54).text, "⚠");
    assert_ne!(at(56).fg, at(54).fg, "the count has the warning colour");
    assert_eq!(at(54).fg, at(67).fg, "the separator is secondary");
    assert!(
        at(69).attrs.0 & rtwk_core::style::Attrs::BOLD.0 != 0,
        "f2 is bold"
    );
}

// `plural`, and the compact form when the full one does not fit beside the
// hints (`passive_hints`: the warning is a label of the hint row and collapses
// by the oracle's order, not by a budget of its own).
#[test]
fn warning_notice_pluralizes_and_compacts() {
    let h = session(80, &retained(1));
    assert!(
        rows(&h)
            .last()
            .expect("rows")
            .ends_with("⚠ 1 warning · f2 to view")
    );
    let h = session(48, &retained(3));
    let last = rows(&h).last().cloned().expect("rows");
    assert!(last.ends_with("⚠ 3 warnings · f2 to view"), "{last}");
    let h = session(30, &retained(3));
    let last = rows(&h).last().cloned().expect("rows");
    assert!(last.ends_with("⚠ 3 · f2"), "{last}");
}

// No warnings, no notice.
#[test]
fn no_warnings_no_notice() {
    let h = session(80, "");
    assert!(!h.t.screen().contains("warning"), "{}", h.t.screen());
}

// F2 opens the viewer over the retained warnings; closing it dismisses what
// was shown and the count follows.
#[test]
fn f2_opens_the_retained_warnings_and_dismissal_updates_the_count() {
    let mut h = session(80, &retained(3));
    h.press("f2");
    h.wait_for("the viewer", |s| s.contains("warning 1"));
    h.press("escape");
    h.wait_for("the count after the dismissal", |s| {
        s.contains("⚠ 2 warnings · f2 to view")
    });
    h.t.pump_for(crate::common::ms(500));
    let screen = h.t.screen();
    assert!(!screen.contains("warning 1"), "{screen}");
    assert!(screen.contains("⚠ 2 warnings · f2 to view"), "{screen}");
}

// F2 with nothing retained shows the viewer's empty page.
#[test]
fn f2_without_warnings_shows_the_empty_viewer() {
    let mut h = session(80, "");
    h.press("f2");
    h.wait_for("the empty viewer", |s| s.contains("No warnings"));
    assert!(h.t.screen().contains("No warnings"), "{}", h.t.screen());
}
