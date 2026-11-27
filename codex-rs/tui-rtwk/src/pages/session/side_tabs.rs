//! The native tab controller presents the core's ordered, open children for one parent.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use codex_tui_core::ids::ThreadId;
use rtwk::controls::{Button, TabBar, TabController};
use rtwk::focus::{FocusNode, KeyBindings};
use rtwk::foundation::{Listenable, VoidCallback};
use rtwk::l10n::{Arg, Fmt};
use rtwk::layout::{Expanded, Row};
use rtwk::{AnyWidget, Cx};

use super::side_pane::{DockState, Msg};

pub(super) struct Tabs {
    children: Rc<RefCell<Rc<[ThreadId]>>>,
    projecting: Rc<Cell<bool>>,
    controller: TabController,
    listener: Option<VoidCallback>,
    pub focus: FocusNode,
}

impl Tabs {
    pub fn new(children: Rc<[ThreadId]>, selected: &ThreadId) -> Self {
        let index = children.iter().position(|id| id == selected).unwrap_or(0);
        Self {
            controller: TabController::with_initial_index(children.len(), index),
            children: Rc::new(RefCell::new(children)),
            projecting: Rc::new(Cell::new(false)),
            listener: None,
            focus: FocusNode::new(),
        }
    }

    pub fn listen(&mut self, tell: Rc<dyn Fn(Msg)>) {
        let children = self.children.clone();
        let projecting = self.projecting.clone();
        let controller = self.controller.clone();
        let listener: VoidCallback = Rc::new(move || {
            if !projecting.get()
                && let Some(id) = children.borrow().get(controller.index())
            {
                tell(Msg::SelectTab(id.clone()));
            }
        });
        self.controller.add_listener(listener.clone());
        self.listener = Some(listener);
    }

    pub fn project(&self, children: &Rc<[ThreadId]>, selected: &ThreadId) {
        let Some(index) = children.iter().position(|id| id == selected) else {
            return;
        };
        self.projecting.set(true);
        *self.children.borrow_mut() = children.clone();
        self.controller.set_length(children.len());
        self.controller.set_index(index);
        self.projecting.set(false);
    }

    pub fn dispose(&mut self) {
        if let Some(listener) = self.listener.take() {
            self.controller.remove_listener(&listener);
        }
        self.controller.dispose();
        self.focus.dispose();
    }

    pub fn view(&self, cx: &mut Cx<'_, DockState>, selected: &ThreadId) -> AnyWidget {
        let words = super::strings::of(cx);
        let count = self.controller.length();
        let labels = (1..=count).map(|index| {
            if count == 1 {
                words.side_chat.trim().to_owned()
            } else {
                Fmt::new(words.side_tab, [Arg::Int(index as u64)], &words.num).to_string()
            }
        });
        let tabs = TabBar::strip(&self.controller, labels).focus_node(Some(self.focus.clone()));
        let tabs = KeyBindings::new([("enter", cx.send(Msg::FocusChild))], tabs);
        let close = Button::new(words.side_close_tab_symbol)
            .can_request_focus(false)
            .on_pressed(cx.send(Msg::CloseTab(selected.clone())));
        Row::new().child(Expanded::new(1, tabs)).child(close).into()
    }
}
