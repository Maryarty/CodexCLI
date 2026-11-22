# Brief: /status card (screen map C10, tm alias `status-card`, S19)

Recon 2026-11-22 (finished by recon-usage-status, read-only). Paths: `codex-rs/tui-rtwk` (ours), `codex-rs/tui/src` (oracle),
`app-server-protocol/src/protocol/v2` (protocol). Porter worktree: `.claude/worktrees/agent-a20166eef4c65167b` (branch base 21d470500).

## 0. First step for the porter
- Chunk 1 is mostly done in the running porter's worktree (below). Commit the uncommitted work, rebase onto the integration
  tip (b7e2a0aef or later), then run the `status` shard with `KIT_PROBE_PENDING=1` and `tests/oracle/tools/summary.py --list stale`.
- What the worktree holds at recon time:
  - committed `981ba47ae`: core `core/src/screens/status/{card.rs 311, helpers.rs 175, limits.rs 335}` (typed `StatusCard`, `Limits`/`LimitData`,
    `Credits`, `SpendLimit`, window names, plan labels, agents summary, permissions label, reset times) + `core/tests/status.rs` (33 tests incl. the 7 n/a helpers).
  - uncommitted: `src/pages/session/cells/status/cell.rs` (469, `StatusCell` stateless widget: `LayoutBuilder` + `Grid` content|fr, usage link via
    `Span::link`, window bar), `strings.rs` (+453), `mod.rs` (32 `cell_entry!` debug entries, `EntryOpen::Route(cell)`; `card_dialog` entry keeps the old dialog),
    33 fixtures in `cells/status/fixtures/`, `tests/oracle/status.rs` (25 tests unpended).
- Stale pendings to probe in other shards once the live cell lands (reasons are out of date: "/status is not dispatched"; ours dispatches, as a dialog):
  `chat_status_2.rs:626-795` (11 pending), `chat_live_6.rs:401,418,433` (3 pending).
- The debt task the porter's code names, `status-card-adr003`, does NOT exist in tm (get_task: not found): the orchestrator files it (ADR-003 departure:
  `ScreenStates.status.card` and `.cell` are copies of thread facts outside the thread) before chunk 1 lands, or chunk 1 lands with step 3 below.
- Missing core test: `format_directory_display_truncates_halfwidth_sound_marks` (n/a in the shard, not yet in `core/tests/status.rs`); add it with the 3
  `status/thread_usage_tests.rs` tests in step 5.

## 1. Scope
Oracle: `status/card.rs` (79-146 `StatusHistoryHandle`: `copy_text`, `finish_rate_limit_refresh`, `set_thread_usage`; 150-430 constructors from Config;
757-972 `content_lines(width)`; 982-1000 `display_hyperlink_lines` (the usage URL as OSC 8)), `status/rate_limits.rs` (550), `status/helpers.rs` (351),
`status/format.rs`, `status/account.rs`, `status/thread_usage.rs` (356), `status/remote_connection.rs` (1-60 `remote_connection_status_value`),
`chatwidget/status_controls.rs:212-330` (`add_status_output(refreshing, request_id)`, copy targets 279-296, `finish_status_rate_limit_refresh`),
`chatwidget/thread_usage.rs` (146 `request_thread_usage_for_status`, 181 `finish_thread_usage_refresh`, 331 `request_thread_usage`), `chatwidget/copy_picker.rs:22-35`.
User sees: `/status` appends a transcript cell (magenta `/status`, blank, `>_ OpenAI Codex (vX)`, usage link, label|value rows, limit bars, thread usage
block); it is live after commit: a rate-limit refresh and the thread-usage estimate fill only that cell; `/copy` offers "Whole status" + fields.

