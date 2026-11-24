//! Shard `markdown_streaming_1` (screens S11, owner chat-live): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Each test feeds the oracle's markdown as an agent message (fixtures/markdown_streaming_1/*.jsonl)
//! and compares what the chat draws with the oracle's expected rows. The terminal width is the
//! oracle's render width where the oracle gives one; the chat's own gutter is set by the
//! chat-live port. Snapshot heights cover the snapshot's rows.

use crate::kit::{self, Owner, Screen};

// ---- oracle: markdown.rs (11 screen-level tests here; 7 state tests left to chat-live)
// ORACLE markdown.rs:618 citations_render_as_plain_text kind=ui owner=chat-live
#[test]
fn citations_render_as_plain_text() {
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_1/citations_render_as_plain_text.jsonl"
        ))
        .open();
    s.assert_in_order(&["Before 【F:/x.rs†L1】", "After 【F:/x.rs†L3】"]);
}

// ORACLE markdown.rs:643 append_markdown_preserves_full_text_line kind=ui owner=chat-live
#[test]
fn append_markdown_preserves_full_text_line() {
    let s = Screen::build(Owner::ChatLive)
        .markdown(120, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_1/append_markdown_preserves_full_text_line.jsonl"
        ))
        .open();
    s.assert_contains(
        "Hi! How can I help with codex-rs today? Want me to explore the repo, run tests, or work on a specific change?",
    );
}

// ORACLE markdown.rs:678 append_markdown_keeps_ordered_list_line_unsplit_in_context kind=ui owner=chat-live
#[test]
fn append_markdown_keeps_ordered_list_line_unsplit_in_context() {
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_1/append_markdown_keeps_ordered_list_line_unsplit_in_context.jsonl"
        ))
        .open();
    s.assert_in_order(&["Loose vs. tight list items:", "1. Tight item"]);
}

// ORACLE markdown.rs:700 append_markdown_agent_unwraps_markdown_fences_for_table_rendering kind=ui owner=chat-live
#[test]
fn append_markdown_agent_unwraps_markdown_fences_for_table_rendering() {
    kit::pending(
        Owner::ChatLive,
        "G73: item 3, the oracle draws a table as columns two spaces apart with a `━` rule under the header and `─` rules between logical rows (tui markdown_render.rs:112 TABLE_HEADER_SEPARATOR_CHAR, :1851 render_table_separator); rtwk's MarkdownView draws a `│` grid with one `─┼─` header rule",
    );
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_1/append_markdown_agent_unwraps_markdown_fences_for_table_rendering.jsonl"
        ))
        .open();
    s.assert_contains("━");
    s.assert_contains(" 1      2");
}

// ORACLE markdown.rs:710 append_markdown_agent_unwraps_markdown_fences_for_no_outer_table_rendering kind=ui owner=chat-live
#[test]
fn append_markdown_agent_unwraps_markdown_fences_for_no_outer_table_rendering() {
    kit::pending(
        Owner::ChatLive,
        "G73: item 3, the oracle draws a table as columns two spaces apart with a `━` rule under the header and `─` rules between logical rows (tui markdown_render.rs:112 TABLE_HEADER_SEPARATOR_CHAR, :1851 render_table_separator); rtwk's MarkdownView draws a `│` grid with one `─┼─` header rule",
    );
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_1/append_markdown_agent_unwraps_markdown_fences_for_no_outer_table_rendering.jsonl"
        ))
        .open();
    s.assert_contains("━");
    s.assert_contains(" Col A    Col B    Col C");
    s.assert_absent("Col A | Col B | Col C");
}

// ORACLE markdown.rs:729 append_markdown_agent_unwraps_markdown_fences_for_two_column_no_outer_table kind=ui owner=chat-live
#[test]
fn append_markdown_agent_unwraps_markdown_fences_for_two_column_no_outer_table() {
    kit::pending(
        Owner::ChatLive,
        "G73: item 3, the oracle draws a table as columns two spaces apart with a `━` rule under the header and `─` rules between logical rows (tui markdown_render.rs:112 TABLE_HEADER_SEPARATOR_CHAR, :1851 render_table_separator); rtwk's MarkdownView draws a `│` grid with one `─┼─` header rule",
    );
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_1/append_markdown_agent_unwraps_markdown_fences_for_two_column_no_outer_table.jsonl"
        ))
        .open();
    s.assert_contains("━");
    s.assert_contains(" left    right");
    s.assert_absent("A | B");
}

// ORACLE markdown.rs:740 append_markdown_agent_unwraps_markdown_fences_for_single_column_table kind=ui owner=chat-live
#[test]
fn append_markdown_agent_unwraps_markdown_fences_for_single_column_table() {
    kit::pending(
        Owner::ChatLive,
        "G73: item 3, the oracle draws a table as columns two spaces apart with a `━` rule under the header and `─` rules between logical rows (tui markdown_render.rs:112 TABLE_HEADER_SEPARATOR_CHAR, :1851 render_table_separator); rtwk's MarkdownView draws a `│` grid with one `─┼─` header rule",
    );
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_1/append_markdown_agent_unwraps_markdown_fences_for_single_column_table.jsonl"
        ))
        .open();
    s.assert_contains("━");
    s.assert_absent("| Only |");
}

