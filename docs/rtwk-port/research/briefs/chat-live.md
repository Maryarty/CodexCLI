# Recon brief: chat-live (SessionPage live turn, screen map S10/C06, tm `chat-live-shards`)

Finished 2026-11-22 (recon-chat-live) over the C1 porter's commits f0d33b4fd and ba804df0a (worktree
`.claude/worktrees/agent-ac5b1e5d1a862e8d5`, on top of b7e2a0aef). Read-only recon: no cargo was run. Counts below are
read from the shard files at C1's tip.

## 0. First step for the porter
1. Start from C1's landed tip (or rebase onto it): C1 carved `session_page.rs` into `StatusSlot` (activity.rs),
   `PendingInput` (pending_input.rs), `NoticeLine` + `BackendBanners` (banners.rs), un-pended 5 tests and made the G1
   pending reasons true. Everything still pending in §2 is after C1.
2. Probe: `KIT_PROBE_PENDING=1` over `chat_live_1..6` + `tests/oracle/tools/summary.py --list stale`. Candidates from
   reading (reason vs current code):
   - chat_live_5 `ctrl_c_interrupts_without_arming_quit_when_double_press_disabled` ("turn/started is not applied to the
     bottom pane": Activity runs with the turn now; C1's page test `ctrl_c_interrupts_a_running_turn_and_keeps_the_app`).
   - chat_live_2 `compaction_status_survives_follow_up_and_preserves_turn_time`, `compaction_status_clears_when_turn_ends_without_item_complet`
     (reason "compaction header not built": activity.rs:139-141 draws `Compacting` from `ThreadPart::is_compacting`).
   - chat_live_1 `startup_config_warning_is_not_repeated_by_thread` (reason "no startup warnings panel": src/sheets/dialogs/warnings.rs exists now).
   - chat_live_5 side gates `slash_commands_without_side_flag_are_rejected_for_side_threads`, `slash_side_is_rejected_for_side_threads`
     (core/src/commands/dispatch.rs:311-312 gates already; the bodies lack a `screen` `chat` `Flags{side_conversation_active:true}` step: add it).
3. Decide nothing about Enter-while-running: §7 carries the person's decision; build to the oracle (steer) unless told otherwise.

## 1. Scope
Oracle files (all `codex-rs/tui/src/`):
- Working row and its headers: `status_indicator_widget.rs` 1-589, `chatwidget/status_state.rs` 1-196, `turn_lifecycle.rs` 1-114, `turn_runtime.rs` 1-625.
- Streaming and completion: `chatwidget/streaming.rs` 1-851, `completion.rs` 1-21, `compaction.rs` 1-57.
- Submit / steer / queue / interrupt / restore: `chatwidget/input_flow.rs` 35-120 (Enter vs Tab), `input_submission.rs` 250-600
  (pending steer :482-495, :596-599), `input_queue.rs` 1-195, `input_restore.rs` 253-690 (pop queued :253, rejected steer :357-389,
  interrupted turn :390-525, drain for restore :525-612), `interaction.rs` 285-340 (edit-queued key, review steer warning :301, Esc with
  pending steers), `interrupts.rs` 1-160, `app_server_session/user_message_submission.rs` 120-180 (`turn/steer`, not-steerable -> queued,
  turn-id race retry), `bottom_pane/pending_input_preview.rs` 1-250.
- Banners and notices: `chatwidget/backend_banners.rs` 1-527, `bottom_pane/actionable_banner.rs` 1-291, `luna_reserve_model.rs` 1-67,
  `usage_notice.rs` 1-194, `rate_limits.rs` 1-710 (the switch prompt and warning parts).
- Safety and policy: `chatwidget/safety_buffering.rs` 1-296, `misalignment_policy.rs` 1-248.
- MCP startup: `chatwidget/mcp_startup.rs` 1-226; lag: `app/app_server_events.rs` ~70-80.
- Warnings and errors: `chatwidget/warnings.rs` 1-157, `provider_error.rs` 1-177, `protocol.rs` 1-773 (error wording by `codex_error_info`).
- Review mode, goal, plan mode: `chatwidget/review.rs`, `goal_status.rs` 1-228, `plan_implementation.rs` 1-106; side gates `chatwidget/side.rs` 1-46.
- Replay: `chatwidget/replay.rs` 1-484 (`replay_thread_turns` :111, `replay_thread_item` :258, `ReplayKind`).
User sees: while a turn runs, the Working row (spinner, header, elapsed, `esc to interrupt`), the answer streaming, Enter sending a steer
(`Messages to be submitted after next tool call`) and Tab queuing (`Queued follow-up inputs`), Esc/Ctrl+C interrupting with the right notice and
the drafts restored; banners with numbered actions (backend banner, Luna Reserve, safety retry), MCP startup header and warnings, review mode
banners, Plan mode's Implement-this-plan prompt, and a resumed thread replayed as it was.

## 2. Tests (251 in chat_live_1..6: 8 pass, 33 n/a, 30 todo, 180 pending at C1's tip)
Per shard/group: count (P pending / NA / T todo / pass) -> chunk.
- chat_live_1 (56): input_queue 1 P -> here. pets 1 NA. rendering_tests 14 (11 P, 3 NA): external writer x5 + provider error 1 -> here;
  active cell x2 + clipped markdown hyperlinks 1 -> cells-markdown; initial session header 1 + owned live history 1 -> transcript.
  app_server 40 (3 pass, 1 NA, 36 P): safety x7, warnings x6, startup warnings panel 1, errors x4, retry header 1, Worked-for clears working 1
  -> here (20); command cells x2, sub-agent 1, collab x3, user-message cells x2 -> cells-markdown (8); session header catalog name 1,
  thread_settings_updated without transcript 1 -> transcript (2); session_and_settings_sync_server_provider_id 1, thread name x2 (status
  surfaces) -> chat-status (3); thread_name_suggestion (rename prompt) 1 -> chat-commands; tool_suggestion install URL 1, strict review notice 1
  -> chat-approvals-live (2).
- chat_live_2 (56): backend_banners 12 P, luna_reserve 4 P, compaction 4 P -> here. history_replay 24 T + history_projection 6 T -> here
  (C4: write each; cell-only failures pend on cells-markdown). completion_styling 2 (1 P, 1 NA) and list_spacing 1 P -> cells-markdown
  (final separator / streamed list reflow are history cells). config_errors 1 P -> popups (the /model save failure writes it).
  goal_validation 2 P -> chat-commands (`/goal`).
- chat_live_3 (23): mcp_startup 20 P, misalignment 3 P -> here.
- chat_live_4 (41): plan_mode 40 (1 pass, 15 NA, 24 P): 23 P -> here (Plan mode); vim_mode_default_enabled 1 -> composer. replay_render 1 pass.
- chat_live_5 (43): review_mode 33 (3 pass, 4 NA, 26 P): steer x10, interrupt notices x5, review banners/prompt x3, Esc/Ctrl+C x2,
  budget x2, interrupt restore x3 -> here (22 P); review commit picker, custom prompt escape, branch picker escape -> popups (3);
  interrupt_exec_marks_failed -> cells-markdown (1). side 10 (1 NA, 9 P): 5 gates/notice -> here; context label snapshots x3 -> chat-status
  (footer); side_aliases_request_forked_questions_while_task_running -> S22 sidebar/side panes (not in this batch).
- chat_live_6 (32): usage 26 (21 P, 5 NA) + thread_usage 5 (3 P, 2 NA) -> usage-status (reset-credits flow and /status thread billing;
  the batch list puts reset credits there). usage_notice 1 P -> here.
In chunk: 184 = 8 pass, 25 n/a, 30 todo, 121 pending. Out of chunk: 67 (usage-status 31, cells-markdown 15, chat-status 6,
transcript 4, popups 4, chat-commands 3, chat-approvals-live 2, composer 1, S22 1).
Widget groups in other chunks' shard files that test chat-live's widgets (decision §7, recommended here): chat_status_3.rs:372-500
status_indicator_widget 6 + effects 1; pane_footer_2.rs:810-1018 pending_input_preview 13 (12 P, 1 NA); pane_footer_2.rs:1019-1147
actionable_banner 6 P. With them: 210.

## 3. What exists in ours (C1 tip)
- src/pages/session/activity.rs:30-91 `Run` (select of ThreadModel: turn, can_interrupt, read_only, failure, chat ThreadPart, `word()`
  for the status line); :94-161 `StatusSlot` (Reconnecting / failure / Starting / `Activity` keyed (thread, turn)); :167-319 `Activity`
  (rtwk Spinner, shimmer by AnimationController+AnimatedBuilder, 1 s periodic, `esc to interrupt`, step line); :322 `compact_elapsed`.
  Missing: MCP startup header, retry header (`Error{will_retry}`), safety header, details line variants of status_indicator_widget.
- src/pages/session/pending_input.rs:21-60 `PendingInput`: first 3 queued as `queued(text)` with max_lines 2 + `more queued`. Missing:
  oracle layout (three sections: pending steers / rejected steers / queued, `• ` headers, `  ↳ ` prefix, `    …`, edit hint `⌥ + ↑ edit last
  queued message`, interrupt-key hint), steers at all.
- src/pages/session/banners.rs:25-53 `NoticeLine`; :55-135 `BackendBanners` draws `session.backend_banners` as rtwk `Callout` (boxed, Hyperlink
  actions, close button). Oracle draws the actionable banner unboxed: title, description, numbered options, `Press a number to choose · esc to
  dismiss · type to continue` (actionable_banner.rs:203-208). The Callout is a visual delta to replace; C1's page tests at
  tests/pages/chat_live.rs:95-135 (close-button click) change with it.
- src/pages/session/session_page.rs (668): build column :539-591 (transcript, StatusSlot, PendingInput, NoticeLine, panel, control,
  BackendBanners, composer, footer); Esc :354 (interrupt when `can_interrupt`), Ctrl+C interrupt-or-quit, Tab -> `QueueDraft` :268/:638,
  footer queue hint :501-502; read-only external writer :488-489 (external_writer.rs 1-86: notice, footer, r/f/q).
- core/src/screens/chat.rs (651): `ThreadPart` :310-324 (error, name, cwd, model, effort, provider, service_tier, daybreak, activity, compaction,
  progress, step); `project()` :422 (ThreadStarted, ThreadNameUpdated, ThreadSettingsUpdated, ReasoningSummaryPartAdded, ContextCompaction);
  `seed()` :605 (Flags incl. `side_conversation_active` :594, ServiceTiers, SignedIn, Status). chat/banner.rs 1-207 `Banner`/`BannerOccurrence`
  (parse :43, visible per model :82, reserve_pending :94, dismiss :167).
- core/src/threads.rs (1703): `queue: Vec<UserMessage>` :692; `ThreadEv::Submit` :1426-1441 queues while busy (R10); Interrupt :1443-1455;
  `send_next` :1654 (one per turn end); `ThreadAsk` :908 (Resume, ResumeAt, StartTurn, Interrupt...; no Steer); `project()` :1190-1280
  (`Error` only when `!will_retry` :1213); item mapping delegates to threads/activity.rs, cells.rs, collab.rs (:1340-1355).
  `Notice` enum threads/cells.rs:208-212 (ContextCompacted, EnteredReview, ExitedReview), drawn by src/pages/session/cells/notice.rs:33-39.
- core/src/app.rs: `AccountRateLimitsUpdated` -> rates + banner refresh :793-806; thread-less Warning / ConfigWarning / Deprecation ->
  `backend_notices` :811-827; `BackendEv::Lagged` :982-993 (no MCP settle); no `McpServerStatusUpdated` arm.
- src/pages/session/actions.rs:59-66 `DispatchCx.review_mode: false` (a session-core note in the code): the review state is not modelled.
- Kit: tests/kit/seed.rs feed() :197-235 takes seed/screen/notify/request/key/resize; no client-RPC answer step (the fake has
  `FakeHandle::respond`/`requests`, src/services/backend/fake.rs:249-281) and no `lagged` step (core/src/fixtures/script.rs:36 `Step`).
Protocol not consumed yet (app-server-protocol common.rs): `TurnSteer` "turn/steer" :1126 (TurnSteerParams v2/turn.rs:328: thread_id,
client_user_message_id, input, expected_turn_id; response turn_id); `ModelSafetyBufferingUpdated` :2140 (v2/model.rs:219, has thread_id/turn_id);
`McpServerStatusUpdated` "mcpServer/startupStatus/updated" :2118 (v2/mcp.rs:374, thread_id Option); `ThreadGoalUpdated`/`Cleared` :2066/:2069;
`ModelVerification` :2135, `AuthRecoveryStarted/Completed` :2136-2137, `GuardianWarning` :2142 (warning cells); `ReviewStart` :1179 is
chat-commands' (/review). Entry points: thread-scoped ones through `screens::chat::project` (no threads.rs edit) or threads.rs `project`;
`turn/steer` only as `ThreadAsk::Steer` + `ThreadFx` (ADR-003); thread-less MCP status through core/src/app.rs -> `chat::apply`.

## 4. Flutter -> rtwk
- Working row: StatefulWidget + AnimationController + Timer.periodic -> exists: activity.rs (`rtwk::animation::Spinner`
  rtwk-animation/src/spinner.rs:47, `AnimationController` controller.rs:154, `AnimatedBuilder` builder.rs:31, `AppBinding::start_periodic`
  rtwk-widgets/src/binding/app_binding.rs:672). New headers are a `header` string from core state, no new widget.
- Pending input preview: a StatelessWidget, `Column` of section headers and `Row[SizedBox(prefix), Expanded(Text.maxLines+ellipsis)]` ->
  `rtwk::layout::Row`/`Expanded` (rtwk-widgets/src/layout/flex.rs:197/:270), `rtwk::text::Text` (text/widgets.rs:66) max_lines + TextOverflow;
  prefix widths fixed by `SizedBox` (G26: Text drops a trailing blank). URLs kept whole: G72 (linkify) where the oracle links them.
- Actionable banner (backend banner, Luna Reserve selector, `Approaching rate limits` switch): MaterialBanner with numbered actions ->
  `Column[Text title, Text description, Options rows, Text hint]` with the app's `widgets/options.rs` (SelectionController cursor, tap /
  double-tap, number keys) inside `Focus` + `KeyBindings` (rtwk-framework/src/focus/widgets.rs:85, focus/shortcuts.rs:1257); not `Callout`.
