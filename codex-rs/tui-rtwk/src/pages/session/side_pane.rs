//! The two retained chats share the workspace. The model's active thread
//! owns input while the other pane keeps its transcript and draft mounted.

use std::rc::Rc;

use codex_tui_core::ids::ThreadId;
use codex_tui_core::keymap::actions::{
    GLOBAL_CLOSE_SIDEBAR_TAB, GLOBAL_EQUALIZE_PANE_WIDTHS, GLOBAL_FOCUS_MAIN_COMPOSER,
    GLOBAL_FOCUS_PANE_LEFT, GLOBAL_FOCUS_PANE_RIGHT, GLOBAL_FOCUS_SIDEBAR_CONTENT,
    GLOBAL_HIDE_PANE, GLOBAL_MAXIMIZE_PANE_WIDTH, GLOBAL_MINIMIZE_PANE_WIDTH,
    GLOBAL_MOVE_DIVIDER_LEFT, GLOBAL_MOVE_DIVIDER_RIGHT, GLOBAL_RESIZE_PANES,
    GLOBAL_TOGGLE_PANE_ZOOM,
};
use codex_tui_core::keymap::{ActionId, Keymap};
use codex_tui_core::threads::ThreadModel;
use rtwk::binding::InputListener;
use rtwk::focus::{Focus, FocusNode, FocusScope, KeyBindings, KeyCombo, KeyEventResult, KeyPress};
use rtwk::gestures::{Listener, PointerEvent};
use rtwk::layout::{
    Column, Expanded, Offstage, SizedBox, SplitController, SplitPane, SplitView, Stack, StackFit,
};
use rtwk::prelude::HitTestBehavior;
use rtwk::provider::{Provider, read};
use rtwk::widgets::Text;
use rtwk::{
    AnyWidget, Cx, GlobalKey, Handler, Key, LayoutBuilder, Rebuild, State, StatefulWidget,
    impl_widget,
};
use rtwk_core::input::{InputEvent, KeyCode, Mods};

use crate::app::{UiApp, view::Hs};
use crate::features::{self, PageKey};
use crate::widgets::thread_ui::ThreadUi;

use super::session_page::{Chat, Presentation};

const SPLIT_AT: i32 = 100;
const PARENT_MIN: i32 = 63;
const CHILD_MIN: i32 = 36;

#[derive(Clone)]
struct Pane {
    model: Rc<ThreadModel>,
    ui: Rc<ThreadUi>,
}

impl PartialEq for Pane {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.model, &other.model) && Rc::ptr_eq(&self.ui, &other.ui)
    }
}

#[derive(Clone)]
struct PaneKey(GlobalKey);

#[derive(Clone)]
struct ComposerNode(FocusNode);

pub(super) fn composer_node(ui: &ThreadUi) -> FocusNode {
    ui.held(|| ComposerNode(FocusNode::new())).0
}

#[derive(Clone, PartialEq)]
pub(super) struct Dock {
    parent: Pane,
    child: Pane,
    active: ThreadId,
    preferred_width: Option<u8>,
}

impl Dock {
    pub(super) fn of(app: &UiApp) -> Option<Self> {
        let settings = app.core.tui();
        if !settings.transcript_owned || !settings.workspace_enabled {
            return None;
        }
        let active = app.core.session().active.as_ref()?;
        let side = &app.core.features().side;
        let (parent, child) = if let Some(link) = side.link(active) {
            (&link.parent, active)
        } else {
            (active, side.side_of(active)?)
        };
        let pane = |id| {
            Some(Pane {
                model: app.core.thread(id)?.clone(),
                ui: app.ui.thread_ui.get(Some(id)),
            })
        };
        Some(Self {
            parent: pane(parent)?,
            child: pane(child)?,
            active: active.clone(),
            preferred_width: settings.sidebar_width_percent,
        })
    }

    fn on_child(&self) -> bool {
        &self.active == self.child.model.id()
    }

    pub(super) fn keyed(self) -> AnyWidget {
        let key = Key::value((
            self.parent.model.id().clone(),
            self.child.model.id().clone(),
        ));
        AnyWidget::from(self).with_key(key)
    }
}

