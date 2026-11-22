//! The experimental features (the oracle's `ExperimentalFeaturesView`:
//! `bottom_pane/experimental_features_view.rs`): a title, a subtitle, a line
//! for the state of discovery and of the write, checkable rows whose
//! descriptions wrap under their own column, and a key hint. In Flutter it
//! is a bottom sheet whose content is a `Column(min)` over a `ListView` with
//! `shrinkWrap` and a `ListController`; the cursor is a
//! `SelectionController` keyed by the feature, the keys come from the
//! `FocusNode.onKeyEvent` of a `Focus`. What is checked, what a write
//! sends and what its answer did are the model's (`Intent::Toggle`,
//! `Intent::Save`, `Ev::FeaturesSaved`, ADR-003); the sheet keeps the
//! controllers only. The mouse (ADR-011): a tap moves the cursor to a row,
//! a tap on the row the cursor is on flips it; the wheel is the `ListView`'s.

use std::rc::Rc;

use rtwk::controls::SelectionController;
use rtwk::focus::{Focus, FocusNode, KeyEventResult, KeyPress};
use rtwk::gestures::{GestureDetector, PointerEvent};
use rtwk::layout::{
    BoxWidget, Column, CrossAxisAlignment, EdgeInsets, Expanded, Flexible, MainAxisSize, Padding,
    Row, SizedBox,
};
use rtwk::overlay::{Dialog, close_dialog};
use rtwk::prelude::{KeyCode, Mods};
use rtwk::provider::{read, select};
use rtwk::scroll::{ListController, ListView};
use rtwk::style::Style;
use rtwk::text::{Text, TextWrap, segmenter_of};
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::ScreenStates;
use codex_tui_core::screens::settings::{Experimental, FeaturesStatus, Intent, Open};

use super::setup::{letter_of, move_of};
use super::strings::{self, Strings};
use crate::app::theme::Tokens;
use crate::app::view::Hs;
use crate::widgets::session_list::{Move, page_rows, target};

/// The sheet of the experimental features.
#[derive(Clone, Debug, PartialEq)]
pub struct FeaturesSheet;

/// Widget state: the controllers (R14).
pub struct FeaturesState {
    shown: bool,
    node: FocusNode,
    cursor: SelectionController<String>,
    list: ListController,
}

/// What a key means to the list (the oracle's list keymap and
/// `ExperimentalFeaturesView::handle_key_event`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    Move(Move),
    Toggle,
    Save,
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
        (KeyCode::Enter, _, false, false) => Some(Act::Save),
        (KeyCode::Escape, _, false, false) => Some(Act::Close),
        // Ctrl+C is the view's cancel too (`on_ctrl_c`).
        (KeyCode::Char, Some('c'), true, false) => Some(Act::Close),
        _ => None,
    }
}

impl StatefulWidget for FeaturesSheet {
    type State = FeaturesState;

    fn create_state(&self) -> FeaturesState {
        FeaturesState {
            shown: false,
            node: FocusNode::new(),
            cursor: SelectionController::new(),
            list: ListController::new(),
        }
    }
}

impl_widget!(stateful eq FeaturesSheet);

/// The dialog the model holds open.
fn open_dialog(cx: &mut Cx<'_, FeaturesState>) -> Option<Experimental> {
    select::<ScreenStates, Option<Experimental>, _>(cx, |states| match &states.settings.open {
        Some(Open::Experimental(dialog)) => Some(dialog.clone()),
        _ => None,
    })
}

impl FeaturesState {
    /// The row the cursor is on.
    fn at(&self, dialog: &Experimental) -> Option<usize> {
        let key = self.cursor.cursor()?;
        dialog.features.iter().position(|f| f.key == key)
    }

    fn put(&self, dialog: &Experimental, at: usize) {
        let key = dialog.features[at].key.clone();
        self.cursor.select(key.clone());
        self.cursor.set_cursor(Some(key));
        self.list.ensure_index_visible(at);
    }