- Safety retry prompt, misalignment precaution/review confirmation, Implement-this-plan, reasoning scope: showModalBottomSheet with a
  ListTile list -> the S14 generic selection dialog `Menu::List` (core/src/screens/select.rs:43-50, view src/sheets/select) opened by id;
  the pick comes back as `Picked::Item(id)`.
- Interrupt / steer / queue keys: Shortcuts + Actions -> `Focus::bind` on the chat (exists :634-638) and `KeyBindings`.
- External-writer notice in place of the composer: AnimatedSwitcher between two children -> already a branch in session_page (:488).
- State (bloc per thread): pending steers, rejected steers, queue on `ThreadModel` (threads.rs, apply only); per-thread live facts (safety
  buffering, review mode, goal, retry header, plan mode collaboration state) on `chat::ThreadPart`; app-scoped MCP startup round on
  `chat::Model`; views read by `select::<ThreadModel, ..>` / `watch::<ScreenStates>` (ADR-003, no copy in widgets).
- Replay: no widget; `ListView` over the transcript exists; replay is core projection only.
No new gap rows. Existing rows that may bite: G26, G48 (blank-only wrap in wrapped preview rows), G72.

## 5. Area and conflicts
Porter area (extends the current `chat-live-shards` area):
- src/pages/session/{session_page,activity,pending_input,banners,external_writer}.rs, src/pages/session/mod.rs (2 lines per new module),
  new src/pages/session/live/ (new widgets + its own `strings.rs`, ADR-010 allows a per-directory strings.rs; do not use
  src/pages/session/strings.rs, the transcript porter holds it).
