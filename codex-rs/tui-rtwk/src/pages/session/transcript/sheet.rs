//! The transcript sheet (oracle `pager_overlay/transcript.rs`, Ctrl+T): the
//! whole transcript with its reasoning and warnings, full screen over the
//! chat, read with the pager's keys, and the prompts browsed with Esc.
//!
//! Flutter's full-screen route of a `Column`: the title row, the same
//! transcript list the chat shows (`Pane`, detailed), the status
//! row (the news and the select key, or the browsing keys), two hint rows;
//! the rows around the list are the `Chrome` the opener asks for.
//! The scroll state is the thread's `Follow` controller, shared with the chat
//! so the reader's place survives the sheet; the mode is the thread's `Modes`
//! controller; the document is the thread model's, read through the provider.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use rtwk::focus::{Focus, FocusNode, KeyEventResult};
use rtwk::foundation::{Listenable, ListenableBuilder, VoidCallback};
use rtwk::layout::{Column, CrossAxisAlignment, Expanded, MainAxisSize, SizedBox};
use rtwk::navigation::Navigator;
use rtwk::provider::{read, select};
use rtwk::style::{Span, Style};
use rtwk::text::{Text, TextOverflow, segmenter_of};
use rtwk::{AnyWidget, Cx, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::app::facts::TuiSettings;
use codex_tui_core::keymap::actions::{GLOBAL_FIND_TRANSCRIPT, GLOBAL_FOCUS_ACTIVITY, PAGER_FIND};
use codex_tui_core::keymap::fixed;
use codex_tui_core::screens::backtrack::{EscStep, Targets};
use codex_tui_core::screens::{ScreenThreadIntent, chat};
use codex_tui_core::threads::{HistoryState, ThreadModel};

use super::TranscriptMode;
use super::activity_focus::{self, Step as ActivityStep};
use super::copy::{self, CopyJob};
use super::find;
use super::find_footer;
use super::history;
use super::pager;
use super::select::{self, Chord, Step};
use super::selection;
use super::strings::{self, Strings};
use crate::app::debug::seeded::Seeded;
use crate::app::debug::{DebugEntry, EntryOpen};
use crate::app::view::Hs;
use crate::pages::session::backtrack;
use crate::pages::session::transcript_pane::Pane;
use crate::widgets::finder::{Finder, Mode as FindMode};
use crate::widgets::thread_ui::{Follow, Input, Modes, TextSelection};

/// What the sheet draws around the transcript (the person's decision on the
/// pager chrome: as the oracle has it).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chrome {
    /// The title row, the status row and the hint rows, no percent rule
    /// (`TranscriptOverlay`: the inline overlay and the resume preview).
    Overlay,
    /// The transcript alone, the chat's own footer speaks (the owned
    /// full-screen Ctrl+T and backtrack draw no pager chrome).
    Bare,
    /// The transcript alone as the chat draws it, not as the Ctrl+T sheet does (compact: what a
    /// cell hides stays hidden): the view the oracle's find tests over hidden content drive.
    Compact,
}

/// The sheet over the transcript of the thread below it.
#[derive(Clone, PartialEq)]
pub struct TranscriptSheet {
    pub follow: Follow,
    pub selection: TextSelection,
    pub find: Finder,
    pub modes: Modes,
    pub chrome: Chrome,
}

/// The sheet of the active thread inside a seeded debug entry, with the
/// overlay's chrome.
pub fn seeded(seeded: &Seeded<'_>) -> AnyWidget {
    seeded_with(seeded, Chrome::Overlay)
}

/// The same sheet without chrome, as the owned full-screen Ctrl+T shows it.
pub fn seeded_bare(seeded: &Seeded<'_>) -> AnyWidget {
    seeded_with(seeded, Chrome::Bare)
}

/// The bare sheet over the compact transcript (`Pane::detailed` false), the chat's own rows
/// without its composer.
pub fn seeded_bare_compact(seeded: &Seeded<'_>) -> AnyWidget {
    seeded_with(seeded, Chrome::Compact)
}

