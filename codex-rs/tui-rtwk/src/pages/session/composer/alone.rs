//! The composer alone with its footer (`composer/alone`): the real composer
//! and the real key footer at the screen's size and nothing else, as the
//! oracle's composer harness draws them (`snapshot_composer_state_with_width`:
//! a `ChatComposer` rendered over `footer_height + 8` rows). The page's
//! bottom pane is the same two widgets under a transcript and a status row.

use std::rc::Rc;

use rtwk::focus::FocusNode;
use rtwk::layout::{BoxWidget, Column, CrossAxisAlignment, EdgeInsets, Expanded, MainAxisSize};
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use crate::app::theme;
use crate::pages::session::footer::{Footer, Line};
use crate::widgets::thread_ui::ThreadUi;

use super::{Composer, search};

/// The composer of `ui` over its footer, filling the screen.
#[derive(Clone)]
pub struct Alone {
    pub ui: Rc<ThreadUi>,
}

impl PartialEq for Alone {
    fn eq(&self, other: &Alone) -> bool {
        Rc::ptr_eq(&self.ui, &other.ui)
    }
}

impl_widget!(stateful eq Alone);

/// What the footer reads of the draft, kept by the state as the page keeps it.
pub struct AloneState {
    node: FocusNode,
    draft_empty: bool,
    draft_shell: bool,
}

impl StatefulWidget for Alone {
    type State = AloneState;

    fn create_state(&self) -> AloneState {
        AloneState {
            node: FocusNode::new(),
            draft_empty: self.ui.composer.text().is_empty(),
            draft_shell: self.ui.draft.shell(),
        }
    }
}

impl State for AloneState {
    type Widget = Alone;
    type Msg = Rc<str>;

    fn update(&mut self, _: Rc<str>, w: &Alone, _: &mut Cx<'_, Self>) -> Rebuild {
        self.draft_shell = w.ui.draft.shell();
        self.draft_empty = w.ui.composer.text().is_empty() && !self.draft_shell;
        Rebuild::Yes
    }

    fn build(&self, w: &Alone, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let tokens = theme::Tokens::of(cx);
        let composer = Composer::new(w.ui.clone(), self.node.clone()).on_changed(cx.map(|t| t));
        let footer: AnyWidget = if let Some((query, status)) = w.ui.draft.search_state() {
            search::footer(cx, query, status)
        } else {
            Footer {
                line: Line::Passive,
                agents: false,
                draft_empty: self.draft_empty,
                shell: self.draft_shell,
                on_queue: None,
                on_warnings: None,
                on_close: None,
            }
            .into()
        };
        Column::new()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .main_axis_size(MainAxisSize::Max)
            .child(Expanded::new(
                1,
                BoxWidget::new()
                    .style(tokens.prompt_surface(cx, false))
                    .padding(EdgeInsets::only(0, 1, 1, 1))
                    .child(composer),
            ))
            .child(footer)
            .into()
    }
}
