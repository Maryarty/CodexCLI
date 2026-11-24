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
// ORACLE markdown_render_tests.rs:697 loose_list_item_multiple_paragraphs kind=ui owner=chat-live
#[test]
fn loose_list_item_multiple_paragraphs() {
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
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_3/tight_item_with_soft_break.jsonl"
        ))
        .open();
    // The oracle's two lines, the item's bullet then its indent, one under the other.
    s.assert_row_then("• item line1", &["  item line2"]);
}

// ORACLE markdown_render_tests.rs:723 deeply_nested_mixed_three_levels kind=ui owner=chat-live
#[test]
fn deeply_nested_mixed_three_levels() {
    kit::pending(
        Owner::ChatLive,
        "G73: item 4, the oracle draws every unordered bullet as `• ` at any depth (tui markdown_render.rs:1131, styles.unordered_list_marker); rtwk's MarkdownView draws `•` at depth 1 and `◦` below, from Glyphs, with no bullet option",
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
    // The oracle's snapshot is a Debug dump of the styled lines under six themes; the colours
    // of its heading, link, ordered marker and path spans are asserted per theme (the first
    // two of the oracle's list are mocha on a dark and on a light terminal: the roles are
    // taken to read on the background they sit on).
    const DARK: (u32, u32) = (0xdc_dc_dc, 0x14_14_14);
    const LIGHT: (u32, u32) = (0x1e_1e_1e, 0xff_ff_ff);
    // (theme, terminal, heading, link, ordered marker, path)
    let rows = [
        (
            "catppuccin-mocha",
            DARK,
            "Rgb(243,139,168)",
            "Rgb(137,180,250)",
            "LightBlue",
            "Rgb(166,227,161)",
        ),
        (
            "catppuccin-mocha",
            LIGHT,
            "Rgb(169,97,117)",
            "Rgb(91,119,166)",
            "LightBlue",
            "Rgb(166,227,161)",
        ),
        (
            "catppuccin-latte",
            LIGHT,
            "Rgb(210,15,57)",
            "Rgb(30,102,245)",
            "LightBlue",
            "Rgb(64,160,43)",
        ),
        (
            "nord",
            DARK,
            "Rgb(136,192,208)",
            "Rgb(99,168,248)",
            "Rgb(216,222,233)",
            "Rgb(143,188,187)",
        ),
        (
            "ansi",
            DARK,
            "Blue",
            "Rgb(99,168,248)",
            "LightBlue",
            "Green",
        ),
        // the oracle's `Theme::default()`: the path span is the accent
        (
            "",
            DARK,
            "Reset",
            "Rgb(99,168,248)",
            "LightBlue",
            "Rgb(99,168,248)",
        ),
    ];
    for (name, (fg, bg), heading, link, marker, path) in rows {
        let s = Screen::build(Owner::ChatLive)
            .markdown(24, 40)
            .terminal_colors(fg, bg)
            .syntax_theme(name)
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
        if heading != "Reset" {
            s.assert_text_style("Heading", &format!("fg={heading}"));
        }
        s.assert_text_style("Link", &format!("fg={link}"));
        s.assert_text_style("1. ", &format!("fg={marker}"));
        s.assert_text_style("src/main.rs", &format!("fg={path}"));
        s.assert_text_style("src/lib.rs", &format!("fg={path}"));
    }
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
    kit::pending(
        Owner::ChatLive,
        "G73: item 3, the oracle wraps a table cell inside its column and shows each link as `label (https://example.com)`; rtwk's MarkdownView keeps one line per table row and cuts the cell with `…` (`plain (https://example.com) cod…` at 32 columns); the prose half of the test holds",
    );
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
    // The oracle's line is unwrapped: the label and its path are one 85-cell line.
    let s = Screen::build(Owner::ChatLive).markdown(100, 24).script(include_str!("fixtures/markdown_streaming_3/file_link_preserves_label_and_decodes_fallback_destination.jsonl")).cwd("/Users/example/code/codex").open();
    s.assert_contains(
        "/Users/example/code/codex/codex-rs/tui/src/My%20File.rs (codex-rs/tui/src/My File.rs)",
    );
}

// ORACLE markdown_render_tests.rs:982 file_link_keeps_descriptive_label_and_target kind=snap owner=chat-live snap=codex_tui__markdown_render__markdown_render_tests__file_link_keeps_descriptive_label_and_target
#[test]
fn file_link_keeps_descriptive_label_and_target() {
    // The oracle's line is unwrapped (130 cells).
    let s = Screen::build(Owner::ChatLive)
        .markdown(140, 40)
        .script(include_str!(
            "fixtures/markdown_streaming_3/file_link_keeps_descriptive_label_and_target.jsonl"
        ))
        .cwd("/home/dev-user/code/openai")
        .open();
    let rows = snap_lines("file_link_keeps_descriptive_label_and_target");
    let frags: Vec<&str> = rows.iter().map(String::as_str).collect();
    s.assert_in_order(&frags);
}

// ORACLE markdown_render_tests.rs:1017 file_link_preserves_tilde_and_absolute_destinations kind=snap owner=chat-live snap=codex_tui__markdown_render__markdown_render_tests__file_link_preserves_tilde_and_absolute_destinations
#[test]
fn file_link_preserves_tilde_and_absolute_destinations() {
    let s = Screen::build(Owner::ChatLive).markdown(120, 40).script(include_str!("fixtures/markdown_streaming_3/file_link_preserves_tilde_and_absolute_destinations.jsonl")).cwd("/home/alice/project").open();
    let rows = snap_lines("file_link_preserves_tilde_and_absolute_destinations");
    let frags: Vec<&str> = rows.iter().map(String::as_str).collect();
    s.assert_in_order(&frags);
}

// ORACLE markdown_render_tests.rs:1031 file_link_preserves_path_labels_and_formats_destinations kind=snap owner=chat-live snap=codex_tui__markdown_render__markdown_render_tests__file_link_preserves_path_labels_and_formats_destinations
#[test]
fn file_link_preserves_path_labels_and_formats_destinations() {
    let s = Screen::build(Owner::ChatLive).markdown(120, 40).script(include_str!("fixtures/markdown_streaming_3/file_link_preserves_path_labels_and_formats_destinations.jsonl")).cwd("/repo").open();
    let rows = snap_lines("file_link_preserves_path_labels_and_formats_destinations");
    // The oracle keeps the platform-dependent UNC display out of its snapshot with
    // `.replace("My%20File.rs", "My File.rs")` on the rendered text; the screen text goes
    // through the same replace before the rows are matched in order.
    let text = s.text().replace("My%20File.rs", "My File.rs");
    let mut from = 0;
    for row in &rows {
        match text[from..].find(row.as_str()) {
            Some(at) => from += at + row.len(),
            None => panic!("`{row}` is not on the screen after the rows before it:\n{text}"),
        }
    }
}

// ORACLE markdown_render_tests.rs:1065 file_link_decodes_percent_encoded_bare_path_destination kind=ui owner=chat-live
#[test]
fn file_link_decodes_percent_encoded_bare_path_destination() {
    let s = Screen::build(Owner::ChatLive).markdown(80, 24).script(include_str!("fixtures/markdown_streaming_3/file_link_decodes_percent_encoded_bare_path_destination.jsonl")).cwd("/Users/example/code/codex").open();
    s.assert_contains("open Example Folder/Résumé/report.md (Example Folder/Résumé/report.md)");
}

// ORACLE markdown_render_tests.rs:1083 file_link_preserves_labels_with_invalid_percent_encoding kind=snap owner=chat-live snap=codex_tui__markdown_render__markdown_render_tests__file_link_preserves_labels_with_invalid_percent_encoding
#[test]
fn file_link_preserves_labels_with_invalid_percent_encoding() {
    let s = Screen::build(Owner::ChatLive).markdown(120, 40).script(include_str!("fixtures/markdown_streaming_3/file_link_preserves_labels_with_invalid_percent_encoding.jsonl")).cwd("/repo").open();
    let rows = snap_lines("file_link_preserves_labels_with_invalid_percent_encoding");
    let frags: Vec<&str> = rows.iter().map(String::as_str).collect();
    s.assert_in_order(&frags);
}

// ORACLE markdown_render_tests.rs:1092 file_link_preserves_directory_labels_and_destinations kind=snap owner=chat-live snap=codex_tui__markdown_render__markdown_render_tests__file_link_preserves_directory_labels_and_destinations
#[test]
fn file_link_preserves_directory_labels_and_destinations() {
    let s = Screen::build(Owner::ChatLive).markdown(120, 40).script(include_str!("fixtures/markdown_streaming_3/file_link_preserves_directory_labels_and_destinations.jsonl")).cwd("/repo").open();
    let rows = snap_lines("file_link_preserves_directory_labels_and_destinations");
    let frags: Vec<&str> = rows.iter().map(String::as_str).collect();
    s.assert_in_order(&frags);
}

// ORACLE markdown_render_tests.rs:1101 file_link_keeps_unrelated_relative_label_with_matching_suffix kind=snap owner=chat-live snap=codex_tui__markdown_render__markdown_render_tests__file_link_keeps_unrelated_relative_label_with_matching_suffix
#[test]
fn file_link_keeps_unrelated_relative_label_with_matching_suffix() {
    // The oracle's `src/lib.rs` is a code span with a theme style: not asserted.
    let s = Screen::build(Owner::ChatLive).markdown(80, 24).script(include_str!("fixtures/markdown_streaming_3/file_link_keeps_unrelated_relative_label_with_matching_suffix.jsonl")).cwd("/repo").open();
    s.assert_contains("other/src/lib.rs (src/lib.rs)");
}

// ORACLE markdown_render_tests.rs:1115 file_link_appends_line_number_when_label_lacks_it kind=ui owner=chat-live
#[test]
fn file_link_appends_line_number_when_label_lacks_it() {
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_3/file_link_appends_line_number_when_label_lacks_it.jsonl"
        ))
        .cwd("/Users/example/code/codex")
        .open();
    s.assert_contains("markdown_render.rs (codex-rs/tui/src/markdown_render.rs:74)");
}