// ORACLE markdown.rs:750 append_markdown_agent_keeps_non_markdown_fences_as_code kind=ui owner=chat-live
#[test]
fn append_markdown_agent_keeps_non_markdown_fences_as_code() {
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_1/append_markdown_agent_keeps_non_markdown_fences_as_code.jsonl"
        ))
        .open();
    s.assert_in_order(&["| A | B |", "|---|---|", "| 1 | 2 |"]);
}

// ORACLE markdown.rs:766 append_markdown_agent_unwraps_blockquoted_markdown_fence_table kind=ui owner=chat-live
#[test]
fn append_markdown_agent_unwraps_blockquoted_markdown_fence_table() {
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_1/append_markdown_agent_unwraps_blockquoted_markdown_fence_table.jsonl"
        ))
        .open();
    s.assert_absent("```");
}

// ORACLE markdown.rs:783 append_markdown_agent_keeps_markdown_fence_when_content_is_not_table kind=ui owner=chat-live
#[test]
fn append_markdown_agent_keeps_markdown_fence_when_content_is_not_table() {
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_1/append_markdown_agent_keeps_markdown_fence_when_content_is_not_table.jsonl"
        ))
        .open();
    s.assert_contains("**bold**");
    s.assert_absent("```");
}

// ORACLE markdown.rs:799 append_markdown_agent_keeps_markdown_fence_with_blank_line_between_header_and_delimiter kind=ui owner=chat-live
#[test]
fn append_markdown_agent_keeps_markdown_fence_with_blank_line_between_header_and_delimiter() {
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_1/append_markdown_agent_keeps_markdown_fence_with_blank_line_between_header_and_delimiter.jsonl"
        ))
        .open();
    s.assert_contains("| A | B |");
    s.assert_contains("| 1 | 2 |");
}

// ---- oracle: markdown_copy/table_tests.rs (1 screen-level tests here; 0 state tests left to chat-live)
// ORACLE markdown_copy/table_tests.rs:8 table_layouts_preserve_cell_fragments kind=ui owner=chat-live
#[test]
fn table_layouts_preserve_cell_fragments() {
    kit::na(
        Owner::ChatLive,
        "asserts the retained table copy fragments and their byte ranges (copy metadata), not drawn on the screen",
    );
}

// ---- oracle: markdown_copy_tests.rs (3 screen-level tests here; 1 state tests left to chat-live)
// ORACLE markdown_copy_tests.rs:31 deeply_nested_lazy_continuations_keep_bounded_relative_copy_prefixes kind=ui owner=chat-live
#[test]
fn deeply_nested_lazy_continuations_keep_bounded_relative_copy_prefixes() {
    kit::na(
        Owner::ChatLive,
        "asserts the copy prefixes and continuations of each line (copy metadata), not drawn on the screen",
    );
}

// ORACLE markdown_copy_tests.rs:59 code_display_padding_does_not_displace_a_retained_container kind=ui owner=chat-live
#[test]
fn code_display_padding_does_not_displace_a_retained_container() {
    kit::na(
        Owner::ChatLive,
        "asserts the copy prefixes of code lines (copy metadata), not drawn on the screen",
    );
}

// ORACLE markdown_copy_tests.rs:79 link_runs_share_destinations_and_bound_nested_formatting kind=ui owner=chat-live
#[test]
fn link_runs_share_destinations_and_bound_nested_formatting() {
    kit::na(
        Owner::ChatLive,
        "asserts the copy runs and inline destinations of a link (copy metadata), not drawn on the screen",
    );
}

// ---- oracle: markdown_render.rs (18 screen-level tests here; 17 state tests left to chat-live)
// ORACLE markdown_render.rs:2684 wraps_plain_text_when_width_provided kind=ui owner=chat-live
#[test]
fn wraps_plain_text_when_width_provided() {
    let s = Screen::build(Owner::ChatLive)
        .markdown(16, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_1/wraps_plain_text_when_width_provided.jsonl"
        ))
        .open();
    s.assert_in_order(&["This is a simple", "sentence that", "should wrap."]);
}

// ORACLE markdown_render.rs:2699 wraps_list_items_preserving_indent kind=ui owner=chat-live
#[test]
fn wraps_list_items_preserving_indent() {
    let s = Screen::build(Owner::ChatLive)
        .markdown(14, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_1/wraps_list_items_preserving_indent.jsonl"
        ))
        .open();
    s.assert_in_order(&["• first second", "  third fourth"]);
}

// ORACLE markdown_render.rs:2710 wraps_nested_lists kind=ui owner=chat-live
#[test]
fn wraps_nested_lists() {
    kit::pending(
        Owner::ChatLive,
        "G64: MarkdownView indents a nested list by its parent's marker (2 columns), the oracle by 4 (flutter_markdown listIndent)",
    );
    let s = Screen::build(Owner::ChatLive)
        .markdown(20, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_1/wraps_nested_lists.jsonl"
        ))
        .open();
    s.assert_in_order(&[
        "• outer item with",
        "  several words to",
        "  wrap",
        "    • inner item",
        "      that also",
        "      needs wrapping",
    ]);
}

