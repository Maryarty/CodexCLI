# settings (S15): parity pack (partial landing 2, in progress)

Task `settings`, branch `claude/settings`. Oracle: `codex-rs/tui` `bottom_pane/{multi_select_picker,
status_line_setup, title_setup, skills_toggle_view, experimental_features_view, ...}`. Twin: modal sheets
in `codex-rs/tui-rtwk/src/sheets/settings`, core state in `core/src/screens/settings.rs`.

## Plan (dialogs by oracle test count)

| # | dialog | oracle tests | state |
|---|--------|--------------|-------|
| 1 | core + shared kit | - | done |
| 2 | status line setup, title setup | 4 shard + 12 state | done |
| 3 | theme picker (sheet, preview in the model, `/theme` through the app) | picker.rs 1 shard | done (host feed: needs) |
| 4 | skills toggle | small_dialogs_2: 4 of 5 shard green (`SetupSheet` kind Skills) | done; 1 pending (keymap hint) |
| 5 | experimental features (`FeaturesSheet`) | small_dialogs_1: 3 of 3 shard green | done (sheet + model); `/experimental` and host feed: needs |
| 6 | memories settings + reset confirmation (`MemoriesSheet`, `/memories` through the app) | picker/small_dialogs rows | done (landed 04ff6c716; enable prompt and host feed: needs) |
| 7 | hooks browser (`HooksSheet`, 17 entries `settings/hooks_*`) | small_dialogs_2: 19 of 23 hooks tests green, 4 pending after their first asserts; small_dialogs_1: 2 pending (wide_titles' reverse video, wrapped URL) | done (host `hooks/list` and persistence: needs) |
| 8 | config panel + child pickers (19 snaps, no shard rows), keymap setup/capture/debug, provider editor | | left |

## What the sheets use (rtwk, Flutter counterpart)

- Bottom sheet: `Dialog::bare(BoxWidget)` at the bottom edge; `Column(min)`/`Row`/`Expanded`/`SizedBox`.
  A bare dialog paints nothing of its own: the sheet gives its surface (`Tokens::prompt_surface`, default
  background when the terminal's is unknown) so a bottom-pane look clears what is under it
  (`FeaturesSheet`; the older sheets are transparent in blank cells, see "Deltas").
- Search field (setups, skills, theme): `TextField` + `TextEditingController`, `on_paste` through
  `normalize_paste`. Cursor: `SelectionController<String>` keyed by id. Scroll: `ListView.builder` +
  `ListController`. Keys: `FocusNode::set_on_key_event` (`Focus.autofocus` where there is no field).
- Experimental features: rows of unknown height (descriptions wrap under their column):
  `ListView.builder(..).shrink_wrap(true)` in a `Flexible`, rows are `Row(Start)` of lead, a `SizedBox`
  name column and an `Expanded(Text wrap Word)`; the column stretches (`CrossAxisAlignment::Stretch`) so the
  bottom-centred sheet is the screen's width.
- Mouse (ADR-011, beyond the oracle): a tap puts the cursor on a row, a second tap flips it; the wheel is
  the `ListView`'s.
- Cursor, tab and query are not core state: they are the view's controllers; core holds the open dialog
  (`Open`), the flags and order (`Setup`, `Experimental`), what a dialog asked for (`Edit`) and intents.
  Async answers of the experimental dialog carry the open's `generation`; a stale one is dropped (core test
  `answers_of_another_open_and_read_only_rows_change_nothing`, shown biting by removing the guard).

- Hooks browser (Flutter: a `Navigator`-less two-page sheet, the model's `page` picks the page): one
  `SelectionController<usize>` per page, one `ListController` + a `ValueNotifier<Scrolled>` for the window
  marks (`↑`/`↓` rows of the oracle) fed by a `NotificationListener<ScrollNotification>`; the list sits in a
  `Flexible` under the header and above the details so a short terminal shrinks the window first
  (`overflow_intended` on the page columns: below the header and the two marks the oracle clips too). The
  events table picks wide or narrow from the sheet's width in a `LayoutBuilder` (72 cells); the details are
  `Row(label in a ConstrainedBox min 10, Expanded(Text wrap Word))`; a title that does not fit is `dots_end`.
  The page's cursor is reset in `build` (`arrive`), not in `did_change_dependencies`: a `select`ed model
  value that changes after the first call does not re-call it (found by the handler pages' stray `↓`).
  Mouse: a tap puts the cursor on a row, a second tap opens the event / toggles the hook, the wheel is the
  `ListView`'s. Keys: the oracle's list keymap (`move_of`), space/enter toggle, `t` trust (all on the events
  page), Esc back then close, Ctrl+C closes from either page (`Intent::HookClose`).

## What is wired through the app

`/theme` opens the picker from `Facts` (default: the 32 bundled themes); the cursor previews in
`Model.syntax_theme`, Esc restores it, Enter records `Edit::ThemeSelected`. Real-path sheet tests drive it
(`slash_theme_*`, `the_cursor_previews_in_the_model_...`, `enter_picks_the_theme_...`). Shared file edited
(additive): `src/pages/session/actions.rs` (`Outcome::Open` emits `ScreenIntent::Settings(Intent::Open(..))`
for `/theme`; area widened by `task_update`).

`Screen` (`src/sheets/settings/mod.rs`) is a `StatefulWidget`: when the model's slot goes empty after it
showed a sheet (Esc, Enter, a save) it closes its own dialog (`close_dialog` in `did_change_dependencies`).
Before, Esc on an app-opened sheet left the empty-slot dialog up (found by `slash_theme_opens_the_settings`
in `tests/pages/transitions.rs`, which now passes unchanged).

## Deltas and open items

- Per-theme syntax colours: deferred to the end of the project as tm task `syntax-themes` (the person,
  2026-11-22). The picker only changes the model's `syntax_theme`; the sample uses the app's syntax theme.
  No rtwk gap row is filed.
- The name column in the setup sheets follows the rows in view (G39b, existing); `FeaturesSheet` measures
  all rows (same up to the eight rows the oracle shows; a long name past the eighth row would widen it).
- The experimental list is bounded by the screen, not by `MAX_POPUP_ROWS` (eight): the same up to eight
  features.
- Older sheets (setups, skills, theme) paint no surface of their own: in blank cells the screen below shows
  through at the margin columns; `FeaturesSheet` clears its area. Worth one shared fix (a `sheet surface`
  helper in `src/sheets/mod.rs`, outside this area).
- Experimental: the oracle writes `prevent_idle_sleep` and `guardian_approval` through their own
  `UpdateFeatureFlags` event; the model sends every writable change as one `Edit::FeaturesSave` (the host
  decides). The model holds no stage/retired filter: the host leaves out what the dialog hides.
- Shard edits (screen heights): the oracle renders into `desired_height(width)` rows, the port's sheet sits
  at the bottom edge over the debug menu, so `experimental_features_keep_following_choice_at_wrap_boundary`
  and `..._discovery_...` use the per-width heights `(68,13) (69,13) (70,12) (71,11) (72,11)` and `100x12`
  instead of 20 rows; the discovery keys end with `up up`: the oracle flips the third row directly and keeps
  its cursor on the first. Same expected snapshots.
- `footer_hint_uses_list_keymap_accept_and_cancel` stays pending: no keymap config in the port (kit has no
  `.keymap()`); entry `settings/skills_empty` exists.
- `picker_previews_the_pr_link_and_preserves_the_color_preference` stays pending: no entries
  `status_line_pr_*`, and `Setup.values` are text.
- Ctrl+C is the experimental sheet's cancel (the oracle's `on_ctrl_c`); other sheets map Esc only.
- The select dialog (`Menu::List`) could not draw the skills layout (private `rows`, no top blank, no
  reserved rows), so `SetupSheet` was generalised (kind Skills) instead of using it.

- Hooks: the selected row is the theme's accent and bold (the port's selection style in every dialog), not
  the oracle's `selection_style()` of an unprobed palette: `selected_event_rows_use_the_shared_selection_style`,
  the review-column count cell and the reverse-video part of `wide_titles` are `pending` with that reason
  (the shared selection style, not this dialog).
- Hooks, gaps: G74 (new): `Text(maxLines: 3, overflow: ellipsis)` cuts the third row through the rest of the
  text (`fourteen fift…`), the oracle wraps first and appends `…` to the third row: `renders_command_details_with_three_line_cap`
  `pending(G74)` after its first assert. G49/G72 (existing): the wrapped URL command
  (`wrapped_hook_command_preserves_the_complete_url_destination`) breaks at a hyphen and carries no link.
  G50/G51 (existing): `Column` cannot rank flex children; the list gives way first and the details keep
  their rows, which is the oracle's order for every size the oracle tests.
- Hooks, tests: `review_needed_handler_rows_use_warning_color` located its text by the command
  (`untrusted.sh`), which is on screen only in the selected row's details; the oracle's subject is the second
  row's line (`handler_row_lines(..)[1]`), so the shard asserts "Unnamed hook · new" yellow. The footer hints
  are fixed strings (no keymap config): `renders_event_browser`'s `hooks_events_no_accept` part is pending.
- `small_dialogs_2::warning_pages_wrap_and_scroll_long_diagnostics` is red in the shard (the warnings viewer,
  S23, `/warnings`; the script route opens no dialog): not this task.

## Needs outside the area (the orchestrator)

- Host (`host/src`, claimed by startup-rest): `facts::settings(&config)` reading `config.tui_theme`, listing
  `{codex_home}/themes/*.tmTheme` stems, `themes_dir` as `~/...`; emit `Ev::Facts` as
  `AppEv::Screen(ScreenEv::Settings(..))` (`in_process.rs`, `startup.rs` after reload). The experimental
  dialog needs `Open::Experimental` built from `codex_features::FEATURES` (+ `Ev::FeaturesDiscovered` from
  the app-server `experimentalFeature/list`, `Ev::FeaturesSaved` from the write) and `/experimental` to open it.
- Persistence: a root `app::Command` variant (pattern `SaveGrouping`, `ConfigBatchWrite`) for
  `Edit::ThemeSelected`, `SetupSaved`, `SkillToggled`, `Toggled`, `FeaturesSave`, with a confirmation `Ev`.
- Skills: the real list and the `skills/config` write.
- Keymap config for the hint footers (skills, experimental accept/cancel chords, hooks).
- Hooks browser: the host `hooks/list` RPC mapped to `Open::Hooks(Hooks::new(hooks, warnings, errors))`
  (`HookMetadata` -> `Hook`: the source path as the directory display writes it, the server's display order)
  sent as `Ev::Opened`, and `/hooks` (`SessionIntent::Hooks`, `src/pages/session/actions.rs`) to open the
  slot; the edits `Edit::HookEnabled` / `HookTrusted` / `HooksTrusted` to `SetHookEnabled`, `TrustHook`,
  `TrustHooks` (persistence `Command`, with the others above).
- Memories: the enable prompt of the oracle and the host `Facts::memories`.
- A shared sheet surface helper (the older sheets are transparent in blank cells; `HooksSheet` clears its
  rows with `BoxWidget::new().style(Style::new())` under the footer).

## Left (next agent)

- Config panel (tabs, search, value column, child pickers: effects, copy on select, notification condition,
  session picker layout; core `ConfigPanel` exists with `visible`, fixture `config_panel.jsonl`; 18 oracle
  snapshots in `codex-rs/tui/src/config_panel/snapshots`, the 15 tests of `config_panel_tests.rs` have no
  shard rows yet: the owner adds a shard file or checks the snapshots through the kit's `assert_snap`).
- Keymap picker/capture/debug (core `KeymapPicker` exists, no view), provider editor.
- chat_popups_4 experimental rows (`/experimental` through the app) pend on the host data above.

## Checks

- Oracle shards under `--profile oracle` (summary.py, after the rebase onto c797c066f): small_dialogs_1
  5 pass / 45 pending; small_dialogs_2 28 pass / 13 pending / 1 red (the warnings viewer, above); picker
  24 pass / 36 pending; chat_popups_4 2 pass / 52 pending; 0 todo, 0 stale in all four.
- Sheet tests `settings::` 37 (6 hooks: keys, Ctrl+C, trust, the window, tap, tap + wheel), core settings
  tests 37 (5 new hooks: rows and initial event, pages and Ctrl+C, toggle, trust, trust all).
  Shown biting: the second-tap branch of `Msg::Row` switched off, both tap tests fail.
- The flaky `settings::esc_goes_back_from_the_confirmation_...` and its three memories siblings: cause was
  the shared multi-thread runtime of `start()`, on which a key's intent to the shell and its answer take real
  time (a press followed by another before the model answered read a stale `confirming`); the `/memories`
  and `/theme` command tests now start on `start_driven` (the runtime runs only when the test lets it;
  `press` = key pace + `advance(100)` + `quiesce`). 12 runs of the 31 `settings::` tests at machine load
  40..75: 372 of 372 green; before the fix 2 of 2 red at load 40.
- clippy `--workspace --all-targets -D warnings` clean; fmt clean; `binary(rules)` 66/66 (R14: the hooks
  state holds no model type, R15: the label column is a `ConstrainedBox(min 10)`, no segmenter).

## Binary run

`acargo build -q -p codex-rtwk --bin codex-rtwk`, then `codex-rtwk --debug-screens` in a 112x30 pty (a
scratch driver outside the repo: `tools/rtwk-port/mock-model/check.py`'s `spawn_pty`, a small VT reader, keys
and SGR mouse; the mock-model has no `settings` scenario and `/hooks` has no host feed, so the debug menu's
entries are the way in). Walked on `settings/hooks_events` and `settings/hooks_many`:

- events page: `Hooks` / subtitle / table (`Event Installed Active Description`, 112 wide) / `↓` mark /
  `enter details · esc close`; Down, Up move the cursor; Enter opens `PreToolUse hooks` with the details of
  the row under the cursor and `space/enter toggle · esc back`; Space flips `[x]`/`[ ]`, Down + Enter flips
  the second hook; Esc returns to the events with the cursor on the event it came from; Ctrl+C closes
  from either page; a managed hook's page reads `Managed (always on) · esc back` and Space does nothing.
- mouse: a tap on `PermissionRequest` moves the cursor, a second tap opens its page; on `hooks_many` End
  scrolls the window to hook 11 (`↑` mark, no `↓`), the wheel up x3 scrolls it back (`↓` mark), Home returns.
- One checkpoint against the oracle (the events page, 112 wide, `hooks_browser_events`) is equal to the
  snapshot in the shard test (`renders_event_browser`, green); the differing rows of any hooks checkpoint:
  none but the selection colour (accent and bold, not the oracle's palette-less selection).
- A false lead worth keeping: the reader showed a stray `Event` after the events page was left: the
  presenter deletes characters (`CSI n P`) and the reader had no `P`; not the app.
