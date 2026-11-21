//! Shard `markdown_streaming_4` (screens S11, owner chat-live): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//!
//! Every body is written in full under `pending`: the live chat draws an agent
//! message as its plain text, so no markdown rendering is on the screen yet.
//! Blank rows of the oracle's lines are not checked (no kit call asserts a blank
//! row between two rows); whole-screen absence checks are left out where the
//! chat's own chrome would be seen (missed kit calls, reported).

use crate::kit::{self, Owner, Screen};

/// The live chat draws an agent message as its plain text: the markdown renderer (rtwk-rich MarkdownView over the streamed answer) is not in the transcript yet.
const MD: &str = "the live chat draws an agent message as its plain text: the markdown renderer (rtwk-rich MarkdownView over the streamed answer) is not in the transcript yet";
/// As `MD`, and the oracle renders file links relative to a session cwd: the kit's live chat has no cwd seed.
const MD_CWD: &str = "the live chat draws an agent message as its plain text (no markdown renderer yet) and the kit has no cwd seed: the oracle renders file links relative to /Users/example/code/codex";

/// The streamed answer: the fixture's four steps (seed, turn started, one delta, turn completed).
const ANSWER: &str = include_str!("fixtures/markdown_streaming_4/answer.jsonl");

/// `text` as the body of a JSON string.
fn json_string(text: &str) -> String {
    let mut out = String::new();
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
    out
}

/// One delta step of the answer's message carrying `chunk`.
fn delta(chunk: &str) -> String {
    let template = ANSWER.lines().nth(2).unwrap_or_default();
    template.replace("{MARKDOWN}", &json_string(chunk))
}

/// The steps of the answer streamed as `chunks`; `complete` ends the turn.
fn stream(chunks: &[&str], complete: bool) -> String {
    let lines: Vec<&str> = ANSWER.lines().collect();
    let mut out = String::new();
    for step in lines.iter().take(2) {
        out.push_str(step);
        out.push('\n');
    }
    for chunk in chunks {
        out.push_str(&delta(chunk));
        out.push('\n');
    }
    if let Some(done) = lines.get(3).filter(|_| complete) {
        out.push_str(done);
        out.push('\n');
    }
    out
}

/// The live chat at `width` x `height` after the answer `chunks` has streamed in.
/// The screen is read once the app has drawn the answer (`settle`): a read right
/// after the feed can still show the chat before the first delta.
fn streamed(width: u16, height: u16, chunks: &[&str], complete: bool) -> Screen {
    let mut s = Screen::build(Owner::ChatLive)
        .size(width, height)
        .live()
        .open();
    s.feed(&stream(chunks, complete));
    settle(&mut s);
    s
}

/// Lets the app draw what the feed sent (the backend runs on its own thread).
fn settle(s: &mut Screen) {
    s.wait(1000);
}

/// The live chat after one whole agent answer `markdown`.
fn chat(width: u16, height: u16, markdown: &str) -> Screen {
    streamed(width, height, &[markdown], true)
}

/// Each prefix of `chunks`, streamed with the turn still open, draws what the same text draws as one message.
fn assert_streams_like_whole(width: u16, height: u16, chunks: &[&str]) {
    for n in 1..=chunks.len() {
        let whole = chunks[..n].concat();
        let want = streamed(width, height, &[whole.as_str()], false).shot();
        streamed(width, height, &chunks[..n], false).assert_same_rows(&want);
    }
}

/// The oracle's rows, in order: each non-blank one is a row of the screen, read exactly (trailing blanks aside).
fn rows_in_order(s: &Screen, rows: &[&str]) {
    let rows: Vec<&str> = rows.iter().copied().filter(|r| !r.is_empty()).collect();
    for row in &rows {
        s.assert_has_row(row);
    }
    s.assert_in_order(&rows);
}

// ---- oracle: markdown_render_tests.rs (38 screen-level tests here; 0 state tests left to chat-live)
// ORACLE markdown_render_tests.rs:1439 code_block_unknown_lang_plain kind=ui owner=chat-live
#[test]
fn code_block_unknown_lang_plain() {
    kit::pending(Owner::ChatLive, MD);
    let s = chat(80, 24, "```xyzlang\nhello world\n```\n");
    rows_in_order(&s, &["hello world"]);
    // The oracle's span has no fg: no syntax colour, the terminal default.
    s.assert_text_style("hello world", "fg=Reset");
    // The oracle's text has no fence line: the source fences are not drawn.
    s.assert_absent("```");
}

// ORACLE markdown_render_tests.rs:1471 code_block_no_lang_plain kind=ui owner=chat-live
#[test]
fn code_block_no_lang_plain() {
    kit::pending(Owner::ChatLive, MD);
    let s = chat(80, 24, "```\nno lang specified\n```\n");
    rows_in_order(&s, &["no lang specified"]);
    // The oracle's text has no fence line: the source fences are not drawn.
    s.assert_absent("```");
}

