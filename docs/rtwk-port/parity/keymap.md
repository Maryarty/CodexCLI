# keymap — parity (steps 1-4 of the 2026-11-22 paragraph: config → Keymap → app fact → seed → agents page)

Slice: the `tui.keymap.*` config of codex-tui (`keymap.rs`, `RuntimeKeymap`) as a `Keymap` the whole app reads.
Oracle: `codex-rs/tui/src/keymap.rs`, `agent_center/input.rs`, `agents_overview_view.rs`, tests
`agent_center_tests.rs`, `agents_overview_tests.rs`. Vim mode is not part of this run. Mouse (ADR-011): no new
selectable view; `tests/pages/agents_mouse.rs` is unchanged and green.

## The `.keymap` oracle tests (the metric)

57 oracle tests call `.keymap(..)` (`grep -l "\.keymap(" tests/oracle/*.rs`, 20 shards; verdicts by
`python3 -I tests/oracle/tools/summary.py` junit + a per-test list, `/tmp/keymap-work/kmtests.py`).

| | pass | pending |
|---|---|---|
| before | 0 | 57 |
| after | 0 | 57 |

Honest reading: none of the 57 moves, because each one also needs a screen that does not exist yet or a screen
another porter holds. What changed is that the seed now works (`kit_proof` shows it biting), so the reason of each
is no longer "no keymap seed". Probe: the 8 tests of the files in this task's area (`pane_footer_1/2`,
`chat_commands_1/3`) were un-pended and run on the tip: 0 of 8 pass (chat: no copy action, no queued-message hint,
no `chat.interrupt_turn` read, no `?` overlay, no selection view / steer preview / banner / voice strip entry);
their markers stay, with the reasons rewritten to what is left (`chat_commands_1`, `chat_commands_3`,
`pane_footer_2` ×3). The other 49 are in files held by others and were not touched; by owner, what each must wire:

| owner | tests | to wire |
|---|---|---|
| chunk-approvals | `approvals_1` 7, `approvals_2` 6, `chat_approvals_2` 1 = 14 | the dialog's list keys, question editor, queued messages |
| chunk-popups | `chat_popups_1` 3, `chat_popups_3` 1 = 4 | select dialog hints/keys (`list.accept`, `list.cancel`) |
| settings | `picker` 9, `chat_popups_4` 3, `small_dialogs_1` 2, `small_dialogs_2` 3 = 17 | picker/list keys, sheet hints |
| resume | `resume_2` 1 | page/jump keys of the list |
| chat-live / chat-transcript | `composer_2` 3, `composer_3` 2, `composer_4` 3, `chat_live_1` 1, `transcript_view_1` 3, `transcript_view_4` 1 = 13 | composer, transcript and pager keys |
| mine, blocked on pages that do not exist | `pane_footer_1` 1, `pane_footer_2` 5, `chat_commands_1/3` 2 = 8 | `?` overlay, selection view, steer preview, banner, voice strip, copy and queued-edit hints |

## What is here

- **Step 1, core** (`core/src/keymap/{actions,catalogue,key,resolve,spec}.rs`, no rtwk): `Keymap` per context (9
  contexts, 173 actions with typed `actions::*` handles), the oracle's defaults, `tui.keymap.*` specs read as single
  keys, chords and the leader key, the oracle's conflict passes (`resolve`; `resolve_unchecked` for tests and the
  seed, as the oracle's tests set the fields unchecked). `core/tests/keymap.rs`: 32 tests.
- **Step 2, host** (`host/src/facts.rs`, `in_process.rs`): the config's `tui.keymap` becomes `AppEv::Keymap(Box<Keymap>)`;
  `apply` sets `CodexApp.keymap: Rc<Keymap>` (ADR-003, one writer). An invalid config is a startup warning and the
  defaults stay. Run in the binary: see below.
- **Step 3, kit** (`tests/kit/seed.rs`, `tests/common/mod.rs`): `.keymap(&[("list.accept", "f3")])` sets the
  bindings; `Harness::connect()` + `Harness::set_keymap(&[(action, key)])` is the same for plain tests (an action is
  `context.name`, `app.` or a bare name is `global`, a key as the config writes it: `ctrl-x enter`; an empty key
  unbinds; pairs of one action add up). Two real tests in `tests/kit_proof.rs` replace the pending proof.
- **Step 4, view API** (`src/keymap/mod.rs`, provider above the app in `src/app/view.rs`): `keymap(cx)`,
  `names(keymap, id)` / `bindings` (rtwk key text for `KeyBindings`), `pressed`, `label` / `binding_label` /
  `stroke_label` (the shipped platform form: `⌃x ↑` on macOS), `first_of_each` (rtwk's `KeyBindings::new` panics on
  one key twice, `shift+?` and `?` are one key; the first pair wins). The chunks wire their pages with these.
- **Agents command center** (`src/pages/agents/{keys,hints,state,strings}.rs`, `view/*`): keys, footer hints, the
  help (`?`) and the filter hint all read the same `Keymap`. The order of asking is the oracle's: search, task
  actions (resume, grouping, pin, new, new worktree, fork, rename, archive, delete, hide, stop), list actions; a
  key two actions share does what the first does; `?`, Tab, Shift+Tab yield to any center action bound to them
  (`center_shortcut_keys`); an editing line types plain text first and takes every non-text task action (F10 stops
  from the search line), the cancel key and `ctrl+c` always cancel. The oracle's `live_center_custom_footer` and
  `live_center_custom_help` snapshots are equal (`live_center_hints_follow_configured_bindings`, passing).
