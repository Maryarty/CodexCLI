//! What the debug menu lists for the editor of the async questions: the states the oracle's
//! `async_questions/state_tests.rs` snapshots draw, each built here (the editor reads no
//! request: the questions are the widget's own).

use std::rc::Rc;

use rtwk::AnyWidget;

use super::{AQuestion, AsyncQuestions, Seed};
use crate::app::debug::{DebugEntry, EntryOpen};

const FIXTURE: &str = include_str!("../fixtures/async_questions.jsonl");

fn q(id: &str, title: &str, options: &[&str]) -> AQuestion {
    AQuestion {
        id: id.to_owned(),
        title: title.to_owned(),
        options: options.iter().map(|o| (*o).to_owned()).collect(),
    }
}

fn editor(questions: Vec<AQuestion>, seed: Seed) -> AnyWidget {
    AsyncQuestions {
        questions: Rc::new(questions),
        seed,
        queued: false,
        options_only: false,
        fit: false,
        saved: None,
        host: None,
        on_outcome: None,
    }
    .into()
}

/// The oracle's `editor()`: a freeform `First` and `Second` with the choice `Named`.
fn pair() -> Vec<AQuestion> {
    vec![q("q1", "First", &[]), q("q2", "Second", &["Named"])]
}

fn on(current: usize) -> Seed {
    Seed {
        current,
        ..Seed::default()
    }
}

macro_rules! entry {
    ($name:literal, $description:literal, $make:expr) => {
        DebugEntry {
            name: $name,
            description: $description,
            script: FIXTURE,
            open: EntryOpen::Route(|_| -> AnyWidget { $make }),
        }
    };
}

const WRAPPED: &str = "A suggested answer that is long enough to wrap across multiple rows";
const WRAPPED_FIRST: &str =
    "Another suggested answer that wraps over several rows before the selected option";
const URL: &str = "https://github.com/openai/codex/pull/12345?diff=split";

pub fn debug_entries() -> Vec<DebugEntry> {
    vec![
        entry!(
            "user_input_two_questions",
            "async questions: a freeform question and one with a choice, on the second",
            editor(pair(), on(1))
        ),
        entry!(
            "user_input_long_prompt",
            "async questions: a long prompt over a typed answer",
            {
                let mut questions = pair();
                questions[0].title = std::iter::repeat_n("A lengthy prompt. ", 30).collect();
                editor(
                    questions,
                    Seed {
                        drafts: vec!["visible answer".into()],
                        ..Seed::default()
                    },
                )
            }
        ),
        entry!(
            "user_input_wrapped_url",
            "async questions: a question whose URL wraps",
            {
                let mut questions = pair();
                questions[1].title = ["Review this change (", URL, ")?"].concat();
                editor(questions, on(1))
            }
        ),
        entry!(
            "user_input_markdown",
            "async questions: a Markdown question with a fence and a link",
            {
                let mut questions = pair();
                questions[0].title = "Run **this command**:\n\n```sh\nprintf '診断'\n```\nThen review the [diagnostics](https://example.com/diagnostics?view=full)?\n".into();
                editor(questions, Seed::default())
            }
        ),
        entry!(
            "user_input_other",
            "async questions: Other is the row on, its input empty, under a long question",
            {
                let mut questions = pair();
                questions[1].title = std::iter::repeat_n("Long question ", 40).collect();
                editor(
                    questions,
                    Seed {
                        current: 1,
                        selected: vec![None, Some(1)],
                        drafts: Vec::new(),
                    },
                )
            }
        ),
        entry!(
            "user_input_twelve",
            "async questions: twelve choices that wrap",
            {
                let mut questions = pair();
                let options: Vec<String> = (1..=12)
                    .map(|i| {
                        [
                            "Option ",
                            &i.to_string(),
                            " is long enough to wrap across rows",
                        ]
                        .concat()
                    })
                    .collect();
                questions[1].options = options;
                editor(questions, on(1))
            }
        ),
        entry!(
            "user_input_named_other",
            "async questions: a choice named Other beside the row Other",
            {
                let mut questions = pair();
                questions[1].options = vec!["Other".into()];
                editor(
                    questions,
                    Seed {
                        current: 1,
                        selected: vec![None, Some(0)],
                        drafts: Vec::new(),
                    },
                )
            }
        ),
        entry!(
            "user_input_wrap_boundary",
            "async questions: two choices exactly one column too wide",
            editor(
                vec![q(
                    "q2",
                    "Second",
                    &["abcdefgh *ijklmno*", "pqrstuvw xyzabcd"]
                )],
                Seed::default()
            )
        ),
        entry!(
            "user_input_other_clipped",
            "async questions: Other with no room for a whole choice (the choices part alone)",
            AsyncQuestions {
                questions: Rc::new(vec![q(
                    "q2",
                    "Second",
                    &["abcdefgh ijklmno", "pqrstuvw xyzabcd"]
                )]),
                seed: Seed {
                    current: 0,
                    selected: vec![Some(2)],
                    drafts: Vec::new(),
                },
                queued: false,
                options_only: true,
                fit: false,
                saved: None,
                host: None,
                on_outcome: None,
            }
            .into()
        ),
        entry!(
            "user_input_wrapped_option",
            "async questions: one wrapped choice",
            editor(vec![q("q2", "Second", &[WRAPPED])], Seed::default())
        ),
        entry!(
            "user_input_wrapped_options",
            "async questions: two wrapped choices, the second on",
            editor(
                vec![q("q2", "Second", &[WRAPPED_FIRST, WRAPPED])],
                Seed {
                    current: 0,
                    selected: vec![Some(1)],
                    drafts: Vec::new(),
                },
            )
        ),
        entry!(
            "user_input_clipped_prompt",
            "async questions: a question taller than any room",
            editor(
                vec![q(
                    "q2",
                    &std::iter::repeat_n("Long question ", 300).collect::<String>(),
                    &["x", "y"]
                )],
                Seed::default()
            )
        ),
        entry!(
            "user_input_wrapped_other",
            "async questions: Other holds an answer that wraps",
            editor(
                vec![q("q2", "Second", &[WRAPPED])],
                Seed {
                    current: 0,
                    selected: vec![Some(1)],
                    drafts: vec![
                        "An alternative answer that wraps across several option rows".into()
                    ],
                },
            )
        ),
        entry!(
            "user_input_capped_other",
            "async questions: Other holds a draft of a thousand lines",
            editor(
                vec![q("q2", "Second", &[WRAPPED])],
                Seed {
                    current: 0,
                    selected: vec![Some(1)],
                    drafts: vec![std::iter::repeat_n("long draft\n", 1000).collect()],
                },
            )
        ),
        entry!(
            "user_input_linked_option",
            "async questions: a choice with a link and emphasis",
            {
                let mut questions = pair();
                questions[1].options = vec!["*Named* [guide](https://example.com)".into()];
                editor(questions, on(1))
            }
        ),
    ]
}
