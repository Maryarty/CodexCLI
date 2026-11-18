//! `pets`: the view half of S24, the `/pets` picker, as a page drawn without a
//! frame, as the oracle draws its pane: a title, a subtitle, a search field,
//! the list with a `›` on the cursor row and the preview status to its right,
//! a footer. It reads the picker the model holds (`core::screens::pets`)
//! through a provider projection and returns what the user does as intents.
//! Oracle: `pets/picker.rs` (`build_pet_picker_params`), `pets/preview.rs`.

use std::cell::Cell;

use rtwk::editing::TextField;
use rtwk::focus::{Focus, KeyPress};
use rtwk::layout::{Column, CrossAxisAlignment, Expanded, MainAxisSize, Row, SizedBox};
use rtwk::navigation::Navigator;
use rtwk::provider::{read, select};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::ScreenStates;
use codex_tui_core::screens::pets::{DISABLED, ID, Intent, NAME, Picker, Preview};

use crate::screens::DebugEntry;
use crate::screens::DebugOpen;
use crate::screens::EntryOpen;
use crate::theme::Tokens;
use crate::ui::view::Hs;

pub mod strings;

/// Rows of the list shown at once (`max_visible_rows` of the picker).
const WINDOW: usize = 8;
/// The preview area's width (`PET_PICKER_PREVIEW_WIDTH`).
const PREVIEW_WIDTH: i32 = 30;
/// The blank columns at the right edge of the pane.
const RIGHT_MARGIN: i32 = 2;
/// The preview area's height (`PetPickerPreviewRenderable::desired_height`).
const PREVIEW_HEIGHT: i32 = 4;

/// What the debug menu lists for this directory: the picker on three
/// configurations, each made by a fixture script.
pub fn debug_entries() -> Vec<DebugEntry> {
    let entries = [
        (
            "picker",
            "the picker, no pet configured",
            include_str!("fixtures/picker.jsonl"),
        ),
        (
            "picker_custom",
            "the picker, a custom pet configured",
            include_str!("fixtures/picker_custom.jsonl"),
        ),
        (
            "picker_disabled",
            "the picker, pets disabled",
            include_str!("fixtures/picker_disabled.jsonl"),
        ),
    ];
    entries
        .into_iter()
        .map(|(name, description, script)| DebugEntry {
            name,
            description,
            script,
            open: EntryOpen::Route(|_| Screen.into()),
        })
        .collect()
}

/// The part as a widget, placed by the chat screen's subtree.
pub fn part() -> AnyWidget {
    Screen.into()
}

/// Opens the page with no seeded model (debug menu): nothing to pick from.
pub fn open_empty() -> DebugOpen {
    DebugOpen::Route(|_| Screen.into())
}

/// The picker page.
#[derive(Clone, Debug, PartialEq)]
pub struct Screen;

/// Widget state: ids and UI-only positions (R14): whether a picker was shown
/// (to leave when the model drops it), which search field is current, and the
/// first row of the window.
#[derive(Debug)]
pub struct ScreenState {
    shown: bool,
    field: u32,
    top: Cell<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Move(i32),
    Search(String),
    Pick,
    Close,
}

impl StatefulWidget for Screen {
    type State = ScreenState;

    fn create_state(&self) -> ScreenState {
        ScreenState {
            shown: false,
            field: 0,
            top: Cell::new(0),
        }
    }
}

impl_widget!(stateful eq Screen);

impl State for ScreenState {
    type Widget = Screen;
    type Msg = Msg;

    fn did_change_dependencies(&mut self, _: &Screen, cx: &mut Cx<'_, Self>) {
        let open = select::<ScreenStates, bool, _>(cx, |states| states.pets.open.is_some());
        if !open && self.shown {
            Navigator::maybe_pop(cx);
        }
        self.shown = open;
    }

    fn update(&mut self, msg: Msg, _: &Screen, cx: &mut Cx<'_, Self>) -> Rebuild {
        let hs = read::<Hs, _>(cx);
        let intent = match msg {
            Msg::Move(delta) => Intent::Move(delta),
            Msg::Search(text) => Intent::Search(text),
            Msg::Pick => Intent::Pick,
            Msg::Close => {
                // A search typed is cleared by Esc: its field starts afresh.
                let typed =
                    select::<ScreenStates, bool, _>(cx, |states| {
                        states.pets.open.as_ref().is_some_and(|p| !p.query.is_empty())
                    });
                if typed {
                    self.field += 1;
                }
                Intent::Close
            }
        };
        cx.emit(&hs.screen, ScreenIntent::Pets(intent));
        Rebuild::Yes
    }

