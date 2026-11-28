//! The transcript's selection area (oracle `transcript_view/selection.rs`, `input.rs`): rtwk's
//! `SelectionArea` owns the gestures (a drag, a double and a triple click, the highlight, the
//! clear key) and tells what a release copied; the host around it turns what the reader asks to
//! copy into the app's clipboard ask (`Hs.copy`: Flutter's `SelectableRegion` with an
//! `onSelectionChanged` of the page's), copies with a right click (`input.rs` `Down(Right)`),
//! and lets the selection go once a copy it asked for is confirmed
//! (`tui.copy_on_select_clear_selection`; `finish_copy`).
//!
//! The selection is drawn state: it lives in the area and its controller, which the thread's
//! `ThreadUi` keeps so that the page's keys can read it too (`key`).

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

use rtwk::binding::InputListener;
use rtwk::focus::{KeyCombo, KeyPress};
use rtwk::foundation::{Listenable, ListenableBuilder};
use rtwk::gestures::{Listener, MouseTracker, PointerEvent, RenderExcludeSelection, SelectionArea};
use rtwk::navigation::Route;
use rtwk::provider::{Provider, read, watch};
use rtwk::scroll::{ScrollMetrics, ScrollNotification, ScrollUpdateNotification};
use rtwk::style::{Attrs, Style};
use rtwk::{
    AnyWidget, Cx, NotificationListener, Rebuild, State, StatefulWidget, impl_widget, map_static,
};
use rtwk_core::input::{InputEvent, Mods};
use rtwk_render::HitTestResult;
use rtwk_rich::markdown::MarkdownLinkTap;

use codex_tui_core::app::RequestQueue;
use codex_tui_core::app::effects::{CopyDestination, CopyStatus, PrimaryOwner};
use codex_tui_core::app::facts::TuiSettings;
use codex_tui_core::ids::ThreadId;
use codex_tui_core::threads::ThreadModel;

use super::TranscriptMode;
use super::copy::{self, CopyJob, CopyOrigin, Shown};
use crate::app::view::Hs;
use crate::widgets::finder::Finder;
use crate::widgets::markdown::LinkTaps;
use crate::widgets::thread_ui::{Follow, TextSelection};

/// The button a right click is in a `PointerEvent`.
const RIGHT_BUTTON: u8 = 2;

/// The selected reading revision belongs to the same thread as its native range.
#[derive(Clone, Default)]
pub(in crate::pages::session) struct SelectedRevision(Rc<RefCell<ReadingRevision>>);

#[derive(Default)]
struct ReadingRevision {
    range: Option<u64>,
    held: Option<Rc<ThreadModel>>,
}

impl SelectedRevision {
    pub(in crate::pages::session) fn of(finder: &Finder) -> Self {
        finder.held(Self::default)
    }

    pub(in crate::pages::session) fn read(
        &self,
        live: &Rc<ThreadModel>,
        follow: &Follow,
        selection: &TextSelection,
        ordinary: bool,
    ) -> Option<Rc<ThreadModel>> {
        let mut reading = self.0.borrow_mut();
        // A range the active mode already showed stays the mode's when it leaves it behind.
        // A fresh range can start even if a clear and a second selection share one frame.
        let range = selection.range_started_at();
        let started = range.is_some() && range != reading.range;
        reading.range = range;
        let held = &mut reading.held;
        if !follow.holding_selection() {
            held.take();
        }
        if held.as_ref().is_some_and(|before| {
            before.id() != live.id()
                || before.transcript().reset_revision() != live.transcript().reset_revision()
        }) {
            held.take();
            follow.release_selection();
            selection.controller().clear();
            return None;
        }
        if !ordinary || (!selection.held() && !follow.at_tail()) {
            follow.release_selection();
            held.take();
            return None;
        }
        // An older page updates the selected reader. Tail appends and deltas preserve the
        // displayed revision and its copy payload.
        let earlier = held.as_ref().is_some_and(|before| {
            before.transcript().get(0).map(|(key, _)| key)
                != live.transcript().get(0).map(|(key, _)| key)
        });
        if earlier {
            follow.release_selection();
            held.take();
        }
        if held.is_none() && (started || (earlier && range.is_some())) {
            follow.hold_selection(selection);
            *held = Some(live.clone());
        }
        held.clone()
    }
}

