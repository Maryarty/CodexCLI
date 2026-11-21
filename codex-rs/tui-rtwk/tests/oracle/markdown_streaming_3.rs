//! Shard `markdown_streaming_3` (screens S11, owner chat-live): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen};

/// The non-blank rows of an oracle snapshot of rendered text (`plain_lines`).
fn snap_lines(name: &str) -> Vec<String> {
    kit::snap_rows(name, "")
        .into_iter()
        .filter(|r| !r.is_empty())
        .collect()
}

// ---- oracle: markdown_render_tests.rs (40 screen-level tests here; 0 state tests left to chat-live)
const G63: &str = "G63: MarkdownView joins a soft break with a blank; the oracle breaks the line (flutter_markdown softLineBreak)";

// ORACLE markdown_render_tests.rs:697 loose_list_item_multiple_paragraphs kind=ui owner=chat-live
#[test]
fn loose_list_item_multiple_paragraphs() {
    kit::pending(
        Owner::ChatLive,
        "item 11 (lander note): the ordered-list marker colour (oracle LightBlue) is src/app/theme.rs transcript_markdown, startup-finish's area",
    );
    // The oracle's blank rows between the paragraphs and the items are not asserted: the kit has no row-gap check.
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_3/loose_list_item_multiple_paragraphs.jsonl"
        ))
        .open();
    s.assert_in_order(&[
        "1. First paragraph",
        "Second paragraph of same item",
        "2. Next item",
    ]);
    s.assert_text_style("1. ", "fg=LightBlue");
    s.assert_text_style("2. ", "fg=LightBlue");
}

// ORACLE markdown_render_tests.rs:711 tight_item_with_soft_break kind=ui owner=chat-live
#[test]
fn tight_item_with_soft_break() {
    kit::pending(Owner::ChatLive, G63);
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_3/tight_item_with_soft_break.jsonl"
        ))
        .open();
    s.assert_has_row("• • item line1");
    s.assert_has_row("    item line2");
}

// ORACLE markdown_render_tests.rs:723 deeply_nested_mixed_three_levels kind=ui owner=chat-live
#[test]
fn deeply_nested_mixed_three_levels() {
    kit::pending(
        Owner::ChatLive,
        "item 11 (lander note): the ordered-list marker colour (oracle LightBlue) is src/app/theme.rs transcript_markdown, startup-finish's area",
    );
    // The oracle's blank row before `2. D` is not asserted: the kit has no row-gap check.
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_3/deeply_nested_mixed_three_levels.jsonl"
        ))
        .open();
    s.assert_in_order(&["1. A", "• B", "1. C", "2. D"]);
    s.assert_text_style("2. ", "fg=LightBlue");
}

// ORACLE markdown_render_tests.rs:738 loose_items_due_to_blank_line_between_items kind=ui owner=chat-live
#[test]
fn loose_items_due_to_blank_line_between_items() {
    kit::pending(
        Owner::ChatLive,
        "item 11 (lander note): the ordered-list marker colour (oracle LightBlue) is src/app/theme.rs transcript_markdown, startup-finish's area",
    );
    // The oracle's blank row between the two items is not asserted: the kit has no row-gap check.
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_3/loose_items_due_to_blank_line_between_items.jsonl"
        ))
        .open();
    s.assert_in_order(&["1. First", "2. Second"]);
    s.assert_text_style("1. ", "fg=LightBlue");
    s.assert_text_style("2. ", "fg=LightBlue");
}

// ORACLE markdown_render_tests.rs:749 mixed_tight_then_loose_in_one_list kind=ui owner=chat-live
#[test]
fn mixed_tight_then_loose_in_one_list() {
    kit::pending(
        Owner::ChatLive,
        "item 11 (lander note): the ordered-list marker colour (oracle LightBlue) is src/app/theme.rs transcript_markdown, startup-finish's area",
    );
    // The oracle has no blank row between the items: adjacency of rows is not asserted (the kit has no row-gap check).
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_3/mixed_tight_then_loose_in_one_list.jsonl"
        ))
        .open();
    s.assert_in_order(&["1. Tight", "2. Loose"]);
    s.assert_text_style("2. ", "fg=LightBlue");
}