// ORACLE markdown_render.rs:2729 wraps_ordered_lists kind=ui owner=chat-live
#[test]
fn wraps_ordered_lists() {
    let s = Screen::build(Owner::ChatLive)
        .markdown(18, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_1/wraps_ordered_lists.jsonl"
        ))
        .open();
    s.assert_in_order(&[
        "1. ordered item",
        "   contains many",
        "   words for",
        "   wrapping",
    ]);
}

// ORACLE markdown_render.rs:2745 wraps_blockquotes kind=ui owner=chat-live
#[test]
fn wraps_blockquotes() {
    kit::pending(
        Owner::ChatLive,
        "G73: item 1, the oracle draws a quote as `> ` rows (tui markdown_render.rs:884 start_blockquote, the `> ` indent); rtwk's MarkdownTheme draws a `│` bar from Glyphs and has no source-marker option, so the rows read `│ …`",
    );
    // The oracle's source-range style check (`styled_range`) is copy metadata and is not
    // drawn; the visible half, the wrapped quote rows, is asserted here.
    let s = Screen::build(Owner::ChatLive)
        .markdown(22, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_1/wraps_blockquotes.jsonl"
        ))
        .open();
    s.assert_in_order(&[
        "> block quote with",
        "> content that should",
        "> wrap nicely",
    ]);
}

// ORACLE markdown_render.rs:2771 wraps_blockquotes_inside_lists kind=ui owner=chat-live
#[test]
fn wraps_blockquotes_inside_lists() {
    kit::pending(
        Owner::ChatLive,
        "G73: item 1, the oracle draws a quote as `> ` rows (tui markdown_render.rs:884 start_blockquote, the `> ` indent); rtwk's MarkdownTheme draws a `│` bar from Glyphs and has no source-marker option, so the rows read `│ …`",
    );
    let s = Screen::build(Owner::ChatLive)
        .markdown(24, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_1/wraps_blockquotes_inside_lists.jsonl"
        ))
        .open();
    s.assert_in_order(&[
        "• list item",
        "  > block quote inside",
        "  > list that wraps",
    ]);
}

// ORACLE markdown_render.rs:2786 wraps_list_items_containing_blockquotes kind=ui owner=chat-live
#[test]
fn wraps_list_items_containing_blockquotes() {
    kit::pending(
        Owner::ChatLive,
        "G73: item 1, the oracle draws a quote as `> ` rows (tui markdown_render.rs:884 start_blockquote, the `> ` indent); rtwk's MarkdownTheme draws a `│` bar from Glyphs and has no source-marker option, so the rows read `│ …`",
    );
    let s = Screen::build(Owner::ChatLive)
        .markdown(24, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_1/wraps_list_items_containing_blockquotes.jsonl"
        ))
        .open();
    s.assert_in_order(&[
        "1. item with quote",
        "   > quoted text that",
        "   > should wrap",
    ]);
}

// ORACLE markdown_render.rs:2801 does_not_wrap_code_blocks kind=ui owner=chat-live
#[test]
fn does_not_wrap_code_blocks() {
    kit::pending(
        Owner::ChatLive,
        "the chat draws no fenced code block: a code line wider than the content is not kept on one row",
    );
    let s = Screen::build(Owner::ChatLive)
        .markdown(10, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_1/does_not_wrap_code_blocks.jsonl"
        ))
        .open();
    s.assert_contains("fn main() { println!(\"hi from a long line\"); }");
}

// ORACLE markdown_render.rs:2812 does_not_split_long_url_like_token_without_scheme kind=ui owner=chat-live
#[test]
fn does_not_split_long_url_like_token_without_scheme() {
    kit::pending(
        Owner::ChatLive,
        "the chat wraps prose without a word break rule: a long URL-like token is split across rows",
    );
    let s = Screen::build(Owner::ChatLive)
        .markdown(24, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_1/does_not_split_long_url_like_token_without_scheme.jsonl"
        ))
        .open();
    s.assert_contains(
        "example.test/api/v1/projects/alpha-team/releases/2026-02-17/builds/1234567890",
    );
}

// ORACLE markdown_render.rs:2826 fenced_code_info_string_with_metadata_highlights kind=ui owner=chat-live
#[test]
fn fenced_code_info_string_with_metadata_highlights() {
    // The oracle checks an Rgb foreground on some span for each info string. No kit call says
    // "some cell has an Rgb foreground", and the oracle sets the terminal default colours
    // (`with_test_default_colors`) that the highlight depends on: kit gaps, reported.
    let comma = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_1/fenced_code_info_string_with_metadata_highlights_comma.jsonl"
        ))
        .open();
    comma.assert_contains("fn main() {}");
    let space = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_1/fenced_code_info_string_with_metadata_highlights_space.jsonl"
        ))
        .open();
    space.assert_contains("fn main() {}");
    let title = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_1/fenced_code_info_string_with_metadata_highlights_title.jsonl"
        ))
        .open();
    title.assert_contains("fn main() {}");
}

// ORACLE markdown_render.rs:2852 crlf_code_block_no_extra_blank_lines kind=ui owner=chat-live
#[test]
fn crlf_code_block_no_extra_blank_lines() {
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_1/crlf_code_block_no_extra_blank_lines.jsonl"
        ))
        .open();
    s.assert_in_order(&["fn main() {}", "    line2"]);
}