- **Not wired, because a chunk owns them**: `src/sheets/select` and the picker (chunk-popups), approvals and the
  question editor (chunk-approvals); `src/widgets/options.rs` and `session_list.rs` hold no key of their own (the
  pages bind, G35), so there is nothing to wire there. Pages nobody holds (startup, import, onboarding, sign-in,
  pets, voice) have no `.keymap` oracle test, so nothing there counts toward the metric; not wired in this run.

## Tests

- `agents::filtered_dashboard_actions_use_configured_shortcuts` (debug entry `keys_two`: `v` moves, `k` still
  moves up, `j` does not, F6 searches, F8 opens the resume picker, F7 closes the line, empty search leaves no task
  highlighted, online and offline) and `agents::configured_task_actions_ask_the_backend` (live path on the fake
  backend: F10 on the idle task asks nothing, on the working one asks `ActiveTurn`, also from the search line;
  Enter asks `Resume`). A debug entry runs on its own model and sends no command, hence the live path.
- `agents::the_page_keys_yield_to_the_action_bound_to_them` (Tab, `?`, Shift+`?`, F9, Enter each as the resume
  key: the page gives the key up, hint and help follow, the picker opens), `agents::live_center_hints_follow_configured_bindings`.
- `keymap::a_key_named_twice_is_left_to_the_first_of_its_pairs` (unit test of `first_of_each`).
- Shown biting once: with `fixed()` ignoring the taken keys, `the_page_keys_yield…` fails (30 s timeout waiting
  for the picker) and with the seed not applied `configured_task_actions_ask_the_backend` fails; both reverted.
- New debug entries (`agents`): `keys_two`, `keys_empty`, `keys_task` (can_return, fixtures
  `src/pages/agents/fixtures/keys_*.jsonl`).
- `.config/nextest.toml`: the two un-pended agents tests left the pending lists; `live_center_fixed_shortcuts_yield_to_configured_actions`
  stays listed (equal to the `pending(` markers of `tests/pages/agents.rs`, the rules test is green).

## Gap

- **G77** (`rtwk-gaps.md`): Flutter's `Shortcuts` above a `TextField` handle a plain-character shortcut before the
  field types it; rtwk's `KeyBindings` above a focused `TextField` get only what the field does not take (`is_text`).
  The oracle's `list.cancel = 'q'` leaves the search and the rename line; here the `q` is typed. Test:
  `live_center_fixed_shortcuts_yield_to_configured_actions` is `pending(G77)` and kept whole (its second half is the
  green test above). No workaround: while a line is open the table binds no plain character.

## Departures and debt

- Config diagnostics (`KeymapError`) are the oracle's English text carried into core (23 messages, tested by
  `core/tests/keymap.rs`). The layers and ADR-010 rules scan single-line `format!` literals, which caught three
  sites (config path, leader keyword, ambiguity relation: fixed without words in a `format!`); the rest is the same
  prose in a multi-line `format!` the scanner does not see. Debt: a typed `KeymapError` (variants with the action,
  key and scope) and its text in a strings table of the host (a task to create: "keymap-error-typed"; this agent
  has no tm create call).
- Scopes are data (a table of what is active together), not per-context hand validation; side-pane actions are
  ignored (the port has no panes).
- `startup.rs` `ReloadConfig` does not send the keymap again (a changed `tui.keymap` needs a restart).
- Shift on a non-cased character is dropped when naming an rtwk key (`shift+?` is `?`).
- Labels use the shipped platform form (`stroke_label`); core's `Stroke::label` is the `cfg(test)` glyph form.
- The rename line binds the list's accept key not at all (the field's own submit takes Enter; a configured other accept key does nothing there) and no chord with a plain prefix (G77 family).
- `Esc` is swallowed by the page when the user's cancel key is another: the app `Navigator`'s `back_key` would pop
  the route on it, the oracle's center has no back but `list.cancel`.

## Run in the binary

`codex-rtwk` in a pty (`/tmp/keymap-work/run_agents.py`, a temp `CODEX_HOME` with `[tui.keymap.agents] search = "f6"
stop = "f10"` and `[tui.keymap.list] cancel = "f7"`), `/agents`, then F6, F7, `?`, Esc; the oracle binary under
the same config and driver. The existing mock-model scenarios cannot press keys after a prompt, so no
`check.py --only` scenario covers the center.
- rtwk reads the config: the footer says `? help  f7 back  ↑/↓ move  enter open  n new`, F6 opens `Search ›` and
  the footer is `f7 back  enter open`, F7 closes it, `?` lists `f6 Search` and `f10 Stop`. The oracle shows the same
  rows at each of the four checkpoints except as below.
- Differing rows (checkpoint `1-center`, the details pane): the project path wraps at different cells
  (`/private/var/folders/5x/07fl6yc568q8ts` / `21cbcdcny00000gn/T/…` against the oracle's `/private/var/` /
  `folders/5x/07fl6yc568q8ts21cbcdcny0000` / `0gn/T/…`): the details wrap of a path with no space, not a keymap
  row. Stale cells in my small VT reader (a leftover `t` and `Ready`) are not counted: the same screens in the
  tester are clean.
- Mouse: the pty run has no pointer; the clicks are `tests/pages/agents_mouse.rs` (green).

## Other changes outside the page

- `src/app/view.rs`: the keymap provider; `core/src/app.rs`: `AppEv::Keymap`; `tests/common/mod.rs`: `connect`,
  `set_keymap`; `docs/rtwk-port/rtwk-gaps.md`: G77.
- Disk: the volume filled once during the run; I deleted my own regenerable copy `target/debug/codex-rtwk` (739 MB)
  and rebuilt it for the run above.