    fn act(&self, act: Act, dialog: &Experimental, cx: &mut Cx<'_, Self>) {
        let hs = read::<Hs, _>(cx);
        let at = self.at(dialog);
        match act {
            Act::Move(mv) => {
                if let Some(to) = target(
                    dialog.features.len(),
                    at,
                    &|_| true,
                    mv,
                    page_rows(&self.list),
                ) {
                    self.put(dialog, to);
                }
            }
            Act::Toggle => {
                if let Some(at) = at {
                    let key = dialog.features[at].key.clone();
                    cx.emit(&hs.screen, ScreenIntent::Settings(Intent::Toggle(key)));
                }
            }
            Act::Save => cx.emit(&hs.screen, ScreenIntent::Settings(Intent::Save)),
            Act::Close => cx.emit(&hs.screen, ScreenIntent::Settings(Intent::Close)),
        }
    }
}

impl State for FeaturesState {
    type Widget = FeaturesSheet;
    type Msg = Msg;

    fn init_state(&mut self, _: &FeaturesSheet, cx: &mut Cx<'_, Self>) {
        let send = cx.local_sender();
        self.node
            .set_on_key_event(Some(Rc::new(move |_, key| match act_of(&key) {
                Some(act) => {
                    send(Msg::Act(act));
                    KeyEventResult::Handled
                }
                None => KeyEventResult::Ignored,
            })));
    }

    fn dispose(&mut self) {
        self.node.set_on_key_event(None);
        self.cursor.dispose();
    }

    fn did_change_dependencies(&mut self, _: &FeaturesSheet, cx: &mut Cx<'_, Self>) {
        let dialog = open_dialog(cx);
        if dialog.is_none() && self.shown {
            close_dialog::<_, ()>(cx, None);
        }
        self.shown = dialog.is_some();
        // The cursor takes the first row when there is one and it has none
        // (`initialize_selection`; the catalogue may arrive after the open).
        if let Some(dialog) = &dialog
            && self.at(dialog).is_none()
            && !dialog.features.is_empty()
        {
            self.put(dialog, 0);
        }
    }

    fn update(&mut self, msg: Msg, _: &FeaturesSheet, cx: &mut Cx<'_, Self>) -> Rebuild {
        let Some(dialog) = open_dialog(cx) else {
            return Rebuild::No;
        };
        match msg {
            Msg::Act(act) => self.act(act, &dialog, cx),
            Msg::Row(row) => {
                if self.at(&dialog) == Some(row) {
                    self.act(Act::Toggle, &dialog, cx);
                } else if row < dialog.features.len() {
                    self.put(&dialog, row);
                }
            }
        }
        Rebuild::Yes
    }

