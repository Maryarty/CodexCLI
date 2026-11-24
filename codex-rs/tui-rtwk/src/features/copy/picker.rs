//! The copy picker (oracle `chatwidget/copy_picker.rs` `show_copy_picker`): `/copy` over the last
//! response lists the whole response (as Markdown), then each code block and quote of its source
//! (as plain text), each with a one-line preview. What is picked goes back to the feature as
//! `Intent::Text`; a preview is cut at 72 graphemes and never changes what is copied.
//! Flutter's `showModalBottomSheet` over a list of options.

use std::rc::Rc;

use rtwk::overlay::close_dialog;
use rtwk::{AnyWidget, Cx, Handler, Rebuild, State, StatefulWidget, impl_widget};
use rtwk_core::unicode::grapheme_boundaries;

use codex_tui_core::app::effects::CopyFormat;
use codex_tui_core::features::copy::{Intent, Response, strip_line_directives};

use super::strings::PICKER_TITLE;
use super::targets::{self, Target};
use crate::features::verb;
use crate::widgets::picker::list::{Columns, Item, List};
use crate::widgets::picker::{Event, Picker};

/// The most graphemes of a row's preview.
const PREVIEW_GRAPHEMES: usize = 72;

/// The picker over `response`.
#[derive(Clone, Debug, PartialEq)]
pub struct CopyPicker {
    pub response: Response,
}

pub struct PickerState;

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Picked(Event),
}

/// One row: what it is called, what it copies and how.
struct Choice {
    label: String,
    text: String,
    format: CopyFormat,
}

/// The rows of the picker for `response` (`show_copy_picker`).
fn choices(response: &Response) -> Vec<Choice> {
    let mut choices = vec![Choice {
        label: "Whole response".to_owned(),
        text: response.markdown.clone(),
        format: CopyFormat::Markdown,
    }];
    for target in targets::extract(&response.source) {
        match target {
            Target::Code { language, content } => choices.push(Choice {
                label: language.map_or_else(
                    || "Code block".to_owned(),
                    |language| format!("{language} code"),
                ),
                text: content,
                format: CopyFormat::PlainText,
            }),
            Target::Quote(content) => {
                let content: String = content
                    .split_inclusive('\n')
                    .map(strip_line_directives)
                    .collect();
                if !content.trim().is_empty() {
                    choices.push(Choice {
                        label: "Blockquote".to_owned(),
                        text: content,
                        format: CopyFormat::PlainText,
                    });
                }
            }
        }
    }
    choices
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
        PickerState
    }
}

impl_widget!(stateful eq CopyPicker);

impl State for PickerState {
    type Widget = CopyPicker;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, w: &CopyPicker, cx: &mut Cx<'_, Self>) -> Rebuild {
        let Msg::Picked(event) = msg;
        match event {
            Event::Choose(id) => {
                let picked = id
                    .parse::<usize>()
                    .ok()
                    .and_then(|index| choices(&w.response).into_iter().nth(index));
                let Some(Choice {
                    label,
                    text,
                    format,
                }) = picked
                else {
                    return Rebuild::No;
                };
                verb(
                    cx,
                    Intent::Text {
                        text,
                        label,
                        format,
                    },
                );
            }
            Event::Cancel => {}
            _ => return Rebuild::No,
        }
        close_dialog::<_, ()>(cx, None);
        Rebuild::No
    }

    fn build(&self, w: &CopyPicker, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let on: Handler<Event> = cx.map(Msg::Picked);
        Picker::new(Rc::new(list(&choices(&w.response))), on).into()
    }
}
