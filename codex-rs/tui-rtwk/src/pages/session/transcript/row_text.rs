//! Shared row text for the search model and the visible selection model.

use std::borrow::Cow;
use std::collections::HashMap;
use std::ops::Range;
use std::rc::{Rc, Weak};

use codex_tui_core::app::facts::Rendering;
use codex_tui_core::ids::ThreadId;
use codex_tui_core::threads::{Item, ItemKey, ThreadModel, Transcript};

use super::find::RowTexts;
use super::item_text::{Reading, row_parts};
use super::turn_details::{Kind, Row};
use crate::pages::session::cells::text::Part;
use crate::widgets::measure::Measure;
use crate::widgets::thread_ui::Disclosed;

// Whole-output search may materialize a large row but must not keep it in this cache across frames.
const ROW_TEXT_LIMIT: usize = 64 * 1024;
const RETAINED_TEXT_LIMIT: usize = 8 * 1024 * 1024;

/// The row's visible text, with ranges that can be masked for Find.
#[derive(PartialEq, Eq)]
pub(super) struct RowText {
    pub(super) text: String,
    pub(super) gutters: Vec<bool>,
    hidden: Vec<Range<usize>>,
}

impl RowText {
    fn from_parts(parts: Vec<Part>) -> Self {
        let mut row = Self {
            text: String::new(),
            gutters: Vec::new(),
            hidden: Vec::new(),
        };
        for (index, part) in parts.into_iter().enumerate() {
            if index > 0 {
                row.text.push('\n');
            }
            let start = row.text.len();
            row.text.push_str(&part.text);
            if part.gutter && row.text.len() > start {
                row.hidden.push(start..row.text.len());
            }
            row.gutters
                .extend(part.text.split('\n').map(|_| part.gutter));
        }
        row
    }

    fn allocated_bytes(&self) -> usize {
        self.text.capacity()
            + self.gutters.capacity()
            + self.hidden.capacity() * std::mem::size_of::<Range<usize>>()
    }

    /// Search cannot match a gutter. Keep its byte offsets without storing a second row.
    pub(super) fn without_gutters(&self) -> Cow<'_, str> {
        if self.hidden.is_empty() {
            return Cow::Borrowed(&self.text);
        }
        let mut masked = String::with_capacity(self.text.len());
        let mut written = 0;
        for range in &self.hidden {
            masked.push_str(&self.text[written..range.start]);
            masked.extend(std::iter::repeat_n('\0', range.len()));
            written = range.end;
        }
        masked.push_str(&self.text[written..]);
        Cow::Owned(masked)
    }
}

#[derive(PartialEq, Eq)]
struct Context {
    thread: ThreadId,
    // The pane reads only the published translation tables. Their addresses identify them.
    cells: *const crate::pages::session::cells::strings::Strings,
    chat: *const crate::pages::session::strings::Strings,
    status: *const crate::pages::session::cells::status::strings::Strings,
    approvals: *const crate::sheets::approvals::strings::Strings,
    cwd: Option<String>,
    bullet: String,
    rendering: Rendering,
    detailed: bool,
    expanded: bool,
    inline: bool,
    raw: bool,
    raw_reasoning: bool,
    width: i32,
    measure: Measure,
}

impl Context {
    fn of(thread: &ThreadModel, reading: &Reading<'_>) -> Self {
        Self {
            thread: thread.id().clone(),
            cells: reading.env.cells,
            chat: reading.env.chat,
            status: reading.status_words,
            approvals: reading.approval_words,
            cwd: reading.cwd.map(str::to_owned),
            bullet: reading.bullet.to_owned(),
            rendering: reading.rendering,
            detailed: reading.env.detailed,
            expanded: reading.env.expanded,
            inline: reading.inline,
            raw: reading.raw,
            raw_reasoning: reading.raw_reasoning,
            width: reading.room.width,
            measure: reading.room.segmenter,
        }
    }
}

struct Entry {
    revision: u64,
    rest: Vec<(ItemKey, u64)>,
    revealed: bool,
    text: Rc<RowText>,
}

type Projections = [Option<Rc<Entry>>; 2];

