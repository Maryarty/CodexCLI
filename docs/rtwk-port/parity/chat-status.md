# chunk-status — parity (in progress: steps 1, 2, 3 and 4 of 8 landed on the branch)
Chunk of docs/rtwk-port/research/briefs/chat-status.md §6 (8 steps). Branch `worktree-agent-ae21c7b1bd5dacf8e`.
Shard counts (summary.py, oracle profile; pass / n-a / pending / todo):
before: chat_status_1 0/6/52 · _2 0/1/58 · _3 0/1/31 · pane_footer_1 0/0/55 · pane_footer_2 0/11/63 — total 0 / 19 / 259 of 278.
after step 1: chat_status_1 0/6/52 · _2 0/1/58 · _3 0/1/31 · pane_footer_1 0/0/55 · pane_footer_2 1/11/62 — total 1 / 19 / 258 of 278, 0 red, 0 stale.
after step 2: chat_status_1 0/6/52 · _2 0/1/58 · _3 0/1/31 · pane_footer_1 1/0/54 · pane_footer_2 1/11/62 — total 2 / 19 / 257 of 278, 0 red, 0 stale (whole nextest 1917 passed).
after step 3: chat_status_1 0/6/52 · _2 2/1/56 · _3 2/1/29 · pane_footer_1 1/0/54 · pane_footer_2 1/11/62 — total 6 / 19 / 253 of 278, 0 red, 0 stale.
after step 4: chat_status_1 0/6/52 · _2 2/1/56 · _3 2/1/29 · pane_footer_1 6/0/49 · pane_footer_2 1/11/62 — total 11 / 19 / 248 of 278, 0 red, 0 stale (whole nextest 1031 passed, 16 skipped; clippy --all-targets clean).
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

## Step 2 — status rows (done as far as the model allows)
- `footer/status_rows.rs` (mine): `StatusRows { spans, composer, hints }`. Flutter: `Text(maxLines: n)` with `n` from a `LayoutBuilder` over the frame the page gives its
  bottom pane (rtwk gives an inflexible child of a bounded `Column` the whole height: no `MediaQuery` needed). `rows_for(frame, input, hints)` is `FooterLayout`
  (`composer_height_limit` = two thirds of the frame, at least 8; cap = a quarter of that, 2 to 4 rows; the draft keeps its rows, the hints keep theirs); the draft's rows
  come from the composer's own `TextEditingController` (`ListenableBuilder`, measured by `measure::rows`), the hints' rows from the footer's own fit (`Footer::spec`). Nothing
  is cut here: `Text` wraps and puts the ellipsis on the last row. 3 unit tests, 2 page tests (the line yields rows to a growing draft, bitten once: draft not measured → red;
  the line stays under the composer while the popup is open), oracle `status_rows_respect_host_budget_and_yield_to_the_draft` turned on (row by row against the snapshot).
