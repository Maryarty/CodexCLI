//! The copy picker (oracle `chatwidget/copy_picker.rs` `show_copy_picker`): `/copy` over the last
//! response lists the whole response (as Markdown), then each code block and quote of its source
//! (as plain text), each with a one-line preview. What is picked goes back to the feature as
//! `CopyChoice`; a preview is cut at 72 graphemes and never changes what is copied.
//! Flutter's `showModalBottomSheet` over a list of options.

use std::rc::Rc;
use std::sync::Arc;

use rtwk::overlay::close_dialog;
use rtwk::{AnyWidget, Cx, Handler, Rebuild, State, StatefulWidget, impl_widget};
use rtwk_core::unicode::grapheme_boundaries;

use codex_tui_core::features::copy::{Response, strip_line_directives};
use codex_tui_core::screens::status::StatusCard;
use codex_tui_core::threads::queue::CopyChoice;

use super::strings::{
    BLOCKQUOTE, CODE_BLOCK, PICKER_TITLE, STATUS_DIRECTORY, STATUS_MODEL, STATUS_SESSION_ID,
    STATUS_THREAD_NAME, WHOLE_RESPONSE, WHOLE_STATUS, code_in,
};
use super::targets::{self, Target};
use crate::pages::session::cells::status::StatusCell;
use crate::widgets::picker::list::{Columns, Item, List};
use crate::widgets::picker::{Event, Picker};

/// The most graphemes of a row's preview.
const PREVIEW_GRAPHEMES: usize = 72;

#[derive(Clone, Debug, PartialEq)]
pub enum Source {
    Response(Response),
    Status(Arc<StatusCard>),
}

/// The picker over the copy source selected by the thread.
#[derive(Clone, Debug, PartialEq)]
pub struct CopyPicker {
    pub source: Source,
}

#[derive(Default)]
pub struct PickerState {
    choices: Vec<Choice>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Picked(Event),
}

/// One row: what it is called, what it copies and how.
struct Choice {
    label: String,
    text: String,
    markdown: bool,
}

/// The rows of the picker for `response` (`show_copy_picker`).
fn response_choices(response: &Response) -> Vec<Choice> {
    let mut choices = vec![Choice {
        label: WHOLE_RESPONSE.to_owned(),
        text: response.markdown.clone(),
        markdown: true,
    }];
    for target in targets::extract(&response.source) {
        match target {
            Target::Code { language, content } => choices.push(Choice {
                label: language
                    .map_or_else(|| CODE_BLOCK.to_owned(), |language| code_in(&language)),
                text: content,
                markdown: false,
            }),
            Target::Quote(content) => {
                let content: String = content
                    .split_inclusive('\n')
                    .map(strip_line_directives)
                    .collect();
                if !content.trim().is_empty() {
                    choices.push(Choice {
                        label: BLOCKQUOTE.to_owned(),
                        text: content,
                        markdown: false,
                    });
                }
            }
        }
    }
    choices
}

fn choices(source: &Source, cx: &mut Cx<'_, PickerState>) -> Vec<Choice> {
    let card = match source {
        Source::Status(card) => card,
        Source::Response(response) => return response_choices(response),
    };
    let whole = StatusCell::export_text(card, cx);
    [
        (WHOLE_STATUS, Some(whole.as_str())),
        (
            STATUS_MODEL,
            Some(card.model_id.as_deref().unwrap_or(&card.model)),
        ),
        (STATUS_DIRECTORY, Some(card.directory.as_str())),
        (STATUS_THREAD_NAME, card.thread_name()),
        (
            STATUS_SESSION_ID,
            card.session_id.as_deref().filter(|id| !id.is_empty()),
        ),
    ]
    .into_iter()
    .filter_map(|(label, text)| {
        Some(Choice {
            label: label.to_owned(),
            text: text?.to_owned(),
            markdown: false,
        })
    })
    .collect()
}

/// `truncate_text`: at most `max` graphemes, the last three dots when it is cut.
fn cut(text: &str, max: usize) -> String {
    let Some((head, _)) = grapheme_boundaries(text).nth(max) else {
        return text.to_owned();
    };
    let keep = grapheme_boundaries(text)
        .nth(max - 3)
        .map_or(head, |(start, _)| start);
    format!("{}...", &text[..keep])
}

/// The picker's list: a row per choice, its id the position.
fn list(choices: &[Choice]) -> List {
    let items = choices
        .iter()
        .enumerate()
        .map(|(index, choice)| Item {
            id: index.to_string(),
            name: choice.label.clone(),
            description: choice
                .text
                .lines()
                .find(|line| !line.trim().is_empty())
                .map(|line| cut(line.trim(), PREVIEW_GRAPHEMES)),
            ..Item::default()
        })
        .collect();
    List {
        title: Some(PICKER_TITLE.to_owned()),
        standard_hint: true,
        items,
        columns: Columns::AutoAllRows,
        hide_description_below: Some(24),
        panel: true,
        ..List::default()
    }
}

impl StatefulWidget for CopyPicker {
    type State = PickerState;

    fn create_state(&self) -> PickerState {
        PickerState::default()
    }
}

impl_widget!(stateful eq CopyPicker);

impl State for PickerState {
    type Widget = CopyPicker;
    type Msg = Msg;

    fn did_change_dependencies(&mut self, w: &CopyPicker, cx: &mut Cx<'_, Self>) {
        if self.choices.is_empty() {
            self.choices = choices(&w.source, cx);
        }
    }

    fn update(&mut self, msg: Msg, _w: &CopyPicker, cx: &mut Cx<'_, Self>) -> Rebuild {
        let Msg::Picked(event) = msg;
        let outcome = match event {
            Event::Choose(id) => {
                let picked = id
                    .parse::<usize>()
                    .ok()
                    .and_then(|index| self.choices.get(index));
                let Some(Choice {
                    label,
                    text,
                    markdown,
                }) = picked
                else {
                    return Rebuild::No;
                };
                Some(if *markdown {
                    CopyChoice::markdown(text.clone(), label.clone())
                } else {
                    CopyChoice::plain_text(text.clone(), label.clone())
                })
            }
            Event::Cancel => None,
            _ => return Rebuild::No,
        };
        close_dialog::<_, CopyChoice>(cx, outcome);
        Rebuild::No
    }

    fn build(&self, _w: &CopyPicker, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let on: Handler<Event> = cx.map(Msg::Picked);
        Picker::new(Rc::new(list(&self.choices)), on).into()
    }
}
