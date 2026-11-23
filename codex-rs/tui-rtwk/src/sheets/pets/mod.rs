//! `pets`: the view half of S24, the `/pets` picker, as a page drawn without a
//! frame, as the oracle draws its pane: a title, a subtitle, a search field,
//! the list with a `›` on the cursor row and the preview status to its right,
//! a footer. It reads the picker the model holds (`core::screens::pets`)
//! through a provider projection and returns what the user does as intents.
//! Oracle: `pets/picker.rs` (`build_pet_picker_params`), `pets/preview.rs`.

use std::rc::Rc;

use rtwk::NotificationListener;
use rtwk::controls::SelectionController;
use rtwk::editing::{TextEditingController, TextField};
use rtwk::focus::Focus;
use rtwk::foundation::{ListenableBuilder, ValueListenableBuilder, ValueNotifier};
use rtwk::gestures::{GestureDetector, PointerEvent};
use rtwk::layout::{
    Align, Alignment, Column, CrossAxisAlignment, Expanded, MainAxisSize, Row, SizedBox,
};
use rtwk::overlay::{Dialog, DialogOptions, DialogResult, close_dialog, show_dialog};
use rtwk::provider::{read, select};
use rtwk::scroll::{ListController, ListView, ScrollNotification};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, Handler, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::keymap::fixed;
use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::ScreenStates;
use codex_tui_core::screens::pets::{DISABLED, ID, Intent, NAME, Picker, Preview};

use crate::app::debug::DebugEntry;
use crate::app::debug::DebugOpen;
use crate::app::debug::EntryOpen;
use crate::app::theme::Tokens;
use crate::app::view::Hs;
use crate::keymap::bind_fixed;

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
            open: EntryOpen::Dialog(|_| Screen.into()),
        })
        .collect()
}

/// Opens the picker as a modal sheet over the session page: no frame, no
/// dimming, at the bottom edge, as the oracle draws it in place of the
/// composer (architecture §1, amendment 2026-11-19b).
pub fn open<S>(cx: &mut Cx<'_, S>) -> DialogResult<()> {
    show_dialog(
        cx,
        move |_| Screen.into(),
        // Esc is the picker's own key when a picker is up (it clears a
        // search first); the barrier closes the unseeded stub.
        DialogOptions {
            barrier_dismissible: true,
            ..crate::sheets::sheet_options()
        },
    )
}

/// Opens the sheet with no seeded model (debug menu): nothing to pick from.
pub fn open_empty() -> DebugOpen {
    DebugOpen::Dialog(|_| Screen.into())
}

/// The picker sheet.
#[derive(Clone, Debug, PartialEq)]
pub struct Screen;

/// Widget state: rtwk's controllers (R14): the cursor row (a key into the
/// picker's pets), the search text, and whether rows lie below the window.
pub struct ScreenState {
    /// The catalog was seen: the cursor started on its configured pet.
    started: bool,
    cursor: SelectionController<usize>,
    list: ListController,
    query: TextEditingController,
    below: Rc<ValueNotifier<bool>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Move(i32),
    /// A tap on the pet at an index: the cursor goes there, a tap on the
    /// pet it is on picks it.
    Row(usize),
    Search,
    Pick,
    Close,
}

impl StatefulWidget for Screen {
    type State = ScreenState;

    fn create_state(&self) -> ScreenState {
        ScreenState {
            started: false,
            cursor: SelectionController::new(),
            list: ListController::new(),
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

    /// Puts the cursor on `index` and asks the model for the preview of that row.
    fn put(&self, picker: &Picker, index: usize, cx: &mut Cx<'_, Self>) {
        self.cursor.select(index);
        self.cursor.set_cursor(Some(index));
        Self::preview_of(picker, index, cx);
    }

    fn preview_of(picker: &Picker, index: usize, cx: &mut Cx<'_, Self>) {
        if let Some(pet) = picker.pets.get(index) {
            let hs = read::<Hs, _>(cx);
            cx.emit(
                &hs.screen,
                ScreenIntent::Pets(Intent::LoadPreview(pet.selector.clone())),
            );
        }
    }

    /// Leaves the picker: its preview is not wanted any more.
    fn leave(cx: &mut Cx<'_, Self>) {
        let hs = read::<Hs, _>(cx);
        cx.emit(&hs.screen, ScreenIntent::Pets(Intent::ReleasePreview));
        close_dialog::<_, ()>(cx, None);
    }
}

impl State for ScreenState {
    type Widget = Screen;
    type Msg = Msg;

    fn did_change_dependencies(&mut self, _: &Screen, cx: &mut Cx<'_, Self>) {
        let picker =
            select::<ScreenStates, Option<Picker>, _>(cx, |states| states.pets.catalog.clone());
        if let (Some(picker), false) = (&picker, self.started) {
            // Seen first: the cursor starts on the configured pet, whose preview is asked for.
            let initial = picker.initial();
            self.cursor.select(initial);
            self.cursor.set_cursor(Some(initial));
            self.below.set_value(picker.pets.len() > WINDOW);
            Self::preview_of(picker, initial, cx);
            self.started = true;
        }
    }

    fn update(&mut self, msg: Msg, _: &Screen, cx: &mut Cx<'_, Self>) -> Rebuild {
        let Some(picker) =
            select::<ScreenStates, Option<Picker>, _>(cx, |states| states.pets.catalog.clone())
        else {
            // No pets listed (the unseeded stub): Esc leaves it.
            if matches!(msg, Msg::Close) {
                close_dialog::<_, ()>(cx, None);
            }
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
                    let to =
                        usize::try_from((from + i64::from(delta)).rem_euclid(len)).unwrap_or(0);
                    self.put(&picker, visible[to], cx);
                    self.list.ensure_index_visible(to);
                }
            }
            Msg::Row(index) => {
                if at.is_some_and(|at| visible[at] == index) {
                    if let Some(pet) = picker.pets.get(index) {
                        cx.emit(
                            &hs.screen,
                            ScreenIntent::Pets(Intent::Pick(pet.selector.clone())),
                        );
                        close_dialog::<_, ()>(cx, None);
                    }
                } else if let Some(row) = visible.iter().position(|pet| *pet == index) {
                    self.put(&picker, index, cx);
                    self.list.ensure_index_visible(row);
                }
            }
            // The search changed: the cursor goes back to the first row.
            Msg::Search => {
                if let Some(first) = visible.first() {
                    self.put(&picker, *first, cx);
                    self.list.jump_to_index(0);
                }
            }
            Msg::Pick => {
                if let Some(pet) = at.and_then(|at| picker.pets.get(visible[at])) {
                    cx.emit(
                        &hs.screen,
                        ScreenIntent::Pets(Intent::Pick(pet.selector.clone())),
                    );
                    close_dialog::<_, ()>(cx, None);
                }
            }
            // Esc clears a search typed, then closes.
            Msg::Close => {
                if self.query.text().trim().is_empty() && self.query.text().is_empty() {
                    Self::leave(cx);
                } else {
                    self.query.clear();
                    if let Some(first) = picker.visible("").first() {
                        self.put(&picker, *first, cx);
                        self.list.jump_to_index(0);
                    }
                }
            }
        }
        Rebuild::Yes
    }

