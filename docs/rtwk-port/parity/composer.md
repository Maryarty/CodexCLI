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
- G68: `Autocomplete` popup layout: floating rows, label/detail two spaces apart, no
  aligned columns, no hint line (command and mention popup snapshots: 5 composer_popups
  and 2 composer_2 tests).
- G69: `Autocomplete::on_selected` tells the pick, not the key that made it (the
  oracle's Tab completes `/compact ` as text, Enter runs it): `took` runs before the key
  hook, so no `EditingKeys` arm sees a pick key; 2 composer_2 tests. Rechecked in the fix
  round: the highlighted suggestion IS readable (`AutocompleteState::suggestions()` through
  a `GlobalKey` and `with_state`), the first row claimed it was not. The typed `/` that
  completes the highlighted command (composer_2 `slash_key_completes_...`) is therefore
  not a gap but not built (the oracle's version keeps the draft's tail as inline
  arguments): pending to composer-2 with that reason.
- G70: `Autocomplete` rows take no mouse: a click does nothing (`composer_mouse`
  `a_click_on_a_popup_row_picks_it`, `#[ignore]`d as pending(G70)).

## Pending and why (the rest, none built here)
- Paste burst, what stays pending: composer_3 `shortcut_modified_input_does_not_enter_paste_bursts`
  and `non_text_input_flushes_pending_paste_without_inserting_text` (the kit's `keys()` has no
  SUPER modifier: the kit does not enable the kitty keyboard protocol),
  composer_3 `plain_and_shift_spaces_insert_normally` (the oracle's snapshot is the composer
  alone with its own `100% context left` footer; the typing is the page test
  `composer_burst a_space_inside_a_burst_is_kept`),
  composer_2 `ascii_burst_treats_enter_as_newline_even_when_parent_owned` (no parent-owned
  thread seed), composer_4 `paste_burst_tab_does_not_accept_a_completion` (G69).
- Images `[Image #N]`: core `StartTurn` is text only (no attachment in the protocol
  mapping), and the kit has no `attach_image` seed.
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
A click on a popup row is G70 (rtwk). In the oracle shards: composer_4
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