- core/src/screens/chat.rs, core/src/screens/chat/ (banner.rs + new safety.rs, mcp.rs, review.rs, goal.rs, plan.rs).
- core/src/threads.rs (steer, queue categories, interrupt restore, error info, retry header) — NOT threads/{cells,activity,collab}.rs.
- tests/oracle/chat_live_1..6.rs (chat_live_6 only the usage_notice hunk if usage-status holds the file), tests/pages/chat_live.rs,
  tests/pages/chat.rs, core/tests/model.rs (thread-model tests; core/tests/all.rs is held by keymap), docs/rtwk-port/parity/chat-live.md.
Shared files, minimal change:
- core/src/app.rs (held by keymap): one arm `ServerNotification::McpServerStatusUpdated` -> `chat::apply(Ev::McpStartup{..})` and
  `BackendEv::Lagged` -> `chat::apply(Ev::McpLagged)`; no other app.rs change. Needs: keymap lands first, or the orchestrator hands the two arms.
- core/src/fixtures/script.rs + tests/kit/seed.rs (seed.rs held by keymap): a `lagged` step (BackendEv::Lagged) and a `respond` step
  (the fake answers a client request by method: `turn/steer` ok / not-steerable error, `thread/turns/list` for replay).
  tests/kit/mod.rs (unclaimed): `Screen::respond(method, result)` / `Screen::requests(method)` over FakeHandle.