// ORACLE markdown_render_tests.rs:1133 file_link_keeps_absolute_paths_outside_cwd kind=ui owner=chat-live
#[test]
fn file_link_keeps_absolute_paths_outside_cwd() {
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_3/file_link_keeps_absolute_paths_outside_cwd.jsonl"
        ))
        .cwd("/Users/example/code/codex/codex-rs/tui")
        .open();
    s.assert_contains("README.md:74 (/Users/example/code/codex/README.md:74)");
}

// ORACLE markdown_render_tests.rs:1151 file_links_preserve_foreign_windows_paths_and_anchors_snapshot kind=snap owner=chat-live
#[test]
fn file_links_preserve_foreign_windows_paths_and_anchors_snapshot() {
    kit::pending(
        Owner::ChatLive,
        "kit: a thread's cwd is an AbsolutePathBuf of the host (`Open::cwd` goes through `thread/started`), so a session cannot open in `C:/repo` or `//server/share` on this platform; the display itself is held by core/tests/citations.rs `a_windows_or_unc_cwd_strips_its_own_file_urls_and_the_anchor_reads_as_a_location`",
    );
    // The oracle's expected rows are an inline snapshot of the test, not a `.snap` file; it
    // renders each destination under its own cwd, so each is a screen of its own here.
    for (destination, cwd) in [
        ("file:///C:/repo/Quarterly%20Report.xlsx", "C:/repo"),
        (
            "file://server/share/Quarterly%20Report.xlsx",
            "//server/share",
        ),
    ] {
        let s = Screen::build(Owner::ChatLive)
            .markdown(80, 24)
            .agent_says(&format!("[report]({destination}#L4C2)"))
            .cwd(cwd)
            .open();
        s.assert_has_row("report (Quarterly Report.xlsx:4:2)");
    }
}

