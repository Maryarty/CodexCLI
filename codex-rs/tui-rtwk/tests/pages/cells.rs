//! Screen tests of the `cells` slot. A slice adds its tests here, with its
//! snapshots in `tests/snapshots/cells/`.

use crate::common::slot;

#[test]
fn cells_slot_is_registered_and_empty() {
    slot("cells");
}

// ---- the patch cell's disclosure (oracle transcript_view/text.rs
// `with_control`: an activity control sits a quarter of the width in, at most
// four columns; the oracle's pty capture of the patch-approval scenario reads
// `    + Show details` at 120 columns) ----

mod patch_disclosure {
    use std::sync::Arc;

    use codex_tui_core::threads::{PatchActivity, PatchChange, PatchKind, ToolStatus};
    use codex_tui_rtwk::pages::session::cells::patch::PatchCell;

    fn cell() -> PatchCell {
        PatchCell {
            activity: Arc::new(PatchActivity {
                changes: vec![PatchChange {
                    path: "notes.txt".into(),
                    kind: PatchKind::Add,
                    diff: "one\ntwo\nthree\nfour\n".into(),
                }],
                status: ToolStatus::Completed,
            }),
            cwd: None,
            detailed: false,
        }
    }

    fn rows(t: &rtwk_test::Tester) -> Vec<String> {
        t.screen()
            .lines()
            .map(|line| line.trim_end().to_string())
            .collect()
    }

    #[test]
    fn patch_disclosure_control_is_indented_like_the_oracle() {
        let mut t = rtwk_test::Tester::with(rtwk_test::Options::new().size(120, 8));
        t.pump_widget(cell());
        let shown = rows(&t);
        assert!(
            shown.iter().any(|row| row == "    + Show details"),
            "{shown:#?}"
        );

        let mut narrow = rtwk_test::Tester::with(rtwk_test::Options::new().size(12, 8));
        narrow.pump_widget(cell());
        let shown = rows(&narrow);
        assert!(
            shown.iter().any(|row| row.starts_with("   + Show")),
            "a quarter of 12 columns: {shown:#?}"
        );
    }

    #[test]
    fn tapping_the_patch_disclosure_reveals_every_line() {
        let mut t = rtwk_test::Tester::with(rtwk_test::Options::new().size(120, 8));
        t.pump_widget(cell());
        assert!(!rows(&t).iter().any(|row| row.contains("4 +four")));
        let y = rows(&t)
            .iter()
            .position(|row| row.contains("+ Show details"))
            .expect("the control");
        t.click(6, y as u16);
        t.pump();
        let shown = rows(&t);
        assert!(
            shown.iter().any(|row| row == "    − Hide details"),
            "{shown:#?}"
        );
        assert!(
            shown.iter().any(|row| row.contains("4 +four")),
            "{shown:#?}"
        );
    }
}

// ---- the compact patch preview (oracle diff_render.rs
// `create_diff_preview_with_links`: the gutter is measured over every hunk
// line, the context is then left out, and a file stops at three screen rows) ----

mod patch_preview {
    use std::sync::Arc;

    use codex_tui_core::threads::{PatchActivity, PatchChange, PatchKind, ToolStatus};
    use codex_tui_rtwk::pages::session::cells::patch::PatchCell;

    fn cell(diff: &str) -> PatchCell {
        PatchCell {
            activity: Arc::new(PatchActivity {
                changes: vec![PatchChange {
                    path: "a.txt".into(),
                    kind: PatchKind::Update { move_path: None },
                    diff: diff.into(),
                }],
                status: ToolStatus::Completed,
            }),
            cwd: None,
            detailed: false,
        }
    }

    fn rows(width: u16, cell: PatchCell) -> Vec<String> {
        let mut t = rtwk_test::Tester::with(rtwk_test::Options::new().size(width, 12));
        t.pump_widget(cell);
        t.screen()
            .lines()
            .map(|line| line.trim_end().to_string())
            .collect()
    }