- core/src/threads/cells.rs `Notice` (cells-markdown's): three variants `Interrupted`, `InterruptedForSteer`, `GoalBudgetReached` (+ a
  `Suppressed` mode is chat state, not a variant) and their rows in src/pages/session/cells/notice.rs:33-39. Give to chat-live as one hunk, or
  cells-markdown adds them first.
- src/pages/session/actions.rs (settings holds it): `review_mode: false` :63-65 -> read `ThreadPart.review_mode`. One line.
- src/pages/session/composer/footer.rs (transcript porter holds it): Plan-mode indicator and steer/queue hint words (chat-status owns the
  footer's shards). Plan mode step waits for it.
- Shard files of chat-status (chat_status_3.rs, pane_footer_2.rs) for the three widget groups if the person moves them here (§7).
Pieces that belong elsewhere (one line each):
- chat-status: status line / footer / terminal title items (thread name, provider, side context label), the footer queue/steer hint row and
  Plan-mode label in the footer; reads `Run`/`ThreadPart`, does not edit activity.rs/pending_input.rs/banners.rs.
- cells-markdown: every history cell's drawing (command/exec incl. interrupted-failed, sub-agent, collab, user message, final separator
  `Worked for`, streamed list reflow, active cell, clipped links, warning/error cell look); chat-live only decides which item/notice is added.
