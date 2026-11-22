# Parity pack: chunk-popups (the Picker brick, /model family, /permissions, generic pickers, feedback, goal, plugins, apps)

Brief: `docs/rtwk-port/research/briefs/popups.md` (§6 plan). Branch `worktree-agent-a032b2526c8243bfe`, based on `claude/tui-rtwk` 252acefb6.
This pack is a work in progress: step 1 (the Picker brick) and step 2 (core `select::Model {stack, catalog}`, the /model family as typed menus, the
presenter over the stack head) are done as far as my area reaches (1a-1f, 2 below); steps 3-7 are untouched and the live routing of 2-7 waits on files I do
not own (see Needs). A successor continues from "Left".

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

- 2 `cece9a556` the /model family. Core: `core/src/screens/select/models.rs` (data and rules, no words: `Preset`, `Catalog {presets, pending, issued}`,
  `Current` snapshot, `ModelMenu::{Loading, Models, AllModels, Reasoning, Advanced, PlanScope}`, `Page {heading, rows, initial}`, `Choice {id, kind, current,
  effort, session}`, `Selection`, `list_request`/`presets` for the `model/list` wire) and `select::Model { stack, catalog, picked, notes }` (head `menu()`,
  `page()`; Esc pops one, a final pick clears the stack, a refresh replaces the catalog in place and dismisses a menu left with no choice, an answer of
  another request is dropped). 37 state tests in `core/tests/models.rs` (each shown biting once): loading, request guard, quick/all lists (auto order,
  hidden models, no-auto catalog = all list), reasoning (default/current marks, initial row, xhigh/high warning only for gpt-5.1-codex*/gpt-5.2*, More
  reasoning…), advanced (Max, Ultra last), single-effort skip, session key, Ultra warning at >= 8 threads, plan scope (Plan-only/all modes, no-op skip),
  refresh/dismissal, wire decode, seeded stack. View: `sheets/select/lists.rs` `of(menu, page, s)` builds the typed menus (rows `id = choice.id`, `current`,
  session `Secondary { key: 's', hint: word, accept: default|apply }`, `selected_description` for the Plus-plan warning, `initial`, `standard_hint`, Info
  subtitle for Advanced); the words are in `sheets/select/strings.rs` (ADR-010, `Msg<N>` for the argument ones); `dialog.rs` reads the head
  through a provider projection, keys the Picker by stack depth, Esc emits `Intent::Close` only (the dialog closes when the model's stack is empty).
  Picker: footer precedence item -> secondary action (`enter default · s session · esc back`) -> list; the cursor follows its row by id when a refresh
  moves it; `List.subtitle_role` (`Role::Info` = cyan). 5 debug entries (`models_quick`, `model_reasoning`, `model_advanced`, `model_plan_scope`,
  `model_loading`) are fixture scripts through core `apply`; `model_picker` is now a catalog + all-models script. 9 sheet tests: the quick, reasoning and
  advanced menus' rows equal the oracle snapshots (`model_picker_refreshes_auto_models`, `model_reasoning_selection_popup`,
  `model_advanced_reasoning_selection_popup`), loading, plan scope words, Enter/Esc/Esc through the stack, the high-effort warning while highlighted, the
  secondary footer, a tap on a model with efforts and on an effort.

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
2. Step 2 deferred inside the area (not started): the Luna reserve picker (`restrict_model_picker_to_luna_reserve`, the rate-limit slug), the service-tier
   controls and `model_picker_refreshes_service_tier_controls`, the reasoning shortcuts, the Bedrock seeds, the cloud-picker `Accepted for future turns` header
   line, the custom base-URL warning line, and the parent's cursor on Esc (the oracle's parent view keeps its highlight; here the Picker under a pushed
   menu is rebuilt, so the cursor starts on its `initial` row again: the id of the row chosen is data the core could keep per stack entry).
   Steps 3-7 untouched: /permissions, review/worktree/quick dials/plan + feedback (takes `sheets/dialogs/feedback.rs` out of
   R15_NOT_YET), goal, plugins + apps, handover sweep (this pack, `parity/select-dialog.md`, un-pend `chat_popups_4.rs` after settings lands).
   (/permissions has its typed `Menu::Permissions` since step 2; its words stay in `strings.rs`.)
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
- `src/pages/session/open.rs` (chunk-transcript-2): pass the command to `select::open`; `core/src/commands/dispatch.rs` answers `/model` with `Target::Select`
  and the dispatcher must send `select::Intent::OpenModels(Current)` (the `Current` snapshot read from the thread's part: model, effective effort, the
  Plan mode's model/effort/source, `multi_agent_v2.max_concurrent_threads_per_session`).
- `model/list` request and reply (chunk-live / host/src/facts.rs / core/src/app.rs): the dispatcher takes `Model::catalog.pending` (request id) after
  `OpenModels`, sends `select::models::list_request(id)` and feeds the reply as `select::Ev::Fetched { request, result: models::presets(value) }`; the startup
  catalog is `select::Ev::Loaded(presets)`.
- the effect of `select::Model::picked` (`Picked::Selection { model, effort, apply, warning }`: `thread/settings/update`, `config/batchWrite`, the
  Plan-mode override) and `Model::notes` (`NoAdditionalModels`, `ModelsNotLoaded` as history cells): settings/chat owners.
- `core/tests/all.rs`: one line `mod models;` (done in my commit because the state tests do not run without it; flagged: a file outside my area).
- the 4 chat_popups shards' `/model` tests (`model_selection_popup_snapshot`, `model_reasoning_selection_popup_snapshot`, ...) go through `ChatWidget`
  (`open_model_popup`), so they flip only when the routing above lands; the pass counts of the three shards are unchanged by step 2.
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

## Checks at the tip (cece9a556)

fmt clean; `clippy --workspace --all-targets -D warnings` no error; whole nextest 1980 run, 1979 passed, 1 failed (`startup::session::a_remembered_mode_opens_without_a_prompt`:
passes alone, a load flake outside my area); `shards.py --check`: 55 shards, 2560 tests, 0 stale; oracle shards: picker 63 (43 pass + 5 n/a = 48 green, 15 pending),
chat_popups_1 26 (5 n/a), chat_popups_2 46 (7 green), unchanged by step 2 (see Needs); sheet tests `select::` 53 pass; state tests `models::` 37 pass.
