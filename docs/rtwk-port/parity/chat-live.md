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

# chunk-live C3 — MCP startup, retry header, review state, keymap-landed needs (brief §6 steps 6, 3 part, 7 part; Tab/Esc/edit-queued wiring)
Branch worktree-agent-ae321221c34cc050b, rebased on claude/tui-rtwk 0eb53b241. Oracle chat_live_1..5 (pass / n/a / pending / todo / red / stale), before the chunk's C2 baseline where I have it:
chat_live_1 5/5/46/0/0/0 · chat_live_2 0/1/25/30/0/0 · chat_live_3 15/0/8/0/0/0 (was 4 pass) · chat_live_4 2/15/24/0/0/0 · chat_live_5 10/5/28/0/0/0 (was 3 pass at C2, 4 at the start of C3).
TOTAL 15 -> 32 pass, 0 red, 0 stale. `shards.py --check`: 0 files stale.

## Built (ADR-003: core owns the state, one writer; the page reads and emits)
- Typed values in core, the words in `session/strings.rs` (rules `core_sources_hold_no_user_visible_prose`, ADR-010): `McpWarning`, `McpHeader`, `StatusHeader`/`Retry` (core `screens/chat.rs`, `chat/mcp.rs`), `Fault::{Message, HighLoad}` and `Item::{StartupWarning(McpWarning), HighLoad}` (`threads.rs`).
  The view words them: `Strings::{status_header, mcp_warning, high_load}`; `activity.rs` takes the header from `run.chat.header()`, `transcript_pane.rs` has the two arms.
- MCP startup round (oracle `chatwidget/mcp_startup.rs`): `chat::McpStartup` (round, header, failure dedup, summary Interrupted/Incomplete, lag settle), `ThreadEv::{Mcp, McpLagged}`; a submit while a server starts is held and goes out when the round settles (`threads.rs`); thread-less status updates apply to the displayed thread (`project_unrouted`).
- Retry header (`ThreadEv::Retry`, a 429 reconnect is `Retry::Busy`), cleared by the thread's next notification; review mode is the thread's state (`note_review`), read by the slash gates (`actions.rs` `review_mode` line).
- keymap files handed over by the lander: `Hs.thread_screen` (view.rs, + the debug `seeded.rs` Hs), Tab queues behind a running turn (`Delivery::Queue` in `run_line`), Esc with steers waiting emits `InterruptForSteers`, the edit-queued key is the keymap's `chat.edit_queued_message` (`KeyBindings` from `keymap::bindings`), `BackendEv::Lagged` also settles the thread's MCP round (app.rs, one hunk), `lagged` script step (`fixtures/script.rs`, `kit/seed.rs`).
- Removed a duplicated intent arm pair in `threads.rs` (an earlier edit script had inserted it twice).
- Tests: core/tests/model.rs (MCP header/long list/failure once/cancelled order/outcome without a start/held submit/lag/no-words failure/retry header/high load), tests/pages/chat_live.rs `the_edit_queued_key_puts_the_newest_queued_message_back_in_the_composer` (shown biting: with the key changed to alt+down it fails), the four Tab tests un-ignored, 17 oracle tests un-pended (6 lag, 5 MCP-while-queued, 6 steer/restore/review). `late_failure_does_not_reopen_the_next_round` now asserts the oracle's no-summary rows (`assert_same_rows`); its old `assert_rows_differ` contradicted the oracle.

## Needs (other owners' files; none worked around)
- chunk-transcript, `session_page.rs` (mine): wrap the page body in the transcript's `ModeKeys` and forward Esc / OpenTranscript / FindTranscript to the mode controller, drop `detailed_transcript` and `esc_primed`. NOT done: the controller is not on claude/tui-rtwk yet; do it when it lands (text: parity/transcript.md "needs", branch worktree-agent-a78257323a3aeb7ab).
- chunk-transcript also lists for me: the Revert command in `ThreadFx` and the `ThreadReverted` apply arm (threads.rs), `UserPrompt` keeping images and placeholders: not started (backtrack).
- chat-status (`activity.rs`, `pending_input.rs`, footer): the `header: String` hunk and `session_strings.status_header` line in `activity.rs` (kept: it is what draws the MCP and retry headers); the three-section pending preview; footer/status-line rows of `mcp_startup_header_booting_snapshot` and `app_server_mcp_startup_failure_renders_warning_history` (both re-pended with that reason).
- transcript (`transcript_pane.rs`): the `StartupWarning`/`HighLoad` arms (kept), and the oracle's startup warnings touch (no blank separator between two consecutive startup warnings).
- cells: `Notice` variants Interrupted, InterruptedForSteer, GoalBudgetReached and rows; warning/error cell wording; the user-message echo rule for the review prompt; the oracle cause lines (cyber-policy, safety-access, upstream-message variants need `codex_error_info`: `Fault` is where they go).
- popups: `Menu::List` opens for the safety retry and Implement-plan prompts. chat-commands: command notices as "■" error rows (side-gate tests).
- kit: a `respond` step (answering a server request) and a foreign-writer feed step (step 8) are not built; no test I un-pended needs them.

