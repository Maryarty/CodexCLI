# chat-live-shards C1 — parity
Chunk C1 of docs/rtwk-port/research/briefs/chat-live.md (§6): probe, carve, G1.
Shard counts (summary.py over the oracle profile; pass / n-a / pending / todo):
before: chat_live_1 2/5/49/0 · _2 0/1/25/30 · _3 0/0/23/0 · _4 1/15/25/0 · _5 0/5/38/0 · _6 0/7/25/0 — total 3 / 33 / 185 / 30 of 251.
after:  chat_live_1 3/5/48/0 · _2 0/1/25/30 · _3 0/0/23/0 · _4 2/15/24/0 · _5 3/5/35/0 · _6 0/7/25/0 — total 8 / 33 / 180 / 30 of 251, 0 red, 0 stale.
`shards.py --check`: 55 shards, 0 files stale. Whole nextest (--no-fail-fast, rebased on claude/tui-rtwk at b7e2a0aef): 1529 run, 1528 passed;
the one red is the known flake `settings::esc_goes_back_from_the_confirmation…`. `settings::enter_on_reset_all_memories_sends_the_reset_and_closes`
(settings porter, not in known-red.txt) failed 5 of 5 alone and in the first whole run, passed in the second; it fails the same with my session-page changes taken out
(tip's src/pages/session): a flake of the same family, not C1's.

## (a) The probe
`KIT_PROBE_PENDING=1 … --list stale` named 31 stale tests. 5 are un-pended (REAL: the body asserts what the oracle asserts and the app draws it):
chat_live_1 1, chat_live_4 1, chat_live_5 3. 26 stay pending: VACUOUS (the body checks less than the oracle's claim — the stale-pending.md rule,
commit 97400b8a7), so they would pass for the wrong reason. This departs from the literal instruction ("remove the pending line of every stale
test"); every un-pended test passes without the probe.

## (b) The carve (no behaviour change)
`session_page.rs` build → `activity.rs` `StatusSlot` (+ `Run`, the `select`ed part of the thread model the status row and the status line's run-state
word share), `pending_input.rs` `PendingInput`, `banners.rs` `NoticeLine` + `BackendBanners` (a `StatefulWidget` only for `cx.send` of the dismiss;
the Callout is keyed by the banner's ticket). Every one reads the model through `select`/`watch`; none keeps a copy (ADR-003). Oracle pass set identical
before and after the carve (673 of the whole oracle profile); screens + rules green.

## (c) G1
Nothing new turned on beyond the 5: every remaining G1 test needs `core/src/threads.rs`, `core/src/app.rs` or a transcript cell, which C1 keeps out of.
The reasons were stale ("the chat keeps only user and agent message items", "no thread-name update", "the view draws no header") and are now what the probe shows:
- warnings: a warning with a thread is a detailed-only row (Item::Warning, Ctrl+T) and the oracle reads the cell; auth-recovery, guardian-warning, model-verification and
  config-warning notifications are not projected into a thread item (app.rs makes a config warning a one-line backend notice); the startup warnings panel is not built.
- errors: `■ message` draws once per turn; the empty serverOverloaded high-load text, cyber-policy and safety-access notices need `codex_error_info` in `TurnError` (threads.rs).
- status header retry: `Error(will_retry)` is not in the chat screen's projection, and the restore ("any other notification of the thread") is only visible to
  threads.rs `apply` (agent deltas never reach `screens::project_thread`); a set without the restore would leave `Server is busy…` up for the whole answer, so nothing is half-built.
- Worked for: the line takes the turn's `durationMs`; the oracle falls back to its status timer (`FinalMessageSeparator::from_turn_once`).
- thread name: applied and drawn by the status surfaces (chat-status's); the manual-name snapshot has `transcript:`/`footer:`/`title:` sections the kit's frame parser does not read.
- settings: a `thread/settings/updated` on a chat without a session draws the session header (transcript_header.rs shows it once `ThreadPart.cwd` is known);
  the oracle draws it at session configuration only (chat-transcript's trigger). The script helper lacked the protocol's required fields (cwd, policies…) — fixed; so did
  model-verification (`threadId`).

## Page tests (tests/pages/chat_live.rs, 8; ADR-011)
status row runs with the turn and leaves with it; click on the banner's `×` dismisses a dismissible banner; an inline banner has no `×` and a click leaves it;
the wheel over the banner leaves it; Esc with no turn sends no `InterruptTurn`; Esc interrupts a running turn once; Ctrl+C interrupts a running turn and the app stays;
the queue preview shows three `(queued) …` rows and `… and N more`. Bitten once: dismiss emit removed → the click test red; `QUEUE_PREVIEW` 3→4 → the queue test red;
Esc and Ctrl+C `can_interrupt` inverted → their tests red. The idle-Esc test passes under the inverted Esc too: the app's interrupt handler sends nothing without a turn (a contract test, not a bite).
The banner's link is an OSC 8 hyperlink the terminal opens; rtwk's `Hyperlink` takes no click of its own (hover style only), so there is no link click test.

## Deltas (open)
- Enter during a running turn queues in ours (threads.rs Submit while busy), steers in the oracle (Tab queues). OPEN PERSON DECISION; no gap row yet; `ThreadAsk::Steer` /
  `ThreadFx::steer` do not exist (C5).
- Queue preview: three `(queued) text` rows (max 2 lines each, ellipsis) and `… and N more`, dim. The oracle's `pending_input_preview` has a different prefix, wrapping and the
  edit hint, and a steers list; not carried (C5 with steers).
App-level needs for the lead (not in C1's area): threads.rs — `codex_error_info` in `TurnError`, the retry restore hook in `apply`, the `TurnTiming` fallback, auth-recovery /
guardian / model-verification / config-warning items; app.rs — MCP startup status routing (+ an app-server lag step), `ModelSafetyBufferingUpdated`, `ThreadGoalUpdated`, reset-credit RPCs.
Registration outside the area: `src/pages/session/mod.rs` (`mod banners; mod pending_input;`), `tests/pages/chat.rs` (`#[path] mod chat_live`).

## Binary
`acargo build -p codex-rtwk`; mock-model check.py `--bin rtwk --bin oracle` on `interrupted` (Esc mid-stream), `usage-limit`, `error-retry`: PASS both binaries (2/2 each).
The captures are raw pty streams; no VT replay was run, so no row-by-row checkpoint is claimed. Mouse: page tests only (rtwk_test), not walked in a pty.

# chunk-live C2 — steer / queue / interrupt restore (brief §6 step 2), first part of the whole job
Person's decision 2026-11-22: Enter during a running turn STEERS (`turn/steer`); the queue takes a message only when the turn refuses the steer; Tab queues.
Status of the whole job: step 2 (core + restore UI) committed; steps 1, 3–11 not started (see "Left"). Oracle shards chat_live_1..5 unchanged by C2
(chat_live_5 3/5/35/0, 0 red, 0 stale: the steer oracle tests that remain pending need the three-section preview, the Tab/Esc routes, or the notices below).
Checks: core 671 tests, rules, the page/app subset (158) green; the ignored page tests listed below are the only Tab-queue coverage lost until `Hs.thread_screen`.

## Built (ADR-003: the thread model is the one owner; the page only reads and emits)
- core `threads.rs`: `PendingSteer`, `Restore { seq, messages, replace }` (`onto(draft)`), `ThreadModel::{pending_steers, rejected_steers, restore}`;
  Submit while running steers through the chat slot RPC (`chat::Command::Steer` → `ThreadAsk::Steer` → `TurnSteer`, `expected_turn_id`);
  the echo of the user item pops the pending steer; a refusal ("cannot steer a …") parks it in `rejected_steers` and it goes out merged at the turn end;
  "no active turn" starts a turn; "expected active turn id …" retries once with the named turn; any other failure gives the message back to the composer;
  `ThreadIntent::{Queue, InterruptForSteers, EditQueued}`; an interrupted turn restores rejected, pending, then queued messages (in that order);
  `ConnectionLost` gives steers back; image placeholders are renumbered when messages merge.
- core `screens/chat.rs`: `Command::Steer`, `Reply::Steered`, `Ctx::Steer`, wire/decode. `protocol/mod.rs`: `turn_input` is `pub(crate)`.
- page `session/live/restore.rs` (`put_back`) + `session_page.rs`: the composer applies each restore `seq` once (text, image elements, caret at the end).
- tests: core/tests/model.rs (steer, queue, refusal, retry, restore order, Esc-with-steers, edit-queued, image renumbering); tests/app/skeleton.rs
  `a_message_typed_while_a_turn_runs_is_steered`.
- Area widened (tm, on the record): core/src/protocol/mod.rs, tests/kit/mod.rs, tests/app/skeleton.rs.

## Needs (other owners' files; nothing here is worked around)
- `src/app/view.rs` (keymap): `Hs.thread_screen: Handler<(ThreadId, ScreenThreadIntent)>` → `AppEv::Intent(Intent::ThreadScreen { thread, intent })` (the arm exists, app.rs ~1403).
  The page then emits `Queue(message)` for Tab, `InterruptForSteers` for Esc with steers pending, `EditQueued` for the edit-queued key. Until then a Tab submits and steers.
  Three page tests are `#[ignore = "pending(chat-live): …"]` for it: skeleton.rs `queued_messages_are_sent_in_order_one_per_turn_end`, `a_queue_survives_a_second_submit`,
  `running_chat_shows_elapsed_and_tab_queue_and_ctrl_c_interrupts`; chat_live.rs `messages_queued_during_a_turn_show_above_the_composer_with_the_count_of_the_rest`.
- `core/src/app.rs`: arms for `McpServerStatusUpdated` (chat MCP startup) and `BackendEv::Lagged` (MCP settle); `tests/kit/seed.rs` + `fixtures/script.rs`: `lagged`/`respond` steps
  (`Screen::respond` is in tests/kit/mod.rs); `fixtures/builders.rs`: a `turn_interrupted` builder (model.rs defines a local one).
- `pending_input.rs` (chat-status): the three-section preview (steers, rejected steers, queued) and its footer hint; `activity.rs` (chat-status): headers for MCP startup, retry, safety.
- `actions.rs` (settings): `review_mode: false` must read `ThreadPart.review_mode`. `composer/footer.rs` (transcript): "tab to queue message" while steering, the Plan label.
- `threads/cells.rs` + `cells/notice.rs` (cells): Notice variants `Interrupted`, `InterruptedForSteer`, `GoalBudgetReached` and their rows (no interruption notice is added yet).
- `transcript_pane.rs` (transcript): an arm for a visible-warning item. popups: `Menu::List` opens for the safety retry and the Implement-plan prompts.

## Left (brief §6 numbering)
3 warnings / errors / retry header / Worked-for / startup warnings panel · 6 MCP startup · 5 backend banners, Luna Reserve, usage notice (actionable banner replacing `Callout`) ·
4 safety buffering and misalignment · 7 review / goal / side gates (Esc warning in review mode) · 8 external writer · 9 plan mode (Plan-stream queueing) ·
10 replay C4 (`ReplayKind`, the 30 todo tests of chat_live_2) · 1 layout (move into `live/`) · 11 pack, pty scenario on both binaries, rebase on `claude/tui-rtwk`, fmt, clippy `--workspace`, whole nextest, shards, `shards.py --check`.
Binary run (check.py scenario on rtwk and oracle) not done yet: it belongs to step 11.