    fn build(&self, _: &FeaturesSheet, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let s = strings::of(cx);
        let tokens = Tokens::of(cx);
        let seg = segmenter_of(cx);
        let Some(dialog) = open_dialog(cx) else {
            return Dialog::bare(Text::new(String::new())).into();
        };
        let status = match &dialog.status {
            FeaturesStatus::Quiet => None,
            FeaturesStatus::Loading => Some(s.features_loading),
            FeaturesStatus::Saving => Some(s.features_saving),
            FeaturesStatus::NoExperiments => Some(s.features_none),
            FeaturesStatus::Unavailable => Some(s.features_unavailable),
            FeaturesStatus::Message(text) => Some(text.as_str()),
        };
        let hint = if dialog.saving {
            s.features_hint_saving
        } else if !dialog.unconfirmed.is_empty() {
            s.features_hint_retained
        } else {
            s.features_hint
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

        // The name column is as wide as the widest name (the oracle's is the
        // widest in view: the same up to the eight rows it shows at once).
        let wide = dialog
            .features
            .iter()
            .map(|f| seg.width(&name_of(s, &f.name, f.writable)))
            .max()
            .unwrap_or(0);
        let wide = i32::try_from(wide).unwrap_or(0);

        let taps: Rc<Vec<_>> = Rc::new(
            (0..dialog.features.len())
                .map(|row| cx.map(move |_: PointerEvent| Msg::Row(row)))
                .collect(),
        );
        let body: AnyWidget = if dialog.features.is_empty() {
            // While the catalogue is on its way the row is reserved and
            // left blank; after, the empty message stands in it.
            if dialog.loading {
                Text::new(String::new()).into()
            } else {
                Text::line(format!("{}{}", s.indent, s.features_no_rows))
                    .style(tokens.dim())
                    .into()
            }
        } else {
            let dialog = Rc::new(dialog.clone());
            let cursor = self.cursor.clone();
            let tokens = tokens.clone();
            Flexible::new(
                1,
                ListView::builder(dialog.features.len(), move |at: usize| -> AnyWidget {
                    let row = feature_row(s, &dialog, at, wide, &cursor, &tokens);
                    match taps.get(at) {
                        Some(tap) => GestureDetector::new().on_tap(tap.clone()).child(row).into(),
                        None => row,
                    }
                })
                .shrink_wrap(true)
                .controller(self.list.clone()),
            )
            .into()
        };

        // Stretched: the sheet is as wide as the screen, not as its widest line.
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(Text::new(String::new()))
            .child(Text::line(format!("{}{}", s.indent, s.features_title)).style(tokens.bold()))
            .child(note(s.features_subtitle));
        if let Some(status) = status {
            column = column.child(note(status));
        }
        let column = column
            .child(Text::new(String::new()))
            .child(Text::new(String::new()))
            .child(body)
            .child(Text::new(String::new()))
            .child(Text::new(String::new()))
            .child(note(hint));
        // The surface is the sheet's own (`user_message_style`): a bare dialog
        // leaves what it does not paint to the app under it.
        let surface = tokens.prompt_surface(cx, false);
        Focus::new(Dialog::bare(BoxWidget::new().style(surface).child(column)))
            .focus_node(self.node.clone())
            .autofocus()
            .into()
    }
}

/// The name of a row: the name, and a mark on a feature that cannot be
/// flipped.
fn name_of(s: &Strings, name: &str, writable: bool) -> String {
    if writable {
        name.to_string()
    } else {
        format!("{name}{}", s.features_read_only)
    }
}

/// A row of a feature: the marker of the cursor, the check, the name in a
/// column of `wide` cells and the description beside it, wrapped under its
/// own column. A row that cannot be flipped (read-only, or a write on its
/// way) is dim and takes no selection colour.
fn feature_row(
    s: &'static Strings,
    dialog: &Experimental,
    at: usize,
    wide: i32,
    cursor: &SelectionController<String>,
    tokens: &Tokens,
) -> AnyWidget {
    let feature = &dialog.features[at];
    let on = cursor.cursor().as_ref() == Some(&feature.key);
    let disabled = !feature.writable || dialog.saving;
    let lead = format!(
        "{}{}",
        if on { s.cursor_marker } else { s.indent },
        if feature.enabled {
            s.check_on
        } else {
            s.check_off
        }
    );
    let (lead_style, name_style, description_style) = if disabled {
        (tokens.dim(), tokens.dim(), tokens.dim())
    } else if on {
        let selected = tokens.selected();
        (selected.clone(), selected.clone(), selected)
    } else {
        (Style::new(), Style::new(), tokens.dim())
    };
    Row::new()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .child(Text::line(lead).style(lead_style))
        .child(SizedBox::new(
            Some(wide + 2),
            None,
            Some(
                Text::line(name_of(s, &feature.name, feature.writable))
                    .style(name_style)
                    .into(),
            ),
        ))
        .child(Expanded::new(
            1,
            Text::new(feature.description.clone())
                .style(description_style)
                .wrap(TextWrap::Word),
        ))
        .into()
}