fn seeded_with(seeded: &Seeded<'_>, chrome: Chrome) -> AnyWidget {
    let ui = seeded.thread_ui.get(seeded.core.session().active.as_ref());
    TranscriptSheet {
        follow: ui.follow.clone(),
        selection: ui.selection.clone(),
        find: ui.find.clone(),
        modes: ui.modes.clone(),
        chrome,
    }
    .into()
}

/// Widget state: only what the keys carry between two presses (R14); the scroll is the `Follow`
/// controller's, the mode the `Modes` controller's, select mode the thread's `Select`.
pub struct SheetState {
    /// The leader of `select_transcript` was pressed: the next key may complete the chord.
    leader: Rc<Cell<bool>>,
    /// The listener on select mode that closes the sheet when the mode is refused.
    selecting: RefCell<Option<(select::Select, VoidCallback)>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Pager(pager::Msg),
    /// F3 (`/`): Find opens on the sheet's transcript (`begin_search`).
    Find,
    /// F4 (`focus_activity`): the focus on the folds and the cells that disclose begins or ends
    /// (`handle_disclosure_key`). The compact view's only: the detailed transcript has all open.
    Activity,
    /// A copy key or Esc while the reader's selection pauses Find (`handle_selection_key`).
    Selection(selection::Key),
    /// `select_transcript` (`leader [`): select mode begins (`begin_select_mode`).
    Select,
    /// Select mode ended by Exit or by Enter's copy, or was refused (nothing to select): the
    /// sheet closes with it (`ExitSelectMode`, `finish_select_follow`, `pending_select_mode`).
    Left,
    Esc,
    Older,
    Newer,
    Confirm,
    /// Home, Ctrl+Home, Alt+<: to the first row, once every older page is there.
    Beginning,
    /// Ctrl+End, Alt+>: back to the last row.
    Latest,
}

impl StatefulWidget for TranscriptSheet {
    type State = SheetState;

    fn create_state(&self) -> SheetState {
        SheetState {
            leader: Rc::default(),
            selecting: RefCell::new(None),
        }
    }
}

impl_widget!(stateful eq TranscriptSheet);

impl SheetState {
    /// The pane ends select mode where nothing is selectable (`begin_select_mode` refused): the
    /// sheet closes with it, as the overlay that finds nothing to select is done. One listener, on
    /// the select mode of the thread shown.
    fn listen_select(&self, select: &select::Select, send: Rc<dyn Fn(Msg)>) {
        let mut held = self.selecting.borrow_mut();
        if held.as_ref().is_some_and(|(held, _)| held == select) {
            return;
        }
        if let Some((old, listener)) = held.take() {
            old.listenable().remove_listener(&listener);
        }
        let refused = select.clone();
        let listener: VoidCallback = Rc::new(move || {
            if refused.refused() {
                send(Msg::Left);
            }
        });
        select.listenable().add_listener(listener.clone());
        *held = Some((select.clone(), listener));
    }
}

impl State for SheetState {
    type Widget = TranscriptSheet;
    type Msg = Msg;

    fn dispose(&mut self) {
        if let Some((select, listener)) = self.selecting.get_mut().take() {
            select.listenable().remove_listener(&listener);
        }
    }

