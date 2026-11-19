//! The generic selection dialog: a title, a list of rows (number, name, a
//! mark for the row in use, a description column that wraps) and a footer
//! hint, over the menu the model holds. Enter and the digits pick a row, Esc
//! closes. Oracle: `bottom_pane/list_selection_view.rs`.

use std::rc::Rc;

use rtwk::focus::Focus;
use rtwk::layout::{Column, MainAxisSize};
use rtwk::overlay::{Dialog, close_dialog};
use rtwk::provider::{read, select};
use rtwk::style::{Attrs, Style};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::ScreenStates;
use codex_tui_core::screens::select::{Access, Intent, Menu};

use super::strings::{self, Strings};
use crate::theme;
use crate::ui::options::{OptionRow, OptionStyles, Options};
use crate::ui::view::Hs;

#[derive(Clone, Debug, PartialEq)]
pub struct MenuDialog;

pub struct MenuDialogState {
    shown: bool,
    options: Options,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Pick(usize),
    Move(i64),
    Enter,
    Close,
}

/// One row of the list: its label (`1. name (current)`), the description.
#[derive(Clone)]
struct RowView {
    label: String,
    description: String,
}

impl StatefulWidget for MenuDialog {
    type State = MenuDialogState;

    fn create_state(&self) -> MenuDialogState {
        MenuDialogState {
            shown: false,
            options: Options::new(),
        }
    }
}

impl_widget!(stateful eq MenuDialog);

impl State for MenuDialogState {
    type Widget = MenuDialog;
    type Msg = Msg;

    fn dispose(&mut self) {
        self.options.dispose();
    }

    fn did_change_dependencies(&mut self, _: &MenuDialog, cx: &mut Cx<'_, Self>) {
        let present = select::<ScreenStates, bool, _>(cx, |states| states.select.menu.is_some());
        if !present && self.shown {
            close_dialog::<_, ()>(cx, None);
        }
        self.shown = present;
    }

    fn update(&mut self, msg: Msg, _: &MenuDialog, cx: &mut Cx<'_, Self>) -> Rebuild {
        let hs = read::<Hs, _>(cx);
        match msg {
            Msg::Move(delta) => {
                let len = select::<ScreenStates, usize, _>(cx, |states| {
                    states.select.menu.as_ref().map_or(0, Menu::len)
                });
                self.options.step(delta, len);
            }
            Msg::Enter => {
                cx.emit(
                    &hs.screen,
                    ScreenIntent::Select(Intent::Pick(self.options.cursor())),
                );
            }
            // The model drops the menu on a pick; the dialog closes with it.
            Msg::Pick(index) => {
                cx.emit(&hs.screen, ScreenIntent::Select(Intent::Pick(index)));
            }
            Msg::Close => {
                cx.emit(&hs.screen, ScreenIntent::Select(Intent::Close));
                close_dialog::<_, ()>(cx, None);
            }
        }
        Rebuild::No
    }

    fn build(&self, _: &MenuDialog, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let menu = select::<ScreenStates, Option<Menu>, _>(cx, |states| states.select.menu.clone());
        let Some(menu) = &menu else {
            return AnyWidget::default();
        };
        let tokens = theme::Tokens::of(cx);
        let s = strings::of(cx);
        let (title, hint, rows) = parts(menu, s);
        let count = rows.len();
        let rows = Rc::new(
            rows.into_iter()
                .map(|row| OptionRow {
                    label: row.label,
                    description: Some(row.description),
                })
                .collect::<Vec<_>>(),
        );
        let list = self.options.view(
            rows,
            OptionStyles {
                cursor: tokens.accent().attrs(Attrs::BOLD),
                label: Style::new(),
                description: tokens.dim(),
            },
            0,
        );
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .child(list);
        if let Some(hint) = hint {
            column = column
                .child(Text::new(String::new()))
                .child(Text::new(hint.to_string()).style(tokens.dim()));
        }
        let mut focus = Focus::new(Dialog::new(column).max_width(Some(83)).title(title.to_string()))
            .can_request_focus(false)
            .bind(rtwk::focus::KeyPress::ESC, cx.send(Msg::Close))
            .bind(rtwk::focus::KeyPress::UP, cx.send(Msg::Move(-1)))
            .bind(rtwk::focus::KeyPress::DOWN, cx.send(Msg::Move(1)))
            .bind(rtwk::focus::KeyPress::ENTER, cx.send(Msg::Enter));
        for index in 0..count.min(9) {
            if let Some(digit) = char::from_digit(u32::try_from(index + 1).unwrap_or(0), 10) {
                focus = focus.bind(digit, cx.send(Msg::Pick(index)));
            }
        }
        focus.into()
    }
}

/// The title, the footer hint and the rows of a menu.
fn parts(menu: &Menu, s: &Strings) -> (&'static str, Option<&'static str>, Vec<RowView>) {
    match menu {
        Menu::Models(models) => (
            s.model_title,
            None,
            models
                .iter()
                .enumerate()
                .map(|(index, row)| RowView {
                    label: format!(
                        "{}. {}{}",
                        index + 1,
                        row.name,
                        if row.current { s.current } else { "" }
                    ),
                    description: row.description.clone(),
                })
                .collect(),
        ),
        Menu::Permissions { current } => {
            let rows = [
                (Access::Ask, s.access_ask, s.access_ask_description),
                (Access::Full, s.access_full, s.access_full_description),
            ];
            (
                s.permissions_title,
                Some(s.permissions_hint),
                rows.iter()
                    .enumerate()
                    .map(|(index, (access, name, description))| RowView {
                        label: format!(
                            "{}. {name}{}",
                            index + 1,
                            if Some(access) == current.as_ref() { s.current } else { "" }
                        ),
                        description: (*description).to_string(),
                    })
                    .collect(),
            )
        }
    }
}