/// What the reader selects in: `child` (the transcript's list), served by `selection`; `follow`
/// is the list's reader, whom a copy by Enter takes back to the last row.
#[derive(Clone, PartialEq)]
pub(in crate::pages::session) struct SelectionHost {
    pub child: AnyWidget,
    pub thread: ThreadId,
    pub selection: TextSelection,
    pub follow: Follow,
    pub detailed: bool,
    pub raw: bool,
    /// Select mode of the thread: while only its caret is shown, the caret's cell is drawn bold
    /// and underlined as well as reversed (the oracle's `render_select_cursor`).
    pub caret: bool,
    /// The thread's Find: while it reads a match, Esc is Find's (back to the latest row), not the
    /// area's (let the match go).
    pub find: Finder,
}

/// What a key does while something is selected (oracle `handle_selection_key`): the keys are
/// the selection's before they are the composer's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    /// Ctrl+C, Cmd+C, Ctrl+Shift+C and Ctrl+Insert (`text_selection::is_copy_key`).
    Copy,
    /// Enter: copy, and back to the last row once it is delivered.
    CopyAndFollow,
    /// Esc: let the selection go.
    Clear,
}

/// The chords of a selection (oracle `handle_selection_key`, `text_selection::is_copy_key`): Kitty
/// reports Cmd+C as Super+C, and Windows Terminal forwards Ctrl+Insert for its copy action.
const CHORDS: [(&str, Key); 6] = [
    ("ctrl+c", Key::Copy),
    ("super+c", Key::Copy),
    ("ctrl+shift+c", Key::Copy),
    ("ctrl+insert", Key::Copy),
    ("enter", Key::CopyAndFollow),
    ("esc", Key::Clear),
];

/// The selection's meaning of `key`, if it has one.
pub(in crate::pages::session) fn key(key: &KeyPress) -> Option<Key> {
    CHORDS.iter().find_map(|(spec, step)| {
        KeyCombo::parse(spec)
            .ok()
            .filter(|combo| combo.accepts_press(key))
            .map(|_| *step)
    })
}

pub(in crate::pages::session) enum Msg {
    /// A release copied what its press selected: the text of it.
    Copied(String),
    /// A press of the mouse over the transcript; the right button copies what is selected.
    Press(PointerEvent),
    Drag(PointerEvent),
    Release(PointerEvent),
    CancelPoint,
    Link(MarkdownLinkTap),
}

#[derive(Default)]
pub(in crate::pages::session) struct HostState {
    /// The host has looked at the feedback once: what stood then is older than the host.
    mounted: bool,
    /// The newest answer of a copy this host has taken in: an older one is none of its business.
    answered: Option<u64>,
    /// Screen cell and reader position of a press that started without a range.
    empty_press: Option<PressOrigin>,
    /// A primary press without a drag, paired with its native mouse sequence.
    point_press: Option<u64>,
    /// Whether the current left press can activate a semantic link when released.
    link_press: Rc<Cell<bool>>,
    /// The native Text callback is delivered after the pointer event's listeners.
    link_released: bool,
    /// A local PRIMARY publication belongs to one left press in this transcript.
    primary_press: bool,
    primary_scope: Option<PrimarySelection>,
}

/// Cancels deferred publication when the range is cleared or a reading mode takes it over.
struct PrimarySelection {
    changes: Rc<dyn Listenable>,
    invalidate: Rc<dyn Fn()>,
}

impl PrimarySelection {
    fn new(w: &SelectionHost) -> (Self, PrimaryOwner) {
        let selection = Arc::new(());
        let owner = PrimaryOwner::new(&selection);
        let held = Cell::new(Some(selection));
        let invalidate: Rc<dyn Fn()> = Rc::new(move || {
            held.take();
        });
        let changes = <dyn Listenable>::merge([
            Some(w.selection.listenable()),
            Some(w.find.listenable()),
            Some(super::select::of(&w.find).listenable()),
        ]);
        changes.add_listener(invalidate.clone());
        (
            Self {
                changes,
                invalidate,
            },
            owner,
        )
    }
}

impl Drop for PrimarySelection {
    fn drop(&mut self) {
        self.changes.remove_listener(&self.invalidate);
        (self.invalidate)();
    }
}