    fn update(&mut self, msg: Msg, w: &TranscriptSheet, cx: &mut Cx<'_, Self>) -> Rebuild {
        let thread = read::<ThreadModel, _>(cx);
        let targets = Targets::of(thread.transcript(), thread.can_load_history());
        match msg {
            Msg::Find => {
                // a search lets go of the stop in focus (`begin_search`)
                activity_focus::of(&w.find).leave();
                w.find.begin(&w.follow, &w.selection);
            }
            Msg::Activity => {
                let activity = activity_focus::of(&w.find);
                if !activity.focused() {
                    w.selection.controller().clear();
                }
                activity.begin();
            }
            Msg::Select => {
                select::of(&w.find).begin(&w.follow);
                w.modes.hold(Input::Select);
            }
            Msg::Left => {
                // the mode has ended already (`Select::take`, or refused by the pane)
                w.modes.hold(Input::Plain);
                Navigator::maybe_pop(cx);
            }
            Msg::Selection(key) => {
                let held = w.selection.controller();
                let text = held.text();
                if key == selection::Key::Clear {
                    held.clear();
                } else if !text.is_empty() {
                    let hs = read::<Hs, _>(cx);
                    let job = copy::mode_of(&w.find).shape(CopyJob::selected(
                        text,
                        true,
                        &read::<TuiSettings, _>(cx),
                    ));
                    cx.emit(
                        &hs.copy,
                        if key == selection::Key::CopyAndFollow {
                            job.following()
                        } else {
                            job
                        },
                    );
                }
            }
            Msg::Pager(pager::Msg::Rows(rows)) => w.follow.scroll_by(rows),
            Msg::Pager(pager::Msg::Pages(pages)) => w.follow.page(pages),
            Msg::Pager(pager::Msg::HalfPages(pages)) => w.follow.half_page(pages),
            Msg::Pager(pager::Msg::Top) | Msg::Beginning => {
                // the first row, or, with older pages to come, the wait for it: the view holds
                // while each page asks for the next (`jump_to_beginning`)
                match thread.history_state() {
                    HistoryState::Complete => w.follow.jump_to_first(),
                    HistoryState::Loading => w.follow.begin(),
                    HistoryState::Partial | HistoryState::Failed => {
                        w.follow.begin();
                        let hs = read::<Hs, _>(cx);
                        cx.emit(&hs.load_history, thread.id().clone());
                    }
                }
            }
            Msg::Pager(pager::Msg::Bottom) | Msg::Latest => w.follow.jump_to_latest(),
            // closing is the sheet's; what the opener does with it is the opener's
            Msg::Pager(pager::Msg::Close | pager::Msg::Action) => {
                select::of(&w.find).end(&w.selection);
                w.modes.hold(Input::Plain);
                w.modes.reset();
                Navigator::maybe_pop(cx);
            }
            // one Esc starts browsing, the next ends it (`overlay_forward_event`);
            // with no prompt to browse the transcript closes and the chat says so
            // (`begin_overlay_backtrack_preview`)
            Msg::Esc => {
                if w.modes.esc(Some(thread.id()), targets) == EscStep::NoPrevious {
                    let hs = read::<Hs, _>(cx);
                    let intent = ScreenThreadIntent::Chat(chat::ThreadIntent::NoPreviousMessage);
                    cx.emit(&hs.thread_screen, (thread.id().clone(), intent));
                    w.modes.reset();
                    Navigator::maybe_pop(cx);
                }
            }
            Msg::Older if w.modes.browsing() => {
                w.modes.older(targets.count);
            }
            Msg::Newer if w.modes.browsing() => {
                w.modes.newer(targets.count);
            }
            // the prompt chosen: the sheet closes; the edit itself is the
            // thread's intent (`ThreadIntent::EditPrompt`), the chat's to send
            Msg::Confirm => {
                if let Some(chosen) = w.modes.confirm(Some(thread.id()), targets.count) {
                    if let Some(intent) = backtrack::edit_intent(cx, &thread, &chosen) {
                        let hs = read::<Hs, _>(cx);
                        cx.emit(&hs.thread_screen, (thread.id().clone(), intent));
                    }
                    Navigator::maybe_pop(cx);
                }
            }
            Msg::Older | Msg::Newer => {}
        }
        Rebuild::No
    }

