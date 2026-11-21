# settings (S15): parity pack (partial landing, in progress)

Task `settings`, branch `claude/settings`. Oracle: `codex-rs/tui` `bottom_pane/{multi_select_picker,
status_line_setup,title_setup,action_required_title,...}`. Twin: modal sheets in
`codex-rs/tui-rtwk/src/sheets/settings`, core state in `core/src/screens/settings.rs`.

## Plan (dialogs by oracle test count, built in this order)

| # | dialog | oracle tests | state |
|---|--------|--------------|-------|
| 1 | core + shared kit | - | done (`1a7ebd758`) |
| 2 | status line setup, title setup | 4 shard tests (3 status-line snaps, 1 title snap) + 12 state tests | done (`d58227adf`) |
| 3 | theme picker, config panel + child pickers | config/theme shard rows | left |
| 4 | skills toggle (5), experimental (3), memories (1) | small_dialogs_2 | left |
| 5 | hooks browser (25 shard tests, entries `dialogs/hooks_*`), keymap setup/capture/debug, provider editor | | left |

## What the setups use (rtwk, Flutter counterpart)

- `Dialog::bare(BoxWidget)` at the bottom edge (a bottom sheet), `Column`/`Row`/`Expanded`/`SizedBox`.
- Search field: `TextField` + `TextEditingController`. Cursor: `SelectionController<String>` (item id).
  Scroll: `ListView.builder` + `ListController`, `item_extent(1)`. Keys: `FocusNode::set_on_key_event`
  (Space/Left/Right/Enter/Esc/arrows/paging go to the sheet before the field). Mouse (ADR-011, beyond the
  oracle): a tap puts the cursor on the row, a second tap toggles.
- Cursor, tab and query are not core state: they are the view's controllers (the brief); core holds the
  open dialog (`Open`), the order and the flags (`Setup`), edits (`Edit`) and intents (`Intent`).
- Description cut with `…`: `widgets/dots.rs::dots_end` (rtwk `TextOverflow::Ellipsis` drops the blank
  next to the `…`, oracle keeps it; rtwk-gaps N1 closed as not a gap).
- Name column: measured with `segmenter_of(cx).width(..)` over the rows in view.

## Deltas and open items

- Name column over the rows in view (oracle `ColumnWidthMode::AutoVisible`): done with the `Scrolled`
  notifier (ScrollNotification) and the segmenter. Same hook as G39b (`parity/select-dialog.md`): rtwk has
  no layout-time window hook, so `Grid`/`Table` size the column by all rows. Flag for the orchestrator.
- `picker_previews_the_pr_link_and_preserves_the_color_preference` stays pending: no entries
  `status_line_pr_plain`/`_theme`, and `Setup.values` are text (the PR item needs a hyperlink span).
- No real data path yet: `Target::Settings` in `core/src/commands/dispatch.rs` maps /skills, /config,
  /theme, /title, /statusline, /experimental, /memories, /keymap, /tui to one target and
  `session/open.rs` calls `settings::open(cx)`, but nothing builds an `Open` from the config or the
  runtime values. The sheets open from fixtures (debug menu rows `settings/*`) only.
- Config persistence (`Edit::SetupSaved`, theme pick, toggles): the codex config crate write-through is
  not wired (needs an additive `Command` + protocol + host wiring).
- Area widening: the task area was widened with `task_update` to take `core/tests/settings.rs`.

## Checks (so far)

- `settings::` sheet tests (9), `core` settings tests (23), oracle `small_dialogs_2` default profile (all
  pass, rest pending), `--profile oracle`: the 4 un-pended tests pass, the others fail as pending.
- clippy `--all-targets -D warnings` clean (run with `RUSTUP_TOOLCHAIN=1.99.1-aarch64-apple-darwin` when
  started from the worktree root, else `cargo clippy` picks the 1.99.0 stable and fails with E0514).
- Mutation: Right reorders by +1; flipped to -1 the test `right_moves_the_item_down_the_order_and_left_back`
  fails.

## Binary run

Not done yet (left for the closing run): `check.py --only settings` with the entries
`status_line_preview`, `terminal_title`.