// ORACLE markdown_render_tests.rs:760 ordered_item_with_indented_continuation_is_tight kind=ui owner=chat-live
#[test]
fn ordered_item_with_indented_continuation_is_tight() {
    kit::pending(
        Owner::ChatLive,
        "item 11 (lander note): the ordered-list marker colour (oracle LightBlue) is src/app/theme.rs transcript_markdown, startup-finish's area",
    );
    // The oracle has no blank row between `Foo` and `Bar`: adjacency is not asserted (the kit has no row-gap check).
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_3/ordered_item_with_indented_continuation_is_tight.jsonl"
        ))
        .open();
    s.assert_in_order(&["1. Foo", "Bar"]);
    s.assert_text_style("1. ", "fg=LightBlue");
}

// ORACLE markdown_render_tests.rs:771 inline_code kind=ui owner=chat-live
#[test]
fn inline_code() {
    // The code span's style is `MarkdownStyles::default().code`, a theme value with no literal in the test: not asserted.
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_3/inline_code.jsonl"
        ))
        .open();
    s.assert_contains("Example of Inline code");
}

// ORACLE markdown_render_tests.rs:782 inline_code_and_file_paths_follow_syntax_theme kind=snap owner=chat-live snap=codex_tui__markdown_render__markdown_render_tests__inline_code_and_file_paths_follow_syntax_theme
#[test]
fn inline_code_and_file_paths_follow_syntax_theme() {
    kit::pending(
        Owner::ChatLive,
        "the kit has no syntax-theme seed for the oracle's six themes; first difference at the chat-cells-2 probe: the screen has no `src/lib.rs`:",
    );
    // The oracle's snapshot is a Debug dump of six themes; the kit has no theme seed, so the theme rows are not compared.
    let s = Screen::build(Owner::ChatLive)
        .markdown(24, 40)
        .script(include_str!(
            "fixtures/markdown_streaming_3/inline_code_and_file_paths_follow_syntax_theme.jsonl"
        ))
        .open();
    s.assert_in_order(&[
        "Heading",
        "Bold Italic",
        "Quote",
        "Ordered",
        "Unordered",
        "Use src/main.rs and",
    ]);
    s.assert_contains("src/main.rs");
    s.assert_contains("src/lib.rs");
}

// ORACLE markdown_render_tests.rs:848 strong kind=ui owner=chat-live
#[test]
fn strong() {
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!("fixtures/markdown_streaming_3/strong.jsonl"))
        .open();
    s.assert_text_style("Strong", "bold");
}

// ORACLE markdown_render_tests.rs:857 emphasis kind=ui owner=chat-live
#[test]
fn emphasis() {
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!("fixtures/markdown_streaming_3/emphasis.jsonl"))
        .open();
    s.assert_text_style("Emphasis", "italic");
}

// ORACLE markdown_render_tests.rs:866 strikethrough kind=ui owner=chat-live
#[test]
fn strikethrough() {
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_3/strikethrough.jsonl"
        ))
        .open();
    s.assert_text_style("Strikethrough", "crossed_out");
}

// ORACLE markdown_render_tests.rs:874 strong_emphasis kind=ui owner=chat-live
#[test]
fn strong_emphasis() {
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_3/strong_emphasis.jsonl"
        ))
        .open();
    s.assert_text_style("Strong", "bold");
    s.assert_text_style("emphasis", "bold italic");
}

// ORACLE markdown_render_tests.rs:885 link kind=ui owner=chat-live
#[test]
fn link() {
    kit::pending(
        Owner::ChatLive,
        "first difference at the chat-cells-2 probe (cause not traced yet): the screen has no `Link (https://example.com)`:",
    );
    // The link's accent foreground is contrast-computed (`style.rs:307`), with no literal in the test: not asserted.
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!("fixtures/markdown_streaming_3/link.jsonl"))
        .open();
    s.assert_contains("Link (https://example.com)");
    s.assert_text_style("Link", "underlined");
    s.assert_text_style("https://example.com", "underlined");
}

