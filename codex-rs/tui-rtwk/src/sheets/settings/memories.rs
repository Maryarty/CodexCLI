//! The memories dialog (the oracle's `MemoriesSettingsView`:
//! `bottom_pane/memories_settings_view.rs`): two switches and a reset row, and
//! the reset's confirmation, which replaces them in the same sheet. In Flutter
//! it is a bottom sheet whose content is a `Column(min)` of a header, a few
//! tappable rows and a hint; the cursor of each list is a
//! `SelectionController` (the sheet owns both, the confirmation's starts on
//! its first row each time), the keys come from the `FocusNode.onKeyEvent` of
//! a `Focus`. What the switches are, whether the confirmation is up and what
//! Enter sends are the model's (`Memories`, `Intent::Toggle`, `Save`,
//! `ConfirmReset`, `Reset`, `Close`, ADR-003). The mouse (ADR-011, beyond the
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
use rtwk::prelude::{KeyCode, Mods};
use rtwk::provider::{read, select};
use rtwk::style::{Span, Style};
use rtwk::text::{Text, TextWrap, segmenter_of};
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::ScreenStates;
use codex_tui_core::screens::settings::{Intent, MEMORIES_GENERATE, MEMORIES_USE, Memories, Open};

use super::setup::{letter_of, move_of};
use super::strings::{self, Strings};
use crate::app::theme::Tokens;
use crate::app::view::Hs;
use crate::widgets::session_list::{Move, target};

/// The sheet of the memories dialog and its reset confirmation.
#[derive(Clone, Debug, PartialEq)]
pub struct MemoriesSheet;

/// Widget state: the controllers (R14).
pub struct MemoriesState {
    shown: bool,
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

fn act_of(key: &KeyPress) -> Option<Act> {
    if let Some(mv) = move_of(key) {
        return Some(Act::Move(mv));
    }
    let ctrl = key.mods.contains(Mods::CTRL);
    let alt = key.mods.contains(Mods::ALT);
    match (key.code, letter_of(key), ctrl, alt) {
        (KeyCode::Char, Some(' '), false, false) if key.mods.is_empty() => Some(Act::Toggle),
        (KeyCode::Enter, _, false, false) => Some(Act::Accept),
        (KeyCode::Escape, _, false, false) => Some(Act::Close),
        // Ctrl+C is the view's cancel too (`on_ctrl_c`).
        (KeyCode::Char, Some('c'), true, false) => Some(Act::Close),
        _ => None,
    }
}

impl StatefulWidget for MemoriesSheet {
    type State = MemoriesState;

    fn create_state(&self) -> MemoriesState {
        MemoriesState {
            shown: false,
            node: FocusNode::new(),
            main: SelectionController::new(),
            confirm: SelectionController::new(),
        }
    }
}

impl_widget!(stateful eq MemoriesSheet);

/// The dialog the model holds open.
fn open_dialog(cx: &mut Cx<'_, MemoriesState>) -> Option<Memories> {
    select::<ScreenStates, Option<Memories>, _>(cx, |states| match &states.settings.open {
        Some(Open::Memories(dialog)) => Some(dialog.clone()),
        _ => None,
    })
}

impl MemoriesState {
    /// The list the dialog shows now, and how many rows it has.
    fn active(&self, dialog: &Memories) -> (&SelectionController<usize>, usize) {
        if dialog.confirming {
            (&self.confirm, 2)
        } else {
            (&self.main, ROWS)
        }
    }

    fn put(controller: &SelectionController<usize>, at: usize) {
        controller.select(at);
        controller.set_cursor(Some(at));
    }

