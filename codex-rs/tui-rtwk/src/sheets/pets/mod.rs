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
use rtwk::graphics::GraphicsScope;
use rtwk::layout::{
    Align, Alignment, BoxWidget, Column, CrossAxisAlignment, Expanded, MainAxisSize, Row, SizedBox,
};
use rtwk::overlay::{Dialog, DialogResult, close_dialog, show_dialog};
use rtwk::provider::{read, select};
use rtwk::scroll::{ListController, ListView, ScrollNotification};
use rtwk::style::Style;
use rtwk::text::{Text, TextWrap};
use rtwk::{AnyWidget, Cx, Handler, Rebuild, State, StatefulWidget, impl_widget};
use rtwk_core::graphics::GraphicsProtocol;
use rtwk_rich::image::Image;

use codex_tui_core::features::FeatureStates;
use codex_tui_core::keymap::fixed;
use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::ScreenStates;
use codex_tui_core::screens::pets::{DISABLED, ID, Intent, NAME, Picker, Preview};

use crate::app::debug::DebugEntry;
use crate::app::debug::DebugOpen;
use crate::app::debug::EntryOpen;
use crate::app::theme::Tokens;
use crate::app::view::Hs;
use crate::features::{
    PetIntent as LiveIntent, PetPhase as Phase, PetVisit as Visit, is_pet_disable as is_disable,
    pick_pet, verb, verb_by,
};
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
            open: EntryOpen::Dialog(|_| Screen { visit: None }.into()),
        })
        .collect()
}

/// Opens the picker as a modal sheet over the session page: no frame, no
/// dimming, at the bottom edge, as the oracle draws it in place of the
/// composer (architecture §1, amendment 2026-11-19b).
pub fn open<S: 'static>(cx: &mut Cx<'_, S>) -> DialogResult<()> {
    open_visit(cx, None)
}

/// The slash command keeps its argument all the way to the local pet feature.
pub fn command<S: State + 'static>(cx: &mut Cx<'_, S>, args: &str) {
    let args = args.trim();
    if is_disable(args) {
        verb(
            cx,
            LiveIntent::Request {
                visit: Visit::fresh(),
                selector: Some(args.to_owned()),
                graphics: false,
            },
        );
    } else {
        let previous_focus = Focus::maybe_of(cx, false)
            .and_then(|node| node.manager())
            .and_then(|manager| manager.primary_focus());
        let dialog = open_visit(cx, (!args.is_empty()).then(|| args.to_owned()));
        cx.spawn(async move {
            if dialog.await.is_some()
                && let Some(focus) = previous_focus
            {
                focus.request_focus();
            }
            None
        });
    }
}

fn open_visit<S: 'static>(cx: &mut Cx<'_, S>, selector: Option<String>) -> DialogResult<()> {
    let visit = Visit::fresh();
    let graphics =
        GraphicsScope::of(cx).is_some_and(|caps| caps.protocol != GraphicsProtocol::None);
    verb(
        cx,
        LiveIntent::Request {
            visit,
            selector,
            graphics,
        },
    );
    show_dialog(
        cx,
        move |_| Screen { visit: Some(visit) }.into(),
        crate::sheets::sheet_options(),
    )
}

/// Opens the sheet with no seeded model (debug menu): nothing to pick from.
pub fn open_empty() -> DebugOpen {
    DebugOpen::Dialog(|_| Screen { visit: None }.into())
}

/// The picker sheet.
#[derive(Clone, Debug, PartialEq)]
pub struct Screen {
    visit: Option<Visit>,
}

/// Widget state: rtwk's controllers (R14): the cursor row (a key into the
/// picker's pets), the search text, and whether rows lie below the window.
pub struct ScreenState {
    /// The catalog was seen: the cursor started on its configured pet.
    started: bool,
    live_seen: bool,
    live: Option<(Rc<Hs>, Visit)>,
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
    Finished,
    Retired,
}

impl StatefulWidget for Screen {
    type State = ScreenState;