## Left (brief §6 numbering; each is its own piece of work, none started)
5 backend banners / Luna Reserve / usage notice (the actionable banner replacing `Callout`) · 4 safety buffering and misalignment · 3 remainder (Worked-for line, startup warnings panel, warning/error wording through cells) · 7 remainder (goal/budget notices, Esc warning with review-queued steers, review steer notice) · 8 external writer · 9 plan mode · 10 replay C4 (`ReplayKind`, the 30 todo tests of chat_live_2) · 1 layout into `live/` (activity.rs / pending_input.rs are chat-status's).

## Checks and binary run (C3)
- fmt `--check` clean; clippy `--workspace --all-targets -D warnings` clean (the one warning is the backend pin's `bail!`); whole nextest 1722 passed, 15 skipped, 0 failed; `shards.py --check` 0 stale.
- A core test shown biting once: `HEADER_SERVERS` 3 -> 2 fails `mcp_startup_header_cuts_a_long_list_after_three_servers`; reverted. The edit-queued page test fails with the key changed (see Built).
- Binary (`check.py --bin rtwk --bin oracle`, capture /tmp/chunk-live-cap): `interrupted` PASS on both (a single Esc 4 s after the request, the stream disconnected in 0.42 s on rtwk, 0.46 s on the oracle, the next prompt answered); `mcp-tool` PASS on both (the rmcp server boots, two tool calls, sentinel on screen).
  The scenarios cannot press Tab or the edit-queued key (those are covered by the pages/app tests above); the captures are raw ANSI streams, not rendered rows, so a row-by-row diff was not made.
- Open delta seen in the captures (not investigated, not chat-live's file): during turn 1 of `mcp-tool` the oracle's footer reads `⚠ 1 warning · f2 to view`; the rtwk capture has no warning count at that checkpoint (footer = chat-status; the cause may be the oracle's model-metadata warning notification).
- The claim for this task was reported "already claimed" by `task_start` (held by this worktree's own earlier run); I continued under it.

# chunk-live C5 — ThreadAsk::ReadTurns/Revert and the backend banner (brief §6 step 5, first part)

## Built (ADR-003: asks only in `threads.rs`; the page reads and emits)
- STEP 0, for chunk-transcript-2's backtrack: `ThreadAsk::ReadTurns{thread}` and `ThreadAsk::Revert{thread, before_turn}` in `core/src/threads.rs` (the only place that names `ThreadRevertParams`; the layers scan lists it), `ThreadEv::Reverted` from `ServerNotification::ThreadReverted` clears the transcript, history and finished set and `app.rs` re-reads the retained turns (`ThreadAsk::History`). Core tests: `the_prompt_edit_reads_the_turns_then_reverts_before_one`, `a_reverted_thread_clears_its_transcript_and_reads_the_retained_turns`. `backtrack.rs` not touched.
- Backend banner = the oracle's actionable banner (`bottom_pane/actionable_banner.rs`), `Callout` gone: blank row, bold title + description wrapped at width-4 two cells in (more than 8 rows keep 7 and a dim `…`), numbered rows (`Options` under `IntrinsicHeight`, cursor `› `), the hint (`Press a number to choose`, `· esc to dismiss`, `· type to continue`; information-only banners have the Esc hint only). Words in `pages/session/strings.rs`.
- Keys on the page's composer `FocusNode` handler (`session_page.rs`), the oracle's `handle_inline_banner_key` gate: composer empty, no paste burst, no modifier, no running turn; Esc only for a dismissible banner and not while the shortcut overlay is open; a digit with no row of its own is the composer's. `BannerGate` shares what the banner shows (keys) and its `Options` cursor with the handler; choosing moves the cursor and does not dismiss.
- Core: `Banner::ctas()` (label rules, `BannerAction::{OpenUrl, NotifyOwner(Nudge), ResetUsage}` as `resolve_action`), `Intent::PickBannerCta{backend, ticket, row}` -> `Command::OpenUrl{url}`; a stale ticket or a row past the end does nothing. `Hs.pick_banner` (view.rs + debug `seeded.rs`).
- Tests: `tests/pages/chat_live.rs` (rows and hint layout, number key picks and the banner stays, a number with a draft is text, Esc dismisses only dismissible, tap on a row picks, tap on the title inert, wheel inert), `core/tests/model.rs::choosing_a_banner_row_opens_its_page_and_keeps_the_banner`.

## Needs
- host/shell: an arm for `Command::OpenUrl` (the browser seam `Startup.browser`, `webbrowser::open`); the fake records it. NotifyOwner (the add-credits nudge RPC) and ResetUsage (usage-status's confirm flow) are typed in `BannerAction` and not handled.
- `{time}` in the banner copy: chat core has no clock facts (now, utc offset, hour24); the status card's `Limits.now/utc_offset_secs` and `ResetTime` are the model.
- chunk-transcript-2 landed: `ModeKeys` wrapper and Esc / Ctrl+T / F3 forwarding in `session_page.rs`, mount of `TranscriptSheet`, and the one-line `backtrack::apply_thread(.., Reverted)` in the `ThreadEv::Reverted` arm (neither the controller nor core `Backtrack` exist on this base).

## Left
Luna Reserve selector, automatic switch and usage notice (step 5 rest; the 12 `chat_live_2` banner tests stay pending: they need a seed for an upsell rate-limit response) · 4 · 8 · 9 · 10 · 1 · rest of 3 and 7 (see C3 Left).

## Checks (ac5e0427b)
fmt, clippy `--workspace --all-targets -D warnings` clean, whole nextest 1909 passed 17 skipped (one load flake, `importing_sends_the_checked_items_and_leaves_the_page_for_the_chat`, passes alone and on rerun), `shards.py --check` 0 stale. Binary run of the banner not done (no check.py scenario serves an upsell response).
