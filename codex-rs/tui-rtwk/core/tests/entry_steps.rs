//! The `entry` and `commit` steps of a script: the transcript's entries in the shorthand the
//! transcript tests write, played through `ingest` like the notifications they stand for.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use codex_tui_core::app::CodexApp;
use codex_tui_core::fixtures::script::{build, parse};
use codex_tui_core::threads::{Item, ToolKind, ToolStatus, TurnState};
use pretty_assertions::assert_eq;

const SESSION: &str = r#"{"step":"seed","seed":"session","thread":"thread-1"}"#;

fn play(lines: &[&str]) -> CodexApp {
    let text = format!("{SESSION}\n{}\n", lines.join("\n"));
    build(&text).expect("plays").app
}

fn items(app: &CodexApp) -> Vec<Item> {
    app.active_thread()
        .transcript()
        .iter()
        .map(|(_, item)| item.clone())
        .collect()
}

#[test]
fn every_shape_makes_its_item_and_a_tab_reaches_the_item() {
    let app = play(&[
        r#"{"step":"entry","kind":"user","text":"the prompt"}"#,
        r#"{"step":"entry","kind":"agent","text":"a\tb"}"#,
        r#"{"step":"entry","kind":"reasoning","text":"the reasoning"}"#,
        r#"{"step":"entry","kind":"plan","text":"the plan"}"#,
        r#"{"step":"entry","kind":"plan_update","steps":[["Write it","completed"]],"explanation":"why"}"#,
        r#"{"step":"entry","kind":"exec","command":"false","output":"bad\n","exit":1}"#,
        r#"{"step":"entry","kind":"exec","command":"sleep 9","running":true}"#,
    ]);
    let items = items(&app);
    assert_eq!(items.len(), 7, "one item per entry: {items:#?}");
    assert!(matches!(&items[0], Item::User(prompt) if prompt.text == "the prompt"));
    assert_eq!(
        items[1],
        Item::Agent {
            text: "a\tb".to_owned(),
            phase: None
        },
        "the model's text is kept as it said it, tab included"
    );
    assert!(matches!(&items[2], Item::Reasoning { summary, .. } if summary == "the reasoning"));
    assert_eq!(
        items[3],
        Item::Plan {
            text: "the plan".to_owned()
        }
    );
    assert!(matches!(&items[4], Item::PlanUpdate(update) if update.steps.len() == 1));
    let shell = |item: &Item| match item {
        Item::Tool(tool) => matches!(tool.kind, ToolKind::Shell { .. }).then(|| tool.status),
        _ => None,
    };
    assert_eq!(shell(&items[5]), Some(ToolStatus::Failed));
    assert_eq!(shell(&items[6]), Some(ToolStatus::Running));
}

#[test]
fn an_entry_without_an_end_is_the_live_tail_and_a_commit_ends_its_turn() {
    let mut app = play(&[
        r#"{"step":"entry","kind":"agent","turn":"t","text":"first","end":false}"#,
        r#"{"step":"entry","kind":"agent","turn":"t","text":"second","start":false,"end":false}"#,
    ]);
    assert!(matches!(app.active_thread().turn(), TurnState::Running(_)));
    assert_eq!(items(&app).len(), 2, "both are on the transcript, live");
    let commit = parse(r#"{"step":"commit","turn":"t"}"#).expect("parses");
    commit.play(&mut app).expect("plays");
    assert!(
        !matches!(app.active_thread().turn(), TurnState::Running(_)),
        "the turn is over: its live items are committed"
    );
    assert_eq!(items(&app).len(), 2);
}

#[test]
fn a_bad_entry_names_its_line() {
    let err = parse(&format!("{SESSION}\n{}", r#"{"step":"entry","kind":"nonsense"}"#))
        .expect_err("fails");
    assert_eq!(err.line, 2);
    let err = parse(&format!(
        "{SESSION}\n{}",
        r#"{"step":"entry","kind":"plan_update","steps":[["only a step"]]}"#
    ))
    .expect_err("fails");
    assert_eq!(err.line, 2);
}
