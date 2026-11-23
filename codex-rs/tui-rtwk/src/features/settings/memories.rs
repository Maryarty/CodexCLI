//! The memories dialog (the oracle's `MemoriesSettingsView`:
//! `bottom_pane/memories_settings_view.rs`): two switches and a reset row, and
//! the reset's confirmation, which replaces them in the same sheet. In Flutter
//! it is a bottom sheet whose content is a `Column(min)` of a header, a few
//! tappable rows and a hint; the cursor of each list is a
//! `SelectionController` (the sheet owns both, the confirmation's starts on
//! its first row each time), the keys come from the `FocusNode.onKeyEvent` of
//! a `Focus`. What the switches are before Save and whether the confirmation
//! is up are the sheet's draft (`MemoriesDraft`); what Enter sends is the
//! model's request (`Edit::MemoriesSaved`, `Edit::MemoriesReset`, ADR-013). The mouse (ADR-011, beyond the
//! oracle): a tap puts the cursor on a row, a tap on the row the cursor is on
//! does what Enter (a switch: Space) does.

use std::rc::Rc;

use rtwk::controls::SelectionController;
use rtwk::focus::{Focus, FocusNode, KeyEventResult, KeyPress};
use rtwk::gestures::{GestureDetector, PointerEvent};
use rtwk::layout::{
    BoxWidget, Column, CrossAxisAlignment, EdgeInsets, Expanded, MainAxisSize, Padding, Row,
    SizedBox,
};
use rtwk::overlay::{Dialog, close_dialog};
use rtwk::provider::select;
use rtwk::style::{Span, Style};
use rtwk::text::{Text, TextWrap, segmenter_of};
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::features::FeatureStates;
use codex_tui_core::features::settings::{Memories, Offer, Which};
use codex_tui_core::keymap::Keymap;

use super::draft::MemoriesDraft;
use super::keys::{Keys, ListKey, is_interrupt, is_space, list_key};
use super::strings::{self, Strings};
use crate::app::theme::Tokens;
use crate::widgets::session_list::{Move, target};

/// The sheet of the memories dialog and its reset confirmation.
#[derive(Clone, Debug, PartialEq)]
pub struct MemoriesSheet {
    /// The dialog opens on the reset confirmation (a debug entry's start).
    pub confirming: bool,
}

/// Widget state: the controllers (R14).
pub struct MemoriesState {
    keys: Keys,
    draft: Option<MemoriesDraft>,
    node: FocusNode,
    /// The switches and the reset row.
    main: SelectionController<usize>,
    /// The confirmation's two choices.
    confirm: SelectionController<usize>,
}

/// The rows of the switches: the reset row is the last.
const RESET_ROW: usize = 2;
const ROWS: usize = 3;

/// What a key means to the dialog (the oracle's list keymap and
/// `MemoriesSettingsView::handle_key_event`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    Move(Move),
    Toggle,
    Accept,
    Close,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Act(Act),
    /// A tap on the row at an index.
    Row(usize),
}

fn act_of(keys: &Keymap, key: &KeyPress) -> Option<Act> {
    if is_space(key) {
        return Some(Act::Toggle);
    }
    // Ctrl+C is the view's cancel too (`on_ctrl_c`).
    if is_interrupt(key) {
        return Some(Act::Close);
    }
    match list_key(keys, key, false)? {
        ListKey::Move(mv) => Some(Act::Move(mv)),
        ListKey::Accept => Some(Act::Accept),
        ListKey::Cancel => Some(Act::Close),
        ListKey::Left | ListKey::Right => None,
    }
}

impl StatefulWidget for MemoriesSheet {
    type State = MemoriesState;

    fn create_state(&self) -> MemoriesState {
        MemoriesState {
            keys: Keys::new(),
            draft: None,
            node: FocusNode::new(),
            main: SelectionController::new(),
            confirm: SelectionController::new(),
        }
    }
}

impl_widget!(stateful eq MemoriesSheet);

/// The switches the model offers.
fn offered(cx: &mut Cx<'_, MemoriesState>) -> Option<Memories> {
    select::<FeatureStates, Option<Memories>, _>(cx, |states| {
        match states.settings.offers.get(&Which::Memories) {
            Some(Offer::Memories(dialog)) => Some(dialog.clone()),
            _ => None,
        }
    })
}

impl MemoriesState {
    /// The list the dialog shows now, and how many rows it has.
    fn active(&self, confirming: bool) -> (&SelectionController<usize>, usize) {
        if confirming {
            (&self.confirm, 2)
        } else {
            (&self.main, ROWS)
        }
    }

