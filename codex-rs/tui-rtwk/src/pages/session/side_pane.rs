//! The two retained chats share the workspace. The model's active thread
//! owns input while the other pane keeps its transcript and draft mounted.

use std::rc::Rc;

use codex_tui_core::ids::ThreadId;
use codex_tui_core::keymap::actions::{
    GLOBAL_CLOSE_SIDEBAR_TAB, GLOBAL_EQUALIZE_PANE_WIDTHS, GLOBAL_FOCUS_MAIN_COMPOSER,
    GLOBAL_FOCUS_PANE_LEFT, GLOBAL_FOCUS_PANE_RIGHT, GLOBAL_FOCUS_SIDEBAR_CONTENT,
    GLOBAL_FOCUS_SIDEBAR_TABS, GLOBAL_HIDE_PANE, GLOBAL_MAXIMIZE_PANE_WIDTH,
    GLOBAL_MINIMIZE_PANE_WIDTH, GLOBAL_MOVE_DIVIDER_LEFT, GLOBAL_MOVE_DIVIDER_RIGHT,
    GLOBAL_RESIZE_PANES, GLOBAL_SIDEBAR_DOWN, GLOBAL_SIDEBAR_UP, GLOBAL_TOGGLE_PANE_ZOOM,
};
use codex_tui_core::keymap::{ActionId, Keymap};
use codex_tui_core::threads::{ThreadModel, UserMessage};
use rtwk::binding::{AppBinding, InputListener};
use rtwk::controls::Pressable;
use rtwk::focus::{Focus, FocusNode, FocusScope, KeyBindings, KeyCombo, KeyEventResult, KeyPress};
use rtwk::gestures::{AbsorbPointer, Listener, PointerEvent};
use rtwk::layout::{
    BorderType, BoxWidget, Column, CrossAxisAlignment, EdgeInsets, Expanded, MainAxisSize,
    Offstage, Positioned, Row, SizedBox, SplitController, SplitPane, SplitView, Stack, StackFit,
};
use rtwk::overlay::{Dialog, DialogHandle, DialogOptions, close_dialog, show_dialog};
use rtwk::prelude::HitTestBehavior;
use rtwk::provider::{Provider, read};
use rtwk::semantics::{Semantics, SemanticsRole};
use rtwk::style::Theme;
use rtwk::widgets::Text;
use rtwk::{
    AnyWidget, Cx, GlobalKey, Handler, LayoutBuilder, Rebuild, State, StatefulWidget, Str,
    impl_widget,
};
use rtwk_core::input::InputEvent;

use crate::app::{UiApp, view::Hs};
use crate::features;
use crate::keymap::press::{
    PaneActivationGuard, PaneActivationKey, ResizeKey, horizontal_focus, resize_key,
};
use crate::widgets::thread_scope::thread_scope;
use crate::widgets::thread_ui::ThreadUi;

use super::side_tabs::{self, tab_name};

mod chooser;
use super::{Presentation, session_page::Chat};

const SPLIT_AT: i32 = 100;
const PARENT_MIN: i32 = 63;
const CHILD_MIN: i32 = 36;

#[derive(Clone)]
struct Pane {
    model: Rc<ThreadModel>,
    ui: Rc<ThreadUi>,
    temporary: bool,
}

impl PartialEq for Pane {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.model, &other.model)
            && Rc::ptr_eq(&self.ui, &other.ui)
            && self.temporary == other.temporary
    }
}

#[derive(Clone)]
struct ComposerNode(FocusNode);

pub(super) fn composer_node(ui: &ThreadUi) -> FocusNode {
    ui.held(|| ComposerNode(FocusNode::new())).0
}

