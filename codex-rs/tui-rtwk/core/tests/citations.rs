//! File citations of an assistant message. Expected values follow the oracle's
//! codex-rs/tui/src/markdown_render/file_citations_tests.rs (what the source
//! holds, and what a citation's path reads as); the rest of that file is the
//! view's, in tests/pages/cells.rs.

use codex_tui_core::threads::citations::{Citation, citations, label};
use pretty_assertions::assert_eq;

fn paths(input: &str) -> Vec<String> {
    citations(input)
        .into_iter()
        .map(|citation| citation.path)
        .collect()
}

#[test]
fn a_citation_is_its_directive_from_colon_to_brace() {
    let input = r#"see :codex-file-citation{path="/tmp/a.xlsx" purpose="output"}."#;
    let start = input.find(':').expect("colon");
    let end = input.rfind('}').expect("brace") + 1;
    assert_eq!(
        citations(input),
        [Citation {
            range: start..end,
            path: "/tmp/a.xlsx".into()
        }]
    );
}

#[test]
fn quoted_unquoted_and_windows_paths_are_read_as_written() {
    for (directive, path) in [
        (
            r#":codex-file-citation{path="C:\Users\me\.codex\report.xlsx"}"#,
            r"C:\Users\me\.codex\report.xlsx",
        ),
        (
            ":codex-file-citation{path=/tmp/a*b*.txt purpose=output}",
            "/tmp/a*b*.txt",
        ),
        (
            r#":codex-file-citation{path="C:\repo\" purpose="output" sheet="Q1=Actual"}"#,
            r"C:\repo\",
        ),
        (
            r#":codex-file-citation{path="/tmp/a\"b" label="team's \"report\""}"#,
            "/tmp/a\"b",
        ),
    ] {
        assert_eq!(paths(directive), [path], "{directive}");
    }
}

