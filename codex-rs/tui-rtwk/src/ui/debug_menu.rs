//! The debug menu (`codex-rtwk --debug-screens`): every screen slot, its
//! kind and its entries — the states of the screen on made-up data a slice
//! registers in its directory's `debug_entries`. Enter opens the highlighted
//! one; the up and down keys move; Esc or q quits.

use rtwk::focus::{Focus, KeyPress};
use rtwk::layout::{Column, Expanded};
use rtwk::navigation::{Navigator, PageRoute};
use rtwk::overlay::{Dialog, DialogOptions, show_dialog};
use rtwk::provider::read;
use rtwk::scroll::ListView;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use crate::screens::{self, DebugOpen, Kind};
use crate::theme;
use crate::ui::view::Hs;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DebugMenu;

/// One line of the menu: a directory, or one of its entries.
struct Row {
    label: String,
    open: Option<DebugOpen>,
    heading: bool,
}

fn rows() -> Vec<Row> {
    let mut rows = Vec::new();
    for dir in screens::dirs() {
        let kind = match dir.kind {
            Kind::Home => "home",
            Kind::Route => "route",
            Kind::Dialog => "dialog",
            Kind::Part => "part",
        };
        rows.push(Row {
            label: format!("{:<11} {:<8} {:<6} {}", dir.name, dir.id, kind, dir.title),
            open: Some((dir.open_empty)()),
            heading: true,
        });
        for entry in (dir.debug_entries)() {
            rows.push(Row {
                label: format!("    {} - {}", entry.name, entry.description),
                open: Some(entry.open),
                heading: false,
            });
        }
    }
    rows
}

pub struct DebugMenuState {
    selected: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MenuMsg {
    Up,
    Down,
    Open,
    Quit,
}

impl StatefulWidget for DebugMenu {
    type State = DebugMenuState;

    fn create_state(&self) -> DebugMenuState {
        DebugMenuState { selected: 0 }
    }
}

impl_widget!(stateful eq DebugMenu);

impl State for DebugMenuState {
    type Widget = DebugMenu;
    type Msg = MenuMsg;

    fn update(&mut self, msg: MenuMsg, _: &DebugMenu, cx: &mut Cx<'_, Self>) -> Rebuild {
        let count = rows().len();
        match msg {
            MenuMsg::Up => {
                self.selected = self.selected.saturating_sub(1);
                Rebuild::Yes
            }
            MenuMsg::Down => {
                self.selected = (self.selected + 1).min(count.saturating_sub(1));
                Rebuild::Yes
            }
            MenuMsg::Open => {
                if let Some(Row {
                    open: Some(open), ..
                }) = rows().into_iter().nth(self.selected)
                {
                    match open {
                        DebugOpen::Route(build) => {
                            // The route's result is not awaited: a debug page.
                            drop(
                                Navigator::of(cx)
                                    .push::<()>(PageRoute::<()>::new(move |_| build())),
                            );
                        }
                        DebugOpen::Dialog(build) => {
                            drop(show_dialog::<(), _>(
                                cx,
                                move |_| Dialog::new(build()).into(),
                                DialogOptions::default(),
                            ));
                        }
                    }
                }
                Rebuild::No
            }
            MenuMsg::Quit => {
                let hs = read::<Hs, _>(cx);
                cx.emit(&hs.quit, ());
                Rebuild::No
            }
        }
    }

    fn build(&self, _: &DebugMenu, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let rows = rows();
        let selected = self.selected;
        let list = ListView::builder(rows.len(), move |index| {
            let Some(row) = rows.get(index) else {
                return AnyWidget::default();
            };
            let marker = if index == selected { "> " } else { "  " };
            let style = match (index == selected, row.heading) {
                (true, _) => theme::selected(),
                (false, true) => theme::bold(),
                (false, false) => theme::dim(),
            };
            Text::new(format!("{marker}{}", row.label))
                .style(style)
                .into()
        });
        Focus::new(
            Column::new()
                .child(Text::new("codex-rtwk debug screens").style(theme::accent()))
                .child(Text::new("up/down move, enter opens, q quits").style(theme::dim()))
                .child(Expanded::new(1, list)),
        )
        .autofocus()
        .bind(KeyPress::UP, cx.send(MenuMsg::Up))
        .bind(KeyPress::DOWN, cx.send(MenuMsg::Down))
        .bind(KeyPress::ENTER, cx.send(MenuMsg::Open))
        .bind('q', cx.send(MenuMsg::Quit))
        .into()
    }
}