    fn build(&self, w: &TranscriptSheet, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let table = strings::of(cx);
        let tokens = crate::app::theme::Tokens::of(cx);
        let (dim, bold, accent) = (tokens.dim(), tokens.bold(), tokens.accent());
        let (follow, modes) = (w.follow.clone(), w.modes.clone());
        let marked = {
            let (follow, modes) = (w.follow.clone(), w.modes.clone());
            let (selection, find) = (w.selection.clone(), w.find.clone());
            let detailed = w.chrome != Chrome::Compact;
            ListenableBuilder::new(modes.listenable(), move |_, _| {
                Pane {
                    follow: follow.clone(),
                    selection: selection.clone(),
                    find: find.clone(),
                    detailed,
                    mark: modes.selected(),
                }
                .into()
            })
        };
        let find = w.find.clone();
        let bottom = (w.chrome == Chrome::Overlay).then(|| {
            // what the thread's history is doing: loaded, loadable, on its way, refused
            let state = select::<ThreadModel, _, _>(cx, ThreadModel::history_state);
            // the reader and the mode both change what the rows below say
            let listenable = <dyn Listenable>::merge([
                Some(follow.listenable()),
                Some(modes.listenable() as Rc<dyn Listenable>),
            ]);
            ListenableBuilder::new(Rc::new(listenable), move |_, _| {
                let browsing = modes.browsing();
                let look = history::Look {
                    accent: accent.clone(),
                    dim: dim.clone(),
                    key: bold.clone(),
                };
                let hints = second(modes.selected().is_some(), table);
                let mut rows = Column::new()
                    .main_axis_size(MainAxisSize::Min)
                    .cross_axis_alignment(CrossAxisAlignment::Stretch);
                if browsing {
                    rows = rows
                        .child(browsing_footer(
                            table,
                            bold.clone(),
                            accent.clone(),
                            dim.clone(),
                        ))
                        .child(pager::hint_row(
                            &pager::navigation(table),
                            table,
                            dim.clone(),
                        ))
                        .child(pager::hint_row(&hints, table, dim.clone()));
                } else if follow.reading() || history::pending(state) {
                    // the reader is away or a page is on its way or refused: one status row, the
                    // hints give way to it (`footer_with_navigation`)
                    rows = rows
                        .child(status(
                            follow.reading(),
                            true,
                            follow.unseen(),
                            state,
                            table,
                            &look,
                        ))
                        // the hints' two rows and the blank one under them stay (`chrome_height` 5)
                        .child(SizedBox::new(None, Some(3), None));
                } else {
                    rows = rows
                        .child(status(false, false, false, state, table, &look))
                        .child(pager::hint_row(
                            &pager::navigation(table),
                            table,
                            dim.clone(),
                        ))
                        .child(pager::hint_row(&hints, table, dim.clone()))
                        .child(SizedBox::new(None, Some(1), None));
                }
                rows.into()
            })
        });
        // while Find is open its query row and status row take the place of the status row and the
        // hints (`footer_with_navigation`), from the first column
        let bottom = match bottom {
            Some(rows) => Some(find_footer::slot_in(cx, &find, rows.into(), 0)),
            None => Some(bare_bottom(cx, &find)),
        };
        let mut column = Column::new().cross_axis_alignment(CrossAxisAlignment::Stretch);
        if w.chrome == Chrome::Overlay {
            column = column.child(pager::header(table.sheet_title, table, tokens.dim()));
        }
        column = column.child(Expanded::new(1, marked));
        if let Some(bottom) = bottom {
            column = column.child(bottom);
        }
        let reading = w.find.clone();
        let send = cx.local_sender();
        let (caret, selection, leader) = (
            select::of(&w.find),
            w.selection.clone(),
            self.leader.clone(),
        );
        // the focus on the folds and the cells: the compact view's, which keeps what a cell hides
        let activity = (w.chrome == Chrome::Compact).then(|| activity_focus::of(&w.find));
        let keymap = crate::keymap::keymap(cx);
        let held = keymap.clone();
        let back: Rc<RefCell<Option<FocusNode>>> = Rc::default();
        self.listen_select(&caret, send.clone());
        let mut focus = Focus::new(column)
            .autofocus()
            // what the reader selected while Find is open: its copy keys and Esc come first
            // (`handle_selection_key`); a match being read (Find accepted): `Ctrl+P`, `Ctrl+N`,
            // Enter (swallowed), Esc and `Ctrl+C` are Find's, before the sheet's own
            // (`handle_search_key`); select mode's keys come before Find's chords and the
            // sheet's own (`handle_select_mode_key`), its chord begins it where neither Find nor
            // the mode asks (`select_transcript`)
            .can_request_focus(true)
            .on_key_event(move |node, key| {
                // the stops take the keys the list's own scroll would take first (Home, Up, the
                // pages): while they have the focus the sheet's node is the primary one, and the
                // node that was gives it back when they let go
                let focusing = activity.is_some()
                    && crate::keymap::pressed(&held, GLOBAL_FOCUS_ACTIVITY, &key);
                if !focusing && stops_let_go(node, activity.as_ref()) {
                    FocusNode::give_back(back.borrow().as_ref(), None);
                }
                if focusing && !node.has_primary_focus() {
                    *back.borrow_mut() = node.manager().and_then(|manager| manager.primary_focus());
                    node.request_focus();
                }
                let result = if reading.paused()
                    && let Some(step) = selection::key(&key)
                {
                    send(Msg::Selection(step));
                    KeyEventResult::Handled
                } else if caret.take(&held, &key, &selection, &reading, &|step| match step {
                    Step::Copy => send(Msg::Selection(selection::Key::Copy)),
                    Step::CopyAndFollow => {
                        send(Msg::Selection(selection::Key::CopyAndFollow));
                        send(Msg::Left);
                    }
                    Step::Exit => send(Msg::Left),
                    Step::Moved | Step::Find => {}
                }) {
                    KeyEventResult::Handled
                } else if reading.mode() != FindMode::Editing
                    && let Some(step) = activity
                        .as_ref()
                        .and_then(|activity| activity.key(&held, &key))
                {
                    // the keys of a list while a stop has the focus, before Find's own while a
                    // match is read (`handle_disclosure_key`); any other key lets go and goes on
                    match step {
                        ActivityStep::Taken => KeyEventResult::Handled,
                        ActivityStep::Left => KeyEventResult::Ignored,
                    }
                } else if !reading.active()
                    && !caret.active()
                    && let Some(chord) = select::chord(&held, &leader, &key)
                {
                    if chord == Chord::Begin {
                        send(Msg::Select);
                    }
                    KeyEventResult::Handled
                } else if reading.reading()
                    && let Some(taken) = find::key(reading.mode(), &key)
                    && reading.take(taken)
                {
                    KeyEventResult::Handled
                } else {
                    KeyEventResult::Ignored
                };
                if !focusing && stops_let_go(node, activity.as_ref()) {
                    FocusNode::give_back(back.borrow().as_ref(), None);
                }
                result
            });
        for (key, msg) in pager::bindings(&keymap, false) {
            focus = focus.bind(key, cx.send(Msg::Pager(msg)));
        }
        let step = |id, msg: Msg| {
            crate::keymap::presses(&keymap, id)
                .into_iter()
                .map(|key| (key, msg.clone()))
                .collect::<Vec<_>>()
        };
        let focusing = if w.chrome == Chrome::Compact {
            step(GLOBAL_FOCUS_ACTIVITY, Msg::Activity)
        } else {
            Vec::new()
        };
        for (key, msg) in [
            focusing,
            step(PAGER_FIND, Msg::Find),
            step(GLOBAL_FIND_TRANSCRIPT, Msg::Find),
            step(fixed::DISMISS, Msg::Esc),
            step(fixed::BROWSE_OLDER, Msg::Older),
            step(fixed::BROWSE_OLDER_VIM, Msg::Older),
            step(fixed::BROWSE_NEWER, Msg::Newer),
            step(fixed::BROWSE_NEWER_VIM, Msg::Newer),
            step(fixed::ENTER, Msg::Confirm),
            step(fixed::TRANSCRIPT_TOP, Msg::Beginning),
            step(fixed::TRANSCRIPT_TOP_ALT, Msg::Beginning),
            step(fixed::TRANSCRIPT_LATEST, Msg::Latest),
            step(fixed::TRANSCRIPT_LATEST_ALT, Msg::Latest),
        ]
        .into_iter()
        .flatten()
        {
            focus = focus.bind(key, cx.send(msg));
        }
        focus.into()
    }
}

