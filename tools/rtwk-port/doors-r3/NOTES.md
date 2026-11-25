# doors-r3: notes the deliverable has no room for

Doors only; what a need turned out to be a feature of is left to its package (below). No script: every
item is a one-off hand edit (no pattern repeats across files).

## Where each door is, and its test
1. IDE context in the footer: `small_cmds::Fact::IdeOn` (seed `{"kind":"ide_on"}`, `core/src/features/small_cmds/seed.rs`),
   `footer::indicators::ide` (`Spec.ide`, `Kind::Ide`). Tests: `core/tests/doors.rs`
   `the_ide_on_seed_leaves_the_context_on_without_a_note`, `tests/pages/doors.rs` `footer_facts`.
   Deviation: the flag is the feature's state (`small_cmds.ide.enabled`), not a `StatusFacts` field (ADR-013: the
   footer only reads it, `/ide` and the host's probe own it); a config reload cannot wipe it.
2. Footer flash: `chat::facts::Ev::Flash(Option<FlashLine>)`, `Facts::flash()`, `footer::flash::Flash::of`,
   `session/slot.rs` hands it to `Timed`. Tests: `core/tests/doors.rs` `the_flash_fact_numbers_every_showing...`,
   `footer_facts::a_flash_stays_for_its_time_and_a_new_showing_restarts_it`. Edge: the clock is the view's (`Timed`): the flash stays in the facts after its time and the footer
   ends it, so a footer built anew (a thread switch, a rebuilt page) shows it again for its full time; a caller that
   wants it gone for good tells `None`. The same line told again is a new showing (`seq`), its time starts anew.
3. `oracle_snap::cut` / `cut_state`, `kit::snap_cuts`, `Snap::cut(index)`: explicit, because 29 snapshots hold
   a legitimate bare `---` row of their own. Test: `snapshot_cuts`.
4. Fake and kit: `FakeHandle::usage_accounts(Vec<Result<AccountReply, String>>)` (one answer per ask, the last
   stays) and `usage_report`; kit `Open::dir("usage")` (the live page, its debug-menu row) with
   `Open::usage_accounts` / `usage_report` (JSON of the reply). Tests: `usage_host`.
5. Kit: keys with release/repeat and a keymap were already there (`Screen::key_release`, `key_repeat`,
   `Open::keymap`); the pending reason of the two navigation tests is stale. New: `kit::palette`
   (`LIGHT`/`DARK`, `contrast`, `Grid::contrast_at`, `Grid::low_contrast`), `Snap::states(heights, index)` /
   `kit::snap_states` for bodies of different heights joined by a blank row. Tests: `kit_usage`.
   The `panel(Section, 54, 3)` bodies are still a model-only function in the oracle: no screen of the page draws a
   chart of 3 rows at 54 columns; that is a page feature (w2-status), the kit only compares.
6. `widgets::finder::{fold, search}` are `pub`; `transcript::find::repeat` calls `search` (w2-transcript may drop
   its own copy of the scan). Test: `finder_search`.
7. F4 on a cell whose `expanded` is local: `ThreadUi::disclosed()` (the `Disclosed` controller, held per thread),
   `cells::disclosure::{tile, expanded}`. A row puts its cell in a `tile(disclosed, key, cell)`; the cell reads
   `expanded(cx, local)` and its control toggles the controller. Not done (w2-transcript): the row wrapper in
   `transcript_pane`, and each of the four cells (exec, MCP, exploration, guardian) reading `expanded`. An un-rewired
   cell under a tile keeps its own flag, so the key would not open it. Tests: `disclosed_cells`.
8. 4-6 rows: see below. Tests: `small_windows` (one per size).
9. Composer command popup across Find: the popup is not painted while Find is active and its rows are kept; the
   focus leaving to Find's query does not dismiss it. Test: `command_popup_across_find`.

## Item 8: the finding
Root causes (by reading the oracle and the port; the probe run was refused by the permission layer, so the page
tests of `small_windows` and the rest of the batch are UNRUN by me):
- 4 rows: the bottom pane's fixed gap row (1 row) pushed the footer out of the window (a Column overflows at its end).
  The gap is now `Flexible(1)`, cut before the footer.
- 32 columns: the transient footer line (the `esc again to edit previous` hint is 34 cells) overflowed by the
  2-cell indent; it is now Expanded + Clip as the oracle's clipped `Paragraph`.
- 5 and 6 rows: already as the oracle (the list gets 0 and 1 rows).
The oracle's `transcript_view` tests drive the view alone (no bottom pane), so the pending 32x6 `sm_entry_*` tests
still cannot see the two rows in the chat: those want the transcript sheet (Ctrl+T) or the view alone, an owner decision.

## What each door opens (per package)
- w2-status: footer IDE indicator (`pane_footer_1` `MODE_REASON`), the footer flash callers (`FLASH_REASON`: copy results
  tell `Ev::Flash`), `rate_limit_reset_popup_states` (cut), usage tests (below).
- usage pending tests opened: `account_lookup_failure_recovers_on_refresh_with_the_server_plan` (4),
  `navigation_keeps_its_row_when_sections_change` (4, `usage_accounts` with two plans),
  `configured_question_mark_keeps_its_action_and_hides_help_hint` and
  `horizontal_aliases_ignore_modifiers_and_release_but_accept_repeat` (5, `keymap` + `key_release`/`key_repeat`),
  `analytics_sections_adapt_to_terminal_colors` and `chat_selection_keeps_a_highlighted_trailing_blank_after_wrapping`
  (5, palette; they still need an entry per section), `analytics_dates_distinguish_zero_missing_and_exact_details`
  (5, `Snap::states`; the page's model-only panel remains).
- w2-transcript: `find::repeat` dedup (6), F4 on activity cells (7), the 191 `transcript_view_*` tests partly (8).
- w2-composer / w2-status: the command popup across Find (9).
- rtwk gaps, not built: display-rows query; `ListController::ensure_row_visible` / `item_extent` / `visible_range`
  (G-new-usage-2); `Text` word ellipsis with an unlinked `…`.
