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

// ---- the viewed image's disclosure (oracle transcript_view/layout.rs `activity_layout`: compact
// `• Viewed image example.png` and `+ Show details`; expanded the transcript lines, the full path
// with a two-column hanging indent, and `− Hide details`; the detailed transcript shows the
// full path and no control) ----

mod viewed_image_disclosure {
    use codex_tui_rtwk::pages::session::cells::viewed_image::ViewedImageCell;

    const PATH: &str = "/workspace/assets/example.png";

    fn cell(detailed: bool) -> ViewedImageCell {
        ViewedImageCell {
            name: "example.png".into(),
            path: PATH.into(),
            detailed,
        }
    }

    fn rows(t: &rtwk_test::Tester) -> Vec<String> {
        t.screen()
            .lines()
            .map(|line| line.trim_end().to_string())
            .filter(|line| !line.is_empty())
            .collect()
    }

    fn control_row(t: &rtwk_test::Tester, text: &str) -> u16 {
        t.screen()
            .lines()
            .position(|row| row.contains(text))
            .unwrap_or_else(|| panic!("no `{text}`: {:#?}", rows(t))) as u16
    }

    #[test]
    fn compact_is_the_summary_row_and_an_indented_control() {
        let mut t = rtwk_test::Tester::with(rtwk_test::Options::new().size(80, 6));
        t.pump_widget(cell(false));
        assert_eq!(
            rows(&t),
            ["• Viewed image example.png", "    + Show details"]
        );
    }

    #[test]
    fn tapping_the_control_reveals_the_full_path_and_tapping_again_hides_it() {
        let mut t = rtwk_test::Tester::with(rtwk_test::Options::new().size(80, 6));
        t.pump_widget(cell(false));
        let y = control_row(&t, "+ Show details");
        t.click(6, y);
        t.pump();
        assert_eq!(
            rows(&t),
            [
                format!("• Viewed image {PATH}"),
                "    − Hide details".to_owned()
            ]
        );
        let y = control_row(&t, "− Hide details");
        t.click(6, y);
        t.pump();
        assert_eq!(
            rows(&t),
            ["• Viewed image example.png", "    + Show details"]
        );
    }

    #[test]
    fn the_detailed_transcript_shows_the_full_path_and_no_control() {
        let mut t = rtwk_test::Tester::with(rtwk_test::Options::new().size(80, 6));
        t.pump_widget(cell(true));
        assert_eq!(rows(&t), [format!("• Viewed image {PATH}")]);
    }