/// The footer of the bare views: none while the transcript is only read, as the oracle's view
/// alone has no bottom pane (Find's query is edited all the same, in a field that draws nothing).
/// While select mode (or copy mode) is on the rows of the footer show and take the list's last
/// rows, as they do under the oracle's app: its hints, and Find's query and status over them.
fn bare_bottom(cx: &mut Cx<'_, SheetState>, find: &Finder) -> AnyWidget {
    let nothing = || -> AnyWidget { SizedBox::new(None, Some(0), None).into() };
    let seen = find_footer::slot_in(cx, find, nothing(), 0);
    let unseen = find_footer::unseen_query(cx, find);
    let (caret, copier, activity) = (
        select::of(find),
        copy::mode_of(find),
        activity_focus::of(find),
    );
    // the footer shows while select mode, copy mode or the focus on a stop has something to say
    let listenable = <dyn Listenable>::merge([
        Some(caret.listenable() as Rc<dyn Listenable>),
        Some(activity.listenable() as Rc<dyn Listenable>),
    ]);
    ListenableBuilder::new(Rc::new(listenable), move |_, _| {
        if caret.active() || copier.active() || activity.focused() {
            seen.clone()
        } else {
            unseen.clone()
        }
    })
    .into()
}

/// The second hint row: close, find, browse, and with a prompt marked the
/// keys that edit it (`render_hints`).
fn second(highlight: bool, table: &'static Strings) -> Vec<pager::Pair> {
    let mut pairs = vec![
        (vec![table.key_close], table.hint_close),
        (vec![table.key_find], table.hint_find),
        (vec![table.key_esc], table.hint_browse),
    ];
    if highlight {
        pairs.push((vec![table.key_right], table.hint_edit_next));
        pairs.push((vec![table.key_enter], table.hint_edit_message));
    }
    pairs
}

