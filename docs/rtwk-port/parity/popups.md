# Parity pack: chunk-popups (the Picker brick, /model family, /permissions, generic pickers, feedback, goal, plugins, apps)

Brief: `docs/rtwk-port/research/briefs/popups.md` (§6 plan). Branch `worktree-agent-a032b2526c8243bfe`, base `claude/tui-rtwk` c797c066f.
This pack is a work in progress: steps 1a, 1b, 1c (Panel browser layout) and the first of step 1's rest (`HideWhenNarrow`) are done, the rest is in "Left" (a successor continues from it).

## Oracle numbers (summary.py --run, before this chunk)

| shard | pass | n/a | pending | total |
|---|---|---|---|---|
| chat_popups_1 | 0 | 5 | 21 | 26 |
| chat_popups_2 | 1 | 6 | 39 | 46 |
| chat_popups_3 | 0 | 2 | 41 | 43 |
| chat_popups_4 | 2 | 7 | 52 | 61 |
| picker | 24 | 3 | 36 | 63 |

After step 1c (probe run, `KIT_PROBE_PENDING=1 summary.py --run picker --list stale|red`): 6 picker tests are stale and pass:
`horizontal_list_keys_reorder_orderable_items`, `searchable_plain_j_updates_query_instead_of_navigating`,
`browser_page_navigation_uses_rows_that_fit`, `overflow_arrows_follow_the_painted_picker_surface`,
`shrinking_the_browser_keeps_selection_visible_without_navigation`, `tiny_viewports_preserve_query_and_selection_when_enlarged`.
`picker.rs` is settings' file: the `kit::pending` line of each is removed when the orchestrator hands the file over (a ready
patch: delete those six `kit::pending(..)` calls; picker would go 24 -> 30 pass). The shards chat_popups_1..3 are unchanged
(every pending there waits on a seed or routing I do not own, see Needs).

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

- 1c Panel browser layout: core `screens/select/layout.rs` (`Sizes`, `Heights`, `Surface::{Terminal, Panel, Multi}`, `heights()` = the
  `picker_areas` arithmetic, `split()` = blank row on top / footer yields / multi-select footer never yields, arrows carved out of the
  multi list area when it has >= 3 rows; 5 unit tests); view `widgets/picker/pane.rs` (`Pane` over a `LayoutBuilder`: measures the
  rows each piece takes at the width with `widgets/measure.rs`, asks `layout`, builds each piece at its height); `List.panel` /
  `List::surface()`; `Rows::measure/wanted/list`; the picker holds a `viewport` row count and keeps the cursor row visible when the
  window shrinks (a `ScrollController` listener, `ensure_index_visible`). 4 entries: `browser_keymap`, `browser_single_line`,
  `browser_wrapped`, `shared_menu`. Tests (tests/sheets/select.rs): 5 `short_browser` frames, page down by the rows that fit, 7 multi
  heights (`height=1..7`), shrink keeps the selected row (single-line and wrapped), plus the layout unit tests. All five oracle frames
  of `short_browser_*`, `browser_page_navigation` and `multi_select ... height=1..7` are equal to the snapshots.
- 1d `HideWhenNarrow`: `List.hide_description_below: Option<u16>` (core), `Rows.hide_description_below` (view): the description column
  hides when the room beside the label column would be narrower; entry `description_hide_when_narrow`; sheets test
  `the_description_column_hides_when_it_would_be_narrow` (width 48 hides, 49 and 80 show). The preview line of a multi-select is one
  unwrapped line (`TextWrap::None`) as the oracle draws it.

## Left (brief §6, in order)