// ORACLE markdown_render_tests.rs:898 web_link_labels_use_link_style_and_preserve_inline_formatting kind=ui owner=chat-live
#[test]
fn web_link_labels_use_link_style_and_preserve_inline_formatting() {
    kit::pending(
        Owner::ChatLive,
        "first difference at the chat-cells-2 probe (cause not traced yet): `before plain (https://example.com) after code` is not on the screen after the fragments before it ([\"before plain (https://example.com) after code...",
    );
    // The four labels are four paragraphs of one fixture (`fixtures/markdown_streaming_3/`), one screen.
    let s = Screen::build(Owner::ChatLive).markdown(80, 24).script(include_str!("fixtures/markdown_streaming_3/web_link_labels_use_link_style_and_preserve_inline_formatting.jsonl")).open();
    s.assert_in_order(&[
        "before plain (https://example.com) after code",
        "before code (https://example.com) after code",
        "before bold (https://example.com) after code",
        "before italic (https://example.com) after code",
    ]);
    s.assert_text_style("bold", "bold underlined");
    s.assert_text_style("italic", "italic underlined");
}

// ORACLE markdown_render_tests.rs:926 web_link_labels_keep_link_style_in_wrapped_prose_and_tables kind=ui owner=chat-live
#[test]
fn web_link_labels_keep_link_style_in_wrapped_prose_and_tables() {
    // The oracle's two inputs (prose, table) at widths 32 and 80: one screen per input and width.
    for w in [32u16, 80] {
        let s = Screen::build(Owner::ChatLive).markdown(w, 24).script(include_str!("fixtures/markdown_streaming_3/web_link_labels_keep_link_style_in_wrapped_prose_and_tables_prose.jsonl")).open();
        s.assert_in_order(&["plain", "code", "<b>"]);
        s.assert_text_style("plain", "underlined");
        s.assert_text_style("<b>", "underlined");
        let t = Screen::build(Owner::ChatLive).markdown(w, 24).script(include_str!("fixtures/markdown_streaming_3/web_link_labels_keep_link_style_in_wrapped_prose_and_tables_table.jsonl")).open();
        t.assert_in_order(&["plain", "code", "<b>"]);
        t.assert_text_style("<b>", "underlined");
    }
}

// ORACLE markdown_render_tests.rs:956 web_link_labels_have_a_visible_underline_snapshot kind=snap owner=chat-live snap=codex_tui__markdown_render__markdown_render_tests__web_link_labels_have_a_visible_underline_snapshot
#[test]
fn web_link_labels_have_a_visible_underline_snapshot() {
    kit::pending(
        Owner::ChatLive,
        "first difference at the chat-cells-2 probe (cause not traced yet): `plain plain (https://example.com)` is not on the screen after the fragments before it ([\"plain plain (https://example.com)\"]):",
    );
    // The oracle asserts a Debug dump of the Text; the drawn row and the link underline are what is compared.
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_3/web_link_labels_have_a_visible_underline_snapshot.jsonl"
        ))
        .open();
    s.assert_in_order(&["plain plain (https://example.com)"]);
    s.assert_text_style("https://example.com", "underlined");
}

// ORACLE markdown_render_tests.rs:964 file_link_preserves_label_and_decodes_fallback_destination kind=ui owner=chat-live
#[test]
fn file_link_preserves_label_and_decodes_fallback_destination() {
    kit::pending(
        Owner::ChatLive,
        "the kit cannot set the session cwd that the oracle shows local file links relative to (no cwd seed); first difference at the chat-cells-2 probe: the screen has no `/Users/example/code/codex/codex-rs/tui/src/My%20File.rs (codex-rs/tui/src/My File.rs)`:",
    );
    let s = Screen::build(Owner::ChatLive).markdown(80, 24).script(include_str!("fixtures/markdown_streaming_3/file_link_preserves_label_and_decodes_fallback_destination.jsonl")).open();
    s.assert_contains(
        "/Users/example/code/codex/codex-rs/tui/src/My%20File.rs (codex-rs/tui/src/My File.rs)",
    );
}

