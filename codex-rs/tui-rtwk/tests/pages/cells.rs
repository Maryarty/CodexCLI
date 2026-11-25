//! Screen tests of the `cells` slot. A slice adds its tests here, with its
//! snapshots in `tests/snapshots/cells/`.

use crate::common::slot;

#[path = "../oracle_snap/mod.rs"]
mod oracle_snap;

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

    #[test]
    fn an_added_or_removed_row_is_tinted_over_its_whole_width_and_context_is_not() {
        // oracle `style_line_bg_for`: the tint fills the row from the indent to the right edge,
        // not only the text; the two tints differ, the context row has none.
        let mut t = rtwk_test::Tester::with(rtwk_test::Options::new().size(30, 12));
        t.pump_widget(PatchCell {
            detailed: true,
            ..cell("@@ -1,2 +1,2 @@\n-a\n+b\n c\n")
        });
        let rows: Vec<String> = t
            .screen()
            .lines()
            .map(|l| l.trim_end().to_string())
            .collect();
        let at = |needle: &str| {
            rows.iter()
                .position(|row| row.ends_with(needle))
                .unwrap_or_else(|| panic!("no row ends with `{needle}`: {rows:#?}"))
        };
        let (removed, added, context) = (at("1 -a"), at("1 +b"), at("2  c"));
        for x in [0, 3, 7, 29] {
            assert_eq!(
                t.cell(x, removed).bg,
                t.cell(0, removed).bg,
                "removed row at {x}"
            );
            assert_eq!(t.cell(x, added).bg, t.cell(0, added).bg, "added row at {x}");
        }
        assert_ne!(
            t.cell(0, added).bg,
            t.cell(0, context).bg,
            "the added row is tinted"
        );
        assert_ne!(
            t.cell(0, removed).bg,
            t.cell(0, context).bg,
            "the removed row is tinted"
        );
        assert_ne!(
            t.cell(0, added).bg,
            t.cell(0, removed).bg,
            "the two tints differ"
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

    /// The plain words take the inherited text style (rtwk main's paragraphs
    /// follow `DefaultTextStyle`; gap G71 closed by the repin to 5583c142).
    #[test]
    fn plain_words_are_dim_and_italic_too() {
        let plain = attrs("plain **strong** `code`", 2);
        assert!(dim_italic(plain), "{plain:?}");
    }
}

// ---- file citations (oracle markdown_render/inline_directives.rs: the directive
// reads as the file's name in the transcript's code style; code, HTML, link
// text, escaped and nested directives stay literal) ----

mod citations {
    use codex_tui_rtwk::pages::session::cells::{citation, strings};

    const C: &str = r#":codex-file-citation{path="/tmp/report.xlsx" purpose="output"}"#;

    fn show(source: &str, cwd: Option<&str>) -> (rtwk_test::Tester, Vec<String>) {
        let mut t = rtwk_test::Tester::with(rtwk_test::Options::new().size(80, 12));
        t.pump_widget(citation::view(source, false, cwd, &strings::EN));
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

mod hook_cell {
    use std::sync::Arc;

    use codex_tui_core::threads::{HookEntry, HookEntryKind, HookRun, HookStatus};
    use codex_tui_rtwk::pages::session::cells::hook::HookCell;
    use rtwk::layout::Column;
    use rtwk::style::{Attrs, Color};
    use rtwk::widgets::Text;

    use HookEntryKind::{Error, Feedback, Stop, Warning};
    use HookStatus::{Blocked, Completed, Failed, Stopped};

    fn show(
        width: u16,
        status: HookStatus,
        entries: &[(HookEntryKind, &str)],
    ) -> rtwk_test::Tester {
        let run = Arc::new(HookRun {
            status,
            entries: entries
                .iter()
                .map(|(kind, text)| HookEntry {
                    kind: *kind,
                    text: (*text).to_owned(),
                })
                .collect(),
        });
        let mut t = rtwk_test::Tester::with(rtwk_test::Options::new().size(width, 16));
        t.pump_widget(HookCell { run });
        t
    }

    fn rows(t: &rtwk_test::Tester) -> Vec<String> {
        t.screen()
            .lines()
            .map(|line| line.trim_end().to_string())
            .collect::<Vec<_>>()
    }

    /// The drawn rows without the empty tail of the surface.
    fn drawn(t: &rtwk_test::Tester) -> Vec<String> {
        let mut rows = rows(t);
        while rows.last().is_some_and(String::is_empty) {
            rows.pop();
        }
        rows
    }

    #[test]
    fn a_completed_run_shows_its_system_message_after_a_dim_provenance() {
        let t = show(80, Completed, &[(Warning, "Heads up from the hook")]);
        assert_eq!(drawn(&t), ["↳ Hook · Heads up from the hook"]);
        assert!(
            t.cell(0, 0).attrs.intersects(Attrs::DIM),
            "{:?}",
            t.cell(0, 0)
        );
        let text = t.cell(10, 0);
        assert!(!text.attrs.intersects(Attrs::DIM), "{text:?}");
        assert_ne!(text.fg, Color::GREEN, "{text:?}");
    }

    #[test]
    fn only_the_first_line_carries_the_provenance_and_a_final_newline_is_a_blank_row() {
        let t = show(
            80,
            Completed,
            &[(Warning, "Heads up\n\nReview generated files\n")],
        );
        assert_eq!(
            drawn(&t),
            ["↳ Hook · Heads up", "", "    Review generated files"]
        );
        // The trailing newline leaves a blank row that is part of the cell:
        // what follows the cell starts a row below it.
        let mut t = rtwk_test::Tester::with(rtwk_test::Options::new().size(80, 16));
        t.pump_widget(
            Column::new()
                .child(HookCell {
                    run: Arc::new(HookRun {
                        status: Completed,
                        entries: vec![HookEntry {
                            kind: Warning,
                            text: "Heads up\n\nReview generated files\n".to_owned(),
                        }],
                    }),
                })
                .child(Text::new("after")),
        );
        assert_eq!(
            rows(&t).iter().position(|row| row.trim() == "after"),
            Some(4)
        );
    }

    #[test]
    fn sgr_codes_in_the_system_message_read_as_styles() {
        let t = show(
            80,
            Completed,
            &[(
                Warning,
                "Styled \u{1b}[1;31moutput\u{1b}[0m\nacross lines plain",
            )],
        );
        assert_eq!(
            drawn(&t),
            ["↳ Hook · Styled output", "    across lines plain"]
        );
        let styled = t.cell(17, 0);
        assert!(styled.attrs.intersects(Attrs::BOLD), "{styled:?}");
        assert_eq!(styled.fg, Color::RED, "{styled:?}");
        let plain = t.cell(8, 1);
        assert!(!plain.attrs.intersects(Attrs::BOLD), "{plain:?}");
    }

    #[test]
    fn an_unsuccessful_run_has_a_bold_red_bullet_header_and_its_message_under_a_branch() {
        for (status, header) in [
            (Failed, "• Hook failed"),
            (Blocked, "• Blocked by hook"),
            (Stopped, "• Hook stopped"),
        ] {
            let t = show(80, status, &[(Warning, "Policy prevented this action.")]);
            assert_eq!(
                drawn(&t),
                [header, "  └ Policy prevented this action."],
                "{status:?}"
            );
            let bullet = t.cell(0, 0);
            assert!(bullet.attrs.intersects(Attrs::BOLD), "{bullet:?}");
            assert_eq!(bullet.fg, Color::RED, "{bullet:?}");
        }
    }

    #[test]
    fn a_completed_run_with_no_system_message_is_a_green_header() {
        let t = show(80, Completed, &[(Error, "boom")]);
        assert_eq!(drawn(&t), ["• Hook completed", "  └ boom"]);
        let bullet = t.cell(0, 0);
        assert!(bullet.attrs.intersects(Attrs::BOLD), "{bullet:?}");
        assert_eq!(bullet.fg, Color::GREEN, "{bullet:?}");
    }

    #[test]
    fn the_other_entries_are_drawn_whole_in_the_order_they_came() {
        for kind in [Warning, Stop, Feedback, Error] {
            let t = show(
                20,
                Stopped,
                &[(kind, "first\nsecond\nthird\nfourth\nfifth")],
            );
            assert_eq!(
                drawn(&t),
                [
                    "• Hook stopped",
                    "  └ first",
                    "    second",
                    "    third",
                    "    fourth",
                    "    fifth",
                ],
                "{kind:?}"
            );
        }
    }

    #[test]
    fn a_blank_line_inside_an_entry_stays_blank() {
        let t = show(40, Failed, &[(Stop, "one\n\ntwo")]);
        assert_eq!(drawn(&t), ["• Hook failed", "  └ one", "", "    two"]);
    }
}

mod collab_cell {
    use std::sync::Arc;

    use codex_tui_core::threads::{
        AgentOutcome, AgentRef, CollabEvent, SpawnRequest, SubAgentKind,
    };
    use codex_tui_rtwk::pages::session::cells::collab::CollabCell;
    use rtwk::style::{Attrs, Color};

    use AgentOutcome::{Completed, Errored, Interrupted, NotFound, PendingInit, Running, Shutdown};

    fn agent(id: &str, nickname: Option<&str>, role: Option<&str>) -> AgentRef {
        AgentRef {
            id: id.to_owned(),
            nickname: nickname.map(str::to_owned),
            role: role.map(str::to_owned),
        }
    }

    fn robie() -> AgentRef {
        agent("robie-id", Some("Robie"), Some("explorer"))
    }

    fn show(width: u16, event: CollabEvent) -> rtwk_test::Tester {
        let mut t = rtwk_test::Tester::with(rtwk_test::Options::new().size(width, 12));
        t.pump_widget(CollabCell {
            event: Arc::new(event),
        });
        t
    }

    fn rows(t: &rtwk_test::Tester) -> Vec<String> {
        let mut rows: Vec<String> = t
            .screen()
            .lines()
            .map(|line| line.trim_end().to_string())
            .collect();
        while rows.last().is_some_and(String::is_empty) {
            rows.pop();
        }
        rows
    }

    /// The column where `needle` starts on row `y`.
    fn at(t: &rtwk_test::Tester, y: usize, needle: &str) -> usize {
        let row = &rows(t)[y];
        let start = row
            .find(needle)
            .unwrap_or_else(|| panic!("`{needle}` in {row:?}"));
        row[..start].chars().count()
    }

    #[test]
    fn the_oracle_transcript_snapshot_rows() {
        // oracle snapshots/codex_tui__multi_agents__tests__collab_agent_transcript.snap
        let cases = [
            (
                CollabEvent::Spawned {
                    agent: robie(),
                    request: Some(SpawnRequest {
                        model: "gpt-5".into(),
                        effort: "high".into(),
                    }),
                    prompt: Some("Compute 11! and reply with just the integer result.".into()),
                },
                vec![
                    "• Spawned Robie [explorer] (gpt-5 high)",
                    "  └ Compute 11! and reply with just the integer result.",
                ],
            ),
            (
                CollabEvent::SentInput {
                    agent: robie(),
                    prompt: Some("Please continue and return the answer only.".into()),
                },
                vec![
                    "• Sent input to Robie [explorer]",
                    "  └ Please continue and return the answer only.",
                ],
            ),
            (
                CollabEvent::Waiting {
                    agents: vec![robie()],
                },
                vec!["• Waiting for Robie [explorer]"],
            ),
            (
                CollabEvent::FinishedWaiting {
                    agents: vec![
                        (robie(), Completed(Some("39916800".into()))),
                        (
                            agent("bob-id", Some("Bob"), Some("worker")),
                            Errored(Some("tool timeout".into())),
                        ),
                    ],
                },
                vec![
                    "• Finished waiting",
                    "  └ Robie [explorer]: Completed - 39916800",
                    "    Bob [worker]: Error - tool timeout",
                ],
            ),
            (
                CollabEvent::Closed { agent: robie() },
                vec!["• Closed Robie [explorer]"],
            ),
            (
                // collab_resume_interrupted.snap
                CollabEvent::Resumed {
                    agent: robie(),
                    outcome: Some(Interrupted),
                },
                vec!["• Resumed Robie [explorer]", "  └ Interrupted"],
            ),
        ];
        for (event, expected) in cases {
            let t = show(80, event.clone());
            assert_eq!(rows(&t), expected, "{event:?}");
        }
    }

    #[test]
    fn an_agent_without_a_nickname_is_its_thread_id_and_an_empty_role_is_no_role() {
        let t = show(
            80,
            CollabEvent::Closed {
                agent: agent(
                    "00000000-0000-0000-0000-000000000002",
                    Some("  "),
                    Some(" "),
                ),
            },
        );
        assert_eq!(rows(&t), ["• Closed 00000000-0000-0000-0000-000000000002"]);
    }

    #[test]
    fn waiting_names_nobody_one_agent_or_a_count_with_the_agents_listed() {
        assert_eq!(
            rows(&show(80, CollabEvent::Waiting { agents: vec![] })),
            ["• Waiting for agents"]
        );
        let t = show(
            80,
            CollabEvent::Waiting {
                agents: vec![robie(), agent("bob-id", Some("Bob"), None)],
            },
        );
        assert_eq!(
            rows(&t),
            ["• Waiting for 2 agents", "  └ Robie [explorer]", "    Bob"]
        );
    }

    #[test]
    fn a_wait_nobody_finished_says_so() {
        assert_eq!(
            rows(&show(80, CollabEvent::FinishedWaiting { agents: vec![] })),
            ["• Finished waiting", "  └ No agents completed yet"]
        );
    }

    #[test]
    fn a_failed_spawn_and_a_failed_resume_read_as_errors() {
        assert_eq!(
            rows(&show(
                80,
                CollabEvent::SpawnFailed {
                    prompt: Some("go".into())
                }
            )),
            ["• Agent spawn failed", "  └ go"]
        );
        assert_eq!(
            rows(&show(
                80,
                CollabEvent::Resumed {
                    agent: robie(),
                    outcome: None
                }
            )),
            [
                "• Resumed Robie [explorer]",
                "  └ Error - Agent resume failed"
            ]
        );
        assert_eq!(
            rows(&show(80, CollabEvent::Resuming { agent: robie() })),
            ["• Resuming Robie [explorer]"]
        );
    }

    #[test]
    fn every_state_has_its_words() {
        let words = [
            (PendingInit, "Pending init"),
            (Running, "Running"),
            (Interrupted, "Interrupted"),
            (Completed(None), "Completed"),
            (Errored(None), "Error - Agent errored"),
            (Errored(Some(String::new())), "Error"),
            (Shutdown, "Shutdown"),
            (NotFound, "Not found"),
        ];
        for (outcome, word) in words {
            let t = show(
                80,
                CollabEvent::FinishedWaiting {
                    agents: vec![(robie(), outcome.clone())],
                },
            );
            assert_eq!(
                rows(&t)[1],
                format!("  └ Robie [explorer]: {word}"),
                "{outcome:?}"
            );
        }
    }

    #[test]
    fn a_sub_agent_activity_names_its_path_in_backticks() {
        for (kind, text) in [
            (SubAgentKind::Started, "• Started `/root/helper`"),
            (SubAgentKind::Interacted, "• Interacted with `/root/helper`"),
            (SubAgentKind::Interrupted, "• Interrupted `/root/helper`"),
            (SubAgentKind::Completed, "• Completed `/root/helper`"),
        ] {
            let t = show(
                80,
                CollabEvent::Activity {
                    kind,
                    path: "/root/helper".into(),
                },
            );
            assert_eq!(rows(&t), [text]);
        }
    }

    #[test]
    fn the_title_styles_the_nickname_the_role_and_the_request() {
        // oracle multi_agents.rs title_styles_nickname_and_role
        let t = show(
            200,
            CollabEvent::Spawned {
                agent: robie(),
                request: Some(SpawnRequest {
                    model: "gpt-5".into(),
                    effort: "high".into(),
                }),
                prompt: None,
            },
        );
        let bullet = t.cell(0, 0);
        assert!(bullet.attrs.intersects(Attrs::DIM), "{bullet:?}");
        let verb = t.cell(2, 0);
        assert!(verb.attrs.intersects(Attrs::BOLD), "{verb:?}");
        let nickname = t.cell(at(&t, 0, "Robie"), 0);
        assert!(nickname.attrs.intersects(Attrs::BOLD), "{nickname:?}");
        let role = t.cell(at(&t, 0, "[explorer]"), 0);
        assert!(!role.attrs.intersects(Attrs::DIM | Attrs::BOLD), "{role:?}");
        assert_ne!(
            role.fg, nickname.fg,
            "the nickname is the accent, the role plain"
        );
        let request = t.cell(at(&t, 0, "(gpt-5"), 0);
        // The oracle's magenta is the theme's accent here (status_line.rs).
        assert_eq!(request.fg, nickname.fg, "{request:?}");
        assert!(
            !request.attrs.intersects(Attrs::DIM | Attrs::BOLD),
            "{request:?}"
        );
    }

    #[test]
    fn the_states_are_coloured_as_the_oracle_does() {
        let t = show(
            80,
            CollabEvent::FinishedWaiting {
                agents: vec![
                    (robie(), Completed(Some("done".into()))),
                    (agent("b", Some("B"), None), Errored(Some("bad".into()))),
                    (agent("c", Some("C"), None), NotFound),
                ],
            },
        );
        assert_eq!(t.cell(at(&t, 1, "Completed"), 1).fg, Color::GREEN);
        assert_eq!(t.cell(at(&t, 2, "Error"), 2).fg, Color::RED);
        assert_eq!(t.cell(at(&t, 3, "Not found"), 3).fg, Color::RED);
        let dash = t.cell(at(&t, 1, " - ") + 1, 1);
        assert!(dash.attrs.intersects(Attrs::DIM), "{dash:?}");
        let message = t.cell(at(&t, 1, "done"), 1);
        assert!(!message.attrs.intersects(Attrs::DIM), "{message:?}");
    }
}

mod delegated_and_generated {
    use std::sync::Arc;

    use codex_tui_core::threads::{DelegatedMessage, GeneratedImage};
    use codex_tui_rtwk::pages::session::cells::delegated::DelegatedCell;
    use codex_tui_rtwk::pages::session::cells::generated_image::GeneratedImageCell;
    use rtwk::style::{Attrs, Color};

    fn rows(t: &rtwk_test::Tester) -> Vec<String> {
        let mut rows: Vec<String> = t
            .screen()
            .lines()
            .map(|line| line.trim_end().to_string())
            .collect();
        while rows.last().is_some_and(String::is_empty) {
            rows.pop();
        }
        rows
    }

    #[test]
    #[ignore = "pending(G73): rtwk's MarkdownView draws a tight list compact, no blank row between items; the oracle's redrawable transcript spaces every item (ListSpacing::Uniform, tui markdown_render.rs:1084 start_item)"]
    fn a_delegated_message_has_an_attribution_and_an_indented_body_with_uniform_list_spacing() {
        // oracle snapshots/..delegated_message_list_spacing_in_scrollback_and_transcript.snap,
        // the "Redrawable transcript" half: the chat's.
        let mut t = rtwk_test::Tester::with(rtwk_test::Options::new().size(32, 16));
        t.pump_widget(DelegatedCell {
            message: Arc::new(DelegatedMessage {
                source: "other".into(),
                markdown: "- One\n- This item has enough words to wrap\n- Three".into(),
            }),
        });
        assert_eq!(
            rows(&t),
            [
                "• Sent by Codex from task other",
                "  • One",
                "",
                "  • This item has enough words",
                "    to wrap",
                "",
                "  • Three",
            ]
        );
        let bullet = t.cell(0, 0);
        assert!(bullet.attrs.intersects(Attrs::DIM), "{bullet:?}");
    }

    #[test]
    fn a_long_attribution_wraps_under_the_text_and_not_the_bullet() {
        let mut t = rtwk_test::Tester::with(rtwk_test::Options::new().size(20, 8));
        t.pump_widget(DelegatedCell {
            message: Arc::new(DelegatedMessage {
                source: "0123456789abcdef".into(),
                markdown: "Body".into(),
            }),
        });
        // Word wrap: `task` and the 16-column id do not fit in the 18 columns
        // beside the bullet, and the continuation sits under the text.
        assert_eq!(
            rows(&t),
            [
                "• Sent by Codex from",
                "  task",
                "  0123456789abcdef",
                "  Body"
            ]
        );
    }

    fn generated(failed: bool, detail: &str, saved: Option<&str>) -> rtwk_test::Tester {
        let mut t = rtwk_test::Tester::with(rtwk_test::Options::new().size(60, 8));
        t.pump_widget(GeneratedImageCell {
            image: Arc::new(GeneratedImage {
                failed,
                detail: detail.into(),
                saved_to: saved.map(str::to_owned),
            }),
        });
        t
    }

    #[test]
    fn a_generated_image_shows_its_prompt_and_where_it_was_saved() {
        let t = generated(
            false,
            "A tiny blue square",
            Some("file:///tmp/generated-image.png"),
        );
        assert_eq!(
            rows(&t),
            [
                "• Generated Image:",
                "  └ A tiny blue square",
                "  └ Saved to: file:///tmp/generated-image.png",
            ]
        );
        assert!(t.cell(0, 0).attrs.intersects(Attrs::DIM));
        assert!(t.cell(2, 0).attrs.intersects(Attrs::BOLD));
        assert!(
            t.cell(4, 1).attrs.intersects(Attrs::DIM),
            "the prompt is dim"
        );
        assert!(
            t.cell(4, 2).attrs.intersects(Attrs::DIM),
            "`Saved to:` is dim"
        );
        assert!(
            !t.cell(14, 2).attrs.intersects(Attrs::DIM),
            "the URL is not"
        );
    }

    #[test]
    fn a_failed_generation_has_a_red_bold_cross_and_no_saved_row_without_a_path() {
        let t = generated(true, "call-1", None);
        assert_eq!(rows(&t), ["✗ Image generation failed", "  └ call-1"]);
        let cross = t.cell(0, 0);
        assert_eq!(cross.fg, Color::RED, "{cross:?}");
        assert!(cross.attrs.intersects(Attrs::BOLD), "{cross:?}");
    }
}

mod error_and_warning {
    use codex_tui_rtwk::pages::session::cells::error::ErrorCell;
    use codex_tui_rtwk::pages::session::cells::warning::WarningCell;
    use rtwk::style::{Attrs, Color};

    fn rows(t: &rtwk_test::Tester) -> Vec<String> {
        let mut rows: Vec<String> = t
            .screen()
            .lines()
            .map(|line| line.trim_end().to_string())
            .collect();
        while rows.last().is_some_and(String::is_empty) {
            rows.pop();
        }
        rows
    }

    #[test]
    fn an_error_is_one_red_row_with_a_square_and_wraps_flush_left() {
        // oracle notices.rs new_error_event: `format!("■ {message}").red()`, a plain cell.
        let mut t = rtwk_test::Tester::with(rtwk_test::Options::new().size(40, 4));
        t.pump_widget(ErrorCell {
            message: "Quota exceeded. Check your plan and billing details.".into(),
        });
        assert_eq!(
            rows(&t),
            ["■ Quota exceeded. Check your plan and", "billing details."]
        );
        for (x, y) in [(0, 0), (2, 0), (0, 1)] {
            assert_eq!(t.cell(x, y).fg, Color::RED, "({x}, {y})");
        }
    }

    #[test]
    fn a_warning_has_a_mark_and_its_rows_wait_two_columns_in() {
        // oracle notices.rs new_warning_event: `PrefixedWrappedHistoryCell` of `⚠ ` and `  `.
        let mut t = rtwk_test::Tester::with(rtwk_test::Options::new().size(18, 4));
        t.pump_widget(WarningCell {
            message: "Heads up: the sandbox is locked now".into(),
        });
        assert_eq!(
            rows(&t),
            ["⚠ Heads up: the", "  sandbox is", "  locked now"]
        );
        let mark = t.cell(0, 0);
        assert!(mark.attrs.intersects(Attrs::BOLD), "{mark:?}");
        assert_ne!(mark.fg, Color::RED);
        // The mark and the message share the Attention tone.
        assert_eq!(t.cell(2, 0).fg, mark.fg);
        assert!(t.cell(2, 0).attrs.intersects(Attrs::BOLD));
        assert_eq!(t.cell(2, 2).fg, mark.fg);
    }
}

mod deprecation_cell {
    use codex_tui_rtwk::pages::session::cells::deprecation::DeprecationCell;
    use rtwk::style::{Attrs, Color};

    fn rows(t: &rtwk_test::Tester) -> Vec<String> {
        let mut rows: Vec<String> = t
            .screen()
            .lines()
            .map(|line| line.trim_end().to_string())
            .collect();
        while rows.last().is_some_and(String::is_empty) {
            rows.pop();
        }
        rows
    }

    fn zsh() -> DeprecationCell {
        DeprecationCell {
            summary: "`[features].shell_zsh_fork` is deprecated and ignored.".into(),
            details: Some(
                "The patched zsh backend has been removed. Shell commands now use the standard \
shell executor. Remove `shell_zsh_fork` from [features] in config.toml or profile configuration \
files."
                    .into(),
            ),
        }
    }

    #[test]
    fn the_summary_and_the_details_wrap_four_columns_short_and_flush_left() {
        // oracle snapshots/..legacy_zsh_migration_notice_snapshot.snap (transcript_lines(80)).
        let mut t = rtwk_test::Tester::with(rtwk_test::Options::new().size(80, 8));
        t.pump_widget(zsh());
        assert_eq!(
            rows(&t),
            [
                "⚠ `[features].shell_zsh_fork` is deprecated and ignored.",
                "The patched zsh backend has been removed. Shell commands now use the",
                "standard shell executor. Remove `shell_zsh_fork` from [features] in",
                "config.toml or profile configuration files.",
            ]
        );
    }

    #[test]
    fn the_mark_is_bold_red_the_summary_red_and_the_details_dim() {
        let mut t = rtwk_test::Tester::with(rtwk_test::Options::new().size(80, 8));
        t.pump_widget(zsh());
        let mark = t.cell(0, 0);
        assert_eq!(mark.fg, Color::RED);
        assert!(mark.attrs.intersects(Attrs::BOLD), "{mark:?}");
        let summary = t.cell(2, 0);
        assert_eq!(summary.fg, Color::RED);
        assert!(!summary.attrs.intersects(Attrs::BOLD), "{summary:?}");
        let details = t.cell(0, 1);
        assert!(details.attrs.intersects(Attrs::DIM), "{details:?}");
        assert_ne!(details.fg, Color::RED);
    }

    #[test]
    fn a_notice_without_details_is_the_summary_alone() {
        let mut t = rtwk_test::Tester::with(rtwk_test::Options::new().size(30, 4));
        t.pump_widget(DeprecationCell {
            summary: "Feature flag `foo`".into(),
            details: None,
        });
        assert_eq!(rows(&t), ["⚠ Feature flag `foo`"]);
    }
}

/// The notices that are a card or a refusal (oracle history_cell/tests.rs:
/// cyber policy, safety access block, update available, server version),
/// mounted at the oracle's widths and compared with its snapshots.
mod notice_cells {
    use codex_tui_core::screens::cells::{AnsweredQuestion, Daybreak, SessionInfo};
    use codex_tui_core::threads::Notice;
    use codex_tui_rtwk::pages::session::cells::notice::NoticeCell;
    use rtwk::style::{Attrs, Color};

    use super::oracle_snap;

    fn rows(t: &rtwk_test::Tester) -> Vec<String> {
        oracle_snap::screen(t)
    }

    fn show(notice: Notice, width: u16) -> rtwk_test::Tester {
        show_tall(notice, width, 14)
    }

    fn show_tall(notice: Notice, width: u16, height: u16) -> rtwk_test::Tester {
        let mut t = rtwk_test::Tester::with(rtwk_test::Options::new().size(width, height));
        t.pump_widget(NoticeCell { notice });
        t
    }

    fn snap(name: &str) -> Vec<String> {
        oracle_snap::rows(&oracle_snap::find(name))
    }

    #[test]
    fn the_cyber_policy_apply_form_at_80() {
        let t = show(Notice::CyberPolicy(Daybreak::Apply), 80);
        let expected = snap("cyber_policy_error_event_snapshot");
        // The oracle breaks the apply address after its last slash
        // (`...-for-` / `cyber/`); rtwk's Word wrap moves the whole token (G49).
        oracle_snap::assert_fragment("cyber apply", &rows(&t), 0, &expected[..3]);
    }

    #[test]
    #[ignore = "pending(G49): TextWrap::Word never breaks after a slash or a hyphen, so an address that does not fit moves whole to the next row"]
    fn the_cyber_policy_apply_form_at_80_whole() {
        let t = show(Notice::CyberPolicy(Daybreak::Apply), 80);
        oracle_snap::assert_same(
            "cyber apply",
            &rows(&t),
            &snap("cyber_policy_error_event_snapshot"),
        );
    }

    #[test]
    #[ignore = "pending(G49): TextWrap::Word never breaks after a slash or a hyphen, so an address that does not fit moves whole to the next row"]
    fn the_cyber_policy_apply_form_at_36() {
        let t = show(Notice::CyberPolicy(Daybreak::Apply), 36);
        oracle_snap::assert_same(
            "cyber narrow",
            &rows(&t),
            &snap("cyber_policy_error_event_narrow_snapshot"),
        );
    }

    #[test]
    fn the_cyber_policy_narrow_message_wraps_like_the_oracle() {
        let t = show(Notice::CyberPolicy(Daybreak::Apply), 36);
        let expected = snap("cyber_policy_error_event_narrow_snapshot");
        oracle_snap::assert_fragment("cyber narrow message", &rows(&t), 0, &expected[..6]);
    }

    #[test]
    fn the_disabled_and_enabled_forms_are_the_body_and_learn_more() {
        let expected = snap("cyber_policy_error_event_available_snapshot");
        let first = show(Notice::CyberPolicy(Daybreak::Disabled), 80);
        oracle_snap::assert_same("disabled", &rows(&first), &expected[..4]);
        let second = show(Notice::CyberPolicy(Daybreak::Enabled), 80);
        oracle_snap::assert_same("enabled", &rows(&second), &expected[5..]);
    }

    #[test]
    fn the_astra_form_names_astra() {
        let t = show(Notice::CyberPolicy(Daybreak::Astra), 80);
        oracle_snap::assert_same(
            "astra",
            &rows(&t),
            &snap("cyber_policy_error_event_astra_snapshot"),
        );
    }

    #[test]
    fn the_safety_block_is_its_explanation_the_body_and_two_actions() {
        let t = show(Notice::SafetyAccessBlock, 80);
        oracle_snap::assert_same(
            "safety",
            &rows(&t),
            &snap("safety_access_block_event_snapshot"),
        );
    }

    #[test]
    fn the_mark_is_accent_the_title_bold_the_message_dim_the_address_underlined() {
        let t = show(Notice::SafetyAccessBlock, 80);
        let mark = t.cell(0, 0);
        assert_ne!(mark.fg, Color::default(), "{mark:?}");
        let title = t.cell(3, 0);
        assert!(title.attrs.intersects(Attrs::BOLD), "{title:?}");
        let message = t.cell(2, 1);
        assert!(message.attrs.intersects(Attrs::DIM), "{message:?}");
        let label = t.cell(2, 4);
        assert!(label.attrs.intersects(Attrs::DIM), "{label:?}");
        let address = t.cell(20, 4);
        assert!(address.attrs.intersects(Attrs::UNDERLINE), "{address:?}");
    }

    fn update(command: Option<&str>) -> Notice {
        Notice::UpdateAvailable {
            current: "0.0.0".into(),
            latest: "9.9.9".into(),
            command: command.map(Into::into),
        }
    }

    #[test]
    fn the_update_card_is_a_box_as_wide_as_its_words() {
        let t = show(update(Some("pnpm add -g @openai/codex")), 110);
        oracle_snap::assert_same(
            "pnpm",
            &rows(&t),
            &snap("pnpm_update_available_history_cell_snapshot"),
        );
    }

    #[test]
    fn a_long_command_wraps_inside_the_box_to_the_width() {
        let t = show(
            update(Some(
                "sh -c 's=$(curl -fsSL https://chatgpt.com/codex/install.sh) && CODEX_NON_INTERACTIVE=1 eval \"$s\"'",
            )),
            110,
        );
        oracle_snap::assert_same(
            "standalone unix",
            &rows(&t),
            &snap("standalone_unix_update_available_history_cell_snapshot"),
        );
    }

    #[test]
    fn the_windows_command_wraps_after_its_words() {
        let t = show(
            update(Some(
                "powershell -ExecutionPolicy Bypass -c '$env:CODEX_NON_INTERACTIVE=1; irm https://chatgpt.com/codex/install.ps1 | iex'",
            )),
            110,
        );
        oracle_snap::assert_same(
            "standalone windows",
            &rows(&t),
            &snap("standalone_windows_update_available_history_cell_snapshot"),
        );
    }

    #[test]
    fn without_a_command_it_points_at_the_install_options() {
        let t = show(update(None), 110);
        let rows = rows(&t);
        assert!(
            rows[2].contains("See https://github.com/openai/codex for installation options."),
            "{rows:?}"
        );
    }

    fn version(message: &str, offer_update: bool) -> Notice {
        Notice::ServerVersion {
            message: message.into(),
            offer_update,
        }
    }

    #[test]
    fn a_connected_server_notice_is_one_warning_row() {
        let t = show(
            version(
                "A background Codex service is running v0.152.1, older than your Codex CLI v0.153.0.",
                false,
            ),
            100,
        );
        oracle_snap::assert_same(
            "connected",
            &rows(&t),
            &snap("connected_server_version_notice_snapshot"),
        );
    }

    #[test]
    fn a_local_daemon_notice_adds_the_daemon_hint_and_the_interruption_warning() {
        let t = show(
            version(
                "A background Codex service is running v0.152.1, older than your Codex CLI v0.153.0.",
                true,
            ),
            100,
        );
        oracle_snap::assert_same(
            "local daemon",
            &rows(&t),
            &snap("local_daemon_version_notice_snapshot"),
        );
    }

    fn expected(rows: &[&str]) -> Vec<String> {
        rows.iter().map(|row| (*row).to_owned()).collect()
    }

    fn interaction(command: Option<&str>, stdin: &str) -> Notice {
        Notice::TerminalInteraction {
            command: command.map(Into::into),
            stdin: stdin.into(),
        }
    }

    #[test]
    fn a_terminal_write_keeps_every_input_line_under_the_branch() {
        // oracle history_cell/tests.rs unified_exec_interaction_cell_retains_detailed_input
        let input: String = (1..=16).map(|line| format!("line {line}\n")).collect();
        let t = show_tall(interaction(Some("cat"), &input), 80, 20);
        let mut want = vec![
            "↳ Interacted with background terminal · cat".to_owned(),
            "  └ line 1".to_owned(),
        ];
        want.extend((2..=16).map(|line| format!("    line {line}")));
        let rows = rows(&t);
        oracle_snap::assert_fragment("interaction", &rows, 0, &want);
        let dim = t.cell(0, 0);
        assert!(dim.attrs.intersects(Attrs::DIM), "{dim:?}");
        let title = t.cell(3, 0);
        assert!(title.attrs.intersects(Attrs::BOLD), "{title:?}");
    }

    #[test]
    fn a_wait_is_one_bold_row() {
        // oracle unified_exec_interaction_cell_retains_detailed_wait
        let t = show(interaction(None, ""), 80);
        assert_eq!(rows(&t)[0], "• Waited for background terminal");
        let bullet = t.cell(0, 0);
        assert!(bullet.attrs.intersects(Attrs::BOLD), "{bullet:?}");
    }

    const URL_LIKE: &str =
        "example.test/api/v1/projects/alpha-team/releases/2026-02-17/builds/1234567890";

    #[test]
    fn a_long_stdin_line_wraps_under_its_branch() {
        // oracle unified_exec_interaction_details_wrap_long_input
        let t = show(interaction(Some("true"), URL_LIKE), 90);
        assert_eq!(rows(&t)[1], format!("  └ {URL_LIKE}"));
        let narrow = show(interaction(Some("true"), URL_LIKE), 24);
        let rows = rows(&narrow);
        assert!(rows[0].starts_with("↳ Interacted with"), "{rows:?}");
        // the header wraps with no indent, the input under its branch
        assert_eq!(rows[1], "background terminal ·", "{rows:?}");
        assert!(rows[3].starts_with("  └ example"), "{rows:?}");
        assert!(rows[4].starts_with("    "), "{rows:?}");
    }

    #[test]
    #[ignore = "pending(G-urltoken): TextWrap::Word breaks a token longer than the row after a hyphen or a slash; the oracle's adaptive wrap keeps a URL-like token whole on one row (clipped)"]
    fn a_url_like_stdin_token_stays_whole() {
        // oracle unified_exec_interaction_details_do_not_split_url_like_stdin_token (24 columns)
        let t = show(interaction(Some("true"), URL_LIKE), 24);
        let whole = rows(&t).iter().filter(|row| row.contains(URL_LIKE)).count();
        assert_eq!(whole, 1, "{:?}", rows(&t));
    }

    #[test]
    fn the_recap_loading_row_is_a_static_bullet_and_the_bold_title() {
        let t = show(Notice::RecapLoading, 80);
        assert_eq!(rows(&t)[0], "• Generating conversation recap…");
        let title = t.cell(3, 0);
        assert!(title.attrs.intersects(Attrs::BOLD), "{title:?}");
    }

    fn question(text: &str, answers: &[&str]) -> AnsweredQuestion {
        AnsweredQuestion {
            question: text.into(),
            secret: false,
            has_options: true,
            answers: answers.iter().map(|answer| (*answer).into()).collect(),
        }
    }

    fn questions(list: Vec<AnsweredQuestion>, interrupted: bool) -> Notice {
        Notice::Questions {
            questions: list,
            interrupted,
        }
    }

    const ASK: &str = "Which approach should we use for the next step?";

    #[test]
    fn a_completed_exchange_lists_the_answer_and_the_note() {
        // oracle request_user_input_tests.rs completed_and_interrupted_results, "completed" (40 columns)
        let t = show(
            questions(
                vec![question(
                    ASK,
                    &["Keep it small", "user_note: Preserve the behavior"],
                )],
                false,
            ),
            40,
        );
        oracle_snap::assert_fragment(
            "completed",
            &rows(&t),
            0,
            &expected(&[
                "• Questions 1/1 answered",
                "  • Which approach should we use for the",
                "    next step?",
                "    answer: Keep it small",
                "    note: Preserve the behavior",
            ]),
        );
        let label = t.cell(4, 3);
        assert!(label.attrs.intersects(Attrs::DIM), "{label:?}");
        let answer = t.cell(14, 3);
        assert_ne!(answer.fg, Color::default(), "{answer:?}");
        assert!(!answer.attrs.intersects(Attrs::DIM), "{answer:?}");
    }

    #[test]
    fn an_interrupted_exchange_marks_what_was_not_answered() {
        // the same test's "interrupted" frame
        let t = show(questions(vec![question(ASK, &[])], true), 40);
        oracle_snap::assert_fragment(
            "interrupted",
            &rows(&t),
            0,
            &expected(&[
                "• Questions 0/1 answered (interrupted)",
                "  • Which approach should we use for the",
                "    next step? (unanswered)",
                "  ↳ interrupted with 1 unanswered",
            ]),
        );
        let flag = t.cell(25, 0);
        assert_ne!(flag.fg, Color::default(), "{flag:?}");
    }

    #[test]
    fn a_secret_answer_is_bullets_and_a_note_without_options_reads_as_the_answer() {
        let mut secret = question("Token?", &["hunter2"]);
        secret.secret = true;
        let mut free = question("Why?", &["user_note: because"]);
        free.has_options = false;
        let t = show(questions(vec![secret, free], false), 60);
        oracle_snap::assert_fragment(
            "secret",
            &rows(&t),
            0,
            &expected(&[
                "• Questions 2/2 answered",
                "  • Token?",
                "    answer: ••••••",
                "  • Why?",
                "    answer: because",
            ]),
        );
    }

    #[test]
    fn the_first_session_card_is_the_help() {
        let t = show(
            Notice::SessionInfo(SessionInfo {
                help: true,
                ..SessionInfo::default()
            }),
            80,
        );
        oracle_snap::assert_fragment(
            "help",
            &rows(&t),
            0,
            &expected(&[
                "  To get started, describe a task or try one of these commands:",
                "",
                "  /init - create an AGENTS.md file with instructions for Codex",
                "  /status - show current session configuration",
                "  /permissions - choose what Codex is allowed to do",
                "  /model - choose what model and reasoning effort to use",
                "  /review - review any changes and find issues",
            ]),
        );
        let words = t.cell(10, 2);
        assert!(words.attrs.intersects(Attrs::DIM), "{words:?}");
    }

    #[test]
    fn a_tip_is_markdown_two_columns_in() {
        // oracle session_info_availability_nux_tooltip_snapshot (below the header's rows)
        let t = show(
            Notice::SessionInfo(SessionInfo {
                tip: Some("Model just became available".into()),
                ..SessionInfo::default()
            }),
            80,
        );
        oracle_snap::assert_fragment(
            "tip",
            &rows(&t),
            0,
            &expected(&["  Tip: Model just became available"]),
        );
        // oracle session_info_preserves_styled_tooltip_links (30 columns)
        let styled = show(
            Notice::SessionInfo(SessionInfo {
                tip: Some(
                    "Use **/copy** or `ctrl+y`; visit the [Codex community forum](https://example.com)."
                        .into(),
                ),
                ..SessionInfo::default()
            }),
            30,
        );
        oracle_snap::assert_fragment(
            "styled tip",
            &rows(&styled),
            0,
            &expected(&[
                "  Tip: Use /copy or ctrl+y;",
                "  visit the Codex community",
                "  forum (https://example.com).",
            ]),
        );
    }

    #[test]
    fn a_model_other_than_the_requested_one_is_a_three_row_block() {
        let t = show(
            Notice::SessionInfo(SessionInfo {
                model_changed: Some(("gpt-5".into(), "gpt-5-mini".into())),
                ..SessionInfo::default()
            }),
            80,
        );
        oracle_snap::assert_fragment(
            "model changed",
            &rows(&t),
            0,
            &expected(&["model changed:", "requested: gpt-5", "used: gpt-5-mini"]),
        );
        let title = t.cell(0, 0);
        assert!(title.attrs.intersects(Attrs::BOLD), "{title:?}");
    }
}

mod raw_mode {
    use codex_tui_core::fixtures::builders::{agent_completed, turn_completed};
    use codex_tui_rtwk::services::backend::fake::FAKE_THREAD;

    use codex_tui_core::threads::Item;
    use codex_tui_rtwk::pages::session::cells::{raw, strings};

    use crate::common::{Harness, start};

    const SOURCE: &str = "- first item\n\nfinalized **markdown**\n\n```text\ncopy me\n```";

    fn chat_with_an_answer() -> Harness {
        let mut h = start();
        h.wait_idle();
        h.fake.script_turn(vec![
            agent_completed(FAKE_THREAD, "turn-1", "item-1", SOURCE),
            turn_completed(FAKE_THREAD, "turn-1"),
        ]);
        h.t.type_text("hi");
        h.press("enter");
        h.wait_for("the answer", |s| s.contains("finalized"));
        h.wait_idle();
        h
    }

    #[test]
    fn source_lines_keep_blank_lines_and_drop_the_trailing_newline() {
        // oracle history_cell/tests.rs raw_lines_from_source_* (the lines of a source)
        assert_eq!(raw::source_lines("alpha\n\nbeta"), ["alpha", "", "beta"]);
        assert_eq!(raw::source_lines("alpha\n"), ["alpha"]);
        assert_eq!(raw::source_lines("alpha\n\n"), ["alpha", ""]);
        assert!(raw::source_lines("").is_empty());
    }

    #[test]
    fn an_item_with_a_source_has_raw_lines_and_one_without_has_none() {
        let text = |item: Item| raw::lines(&item, &strings::EN);
        let agent = Item::Agent {
            text: "- a\n\n| A |\n".into(),
            phase: None,
        };
        assert_eq!(text(agent).unwrap(), ["- a", "", "| A |"]);
        let plan = Item::Plan {
            text: "1. Inspect\n\n```sh\ncargo test\n```".into(),
        };
        assert_eq!(
            text(plan).unwrap(),
            ["1. Inspect", "", "```sh", "cargo test", "```"]
        );
        let reasoning = Item::Reasoning {
            summary: "first thought\n\nsecond thought\n".into(),
            with_raw: None,
        };
        assert_eq!(
            text(reasoning).unwrap(),
            ["first thought", "", "second thought"]
        );
    }

    #[test]
    fn alt_r_flips_an_answer_between_its_rendering_and_its_source() {
        // oracle history_cell/messages_tests.rs raw_markdown_bypasses_the_rich_render_cache,
        // history_cell/tests.rs raw_mode_toggle_transcript_snapshot (the agent cell)
        let mut h = chat_with_an_answer();
        let rich = h.t.screen();
        assert!(rich.contains("• "), "{rich}");
        assert!(!rich.contains("**markdown**"), "{rich}");
        h.press("alt+r");
        h.wait_for("the source", |s| s.contains("finalized **markdown**"));
        let raw = h.t.screen();
        let rows: Vec<&str> = raw.lines().map(str::trim_end).collect();
        let at = rows
            .iter()
            .position(|row| *row == "- first item")
            .unwrap_or_else(|| panic!("{raw}"));
        // the source lines, a blank row kept, no bullet, no fence drawn as a rule
        assert_eq!(rows[at + 1], "", "{raw}");
        assert_eq!(rows[at + 2], "finalized **markdown**", "{raw}");
        assert_eq!(rows[at + 4], "```text", "{raw}");
        assert_eq!(rows[at + 5], "copy me", "{raw}");
        assert_eq!(rows[at + 6], "```", "{raw}");
        // Alt+R is the silent path (`apply_raw_output_mode(notify: false)`): no notice
        assert!(!raw.contains("Raw output mode"), "{raw}");
        h.press("alt+r");
        h.wait_for("the rendering", |s| !s.contains("**markdown**"));
        assert!(!h.t.screen().contains("Raw output mode"));
    }

    #[test]
    fn a_prompt_is_its_text_without_the_chevron_in_raw_mode() {
        // oracle source_backed_cells_render_raw_source_without_prefix_or_style (the prompt)
        let mut h = chat_with_an_answer();
        assert!(h.t.screen().contains("› hi"), "{}", h.t.screen());
        h.press("alt+r");
        h.wait_for("raw", |s| s.lines().any(|row| row.trim_end() == "hi"));
        let screen = h.t.screen();
        assert!(!screen.contains("› hi"), "{screen}");
        assert!(screen.lines().any(|row| row.trim_end() == "hi"), "{screen}");
    }

    #[test]
    fn the_toggle_changes_the_cells_and_leaves_the_rest_of_the_page_alone() {
        // Flutter: a ListView's items rebuild when the mode flag changes, the composer
        // above/below them is a different subtree: its draft and caret stay.
        let mut h = chat_with_an_answer();
        h.t.type_text("a draft");
        h.wait_for("the draft", |s| s.contains("› a draft"));
        h.press("alt+r");
        h.wait_for("the source", |s| s.contains("finalized **markdown**"));
        assert!(h.t.screen().contains("› a draft"), "{}", h.t.screen());
        h.t.type_text("!");
        h.wait_for("the caret kept", |s| s.contains("› a draft!"));
        h.press("alt+r");
        h.wait_for("the rendering", |s| !s.contains("**markdown**"));
        assert!(h.t.screen().contains("› a draft!"), "{}", h.t.screen());
    }

    #[test]
    fn the_slash_command_and_the_key_are_one_toggle() {
        let mut h = chat_with_an_answer();
        h.t.type_text("/raw");
        h.press("enter");
        h.wait_for("raw on", |s| s.contains("finalized **markdown**"));
        h.press("alt+r");
        h.wait_for("raw off", |s| !s.contains("**markdown**"));
    }
}

mod errors_and_warnings_in_the_chat {
    use codex_app_server_protocol::{ServerNotification, WarningNotification};
    use codex_tui_core::app::BackendEv;
    use codex_tui_core::fixtures::builders::{turn_completed, turn_error, turn_failed};
    use codex_tui_rtwk::services::backend::fake::FAKE_THREAD;

    use crate::common::{Harness, start};

    fn warning(message: &str) -> BackendEv {
        BackendEv::Notification(Box::new(ServerNotification::Warning(WarningNotification {
            thread_id: Some(FAKE_THREAD.to_string()),
            message: message.to_string(),
        })))
    }

    fn run_turn(h: &mut Harness, events: Vec<BackendEv>) {
        h.wait_idle();
        h.fake.script_turn(events);
        h.t.type_text("hi");
        h.press("enter");
        h.wait_idle();
    }

    #[test]
    fn a_warning_is_hidden_in_the_chat_and_shown_by_the_detailed_transcript() {
        // oracle notices.rs new_warning_event: `visible_in_transcript: false`; its
        // `transcript_lines` (the Ctrl+T view) is `⚠ ` and the message.
        let mut h = start();
        run_turn(
            &mut h,
            vec![
                warning("Careful now."),
                turn_completed(FAKE_THREAD, "turn-1"),
            ],
        );
        assert!(!h.t.screen().contains("Careful now."), "{}", h.t.screen());
        h.press("ctrl+t");
        h.wait_for("the detailed transcript", |s| s.contains("⚠ Careful now."));
        h.press("ctrl+t");
        h.wait_for("the chat", |s| !s.contains("Careful now."));
    }

    #[test]
    fn a_turn_error_is_a_row_of_the_chat_and_there_is_no_status_banner() {
        // oracle turn_runtime.rs on_error: `add_to_history(new_error_event(message))`.
        let mut h = start();
        run_turn(
            &mut h,
            vec![
                turn_error(FAKE_THREAD, "turn-1", "Quota exceeded.", false),
                turn_failed(FAKE_THREAD, "turn-1"),
            ],
        );
        // The turn's events reach the app from the fake's thread: wait for the row.
        h.wait_for("the error row", |s| s.contains("■ Quota exceeded."));
        let screen = h.t.screen();
        assert_eq!(
            screen
                .lines()
                .filter(|row| row.trim() == "■ Quota exceeded.")
                .count(),
            1,
            "{screen}"
        );
        assert!(!screen.contains("error: Quota"), "{screen}");
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

// ---- local file links and soft breaks (oracle markdown_render.rs `soft_break`,
// `pop_link`, `pending_local_link_soft_break`; markdown_render/local_links.rs): a link to a
// local file reads its label and then the path in code style; a line break of the source
// is a line break of the screen, except before a `:` that follows such a link ----

mod local_links {
    use codex_tui_rtwk::pages::session::cells::{citation, strings};

    const CWD: Option<&str> = Some("/work/repo");

    fn show(source: &str) -> (rtwk_test::Tester, Vec<String>) {
        let mut t = rtwk_test::Tester::with(rtwk_test::Options::new().size(60, 12));
        t.pump_widget(citation::view(source, false, CWD, &strings::EN));
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
    fn a_local_link_reads_its_label_then_the_path_below_the_cwd_in_code_style() {
        let (t, rows) = show("see [the lib](/work/repo/src/lib.rs#L12C3) now");
        assert_eq!(rows, ["see the lib (src/lib.rs:12:3) now"]);
        let style = |x| {
            let cell = t.cell(x, 0);
            (cell.fg, cell.bg, cell.attrs)
        };
        assert_eq!(
            style(4),
            style(0),
            "the label is plain text, not link style"
        );
        assert_ne!(style(13), style(0), "the path is drawn as code");
        assert_eq!(style(13), style(25), "one style over the whole path");
    }

    #[test]
    fn a_local_link_with_no_label_text_reads_the_path_alone() {
        let (_, rows) = show("open [](/work/repo/a.rs:3) and [ ](~/notes)");
        assert_eq!(rows, ["open a.rs:3 and ~/notes"]);
    }

    #[test]
    fn a_local_link_is_found_through_a_reference_definition() {
        let (_, rows) = show("see [lib][l]\n\n[l]: /work/repo/src/lib.rs:7");
        assert_eq!(rows, ["see lib (src/lib.rs:7)"]);
    }

    #[test]
    fn a_web_link_and_a_relative_name_are_not_local_links() {
        let (_, rows) = show("[docs](https://example.com/docs) and [a](a.rs)");
        let text = rows.join(" ");
        assert!(text.contains("docs"), "{text}");
        assert!(!text.contains("(a.rs:"), "{text}");
    }

    #[test]
    fn a_line_break_of_the_source_is_a_line_break_of_the_screen() {
        let (_, rows) = show("first\nsecond\nthird");
        assert_eq!(rows, ["first", "second", "third"]);
        // In a quote and a list item as in a paragraph (the quote's prefix is G73's).
        let (_, rows) = show("> quoted\n> again");
        assert_eq!(rows.len(), 2, "{rows:?}");
        assert!(
            rows[0].ends_with("quoted") && rows[1].ends_with("again"),
            "{rows:?}"
        );
        let (_, rows) = show("- item\n  more");
        assert_eq!(rows.len(), 2, "{rows:?}");
        assert!(
            rows[0].ends_with("item") && rows[1].ends_with("more"),
            "{rows:?}"
        );
    }

    #[test]
    fn a_line_break_after_a_local_link_stays_inline_before_a_colon_only() {
        let (_, rows) = show("[x](/work/repo/a.rs:3)\n: why");
        assert_eq!(rows, ["x (a.rs:3): why"]);
        let (_, rows) = show("[x](/work/repo/a.rs:3)\n  : why");
        assert_eq!(rows, ["x (a.rs:3): why"]);
        let (_, rows) = show("[x](/work/repo/a.rs:3)\nwhy");
        assert_eq!(rows, ["x (a.rs:3)", "why"]);
        let (_, rows) = show("[x](https://example.com)\n: why");
        assert_eq!(rows.len(), 2, "{rows:?}");
    }

    #[test]
    fn a_line_break_inside_a_local_link_label_is_a_space() {
        let (_, rows) = show("[two\nlines](/work/repo/a.rs)");
        assert_eq!(rows, ["two lines (a.rs)"]);
    }

    #[test]
    fn a_table_cell_keeps_a_line_break_as_a_space_and_reads_its_local_link() {
        let (_, rows) = show("| f |\n|---|\n| [x](/work/repo/a.rs:1) |\n");
        let text = rows.join("\n");
        assert!(text.contains("x (a.rs:1)"), "{text}");
    }
}

// What the Markdown widget rewrites besides links (`widgets/markdown/inlines.rs`).
mod markdown_inlines {
    use codex_tui_rtwk::pages::session::cells::{citation, strings};

    fn rows(source: &str) -> Vec<String> {
        let mut t = rtwk_test::Tester::with(rtwk_test::Options::new().size(40, 8));
        t.pump_widget(citation::view(source, false, None, &strings::EN));
        let mut rows: Vec<String> = t
            .screen()
            .lines()
            .map(|line| line.trim_end().to_string())
            .collect();
        while rows.last().is_some_and(String::is_empty) {
            rows.pop();
        }
        rows
    }

    #[test]
    fn an_ordered_list_is_numbered_with_a_dot_whatever_its_source_delimiter() {
        assert_eq!(rows("1) one\n2) two"), ["1. one", "2. two"]);
        assert_eq!(rows("1. one\n2. two"), ["1. one", "2. two"]);
    }

    #[test]
    fn a_url_in_the_text_reads_with_its_character_references_decoded() {
        assert_eq!(
            rows("see https://example.com/a&amp;b&#126;c now"),
            ["see https://example.com/a&b~c now"]
        );
        // Not a reference: a name the table does not know stays as written.
        assert_eq!(
            rows("https://example.com/?a=1&b=2&nope;"),
            ["https://example.com/?a=1&b=2&nope;"]
        );
    }
}

// The user's `tui.rendering` preferences (oracle markdown_render/preferences_tests.rs): a table or
// list renderer that is off keeps the source text.
mod rendering_preferences {
    use codex_tui_core::app::facts::Rendering;
    use codex_tui_rtwk::pages::session::cells::{citation, strings};

    fn rows(source: &str, rendering: Rendering) -> Vec<String> {
        let mut t = rtwk_test::Tester::with(rtwk_test::Options::new().size(40, 14));
        t.pump_widget(citation::view_with(
            source,
            false,
            None,
            &strings::EN,
            rendering,
        ));
        let mut rows: Vec<String> = t
            .screen()
            .lines()
            // a code block is drawn one column in from the text around it: compared without
            .map(|line| line.trim().to_string())
            .collect();
        while rows.last().is_some_and(String::is_empty) {
            rows.pop();
        }
        rows
    }

    fn no_tables() -> Rendering {
        Rendering {
            tables: false,
            ..Rendering::default()
        }
    }

    fn no_lists() -> Rendering {
        Rendering {
            lists: false,
            ..Rendering::default()
        }
    }

    const TABLE: &str = "| A | B |\n|---|---|\n| **a** | `b` |\n";

    #[test]
    fn a_table_is_its_source_while_tables_are_off() {
        assert_eq!(
            rows(TABLE, no_tables()),
            ["| A | B |", "|---|---|", "| **a** | `b` |"]
        );
        // on, it is laid out as a table
        let drawn = rows(TABLE, Rendering::default());
        assert!(!drawn.iter().any(|row| row == "|---|---|"), "{drawn:?}");
    }

    #[test]
    fn the_text_around_a_literal_table_keeps_its_place() {
        let source = format!("Before.\n\n{TABLE}\nAfter.\n");
        let drawn = rows(&source, no_tables());
        let at = drawn.iter().position(|row| row == "| A | B |").unwrap();
        assert_eq!(drawn.first().map(String::as_str), Some("Before."));
        assert_eq!(drawn[at + 2], "| **a** | `b` |");
        assert_eq!(drawn.last().map(String::as_str), Some("After."));
    }

    #[test]
    fn a_markdown_fence_with_a_table_keeps_its_fence_lines() {
        let source = "```markdown\n| A | B |\n|---|---|\n| a | b |\n```\n";
        assert_eq!(
            rows(source, no_tables()),
            ["```markdown", "| A | B |", "|---|---|", "| a | b |", "```"]
        );
        // on, the table is unwrapped and laid out
        let drawn = rows(source, Rendering::default());
        assert!(!drawn.iter().any(|row| row.contains("```")), "{drawn:?}");
    }

    #[test]
    fn a_markdown_fence_without_a_table_is_an_ordinary_code_block() {
        assert_eq!(
            rows("```markdown\n**Not a table**\n```\n", no_tables()),
            ["**Not a table**"]
        );
    }

    #[test]
    fn a_fence_inside_a_list_item_is_not_wrapped_twice() {
        let source = "1.  ```md\n    | A | B |\n    |---|---|\n    | a | b |\n    ```\n";
        let drawn = rows(source, no_tables()).join("\n");
        assert_eq!(drawn.matches("```").count(), 2, "{drawn}");
        assert!(drawn.contains("```md"), "{drawn}");
        assert!(drawn.contains("| a | b |"), "{drawn}");
    }

    #[test]
    fn a_table_in_a_code_fence_is_left_as_it_is() {
        let source = "```text\n| A | B |\n|---|---|\n```\n";
        assert_eq!(
            rows(source, no_tables()),
            ["| A | B |", "|---|---|"],
            "a table in a code block is code"
        );
    }

    #[test]
    fn a_task_items_box_is_its_text_while_lists_are_off() {
        let drawn = rows("- [ ] Pending\n- [x] Done\n- Ordinary\n", no_lists());
        let text = drawn.join("\n");
        assert!(text.contains("[ ] Pending"), "{text}");
        assert!(text.contains("[x] Done"), "{text}");
        assert!(text.contains("Ordinary"), "{text}");
        // nested and quoted items too
        let nested = rows("> - [x] Quoted\n>   - [ ] Child\n", no_lists()).join("\n");
        assert!(nested.contains("[x] Quoted"), "{nested}");
        assert!(nested.contains("[ ] Child"), "{nested}");
        // on, the box takes the bullet's place and no `[ ] ` text follows it
        let on = rows("- [ ] Pending\n", Rendering::default());
        assert_ne!(on, rows("- [ ] Pending\n", no_lists()));
    }
}

// ---- the preview of an answer that is still streaming (oracle streaming/prose_preview.rs
// `rich_safe_len`, markdown_render/inline_directives.rs follow-up labels, inline_visualization.rs):
// the unfinished tail waits, a follow-up reads as its label once closed ----

mod streaming_preview {
    use codex_tui_rtwk::pages::session::cells::{citation, strings};

    fn rows(source: &str, live: bool) -> Vec<String> {
        let mut t = rtwk_test::Tester::with(rtwk_test::Options::new().size(50, 8));
        t.pump_widget(citation::view(source, live, None, &strings::EN));
        let mut rows: Vec<String> = t
            .screen()
            .lines()
            .map(|line| line.trim_end().to_string())
            .collect();
        while rows.last().is_some_and(String::is_empty) {
            rows.pop();
        }
        rows
    }

    #[test]
    fn a_live_answer_hides_the_prompt_of_an_open_follow_up() {
        let open = "Next:\n:codex-followup[Try it]{prompt=\"run the su";
        assert_eq!(rows(open, true), ["Next:"]);
        // The finished answer holds the same text: nothing is held back.
        assert_ne!(rows(open, false), ["Next:"]);
    }

    #[test]
    fn a_closed_follow_up_reads_as_its_label_and_never_shows_the_prompt() {
        let closed = "Next: :codex-followup[Try **it**]{prompt=\"run the suite\"}";
        for live in [true, false] {
            let got = rows(closed, live).join("\n");
            assert!(got.contains("Try it"), "{got}");
            assert!(!got.contains("run the suite"), "{got}");
            assert!(!got.contains("codex-followup"), "{got}");
        }
    }

    #[test]
    fn an_unfinished_link_destination_waits_for_its_closing_paren() {
        // The destination waits: only what stands before its `(` shows.
        assert_eq!(rows("See [docs](https://exam", true), ["See [docs]"]);
        assert_eq!(
            rows("See [docs](https://example.com) now", true),
            ["See docs now"]
        );
        // Finished, the unclosed destination is plain text, as the oracle's finalize shows it.
        assert_eq!(
            rows("See [docs](https://exam", false),
            ["See [docs](https://exam"]
        );
    }

    #[test]
    fn complete_lines_show_whole_and_an_unfinished_table_row_waits() {
        assert_eq!(rows("one\ntwo", true), ["one", "two"]);
        assert_eq!(rows("one\n| a | b", true), ["one"]);
    }

    #[test]
    fn a_visualization_line_reads_as_the_unavailable_note_and_code_is_untouched() {
        let got = rows("::codex-inline-vis{file=\"x.html\"}\n", true).join("\n");
        assert!(
            got.contains("Visualization unavailable on this device."),
            "{got}"
        );
        let code = rows("```\n::codex-inline-vis{file=\"x.html\"}\n```\n", false).join("\n");
        assert!(code.contains("::codex-inline-vis"), "{code}");
    }
}

// ---- MCP, dynamic and computer-activity cells in the transcript (oracle
// history_cell/{mcp,mcp_preview,dynamic,computer_activity}.rs: the compact
// rows are `compact_hyperlink_lines`; the shared control, a tap or Enter on
// the cell, reveals `transcript_hyperlink_lines`) ----

mod tool_cells_in_transcript {
    use codex_tui_core::fixtures::builders::turn_started;
    use codex_tui_rtwk::services::backend::fake::FAKE_THREAD;
    use serde_json::json;

    use crate::common::{self, Harness};

    fn item(payload: serde_json::Value) -> codex_tui_core::app::BackendEv {
        match serde_json::from_value(json!({
            "method": "item/completed",
            "params": {"threadId": FAKE_THREAD, "turnId": "turn-1", "completedAtMs": 0,
                "item": payload}
        })) {
            Ok(notification) => {
                codex_tui_core::app::BackendEv::Notification(Box::new(notification))
            }
            Err(error) => panic!("not a server notification: {error}"),
        }
    }

    fn mcp(
        id: &str,
        server: &str,
        arguments: serde_json::Value,
        rows: &[&str],
    ) -> serde_json::Value {
        let content: Vec<_> = rows
            .iter()
            .map(|text| json!({"type": "text", "text": text}))
            .collect();
        json!({"type": "mcpToolCall", "id": id, "server": server, "tool": "js",
            "status": "completed", "arguments": arguments, "appContext": null, "mcpAppUi": null,
            "pluginId": null, "readOnlyHint": null,
            "result": {"content": content, "structuredContent": null, "_meta": null,
                "isError": false},
            "error": null, "durationMs": null})
    }

    fn show(items: Vec<serde_json::Value>, until: &str) -> Harness {
        let mut h = common::start();
        h.wait_idle();
        let mut events = vec![turn_started(FAKE_THREAD, "turn-1")];
        events.extend(items.into_iter().map(item));
        h.fake.script_turn(events);
        h.t.type_text("go");
        h.press("enter");
        h.wait_for("the cells", |screen| screen.contains(until));
        h
    }

    fn row_of(h: &Harness, text: &str) -> (usize, usize) {
        let screen = h.t.screen();
        let y = screen
            .lines()
            .position(|row| row.contains(text))
            .unwrap_or_else(|| panic!("no row with {text:?}:\n{screen}"));
        let x = screen
            .lines()
            .nth(y)
            .and_then(|row| row.find(text))
            .unwrap_or_default();
        (x, y)
    }

    #[test]
    fn an_mcp_call_keeps_the_last_rows_and_a_tap_on_the_control_reveals_all() {
        let mut h = show(
            vec![mcp(
                "m1",
                "docs",
                json!({"q": "x"}),
                &["one", "two", "three", "four", "five"],
            )],
            "five",
        );
        let screen = h.t.screen();
        assert!(screen.contains("• Called docs.js"), "{screen}");
        assert!(
            !screen.contains("one") && screen.contains("three"),
            "{screen}"
        );
        assert!(screen.contains("+ Show details"), "{screen}");
        let (x, y) = row_of(&h, "+ Show details");
        h.t.click(x as u16, y as u16);
        h.wait_for("the whole result", |s| {
            s.contains("− Hide details") && s.contains("one")
        });
        assert!(
            h.t.screen().contains("docs.js({\"q\":\"x\"})"),
            "{}",
            h.t.screen()
        );
        let (x, y) = row_of(&h, "− Hide details");
        h.t.click(x as u16, y as u16);
        h.wait_for("the compact form", |s| {
            s.contains("+ Show details") && !s.contains("one")
        });
    }

    #[test]
    fn a_run_of_computer_calls_is_one_group_a_tap_on_its_header_reveals_each_call() {
        let titled =
            |id: &str, title: &str| mcp(id, "cua_repl", json!({"title": title}), &["done"]);
        let mut h = show(
            vec![
                titled("c1", "Inspect page 1"),
                titled("c2", "Inspect page 2"),
            ],
            "Inspect page 2",
        );
        let screen = h.t.screen();
        assert!(screen.contains("• Used computer · 2 actions"), "{screen}");
        assert!(
            screen.contains("├ Inspect page 1") && screen.contains("└ Inspect page 2"),
            "{screen}"
        );
        let (x, y) = row_of(&h, "Used computer");
        h.t.click(x as u16, y as u16);
        h.wait_for("each call", |s| s.contains("− Hide details"));
        let screen = h.t.screen();
        assert_eq!(screen.matches("Called cua_repl.js(").count(), 2, "{screen}");
        h.press("enter");
        h.wait_for("the group again", |s| {
            s.contains("• Used computer · 2 actions")
        });
    }

    #[test]
    fn a_dynamic_call_names_its_duration_and_the_last_rows_of_its_output() {
        let call = json!({"type": "dynamicToolCall", "id": "d1", "namespace": "example",
            "tool": "inspect", "arguments": {"path": "a"}, "status": "completed",
            "success": true, "durationMs": 25,
            "contentItems": [{"type": "inputText", "text": "r1\nr2\nr3\nr4"}]});
        let h = show(vec![call], "r4");
        let screen = h.t.screen();
        assert!(
            screen.contains("• Called example.inspect · 25ms"),
            "{screen}"
        );
        assert!(screen.contains("… 1 earlier lines hidden"), "{screen}");
        assert!(!screen.contains("r1"), "{screen}");
    }
}

// ---- the series token (owed to the usage plot: oracle plot/painting.rs, the
// terminal's cyan, magenta and green, then its own ink) ----

// ---- the separator under a finished turn follows the host's facts: the clock setting and the
// turn's runtime metrics arrive after the row is drawn and redraw it.
mod separator_facts {
    use codex_tui_core::app::{AppEv, BackendEv};
    use codex_tui_core::fixtures::builders::{agent_completed, turn_started};
    use codex_tui_core::ids::TurnId;
    use codex_tui_core::screens::ScreenEv;
    use codex_tui_core::screens::cells::{ClockFormat, Ev, RuntimeMetrics, Totals, ViewerClock};
    use codex_tui_rtwk::services::backend::fake::FAKE_THREAD;
    use serde_json::json;

    use crate::common::{self, Harness};

    /// `turn/completed` of turn-1 with its duration and end time.
    fn finished(duration_ms: i64, completed_at: i64) -> BackendEv {
        match serde_json::from_value(json!({
            "method": "turn/completed",
            "params": {"threadId": FAKE_THREAD, "turn": {
                "id": "turn-1", "rootTurnId": null, "items": [], "itemsView": "summary",
                "status": "completed", "error": null, "startedAt": null,
                "completedAt": completed_at, "durationMs": duration_ms}}
        })) {
            Ok(notification) => BackendEv::Notification(Box::new(notification)),
            Err(error) => panic!("not a server notification: {error}"),
        }
    }

    fn separator() -> Harness {
        let mut h = common::start();
        h.wait_idle();
        h.fake.script_turn(vec![
            turn_started(FAKE_THREAD, "turn-1"),
            agent_completed(FAKE_THREAD, "turn-1", "msg-1", "Done."),
            // 2000-09-06 14:32 UTC, long before the viewer's today.
            finished(125_000, 968_250_720),
        ]);
        h.t.type_text("go");
        h.press("enter");
        h.wait_for("the separator", |screen| {
            screen.contains("Worked for 2m 5s")
        });
        h
    }

    fn cells(ev: Ev) -> AppEv {
        AppEv::Screen(ScreenEv::Cells(ev))
    }

    #[test]
    fn the_clock_setting_and_the_runtime_metrics_arrive_after_the_row_and_redraw_it() {
        let mut h = separator();
        h.inject([cells(Ev::Viewer(ViewerClock {
            format: ClockFormat::TwentyFourHour,
            utc_offset: Some(0),
        }))]);
        h.wait_for("the 24-hour time with its date", |screen| {
            screen.contains("Worked for 2m 5s • Sep 6, 2000 at 14:32")
        });
        h.inject([cells(Ev::Viewer(ViewerClock {
            format: ClockFormat::TwelveHour,
            utc_offset: Some(0),
        }))]);
        h.wait_for("the 12-hour time", |screen| {
            screen.contains("Worked for 2m 5s • Sep 6, 2000 at 2:32 PM")
        });
        h.inject([cells(Ev::RuntimeMetrics {
            turn: TurnId::new("turn-1"),
            metrics: RuntimeMetrics {
                tool_calls: Totals {
                    count: 1,
                    duration_ms: 5,
                },
                ..RuntimeMetrics::default()
            },
        })]);
        h.wait_for("the metrics after the time", |screen| {
            screen.contains("at 2:32 PM • Local tools: 1 call (5ms)")
        });
    }
}

mod series_token {
    use std::rc::Rc;

    use codex_tui_rtwk::app::theme::Tokens;
    use rtwk::prelude::ThemeData;
    use rtwk::style::Style;

    #[test]
    fn the_series_are_the_info_accent_and_success_hues_then_the_terminals_ink() {
        let tokens = Tokens::new(Rc::new(ThemeData::DARK));
        assert_eq!(tokens.series(0), tokens.info());
        assert_eq!(tokens.series(1), tokens.accent());
        assert_eq!(tokens.series(2), tokens.success());
        assert_eq!(tokens.series(3), Style::new());
        assert_eq!(tokens.series(9), Style::new());
    }
}

/// The assistant's and the user's message cells at the widths the oracle's
/// `wrapped_and_prefixed_cells_handle_tiny_widths` draws them at: the mark is
/// cut when the cell is narrower than it, never an overflow (the cells alone;
/// the chat's composer and status rows are not in the way here). Width 3 is
/// left out: the oracle's wrap width never goes below one column (`› t`), the
/// port's body loses its column to the free column on its right (`›`).
mod message_cells_at_tiny_widths {
    use codex_tui_core::threads::UserPrompt;
    use codex_tui_rtwk::pages::session::cells::strings::EN as CELLS;
    use codex_tui_rtwk::pages::session::cells::{agent, user};
    use codex_tui_rtwk::pages::session::strings::EN as CHAT;
    use rtwk::style::Style;
    use rtwk_rich::markdown::MarkdownTheme;

    fn rows(t: &rtwk_test::Tester) -> Vec<String> {
        t.screen()
            .lines()
            .map(|line| line.trim_end().to_string())
            .collect()
    }

    #[test]
    fn an_agent_message_narrower_than_its_bullet_shows_the_bullet_cut() {
        let theme = MarkdownTheme::default();
        for (width, first) in [(1, "•"), (2, "•"), (4, "• t")] {
            let mut t = rtwk_test::Tester::with(rtwk_test::Options::new().size(width, 6));
            t.pump_widget(agent::message(
                "tiny width agent line",
                false,
                None,
                &agent::Look {
                    bullet: "• ",
                    bullet_style: Style::new(),
                    markdown: &theme,
                    cells: &CELLS,
                },
            ));
            assert_eq!(rows(&t)[0], first, "width {width}");
        }
    }

    #[test]
    fn a_user_prompt_narrower_than_its_mark_shows_the_mark_cut() {
        let tokens = codex_tui_rtwk::app::theme::Tokens::new(std::rc::Rc::new(
            rtwk::prelude::ThemeData::DARK,
        ));
        let theme = MarkdownTheme::default();
        let prompt = UserPrompt {
            text: "tiny width user line".into(),
            ..UserPrompt::default()
        };
        for (width, first) in [(1, "›"), (2, "›"), (4, "› t")] {
            let mut t = rtwk_test::Tester::with(rtwk_test::Options::new().size(width, 8));
            t.pump_widget(user::panel(
                &prompt,
                &user::Look {
                    surface: Style::new(),
                    tokens: &tokens,
                    chat: &CHAT,
                    cells: &CELLS,
                    markdown: &theme,
                    highlighted: false,
                },
                false,
            ));
            // the cell's blank row of the surface above the mark
            assert_eq!(rows(&t)[1], first, "width {width}");
        }
    }
}

/// The text contract (`cells::text`): the texts the app keeps of an item
/// (what `ListView::item_text` is given) are the strings of the `Text`s its cell builds, in
/// tree order, joined by line breaks; what a `SelectionArea` over the mounted
/// cell selects. Gutters (a bullet or prompt mark of its own `Text`, the
/// disclosure control) are left out of the selection by `ExcludeSelection`,
/// so they are not in it.
mod text_contract {
    use std::rc::Rc;
    use std::sync::Arc;

    use codex_tui_core::screens::cells::Daybreak;
    use codex_tui_core::screens::chat::{McpWarning, Nudge, NudgeOutcome};
    use codex_tui_core::threads::{
        GeneratedImage, HookEntry, HookEntryKind, HookRun, HookStatus, Item, Notice, UserPrompt,
        WebAction, WebActivity,
    };
    use codex_tui_rtwk::pages::session::cells::strings::EN as CELLS;
    use codex_tui_rtwk::pages::session::cells::text::{Env, Gap, Texts, texts};
    use codex_tui_rtwk::pages::session::cells::{
        CellEnv, Place, agent, deprecation, error, generated_image, hook, note, notice, user,
        viewed_image, warning, web,
    };
    use codex_tui_rtwk::pages::session::strings::EN as CHAT;
    use rtwk::AnyWidget;
    use rtwk::gestures::{SelectionArea, SelectionAreaController};
    use rtwk::style::Style;
    use rtwk_core::input::{Mods, MouseKind};
    use rtwk_rich::markdown::MarkdownTheme;

    const WIDTH: u16 = 60;
    const HEIGHT: u16 = 24;

    fn env(detailed: bool, expanded: bool) -> Env<'static> {
        Env {
            cells: &CELLS,
            chat: &CHAT,
            detailed,
            expanded,
        }
    }

    fn widget(item: &Item, detailed: bool) -> AnyWidget {
        let tokens =
            codex_tui_rtwk::app::theme::Tokens::new(Rc::new(rtwk::prelude::ThemeData::DARK));
        let theme = MarkdownTheme::default();
        match item {
            Item::User(prompt) => user::panel(
                prompt,
                &user::Look {
                    surface: Style::new(),
                    tokens: &tokens,
                    chat: &CHAT,
                    cells: &CELLS,
                    markdown: &theme,
                    highlighted: false,
                },
                false,
            )
            .into(),
            Item::Agent { text, .. } => agent::message(
                text,
                false,
                None,
                &agent::Look {
                    bullet: "• ",
                    bullet_style: Style::new(),
                    markdown: &theme,
                    cells: &CELLS,
                },
            ),
            Item::Error { message } => error::ErrorCell {
                message: message.clone(),
            }
            .into(),
            Item::HighLoad => error::ErrorCell {
                message: CHAT.high_load.to_string(),
            }
            .into(),
            Item::Warning { message } => warning::WarningCell {
                message: message.clone(),
            }
            .into(),
            Item::StartupWarning(w) => warning::WarningCell {
                message: CHAT.mcp_warning(w),
            }
            .into(),
            Item::Deprecation { summary, details } => deprecation::DeprecationCell {
                summary: summary.clone(),
                details: details.clone(),
            }
            .into(),
            // the chat's end of a review is a cell of its own (no review text)
            Item::Notice(Notice::ExitedReview(_)) if !detailed => notice::view(
                item,
                &CellEnv {
                    tokens: &tokens,
                    markdown: &theme,
                    prompt_surface: &Style::new(),
                    chat: &CHAT,
                    cells: &CELLS,
                    bullet: "• ",
                    bullet_style: &Style::new(),
                    cwd: None,
                    detailed,
                    raw: false,
                    show_raw_reasoning: false,
                },
                &Place::default(),
            ),
            Item::Notice(n) => notice::NoticeCell { notice: n.clone() }.into(),
            Item::Hook(run) => hook::HookCell { run: run.clone() }.into(),
            Item::Note(note) => note::NoteCell { note: note.clone() }.into(),
            Item::Generated(image) => generated_image::GeneratedImageCell {
                image: image.clone(),
            }
            .into(),
            Item::Web(activity) => web::WebCell {
                activity: activity.clone(),
                detailed,
            }
            .into(),
            Item::ImageView { path, name } => viewed_image::ViewedImageCell {
                name: name.clone(),
                path: path.clone(),
                detailed,
            }
            .into(),
            other => panic!("no widget in this test for {other:?}"),
        }
    }

    /// What a drag over the whole mounted cell selects, as the area gives it.
    fn selected(widget: AnyWidget) -> String {
        let selection = SelectionAreaController::new();
        let mut t = rtwk_test::Tester::with(rtwk_test::Options::new().size(WIDTH, HEIGHT));
        t.pump_widget(
            SelectionArea::new(widget)
                .trim_trailing_blanks(false)
                .controller(selection.clone()),
        );
        t.mouse(MouseKind::Down, 0, 0, 0, Mods::NONE);
        t.mouse(MouseKind::Drag, WIDTH - 1, HEIGHT - 1, 0, Mods::NONE);
        t.mouse(MouseKind::Up, WIDTH - 1, HEIGHT - 1, 0, Mods::NONE);
        t.pump();
        selection.text()
    }

    #[track_caller]
    fn agrees(item: Item) {
        agrees_in(item, false);
    }

    #[track_caller]
    fn agrees_in(item: Item, detailed: bool) {
        let texts = texts(&item, &env(detailed, false));
        let Some(want) = texts.as_mounted() else {
            panic!("{item:?}: the contract names no texts for it yet: {texts:?}");
        };
        assert_eq!(want, selected(widget(&item, detailed)), "{item:?}");
    }

    fn prompt(text: &str) -> UserPrompt {
        UserPrompt {
            text: text.into(),
            ..UserPrompt::default()
        }
    }

    fn hook_run(status: HookStatus, entries: &[(HookEntryKind, &str)]) -> Arc<HookRun> {
        Arc::new(HookRun {
            status,
            entries: entries
                .iter()
                .map(|(kind, text)| HookEntry {
                    kind: *kind,
                    text: (*text).into(),
                })
                .collect(),
        })
    }

    #[test]
    fn a_blank_prompt_draws_no_rows_and_a_prompt_with_text_does() {
        // oracle `UserHistoryCell`: a message of spaces has no lines at all, not even the pad
        let tokens =
            codex_tui_rtwk::app::theme::Tokens::new(Rc::new(rtwk::prelude::ThemeData::DARK));
        let theme = MarkdownTheme::default();
        let rows = |text: &str| {
            let item = Item::User(prompt(text));
            let cell = user::view(
                &item,
                &CellEnv {
                    tokens: &tokens,
                    markdown: &theme,
                    prompt_surface: &Style::new(),
                    chat: &CHAT,
                    cells: &CELLS,
                    bullet: "• ",
                    bullet_style: &Style::new(),
                    cwd: None,
                    detailed: false,
                    raw: false,
                    show_raw_reasoning: false,
                },
                &Place {
                    separator: true,
                    ..Place::default()
                },
            );
            let mut t = rtwk_test::Tester::with(rtwk_test::Options::new().size(WIDTH, HEIGHT));
            t.pump_widget(
                rtwk::layout::Column::new()
                    .main_axis_size(rtwk::layout::MainAxisSize::Min)
                    .child(cell)
                    .child(rtwk::widgets::Text::new("end")),
            );
            t.lines()
                .iter()
                .position(|line| line.trim() == "end")
                .expect("the marker is drawn")
        };
        assert_eq!(rows("   \n "), 0);
        // the separator, the pad, the text, the pad
        assert_eq!(rows("hello"), 4);
    }

    #[test]
    fn a_users_prompt_is_its_text_without_the_mark() {
        agrees(Item::User(prompt("fix the build\nthen the tests")));
        agrees(Item::User(UserPrompt {
            text: "look at [Image #1] and @src/main.rs".into(),
            elements: vec![8..18, 23..34],
            images: vec![1],
        }));
        agrees(Item::User(UserPrompt {
            text: "".into(),
            elements: vec![],
            images: vec![1, 2],
        }));
    }

    #[test]
    fn a_note_is_one_text_with_its_mark_in_front() {
        use codex_tui_core::threads::{Note, NoteKey, Tone};
        for (tone, key) in [
            (Tone::Info, NoteKey::WorkingDirectory("/work".into())),
            (Tone::Info, NoteKey::DesktopOpened),
            (Tone::Done, NoteKey::ServerDone("Recovered".into())),
            (Tone::Error, NoteKey::Rollout(None)),
        ] {
            agrees(Item::Note(Note { tone, key }));
        }
    }

    #[test]
    fn the_error_family_is_one_text_each() {
        agrees(Item::Error {
            message: "stream closed".into(),
        });
        agrees(Item::HighLoad);
        agrees(Item::Warning {
            message: "careful now".into(),
        });
        agrees(Item::StartupWarning(McpWarning::Failed {
            server: "docs".into(),
            error: None,
        }));
        agrees(Item::StartupWarning(McpWarning::Incomplete {
            servers: vec!["a".into(), "b".into()],
        }));
    }

    #[test]
    fn a_deprecation_is_the_summary_row_then_the_details() {
        agrees(Item::Deprecation {
            summary: "old flag".into(),
            details: None,
        });
        agrees(Item::Deprecation {
            summary: "old flag".into(),
            details: Some("use the new one".into()),
        });
    }

    #[test]
    fn notices_are_their_row() {
        agrees(Item::Notice(Notice::ContextCompacted { took: None }));
        agrees(Item::Notice(Notice::ContextCompacted { took: Some(83) }));
        agrees(Item::Notice(Notice::EnteredReview("the diff".into())));
        // the chat's end of a review has no review text; the detailed transcript's names it
        let finished = Item::Notice(Notice::ExitedReview("the diff".into()));
        agrees(finished.clone());
        agrees_in(finished.clone(), true);
        assert_eq!(
            texts(&finished, &env(false, false)).as_mounted().as_deref(),
            Some("<< Code review finished >>")
        );
        assert_eq!(
            texts(&finished, &env(true, false)).as_mounted().as_deref(),
            Some("<< Code review finished: the diff >>")
        );
        for outcome in [
            NudgeOutcome::Sent,
            NudgeOutcome::Cooldown,
            NudgeOutcome::Failed,
        ] {
            for nudge in [Nudge::Credits, Nudge::UsageLimit] {
                agrees(Item::Notice(Notice::AddCreditsNudge { nudge, outcome }));
            }
        }
    }

    #[test]
    fn a_refusal_a_card_and_a_version_notice_agree_with_their_widget() {
        agrees(Item::Notice(Notice::SafetyAccessBlock));
        for kind in [
            Daybreak::Apply,
            Daybreak::Disabled,
            Daybreak::Astra,
            Daybreak::Enabled,
        ] {
            agrees(Item::Notice(Notice::CyberPolicy(kind)));
        }
        agrees(Item::Notice(Notice::UpdateAvailable {
            current: "0.0.0".into(),
            latest: "9.9.9".into(),
            command: Some("pnpm add -g @openai/codex".into()),
        }));
        agrees(Item::Notice(Notice::UpdateAvailable {
            current: "0.0.0".into(),
            latest: "9.9.9".into(),
            command: None,
        }));
        for offer_update in [false, true] {
            agrees(Item::Notice(Notice::ServerVersion {
                message: "A background Codex service is running v1.".into(),
                offer_update,
            }));
        }
    }

    #[test]
    fn a_terminal_write_a_recap_a_question_result_and_the_card_agree_with_their_widget() {
        use codex_tui_core::screens::cells::{AnsweredQuestion, SessionInfo};
        agrees(Item::Notice(Notice::TerminalInteraction {
            command: Some("cat".into()),
            stdin: "one\ntwo\nthree\n".into(),
        }));
        agrees(Item::Notice(Notice::TerminalInteraction {
            command: None,
            stdin: String::new(),
        }));
        agrees(Item::Notice(Notice::RecapLoading));
        let ask = |answers: &[&str], secret: bool| AnsweredQuestion {
            question: "Which one?".into(),
            secret,
            has_options: true,
            answers: answers.iter().map(|answer| (*answer).into()).collect(),
        };
        for interrupted in [false, true] {
            agrees(Item::Notice(Notice::Questions {
                questions: vec![
                    ask(&["a", "user_note: because"], false),
                    ask(&[], false),
                    ask(&["x"], true),
                ],
                interrupted,
            }));
        }
        agrees(Item::Notice(Notice::SessionInfo(SessionInfo {
            help: true,
            ..SessionInfo::default()
        })));
        agrees(Item::Notice(Notice::SessionInfo(SessionInfo {
            model_changed: Some(("a".into(), "b".into())),
            ..SessionInfo::default()
        })));
    }

    #[test]
    fn a_tip_makes_the_card_a_markdown_gap() {
        use codex_tui_core::screens::cells::SessionInfo;
        let item = Item::Notice(Notice::SessionInfo(SessionInfo {
            tip: Some("Use **/copy**".into()),
            ..SessionInfo::default()
        }));
        assert!(matches!(
            texts(&item, &env(false, false)),
            Texts::Gap(Gap::Markdown)
        ));
    }

    #[test]
    fn a_hook_is_its_rows_a_blank_one_included() {
        agrees(Item::Hook(hook_run(
            HookStatus::Completed,
            &[(HookEntryKind::Warning, "all good\n")],
        )));
        agrees(Item::Hook(hook_run(
            HookStatus::Failed,
            &[
                (HookEntryKind::Warning, "bad\nthing"),
                (HookEntryKind::Error, "first\n\nsecond"),
            ],
        )));
        agrees(Item::Hook(hook_run(HookStatus::Blocked, &[])));
    }

    #[test]
    fn a_generated_image_is_the_heading_the_detail_and_where_it_went() {
        agrees(Item::Generated(Arc::new(GeneratedImage {
            failed: false,
            detail: "a red fox".into(),
            saved_to: Some("file:///tmp/fox.png".into()),
        })));
        agrees(Item::Generated(Arc::new(GeneratedImage {
            failed: true,
            detail: "no quota".into(),
            saved_to: None,
        })));
    }

    #[test]
    fn a_web_row_reads_as_drawn() {
        for (completed, action, detail) in [
            (true, WebAction::Search { known: true }, "rust async"),
            (false, WebAction::Search { known: false }, ""),
            (true, WebAction::OpenPage, "https://example.com"),
            (false, WebAction::OpenPage, ""),
            (true, WebAction::FindInPage { pattern: true }, "'x' in y"),
            (true, WebAction::FindInPage { pattern: false }, "y"),
        ] {
            for detailed in [false, true] {
                agrees_in(
                    Item::Web(WebActivity {
                        completed,
                        action,
                        detail: detail.into(),
                    }),
                    detailed,
                );
            }
        }
    }

    #[test]
    fn a_viewed_image_in_the_detailed_transcript_is_the_bullet_and_the_full_path() {
        agrees_in(
            Item::ImageView {
                path: "/workspace/assets/example.png".into(),
                name: "example.png".into(),
            },
            true,
        );
    }

    /// A drag over a disclosing cell is taken by its control (the tap opens
    /// it), so what it selects is not read here: the rows it shows hold the
    /// parts, in order, and the control is the part that is a gutter.
    #[test]
    fn a_viewed_image_with_its_control_shows_the_row_then_the_control() {
        let item = Item::ImageView {
            path: "/workspace/assets/example.png".into(),
            name: "example.png".into(),
        };
        let Texts::Exact(parts) = texts(&item, &env(false, false)) else {
            panic!("exact");
        };
        let mut t = rtwk_test::Tester::with(rtwk_test::Options::new().size(WIDTH, HEIGHT));
        t.pump_widget(widget(&item, false));
        let rows: Vec<String> = t
            .screen()
            .lines()
            .map(|line| line.trim_end().to_string())
            .filter(|line| !line.is_empty())
            .collect();
        assert_eq!(parts.len(), rows.len(), "{rows:?}");
        for (part, row) in parts.iter().zip(&rows) {
            assert!(
                row.trim_start().starts_with(part.text.trim_start()),
                "{part:?} in {row:?}"
            );
        }
        assert!(!parts[0].gutter && parts[1].gutter);
    }

    /// The texts of a Markdown body are a declared gap (pack: "G-new text of
    /// Markdown"): rtwk gives no function of the source for the strings a
    /// `MarkdownView` under a `SelectionArea` selects. The day it does, this
    /// test turns into `agrees(Item::Agent { .. })` for each of these.
    #[test]
    fn a_markdown_body_is_a_declared_gap() {
        let md = "Hello **bold**\n\n- one\n- two";
        for item in [
            Item::Agent {
                text: md.into(),
                phase: None,
            },
            Item::Reasoning {
                summary: md.into(),
                with_raw: None,
            },
            Item::Plan { text: md.into() },
            Item::User(prompt(md)),
        ] {
            assert_eq!(
                texts(&item, &env(false, false)),
                Texts::Gap(Gap::Markdown),
                "{item:?}"
            );
        }
    }
}