    #[test]
    fn a_long_path_wraps_under_a_two_column_hanging_indent() {
        let mut t = rtwk_test::Tester::with(rtwk_test::Options::new().size(24, 8));
        t.pump_widget(cell(true));
        let shown = rows(&t);
        assert!(shown.len() >= 2, "{shown:#?}");
        // The label stays whole; the path, too long for a row, breaks at the width.
        assert_eq!(shown[0], "• Viewed image", "{shown:#?}");
        assert!(
            shown[1..].iter().all(|row| row.starts_with("  ")),
            "{shown:#?}"
        );
        let text: String = shown[1..].iter().map(|row| row.trim_start()).collect();
        assert_eq!(text, PATH);
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

// ---- web search: item/completed after item/started leaves both rows (oracle
// tool_lifecycle.rs: `handle_thread_item` runs `on_web_search_begin`, which
// commits the started cell, then `on_web_search_end`; the oracle's pty capture of
// the web-search scenario reads `• Browsing the web` above `• Searched the web for …`) ----

mod web_search_in_transcript {
    use codex_tui_core::fixtures::builders::turn_started;
    use codex_tui_rtwk::services::backend::fake::FAKE_THREAD;
    use serde_json::json;

    use crate::common;

    fn item(method: &str, payload: serde_json::Value) -> codex_tui_core::app::BackendEv {
        match serde_json::from_value(json!({
            "method": method,
            "params": {"threadId": FAKE_THREAD, "turnId": "turn-1", "startedAtMs": 0,
                "completedAtMs": 0, "item": payload}
        })) {
            Ok(notification) => {
                codex_tui_core::app::BackendEv::Notification(Box::new(notification))
            }
            Err(error) => panic!("not a server notification: {error}"),
        }
    }

    #[test]
    fn a_completed_search_follows_the_browsing_row() {
        let mut h = common::start();
        h.wait_idle();
        h.fake.script_turn(vec![
            turn_started(FAKE_THREAD, "turn-1"),
            item(
                "item/started",
                json!({"type": "webSearch", "id": "w", "query": ""}),
            ),
            item(
                "item/completed",
                json!({"type": "webSearch", "id": "w", "query": "rtwk",
                    "action": {"type": "search", "query": "rtwk"}}),
            ),
        ]);
        h.t.type_text("go");
        h.press("enter");
        h.wait_for("the completed search", |s| {
            s.contains("Searched the web for rtwk")
        });
        let rows: Vec<String> =
            h.t.screen()
                .lines()
                .map(|r| r.trim_end().to_string())
                .collect();
        let at = |text: &str| rows.iter().position(|row| row.contains(text));
        let (browsing, searched) = (at("• Browsing the web"), at("• Searched the web for rtwk"));
        assert!(
            browsing.is_some() && searched.is_some() && browsing < searched,
            "{rows:#?}"
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

// ---- the command cell (oracle exec_cell/render.rs `dimmed_output_line`: the
// output's own SGR codes colour its rows, they are never drawn as text; the
// bullet of a running command is static without animations) ----

mod command_in_transcript {
    use codex_tui_core::fixtures::builders::turn_started;
    use codex_tui_rtwk::services::backend::fake::FAKE_THREAD;
    use serde_json::json;

    use crate::common;

    fn item(method: &str, payload: serde_json::Value) -> codex_tui_core::app::BackendEv {
        match serde_json::from_value(json!({
            "method": method,
            "params": {"threadId": FAKE_THREAD, "turnId": "turn-1", "startedAtMs": 0,
                "completedAtMs": 0, "item": payload}
        })) {
            Ok(notification) => {
                codex_tui_core::app::BackendEv::Notification(Box::new(notification))
            }
            Err(error) => panic!("not a server notification: {error}"),
        }
    }

    fn command(status: &str, output: Option<&str>) -> serde_json::Value {
        json!({"type": "commandExecution", "id": "c", "command": "bash -lc 'echo done'",
            "cwd": "/workspace", "processId": null, "source": "agent", "status": status,
            "commandActions": [], "aggregatedOutput": output,
            "exitCode": if output.is_some() { json!(0) } else { json!(null) }, "durationMs": null})
    }

    #[test]
    fn output_colours_are_styles_not_text() {
        let mut h = common::start();
        h.wait_idle();
        h.fake.script_turn(vec![
            turn_started(FAKE_THREAD, "turn-1"),
            item(
                "item/completed",
                command("completed", Some("\u{1b}[31mred words\u{1b}[0m plain")),
            ),
        ]);
        h.t.type_text("go");
        h.press("enter");
        h.wait_for("the command output", |s| s.contains("red words plain"));
        let screen = h.t.screen();
        assert!(!screen.contains("[31m"), "{screen}");
        let y = screen
            .lines()
            .position(|row| row.contains("red words"))
            .expect("the output row");
        let x = screen
            .lines()
            .nth(y)
            .and_then(|row| row.find("red"))
            .expect("column");
        let (red, plain) = (h.t.cell(x, y), h.t.cell(x + 10, y));
        assert_ne!(
            red.fg, plain.fg,
            "the red word is coloured, the rest is not"
        );
    }

    #[test]
    fn a_running_commands_bullet_row_is_stable() {
        let mut h = common::start();
        h.wait_idle();
        h.fake.script_turn(vec![
            turn_started(FAKE_THREAD, "turn-1"),
            item("item/started", command("inProgress", None)),
        ]);
        h.t.type_text("go");
        h.press("enter");
        h.wait_for("the running command", |s| s.contains("Running echo done"));
        let row = |s: &str| {
            s.lines()
                .find(|row| row.contains("Running echo done"))
                .map(str::to_owned)
        };
        let before = row(&h.t.screen());
        h.advance(1000);
        assert_eq!(before, row(&h.t.screen()));
        assert!(before.is_some_and(|row| row.starts_with('•')));
    }
}

// ---- the plan update (oracle history_cell/plans.rs `PlanUpdateCell`: the
// compact rows are `compact_hyperlink_lines`; the control appears when
// `activity_disclosure` says so, and revealing it shows
// `transcript_hyperlink_lines`) ----

mod plan_update {
    use std::sync::Arc;

    use codex_tui_core::threads::{PlanStep, PlanUpdate, StepStatus};
    use codex_tui_rtwk::pages::session::cells::plan_update::PlanUpdateCell;
    use rtwk::style::Attrs;

    use StepStatus::{Completed, InProgress, Pending};

    fn update(note: Option<&str>, steps: &[(&str, StepStatus)]) -> Arc<PlanUpdate> {
        Arc::new(PlanUpdate {
            explanation: note.map(str::to_owned),
            steps: steps
                .iter()
                .map(|(text, status)| PlanStep {
                    text: (*text).to_owned(),
                    status: *status,
                })
                .collect(),
        })
    }

    fn show(width: u16, update: Arc<PlanUpdate>, detailed: bool) -> rtwk_test::Tester {
        let mut t = rtwk_test::Tester::with(rtwk_test::Options::new().size(width, 24));
        t.pump_widget(PlanUpdateCell { update, detailed });
        t
    }

    fn rows(t: &rtwk_test::Tester) -> Vec<String> {
        t.screen()
            .lines()
            .map(|line| line.trim_end().to_string())
            .filter(|line| !line.is_empty())
            .collect()
    }

    fn tap(t: &mut rtwk_test::Tester, label: &str) {
        let y = t
            .screen()
            .lines()
            .position(|row| row.contains(label))
            .unwrap_or_else(|| panic!("no `{label}`: {:#?}", rows(t))) as u16;
        t.click(6, y);
        t.pump();
    }

    #[test]
    fn compact_counts_the_steps_done_and_lists_the_open_steps_first() {
        let steps = [
            ("Investigate", Completed),
            ("Write tests", Pending),
            ("Harden the client", InProgress),
            ("Document", Pending),
        ];
        let t = show(40, update(None, &steps), false);
        assert_eq!(
            rows(&t),
            [
                "• Updated Plan · 1/4 complete",
                "  └ □ Harden the client",
                "    □ Write tests",
                "    □ Document",
                "    + Show details",
            ]
        );
    }

    #[test]
    fn a_short_plan_without_a_note_has_no_control() {
        let steps = [
            ("Define error taxonomy", Pending),
            ("Implement mapping", Pending),
        ];
        let t = show(40, update(None, &steps), false);
        assert_eq!(
            rows(&t),
            [
                "• Updated Plan · 0/2 complete",
                "  └ □ Define error taxonomy",
                "    □ Implement mapping",
            ]
        );
    }

    #[test]
    fn a_completed_step_is_listed_when_the_open_ones_leave_room() {
        let steps = [("Done one", Completed), ("Open one", Pending)];
        let t = show(40, update(None, &steps), false);
        assert_eq!(
            rows(&t),
            [
                "• Updated Plan · 1/2 complete",
                "  └ □ Open one",
                "    ✔ Done one",
            ]
        );
    }

    #[test]
    fn a_note_a_wide_step_or_a_fourth_step_each_offer_the_control() {
        let one = [("Step", Pending)];
        assert!(
            rows(&show(40, update(Some("Why"), &one), false))
                .contains(&"    + Show details".to_owned())
        );
        assert!(
            !rows(&show(40, update(Some("  \n"), &one), false))
                .iter()
                .any(|r| r.contains("Show details"))
        );
        // A step takes its width plus six columns: "abcd" fits ten, not nine.
        let wide = [("abcd", Pending)];
        assert!(
            !rows(&show(10, update(None, &wide), false))
                .iter()
                .any(|r| r.contains("Show details"))
        );
        // (the control wraps at nine columns: its first word is the sign)
        assert!(
            rows(&show(9, update(None, &wide), false))
                .iter()
                .any(|r| r.contains("+ Show"))
        );
        let four = [
            ("a", Pending),
            ("b", Pending),
            ("c", Pending),
            ("d", Pending),
        ];
        assert!(
            rows(&show(40, update(None, &four), false))
                .iter()
                .any(|r| r.contains("Show details"))
        );
    }

    #[test]
    fn compact_rows_clip_with_an_ellipsis_at_the_width() {
        let steps = [("Harden Grafana client error handling", InProgress)];
        let t = show(20, update(None, &steps), false);
        let shown = rows(&t);
        assert_eq!(shown[0], "• Updated Plan · 0/…", "{shown:#?}");
        assert_eq!(shown[1], "  └ □ Harden Grafan…", "{shown:#?}");
    }

    #[test]
    fn an_empty_plan_says_so() {
        let t = show(40, update(None, &[]), false);
        assert_eq!(
            rows(&t),
            ["• Updated Plan · 0/0 complete", "  └ (no steps provided)"]
        );
    }

    #[test]
    fn tapping_the_control_shows_the_note_and_every_step_in_plan_order_wrapped() {
        let steps = [
            (
                "Investigate existing error paths and logging around HTTP timeouts",
                Completed,
            ),
            ("Harden Grafana client error handling", InProgress),
        ];
        let note = "I’ll update Grafana call error handling by adding retries.";
        let mut t = show(32, update(Some(note), &steps), false);
        assert_eq!(rows(&t)[0], "• Updated Plan · 1/2 complete");
        tap(&mut t, "+ Show details");
        assert_eq!(
            rows(&t),
            [
                "• Updated Plan",
                "  └ I’ll update Grafana call",
                "    error handling by adding",
                "    retries.",
                "    ✔ Investigate existing error",
                "      paths and logging around",
                "      HTTP timeouts",
                "    □ Harden Grafana client",
                "      error handling",
                "    − Hide details",
            ]
        );
        tap(&mut t, "− Hide details");
        assert_eq!(rows(&t)[0], "• Updated Plan · 1/2 complete");
    }

    fn snapshot_cell() -> Arc<PlanUpdate> {
        let steps = [
            (
                "Investigate existing error paths and logging around HTTP timeouts",
                Completed,
            ),
            (
                "Harden Grafana client error handling with retry/backoff and user‑friendly messages",
                InProgress,
            ),
            (
                "Add tests for transient failure scenarios and surfacing to the UI",
                Pending,
            ),
            ("Document the retry behavior", Pending),
        ];
        let note = "I’ll update Grafana call error handling by adding retries and clearer messages when the backend is unreachable.";
        update(Some(note), &steps)
    }

    /// The `Compact` frame of `plan_update_with_note_and_wrapping_snapshot`
    /// (history_cell/snapshots), 32 columns.
    #[test]
    fn the_oracle_snapshot_compact_rows() {
        let t = show(32, snapshot_cell(), false);
        assert_eq!(
            rows(&t),
            [
                "• Updated Plan · 1/4 complete",
                "  └ □ Harden Grafana client err…",
                "    □ Add tests for transient f…",
                "    □ Document the retry behavi…",
                "    + Show details",
            ]
        );
    }

    /// The `Full` frame of the same snapshot: the oracle's wrapper breaks
    /// `retry/backoff` after the slash (unicode line breaks).
    #[test]
    #[ignore = "pending(G49): TextWrap::Word never breaks after a slash, so `retry/backoff` moves whole to the next row"]
    fn the_oracle_snapshot_full_rows() {
        let mut t = show(32, snapshot_cell(), false);
        tap(&mut t, "+ Show details");
        let shown = rows(&t);
        assert_eq!(
            &shown[..shown.len() - 1],
            [
                "• Updated Plan",
                "  └ I’ll update Grafana call",
                "    error handling by adding",
                "    retries and clearer messages",
                "    when the backend is",
                "    unreachable.",
                "    ✔ Investigate existing error",
                "      paths and logging around",
                "      HTTP timeouts",
                "    □ Harden Grafana client",
                "      error handling with retry/",
                "      backoff and user‑friendly",
                "      messages",
                "    □ Add tests for transient",
                "      failure scenarios and",
                "      surfacing to the UI",
                "    □ Document the retry",
                "      behavior",
            ]
        );
    }

    #[test]
    fn revealed_without_a_note_the_first_step_carries_the_branch() {
        let steps = [
            ("First", Pending),
            ("Second", Pending),
            ("Third", Pending),
            ("Fourth", Pending),
        ];
        let mut t = show(40, update(None, &steps), false);
        tap(&mut t, "+ Show details");
        assert_eq!(
            rows(&t),
            [
                "• Updated Plan",
                "  └ □ First",
                "    □ Second",
                "    □ Third",
                "    □ Fourth",
                "    − Hide details",
            ]
        );
    }

    #[test]
    fn the_detailed_transcript_shows_everything_and_no_control() {
        let steps = [("Only", Completed)];
        let t = show(40, update(Some("Why"), &steps), true);
        assert_eq!(rows(&t), ["• Updated Plan", "  └ Why", "    ✔ Only"]);
    }

    #[test]
    fn steps_are_styled_by_status() {
        let steps = [("Done", Completed), ("Now", InProgress), ("Later", Pending)];
        let t = show(40, update(Some("n"), &steps), true);
        let x = 6;
        // Rows: header, note + first step share the branch, so the steps start at row 2.
        let done = t.cell(x, 2);
        let now = t.cell(x, 3);
        let later = t.cell(x, 4);
        assert!(done.attrs.intersects(Attrs::DIM), "{done:?}");
        assert!(done.attrs.intersects(Attrs::STRIKE), "{done:?}");
        assert!(now.attrs.intersects(Attrs::BOLD), "{now:?}");
        assert_ne!(now.fg, later.fg, "the running step is accented");
        assert!(later.attrs.intersects(Attrs::DIM), "{later:?}");
        assert!(!later.attrs.intersects(Attrs::STRIKE), "{later:?}");
    }
}

mod plan_in_transcript {
    use codex_tui_core::fixtures::builders::turn_started;
    use codex_tui_rtwk::services::backend::fake::FAKE_THREAD;
    use serde_json::json;

    use crate::common;

    fn plan(steps: serde_json::Value) -> codex_tui_core::app::BackendEv {
        match serde_json::from_value(json!({
            "method": "turn/plan/updated",
            "params": {"threadId": FAKE_THREAD, "turnId": "turn-1",
                "explanation": null, "plan": steps}
        })) {
            Ok(notification) => {
                codex_tui_core::app::BackendEv::Notification(Box::new(notification))
            }
            Err(error) => panic!("not a server notification: {error}"),
        }
    }

    #[test]
    fn each_plan_update_is_a_cell_and_a_revealed_one_stays_revealed() {
        let mut h = common::start();
        h.wait_idle();
        h.fake.script_turn(vec![
            turn_started(FAKE_THREAD, "turn-1"),
            plan(json!([{"step": "One", "status": "inProgress"},
                {"step": "Two", "status": "pending"},
                {"step": "Three", "status": "pending"},
                {"step": "Four", "status": "pending"}])),
            plan(json!([{"step": "One", "status": "completed"},
                {"step": "Two", "status": "inProgress"}])),
        ]);
        h.t.type_text("go");
        h.press("enter");
        h.wait_for("both plan cells", |s| {
            s.matches("Updated Plan").count() == 2
        });
        let screen = h.t.screen();
        assert!(screen.contains("Updated Plan · 0/4 complete"), "{screen}");
        assert!(screen.contains("Updated Plan · 1/2 complete"), "{screen}");
        // The first plan has four steps: it offers the control, and revealing it keeps the
        // later update's row where the tree puts it.
        let y = screen
            .lines()
            .position(|row| row.contains("+ Show details"))
            .expect("the control");
        h.t.click(8, y as u16);
        h.t.pump();
        let screen = h.t.screen();
        assert!(screen.contains("− Hide details"), "{screen}");
        assert!(screen.contains("□ Four"), "{screen}");
        h.advance(1000);
        assert!(h.t.screen().contains("− Hide details"));
    }
}