// ORACLE markdown_render_tests.rs:1172 file_link_appends_hash_anchor_when_label_lacks_it kind=ui owner=chat-live
#[test]
fn file_link_appends_hash_anchor_when_label_lacks_it() {
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_3/file_link_appends_hash_anchor_when_label_lacks_it.jsonl"
        ))
        .cwd("/Users/example/code/codex")
        .open();
    s.assert_contains("markdown_render.rs (codex-rs/tui/src/markdown_render.rs:74:3)");
}

// ORACLE markdown_render_tests.rs:1190 file_link_uses_target_path_for_hash_anchor kind=ui owner=chat-live
#[test]
fn file_link_uses_target_path_for_hash_anchor() {
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_3/file_link_uses_target_path_for_hash_anchor.jsonl"
        ))
        .cwd("/Users/example/code/codex")
        .open();
    s.assert_contains("markdown_render.rs#L74C3 (codex-rs/tui/src/markdown_render.rs:74:3)");
}

// ORACLE markdown_render_tests.rs:1208 file_link_appends_range_when_label_lacks_it kind=ui owner=chat-live
#[test]
fn file_link_appends_range_when_label_lacks_it() {
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_3/file_link_appends_range_when_label_lacks_it.jsonl"
        ))
        .cwd("/Users/example/code/codex")
        .open();
    s.assert_contains("markdown_render.rs (codex-rs/tui/src/markdown_render.rs:74:3-76:9)");
}

// ORACLE markdown_render_tests.rs:1226 file_link_uses_target_path_for_range kind=ui owner=chat-live
#[test]
fn file_link_uses_target_path_for_range() {
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_3/file_link_uses_target_path_for_range.jsonl"
        ))
        .cwd("/Users/example/code/codex")
        .open();
    s.assert_contains(
        "markdown_render.rs:74:3-76:9 (codex-rs/tui/src/markdown_render.rs:74:3-76:9)",
    );
}