- `widgets/status_line.rs`: `status_spans` (the spans) beside `status_line` (the title setup's preview, 3 rows).
- The footer row collapses when no label fits (`footer_hint_height` is 0 then; step 1 kept an empty row): a draft with the status line on and nothing to hint leaves no blank row.
- pane_footer_1 pending reasons rewritten one by one (they all said "not built"): extension status (ui_tree; person decision), provider status, link (PR fact), agent label,
  Plan/side (collaboration fact), goal, queue/flash (step 5), popup rows (popups chunk's), effort (step 6).

## Step 3 — status items and the title (done as far as the model allows)
- `status_surfaces.rs` (mine): the terminal title is `StatusContext::title(facts, TitleState { spinner, action_required })` -> `Title` (pieces: spinner slot, text); `Title::text(frame)` joins as the oracle
  (`separator_from_previous`: a space beside the spinner, ` | ` between the others; the old code joined ` · `), each item cut to its graphemes (`truncate_terminal_title_part`: project 24, current-dir 32,
  thread 48, the rest 32, dots at the end), ids that are not `TerminalTitleItem`s dropped, the action-required line = prefix + the other items except the spinner and run state, ` | `.
  `window_title.rs` takes a `Title` and gives its spinner slot the animation's frame (the same `AnimationController`). `session_page.rs`: the title block is one `context.title(..)` (wiring, listed below).
- `daybreak` reads `Daybreak off` until the thread says on (oracle `daybreak_enabled` is false by default); it was omitted.
- Turned on: `thread_identity_renders_names_and_fallbacks_on_both_surfaces`, `terminal_title_shows_action_required_while_exec_approval_is_pending` (chat_status_3), `status_line_hostname_renders_current_machine_hostname`,
  `status_line_and_terminal_title_reasoning_render_only_effort` (chat_status_2). Page tests (3) + unit tests (3): spinner before the project while a turn runs, action required replaces the spinner item only,
  daybreak off by default; title join, cut, item ids. Bitten once: separator ` · ` -> thread identity red.
- Delta of the identity test: the live harness names its thread `thread-1` (the script seed's `thread` is ignored: kit need), so the 36-character id of the oracle is not on the screen; its cut
  (`550e8400-e29b-41d4-a716-44665...`) is in the unit test `a_title_part_is_cut_to_its_graphemes_with_dots`.
- Not built, with the need (all pending reasons rewritten one by one):
  - every footer snapshot of the shard (renamed thread, fast mode, plan mode, speed, context remaining, goals, estimated cost): the oracle's unit-test chat has agents navigation off, ours reads
    `← for agents · ? for shortcuts` (rows 0-4 of `renamed_thread_footer_title` equal): kit option or StatusFacts flag for the command center.
  - context-used / -remaining / legacy: `token_usage_pending` (a fresh thread reads 100% left, a resumed one hides it): a resumed/pending fact.
  - thread credits / estimated cost (+ title): the status-card porter's per-thread `Usage` command and `status::ThreadPart`.
  - workspace headline, git summary (`pull-request-number`, `branch-changes`), raw output, goal (indicator, clock, footer snapshots), collaboration mode, side conversation, invalid-item warning once,
    model catalog (display name `GPT-5.5`, tier names `fast`/`Flex`): StatusFacts / ThreadPart fields of chunk-live; the host probes are the keymap porter's.
  - the title's action-required blink (`[ . ]` every second) needs `effects.title` in StatusFacts (chunk-live's).
- Needs / lander (step 3): `session_page.rs` title block (`TitleState` + `context.title`, `use` line) and nothing else outside my files.

## Step 4 — the `?` overlay (done; 5 shard tests on, 3 stay pending with their reason)
- `sheets/dialogs/shortcuts.rs` (dialogs' file, widened for this step): `panel(s, tokens, sections)` is a `Flexible(OUTWEIGHS_THE_GAP, Row[indent, Expanded(LayoutBuilder)])`: the title, the groups
  in three columns, two stacked or one (by the measured width: `group_lines`), a blank, `/keymap customize`. Given less height than its rows it is Flutter's
  `Column(min)[Flexible(SingleChildScrollView(body, NeverScrollable)), note, customize]` with the note `… resize to see all` over `/keymap customize` (oracle `render`); no height at all is a 0x0 box.
  Narrower than the widest row the groups sit in a horizontal `SingleChildScrollView` (the oracle wraps by word: delta below).
- The rows come from the user's keymap and the turn (`resolve`, oracle `bindings`): `dialogs/strings.rs` holds the table (`Section`/`Entry`/`Keys::{Plain, Chord, Pair, Bound(ActionId)}`, ADR-010);
  a `Bound` row is left out while its action is unbound, spelt by `keymap::stroke_label` (glyphs by platform); `Send message`/`Quit` read `Queue message`/`Interrupt` while a turn runs.
  The old static `shortcut_groups` stays for the dialog's list page (debug entry); it is retired with that entry.
- `footer/mod.rs`: `Line::Close` is a unit variant now; `close_row` (the toggle's keymap label, ` / `, `esc`, ` close`; `esc close` when the toggle is unbound or the whole does not fit) is a
  `LayoutBuilder` over the width and every span taps (`Footer.on_close`): a click on `? / esc close` closes the overlay (ADR-011).
- Priority against the reader's gap: the gap (`transcript::follow::gap`, chunk-live's) is `Flexible(1)` beside the panel in the bottom pane's `Column`; a Flutter `Column` shares its free height by
  flex, so the panel's flex is 8 (`OUTWEIGHS_THE_GAP`): full help at 24 rows, the gap keeps its row from a free height of nine on.
- Tests: oracle `shortcut_help_stays_above_the_prompt_and_bottom_status` (wide 130x24 and narrow 40x40 against the snapshots, the page's gap row and the placeholder aside), `..._keeps_bottom_anchored_composer_status_and_cursor_stable`,
  `..._keeps_input_and_close_hint_visible_when_height_is_tiny` (40 columns, heights 4 to 7), `shortcut_help_close_row_uses_runtime_binding_and_narrow_fallback`, `activity_shortcut_help_follows_runtime_binding`
  (three keymaps in one snapshot: `f4`, `ctrl-x t`, none; split on its `---`). Page tests (`tests/pages/chat_status.rs`, 3): click on `esc close` closes (bitten once: `on_close: None` -> red); rows follow the keymap and
  the running turn; Esc closes the help before it dismisses the banner. Fixture `tests/oracle/fixtures/pane_footer_1/help_status.jsonl` (the oracle's `composer()`: status line `MODEL · ~/project · Context 20% used`).
- Still pending, with reasons in the files: the two sparkle tests under the help (step 7); `escape_closes_shortcut_help_before_inline_banner` waits for the entry `chat/inline_banner_short` (chunk-live registers
  entries; the same keys are page-tested on the backend banner).
- Deltas: (1) the narrow groups (a column wider than the room) are clipped at the right edge, the oracle wraps by word: no test of the shards reaches it (the 22-column case of the close-row test only asserts the
  last rows). (2) The terminal's enhanced-keys fact is not modeled: the newline row reads `⌃j` (the keymap's first binding; the oracle's status tests, `enhanced_keys_supported = false`) where the oracle's
  `footer_mode_shortcut_overlay` (enhanced on, Esc hint primed) reads `⇧enter` and `esc`; `esc` is always `esc esc` (no esc-backtrack hint fact). `tests/sheets/dialogs.rs`
  `session_question_mark_opens_the_footer_mode_shortcut_overlay` compared those rows; its structure asserts stay and the row compare moved to an `#[ignore]`d twin with the reason. (3) Voice, Change mode and the
  WSL paste key rows are omitted (no voice availability, collaboration-mode or WSL fact).
- Needs / lander (step 4): `session_page.rs` (chunk-live's): `shortcut_panel(cx, running)` (one line before the composer arm), `body = body.child(panel)` (the panel is its own `Flexible`) under `self.shortcuts && searching.is_none()`,
  `Line::Close` (unit), `on_close: Some(cx.send(ChatMsg::Shortcuts))` in the `Footer { .. }`. `tests/sheets/dialogs.rs` (dialogs'): the split above. `footer/entries.rs` (mine): `on_close: None`.

## Deltas
- The oracle has `DOUBLE_PRESS_QUIT_SHORTCUT_ENABLED = false`: Ctrl+C / Ctrl+D never show `ctrl c again to quit`, the double press does not exist. The brief's quit-reminder
  page tests are therefore not written; `Line::Quit` and `Line::EscIdle` exist for the snapshot entries only (`footer_ctrl_c_quit_idle`, `footer_esc_hint_idle`).
- `warning_notice_pluralizes_and_compacts` (tests/pages/status_data.rs, an unowned old test) asserted the compact `⚠ 3 · f2` at 48 columns from the older
  `warning_notice_layout` budget (half the width, min 14). The oracle's `passive_hints` puts the warning in the hint row as a label (`warning_notice_layout` is only
  the lone-context-meter case), so the full notice stays beside `← for agents` at 48 and the compact one shows at 30. The test now says so.
- Passive footer with a draft: no `? for shortcuts` / `← for agents` (oracle `ComposerHasDraft`); the old footer showed them.

## Needs / lander (edits in files that are not mine; each is the smallest wiring, rebased clean onto 252acefb6)
- `src/pages/session/mod.rs` (chunk-live's): `mod footer;` and the two lines that chain `footer::entries::debug_entries()` into `debug_entries()`.
- `src/pages/session/session_page.rs` (chunk-live's): the `use` line, and the build's footer block: `Footer { line, agents, draft_empty, shell, on_queue, on_warnings }`
  replaces `composer::footer::{footer, with_shell_mode, FooterLine}` (3 spellings of the footer line become `Line::{Close, EscPrimed, Passive}`); the `with_warning_notice`
  wrap stays only for the read-only thread (the Footer carries its own notice). A lander that has chunk-live's newer `session_page.rs` re-applies exactly these hunks.
- `tests/pages.rs` (registration, one line: `mod chat_status;`) and `tests/pages/transcript_follow.rs` (chunk-transcript-2's; two assertions `edit the previous
  message` → `edit previous message`, the oracle's `esc again to edit previous message`).
- `tests/pages/transcript_follow.rs` (second edit, step 2): `the_page_keys_are_the_transcripts_with_a_multiline_draft` presses `pagedown` twice: with the status line on and
  nothing to hint the footer row is gone (oracle `footer_hint_height` 0), so the viewport is one row taller at the bottom than while the follow control's footer shows, and the
  page-down from the away state ends one row short (the page is the viewport at the time of the key).
- `src/pages/session/session_page.rs` (second edit, step 2): the status `Row` block is `StatusRows { spans, composer: w.ui.composer.clone(), hints }` (+ `let mut hints` set by
  the non-search arm, `status_spans` instead of `status_line`).
- `src/pages/session/composer/footer.rs` and its `mod footer;` in `composer/mod.rs` (transcript/composer's): dead since step 1 (nothing names `FooterLine`, `footer`, `with_shell_mode`);
  the lander deletes both.
- Kit (tests/oracle_snap/mod.rs, the kit owner's): the oracle's `frame()` snapshots (rows between `|`) are not read; `status_rows_respect_host_budget...` compares row by row until
  they are. A palette seed for the terminal's default colours (`light_status_line_corrects_pale_custom_theme_colors`) and a colour-depth option.
- `tests/pages/status_data.rs` (old unowned test): one assertion changed (see Deltas).
- `tests/oracle/pane_footer_2.rs` (un-pend of the Ctrl+C history-search test): in my area (widened on the record).

## session_page.rs after the rebase onto af5799546 (chunk-live's file; tip's version + only these lines)
- imports: `use crate::pages::session::footer::{Footer, Line, StatusRows};` replaces the `composer::footer` imports; `status_spans` replaces `status_line` in the `widgets::status_line` import;
  `super::status_surfaces::{StatusContext, TitleInputs}`.
- the composer arm of the bottom pane: `let mut hints: Option<Footer> = None;` before the `(composer, footer)` tuple; the Passive/Close/EscPrimed `Line` choice, `Footer { line, agents, draft_empty,
  shell, on_queue, on_warnings }` (kept as `props`, cloned into `hints`) in place of `footer(cx, line)` + `with_shell_mode`; the follow control still wraps the Passive footer.
- the `with_warning_notice` wrap: `searching.is_none() && thread.read_only()` (the composer's Footer carries its own notice).
- the status row block: `status_spans(..)` and `StatusRows { spans, composer, hints }` in place of the `Row` + `Expanded(line)`.
- the title block: `context.title(facts, TitleInputs { spinner, action_required })`; `titles.push(WindowTitle { title })` or `titles.push(TerminalTitle::new(title.text(None), ..))` (the tip's `titles` list kept).

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
3 status items, warnings, title; (4 the `?` overlay: done);
5 Working row, preview, background terminals; 6 effort transition/ignition; 7 sparkle; 8 voice footer/strip. Then the pty run (idle footer, `?`, a running turn with a queued
message, a 40-column terminal), the 6 ui_tree status-line tests last (person decision pending), and the final checks.