    fn act(&self, act: Act, dialog: &Memories, cx: &mut Cx<'_, Self>) {
        let hs = read::<Hs, _>(cx);
        let (cursor, len) = self.active(dialog);
        let at = cursor.cursor();
        let send = |cx: &mut Cx<'_, Self>, intent: Intent| {
            cx.emit(&hs.screen, ScreenIntent::Settings(intent));
        };
        match act {
            Act::Move(mv) => {
                if let Some(to) = target(len, at, &|_| true, mv, len) {
                    Self::put(cursor, to);
                }
            }
            // Space flips a switch; on the other rows it does nothing.
            Act::Toggle if !dialog.confirming => match at {
                Some(0) => send(cx, Intent::Toggle(MEMORIES_USE.to_string())),
                Some(1) => send(cx, Intent::Toggle(MEMORIES_GENERATE.to_string())),
                _ => {}
            },
            Act::Toggle => {}
            // Enter: the reset row asks to confirm, a switch saves; in the
            // confirmation the first choice resets and the second goes back.
            Act::Accept if dialog.confirming => match at {
                Some(0) => send(cx, Intent::Reset),
                _ => self.back(cx, &send),
            },
            Act::Accept => match at {
                Some(RESET_ROW) => send(cx, Intent::ConfirmReset),
                _ => send(cx, Intent::Save),
            },
            Act::Close if dialog.confirming => self.back(cx, &send),
            Act::Close => send(cx, Intent::Close),
        }
    }

    /// The confirmation goes back: the reset row is the cursor again
    /// (`close_reset_confirmation`), and the next confirmation starts on its
    /// first choice (`open_reset_confirmation`).
    fn back(&self, cx: &mut Cx<'_, Self>, send: &dyn Fn(&mut Cx<'_, Self>, Intent)) {
        Self::put(&self.main, RESET_ROW);
        Self::put(&self.confirm, 0);
        send(cx, Intent::Close);
    }
}

impl State for MemoriesState {
    type Widget = MemoriesSheet;
    type Msg = Msg;

    fn init_state(&mut self, _: &MemoriesSheet, cx: &mut Cx<'_, Self>) {
        let send = cx.local_sender();
        self.node
            .set_on_key_event(Some(Rc::new(move |_, key| match act_of(&key) {
                Some(act) => {
                    send(Msg::Act(act));
                    KeyEventResult::Handled
                }
                None => KeyEventResult::Ignored,
            })));
        Self::put(&self.main, 0);
        Self::put(&self.confirm, 0);
    }

    fn dispose(&mut self) {
        self.node.set_on_key_event(None);
        self.main.dispose();
        self.confirm.dispose();
    }

    fn did_change_dependencies(&mut self, _: &MemoriesSheet, cx: &mut Cx<'_, Self>) {
        let dialog = open_dialog(cx);
        if dialog.is_none() && self.shown {
            close_dialog::<_, ()>(cx, None);
        }
        self.shown = dialog.is_some();
    }

    fn update(&mut self, msg: Msg, _: &MemoriesSheet, cx: &mut Cx<'_, Self>) -> Rebuild {
        let Some(dialog) = open_dialog(cx) else {
            return Rebuild::No;
        };
        match msg {
            Msg::Act(act) => self.act(act, &dialog, cx),
            Msg::Row(row) => {
                let (cursor, len) = self.active(&dialog);
                if cursor.cursor() == Some(row) {
                    let accept = dialog.confirming || row == RESET_ROW;
                    self.act(if accept { Act::Accept } else { Act::Toggle }, &dialog, cx);
                } else if row < len {
                    Self::put(cursor, row);
                }
            }
        }
        Rebuild::Yes
    }

    fn build(&self, _: &MemoriesSheet, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let s = strings::of(cx);
        let tokens = Tokens::of(cx);
        let seg = segmenter_of(cx);
        let Some(dialog) = open_dialog(cx) else {
            return Dialog::bare(Text::new(String::new())).into();
        };
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

        let (cursor, _) = self.active(&dialog);
        let at = cursor.cursor();
        let rows = rows_of(s, &dialog);
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

        let (title, subtitle, hint) = if dialog.confirming {
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
        if dialog.confirming {
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

fn rows_of(s: &Strings, dialog: &Memories) -> Vec<MemoryRow> {
    if dialog.confirming {
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
