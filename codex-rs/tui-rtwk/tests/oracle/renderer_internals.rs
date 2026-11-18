//! Shard `renderer_internals` (screens S11, owner chat-live): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen};

/// The turn start of a streamed agent turn, as a JSONL step (`turn/started`).
const TURN_START: &str = r#"{"step":"notify","message":{"method":"turn/started","params":{"threadId":"thread-1","turn":{"id":"turn-1","rootTurnId":null,"items":[],"itemsView":"full","status":"inProgress","error":null,"startedAt":null,"completedAt":null,"durationMs":null}}}}"#;

/// A JSON string literal for `text` (the kit's script lines are JSON).
fn json_str(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// One agent turn streamed as `deltas`, as a JSONL script to `feed`.
fn stream_feed(deltas: &[String]) -> String {
    let mut lines = vec![TURN_START.to_string()];
    for delta in deltas {
        lines.push(format!(
            r#"{{"step":"notify","message":{{"method":"item/agentMessage/delta","params":{{"threadId":"thread-1","turnId":"turn-1","itemId":"msg-1","delta":{}}}}}}}"#,
            json_str(delta)
        ));
    }
    lines.join("\n")
}

/// The row holding `text` is preceded by a blank row (the separator the oracle
/// commits before a heading).
fn assert_blank_before(s: &Screen, text: &str) {
    let rows = s.rows();
    let Some(at) = rows.iter().position(|r| r.contains(text)) else {
        panic!("no `{text}` on the screen:\n{}", s.text());
    };
    let Some(before) = at.checked_sub(1) else {
        panic!(
            "`{text}` is the first row: no blank row before it:\n{}",
            s.text()
        );
    };
    s.assert_row(before, "");
}

/// No row holds both `a` and `b`: the oracle does not inline the one into the other's line.
fn assert_not_inlined(s: &Screen, a: &str, b: &str) {
    if let Some(row) = s.rows().iter().find(|r| r.contains(a) && r.contains(b)) {
        panic!("`{a}` and `{b}` share a row: {row:?}\n{}", s.text());
    }
}

// ---- oracle: markdown_stream.rs (7 screen-level tests here; 17 state tests left to chat-live)
// ORACLE markdown_stream.rs:370 heading_starts_on_new_line_when_following_paragraph kind=ui owner=chat-live
#[test]
fn heading_starts_on_new_line_when_following_paragraph() {
    // oracle: "Hello.\n" commits the paragraph alone; "## Heading\n" then commits a blank separator and the heading line.
    let s = Screen::build(Owner::ChatLive)
        .script(include_str!(
            "fixtures/renderer_internals/heading_paragraph.jsonl"
        ))
        .open();
    s.assert_in_order(&["Hello.", "## Heading"]);
    assert_blank_before(&s, "## Heading");
}

// ORACLE markdown_stream.rs:425 heading_not_inlined_when_split_across_chunks kind=ui owner=chat-live
#[test]
fn heading_not_inlined_when_split_across_chunks() {
    // oracle: "Sounds good!" then "\n## Adding Bird subcommand" commits the paragraph only (the heading
    // has no newline yet); the final "\n" commits a blank separator and the heading line.
    let mut s = Screen::build(Owner::ChatLive)
        .script(include_str!(
            "fixtures/renderer_internals/heading_split_1.jsonl"
        ))
        .open();
    s.feed(include_str!(
        "fixtures/renderer_internals/heading_split_2.jsonl"
    ));
    s.assert_contains("Sounds good!");
    assert_not_inlined(&s, "Sounds good!", "## Adding Bird subcommand");
    s.feed(include_str!(
        "fixtures/renderer_internals/heading_split_3.jsonl"
    ));
    s.assert_in_order(&["Sounds good!", "## Adding Bird subcommand"]);
    assert_blank_before(&s, "## Adding Bird subcommand");
}

// ORACLE markdown_stream.rs:557 utf8_boundary_safety_and_wide_chars kind=ui owner=chat-live
#[test]
fn utf8_boundary_safety_and_wide_chars() {
    // oracle: the streamed deltas (split inside wide chars, a control char, a combining mark) render as
    // the whole text rendered at once: no duplication, no truncation.
    let streamed = Screen::build(Owner::ChatLive)
        .script(include_str!(
            "fixtures/renderer_internals/utf8_streamed.jsonl"
        ))
        .open();
    let whole = Screen::build(Owner::ChatLive)
        .script(include_str!("fixtures/renderer_internals/utf8_full.jsonl"))
        .open();
    whole.assert_same_rows(&streamed.shot());
}

// ORACLE markdown_stream.rs:641 empty_fenced_block_is_dropped_and_separator_preserved_before_heading kind=ui owner=chat-live
#[test]
fn empty_fenced_block_is_dropped_and_separator_preserved_before_heading() {
    // oracle: an empty ```bash block shows no fence marker; the heading after it is on the screen.
    let s = Screen::build(Owner::ChatLive)
        .script(include_str!(
            "fixtures/renderer_internals/fence_empty_streamed.jsonl"
        ))
        .open();
    s.assert_absent("```");
    s.assert_contains("## Heading");
}

// ORACLE markdown_stream.rs:678 loose_list_with_split_dashes_matches_full_render kind=ui owner=chat-live
#[test]
fn loose_list_with_split_dashes_matches_full_render() {
    // oracle: the streamed "-" split from the list item renders as the full text rendered at once.
    let streamed = Screen::build(Owner::ChatLive)
        .script(include_str!(
            "fixtures/renderer_internals/loose_split_streamed.jsonl"
        ))
        .open();
    let full = Screen::build(Owner::ChatLive)
        .script(include_str!(
            "fixtures/renderer_internals/loose_split_full.jsonl"
        ))
        .open();
    full.assert_same_rows(&streamed.shot());
}

// ORACLE markdown_stream.rs:704 loose_vs_tight_list_items_streaming_matches_full kind=ui owner=chat-live
#[test]
fn loose_vs_tight_list_items_streaming_matches_full() {
    // oracle: the streamed loose/tight list renders as the full text, and as the expected lines in order.
    let streamed = Screen::build(Owner::ChatLive)
        .script(include_str!(
            "fixtures/renderer_internals/loose_tight_streamed.jsonl"
        ))
        .open();
    let full = Screen::build(Owner::ChatLive)
        .script(include_str!(
            "fixtures/renderer_internals/loose_tight_full.jsonl"
        ))
        .open();
    full.assert_same_rows(&streamed.shot());
    streamed.assert_in_order(&[
        "Loose vs. tight list items:",
        "1. Tight item",
        "2. Another tight item",
        "3. Loose item with its own paragraph.",
        "   This paragraph belongs to the same list item.",
        "4. Second loose item with a nested list after a blank line.",
        "    • Nested bullet under a loose item",
        "    • Another nested bullet",
    ]);
}

// ORACLE markdown_stream.rs:861 collector_source_chunks_round_trip_into_agent_fence_unwrapping kind=ui owner=chat-live
#[test]
fn collector_source_chunks_round_trip_into_agent_fence_unwrapping() {
    // oracle: a ```md fence streamed in chunks unwraps to a markdown table: a separator row of ━, no raw header row.
    let s = Screen::build(Owner::ChatLive)
        .script(include_str!(
            "fixtures/renderer_internals/table_fence_streamed.jsonl"
        ))
        .open();
    s.assert_contains("━");
    s.assert_absent("| A | B |");
}

// ---- oracle: render/highlight.rs (2 screen-level tests here; 41 state tests left to chat-live)
// ORACLE render/highlight.rs:986 highlight_crlf_strips_carriage_return kind=ui owner=chat-live
#[test]
fn highlight_crlf_strips_carriage_return() {
    // oracle: a CRLF rust block shows its lines; the oracle's check that no span keeps a `\r` is a cell
    // property the rows do not show (not asserted here).
    let s = Screen::build(Owner::ChatLive)
        .script(include_str!("fixtures/renderer_internals/crlf_code.jsonl"))
        .open();
    s.assert_in_order(&["fn main() {", "println!(\"hi\");", "}"]);
}

// ORACLE render/highlight.rs:1188 ansi_family_foreground_palette_snapshot kind=snap owner=chat-live snap=ansi_family_foreground_palette
#[test]
fn ansi_family_foreground_palette_snapshot() {
    // oracle: the foreground colour names of the ansi, base16 and base16-256 syntax themes (a palette list the
    // highlighter returns; no screen draws the list, and no seed selects a syntax theme).
    kit::na(
        Owner::ChatLive,
        "asserts the foreground colour names of the ansi, base16 and base16-256 syntax themes (a palette list), not a rendered screen",
    );
}

// ---- oracle: render/highlight_streaming_tests.rs (1 screen-level tests here; 3 state tests left to chat-live)
// ORACLE render/highlight_streaming_tests.rs:50 crossing_each_highlighting_limit_requires_a_canonical_render kind=ui owner=chat-live
#[test]
fn crossing_each_highlighting_limit_requires_a_canonical_render() {
    // oracle: a rust block streamed across each highlighting limit (4 KiB a line, 10 000 lines, 512 KiB) renders
    // as the same block delivered whole.
    // The limits: MAX_HIGHLIGHT_LINE_BYTES (4 * 1024), MAX_HIGHLIGHT_LINES (10_000), MAX_HIGHLIGHT_BYTES (512 * 1024).
    let fence = "```rust\n";
    let cases = [
        (String::new(), format!("{}\n", "x".repeat(4 * 1024 + 1))),
        ("x\n".repeat(10_000), "x\n".to_string()),
        (
            format!("{}\n", "x".repeat(1023)).repeat(512),
            "x\n".to_string(),
        ),
    ];
    for (initial, appended) in cases {
        let mut streamed = Screen::build(Owner::ChatLive).live().open();
        streamed.feed(&stream_feed(&[
            format!("{fence}{initial}"),
            appended.clone(),
            "next\n```\n".to_string(),
        ]));
        let mut whole = Screen::build(Owner::ChatLive).live().open();
        whole.feed(&stream_feed(&[format!(
            "{fence}{initial}{appended}next\n```\n"
        )]));
        whole.assert_same_rows(&streamed.shot());
    }
}

// ---- oracle: render/renderable_tests.rs (1 screen-level tests here; 2 state tests left to chat-live)
// ORACLE render/renderable_tests.rs:76 flex_caches_child_height_across_frame_passes kind=ui owner=chat-live
#[test]
fn flex_caches_child_height_across_frame_passes() {
    // oracle: counts how often a child's desired_height is called across frame passes. A call count is not drawn.
    kit::na(
        Owner::ChatLive,
        "asserts how many times a child's desired_height is called (a cache count), not a value any screen draws",
    );
}

// ---- oracle: wrapping.rs (8 screen-level tests here; 39 state tests left to chat-live)
// ORACLE wrapping.rs:1578 wrap_lines_applies_initial_indent_only_once kind=ui owner=chat-live
#[test]
fn wrap_lines_applies_initial_indent_only_once() {
    kit::pending(
        Owner::ChatLive,
        "no seed draws a list item with a `- ` initial indent: the chat's agent prefix is `• ` on the first row, and no seed sets another indent (the oracle test passes the indent to word_wrap_lines)",
    );
    // oracle: width 8, initial indent "- ", subsequent "  ": the first row starts "- ", every later row starts "  ".
    let mut s = Screen::build(Owner::ChatLive).size(10, 24).live().open();
    s.feed(&stream_feed(&["hello world\nfoo bar baz\n".to_string()]));
    s.assert_row_starts_with(0, "- ");
    s.assert_row_starts_with(1, "  ");
}

// ORACLE wrapping.rs:1596 wrap_lines_without_indents_is_concat_of_single_wraps kind=ui owner=chat-live
#[test]
fn wrap_lines_without_indents_is_concat_of_single_wraps() {
    // oracle: width 10, lines "hello" and "world!": the rows are "hello" then "world!". The chat's agent prefix takes 2 columns, so the terminal is 12 wide.
    let mut s = Screen::build(Owner::ChatLive).size(12, 24).live().open();
    s.feed(&stream_feed(&["hello\n\nworld!\n".to_string()]));
    s.assert_in_order(&["hello", "world!"]);
}

// ORACLE wrapping.rs:1604 wrap_lines_accepts_borrowed_iterators kind=ui owner=chat-live
#[test]
fn wrap_lines_accepts_borrowed_iterators() {
    // oracle: width 10, lines "hello world" and "foo bar baz": the rows are "hello", "world", "foo bar", "baz".
    let mut s = Screen::build(Owner::ChatLive).size(12, 24).live().open();
    s.feed(&stream_feed(&["hello world\n\nfoo bar baz\n".to_string()]));
    s.assert_in_order(&["hello", "world", "foo bar", "baz"]);
}

// ORACLE wrapping.rs:1612 wrap_lines_accepts_str_slices kind=ui owner=chat-live
#[test]
fn wrap_lines_accepts_str_slices() {
    // oracle: width 12, lines "hello world" and "goodnight moon": the rows are "hello world", "goodnight", "moon".
    let mut s = Screen::build(Owner::ChatLive).size(14, 24).live().open();
    s.feed(&stream_feed(&[
        "hello world\n\ngoodnight moon\n".to_string()
    ]));
    s.assert_in_order(&["hello world", "goodnight", "moon"]);
}

// ORACLE wrapping.rs:1817 adaptive_wrap_line_mixed_line_counts_leading_spaces_before_first_word kind=ui owner=chat-live
#[test]
fn adaptive_wrap_line_mixed_line_counts_leading_spaces_before_first_word() {
    kit::pending(
        Owner::ChatLive,
        "the port's chat wraps a URL by character (`https://x.` / `co`) and its markdown trims a paragraph's leading spaces; the oracle wraps the raw line with adaptive_wrap_line, and the chat-level counterpart is not verified against the oracle",
    );
    // oracle: width 10, continuation indent 6 spaces: the rows are "      abcd" then "      efgh".
    let mut s = Screen::build(Owner::ChatLive).size(12, 24).live().open();
    s.feed(&stream_feed(&["      abcdefgh https://x.co\n".to_string()]));
    s.assert_row_starts_with(0, "      abcd");
    s.assert_row_starts_with(1, "      efgh");
}

// ORACLE wrapping.rs:1832 adaptive_wrap_line_mixed_line_resplits_long_token_for_continuation_width kind=ui owner=chat-live
#[test]
fn adaptive_wrap_line_mixed_line_resplits_long_token_for_continuation_width() {
    kit::pending(
        Owner::ChatLive,
        "the port's chat wraps a URL by character (`https://x.` / `co`); the oracle wraps the raw line with adaptive_wrap_line, and the chat-level counterpart is not verified against the oracle",
    );
    // oracle: width 10, continuation indent 4 spaces: the rows are "abcdefghij", "    klmnop", "    qrst".
    let mut s = Screen::build(Owner::ChatLive).size(12, 24).live().open();
    s.feed(&stream_feed(&[
        "abcdefghijklmnopqrst https://x.co\n".to_string()
    ]));
    s.assert_in_order(&["abcdefghij", "    klmnop", "    qrst"]);
}

// ORACLE wrapping.rs:1851 adaptive_wrap_line_mixed_url_counts_halfwidth_sound_marks kind=ui owner=chat-live
#[test]
fn adaptive_wrap_line_mixed_url_counts_halfwidth_sound_marks() {
    kit::pending(
        Owner::ChatLive,
        "the port's chat wraps a URL by character (`https://x.` / `co`); the oracle wraps the raw line with adaptive_wrap_line, and the chat-level counterpart is not verified against the oracle",
    );
    // oracle: width 4, "ｶﾞﾊﾟtail https://x.co": the rows are "ｶﾞﾊﾟ", "tail", "https://x.co".
    let mut s = Screen::build(Owner::ChatLive).size(6, 24).live().open();
    s.feed(&stream_feed(&["ｶﾞﾊﾟtail https://x.co\n".to_string()]));
    s.assert_in_order(&["ｶﾞﾊﾟ", "tail", "https://x.co"]);
}

// ORACLE wrapping.rs:1860 adaptive_wrap_line_mixed_url_makes_progress_for_an_indivisible_grapheme kind=ui owner=chat-live
#[test]
fn adaptive_wrap_line_mixed_url_makes_progress_for_an_indivisible_grapheme() {
    kit::pending(
        Owner::ChatLive,
        "the port's chat wraps a URL by character (`https://x.` / `co`); the oracle wraps the raw line with adaptive_wrap_line, and the chat-level counterpart is not verified against the oracle",
    );
    // oracle: width 1, "ｶﾞ https://x.co": the rows are "ｶﾞ", "https://x.co" (the wrap makes progress).
    let mut s = Screen::build(Owner::ChatLive).size(3, 24).live().open();
    s.feed(&stream_feed(&["ｶﾞ https://x.co\n".to_string()]));
    s.assert_in_order(&["ｶﾞ", "https://x.co"]);
}
