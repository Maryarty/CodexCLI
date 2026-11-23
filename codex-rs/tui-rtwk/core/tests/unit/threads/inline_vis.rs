use codex_tui_core::threads::inline_vis::*;
use std::borrow::Cow;

const NOTE: &str = "Gone.";

#[test]
fn a_directive_line_reads_as_the_unavailable_note() {
    assert_eq!(
        rewrite(
            "Before.\n\n::codex-inline-vis{file=\"missing.html\"}\n",
            NOTE
        ),
        "Before.\n\n_Gone._\n"
    );
    assert_eq!(
        rewrite(
            "Before.\n\n\u{e200}visualize\u{e202}{\"path\":\"/tmp/missing.html\"}\u{e201}\n",
            NOTE
        ),
        "Before.\n\n_Gone._\n"
    );
}

#[test]
fn a_half_written_directive_is_dropped_and_text_without_one_is_kept() {
    assert_eq!(rewrite("a\n::codex-inline-vis{file=\"m", NOTE), "a\n");
    assert_eq!(
        rewrite("plain ::codex-inline-vis{file=\"x\"} text", NOTE),
        "plain ::codex-inline-vis{file=\"x\"} text"
    );
    assert_eq!(rewrite("no directive here", NOTE), "no directive here");
}

#[test]
fn code_blocks_keep_their_directive_text() {
    let fenced = "```\n::codex-inline-vis{file=\"x.html\"}\n```\n";
    assert_eq!(rewrite(fenced, NOTE), fenced);
    let open = "~~~\n::codex-inline-vis{file=\"x.html\"}\n";
    assert_eq!(rewrite(open, NOTE), open);
    let indented = "text\n\n    ::codex-inline-vis{file=\"x.html\"}\n";
    assert_eq!(rewrite(indented, NOTE), indented);
}