pub(super) struct DockState {
    width: i32,
    split: SplitController,
    hidden: bool,
    zoom: Option<ThreadId>,
    resizing: bool,
    resize_focus: FocusNode,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Msg {
    Width(i32),
    FocusParent,
    FocusChild,
    Hide,
    Zoom,
    Close,
    Step(i32),
    Share(u8),
    Resize,
    FinishResize,
    Ignore,
}

const ACTIONS: &[(ActionId, Msg)] = &[
    (GLOBAL_FOCUS_MAIN_COMPOSER, Msg::FocusParent),
    (GLOBAL_FOCUS_PANE_LEFT, Msg::FocusParent),
    (GLOBAL_FOCUS_SIDEBAR_CONTENT, Msg::FocusChild),
    (GLOBAL_FOCUS_PANE_RIGHT, Msg::FocusChild),
    (GLOBAL_HIDE_PANE, Msg::Hide),
    (GLOBAL_TOGGLE_PANE_ZOOM, Msg::Zoom),
    (GLOBAL_CLOSE_SIDEBAR_TAB, Msg::Close),
    (GLOBAL_MOVE_DIVIDER_LEFT, Msg::Step(-2)),
    (GLOBAL_MOVE_DIVIDER_RIGHT, Msg::Step(2)),
    (GLOBAL_MINIMIZE_PANE_WIDTH, Msg::Share(0)),
    (GLOBAL_MAXIMIZE_PANE_WIDTH, Msg::Share(100)),
    (GLOBAL_EQUALIZE_PANE_WIDTHS, Msg::Share(50)),
    (GLOBAL_RESIZE_PANES, Msg::Resize),
];

/// Leave workspace prefixes to the native ancestor KeyBindings.
pub(super) fn claims_key(keymap: &Keymap, key: &KeyPress) -> bool {
    ACTIONS.iter().any(|&(action, _)| {
        crate::keymap::pressed(keymap, action, key)
            || crate::keymap::chords_of(keymap, action)
                .iter()
                .any(|(prefix, _)| prefix.accepts_press(key))
    })
}

pub(super) fn single(keymap: &Keymap, key: &KeyPress) -> Option<Msg> {
    ACTIONS
        .iter()
        .find_map(|&(id, msg)| crate::keymap::pressed(keymap, id, key).then_some(msg))
}

pub(super) fn chords(keymap: &Keymap) -> Vec<(KeyCombo, KeyCombo, Msg)> {
    ACTIONS
        .iter()
        .flat_map(|&(id, msg)| {
            crate::keymap::chords_of(keymap, id)
                .into_iter()
                .map(move |(prefix, completion)| (prefix, completion, msg))
        })
        .collect()
}

#[derive(Clone, PartialEq)]
struct Commands(Handler<Msg>);

pub(super) fn take<S>(cx: &mut Cx<'_, S>, message: Msg) {
    if let Some(commands) = Provider::<Commands>::maybe_of(cx, false) {
        cx.emit(&commands.0, message);
    }
}

fn fractions(percent: u8) -> [f64; 2] {
    [f64::from(100 - percent), f64::from(percent)]
}

impl StatefulWidget for Dock {
    type State = DockState;

    fn create_state(&self) -> DockState {
        DockState {
            width: 0,
            split: SplitController::new().with_fractions(
                self.preferred_width
                    .map_or([2.0, 1.0], |width| fractions(width.min(100))),
            ),
            hidden: false,
            zoom: None,
            resizing: false,
            resize_focus: FocusNode::new(),
        }
    }
}

impl_widget!(stateful eq Dock);

impl DockState {
    fn wide(&self) -> bool {
        self.width >= SPLIT_AT && self.zoom.is_none() && !self.hidden
    }

    fn focus(cx: &mut Cx<'_, Self>, pane: &Pane, active: &ThreadId) {
        let id = pane.model.id();
        if id == active {
            composer_node(&pane.ui).request_focus();
            return;
        }
        let hs = read::<Hs, _>(cx);
        cx.emit(&hs.open_thread, id.clone());
    }

    fn pane(
        cx: &mut Cx<'_, Self>,
        pane: &Pane,
        focused: bool,
        activate: Msg,
        heading: bool,
    ) -> AnyWidget {
        let key = pane.ui.held(|| PaneKey(GlobalKey::new()));
        let chat = Chat {
            ui: pane.ui.clone(),
            presentation: if focused {
                Presentation::Focused
            } else {
                Presentation::Passive
            },
        };
        let chat = Provider::value_shared(pane.model.clone())
            .child(Provider::value(Commands(cx.map(|msg: Msg| msg))).child(chat));
        let heading = if heading {
            AnyWidget::from(Text::line(" Side chat"))
        } else {
            SizedBox::shrink().into()
        };
        let column = Column::new().child(heading).child(Expanded::new(1, chat));
        let scope = FocusScope::new(column)
            .descendants_are_focusable(focused)
            .can_request_focus(focused);
        AnyWidget::from(
            Listener::new()
                .behavior(HitTestBehavior::Translucent)
                .on_pointer_down(cx.map(move |event: PointerEvent| {
                    if !focused && event.button == 0 {
                        activate
                    } else {
                        Msg::Ignore
                    }
                }))
                .child(scope),
        )
        .with_key(key.0.key())
    }
}

impl State for DockState {
    type Widget = Dock;
    type Msg = Msg;

    fn did_update_widget(&mut self, old: &Dock, w: &Dock, _cx: &mut Cx<'_, Self>) {
        if old.active != w.active && w.on_child() {
            self.hidden = false;
        }
        if old.active != w.active {
            self.zoom = None;
        }
        if old.preferred_width != w.preferred_width
            && let Some(width) = w.preferred_width
        {
            self.split.set_fractions(&fractions(width.min(100)));
        }
    }