#[derive(Clone, PartialEq)]
pub(super) struct Dock {
    parent: Pane,
    child: Option<Pane>,
    open: Rc<[Pane]>,
    children: Rc<[ThreadId]>,
    tab_names: Rc<[Option<Str>]>,
    active: ThreadId,
    preferred_width: Option<u8>,
    saved: Option<features::SavedChats>,
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
            (&link.parent, Some(active))
        } else {
            (active, side.side_of(active))
        };
        let pane = |id| {
            Some(Pane {
                model: app.core.thread(id)?.clone(),
                ui: app.ui.thread_ui.get(Some(id)),
                temporary: features::side_is_temporary(app.core.features(), id),
            })
        };
        let children = side.children_of(parent);
        let tab_names = children
            .iter()
            .map(|id| {
                app.core
                    .thread(id)
                    .and_then(|thread| tab_name(thread, &app.ui.thread_ui.get(Some(id))))
            })
            .collect();
        Some(Self {
            parent: pane(parent)?,
            child: child.and_then(pane),
            open: children.iter().filter_map(pane).collect(),
            children: Rc::from(children),
            tab_names,
            active: active.clone(),
            preferred_width: settings.sidebar_width_percent,
            saved: side.saved(parent).cloned(),
        })
    }

    fn on_child(&self) -> bool {
        self.child
            .as_ref()
            .is_some_and(|child| child.model.id() == &self.active)
    }

    fn active_pane(&self) -> &Pane {
        self.child
            .as_ref()
            .filter(|pane| pane.model.id() == &self.active)
            .unwrap_or(&self.parent)
    }

    fn open_child(&self, id: &ThreadId) -> Option<&Pane> {
        self.open.iter().find(|pane| pane.model.id() == id)
    }

    fn has_saved(&self, id: &ThreadId) -> bool {
        matches!(&self.saved, Some(features::SavedChats::Ready(rows)) if rows.iter().any(|row| &row.id == id))
    }

    fn name(&self, id: &ThreadId) -> Option<Str> {
        self.children
            .iter()
            .position(|child| child == id)
            .and_then(|index| self.tab_names[index].clone())
            .or_else(|| match &self.saved {
                Some(features::SavedChats::Ready(rows)) => rows
                    .iter()
                    .find(|row| &row.id == id)
                    .and_then(|row| row.name.as_deref().or_else(|| row.preview.lines().next()))
                    .filter(|name| !name.trim().is_empty())
                    .map(Str::copy),
                _ => None,
            })
    }

    pub(super) fn keyed(self) -> AnyWidget {
        let parent = self.parent.model.id().clone();
        thread_scope(&parent, self)
    }
}

pub(super) struct DockState {
    width: i32,
    split: SplitController,
    tabs_focus: FocusNode,
    chooser: Option<chooser::ChooserControls>,
    parent_key: GlobalKey,
    child_key: GlobalKey,
    initial_width: Option<u8>,
    hidden: bool,
    zoom: Option<ThreadId>,
    resize: Option<Resize>,
    resize_focus: FocusNode,
    close_pending: Option<PendingClose>,
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
    FocusTabs,
    DismissChooser,
    Choose(chooser::SideChoice, bool),
    Archive(ThreadId),
    SelectTab(ThreadId),
    StepTab(i32),
    StepFocusedTab(i32),
    CloseSelected,
    CloseTab(ThreadId),
    CloseDecision(Rc<CloseProof>, bool),
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
    archive: bool,
    draft: Option<UserMessage>,
    shell: bool,
    running: bool,
}

impl CloseProof {
    fn capture(
        w: &Dock,
        thread: ThreadId,
        archive: bool,
        cx: &mut Cx<'_, DockState>,
    ) -> Option<Self> {
        let Some(pane) = w.open_child(&thread) else {
            return (archive && w.has_saved(&thread)).then_some(Self {
                thread,
                archive,
                draft: None,
                shell: false,
                running: false,
            });
        };
        if pane.ui.draft.in_burst() || (archive && pane.temporary) {
            return None;
        }
        let words = super::composer::strings::of(cx);
        Some(Self {
            thread,
            archive,
            draft: Some(
                pane.ui
                    .draft
                    .submission(&pane.ui.composer, |n| words.image(n)),
            ),
            shell: pane.ui.draft.shell(),
            running: pane.model.task_running(),
        })
    }