#[derive(PartialEq, Eq)]
struct PressOrigin {
    cell: (i32, i32),
    viewport: ScrollMetrics,
}

impl PressOrigin {
    fn capture(follow: &Follow, event: PointerEvent) -> Option<Self> {
        let position = follow.scroll().as_scroll_controller().try_position().ok()?;
        Some(Self {
            cell: (event.screen_x, event.screen_y),
            viewport: position.metrics(),
        })
    }
}

impl StatefulWidget for SelectionHost {
    type State = HostState;

    fn create_state(&self) -> HostState {
        HostState::default()
    }
}

impl_widget!(stateful eq SelectionHost);

impl State for HostState {
    type Widget = SelectionHost;
    type Msg = Msg;

    fn did_update_widget(&mut self, old: &SelectionHost, w: &SelectionHost, cx: &mut Cx<'_, Self>) {
        if old.thread != w.thread || old.selection != w.selection {
            old.selection.set_copy_surface(cx.id(), false);
            w.selection
                .set_copy_surface(cx.id(), Route::is_current_of(cx));
            self.empty_press = None;
            self.point_press = None;
            self.link_press.set(false);
            self.link_released = false;
            self.primary_press = false;
            self.primary_scope = None;
        }
        if old.detailed != w.detailed || old.raw != w.raw {
            w.selection.revise_copy_surface(cx.id());
        }
    }

    fn deactivate(&mut self, w: &SelectionHost, cx: &mut Cx<'_, Self>) {
        w.selection.set_copy_surface(cx.id(), false);
    }

    fn activate(&mut self, w: &SelectionHost, cx: &mut Cx<'_, Self>) {
        w.selection
            .set_copy_surface(cx.id(), Route::is_current_of(cx));
    }

    fn did_change_dependencies(&mut self, w: &SelectionHost, cx: &mut Cx<'_, Self>) {
        w.selection
            .set_copy_surface(cx.id(), Route::is_current_of(cx));
        if !read::<TuiSettings, _>(cx).primary_selection || !w.pointer_route_current(cx, true) {
            self.primary_press = false;
            self.primary_scope = None;
        }
        let shown = watch::<Shown, _>(cx);
        let first = !std::mem::replace(&mut self.mounted, true);
        let Some(feedback) = &shown.0 else {
            return;
        };
        if self.answered == Some(feedback.version) {
            return;
        }
        self.answered = Some(feedback.version);
        if first || feedback.result != Ok(CopyStatus::Confirmed) {
            return;
        }
        let Some(completion) = &feedback.completion else {
            return;
        };
        if w.copy_origin(cx).as_ref() != Some(&completion.origin) {
            return;
        }
        if completion.clear {
            w.selection.revise_copy_surface(cx.id());
            super::select::of(&w.find).end_extension();
            w.selection.controller().clear();
        }
        if completion.follow {
            w.follow.jump_to_latest();
        }
    }

