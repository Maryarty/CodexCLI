// test root: pages
//! Probe of rtwk gap G9 (selection and copy over a virtualized transcript)
//! against rtwk's ADR-028 (`ListView::selection_text`). The oracle's
//! behaviour (codex-rs/tui `transcript_view/selection_tests.rs`):
//! `selection_highlights_hard_breaks_within_and_between_entries` (the text of
//! one entry and the next are joined by one line break) and
//! `live_selection_survives_commits_finalization_new_live_content_and_resize`
//! (a selection keeps its entries when older history is prepended and new
//! content arrives); the oracle copies the source text of every entry the
//! selection reaches, those scrolled out of the view too.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_methods)]

use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use rtwk::gestures::{SelectionArea, SelectionAreaController};
use rtwk::scroll::{ItemTexts, ListView, ScrollController};
use rtwk::widgets::Text;
use rtwk_core::input::{Mods, MouseKind};
use rtwk_test::{Options, Tester};

const WIDTH: u16 = 24;
const HEIGHT: u16 = 8;

/// The transcript's model: an entry (a cell's source text, two lines) and an
/// id each; the app's, as `ItemTexts`.
#[derive(Default)]
struct Entries {
    items: RefCell<Vec<(u64, String)>>,
    revision: Cell<u64>,
    changed: Cell<usize>,
}

fn entry(id: u64) -> String {
    format!("entry {id} a\nentry {id} b")
}

impl Entries {
    fn new(n: u64) -> Rc<Entries> {
        let e = Entries::default();
        *e.items.borrow_mut() = (0..n).map(|i| (i, entry(i))).collect();
        Rc::new(e)
    }
    fn insert(&self, at: usize, id: u64) {
        self.items.borrow_mut().insert(at, (id, entry(id)));
        self.changed.set(at);
        self.revision.set(self.revision.get() + 1);
    }
}

impl ItemTexts for Entries {
    fn count(&self) -> usize {
        self.items.borrow().len()
    }
    fn text(&self, index: usize) -> Cow<'_, str> {
        Cow::Owned(self.items.borrow()[index].1.clone())
    }
    fn revision(&self) -> u64 {
        self.revision.get()
    }
    fn changed_from(&self, _revision: u64) -> usize {
        self.changed.get()
    }
    fn key(&self, index: usize) -> Option<u64> {
        Some(self.items.borrow()[index].0)
    }
}

fn transcript(
    entries: &Rc<Entries>,
    scroll: &ScrollController,
    selection: &SelectionAreaController,
) -> SelectionArea {
    let model = entries.clone();
    let list = ListView::builder(entries.count(), move |i| {
        Text::new(
            model
                .items
                .borrow()
                .get(i)
                .map_or_else(String::new, |e| e.1.clone()),
        )
        .into()
    })
    .controller(scroll.clone())
    .selection_text(entries.clone());
    SelectionArea::new(list).controller(selection.clone())
}

fn joined(from: u64, to: u64) -> String {
    (from..=to).map(entry).collect::<Vec<_>>().join("\n")
}

fn drag_through(t: &mut Tester, scroll: &ScrollController, rows: i64) {
    t.mouse(MouseKind::Down, 0, 0, 0, Mods::NONE);
    t.mouse(MouseKind::Drag, 5, HEIGHT - 1, 0, Mods::NONE);
    while scroll.offset() < rows {
        t.pump_for(std::time::Duration::from_millis(50));
    }
    t.mouse(MouseKind::Up, 5, HEIGHT - 1, 0, Mods::NONE);
}

#[test]
fn a_drag_from_the_first_entry_copies_the_source_of_entries_scrolled_out() {
    let (entries, scroll, selection) = (
        Entries::new(300),
        ScrollController::new(),
        SelectionAreaController::new(),
    );
    let mut t = Tester::with(Options::new().size(WIDTH, HEIGHT));
    t.pump_widget(transcript(&entries, &scroll, &selection));
    drag_through(&mut t, &scroll, 100);
    let got = selection.text();
    // oracle: the text of every entry the drag reached, entries joined by one
    // line break; the last row of the window is where the drag ended
    let last_row = t.screen().lines().last().unwrap().trim_end().to_owned();
    let id: u64 = last_row.split(' ').nth(1).unwrap().parse().unwrap();
    assert!(id >= 49, "entries scrolled out: reached entry {id}");
    let mut want = joined(0, id - 1);
    want.push('\n');
    if last_row.ends_with('b') {
        want.push_str(&format!("entry {id} a\n"));
    }
    want.push_str(&last_row[..6]);
    assert_eq!(got, want, "{} bytes copied", got.len());
}

#[test]
fn a_selection_keeps_its_entries_when_older_history_is_prepended() {
    // oracle: live_selection_survives_commits_finalization_new_live_content_and_resize
    let (entries, scroll, selection) = (
        Entries::new(40),
        ScrollController::new(),
        SelectionAreaController::new(),
    );
    let mut t = Tester::with(Options::new().size(WIDTH, HEIGHT));
    t.pump_widget(transcript(&entries, &scroll, &selection));
    t.mouse(MouseKind::Down, 0, 0, 0, Mods::NONE);
    t.mouse(MouseKind::Drag, 9, 3, 0, Mods::NONE);
    t.mouse(MouseKind::Up, 9, 3, 0, Mods::NONE);
    let before = selection.text();
    assert!(
        before.starts_with("entry 0 a\nentry 0 b\nentry 1 a"),
        "{before:?}"
    );
    entries.insert(0, 1000);
    t.pump_widget(transcript(&entries, &scroll, &selection));
    assert_eq!(
        selection.text(),
        before,
        "the selection moved off its entries"
    );
}