// ORACLE markdown_render.rs:2867 wrap_cell_preserves_hard_break_lines kind=ui owner=chat-live
#[test]
fn wrap_cell_preserves_hard_break_lines() {
    // The oracle builds the table cell by hand (`TableCell::hard_break`). No markdown source is
    // known to produce that cell on the screen yet: the source is chosen with the markdown slice.
    kit::pending(
        Owner::ChatLive,
        "the chat draws no table: a cell with a hard break (two rows in one cell) has no markdown source and no table layout",
    );
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .live()
        .open();
    s.assert_in_order(&["first line", "second line"]);
}

// ORACLE markdown_render.rs:2898 blank_table_continuations_do_not_repeat_the_first_cell kind=snap owner=chat-live snap=blank_table_continuations_do_not_repeat_the_first_cell snap=short
#[test]
fn blank_table_continuations_do_not_repeat_the_first_cell() {
    // The oracle snapshot is a Debug list of the rendered table rows, not a screen: its rows are
    // compared as screen rows here. The cell source is built by hand in the oracle, see above.
    kit::pending(
        Owner::ChatLive,
        "the chat draws no table: blank continuation rows of a two-column cell have no markdown source and no table layout",
    );
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .live()
        .open();
    s.assert_in_order(&["short    first", "last"]);
}

// ORACLE markdown_render.rs:3228 annotates_explicit_web_link_label_and_visible_destination kind=ui owner=chat-live
#[test]
fn annotates_explicit_web_link_label_and_visible_destination() {
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_1/annotates_explicit_web_link_label_and_visible_destination.jsonl"
        ))
        .open();
    s.assert_link("docs", "https://example.com/reference");
    s.assert_contains("https://example.com/reference");
}

// ORACLE markdown_render.rs:3248 wrapped_table_url_fragments_keep_complete_web_destination kind=ui owner=chat-live
#[test]
fn wrapped_table_url_fragments_keep_complete_web_destination() {
    // The oracle asserts every linked row of the wrapped URL points at the full destination.
    // The wrapped fragments' texts are not in the oracle, and no kit call checks every linked
    // cell of a wrapped row against a destination: kit gap, reported.
    let s = Screen::build(Owner::ChatLive)
        .markdown(32, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_1/wrapped_table_url_fragments_keep_complete_web_destination.jsonl"
        ))
        .open();
    s.assert_contains("report");
}

// ORACLE markdown_render.rs:3280 key_value_table_keeps_web_annotations kind=ui owner=chat-live
#[test]
fn key_value_table_keeps_web_annotations() {
    // The oracle asserts every link of the key-value table points at the destination; a kit
    // call that checks the links of a row whose text wraps is missing: kit gap, reported.
    kit::pending(
        Owner::ChatLive,
        "the chat draws no table: a six-column table at width 20 is not laid out as key-value records with links; probe: the body passes without it (it checks less than the oracle claims)",
    );
    let s = Screen::build(Owner::ChatLive)
        .markdown(20, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_1/key_value_table_keeps_web_annotations.jsonl"
        ))
        .open();
    s.assert_contains("example.com");
}

// ORACLE markdown_render.rs:3300 does_not_annotate_code_or_non_web_markdown_links kind=ui owner=chat-live
#[test]
fn does_not_annotate_code_or_non_web_markdown_links() {
    kit::pending(
        Owner::ChatLive,
        "the chat draws agent links and code without markdown: links are not set on code or mailto text yet",
    );
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 40)
        .script(include_str!(
            "fixtures/markdown_streaming_1/does_not_annotate_code_or_non_web_markdown_links.jsonl"
        ))
        .open();
    s.assert_no_link("https://example.com/inline");
    s.assert_no_link("https://example.com/block");
    s.assert_no_link("mail");
    s.assert_no_link("https://example.com/label");
    s.assert_no_link("https://example.com/table-label");
}

// ORACLE markdown_render.rs:3312 pipe_table_fallback_keeps_web_annotations kind=ui owner=chat-live
#[test]
fn pipe_table_fallback_keeps_web_annotations() {
    // The oracle asserts the links of the fallback pipe table: the destinations of the URL and
    // the label, and no link on the code span. Wrapped texts at width 5 are not in the oracle.
    kit::pending(
        Owner::ChatLive,
        "the chat draws no pipe-table fallback: the table's links are not drawn at width 5",
    );
    let s = Screen::build(Owner::ChatLive)
        .markdown(5, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_1/pipe_table_fallback_keeps_web_annotations.jsonl"
        ))
        .open();
    s.assert_no_link("not-a-link");
}

// ---- oracle: markdown_render/file_citations_tests.rs (9 screen-level tests here; 0 state tests left to chat-live)
// ORACLE markdown_render/file_citations_tests.rs:17 file_citation_paths_preserve_markdown_significant_characters kind=ui owner=chat-live
#[test]
fn file_citation_paths_preserve_markdown_significant_characters() {
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 40)
        .script(include_str!(
            "fixtures/markdown_streaming_1/file_citation_paths_preserve_markdown_significant_characters.jsonl"
        ))
        .open();
    s.assert_in_order(&[
        "/tmp/a*b*.txt",
        "/tmp/$x$/report.md",
        "/tmp/a`b`.txt",
        "/tmp/a<b>.txt",
        "/tmp/report#L10",
        "/tmp/report:10",
        "/tmp/report%20final.xlsx",
        "/tmp/report?final.xlsx",
        "file://[",
    ]);
}

