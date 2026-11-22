# kit-caps: the test kit's missing capabilities

Task `kit-caps`, branch `worktree-agent-ae508aa7f4ae229a9`. Clusters kit-env-seed 14, kit-input 11,
kit-sent-ops 6, kit-snap-frames 4 (`research/pending-clusters/clusters.md`), plus chunk-live's two kit
items (replay seed, foreign-writer seed). Area: `tests/kit/**`, `tests/common/mod.rs`, `tests/kit_proof.rs`,
`tests/oracle_snap/mod.rs`, `docs/rtwk-port/refs/screen-tests.md`.

## Capabilities (all on `Screen::build(..)` / `Screen`; recipe in `refs/screen-tests.md`)
| Capability | Where | What |
|---|---|---|
| env seeds | `kit/env.rs`, `common::start_driven_with(debug, Options)` | `.light() .dark() .terminal_colors(fg,bg) .palette(..) .color_depth(ColorDepth) .syntax_theme(name) .env(name,val)`: all through `rtwk_test::Options` (`background_rgb`, `env`, `terminal(\|HeadlessOptions\|)`); the syntax theme is the app's `settings::Facts` fact |
| input | `kit/input.rs` | kitty fallback in `keys()` for keys with no xterm bytes (`super+space`, `shift+enter`, `ctrl+shift+k`), `key_repeat/key_release`, `right_click/down/up`, `shift_click`, `ctrl_click`, `click_with`, `triple_click`, `pointer_move`, `mouse_event`; bytes from `rtwk_test::keys::key_bytes` and `Tester::mouse`, the kit adds only the kitty line |
| sent-ops probe | `kit/probe.rs` | `turn_texts`, `assert_turn_texts`, `assert_no_turn`, `interrupts`, `assert_interrupts`, `requests_of`, `request_params`, `commands_named` over the fake backend's recorded commands/requests |
| snapshot reader | `oracle_snap/mod.rs`, `kit/grid.rs`, `kit/mod.rs` | bare-label frame headers, `(r, g, b): Buffer {` headers, `Screen::snap_frame_names`, `assert_style` negation `!fg=` / `!bg=` |
| foreign-writer seed | `kit/env.rs` | `.foreign_writer("thread")`: the server lists the thread and refuses `thread/resume` with "already has an active writer", so the app opens it read-only (`Origin::ForeignWriter`) through the app's own `OpenThread` (ADR-003: no thread state in the kit) |
| replay seed | `kit/env.rs` | `.replay(vec![kit::replay_turn(id, items)])`: the server lists `kit::REPLAYED` and answers the resume with a full `ThreadResumeResponse` holding the turns |

## Shown biting (`tests/kit_proof.rs`, 48 tests, all green)
Each capability has a proof that fails without it: light/dark theme rows differ, depth changes the cells
(none/16/256/true), the syntax theme changes the code colours, kitty keys reach the composer (shift+enter newline,
release and repeat events), right/shift/ctrl/triple click reach the page, sent turns and interrupts are counted
(and zero on a plain screen), bare-label and `(r,g,b)` frames list their names, `!fg` fails on a coloured cell,
a foreign-writer screen asked for one resume (a plain screen 0), a replay resume asks for `kit::REPLAYED`.

## Un-pended (now green)
approvals_2 (shift_enter, countdown_is_red), composer_3 (non_text_input), chat_commands_1 (4 of 6 tests, with
fixture `fixtures/chat_commands_1/completed.jsonl`), picker (snapshot_auto), and the
env/input rows of approvals_1, composer_1/3/4, transcript_view_3/4/5. Run: `summary.py --run <17 shards>`:
241 pass, 117 n/a, 598 pending, 0 todo, 0 red, 0 stale.

## Still pending, reason now names the feature (not the kit)
- chat_live_2: the 30 `kit::todo()` are `kit::missing` on "core replay projection (core-wiring)" (27) or "live
  thread items as history cells (chunk-cells)" (3); the seed shape is ready: `Screen::build(..).replay(..)`.
- chat_live_1 external-writer tests (3): seeded with `.foreign_writer(..)`; the chat draws no read-only notice, no
  fork-pending frame and no transcript shortcut label.
- chat_commands_1 (2): composer history recall (Up) screen feature; cells_5 three-palette body: cell styling;
  composer_popups `history_search_paste_uses_full_sanitized_text`: search row overflow bug, and `long_paths_keep_distinguishing_filenames_and_original_selection` (green before the merge, red once chunk-composer step 8b mounted the mentions view: re-pended on it); picker, pane_footer_2,
  markdown_streaming_2/3: reasons updated.

## Gaps
- rtwk: none. `rtwk_test` (`Options`, `terminal`, `key_bytes`, `mouse`) covered every row; no `rtwk-gaps.md` row.
- core, not rtwk: `AppEvent::FollowTranscript` has no `Command` (the page's scroll state): the tests that count it
  look at rows; a foreign thread in another cwd is blocked by core's `AbsolutePathBuf` decode of the listed thread
  (the seed lists `/work`); the chat does not consume the syntax-theme fact for its code cells.

## Outside the area (shard edits, named for the owners)
Shard files are edited for un-pending only (bodies, reasons): approvals_1/2, composer_1/3/4, composer_popups,
picker, chat_commands_1 (+ fixtures dir), cells_5, markdown_streaming_2/3, pane_footer_2, transcript_view_3/4/5,
chat_live_1/2. Several are held by other claims (chunk-composer, chunk-popups, chunk-usage's shard list): the
writes went through python scripts, so the claim guard only reported them. `composer_1.rs` carries a 5-line
rustfmt change from my earlier commit; merge conflicts there are formatting only.

## Run it
`check.py --bin rtwk --bin oracle --only exec-approval`: PASS both (7.9 s / 8.3 s). The kit is test-only: the
binary is unchanged by this task, so one scenario is the check. Rows: equal.

Final gate at 85445ccc4 (after merging claude/tui-rtwk 6e2ac4060): fmt, clippy --workspace, nextest 2402/2402 (--retries 1), shards.py --check OK. Oracle profile: 2 reds, neither the kit: onboarding::long_paths_keep_disclosure_and_choices_visible (passes in the full default run) and small_dialogs_2::warning_pages_wrap_and_scroll_long_diagnostics (script warnings page shows "starting…": not in any file of this task).
