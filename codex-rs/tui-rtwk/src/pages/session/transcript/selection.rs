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

use rtwk::focus::{KeyCombo, KeyPress};
use rtwk::foundation::ListenableBuilder;
use rtwk::gestures::{Listener, PointerEvent, SelectionArea};
use rtwk::provider::{read, watch};
use rtwk::scroll::ScrollMetrics;
use rtwk::style::{Attrs, Style};
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::app::effects::CopyStatus;
use codex_tui_core::app::facts::TuiSettings;

use super::TranscriptMode;
use super::copy::{self, CopyJob, Shown};
use crate::app::view::Hs;
use crate::widgets::finder::Finder;
use crate::widgets::thread_ui::{Follow, TextSelection};

/// The button a right click is in a `PointerEvent`.
const RIGHT_BUTTON: u8 = 2;

/// What the reader selects in: `child` (the transcript's list), served by `selection`; `follow`
/// is the list's reader, whom a copy by Enter takes back to the last row.
#[derive(Clone, PartialEq)]
pub(in crate::pages::session) struct SelectionHost {
    pub child: AnyWidget,
    pub selection: TextSelection,
    pub follow: Follow,
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
    Release(PointerEvent),
}

#[derive(Default)]
pub(in crate::pages::session) struct HostState {
    /// The host has looked at the feedback once: what stood then is older than the host.
    mounted: bool,
    /// The newest answer of a copy this host has taken in: an older one is none of its business.
    answered: Option<u64>,
    /// Screen cell and reader position of a press that started without a range.
    empty_press: Option<PressOrigin>,
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

    fn did_change_dependencies(&mut self, w: &SelectionHost, cx: &mut Cx<'_, Self>) {
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
        // the answer a copy was waiting for: its selection goes, and only that one
        if let Some(text) = &feedback.clear_after
            && w.selection.controller().text() == *text
        {
            super::select::of(&w.find).end_extension();
            w.selection.controller().clear();
        }
        if feedback.follow {
            w.follow.jump_to_latest();
        }
    }

    fn update(&mut self, msg: Msg, w: &SelectionHost, cx: &mut Cx<'_, Self>) -> Rebuild {
        let (text, explicit) = match msg {
            Msg::Copied(text) => (text, false),
            Msg::Press(press) if press.button == RIGHT_BUTTON => {
                (w.selection.controller().text(), true)
            }
            // a press that is not a copy is the reader selecting: not Find's match any more
            Msg::Press(press) => {
                super::select::of(&w.find).end_extension();
                if press.button == 0 {
                    self.empty_press = if w.selection.controller().has_selection() {
                        None
                    } else {
                        PressOrigin::capture(&w.follow, press)
                    };
                }
                w.find.pressed();
                return Rebuild::No;
            }
            Msg::Release(release) => {
                if release.button == 0
                    && let Some(origin) = self.empty_press.take()
                    && PressOrigin::capture(&w.follow, release) == Some(origin)
                    && w.selection.controller().has_selection()
                {
                    w.selection.controller().clear();
                }
                return Rebuild::No;
            }
        };
        if text.is_empty() {
            return Rebuild::No;
        }
        let settings = read::<TuiSettings, _>(cx);
        let handlers = read::<Hs, _>(cx);
        let job = CopyJob::selected(text, explicit, &settings);
        cx.emit(&handlers.copy, copy::mode_of(&w.find).marked(job));
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
            .on_pointer_up(cx.map(Msg::Release))
            .child(w.child.clone());
        let controller = w.selection.controller();
        let copy_on_select = settings.copy_on_select;
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
        Listener::new()
            .on_pointer_down(cx.map(Msg::Press))
            .child(area)
            .into()
    }
}

/// The selection area as a mode of the transcript (`mode_list!`): no fixture states of its own.
pub struct Mode;

impl TranscriptMode for Mode {
    const NAME: &'static str = "select";
}
