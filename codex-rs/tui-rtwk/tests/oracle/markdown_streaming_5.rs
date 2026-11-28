//! Shard `markdown_streaming_5` (screens S11, owner chat-live): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Renderers, Screen};

/// A table draws as its plain text: no table renderer (separators, wrapping, held tail).
const TABLE: &str = "G73: the chat draws a table as its plain text: the table renderer (box separators, cell wrapping at the width, the mutable tail held until the table is stable, the no-outer-pipes and fenced forms) is not in the transcript yet";
/// The plan stream: the heading and quote markers, the table rules and the prompt surface.
const PLAN: &str = "G73: items 1 and 3, the oracle's plan item draws the `#` heading marker and the `>` quote marker from the source and a table with a `━` rule under the header (tui markdown_render.rs); rtwk MarkdownView draws no source markers and a `│ ┼ ─` box table; the plan item stream `item/plan/delta` into a streaming plan item is the transcript owner's";
/// A mermaid fence draws as its source: no native mermaid renderer.
const MERMAID: &str = "G62: rtwk-rich MarkdownView has no mermaid renderer; a mermaid fence draws as its source text: the native mermaid renderer and its holdback are not built in the chat's markdown yet";
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
            c => out.push(c),
        }
    }
    out
}

/// One agent answer of the live chat streamed as `chunks`: one delta line per chunk.
fn answer_chunks(chunks: &[&str]) -> String {
    let mut out = String::new();
    for line in include_str!("fixtures/markdown_streaming_5/answer.jsonl").lines() {
        if line.contains("{MARKDOWN}") {
            for chunk in chunks {
                out.push_str(&line.replace("{MARKDOWN}", &json_string(chunk)));
                out.push('\n');
            }
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// The live chat at `width` x `height` after the answer streamed as `chunks`.
fn chat_chunks(width: u16, height: u16, chunks: &[&str]) -> Screen {
    let mut s = Screen::build(Owner::ChatLive)
        .markdown(width, height)
        .live()
        .open();
    s.feed(&answer_chunks(chunks));
    s
}

/// `chat_chunks` with the `tui.rendering` preferences set before the answer streams.
fn chat_chunks_under(rendering: Renderers, width: u16, height: u16, chunks: &[&str]) -> Screen {
    let mut s = Screen::build(Owner::ChatLive)
        .markdown(width, height)
        .live()
        .open();
    s.rendering(rendering);
    s.feed(&answer_chunks(chunks));
    s
}

/// The live chat at `width` x `height` after the whole answer `markdown` arrived at once.
fn chat(width: u16, height: u16, markdown: &str) -> Screen {
    chat_chunks(width, height, &[markdown])
}

/// The streamed screen equals the whole-answer screen: the oracle's "streamed == full render".
fn assert_matches_whole(s: &Screen, width: u16, height: u16, chunks: &[&str]) {
    s.assert_same_rows(&chat(width, height, &chunks.concat()).shot());
}

/// A turn that has streamed `chunks` and is still running: the oracle's controller state between
/// pushes. Push more with `push`, end the turn with `finish`.
fn streaming(width: u16, height: u16, chunks: &[&str]) -> Screen {
    let mut s = Screen::build(Owner::ChatLive)
        .markdown(width, height)
        .live()
        .open();
    let head: String = include_str!("fixtures/markdown_streaming_5/answer.jsonl")
        .lines()
        .take_while(|line| !line.contains("{MARKDOWN}"))
        .map(|line| format!("{line}\n"))
        .collect();
    s.feed(&head);
    for chunk in chunks {
        push(&mut s, chunk);
    }
    s
}

/// The rows above the Working row: the answer as it stands (the spinner row animates).
fn answer_rows(s: &Screen) -> std::ops::Range<usize> {
    0..s.rows()
        .iter()
        .position(|row| row.contains("Working"))
        .unwrap_or(0)
}

/// The answer's rows equal the shot's.
fn assert_answer_same(s: &Screen, before: &kit::Shot) {
    s.assert_same_rows_in(before, answer_rows(s));
}

/// One more delta of the running turn's answer.
fn push(s: &mut Screen, chunk: &str) {
    let line = include_str!("fixtures/markdown_streaming_5/answer.jsonl")
        .lines()
        .find(|line| line.contains("{MARKDOWN}"))
        .map(|line| line.replace("{MARKDOWN}", &json_string(chunk)))
        .unwrap_or_default();
    s.feed(&format!("{line}\n"));
}

/// The characters of `text` pushed one by one; `check` runs after each.
fn push_chars(s: &mut Screen, text: &str, mut check: impl FnMut(&Screen)) {
    for character in text.chars() {
        push(s, &character.to_string());
        check(s);
    }
}

/// The turn ends: the answer is committed in full.
fn finish(s: &mut Screen) {
    let done = include_str!("fixtures/markdown_streaming_5/answer.jsonl")
        .lines()
        .find(|line| line.contains("turn/completed"))
        .unwrap_or_default();
    s.feed(&format!("{done}\n"));
}

// ---- oracle: streaming/controller.rs (23 screen-level tests here; 22 state tests left to chat-live)
// ORACLE streaming/controller.rs:1025 controller_set_width_rebuilds_queued_lines kind=ui owner=chat-live
#[test]
fn controller_set_width_rebuilds_queued_lines() {
    // The oracle: a 120-column stream queues one long line; resized to 24 columns before its
    // commit, the line is drawn over several rows.
    let mut s = Screen::build(Owner::ChatLive)
        .markdown(120, 24)
        .live()
        .open();
    s.feed(&answer_chunks(&[
        "This is a long line that should wrap into multiple rows when resized.\n",
    ]));
    s.resize(24, 24);
    s.assert_in_order(&["This is a long line", "rows when", "resized."]);
}

// ORACLE streaming/controller.rs:1082 controller_has_live_tail_reflects_tail_presence kind=ui owner=chat-live
#[test]
fn controller_has_live_tail_reflects_tail_presence() {
    kit::na(
        Owner::ChatLive,
        "asserts has_live_tail(), a bool over the controller's tail fields (core.render.lines, enqueued_stable_len) that no screen draws",
    );
}

// ORACLE streaming/controller.rs:1095 plan_controller_has_live_tail_reflects_tail_presence kind=ui owner=chat-live
#[test]
fn plan_controller_has_live_tail_reflects_tail_presence() {
    kit::na(
        Owner::ChatLive,
        "asserts the plan controller's has_live_tail(), a bool over its tail fields that no screen draws",
    );
}

// ORACLE streaming/controller.rs:1122 controller_live_tail_rerenders_table_tail_after_resize kind=ui owner=chat-live
#[test]
fn controller_live_tail_rerenders_table_tail_after_resize() {
    kit::pending(Owner::ChatLive, TABLE);
    let table = "| # | Feature | Details | Link |\n| --- | --- | --- | --- |\n| 1 | RESIZE_REPRO_SENTINEL | long wrapped content that should be reflowed | https://example.com/resize |\n";
    for width in [48, 104, 56] {
        let mut s = chat(96, 24, table);
        s.resize(width, 24);
        // The oracle: the tail at each width is the full render of the source at that width.
        s.assert_same_rows(&chat(width, 24, table).shot());
        s.assert_contains("RESIZE_REPRO_SENTINEL");
    }
}

// ORACLE streaming/controller.rs:1211 controller_set_width_preserves_in_flight_tail kind=ui owner=chat-live
#[test]
fn controller_set_width_preserves_in_flight_tail() {
    // The oracle's rows keep the agent's `• ` bullet, so this screen is read whole.
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&answer_chunks(&["tail without newline"]));
    s.resize(24, 24);
    s.assert_contains("• tail without newline");
}

// ORACLE streaming/controller.rs:1258 plan_controller_set_width_preserves_in_flight_tail kind=ui owner=chat-live
#[test]
fn plan_controller_set_width_preserves_in_flight_tail() {
    // The plan item's text is drawn by the chat as its message: the finalized text survives a resize.
    // The oracle asserts an unwrapped transcript line; at 24 columns the screen wraps the same
    // text, so the fragments are checked in order across the wrap.
    let mut s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .live()
        .open();
    s.feed(&answer_chunks(&["1. Item without newline"]));
    s.resize(24, 24);
    s.assert_in_order(&["Item without", "newline"]);
}

// ORACLE streaming/controller.rs:1294 controller_loose_vs_tight_with_commit_ticks_matches_full kind=ui owner=chat-live
#[test]
fn controller_loose_vs_tight_with_commit_ticks_matches_full() {
    // The oracle's deltas (no width in the oracle test: 100 x 40 here).
    let deltas = [
        "\n\n",
        "Loose",
        " vs",
        ".",
        " tight",
        " list",
        " items",
        ":\n",
        "1",
        ".",
        " Tight",
        " item",
        "\n",
        "2",
        ".",
        " Another",
        " tight",
        " item",
        "\n\n",
        "1",
        ".",
        " Loose",
        " item",
        " with",
        " its",
        " own",
        " paragraph",
        ".\n\n",
        "  ",
        " This",
        " paragraph",
        " belongs",
        " to",
        " the",
        " same",
        " list",
        " item",
        ".\n\n",
        "2",
        ".",
        " Second",
        " loose",
        " item",
        " with",
        " a",
        " nested",
        " list",
        " after",
        " a",
        " blank",
        " line",
        ".\n\n",
        "  ",
        " -",
        " Nested",
        " bullet",
        " under",
        " a",
        " loose",
        " item",
        "\n",
        "  ",
        " -",
        " Another",
        " nested",
        " bullet",
        "\n\n",
    ];
    let s = chat_chunks(100, 40, &deltas);
    assert_matches_whole(&s, 100, 40, &deltas);
    s.assert_in_order(&[
        "Loose vs. tight list items:",
        "1. Tight item",
        "2. Another tight item",
        "3. Loose item with its own paragraph.",
        "This paragraph belongs to the same list item.",
        "4. Second loose item with a nested list after a blank line.",
        "Nested bullet under a loose item",
        "Another nested bullet",
    ]);
}

// ORACLE streaming/controller.rs:1413 controller_streamed_table_matches_full_render_widths kind=ui owner=chat-live
#[test]
fn controller_streamed_table_matches_full_render_widths() {
    let deltas = [
        "| Key | Description |\n",
        "| --- | --- |\n",
        "| -v | Enable very verbose logging output for debugging |\n",
        "\n",
    ];
    let s = chat_chunks(80, 24, &deltas);
    assert_matches_whole(&s, 80, 24, &deltas);
}

// ORACLE streaming/controller.rs:1432 controller_holds_blockquoted_table_tail_until_stable kind=ui owner=chat-live
#[test]
fn controller_holds_blockquoted_table_tail_until_stable() {
    let deltas = [
        "> | A | B |\n",
        "> | --- | --- |\n",
        "> | longvalue | ok |\n",
        "\n",
    ];
    let s = chat_chunks(80, 24, &deltas);
    assert_matches_whole(&s, 80, 24, &deltas);
}

// ORACLE streaming/controller.rs:1451 controller_keeps_pre_table_lines_queued_when_table_is_confirmed kind=snap owner=chat-live snap=completed_table_reference snap=completed_table_scrollback
#[test]
fn controller_keeps_pre_table_lines_queued_when_table_is_confirmed() {
    kit::pending(Owner::ChatLive, TABLE);
    let s = chat_chunks(
        80,
        24,
        &[
            "Intro line before table.\n",
            "| Key | Value |\n",
            "| --- | --- |\n",
            "| first | row |\n\nAfter table.\n\n| Next | Table |\n| --- | --- |\n| second | row |\n",
        ],
    );
    // The pre-table line commits on its own; the completed tables and the prose after them
    // enter scrollback together.
    s.assert_in_order(&["Intro line before table.", "After table."]);
    s.assert_snap("completed_table_scrollback");
    // The reference-definition part (oracle 1490-1530) needs the inline visualization context,
    // which the chat seeds nowhere: it is not ported here.
}

// ORACLE streaming/controller.rs:1538 controller_set_width_during_confirmed_table_stream_matches_finalize_render kind=ui owner=chat-live
#[test]
fn controller_set_width_during_confirmed_table_stream_matches_finalize_render() {
    let deltas = [
        "| Key | Description |\n",
        "| --- | --- |\n",
        "| one | value that should wrap after resize |\n",
    ];
    let mut s = chat_chunks(120, 24, &deltas);
    s.resize(32, 24);
    assert_matches_whole(&s, 32, 24, &deltas);
}

// ORACLE streaming/controller.rs:1612 controller_handles_table_immediately_after_heading kind=ui owner=chat-live
#[test]
fn controller_handles_table_immediately_after_heading() {
    let deltas = [
        "### 1) Basic table\n",
        "| Name | Role | Status |\n",
        "|---|---|---|\n",
        "| Alice | Admin | Active |\n",
        "| Bob | Editor | Pending |\n",
        "\n",
    ];
    let s = chat_chunks(100, 40, &deltas);
    assert_matches_whole(&s, 100, 40, &deltas);
}

// ORACLE streaming/controller.rs:1633 controller_renders_separators_for_multi_table_response_shape kind=ui owner=chat-live
#[test]
fn controller_renders_separators_for_multi_table_response_shape() {
    kit::pending(
        Owner::ChatLive,
        "G73: item 3, the oracle draws a table as columns two spaces apart with a `━` rule under the header and `─` rules between logical rows (tui markdown_render.rs:112 TABLE_HEADER_SEPARATOR_CHAR, :1851 render_table_separator); rtwk's MarkdownView draws a `│` grid with one `─┼─` header rule",
    );
    let source = "Absolutely. Here are several different Markdown table patterns you can use for rendering tests.\n\n| Name  | Role      |
  Location |\n|-------|-----------|----------|\n| Ava   | Engineer  | NYC      |\n| Malik | Designer  | Berlin   |\n| Priya | PM        | Remote
  |\n\n| Item        | Qty | Price | In Stock |\n|:------------|----:|------:|:--------:|\n| Keyboard    |   2 | 49.99 |    Yes   |\n| Mouse       |  10
   | 19.50 |    Yes   |\n| Monitor     |   1 | 219.0 |    No    |\n\n| Field         | Example                         | Notes
  |\n|---------------|----------------------------------|--------------------------|\n| Escaped pipe  | `foo \\| bar`                    | Should stay
  in one cell  |\n| Inline code   | `let x = value;`                | Monospace inline content |\n| Link          | [OpenAI](https://openai.com)    |
  Standard markdown link   |\n";
    let chunks: Vec<&str> = source.split_inclusive('\n').collect();
    let s = chat_chunks(120, 40, &chunks);
    assert_matches_whole(&s, 120, 40, &chunks);
    s.assert_contains("━");
}

// ORACLE streaming/controller.rs:1655 controller_renders_separators_for_no_outer_pipes_table_shape kind=ui owner=chat-live
#[test]
fn controller_renders_separators_for_no_outer_pipes_table_shape() {
    kit::pending(
        Owner::ChatLive,
        "G73: item 3, the oracle draws a table as columns two spaces apart with a `━` rule under the header and `─` rules between logical rows (tui markdown_render.rs:112 TABLE_HEADER_SEPARATOR_CHAR, :1851 render_table_separator); rtwk's MarkdownView draws a `│` grid with one `─┼─` header rule",
    );
    let source = "### 1) Basic\n\n| Name | Role | Active |\n|---|---|---|\n| Alice | Engineer | Yes |\n| Bob | Designer | No |\n\n### 2) No outer
  pipes\n\nCol A | Col B | Col C\n--- | --- | ---\nx | y | z\n10 | 20 | 30\n\n### 3) Another table\n\n| Key | Value |\n|---|---|\n| a | b |\n";
    let chunks: Vec<&str> = source.split_inclusive('\n').collect();
    let s = chat_chunks(100, 40, &chunks);
    assert_matches_whole(&s, 100, 40, &chunks);
    s.assert_absent("Col A | Col B | Col C");
    s.assert_contains("━");
}

// ORACLE streaming/controller.rs:1685 controller_stabilizes_first_no_outer_pipes_table_in_response kind=ui owner=chat-live
#[test]
fn controller_stabilizes_first_no_outer_pipes_table_in_response() {
    kit::pending(
        Owner::ChatLive,
        "G73: item 3, the oracle draws a table as columns two spaces apart with a `━` rule under the header and `─` rules between logical rows (tui markdown_render.rs:112 TABLE_HEADER_SEPARATOR_CHAR, :1851 render_table_separator); rtwk's MarkdownView draws a `│` grid with one `─┼─` header rule",
    );
    let deltas = [
        "### No outer pipes first\n\n",
        "Col A | Col B | Col C\n",
        "--- | --- | ---\n",
        "x | y | z\n",
        "10 | 20 | 30\n",
        "\n",
        "After table paragraph.\n",
    ];
    let s = chat_chunks(100, 40, &deltas);
    assert_matches_whole(&s, 100, 40, &deltas);
    s.assert_contains("━");
    s.assert_absent("Col A | Col B | Col C");
}

// ORACLE streaming/controller.rs:1716 controller_stabilizes_two_column_no_outer_table_in_response kind=ui owner=chat-live
#[test]
fn controller_stabilizes_two_column_no_outer_table_in_response() {
    kit::pending(
        Owner::ChatLive,
        "G73: item 3, the oracle draws a table as columns two spaces apart with a `━` rule under the header and `─` rules between logical rows (tui markdown_render.rs:112 TABLE_HEADER_SEPARATOR_CHAR, :1851 render_table_separator); rtwk's MarkdownView draws a `│` grid with one `─┼─` header rule",
    );
    let deltas = [
        "A | B\n",
        "--- | ---\n",
        "left | right\n",
        "\n",
        "After table paragraph.\n",
    ];
    let s = chat_chunks(80, 24, &deltas);
    assert_matches_whole(&s, 80, 24, &deltas);
    s.assert_contains("━");
    s.assert_absent("A | B");
}

// ORACLE streaming/controller.rs:1771 controller_keeps_markdown_fenced_tables_mutable_until_finalize kind=ui owner=chat-live
#[test]
fn controller_keeps_markdown_fenced_tables_mutable_until_finalize() {
    kit::pending(
        Owner::ChatLive,
        "the oracle's controller state (the table tail held mutable until it is stable) is not observable on the chat; probe: the body passes without it (it checks less than the oracle claims)",
    );
    let deltas = [
        "```md\n",
        "| A | B |\n",
        "|---|---|\n",
        "| 1 | 2 |\n",
        "```\n",
    ];
    let s = chat_chunks(80, 24, &deltas);
    assert_matches_whole(&s, 80, 24, &deltas);
    s.assert_contains("━");
    s.assert_absent("| A | B |");
}

// ORACLE streaming/controller.rs:1798 controller_keeps_markdown_fenced_no_outer_tables_mutable_until_finalize kind=ui owner=chat-live
#[test]
fn controller_keeps_markdown_fenced_no_outer_tables_mutable_until_finalize() {
    kit::pending(
        Owner::ChatLive,
        "the oracle's controller state (the table tail held mutable until it is stable) is not observable on the chat; probe: the body passes without it (it checks less than the oracle claims)",
    );
    let deltas = [
        "```md\n",
        "Col A | Col B | Col C\n",
        "--- | --- | ---\n",
        "x | y | z\n",
        "10 | 20 | 30\n",
        "```\n",
    ];
    let s = chat_chunks(100, 40, &deltas);
    assert_matches_whole(&s, 100, 40, &deltas);
    s.assert_contains("━");
    s.assert_absent("Col A | Col B | Col C");
}

// ORACLE streaming/controller.rs:1829 controller_live_view_matches_render_during_interleaved_table_streaming kind=ui owner=chat-live
#[test]
fn controller_live_view_matches_render_during_interleaved_table_streaming() {
    let source = "Project updates are easier to scan when narrative and structured data alternate.\n\n| Focus Area | Owner | Priority | Status |\n|---|---|---|---|\n| Authentication cleanup | Maya | High | 80% |\n| CLI error messages | Jordan | Medium | 55% |\n| Docs refresh | Lee | Low | 30% |\n\nThe first checkpoint shows progress, but we still have open risks.\n\n| Task | Command / Artifact | Due | State |\n|---|---|---|---|\n| Run unit tests | `cargo test -p codex-core` | Today | ✅ |\n| Snapshot review | `cargo insta pending-snapshots -p codex-tui` | Today | ⏳ |\n| Changelog draft | Release template (https://replacechangelog.com/) | Tomorrow | 📝 |\n\nFinal sign-off criteria are summarized below.\n";
    // The oracle compares the live view with the full render after every delta; a finished
    // turn shows only the last view, so the port compares the streamed end with the whole answer.
    let chunks: Vec<&str> = source.split_inclusive('\n').collect();
    let s = chat_chunks(72, 40, &chunks);
    assert_matches_whole(&s, 72, 40, &chunks);
}

// ORACLE streaming/controller.rs:1903 controller_keeps_non_markdown_fenced_tables_as_code kind=ui owner=chat-live
#[test]
fn controller_keeps_non_markdown_fenced_tables_as_code() {
    let deltas = [
        "```sh\n",
        "| A | B |\n",
        "|---|---|\n",
        "| 1 | 2 |\n",
        "```\n",
    ];
    let s = chat_chunks(80, 24, &deltas);
    assert_matches_whole(&s, 80, 24, &deltas);
    s.assert_contains("| A | B |");
    s.assert_absent("━");
    s.assert_absent("─");
}

// ORACLE streaming/controller.rs:1932 plan_controller_streamed_table_matches_final_render kind=ui owner=chat-live
#[test]
fn plan_controller_streamed_table_matches_final_render() {
    kit::pending(
        Owner::ChatLive,
        "G73: item 3, the oracle draws a table as columns two spaces apart with a `━` rule under the header (tui markdown_render.rs TABLE_HEADER_SEPARATOR_CHAR); rtwk MarkdownView draws a `│ ┼ ─` box-style table; the streamed plan reaches the chat as an agent answer here (the plan item stream `item/plan/delta` is the transcript owner's)",
    );
    let deltas = [
        "## Build plan\n\n",
        "| Step | Owner |\n",
        "|---|---|\n",
        "| Write tests | Agent |\n",
        "| Verify output | User |\n",
        "\n",
    ];
    let s = chat_chunks(80, 24, &deltas);
    assert_matches_whole(&s, 80, 24, &deltas);
    s.assert_contains("━");
    s.assert_absent("| Step | Owner |");
}

// ORACLE streaming/controller.rs:1960 finalized_plan_stream_preserves_semantic_url_fragments kind=snap owner=chat-live snap=finalized_plan_stream_preserves_semantic_url_fragments
#[test]
fn finalized_plan_stream_preserves_semantic_url_fragments() {
    kit::pending(Owner::ChatLive, PLAN);
    // The oracle pins its default colours (fg 30,30,30 / bg 255,255,255) and streams at width 32.
    let destination = "https://example.com/a/very/long/path/to/a/table/artifact";
    let source =
        format!("# Plan\n\n> Quote\n\n| Step | URL |\n| --- | --- |\n| Verify | {destination} |\n");
    let s = chat(32, 40, &source);
    s.assert_snap("finalized_plan_stream_preserves_semantic_url_fragments");
}

// ORACLE streaming/controller.rs:1999 plan_controller_streamed_markdown_fenced_table_matches_final_render kind=ui owner=chat-live
#[test]
fn plan_controller_streamed_markdown_fenced_table_matches_final_render() {
    kit::pending(
        Owner::ChatLive,
        "G73: item 3, the oracle draws a table as columns two spaces apart with a `━` rule under the header (tui markdown_render.rs TABLE_HEADER_SEPARATOR_CHAR); rtwk MarkdownView draws a `│ ┼ ─` box-style table; the streamed plan reaches the chat as an agent answer here (the plan item stream `item/plan/delta` is the transcript owner's)",
    );
    let deltas = [
        "## Build plan\n\n",
        "```md\n",
        "| Step | Owner |\n",
        "|---|---|\n",
        "| Write tests | Agent |\n",
        "| Verify output | User |\n",
        "```\n",
        "\n",
    ];
    let s = chat_chunks(80, 24, &deltas);
    assert_matches_whole(&s, 80, 24, &deltas);
    s.assert_contains("━");
}

// ---- oracle: streaming/controller_preview_tests.rs (6 screen-level tests here; 1 state tests left to chat-live)
// ORACLE streaming/controller_preview_tests.rs:6 followup_preview_hides_prompts_until_complete kind=snap owner=chat-live
#[test]
fn followup_preview_hides_prompts_until_complete() {
    // The oracle pushes `Before `, the directive's opener at once, then its label and prompt one
    // character at a time: the preview stays `Before` while the directive is open.
    let mut s = streaming(40, 24, &["Before "]);
    let prefix = s.shot();
    push(&mut s, ":codex-followup[");
    assert_answer_same(&s, &prefix);
    push_chars(
        &mut s,
        r#"Inspect `items[0]`]{prompt="Use \\"}\" and `ticks`""#,
        |s| assert_answer_same(s, &prefix),
    );
    s.assert_has_row("Before");
    s.assert_absent("prompt=");
    // The directive closes: its label reads in the line, the prompt never drawn.
    push(&mut s, "} after.");
    s.assert_has_row("Before Inspect items[0] after.");
    s.assert_absent("prompt=");
    // A long prompt held back while it grows, then the closing brace.
    let mut s = streaming(40, 24, &["Before "]);
    let prefix = s.shot();
    push(
        &mut s,
        &format!(":codex-followup[Long]{{prompt=\"{}", "界".repeat(3000)),
    );
    push(&mut s, "more prompt");
    assert_answer_same(&s, &prefix);
    push(&mut s, "\"}");
    s.assert_has_row("Before Long");
    push(&mut s, " after.");
    s.assert_has_row("Before Long after.");
    s.assert_absent("prompt=");
}

// ORACLE streaming/controller_preview_tests.rs:55 followup_preview_resumes_after_the_directive_leaves_the_window kind=ui owner=chat-live
#[test]
fn followup_preview_resumes_after_the_directive_leaves_the_window() {
    // The oracle bounds the preview to an 8 KiB window; the chat shows the whole safe tail
    // (pack departure), so the check is the one it ends on: the text ends with `Second after.`
    // and no prompt shows.
    let mut s = streaming(40, 24, &[":codex-followup[First]{prompt=secret}"]);
    push(&mut s, &"ordinary prose ".repeat(700));
    push(&mut s, ":codex-followup[Second]{prompt=hidden} after.");
    s.assert_contains("Second after.");
    s.assert_absent("prompt=");
}

// ORACLE streaming/controller_preview_tests.rs:100 followup_preview_handles_surrounding_unfinished_markdown kind=ui owner=chat-live
#[test]
fn followup_preview_handles_surrounding_unfinished_markdown() {
    let mut s = streaming(40, 24, &["[unfinished "]);
    let prefix = s.shot();
    push(&mut s, ":codex-followup[Inspect]{prompt=\"secret\"");
    assert_answer_same(&s, &prefix);
    s.assert_absent("prompt=");
    s.assert_absent("secret");
    push(&mut s, "}");
    s.assert_has_row("[unfinished Inspect");
    // Not a directive (no attributes): the text is what it is, character by character.
    for source in [
        ":codex-followup[not a directive] continuing text",
        ":codex-followup[not a directive] `code`",
    ] {
        let mut s = streaming(80, 24, &[]);
        push_chars(&mut s, source, |_| {});
        let whole = chat(80, 24, source);
        s.assert_in_order(&[":codex-followup[not a directive]"]);
        whole.assert_in_order(&[":codex-followup[not a directive]"]);
    }
}

// ORACLE streaming/controller_preview_tests.rs:135 unmatched_code_openers_do_not_expose_followup_prompts kind=snap owner=chat-live snap=unmatched_code_openers_do_not_expose_followup_prompts
#[test]
fn unmatched_code_openers_do_not_expose_followup_prompts() {
    // The oracle's inline snapshot: an unmatched code opener shows its text and no prompt.
    let mut s = streaming(80, 24, &["Type ` then "]);
    push(&mut s, ":codex-followup[Open]{prompt=secret}");
    s.assert_has_row("Type ` then");
    s.assert_absent("prompt=secret");
    // The turn ends: the whole text is committed, the unmatched backtick literal and the
    // directive its label.
    finish(&mut s);
    s.assert_has_row("Type ` then Open");
    s.assert_absent("prompt=secret");
}

// ORACLE streaming/controller_preview_tests.rs:180 unfinished_link_destinations_do_not_expand_the_preview kind=ui owner=chat-live
#[test]
fn unfinished_link_destinations_do_not_expand_the_preview() {
    let s = streaming(
        40,
        24,
        &["See [documentation](https://example.com/unfinished"],
    );
    s.assert_has_row("See [documentation]");
    s.assert_absent("example.com");
}

// ORACLE streaming/controller_preview_tests.rs:221 unfinished_links_flush_on_newline_or_completion kind=ui owner=chat-live
#[test]
fn unfinished_links_flush_on_newline_or_completion() {
    for (source, prefix) in [
        (
            "See [documentation](https://example.com/a/very/long/path",
            "See [documentation]",
        ),
        ("See :codex-followup[Inspect]{prompt=\"unfinished", "See"),
    ] {
        for ending in ["", "\n"] {
            let mut s = streaming(24, 24, &[source]);
            s.assert_has_row(prefix);
            s.resize(32, 24);
            s.assert_has_row(prefix);
            // The newline, or the turn's end, commits the source in full: the screen is the
            // whole message's.
            push(&mut s, ending);
            finish(&mut s);
            let whole = chat(32, 24, &format!("{source}{ending}"));
            s.assert_same_rows(&whole.shot());
        }
    }
}

// ---- oracle: streaming/inline_math_tests.rs (1 screen-level tests here; 0 state tests left to chat-live)
// ORACLE streaming/inline_math_tests.rs:7 unicode_math_inline_stream_preserves_display_exclusion_boundaries kind=ui owner=chat-live
#[test]
fn unicode_math_inline_stream_preserves_display_exclusion_boundaries() {
    for (open, close) in [("$$", "$$"), ("\\[", "\\]")] {
        let chunks = [open, "\n\nx^2\n\n", close, "\n\n", "After $\\alpha$.\n"];
        let s = chat_chunks(40, 24, &chunks);
        assert_matches_whole(&s, 40, 24, &chunks);
        for width in [16, 40] {
            let mut resized = chat_chunks(40, 24, &chunks);
            resized.resize(width, 24);
            resized.assert_same_rows(&chat(width, 24, &chunks.concat()).shot());
        }
    }
}

// ---- oracle: streaming/math_tests.rs (5 screen-level tests here; 0 state tests left to chat-live)
// ORACLE streaming/math_tests.rs:8 unicode_math_aligned_stream_matches_complete_render kind=ui owner=chat-live
#[test]
fn unicode_math_aligned_stream_matches_complete_render() {
    let source = "Before.\n\n\\[\n\\begin{aligned}\n\\underset{x}{\\min}&=\\sum_{i=1}^N x_{gpt}\\\\[2mm]\ny&=2\n\\end{aligned}\n\\]\n\nAfter $\\alpha$.\n";
    // The oracle streams one character at a time, at 80 and 12 columns (its Rich and Raw modes
    // are alt+r in the chat, pressed in the test).
    let chars: Vec<String> = source.chars().map(String::from).collect();
    let chunks: Vec<&str> = chars.iter().map(String::as_str).collect();
    for width in [80, 12] {
        let s = chat_chunks(width, 24, &chunks);
        assert_matches_whole(&s, width, 24, &chunks);
    }
}

// ORACLE streaming/math_tests.rs:42 unicode_math_stream_holds_display_until_closed_and_preserves_source kind=ui owner=chat-live
#[test]
fn unicode_math_stream_holds_display_until_closed_and_preserves_source() {
    for (open, close, prefix) in [("$$", "$$", "Before.\n\n"), ("\\[", "\\]", "Before.\n")] {
        let chunks = [
            prefix,
            open,
            "\n",
            "\\frac{a+b}{c}\n",
            "\n",
            close,
            "\n\n",
            "After $\\alpha^2$.\n",
        ];
        let s = chat_chunks(60, 24, &chunks);
        assert_matches_whole(&s, 60, 24, &chunks);
    }
}

// ORACLE streaming/math_tests.rs:98 unicode_math_raw_preview_preserves_line_on_newline kind=snap owner=chat-live snap=codex_tui__streaming__controller__math_tests__unicode_math_raw_preview
#[test]
fn unicode_math_raw_preview_preserves_line_on_newline() {
    kit::delta(
        Owner::ChatLive,
        "the unfinished display is drawn as its source wrapped at the width (`$$\\frac{abcd` over `efghijk}{lmn`); the oracle's preview is one unwrapped line",
    );
    // The oracle's raw preview at 12 columns: the source is one preview line, kept whole.
    let s = chat(12, 24, "$$\\frac{abcdefghijk}{lmnop}");
    s.assert_markdown_snap(
        "codex_tui__streaming__controller__math_tests__unicode_math_raw_preview",
    );
}

// ORACLE streaming/math_tests.rs:123 unicode_math_unfinished_display_is_visible_and_reflows kind=snap owner=chat-live snap=codex_tui__streaming__controller__math_tests__unicode_math_live_source_narrow
#[test]
fn unicode_math_unfinished_display_is_visible_and_reflows() {
    kit::delta(
        Owner::ChatLive,
        "the open display wraps its source at other columns (`\\frac{|a+b+c+d+e+f|}` over `{g+h}`, the oracle's `\\frac{|` over `a+b+c+d+e+f|}{g+h}`); the letters are the same",
    );
    // The oracle's open display at 20 columns: the live tail before its closer arrives.
    let s = chat(20, 24, "$$\n \\frac{|a+b+c+d+e+f|}{g+h}\n");
    s.assert_markdown_snap(
        "codex_tui__streaming__controller__math_tests__unicode_math_live_source_narrow",
    );
}

// ORACLE streaming/math_tests.rs:173 unicode_math_rejected_display_keeps_its_closer_and_following_text kind=ui owner=chat-live
#[test]
fn unicode_math_rejected_display_keeps_its_closer_and_following_text() {
    for (open, close) in [("$$", "$$"), ("\\[", "\\]")] {
        for (prefix, body) in [
            ("", "x".repeat(5000)),
            ("", "`x`".to_owned()),
            ("", format!("`{close}\nx`")),
            ("", "[x](https://example.com)".to_owned()),
            ("Equation: ", "x^2".to_owned()),
        ] {
            let owned = [
                format!("{prefix}{open}\n\n"),
                format!("{body}\n\n"),
                format!("{close}\n\n"),
                "After $\\alpha$.\n\n".to_owned(),
                "$$\\beta$$\n".to_owned(),
            ];
            let chunks: Vec<&str> = owned.iter().map(String::as_str).collect();
            let s = chat_chunks(40, 24, &chunks);
            s.assert_contains("After α.");
            s.assert_contains("β");
        }
    }
}

// ---- oracle: streaming/mermaid_tests.rs (3 screen-level tests here; 0 state tests left to chat-live)
// ORACLE streaming/mermaid_tests.rs:8 mermaid_stream_closes_resizes_and_preserves_raw_source kind=snap owner=chat-live snap=codex_tui__streaming__mermaid_tests__mermaid_stream_closes_resizes_and_preserves_raw_source
#[test]
fn mermaid_stream_closes_resizes_and_preserves_raw_source() {
    kit::pending(Owner::ChatLive, MERMAID);
    let chunks = [
        "Before.\n\n```mermaid\n",
        "flowchart LR\nA[\"Codex reply<br/>completed\"] --> B[\"Rendered in Ghostty\"]\n",
        "```\n",
        "\nAfter.\n",
    ];
    // The oracle's snapshot joins the stream's stages (one per resize); this is the closed stage at 80.
    let s = chat_chunks(80, 24, &chunks);
    s.assert_snap("codex_tui__streaming__mermaid_tests__mermaid_stream_closes_resizes_and_preserves_raw_source");
}

// ORACLE streaming/mermaid_tests.rs:82 mermaid_holdback_tracks_nested_blocks_after_normalized_markdown kind=ui owner=chat-live
#[test]
fn mermaid_holdback_tracks_nested_blocks_after_normalized_markdown() {
    kit::pending(
        Owner::ChatLive,
        "G62: rtwk-rich MarkdownView has no mermaid renderer; a mermaid fence draws as its source text: the native mermaid renderer and its holdback are not built in the chat's markdown yet; probe: the body passes without it (it checks less than the oracle claims)",
    );
    let prefix = "```md\n| Name | Value |\n| --- | --- |\n| A | B |\n```\n\n";
    for block in [
        "> ```mermaid\n> flowchart LR\n> A --> B\n> ```\n",
        "- Diagram:\n\n  ```mermaid\n  flowchart LR\n  A --> B\n  ```\n",
    ] {
        // The oracle's holdback (mutable_fence_start) is internal; the closed diagram's text shows.
        let s = chat(80, 24, &format!("{prefix}{block}"));
        s.assert_contains("A --> B");
    }
}

// ORACLE streaming/mermaid_tests.rs:142 mermaid_controller_keeps_diagram_mutable_and_returns_original_source kind=ui owner=chat-live
#[test]
fn mermaid_controller_keeps_diagram_mutable_and_returns_original_source() {
    kit::pending(Owner::ChatLive, MERMAID);
    let source = "```mermaid\nflowchart LR\nA[Request<br />completed] --> B[Reply]\n```\n";
    let mut s = chat(80, 24, source);
    s.assert_contains("┌");
    s.resize(8, 24);
    s.assert_in_order(&["A[Request", "completed]"]);
    s.resize(80, 24);
    s.assert_same_rows(&chat(80, 24, source).shot());
}

// ---- oracle: streaming/render_tests.rs (17 screen-level tests here; 0 state tests left to chat-live)
// ORACLE streaming/render_tests.rs:89 incremental_render_keeps_final_block_mutable_and_matches_full_render kind=snap owner=chat-live snap=incremental_render_representative_stream
#[test]
fn incremental_render_keeps_final_block_mutable_and_matches_full_render() {
    kit::pending(
        Owner::ChatLive,
        "kit and G73: the snapshot is the Debug dump of the oracle's `HyperlinkLine`s (spans with their styles and link columns), which the kit has no reader for (it compares rows and `Buffer` dumps); the screen also differs by G73 item 1 (quote `> ` marker, fence rows, code padding) and item 3 (the table); the incremental render equals the whole one (`assert_matches_whole` holds)",
    );
    let chunks = [
        "# Heading\n",
        "\n",
        "First paragraph with a [link](https://example.com).\n",
        "continued on the next line.\n\n",
        "1. First item\n",
        "2. Second item\n\n",
        "> Quoted paragraph\n\n",
        "```rust\n",
        "fn main() {}\n",
        "```\n\n",
        "| Key | Value |\n",
        "| --- | --- |\n",
        "| alpha | beta |\n",
    ];
    let s = chat_chunks(80, 24, &chunks);
    assert_matches_whole(&s, 80, 24, &chunks);
    s.assert_snap("incremental_render_representative_stream");
}

// ORACLE streaming/render_tests.rs:119 incremental_file_citations_preserve_metadata_unicode_and_markdown kind=snap owner=chat-live snap=incremental_file_citations
#[test]
fn incremental_file_citations_preserve_metadata_unicode_and_markdown() {
    // The oracle's two cases (a workbook citation, a unicode name) are one snapshot of both
    // renders; each case is streamed and compared with its whole answer here.
    for (filename, prefix) in [
        (
            "Quarterly Report.xlsx",
            "- :codex-file-citation{artifact_kind=\"workbook\" ",
        ),
        ("Résumé *final* ✨.xlsx", "- :codex-file-citation{"),
    ] {
        let tail = format!("path=\"{filename}\"}}\n");
        let chunks = ["# Output\n\n", prefix, tail.as_str(), "\n", "Continue.\n"];
        let s = chat_chunks(80, 24, &chunks);
        assert_matches_whole(&s, 80, 24, &chunks);
    }
}

// ORACLE streaming/render_tests.rs:145 growing_single_top_level_blocks_render_and_scan_in_one_pass kind=ui owner=chat-live
#[test]
fn growing_single_top_level_blocks_render_and_scan_in_one_pass() {
    // The oracle's block-scan counters (last_top_level_block_start, stable_source_len) are internal;
    // the visible part is that each growing block renders as the whole answer does.
    let chunks = [
        "A paragraph that keeps growing\n",
        "without a blank line between chunks.\n",
        "It stays one top-level block.\n",
        "| Key | Value |\n",
        "| --- | --- |\n",
        "| alpha | beta |\n",
        "| gamma | delta |\n",
    ];
    let s = chat_chunks(80, 24, &chunks);
    assert_matches_whole(&s, 80, 24, &chunks);
}

// ORACLE streaming/render_tests.rs:181 incremental_raw_render_preserves_blank_lines kind=ui owner=chat-live
#[test]
fn incremental_raw_render_preserves_blank_lines() {
    // The oracle's raw render: the lines are "alpha", "", "beta", "".
    let mut s = Screen::build(Owner::ChatLive)
        .markdown(80, 24)
        .live()
        .open();
    s.keys("alt+r");
    s.feed(&answer_chunks(&["alpha\n", "\n", "beta\n", "\n"]));
    s.assert_screen_row_then("alpha", &["", "beta"]);
}

// ORACLE streaming/render_tests.rs:205 compact_list_spacing_keeps_incremental_and_full_renders_aligned kind=ui owner=chat-live
#[test]
fn compact_list_spacing_keeps_incremental_and_full_renders_aligned() {
    let chunks = [
        "- Short\n",
        "- A sibling with enough words to wrap across rows\n",
        "- Last\n",
        "\nAfter the list.\n\n",
        "| Name | Value |\n| --- | --- |\n| A | B |\n",
        "\n- Nested\n  - One\n  - Two\n- End\n",
    ];
    for width in [80, 24] {
        let s = chat_chunks(width, 24, &chunks);
        assert_matches_whole(&s, width, 24, &chunks);
    }
}

// ORACLE streaming/render_tests.rs:258 inline_visualization_context_without_directives_keeps_stable_prefix kind=ui owner=chat-live
#[test]
fn inline_visualization_context_without_directives_keeps_stable_prefix() {
    kit::na(
        Owner::ChatLive,
        "asserts stable_source_len and has_inline_visualization_directive, internal flags of the streaming render; no screen draws them",
    );
}

// ORACLE streaming/render_tests.rs:293 inline_visualizations_use_canonical_full_render kind=ui owner=chat-live
#[test]
fn inline_visualizations_use_canonical_full_render() {
    let chunks = ["Before.\n\n", "::codex-inline-vis{file=\"missing.html\"}\n"];
    let s = chat_chunks(80, 24, &chunks);
    assert_matches_whole(&s, 80, 24, &chunks);
    s.assert_absent("codex-inline-vis");
    s.assert_has_row("Visualization unavailable on this device.");
    // A directive still being written is not shown: its line waits for the closing brace.
    let mut live = streaming(80, 24, &["Before.\n\n", "::codex-inline-vis{file=\"mis"]);
    live.assert_absent("codex-inline-vis");
    push(&mut live, "sing.html\"}\n");
    live.assert_absent("codex-inline-vis");
    live.assert_has_row("Visualization unavailable on this device.");
}

// ORACLE streaming/render_tests.rs:328 inline_visualizations_without_context_use_canonical_full_render kind=snap owner=chat-live snap=inline_visualizations_without_context_use_canonical_full_render
#[test]
fn inline_visualizations_without_context_use_canonical_full_render() {
    let chunks = ["Before.\n\n", "::codex-inline-vis{file=\"missing.html\"}\n"];
    let s = chat_chunks(80, 24, &chunks);
    assert_matches_whole(&s, 80, 24, &chunks);
    // The snapshot is a `Vec<HyperlinkLine>` Debug dump, which the kit's snapshot reader does not
    // parse (4 snaps of the shard are in that form); its rows are asserted as the dump's literals:
    // `Before.`, an empty line, the note in italics.
    s.assert_in_order(&["Before.\n\nVisualization unavailable on this device."]);
    s.assert_text_style("Visualization unavailable on this device.", "italic");
}

// ORACLE streaming/render_tests.rs:343 inline_visualization_content_references_use_canonical_full_render kind=ui owner=chat-live
#[test]
fn inline_visualization_content_references_use_canonical_full_render() {
    let chunks = [
        "Before.\n\n",
        "\u{e200}visualize\u{e202}{\"path\":\"/tmp/missing.html\"}\u{e201}\n",
    ];
    let s = chat_chunks(80, 24, &chunks);
    assert_matches_whole(&s, 80, 24, &chunks);
    s.assert_absent("visualize");
    s.assert_has_row("Visualization unavailable on this device.");
}

// ORACLE streaming/render_tests.rs:357 inline_visualization_directive_survives_raw_to_rich_render_mode_switch kind=ui owner=chat-live
#[test]
fn inline_visualization_directive_survives_raw_to_rich_render_mode_switch() {
    // The oracle streams the directive in Raw, then switches to Rich: the directive survives.
    // Raw rows have no bullet column: the screen is read whole, not through `markdown()`.
    let mut s = Screen::build(Owner::ChatLive).size(83, 24).live().open();
    s.keys("alt+r");
    s.feed(&answer_chunks(&[
        "::codex-inline-vis{file=\"missing.html\"}\n",
    ]));
    s.assert_has_row("::codex-inline-vis{file=\"missing.html\"}");
    s.keys("alt+r");
    s.assert_absent("codex-inline-vis");
    s.assert_has_row("• Visualization unavailable on this device.");
}

// ORACLE streaming/render_tests.rs:398 reference_link_definition_recomputes_earlier_and_later_blocks kind=ui owner=chat-live
#[test]
fn reference_link_definition_recomputes_earlier_and_later_blocks() {
    let chunks = [
        "Earlier [reference][id].\n\n",
        "An unrelated paragraph.\n\n",
        "[id]: https://example.com/reference\n",
        "\n",
        "Later [reference][id].\n",
    ];
    let s = chat_chunks(80, 24, &chunks);
    assert_matches_whole(&s, 80, 24, &chunks);
    s.assert_in_order(&["Earlier", "Later"]);
}

// ORACLE streaming/render_tests.rs:426 reference_link_definition_survives_raw_to_rich_render_mode_switch kind=ui owner=chat-live
#[test]
fn reference_link_definition_survives_raw_to_rich_render_mode_switch() {
    // Raw rows have no bullet column: the screen is read whole, not through `markdown()`.
    let chunks = [
        "[id]: https://example.com/reference\n\n",
        "First [reference][id].\n\n",
        "Later [reference][id].\n",
    ];
    let mut s = Screen::build(Owner::ChatLive).size(83, 24).live().open();
    s.keys("alt+r");
    s.feed(&answer_chunks(&chunks));
    s.assert_has_row("[id]: https://example.com/reference");
    s.assert_has_row("First [reference][id].");
    s.keys("alt+r");
    s.assert_absent("[id]: https://example.com/reference");
    // The definition resolved both references that came after it in raw mode.
    s.assert_in_order(&[
        "First reference (https://example.com/reference).",
        "Later reference (https://example.com/reference).",
    ]);
}

// ORACLE streaming/render_tests.rs:455 incremental_render_does_not_add_blank_line_before_html_block kind=ui owner=chat-live
#[test]
fn incremental_render_does_not_add_blank_line_before_html_block() {
    let chunks = [
        "Paragraph.\n\n",
        "<div>x</div>\n\n",
        "Following paragraph.\n",
    ];
    let s = chat_chunks(80, 24, &chunks);
    assert_matches_whole(&s, 80, 24, &chunks);
}

// ORACLE streaming/render_tests.rs:474 incremental_render_preserves_heading_and_normalized_html_seams kind=ui owner=chat-live
#[test]
fn incremental_render_preserves_heading_and_normalized_html_seams() {
    let chunks = [
        "Paragraph.\n\n",
        "<div>x</div>\n\n",
        "```markdown\n",
        "| Key | Value |\n",
        "| --- | --- |\n",
        "| alpha | beta |\n",
        "\n",
        "Following paragraph.\n",
        "```\n",
    ];
    let s = chat_chunks(80, 24, &chunks);
    assert_matches_whole(&s, 80, 24, &chunks);
}

// ORACLE streaming/render_tests.rs:495 paragraphs_after_unwrapped_table_fence_advance_stable_source kind=ui owner=chat-live
#[test]
fn paragraphs_after_unwrapped_table_fence_advance_stable_source() {
    // The oracle's stable_source_len advance is internal; the visible part is the whole render.
    let chunks = [
        "```markdown\n",
        "| Key | Value |\n",
        "| --- | --- |\n",
        "| alpha | beta |\n",
        "```\n\n",
    ];
    let s = chat_chunks(80, 24, &chunks);
    assert_matches_whole(&s, 80, 24, &chunks);
}

// ORACLE streaming/render_tests.rs:518 shell_pid_preserves_following_equations kind=ui owner=chat-live
#[test]
fn shell_pid_preserves_following_equations() {
    for shell in [
        "Shell examples: $HOME and echo $$.",
        "Shell examples: $HOME.",
    ] {
        let source = format!("{shell}\n\nAfter rejected equations: $\\alpha$.\n\n$$\\beta$$");
        let chunks: Vec<&str> = source.split_inclusive('$').collect();
        for width in [80, 24] {
            let s = chat_chunks(width, 24, &chunks);
            assert_matches_whole(&s, width, 24, &chunks);
            if width == 80 {
                s.assert_contains("After rejected equations: α.");
            } else {
                s.assert_in_order(&["After rejected", "equations: α."]);
            }
            s.assert_contains("β");
        }
    }
}

// ORACLE streaming/render_tests.rs:545 rejected_math_openers_allow_bounded_incremental_rendering kind=ui owner=chat-live
#[test]
fn rejected_math_openers_allow_bounded_incremental_rendering() {
    for (open, close) in [
        ("echo $$", "$$"),
        ("Equation: \\[", "\\]"),
        (r"\[label\]\*", "\\]"),
    ] {
        let opener = format!("{open}\n\n");
        let distant_closer = format!("{close}\n\nAfter $\\alpha$.\n\n");
        let mut chunks: Vec<&str> = vec![opener.as_str()];
        let filler = "An ordinary paragraph that must not retain the entire response.\n\n";
        chunks.extend(std::iter::repeat_n(filler, 40));
        chunks.push(distant_closer.as_str());
        let s = chat_chunks(80, 24, &chunks);
        assert_matches_whole(&s, 80, 24, &chunks);
    }
}

// ---- oracle: streaming/rendering_preferences_tests.rs (3 screen-level tests here; 1 state tests left to chat-live)
// ORACLE streaming/rendering_preferences_tests.rs:9 empty_list_item_stays_mutable_until_finalization kind=ui owner=chat-live
#[test]
fn empty_list_item_stays_mutable_until_finalization() {
    for (marker, continuation) in [
        ("8.", ""),
        ("8.\n", "   answer\n"),
        ("> 8.\n", ">    answer\n"),
    ] {
        let chunks = [marker, continuation];
        let s = chat_chunks(24, 24, &chunks);
        assert_matches_whole(&s, 24, 24, &chunks);
    }
}

// ORACLE streaming/rendering_preferences_tests.rs:62 lists_match_incremental_and_emitted_responses kind=ui owner=chat-live
#[test]
fn lists_match_incremental_and_emitted_responses() {
    let chunks = [
        "- [ ] First task with enough words to wrap\n  - [x] Nested task\n- [X] Done\n\n",
        "> 9. [ ] [link](https://example.com)\n> 10. [x] Done\n\n",
        "After.\n\nOne.\n\nTwo.\n\nThree.\n",
    ];
    // the oracle's `lists` preference on and off: the streamed screen is the whole one
    for lists in [true, false] {
        let rendering = Renderers {
            lists,
            ..Renderers::default()
        };
        let s = chat_chunks_under(rendering, 80, 24, &chunks);
        s.assert_same_rows(&chat_chunks_under(rendering, 80, 24, &[&chunks.concat()]).shot());
    }
    // the preference changes the drawing: a task's box is its source text only while lists are off
    let on = chat_chunks_under(Renderers::default(), 80, 24, &chunks);
    let off = chat_chunks_under(
        Renderers {
            lists: false,
            ..Renderers::default()
        },
        80,
        24,
        &chunks,
    );
    on.assert_rows_differ(&off.shot());
    off.assert_has_row("• [ ] First task with enough words to wrap");
    // turning the preference off redraws a message that is already on the screen
    let mut flipped = chat_chunks_under(Renderers::default(), 80, 24, &chunks);
    flipped.rendering(Renderers {
        lists: false,
        ..Renderers::default()
    });
    flipped.assert_same_rows(&off.shot());
}

// ORACLE streaming/rendering_preferences_tests.rs:108 rendering_preferences_match_incremental_final_and_emitted_responses kind=ui owner=chat-live
#[test]
fn rendering_preferences_match_incremental_final_and_emitted_responses() {
    let chunks = [
        "Before.\n\n",
        "```markdown\n| A | B |\n|---|---|\n| `a` | \\(x_1\\) |\n```\n\n",
        "```mermaid\nflowchart LR\nA --> B\n```\n\n",
        "Inline \\(\\alpha_1\\) and $x^2$.\n\n\\[\n\\frac{a}{b}\n\\]\n\n",
        "| Name | Value |\n|---|---|\n| **a** | $x^2$ |\n\n",
        "Prose after the table.\n\nOne.\n\nTwo.\n\nThree.\n\nFour.\n\n",
        "| **Name** | Value |\n|---|---|\n| `a` | b |\n\nAfter.\n",
    ];
    // the oracle's eight combinations of mermaid, math and tables
    for bits in 0..8u8 {
        let rendering = Renderers {
            mermaid: bits & 1 != 0,
            math: bits & 2 != 0,
            tables: bits & 4 != 0,
            lists: true,
        };
        let s = chat_chunks_under(rendering, 80, 24, &chunks);
        s.assert_same_rows(&chat_chunks_under(rendering, 80, 24, &[&chunks.concat()]).shot());
    }
}