// ORACLE markdown_render/file_citations_tests.rs:35 file_url_citation_preserves_literal_query_delimiters_snapshot kind=snap owner=chat-live
#[test]
fn file_url_citation_preserves_literal_query_delimiters_snapshot() {
    // The oracle's expected text is an inline snapshot (`report?final.xlsx`), not a .snap file.
    // It runs under three session cwds: the temp dir and `C:/repo` twice. A session cwd is an
    // absolute path of the host (`AbsolutePathBuf`), so `C:/repo` cannot be seeded here: the
    // temp-dir run is this one, the `C:/repo` runs are core/tests/citations.rs.
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .agent_says(":codex-file-citation{path=\"file:///tmp/report?final.xlsx\"}\n")
        .cwd("/tmp")
        .open();
    s.assert_has_row("report?final.xlsx");
}

// ORACLE markdown_render/file_citations_tests.rs:51 file_citations_inside_code_existing_links_and_html_remain_literal kind=ui owner=chat-live
#[test]
fn file_citations_inside_code_existing_links_and_html_remain_literal() {
    kit::pending(
        Owner::ChatLive,
        "the oracle's last assertion reads ` (https://example.com)` after the label: its display for a terminal not known to support OSC 8 (WebLinkDisplay::for_terminal); the port has no terminal detection and draws a web label alone with its hyperlink (`LinkDestinations::Auto`, as the oracle does on a supporting terminal)",
    );
    let c = r#":codex-file-citation{path="/tmp/report.xlsx" purpose="output"}"#;
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 40)
        .script(include_str!(
            "fixtures/markdown_streaming_1/file_citations_inside_code_existing_links_and_html_remain_literal.jsonl"
        ))
        .open();
    s.assert_contains(c);
    s.assert_contains(&format!("<span title='{c}'>"));
    s.assert_contains(&format!("<!-- {c} -->"));
    s.assert_contains(&format!("{c} (https://example.com)"));
}

// ORACLE markdown_render/file_citations_tests.rs:76 file_citations_accept_unquoted_paths_and_trailing_windows_separators kind=ui owner=chat-live
#[test]
fn file_citations_accept_unquoted_paths_and_trailing_windows_separators() {
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_1/file_citations_accept_unquoted_paths_and_trailing_windows_separators.jsonl"
        ))
        .open();
    s.assert_in_order(&[
        "C:/Users/me/.codex/report.xlsx",
        "/tmp/a*b*.txt",
        "C:/repo/",
        "/tmp/a\"b",
    ]);
}

// ORACLE markdown_render/file_citations_tests.rs:100 file_citations_preserve_escaped_nested_and_reference_directives kind=ui owner=chat-live
#[test]
fn file_citations_preserve_escaped_nested_and_reference_directives() {
    // The oracle's 16000-repetition inputs are not fixtures (too large for a JSONL line): kit
    // gap for generated seeds, reported. The rest of its literal and reference cases are here.
    let c = ":codex-file-citation{path=/tmp/report.xlsx}";
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 40)
        .script(include_str!(
            "fixtures/markdown_streaming_1/file_citations_preserve_escaped_nested_and_reference_directives.jsonl"
        ))
        .open();
    s.assert_contains(c);
    s.assert_contains(&format!("{c} and /tmp/report.xlsx"));
    s.assert_contains(&format!(":unsupported{{value=\"{c}\"}}"));
    s.assert_contains(&format!(":::{c}"));
    s.assert_contains(
        r#":unsupported{path="C:\repo" value=":codex-file-citation{path=/tmp/report.xlsx}"}"#,
    );
    s.assert_in_order(&[c, &format!("report ({c})"), "/tmp/report.xlsx"]);
}

// ORACLE markdown_render/file_citations_tests.rs:140 multiple_file_citations_render_without_interpreting_encoded_source kind=ui owner=chat-live
#[test]
fn multiple_file_citations_render_without_interpreting_encoded_source() {
    // The oracle's cwd is the temp dir and its text is not wrapped (`rendered_text`): the
    // terminal is wide enough for the 85-column row. The 512-citation run is
    // core/tests/citations.rs (`malformed_runs_stay_inside_the_scan_budget_and_many_citations_stay_all_found`).
    let s = Screen::build(Owner::ChatLive)
        .markdown(100, 24)
        .agent_says(concat!(
            r#"&#58;codex-file-citation{path="ignored.xlsx"} "#,
            r#":codex-file-citation{artifact_kind="workbook" path="reports/final%20report.xlsx" purpose="output" sheet="Dashboard" range="A1:D8"} "#,
            r#"and ::codex-file-citation{path="/tmp/second.xlsx"}"#,
        ))
        .cwd("/tmp")
        .open();
    s.assert_has_row(
        r#":codex-file-citation{path="ignored.xlsx"} reports/final%20report.xlsx and second.xlsx"#,
    );
}