// ORACLE markdown_render_tests.rs:982 file_link_keeps_descriptive_label_and_target kind=snap owner=chat-live snap=codex_tui__markdown_render__markdown_render_tests__file_link_keeps_descriptive_label_and_target
#[test]
fn file_link_keeps_descriptive_label_and_target() {
    kit::pending(
        Owner::ChatLive,
        "the kit cannot set the session cwd that the oracle shows local file links relative to (no cwd seed); first difference at the chat-cells-2 probe: `Your codex launcher automatically adds those overrides (project/dotslash-gen/bin/codex:1105), even though you did not specify any.` is not on the ...",
    );
    let s = Screen::build(Owner::ChatLive)
        .markdown(120, 40)
        .script(include_str!(
            "fixtures/markdown_streaming_3/file_link_keeps_descriptive_label_and_target.jsonl"
        ))
        .open();
    let rows = snap_lines("file_link_keeps_descriptive_label_and_target");
    let frags: Vec<&str> = rows.iter().map(String::as_str).collect();
    s.assert_in_order(&frags);
}

// ORACLE markdown_render_tests.rs:1017 file_link_preserves_tilde_and_absolute_destinations kind=snap owner=chat-live snap=codex_tui__markdown_render__markdown_render_tests__file_link_preserves_tilde_and_absolute_destinations
#[test]
fn file_link_preserves_tilde_and_absolute_destinations() {
    kit::pending(
        Owner::ChatLive,
        "the kit cannot set the session cwd that the oracle shows local file links relative to (no cwd seed); first difference at the chat-cells-2 probe: `~/notes (~/notes)` is not on the screen after the fragments before it ([\"~/notes (~/notes)\", \"/home/alice/notes (/home/alice/notes)\", \"/home/alice...",
    );
    let s = Screen::build(Owner::ChatLive).markdown(120, 40).script(include_str!("fixtures/markdown_streaming_3/file_link_preserves_tilde_and_absolute_destinations.jsonl")).open();
    let rows = snap_lines("file_link_preserves_tilde_and_absolute_destinations");
    let frags: Vec<&str> = rows.iter().map(String::as_str).collect();
    s.assert_in_order(&frags);
}

// ORACLE markdown_render_tests.rs:1031 file_link_preserves_path_labels_and_formats_destinations kind=snap owner=chat-live snap=codex_tui__markdown_render__markdown_render_tests__file_link_preserves_path_labels_and_formats_destinations
#[test]
fn file_link_preserves_path_labels_and_formats_destinations() {
    kit::pending(
        Owner::ChatLive,
        "the kit cannot set the session cwd that the oracle shows local file links relative to (no cwd seed); first difference at the chat-cells-2 probe: `file:///repo/src/lib.rs (src/lib.rs)` is not on the screen after the fragments before it ([\"file:///repo/src/lib.rs (src/lib.rs)\", \"SRC\\\\LIB.RS (s...",
    );
    let s = Screen::build(Owner::ChatLive).markdown(120, 40).script(include_str!("fixtures/markdown_streaming_3/file_link_preserves_path_labels_and_formats_destinations.jsonl")).open();
    let rows = snap_lines("file_link_preserves_path_labels_and_formats_destinations");
    let frags: Vec<&str> = rows.iter().map(String::as_str).collect();
    s.assert_in_order(&frags);
}

// ORACLE markdown_render_tests.rs:1065 file_link_decodes_percent_encoded_bare_path_destination kind=ui owner=chat-live
#[test]
fn file_link_decodes_percent_encoded_bare_path_destination() {
    kit::pending(
        Owner::ChatLive,
        "the kit cannot set the session cwd that the oracle shows local file links relative to (no cwd seed); first difference at the chat-cells-2 probe: the screen has no `open Example Folder/Résumé/report.md (Example Folder/Résumé/report.md)`:",
    );
    let s = Screen::build(Owner::ChatLive).markdown(80, 24).script(include_str!("fixtures/markdown_streaming_3/file_link_decodes_percent_encoded_bare_path_destination.jsonl")).open();
    s.assert_contains("open Example Folder/Résumé/report.md (Example Folder/Résumé/report.md)");
}