- chat-approvals-live: tool-suggestion install link form, strict auto-approval review notice, approval requests during a turn.
- popups: /review preset, branch, commit, custom-prompt pickers; /model save failure (config_errors); reasoning picker rows that the Plan
  reasoning-scope prompt follows.
- chat-commands: /goal, /rename suggestion prompt, /review and /fork dispatch (chat-live only holds them back during MCP startup).
- transcript: session header, owned live history, Ctrl+T detailed rows (a thread warning's main-screen visibility is chat-live's projection
  plus transcript's filter: chat-live adds the Item; transcript decides it shows on the main screen).
- usage-status: /usage menu, reset-credit picker/confirmation/redemption, startup reset hint, /status thread billing (chat_live_6 usage +
  thread_usage, 31 tests).

## 6. The plan (one job, after C1)
1. Layout to the oracle's widgets (tests first: actionable_banner 6 + pending_input_preview 13 + status_indicator 7 if moved here; else page
   tests in tests/pages/chat_live.rs). Move activity/pending_input/banners into src/pages/session/live/ with live/strings.rs. Rebuild
   `BackendBanners` as the actionable banner (options.rs rows, number keys, Esc dismiss, tap/double-tap, wheel inert) reading
   chat/banner.rs; `PendingInput` three-section layout (reads ThreadModel queue now; steers in step 2). Oracle: actionable_banner.rs
   150-291, pending_input_preview.rs 85-250, status_indicator_widget.rs 1-400.