    #[test]
    fn the_gutter_is_measured_over_the_context_the_preview_leaves_out() {
        // Lines 8..=10 of the new file: the context line `d` is number 10, so
        // the gutter takes two columns although only 8 is shown.
        let shown = rows(40, cell("@@ -8,3 +8,3 @@\n-a\n+b\n c\n d\n"));
        assert!(shown.iter().any(|row| row == "     8 -a"), "{shown:#?}");
        assert!(shown.iter().any(|row| row == "     8 +b"), "{shown:#?}");
        assert!(
            !shown
                .iter()
                .any(|row| row.contains(" c") && row.contains("10")),
            "the context stays out of the preview: {shown:#?}"
        );
    }

    #[test]
    fn a_file_stops_at_three_screen_rows_and_a_wrapped_line_counts_each_row() {
        // The first line wraps into several rows of a 14-column screen, so
        // the three rows reach neither `-c` nor `-d`.
        let diff = "@@ -1,4 +1,0 @@\n-0123456789abcdefghijklmnop\n-c\n-d\n-e\n";
        let shown = rows(14, cell(diff));
        assert!(
            shown.iter().any(|row| row.contains("-0123456")),
            "{shown:#?}"
        );
        assert!(
            !shown
                .iter()
                .any(|row| row.contains("-c") || row.contains("-d")),
            "a wrapped line takes the rows it needs: {shown:#?}"
        );
    }
}

// ---- a revealed patch survives the turn's timing row (the row sits in the
// cell's own column, so the cell keeps its place and its `expanded`) ----

mod patch_in_transcript {
    use codex_tui_core::app::{AppEv, BackendEv};
    use codex_tui_core::fixtures::builders::turn_started;
    use codex_tui_rtwk::services::backend::fake::FAKE_THREAD;
    use serde_json::json;

    use crate::common;

    fn notify(value: serde_json::Value) -> BackendEv {
        match serde_json::from_value(value) {
            Ok(notification) => BackendEv::Notification(Box::new(notification)),
            Err(error) => panic!("not a server notification: {error}"),
        }
    }

    #[test]
    fn a_revealed_patch_stays_revealed_when_the_timing_row_arrives() {
        let mut h = common::start();
        h.wait_idle();
        h.fake.script_turn(vec![
            turn_started(FAKE_THREAD, "turn-1"),
            notify(json!({
                "method": "item/completed",
                "params": {
                    "threadId": FAKE_THREAD, "turnId": "turn-1", "completedAtMs": 0,
                    "item": {
                        "type": "fileChange", "id": "patch-1", "status": "completed",
                        "changes": [{
                            "path": "notes.txt", "kind": {"type": "add"},
                            "diff": "one\ntwo\nthree\nfour\n"
                        }]
                    }
                }
            })),
        ]);
        h.t.type_text("go");
        h.press("enter");
        h.wait_for("the collapsed patch", |s| s.contains("+ Show details"));
        let y =
            h.t.screen()
                .lines()
                .position(|row| row.contains("+ Show details"))
                .expect("the control");
        h.t.click(6, y as u16);
        h.wait_for("the revealed patch", |s| s.contains("4 +four"));

        h.inject([AppEv::Backend(notify(json!({
            "method": "turn/completed",
            "params": {
                "threadId": FAKE_THREAD,
                "turn": {
                    "id": "turn-1", "rootTurnId": null, "items": [], "itemsView": "notLoaded",
                    "status": "completed", "error": null,
                    "startedAt": 1, "completedAt": 6, "durationMs": 5000
                }
            }
        })))]);
        h.wait_for("the timing row", |s| s.contains("Worked for"));
        assert!(
            h.t.screen().contains("4 +four"),
            "the patch collapsed with the timing row: {}",
            h.t.screen()
        );
    }
}

// ---- the reasoning summary: every span dim and italic (oracle
// `ReasoningSummaryCell::lines`, `span.patch_style(dim().italic())`) ----