// ORACLE markdown_render_tests.rs:1492 code_block_multiple_lines_root kind=ui owner=chat-live
#[test]
fn code_block_multiple_lines_root() {
    kit::pending(Owner::ChatLive, MD);
    let s = chat(80, 24, "```\nfirst\nsecond\n```\n");
    rows_in_order(&s, &["first", "second"]);
    // The oracle's text has no fence line: the source fences are not drawn.
    s.assert_absent("```");
}

// ORACLE markdown_render_tests.rs:1503 code_block_indented kind=ui owner=chat-live
#[test]
fn code_block_indented() {
    kit::pending(Owner::ChatLive, MD);
    let s = chat(
        80,
        24,
        "    function greet() {\n      console.log(\"Hi\");\n    }\n",
    );
    rows_in_order(
        &s,
        &[
            "    function greet() {",
            "      console.log(\"Hi\");",
            "    }",
        ],
    );
}

// ORACLE markdown_render_tests.rs:1515 horizontal_rule_renders_em_dashes kind=ui owner=chat-live
#[test]
fn horizontal_rule_renders_em_dashes() {
    kit::pending(Owner::ChatLive, MD);
    let s = chat(80, 24, "Before\n\n---\n\nAfter\n");
    rows_in_order(&s, &["Before", "", "———", "", "After"]);
}

// ORACLE markdown_render_tests.rs:1532 code_block_with_inner_triple_backticks_outer_four kind=ui owner=chat-live
#[test]
fn code_block_with_inner_triple_backticks_outer_four() {
    kit::pending(Owner::ChatLive, MD);
    let md = r#"````text
Here is a code block that shows another fenced block:

```md
# Inside fence
- bullet
- `inline code`
```
````
"#;
    let s = chat(80, 24, md);
    rows_in_order(
        &s,
        &[
            "Here is a code block that shows another fenced block:",
            "",
            "```md",
            "# Inside fence",
            "- bullet",
            "- `inline code`",
            "```",
        ],
    );
    // The outer four-backtick fence is not drawn: the oracle's text has no such line.
    s.assert_absent("````");
}

// ORACLE markdown_render_tests.rs:1578 code_block_inside_unordered_list_item_is_indented kind=ui owner=chat-live
#[test]
fn code_block_inside_unordered_list_item_is_indented() {
    kit::pending(Owner::ChatLive, MD);
    let s = chat(80, 24, "- Item\n\n  ```\n  code line\n  ```\n");
    rows_in_order(&s, &["• Item", "", "  code line"]);
}

// ORACLE markdown_render_tests.rs:1595 code_block_multiple_lines_inside_unordered_list kind=ui owner=chat-live
#[test]
fn code_block_multiple_lines_inside_unordered_list() {
    kit::pending(Owner::ChatLive, MD);
    let s = chat(80, 24, "- Item\n\n  ```\n  first\n  second\n  ```\n");
    rows_in_order(&s, &["• Item", "", "  first", "  second"]);
}

// ORACLE markdown_render_tests.rs:1612 list_item_after_code_block_keeps_blank_separator kind=snap owner=chat-live snap=codex_tui__markdown_render__markdown_render_tests__list_item_after_code_block_keeps_blank_separator
#[test]
fn list_item_after_code_block_keeps_blank_separator() {
    kit::pending(Owner::ChatLive, MD);
    let s = chat(
        80,
        24,
        "1. First:\n\n   ```rust\n   fn first() {}\n   ```\n\n2. Second:\n",
    );
    rows_in_order(&s, &["1. First:", "", "   fn first() {}", "", "2. Second:"]);
    s.assert_snap_region("list_item_after_code_block_keeps_blank_separator", 0, 0);
}

// ORACLE markdown_render_tests.rs:1627 outer_list_item_after_nested_code_block_keeps_blank_separator kind=ui owner=chat-live
#[test]
fn outer_list_item_after_nested_code_block_keeps_blank_separator() {
    kit::pending(Owner::ChatLive, MD);
    let s = chat(
        80,
        24,
        "1. First:\n   - Nested:\n\n     ```rust\n     fn first() {}\n     ```\n\n2. Second:\n",
    );
    rows_in_order(
        &s,
        &[
            "1. First:",
            "    • Nested:",
            "",
            "      fn first() {}",
            "",
            "2. Second:",
        ],
    );
}

// ORACLE markdown_render_tests.rs:1646 list_item_after_simple_item_stays_compact kind=ui owner=chat-live
#[test]
fn list_item_after_simple_item_stays_compact() {
    kit::pending(Owner::ChatLive, MD);
    let s = chat(80, 24, "1. First\n\n2. Second\n");
    // The oracle's lines are these two rows; the compact spacing (no blank row between) is not checked.
    rows_in_order(&s, &["1. First", "2. Second"]);
}