2. Steer / queue / interrupt restore (input_queue 1, review_mode steer x10 + restore x3 + Esc/Ctrl+C x2 + notices x5, plan_mode queued 2).
   threads.rs: `ThreadAsk::Steer{thread, turn, client_id, input}` + `ThreadFx::steer`; `ThreadEv::Submit` while running -> steer (pending_steers
   on ThreadModel, compare key = text); reply ok -> stays pending until the matching UserMessage ItemCompleted pops the front
   (item_completed_only_pops_front_pending_steer); not-steerable error -> rejected_steers; turn-id mismatch -> one retry with the actual turn;
   `ThreadEv::Queue` (Tab) -> queue; turn end -> rejected first, then queue (input_restore.rs:253-320); Esc with pending steers -> interrupt
   and send them merged after it (interaction.rs:314-327, `submit_pending_steers_after_interrupt`); manual interrupt -> steers then queue restored to the composer (input_restore.rs:433-612; the
   composer draft is the composer's controller: emit an intent, do not reach into it); review mode + pending steer + Esc -> warning
   (interaction.rs:301); notices Interrupted / InterruptedForSteer (input_restore.rs:448); edit-last-queued key pops the newest queued into the
   composer (interaction.rs:285-298). Kit `respond` step first. core tests in core/tests/model.rs for each rule (double Enter, late reply,
   reply after turn end, interrupt with nothing pending).
3. Warnings, errors, retry header, provider error (app_server WARN x6, ERR x4, STATUS 1, Worked-for clears 1, startup panel 1; rendering
   provider_error 1). Project ModelVerification / GuardianWarning / AuthRecovery / thread ConfigWarning into Item::Warning (threads.rs project);
   TurnError wording from `codex_error_info` in core (protocol.rs, provider_error.rs:84-124); `Error{will_retry}` -> ThreadPart retry header,
   restored by the thread's next notification in threads.rs apply; startup warnings -> dialogs `Open::Warnings` once, not per thread.
4. Safety buffering + misalignment (app_server SAFETY x7, misalignment 3). chat/safety.rs from `model/safetyBuffering/updated`
   (safety_buffering.rs:104-240: hidden/stale/historical ignored, side conversation without retry, short message without faster model);
   header in StatusSlot; retry menu via select `Menu::List` (Retry / Dismiss and keep waiting / Learn more), confirmation, closes on turn
   complete or response start; retry = ThreadFx start of a turn with the faster model. Misalignment: precaution popup, drafts discarded,
   review confirmation (misalignment_policy.rs:60-248).
5. Backend banners, Luna Reserve, usage notice (backend_banners 12, luna 4, usage_notice 1). Bodies as `notify account/rateLimits/updated`
   steps (core/src/app.rs:793 already routes); dismissal per occurrence, model-switch preservation, fallback candidates, notify-owner cta
   (backend_banners.rs), Luna selector and `Luna Reserve default` status (luna_reserve_model.rs), usage warning line (usage_notice.rs).
   Needs a `set_model` step: `screen chat` Status seed already carries model (chat.rs seed); add one if not.
6. MCP startup (mcp_startup 20). chat/mcp.rs on chat::Model: per-server state, header `Booting MCP server: x` / `Starting MCP servers
   (n/m)`, failure + summary warnings once per round, lag settle (needs the app.rs arms and `lagged` step); while starting, submits and
   queued slash commands (/review, /compact, /fork, shell) are held, not rejected (input_queue `QueuedInputAction`).
7. Review mode, goal, side gates (review banners/prompt x3, budget x2, side 5). ThreadPart.review_mode from Entered/ExitedReviewMode;
   the review prompt item hidden; actions.rs reads it; `thread/goal/updated` -> ThreadPart.goal, budget-limited turn notice; side gates via
   Flags seed; Suppressed interrupted-notice mode for side threads.
8. External writer + rendering (rendering external writer x5): kit step for a foreign writer (thread resumed read-only), fork-in-progress
   frame, command-center offer under the notice (external_writer.rs).
9. Plan mode (plan_mode 23 P). Collaboration mode on ThreadPart/flags, `/plan` literal submit, Implement-this-plan prompt after a proposed
   plan (skips: replayed completion, queued messages, steer after plan, rate-limit prompt pending), reasoning-scope prompt, mode-switch
   model/reasoning notices, mode change during a running turn refused. Footer label waits for composer/footer.rs (needs).
10. Replay/projection C4 (history_replay 24 T, history_projection 6 T, compaction replay 2). Write all 30 from the oracle
   (chatwidget/tests/history_replay.rs, history_projection.rs); `ReplayKind` in core: no timers, in-progress turn marks running, review prompts
   do not seed composer history, user messages do (popovers Record intent); cell-only reds pend with "cells-markdown: <cell>".
11. Close: parity pack docs/rtwk-port/parity/chat-live.md, pty scenario both binaries (Enter/Tab during a turn, Esc, banner keys/mouse),
   full checks per fleet brief.
Parallel: step 10 is independent of 1-9; steps 5-8 are independent of each other after 1-2.

## 7. Gaps, risks, decisions
- Risk: threads.rs is touched by chat-live (apply/ask/queue) and cells-markdown (item_entry delegations :1340-1355): disjoint hunks; one
  owner at a time or cells-markdown stays in threads/*.rs files.
- Risk: the Callout banner and its C1 page tests are replaced in step 1 (visual delta today: box and close button vs oracle's numbered rows).
- Risk: kit steps (`respond`, `lagged`, foreign writer, history replay) live in keymap-held files: sequence after keymap.
- Risk: reset-credit flow ownership differs between this batch list (usage-status) and briefs/usage.md:48 (chat-live): one owner needed.
- Decision (person): Enter during a running turn — queue (ours today, threads.rs:1426-1433) vs oracle steer. Recommend the oracle:
  Enter steers, Tab queues. Evidence: input_flow.rs:58-64 submits now while `agent_turn_running`; input_submission.rs:257 + :482-495 makes it a
  `PendingSteer`; user_message_submission.rs:134-157 sends `turn/steer` with the expected turn and queues only when the turn is not steerable;
  chat_composer.rs:3335 Tab -> `InputResult::Queued`; footer.rs:321-324 `tab to queue message`; pending_input_preview.rs:99/:149 two
  sections; 12 chat_live_5 tests assert it. Queue-on-Enter is a user-visible behaviour delta with no gap row.
- Decision (person/orchestrator): move the widget-level groups status_indicator_widget (7), pending_input_preview (13), actionable_banner (6)
  from chat-status's shards to chat-live, since chat-live owns those widget files. Recommend yes (one owner per widget).
- Decision: reset-credit flow (chat_live_6 usage 26 + thread_usage 5) to usage-status. Recommend usage-status (all /usage sheets in one chunk).
