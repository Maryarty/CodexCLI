# composer — parity (interim: second agent's handoff)

S10, the SessionPage composer. Oracle `bottom_pane/chat_composer.rs`, `textarea.rs`,
`chat_composer_history.rs`. View `src/pages/session/composer/`:
- `mod.rs`: the `Composer` widget: the prompt (`›`, or shell mode's light-red `!`),
  then `TextArea` under `Autocomplete`, then `EditingKeys`. `take_submission` is
  what Enter and Tab send.
- `editing.rs`: Codex's keys (atomic placeholders, kill ring, history at the ends,
  shell mode in/out, Ctrl+C clear).
- `draft.rs`: `ComposerDraft`, the per-thread companion of the `TextEditingController`
  in `ThreadUi::draft` (elements, kill ring, history walk, shell mode).
- `footer.rs` (with `with_shell_mode`, the right-hand `Shell mode` label), `strings.rs`.

The core rule is `screens::composer::large_paste`.

## Shard counts (pass / n/a / pending / red of total), at ba9df9960
- composer_1: 8 / 1 / 41 / 0 of 50
- composer_2: 17 / 4 / 20 / 9 of 50
- composer_3: 17 / 2 / 37 / 0 of 56
- composer_4: 5 / 0 / 68 / 0 of 73
- composer_popups: 10 / 0 / 18 / 15 of 43
- textarea: 11 / 2 / 46 / 0 of 59
- total 68 pass (50 at the first handoff, 15 against base).

Red: 9 composer_2 tests were red on base (slash dispatch and completion, mention popup
snapshot, `handle_paste_small_inserts_text`); 15 composer_popups reds (command popup,
history search styling and paste, skill completion) are not yet checked against base.

## Decisions
- Placeholders are atomic elements through the controller (EditingKeys and a
  controller listener), the person's decision on chat-composer 2026-11-19. Not a gap.
- Shell mode (`is_bash_mode`) is UI state in `ComposerDraft`: a `!` typed at the
  start, or yanked there, becomes the prompt; Backspace at the start and Esc on an
  empty draft leave it; history recall of `!x` restores it; no command popup in it.
  Tab submits when idle, except a shell command. The `!cmd` is sent as text through
  `run_line` (running the user shell command is not the composer's).
- Local history entries keep their placeholders in `ComposerDraft`. The core
  history stays `Vec<String>` (ADR-003: submits go through `run_line` / `Intent::Record`).
- The textarea shard opens the debug entry `composer/textarea`, the bare TextArea.
  The debug menu under it marks its Column `overflow_intended` (one line in
  `src/app/debug/menu.rs`, the announced exception).
- A `shell()` token was added to `src/app/theme.rs` (light red, the oracle's
  `light_red()`): startup-finish's area; the lander resolves.

## Gap rows
- G65 (new): rtwk `TextArea` wrap and caret at full rows, wide/half-width graphemes,
  hanging spaces and tabs, rect clearing and scroll (21 textarea tests).

## Pending and why
- vim -> keymap task. Paste burst, blockquote paste, images `[Image #N]`, mentions/skills/
  plugins seeds, input-size limit, external editor, footer modes (NO_FOOTER,
  NO_LAYOUT, NO_FLASH) -> not built yet.
- Kept markers whose bodies pass but do not express the oracle's point (the feature is
  missing, so they pass vacuously): composer_2 reopening_identical_unified_mention_*,
  restored_bound_at_mentions_*, non_ascii_burst_*; composer_3 dismissed_file_popup_* (2),
  submit_captures_recent_mention_bindings_*, history_navigation_restores_remote_only_*,
  left_navigates_only_from_an_empty_prompt; composer_4 vim/image/handoff/reconnect/
  right-click ones; composer_popups 4 seed rows; textarea KEYMAP and LINK ones.

## Unit tests
- `composer::editing::tests`: the oracle's kill and yank state tests, plus the atomic
  placeholder keys. Bite shown: breaking the EOL kill and the yank turns them red.
- `composer::draft::tests`.

## Mouse
TextArea's own mouse: a drag selects (composer_4 blockquote_paste_replacing_selected_*,
backspace_deletes_a_body_selection_*; textarea target_replaces_*), a click inside a
placeholder snaps the caret to its end.

## Binary run (ba9df9960)
`check.py --bin rtwk --bin oracle --only multi-turn,interrupted --capture-dir /tmp/composer-cap`:
4/4 PASS (type and submit three turns; Esc interrupts a streaming turn and the next
prompt works, 0.47 s rtwk / 0.45 s oracle). Captures are raw ANSI; the composer rows of
`interrupted/turn-1-working` differ: the oracle's footer is the model/cwd status line,
ours is `tab to queue message` (the submitted draft is still in the composer until the
model accepts it) and the key hints. No pty scenario types `!`, pastes or queues yet.