// ORACLE markdown_render_tests.rs:1653 multiline_finding_items_are_separated_snapshot kind=snap owner=chat-live snap=codex_tui__markdown_render__markdown_render_tests__multiline_finding_items_are_separated_snapshot
#[test]
fn multiline_finding_items_are_separated_snapshot() {
    kit::pending(Owner::ChatLive, MD);
    let md = r#"**Findings**

1. **Correctness issue: server tool-search completions are always rejected.**

   In `next_prompt_suggestion.rs`, the output is ignored, suppressing suggestions after completed searches.

   Minimal correction: count matching outputs and suppress only missing ones.

2. **High-confidence simplification: remove the unused error channel.**

   The implementation resolves failures to `None`, so its contract can be narrower.

3. **High-confidence churn reduction: consolidate table-driven filter tests.**
"#;
    let s = chat(120, 24, md);
    s.assert_snap_region("multiline_finding_items_are_separated_snapshot", 0, 0);
}

// ORACLE markdown_render_tests.rs:1673 wrapped_list_item_is_separated_from_next_sibling kind=ui owner=chat-live
#[test]
fn wrapped_list_item_is_separated_from_next_sibling() {
    kit::pending(Owner::ChatLive, MD);
    let s = chat(
        24,
        24,
        "1. This item wraps onto another visible rendered line\n2. Next item\n",
    );
    rows_in_order(
        &s,
        &[
            "1. This item wraps onto",
            "   another visible",
            "   rendered line",
            "",
            "2. Next item",
        ],
    );
}

// ORACLE markdown_render_tests.rs:1689 mixed_url_markdown_wraps_prose_without_splitting_words_snapshot kind=snap owner=chat-live snap=codex_tui__markdown_render__markdown_render_tests__mixed_url_markdown_wraps_prose_without_splitting_words_snapshot
#[test]
fn mixed_url_markdown_wraps_prose_without_splitting_words_snapshot() {
    kit::pending(Owner::ChatLive, MD);
    let md = "This paragraph keeps **strikethrough** intact near a [link](https://example.com/path) while enough surrounding prose forces wrapping.";
    let s = chat(48, 24, md);
    s.assert_snap_region(
        "mixed_url_markdown_wraps_prose_without_splitting_words_snapshot",
        0,
        0,
    );
}