## 2. Tests (in-chunk total 34 + 12 + 3 = 49 screen tests, + core tests)
- `tests/oracle/status.rs` 34: worktree state 25 unpended (porter's run decides pass), 2 pending, 7 n/a (ported as core tests).
  - pending `status_snapshot_wraps_halfwidth_kana_in_narrow_terminal` (42 cols): build the entry `halfwidth_kana_narrow`; expect G44
    (halfwidth sound mark is one grapheme in rtwk) -> `pending(G44)` with the probe's diff, not "no entry".
  - pending `transcript_overlay_remeasures_status_after_rate_limit_refresh` (80x30, frames before/after): needs the pager / transcript overlay
    (transcript chunk, tm `pager`): stays pending, reason names the pager task.
- `tests/oracle/chat_status_2.rs:620-795` `status_command_*` 12 (1 n/a, 11 pending): in this chunk (live cell, refresh with request id, overlapping
  refreshes, catalog default reasoning, instruction sources, thread usage rows, snaps `status_command_estimated_thread_usage(_without_usd)`).
  File owned by chat-status: these 12 functions only (orchestrator gives the porter those functions, or the chat-status porter leaves them alone).
- `tests/oracle/chat_live_6.rs:395-450` 3 pending (`status_history_survives_exhausted_billing_retries`, `..._updates_again_after_billing_settles`,
  `billing_surfaces_render_for_every_supported_enterprise_plan`) + 2 n/a (frame scheduling): in this chunk (thread usage). File owned by chat-live.
- Not this chunk: `chat_commands_2.rs:522-620` `slash_copy_*status*` 4 (the `/copy` picker: chat-commands / transcript chunk); this chunk supplies the
  copy data (step 4). `chat_status_3.rs:145,223` thread-usage items in status-line/title setup: chat-status.
- Own tests: `tests/pages/status.rs` (cell), `core/tests/status.rs` (33 now), `tests/kit_proof.rs:76,100` use `status` entries (keep names).

## 3. What exists in ours
- Main tip: `core/src/screens/status.rs` (271) dialog `Card` + `Model.card`; `core/src/app/session.rs:31-67` `open_status` builds `Card` from
  `StatusFacts` (`core/src/screens/chat.rs:55-90`), `ThreadPart` (tokens, context), `session.rate_limits` (`core/src/app.rs:82`, codex bucket only);
  `refresh_rates` (`session.rs:20-29`, `AppCtx::RateLimits{backend}`, no request id). View `src/pages/session/cells/status/card.rs` `CardDialog`
  via `show_dialog` (`mod.rs` `open`, `src/pages/session/open.rs:51`).
- Dispatch: `core/src/commands/dispatch.rs:355` `Status => open(Target::Status, ..)` (a route/sheet open, oracle: a history cell);
  `src/pages/session/actions.rs:136-142` emits `ScreenIntent::Status(Intent::Open)`.
- Transcript: `core/src/threads.rs:78-130` `enum Item` (no Status); `src/pages/session/transcript_pane.rs:149-300` maps Item -> cell widget.
- Protocol data, missing pieces and where they enter:
  - Rate limits: `core/src/protocol/mod.rs:95-110` keeps the `codex` bucket's primary/secondary; drop of `limit_name`, `credits{has_credits,unlimited,balance}`,
    `individual_limit` (spend), other buckets, `plan_type`, `rate_limit_reset_credits` (`GetAccountRateLimitsResponse`, v2/account.rs:369, 749). Also the
    notification path `core/src/app.rs:798 install_rates`. Enter: `status::RateLimits` -> `status::Snapshot` (worktree `limits.rs:48`) for all buckets.
  - Account: `core/src/protocol/agents.rs:425-445` `account_ev` keeps plan + backend auth (drops email, workspace name/id): extend to `status::Account`.
  - Thread facts: name + `forked_from_id` (`Thread`, v2/thread_data.rs:205,220), `instruction_sources` (`ThreadStartResponse`, v2/thread.rs:190-206) and the
    collaboration mode: on the ThreadModel (`core/src/threads.rs`), read when the card is built.
  - Thread usage: `ClientRequest::GetAccountTokenUsage` = `account/usage/read {threadId}` (common.rs:1449; the oracle's `RefreshThreadUsage`,
    `app/event_dispatch.rs:2074`) -> `GetAccountTokenUsageResponse.thread_usage: Option<ThreadUsage>` (v2/account.rs:478-484). Ours already wires it for
    the agents page: `core/src/protocol/agents.rs:90-95,187-193` (`Command::Usage{thread}` -> `UsageOutcome::{Available,Disabled}`,
    `core/src/screens/agents/usage.rs:21`). The chat has no call (chat-status brief 3: "no thread-usage RPC").
  - Server row: host knows the target (`host/src/in_process.rs` embedded / local daemon / remote / Cloud) and `remote.server_version()` (`in_process.rs:561-571`
    `emit_version`, sent only as the agents page's `Ev::ServerVersion` notice text). Core gets no connection fact: the card's `Remote{address, version,
    local_daemon}` (worktree `card.rs:17`) has no live source. Oracle value: `remote_connection_status_value(target, server_version)`
    (`status/remote_connection.rs:22-58`: Embedded -> none; Cloud -> `Cloud` + raw version; daemon/remote -> sanitized ws URL or `unix://path`, `v{version}` or `unknown`).

## 4. Flutter -> rtwk
- Cell in a chat list = a stateless `ListTile`-like item fed an immutable value: `StatusCell { card: Arc<StatusCard> }` (worktree `cell.rs`), registered in
  `transcript_pane.rs` like `Item::Notice` -> `cells::notice`. A refresh REPLACES the item (upsert by key) and the list rebuilds one row: no mutable handle.
- Label | value table: `Table` with `IntrinsicColumnWidth` = rtwk `Grid` `GridTrack::content()` + `GridTrack::fr(1)` (rtwk-widgets/src/layout/grid.rs), in a
  `LayoutBuilder` that picks the spelling (bar vs percent only, inline vs wrapped reset) from the measured width: the composition rule, not a workaround.
- Limit bar: `CustomPaint` + `CellPainter::fill_char` (G40 "not a gap", rtwk-widgets/src/layout/custom_paint.rs).
- Usage link: `Span::link(url)` (rtwk-widgets/src/style/style.rs:413; OSC 8 written by rtwk-core/src/line_encoder.rs:203), as the sign-in page does
  (`src/pages/startup/onboarding/signin/mod.rs:878`). Already in worktree `cell.rs:136`.
- Copy: `Clipboard.setData` with a model-derived string = a pure core fn over `StatusCard` (not painted cells): see step 4.
- Debug entry without a frame: `EntryOpen::Route` (`src/app/debug/mod.rs:36`), as the worktree does.

## 5. Area and conflicts (tm active claims 2026-11-22)
- Own (claimed): `core/src/screens/status.rs`, `core/src/screens/status/`, `core/tests/status.rs`, `src/pages/session/cells/status/`, `tests/oracle/status.rs`,
  `tests/pages/status.rs`, `docs/rtwk-port/parity/status-card.md`.
- Widen for steps 3-5 (free now): `core/src/threads.rs` (Item::Status, thread name/fork/instruction sources), `core/src/app/session.rs` (open_status,
  refresh_rates origin), `core/src/protocol/mod.rs` (rate-limit decode), `core/src/protocol/agents.rs:415-445` (account decode),
  `core/src/commands/dispatch.rs:355` (one line), `core/src/commands.rs` (a `SessionIntent::Status` variant).
- Held by others, minimal changes as needs:
  - `core/src/app.rs` (keymap porter): `AppCtx::RateLimits{backend, origin}` with `enum RateLimitOrigin { Status(u64), UsageMenu, ResetPicker(u64), Startup }`
    (the oracle's `RateLimitRefreshOrigin`; usage chunk uses the last three), a `connection: Option<status::Remote>` fact on the app, `install_rates` keeps
    full snapshots. Sequence after keymap lands, or the orchestrator grants these lines.
  - `host/src/in_process.rs` (keymap porter): emit the connection fact next to `emit_version` (`:561`) for every target, incl. Embedded (none) and Cloud.
  - `src/pages/session/transcript_pane.rs` (transcript porter): one arm `Item::Status(card) => cell(StatusCell{card})`.
  - `src/pages/session/actions.rs:136-142` (settings porter): drop `Target::Status` from the Open branch once /status is a session intent.
  - `tests/oracle/chat_status_2.rs` (chat-status) and `tests/oracle/chat_live_6.rs` (chat-live): only the functions named in section 2.
- Shared with the usage chunk: rate-limit decode + origins (`protocol/mod.rs`, `session.rs`, `app.rs`): this chunk builds them (step 3); the usage porter's
  reset flow consumes `RateLimitOrigin::{UsageMenu, ResetPicker, Startup}` and `rate_limit_reset_credits` and lands after it.

## 6. The plan (one job)
1. Finish chunk 1 (worktree): commit, rebase, probe; `halfwidth_kana_narrow` entry -> `pending(G44)` if the probe shows the one-cell shift; snaps green for the
   other 25. Reads `status/tests.rs` (fixture values are already in the 33 fixtures).
2. Tests first for the live cell: `tests/pages/status.rs` live path (`/status` typed in the chat seed -> the cell in the transcript at the bottom, no dialog);
   then unpend `chat_status_2::status_command_renders_immediately_*`, `..._uses_catalog_default_reasoning_*`, `..._native_and_foreign_instruction_sources`.
3. Live /status as a cell (ADR-003): `SessionIntent::Status` (dispatch.rs:355) -> `session_intent` builds `StatusCard` from config facts + ThreadModel +
   `session.rate_limits` + account + connection, appends `Item::Status(Arc<StatusCard>)` to the ACTIVE thread through the thread's apply (minted `ItemKey`, like
   `PlanUpdate`, threads.rs:303); refresh with `RateLimitOrigin::Status(request)`; the reply replaces only the items of that request (overlapping refreshes
   test, stale reply dropped). Remove the dialog `Card`, `Model.card`, `CardDialog`, the `card_dialog` entry, `Intent::{Open,Close}` and the
   `show_dialog` path; `Model.cell` stays a seed-only input of the debug entries (no live writer). This closes the ADR-003 departure (`status-card-adr003`). Decode full rate limits + account + thread facts
   here (section 3). Oracle lines: `status_controls.rs:212-330`, `card.rs:279-430`.
4. Copy data: core `StatusCard::copy_text()` = the card's lines at unbounded width, trimmed (oracle `card.rs:84-93`), and `copy_fields()` =
   Model, Directory (full cwd), Thread name (non-empty), Session ID (`status_controls.rs:279-296`); the transcript keeps "last status" by item key for the
   `/copy` picker (chat-commands/transcript build the picker; this chunk exports the data + a core test against `slash_copy_whole_status.snap`).
5. Thread usage rows: the thread's status slot (`ThreadPart`) gains `usage: Option<ThreadUsage>` + request state (`pending`, `next_request`, settlement
   request, retry count) as in `chatwidget/thread_usage.rs:39-60`; the call is the slot's RPC (`ThreadFx::request(status::Command::ThreadUsage)` ->
   `account/usage/read {threadId}`, decode as agents.rs:187-193; share the decode fn, do not copy it). Rows per `status/thread_usage.rs` (Thread usage,
   Models, Reasoning, Speed, Billed tokens; credits `~$`), label width reserved before the estimate arrives. Unpend `chat_status_2:728-795` (4) and
   `chat_live_6:401-450` (3); core tests from `status/thread_usage_tests.rs` (3). The status line's thread-usage item (chat-status) reads the same slot.
