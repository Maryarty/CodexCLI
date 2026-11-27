//! The workspace strip addresses every open child independently.

use std::cell::RefCell;
use std::rc::Rc;

use codex_tui_core::ids::ThreadId;
use codex_tui_core::threads::{Item, ThreadModel};
use rtwk::controls::{Button, Pressable};
use rtwk::focus::{Focus, FocusNode, KeyEventResult};
use rtwk::gestures::{Listener, PointerEvent};
use rtwk::input::MouseKind;
use rtwk::l10n::{Arg, Fmt};
use rtwk::layout::{Axis, BoxWidget, EdgeInsets, Expanded, MainAxisSize, Row};
use rtwk::prelude::HitTestBehavior;
use rtwk::scroll::SingleChildScrollView;
use rtwk::style::Theme;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, Key, Str};

use crate::keymap::press::{PaneActivationGuard, PaneActivationKey, horizontal_focus};

use crate::widgets::thread_ui::ThreadUi;
use rtwk::provider::read;

use super::side_pane::{DockState, Msg};

/// The first displayed prompt remains the unnamed tab's label when later turns are sent.
#[derive(Clone, Default)]
struct FirstPrompt(Rc<RefCell<Option<Str>>>);

pub(super) fn tab_name(thread: &ThreadModel, ui: &ThreadUi) -> Option<Str> {
    let line = |text: &str| {
        text.lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .map(Str::copy)
    };
    if let Some(name) = thread.screen_slots().chat.name.as_deref().and_then(line) {
        return Some(name);
    }
    let cached = ui.held(FirstPrompt::default);
    if let Some(name) = cached.0.borrow().as_ref() {
        return Some(name.clone());
    }
    let name = thread.transcript().iter().find_map(|(_, item)| match item {
        Item::User(prompt) => line(&prompt.shown().text),
        _ => None,
    });
    *cached.0.borrow_mut() = name.clone();
    name
}

pub(super) fn view(
    cx: &mut Cx<'_, DockState>,
    focus: &FocusNode,
    selected: Option<&ThreadId>,
    children: &[ThreadId],
    names: &[Option<Str>],
) -> AnyWidget {
    let words = super::strings::of(cx);
    let mut items = Row::new().main_axis_size(MainAxisSize::Min);
    for (index, (child, name)) in children.iter().zip(names).enumerate() {
        let label = name.clone().unwrap_or_else(|| {
            if children.len() == 1 {
                Str::Static(words.side_chat.trim())
            } else {
                Fmt::new(words.side_tab, [Arg::Int(index as u64 + 1)], &words.num)
                    .to_string()
                    .into()
            }
        });
        let selected = selected == Some(child);
        let select = Pressable::new(move |cx, _| {
            let theme = Theme::of(cx);
            let style = if selected {
                &theme.selection
            } else {
                &theme.primary
            };
            BoxWidget::new()
                .padding(EdgeInsets::x(1))
                .child(Text::line(label.clone()).style(style.clone()))
                .into()
        })
        .can_request_focus(false)
        .behavior(HitTestBehavior::Opaque)
        .on_tap({
            let child = child.clone();
            cx.map(move |_| Msg::SelectTab(child.clone()))
        });
        let close = Button::new(words.side_close_tab_symbol)
            .can_request_focus(false)
            .on_pressed({
                let child = child.clone();
                cx.map(move |()| Msg::CloseTab(child.clone()))
            });
        let middle = child.clone();
        let item = Listener::new()
            .behavior(HitTestBehavior::Translucent)
            .on_pointer_down(cx.map(move |event: PointerEvent| {
                if event.button == 1 {
                    Msg::CloseTab(middle.clone())
                } else {
                    Msg::Ignore
                }
            }))
            .child(
                Row::new()
                    .main_axis_size(MainAxisSize::Min)
                    .child(select)
                    .child(close),
            );
        items = items.child(AnyWidget::from(item).with_key(Key::value(child.clone())));
    }
    let list = SingleChildScrollView::new(items)
        .scroll_direction(Axis::Horizontal)
        .can_request_focus(false);
    let tell = cx.local_sender();
    let activation = read::<PaneActivationGuard, _>(cx);
    let list = Focus::new(list)
        .focus_node(focus.clone())
        .on_key_event(move |node, key| {
            if !node.has_primary_focus() {
                return KeyEventResult::Ignored;
            }
            if let Some(right) = horizontal_focus(&key) {
                tell(Msg::StepTab(if right { 1 } else { -1 }));
            } else if PaneActivationKey::of_press(&key) == Some(PaneActivationKey::Enter) {
                activation.arm(PaneActivationKey::Enter);
                tell(Msg::FocusChild);
            } else {
                return KeyEventResult::Ignored;
            }
            KeyEventResult::Handled
        });
    let add = Button::new(words.side_add_tab_symbol).on_pressed(cx.map(|()| Msg::FocusTabs));
    Listener::new()
        .behavior(HitTestBehavior::Opaque)
        .scroll_axis(Axis::Vertical)
        .on_scroll(cx.map(|event: PointerEvent| match event.kind {
            MouseKind::WheelUp => Msg::StepTab(-1),
            MouseKind::WheelDown => Msg::StepTab(1),
            _ => Msg::Ignore,
        }))
        .child(Row::new().child(Expanded::new(1, list)).child(add))
        .into()
}