/// The status row (`status_line_with_navigation`): the news the reader has not seen, what the
/// thread's history is doing or can do (`history`), then the key that goes on from here: the way
/// back to the latest while the reader is away (`away`), nothing while only the history speaks
/// (`passive` without `away`: the footer row of `footer_with_navigation`), the select key
/// otherwise. When the row is too wide the news shortens,
/// then goes, then the history text (the oracle's fallbacks and `first_fitting_line`).
fn status(
    away: bool,
    passive: bool,
    news: bool,
    history: HistoryState,
    table: &'static Strings,
    look: &history::Look,
) -> AnyWidget {
    let look = look.clone();
    let (key, dim) = (look.key.clone(), look.dim.clone());
    LayoutBuilder::new(move |cx, constraints| {
        let segmenter = segmenter_of(cx);
        let navigation = if away {
            vec![
                Span::styled(table.key_end, key.clone()),
                Span::styled(table.latest_action, dim.clone()),
            ]
        } else if passive {
            Vec::new()
        } else {
            vec![
                Span::styled(table.status_select_key, key.clone()),
                Span::styled(table.status_select_action, dim.clone()),
            ]
        };
        let fits = |spans: &[Span]| {
            let cells: usize = spans.iter().map(|span| segmenter.width(&span.text)).sum();
            i32::try_from(cells).unwrap_or(i32::MAX) <= constraints.max_width
        };
        let say = history::Say {
            history,
            news,
            navigation: &navigation,
            selected: false,
        };
        let chosen = if passive {
            history::candidates(table, &look, &say, &fits)
                .into_iter()
                .find(|spans| fits(spans))
                .unwrap_or_default()
        } else {
            history::whole(table, &look, &say)
        };
        Text::rich_line(chosen).overflow(TextOverflow::Clip).into()
    })
    .into()
}