// ORACLE markdown_render_tests.rs:1083 file_link_preserves_labels_with_invalid_percent_encoding kind=snap owner=chat-live snap=codex_tui__markdown_render__markdown_render_tests__file_link_preserves_labels_with_invalid_percent_encoding
#[test]
fn file_link_preserves_labels_with_invalid_percent_encoding() {
    kit::pending(
        Owner::ChatLive,
        "the kit cannot set the session cwd that the oracle shows local file links relative to (no cwd seed); first difference at the chat-cells-2 probe: `bad%FF label (/tmp/)` is not on the screen after the fragments before it ([\"bad%FF label (/tmp/)\", \"bad%FF label (/)\"]):",
    );
    let s = Screen::build(Owner::ChatLive).markdown(120, 40).script(include_str!("fixtures/markdown_streaming_3/file_link_preserves_labels_with_invalid_percent_encoding.jsonl")).open();
    let rows = snap_lines("file_link_preserves_labels_with_invalid_percent_encoding");
    let frags: Vec<&str> = rows.iter().map(String::as_str).collect();
    s.assert_in_order(&frags);
}

// ORACLE markdown_render_tests.rs:1092 file_link_preserves_directory_labels_and_destinations kind=snap owner=chat-live snap=codex_tui__markdown_render__markdown_render_tests__file_link_preserves_directory_labels_and_destinations
#[test]
fn file_link_preserves_directory_labels_and_destinations() {
    kit::pending(
        Owner::ChatLive,
        "the kit cannot set the session cwd that the oracle shows local file links relative to (no cwd seed); first difference at the chat-cells-2 probe: `dir (./dir/)` is not on the screen after the fragments before it ([\"dir (./dir/)\", \"dir/ (./dir)\", \"dir (/outside/dir/)\", \"dir/ (/outside/dir)\"]):",
    );
    let s = Screen::build(Owner::ChatLive).markdown(120, 40).script(include_str!("fixtures/markdown_streaming_3/file_link_preserves_directory_labels_and_destinations.jsonl")).open();
    let rows = snap_lines("file_link_preserves_directory_labels_and_destinations");
    let frags: Vec<&str> = rows.iter().map(String::as_str).collect();
    s.assert_in_order(&frags);
}

// ORACLE markdown_render_tests.rs:1101 file_link_keeps_unrelated_relative_label_with_matching_suffix kind=snap owner=chat-live snap=codex_tui__markdown_render__markdown_render_tests__file_link_keeps_unrelated_relative_label_with_matching_suffix
#[test]
fn file_link_keeps_unrelated_relative_label_with_matching_suffix() {
    kit::pending(
        Owner::ChatLive,
        "the kit cannot set the session cwd that the oracle shows local file links relative to (no cwd seed); first difference at the chat-cells-2 probe: the screen has no `other/src/lib.rs (src/lib.rs)`:",
    );
    // The oracle's `src/lib.rs` is a code span with a theme style: not asserted.
    let s = Screen::build(Owner::ChatLive).markdown(80, 24).script(include_str!("fixtures/markdown_streaming_3/file_link_keeps_unrelated_relative_label_with_matching_suffix.jsonl")).open();
    s.assert_contains("other/src/lib.rs (src/lib.rs)");
}

// ORACLE markdown_render_tests.rs:1115 file_link_appends_line_number_when_label_lacks_it kind=ui owner=chat-live
#[test]
fn file_link_appends_line_number_when_label_lacks_it() {
    kit::pending(
        Owner::ChatLive,
        "the kit cannot set the session cwd that the oracle shows local file links relative to (no cwd seed); first difference at the chat-cells-2 probe: the screen has no `markdown_render.rs (codex-rs/tui/src/markdown_render.rs:74)`:",
    );
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_3/file_link_appends_line_number_when_label_lacks_it.jsonl"
        ))
        .open();
    s.assert_contains("markdown_render.rs (codex-rs/tui/src/markdown_render.rs:74)");
}