    fn build(&self, _: &Screen, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let picker = select::<ScreenStates, Option<Picker>, _>(cx, |states| {
            states.pets.open.clone()
        });
        let s = strings::of(cx);
        let tokens = Tokens::of(cx);
        let Some(picker) = picker else {
            // No picker seeded (the debug menu's `pets` row): the skeleton's
            // empty-slot line, as before the slot was filled.
            let shared = crate::l10n::strings::of(cx);
            return Text::new(shared.empty_slot(NAME, ID).to_string()).into();
        };
        let visible = picker.visible();
        let top = window_top(self.top.get(), picker.cursor, visible.len());
        self.top.set(top);
        let shown: Vec<AnyWidget> = visible
            .iter()
            .enumerate()
            .skip(top)
            .take(WINDOW)
            .map(|(row, index)| {
                let pet = &picker.pets[*index];
                let name = if pet.selector == DISABLED {
                    s.disable_row
                } else {
                    pet.name.as_str()
                };
                let marker = if picker.is_current(pet) {
                    s.current_marker
                } else {
                    ""
                };
                let at_cursor = row == picker.cursor;
                let text = format!(
                    "{}{name}{marker}",
                    if at_cursor { s.cursor_marker } else { s.indent }
                );
                let text = Text::line(text);
                if at_cursor {
                    text.style(tokens.selected()).into()
                } else {
                    text.into()
                }
            })
            .collect();
        let mut list = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Start);
        for row in shown {
            list = list.child(row);
        }
        let body = Row::new()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .child(Expanded::new(1, list))
            .child(SizedBox::new(
                Some(PREVIEW_WIDTH),
                Some(PREVIEW_HEIGHT),
                Some(preview(&picker, s, &tokens)),
            ))
            .child(SizedBox::new(Some(RIGHT_MARGIN), None, None));
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .child(Text::line(format!("{}{}", s.indent, s.picker_title)).style(tokens.bold()))
            .child(Text::line(format!("{}{}", s.indent, s.picker_subtitle)).style(tokens.dim()))
            .child(Text::new(String::new()))
            .child(
                Row::new()
                    .child(SizedBox::new(Some(2), None, None))
                    .child(Expanded::new(
                        1,
                        AnyWidget::from(
                            TextField::new()
                                .placeholder(s.search_placeholder)
                                .autofocus(true)
                                .on_changed(cx.map(|text: std::rc::Rc<str>| {
                                    Msg::Search(text.to_string())
                                }))
                                .on_submitted(cx.map(|_: std::rc::Rc<str>| Msg::Pick)),
                        )
                        .with_key(self.field),
                    )),
            )
            .child(Text::new(String::new()))
            .child(body);
        if top + WINDOW < visible.len() {
            column = column.child(Text::line(s.more_below.to_string()).style(tokens.dim()));
        }
        let column = column.child(Text::line(format!("{}{}", s.indent, s.hint)).style(tokens.dim()));
        Focus::new(column)
            .can_request_focus(false)
            .bind(KeyPress::ESC, cx.send(Msg::Close))
            .bind(KeyPress::UP, cx.send(Msg::Move(-1)))
            .bind(KeyPress::DOWN, cx.send(Msg::Move(1)))
            .into()
    }
}

/// The first row of the window: the old one while the cursor is in it, else
/// the nearest that shows the cursor.
fn window_top(top: usize, cursor: usize, len: usize) -> usize {
    let top = top.min(len.saturating_sub(WINDOW));
    if cursor < top {
        cursor
    } else if cursor >= top + WINDOW {
        cursor + 1 - WINDOW
    } else {
        top
    }
}

/// The preview area: its status, centred (`PetPickerPreviewRenderable`).
fn preview(picker: &Picker, s: &strings::Strings, tokens: &Tokens) -> AnyWidget {
    let Some((_, Preview::Loading)) = &picker.preview else {
        return AnyWidget::default();
    };
    let text = s.preview_loading;
    let left = usize::try_from(PREVIEW_WIDTH).unwrap_or(0).saturating_sub(text.chars().count()) / 2;
    Column::new()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .child(Text::new(String::new()))
        .child(
            Row::new()
                .child(SizedBox::new(Some(i32::try_from(left).unwrap_or(0)), None, None))
                .child(Text::line(text.to_string()).style(tokens.bold())),
        )
        .into()
}