mod reasoning_style {
    use codex_tui_rtwk::pages::session::cells::reasoning::ReasoningCell;
    use rtwk::style::Attrs;

    fn attrs(summary: &str, at: usize) -> Attrs {
        let mut t = rtwk_test::Tester::with(rtwk_test::Options::new().size(40, 4));
        t.pump_widget(ReasoningCell {
            summary: summary.into(),
        });
        let screen = t.screen();
        let row = screen.lines().next().unwrap_or_default();
        assert!(row.starts_with("• "), "{row:?}");
        t.cell(at, 0).attrs
    }

    fn dim_italic(attrs: Attrs) -> bool {
        attrs.intersects(Attrs::DIM) && attrs.intersects(Attrs::ITALIC)
    }

    #[test]
    fn strong_and_code_spans_are_dim_and_italic_over_their_own_style() {
        let strong = attrs("plain **strong** `code`", 8);
        assert!(dim_italic(strong), "{strong:?}");
        assert!(strong.intersects(Attrs::BOLD), "{strong:?}");
        let code = attrs("plain **strong** `code`", 15);
        assert!(dim_italic(code), "{code:?}");
    }

    /// Gap G71, pinned: when rtwk's paragraphs follow the inherited text style
    /// (Flutter `DefaultTextStyle`) or the theme gets a plain-text style, this
    /// fails and the plain text of `ReasoningCell` takes its dim italic from it.
    #[test]
    fn probe_g71_plain_markdown_text_ignores_default_text_style() {
        use rtwk::style::Style;
        use rtwk::text::DefaultTextStyle;
        use rtwk_rich::markdown::MarkdownView;

        let mut t = rtwk_test::Tester::with(rtwk_test::Options::new().size(20, 2));
        t.pump_widget(DefaultTextStyle::new(
            Style::new().attrs(Attrs::DIM | Attrs::ITALIC),
            MarkdownView::new("plain"),
        ));
        let plain = t.cell(0, 0);
        assert_eq!(plain.text, "p");
        assert!(
            !dim_italic(plain.attrs),
            "G71: the paragraph takes the inherited style: {plain:?}"
        );
    }
}

// ---- file citations (oracle markdown_render/inline_directives.rs: the directive
// reads as the file's name in the transcript's code style; code, HTML, link
// text, escaped and nested directives stay literal) ----

mod citations {
    use codex_tui_rtwk::pages::session::cells::citation;

    const C: &str = r#":codex-file-citation{path="/tmp/report.xlsx" purpose="output"}"#;

    fn show(source: &str, cwd: Option<&str>) -> (rtwk_test::Tester, Vec<String>) {
        let mut t = rtwk_test::Tester::with(rtwk_test::Options::new().size(80, 12));
        t.pump_widget(citation::view(source, cwd));
        let mut rows: Vec<String> = t
            .screen()
            .lines()
            .map(|line| line.trim_end().to_string())
            .collect();
        while rows.last().is_some_and(String::is_empty) {
            rows.pop();
        }
        (t, rows)
    }

    #[test]
    fn a_citation_reads_as_its_path_below_the_cwd_in_code_style() {
        let source = r#"see :codex-file-citation{path="/work/repo/src/lib.rs"} now"#;
        let (t, rows) = show(source, Some("/work/repo"));
        assert_eq!(rows, ["see src/lib.rs now"]);
        let style = |x| {
            let cell = t.cell(x, 0);
            (cell.fg, cell.bg, cell.attrs)
        };
        assert_ne!(style(4), style(0), "the path is drawn as code");
        assert_eq!(style(4), style(10), "one style over the whole path");
        assert_eq!(style(0), style(15), "the words around it are plain");
    }

