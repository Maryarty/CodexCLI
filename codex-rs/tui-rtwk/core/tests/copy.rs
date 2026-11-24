#![allow(clippy::expect_used, clippy::unwrap_used)]

//! `/copy` and Ctrl+O (oracle `chatwidget/copy_picker.rs`, `interaction.rs`
//! `prepare_last_response_copy`, `show_copy_result`): the verbs `Command`, `Last` and `Text`,
//! what a copy holds of the last response, the picker's wish and the host's answers as notes. The
//! picker's rows and the words are the UI's: these tests read data.

use codex_tui_core::app::effects::{Ask, CopyDestination, CopyFormat, CopyStatus, Done};
use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Intent, apply_app};
use codex_tui_core::commands::{SessionIntent, SlashCommand};
use codex_tui_core::features::copy::{Intent as CopyIntent, Note as CopyNote, Response, Wish};
use codex_tui_core::features::{FeatureIntent, FeatureNote, FeatureWish};
use codex_tui_core::fixtures::script::build;
use codex_tui_core::ids::ThreadId;
use codex_tui_core::screens::{ScreenIntent, chat};
use codex_tui_core::threads::{Item as Entry, Note, NoteKey};
use pretty_assertions::assert_eq;
use serde_json::json;

const T: &str = "thread-1";

fn line(value: serde_json::Value) -> String {
    json!({"step": "notify", "message": value}).to_string()
}

fn said(id: &str, text: &str) -> String {
    line(
        json!({"method": "item/completed", "params": {"threadId": T, "turnId": "turn-1",
        "completedAtMs": 0, "startedAtMs": 0,
        "item": {"type": "agentMessage", "id": id, "text": text, "phase": null}}}),
    )
}

/// An app whose thread has said `messages`.
fn app(messages: &[&str]) -> CodexApp {
    let mut script = vec![r#"{"step":"seed","seed":"session","thread":"thread-1"}"#.to_owned()];
    script.extend(
        messages
            .iter()
            .enumerate()
            .map(|(i, m)| said(&format!("a{i}"), m)),
    );
    build(&script.join("\n")).expect("plays").app
}

fn run(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    app.assert_consistent();
    fx
}

fn verb(app: &mut CodexApp, intent: CopyIntent) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Feature(FeatureIntent::Copier(intent))),
    )
}

fn command(app: &mut CodexApp) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Chat(chat::Intent::Session {
            thread: Some(ThreadId::new(T)),
            command: SlashCommand::Copy,
            intent: SessionIntent::Feature {
                args: String::new(),
            },
        }))),
    )
}

fn asks(fx: &AppFx) -> Vec<(u64, Ask)> {
    fx.commands()
        .iter()
        .filter_map(|c| match c {
            Command::Effect { request, ask, .. } => Some((*request, ask.clone())),
            _ => None,
        })
        .collect()
}

fn notes(app: &CodexApp) -> Vec<CopyNote> {
    app.active_thread()
        .transcript()
        .iter()
        .filter_map(|(_, item)| match item {
            Entry::Note(Note {
                key: NoteKey::Feature(FeatureNote::Copier(note)),
                ..
            }) => Some(note.clone()),
            _ => None,
        })
        .collect()
}

fn wishes(app: &CodexApp) -> Vec<FeatureWish> {
    app.wishes().pending().map(|w| w.wish.clone()).collect()
}

#[test]
fn ctrl_o_copies_the_last_response_as_markdown() {
    let mut app = app(&["first", "# Title\n\nbody"]);
    let fx = verb(&mut app, CopyIntent::Last);
    assert_eq!(
        asks(&fx)
            .into_iter()
            .map(|(_, ask)| ask)
            .collect::<Vec<_>>(),
        vec![Ask::Copy {
            text: "# Title\n\nbody".to_owned(),
            format: CopyFormat::Markdown,
            destination: CopyDestination::Clipboard,
        }]
    );
}

#[test]
fn without_a_response_both_verbs_say_so() {
    let mut app = app(&[]);
    assert!(asks(&verb(&mut app, CopyIntent::Last)).is_empty());
    assert!(asks(&command(&mut app)).is_empty());
    assert!(wishes(&app).is_empty());
    assert_eq!(
        notes(&app),
        vec![CopyNote::NoResponse, CopyNote::NoResponse]
    );
}

#[test]
fn a_response_with_nothing_visible_is_no_response() {
    let mut app = app(&["real", "::git-stage{cwd=\"/repo\"}"]);
    let fx = verb(&mut app, CopyIntent::Last);
    let binding = asks(&fx);
    let [(_, Ask::Copy { text, .. })] = binding.as_slice() else {
        panic!("one copy: {fx:?}");
    };
    assert_eq!(
        text, "real",
        "a message of directives only is skipped, as the oracle's record"
    );
}

#[test]
fn a_copy_holds_the_visible_markdown_and_the_picker_the_source_too() {
    let source =
        "Done\n\n::git-stage{cwd=\"/repo\"}\n\nNext :codex-followup[go on]{prompt=\"more\"}";
    let mut app = app(&[source]);
    command(&mut app);
    assert_eq!(
        wishes(&app),
        vec![FeatureWish::from(Wish::Picker {
            thread: ThreadId::new(T),
            response: Response {
                markdown: "Done\n\n\n\nNext go on".to_owned(),
                source: source.to_owned(),
            },
        })]
    );
}

#[test]
fn a_picker_row_copies_its_text_in_its_format_under_its_label() {
    let mut app = app(&["x"]);
    let fx = verb(
        &mut app,
        CopyIntent::Text {
            text: "fn main() {}".to_owned(),
            label: "rust code".to_owned(),
            format: CopyFormat::PlainText,
        },
    );
    let asks = asks(&fx);
    assert_eq!(asks.len(), 1);
    assert_eq!(
        asks[0].1,
        Ask::Copy {
            text: "fn main() {}".to_owned(),
            format: CopyFormat::PlainText,
            destination: CopyDestination::Clipboard,
        }
    );
}

#[test]
fn the_hosts_answers_are_notes_under_the_label_that_was_asked() {
    let mut app = app(&["x"]);
    for (done, label) in [
        (Done::Copy(Ok(CopyStatus::Pending(3))), "last message"),
        (Done::Copy(Ok(CopyStatus::Confirmed)), "last message"),
        (Done::Copy(Err("no clipboard".to_owned())), "Blockquote"),
    ] {
        let fx = verb(
            &mut app,
            CopyIntent::Text {
                text: "t".to_owned(),
                label: label.to_owned(),
                format: CopyFormat::PlainText,
            },
        );
        let request = asks(&fx)[0].0;
        run(&mut app, AppEv::Effect { request, done });
    }
    assert_eq!(
        notes(&app),
        vec![
            CopyNote::Copied {
                label: "last message".to_owned(),
                result: Ok(CopyStatus::Pending(3)),
            },
            CopyNote::Copied {
                label: "last message".to_owned(),
                result: Ok(CopyStatus::Confirmed),
            },
            CopyNote::Copied {
                label: "Blockquote".to_owned(),
                result: Err("no clipboard".to_owned()),
            },
        ]
    );
}
