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
use codex_tui_core::threads::{ThreadModel, UserMessage};
use rtwk::binding::InputListener;
use rtwk::controls::Button;
use rtwk::focus::{Focus, FocusNode, FocusScope, KeyBindings, KeyCombo, KeyEventResult, KeyPress};
use rtwk::gestures::{AbsorbPointer, Listener, PointerEvent};
use rtwk::layout::{
    BoxWidget, Column, Expanded, MainAxisSize, Offstage, Positioned, Row, SizedBox,
    SplitController, SplitPane, SplitView, Stack, StackFit,
};
use rtwk::overlay::{Dialog, DialogOptions, close_dialog, show_dialog};
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
    close_pending: bool,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Msg {
    Width(i32),
    FocusParent,
    FocusChild,
    Hide,
    Zoom,
    Close,
    CloseDecision(Option<Rc<CloseProof>>),
    Step(i32),
    Nudge(i32),
    Share(u8),
    Resize,
    FinishResize,
    Ignore,
}

#[derive(Debug, PartialEq)]
pub struct CloseProof {
    thread: ThreadId,
    draft: UserMessage,
    shell: bool,
}

const ACTIONS: &[(ActionId, Msg)] = &[
    (GLOBAL_FOCUS_MAIN_COMPOSER, Msg::FocusParent),
    (GLOBAL_FOCUS_PANE_LEFT, Msg::FocusParent),
    (GLOBAL_FOCUS_SIDEBAR_CONTENT, Msg::FocusChild),
    (GLOBAL_FOCUS_PANE_RIGHT, Msg::FocusChild),
    (GLOBAL_HIDE_PANE, Msg::Hide),
    (GLOBAL_TOGGLE_PANE_ZOOM, Msg::Zoom),
    (GLOBAL_CLOSE_SIDEBAR_TAB, Msg::Close),
    (GLOBAL_MOVE_DIVIDER_LEFT, Msg::Nudge(-1)),
    (GLOBAL_MOVE_DIVIDER_RIGHT, Msg::Nudge(1)),
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
        .find_map(|(id, msg)| crate::keymap::pressed(keymap, *id, key).then(|| msg.clone()))
}