// ORACLE markdown_render_tests.rs:1696 markdown_render_complex_snapshot kind=snap owner=chat-live snap=codex_tui__markdown_render__markdown_render_tests__markdown_render_complex_snapshot
#[test]
fn markdown_render_complex_snapshot() {
    kit::pending(Owner::ChatLive, MD);
    let md = r#"# H1: Markdown Streaming Test
Intro paragraph with bold **text**, italic *text*, and inline code `x=1`.
Combined bold-italic ***both*** and escaped asterisks \*literal\*.
Auto-link: <https://example.com> and reference link [ref][r1].
Link with title: [hover me](https://example.com "Example") and mailto <mailto:test@example.com>.
Image: ![alt text](https://example.com/img.png "Title")
> Blockquote level 1
>> Blockquote level 2 with `inline code`
- Unordered list item 1
  - Nested bullet with italics _inner_
- Unordered list item 2 with ~~strikethrough~~
1. Ordered item one
2. Ordered item two with sublist:
   1) Alt-numbered subitem
- [ ] Task: unchecked
- [x] Task: checked with link [home](https://example.org)
---
Table below (alignment test):
| Left | Center | Right |
|:-----|:------:|------:|
| a    |   b    |     c |
Inline HTML: <sup>sup</sup> and <sub>sub</sub>.
HTML block:
<div style="border:1px solid #ccc;padding:2px">inline block</div>
Escapes: \_underscores\_, backslash \\, ticks ``code with `backtick` inside``.
Emoji shortcodes: :sparkles: :tada: (if supported).
Hard break test (line ends with two spaces)
Next line should be close to previous.
Footnote reference here[^1] and another[^longnote].
Horizontal rule with asterisks:
***
Fenced code block (JSON):
```json
{ "a": 1, "b": [true, false] }
```
Fenced code with tildes and triple backticks inside:
~~~markdown
To close ``` you need tildes.
~~~
Indented code block:
    for i in range(3): print(i)
Definition-like list:
Term
: Definition with `code`.
Character entities: &amp; &lt; &gt; &quot; &#39;
[^1]: This is the first footnote.
[^longnote]: A longer footnote with a link to [Rust](https://www.rust-lang.org/).
Escaped pipe in text: a \| b \| c.
URL with parentheses: [link](https://example.com/path_(with)_parens).
[r1]: https://example.com/ref "Reference link title"
"#;
    let s = chat(120, 70, md);
    s.assert_snap_region("markdown_render_complex_snapshot", 0, 0);
}

// ORACLE markdown_render_tests.rs:1767 ordered_item_with_code_block_and_nested_bullet kind=ui owner=chat-live
#[test]
fn ordered_item_with_code_block_and_nested_bullet() {
    kit::pending(Owner::ChatLive, MD);
    let md = "1. **item 1**\n\n2. **item 2**\n   ```\n   code\n   ```\n   - `PROCESS_START` (a `OnceLock<Instant>`) keeps the start time for the entire process.\n";
    let s = chat(120, 24, md);
    rows_in_order(
        &s,
        &[
            "1. item 1",
            "2. item 2",
            "",
            "   code",
            "    • PROCESS_START (a OnceLock<Instant>) keeps the start time for the entire process.",
        ],
    );
}

// ORACLE markdown_render_tests.rs:1793 nested_five_levels_mixed_lists kind=ui owner=chat-live
#[test]
fn nested_five_levels_mixed_lists() {
    kit::pending(Owner::ChatLive, MD);
    let md = "1. First\n   - Second level\n     1. Third level (ordered)\n        - Fourth level (bullet)\n          - Fifth level to test indent consistency\n";
    let s = chat(80, 24, md);
    rows_in_order(
        &s,
        &[
            "1. First",
            "    • Second level",
            "        1. Third level (ordered)",
            "            • Fourth level (bullet)",
            "                • Fifth level to test indent consistency",
        ],
    );
    // The oracle's "1. " is light blue (the first ordered marker).
    s.assert_text_style("1.", "fg=LightBlue");
}

// ORACLE markdown_render_tests.rs:1811 html_inline_is_verbatim kind=ui owner=chat-live
#[test]
fn html_inline_is_verbatim() {
    kit::pending(Owner::ChatLive, MD);
    let s = chat(80, 24, "Hello <span>world</span>!");
    rows_in_order(&s, &["Hello <span>world</span>!"]);
}

// ORACLE markdown_render_tests.rs:1819 html_block_is_verbatim_multiline kind=ui owner=chat-live
#[test]
fn html_block_is_verbatim_multiline() {
    kit::pending(Owner::ChatLive, MD);
    let s = chat(80, 24, "<div>\n  <span>hi</span>\n</div>\n");
    rows_in_order(&s, &["<div>", "  <span>hi</span>", "</div>"]);
}

// ORACLE markdown_render_tests.rs:1831 html_in_tight_ordered_item_soft_breaks_with_space kind=ui owner=chat-live
#[test]
fn html_in_tight_ordered_item_soft_breaks_with_space() {
    kit::pending(Owner::ChatLive, MD);
    let s = chat(80, 24, "1. Foo\n   <i>Bar</i>\n");
    rows_in_order(&s, &["1. Foo", "   <i>Bar</i>"]);
    s.assert_text_style("1.", "fg=LightBlue");
}

// ORACLE markdown_render_tests.rs:1842 html_continuation_paragraph_in_unordered_item_indented kind=ui owner=chat-live
#[test]
fn html_continuation_paragraph_in_unordered_item_indented() {
    kit::pending(Owner::ChatLive, MD);
    let s = chat(80, 24, "- Item\n\n  <em>continued</em>\n");
    rows_in_order(&s, &["• Item", "", "  <em>continued</em>"]);
}

// ORACLE markdown_render_tests.rs:1855 unordered_item_continuation_paragraph_is_indented kind=ui owner=chat-live
#[test]
fn unordered_item_continuation_paragraph_is_indented() {
    kit::pending(Owner::ChatLive, MD);
    let s = chat(
        80,
        24,
        "- Intro\n\n  Continuation paragraph line 1\n  Continuation paragraph line 2\n",
    );
    rows_in_order(
        &s,
        &[
            "• Intro",
            "",
            "  Continuation paragraph line 1",
            "  Continuation paragraph line 2",
        ],
    );
}

// ORACLE markdown_render_tests.rs:1880 ordered_item_continuation_paragraph_is_indented kind=ui owner=chat-live
#[test]
fn ordered_item_continuation_paragraph_is_indented() {
    kit::pending(Owner::ChatLive, MD);
    let s = chat(80, 24, "1. Intro\n\n   More details about intro\n");
    rows_in_order(&s, &["1. Intro", "", "   More details about intro"]);
    s.assert_text_style("1.", "fg=LightBlue");
}

// ORACLE markdown_render_tests.rs:1892 nested_item_continuation_paragraph_is_indented kind=ui owner=chat-live
#[test]
fn nested_item_continuation_paragraph_is_indented() {
    kit::pending(Owner::ChatLive, MD);
    let s = chat(80, 24, "1. A\n    - B\n\n      Continuation for B\n2. C\n");
    rows_in_order(
        &s,
        &[
            "1. A",
            "    • B",
            "",
            "      Continuation for B",
            "",
            "2. C",
        ],
    );
    s.assert_text_style("1.", "fg=LightBlue");
}

// ORACLE markdown_render_tests.rs:1908 code_block_preserves_trailing_blank_lines kind=ui owner=chat-live
#[test]
fn code_block_preserves_trailing_blank_lines() {
    kit::pending(Owner::ChatLive, MD);
    let s = chat(80, 24, "```rust\nfn main() {}\n\n```\n");
    // The oracle's trailing blank row after `fn main() {}` is not checked (blank rows: missed kit call).
    rows_in_order(&s, &["fn main() {}"]);
    // The oracle's text has no fence line: the source fences are not drawn.
    s.assert_absent("```");
}

// ORACLE markdown_render_tests.rs:1942 table_renders_app_style_rows_with_themed_bold_header kind=ui owner=chat-live
#[test]
fn table_renders_app_style_rows_with_themed_bold_header() {
    kit::pending(Owner::ChatLive, MD);
    let s = chat(80, 24, "| A | B |\n|---|---|\n| 1 | 2 |\n");
    rows_in_order(&s, &[" A      B", "━━━━━  ━━━━━", " 1      2"]);
    // Header line bold, the rule's first span dim, the body row not bold.
    s.assert_text_style(" A      B", "bold");
    s.assert_text_style("━━━━━", "dim");
    s.assert_text_style(" 1      2", "!bold");
}

// ORACLE markdown_render_tests.rs:1974 table_alignment_respects_markers kind=ui owner=chat-live
#[test]
fn table_alignment_respects_markers() {
    kit::pending(Owner::ChatLive, MD);
    let s = chat(
        80,
        24,
        "| Left | Center | Right |\n|:-----|:------:|------:|\n| a | b | c |\n",
    );
    rows_in_order(
        &s,
        &[" Left    Center    Right", " a         b           c"],
    );
}

// ORACLE markdown_render_tests.rs:1988 table_separates_logical_rows_after_wrapped_content kind=ui owner=chat-live
#[test]
fn table_separates_logical_rows_after_wrapped_content() {
    kit::pending(Owner::ChatLive, MD);
    let s = chat(
        30,
        24,
        "| Key | Description |\n| --- | --- |\n| -v | Enable very verbose logging output for debugging |\n| -q | Quiet output |\n",
    );
    s.assert_contains("Enable very verbose");
    s.assert_contains("logging output");
    // The oracle's grid draws no vertical bars: the source table's pipes are not drawn.
    s.assert_absent("|");
    // The oracle counts two rule rows, the second after the wrapped row, and no rule last: not checked (missed kit call).
}

// ORACLE markdown_render_tests.rs:2027 table_wraps_file_paths_before_collapsing_narrative_columns_snapshot kind=snap owner=chat-live snap=codex_tui__markdown_render__markdown_render_tests__table_wraps_file_paths_before_collapsing_narrative_columns_snapshot
#[test]
fn table_wraps_file_paths_before_collapsing_narrative_columns_snapshot() {
    kit::pending(Owner::ChatLive, MD_CWD);
    let md = r#"| Unit | Files | Adds | Removes | What It Adds |
|---|---:|---:|---:|---|
| Suggestion engine and unit coverage | [next_prompt_suggestion.rs](/Users/example/code/codex/codex-rs/core/src/next_prompt_suggestion.rs:1), [next_prompt_suggestion_tests.rs](/Users/example/code/codex/codex-rs/core/src/next_prompt_suggestion_tests.rs:1) | 704 | 0 | Sampling workflow, stable-history checks, tool-flow suppression, fast reasoning profile, filtering rules, cancellation and timeout. |
| Model instruction fragment and contextual isolation | [next_prompt_suggestion.rs](/Users/example/code/codex/codex-rs/core/src/context/next_prompt_suggestion.rs:1), [contextual_user_message_tests.rs](/Users/example/code/codex/codex-rs/core/src/context/contextual_user_message_tests.rs:1) | 54 | 0 | Synthetic suggestion prompt and an isolation test for ordinary user text. |
"#;
    let s = chat(120, 40, md);
    s.assert_snap_region(
        "table_wraps_file_paths_before_collapsing_narrative_columns_snapshot",
        0,
        0,
    );
}

// ORACLE markdown_render_tests.rs:2043 table_renders_stacked_key_value_records_when_path_column_becomes_too_narrow_snapshot kind=snap owner=chat-live snap=codex_tui__markdown_render__markdown_render_tests__table_renders_stacked_key_value_records_when_path_column_becomes_too_narrow_snapshot
#[test]
fn table_renders_stacked_key_value_records_when_path_column_becomes_too_narrow_snapshot() {
    kit::pending(Owner::ChatLive, MD);
    let md = r#"| Session | Why useful | Detected table blocks |
| --- | --- | --- |
| [2026-05-25 current gallery](/Users/felipe.coury/.codex/sessions/2026/05/25/rollout-2026-05-25T18-13-09-019e60fc-0518-7c21-9596-980fe97225ba.jsonl) | The large gallery from this thread: emojis, links, emphasis, code, alignment, paragraphs, and a 30+ row table | 7 |
| [2026-05-14 renderer testing](/Users/felipe.coury/.codex/sessions/2026/05/14/rollout-2026-05-14T12-57-18-019e2734-e500-7011-8278-975c94d06000.jsonl) | Explicit "markdown tables for testing" session with several successive assistant samples | 16 |
| [2026-05-14 five-table test](/Users/felipe.coury/.codex/sessions/2026/05/14/rollout-2026-05-14T12-27-57-019e271a-064c-78c3-a5cd-a6f20a0c1ad5.jsonl) | Explicit request for five tables containing emojis, code, italics, and varied cell content | 10 |
"#;
    let s = chat(42, 40, md);
    s.assert_snap_region(
        "table_renders_stacked_key_value_records_when_path_column_becomes_too_narrow_snapshot",
        0,
        0,
    );
}

// ORACLE markdown_render_tests.rs:2056 table_renders_records_when_multiple_prose_columns_are_starved_snapshot kind=snap owner=chat-live snap=codex_tui__markdown_render__markdown_render_tests__table_renders_records_when_multiple_prose_columns_are_starved_snapshot
#[test]
fn table_renders_records_when_multiple_prose_columns_are_starved_snapshot() {
    kit::pending(Owner::ChatLive, MD);
    let md = r#"| Issue | Activity | Complexity | Why start |
| --- | ---: | ---: | --- |
| [#24485: newline shortcut fails in PyCharm terminal on Windows](https://github.com/openai/codex/issues/24485) | `+1` 0, substantive comments 0 | Low | New, deterministic regression range; localized composer/keymap path. |
| [#23926: Vim composer `e` stalls at word end](https://github.com/openai/codex/issues/23926) | `+1` 0, comments 0 | Low | Standing best quick win; deterministic motion bug. |
| [#23651: Zellij scrollback misses Codex transcript over SSH](https://github.com/openai/codex/issues/23651) | `+1` 3, human comments 2 | Medium | Clear regression and strong scrollback evidence. |
| [#23740: raw ANSI/control sequences in Windows Terminal](https://github.com/openai/codex/issues/23740) | `+1` 7, human comments 7 | Medium | Highest activity; established Windows rendering regression family. |
| [#24527: typing lag increases with session length](https://github.com/openai/codex/issues/24527) | `+1` 0, substantive comments 0 | Medium | New TUI-visible performance report; needs profiling before implementation. |
"#;
    let s = chat(76, 40, md);
    s.assert_snap_region(
        "table_renders_records_when_multiple_prose_columns_are_starved_snapshot",
        0,
        0,
    );
}

// ORACLE markdown_render_tests.rs:2071 table_keeps_grid_when_only_one_compact_record_fragments_snapshot kind=snap owner=chat-live snap=codex_tui__markdown_render__markdown_render_tests__table_keeps_grid_when_only_one_compact_record_fragments_snapshot
#[test]
fn table_keeps_grid_when_only_one_compact_record_fragments_snapshot() {
    kit::pending(Owner::ChatLive, MD);
    let md = r#"| Key | Date | State |
| --- | --- | --- |
| short | 2025-01-01 | Ready |
| verylongidentifier | 2025-02-02 | Ready |
| final | 2025-03-03 | Done |
"#;
    let s = chat(40, 24, md);
    s.assert_snap_region(
        "table_keeps_grid_when_only_one_compact_record_fragments_snapshot",
        0,
        0,
    );
}

// ORACLE markdown_render_tests.rs:2084 table_renders_key_value_records_when_compact_fragmentation_is_systemic_snapshot kind=snap owner=chat-live snap=codex_tui__markdown_render__markdown_render_tests__table_renders_key_value_records_when_compact_fragmentation_is_systemic_snapshot
#[test]
fn table_renders_key_value_records_when_compact_fragmentation_is_systemic_snapshot() {
    kit::pending(Owner::ChatLive, MD);
    let md = r#"| Key | Notes |
| --- | --- |
| firstlongid | A readable explanatory sentence for this row. |
| secondlongid | Another readable explanatory sentence for this row. |
| short | A final readable explanatory sentence for this row. |
"#;
    let s = chat(17, 30, md);
    s.assert_snap_region(
        "table_renders_key_value_records_when_compact_fragmentation_is_systemic_snapshot",
        0,
        0,
    );
}

// ORACLE markdown_render_tests.rs:2097 table_renders_halfwidth_sound_marks_at_constrained_width_snapshot kind=snap owner=chat-live snap=codex_tui__markdown_render__markdown_render_tests__table_renders_halfwidth_sound_marks_at_constrained_width_snapshot
#[test]
fn table_renders_halfwidth_sound_marks_at_constrained_width_snapshot() {
    kit::pending(Owner::ChatLive, MD);
    let md = r#"| Key | Notes |
| --- | --- |
| ｶﾞﾊﾟtail | First ｶﾞ row with an escaped \| pipe. |
| ﾊﾟｶﾞtail | Second ﾊﾟ row with an escaped \| pipe. |
| short | Final ｶﾞﾊﾟ row. |
"#;
    // The oracle's snapshot joins two renders: the grid at 23 cells, then the records at 17 cells.
    // Its labels are the test's own text; the rows below are the oracle's output, read in that order.
    let mut s = chat(23, 40, md);
    rows_in_order(
        &s,
        &[
            " Key         Notes",
            "━━━━━━━━━━  ━━━━━━━━━━━",
            " ｶﾞﾊﾟtail    First ｶﾞ",
            "             row with",
            "             an",
            "             escaped |",
            "             pipe.",
            "──────────  ───────────",
            " ﾊﾟｶﾞtail    Second ﾊﾟ",
            "             row with",
            "             an",
            "             escaped |",
            "             pipe.",
            "──────────  ───────────",
            " short       Final ｶﾞ",
            "             ﾊﾟ row.",
        ],
    );
    s.resize(17, 40);
    rows_in_order(
        &s,
        &[
            " Key",
            "  ｶﾞﾊﾟtail",
            " Notes",
            "  First ｶﾞ row",
            "  with an escaped",
            "  | pipe.",
            "─────────────────",
            " Key",
            "  ﾊﾟｶﾞtail",
            " Notes",
            "  Second ﾊﾟ row",
            "  with an escaped",
            "  | pipe.",
            "─────────────────",
            " Key",
            "  short",
            " Notes",
            "  Final ｶﾞﾊﾟ row.",
        ],
    );
}

// ORACLE markdown_render_tests.rs:2115 table_inside_blockquote_has_quote_prefix kind=ui owner=chat-live
#[test]
fn table_inside_blockquote_has_quote_prefix() {
    kit::pending(Owner::ChatLive, MD);
    let s = chat(80, 24, "> | A | B |\n> |---|---|\n> | 1 | 2 |\n");
    s.assert_contains("━━━━━  ━━━━━");
    // The oracle's every row starts with "> ": no kit call checks the start of every row (missed).
}

// ORACLE markdown_render_tests.rs:2129 escaped_pipes_render_in_table_cells kind=ui owner=chat-live
#[test]
fn escaped_pipes_render_in_table_cells() {
    let s = chat(80, 24, "| Col |\n| --- |\n| a \\| b |\n");
    s.assert_contains("a | b");
}

// ORACLE markdown_render_tests.rs:2142 table_falls_back_to_key_value_records_if_grid_cannot_fit kind=ui owner=chat-live
#[test]
fn table_falls_back_to_key_value_records_if_grid_cannot_fit() {
    let md = "| c1 | c2 | c3 | c4 | c5 | c6 | c7 | c8 | c9 | c10 |\n|---|---|---|---|---|---|---|---|---|---|\n| 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 |\n";
    let s = chat(20, 40, md);
    // The first row holds c1; the last column's row holds c10 and 10.
    s.assert_contains("c1");
    s.assert_contains("c10");
    s.assert_contains("10");
    // The oracle's rows hold no rule glyph (no grid): the source's rule row is not drawn.
    s.assert_absent("━");
    s.assert_absent("─");
    // The oracle's records draw no bars (the halfwidth snapshot's records have none): the source's pipes are not drawn.
    s.assert_absent("|");
    // Not ported: the oracle's "no row starts with '|'" (a row-start check over every row; no kit call).
}

// ORACLE markdown_render_tests.rs:2165 table_key_value_fallback_preserves_rich_values_and_themed_labels kind=ui owner=chat-live
#[test]
fn table_key_value_fallback_preserves_rich_values_and_themed_labels() {
    kit::pending(Owner::ChatLive, MD);
    let md = "| Key | Content | Extra | More |\n|---|---|---|---|\n| item | [link](https://example.com) | **bold** | `code` |\n";
    let s = chat(16, 40, md);
    s.assert_contains("Key");
    s.assert_contains("item");
    s.assert_contains("link");
    s.assert_contains("bold");
    s.assert_contains("code");
    // The label "Key" is bold; the oracle's "fg is some" (themed) and "some span underlined" are not checked: no kit call for either.
    s.assert_text_style("Key", "bold");
}

// ---- oracle: streaming/code_fence_render_tests.rs (5 screen-level tests here; 0 state tests left to chat-live)
// ORACLE streaming/code_fence_render_tests.rs:13 growing_code_fences_preserve_styles_unicode_links_and_blank_lines kind=snap owner=chat-live
#[test]
fn growing_code_fences_preserve_styles_unicode_links_and_blank_lines() {
    kit::pending(Owner::ChatLive, MD);
    let chunks = [
        "Before [link](https://example.com).\n\n```rust,no_run\n",
        "fn main() {\n",
        "    /* a multiline\n",
        "       comment */\n\n",
        "    println!(\"界 e\u{301} https://example.com\");\n",
        "}\n",
        "```\n\n~~~~python\n",
        "text = \"\"\"a multiline\n",
        "string\"\"\"\n",
        "~~~~\n",
    ];
    // Every prefix of the stream, open, draws what the same text draws in one message.
    assert_streams_like_whole(18, 24, &chunks);
    // The oracle's inline snapshot of the finished answer at width 18 (its blank rows aside).
    let s = streamed(18, 24, &chunks, true);
    rows_in_order(
        &s,
        &[
            "Before link",
            "(https://example.com).",
            "",
            "fn main() {",
            "    /* a multiline",
            "       comment */",
            "",
            "    println!(\"界 e\u{301} https://example.com\");",
            "}",
            "",
            "text = \"\"\"a multiline",
            "string\"\"\"",
        ],
    );
    // The oracle's check that the fence state is closed at the end is internal (not drawn).
}

// ORACLE streaming/code_fence_render_tests.rs:49 long_open_fence_retains_its_rendered_prefix kind=ui owner=chat-live
#[test]
fn long_open_fence_retains_its_rendered_prefix() {
    kit::pending(Owner::ChatLive, MD);
    let lines: Vec<String> = (0..512)
        .map(|index| format!("    let value_{index} = {index};\n"))
        .collect();
    let mut chunks: Vec<&str> = vec!["```rust\nfn main() {\n"];
    chunks.extend(lines.iter().map(String::as_str));
    // The oracle compares after every append; here the open stream is compared once, whole (512 screens would be too heavy).
    let open = streamed(80, 24, &chunks, false);
    let whole = chunks.concat();
    open.assert_same_rows(&streamed(80, 24, &[whole.as_str()], false).shot());
    // The oracle's pointer-identity check on the first span is not visible: not ported.
    let mut closed = chunks.clone();
    closed.push("}\n```\n");
    let done = streamed(80, 24, &closed, true);
    rows_in_order(&done, &["let value_511 = 511;", "}"]);
}

// ORACLE streaming/code_fence_render_tests.rs:77 ambiguous_and_normalized_fences_use_canonical_semantics kind=ui owner=chat-live
#[test]
fn ambiguous_and_normalized_fences_use_canonical_semantics() {
    kit::pending(Owner::ChatLive, MD);
    let streams: &[&[&str]] = &[
        &["```rust\n", "x\n", "``", "`\n"],
        &["```rust\n", "x\n", "```\t\n", "after\n"],
        &["```rust\n", "x\n", "  ````  \n", "after\n"],
        &["~~~rust\n", "x\n", "~~~\u{a0}\n", "after\n"],
        &["```rust\r\n", "let value = 'é';\r\n", "```\r\n"],
        &["```rust\n", "let value = '\0';\n", "```\n"],
        &["```r&#117;st,no_run\n", "fn main() {}\n", "```\n"],
        &[" ```rust\n", "  fn main() {}\n", " ```\n"],
        &["```unknown-language\n", "first\n", "second\n", "```\n"],
        &[
            "```markdown\n| A | B |\n",
            "| --- | --- |\n",
            "| 1 | 2 |\n```\n\n```rust\nfn first() {}\n",
            "fn second() {}\n",
            "```\n",
        ],
    ];
    for chunks in streams {
        assert_streams_like_whole(40, 24, chunks);
    }
    for indent in 0..=3 {
        let closing = format!("{}~~~~~\t\n", " ".repeat(indent));
        assert_streams_like_whole(40, 24, &["~~~~rust\n", "x\n", closing.as_str(), "after\n"]);
    }
}

// ORACLE streaming/code_fence_render_tests.rs:109 crossing_a_highlight_limit_removes_previously_highlighted_colors kind=ui owner=chat-live
#[test]
fn crossing_a_highlight_limit_removes_previously_highlighted_colors() {
    kit::pending(Owner::ChatLive, MD);
    // One line past the oracle's MAX_HIGHLIGHT_LINE_BYTES (render/highlight.rs:658, 4 * 1024).
    let long = format!("{}\n", "x".repeat(4 * 1024 + 1));
    assert_streams_like_whole(
        80,
        24,
        &[
            "```rust\nlet first = 1;\n",
            long.as_str(),
            "let second = 2;\n",
            "```\n",
        ],
    );
}

// ORACLE streaming/code_fence_render_tests.rs:124 recomputing_after_resize_or_mode_change_drops_fence_state kind=ui owner=chat-live
#[test]
fn recomputing_after_resize_or_mode_change_drops_fence_state() {
    kit::pending(Owner::ChatLive, MD);
    let first = "```rust\nfn first() {}\n";
    let mut s = streamed(80, 24, &[first], false);
    // The resize recomputes the answer at width 20: the same rows as the message at 20.
    s.resize(20, 24);
    settle(&mut s);
    s.assert_same_rows(&streamed(20, 24, &[first], false).shot());
    // The next delta extends the answer as the whole message at 20 does.
    s.feed(&delta("fn second() {}\n"));
    settle(&mut s);
    let whole = format!("{first}fn second() {{}}\n");
    s.assert_same_rows(&streamed(20, 24, &[whole.as_str()], false).shot());
    // The oracle's open_code_fence and render.clear() checks are internal state, not drawn.
}