1. Step 1 rest, in `core/src/screens/select/list.rs` + `src/widgets/picker/`:
   - typed span roles (semantic spans, accent selected row: `selected_rows_use_the_shared_accent_style`, `selection_preserves_semantic_spans_*`),
     fuzzy highlight with grapheme alignment, hard line breaks in name/description, link spans (`choice_labels_and_descriptions_keep_complete_url_destinations`),
     `HideWhenNarrow` description (widths 48/49/80, `exact_fit_description_*`), wrapped two-column rows with halfwidth marks (G44),
     side-content background (`Side.bg` exists in core, the view ignores it: `preserve_side_content_bg_*`).
   - entries still missing (names are the shard's): `side_content_blue`, `accept_alternative`, `fuzzy_emoji`, `url_*`, `span_*`, halfwidth, hard breaks
     (see the picker.rs pending reasons). `exact_fit` cannot be an entry: the oracle draws a raw row list without a cursor (`/model  fits exactly`),
     the picker always has a cursor row (`› ` gutter).
   - the oracle's `description_column_visibility` draws raw rows (the second with the number `2.` by hand and the bare reason `Unavailable`);
     the list builds `disabled: Unavailable` and no number for a disabled row (as `selection_row_layout.rs` does), so that picker.rs test stays
     pending even with the entry; the hide rule itself is proven in tests/sheets/select.rs.
2. Steps 2-7 untouched: menu stack + /model family, /permissions, review/worktree/quick dials/plan + feedback (takes `sheets/dialogs/feedback.rs` out of
   R15_NOT_YET), goal, plugins + apps, handover sweep (this pack, `parity/select-dialog.md`, un-pend `chat_popups_4.rs` after settings lands).
3. Windows tests: 3 oracle tests `cfg(windows)` are `kit::na` (chat_popups_*), the 2 side-content clearing tests in picker.rs are n/a (rtwk paints the dialog region).

## Needs from other owners (nothing done in their files)

- tests/kit `Screen::resize(w, h)` at the size the screen already has: the Tester fills the new screen with `#` junk and rtwk repaints only on a
  size change, so `short_browser_keeps_header_tabs_search_and_footer` (first frame `(48, 6)` at 48x6) and `narrow_description_columns_*` (first
  width = the opening size) read a `#` screen. Fix in the kit: skip the resize when the size is equal (a real terminal sends none).
- tests/oracle/picker.rs `picker_appearance_renders_checkboxes_preview_and_overflow`: three frames in one snapshot, `assert_snap` needs
  `assert_snap_section(.., "First page:")` etc. (the kit's `label:` header kind covers `First page:`).
- tests/oracle/picker.rs `non_orderable_items_cannot_move_or_be_crossed` (60x8) and `section_break_after_item_renders_separator_row` (60x6): the
  oracle's multi-select at that height hides the rows it asserts (title, blank, search, arrow, ... the rows of the list area, footer never yields);
  raise the sizes (60x10 / 60x9 show the rows; the same assertions pass at 24 rows in tests/sheets/select.rs).
- `filled_tab_unknown_palette_*` (G54): the tab strip under an unknown palette keeps the theme colours; stays pending with its gap row.
- tests/oracle_snap/mod.rs `header()` has no `label:` kind (`before scroll:`, `width 20:`, `height=N`, `80x24`): 5 picker tests wait (unclaimed test infra).
- tests/kit/seed.rs `.keymap(..)` (keymap's claim): the 9 keymap picker tests.
- `core/src/screens/mod.rs` `project_thread` multi-projection + `core/src/threads.rs` (chat-live): per-thread select state for step 2+ (carry model/list through a ThreadFx select Command).
- `core/src/app.rs` AppCtx routing and `host/src/facts.rs` (keymap): app RPCs `model/list`, `permissionProfile/list`, plugin/app RPCs for steps 2-6.
- `chat.rs ThreadPart` fields (chat-live): policy / collaboration_mode read by /permissions and /plan (one field, one writer).
- `tests/oracle/picker.rs` and `chat_popups_4.rs` (settings): turn the shard tests on once handed over.

## Run it (real binary, `codex-rtwk --debug-screens`, a pty driver of my own: /tmp/popups-drive/drive.py, not kept)

No mock-model scenario reaches the select dialog yet (the app routes `/model` to no catalog: step 2), so the walk is the debug entries.
`browser_keymap` at 80x24: title, subtitle, five tabs, search, `↓`, rows, hint as the oracle's first frame; PageDown moves the cursor by
the rows that fit (Action 16 after 2 Down + PageDown, `↑` and `↓` rows); typing `j` goes to the search (`no matches`, the `↓` stays);
`description_hide_when_narrow` resized to 48 columns shows the names only, at 49 the details come back (the binary equals the sheets test).
Open delta found on `shared_menu` at 80x24 (the oracle's `shared_menu_presentation_at_wide_and_narrow_sizes` frame `80x24`, which the kit
cannot address yet: `label:` header): the oracle cuts the long label of a described row with an ellipsis (`A long option label that still
wraps at narrow wi…`) where the picker wraps it inside the label column (`widths` on a second row), and it draws a disabled row's reason bare
(`This option needs a connected server.`) where `Words.reason_alone` says `disabled: ...` (the oracle's `selection_row_layout.rs:63` does say
`disabled: `; its `render_rows` path does not). Both are the next thing to check in step 1 (`wrapped_two_column_rows_*` family); the fixture
now carries the preset's `hide_description_below: 24`. The mouse in the binary was not re-walked (the click went to the debug list's
background row); the mouse is covered by the tests/sheets/select.rs tap/wheel tests.

## Open decisions / small deltas

- None new. `Rebuild::Yes` in the adapter is deliberate (comment in `dialog.rs`).
- Disk was full for a while (140 MiB free); I removed my own stale test binaries in this worktree's target only.