// ORACLE markdown_render/file_citations_tests.rs:162 file_citations_preserve_adjacent_entities_and_escaped_punctuation kind=ui owner=chat-live
#[test]
fn file_citations_preserve_adjacent_entities_and_escaped_punctuation() {
    kit::pending(
        Owner::ChatLive,
        "G61: MarkdownView has no math renderer, so the first assertion (`$\\alpha$…$\\beta$` as `α/tmp/$x$/report.mdβ`) is drawn as source; the entity and escape cases hold (tests/pages/cells.rs citations)",
    );
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 40)
        .script(include_str!(
            "fixtures/markdown_streaming_1/file_citations_preserve_adjacent_entities_and_escaped_punctuation.jsonl"
        ))
        .open();
    s.assert_contains("α/tmp/$x$/report.mdβ");
    s.assert_contains("Préface &first&amp;.txt🙂second.txt<fin>");
    s.assert_contains("[first.txt]*second.txt!");
}

// ORACLE markdown_render/file_citations_tests.rs:186 file_citation_after_local_link_soft_break_starts_a_new_line_snapshot kind=snap owner=chat-live
#[test]
fn file_citation_after_local_link_soft_break_starts_a_new_line_snapshot() {
    // The oracle's local link targets the temp dir, which is also its cwd.
    let temp = std::env::temp_dir();
    let cwd = temp
        .to_str()
        .expect("a temp dir path")
        .trim_end_matches('/');
    let markdown =
        format!("[first](<{cwd}/first.txt>)\n:codex-file-citation{{path=\"second.txt\"}}");
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .agent_says(&markdown)
        .cwd(cwd)
        .open();
    // The citation starts a new line after the link: the two rows follow each other.
    s.assert_row_then("first (first.txt)", &["second.txt"]);
}

// ORACLE markdown_render/file_citations_tests.rs:201 generic_markdown_keeps_assistant_directives_literal kind=ui owner=chat-live
#[test]
fn generic_markdown_keeps_assistant_directives_literal() {
    // The generic renderer is the reasoning summary's (history_cell/messages.rs
    // `ReasoningSummaryCell`, shown by the detailed transcript); the agent message reads
    // citations. tests/pages/cells.rs holds the widget-level twin.
    let mut s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .reasoning_says(r#":codex-file-citation{path="/tmp/report.xlsx"}"#)
        .open();
    s.keys("ctrl+t");
    s.assert_contains(r#":codex-file-citation{path="/tmp/report.xlsx"}"#);
}

// ---- oracle: markdown_render/followups_tests.rs (2 screen-level tests here; 0 state tests left to chat-live)
// ORACLE markdown_render/followups_tests.rs:11 followup_labels_render_like_ordinary_markdown_snapshot kind=ui owner=chat-live
#[test]
fn followup_labels_render_like_ordinary_markdown_snapshot() {
    // The oracle's expected text is an inline snapshot. The streamed render and the plain render
    // are compared in the oracle; the screen shows the streamed one.
    let s = Screen::build(Owner::ChatLive)
        .markdown(32, 40)
        .script(include_str!(
            "fixtures/markdown_streaming_1/followup_labels_render_like_ordinary_markdown_snapshot.jsonl"
        ))
        .open();
    s.assert_in_order(&[
        "Next actions:",
        "• Add revenue charts",
        "• Format Sheet1",
        "Or explain café totals.",
    ]);
}

// ORACLE markdown_render/followups_tests.rs:48 followups_preserve_literal_and_malformed_source kind=snap owner=chat-live
#[test]
fn followups_preserve_literal_and_malformed_source() {
    // The oracle asserts the internal `InlineDirectives` text equals the source for eight forms.
    // Its visible half: the malformed forms (no close, unfinished prompt, two prompts) stay as
    // typed in the transcript. The other forms (code, fence, HTML, link) change on render.
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_1/followups_preserve_literal_and_malformed_source.jsonl"
        ))
        .open();
    s.assert_in_order(&[
        ":codex-followup[Add charts",
        r#":codex-followup[Add charts]{prompt="unfinished"#,
        r#":codex-followup[Add charts]{prompt=one prompt=two}"#,
    ]);
}

// ---- oracle: markdown_render/list_spacing_tests.rs (2 screen-level tests here; 0 state tests left to chat-live)
// ORACLE markdown_render/list_spacing_tests.rs:8 list_spacing_is_uniform_per_list_and_preserves_item_paragraphs kind=snap owner=chat-live snap=codex_tui__markdown_render__list_spacing_tests__list_spacing_is_uniform_per_list_and_preserves_item_paragraphs
#[test]
fn list_spacing_is_uniform_per_list_and_preserves_item_paragraphs() {
    // The oracle joins ten stages (five sources, Compact and Uniform) with `---`; the screen
    // shows one spacing. The stage split is a kit gap, reported. The five sources are five turns.
    kit::pending(
        Owner::ChatLive,
        "the chat has no list spacing choice (Compact or Uniform): one spacing is drawn, and the oracle snapshot joins ten stages",
    );
    let s = Screen::build(Owner::ChatLive)
        .markdown(24, 120)
        .script(include_str!(
            "fixtures/markdown_streaming_1/list_spacing_is_uniform_per_list_and_preserves_item_paragraphs.jsonl"
        ))
        .open();
    s.assert_snap("codex_tui__markdown_render__list_spacing_tests__list_spacing_is_uniform_per_list_and_preserves_item_paragraphs");
}

