//! The context a submit adds for task mentions and how a replayed prompt reads it back
//! (`kit::mentions::task_refs`; oracle `task_mentions_tests.rs`).

#![allow(clippy::expect_used, clippy::unwrap_used)]

use codex_tui_core::kit::mentions::{TaskRef, decode_task_links, with_task_context};
use pretty_assertions::assert_eq;

fn task(text: &str, title: &str, thread: &str) -> TaskRef {
    let at = text.find(&format!("@{title}")).expect("the mention is in the text");
    TaskRef {
        range: at..at + title.len() + 1,
        title: title.into(),
        thread: thread.into(),
    }
}

#[test]
fn the_context_and_heading_go_before_a_text_without_a_request_heading() {
    let text = "Inspect @Fix it now";
    let tasks = [task(text, "Fix it", "t-1")];
    let sent = with_task_context(text, &tasks, None, &[]).expect("a reference");
    assert!(sent.text.starts_with("## Referenced chats with Codex:\n"));
    assert!(sent.text.contains("You MUST call `read_thread`"));
    assert!(sent.text.contains(r#"[{"threadId":"t-1"}]"#));
    assert!(
        sent.text
            .ends_with("## My request for Codex:\nInspect [@Fix it](thread://t-1) now")
    );
    assert_eq!(&sent.text[sent.tasks[0].clone()], "[@Fix it](thread://t-1)");
}

#[test]
fn an_existing_request_heading_gets_the_context_before_it() {
    let text = "Context\n## My request for Codex:\nlook at @A";
    let tasks = [task(text, "A", "t-2")];
    let sent = with_task_context(text, &tasks, None, &[]).expect("a reference");
    let (before, after) = sent.text.split_once("## My request for Codex:").unwrap();
    assert!(before.starts_with("Context\n## Referenced chats"));
    assert_eq!(after, "\nlook at [@A](thread://t-2)");
    assert_eq!(&sent.text[sent.tasks[0].clone()], "[@A](thread://t-2)");
}

#[test]
fn the_current_thread_and_repeats_are_not_references() {
    let text = "@A @B @A";
    let tasks = [
        task(text, "A", "t-1"),
        task(text, "B", "cur"),
        TaskRef {
            range: 6..8,
            title: "A".into(),
            thread: "t-1".into(),
        },
    ];
    let sent = with_task_context(text, &tasks, Some("cur"), &[]).expect("a reference");
    assert_eq!(sent.text.matches("threadId").count(), 1);
    // the current thread's mention stays as typed, the others become links
    assert!(sent.text.ends_with("[@A](thread://t-1) @B [@A](thread://t-1)"));
    assert!(with_task_context("@B", &tasks[1..2], Some("cur"), &[]).is_none());
}

#[test]
fn at_most_sixteen_threads_are_referenced() {
    let text: String = (0..20).map(|n| format!("@T{n} ")).collect();
    let tasks: Vec<_> = (0..20).map(|n| task(&text, &format!("T{n}"), &format!("t{n}"))).collect();
    let sent = with_task_context(&text, &tasks, None, &[]).expect("references");
    assert_eq!(sent.text.matches("threadId").count(), 16);
    assert!(sent.text.contains("[@T15](thread://t15)"));
    // the seventeenth stays as typed
    assert!(sent.text.contains(" @T16 "));
}

#[test]
fn a_title_with_brackets_is_escaped_and_reads_back() {
    let text = r"see @a]b\c";
    let tasks = [task(text, r"a]b\c", "t-3")];
    let sent = with_task_context(text, &tasks, None, &[]).expect("a reference");
    assert!(sent.text.ends_with(r"[@a\]b\\c](thread://t-3)"));
    let range = sent.tasks[0].clone();
    let (decoded, moved) = decode_task_links(&sent.text, &[range]);
    assert!(decoded.ends_with(r"see @a]b\c"));
    assert_eq!(&decoded[moved[0].clone()], r"@a]b\c");
}

#[test]
fn other_elements_move_with_the_inserted_text() {
    let text = "[Image #1] and @A";
    let tasks = [task(text, "A", "t-4")];
    let image = 0..10;
    let sent = with_task_context(text, &tasks, None, std::slice::from_ref(&image)).unwrap();
    let moved = sent.map(image);
    assert_eq!(&sent.text[moved], "[Image #1]");
}

#[test]
fn a_link_no_element_covers_is_left_alone() {
    let text = "a [@x](thread://t-1) b";
    let (decoded, moved) = decode_task_links(text, &[]);
    assert_eq!(decoded, text);
    assert!(moved.is_empty());
}