    #[test]
    fn citations_in_list_items_headings_emphasis_and_table_cells_read() {
        let source = "- :codex-file-citation{path=/tmp/a.txt}\n\n# :codex-file-citation{path=/tmp/b.txt}\n\n*see :codex-file-citation{path=/tmp/c.txt}*\n\n| f |\n|---|\n| :codex-file-citation{path=/tmp/d.txt} |\n";
        let (_, rows) = show(source, None);
        let text = rows.join("\n");
        for path in ["/tmp/a.txt", "/tmp/b.txt", "/tmp/c.txt", "/tmp/d.txt"] {
            assert!(text.contains(path), "{path} in\n{text}");
        }
        assert!(!text.contains("codex-file-citation"), "{text}");
    }

    /// The oracle renders a reasoning summary with its generic renderer
    /// (history_cell/messages.rs `ReasoningSummaryCell`), which keeps
    /// directives literal (file_citations_tests.rs
    /// `generic_markdown_keeps_assistant_directives_literal`).
    #[test]
    fn a_reasoning_summary_keeps_a_citation_literal() {
        use codex_tui_rtwk::pages::session::cells::reasoning::ReasoningCell;

        let mut t = rtwk_test::Tester::with(rtwk_test::Options::new().size(100, 4));
        t.pump_widget(ReasoningCell { summary: C.into() });
        let screen = t.screen();
        let row = screen.lines().next().unwrap_or_default();
        assert_eq!(row.trim_end(), format!("• {C}"));
    }

    #[test]
    fn a_citation_in_code_html_or_a_link_text_stays_literal() {
        for source in [
            format!("`{C}`"),
            format!("```text\n{C}\n```\n"),
            format!("<span title='{C}'>"),
            format!("<!-- {C} -->"),
            format!("[{C}](https://example.com)"),
        ] {
            let (_, rows) = show(&source, None);
            let text = rows.join("\n");
            assert!(text.contains(C), "{source:?} shows {text:?}");
            assert!(!text.contains("/tmp/report.xlsx\n"), "{text:?}");
        }
        let (_, rows) = show(&format!("`{C}`"), None);
        assert_eq!(rows, [C]);
        let (_, rows) = show(&format!("<span title='{C}'>"), None);
        assert_eq!(rows, [format!("<span title='{C}'>")]);
    }

    #[test]
    fn escaped_nested_and_colon_prefixed_directives_stay_literal_and_the_next_one_reads() {
        let c = ":codex-file-citation{path=/tmp/report.xlsx}";
        let (_, rows) = show(&format!(r"\{c} and {c}"), None);
        assert_eq!(rows, [format!("{c} and /tmp/report.xlsx")]);
        for literal in [format!(r#":unsupported{{value="{c}"}}"#), format!(":::{c}")] {
            let (_, rows) = show(&literal, None);
            assert_eq!(rows, [literal]);
        }
    }

    #[test]
    fn a_reference_definition_keeps_its_directive_and_the_text_after_it_reads() {
        let c = ":codex-file-citation{path=/tmp/report.xlsx}";
        let (_, rows) = show(
            &format!("`{c}`\n\n[report][file]\n\n[file]: {c}\n\n{c}"),
            None,
        );
        assert_eq!(rows.first().map(String::as_str), Some(c));
        assert_eq!(rows.last().map(String::as_str), Some("/tmp/report.xlsx"));
        assert!(rows.iter().any(|row| row.starts_with("report")), "{rows:?}");
    }

    #[test]
    fn a_citation_next_to_entities_and_escapes_reads() {
        let (_, rows) = show(
            r#"Préface &amp;:codex-file-citation{path="first&amp;.txt"}&#x1F642;:codex-file-citation{path="second.txt"}&lt;fin&gt;"#,
            Some("/tmp"),
        );
        assert_eq!(rows, ["Préface &first&amp;.txt🙂second.txt<fin>"]);
        let (_, rows) = show(
            r#"\[:codex-file-citation{path="first.txt"}\]\*:codex-file-citation{path="second.txt"}\!"#,
            Some("/tmp"),
        );
        assert_eq!(rows, ["[first.txt]*second.txt!"]);
    }
}