// ORACLE markdown_render_tests.rs:1244 file_link_appends_hash_range_when_label_lacks_it kind=ui owner=chat-live
#[test]
fn file_link_appends_hash_range_when_label_lacks_it() {
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_3/file_link_appends_hash_range_when_label_lacks_it.jsonl"
        ))
        .cwd("/Users/example/code/codex")
        .open();
    s.assert_contains("markdown_render.rs (codex-rs/tui/src/markdown_render.rs:74:3-76:9)");
}

// ORACLE markdown_render_tests.rs:1262 multiline_file_link_label_after_styled_prefix_does_not_panic kind=ui owner=chat-live
#[test]
fn multiline_file_link_label_after_styled_prefix_does_not_panic() {
    let s = Screen::build(Owner::ChatLive).markdown(80, 24).script(include_str!("fixtures/markdown_streaming_3/multiline_file_link_label_after_styled_prefix_does_not_panic.jsonl")).cwd("/Users/example/code/codex").open();
    s.assert_contains("bold plain foo bar (codex-rs/tui/src/markdown_render.rs:74:3)");
    s.assert_text_style("bold", "bold");
}

// ORACLE markdown_render_tests.rs:1285 file_link_uses_target_path_for_hash_range kind=ui owner=chat-live
#[test]
fn file_link_uses_target_path_for_hash_range() {
    let s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .script(include_str!(
            "fixtures/markdown_streaming_3/file_link_uses_target_path_for_hash_range.jsonl"
        ))
        .cwd("/Users/example/code/codex")
        .open();
    s.assert_contains(
        "markdown_render.rs#L74C3-L76C9 (codex-rs/tui/src/markdown_render.rs:74:3-76:9)",
    );
}

// ORACLE markdown_render_tests.rs:1303 url_link_shows_destination kind=ui owner=chat-live
#[test]
fn url_link_shows_destination() {
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
        "G73: item 3, the oracle's table is a ` File` header over a `━` rule with `───` between rows; rtwk's MarkdownView draws one line per row with its `│` grid and no row rules (the paragraphs above it match)",
    );
    let s = Screen::build(Owner::ChatLive)
        .markdown(120, 40)
        .script(include_str!(
            "fixtures/markdown_streaming_3/markdown_render_file_link_snapshot.jsonl"
        ))
        .cwd("/tmp/codex-link-repro")
        .open();
    let rows = snap_lines("markdown_render_file_link_snapshot");
    let frags: Vec<&str> = rows.iter().map(String::as_str).collect();
    s.assert_in_order(&frags);
}

// ORACLE markdown_render_tests.rs:1329 unordered_list_local_file_link_stays_inline_with_following_text kind=ui owner=chat-live
#[test]
fn unordered_list_local_file_link_stays_inline_with_following_text() {
    let s = Screen::build(Owner::ChatLive).markdown(72, 24).script(include_str!("fixtures/markdown_streaming_3/unordered_list_local_file_link_stays_inline_with_following_text.jsonl")).cwd("/Users/example/code/codex").open();
    s.assert_in_order(&[
        "• binary (codex-rs/README.md:93): core is the agent/business logic, tui",
        "is the terminal UI, exec is the headless automation surface, and cli",
        "is the top-level multitool binary.",
    ]);
}

// ORACLE markdown_render_tests.rs:1356 unordered_list_local_file_link_soft_break_before_colon_stays_inline kind=ui owner=chat-live
#[test]
fn unordered_list_local_file_link_soft_break_before_colon_stays_inline() {
    let s = Screen::build(Owner::ChatLive).markdown(72, 24).script(include_str!("fixtures/markdown_streaming_3/unordered_list_local_file_link_soft_break_before_colon_stays_inline.jsonl")).cwd("/Users/example/code/codex").open();
    s.assert_contains("• binary (codex-rs/README.md:93): core is the agent/business logic.");
}

// ORACLE markdown_render_tests.rs:1379 consecutive_unordered_list_local_file_links_do_not_detach_paths kind=ui owner=chat-live
#[test]
fn consecutive_unordered_list_local_file_links_do_not_detach_paths() {
    let s = Screen::build(Owner::ChatLive).markdown(72, 24).script(include_str!("fixtures/markdown_streaming_3/consecutive_unordered_list_local_file_links_do_not_detach_paths.jsonl")).cwd("/Users/example/code/codex").open();
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