// ORACLE markdown_render_tests.rs:1133 file_link_keeps_absolute_paths_outside_cwd kind=ui owner=chat-live
#[test]
fn file_link_keeps_absolute_paths_outside_cwd() {
    kit::pending(
        Owner::ChatLive,
        "the kit cannot set the session cwd that the oracle shows local file links relative to (no cwd seed); first difference at the chat-cells-2 probe: the screen has no `README.md:74 (/Users/example/code/codex/README.md:74)`:",
    );
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_3/file_link_keeps_absolute_paths_outside_cwd.jsonl"
        ))
        .open();
    s.assert_contains("README.md:74 (/Users/example/code/codex/README.md:74)");
}

// ORACLE markdown_render_tests.rs:1151 file_links_preserve_foreign_windows_paths_and_anchors_snapshot kind=snap owner=chat-live
#[test]
fn file_links_preserve_foreign_windows_paths_and_anchors_snapshot() {
    kit::pending(
        Owner::ChatLive,
        "the kit cannot set the session cwd that the oracle shows local file links relative to (no cwd seed); first difference at the chat-cells-2 probe: `report (Quarterly Report.xlsx:4:2)` is not on the screen after the fragments before it ([\"report (Quarterly Report.xlsx:4:2)\", \"report (Quarterly ...",
    );
    // The oracle's expected rows are an inline snapshot of the test, not a `.snap` file.
    let s = Screen::build(Owner::ChatLive).markdown(80, 24).script(include_str!("fixtures/markdown_streaming_3/file_links_preserve_foreign_windows_paths_and_anchors_snapshot.jsonl")).open();
    s.assert_in_order(&[
        "report (Quarterly Report.xlsx:4:2)",
        "report (Quarterly Report.xlsx:4:2)",
    ]);
}

// ORACLE markdown_render_tests.rs:1172 file_link_appends_hash_anchor_when_label_lacks_it kind=ui owner=chat-live
#[test]
fn file_link_appends_hash_anchor_when_label_lacks_it() {
    kit::pending(
        Owner::ChatLive,
        "the kit cannot set the session cwd that the oracle shows local file links relative to (no cwd seed); first difference at the chat-cells-2 probe: the screen has no `markdown_render.rs (codex-rs/tui/src/markdown_render.rs:74:3)`:",
    );
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_3/file_link_appends_hash_anchor_when_label_lacks_it.jsonl"
        ))
        .open();
    s.assert_contains("markdown_render.rs (codex-rs/tui/src/markdown_render.rs:74:3)");
}

// ORACLE markdown_render_tests.rs:1190 file_link_uses_target_path_for_hash_anchor kind=ui owner=chat-live
#[test]
fn file_link_uses_target_path_for_hash_anchor() {
    kit::pending(
        Owner::ChatLive,
        "the kit cannot set the session cwd that the oracle shows local file links relative to (no cwd seed); first difference at the chat-cells-2 probe: the screen has no `markdown_render.rs#L74C3 (codex-rs/tui/src/markdown_render.rs:74:3)`:",
    );
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_3/file_link_uses_target_path_for_hash_anchor.jsonl"
        ))
        .open();
    s.assert_contains("markdown_render.rs#L74C3 (codex-rs/tui/src/markdown_render.rs:74:3)");
}

// ORACLE markdown_render_tests.rs:1208 file_link_appends_range_when_label_lacks_it kind=ui owner=chat-live
#[test]
fn file_link_appends_range_when_label_lacks_it() {
    kit::pending(
        Owner::ChatLive,
        "the kit cannot set the session cwd that the oracle shows local file links relative to (no cwd seed); first difference at the chat-cells-2 probe: the screen has no `markdown_render.rs (codex-rs/tui/src/markdown_render.rs:74:3-76:9)`:",
    );
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_3/file_link_appends_range_when_label_lacks_it.jsonl"
        ))
        .open();
    s.assert_contains("markdown_render.rs (codex-rs/tui/src/markdown_render.rs:74:3-76:9)");
}

