# Parity pack: chunk-popups (the Picker brick, /model family, /permissions, generic pickers, feedback, goal, plugins, apps)

Brief: `docs/rtwk-port/research/briefs/popups.md` (§6 plan). Branch `worktree-agent-a032b2526c8243bfe`, base `claude/tui-rtwk` c797c066f.
This pack is a work in progress: steps 1a and 1b are done, the rest is in "Left" (a successor continues from it).

## Oracle numbers (summary.py --run, before this chunk)

| shard | pass | n/a | pending | total |
|---|---|---|---|---|
| chat_popups_1 | 0 | 5 | 21 | 26 |
| chat_popups_2 | 1 | 6 | 39 | 46 |
| chat_popups_3 | 0 | 2 | 41 | 43 |
| chat_popups_4 | 2 | 7 | 52 | 61 |
| picker | 24 | 3 | 36 | 63 |

After step 1b (probe run, `KIT_PROBE_PENDING=1 summary.py --run picker --list stale`): one picker test is stale and
passes: `horizontal_list_keys_reorder_orderable_items`. `picker.rs` is settings' file: its `kit::pending` line is removed
when the orchestrator hands the file over (a ready patch: delete that one `kit::pending(..)` call).

## Done

- 1a `73530b9ae` Picker brick: `src/widgets/picker/` (`Picker { list: Rc<List>, on_event: Handler<Event> }`, `Event::{Choose, Secondary,
  Toggle, Reorder, Confirm, Cancel}`; controllers (cursor, scroll, tabs, query, focus) stay in its State). `sheets/select/dialog.rs` is the seeded
  adapter (`MenuDialog`): reads the menu through a provider projection, maps `Event` to `ScreenIntent::Select(Intent::..)`, closes when the model
  drops the menu. Its `update` returns `Rebuild::Yes` (needed: the picker gets the rows the model holds after the flush).
- 1b `d1f0edbcd` multi-select family on the core `List`: `Item.orderable`, `Item.section_break_after`, `List.ordering`, `List.preview`,
  `List::reorder/can_reorder/preview_names`, `Browse::entry_of`, `Row.section_break`; `Intent::Reorder(id, down)`; the rule is a list entry of
  its own (counts as one of the 8 rows, like the oracle's disabled break row); ←/→ and ctrl+h/ctrl+l reorder when the query is empty; the preview
  line sits under the rows. 7 entries: `multi_orderable`, `multi_orderable_two`, `section_break`, `multi_alpha_jupiter`, `multi_appearance`,
  `multi_fields_twelve`, `multi_twelve` (the last three wait for kit rows, see below). Tests: core `tests/select.rs` (3 reorder tests),
  `tests/sheets/select.rs` (rule row, reorder, fixed row, plain letter feeds search, preview line, tap flips a multi row, tap picks, wheel scrolls).
  The core tests were written with the code (compile error was the only "red"); not shown biting separately.

## Left (brief §6, in order)

1. Step 1 rest, in `core/src/screens/select/list.rs` + `src/widgets/picker/`:
   - Panel "browser" layout (oracle `selection_picker_layout.rs picker_areas`, 54 lines): optional gaps/indicators yield on short screens,
     header + search + >=1 row stay. Today the Picker is a `Column(Min)` of fixed chrome around a `Flexible` rows list; at 60x8 the multi-select
     (title, blank, search, arrow, 4 entries, arrow, hint = 10 rows) overflows, so `non_orderable_items_cannot_move_or_be_crossed`,
     `section_break_after_item_renders_separator_row` and `searchable_plain_j_updates_query_instead_of_navigating` (picker.rs, 60x8/60x6 screens) stay red
     under probe (the same checks pass at 24 rows in `tests/sheets/select.rs`). Plan: a `LayoutBuilder` over the pane picks which spacer
     widgets exist from the measured height, the pure sizes as a core function (`picker_areas` arithmetic) with its own tests. Then
     `tiny_viewports_*`, `short_browser_*`, `browser_page_navigation_*`, `shrinking_the_browser_*`, `overflow_arrows_*` need entries (Keymap browser lists).
   - typed span roles (semantic spans, accent selected row: `selected_rows_use_the_shared_accent_style`, `selection_preserves_semantic_spans_*`),
     fuzzy highlight with grapheme alignment, hard line breaks in name/description, link spans (`choice_labels_and_descriptions_keep_complete_url_destinations`),
     `HideWhenNarrow` description (widths 48/49/80, `exact_fit_description_*`), wrapped two-column rows with halfwidth marks (G44),
     side-content background (`Side.bg` exists in core, the view ignores it: `preserve_side_content_bg_*`).
   - entries still missing (names are the shard's): `shared_menu`, `side_content_blue`, `accept_alternative`, browser lists, `fuzzy_*`, `url_*`, `span_*`, ... (see picker.rs pending reasons).
2. Steps 2-7 untouched: menu stack + /model family, /permissions, review/worktree/quick dials/plan + feedback (takes `sheets/dialogs/feedback.rs` out of
   R15_NOT_YET), goal, plugins + apps, handover sweep (this pack, `parity/select-dialog.md`, un-pend `chat_popups_4.rs` after settings lands).
3. Windows tests: 3 oracle tests `cfg(windows)` are `kit::na` (chat_popups_*), the 2 side-content clearing tests in picker.rs are n/a (rtwk paints the dialog region).

## Needs from other owners (nothing done in their files)

- tests/oracle_snap/mod.rs `header()` has no `label:` kind (`before scroll:`, `width 20:`, `height=N`, `80x24`): 5 picker tests wait (unclaimed test infra).
- tests/kit/seed.rs `.keymap(..)` (keymap's claim): the 9 keymap picker tests.
- `core/src/screens/mod.rs` `project_thread` multi-projection + `core/src/threads.rs` (chat-live): per-thread select state for step 2+ (carry model/list through a ThreadFx select Command).
- `core/src/app.rs` AppCtx routing and `host/src/facts.rs` (keymap): app RPCs `model/list`, `permissionProfile/list`, plugin/app RPCs for steps 2-6.
- `chat.rs ThreadPart` fields (chat-live): policy / collaboration_mode read by /permissions and /plan (one field, one writer).
- `tests/oracle/picker.rs` and `chat_popups_4.rs` (settings): turn the shard tests on once handed over.

## Open decisions / small deltas

- None new. `Rebuild::Yes` in the adapter is deliberate (comment in `dialog.rs`).
- Disk was full for a while (140 MiB free); I removed my own stale test binaries in this worktree's target only.