6. Server row: connection fact (needs section 5) -> `Remote`; entries `remote_server` / `local_background_server` already exist; live test with a fake host target.
7. Final: shard `status` 32 live + 2 pending (G44, pager), chat_status_2 12 and chat_live_6 3 green or pending on an open row; parity pack
   `docs/rtwk-port/parity/status-card.md`; pty run of `/status` beside the oracle (scenario per process.md §A4.3).

## 7. Gaps, risks, decisions
- G44 (halfwidth katakana cluster) blocks the 42-column kana snap: existing row, `pending(G44)`.
- Pager overlay snap waits for the pager task (transcript chunk); one test.
- Transcript separator: the cell snaps start at the `/status` row; the debug entry shows the cell alone (Route), the live path is judged by `status_command_*`.
- Local time for reset stamps: pass `now` + `ClockFormat` as facts (pure core); the worktree's `a_reset_is_read_in_the_local_time_zone` covers it.
- Thread usage polling (settlement after a turn, retry on billing errors) is timer-driven in the oracle: use the fake clock (`rtwk_test`) for the retry tests;
  the timer lives in the shell/ThreadFx, never in a widget.
- Decision (orchestrator, not the person): give the 12 `status_command_*` functions of chat_status_2 and the 3 billing functions of chat_live_6 to this
  porter. Recommendation: yes, they test this chunk's data path; the owners of those files skip them.
