//! The destination picker of `/export` and its filename prompt (ADR-013): two sheets over the chat,
//! the second opened by the first's "Save to file" row. In Flutter, a `showModalBottomSheet` of two
//! options that pops and shows a `TextField` sheet; the decision goes back as `Intent::Export`.

use std::rc::Rc;

use rtwk::overlay::{close_dialog, show_dialog};
use rtwk::{AnyWidget, Cx, Handler, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::features::export::{Destination, Intent};
use codex_tui_core::ids::ThreadId;

use super::strings;
use crate::features::prompt::{Suggest, TextPrompt};
use crate::features::verb;
use crate::widgets::picker::{Event, Picker};

/// The filename the prompt starts with, for the conversation in use (`codex-session-{thread}.md`,
/// `codex-session.md` before a thread exists): a filename, not prose, so the oracle's own.
fn filename(thread: Option<&ThreadId>) -> String {
    thread.map_or_else(
        || "codex-session.md".to_owned(),
        |thread| ["codex-session-", thread.as_str(), ".md"].concat(),
    )
}

/// `Export conversation`: `Copy to clipboard`, `Save to file`.
#[derive(Clone, Debug, PartialEq)]
pub struct Destinations {
    pub thread: Option<ThreadId>,
}

pub struct DestinationsState;

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Picked(Event),
}

impl StatefulWidget for Destinations {
    type State = DestinationsState;

    fn create_state(&self) -> DestinationsState {
        DestinationsState
    }
}

impl_widget!(stateful eq Destinations);

impl State for DestinationsState {
    type Widget = Destinations;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, w: &Destinations, cx: &mut Cx<'_, Self>) -> Rebuild {
        let Msg::Picked(event) = msg;
        match event {
            Event::Choose(id) if id == strings::CLIPBOARD => {
                verb(
                    cx,
                    Intent::Export {
                        destination: Destination::Clipboard,
                    },
                );
                close_dialog::<_, ()>(cx, None);
            }
            Event::Choose(id) if id == strings::FILE => {
                close_dialog::<_, ()>(cx, None);
                let thread = w.thread.clone();
                drop(show_dialog::<(), Self>(
                    cx,
                    move |_| {
                        FilePrompt {
                            thread: thread.clone(),
                        }
                        .into()
                    },
                    crate::sheets::sheet_options(),
                ));
            }
            Event::Cancel => close_dialog::<_, ()>(cx, None),
            _ => {}
        }
        Rebuild::No
    }

    fn build(&self, _w: &Destinations, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let on: Handler<Event> = cx.map(Msg::Picked);
        Picker::new(Rc::new(strings::menu()), on).into()
    }
}

/// The filename prompt (`Save conversation`).
#[derive(Clone, Debug, PartialEq)]
pub struct FilePrompt {
    pub thread: Option<ThreadId>,
}

pub struct FilePromptState;

#[derive(Clone, Debug, PartialEq)]
pub enum FileMsg {
    Submit(Rc<str>),
}

impl StatefulWidget for FilePrompt {
    type State = FilePromptState;

    fn create_state(&self) -> FilePromptState {
        FilePromptState
    }
}

impl_widget!(stateful eq FilePrompt);

impl State for FilePromptState {
    type Widget = FilePrompt;
    type Msg = FileMsg;

    /// The typed filename is the destination.
    fn update(&mut self, msg: FileMsg, _w: &FilePrompt, cx: &mut Cx<'_, Self>) -> Rebuild {
        let FileMsg::Submit(path) = msg;
        verb(
            cx,
            Intent::Export {
                destination: Destination::File(path.to_string()),
            },
        );
        Rebuild::No
    }

    fn build(&self, w: &FilePrompt, cx: &mut Cx<'_, Self>) -> AnyWidget {
        TextPrompt {
            title: strings::PROMPT_TITLE.to_owned(),
            placeholder: strings::PROMPT_PLACEHOLDER.to_owned(),
            initial: filename(w.thread.as_ref()),
            suggest: Suggest::Off,
            on_submit: cx.map(FileMsg::Submit),
        }
        .into()
    }
}
