//! `pets`: the view half of S24, the `/pets` picker, as a page drawn without a
//! frame, as the oracle draws its pane: a title, a subtitle, a search field,
//! the list with a `›` on the cursor row and the preview status to its right,
//! a footer. It reads the picker the model holds (`core::screens::pets`)
//! through a provider projection and returns what the user does as intents.
//! Oracle: `pets/picker.rs` (`build_pet_picker_params`), `pets/preview.rs`.

use std::rc::Rc;

use rtwk::NotificationListener;
use rtwk::controls::{SelectionController, SelectionList};
use rtwk::editing::{TextEditingController, TextField};
use rtwk::focus::{Focus, KeyPress};
use rtwk::foundation::{ListenableBuilder, ValueListenableBuilder, ValueNotifier};
use rtwk::layout::{Column, CrossAxisAlignment, Expanded, MainAxisSize, Row, SizedBox};
use rtwk::navigation::Navigator;
use rtwk::provider::{read, select};
use rtwk::scroll::ScrollNotification;
use rtwk::style::Style;
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

/// Widget state: rtwk's controllers (R14): the cursor row (a key into the
/// picker's pets), the search text, and whether rows lie below the window.
pub struct ScreenState {
    shown: bool,
    cursor: SelectionController<usize>,
    query: TextEditingController,
    below: Rc<ValueNotifier<bool>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Move(i32),
    Search,
    Pick,
    Close,
}

impl StatefulWidget for Screen {
    type State = ScreenState;

    fn create_state(&self) -> ScreenState {
        ScreenState {
            shown: false,
            cursor: SelectionController::new(),
            query: TextEditingController::default(),
            below: Rc::new(ValueNotifier::new(false)),
        }
    }
}

impl_widget!(stateful eq Screen);

impl ScreenState {
    /// The rows the search leaves, as indices into the picker's pets.
    fn visible(&self, picker: &Picker) -> Vec<usize> {
        picker.visible(&self.query.text())
    }

    /// Puts the cursor on `index` and tells the model which row that is.
    fn put(&self, picker: &Picker, index: usize, cx: &mut Cx<'_, Self>) {
        self.cursor.select(index);
        self.cursor.set_cursor(Some(index));
        if let Some(pet) = picker.pets.get(index) {
            let hs = read::<Hs, _>(cx);
            cx.emit(
                &hs.screen,
                ScreenIntent::Pets(Intent::Highlight(pet.selector.clone())),
            );
        }
    }
}

impl State for ScreenState {
    type Widget = Screen;
    type Msg = Msg;

    fn did_change_dependencies(&mut self, _: &Screen, cx: &mut Cx<'_, Self>) {
        let picker = select::<ScreenStates, Option<Picker>, _>(cx, |states| states.pets.open.clone());
        if picker.is_none() && self.shown {
            Navigator::maybe_pop(cx);
        }
        if let (Some(picker), false) = (&picker, self.shown) {
            // Opened: the cursor starts on the configured pet.
            let initial = picker.initial();
            self.cursor.select(initial);
            self.cursor.set_cursor(Some(initial));
            self.below.set_value(picker.pets.len() > WINDOW);
        }
        self.shown = picker.is_some();
    }