fn retained_bytes(projections: &Projections) -> usize {
    let [mounted, scan] = projections;
    let mounted_size = mounted
        .as_ref()
        .map_or(0, |entry| entry.text.allocated_bytes());
    let scan_size = scan
        .as_ref()
        .filter(|entry| {
            mounted
                .as_ref()
                .is_none_or(|other| !Rc::ptr_eq(entry, other))
        })
        .map_or(0, |entry| entry.text.allocated_bytes());
    mounted_size + scan_size
}

fn members<'a>(transcript: &'a Transcript, row: &Row) -> impl Iterator<Item = (&'a ItemKey, u64)> {
    row.range.clone().skip(1).filter_map(|index| {
        let (key, _) = transcript.get(index)?;
        Some((key, transcript.item_revision(key)?))
    })
}

/// One representation for each reader projection of an item. The search and the mounted
/// selection share the allocation whenever their members and disclosure agree.
pub(in crate::pages::session) struct RowTextCache {
    context: Option<Context>,
    order: Weak<Vec<ItemKey>>,
    entries: HashMap<ItemKey, Projections>,
    cached_bytes: usize,
    blank: Rc<RowText>,
    #[cfg(test)]
    prepared: usize,
}

impl Default for RowTextCache {
    fn default() -> Self {
        Self {
            context: None,
            order: Weak::new(),
            entries: HashMap::new(),
            cached_bytes: 0,
            blank: Rc::new(RowText {
                text: String::new(),
                gutters: vec![true],
                hidden: Vec::new(),
            }),
            #[cfg(test)]
            prepared: 0,
        }
    }
}

impl RowTextCache {
    pub(in crate::pages::session) fn clear(&mut self) {
        if self.context.take().is_some() {
            self.entries = HashMap::new();
            self.cached_bytes = 0;
            self.order = Weak::new();
        }
    }

    fn prepare(
        &mut self,
        transcript: &Transcript,
        row: &Row,
        key: &ItemKey,
        item: &Item,
        reading: &Reading<'_>,
        scan: bool,
    ) -> Rc<RowText> {
        if let Kind::Tools(group) = &row.kind
            && !group.is_lone_search(transcript)
        {
            return self.blank.clone();
        }
        let revision = transcript.item_revision(key);
        // Usage figures can change without touching the transcript's Status item.
        let retained = revision.filter(|_| !matches!(item, Item::Status { .. }));
        let slot = usize::from(scan);
        if let Some(revision) = retained
            && let Some(projections) = self.entries.get_mut(key)
        {
            let matches = |known: &&Rc<Entry>| {
                known.revision == revision
                    && known.revealed == reading.revealed
                    && members(transcript, row).eq(known.rest.iter().map(|(key, rev)| (key, *rev)))
            };
            if let Some(known) = projections[slot].as_ref().filter(matches) {
                return known.text.clone();
            }
            if let Some(known) = projections[1 - slot].as_ref().filter(matches) {
                let text = known.text.clone();
                let before = retained_bytes(projections);
                projections[slot] = Some(known.clone());
                self.cached_bytes = self.cached_bytes - before + retained_bytes(projections);
                return text;
            }
        }
        #[cfg(test)]
        {
            self.prepared += 1;
        }
        let text = row_parts(transcript, row, reading)
            .map(RowText::from_parts)
            .map_or_else(|| self.blank.clone(), Rc::new);
        if let Some(revision) = retained {
            let projections = self.entries.entry(key.clone()).or_default();
            let before = retained_bytes(projections);
            projections[slot] = None;
            if projections[1 - slot]
                .as_ref()
                .is_some_and(|known| known.revision != revision)
            {
                projections[1 - slot] = None;
            }
            self.cached_bytes = self.cached_bytes - before + retained_bytes(projections);
            let size = text.allocated_bytes();
            if size <= ROW_TEXT_LIMIT && self.cached_bytes + size <= RETAINED_TEXT_LIMIT {
                projections[slot] = Some(Rc::new(Entry {
                    revision,
                    rest: members(transcript, row)
                        .map(|(key, revision)| (key.clone(), revision))
                        .collect(),
                    revealed: reading.revealed,
                    text: text.clone(),
                }));
                self.cached_bytes += size;
            } else if projections.iter().all(Option::is_none) {
                self.entries.remove(key);
            }
        }
        text
    }

