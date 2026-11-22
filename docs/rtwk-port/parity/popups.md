# Parity pack: chunk-popups (the Picker brick, /model family, /permissions, generic pickers, feedback, goal, plugins, apps)

Brief: `docs/rtwk-port/research/briefs/popups.md` (§6 plan). Branch `worktree-agent-a032b2526c8243bfe`, based on `claude/tui-rtwk` 252acefb6.
This pack is a work in progress: step 1 (the Picker brick) is done as far as my area reaches (1a-1f below); steps 2-7 are untouched
and every one of them waits on a file I do not own (see Needs). A successor continues from "Left".

## Oracle numbers (summary.py --run, no probe, at the tip)

| shard | pass | n/a | pending | total |
|---|---|---|---|---|
| chat_popups_1 | 0 | 5 | 21 | 26 |
| chat_popups_2 | 1 | 6 | 39 | 46 |
| chat_popups_3 | 0 | 2 | 41 | 43 |
| picker | 43 | 5 | 15 | 63 |

`tests/oracle/picker.rs` joined the area on the re-grant (07e2b0234): the 10 stale pendings are dropped, the keymap tests' sizes and entries are the
picker's (`› First`/`› Second`/`› Next` for a searchable list, `first_second_searchable_second`, 60x12 list page/jump, 60x18 multi page/jump, 60x12
non-orderable, 60x9 section break, `hint_default` 6 / `hint_picker` 7 rows), `picker_appearance_renders_checkboxes_preview_and_overflow` reads its three
frames through `assert_snap_section` (First page / Last page / No matches) and stays pending with its reason (frame 2: G39b, the Auto-visible column is
measured over the window rows in the oracle; frame 1/3 and every frame: the oracle's Buffer dump says `bg: Reset`, our dialog paints the host
background; the sheet test `the_multi_select_appearance_matches_the_oracle_in_three_frames` holds the three frames' rows), the two side-content
clearing tests are `kit::na`. Two Picker facts found on the way: the `SECTION_BREAK_ROW` rule sizes the label column (oracle `ColumnWidthMode`), and no
arrows are drawn over an empty search result. 15 pending are the raw-row tests, G54, G44, G39b, the side background (`Tokens::tone`), the kit header
kinds, the kit's resize to the same size, the `shared_menu_*` whole-area frame and the palette seed (see Needs).

## Done

- 1a `73530b9ae` Picker brick: `src/widgets/picker/` (`Picker { list: Rc<List>, on_event: Handler<Event> }`, `Event::{Choose, Secondary,
  Toggle, Reorder, Confirm, Cancel}`; controllers (cursor, scroll, tabs, query, focus) stay in its State). `sheets/select/dialog.rs` is the seeded
  adapter (`MenuDialog`): reads the menu through a provider projection, maps `Event` to `ScreenIntent::Select(Intent::..)`, closes when the model
  drops the menu. Its `update` returns `Rebuild::Yes` (needed: the picker gets the rows the model holds after the flush).
- 1b `d1f0edbcd` multi-select family on the core `List`: `Item.orderable`, `Item.section_break_after`, `List.ordering`, `List.preview`,
  `List::reorder/can_reorder/preview_names`, `Browse::entry_of`, `Row.section_break`; `Intent::Reorder(id, down)`; the rule is a list entry of
  its own (counts as one of the 8 rows, like the oracle's disabled break row); left/right and ctrl+h/ctrl+l reorder when the query is empty; the preview
  line sits under the rows. 7 entries (`multi_*`, `section_break`).
- 1c Panel browser layout: core `screens/select/layout.rs` (`Sizes`, `Heights`, `Surface::{Terminal, Panel, Multi}`, `heights()` = the
  `picker_areas` arithmetic, `split()`; 5 unit tests); view `widgets/picker/pane.rs` (`Pane` over a `LayoutBuilder`), `List.panel`, the picker keeps
  the cursor row visible when the window shrinks. 4 entries: `browser_keymap`, `browser_single_line`, `browser_wrapped`, `shared_menu`.
- 1d `HideWhenNarrow`: `List.hide_description_below`, entry `description_hide_when_narrow`.
- 1e `4a812ce3d` the Picker reads the list keymap (Flutter: `Shortcuts` + `Actions` over the dialog, the keymap from the provider):
  `act_of` maps a press to the `LIST_*` actions of `keymap(cx)` (`move_up/down/left/right`, `page_up/down`, `jump_top/bottom`, `accept`, `cancel`);
  a printable key bound to accept/cancel wins over the search line, navigation keys are gated by the search line as in the oracle
  (`list_selection_view.rs handle_key_event`), C0 ctrl+h (arrives as Backspace+CTRL) is normalised to the keymap's `ctrl+h`; the footer is
  `List.standard_hint` (the keymap's accept/cancel labels: `f3 select · f2 back`) unless the row or list gives one. 8 entries
  (`accept_alternative`, `toggle_then_next`, `animations_checkbox`, `first_second_searchable{,_second}`, `twelve_items`, `hint_default`, `hint_picker`);
  7 sheet tests (`tests/sheets/select.rs`: printable accept, tab as move_down over the toggle key, cancel on space, c0 ctrl+p/ctrl+n, page and jump
  maps, the footer under f3/f2). A searchable list draws no numbers (`› Next`, as the oracle: the number keys feed the query).
- 1f `0b2c1f4ca` typed span roles and the row layout of the oracle's dialog path:
  - core `Role::{Plain, Key, Success, Warning, Dim, Strong}` + `Span { text, role }` on `Item.prefix_spans` / `Row.prefix_spans` (the oracle's
    `name_prefix_spans`, raw `"L".underlined()` there; roles here, the view turns them into theme styles: key = underline, success/warning =
    the theme token + italic); the runs sit between the number and the name and hang with the gutter.
  - the cursor row is the accent in bold, its description the accent **not** bold (oracle `apply_row_state_style` takes bold off secondary spans).
  - a row with a description keeps its name on one line cut with an ellipsis at the label column (`selection_row_layout.rs build_full_line`:
    `name_limit` chars, then `…` in the first cell of the gap); a disabled row's `(disabled)` mark stays after the cut name; a row without a
    description wraps its name over the whole room.
  - under `HideWhenNarrow` a disabled row's reason is bare (`Unavailable`, `combined_description`), else `disabled: Unavailable`.
  - entries `semantic_spans`, `selected_row`, `hard_breaks`; 4 sheet tests (roles under the cursor, accent, line breaks, the shared menu: **equal to both
    oracle frames** `80x24`, `40x16` from the blank row above the title to the footer; the oracle draws into the whole area, the dialog is as tall as its
    content, see Open deltas).
  - Open delta found in 1d is closed: long label cut with an ellipsis, bare reason.

## Left (brief §6, in order)

1. Step 1 rest, in `core/src/screens/select/list.rs` + `src/widgets/picker/`:
   - side-content background (`preserve_side_content_bg_keeps_rendered_background_colors`, entry `side_content_blue`): `Side.bg` is data the view ignores.
     A colour needs `src/app/theme.rs` (rules `no_literal_colours_anywhere`: the theme file alone holds colour values) which chunk-cells holds; a
     token `Tokens::tone(Tone) -> Color` (or a `Side` that carries a `Role`) is the one function to add there, then `side::style_of` in
     `widgets/picker/side.rs` is five lines (tried, reverted: the rules test refused `Color::indexed`).
   - link spans (`choice_labels_and_descriptions_keep_complete_url_destinations`): the oracle auto-links URLs found in raw row text; rtwk has `Style::link`
     per `Span` but no URL scanner for plain text, and scanning strings in the view is the workaround R15 forbids. Needs a decision: a `Role::Link(url)`
     on spans by the producer (the apps list knows its URLs), the test stays pending (raw rows, 8 combinations of the oracle's `render_rows`).
   - fuzzy highlight (`fuzzy_matched_emoji_graphemes_keep_description_alignment`): `list_selection_view.rs` never sets `match_indices`; the rows that do
     are the composer popups' (command popup, mentions, skills: chunk-composer). Not a Picker feature; the test renders `render_rows` directly.
   - halfwidth marks G44 stays pending (`snapshot_narrow_width_counts_halfwidth_sound_marks`, `wrapped_two_column_rows_count_halfwidth_sound_marks`).
   - the raw-row tests `exact_fit_description_*`, `wrapped_two_column_rows_*`, `description_column_visibility` frames cannot be green through the Picker
     (the oracle draws `render_rows` into a bare buffer: no gutter mark, no cursor, `wrap_indent: Some(..)` rows the dialog never builds: the dialog sets
     `wrap_indent` only for rows without a description); their behaviours are proven in `tests/sheets/select.rs`.
2. Steps 2-7 untouched: menu stack + /model family, /permissions, review/worktree/quick dials/plan + feedback (takes `sheets/dialogs/feedback.rs` out of
   R15_NOT_YET), goal, plugins + apps, handover sweep (this pack, `parity/select-dialog.md`, un-pend `chat_popups_4.rs` after settings lands).
   The plan I made for step 2 (not started): core `select::Model { stack: Vec<Menu>, catalog, picked }` with typed `Menu` variants (`Models`, `Reasoning
   { model }`, `ServiceTier`, ... data only, ADR-010), `Intent::Open(Menu)` kept compatible (landed `approvals/auto_review.rs` uses
   `Intent::Open(Menu::List(..))`), catalog fed by a `select` seed `screen` event (`{"kind":"catalog","models":[..]}`) and, live, by the app's
   `model/list` reply; state tests in `core/tests/models.rs` (pick -> reasoning, single option skip, Esc pops, refresh keeps the highlight,
   `show_in_picker=false` hidden); the view presenter shows the stack head through `lists::of`; `core/src/commands/dispatch.rs` already answers
   `/model` with `Target::Select` and `command`, `src/pages/session/open.rs` (`Target::Select => drop(select::open(cx))`) drops the command: it has to
   pass it so the dialog can ask for the right menu.
3. Windows tests: 3 oracle tests `cfg(windows)` are `kit::na` (chat_popups_*), the 2 side-content clearing tests in picker.rs are n/a (rtwk paints the dialog region).

## Needs from other owners (nothing done in their files)

- tests/kit `Screen::resize(w, h)` at the size the screen already has: the Tester fills the new screen with `#` junk and rtwk repaints only on a size
  change, so `short_browser_keeps_header_tabs_search_and_footer` and `narrow_description_columns_*` read a `#` screen. Skip the resize when the size is equal.
- tests/oracle_snap/mod.rs `header()` has no `label:` kind (`before scroll:`, `width 20:`, `height=N`, `80x24`, `prefix label:`): 8 picker tests wait.
- the shard frames at `80x24` (`shared_menu_*`) are the view drawn into the whole area (list area fills, footer on the last row); the dialog is as tall as its
  content (10 rows + the footer), the debug page shows around it. Equal inside the dialog (sheet test above); equal on the whole screen only if the dialog
  fills a tight height. The real app's bottom pane is `desired_height` tall, so the dialog's look is the oracle's; a decision for the person whether the
  kit compares the dialog region.
- kit palette seed (`Screen::palette(fg, bg)`): `overflow_arrows_follow_the_painted_picker_surface`, `semantic_spans` under the two palettes.
- `src/app/theme.rs` (chunk-cells): `Tokens::tone(..)` for the side-content background (see Left 1).
- `filled_tab_unknown_palette_*` (G54): the tab strip under an unknown palette keeps the theme colours; stays pending with its gap row.
- `core/src/screens/mod.rs` `project_thread` multi-projection + `core/src/threads.rs` (chat-live): per-thread select state for step 2+ (carry model/list
  through a ThreadFx select Command).
- `core/src/app.rs` AppCtx routing and `host/src/facts.rs`: app RPCs `model/list`, `permissionProfile/list`, `collaborationMode/list`, plugin/app RPCs for steps 2-6.
- `chat.rs ThreadPart` fields (chat-live): policy / collaboration_mode read by /permissions and /plan (one field, one writer).
- `src/pages/session/open.rs` (chunk-transcript-2): pass the command to `select::open`.
- `chat_popups_4.rs` (settings): turn the shard tests on once handed over.

## Run it (real binary, `codex-rtwk --debug-screens`, a pty driver of my own: /tmp/popups-drive/drive.py, not kept)

No mock-model scenario reaches the select dialog yet (the app routes `/model` to no catalog: step 2), so the walk is the debug entries.
`shared_menu` (row 60 of the debug menu): at 80x24 the dialog rows are the oracle's frame (title, subtitle, `Recommended option` with its two-line
description, `A long option label that still wraps at narrow wi…  Additional details`, `Unavailable (disabled)` with the bare reason, the footer);
resized to 40x16 the description column hides and the long label wraps (`A long option label that still / wraps at narrow widths`), as the oracle's `40x16`
frame. Differing rows of that checkpoint: none inside the dialog; the footer sits under the dialog (row 18) where the oracle's frame (whole area) has it on
the last row (see Needs). Earlier (1c/1d): `browser_keymap` at 80x24 equals the oracle's first frame, PageDown moves by the rows that fit, `j` goes to the
search, `description_hide_when_narrow` hides at 48 columns and shows at 49. The mouse in the binary was not re-walked (a click went to the debug list's
background row); the mouse is covered by the tap/wheel tests in tests/sheets/select.rs.

## Checks at the tip (07e2b0234)

fmt clean; `clippy --workspace --all-targets -D warnings` no error (warnings only in the backend pin); whole nextest 1935 passed, 17 skipped;
`shards.py --check`: 55 shards, 2560 tests, 0 stale; select sheet tests 44 pass; oracle shards as in the table above.
