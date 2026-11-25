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

use std::rc::Rc;

use rtwk::focus::{Focus, KeyEventResult};
use rtwk::foundation::{Listenable, ListenableBuilder};
use rtwk::layout::{Column, CrossAxisAlignment, Expanded, MainAxisSize, SizedBox};
use rtwk::navigation::Navigator;
use rtwk::provider::{read, select};
use rtwk::style::{Span, Style};
use rtwk::text::{Text, TextOverflow, segmenter_of};
use rtwk::{AnyWidget, Cx, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::app::facts::TuiSettings;
use codex_tui_core::keymap::actions::{GLOBAL_FIND_TRANSCRIPT, PAGER_FIND};
use codex_tui_core::keymap::fixed;
use codex_tui_core::screens::backtrack::{EscStep, Targets};
use codex_tui_core::screens::{ScreenThreadIntent, chat};
use codex_tui_core::threads::{HistoryState, ThreadModel};

use super::TranscriptMode;
use super::copy::{self, CopyJob};
use super::find;
use super::find_footer;
use super::history;
use super::pager;
use super::selection;
use super::strings::{self, Strings};
use crate::app::debug::seeded::Seeded;
use crate::app::debug::{DebugEntry, EntryOpen};
use crate::app::view::Hs;
use crate::pages::session::backtrack;
use crate::pages::session::transcript_pane::Pane;
use crate::widgets::finder::Finder;
use crate::widgets::thread_ui::{Follow, Modes, TextSelection};

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

/// Widget state: none (R14); the scroll is the `Follow` controller's, the
/// mode the `Modes` controller's.
pub struct SheetState;

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Pager(pager::Msg),
    /// F3 (`/`): Find opens on the sheet's transcript (`begin_search`).
    Find,
    /// A copy key or Esc while the reader's selection pauses Find (`handle_selection_key`).
    Selection(selection::Key),
    Esc,
    Older,
    Newer,
    Confirm,
}

impl StatefulWidget for TranscriptSheet {
    type State = SheetState;

    fn create_state(&self) -> SheetState {
        SheetState
    }
}

impl_widget!(stateful eq TranscriptSheet);

impl State for SheetState {
    type Widget = TranscriptSheet;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, w: &TranscriptSheet, cx: &mut Cx<'_, Self>) -> Rebuild {
        let thread = read::<ThreadModel, _>(cx);
        let targets = Targets::of(thread.transcript(), thread.can_load_history());
        match msg {
            Msg::Find => w.find.begin(&w.follow, &w.selection),
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
            Msg::Pager(pager::Msg::Top) => w.follow.jump_to_first(),
            Msg::Pager(pager::Msg::Bottom) => w.follow.jump_to_latest(),
            // closing is the sheet's; what the opener does with it is the opener's
            Msg::Pager(pager::Msg::Close | pager::Msg::Action) => {
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
                } else if follow.away() {
                    // the reader is away: one status row, the hints give way to it
                    // (`footer_with_navigation`)
                    rows = rows
                        .child(status(
                            true,
                            follow.unseen(),
                            state,
                            table,
                            &bold,
                            &accent,
                            &dim,
                        ))
                        .child(SizedBox::new(None, Some(2), None));
                } else {
                    rows = rows
                        .child(status(false, false, state, table, &bold, &accent, &dim))
                        .child(pager::hint_row(
                            &pager::navigation(table),
                            table,
                            dim.clone(),
                        ))
                        .child(pager::hint_row(&hints, table, dim.clone()));
                }
                rows.into()
            })
        });
        // while Find is open its query row and status row take the place of the status row and the
        // hints (`footer_with_navigation`), from the first column
        let bottom = match bottom {
            Some(rows) => Some(find_footer::slot_in(cx, &find, rows.into(), 0)),
            // the bare view draws no footer: Find's query is edited all the same
            None => Some(find_footer::unseen_query(cx, &find)),
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
        let mut focus = Focus::new(column)
            .autofocus()
            // what the reader selected while Find is open: its copy keys and Esc come first
            // (`handle_selection_key`); a match being read (Find accepted): `Ctrl+P`, `Ctrl+N`,
            // Enter (swallowed), Esc and `Ctrl+C` are Find's, before the sheet's own
            // (`handle_search_key`)
            .on_key_event(move |_, key| {
                if reading.paused()
                    && let Some(step) = selection::key(&key)
                {
                    send(Msg::Selection(step));
                    KeyEventResult::Handled
                } else if reading.reading()
                    && let Some(taken) = find::key(reading.mode(), &key)
                    && reading.take(taken)
                {
                    KeyEventResult::Handled
                } else {
                    KeyEventResult::Ignored
                }
            });
        let keymap = crate::keymap::keymap(cx);
        for (key, msg) in pager::bindings(&keymap, false) {
            focus = focus.bind(key, cx.send(Msg::Pager(msg)));
        }
        let step = |id, msg: Msg| {
            crate::keymap::presses(&keymap, id)
                .into_iter()
                .map(|key| (key, msg.clone()))
                .collect::<Vec<_>>()
        };
        for (key, msg) in [
            step(PAGER_FIND, Msg::Find),
            step(GLOBAL_FIND_TRANSCRIPT, Msg::Find),
            step(fixed::DISMISS, Msg::Esc),
            step(fixed::BROWSE_OLDER, Msg::Older),
            step(fixed::BROWSE_OLDER_VIM, Msg::Older),
            step(fixed::BROWSE_NEWER, Msg::Newer),
            step(fixed::BROWSE_NEWER_VIM, Msg::Newer),
            step(fixed::ENTER, Msg::Confirm),
        ]
        .into_iter()
        .flatten()
        {
            focus = focus.bind(key, cx.send(msg));
        }
        focus.into()
    }
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
/// back to the latest while the reader is away (`away`, the passive row of
/// `footer_with_navigation`), the select key otherwise. When the row is too wide the news shortens,
/// then goes, then the history text (the oracle's fallbacks and `first_fitting_line`).
fn status(
    away: bool,
    news: bool,
    history: HistoryState,
    table: &'static Strings,
    key: &Style,
    accent: &Style,
    dim: &Style,
) -> AnyWidget {
    let look = history::Look {
        accent: accent.clone(),
        dim: dim.clone(),
        key: key.clone(),
    };
    let (key, dim) = (key.clone(), dim.clone());
    LayoutBuilder::new(move |cx, constraints| {
        let segmenter = segmenter_of(cx);
        let (keys, action) = if away {
            (table.key_end, table.latest_action)
        } else {
            (table.status_select_key, table.status_select_action)
        };
        let navigation = vec![
            Span::styled(keys, key.clone()),
            Span::styled(action, dim.clone()),
        ];
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
        let chosen = if away {
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
                name: "sheet_prompts",
                description: "the transcript sheet over three prompts and their answers",
                script: include_str!("fixtures/sheet_prompts.jsonl"),
                open: EntryOpen::Route(seeded),
            },
        ]
    }
}
