// test root: pages
#![allow(clippy::expect_used, clippy::unwrap_used)]

//! What a drag over an assistant's Markdown copies besides its words (oracle
//! `markdown_copy.rs`, `transcript_view/markdown_copy_tests.rs`): the plain text is what the area
//! selected, and the selection is also offered as balanced Markdown (`CopyFormat::MarkdownSelection`,
//! whose HTML the host puts on the clipboard) that keeps the emphasis, the code spans, the links,
//! the list markers, the headings and the fences of what was selected. A selection of only code, or
//! of only a code span, or of a quote, stays plain text.

use codex_tui_core::app::Command;
use codex_tui_core::app::effects::{Ask, CopyFormat};
use serde_json::json;

use crate::kit::{Owner, Screen};

/// One finished turn whose answer is `markdown`.
fn script(markdown: &str) -> String {
    let item = json!({
        "type": "agentMessage", "moderation": null, "id": "msg-1", "text": markdown,
        "phase": "final_answer", "memoryCitation": null, "delivery": null, "questions": null,
        "startedAtMs": null, "completedAtMs": null,
    });
    let turn = json!({
        "method": "turn/completed",
        "params": {"threadId": "thread-1", "turn": {
            "id": "turn-1", "rootTurnId": null, "items": [item], "itemsView": "summary",
            "status": "completed", "error": null, "startedAt": null, "completedAt": null,
            "durationMs": null,
        }},
    });
    format!(
        "{}\n{}\n",
        r#"{"step": "seed", "seed": "session", "thread": "thread-1"}"#,
        json!({"step": "notify", "message": turn}),
    )
}

/// A chat whose only answer is `markdown`, shown once `shown` is on the screen.
fn chat(markdown: &str, shown: &str) -> Screen {
    // the screen keeps its script for the run: a test's, leaked
    let script: &'static str = Box::leak(script(markdown).into_boxed_str());
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(script)
        .open();
    s.wait_for(shown);
    s
}

/// Where `text` starts on the screen: its column and row.
fn at(s: &Screen, text: &str) -> (u16, u16) {
    for (y, row) in s.rows().iter().enumerate() {
        if let Some(byte) = row.find(text) {
            return (
                u16::try_from(row[..byte].chars().count()).expect("a column"),
                u16::try_from(y).expect("a row"),
            );
        }
    }
    panic!("no `{text}` on the screen:\n{}", s.text());
}

/// Drags from the first cell of `from` to the last cell of `to`.
fn select(s: &mut Screen, from: &str, to: &str) {
    let (x, y) = at(s, from);
    let (x2, y2) = at(s, to);
    let width = u16::try_from(to.chars().count()).expect("a width");
    s.drag(x, y, x2 + width - 1, y2);
}

/// The copies the app has asked of the host, oldest first.
fn copies(s: &Screen) -> Vec<(String, CopyFormat)> {
    s.commands()
        .into_iter()
        .filter_map(|command| match command {
            Command::Effect {
                ask: Ask::Copy { text, format, .. },
                ..
            } => Some((text, format)),
            _ => None,
        })
        .collect()
}

/// The one copy a drag made: its text, and the Markdown it came with (none: plain text).
fn copied(s: &Screen) -> (String, Option<String>) {
    let mut all = copies(s);
    assert_eq!(all.len(), 1, "{}", s.text());
    match all.remove(0) {
        (text, CopyFormat::MarkdownSelection(source)) => (text, Some(source.to_string())),
        (text, CopyFormat::PlainText) => (text, None),
        (text, format) => panic!("`{text}` copied as {format:?}"),
    }
}

#[test]
fn a_selection_keeps_the_emphasis_strong_and_code_it_spans() {
    let mut s = chat(
        "Plain **bold text** and _slanted_ with `code` end",
        "slanted",
    );
    select(&mut s, "bold text", "with code");
    let (text, markdown) = copied(&s);
    assert_eq!(text, "bold text and slanted with code");
    assert_eq!(
        markdown.as_deref(),
        Some("**bold text** and _slanted_ with `code`")
    );
}

