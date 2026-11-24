// test root: pages
//! Probe of rtwk gap G9 (selection and copy over a virtualized transcript)
//! on rtwk's ADR-029 shape (`ListView::item_text`, `ListView::selection_of`,
//! twk's `ListView(itemText:)`): the app gives each item one line of text, the
//! item draws its own part of the selection, and an app that changes its items
//! keeps a selection by reading `start`/`end` and selecting between the same
//! places after (`SelectionAreaController::select_between`). The oracle's
//! behaviour (codex-rs/tui `transcript_view/selection_tests.rs`):
//! `selection_highlights_hard_breaks_within_and_between_entries` (the text of
//! one entry and the next are joined by one line break) and
//! `live_selection_survives_commits_finalization_new_live_content_and_resize`
//! (a selection keeps its entries when older history is prepended and new
//! content arrives); the oracle copies the source text of every entry the
//! selection reaches, those scrolled out of the view too. An entry of two lines
//! is two items: an item is one row for the selection.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_methods)]

use std::cell::RefCell;
use std::rc::Rc;

use rtwk::gestures::{SelectionArea, SelectionAreaController, SelectionPoint};
use rtwk::scroll::{ItemText, ListView, ScrollController};
use rtwk::style::Span;
use rtwk::widgets::{Builder, Text};
use rtwk_core::input::{Mods, MouseKind};
use rtwk_test::{Options, Tester};

const WIDTH: u16 = 24;
const HEIGHT: u16 = 8;

/// The transcript's model: the lines of its entries, an entry (a cell's
/// source text) being two of them; the app's, as the list's `item_text` reads it.
#[derive(Default)]
struct Entries {
    lines: RefCell<Vec<String>>,
}

fn entry(id: u64) -> [String; 2] {
    [format!("entry {id} a"), format!("entry {id} b")]
}

impl Entries {
    fn new(n: u64) -> Rc<Entries> {
        let e = Entries::default();
        *e.lines.borrow_mut() = (0..n).flat_map(entry).collect();
        Rc::new(e)
    }
    fn insert(&self, at: usize, id: u64) {
        let lines = entry(id);
        let mut all = self.lines.borrow_mut();
        for (i, line) in lines.into_iter().enumerate() {
            all.insert(at + i, line);
        }
    }
    fn count(&self) -> usize {
        self.lines.borrow().len()
    }
}

fn transcript(
    entries: &Rc<Entries>,
    scroll: &ScrollController,
    selection: &SelectionAreaController,
) -> SelectionArea {
    let (model, texts) = (entries.clone(), entries.clone());
    let list = ListView::builder(entries.count(), move |i| {
        let line = model.lines.borrow().get(i).cloned().unwrap_or_default();
        // built again when the part of the selection in the item changes
        Builder::new(move |cx| {
            let spans = match ListView::selection_of(cx, i) {
                Some(selected) => selected.spans(line.clone()),
                None => vec![Span::new(line.clone())],
            };
            Text::rich(spans).into()
        })
        .into()
    })
    .controller(scroll.clone())
    .item_text(move |i, sink| {
        let lines = texts.lines.borrow();
        sink.give(ItemText::new(lines.get(i).map_or("", String::as_str)))
    });
    SelectionArea::new(list).controller(selection.clone())
}

fn joined(from: u64, to: u64) -> String {
    (from..=to).flat_map(entry).collect::<Vec<_>>().join("\n")
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
    // twk's way (no keys): read where the selection is, change the items,
    // select between the same places of the same items. The points are in
    // the one text of the list, so a prepend of two lines moves them by
    // their bytes and their line breaks.
    let (start, end) = (selection.start().unwrap(), selection.end().unwrap());
    let shift: usize = entry(1000).iter().map(|line| line.len() + 1).sum();
    entries.insert(0, 1000);
    t.pump_widget(transcript(&entries, &scroll, &selection));
    selection.select_between(
        SelectionPoint {
            offset: start.offset + shift,
            ..start
        },
        SelectionPoint {
            offset: end.offset + shift,
            ..end
        },
    );
    t.pump();
    assert_eq!(
        selection.text(),
        before,
        "the selection moved off its entries"
    );
}
