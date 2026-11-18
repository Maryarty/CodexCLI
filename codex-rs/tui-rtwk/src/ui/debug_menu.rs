//! The debug menu (`codex-rtwk --debug-screens`): every screen slot, its
//! kind and its entries — the states of the screen on made-up data a slice
//! registers in its directory's `debug_entries`. Enter opens the highlighted
//! one; the up and down keys move; Esc or q quits.

use std::rc::Rc;

use rtwk::controls::SelectionList;
use rtwk::focus::Focus;
use rtwk::layout::{Column, Expanded};
use rtwk::navigation::{Navigator, PageRoute};
use rtwk::overlay::{Dialog, DialogOptions, show_dialog};
use rtwk::provider::read;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use crate::l10n::Lang;
use crate::l10n::lang;
use crate::l10n::strings as l10n;
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

fn rows(lang: Lang) -> Vec<Row> {
    let mut rows = Vec::new();
    for dir in screens::dirs() {
        let kind = match dir.kind {
            Kind::Home => "home",
            Kind::Route => "route",
            Kind::Dialog => "dialog",
            Kind::Part => "part",
        };
        rows.push(Row {
            label: format!(
                "{:<11} {:<8} {:<6} {}",
                dir.name,
                dir.id,
                kind,
                (dir.title)(lang)
            ),
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

pub struct DebugMenuState;

#[derive(Clone, Debug, PartialEq)]
pub enum MenuMsg {
    /// The row at this index was submitted (Enter, or a click).
    Open(usize, Lang),
    /// `q`: back to the screen below, or quit from the home.
    Back,
}

impl StatefulWidget for DebugMenu {
    type State = DebugMenuState;

    fn create_state(&self) -> DebugMenuState {
        DebugMenuState
    }
}

impl_widget!(stateful eq DebugMenu);

impl State for DebugMenuState {
    type Widget = DebugMenu;
    type Msg = MenuMsg;

    fn update(&mut self, msg: MenuMsg, _: &DebugMenu, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            MenuMsg::Open(index, lang) => {
                if let Some(Row {
                    open: Some(open), ..
                }) = rows(lang).into_iter().nth(index)
                {
                    match open {
                        DebugOpen::Route(build) => {
                            // The route's result is not awaited: a debug page.
                            drop(
                                Navigator::of(cx)
                                    .push::<()>(PageRoute::<()>::new(move |_| build(lang))),
                            );
                        }
                        DebugOpen::Dialog(build) => {
                            drop(show_dialog::<(), _>(
                                cx,
                                move |_| Dialog::new(build(lang)).into(),
                                DialogOptions::default(),
                            ));
                        }
                    }
                }
            }
            MenuMsg::Back => {
                // Back when there is a route to go back to; the menu as the
                // home (`--debug-screens`) quits.
                if !Navigator::maybe_pop(cx) {
                    let hs = read::<Hs, _>(cx);
                    cx.emit(&hs.quit, ());
                }
            }
        }
        Rebuild::No
    }

    fn build(&self, _: &DebugMenu, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let tokens = theme::Tokens::of(cx);
        let strings = l10n::of(cx);
        let lang = lang(cx);
        let hint = if Navigator::can_pop(cx) {
            strings.debug.hint
        } else {
            strings.debug.hint_home
        };
        let rows = Rc::new(rows(lang));
        let count = rows.len();
        let row_tokens = tokens.clone();
        // rtwk's selection list owns the cursor and scrolls it into view
        // (R11): the menu has more rows than a terminal has lines.
        let list = SelectionList::new(
            count,
            |index| index,
            move |index| {
                let Some(row) = rows.get(index) else {
                    return AnyWidget::default();
                };
                let style = if row.heading {
                    row_tokens.bold()
                } else {
                    row_tokens.dim()
                };
                Text::new(row.label.clone()).style(style).into()
            },
        )
        .on_submitted(cx.map(move |index| MenuMsg::Open(index, lang)))
        .autofocus(true);
        Focus::new(
            Column::new()
                .child(Text::new(strings.debug.title).style(tokens.accent()))
                .child(Text::new(hint).style(tokens.dim()))
                .child(Expanded::new(1, list)),
        )
        .bind('q', cx.send(MenuMsg::Back))
        .into()
    }
}