/// The status row while prompts are browsed: the first spelling that fits
/// the row, whole to bare (`prompt_navigation_footer`'s `first_fitting_line`).
fn browsing_footer(table: &'static Strings, key: Style, accent: Style, dim: Style) -> AnyWidget {
    LayoutBuilder::new(move |cx, constraints| {
        let segmenter = segmenter_of(cx);
        let [whole, short] = table.browsing_label;
        let line = |label: &'static str, hints: &[(&'static str, &'static str)]| -> Vec<Span> {
            let mut spans = vec![Span::styled(label, accent.clone())];
            for (keys, action) in hints {
                spans.push(Span::styled(table.hint_joiner, dim.clone()));
                spans.push(Span::styled(*keys, key.clone()));
                if !action.is_empty() {
                    spans.push(Span::styled(table.hint_gap, dim.clone()));
                    spans.push(Span::styled(*action, dim.clone()));
                }
            }
            spans
        };
        let mut full = vec![
            (table.browsing_scroll_keys, table.browsing_scroll),
            (table.browsing_prompt_keys, table.browsing_prompts),
        ];
        let mut compact = vec![(table.browsing_both_keys, "")];
        for hints in [&mut full, &mut compact] {
            hints.push((table.browsing_details_key, table.browsing_details));
            hints.push((table.browsing_rewind_key, table.browsing_rewind));
            hints.push((table.key_esc, table.browsing_back));
        }
        let rewind = (table.browsing_rewind_key, table.browsing_rewind);
        let back = (table.key_esc, table.browsing_back);
        let candidates = [
            line(whole, &full),
            line(short, &full),
            line(short, &compact),
            line(
                short,
                &[(table.browsing_both_keys, ""), rewind, (table.key_esc, "")],
            ),
            line(short, &[rewind, back]),
            line(short, &[back]),
            line(short, &[(table.key_esc, "")]),
            line(short, &[]),
        ];
        let fits = |spans: &Vec<Span>| {
            let cells: usize = spans.iter().map(|span| segmenter.width(&span.text)).sum();
            i32::try_from(cells).unwrap_or(i32::MAX) <= constraints.max_width
        };
        let chosen = candidates
            .iter()
            .find(|spans| fits(spans))
            .cloned()
            .unwrap_or_default();
        Text::rich_line(chosen).overflow(TextOverflow::Clip).into()
    })
    .into()
}

/// The transcript sheet (Ctrl+T, F3) as a mode of the transcript (`mode_list!`).
pub struct Mode;

impl TranscriptMode for Mode {
    const NAME: &'static str = "sheet";

    fn debug_entries() -> Vec<DebugEntry> {
        vec![
            DebugEntry {
                name: "sheet_basic",
                description: "the transcript sheet over a prompt, a reasoning summary and a long answer",
                script: include_str!("fixtures/sheet_basic.jsonl"),
                open: EntryOpen::Route(seeded),
            },
            DebugEntry {
                name: "sheet_owned",
                description: "the transcript sheet without pager chrome (the owned full-screen Ctrl+T)",
                script: include_str!("fixtures/sheet_basic.jsonl"),
                open: EntryOpen::Route(seeded_bare),
            },
            DebugEntry {
                name: "sheet_reveal",
                description: "the transcript sheet over a prompt and a four-line answer",
                script: include_str!("fixtures/sheet_reveal.jsonl"),
                open: EntryOpen::Route(seeded),
            },
            DebugEntry {
                name: "sheet_link",
                description: "the transcript sheet over an answer that is a web link",
                script: include_str!("fixtures/sheet_link.jsonl"),
                open: EntryOpen::Route(seeded),
            },
            DebugEntry {
                name: "sheet_empty",
                description: "the transcript sheet over a thread with no entries (select mode is refused and closes it)",
                script: include_str!("fixtures/sheet_empty.jsonl"),
                open: EntryOpen::Route(seeded),
            },
            DebugEntry {
                name: "sheet_copy_me",
                description: "the transcript sheet over one short answer, `copy me`",
                script: include_str!("fixtures/sheet_copy_me.jsonl"),
                open: EntryOpen::Route(seeded),
            },
            DebugEntry {
                name: "sheet_prompts",
                description: "the transcript sheet over three prompts and their answers",
                script: include_str!("fixtures/sheet_prompts.jsonl"),
                open: EntryOpen::Route(seeded),
            },
        ]
    }
}

/// The sheet's node has the primary focus though no stop has the focus any more.
fn stops_let_go(node: &FocusNode, activity: Option<&activity_focus::Activity>) -> bool {
    node.has_primary_focus() && activity.is_some_and(|activity| !activity.focused())
}