    fn update(&mut self, msg: Msg, _: &Screen, cx: &mut Cx<'_, Self>) -> Rebuild {
        let Some(picker) =
            select::<ScreenStates, Option<Picker>, _>(cx, |states| states.pets.open.clone())
        else {
            return Rebuild::No;
        };
        let visible = self.visible(&picker);
        let at = self
            .cursor
            .cursor()
            .and_then(|key| visible.iter().position(|index| *index == key));
        let hs = read::<Hs, _>(cx);
        match msg {
            Msg::Move(delta) => {
                if !visible.is_empty() {
                    let len = i64::try_from(visible.len()).unwrap_or(1);
                    let from = i64::try_from(at.unwrap_or(0)).unwrap_or(0);
                    let to = usize::try_from((from + i64::from(delta)).rem_euclid(len)).unwrap_or(0);
                    self.put(&picker, visible[to], cx);
                }
            }
            // The search changed: the cursor goes back to the first row.
            Msg::Search => {
                if let Some(first) = visible.first() {
                    self.put(&picker, *first, cx);
                }
            }
            Msg::Pick => {
                if let Some(pet) = at.and_then(|at| picker.pets.get(visible[at])) {
                    cx.emit(&hs.screen, ScreenIntent::Pets(Intent::Pick(pet.selector.clone())));
                }
            }
            // Esc clears a search typed, then closes.
            Msg::Close => {
                if self.query.text().trim().is_empty() && self.query.text().is_empty() {
                    cx.emit(&hs.screen, ScreenIntent::Pets(Intent::Close));
                } else {
                    self.query.clear();
                    if let Some(first) = picker.visible("").first() {
                        self.put(&picker, *first, cx);
                    }
                }
            }
        }
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
        let picker = Rc::new(picker);
        let (cursor, below) = (self.cursor.clone(), self.below.clone());
        // The list and the "more below" row follow the search text.
        let list = {
            let (picker, tokens) = (picker.clone(), tokens.clone());
            ListenableBuilder::new(self.query.listenable(), {
                let query = self.query.clone();
                move |_, _| {
                    let visible = Rc::new(picker.visible(&query.text()));
                    below.set_value(visible.len() > WINDOW);
                    let controller = cursor.clone();
                    let rows = {
                        let (picker, visible, cursor, tokens) =
                            (picker.clone(), visible.clone(), cursor.clone(), tokens.clone());
                        SelectionList::new(
                            visible.len(),
                            {
                                let visible = visible.clone();
                                move |row| visible[row]
                            },
                            move |row: usize| -> AnyWidget {
                                let index = visible[row];
                                let (picker, cursor, tokens) =
                                    (picker.clone(), cursor.clone(), tokens.clone());
                                ListenableBuilder::new(cursor.listenable(), {
                                    move |_, _| {
                                        let pet = &picker.pets[index];
                                        let name = if pet.selector == DISABLED {
                                            s.disable_row
                                        } else {
                                            pet.name.as_str()
                                        };
                                        let marker =
                                            if picker.is_current(pet) { s.current_marker } else { "" };
                                        let on = cursor.cursor() == Some(index);
                                        let text = Text::line(format!(
                                            "{}{name}{marker}",
                                            if on { s.cursor_marker } else { s.indent }
                                        ));
                                        if on {
                                            text.style(tokens.selected()).into()
                                        } else {
                                            text.into()
                                        }
                                    }
                                })
                                .into()
                            },
                        )
                        .controller(controller)
                        .item_extent(1)
                        .bar(false)
                        .idle_style(Style::new())
                        .selected_style(Style::new())
                        .cursor_style(Style::new())
                    };
                    let scroll = below.clone();
                    let rows = NotificationListener::<ScrollNotification>::new(rows).on_notification(
                        move |note| {
                            scroll.set_value(note.metrics.pixels < note.metrics.max_scroll_extent);
                            false
                        },
                    );
                    SizedBox::new(None, Some(i32::try_from(visible.len().min(WINDOW)).unwrap_or(1)), Some(rows.into()))
                        .into()
                }
            })
        };
        let body = Row::new()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .child(Expanded::new(1, list))
            .child(SizedBox::new(
                Some(PREVIEW_WIDTH),
                Some(PREVIEW_HEIGHT),
                Some(preview(&picker, s, &tokens)),
            ))
            .child(SizedBox::new(Some(RIGHT_MARGIN), None, None));
        let more = {
            let style = tokens.dim();
            ValueListenableBuilder::new(self.below.clone(), move |_, below: &bool, _| {
                if *below {
                    Text::line(s.more_below.to_string()).style(style.clone()).into()
                } else {
                    AnyWidget::default()
                }
            })
        };
        let column = Column::new()
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
                        TextField::new()
                            .controller(self.query.clone())
                            .placeholder(s.search_placeholder)
                            .autofocus(true)
                            .on_changed(cx.map(|_: Rc<str>| Msg::Search))
                            .on_submitted(cx.map(|_: Rc<str>| Msg::Pick)),
                    )),
            )
            .child(Text::new(String::new()))
            .child(body)
            .child(more)
            .child(Text::line(format!("{}{}", s.indent, s.hint)).style(tokens.dim()));
        Focus::new(column)
            .can_request_focus(false)
            .bind(KeyPress::ESC, cx.send(Msg::Close))
            .bind(KeyPress::UP, cx.send(Msg::Move(-1)))
            .bind(KeyPress::DOWN, cx.send(Msg::Move(1)))
            .into()
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