    fn build(&self, _: &Screen, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let picker =
            select::<ScreenStates, Option<Picker>, _>(cx, |states| states.pets.catalog.clone());
        let s = strings::of(cx);
        let tokens = Tokens::of(cx);
        let Some(picker) = picker else {
            // No picker seeded (the debug menu's `pets` row): the skeleton's
            // empty-slot line, as before the slot was filled.
            let shared = crate::l10n::strings::of(cx);
            return Dialog::bare(Text::new(shared.empty_slot(NAME, ID).to_string())).into();
        };
        let picker = Rc::new(picker);
        let (cursor, below, list) = (self.cursor.clone(), self.below.clone(), self.list.clone());
        let taps: Rc<Vec<Handler<PointerEvent>>> = Rc::new(
            (0..picker.pets.len())
                .map(|index| cx.map(move |_: PointerEvent| Msg::Row(index)))
                .collect(),
        );
        // The list and the "more below" row follow the search text.
        let list = {
            let (picker, tokens) = (picker.clone(), tokens.clone());
            ListenableBuilder::new(self.query.listenable(), {
                let query = self.query.clone();
                move |_, _| {
                    let visible = Rc::new(picker.visible(&query.text()));
                    below.set_value(visible.len() > WINDOW);
                    let rows = {
                        let (picker, visible, cursor, tokens, taps) = (
                            picker.clone(),
                            visible.clone(),
                            cursor.clone(),
                            tokens.clone(),
                            taps.clone(),
                        );
                        ListView::builder(visible.len(), move |row: usize| -> AnyWidget {
                            let index = visible[row];
                            let (picker, cursor, tokens) =
                                (picker.clone(), cursor.clone(), tokens.clone());
                            let tap = taps[index].clone();
                            let row = ListenableBuilder::new(cursor.listenable(), {
                                move |_, _| {
                                    let pet = &picker.pets[index];
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
                            });
                            GestureDetector::new().on_tap(tap).child(row).into()
                        })
                        .controller(list.clone())
                        .item_extent(1)
                    };
                    let scroll = below.clone();
                    let rows = NotificationListener::<ScrollNotification>::new(rows)
                        .on_notification(move |note| {
                            scroll.set_value(note.metrics.pixels < note.metrics.max_scroll_extent);
                            false
                        });
                    SizedBox::new(
                        None,
                        Some(i32::try_from(visible.len().min(WINDOW)).unwrap_or(1)),
                        Some(rows.into()),
                    )
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
                    Text::line(s.more_below.to_string())
                        .style(style.clone())
                        .into()
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
        let focus = Focus::new(Dialog::bare(column)).can_request_focus(false);
        let focus = bind_fixed(focus, fixed::DISMISS, cx.send(Msg::Close));
        let focus = bind_fixed(focus, fixed::CONFIRM_PREVIOUS, cx.send(Msg::Move(-1)));
        bind_fixed(focus, fixed::CONFIRM_NEXT, cx.send(Msg::Move(1))).into()
    }
}

/// The preview area: its status, centred (`PetPickerPreviewRenderable`).
fn preview(picker: &Picker, s: &strings::Strings, tokens: &Tokens) -> AnyWidget {
    let Some((_, Preview::Loading)) = &picker.preview else {
        return AnyWidget::default();
    };
    Column::new()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(Text::new(String::new()))
        .child(SizedBox::new(
            None,
            Some(1),
            Some(
                Align::new(
                    Alignment::TOP_CENTER,
                    Text::line(s.preview_loading.to_string()).style(tokens.bold()),
                )
                .into(),
            ),
        ))
        .into()
}
