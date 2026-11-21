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
- `draft.rs`: `ComposerDraft`, the per-thread companion of the `TextEditingController`
  in `ThreadUi::draft` (elements, kill ring, history walk, shell mode, paste, the
  separator rule of a completion).
- `footer.rs` (with `with_shell_mode`, the right-hand `Shell mode` label), `strings.rs`.

Core (no rtwk), `core/src/screens/composer.rs`: `large_paste`, `blockquote_paste`,
`sanitize_paste` (unit test in the file); `screens/popovers.rs` lists skills under `@`.

## Shard counts (pass / n/a / pending / red of total), at c14d490d5 + the mouse file
- composer_1: 8 / 1 / 41 / 0 of 50
- composer_2: 19 / 4 / 27 / 0 of 50
- composer_3: 17 / 2 / 37 / 0 of 56
- composer_4: 8 / 0 / 65 / 0 of 73
- composer_popups: 11 / 0 / 32 / 0 of 43
- textarea: 11 / 2 / 46 / 0 of 59
- total 74 pass, 9 n/a, 248 pending, 0 red, 0 stale (68 pass at the second handoff).
`shards.py --check`: 55 shards, 2560 tests, 0 files stale. Whole default nextest
(`--workspace`): 1043 passed, 14 skipped (before the sanitize unit test and the
mouse file, which pass on their own: 2 + 1 ignored). fmt, clippy `--workspace
--all-targets -D warnings`: clean.

Reds on the base tip that this branch moved: 9 composer_2 and 15 composer_popups
tests were red on `claude/tui-rtwk` itself (checked first, as the checkpoint asked).
Fixed: `handle_paste_small_inserts_text` (paste sanitizing), the `$`/`@` skill
completion cases (separator space, punctuation, skills under `@`), the
`/mention` test (it asserted the composer alone; the app inserts the `@`). The rest
are pending with their reason (below), none loosened.

## Decisions
- Placeholders are atomic elements through the controller (EditingKeys and a
  controller listener), the person's decision on chat-composer 2026-11-19. Not a gap.
- Shell mode (`is_bash_mode`) is UI state in `ComposerDraft`: a `!` typed at the
  start, or yanked there, becomes the prompt; Backspace at the start and Esc on an
  empty draft leave it; history recall of `!x` restores it; no command popup in it.
  Tab submits when idle, except a shell command. The `!cmd` is sent as text through
  `run_line` (running the user shell command is not the composer's). Shell input
  pastes stay literal (no blockquote quoting).
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
  red, the oracle's `light_red()`; startup-finish's area, the lander resolves);
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
- G69: `Autocomplete::on_selected` tells the pick, not the key (the oracle's Tab completes
  `/compact ` as text, Enter runs it) and the highlighted suggestion is not readable
  (a typed `/` completes it): 3 composer_2 tests.
- G70: `Autocomplete` rows take no mouse: a click does nothing (`composer_mouse`
  `a_click_on_a_popup_row_picks_it`, `#[ignore]`d as pending(G70)).

## Pending and why (the rest, none built here)
- Paste burst (composer_2 non_ascii_burst_*, ascii_burst_*; composer_3 burst_*; composer_4
  paste_burst_*, newline_suppression_*; composer_popups `..._before_a_punctuation_paste_burst`):
  the oracle's heuristic (3 characters 8 ms apart, 120 ms Enter suppression) reads a
  clock the kit's `type_text` does not give it (all characters at one instant, then 100 ms).
  Needs the kit to pace typed text first.
- Images `[Image #N]`: core `StartTurn` is text only (no attachment in the protocol
  mapping), and the kit has no `attach_image` seed.
- Input-size limit (`composer_2 oversized_*`, composer_4 embedded validation): needs a
  `Notice::InputTooLarge` in `core/src/commands/dispatch.rs` and its string
  (chat-commands' area).
- Vim mode, reverse history search (Ctrl+R), shortcut chords from the keymap, rebindable
  queue key (`Ctrl+Q`) -> the keymap task / chat-nav-pane.
- Footer modes (`100% context left`, flashes, the composer-alone 60-column snapshots) ->
  pane-footer.
- `/init` (no handler) and `/diff` (the pager is not built) -> chat-nav-pane / pager.
- Embedded, disconnected and draft-handoff composers, plugin mention bindings, remote
  image rows: no seed on the chat screen.
- Kept markers whose bodies pass but do not express the oracle's point (the feature is
  missing, so they pass vacuously; the probe lists them as stale): composer_2
  reopening_identical_unified_mention_*, restored_bound_at_mentions_*, non_ascii_burst_*;
  composer_3 dismissed_file_popup_* (2), submit_captures_recent_mention_bindings_*,
  history_navigation_restores_remote_only_*, left_navigates_only_from_an_empty_prompt;
  composer_4 vim/image/handoff/reconnect/right-click ones; composer_popups 4 seed rows;
  textarea KEYMAP and LINK ones.
- The textarea mouse tests (`MOUSE` in textarea.rs) stay pending: their rows are G65's and
  they need rect offsets (a 1-row rect at y 1, a resize mid-test) the kit cannot seed.

## Unit tests
- `composer::editing::tests`: the oracle's kill and yank state tests, plus the atomic
  placeholder keys. Bite shown: breaking the EOL kill and the yank turns them red.
- `composer::draft::tests`.
- `core screens::composer::tests::sanitize_paste_*`: the oracle's `sanitize_user_text`
  cases and the `_count_rows` paste.

## Mouse (ADR-011)
`tests/pages/composer_mouse.rs` (kit mouse events on the live chat screen): a click puts
the caret in the draft (`› helloX world`), a drag selects and the next typed text replaces
it (`› hello X`). Bite shown: without the click and the drag the rows read `› hello worldX`.
A click on a popup row is G70 (rtwk). In the oracle shards: composer_4
blockquote_paste_replacing_selected_*, backspace_deletes_a_body_selection_*, composer_3 a click
inside a placeholder snaps the caret to its end, textarea target_replaces_*.

## Binary run (c14d490d5, debug build)
`check.py --bin rtwk --bin oracle --only multi-turn,interrupted --capture-dir /tmp/composer-cap`:
4/4 PASS (type and submit three turns, 17.6 s rtwk / 14.9 s oracle; Esc interrupts a
streaming turn in 0.46 s rtwk / 0.36 s oracle and the next prompt works). The captures are
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
