//! Terminal effects for the root, delivered through rtwk's binding.

use std::cell::Cell;
use std::rc::Rc;

use rtwk::binding::{AppBinding, InputListener};
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};
use rtwk_core::input::InputEvent;

use codex_tui_core::screens::chat::{DesktopNotice, NoticeKind};

#[derive(Clone)]
pub(super) struct TerminalEffects {
    pub child: AnyWidget,
    pub notice: Option<Rc<DesktopNotice>>,
    pub always: bool,
}

impl PartialEq for TerminalEffects {
    fn eq(&self, other: &Self) -> bool {
        self.child == other.child
            && self.always == other.always
            && same_notice(&self.notice, &other.notice)
    }
}

fn same_notice(left: &Option<Rc<DesktopNotice>>, right: &Option<Rc<DesktopNotice>>) -> bool {
    match (left, right) {
        (Some(a), Some(b)) => Rc::ptr_eq(a, b),
        (None, None) => true,
        _ => false,
    }
}

pub(super) struct TerminalEffectsState {
    focused: Rc<Cell<bool>>,
}

impl StatefulWidget for TerminalEffects {
    type State = TerminalEffectsState;

    fn create_state(&self) -> Self::State {
        TerminalEffectsState {
            focused: Rc::new(Cell::new(true)),
        }
    }
}

impl State for TerminalEffectsState {
    type Widget = TerminalEffects;
    type Msg = ();

    fn did_update_widget(
        &mut self,
        old: &TerminalEffects,
        w: &TerminalEffects,
        cx: &mut Cx<'_, Self>,
    ) {
        if same_notice(&old.notice, &w.notice) || self.focused.get() && !w.always {
            return;
        }
        if let Some(notice) = &w.notice
            && let Some(binding) = AppBinding::maybe_of(cx)
        {
            let title = match notice.kind {
                NoticeKind::TurnComplete => "Agent turn complete",
                NoticeKind::CommandApproval => "Approval requested",
                NoticeKind::EditApproval => "Codex wants to edit files",
                NoticeKind::Elicitation => "Approval requested by a tool",
                NoticeKind::Question => "Input requested",
            };
            binding.notify(title, notice.preview.as_deref().unwrap_or(""));
        }
    }

    fn update(&mut self, (): (), _w: &TerminalEffects, _cx: &mut Cx<'_, Self>) -> Rebuild {
        Rebuild::No
    }

    fn build(&self, w: &TerminalEffects, _cx: &mut Cx<'_, Self>) -> AnyWidget {
        let focused = self.focused.clone();
        InputListener::new(w.child.clone(), move |event| {
            if let InputEvent::Focus(value) = event {
                focused.set(*value);
            }
            false
        })
        .into()
    }
}

impl_widget!(stateful eq TerminalEffects);
