//! Screen tests of S19 (the `/status` card). Expected text: the oracle's
//! snapshot (`dialogs::oracle`). Oracle tests: `status/tests.rs`
//! (`status_snapshot_includes_reasoning_details`), `status/helpers.rs`
//! (`format_tokens_compact`).

use crate::common::{Harness, ms, start_sized};
use crate::dialogs::oracle::assert_rows;

fn open(entry: &str) -> Harness {
    let mut h = start_sized(true, 100, 24);
    h.open_debug_row("status", Some(entry));
    h.t.pump_for(ms(800));
    h
}

#[test]
fn status_snapshot_includes_reasoning_details() {
    let h = open("reasoning_details");
    assert_rows(
        &h.t.screen(),
        "status/snapshots/codex_tui__status__tests__status_snapshot_includes_reasoning_details.snap",
    );
}

#[test]
fn esc_closes_the_card() {
    let mut h = open("reasoning_details");
    h.press("esc");
    h.t.pump_for(ms(800));
    assert!(!h.t.screen().contains("OpenAI Codex"), "{}", h.t.screen());
}

/// The limit bar on rtwk's own gauge (G40): the oracle draws
/// `[██████░░░░░░░░░░░░░░]` for 28% left, 20 whole cells of `█` and `░`
/// between brackets (`render_limit_progress_bar`). The body is the probe of
/// the gap row; the default profile leaves it out (`--run-ignored all` runs it).
#[test]
#[ignore = "pending(G40) (rtwk-gaps.md): no gauge draws whole `█`/`░` cells (Gauge draws eighths and no `░`, LineGauge::segmented draws `▍`); the bar stays `format::progress_bar`, in R15_NOT_YET"]
fn limit_bar_is_a_rtwk_gauge_of_whole_cells() {
    use rtwk::data::{Gauge, LineGauge};
    use rtwk::layout::{Row, SizedBox};
    use rtwk::widgets::Text;
    let want = "[██████░░░░░░░░░░░░░░] 28% left";
    let mut drawn = Vec::new();
    for (name, gauge) in [
        ("Gauge", Gauge::new(0.28).into()),
        ("LineGauge", LineGauge::new(0.28).segmented().into()),
    ] {
        let mut t = rtwk_test::Tester::with(rtwk_test::Options::new().size(40, 1));
        t.pump_widget(
            Row::new()
                .child(Text::new("["))
                .child(SizedBox::new(Some(20), Some(1), Some(gauge)))
                .child(Text::new("] 28% left")),
        );
        let screen = t.screen();
        drawn.push(format!(
            "{name}: {}",
            screen.lines().next().unwrap_or("").trim_end()
        ));
    }
    let want: Vec<String> = ["Gauge", "LineGauge"]
        .iter()
        .map(|n| format!("{n}: {want}"))
        .collect();
    assert_eq!(drawn, want);
}
