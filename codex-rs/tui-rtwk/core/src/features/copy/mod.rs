//! `/copy` and Ctrl+O as a feature slice (ADR-012; oracle `chatwidget/copy_picker.rs`
//! `show_copy_picker`, `interaction.rs` `prepare_last_response_copy`, `app/clipboard.rs`,
//! `event_dispatch.rs` `CopySelection`): the last assistant response, copied as Markdown, or one
//! piece of it (a code block, a quote) the picker offered. The picker is the UI half's
//! (`src/features/copy/`): the targets are cut out of the Markdown it reads from the wish
//! (`Wish::Picker`), and what the user picked comes back as `Intent::Text`. Ctrl+O is
//! `Intent::Last` and copies the whole response without the picker.
//!
//! What a copy holds is the visible Markdown of the last message (`visible`): the directives
//! the Codex App reads are cut out, follow-ups are their labels. Both it and the raw source the
//! targets are cut from are in the wish. The host's answer is worded as a note of the thread in
//! use: a copy answers twice (pending, then how it ended) and says both.

mod visible;

use super::{AppCx, AppView, Slash, ThreadView};
use crate::app::effects::{Ask, CopyDestination, CopyFormat, CopyStatus, Done};
use crate::commands::SlashCommand;
use crate::ids::ThreadId;
use crate::threads::{Item, Tone};

pub use visible::{copy_markdown, strip_line_directives, visible_markdown};

/// The marker type of the feature.
pub struct Copier;

/// The last response of a thread, as a copy holds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Response {
    /// `last_agent_markdown`: the visible Markdown, follow-ups as their labels.
    pub markdown: String,
    /// `last_agent_source`: the message as the server sent it, the targets are cut from it.
    pub source: String,
}

/// What the user asks of `/copy`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// `/copy`, typed in the thread `source`: the picker is asked of the surface; where there
    /// is no response to copy it is the error line.
    Command { source: Option<ThreadId> },
    /// Copy mode could not begin over the owned transcript of `thread` (Find or another mode of
    /// input holds the keys): the view says so, the oracle's `Nothing to copy`.
    Nothing { thread: ThreadId },
    /// Ctrl+O: the whole last response, copied as Markdown (label `last message`).
    Last,
    /// A row of the picker (oracle `AppEvent::CopySelection`): `text` goes to the clipboard in
    /// `format`, and the note says `label` was copied.
    Text {
        text: String,
        label: String,
        format: CopyFormat,
    },
}

/// What an ask of the host was for: the copy of `label`, as the note names it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ctx {
    pub label: String,
}

/// What the surface is asked to show once (`Scope::Thread`): the copy picker (`Copy to
/// clipboard`), answered by `Intent::Text`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Wish {
    Picker {
        thread: ThreadId,
        response: Response,
    },
    /// Copy mode over the chat's own transcript (`show_copy_picker` where the transcript is
    /// owned and no status is copyable): there is no sheet, and no response is needed, a user
    /// message is a target too.
    Mode { thread: ThreadId },
}

/// What `/copy` says in the transcript.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Note {
    /// `No agent response to copy` (an error line).
    NoResponse,
    /// `Nothing to copy` (an info line): copy mode could not begin.
    NothingToCopy,
    /// How the copy of `label` went: the delivery status, or the clipboard's failure
    /// (`show_copy_result`: `Copied {label} to clipboard`, `Copy failed: {error}`).
    Copied {
        label: String,
        result: Result<CopyStatus, String>,
    },
}

/// The messages a copy can take, oldest first: an assistant message or a plan, as the server
/// sent it (oracle `record_agent_markdown`'s callers).
pub fn response_sources<'a>(thread: ThreadView<'a>) -> std::vec::IntoIter<&'a str> {
    thread
        .core()
        .transcript()
        .iter()
        .filter_map(|(_, item)| match item {
            Item::Agent { text, .. } | Item::Plan { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .into_iter()
}

/// The label Ctrl+O copies under.
pub const LAST_MESSAGE: &str = "last message";

/// The last response of `thread`, if it has one: the last message (an assistant message or a
/// plan) with something visible in it (`record_agent_markdown`).
pub fn last_response(view: &AppView<'_>, thread: &ThreadId) -> Option<Response> {
    let cwd = view.working_directory(Some(thread));
    response_sources(view.thread(thread)?)
        .rev()
        .find_map(|source| {
            copy_markdown(source, &cwd).map(|markdown| Response {
                markdown,
                source: source.to_owned(),
            })
        })
}

fn copy(cx: &mut AppCx<'_>, text: String, label: String, format: CopyFormat) {
    let ask = Ask::Copy {
        text,
        format,
        destination: CopyDestination::Clipboard,
    };
    cx.effect(ask, Ctx { label });
}

feature! {
    impl Feature for Copier {
        const NAME: &'static str = "copy";
        const RUNS: &'static [SlashCommand] = &[SlashCommand::Copy];
        type Intent = Intent;
        type Ctx = Ctx;
        type Note = Note;
        type Wish = Wish;
        {
            /// `/copy`: the picker of what the last response holds, asked of the surface.
            fn slash(state: &mut (), slash: Slash, cx: &mut AppCx<'_>) {
                Self::intent(
                    state,
                    Intent::Command {
                        source: slash.source,
                    },
                    cx,
                );
            }

            fn intent(_state: &mut (), intent: Intent, cx: &mut AppCx<'_>) {
                match intent {
                    Intent::Command { source } => {
                        let Some(thread) = source.filter(|id| cx.view().thread(id).is_some()) else {
                            return cx.note(None, Note::NoResponse);
                        };
                        // the oracle's owned transcript begins copy mode whatever it holds
                        if cx.view().facts().tui().transcript_owned {
                            return cx.wish(thread.clone(), Wish::Mode { thread });
                        }
                        match last_response(&cx.view(), &thread) {
                            Some(response) => cx.wish(thread.clone(), Wish::Picker { thread, response }),
                            None => cx.note(Some(thread), Note::NoResponse),
                        }
                    }
                    Intent::Nothing { thread } => cx.note(Some(thread), Note::NothingToCopy),
                    Intent::Last => {
                        let thread = cx.view().active().cloned();
                        let response = thread
                            .as_ref()
                            .and_then(|thread| last_response(&cx.view(), thread));
                        match response {
                            Some(response) => copy(
                                cx,
                                response.markdown,
                                LAST_MESSAGE.to_owned(),
                                CopyFormat::Markdown,
                            ),
                            None => cx.note(thread, Note::NoResponse),
                        }
                    }
                    Intent::Text {
                        text,
                        label,
                        format,
                    } => copy(cx, text, label, format),
                }
            }

            /// The host's answer to a copy, said in the thread in use.
            fn effect_done(_state: &mut (), ctx: Ctx, done: Done, cx: &mut AppCx<'_>) {
                if let Done::Copy(result) = done {
                    cx.note(
                        None,
                        Note::Copied {
                            label: ctx.label,
                            result,
                        },
                    );
                }
            }

            fn note_tone(note: &Note) -> Tone {
                match note {
                    Note::NoResponse | Note::Copied { result: Err(_), .. } => Tone::Error,
                    Note::NothingToCopy | Note::Copied { result: Ok(_), .. } => Tone::Info,
                }
            }
        }
    }
}