    fn put(controller: &SelectionController<usize>, at: usize) {
        controller.select(at);
        controller.set_cursor(Some(at));
    }

    fn act(&mut self, act: Act, cx: &mut Cx<'_, Self>) {
        let Some(confirming) = self.draft.as_ref().map(|draft| draft.confirming) else {
            return;
        };
        let (cursor, len) = self.active(confirming);
        let at = cursor.cursor();
        match act {
            Act::Move(mv) => {
                if let Some(to) = target(len, at, &|_| true, mv, len) {
                    Self::put(cursor, to);
                }
            }
            // Space flips a switch; on the other rows it does nothing.
            Act::Toggle if !confirming => {
                if let (Some(row), Some(draft)) = (at, self.draft.as_mut()) {
                    draft.toggle(row);
                }
            }
            Act::Toggle => {}
            // Enter: the reset row asks to confirm, a switch saves; in the
            // confirmation the first choice resets and the second goes back.
            Act::Accept if confirming => match at {
                Some(0) => {
                    if let Some(edit) = self.draft.as_ref().and_then(MemoriesDraft::reset) {
                        super::request(cx, edit);
                    }
                    close_dialog::<_, ()>(cx, None);
                }
                _ => self.back(),
            },
            Act::Accept => match at {
                Some(RESET_ROW) => {
                    if let Some(draft) = self.draft.as_mut() {
                        draft.confirming = true;
                    }
                }
                _ => {
                    if let Some(edit) = self.draft.as_ref().and_then(MemoriesDraft::save) {
                        super::request(cx, edit);
                    }
                    close_dialog::<_, ()>(cx, None);
                }
            },
            Act::Close if confirming => self.back(),
            Act::Close => close_dialog::<_, ()>(cx, None),
        }
    }

    /// The confirmation goes back: the reset row is the cursor again
    /// (`close_reset_confirmation`), and the next confirmation starts on its
    /// first choice (`open_reset_confirmation`).
    fn back(&mut self) {
        Self::put(&self.main, RESET_ROW);
        Self::put(&self.confirm, 0);
        if let Some(draft) = self.draft.as_mut() {
            draft.confirming = false;
        }
    }
}

impl State for MemoriesState {
    type Widget = MemoriesSheet;
    type Msg = Msg;

    fn init_state(&mut self, _: &MemoriesSheet, cx: &mut Cx<'_, Self>) {
        let send = cx.local_sender();
        let keys = self.keys.clone();
        self.node.set_on_key_event(Some(Rc::new(move |_, key| {
            match act_of(&keys.get(), &key) {
                Some(act) => {
                    send(Msg::Act(act));
                    KeyEventResult::Handled
                }
                None => KeyEventResult::Ignored,
            }
        })));
        Self::put(&self.main, 0);
        Self::put(&self.confirm, 0);
    }

    fn dispose(&mut self) {
        self.node.set_on_key_event(None);
        self.main.dispose();
        self.confirm.dispose();
    }

    fn did_change_dependencies(&mut self, w: &MemoriesSheet, cx: &mut Cx<'_, Self>) {
        self.keys.refresh(cx);
        if self.draft.is_none()
            && let Some(offered) = offered(cx)
        {
            self.draft = Some(MemoriesDraft::new(&offered, w.confirming));
        }
    }

    fn update(&mut self, msg: Msg, _: &MemoriesSheet, cx: &mut Cx<'_, Self>) -> Rebuild {
        let Some(confirming) = self.draft.as_ref().map(|draft| draft.confirming) else {
            return Rebuild::No;
        };
        match msg {
            Msg::Act(act) => self.act(act, cx),
            Msg::Row(row) => {
                let (cursor, len) = self.active(confirming);
                if cursor.cursor() == Some(row) {
                    let accept = confirming || row == RESET_ROW;
                    self.act(if accept { Act::Accept } else { Act::Toggle }, cx);
                } else if row < len {
                    Self::put(cursor, row);
                }
            }
        }
        Rebuild::Yes
    }

