# chunk-popups-picker-rest: parity pack

Landed at 2b550ffdf (branch tip 420993f55: code 7b44f9ef4, merge 8a2b0a12a, fmt 420993f55).
Written by the orchestrator from the task_done deliverable (the task's area had no docs/).

## Features
- The picker's side preview keeps its background tone (oracle `preserve_side_content_bg`).
- A short picker keeps the selected row when overflow spacers yield.
- The shared menu matches the oracle at 80x24 and 40x16; the short browser keeps its header tabs,
  search and footer; narrow description columns hide without stacking.
- The search line of any picker ignores the text field's Ctrl/Alt edit chords (Ctrl-U delta from
  chunk-popups-plugins); the oracle's query takes plain characters and Backspace only.

## Tests
- picker shard: 49 pass / 5 n/a / 9 pending / 0 red (was 44 / 5 / 14). Green now:
  `preserve_side_content_bg_keeps_rendered_background_colors`,
  `short_picker_preserves_selected_row_when_overflow_spacers_yield`,
  `shared_menu_presentation_at_wide_and_narrow_sizes`,
  `short_browser_keeps_header_tabs_search_and_footer`,
  `narrow_description_columns_hide_without_stacking`. G44, G39b, G54 and the raw-row tests stay
  pending on their rows.
- Full oracle profile: no new red; the 13 reds elsewhere (cells_1/4, chat_live_4, chat_popups_4,
  composer_1, markdown_streaming_4, resume_2, small_dialogs_2, transcript_view_2) are the same with the
  kit change disabled; they belong to other tasks. The same-size resize skip turned one
  markdown_streaming_4 test green.
- New: tests/sheets/select.rs `a_search_line_ignores_the_text_fields_edit_chords`.

## What was built
- `Tokens::tone(name)` (src/app/theme.rs): background-only Style of one of the sixteen ANSI names;
  src/widgets/picker/side.rs `with_preview` merges it into the preview style when core `Side.bg` is set.
  Fixture side_content_blue.jsonl in the select debug list.
- tests/oracle_snap header reads `height=N`, `80x24` and `48x6:` sizes; a lone `---` between two
  frames is dropped.
- kit `Screen::resize` skips `t.resize` when the size is unchanged (no `#` fill).
- kit `Snap::dialog()` / `compare::assert_dialog`: the oracle draws the view into the whole area,
  ours is a dialog as tall as its content; compared from the blank row above the first text, the
  content, one spacer row and the footer. This is the shared_menu frames' compare (the person may
  decide otherwise).
- description_hide_numbered.jsonl: the oracle's narrow_description test hand-writes the disabled
  row's `  2. ` prefix; ours numbers both rows through `numbered:false` + shortcuts.
  description_hide_when_narrow stays.
- `act_of` (src/widgets/picker/mod.rs) swallows FIELD_CHORDS (Ctrl a e k u w d y z, Alt b f) under a
  search line unless the keymap binds them or an opener's hot key takes them first.

## Gate
land-check at 8a2b0a12a: fmt red on tests/oracle_snap/mod.rs only, fixed in 420993f55; lint-build,
nextest rc=0, scenarios 39/41 (known reds error-context-window, multi-agent).

## Binary beside the oracle (pty 100x30, mock text-100)
- /model: rows, selection, scroll arrows and footer equal the oracle at open, after Down and after Esc.
- /theme (settings sheet, not this task): differs — (1) opens on the first theme instead of
  preselecting and scrolling to the current one with `↑` shown; (2) footer text
  `Press enter to confirm or esc to go back` vs `enter select · esc back`; (3) a blank row between the
  subtitle and the search line where the oracle reserves the `↑` row. Owner: src/sheets/settings/theme.rs
  (block-settings).
- /skills: `settings (S15): empty slot`, not wired (block-settings).

## Needs
- block-settings, /theme sheet: items (1)-(3) above.

## chunk-popups-sweep (landed 87ad7733c, 2026-11-23, batch 2)
- /model family on the live picker: 13 tests of chat_popups_4 green; `reasoning_up_shortcuts_reach_max` red -> green. Shard: 29 pass / 7 n/a / 25 pending / 0 red.
- Fixes: `src/sheets/select/dialog.rs` keys the cursor per page shape; a menu dialog closes when an intent leaves no menu (was an invisible dialog swallowing keys on /model with hidden models). Seed `{kind:"provider"}` -> `Ev::Provider`. `StatusFacts.agent_threads` from host (`multi_agent_v2.max_concurrent_threads_per_session`).
- Binary beside the oracle: /model rows, cursor and hint equal.
- Left: `ultra_reasoning_selection_warns_for_high_multi_agent_concurrency` pending (needs S16 pager; `drain()` in core/src/app/select.rs drops the warning, should record a hidden thread warning). Provider-write flow not ported: `/model <args>` still NotAvailable.
- Land-check of batch 2 on 87ad7733c: nextest 3092/3092 (59 s after slow-tests), scenarios 7/7 (base + model-verification, reasoning), total 215 s.