// ORACLE markdown_render/list_spacing_tests.rs:40 list_spacing_preserves_links_and_source_when_removing_separators kind=ui owner=chat-live
#[test]
fn list_spacing_preserves_links_and_source_when_removing_separators() {
    // The oracle asserts Compact and Uniform render equal (two renders, not a screen); the
    // visible half, the links survive the list, is asserted here.
    let s = Screen::build(Owner::ChatLive)
        .markdown(100, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_1/list_spacing_preserves_links_and_source_when_removing_separators.jsonl"
        ))
        .open();
    s.assert_link("One", "https://example.com/one");
    s.assert_link("Two", "https://example.com/two");
    s.assert_contains("Three");
}

// ---- oracle: markdown_render/math_display_tests.rs (12 screen-level tests here; 3 state tests left to chat-live)
// ORACLE markdown_render/math_display_tests.rs:23 unicode_math_snapshot kind=snap owner=chat-live snap=codex_tui__markdown_render__math__display_tests__unicode_math_snapshot
#[test]
fn unicode_math_snapshot() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no math renderer: LaTeX display and inline math are drawn as their source",
    );
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 60)
        .script(include_str!(
            "fixtures/markdown_streaming_1/unicode_math_snapshot.jsonl"
        ))
        .open();
    s.assert_snap("codex_tui__markdown_render__math__display_tests__unicode_math_snapshot");
}

// ORACLE markdown_render/math_display_tests.rs:72 issue_48403_renders_zero_and_multiline_display kind=snap owner=chat-live snap=codex_tui__markdown_render__math__display_tests__issue_48403_renders_zero_and_multiline_display
#[test]
fn issue_48403_renders_zero_and_multiline_display() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no math renderer: the zero and multiline display formula is drawn as its source",
    );
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 60)
        .script(include_str!(
            "fixtures/markdown_streaming_1/issue_48403_renders_zero_and_multiline_display.jsonl"
        ))
        .open();
    s.assert_snap("codex_tui__markdown_render__math__display_tests__issue_48403_renders_zero_and_multiline_display");
}

// ORACLE markdown_render/math_display_tests.rs:87 unicode_math_schrodinger_equation_snapshot kind=snap owner=chat-live snap=codex_tui__markdown_render__math__display_tests__unicode_math_schrodinger_equation_snapshot
#[test]
fn unicode_math_schrodinger_equation_snapshot() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no math renderer: the display equation is drawn as its source",
    );
    let s = Screen::build(Owner::ChatLive)
        .markdown(100, 60)
        .script(include_str!(
            "fixtures/markdown_streaming_1/unicode_math_schrodinger_equation_snapshot.jsonl"
        ))
        .open();
    s.assert_snap("codex_tui__markdown_render__math__display_tests__unicode_math_schrodinger_equation_snapshot");
}

// ORACLE markdown_render/math_display_tests.rs:101 unicode_math_optimization_model kind=snap owner=chat-live snap=codex_tui__markdown_render__math__display_tests__unicode_math_optimization_model
#[test]
fn unicode_math_optimization_model() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no math renderer: the boxed aligned model is drawn with its LaTeX commands",
    );
    let s = Screen::build(Owner::ChatLive)
        .markdown(240, 60)
        .script(include_str!(
            "fixtures/markdown_streaming_1/unicode_math_optimization_model.jsonl"
        ))
        .open();
    s.assert_absent("\\");
    s.assert_snap(
        "codex_tui__markdown_render__math__display_tests__unicode_math_optimization_model",
    );
}

// ORACLE markdown_render/math_display_tests.rs:141 unicode_math_display_limits kind=snap owner=chat-live snap=codex_tui__markdown_render__math__display_tests__unicode_math_display_limits
#[test]
fn unicode_math_display_limits() {
    // The oracle also asserts that `render` returns equal output for two source spellings and
    // None for invalid scripts, and that a backticked display formula renders the same as the
    // bare one: internal results and equalities, not drawn; the snapshot is the visible half.
    kit::pending(
        Owner::ChatLive,
        "the chat has no math renderer: display limits are drawn as their source",
    );
    let s = Screen::build(Owner::ChatLive)
        .markdown(100, 60)
        .script(include_str!(
            "fixtures/markdown_streaming_1/unicode_math_display_limits.jsonl"
        ))
        .open();
    s.assert_snap("codex_tui__markdown_render__math__display_tests__unicode_math_display_limits");
}

// ORACLE markdown_render/math_display_tests.rs:192 unicode_math_grouped_scripts_and_annotations kind=snap owner=chat-live snap=codex_tui__markdown_render__math__display_tests__unicode_math_grouped_scripts_and_annotations
#[test]
fn unicode_math_grouped_scripts_and_annotations() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no math renderer: inline grouped scripts and annotations are drawn as their source",
    );
    let s = Screen::build(Owner::ChatLive)
        .markdown(100, 60)
        .script(include_str!(
            "fixtures/markdown_streaming_1/unicode_math_grouped_scripts_and_annotations.jsonl"
        ))
        .open();
    s.assert_snap("codex_tui__markdown_render__math__display_tests__unicode_math_grouped_scripts_and_annotations");
}