    fn update(&mut self, msg: Msg, w: &SelectionHost, cx: &mut Cx<'_, Self>) -> Rebuild {
        let job = match msg {
            Msg::Copied(text) => {
                let settings = read::<TuiSettings, _>(cx);
                if settings.primary_selection {
                    take_native_copy(cx, &text);
                    if !std::mem::take(&mut self.primary_press)
                        || !w.pointer_route_current(cx, false)
                    {
                        return Rebuild::No;
                    }
                    let (scope, owner) = PrimarySelection::new(w);
                    self.primary_scope = Some(scope);
                    let origin = settings.copy_on_select.then(|| w.copy_origin(cx)).flatten();
                    CopyJob::primary(text, &settings, owner, origin)
                } else {
                    CopyJob::selected(text, false, &settings, w.copy_origin(cx))
                }
            }
            Msg::Press(press) if press.button == RIGHT_BUTTON => CopyJob::selected(
                w.selection.controller().text(),
                true,
                &read::<TuiSettings, _>(cx),
                w.copy_origin(cx),
            ),
            Msg::Press(press) => {
                super::select::of(&w.find).end_extension();
                if press.button == 0 {
                    // Only the primary press can start the reader's own selection.
                    w.find.pressed();
                    w.selection.revise_copy_surface(cx.id());
                    self.primary_scope = None;
                    self.primary_press = read::<TuiSettings, _>(cx).primary_selection
                        && w.pointer_route_current(cx, false);
                    self.empty_press = if w.selection.controller().has_selection() {
                        None
                    } else {
                        PressOrigin::capture(&w.follow, press)
                    };
                    self.point_press = if self.empty_press.is_some() && press.modifiers.is_empty() {
                        MouseTracker::of(cx).map(|mouse| mouse.presses())
                    } else {
                        None
                    };
                    self.link_press.set(
                        self.empty_press.is_some()
                            && link_modifiers(press.modifiers)
                            && w.links_enabled(cx),
                    );
                    self.link_released = false;
                }
                return Rebuild::No;
            }
            Msg::Drag(pointer) => {
                if pointer.button == 0 {
                    w.selection.revise_copy_surface(cx.id());
                    self.point_press = None;
                    self.link_press.set(false);
                }
                return Rebuild::No;
            }
            Msg::Release(release) => {
                if release.button == 0 {
                    let press = self.point_press.take();
                    let at_origin = self.empty_press.take().is_some_and(|origin| {
                        PressOrigin::capture(&w.follow, release) == Some(origin)
                    });
                    self.link_released = self.link_press.replace(false)
                        && at_origin
                        && link_modifiers(release.modifiers)
                        && w.links_enabled(cx);
                    if at_origin && w.selection.controller().has_selection() {
                        w.selection.controller().clear();
                    } else if at_origin
                        && release.modifiers.is_empty()
                        && let Some(press) = press
                        && w.links_enabled(cx)
                    {
                        hold_clicked_point(w, cx, release, press);
                    }
                }
                return Rebuild::No;
            }
            Msg::CancelPoint => {
                self.point_press = None;
                w.selection.clear_pointer_point();
                return Rebuild::No;
            }
            Msg::Link(link) => {
                if std::mem::take(&mut self.link_released) && w.links_enabled(cx) {
                    w.selection.revise_copy_surface(cx.id());
                    crate::features::open_transcript_link(cx, w.thread.clone(), link.destination);
                }
                return Rebuild::No;
            }
        };
        if job.text.is_empty() {
            return Rebuild::No;
        }
        let handlers = read::<Hs, _>(cx);
        let job = if matches!(job.destination, CopyDestination::Primary(_)) {
            job
        } else {
            copy::mode_of(&w.find).marked(job)
        };
        cx.emit(&handlers.copy, job);
        Rebuild::No
    }

    fn build(&self, w: &SelectionHost, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let settings = watch::<TuiSettings, _>(cx);
        let attrs = if w.caret {
            Attrs::REVERSE | Attrs::BOLD | Attrs::UNDERLINE
        } else {
            Attrs::REVERSE
        };
        // The child sees the release before the selection area does. A single-cell drag that
        // returned to its origin has an empty range in the oracle, so clear it before copy on
        // release. Presses that selected a word or line keep what their unit selected.
        let child = Listener::new()
            .on_pointer_up(map_static!(cx, PointerEvent => Msg, Msg::Release))
            .child(w.child.clone());
        // A wheel gesture cancels a possible activation even at the scroll limit.
        let pressed = self.link_press.clone();
        let cancel = cx.local_sender();
        let child =
            NotificationListener::<ScrollNotification>::new(child).on_notification(move |_| {
                pressed.set(false);
                cancel(Msg::CancelPoint);
                false
            });
        let selection = w.selection.clone();
        let surface = cx.id();
        let child = NotificationListener::<ScrollUpdateNotification>::new(child).on_notification(
            move |scroll| {
                if scroll.depth() == 0 {
                    selection.revise_copy_surface(surface);
                }
                false
            },
        );
        let child: AnyWidget =
            Provider::value(LinkTaps(map_static!(cx, MarkdownLinkTap => Msg, Msg::Link)))
                .child(child)
                .into();
        let controller = w.selection.controller();
        let copy_on_select = settings.copy_on_select || settings.primary_selection;
        let (find, on_copy) = (w.find.clone(), cx.map(Msg::Copied));
        // the area clears what is selected on Esc where the keys reach it with no focus to start
        // from (the sheet has no composer): the match Find shows is selected, and Esc is Find's
        // while it is read
        let area = ListenableBuilder::new(w.find.listenable(), move |_, _| {
            let area = SelectionArea::new(child.clone())
                // the oracle draws the selection REVERSED on top of the cell's own style
                // (`transcript_view/selection.rs`; `rendered_selection`)
                .style(Style::new().attrs(attrs).pen())
                .controller(controller.clone())
                .copy_on_release(copy_on_select)
                .on_copy(on_copy.clone());
            // the area's own clear key (Esc) is Find's while Find reads
            if find.reading() {
                area.clear_key(None).into()
            } else {
                area.into()
            }
        });
        let reader = Listener::new()
            .on_pointer_down(map_static!(cx, PointerEvent => Msg, Msg::Press))
            .on_pointer_move(map_static!(cx, PointerEvent => Msg, Msg::Drag))
            .child(area);
        let cancel = cx.local_sender();
        InputListener::new(reader, move |event| {
            if matches!(event, InputEvent::Focus(false) | InputEvent::Resize { .. }) {
                cancel(Msg::CancelPoint);
            }
            false
        })
        .into()
    }
}