#[test]
fn escaped_nested_colon_prefixed_and_pathless_directives_are_not_citations() {
    let citation = ":codex-file-citation{path=/tmp/report.xlsx}";
    assert_eq!(paths(&format!(r"\{citation}")), Vec::<String>::new());
    assert_eq!(
        paths(&format!(r"\{citation} and {citation}")),
        ["/tmp/report.xlsx"]
    );
    for literal in [
        format!(r#":unsupported{{value="{citation}"}}"#),
        format!(":::{citation}"),
        ":codex-file-citation{purpose=output}".to_owned(),
        r#":codex-file-citation{path=""}"#.to_owned(),
        r#":codex-followup[First]{prompt="Use :codex-file-citation{path=\"/tmp/private\"}"}"#
            .to_owned(),
    ] {
        assert_eq!(paths(&literal), Vec::<String>::new(), "{literal}");
    }
}

#[test]
fn a_double_colon_directive_is_one_citation_with_both_colons() {
    let input = r#"and ::codex-file-citation{path="second.txt"}"#;
    let found = citations(input);
    assert_eq!(found.len(), 1);
    assert_eq!(
        &input[found[0].range.clone()],
        r#"::codex-file-citation{path="second.txt"}"#
    );
}

#[test]
fn malformed_runs_stay_inside_the_scan_budget_and_many_citations_stay_all_found() {
    for literal in [
        format!("codex-file-citation {}", ":x{v=".repeat(16_000)),
        format!("codex-file-citation {}}}", ":x{v=a v=".repeat(16_000)),
        format!("codex-file-citation {} x bad}}", ":a{k=".repeat(16_000)),
    ] {
        assert_eq!(paths(&literal), Vec::<String>::new());
    }
    let many = [":codex-file-citation{path=/tmp/report.xlsx}"; 512].join(" ");
    assert_eq!(paths(&many), ["/tmp/report.xlsx"; 512]);
}

#[test]
fn paths_keep_their_markdown_significant_characters() {
    for path in [
        "/tmp/a*b*.txt",
        "/tmp/$x$/report.md",
        "/tmp/a`b`.txt",
        "/tmp/a<b>.txt",
        "/tmp/report#L10",
        "/tmp/report:10",
        "/tmp/report%20final.xlsx",
        "/tmp/report?final.xlsx",
        "file://[",
    ] {
        assert_eq!(label(path, None), path);
    }
}

#[test]
fn a_windows_path_reads_with_forward_slashes_and_a_trailing_one_stays() {
    assert_eq!(
        label(r"C:\Users\me\.codex\report.xlsx", None),
        "C:/Users/me/.codex/report.xlsx"
    );
    assert_eq!(label(r"C:\repo\", None), "C:/repo/");
}

#[test]
fn a_relative_path_is_read_against_the_cwd_and_shown_relative_to_it() {
    assert_eq!(label("second.txt", Some("/work/repo")), "second.txt");
    assert_eq!(
        label("/work/repo/src/lib.rs", Some("/work/repo")),
        "src/lib.rs"
    );
    assert_eq!(label("/other/lib.rs", Some("/work/repo")), "/other/lib.rs");
    assert_eq!(label("/work/repo", Some("/work/repo")), "/work/repo");
    assert_eq!(
        label("reports/final%20report.xlsx", Some("/work/repo")),
        "reports/final%20report.xlsx"
    );
    // Without a cwd a relative path keeps the `./` the oracle gives its destination.
    assert_eq!(label("second.txt", None), "./second.txt");
}

#[test]
fn a_file_url_reads_as_its_path_and_the_cwd_strips_it() {
    for (cwd, url) in [
        ("/tmp", "file:///tmp/report?final.xlsx"),
        ("C:/repo", "file:///C:/repo/report?final.xlsx"),
        ("C:/repo", "file://localhost/C:/repo/report?final.xlsx"),
    ] {
        assert_eq!(label(url, Some(cwd)), "report?final.xlsx", "{url}");
    }
    assert_eq!(
        label("file://server/share/a.txt", None),
        "//server/share/a.txt"
    );
}

// ---- local file links (oracle markdown_render/local_links.rs, markdown_render_tests.rs
// `file_link_*`): what a Markdown link to a local file shows after its label.

use codex_tui_core::screens::cells::local_links::{display, is_local_path_like};

const CWD: Option<&str> = Some("/Users/example/code/codex");

#[test]
fn only_path_like_destinations_are_local_links() {
    for local in [
        "/tmp/a.rs",
        "~/notes",
        "./a.rs",
        "../a.rs",
        "file:///repo/a.rs",
        r"C:\repo\a.rs",
        r"\\server\share\a.rs",
    ] {
        assert!(is_local_path_like(local), "{local}");
    }
    for web in [
        "https://example.com",
        "mailto:a@b.c",
        "#anchor",
        "a.rs",
        "docs/a.rs",
    ] {
        assert!(!is_local_path_like(web), "{web}");
    }
}

#[test]
fn a_path_under_the_cwd_shows_relative_and_a_hash_location_reads_as_colons() {
    let base = "/Users/example/code/codex/codex-rs/tui/src/markdown_render.rs";
    let shown = "codex-rs/tui/src/markdown_render.rs";
    for (destination, want) in [
        (base.to_owned(), shown.to_owned()),
        (format!("{base}#L74"), format!("{shown}:74")),
        (format!("{base}#L74C3"), format!("{shown}:74:3")),
        (format!("{base}#L74C3-L76C9"), format!("{shown}:74:3-76:9")),
        (format!("{base}:74:3-76:9"), format!("{shown}:74:3-76:9")),
        (format!("file://{base}#L74C3"), format!("{shown}:74:3")),
    ] {
        assert_eq!(display(&destination, CWD), want, "{destination}");
    }
}

#[test]
fn relative_tilde_and_foreign_paths_keep_their_text_and_percent_escapes_decode() {
    assert_eq!(display("~/notes", CWD), "~/notes");
    assert_eq!(display("./src/lib.rs", CWD), "./src/lib.rs");
    assert_eq!(display("/home/alice/notes", CWD), "/home/alice/notes");
    assert_eq!(
        display(
            "/Users/example/code/codex/Example%20Folder/R%C3%A9sum%C3%A9/report.md",
            CWD
        ),
        "Example Folder/Résumé/report.md"
    );
    // A `#anchor` that is no location stays part of the path text.
    assert_eq!(display("/tmp/a.md#section", CWD), "/tmp/a.md#section");
    // The cwd itself is not "under" the cwd.
    assert_eq!(
        display("/Users/example/code/codex", CWD),
        "/Users/example/code/codex"
    );
}

#[test]
fn a_windows_or_unc_cwd_strips_its_own_file_urls_and_the_anchor_reads_as_a_location() {
    // The oracle's `file_links_preserve_foreign_windows_paths_and_anchors_snapshot`: a thread
    // cwd is an absolute path of the host, so the screen test cannot open a session in
    // `C:/repo`; the display is held here with the cwd as text.
    assert_eq!(
        display(
            "file:///C:/repo/Quarterly%20Report.xlsx#L4C2",
            Some("C:/repo")
        ),
        "Quarterly Report.xlsx:4:2"
    );
    assert_eq!(
        display(
            "file://server/share/Quarterly%20Report.xlsx#L4C2",
            Some("//server/share")
        ),
        "Quarterly Report.xlsx:4:2"
    );
}
