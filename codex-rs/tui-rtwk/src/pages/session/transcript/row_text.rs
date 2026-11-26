//! Shared row text for the search model and the visible selection model.

use std::borrow::Cow;
use std::collections::HashMap;
use std::ops::Range;
use std::rc::Rc;

use codex_tui_core::app::facts::Rendering;
use codex_tui_core::ids::ThreadId;
use codex_tui_core::threads::{Item, ItemKey, ThreadModel, Transcript};

use super::find::RowTexts;
use super::item_text::{Reading, row_parts};
use super::turn_details::{Kind, Row};
use crate::pages::session::cells::text::Part;
use crate::widgets::measure::Measure;
use crate::widgets::thread_ui::Disclosed;

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

fn members<'a>(transcript: &'a Transcript, row: &Row) -> impl Iterator<Item = (&'a ItemKey, u64)> {
    row.range.clone().skip(1).filter_map(|index| {
        let (key, _) = transcript.get(index)?;
        Some((key, transcript.item_revision(key)?))
    })
}

/// At most one prepared representation per item. The scan and the mounted selection share it
/// when their members and disclosure agree. A grouped tool may need both projections in a frame.
pub(in crate::pages::session) struct RowTextCache {
    context: Option<Context>,
    order: Option<Rc<Vec<ItemKey>>>,
    entries: HashMap<ItemKey, Entry>,
    blank: Rc<RowText>,
    #[cfg(test)]
    prepared: usize,
}

impl Default for RowTextCache {
    fn default() -> Self {
        Self {
            context: None,
            order: None,
            entries: HashMap::new(),
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
            self.order = None;
        }
    }

    fn prepare(
        &mut self,
        transcript: &Transcript,
        row: &Row,
        key: &ItemKey,
        item: &Item,
        reading: &Reading<'_>,
    ) -> Rc<RowText> {
        if let Kind::Tools(group) = &row.kind
            && !group.is_lone_search(transcript)
        {
            return self.blank.clone();
        }
        let revision = transcript.item_revision(key);
        // Usage figures can change without touching the transcript's Status item.
        let retained = revision.filter(|_| !matches!(item, Item::Status { .. }));
        if let Some(revision) = retained
            && let Some(known) = self.entries.get(key)
            && known.revision == revision
            && known.revealed == reading.revealed
            && members(transcript, row).eq(known.rest.iter().map(|(key, rev)| (key, *rev)))
        {
            return known.text.clone();
        }
        #[cfg(test)]
        {
            self.prepared += 1;
        }
        let text = row_parts(transcript, row, reading)
            .map(RowText::from_parts)
            .map_or_else(|| self.blank.clone(), Rc::new);
        if let Some(revision) = retained {
            self.entries.insert(
                key.clone(),
                Entry {
                    revision,
                    rest: members(transcript, row)
                        .map(|(key, revision)| (key.clone(), revision))
                        .collect(),
                    revealed: reading.revealed,
                    text: text.clone(),
                },
            );
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
            self.context = Some(context);
        }
        let order = transcript.item_keys();
        if self
            .order
            .as_ref()
            .is_none_or(|old| !Rc::ptr_eq(old, &order))
        {
            self.entries
                .retain(|key, _| transcript.item_revision(key).is_some());
            self.order = Some(order);
        }
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
                    revealed: matches!(item, Item::Tool(_) | Item::Patch(_))
                        && disclosed.is_none_or(|disclosed| disclosed.is_open(key)),
                    ..*reading
                };
                texts.push(self.prepare(transcript, row, key, item, &reading));
            } else {
                keys.push(None);
                texts.push(self.blank.clone());
            }
        }
        RowTexts::new(
            texts,
            keys,
            reverse,
            disclosed.is_none(),
            transcript.layout_revision(),
        )
    }
}

#[cfg(test)]
mod tests {
    use codex_tui_core::app::{AppEv, AppFx, apply_app};
    use codex_tui_core::fixtures::builders::{
        agent_delta, command_output, command_started, turn_started,
    };
    use codex_tui_core::fixtures::script::build;

    use super::{Kind, Part, Reading, Row, RowText, RowTextCache, row_parts};
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
        let Ok(played) = build(&source) else {
            panic!("valid transcript fixture");
        };
        let mut app = played.app;
        let before = app.active_thread();
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

        apply_app(
            &mut app,
            AppEv::Backend(agent_delta(THREAD, "live", "tail", " follow-up")),
            &mut AppFx::default(),
        );
        let after = app.active_thread();
        let scanned = cache.read(&after, &rows, true, true, &reading, None);
        let selected = cache.read(&after, &rows, true, true, &reading, Some(&disclosed));
        assert_eq!(cache.prepared, 82);
        assert!(!old.row_text(81).contains("follow-up"));
        assert!(selected.row_text(81).contains("follow-up"));
        assert!(scanned.scan_text(81).contains("follow-up"));
        assert_eq!(old.row_text(1), selected.row_text(1));
    }

    #[test]
    fn grouped_rows_invalidate_for_any_member_and_disclosure_or_room() {
        let Ok(played) = build(SEED) else {
            panic!("valid seed fixture");
        };
        let mut app = played.app;
        let mut fx = AppFx::default();
        apply_app(
            &mut app,
            AppEv::Backend(turn_started(THREAD, "run")),
            &mut fx,
        );
        for key in ["first", "second"] {
            let Ok(started) = command_started(THREAD, "run", key, "printf test") else {
                panic!("valid shell command");
            };
            apply_app(&mut app, AppEv::Backend(started), &mut fx);
        }
        let before = app.active_thread();
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
        apply_app(&mut app, AppEv::Backend(delta), &mut fx);
        let after = app.active_thread();
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
        let Ok(first) = build(concat!(
            "{\"step\":\"seed\",\"seed\":\"session\",\"thread\":\"thread-1\"}\n",
            "{\"step\":\"entry\",\"kind\":\"agent\",\"text\":\"# First body\"}",
        )) else {
            panic!("valid first thread");
        };
        let Ok(second) = build(concat!(
            "{\"step\":\"seed\",\"seed\":\"session\",\"thread\":\"thread-2\"}\n",
            "{\"step\":\"entry\",\"kind\":\"agent\",\"thread\":\"thread-2\",\"text\":\"# Second body\"}",
        )) else {
            panic!("valid second thread");
        };
        let first = first.app.active_thread();
        let second = second.app.active_thread();
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