    fn build(&self, w: &MemoriesSheet, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let s = strings::of(cx);
        let tokens = Tokens::of(cx);
        let seg = segmenter_of(cx);
        let Some(offered) = offered(cx) else {
            return Dialog::bare(Text::new(String::new())).into();
        };
        let draft = self
            .draft
            .clone()
            .unwrap_or_else(|| MemoriesDraft::new(&offered, w.confirming));
        let dialog = draft.view();
        let confirming = draft.confirming;
        let gutter = EdgeInsets::symmetric(2, 0);
        let note = |text: &str| -> AnyWidget {
            Padding::new(
                gutter,
                Text::new(text.to_string())
                    .style(tokens.dim())
                    .wrap(TextWrap::Word),
            )
            .into()
        };
        let blank = || Text::new(String::new());

        let (cursor, _) = self.active(confirming);
        let at = cursor.cursor();
        let rows = rows_of(s, &dialog, confirming);
        // The name column is as wide as the widest name with its marks (the
        // oracle measures the rows it shows: all of them, at most eight).
        let wide = rows
            .iter()
            .map(|row| seg.width(&row.lead_and_name(s, false)))
            .max()
            .unwrap_or(0);
        let wide = i32::try_from(wide).unwrap_or(0);
        let mut list = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch);
        for (index, row) in rows.iter().enumerate() {
            let tap = cx.map(move |_: PointerEvent| Msg::Row(index));
            let line = row_line(s, row, at == Some(index), wide, &tokens);
            list = list.child(GestureDetector::new().on_tap(tap).child(line));
        }

        let (title, subtitle, hint) = if confirming {
            (s.reset_title, s.reset_subtitle, s.reset_hint)
        } else {
            (s.memories_title, s.memories_subtitle, s.memories_hint)
        };
        // Stretched: the sheet is as wide as the screen, not as its widest line.
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(blank())
            .child(Text::line(format!("{}{title}", s.indent)).style(tokens.bold()))
            .child(note(subtitle))
            .child(blank())
            .child(blank())
            .child(list)
            .child(blank())
            .child(blank());
        if confirming {
            column = column.child(blank());
        } else {
            column = column
                .child(Padding::new(
                    gutter,
                    Text::rich([
                        Span::styled(s.memories_docs.to_string(), tokens.dim()),
                        Span::styled(s.memories_docs_url.to_string(), tokens.accent())
                            .underline()
                            .link(s.memories_docs_url.to_string()),
                    ]),
                ))
                .child(blank());
        }
        let column = column.child(note(hint));
        // The surface is the sheet's own (`user_message_style`): a bare dialog
        // leaves what it does not paint to the app under it.
        let surface = tokens.prompt_surface(cx, false);
        Focus::new(Dialog::bare(BoxWidget::new().style(surface).child(column)))
            .focus_node(self.node.clone())
            .autofocus()
            .into()
    }
}

/// One row of the dialog: a switch (`check` is its value), the reset row, or
/// a choice of the confirmation.
struct MemoryRow {
    check: Option<bool>,
    name: &'static str,
    description: &'static str,
}

impl MemoryRow {
    /// The marker of the cursor, the switch and the name (the oracle's
    /// `"{prefix} [x] {name}"`).
    fn lead_and_name(&self, s: &Strings, on: bool) -> String {
        let marker = if on { s.cursor_marker } else { s.indent };
        let check = match self.check {
            Some(true) => s.check_on,
            Some(false) => s.check_off,
            None => "",
        };
        format!("{marker}{check}{}", self.name)
    }
}

fn rows_of(s: &Strings, dialog: &Memories, confirming: bool) -> Vec<MemoryRow> {
    if confirming {
        return vec![
            MemoryRow {
                check: None,
                name: s.reset_yes,
                description: s.reset_yes_description,
            },
            MemoryRow {
                check: None,
                name: s.reset_no,
                description: s.reset_no_description,
            },
        ];
    }
    vec![
        MemoryRow {
            check: Some(dialog.use_memories),
            name: s.memories_use,
            description: s.memories_use_description,
        },
        MemoryRow {
            check: Some(dialog.generate_memories),
            name: s.memories_generate,
            description: s.memories_generate_description,
        },
        MemoryRow {
            check: None,
            name: s.memories_reset,
            description: s.memories_reset_description,
        },
    ]
}

/// A row: the name column (`wide` cells and a gap) and the description beside
/// it, wrapped under its own column; the row under the cursor is in the
/// selection colour.
fn row_line(s: &Strings, row: &MemoryRow, on: bool, wide: i32, tokens: &Tokens) -> AnyWidget {
    let (name_style, description_style) = if on {
        (tokens.selected(), tokens.selected())
    } else {
        (Style::new(), tokens.dim())
    };
    Row::new()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .child(SizedBox::new(
            Some(wide + 2),
            None,
            Some(
                Text::line(row.lead_and_name(s, on))
                    .style(name_style)
                    .into(),
            ),
        ))
        .child(Expanded::new(
            1,
            Text::new(row.description.to_string())
                .style(description_style)
                .wrap(TextWrap::Word),
        ))
        .into()
}