    fn losses(&self, w: &Dock, cx: &mut Cx<'_, DockState>) -> Vec<&'static str> {
        let words = super::strings::of(cx);
        let mut losses = Vec::new();
        if self.running {
            losses.push(words.side_close_turn);
        }
        if self.shell
            || self.draft.as_ref().is_some_and(|draft| {
                !draft.text.is_empty()
                    || !draft.images.is_empty()
                    || !draft.remote_images.is_empty()
            })
        {
            losses.push(words.side_close_draft);
        }
        if w.open_child(&self.thread)
            .is_some_and(|pane| pane.temporary)
        {
            losses.push(words.side_close_history);
        }
        if self.archive {
            losses.push(words.side_archive_history);
        }
        losses
    }

    fn is_current(&self, w: &Dock, cx: &mut Cx<'_, DockState>) -> bool {
        Self::capture(w, self.thread.clone(), self.archive, cx).is_some_and(|current| {
            self.draft == current.draft
                && self.shell == current.shell
                && (self.running || !current.running)
        })
    }

    fn commit(&self, w: &Dock, cx: &mut Cx<'_, DockState>) {
        let parent = w.parent.model.id().clone();
        if self.archive {
            features::archive_side(cx, parent, self.thread.clone());
        } else {
            features::close_side(cx, parent, self.thread.clone());
        }
    }
}

struct PendingClose {
    proof: Rc<CloseProof>,
    dialog: DialogHandle<()>,
}

const ACTIONS: &[(ActionId, Msg)] = &[
    (GLOBAL_FOCUS_MAIN_COMPOSER, Msg::FocusParent),
    (GLOBAL_FOCUS_PANE_LEFT, Msg::FocusParent),
    (GLOBAL_FOCUS_SIDEBAR_CONTENT, Msg::FocusChild),
    (GLOBAL_FOCUS_SIDEBAR_TABS, Msg::FocusTabs),
    (GLOBAL_SIDEBAR_UP, Msg::StepFocusedTab(-1)),
    (GLOBAL_SIDEBAR_DOWN, Msg::StepFocusedTab(1)),
    (GLOBAL_FOCUS_PANE_RIGHT, Msg::FocusChild),
    (GLOBAL_HIDE_PANE, Msg::Hide),
    (GLOBAL_TOGGLE_PANE_ZOOM, Msg::Zoom),
    (GLOBAL_CLOSE_SIDEBAR_TAB, Msg::CloseSelected),
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
struct Commands(Handler<Msg>, FocusNode, Option<FocusNode>, Option<ThreadId>);

pub(super) fn take<S>(cx: &mut Cx<'_, S>, message: Msg) {
    if let Some(commands) = Provider::<Commands>::maybe_of(cx, false) {
        cx.emit(&commands.0, message);
    }
}

pub(super) fn interrupt_side<S>(cx: &mut Cx<'_, S>) -> bool {
    let Some(commands) = Provider::<Commands>::maybe_of(cx, false) else {
        return false;
    };
    let Some(child) = &commands.3 else {
        return false;
    };
    cx.emit(&commands.0, Msg::Archive(child.clone()));
    true
}

pub(super) fn tabs_focused<S>(cx: &mut Cx<'_, S>) -> bool {
    Provider::<Commands>::maybe_of(cx, false).is_some_and(|commands| {
        commands.1.has_primary_focus()
            || commands
                .2
                .as_ref()
                .is_some_and(FocusNode::has_primary_focus)
    })
}

impl StatefulWidget for Dock {
    type State = DockState;

    fn create_state(&self) -> DockState {
        DockState {
            width: 0,
            split: SplitController::new().with_fractions([2.0, 1.0]),
            tabs_focus: FocusNode::new(),
            chooser: None,
            parent_key: GlobalKey::new(),
            child_key: GlobalKey::new(),
            initial_width: self.preferred_width,
            hidden: false,
            zoom: None,
            resize: None,
            resize_focus: FocusNode::new(),
            close_pending: None,
        }
    }
}

impl_widget!(stateful eq Dock);

impl DockState {
    fn wide(&self) -> bool {
        self.width >= SPLIT_AT && self.zoom.is_none() && !self.hidden
    }

    fn docked(&self, w: &Dock) -> bool {
        self.wide() && (w.child.is_some() || self.chooser.is_some())
    }

    fn focus_after_frame(cx: &mut Cx<'_, Self>, focus: FocusNode) {
        if let Some(binding) = AppBinding::maybe_of(cx) {
            binding.add_post_frame_callback(cx.callback_with(move |_, _, _| {
                focus.request_focus();
                Rebuild::No
            }));
        }
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
            let percent = u8::try_from(percent).unwrap_or(99);
            features::save_sidebar_width(cx, percent);
        }
    }

    fn pane(
        &self,
        cx: &mut Cx<'_, Self>,
        pane: &Pane,
        key: &GlobalKey,
        focused: bool,
        activate: Msg,
        resizing: bool,
    ) -> AnyWidget {
        let chat = Chat {
            ui: pane.ui.clone(),
            presentation: if focused {
                Presentation::Focused
            } else {
                Presentation::Passive
            },
        };
        let controls = Commands(
            cx.map(|msg: Msg| msg),
            self.tabs_focus.clone(),
            self.chooser.as_ref().map(|chooser| chooser.focus.clone()),
            (activate == Msg::FocusChild && !pane.temporary).then(|| pane.model.id().clone()),
        );
        let chat =
            Provider::value_shared(pane.model.clone()).child(Provider::value(controls).child(chat));
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
        let body = Stack::new().fit(StackFit::Expand).child(chat).child(hint);
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
        .with_key(key.key())
    }
}

impl State for DockState {
    type Widget = Dock;
    type Msg = Msg;