    pub(in crate::pages::session) fn read(
        &mut self,
        thread: &ThreadModel,
        rows: &[Row],
        header: bool,
        reverse: bool,
        reading: &Reading<'_>,
        disclosed: Option<&Disclosed>,
    ) -> RowTexts {
        let transcript = thread.transcript();
        let context = Context::of(thread, reading);
        if self.context.as_ref() != Some(&context) {
            self.entries.clear();
            self.cached_bytes = 0;
            self.context = Some(context);
        }
        // The viewport need not keep a past copy of the core's complete key vector alive.
        let order = Rc::downgrade(&transcript.item_keys());
        if !self.order.ptr_eq(&order) {
            self.entries
                .retain(|key, _| transcript.item_revision(key).is_some());
            self.cached_bytes = self.entries.values().map(retained_bytes).sum();
            self.order = order;
        }
        let scan = disclosed.is_none();
        let mut texts = Vec::with_capacity(rows.len() + usize::from(header));
        let mut keys = Vec::with_capacity(texts.capacity());
        if header {
            texts.push(self.blank.clone());
            keys.push(None);
        }
        for row in rows {
            if let Some((key, item)) = transcript.get(row.range.start) {
                keys.push(Some(key.clone()));
                let reading = Reading {
                    revealed: match item {
                        Item::Tool(_) | Item::Patch(_) => {
                            disclosed.is_none_or(|disclosed| disclosed.is_open(key))
                        }
                        Item::ImageView { .. } => {
                            disclosed.is_some_and(|disclosed| disclosed.is_open(key))
                        }
                        _ => false,
                    },
                    ..*reading
                };
                texts.push(self.prepare(transcript, row, key, item, &reading, scan));
            } else {
                keys.push(None);
                texts.push(self.blank.clone());
            }
        }
        RowTexts::new(texts, keys, reverse, scan, transcript.layout_revision())
    }
}

#[cfg(test)]
mod tests {
    use codex_tui_core::app::AppEv;
    use codex_tui_core::fixtures::builders::{
        agent_delta, command_output, command_started, turn_started,
    };
    use codex_tui_core::fixtures::sandbox::Sandbox;

    use super::{Kind, Part, ROW_TEXT_LIMIT, Rc, Reading, Row, RowText, RowTextCache, row_parts};
    use crate::pages::session::cells::text::Env;
    use crate::widgets::thread_ui::Disclosed;

    const THREAD: &str = "thread-1";
    const SEED: &str = r#"{"step":"seed","seed":"session","thread":"thread-1"}"#;

