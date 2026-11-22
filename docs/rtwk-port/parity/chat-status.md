# chunk-status — parity (in progress: step 1 of 8 landed on the branch)
Chunk of docs/rtwk-port/research/briefs/chat-status.md §6 (8 steps). Branch `worktree-agent-ae21c7b1bd5dacf8e`.
Shard counts (summary.py, oracle profile; pass / n-a / pending / todo):
before: chat_status_1 0/6/52 · _2 0/1/58 · _3 0/1/31 · pane_footer_1 0/0/55 · pane_footer_2 0/11/63 — total 0 / 19 / 259 of 278.
after step 1: chat_status_1 0/6/52 · _2 0/1/58 · _3 0/1/31 · pane_footer_1 0/0/55 · pane_footer_2 1/11/62 — total 1 / 19 / 258 of 278, 0 red, 0 stale.
Step 1 turns almost no shard test on by design: `footer_snapshots` (24 entries, one test) needs all 24 entries, and 17 of them need facts the model
does not have yet (see Needs) or steps 2 and 4 (status line in the footer row, the `?` overlay). 7 entries are done and compared one by one in
`tests/pages/chat_status.rs` (`the_footer_entries_equal_the_oracle_snapshots`, bitten once: `esc_idle_rest` shortened → the esc-idle entry red).

## Step 1 — footer row and modes (done)
- `src/pages/session/footer/` (mine): `Footer` StatelessWidget (reads the thread through `select`, never a copy: ADR-003), `hints.rs` (pure fit over
  measured widths: `desired_rows`, `fit`; collapse order Shortcuts → Plan(1) → Warning(1) → Ide(1) → Action(1) → Warning(2) → Agents, then refill),
  `strings.rs` (ADR-010 table), `entries.rs` (7 `chat/footer_*` debug entries over fixture scripts in `fixtures/`).
- The footer labels: shell mode, queue hint (running + draft, long/short, tappable), `← for agents` and `? for shortcuts` (empty draft only, pressable:
  a click presses the key, ADR-011), the context meter (left / `123K used` / unknown; only while the status line is off), the warning notice (3
  spellings, tappable → warnings viewer). Transient lines: `Line::Quit`, `EscIdle`, `EscPrimed`, `Close(hint)`.
- The Row of a footer with no labels keeps its one cell of height (the indent cell is 1 high): a draft takes the hints away and the row stays.
- `session_page.rs` builds the Footer; the follow control (`transcript::follow::footer`) still replaces it while the reader is away.
- Page and mouse tests (`tests/pages/chat_status.rs`, 5): the 7 entries; click on `? for shortcuts` opens the shortcuts; click on `tab to queue message`
  queues (`(queued) later`); click on the warning notice opens the warnings; a draft takes the hints away and the row stays.
- `ctrl_c_cancels_history_search_without_clearing_draft_or_showing_quit_hint` (pane_footer_2) un-pended: passes as is.

## Deltas
- The oracle has `DOUBLE_PRESS_QUIT_SHORTCUT_ENABLED = false`: Ctrl+C / Ctrl+D never show `ctrl c again to quit`, the double press does not exist. The brief's quit-reminder
  page tests are therefore not written; `Line::Quit` and `Line::EscIdle` exist for the snapshot entries only (`footer_ctrl_c_quit_idle`, `footer_esc_hint_idle`).
- `warning_notice_pluralizes_and_compacts` (tests/pages/status_data.rs, an unowned old test) asserted the compact `⚠ 3 · f2` at 48 columns from the older
  `warning_notice_layout` budget (half the width, min 14). The oracle's `passive_hints` puts the warning in the hint row as a label (`warning_notice_layout` is only
  the lone-context-meter case), so the full notice stays beside `← for agents` at 48 and the compact one shows at 30. The test now says so.
- Passive footer with a draft: no `? for shortcuts` / `← for agents` (oracle `ComposerHasDraft`); the old footer showed them.

## Shared edits (flagged)
- `src/pages/session/mod.rs` (`mod footer;` + the footer entries in `debug_entries`), `src/pages/session/session_page.rs` (the Footer replaces `composer::footer`;
  the `with_warning_notice` wrap is now only for the read-only thread) — chat-live's files.
- `tests/pages.rs` (`mod chat_status;`) and `tests/pages/transcript_follow.rs` (two assertions `edit the previous message` → `edit previous message`, the oracle's
  `esc again to edit previous message`) — transcript's files.
- `tests/oracle/pane_footer_2.rs` (un-pend of the Ctrl+C history-search test) — the keymap task's file; my area now holds pane_footer_1/2.rs (area widened on the record).
- `tests/pages/status_data.rs` — old unowned page tests of the status surfaces; one assertion changed (above).

## Needs (other owners)
- Model facts the footer entries need (core, chat-live's area): collaboration mode (Plan indicator + cycle hint), goal status, IDE context, side conversation
  label / active, vim mode, active agent label, git summary, thread headline, per-thread usage. Until they exist the matching `footer_*` entries and the pane_footer_1
  rows 174/651/683/771/830/852/1031 stay pending.
- `transcript::follow::footer` (transcript's): the oracle's transcript footer (`esc latest`) carries the warning notice at its right (width > 12);
  ours replaces the whole footer — `warnings_yield_to_queue_controls_and_complete_navigation_hints` stays pending for this.
- Kit: a colour-depth/palette seed (`light_warning_notice_clears_inherited_dimming`), a row-only cursor assert; the keymap seed for `show_warnings_key`
  (`warning_notice_respects_shortcuts_and_interactive_hints`: ours is always `f2`).
- `input_history_search_uses_the_final_row`: the history search row (composer's) reports no cursor at 48x6 (`cursor is None`), the oracle's is on the final row — chunk-composer.
- l10n/strings.rs (shared): `chat.backtrack_hint` and `chat.queue_hint` are dead now (the footer has its own table).
- `src/pages/session/composer/footer.rs` is not retired yet (nothing uses `footer`/`FooterLine`/`with_shell_mode`; the shell label moved to `Kind::Shell`): retire in step 2.

## Left (steps 2-8 of the brief)
2 status line in the footer row (replaces the hints when it has a value; row budget; footer links; pane_footer_1 27/131/205/269/316/416/489/542/1109/1130; status_line_style);
3 status items, warnings, title; 4 the `?` overlay (`sheets/dialogs/shortcuts.rs` panel exists; HELP 5 tests + the 3 overlay footer entries);
5 Working row, preview, background terminals; 6 effort transition/ignition; 7 sparkle; 8 voice footer/strip. Then the pty run (idle footer, `?`, a running turn with a queued
message, a 40-column terminal), the 6 ui_tree status-line tests last (person decision pending), and the final checks.