    fn dispose(&mut self) {
        self.split.dispose();
        self.resize_focus.dispose();
    }

    fn update(&mut self, msg: Msg, w: &Dock, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Width(width) if width != self.width => {
                self.width = width;
                if width < SPLIT_AT && self.resizing {
                    self.resizing = false;
                    let pane = if w.on_child() { &w.child } else { &w.parent };
                    Self::focus(cx, pane, &w.active);
                }
            }
            Msg::FocusParent => Self::focus(cx, &w.parent, &w.active),
            Msg::FocusChild => {
                self.hidden = false;
                Self::focus(cx, &w.child, &w.active);
            }
            Msg::Hide if w.on_child() => {
                self.hidden = true;
                Self::focus(cx, &w.parent, &w.active);
            }
            Msg::Zoom if !self.hidden && self.width >= SPLIT_AT => {
                self.zoom = self.zoom.is_none().then(|| w.active.clone());
            }
            Msg::Close if w.on_child() => {
                let draft = super::composer::submission(&w.child.ui);
                if let Some(intent) = features::run_key(PageKey::ClosePane, &w.active, &draft, cx) {
                    let hs = read::<Hs, _>(cx);
                    cx.emit(&hs.intent, codex_tui_core::app::Intent::Feature(intent));
                }
            }
            Msg::Step(delta) if self.wide() => {
                if let [left, right] = self.split.sizes().as_slice() {
                    let total = left + right;
                    let left =
                        (left + delta).clamp(PARENT_MIN, (total - CHILD_MIN).max(PARENT_MIN));
                    self.split
                        .set_fractions(&[f64::from(left), f64::from(total - left)]);
                }
            }
            Msg::Share(percent) if self.wide() => {
                let side = if w.on_child() { percent } else { 100 - percent };
                self.split.set_fractions(&fractions(side));
            }
            Msg::Resize if self.wide() => {
                self.resizing = true;
                self.resize_focus.request_focus();
            }
            Msg::FinishResize => {
                self.resizing = false;
                let pane = if w.on_child() { &w.child } else { &w.parent };
                composer_node(&pane.ui).request_focus();
            }
            _ => return Rebuild::No,
        }
        Rebuild::Yes
    }

    fn build(&self, w: &Dock, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let wide = self.wide();
        let on_child = w.on_child();
        let parent = Self::pane(cx, &w.parent, !on_child, Msg::FocusParent, false);
        let child = Self::pane(cx, &w.child, on_child, Msg::FocusChild, wide);
        let shown: AnyWidget = if wide {
            SplitView::new([
                SplitPane::new(parent).min_size(PARENT_MIN),
                SplitPane::new(child).min_size(CHILD_MIN),
            ])
            .controller(&self.split)
            .into()
        } else {
            Stack::new()
                .fit(StackFit::Expand)
                .child(Offstage::new(on_child, parent))
                .child(Offstage::new(!on_child, child))
                .into()
        };
        let sampled = self.width;
        let tell = cx.local_sender();
        let sample = tell.clone();
        let measure = LayoutBuilder::new(move |_, room| {
            if room.max_width != sampled {
                sample(Msg::Width(room.max_width));
            }
            SizedBox::shrink().into()
        });
        let shown = Stack::new()
            .fit(StackFit::Expand)
            .child(measure)
            .child(shown);
        let keymap = crate::keymap::keymap(cx);
        let keys = ACTIONS
            .iter()
            .flat_map(|&(action, msg)| crate::keymap::bindings(&keymap, action, &cx.send(msg)))
            .collect();
        let resize = self.resizing;
        let key = tell.clone();
        let shown = Focus::new(shown)
            .focus_node(self.resize_focus.clone())
            .skip_traversal(true)
            .on_key_event(move |node, press| {
                if !resize || !node.has_primary_focus() {
                    return KeyEventResult::Ignored;
                }
                let message = match (press.code, press.mods, press.ch) {
                    (KeyCode::Escape | KeyCode::Enter, Mods::NONE, _) => Msg::FinishResize,
                    (KeyCode::Left, Mods::SHIFT, _) => Msg::Step(-10),
                    (KeyCode::Right, Mods::SHIFT, _) => Msg::Step(10),
                    (KeyCode::Left, Mods::NONE, _) | (_, Mods::NONE, Some('h')) => Msg::Step(-1),
                    (KeyCode::Right, Mods::NONE, _) | (_, Mods::NONE, Some('l')) => Msg::Step(1),
                    _ => return KeyEventResult::Handled,
                };
                key(message);
                KeyEventResult::Handled
            });
        let shown = KeyBindings::new(crate::keymap::first_of_each(keys), shown);
        InputListener::new(shown, move |event| {
            if let InputEvent::Resize { width, .. } = event
                && let Ok(width) = i32::try_from(*width)
            {
                tell(Msg::Width(width));
            }
            resize && matches!(event, InputEvent::Paste(_))
        })
        .into()
    }
}
