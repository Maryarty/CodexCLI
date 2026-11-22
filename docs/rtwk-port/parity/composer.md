# composer — parity

S10, the SessionPage composer. Oracle `bottom_pane/chat_composer.rs`, `textarea.rs`,
`chat_composer_history.rs`, `chat_composer/{paste_input,paste_tests,skill_completion_tests,
blockquote_paste_tests}.rs`. View `src/pages/session/composer/`:
- `mod.rs`: the `Composer` widget: the prompt (`›`, or shell mode's light-red `!`),
  then `TextArea` under `Autocomplete`, then `EditingKeys`. `take_submission` is
  what Enter and Tab send. `ComposerMsg::Picked` tells a pick of the popup.
- `editing.rs`: Codex's keys (atomic placeholders, kill ring, history at the ends,
  shell mode in/out, Ctrl+C clear, a typed punctuation mark taking a `$skill`
  completion's separator space).
- `src/widgets/composer_draft.rs` (moved down from `pages/session/composer/draft.rs`,
  composer-2; rules test `widgets_never_import_pages`): `ComposerDraft`, the per-thread companion of the `TextEditingController`
  in `ThreadUi::draft` (elements, kill ring, history walk, shell mode, paste, the
  separator rule of a completion).
- `footer.rs` (with `with_shell_mode`, the right-hand `Shell mode` label), `strings.rs`.

Core (no rtwk), `core/src/screens/composer.rs`: `large_paste`, `blockquote_paste`,
`sanitize_paste` (unit test in the file); `screens/popovers.rs` lists skills under `@`.

## chunk-composer (the whole composer as one job; in progress, steps 1-7 of 11 landed)
Branch `worktree-agent-a8b7d6f0f27a204d2` (rebased on `claude/tui-rtwk` 2026-11-22).

Done:
- Keymap wiring: Composer/Editor contexts (submit, queue, the 17 editor actions,
  `kill_whole_line`); every key of the composer goes through `crate::keymap`.
- Vim engine in core (`core/src/screens/composer/vim/{mod,engine,motions,history}.rs`):
  pure, a `VimText` host trait, `VimMode` (not `Mode`: R14 is name-based and the agents
  page has its own `Mode`), 19 state tests from `vim_commands_tests`, `History<S>` (64
  steps, 1 MiB) with 3 unit tests.
- Vim in the view: `src/pages/session/composer/vim.rs` (keymap Vim contexts into `Input`,
  chords from the `.keymap` view API, the `/` query in its own controller, Insert/Replace
  recording for `.`, `u`/Ctrl+R through the draft's history), `src/widgets/composer_draft/vim.rs`
  (`Modal` UI state, `Snapshot` of the whole draft, `VimHost` = `VimText` over the controller plus
  the draft's atomic elements and the kill ring as the register). A paste in Normal mode is
  its own undo step (the draft inserts a plain paste itself in Vim, the step has to close with it).
  11 page tests in `tests/pages/composer_vim.rs` (`global.toggle_vim_mode` on F9).
- Turned on: textarea 14 -> 18, composer_4 12 -> 16 (see counts).
- Step 7, Ctrl+R as a mode of the composer (2c4294dbe): the search lives in the composer
  because a paste reaches only a focused TextArea and the draft's `Snapshot` restores
  text, caret, placeholders and shell mode exactly. Core `screens/popovers.rs`:
  `HistorySearch` (pure: query, cursor, `Idle/Match/NoMatch`, `display_query` with `↵`/`⇥`,
  `step`, `set_query`, `preview`; 4 state tests in `core/tests/popovers.rs`).
  `widgets/composer_draft/search.rs`: `Session` (the search, the draft as it was, Vim's undo
  steps kept aside), `search_begin/edit/step/accept/cancel`, `search_rev` (the page draws
  again when it changes: the footer reads the draft, the page state holds a counter, R14).
  `pages/session/composer/search.rs`: the keys (`handle_history_search_key`: bound keys and
  Up/Down step, Esc/Ctrl+C cancel, Enter accepts a match only, Backspace/Ctrl+H/Ctrl+U edit,
  a character extends, every other key is consumed), the paste into the query (sanitized,
  whole), and the footer row (label dim, query info colour, `enter accept · esc cancel`
  keys bold words dim, `no match` red). The old screen-state path (`ChatMsg::Search`,
  `search_key.rs`, `Intent::Search*`, `Action::SearchPrevious/Next`) is gone. 5 page tests in
  `tests/pages/popovers.rs` (paste into the query, `↵`/`⇥` marks, cancel restores the draft
  with placeholder and caret for Esc and Ctrl+C, footer styles, a click in the preview), each
  shown biting once.

- Step 6, persistent history (pure core, ADR-003): the shared half and the per-composer half
  are split. Core `screens/composer/history.rs` (no rtwk, no IO), a split carry of the oracle's
  `ChatComposerHistory` and `search_batch.rs`:
  - `Store` (shared, changed only through `apply`; `popovers::Model.history: Shared<Store>`):
    the thread's log id and entry count, the entries and batches the host answered (`on_entry`,
    `on_batch`, `on_batch_error`), `rev` for polling. A reply with a stale `log_id` is ignored.
  - `History` (one per composer draft, `ComposerDraft`): the oracle's one offset space
    (persistent entries, then local ones), the walk cursor, the local submissions, replay seeds
    (`record_replayed_submission`), the last recalled text, `navigate_up/down`, `search`,
    `poll`. It never reads or writes the store: it takes `&Store` and puts the reads it needs
    into a `Vec<Lookup>` outbox (`Lookup::Entry{log_id, offset}`, `Lookup::Batch{log_id,
    end_offset}`), so a lookup is a value, not a call.
  - Ctrl+R as the oracle: probe the newest entry as one read, then bounded batches, dedupe by
    exact text, a failed batch retried `MAX_BATCH_READ_RETRIES` = 2 times, then `NoMatch`.
    `SearchStatus::Searching` is drawn in the footer (`searching history`, dim) until the
    reply comes; `Preview::{Show, Original, Keep}` says what the draft shows meanwhile.
  - Recording as the oracle: a submit records the expanded text with its image placeholders and
    local image paths; Ctrl+C records the placeholder text plus `pending_pastes` and writes no
    file; a search accept and a cancel reset the walk as `reset_search`/`reset_navigation` do;
    Vim Normal's last-character caret counts as the end (`vim_end`, `history_navigation_cursor`).
  - Outbox: `popovers::Ask::{Append(text), Lookup(Lookup)}` in `Model.asks`, drained with
    `take_asks()`; events `Ev::{History(seed), HistoryMetadata, HistoryEntry, HistoryBatch,
    HistoryBatchFailed}`; intents `Intent::{Record, Lookup}`.
  - View: `ComposerDraft` keeps `prompts: History` (the old `recalled`/`local` are gone);
    `ComposerMsg::{Lookups, Polled}` send the draft's lookups out through `cx.emit` and resume
    the walk when the store changed (`history_poll`, sent from `build` while waiting).
    `page/composer/{editing,vim,search,strings}.rs` use it.
  - Tests: `core/tests/composer_history.rs` (21: the oracle's history and batch tests carried,
    3 new: a late metadata keeps the replay seeds, a stale log id is ignored, local entries
    survive a metadata change), `core/tests/popovers.rs` (history search over the store, a
    Pending/Unavailable test), `tests/pages/composer_history.rs` (7: Up asks the host and the answer fills the draft,
    an answer for another log is ignored, Ctrl+R says `searching` until the host answers and
    then previews the match, a failed read restores the draft and says nothing, Ctrl+C keeps
    the draft for Up but writes no file, a submit is appended and found by Up, Vim Normal
    walks on from the last character); shown biting once (the lookup left out, the poll left
    out, the Vim end left out).

Counts now (`summary.py --run` of the six shards): composer_1 8/1/41/0 of 50, composer_2
30/4/16/0 of 50, composer_3 33/2/21/0 of 56, composer_4 16/2/55/0 of 73, composer_popups
14/0/29/0 of 43, textarea 18/2/39/0 of 59: 119 pass, 11 n/a, 201 pending, 0 red (109 pass
before this job's Vim steps). `shards.py --check`: 55 shards, 2560 tests, 0 files stale. Whole
`--workspace` nextest 1906 passed, 17 skipped; fmt, clippy `--workspace --all-targets
-D warnings`: clean.

Step 6 counts: core 914 passed (1 skipped), UI crate 955 passed (16 skipped), whole `--workspace`
nextest 1951 passed, 17 skipped (one earlier run failed `startup::session::a_remembered_mode_opens_without_a_prompt`
"the resume never came" under load, startup's, passes alone and in the rerun); fmt, clippy
`--workspace --all-targets -D warnings`, `shards.py --check` (55 shards, 2560 tests, 0 stale): clean; the six shards
are unchanged (119 / 11 / 201 / 0, 0 stale): every pending marker that step 6 touches also
needs a seed this job does not own (see Needs) or a later step (remote image URLs: step 9;
mentions: step 8; a keymap seed: kit). With `KIT_PROBE_PENDING=1` 29 pending bodies pass; their
markers say they would be half passes, so they stay.

Imported oracle tests corrected (the kit's expectation was not the oracle's):
- `removing_the_shell_prefix_is_its_own_undoable_edit`: the oracle compares the whole draft
  (`draft_snapshot() == before`): undo returns to shell mode with "foo", drawn `! foo`.
- `undo_and_redo_restore_deleted_large_paste_payload`: the oracle sets the caret after
  "alpha " and presses `i`; Esc leaves the caret on the space, so the test presses `A`.
- `vim_e_advances_from_each_word_end` stays pending (the snapshot joins three states with
  caret rows; the page test drives the three `e` presses).

Step 7 deltas: while the search is open the terminal cursor stays at the preview's caret (the
oracle puts it at the end of the footer's query, clamped to the last column: `Some((31, 4))`,
`Some((79, 4))`); the Flutter way is a focused read-only `TextField` for the query in the
footer (rtwk's caret belongs to the focused editing widget), not built: the key owner would
move to it. No match highlight (G-new-1: `TextArea` paints no per-range style, the Flutter
`buildTextSpan` override). The `Searching` state of a background lookup and the lookup are
step 6. The four paste snapshot tests (`history_search_pasted_query`, `_large_paste_cursor`,
`_empty_paste`, `_pasted_separators`) are pending `BARE_COMPOSER`: the oracle renders a bare
composer at 5 rows (textarea with a spare row, cursor in the footer), the chat draws the same
composer one row lower under its gap row; the page tests hold the text, marks, footer and
caret. `history_search_paste_preserves_original_draft_on_miss_and_cancel`: the shard body now
sets the caret to offset 2 as the oracle does (`home right right`), it passes.
Stale marker for its owner: `pane_footer_2::ctrl_c_cancels_history_search_without_clearing_draft_or_showing_quit_hint`
passes with the marker probing (ui-pane's shard, not edited here).

Step 6 deltas: a search accept keeps the walk cursor, as the oracle does (the old port had no
place in the walk); `startup_local_history` and its test do not apply (the thread's composer
starts empty and the store comes from the host); images in a recalled entry pair with
`local_image_paths` by position, not by seq or label number (step 9 relabels);
`remote_image_urls` and `mention_bindings` are carried in `HistoryEntry` but not applied to the
draft yet (steps 8 and 9; `HistoryEntry::persisted` is the mention-codec hook, and
`persistent_restore_gates_at_mentions` waits for step 8); replay seeds and local entries
survive a metadata change (each thread has its own composer and store).

Deltas (one line each): `word_pieces` by separator class, not UAX29; no counts (as in the
oracle); a dot-repeated `Yank` replays the register text only; mouse selection is not in
the engine; paste-burst x Replace (`retract_paste_burst`) and `absorb_prompt_prefix` not
ported; Vim j/k move by logical lines; text typed through the paste-burst flush is not
recorded for `.`; `MoveLineStart/End` pass `move_up_at_bol`/`move_down_at_eol` false.

Needs (other owners' files):
- Footer (`composer/footer.rs`, `strings.rs`; chunk-status): the `Vim: <mode>` label, the `/`
  query line and the context line; every composer-alone frame includes the footer row.
- `actions.rs` `Outcome::Session{intent: Vim}`: toggle the draft's Vim plus the info cell
  "Vim mode enabled."/"disabled." (core `app/session.rs`, `chat.rs`, `dispatch.rs`); the
  `tui.vim_mode_default` config.
- `core/src/app.rs` and `host` (chunk-live): history. Core: `Command::History(popovers::Ask)`
  and a drain of `model.popovers.take_asks()` after every apply (until then the outbox only
  grows; `Append` is never persisted). Host: arms over `codex-message-history`
  (`history_metadata`, `append_entry`, `lookup`, `lookup_batch`), its dependency in
  `host/Cargo.toml` in its own commit; the replies map to `popovers::Ev::HistoryEntry`,
  `HistoryBatch{rows, next_older}`, `HistoryBatchFailed`; the host keeps the
  `HistoryBatchCursor` byte anchors by `end_offset`; the metadata is read when a thread opens
  and delivered as `Ev::HistoryMetadata{log_id, entry_count}`.
- Also there: FileSearch (`fuzzyFileSearch`, D1), ClipboardImage.
- Kit (`tests/kit/*`): a Vim seed, SUPER/kitty modifiers, terminal default colours, right
  click, parent-owned and disconnected seeds, a caret-row/multi-frame snapshot compare.
- `CORE_DEPS` in `tests/rules/layers.rs` when core gets a dependency.

Left (steps 8-11): the `remapped_history_search_does_not_fall_back_to_ctrl_r` test needs a
keymap seed (kit); the `@` mentions popup
with file search and tabs; images and attachments (Ctrl+V needs host); reconnect/offline and
Ctrl+G (G67 pending); remaining tests, gap rows (G-new-1 Ctrl+R match highlight and links, G65, G66,
G67, G68, G11), the embedded plain-text composer (D4), the binary run and `check.py`.

## Shard counts (pass / n/a / pending / red of total), composer-2 third run (paste burst in)
- composer_1: 8 / 1 / 41 / 0 of 50
- composer_2: 24 / 4 / 22 / 0 of 50
- composer_3: 20 / 2 / 34 / 0 of 56
- composer_4: 11 / 2 / 60 / 0 of 73
- composer_popups: 12 / 0 / 31 / 0 of 43
- textarea: 11 / 2 / 46 / 0 of 59
- total 86 pass, 11 n/a, 234 pending, 0 red, 0 stale (77 pass at the first handoff, 74 at
  the fix round of the first run). `summary.py --run` of the six shards.
`shards.py --check`: 55 shards, 2560 tests, 0 files stale. Whole default nextest
(`--workspace`): 1058 passed, 15 skipped, 0 failed (including `composer_draft` 5,
`composer_mouse` 2 + 1 ignored, the sanitize unit test). fmt `--check` and clippy
`--workspace --all-targets -D warnings`: clean.

Reds on the base tip that this branch moved: 9 composer_2 and 15 composer_popups
tests were red on `claude/tui-rtwk` itself (checked first, as the checkpoint asked).
Fixed: `handle_paste_small_inserts_text` (paste sanitizing), the `$`/`@` skill
completion cases (separator space, punctuation, skills under `@`), the
`/mention` test (it asserted the composer alone; the app inserts the `@`). The rest
are pending with their reason (below), none loosened.

## Decisions
- Paste burst (composer-2, third run). Core `screens/composer/paste_burst.rs` is the oracle's
  `bottom_pane/paste_burst.rs` carried as it is (REUSE; `Instant` became a `Duration` on the
  app clock, R11; the oracle's 5 unit tests are kept). The view side: `ComposerDraft` holds
  the machine, `pages/session/composer/burst.rs` asks it before the field does (the
  `EditingKeys` door: `take_key` runs `burst::take`, then `edit_key`): a plain char is held
  9 ms (`RetainFirstChar`) or joins a burst, Enter and Tab inside a burst are `\n` and `\t` of
  the paste, a shortcut or navigation key flushes what is held first. What is held is
  flushed by an app timer (`AppBinding::start_timer`, `ComposerMsg::BurstDue`): every
  accepted key arms its own 9 ms timer, an early one finds nothing due. A real paste event
  flushes the held keys first and ends the window (`clear_after_explicit_paste`). The page's
  `?` and `Left` shortcuts of an empty prompt wait while `ComposerDraft::in_burst`
  (`is_in_paste_burst`). A `!` paste at the start enters shell mode before the text
  changes (`absorb_shell_prefix` sets the mode first: the command popup reads it on the
  change, a `!/diff` paste must not open it).
- Kit pacing (tests/kit/mod.rs, tests/common/mod.rs): the keys of a person are
  `KEY_PACE_MS` = 10 ms apart: `Screen::type_text` and now `Screen::keys` (token by token),
  and `Harness::press` waits 10 ms before the key (a `type_text` then `press("enter")`
  must not be one burst). `Screen::type_burst(text)` is a terminal without bracketed paste:
  a key per char 1 ms apart (`\n` is Enter, `\t` Tab), nothing waits after the last. The
  app draws one frame per 16.7 ms, so the screen right after `type_burst` shows the state
  at the first key: tests read it after `wait` (the oracle's "the draft is empty while the
  burst holds" is the core unit test of the machine). Raw `Tester::type_text` in other
  tests is one instant: `tests/services/shell_cycle.rs` got a `pump_for(50)` after its
  typing (the page's Ctrl+X chord does not flush a held burst) and
  `tests/pages/composer_draft.rs` waits for the shell command to reach the fake
  (`wait_idle` was true before it arrived: a flake of the first run).
- Placeholders are atomic elements through the controller (EditingKeys and a
  controller listener), the person's decision on chat-composer 2026-11-19. Not a gap.
- Shell mode (`is_bash_mode`) is UI state in `ComposerDraft`: a `!` typed at the
  start, or yanked there, becomes the prompt; Backspace at the start and Esc on an
  empty draft leave it; history recall of `!x` restores it; no command popup in it.
  Tab submits when idle, except a shell command. Shell input pastes stay literal (no
  blockquote quoting).
- composer-2: a `!cmd` line runs as the user's shell command (`submit_shell_command`), and
  only a draft in visible shell mode does (a pasted placeholder whose payload starts with
  `!` stays a model prompt: `composer::shell_line(line, shell_mode)` in core, unit-tested).
  Path (ADR-003, no RPC in a screen): `run_line` -> `chat::Intent::Shell` -> `app.rs`
  `apply_thread(ScreenThreadIntent::Chat(ThreadIntent::Shell))` -> `chat::apply_thread_intent`
  -> `Command::Shell` -> `chat::wire` -> `ThreadAsk::Shell` (`thread/shellCommand`,
  `core/src/threads.rs`; the rules test `thread_rpcs_only_in_threads` now names
  `ThreadShellCommand` too). `!` alone is the `Notice::ShellHelp` (the oracle's usage
  line). A turn running or not does not matter (`core/tests/composer_shell.rs`). No local
  echo: the result comes back as the server's item, as in the oracle. Tests:
  `composer_shell` (core, 3), `composer_draft` `a_shell_draft_the_thread_took_clears_...`,
  `a_bang_slash_line_is_a_shell_command_not_the_slash_command`,
  `a_bang_after_the_start_is_part_of_the_prompt`; shard composer_2
  `bang_prefixed_slash_text_submits_literal_shell_command` (its first form asserted the
  line as a prompt in the transcript; now the screen shows the cleared composer and the
  request is in the page test).
- composer-2: input-size limit. `composer::too_large(text)` (core, chars over
  `MAX_INPUT_CHARS` = 1 << 20) -> `Notice::InputTooLarge { provided }` (shared
  `core/src/commands/dispatch.rs`, two enum lines) -> `chat_strings.notice()` -> the notice
  above the composer; the draft stays. Both Enter and Tab (queue) go through `run_line`.
  The oracle's `set_steer_enabled` is a no-op (`_enabled`), so `oversized_queued_...` is the
  same Enter there; the port drives its Tab route. Both shard tests assert the placeholder
  row `› [Pasted Content 1048577 chars]`: the oracle's tests set the text with
  `set_text_clearing_elements`, which keys cannot reach (a paste that size is a placeholder).
  Bite shown: with the two checks taken out of `run_line` both tests fail.
  `composer_4 embedded_composer_shows_validation_error_...` stays pending: the
  `Message too long` flash belongs to the plain-text embedded composer, which has no entry.
- composer-2: the drag selection IS drawn, with the theme's `on indexed(237)` background,
  not REVERSED: rtwk's `TextArea::selection_style` (default the theme's `selection`) is the
  Flutter `selectionColor`; the composer sets `Style::new().attrs(Attrs::REVERSE)` as the
  oracle's textarea draws it. Not a gap (the earlier "not diagnosed" row text is gone).
  `selection_release_copies_and_keeps_the_editable_selection` passes;
  `selection_release_reports_only_clipboard_result` is n/a (the oracle's
  `copy_selection_on_release` / `finish_copy` closure API, no screen). Mouse tests:
  `composer_mouse` `a_double_click_selects_the_word_and_typing_replaces_it`,
  `the_drag_is_drawn_reversed_...`.
- Shell mode and the history walk end where the composer clears, not at Enter
  (`ComposerDraft::cleared`, called through `composer::clear_sent` by the page when the
  thread took the submit, by a command that took the line, and by Ctrl+C). A submit the
  thread did not take (no writer) clears nothing and keeps the `!` prompt and the walk;
  the page re-reads its draft flags after each clear and the `Composer` widget carries
  the shell flag it was built with, so its prompt follows. Tests: `composer_draft`
  `a_shell_draft_the_thread_did_not_take_stays_a_shell_draft` and
  `the_history_walk_goes_on_over_a_submit_the_thread_did_not_take` (red on the old code:
  `› ls` and `› two`), and `..._the_thread_took_clears_and_leaves_shell_mode`.
- A small paste whose text starts with `!`, put at the draft's start (caret or selection
  at offset 0, not in shell mode), is shell mode: the `!` becomes the prompt, the caret
  stays after the text (`insert_str_at_target` with `ShellPrefixHandling::Interpret`,
  then `sync_bash_mode_from_text`). A large paste is a placeholder first, as in the oracle.
  `a_pasted_bang_at_the_start_is_shell_mode` (red on the old code: `› !ls -la`) and
  `a_pasted_bang_after_text_stays_text`.
- The shell `!` prompt is bold (`light_red().bold()`); the footer's `Shell mode` label is
  not, so the bold is added at the prompt's call site and the `shell()` token is unchanged.
- Local history entries keep their placeholders in `ComposerDraft`. The core
  history stays `Vec<String>` (ADR-003: submits go through `run_line` / `Intent::Record`).
- Blockquote paste is a pure core rule (`blockquote_paste`): after `> ` a multiline
  paste is quoted line by line, counted for the large-paste placeholder, and the
  caret leaves the quote (`\n\n`).
- Pasted text goes through `sanitize_paste` (the oracle's `sanitize_user_text` after
  its CR/CRLF fold) before the large/quoted checks; a paste the sanitizing changed is
  inserted by the draft, an unchanged small one by the field.
- A mention completion's separator follows the oracle's
  `advance_past_completion_separator`: a plain space already after the caret, with
  only blanks behind it, is the separator (the caret steps over it and the
  completion's own space goes); else the new space stays and, for a skill, the next
  key may swap it for punctuation (`,.;:!?)]}` and the CJK marks).
- `@` lists skills too (the oracle's unified mention popup); a skill picked from
  `@` is inserted as `$name`.
- The textarea shard opens the debug entry `composer/textarea`, the bare TextArea.
  The debug menu under it marks its Column `overflow_intended` (one line in
  `src/app/debug/menu.rs`, the announced exception).
- Landing items outside this area: a `shell()` token in `src/app/theme.rs` (light
  red, the oracle's `light_red()`, not bold: the prompt adds the bold itself;
  startup-finish's area, the lander resolves);
  `[[test]] composer_mouse` in `Cargo.toml` (precedent: `agents_mouse`,
  `startup_mouse`); the session page's import order was formatted
  (`session_page.rs`, fmt only).

## Gap rows (docs/rtwk-port/rtwk-gaps.md)
- G65 (renumbered from G55: startup-wizards took G55, chat-cells G60-G64): TextArea
  wrap and caret at full rows, wide/half-width graphemes, hanging spaces and tabs, rect
  clearing and scroll (21 textarea tests).
- G66: Ctrl+J arrives as Enter (`key_combo.rs`), so no Ctrl+J newline; the tests break
  lines with Alt+Enter.
- G67: no widget-level terminal suspend for `$EDITOR` (Ctrl+G external editor; one
  composer_4 test).
- G68 (still a gap): the popup is `Popover` + `OptionList`; the oracle's rows are aligned
  name/description columns with a `›` marker and an `enter insert · esc close` line, rtwk's
  list has a `▎` bar and a right-aligned detail (5 composer_popups and 2 composer_2 tests).
- G69, G70 (closed, route (b)): `Autocomplete` takes Tab/Enter alike and has no mouse, so
  the composer owns `suggest.rs` (`Suggest` model read by the `EditingKeys` hook, drawn by
  `Popover` + `OptionList`): Enter picks (a command runs, a mention is put in), Tab completes
  `/name ` as text, a typed `/` completes the highlighted command, Up/Down/Ctrl+P/Ctrl+N and
  the wheel move the highlight (wrapping), Esc closes until the text changes, a click picks
  the row, a press outside closes. Page tests `tests/pages/composer_suggest.rs` (10); bite
  shown: with the Tab arm off, 4 of them go red. Shards un-pended: composer_2 the two
  slash-tab tests and `slash_key_completes_selected_slash_command_as_text`, composer_4
  `paste_burst_tab_does_not_accept_a_completion`; the click test of `composer_mouse` runs.

- G68 route 2 not tried: the popup rows composed in the layout under the field from
  `Row` / `SizedBox` name and description columns, a `›` `Text` and the hint line; the
  oracle's look may need no rtwk change, the row stays until someone builds it.
- Review fixes (phase 2): a pick rewrites only the word being completed (an image or large
  paste before a `$`/`@` mention keeps its element and goes with the turn; page tests
  `a_mention_after_an_image_keeps_the_image`, `a_click_on_a_mention_row_after_an_image_...`,
  bite shown: with the old whole-prefix rewrite both fail); Tab on `/skills` runs it as Enter
  does (oracle `selected_command_dispatches_immediately_on_tab`).
- Not built, from the review: history recall does not restore images (`popovers.history` is
  text only, shared with Ctrl+R search and the persisted history; chat_live_5
  `ctrl_c_cleared_prompt_is_recoverable_via_history` is pending again with the image check
  in its body); the popup's pick drops no draft tail but does not keep it as inline
  arguments the oracle's way (`complete_selected_slash_command_preserving_existing_draft_tail_as_inline_args`);
  the startup queue replays every item at the thread start, the oracle drains one item per
  finished turn (the order is the same, a queued `!cmd` runs while the message before it
  runs: the same rule as a shell during a turn); images are sniffed as PNG, JPEG, GIF and
  WebP only (the oracle's `image_dimensions` reads more formats).

## Pending and why (the rest, none built here)
- Paste burst, what stays pending: composer_3 `shortcut_modified_input_does_not_enter_paste_bursts`
  and `non_text_input_flushes_pending_paste_without_inserting_text` (the kit's `keys()` has no
  SUPER modifier: the kit does not enable the kitty keyboard protocol),
  composer_3 `plain_and_shift_spaces_insert_normally` (the oracle's snapshot is the composer
  alone with its own `100% context left` footer; the typing is the page test
  `composer_burst a_space_inside_a_burst_is_kept`),
  composer_2 `ascii_burst_treats_enter_as_newline_even_when_parent_owned` (no parent-owned
  thread seed), composer_4 `paste_burst_tab_does_not_accept_a_completion` (built with the popup, see G69).
- Images `[Image #N]` (built 2026-11-21): a pasted image path (quotes, `file://`, escapes,
  Windows paths; the file must sniff as PNG/JPEG/GIF/WebP) becomes an atomic element in
  `ComposerDraft` (numbered by attach order, relabelled on every edit, a yank gets a new
  number); submit carries `UserMessage { text, images }` through `Intent::Submit`,
  `ThreadEv::Submit` and `Command::StartTurn` to `protocol::turn_input` (`LocalImage`
  items first, the text with its `text_elements`). The kit seeds it (`attach_image`,
  `image_file`); tests `composer_images`. Not built: clipboard image paste (Ctrl+V needs
  the clipboard-image dependency and IO), `/plan <args>` carrying the images, remote
  image rows and the thumbnail strip (transcript_view_1).
- Startup draft (built 2026-11-21): typing while the session is `Starting` works (the
  draft is the page's, not the thread's); Enter before the first thread queues the message
  or the `!cmd` in order in `CodexApp` (`BeforeThread`), and both go out when the thread
  starts (`composer_startup`, core `model.rs`). Not ported: a slash command typed before
  the start is answered by `dispatch`'s notice (`SessionStarting`), the oracle queues it
  (`QueuedInputAction::ParseSlash`) and parses it when configured; the empty-state
  animation dismissal; `thread_stream_disconnected` / rate-limit-recovery queue pauses.
- Vim mode, reverse history search (Ctrl+R), shortcut chords from the keymap, rebindable
  queue key (`Ctrl+Q`) -> the keymap task / chat-nav-pane.
- Footer modes (`100% context left`, flashes, the composer-alone 60-column snapshots) ->
  pane-footer.
- `/init` (no handler) and `/diff` (the pager is not built) -> chat-nav-pane / pager.
- Embedded, disconnected and draft-handoff composers, plugin mention bindings, remote
  image rows: no seed on the chat screen.
- Markers whose body passes (27 of the 248, found by the probe, `KIT_PROBE_PENDING=1
  summary.py --run <the six shards> --list stale`: composer_2 3, composer_3 5, composer_4
  13, composer_popups 4, textarea 2). The table's `stale 0` is the normal run, where a
  marker fails first, so it cannot show them; `shards.py --check`'s "0 files stale" is a
  different check (the shard files against the oracle's sources). The markers stay: each
  body asserts only the half the oracle's test has that the port can draw (the feature
  itself is missing: file search, remote images, handoff, Vim, mention bindings, keymap
  config, links), so a pass would be a false one. Two were corrected in this round because
  they were stale for real: `editor_actions_insert_shell_prefixes` (shell mode and Ctrl+K /
  Ctrl+Y are built; only the Vim part `r!` is missing, and the reason now says so) and
  `composer_mouse_edits_flush_typing_...` (the click places the caret). Nine more reasons
  that still said "no large-paste placeholder" or "no mouse selection" were reworded to
  what the probe shows now (images, Vim, file search; the drag selects and is drawn REVERSED now).
- The textarea mouse tests (`MOUSE` in textarea.rs) stay pending: their rows are G65's and
  they need rect offsets (a 1-row rect at y 1, a resize mid-test) the kit cannot seed.

## Unit tests
- `core screens::composer::paste_burst::tests` (the oracle's five), page tests
  `tests/pages/composer_burst.rs` (9: Enter in a burst, `?` in a burst, `?` alone, Left
  behind a held char, a lone char after 9 ms, a space in a burst, a space at a person's pace, `keys("h i enter")`
  submits, a large burst is one placeholder). Bite shown once: with the page's
  `in_burst()` check off the `?` and Left tests fail; with the Enter arm off the Enter test
  fails.
- `composer::editing::tests`: the oracle's kill and yank state tests, plus the atomic
  placeholder keys. Bite shown: breaking the EOL kill and the yank turns them red.
- `widgets::composer_draft` tests (moved with the file).
- `core screens::composer::tests`: `a_message_over_the_limit_is_too_large_by_its_characters`,
  `only_a_draft_in_shell_mode_runs_a_bang_line`; `core/tests/composer_shell.rs`.
- `core screens::composer::tests::sanitize_paste_*`: the oracle's `sanitize_user_text`
  cases and the `_count_rows` paste.

## Mouse (ADR-011)
`tests/pages/composer_mouse.rs` (kit mouse events on the live chat screen): a click puts
the caret in the draft (`› helloX world`), a drag selects and the next typed text replaces
it (`› hello X`). Bite shown: without the click and the drag the rows read `› hello worldX`.
A click on a popup row picks it (`composer_mouse`, `composer_suggest`). In the oracle shards: composer_4
blockquote_paste_replacing_selected_*, backspace_deletes_a_body_selection_*, composer_3 a click
inside a placeholder snaps the caret to its end, textarea target_replaces_*.

## Binary run (composer-2 third run, paste burst in; debug build)
`check.py --bin rtwk --bin oracle --only multi-turn,interrupted --capture-dir /tmp/composer2-cap`:
4/4 PASS again (multi-turn 15.0 s rtwk / 14.0 s oracle, interrupted: Esc disconnects the
stream in 0.47 s rtwk / 0.33 s oracle). The scenario's typed text and Enter go through the
paste detector in the real binary and the three turns still submit. The text of the previous
run below still holds for the rows.

## Binary run (fix round, debug build)
`check.py --bin rtwk --bin oracle --only multi-turn,interrupted --capture-dir /tmp/composer-cap`:
4/4 PASS (type and submit three turns, 15.0 s rtwk / 14.3 s oracle; Esc interrupts a
streaming turn in 0.27 s rtwk / 0.45 s oracle and the next prompt works). The captures are
raw ANSI, not a rendered grid, so no row was compared from them: the composer rows are
held by the oracle shards. Known difference in the same screens: the oracle's footer is
`? for shortcuts` plus the model/cwd status line, ours is the page's key hints
(`← for agents · ? for shortcuts`, and `tab to queue message` while a draft waits); footer
modes are pane-footer's. No pty scenario types `!`, pastes, queues or uses the mouse yet:
those are covered by the shard tests above, not run in the binary.

## Environment notes
- `~/.cache/rtwk/sccache-main/bin/sccache` was missing on 2026-11-21, so `acargo` failed;
  the checks ran through a copy of `acargo` without `RUSTC_WRAPPER` (same flags, own
  target). An earlier disk-full (ENOSPC) was cleared by removing the scratch worktree
  `/tmp/composer-base` this task had made for the base check.

## Binary run (composer-2 phase 2, popup in; debug build)
`check.py --bin rtwk --bin oracle --only multi-turn,interrupted --capture-dir /tmp/composer2-cap`:
4/4 passed. A pty drive of the rtwk binary (keys one by one, 100x24, ANSI stripped; no screen
emulator, so the diff-rendered stream is read, not a grid): `/stat` opens `▎/status  show
current s...`, `/` lists the commands (`/model`, `/ide`, `/permissions`, `/keymap`...), two
Downs move the bar to `/ide`, Esc closes the list, a typed `/stat` and Enter print the status
card (`Model: gpt-6-sol  Directory: ...`). Tab, startup draft and images were run in the page
tests only. Differing rows against the oracle: the G68 layout (`▎` bar, right-aligned detail,
no hint line); the rest equal in what was driven. A `/stat` plus Enter written in ONE write is a
paste burst in both binaries (Enter is a newline), as the oracle does.