// ORACLE markdown_render_tests.rs:1226 file_link_uses_target_path_for_range kind=ui owner=chat-live
#[test]
fn file_link_uses_target_path_for_range() {
    kit::pending(
        Owner::ChatLive,
        "the kit cannot set the session cwd that the oracle shows local file links relative to (no cwd seed); first difference at the chat-cells-2 probe: the screen has no `markdown_render.rs:74:3-76:9 (codex-rs/tui/src/markdown_render.rs:74:3-76:9)`:",
    );
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_3/file_link_uses_target_path_for_range.jsonl"
        ))
        .open();
    s.assert_contains(
        "markdown_render.rs:74:3-76:9 (codex-rs/tui/src/markdown_render.rs:74:3-76:9)",
    );
}

// ORACLE markdown_render_tests.rs:1244 file_link_appends_hash_range_when_label_lacks_it kind=ui owner=chat-live
#[test]
fn file_link_appends_hash_range_when_label_lacks_it() {
    kit::pending(
        Owner::ChatLive,
        "the kit cannot set the session cwd that the oracle shows local file links relative to (no cwd seed); first difference at the chat-cells-2 probe: the screen has no `markdown_render.rs (codex-rs/tui/src/markdown_render.rs:74:3-76:9)`:",
    );
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_3/file_link_appends_hash_range_when_label_lacks_it.jsonl"
        ))
        .open();
    s.assert_contains("markdown_render.rs (codex-rs/tui/src/markdown_render.rs:74:3-76:9)");
}

// ORACLE markdown_render_tests.rs:1262 multiline_file_link_label_after_styled_prefix_does_not_panic kind=ui owner=chat-live
#[test]
fn multiline_file_link_label_after_styled_prefix_does_not_panic() {
    kit::pending(
        Owner::ChatLive,
        "the kit cannot set the session cwd that the oracle shows local file links relative to (no cwd seed); first difference at the chat-cells-2 probe: the screen has no `bold plain foo bar (codex-rs/tui/src/markdown_render.rs:74:3)`:",
    );
    let s = Screen::build(Owner::ChatLive).markdown(80, 24).script(include_str!("fixtures/markdown_streaming_3/multiline_file_link_label_after_styled_prefix_does_not_panic.jsonl")).open();
    s.assert_contains("bold plain foo bar (codex-rs/tui/src/markdown_render.rs:74:3)");
    s.assert_text_style("bold", "bold");
}

// ORACLE markdown_render_tests.rs:1285 file_link_uses_target_path_for_hash_range kind=ui owner=chat-live
#[test]
fn file_link_uses_target_path_for_hash_range() {
    kit::pending(
        Owner::ChatLive,
        "the kit cannot set the session cwd that the oracle shows local file links relative to (no cwd seed); first difference at the chat-cells-2 probe: the screen has no `markdown_render.rs#L74C3-L76C9 (codex-rs/tui/src/markdown_render.rs:74:3-76:9)`:",
    );
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_3/file_link_uses_target_path_for_hash_range.jsonl"
        ))
        .open();
    s.assert_contains(
        "markdown_render.rs#L74C3-L76C9 (codex-rs/tui/src/markdown_render.rs:74:3-76:9)",
    );
}

// ORACLE markdown_render_tests.rs:1303 url_link_shows_destination kind=ui owner=chat-live
#[test]
fn url_link_shows_destination() {
    kit::pending(
        Owner::ChatLive,
        "first difference at the chat-cells-2 probe (cause not traced yet): the screen has no `docs (https://example.com/docs)`:",
    );
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_3/url_link_shows_destination.jsonl"
        ))
        .open();
    s.assert_contains("docs (https://example.com/docs)");
    s.assert_text_style("docs", "underlined");
    s.assert_text_style("https://example.com/docs", "underlined");
}

