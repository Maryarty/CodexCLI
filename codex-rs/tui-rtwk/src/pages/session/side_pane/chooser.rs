//! Native selection for the visible conversations and this parent's saved side history.

use std::rc::Rc;

use codex_tui_core::ids::ThreadId;
use rtwk::controls::{SelectionController, SelectionList};
use rtwk::focus::{Focus, FocusNode, KeyEventResult};
use rtwk::gestures::{Listener, PointerEvent};
use rtwk::layout::{Column, Expanded};
use rtwk::prelude::HitTestBehavior;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, Handler, Str};

use crate::features::SavedChats;
use crate::keymap::press::{PaneActivationGuard, PaneActivationKey, horizontal_focus, is_ctrl_c};

use super::{Dock, DockState, Msg};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum SideChoice {
    Create,
    Open(ThreadId),
    Retry,
}

struct Row {
    choice: SideChoice,
    label: Str,
    press: Handler<PointerEvent>,
}

impl Row {
    fn new(cx: &mut Cx<'_, DockState>, choice: SideChoice, label: Str) -> Self {
        let target = choice.clone();
        let press = cx.map(move |pointer: PointerEvent| {
            if pointer.button == 0 {
                Msg::Choose(target.clone(), false)
            } else {
                Msg::Ignore
            }
        });
        Self {
            choice,
            label,
            press,
        }
    }
}

pub(super) struct ChooserControls {
    pub(super) focus: FocusNode,
    selection: SelectionController<SideChoice>,
}

impl ChooserControls {
    pub(super) fn new() -> Self {
        Self {
            focus: FocusNode::new(),
            selection: SelectionController::with(false, [], Some(SideChoice::Create)),
        }
    }

    pub(super) fn view(&self, w: &Dock, cx: &mut Cx<'_, DockState>) -> AnyWidget {
        let words = crate::pages::session::strings::of(cx);
        let mut rows = vec![Row::new(
            cx,
            SideChoice::Create,
            Str::Static(words.side_saved_new),
        )];
        for child in w.children.iter() {
            rows.push(Row::new(
                cx,
                SideChoice::Open(child.clone()),
                w.name(child)
                    .unwrap_or_else(|| Str::Static(words.side_chat.trim())),
            ));
        }
        let mut title = Column::new().child(Text::line(words.side_saved_title));
        match &w.saved {
            None | Some(SavedChats::Loading) => {
                title = title.child(Text::line(words.side_saved_loading));
            }
            Some(SavedChats::Unavailable) => {
                title = title.child(Text::line(words.side_saved_unavailable));
            }
            Some(SavedChats::Failed(error)) => {
                title = title.child(Text::new(error.clone()));
                rows.push(Row::new(
                    cx,
                    SideChoice::Retry,
                    Str::Static(words.side_saved_retry),
                ));
            }
            Some(SavedChats::Ready(saved)) => {
                rows.extend(
                    saved
                        .iter()
                        .filter(|row| !w.children.contains(&row.id))
                        .map(|row| {
                            Row::new(
                                cx,
                                SideChoice::Open(row.id.clone()),
                                w.name(&row.id)
                                    .unwrap_or_else(|| Str::Static(words.side_chat.trim())),
                            )
                        }),
                );
            }
        }
        let rows: Rc<[Row]> = rows.into();
        let list = SelectionList::of(
            rows,
            |row| row.choice.clone(),
            |row| {
                Listener::new()
                    .behavior(HitTestBehavior::Translucent)
                    .on_pointer_down(row.press.clone())
                    .child(Text::line(row.label.clone()))
                    .into()
            },
        )
        .controller(self.selection.clone())
        .focus_node(self.focus.clone())
        .autofocus(true)
        .on_submitted(cx.map(|choice| Msg::Choose(choice, true)));
        let selection = self.selection.clone();
        let tell = cx.local_sender();
        let guard = rtwk::provider::read::<PaneActivationGuard, _>(cx);
        let has_live = w.child.is_some();
        let content = title.child(Expanded::new(1, list));
        Focus::new(content)
            .can_request_focus(false)
            .on_key_event(move |_, key| {
                if PaneActivationKey::of_press(&key) == Some(PaneActivationKey::Escape) {
                    guard.arm(PaneActivationKey::Escape);
                    tell(Msg::DismissChooser);
                } else if has_live && horizontal_focus(&key).is_some() {
                    tell(Msg::DismissChooser);
                } else if is_ctrl_c(&key) {
                    if let Some(SideChoice::Open(child)) = selection.cursor() {
                        tell(Msg::Archive(child));
                    }
                } else {
                    return KeyEventResult::Ignored;
                }
                KeyEventResult::Handled
            })
            .into()
    }
}

impl Drop for ChooserControls {
    fn drop(&mut self) {
        self.selection.dispose();
        self.focus.dispose();
    }
}