pub(super) fn chords(keymap: &Keymap) -> Vec<(KeyCombo, KeyCombo, Msg)> {
    ACTIONS
        .iter()
        .flat_map(|(id, msg)| {
            crate::keymap::chords_of(keymap, *id)
                .into_iter()
                .map(move |(prefix, completion)| (prefix, completion, msg.clone()))
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
            close_pending: false,
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

    fn close(cx: &mut Cx<'_, Self>, child: &ThreadId) {
        if let Some(intent) = features::run_key(PageKey::ClosePane, child, "", cx) {
            let hs = read::<Hs, _>(cx);
            cx.emit(&hs.intent, codex_tui_core::app::Intent::Feature(intent));
        }
    }

    fn step(&self, delta: i32) {
        if let [left, right] = self.split.sizes().as_slice() {
            let total = left + right;
            let left = (left + delta).clamp(PARENT_MIN, (total - CHILD_MIN).max(PARENT_MIN));
            self.split
                .set_fractions(&[f64::from(left), f64::from(total - left)]);
        }
    }

    fn pane(
        cx: &mut Cx<'_, Self>,
        pane: &Pane,
        focused: bool,
        activate: Msg,
        heading: bool,
        resizing: bool,
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
        let hint: AnyWidget = if resizing {
            let label = crate::oracle_strings::commands::SIDE_PANE_RESIZE_H_L_SHIFT_10_COLS;
            Positioned::new(BoxWidget::new().child(Text::line(format!("  {label}"))))
                .left(0)
                .right(0)
                .bottom(0)
                .into()
        } else {
            SizedBox::shrink().into()
        };
        let body = Stack::new().fit(StackFit::Expand).child(column).child(hint);
        let scope = FocusScope::new(body)
            .descendants_are_focusable(focused)
            .can_request_focus(focused);
        AnyWidget::from(
            Listener::new()
                .behavior(HitTestBehavior::Translucent)
                .on_pointer_down(cx.map(move |event: PointerEvent| {
                    if !focused && event.button == 0 {
                        activate.clone()
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
            Msg::Close if w.on_child() && !self.close_pending => {
                let ui = &w.child.ui;
                if ui.draft.in_burst() {
                    return Rebuild::No;
                }
                let words = super::composer::strings::of(cx);
                let draft = ui.draft.submission(&ui.composer, |n| words.image(n));
                let shell = ui.draft.shell();
                let unsent = shell
                    || !draft.text.is_empty()
                    || !draft.images.is_empty()
                    || !draft.remote_images.is_empty();
                // The current lifecycle uses temporary side conversations.
                let history =
                    !w.child.model.transcript().is_empty() || w.child.model.task_running();
                let loss = match (unsent, history) {
                    (true, true) => {
                        "This will discard unsent input and lose this temporary chat’s history."
                    }
                    (true, false) => "This will discard unsent input.",
                    (false, true) => "This will lose this temporary chat’s history.",
                    (false, false) => {
                        Self::close(cx, w.child.model.id());
                        return Rebuild::No;
                    }
                };
                self.close_pending = true;
                let result = show_dialog(
                    cx,
                    move |_| ConfirmClose { loss }.into(),
                    DialogOptions::default(),
                );
                let proof = Rc::new(CloseProof {
                    thread: w.child.model.id().clone(),
                    draft,
                    shell,
                });
                cx.spawn(async move {
                    Some(Msg::CloseDecision(
                        (result.await == Some(true)).then_some(proof),
                    ))
                });
            }
            Msg::CloseDecision(proof) => {
                self.close_pending = false;
                let ui = &w.child.ui;
                let words = super::composer::strings::of(cx);
                // A rejected send can bring input back while the dialog owns the keys.
                if let Some(proof) = proof
                    && &proof.thread == w.child.model.id()
                    && !ui.draft.in_burst()
                    && proof.shell == ui.draft.shell()
                    && proof.draft == ui.draft.submission(&ui.composer, |n| words.image(n))
                {
                    Self::close(cx, &proof.thread);
                }
            }
            Msg::Step(delta) if self.wide() => self.step(delta),
            Msg::Nudge(direction) if self.wide() && w.on_child() => {
                let step = (self.width.saturating_mul(2).saturating_add(50) / 100).max(1);
                self.step(direction * step);
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
        let parent = Self::pane(
            cx,
            &w.parent,
            !on_child,
            Msg::FocusParent,
            false,
            self.resizing && !on_child,
        );
        let child = Self::pane(
            cx,
            &w.child,
            on_child,
            Msg::FocusChild,
            wide,
            self.resizing && on_child,
        );
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
            .flat_map(|(action, msg)| {
                crate::keymap::bindings(&keymap, *action, &cx.send(msg.clone()))
            })
            .collect();
        let resize = self.resizing;
        let key = tell.clone();
        let shown = Focus::new(AbsorbPointer::new().absorbing(resize).child(shown))
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

#[derive(Clone, Copy, PartialEq)]
struct ConfirmClose {
    loss: &'static str,
}

struct ConfirmCloseState;

impl StatefulWidget for ConfirmClose {
    type State = ConfirmCloseState;

    fn create_state(&self) -> Self::State {
        ConfirmCloseState
    }
}

impl_widget!(stateful eq ConfirmClose);

impl State for ConfirmCloseState {
    type Widget = ConfirmClose;
    type Msg = bool;

    fn update(&mut self, close: bool, _: &ConfirmClose, cx: &mut Cx<'_, Self>) -> Rebuild {
        close_dialog(cx, Some(close));
        Rebuild::No
    }

    fn build(&self, w: &ConfirmClose, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let actions = Row::new()
            .main_axis_size(MainAxisSize::Min)
            .child(
                Button::new("Cancel")
                    .autofocus(true)
                    .on_pressed(cx.send(false)),
            )
            .child(Button::new("Close").on_pressed(cx.send(true)));
        let body = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .child(Text::new(w.loss))
            .child(actions);
        Focus::new(Dialog::new(body).title("Close side chat?"))
            .can_request_focus(false)
            .on_key_event(|node, key| {
                let moved = match (key.code, key.mods) {
                    (KeyCode::Left, Mods::NONE) => node.previous_focus(),
                    (KeyCode::Right, Mods::NONE) => node.next_focus(),
                    _ => false,
                };
                if moved {
                    KeyEventResult::Handled
                } else {
                    KeyEventResult::Ignored
                }
            })
            .into()
    }
}