// ORACLE markdown_render_tests.rs:1316 markdown_render_file_link_snapshot kind=snap owner=chat-live snap=codex_tui__markdown_render__markdown_render_tests__markdown_render_file_link_snapshot
#[test]
fn markdown_render_file_link_snapshot() {
    kit::pending(
        Owner::ChatLive,
        "the kit cannot set the session cwd that the oracle shows local file links relative to (no cwd seed); first difference at the chat-cells-2 probe: `analysis.md (analysis/021_all_experiments_best_per_group/analysis.md)` is not on the screen after the fragments before it ([\"analysis.md (analysis...",
    );
    let s = Screen::build(Owner::ChatLive)
        .markdown(120, 40)
        .script(include_str!(
            "fixtures/markdown_streaming_3/markdown_render_file_link_snapshot.jsonl"
        ))
        .open();
    let rows = snap_lines("markdown_render_file_link_snapshot");
    let frags: Vec<&str> = rows.iter().map(String::as_str).collect();
    s.assert_in_order(&frags);
}

// ORACLE markdown_render_tests.rs:1329 unordered_list_local_file_link_stays_inline_with_following_text kind=ui owner=chat-live
#[test]
fn unordered_list_local_file_link_stays_inline_with_following_text() {
    kit::pending(
        Owner::ChatLive,
        "the kit cannot set the session cwd that the oracle shows local file links relative to (no cwd seed); first difference at the chat-cells-2 probe: `• binary (codex-rs/README.md:93): core is the agent/business logic, tui` is not on the screen after the fragments before it ([\"• binary (codex-rs/...",
    );
    let s = Screen::build(Owner::ChatLive).markdown(72, 24).script(include_str!("fixtures/markdown_streaming_3/unordered_list_local_file_link_stays_inline_with_following_text.jsonl")).open();
    s.assert_in_order(&[
        "• binary (codex-rs/README.md:93): core is the agent/business logic, tui",
        "is the terminal UI, exec is the headless automation surface, and cli",
        "is the top-level multitool binary.",
    ]);
}

// ORACLE markdown_render_tests.rs:1356 unordered_list_local_file_link_soft_break_before_colon_stays_inline kind=ui owner=chat-live
#[test]
fn unordered_list_local_file_link_soft_break_before_colon_stays_inline() {
    kit::pending(
        Owner::ChatLive,
        "the kit cannot set the session cwd that the oracle shows local file links relative to (no cwd seed); first difference at the chat-cells-2 probe: the screen has no `• binary (codex-rs/README.md:93): core is the agent/business logic.`:",
    );
    let s = Screen::build(Owner::ChatLive).markdown(72, 24).script(include_str!("fixtures/markdown_streaming_3/unordered_list_local_file_link_soft_break_before_colon_stays_inline.jsonl")).open();
    s.assert_contains("• binary (codex-rs/README.md:93): core is the agent/business logic.");
}

// ORACLE markdown_render_tests.rs:1379 consecutive_unordered_list_local_file_links_do_not_detach_paths kind=ui owner=chat-live
#[test]
fn consecutive_unordered_list_local_file_links_do_not_detach_paths() {
    kit::pending(
        Owner::ChatLive,
        "the kit cannot set the session cwd that the oracle shows local file links relative to (no cwd seed); first difference at the chat-cells-2 probe: `• binary (codex-rs/README.md:93): cli is the top-level multitool binary.` is not on the screen after the fragments before it ([\"• binary (codex-rs...",
    );
    let s = Screen::build(Owner::ChatLive).markdown(72, 24).script(include_str!("fixtures/markdown_streaming_3/consecutive_unordered_list_local_file_links_do_not_detach_paths.jsonl")).open();
    s.assert_in_order(&[
        "• binary (codex-rs/README.md:93): cli is the top-level multitool binary.",
        "• expectations (codex-rs/core/README.md:1): codex-core owns the real",
        "runtime behavior.",
    ]);
}

// ORACLE markdown_render_tests.rs:1406 code_block_known_lang_has_syntax_colors kind=ui owner=chat-live
#[test]
fn code_block_known_lang_has_syntax_colors() {
    // The oracle asserts that some span has a foreground colour; the kit has no any-coloured check, so the colour is not asserted.
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_3/code_block_known_lang_has_syntax_colors.jsonl"
        ))
        .open();
    s.assert_contains("fn main() {}");
}
