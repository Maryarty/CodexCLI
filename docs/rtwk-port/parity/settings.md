# settings (S15): parity pack (partial landing, in progress)

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
| 6 | memories settings + reset confirmation (1 shard) | | left (a plain select list: see "Left") |
| 7 | hooks browser (23 + 2 shard, entries `dialogs/hooks_*`), config panel + child pickers (19 snaps, no shard rows), keymap setup/capture/debug, provider editor | | left |

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

## Needs outside the area (the orchestrator)

- Host (`host/src`, claimed by startup-rest): `facts::settings(&config)` reading `config.tui_theme`, listing
  `{codex_home}/themes/*.tmTheme` stems, `themes_dir` as `~/...`; emit `Ev::Facts` as
  `AppEv::Screen(ScreenEv::Settings(..))` (`in_process.rs`, `startup.rs` after reload). The experimental
  dialog needs `Open::Experimental` built from `codex_features::FEATURES` (+ `Ev::FeaturesDiscovered` from
  the app-server `experimentalFeature/list`, `Ev::FeaturesSaved` from the write) and `/experimental` to open it.
- Persistence: a root `app::Command` variant (pattern `SaveGrouping`, `ConfigBatchWrite`) for
  `Edit::ThemeSelected`, `SetupSaved`, `SkillToggled`, `Toggled`, `FeaturesSave`, with a confirmation `Ev`.
- Skills: the real list and the `skills/config` write.
- Keymap config for the hint footers (skills, experimental accept/cancel chords).

## Left (next agent)

- Memories reset confirmation: a plain select list (title, subtitle, two items with descriptions, hint
  `enter select · esc back`); try the select sheet (`MenuDialog` with a `dir: select` fixture) first, with
  per-width heights as above.
- Hooks browser (`hooks_browser_view.rs`, 1700 lines; shard entries `dialogs/hooks_*` renamed to
  `settings/hooks_*`), config panel (tabs, search, value column; core `ConfigPanel` exists, no view),
  keymap picker (core `KeymapPicker` exists, no view), provider editor, memories settings toggles.
- chat_popups_4 experimental rows (`/experimental` through the app) pend on the host data above.

## Checks

- `settings::` sheet tests 25/25 (three runs), core settings tests 30/30 (incl. the 4 experimental), oracle
  `small_dialogs_1::experimental_features*` 3/3 and `small_dialogs_2` skills 4 under `--profile oracle`.
- clippy `--workspace --all-targets -D warnings` clean (RUSTUP_TOOLCHAIN=1.99.1-aarch64-apple-darwin);
  `binary(rules)` green (no literal colours: the experimental sheet's surface is `prompt_surface` as is;
  ADR-010: the diff gutter cell is `Strings.diff_gutter`).
- Mutations: Right reorders by +1, flipped: `right_moves_the_item_down_the_order_and_left_back` fails;
  the stale-generation guard removed: `answers_of_another_open...` fails; Esc saving after an unconfirmed
  write: `a_failed_write_keeps_the_choices_...` fails.

## Binary run

Not done. The mock-model driver has no `settings` scenario and its turns are model turns (a slash command
that leaves a sheet open has no sentinel); `/theme` opens in the real binary through the app. To do:
`acargo build -q -p codex-rtwk --bin codex-rtwk`, `check.py --bin rtwk --bin oracle --only settings`
(new `scenarios/settings.py`, area widening) and record one checkpoint's differing rows.