#[test]
fn a_selection_inside_a_word_of_a_styled_run_wraps_only_what_was_selected() {
    let mut s = chat("Some **strongly worded** advice", "advice");
    select(&mut s, "worded", "advice");
    let (text, markdown) = copied(&s);
    assert_eq!(text, "worded advice");
    assert_eq!(markdown.as_deref(), Some("**worded** advice"));
}

#[test]
fn only_a_code_span_is_copied_as_the_words_it_holds() {
    let mut s = chat("Call `frobnicate()` twice", "twice");
    select(&mut s, "frobnicate()", "frobnicate()");
    assert_eq!(copied(&s), ("frobnicate()".to_owned(), None));
}

#[test]
fn a_link_keeps_its_destination_and_what_is_not_markup_is_escaped() {
    let mut s = chat(
        "See [the docs](https://example.com/docs) for a_b and *stars*.",
        "stars",
    );
    select(&mut s, "the docs", "stars.");
    let (text, markdown) = copied(&s);
    assert!(text.starts_with("the docs (https://example.com/docs) for a_b"));
    assert_eq!(
        markdown.as_deref(),
        Some(
            "[the docs (https://example.com/docs)](<https://example.com/docs>) for a\\_b and _stars_."
        )
    );
}

#[test]
fn list_items_keep_their_markers_and_a_heading_its_hashes() {
    let mut s = chat("# Plan\n\n- first\n- second **bold**\n- third", "third");
    select(&mut s, "# Plan", "second bold");
    let (text, markdown) = copied(&s);
    assert_eq!(text, "# Plan\n\nfirst\nsecond bold");
    assert_eq!(
        markdown.as_deref(),
        Some("# Plan\n\n- first\n- second **bold**")
    );
}

#[test]
fn a_selection_that_starts_in_an_item_keeps_the_marker_of_the_item() {
    let mut s = chat("1. alpha\n2. beta\n3. gamma", "gamma");
    select(&mut s, "beta", "gamma");
    let (_, markdown) = copied(&s);
    assert_eq!(markdown.as_deref(), Some("2. beta\n3. gamma"));
}

#[test]
fn code_selected_with_prose_is_fenced_and_code_alone_is_literal() {
    let source = "Run this:\n\n```sh\nls -l\n```\n\nthen `exit`";
    let mut s = chat(source, "then exit");
    select(&mut s, "Run this:", "then exit");
    let (_, markdown) = copied(&s);
    assert_eq!(
        markdown.as_deref(),
        Some("Run this:\n\n```\nls -l\n```\n\nthen `exit`")
    );

    let mut s = chat(source, "then exit");
    select(&mut s, "ls -l", "ls -l");
    assert_eq!(copied(&s), ("ls -l".to_owned(), None));
}

#[test]
fn a_quote_alone_is_copied_without_its_marks_as_plain_text() {
    let mut s = chat("> quoted **words**\n\nafter", "after");
    select(&mut s, "quoted", "words");
    let (text, markdown) = copied(&s);
    assert_eq!(text, "quoted words");
    assert_eq!(markdown, None);
}

#[test]
fn a_quote_selected_with_prose_keeps_its_mark() {
    let mut s = chat("before\n\n> quoted **words**\n\nafter", "after");
    select(&mut s, "before", "after");
    let (_, markdown) = copied(&s);
    assert_eq!(
        markdown.as_deref(),
        Some("before\n\n> quoted **words**\n\nafter")
    );
}

#[test]
fn ctrl_c_copies_the_selection_with_its_markdown_too() {
    let mut s = chat("Plain **bold text** here", "here");
    select(&mut s, "bold text", "here");
    s.keys("ctrl+c");
    s.wait(50);
    let all = copies(&s);
    assert_eq!(all.len(), 2, "{}", s.text());
    assert_eq!(all[0], all[1]);
    assert_eq!(
        all[1].1,
        CopyFormat::MarkdownSelection("**bold text** here".into())
    );
}

#[test]
fn words_with_no_formatting_stay_plain_text() {
    let mut s = chat("one two three", "three");
    select(&mut s, "two", "three");
    assert_eq!(copied(&s), ("two three".to_owned(), None));
}