// ORACLE markdown_render/math_display_tests.rs:202 integral_limits_and_parenthesized_matrices kind=snap owner=chat-live snap=codex_tui__markdown_render__math__display_tests__integral_limits_and_parenthesized_matrices
#[test]
fn integral_limits_and_parenthesized_matrices() {
    // The oracle also asserts each formula at width 10 renders as its backticked form: two
    // renders compared, not drawn; the snapshot is the visible half.
    kit::pending(
        Owner::ChatLive,
        "the chat has no math renderer: integral limits and parenthesized matrices are drawn as their source",
    );
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 60)
        .script(include_str!(
            "fixtures/markdown_streaming_1/integral_limits_and_parenthesized_matrices.jsonl"
        ))
        .open();
    s.assert_snap("codex_tui__markdown_render__math__display_tests__integral_limits_and_parenthesized_matrices");
}

// ORACLE markdown_render/math_display_tests.rs:220 unicode_math_aligned_fraction_and_narrow_fallback kind=snap owner=chat-live snap=codex_tui__markdown_render__math__display_tests__unicode_math_aligned_fraction_and_narrow_fallback
#[test]
fn unicode_math_aligned_fraction_and_narrow_fallback() {
    // The oracle snapshot joins a Wide render (width 80) and a Narrow one (width 12) under two
    // labels: one screen cannot show both widths, and no kit call takes one labelled part of a
    // joined snapshot: kit gap, reported.
    kit::pending(
        Owner::ChatLive,
        "the chat has no math renderer: the aligned fraction is drawn as its source at width 80 and 12",
    );
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 60)
        .script(include_str!(
            "fixtures/markdown_streaming_1/unicode_math_aligned_fraction_and_narrow_fallback.jsonl"
        ))
        .open();
    s.assert_snap("codex_tui__markdown_render__math__display_tests__unicode_math_aligned_fraction_and_narrow_fallback");
}

// ORACLE markdown_render/math_display_tests.rs:235 unicode_math_aligned_preserves_fraction_geometry kind=snap owner=chat-live snap=codex_tui__markdown_render__math__display_tests__unicode_math_aligned_preserves_fraction_geometry
#[test]
fn unicode_math_aligned_preserves_fraction_geometry() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no math renderer: aligned fractions are drawn as their source",
    );
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 60)
        .script(include_str!(
            "fixtures/markdown_streaming_1/unicode_math_aligned_preserves_fraction_geometry.jsonl"
        ))
        .open();
    s.assert_snap("codex_tui__markdown_render__math__display_tests__unicode_math_aligned_preserves_fraction_geometry");
    let spaced = Screen::build(Owner::ChatLive)
        .markdown(80, 60)
        .script(include_str!(
            "fixtures/markdown_streaming_1/unicode_math_aligned_preserves_fraction_geometry_spaced.jsonl"
        ))
        .open();
    s.assert_same_rows(&spaced.shot());
}

// ORACLE markdown_render/math_display_tests.rs:260 unicode_math_structured_bounds_and_invalid_input kind=ui owner=chat-live
#[test]
fn unicode_math_structured_bounds_and_invalid_input() {
    // The oracle's `render` results (the 252-character width, the None cases) are internal; the
    // visible half is the sixteen-row aligned block, drawn as sixteen `x =1` rows.
    kit::pending(
        Owner::ChatLive,
        "the chat has no math renderer: the sixteen-row aligned block is drawn as its LaTeX source",
    );
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 40)
        .script(include_str!(
            "fixtures/markdown_streaming_1/unicode_math_structured_bounds_and_invalid_input.jsonl"
        ))
        .open();
    s.assert_in_order(&["x =1"; 16]);
}

// ORACLE markdown_render/math_display_tests.rs:313 unicode_math_narrow_layout_stays_meaningful kind=snap owner=chat-live snap=codex_tui__markdown_render__math__display_tests__unicode_math_narrow_layout_stays_meaningful
#[test]
fn unicode_math_narrow_layout_stays_meaningful() {
    // The oracle also asserts the backticked and bare forms render the same at widths 80 and 14,
    // and that a too-wide fraction stays source: equalities of two renders, not drawn.
    kit::pending(
        Owner::ChatLive,
        "the chat has no math renderer: at width 12 a display fraction is drawn as its source, wrapped as text",
    );
    let s = Screen::build(Owner::ChatLive)
        .markdown(12, 60)
        .script(include_str!(
            "fixtures/markdown_streaming_1/unicode_math_narrow_layout_stays_meaningful.jsonl"
        ))
        .open();
    s.assert_snap("codex_tui__markdown_render__math__display_tests__unicode_math_narrow_layout_stays_meaningful");
}

// ORACLE markdown_render/math_display_tests.rs:355 unicode_math_nested_fractions_and_oversized_pending_stay_source kind=ui owner=chat-live
#[test]
fn unicode_math_nested_fractions_and_oversized_pending_stay_source() {
    // The oracle's `render` is None for nested fractions and its pending offset for a 5000-character
    // display is internal. The visible half: the nested fractions stay as typed source.
    kit::pending(
        Owner::ChatLive,
        "the chat has no math renderer: nested fractions are drawn as their source; the oversized pending display is not built; probe: the body passes without it (it checks less than the oracle claims)",
    );
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_1/unicode_math_nested_fractions_and_oversized_pending_stay_source.jsonl"
        ))
        .open();
    s.assert_contains(r"\frac{\frac{a}{b}}{c}");
    s.assert_contains(r"\frac{a}{\frac{b}{c}}");
}