    fn create_state(&self) -> ScreenState {
        ScreenState {
            started: false,
            live_seen: false,
            live: None,
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
    fn put(&self, w: &Screen, picker: &Picker, index: usize, cx: &mut Cx<'_, Self>) {
        self.cursor.select(index);
        self.cursor.set_cursor(Some(index));
        Self::preview_of(w, picker, index, cx);
    }

    fn preview_of(w: &Screen, picker: &Picker, index: usize, cx: &mut Cx<'_, Self>) {
        if let Some(pet) = picker.pets.get(index) {
            if let Some(visit) = w.visit {
                verb(
                    cx,
                    LiveIntent::ReadPreview {
                        visit,
                        selector: Some(pet.selector.clone()),
                    },
                );
                return;
            }
            let hs = read::<Hs, _>(cx);
            cx.emit(
                &hs.screen,
                ScreenIntent::Pets(Intent::LoadPreview(pet.selector.clone())),
            );
        }
    }

    /// Leaves the picker: its preview is not wanted any more.
    fn leave(w: &Screen, cx: &mut Cx<'_, Self>) {
        if let Some(visit) = w.visit {
            verb(cx, LiveIntent::RetirePreview(visit));
        } else {
            let hs = read::<Hs, _>(cx);
            cx.emit(&hs.screen, ScreenIntent::Pets(Intent::ReleasePreview));
        }
        close_dialog(cx, w.visit.map(|_| ()));
    }

    fn apply(w: &Screen, selector: String, cx: &mut Cx<'_, Self>) {
        if let Some(visit) = w.visit {
            verb(cx, LiveIntent::Apply { visit, selector });
        } else {
            pick_pet(cx, selector);
            Self::leave(w, cx);
        }
    }
}

fn phase(w: &Screen, cx: &mut Cx<'_, ScreenState>) -> Option<Phase> {
    let visit = w.visit?;
    select::<FeatureStates, Option<Phase>, _>(cx, move |states| states.pets.phase(visit).cloned())
}

fn picker(w: &Screen, cx: &mut Cx<'_, ScreenState>) -> Option<Picker> {
    if w.visit.is_some() {
        return match phase(w, cx) {
            Some(Phase::Ready(picker)) => Some(picker),
            _ => None,
        };
    }
    select::<ScreenStates, Option<Picker>, _>(cx, |states| states.pets.catalog.clone())
}

impl State for ScreenState {
    type Widget = Screen;
    type Msg = Msg;

    fn dispose_with(&mut self, cx: &mut Cx<'_, Self>) {
        if let Some((hs, visit)) = self.live.take() {
            verb_by(cx, &hs, LiveIntent::RetirePreview(visit));
        }
    }

    fn did_change_dependencies(&mut self, w: &Screen, cx: &mut Cx<'_, Self>) {
        if let Some(visit) = w.visit {
            if self.live.is_none() {
                self.live = Some((read::<Hs, _>(cx), visit));
            }
            let phase = phase(w, cx);
            if matches!(phase, Some(Phase::Finished)) {
                cx.local_sender()(Msg::Finished);
            } else if self.live_seen && phase.is_none() {
                cx.local_sender()(Msg::Retired);
            }
            self.live_seen |= phase.is_some();
        }
        let picker = picker(w, cx);
        if let (Some(picker), false) = (&picker, self.started) {
            // Seen first: the cursor starts on the configured pet, whose preview is asked for.
            let initial = picker.initial();
            self.cursor.select(initial);
            self.cursor.set_cursor(Some(initial));
            self.below.set_value(picker.pets.len() > WINDOW);
            Self::preview_of(w, picker, initial, cx);
            self.started = true;
        }
    }

    fn update(&mut self, msg: Msg, w: &Screen, cx: &mut Cx<'_, Self>) -> Rebuild {
        if matches!(msg, Msg::Finished) {
            Self::leave(w, cx);
            return Rebuild::No;
        }
        if matches!(msg, Msg::Retired) {
            close_dialog::<_, ()>(cx, None);
            return Rebuild::No;
        }
        let Some(picker) = picker(w, cx) else {
            if matches!(msg, Msg::Close)
                || (matches!(msg, Msg::Pick) && matches!(phase(w, cx), Some(Phase::Unavailable(_))))
            {
                Self::leave(w, cx);
            }
            return Rebuild::No;
        };
        let visible = self.visible(&picker);
        let at = self
            .cursor
            .cursor()
            .and_then(|key| visible.iter().position(|index| *index == key));
        match msg {
            Msg::Move(delta) => {
                if !visible.is_empty() {
                    let len = i64::try_from(visible.len()).unwrap_or(1);
                    let from = i64::try_from(at.unwrap_or(0)).unwrap_or(0);
                    let to =
                        usize::try_from((from + i64::from(delta)).rem_euclid(len)).unwrap_or(0);
                    self.put(w, &picker, visible[to], cx);
                    self.list.ensure_index_visible(to);
                }
            }
            Msg::Row(index) => {
                if at.is_some_and(|at| visible[at] == index) {
                    if let Some(pet) = picker.pets.get(index) {
                        Self::apply(w, pet.selector.clone(), cx);
                    }
                } else if let Some(row) = visible.iter().position(|pet| *pet == index) {
                    self.put(w, &picker, index, cx);
                    self.list.ensure_index_visible(row);
                }
            }
            // The search changed: the cursor goes back to the first row.
            Msg::Search => {
                if let Some(first) = visible.first() {
                    self.put(w, &picker, *first, cx);
                    self.list.jump_to_index(0);
                } else if let Some(visit) = w.visit {
                    verb(
                        cx,
                        LiveIntent::ReadPreview {
                            visit,
                            selector: None,
                        },
                    );
                }
            }
            Msg::Pick => {
                if let Some(pet) = at.and_then(|at| picker.pets.get(visible[at])) {
                    Self::apply(w, pet.selector.clone(), cx);
                }
            }
            // Esc clears a search typed, then closes.
            Msg::Close => {
                if self.query.text().trim().is_empty() && self.query.text().is_empty() {
                    Self::leave(w, cx);
                } else {
                    self.query.clear();
                    if let Some(first) = picker.visible("").first() {
                        self.put(w, &picker, *first, cx);
                        self.list.jump_to_index(0);
                    }
                }
            }
            Msg::Finished | Msg::Retired => {}
        }
        Rebuild::Yes
    }

    fn build(&self, w: &Screen, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let picker = picker(w, cx);
        let s = strings::of(cx);
        let tokens = Tokens::of(cx);
        let Some(picker) = picker else {
            if w.visit.is_some() {
                return live_status(w, cx, s, &tokens);
            }
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
        // the cells the rows leave blank (the preview's) are the pane's own, not the page's under it
        let focus = Focus::new(Dialog::bare(
            BoxWidget::new().style(Style::new()).child(column),
        ))
        .can_request_focus(false);
        let focus = bind_fixed(focus, fixed::DISMISS, cx.send(Msg::Close));
        let focus = bind_fixed(focus, fixed::CONFIRM_PREVIOUS, cx.send(Msg::Move(-1)));
        bind_fixed(focus, fixed::CONFIRM_NEXT, cx.send(Msg::Move(1))).into()
    }
}

/// The preview area: its status, centred (`PetPickerPreviewRenderable`).
fn preview(picker: &Picker, s: &strings::Strings, tokens: &Tokens) -> AnyWidget {
    let (label, detail) = match &picker.preview {
        Some((_, Preview::Image(image))) => {
            return Align::new(Alignment::CENTER, Image::memory(image.png.clone())).into();
        }
        Some((_, Preview::Loading)) => (s.preview_loading, None),
        Some((_, Preview::Disabled)) => (s.preview_disabled, Some(s.preview_disabled_detail)),
        Some((_, Preview::Failed(error))) => (s.preview_unavailable, Some(error.as_str())),
        _ => return AnyWidget::default(),
    };
    let mut text = Column::new()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(Text::new(String::new()))
        .child(SizedBox::new(
            None,
            Some(1),
            Some(
                Align::new(
                    Alignment::TOP_CENTER,
                    Text::line(label.to_owned()).style(tokens.bold()),
                )
                .into(),
            ),
        ));
    if let Some(detail) = detail {
        text = text.child(Expanded::new(
            1,
            Align::new(
                Alignment::TOP_CENTER,
                Text::new(detail.to_owned())
                    .style(tokens.dim())
                    .wrap(TextWrap::Word),
            ),
        ));
    }
    text.into()
}

fn live_status(
    w: &Screen,
    cx: &mut Cx<'_, ScreenState>,
    s: &strings::Strings,
    tokens: &Tokens,
) -> AnyWidget {
    let (title, detail, unavailable) = match phase(w, cx) {
        Some(Phase::Unavailable(reason)) => (s.unavailable_title, s.unavailable(reason), true),
        Some(Phase::Applying | Phase::Finished) => (s.apply_title, s.apply_detail, false),
        _ => (s.picker_title, s.catalog_loading, false),
    };
    let mut column = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .child(Text::line(format!("{}{title}", s.indent)).style(tokens.bold()))
        .child(
            Row::new()
                .child(SizedBox::new(Some(2), None, None))
                .child(Expanded::new(
                    1,
                    Text::new(detail).style(tokens.dim()).wrap(TextWrap::Word),
                ))
                .child(SizedBox::new(Some(RIGHT_MARGIN), None, None)),
        );
    if unavailable {
        column = column
            .child(Text::new(String::new()))
            .child(Text::new(String::new()))
            .child(
                GestureDetector::new()
                    .on_tap(cx.map(|_: PointerEvent| Msg::Pick))
                    .child(
                        Text::line(format!("{}{}", s.cursor_marker, s.close))
                            .style(tokens.selected()),
                    ),
            )
            .child(Text::new(String::new()))
            .child(Text::line(format!("{}{}", s.indent, s.hint)).style(tokens.dim()));
    }
    let focus = Focus::new(Dialog::bare(
        BoxWidget::new().style(Style::new()).child(column),
    ))
    .autofocus();
    let focus = bind_fixed(focus, fixed::DISMISS, cx.send(Msg::Close));
    bind_fixed(focus, fixed::POPUP_ACCEPT, cx.send(Msg::Pick)).into()
}
