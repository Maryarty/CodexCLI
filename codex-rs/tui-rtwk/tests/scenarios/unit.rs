//! The harness's own tests: the normalizer, the script parser, the
//! comparison. No binary and no pty.

use std::time::Duration;

use crate::support::compare::{Finding, Report, compare, side_by_side};
use crate::support::normalize::Normalize;
use crate::support::pty::Shot;
use crate::support::script::{Step, key_bytes, parse};

fn shot(rows: &[&str], cursor: (usize, usize)) -> Shot {
    Shot {
        rows: rows.iter().map(|r| (*r).to_owned()).collect(),
        cursor,
        cursor_visible: true,
        ..Shot::default()
    }
}

// ---- the normalizer

#[test]
fn timers_become_a_placeholder() {
    let n = Normalize::new();
    assert_eq!(
        n.row("Working (12s • esc to interrupt)"),
        "Working (<t> • esc to interrupt)"
    );
    assert_eq!(n.row("Worked for 1m 23s"), "Worked for <t>");
    assert_eq!(n.row("took 100ms"), "took <t>");
    assert_eq!(n.row("3d ago"), "<t> ago");
}

#[test]
fn clock_times_become_a_placeholder() {
    let n = Normalize::new();
    assert_eq!(n.row("Worked for 1s • 11:59"), "Worked for <t> • <clock>");
    assert_eq!(n.row("at 9:05 and 23:40."), "at <clock> and <clock>.");
    assert_eq!(n.row("ratio 1:2 or 12:345"), "ratio 1:2 or 12:345");
}

#[test]
fn text_that_only_looks_like_a_timer_stays() {
    let n = Normalize::new();
    for row in [
        "w0001 w0002",
        "END-text-100",
        "gpt-5.4 high",
        "5 files",
        "x12s",
    ] {
        assert_eq!(n.row(row), row);
    }
}

#[test]
fn ids_versions_and_spinners_become_placeholders() {
    let n = Normalize::new();
    assert_eq!(
        n.row("codex resume 5e4c0001-0000-4000-8000-000000000001 now"),
        "codex resume <uuid> now"
    );
    assert_eq!(n.row("OpenAI Codex (v0.170.0)"), "OpenAI Codex (vX)");
    assert_eq!(n.row("⠋ Working"), "<spin> Working");
}

#[test]
fn literals_replace_paths_and_urls_and_rows_lose_trailing_blanks() {
    let n = Normalize::new()
        .literal("/private/var/x/rtwk-scn-1", "<ROOT>")
        .literal("http://127.0.0.1:5555", "<MOCK>");
    assert_eq!(
        n.row("cwd /private/var/x/rtwk-scn-1/work  via http://127.0.0.1:5555/v1   "),
        "cwd <ROOT>/work  via <MOCK>/v1"
    );
}

#[test]
fn a_shot_is_normalized_row_by_row_and_in_its_scrollback() {
    let mut s = shot(&["a 5s  ", "b"], (1, 0));
    s.scrollback = vec!["old 7s".to_owned()];
    let out = Normalize::new().shot(&s);
    assert_eq!(out.rows, ["a <t>", "b"]);
    assert_eq!(out.scrollback, ["old <t>"]);
    assert_eq!(out.cursor, (1, 0));
}

// ---- the script

const FILE: &str = r#"
{"scenario":"demo","mock":"text-100","size":[100,30],"seed":"agents-3","oracle_args":["agents","--no-daemon"],"rtwk_args":["agents"],"scrollback":true,"timeout_ms":3000}
{"wait":"›"}
{"send":"hi\r"}
{"key":"ctrl-c"}
{"paste":"two\nlines"}
{"resize":[80,24]}
{"sleep_ms":300}
{"check":"first","until":"done"}
{"check":"second","gap":"VS2: chat"}
{"wait_exit":true,"gap":"VS2: exit"}
"#;

#[test]
fn a_scenario_file_parses_to_its_header_and_steps() {
    let scn = parse(FILE).expect("parses");
    let h = &scn.header;
    assert_eq!(h.name, "demo");
    assert_eq!(h.mock, "text-100");
    assert_eq!(h.seed.as_deref(), Some("agents-3"));
    assert_eq!(h.size, (100, 30));
    assert_eq!(h.oracle_args, ["agents", "--no-daemon"]);
    assert_eq!(h.rtwk_args, ["agents"]);
    assert!(h.scrollback);
    assert_eq!(h.timeout, Duration::from_secs(3));
    assert_eq!(
        scn.steps,
        vec![
            Step::Wait("›".into()),
            Step::Send(b"hi\r".to_vec()),
            Step::Send(vec![3]),
            Step::Paste("two\nlines".into()),
            Step::Resize(80, 24),
            Step::Sleep(Duration::from_millis(300)),
            Step::Check {
                name: "first".into(),
                until: Some("done".into()),
                gap: None
            },
            Step::Check {
                name: "second".into(),
                until: None,
                gap: Some("VS2: chat".into())
            },
            Step::WaitExit {
                gap: Some("VS2: exit".into())
            },
        ]
    );
}