    fn env() -> Env<'static> {
        Env {
            cells: &crate::pages::session::cells::strings::EN,
            chat: &crate::pages::session::strings::EN,
            detailed: false,
            expanded: false,
        }
    }

    #[test]
    fn gutter_mask_preserves_byte_offsets_for_search_and_original_text_for_copy() {
        let row = RowText::from_parts(vec![
            Part::gutter("•\n→"),
            Part::text("Résumé"),
            Part::gutter(""),
            Part::text("👋"),
        ]);
        assert_eq!(row.text, "•\n→\nRésumé\n\n👋");
        assert_eq!(row.without_gutters(), "\0\0\0\0\0\0\0\nRésumé\n\n👋");
        assert_eq!(row.text.len(), row.without_gutters().len());
        assert_eq!(row.gutters, [true, true, false, true, false]);
    }

    #[test]
    fn find_and_selection_prepare_history_once_and_only_rebuild_the_live_delta() {
        const HISTORY: &str = r##"{"step":"entry","kind":"agent","text":"# Saved\n\nAn **earlier** message with a [site](https://example.com).\n\n| A | B |\n| --- | --- |\n| first | second |"}"##;
        const LIVE: &str = r##"{"step":"entry","kind":"agent","turn":"live","item":"tail","text":"# Tail\n\nNeedle","end":false}"##;
        let mut source = String::from(SEED);
        source.push('\n');
        for _ in 0..80 {
            source.push_str(HISTORY);
            source.push('\n');
        }
        source.push_str(LIVE);
        let Ok(mut sandbox) = Sandbox::play(&source) else {
            panic!("valid transcript fixture");
        };
        let before = sandbox.app().active_thread();
        let rows: Vec<_> = (0..before.transcript().len())
            .map(|index| Row {
                range: index..index + 1,
                kind: Kind::Cell,
            })
            .collect();
        let env = env();
        let reading = Reading::new(&env, "•");
        let mut cache = RowTextCache::default();
        let disclosed = Disclosed::default();
        let old = cache.read(&before, &rows, true, true, &reading, Some(&disclosed));
        assert_eq!(rows.len(), 81);
        assert_eq!(cache.prepared, 81);
        for _ in 0..7 {
            let scanned = cache.read(&before, &rows, true, true, &reading, None);
            let selected = cache.read(&before, &rows, true, true, &reading, Some(&disclosed));
            assert!(selected == old);
            assert_eq!(scanned.scan_text(81).len(), selected.row_text(81).len());
            assert_eq!(cache.prepared, 81);
        }

        sandbox.send(AppEv::Backend(agent_delta(
            THREAD,
            "live",
            "tail",
            " follow-up",
        )));
        let after = sandbox.app().active_thread();
        let scanned = cache.read(&after, &rows, true, true, &reading, None);
        let selected = cache.read(&after, &rows, true, true, &reading, Some(&disclosed));
        assert_eq!(cache.prepared, 82);
        assert!(!old.row_text(81).contains("follow-up"));
        assert!(selected.row_text(81).contains("follow-up"));
        assert!(scanned.scan_text(81).contains("follow-up"));
        assert_eq!(old.row_text(1), selected.row_text(1));
    }

    #[test]
    fn alternating_search_and_mounted_views_reuses_closed_tool_projections() {
        let Ok(mut sandbox) = Sandbox::play(SEED) else {
            panic!("valid seed fixture");
        };
        sandbox.send(AppEv::Backend(turn_started(THREAD, "run")));
        let output = "first output\nsecond output\nthird output\nfourth output\n";
        for item in ["first", "second", "third", "fourth"] {
            let Ok(started) = command_started(THREAD, "run", item, "printf test") else {
                panic!("valid shell command");
            };
            let Ok(delta) = command_output(THREAD, "run", item, output) else {
                panic!("valid output notification");
            };
            sandbox.send(AppEv::Backend(started));
            sandbox.send(AppEv::Backend(delta));
        }
        let thread = sandbox.app().active_thread();
        let rows: Vec<_> = (0..thread.transcript().len())
            .map(|index| Row {
                range: index..index + 1,
                kind: Kind::Cell,
            })
            .collect();
        let env = env();
        let reading = Reading::new(&env, "•");
        let mut cache = RowTextCache::default();
        let disclosed = Disclosed::default();
        let scan = cache.read(&thread, &rows, false, false, &reading, None);
        let shown = cache.read(&thread, &rows, false, false, &reading, Some(&disclosed));
        assert_eq!(rows.len(), 4);
        assert_eq!(cache.prepared, 8);
        assert!(scan.scan_text(0).contains("first output"));
        assert!(!shown.row_text(0).contains("first output"));
        for _ in 0..5 {
            let scan = cache.read(&thread, &rows, false, false, &reading, None);
            let shown = cache.read(&thread, &rows, false, false, &reading, Some(&disclosed));
            assert!(scan.scan_text(0).contains("first output"));
            assert!(!shown.row_text(0).contains("first output"));
            assert_eq!(cache.prepared, 8);
        }
    }

    #[test]
    fn large_search_rows_keep_every_byte_without_pinning_them_in_the_cache() {
        let Ok(mut sandbox) = Sandbox::play(SEED) else {
            panic!("valid seed fixture");
        };
        sandbox.send(AppEv::Backend(turn_started(THREAD, "run")));
        let Ok(started) = command_started(THREAD, "run", "tool", "printf test") else {
            panic!("valid shell command");
        };
        sandbox.send(AppEv::Backend(started));
        let mut output = "older output\n".repeat(ROW_TEXT_LIMIT / 4);
        output.push_str("tail of output\n");
        let Ok(delta) = command_output(THREAD, "run", "tool", &output) else {
            panic!("valid output notification");
        };
        sandbox.send(AppEv::Backend(delta));
        let thread = sandbox.app().active_thread();
        let rows = [Row {
            range: 0..1,
            kind: Kind::Cell,
        }];
        let env = env();
        let reading = Reading::new(&env, "•");
        let mut cache = RowTextCache::default();
        let order = thread.transcript().item_keys();
        let strong = Rc::strong_count(&order);
        let scan = cache.read(&thread, &rows, false, false, &reading, None);
        assert!(scan.scan_text(0).contains("older output"));
        assert!(scan.scan_text(0).contains("tail of output"));
        assert!(scan.scan_text(0).len() > ROW_TEXT_LIMIT);
        assert!(cache.entries.is_empty());
        assert_eq!(cache.cached_bytes, 0);
        assert_eq!(Rc::strong_count(&order), strong);
    }

    #[test]
    fn grouped_rows_invalidate_for_any_member_and_disclosure_or_room() {
        let Ok(mut sandbox) = Sandbox::play(SEED) else {
            panic!("valid seed fixture");
        };
        sandbox.send(AppEv::Backend(turn_started(THREAD, "run")));
        for key in ["first", "second"] {
            let Ok(started) = command_started(THREAD, "run", key, "printf test") else {
                panic!("valid shell command");
            };
            sandbox.send(AppEv::Backend(started));
        }
        let before = sandbox.app().active_thread();
        let rows = [Row {
            range: 0..2,
            kind: Kind::Cell,
        }];
        let env = env();
        let mut reading = Reading::new(&env, "•");
        let mut cache = RowTextCache::default();
        let disclosed = Disclosed::default();
        cache.read(&before, &rows, false, false, &reading, Some(&disclosed));
        cache.read(&before, &rows, false, false, &reading, Some(&disclosed));
        assert_eq!(cache.prepared, 1);
        let Ok(delta) = command_output(THREAD, "run", "second", "new output") else {
            panic!("valid output notification");
        };
        sandbox.send(AppEv::Backend(delta));
        let after = sandbox.app().active_thread();
        cache.read(&after, &rows, false, false, &reading, Some(&disclosed));
        assert_eq!(cache.prepared, 2);
        let Some((first, _)) = after.transcript().get(0) else {
            panic!("first member");
        };
        disclosed.set(first, true);
        let shown = cache.read(&after, &rows, false, false, &reading, Some(&disclosed));
        reading.revealed = true;
        let cold = row_parts(after.transcript(), &rows[0], &reading).map(RowText::from_parts);
        assert_eq!(
            Some(shown.row_text(0)),
            cold.as_ref().map(|row| row.text.as_str())
        );
        assert_eq!(cache.prepared, 3);
        reading.room.width = 24;
        cache.read(&after, &rows, false, false, &reading, Some(&disclosed));
        assert_eq!(cache.prepared, 4);
    }

    #[test]
    fn reader_context_change_drops_old_thread_text_even_at_the_same_revision() {
        let Ok(first) = Sandbox::play(concat!(
            "{\"step\":\"seed\",\"seed\":\"session\",\"thread\":\"thread-1\"}\n",
            "{\"step\":\"entry\",\"kind\":\"agent\",\"text\":\"# First body\"}",
        )) else {
            panic!("valid first thread");
        };
        let Ok(second) = Sandbox::play(concat!(
            "{\"step\":\"seed\",\"seed\":\"session\",\"thread\":\"thread-2\"}\n",
            "{\"step\":\"entry\",\"kind\":\"agent\",\"thread\":\"thread-2\",\"text\":\"# Second body\"}",
        )) else {
            panic!("valid second thread");
        };
        let first = first.app().active_thread();
        let second = second.app().active_thread();
        let rows = [Row {
            range: 0..1,
            kind: Kind::Cell,
        }];
        let env = env();
        let reading = Reading::new(&env, "•");
        let mut cache = RowTextCache::default();
        assert!(
            cache
                .read(&first, &rows, false, false, &reading, None)
                .scan_text(0)
                .contains("First body")
        );
        assert!(
            cache
                .read(&second, &rows, false, false, &reading, None)
                .scan_text(0)
                .contains("Second body")
        );
        assert_eq!(cache.entries.len(), 1);
        cache.clear();
        assert!(cache.entries.is_empty());
    }
}