fn hold_clicked_point(
    widget: &SelectionHost,
    cx: &mut Cx<'_, HostState>,
    event: PointerEvent,
    press: u64,
) {
    let owner = cx.id();
    let Some(mouse) = MouseTracker::of(cx) else {
        return;
    };
    let selection = widget.selection.clone();
    let identity = selection.copy_identity();
    let _ = cx.request_viewport(
        move |viewport| {
            if mouse.presses() != press
                || identity != selection.copy_identity()
                || !viewport.has_current_layout_size()
            {
                return false;
            }
            let Some(surface) = viewport.render_object_of(owner) else {
                return false;
            };
            let tree = viewport.render_tree();
            let mut hits = HitTestResult::new();
            tree.hit_test(&mut hits, event.screen_x, event.screen_y);
            let mut selectable = false;
            for (id, x, y) in hits.iter() {
                if id == surface {
                    if selectable {
                        selection.hold_pointer_point(owner, press);
                    }
                    return selectable;
                }
                if tree.get::<RenderExcludeSelection>(id).is_some() {
                    selectable = false;
                } else if let Some(text) = tree.selectable(id) {
                    let (start, end) = text.selectable_range_in(tree, id, x, y);
                    selectable |= start < end;
                }
            }
            false
        },
        cx.callback_with(|_, _, _| Rebuild::No),
    );
}

fn link_modifiers(modifiers: Mods) -> bool {
    modifiers.is_empty() || modifiers.contains(Mods::CTRL) || modifiers.contains(Mods::SUPER)
}

fn take_native_copy(cx: &Cx<'_, HostState>, text: &str) {
    if let Some(tracker) = MouseTracker::of(cx) {
        // SelectionArea appends its OSC 52 payload just before invoking on_copy synchronously.
        // The typed host effect handles this one, preserving any earlier requests.
        let mut queued = tracker.take_clipboard();
        if queued.last().is_some_and(|last| last == text) {
            queued.pop();
        }
        for copy in queued {
            tracker.queue_clipboard(copy);
        }
    }
}

impl SelectionHost {
    fn copy_origin(&self, cx: &mut Cx<'_, HostState>) -> Option<CopyOrigin> {
        let surface = cx.id();
        CopyOrigin::capture(&self.selection, &self.follow, &self.find, cx)
            .filter(|origin| origin.selection.belongs_to(surface))
    }

    fn links_enabled(&self, cx: &mut Cx<'_, HostState>) -> bool {
        let select = super::select::of(&self.find);
        self.pointer_route_current(cx, false)
            && !self.find.active()
            && !select.active()
            && !select.extending()
            && !copy::mode_of(&self.find).active()
    }

    fn pointer_route_current(&self, cx: &mut Cx<'_, HostState>, listen: bool) -> bool {
        Route::is_current_of(cx)
            && !Provider::<RequestQueue>::maybe_of(cx, listen).is_some_and(|queue| {
                queue
                    .entries
                    .iter()
                    .any(|entry| entry.key.thread == self.thread)
            })
    }
}

/// The selection area as a mode of the transcript (`mode_list!`): no fixture states of its own.
pub struct Mode;

impl TranscriptMode for Mode {
    const NAME: &'static str = "select";
}