#[test]
fn header_defaults() {
    let scn = parse(r#"{"scenario":"d","mock":"text-100"}"#).expect("parses");
    let h = &scn.header;
    assert_eq!(h.size, (100, 30));
    assert!(h.oracle_args.is_empty() && h.rtwk_args.is_empty());
    assert!(!h.scrollback && h.seed.is_none());
    assert_eq!(h.timeout, Duration::from_secs(20));
    assert!(scn.steps.is_empty());
}

#[test]
fn parse_errors_name_the_line() {
    let err =
        parse("{\"scenario\":\"d\",\"mock\":\"m\"}\n{\"key\":\"nope\"}").expect_err("bad key");
    assert!(err.contains("line 2") && err.contains("nope"), "{err}");
    let err = parse("{\"scenario\":\"d\",\"mock\":\"m\"}\n{\"frob\":1}").expect_err("bad step");
    assert!(err.contains("line 2"), "{err}");
    let err = parse("{\"wait\":\"x\"}").expect_err("no header");
    assert!(err.contains("line 1") && err.contains("scenario"), "{err}");
    let err = parse("{\"scenario\":\"d\",\"mock\":\"m\"}\nnot json").expect_err("not json");
    assert!(err.contains("line 2"), "{err}");
}

#[test]
fn named_keys_are_the_bytes_a_terminal_sends() {
    let k = |name| key_bytes(name).expect(name);
    assert_eq!(k("enter"), b"\r");
    assert_eq!(k("esc"), b"\x1b");
    assert_eq!(k("tab"), b"\t");
    assert_eq!(k("backtab"), b"\x1b[Z");
    assert_eq!(k("backspace"), b"\x7f");
    assert_eq!(k("space"), b" ");
    assert_eq!(k("up"), b"\x1b[A");
    assert_eq!(k("down"), b"\x1b[B");
    assert_eq!(k("right"), b"\x1b[C");
    assert_eq!(k("left"), b"\x1b[D");
    assert_eq!(k("home"), b"\x1b[H");
    assert_eq!(k("end"), b"\x1b[F");
    assert_eq!(k("pageup"), b"\x1b[5~");
    assert_eq!(k("pagedown"), b"\x1b[6~");
    assert_eq!(k("delete"), b"\x1b[3~");
    assert_eq!(k("ctrl-c"), [3]);
    assert_eq!(k("ctrl-d"), [4]);
    assert_eq!(k("alt-b"), b"\x1bb");
    assert!(key_bytes("hyper-q").is_err());
}

// ---- the comparison

#[test]
fn equal_shots_have_no_differences() {
    let a = shot(&["one", "two"], (3, 1));
    assert!(compare(&a, &a.clone(), &Normalize::new(), false).is_empty());
}

#[test]
fn rows_are_compared_after_normalizing_and_a_changed_row_is_named() {
    let a = shot(&["Working 3s", "same", "left"], (0, 0));
    let b = shot(&["Working 9s", "same", "right"], (0, 0));
    let diffs = compare(&a, &b, &Normalize::new(), false);
    assert_eq!(diffs.len(), 1, "{diffs:?}");
    assert!(
        diffs[0].contains("row 3") && diffs[0].contains("left") && diffs[0].contains("right"),
        "{diffs:?}"
    );
}

#[test]
fn the_cursor_and_the_row_count_are_compared() {
    let a = shot(&["x"], (1, 0));
    let b = shot(&["x"], (2, 0));
    let diffs = compare(&a, &b, &Normalize::new(), false);
    assert!(diffs.iter().any(|d| d.contains("cursor")), "{diffs:?}");
    let c = shot(&["x", "y"], (1, 0));
    let diffs = compare(&a, &c, &Normalize::new(), false);
    assert!(diffs.iter().any(|d| d.contains("rows")), "{diffs:?}");
    let mut hidden = a.clone();
    hidden.cursor_visible = false;
    let diffs = compare(&a, &hidden, &Normalize::new(), false);
    assert!(diffs.iter().any(|d| d.contains("cursor")), "{diffs:?}");
}

/// A cursor that is hidden on both sides is nowhere: where the terminal
/// parked it is not compared (the oracle and codex-rtwk park it differently
/// when they hide it).
#[test]
fn a_cursor_hidden_on_both_sides_is_not_compared() {
    let mut a = shot(&["x"], (1, 0));
    let mut b = shot(&["x"], (7, 3));
    a.cursor_visible = false;
    b.cursor_visible = false;
    let diffs = compare(&a, &b, &Normalize::new(), false);
    assert!(diffs.is_empty(), "{diffs:?}");
}

/// Visible on one side only is still a difference, and so is a different
/// position while both are visible.
#[test]
fn a_cursor_visible_on_one_side_or_at_another_place_differs() {
    let a = shot(&["x"], (1, 0));
    let mut hidden = shot(&["x"], (7, 3));
    hidden.cursor_visible = false;
    let diffs = compare(&a, &hidden, &Normalize::new(), false);
    assert!(diffs.iter().any(|d| d.contains("cursor")), "{diffs:?}");
    let diffs = compare(&hidden, &a, &Normalize::new(), false);
    assert!(diffs.iter().any(|d| d.contains("cursor")), "{diffs:?}");
    let moved = shot(&["x"], (2, 0));
    let diffs = compare(&a, &moved, &Normalize::new(), false);
    assert!(diffs.iter().any(|d| d.contains("cursor")), "{diffs:?}");
}

#[test]
fn the_scrollback_counts_only_when_asked() {
    let mut a = shot(&["x"], (0, 0));
    let mut b = a.clone();
    a.scrollback = vec!["old".into()];
    b.scrollback = vec!["older".into()];
    assert!(compare(&a, &b, &Normalize::new(), false).is_empty());
    let diffs = compare(&a, &b, &Normalize::new(), true);
    assert!(diffs.iter().any(|d| d.contains("scrollback")), "{diffs:?}");
}

#[test]
fn side_by_side_puts_the_oracle_left_and_marks_differing_rows() {
    let a = shot(&["same", "left"], (0, 0));
    let b = shot(&["same", "right"], (0, 0));
    let text = side_by_side(&a, &b, &Normalize::new());
    let rows: Vec<&str> = text.lines().collect();
    assert!(
        rows.iter()
            .any(|r| r.starts_with("  same") && r.contains("| same")),
        "{text}"
    );
    assert!(
        rows.iter()
            .any(|r| r.starts_with("! left") && r.contains("| right")),
        "{text}"
    );
}

// ---- the report: gaps are named, never silent

fn finding(checkpoint: &str, gap: Option<&str>, diffs: &[&str]) -> Finding {
    Finding {
        checkpoint: checkpoint.to_owned(),
        gap: gap.map(str::to_owned),
        diffs: diffs.iter().map(|d| (*d).to_owned()).collect(),
    }
}

#[test]
fn a_difference_without_a_gap_fails() {
    let report = Report {
        general: vec![],
        findings: vec![finding("a", None, &["row 1: x"]), finding("b", None, &[])],
    };
    let failures = report.failures(false);
    assert_eq!(failures.len(), 2, "{failures:?}");
    assert!(
        failures[0].contains("a") && failures[1].contains("row 1"),
        "{failures:?}"
    );
    assert!(report.expected_gaps().is_empty());
}

#[test]
fn a_difference_under_a_gap_is_reported_as_expected_and_fails_only_when_strict() {
    let report = Report {
        general: vec![],
        findings: vec![finding("a", Some("VS2: chat"), &["row 1: x", "row 2: y"])],
    };
    assert!(report.failures(false).is_empty());
    let expected = report.expected_gaps();
    assert_eq!(expected.len(), 1);
    assert!(
        expected[0].contains("VS2: chat") && expected[0].contains("2 difference"),
        "{expected:?}"
    );
    let strict = report.failures(true);
    assert!(strict.iter().any(|f| f.contains("row 1")), "{strict:?}");
}

#[test]
fn a_gap_that_closed_must_be_removed_from_the_script() {
    let report = Report {
        general: vec![],
        findings: vec![finding("a", Some("VS2: chat"), &[])],
    };
    let failures = report.failures(false);
    assert!(
        failures
            .iter()
            .any(|f| f.contains("a") && f.contains("remove")),
        "{failures:?}"
    );
    assert!(report.failures(true).is_empty());
    assert!(report.expected_gaps().is_empty());
}

#[test]
fn general_notes_always_fail() {
    let report = Report {
        general: vec!["rtwk: did not exit".into()],
        findings: vec![],
    };
    assert_eq!(report.failures(false), ["rtwk: did not exit"]);
    assert_eq!(report.failures(true), ["rtwk: did not exit"]);
}