    fn did_update_widget(&mut self, old: &Dock, w: &Dock, cx: &mut Cx<'_, Self>) {
        if old.active != w.active && w.on_child() {
            self.hidden = false;
        }
        if old.active != w.active {
            self.zoom = match (&self.zoom, old.on_child(), w.on_child()) {
                (Some(_), true, true) => Some(w.active.clone()),
                _ => None,
            };
        }
        if old.active != w.active && w.on_child() && self.chooser.take().is_some() {
            Self::focus_after_frame(cx, composer_node(&w.active_pane().ui));
        }
        let missing = self.close_pending.as_ref().is_some_and(|pending| {
            w.open_child(&pending.proof.thread).is_none()
                && !(pending.proof.archive && w.has_saved(&pending.proof.thread))
        });
        if missing && let Some(pending) = self.close_pending.take() {
            pending.dialog.close(None);
        }
    }

    fn dispose(&mut self) {
        if let Some(pending) = self.close_pending.take() {
            pending.dialog.close(None);
        }
        self.split.dispose();
        self.tabs_focus.dispose();
        self.resize_focus.dispose();
    }

    fn update(&mut self, msg: Msg, w: &Dock, cx: &mut Cx<'_, Self>) -> Rebuild {
        let msg = match msg {
            Msg::CloseSelected if w.on_child() => Msg::CloseTab(w.active.clone()),
            Msg::StepFocusedTab(step) if w.on_child() || self.tabs_focus.has_primary_focus() => {
                Msg::StepTab(step)
            }
            other => other,
        };
        match msg {
            Msg::Width(width) if width != self.width => {
                self.width = width;
                if width >= SPLIT_AT
                    && let Some(percent) = self.initial_width.take()
                {
                    self.set_child_width((width * i32::from(percent) + 50) / 100);
                }
                if let Some(Resize::Keyboard(_)) = self.resize.take() {
                    Self::focus(cx, w.active_pane(), &w.active);
                }
                if width < SPLIT_AT && self.chooser.take().is_some() {
                    Self::focus(cx, w.active_pane(), &w.active);
                }
            }
            Msg::FocusParent => {
                self.chooser = None;
                Self::focus(cx, &w.parent, &w.active);
            }
            Msg::FocusChild => {
                if let Some(child) = &w.child {
                    self.chooser = None;
                    self.hidden = false;
                    Self::focus(cx, child, &w.active);
                }
            }
            Msg::FocusTabs if self.width >= SPLIT_AT => {
                if self.chooser.is_some() {
                    self.chooser = None;
                    Self::focus(cx, w.child.as_ref().unwrap_or(&w.parent), &w.active);
                } else {
                    self.hidden = false;
                    self.zoom = None;
                    let chooser = chooser::ChooserControls::new();
                    if w.on_child() {
                        Self::focus(cx, &w.parent, &w.active);
                    }
                    // Native autofocus fills an empty scope. The outgoing composer can still
                    // own focus while the chooser and its parent are being mounted.
                    Self::focus_after_frame(cx, chooser.focus.clone());
                    self.chooser = Some(chooser);
                    features::refresh_saved_side(cx, w.parent.model.id().clone());
                }
            }
            Msg::DismissChooser if self.chooser.is_some() => {
                self.chooser = None;
                Self::focus(cx, w.child.as_ref().unwrap_or(&w.parent), &w.active);
            }
            Msg::Choose(choice, keyboard) if self.chooser.is_some() && !w.on_child() => {
                if keyboard {
                    read::<PaneActivationGuard, _>(cx).arm(PaneActivationKey::Enter);
                }
                let parent = w.parent.model.id().clone();
                match choice {
                    chooser::SideChoice::Create => features::create_side(cx, parent),
                    chooser::SideChoice::Open(child) if w.open_child(&child).is_some() => {
                        features::select_side(cx, parent, child);
                    }
                    chooser::SideChoice::Open(child) if w.has_saved(&child) => {
                        features::open_saved_side(cx, parent, child);
                    }
                    chooser::SideChoice::Retry => features::refresh_saved_side(cx, parent),
                    chooser::SideChoice::Open(_) => return Rebuild::No,
                }
            }
            Msg::SelectTab(child) if w.open_child(&child).is_some() => {
                self.hidden = false;
                if child != w.active {
                    features::select_side(cx, w.parent.model.id().clone(), child);
                } else {
                    Self::focus(cx, w.active_pane(), &w.active);
                }
            }
            Msg::StepTab(step) => {
                let Some(index) = w
                    .child
                    .as_ref()
                    .and_then(|child| w.children.iter().position(|id| id == child.model.id()))
                else {
                    return Rebuild::No;
                };
                let count = w.children.len();
                let next = if step < 0 {
                    (index + count - 1) % count
                } else {
                    (index + 1) % count
                };
                if next != index {
                    features::select_side(
                        cx,
                        w.parent.model.id().clone(),
                        w.children[next].clone(),
                    );
                }
            }
            Msg::Hide if w.on_child() => {
                self.hidden = true;
                Self::focus(cx, &w.parent, &w.active);
            }
            Msg::Zoom if !self.hidden && self.width >= SPLIT_AT => {
                self.zoom = self.zoom.is_none().then(|| w.active.clone());
            }
            Msg::CloseTab(_) | Msg::Archive(_) if self.close_pending.is_none() => {
                let (target, archive) = match msg {
                    Msg::CloseTab(target) => (target, false),
                    Msg::Archive(target) => (target, true),
                    _ => unreachable!(),
                };
                let Some(proof) = CloseProof::capture(w, target, archive, cx) else {
                    return Rebuild::No;
                };
                let losses = proof.losses(w, cx);
                if losses.is_empty() {
                    proof.commit(w, cx);
                    return Rebuild::No;
                }
                let proof = Rc::new(proof);
                let dialog = ConfirmClose {
                    name: w.name(&proof.thread),
                    losses,
                    archive,
                    on_decision: {
                        let proof = proof.clone();
                        cx.map(move |accepted| Msg::CloseDecision(proof.clone(), accepted))
                    },
                };
                let result =
                    show_dialog(cx, move |_| dialog.clone().into(), DialogOptions::default());
                self.close_pending = Some(PendingClose {
                    proof,
                    dialog: result.handle(),
                });
            }
            Msg::CloseDecision(proof, accepted)
                if self
                    .close_pending
                    .as_ref()
                    .is_some_and(|pending| Rc::ptr_eq(&pending.proof, &proof)) =>
            {
                self.close_pending = None;
                // A rejected send can bring input back while the dialog owns the keys.
                if accepted && proof.is_current(w, cx) {
                    proof.commit(w, cx);
                }
            }
            Msg::Step(delta) if self.keyboard_resize() => {
                self.set_child_width(self.child_width() - delta);
            }
            Msg::Nudge(direction) if self.docked(w) && w.on_child() => {
                let step = (self.width.saturating_mul(2).saturating_add(50) / 100).max(1);
                let before = self.child_width();
                self.set_child_width(before - direction * step);
                self.save_width(before, cx);
            }
            Msg::Share(percent) if self.docked(w) => {
                let side = if w.on_child() { percent } else { 100 - percent };
                let before = self.child_width();
                self.set_child_width(((self.width - 1) * i32::from(side) + 99) / 100);
                self.save_width(before, cx);
            }
            Msg::Resize if self.docked(w) && self.resize.is_none() => {
                self.resize = Some(Resize::Keyboard(self.child_width()));
                self.resize_focus.request_focus();
            }
            Msg::FinishResize if self.keyboard_resize() => {
                if let Some(Resize::Keyboard(before)) = self.resize.take() {
                    self.save_width(before, cx);
                }
                composer_node(&w.active_pane().ui).request_focus();
            }
            Msg::DividerDown(column) if self.docked(w) && self.resize.is_none() => {
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
        let wide = self.docked(w);
        let on_child = w.on_child();
        let resize = self.keyboard_resize();
        let parent = self.pane(
            cx,
            &w.parent,
            &self.parent_key,
            !on_child,
            Msg::FocusParent,
            resize && !on_child,
        );
        let child = w.child.as_ref().map(|child| {
            self.pane(
                cx,
                child,
                &self.child_key,
                on_child && self.chooser.is_none(),
                Msg::FocusChild,
                resize && on_child,
            )
        });
        let shown: AnyWidget = if wide {
            let strip = side_tabs::view(
                cx,
                &self.tabs_focus,
                w.child.as_ref().map(|child| child.model.id()),
                &w.children,
                &w.tab_names,
            );
            let content: AnyWidget = match (&self.chooser, child) {
                (Some(chooser), Some(child)) => Stack::new()
                    .fit(StackFit::Expand)
                    .child(Offstage::new(true, child))
                    .child(chooser.view(w, cx))
                    .into(),
                (Some(chooser), None) => chooser.view(w, cx),
                (None, Some(child)) => child,
                (None, None) => SizedBox::shrink().into(),
            };
            let child = Column::new().child(strip).child(Expanded::new(1, content));
            let mut split = SplitView::new([
                SplitPane::new(parent).min_size(PARENT_MIN),
                SplitPane::new(child).min_size(CHILD_MIN),
            ])
            .controller(&self.split);
            if self.close_pending.is_some() {
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
        } else if let Some(child) = child {
            Stack::new()
                .fit(StackFit::Expand)
                .child(Offstage::new(on_child, parent))
                .child(Offstage::new(!on_child, child))
                .into()
        } else {
            parent
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
        let activation = read::<PaneActivationGuard, _>(cx);
        let arm = activation.clone();
        let shown = Focus::new(AbsorbPointer::new().absorbing(resize).child(shown))
            .focus_node(self.resize_focus.clone())
            .skip_traversal(true)
            .on_key_event(move |node, press| {
                if !resize || !node.has_primary_focus() {
                    return KeyEventResult::Ignored;
                }
                if let Some(action) = resize_key(&press) {
                    key(match action {
                        ResizeKey::Finish(activation) => {
                            arm.arm(activation);
                            Msg::FinishResize
                        }
                        ResizeKey::Step(delta) => Msg::Step(delta),
                    });
                }
                KeyEventResult::Handled
            });
        let shown = KeyBindings::new(crate::keymap::first_of_each(keys), shown);
        let resize_focus = self.resize_focus.clone();
        InputListener::with_raw_key_repeats(shown, move |event| {
            if let InputEvent::Resize { width, .. } = event
                && let Ok(width) = i32::try_from(*width)
            {
                tell(Msg::Width(width));
            }
            if matches!(event, InputEvent::Focus(false)) {
                activation.clear();
            }
            resize
                && (matches!(event, InputEvent::Paste(_))
                    || matches!(event, InputEvent::Key(key)
                        if resize_focus.has_primary_focus() && PaneActivationKey::repeats(key)))
        })
        .into()
    }
}

#[derive(Clone, PartialEq)]
struct ConfirmClose {
    name: Option<Str>,
    losses: Vec<&'static str>,
    archive: bool,
    on_decision: Handler<bool>,
}

struct ConfirmCloseState {
    on_decision: Option<Handler<bool>>,
    cancel: FocusNode,
    close: FocusNode,
}

impl StatefulWidget for ConfirmClose {
    type State = ConfirmCloseState;

    fn create_state(&self) -> Self::State {
        ConfirmCloseState {
            on_decision: Some(self.on_decision.clone()),
            cancel: FocusNode::new(),
            close: FocusNode::new(),
        }
    }
}

impl_widget!(stateful eq ConfirmClose);

impl ConfirmCloseState {
    fn button(
        cx: &mut Cx<'_, Self>,
        label: &'static str,
        node: &FocusNode,
        other: &FocusNode,
        close: bool,
    ) -> AnyWidget {
        let guard = read::<PaneActivationGuard, _>(cx);
        let decide = cx.local_sender();
        let other = other.clone();
        let button = Pressable::new(move |cx, focused| {
            let theme = Theme::of(cx);
            let style = if focused {
                &theme.selection
            } else {
                &theme.primary
            };
            BoxWidget::new()
                .padding(EdgeInsets::x(1))
                .child(Text::line(label).style(style.clone()))
                .into()
        })
        .focus_node(Some(node.clone()))
        .autofocus(!close)
        .behavior(HitTestBehavior::Opaque)
        .on_tap(cx.map(move |_| close))
        .on_key_event(move |_, press| {
            if horizontal_focus(&press).is_some() {
                other.request_focus();
                return KeyEventResult::Handled;
            }
            let Some(key) = PaneActivationKey::of_press(&press) else {
                return KeyEventResult::Ignored;
            };
            guard.arm(key);
            decide(close && key == PaneActivationKey::Enter);
            KeyEventResult::Handled
        });
        Semantics::new(button)
            .role(SemanticsRole::Button)
            .label(label)
            .into()
    }
}

impl State for ConfirmCloseState {
    type Widget = ConfirmClose;
    type Msg = bool;

    fn update(&mut self, close: bool, _: &ConfirmClose, cx: &mut Cx<'_, Self>) -> Rebuild {
        if let Some(on_decision) = self.on_decision.take() {
            cx.emit(&on_decision, close);
            close_dialog(cx, None::<()>);
        }
        Rebuild::No
    }

    fn dispose_with(&mut self, cx: &mut Cx<'_, Self>) {
        self.cancel.dispose();
        self.close.dispose();
        if let Some(on_decision) = self.on_decision.take() {
            cx.emit(&on_decision, false);
        }
    }

    fn build(&self, w: &ConfirmClose, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let words = super::strings::of(cx);
        let actions = Row::new()
            .main_axis_size(MainAxisSize::Min)
            .child(Self::button(
                cx,
                words.side_close_cancel,
                &self.cancel,
                &self.close,
                false,
            ))
            .child(Self::button(
                cx,
                if w.archive {
                    words.side_archive
                } else {
                    words.side_close
                },
                &self.close,
                &self.cancel,
                true,
            ));
        let mut body = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Start);
        if let Some(name) = &w.name {
            body = body.child(Text::line(name.clone()));
        }
        for loss in &w.losses {
            body = body.child(Text::new(*loss));
        }
        let title = if w.archive {
            words.side_archive_title
        } else {
            words.side_close_title
        };
        let dialog = Dialog::new(body.child(actions)).title(title);
        let cancel = self.cancel.clone();
        let close = self.close.clone();
        InputListener::with_raw_key_repeats(dialog, move |event| {
            matches!(event, InputEvent::Key(key)
                if (cancel.has_primary_focus() || close.has_primary_focus())
                    && PaneActivationKey::repeats(key))
        })
        .into()
    }
}
