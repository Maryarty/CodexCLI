//! The two retained chats share the workspace. The model's active thread
//! owns input while the other pane keeps its transcript and draft mounted.

use std::rc::Rc;

use codex_tui_core::features::settings::{Edit, Intent as SettingsIntent};
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
    BorderType, BoxWidget, Column, CrossAxisAlignment, Expanded, MainAxisSize, Offstage,
    Positioned, Row, SizedBox, SplitController, SplitPane, SplitView, Stack, StackFit,
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
    initial_width: Option<u8>,
    hidden: bool,
    zoom: Option<ThreadId>,
    resize: Option<Resize>,
    resize_focus: FocusNode,
    close_pending: bool,
}

#[derive(Clone, Copy)]
enum Resize {
    Keyboard(i32),
    Pointer(i32),
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
    DividerDown(i32),
    DividerUp,
    Ignore,
}

#[derive(Debug, PartialEq)]
pub struct CloseProof {
    thread: ThreadId,
    draft: UserMessage,
    shell: bool,
    running: bool,
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

impl StatefulWidget for Dock {
    type State = DockState;

    fn create_state(&self) -> DockState {
        DockState {
            width: 0,
            split: SplitController::new().with_fractions([2.0, 1.0]),
            initial_width: self.preferred_width,
            hidden: false,
            zoom: None,
            resize: None,
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

    fn keyboard_resize(&self) -> bool {
        matches!(self.resize, Some(Resize::Keyboard(_)))
    }

    fn clamped_child_width(&self, cells: i32) -> i32 {
        cells.clamp(CHILD_MIN, (self.width - 1 - PARENT_MIN).max(CHILD_MIN))
    }

    fn set_child_width(&self, cells: i32) {
        let child = self.clamped_child_width(cells);
        self.split
            .set_fractions(&[f64::from(self.width - 1 - child), f64::from(child)]);
    }

    fn child_width(&self) -> i32 {
        // SplitView updates its fractions before its next layout publishes the measured sizes.
        let fractions = self.split.fractions();
        let exact = f64::from(self.width - 1) * fractions[1];
        #[allow(clippy::cast_possible_truncation)]
        let cells = exact.round() as i32;
        self.clamped_child_width(cells)
    }

    fn save_width(&self, before: i32, cx: &mut Cx<'_, Self>) {
        let cells = self.child_width();
        if before != cells {
            let percent = ((cells * 100 + self.width / 2) / self.width).clamp(1, 99);
            let percent = u8::try_from(percent).expect("sidebar width is between 1 and 99 percent");
            features::verb(cx, SettingsIntent::Request(Edit::SidebarWidth(percent)));
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
    }

    fn dispose(&mut self) {
        self.split.dispose();
        self.resize_focus.dispose();
    }

    fn update(&mut self, msg: Msg, w: &Dock, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Width(width) if width != self.width => {
                self.width = width;
                if width >= SPLIT_AT
                    && let Some(percent) = self.initial_width.take()
                {
                    self.set_child_width((width * i32::from(percent) + 50) / 100);
                }
                if let Some(Resize::Keyboard(_)) = self.resize.take() {
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
                let running = w.child.model.task_running();
                let mut losses = Vec::new();
                if running {
                    losses.push("This will stop the running turn.");
                }
                if unsent {
                    losses.push("This will discard unsent input.");
                }
                // All side conversations use temporary storage at this boundary.
                losses.push("This will lose this temporary chat’s history.");
                self.close_pending = true;
                let dialog = ConfirmClose { losses };
                let result =
                    show_dialog(cx, move |_| dialog.clone().into(), DialogOptions::default());
                let proof = Rc::new(CloseProof {
                    thread: w.child.model.id().clone(),
                    draft,
                    shell,
                    running,
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
                    && (proof.running || !w.child.model.task_running())
                    && proof.shell == ui.draft.shell()
                    && proof.draft == ui.draft.submission(&ui.composer, |n| words.image(n))
                {
                    Self::close(cx, &proof.thread);
                }
            }
            Msg::Step(delta) if self.keyboard_resize() => {
                self.set_child_width(self.child_width() - delta);
            }
            Msg::Nudge(direction) if self.wide() && w.on_child() => {
                let step = (self.width.saturating_mul(2).saturating_add(50) / 100).max(1);
                let before = self.child_width();
                self.set_child_width(before - direction * step);
                self.save_width(before, cx);
            }
            Msg::Share(percent) if self.wide() => {
                let side = if w.on_child() { percent } else { 100 - percent };
                let before = self.child_width();
                self.set_child_width(((self.width - 1) * i32::from(side) + 99) / 100);
                self.save_width(before, cx);
            }
            Msg::Resize if self.wide() && self.resize.is_none() => {
                self.resize = Some(Resize::Keyboard(self.child_width()));
                self.resize_focus.request_focus();
            }
            Msg::FinishResize if self.keyboard_resize() => {
                if let Some(Resize::Keyboard(before)) = self.resize.take() {
                    self.save_width(before, cx);
                }
                let pane = if w.on_child() { &w.child } else { &w.parent };
                composer_node(&pane.ui).request_focus();
            }
            Msg::DividerDown(column) if self.wide() && self.resize.is_none() => {
                if let [left, right] = self.split.sizes().as_slice()
                    && left + right + 1 == self.width
                    && column == *left
                {
                    self.resize = Some(Resize::Pointer(*right));
                }
                return Rebuild::No;
            }
            Msg::DividerUp if matches!(self.resize, Some(Resize::Pointer(_))) => {
                if let Some(Resize::Pointer(before)) = self.resize.take()
                    && self.wide()
                {
                    self.save_width(before, cx);
                }
                return Rebuild::No;
            }
            _ => return Rebuild::No,
        }
        Rebuild::Yes
    }

    fn build(&self, w: &Dock, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let wide = self.wide();
        let on_child = w.on_child();
        let resize = self.keyboard_resize();
        let parent = Self::pane(
            cx,
            &w.parent,
            !on_child,
            Msg::FocusParent,
            false,
            resize && !on_child,
        );
        let child = Self::pane(
            cx,
            &w.child,
            on_child,
            Msg::FocusChild,
            wide,
            resize && on_child,
        );
        let shown: AnyWidget = if wide {
            let mut split = SplitView::new([
                SplitPane::new(parent).min_size(PARENT_MIN),
                SplitPane::new(child).min_size(CHILD_MIN),
            ])
            .controller(&self.split);
            if self.close_pending {
                // Keep the gutter but clear its line under the modal's text.
                split = split.kind(BorderType::None);
            }
            Listener::new()
                .on_pointer_down(cx.map(|event: PointerEvent| match event.button {
                    0 => Msg::DividerDown(event.x),
                    _ => Msg::Ignore,
                }))
                .on_pointer_up(cx.map(|event: PointerEvent| match event.button {
                    0 => Msg::DividerUp,
                    _ => Msg::Ignore,
                }))
                .child(split)
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

#[derive(Clone, PartialEq)]
struct ConfirmClose {
    losses: Vec<&'static str>,
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
        let mut body = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Start);
        for loss in &w.losses {
            body = body.child(Text::new(*loss));
        }
        Focus::new(Dialog::new(body.child(actions)).title("Close side chat?"))
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
