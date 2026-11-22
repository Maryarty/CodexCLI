# Recon brief: chat-approvals-live (approvals in the live chat, tm `recon-approvals-live`, 2026-11-22)

Base: claude/tui-rtwk b7e2a0aef. Read-only recon, no cargo. The junit (`target/nextest/oracle/junit.xml`,
2026-11-21 22:22) is OLDER than the approvals commits (9c85fdbbe..586b4763d, 2026-11-22 00:31); the counts
below come from the shard files and from `docs/rtwk-port/parity/approvals.md` (the porter's own `summary.py --run`).

## 0. First step for the porter
1. Rebase onto the tip after the **keymap** task lands (it holds `core/src/app.rs`, `tests/kit/seed.rs`,
   `src/app/view.rs`; it adds the kit's `.keymap(..)` seed). 15 in-chunk pendings wait only for that seed.
2. Probe: `KIT_PROBE_PENDING=1 python3 -I tests/oracle/tools/summary.py --run approvals_1 approvals_2
   chat_approvals_1 chat_approvals_2 pane_footer_2` then `--list stale`. Candidates from reading the reasons
   against the code:
   - `chat_approvals_1::app_server_write_stdin_approval_renders_terminal_input` (`.script(write_stdin.jsonl)`;
     `core/src/screens/approvals.rs:578-583` already reads `kind: writeStdin` and the `Input:` row);
   - `chat_approvals_1::approval_modal_exec_multiline_prefix_hides_execpolicy_option_snapshot` (`.script(..)`, the
     reason says "no entry", but a script reaches the live presenter);
   - `chat_approvals_1::approval_modal_patch_snapshot` (entry `approvals/patch_edits` exists, `src/sheets/approvals/mod.rs:96`);
   - after keymap: the 15 `keymap(..)` tests (approvals_1 7, approvals_2 6, chat_approvals_2 2).
   Known vacuous stale (keep pending): `user_message_during_user_shell_command_is_queued_not_steered`,
   `ordinary_follow_up_clears_unanswered_questions_after_accepted_input` (bodies say "needs feed").
3. The orchestrator re-tags the 35 non-approval `exec_flow` tests of chat_approvals_1 (section 2) before the
   porter starts; they are NOT this chunk's work.

## 1. Scope
Oracle (codex-rs/tui/src, never edited):
- Overlay and its decisions: `bottom_pane/approval_overlay.rs` 1-1230 (queue 222, 507; decisions 340-500; cancel
  515-551; keys 553-610 incl. `open_fullscreen` 555-563, `open_thread` 566-574; header pieces 660-830);
  `approval_overlay/clipping_tests.rs` 1-80 (clipped header, `[… N lines] ⌃g view all`).
- Arrival in the chat: `chatwidget/tool_requests.rs` 10-35 (defer), 284-491 (handle_*_now), guardian 37-255;
  `chatwidget/streaming.rs` 702-725 (`defer_or_handle`: wait while the answer streams); `bottom_pane/mod.rs`
  250 (`APPROVAL_PROMPT_TYPING_IDLE_DELAY` 1 s), 293-316, 808-884 (delayed queue), 2085-2160 (push_* , view stack).
- Other threads: `app/thread_routing.rs` 233 (thread_label), 339-520 (label, inactive patch preview 507-519),
  521-565, 1069-1100 (pending set), 1440-1490 (enqueue); `app/pending_interactive_replay.rs` 1-643;
  `bottom_pane/pending_thread_approvals.rs` 1-108; `app/file_change_approvals.rs` 1-72 (patch changes from the item).
- Decision cells: `history_cell/approvals.rs` 1-351 (user decisions 26-280, guardian denied/timed-out 282-351).
- Full-screen preview: `app/event_dispatch.rs` 3673-3730 (`E X E C`, `P A T C H`, permissions,
  `U S E R  V E R I F I C A T I O N` static overlays).
- MCP elicitation `bottom_pane/mcp_server_elicitation.rs` 1-2100 (queue 1161); request_user_input
  `bottom_pane/request_user_input/{mod.rs 1-1560, layout.rs, render.rs}` (auto-resolution 73-320, 917; queue 251);
  user verification `bottom_pane/user_verification.rs` 1-366 + `app/user_verification{,_requests,_errors}.rs`.
- Guardian / auto-review: `chatwidget/tool_requests.rs` 37-255, `auto_review_denials.rs` 1-146,
  `chatwidget/permission_popups.rs` 184-270 (`/auto-review` popup + approve).
- Async questions (questions inside agent messages): `bottom_pane/async_questions/*.rs` (1228),
  `bottom_pane/questions.rs` 1-93, `chatwidget/questions.rs` 1-194, `async_question_reply.rs` 1-67,
  `chatwidget/input_submission.rs` 311-335, 547, 584, `chatwidget/protocol.rs` 729, `chatwidget/streaming.rs` 541.
User sees: a running turn asks (run a command, edit files, grant permissions, an MCP form, a question); the bottom
pane becomes the prompt (after 1 s of no typing), the answer goes back, the transcript records `✔ You approved …` /
`✗ You canceled …`; requests of other threads carry the thread label, `o` opens it, a banner lists them; auto-review
shows `Reviewing approval request` and denial cells; questions in agent messages get an inline editor above the composer.

## 2. Tests
State per shard file (`kit::pending` = pending, `kit::na` = n/a). Totals: approvals_1 58 = 15/17/26,
approvals_2 67 = 31/14/22, chat_approvals_1 54 = 2/5/47, chat_approvals_2 48 = 0/18/30 (pass/na/pending).
- approvals_1
  - overlay 28: 8 pass, 13 na, 7 pending: `configured_list_cancel_*` 2 (keymap), `approval_headers_preserve_wrapped_url_destinations`
    (G72), `network_exec_prompt_title_includes_host` (kit: `tests/oracle_snap` cannot parse a row with `\"`), 3 decision
    cells (`exec_history_cell_wraps_with_two_space_indent`, `..._does_not_render_blank_action_for_empty_command`,
    `network_access_command_history_uses_target_without_structured_context`).
  - clipping 1 pending (`clipped_exec_approval_opens_the_complete_command`: keymap + entry `exec_clipped` + clipped header).
  - async questions (`async_questions/state_tests.rs`) 19: 4 na, 15 pending (entries `approvals/user_input_*` missing;
    vim 1, history 2, keymap 2).
  - MCP elicitation 10: 7 pass, 3 pending (2 keymap, `form_prompt_preserves_the_complete_wrapped_url_destination` G49+G72).
- approvals_2
  - banner 3: 1 na, 2 pending (`render_single/multiple_threads_snapshot`: widget exists, not placed on the SessionPage).
  - `questions_tests.rs` 1 pending (async questions).
  - request_user_input 55: 30 pass, 11 na, 14 pending: countdown 2, keymap 6, `backspace_in_options_clears_selection`
    (Options has no empty cursor), `freeform_shift_enter_*` (kit sends no kitty shift+enter), `long_option_text` (G49+G39b),
    `hidden_options_footer` (G39b), `wrapped_question_preserves_the_complete_url_destination` (G49+G72),
    `unanswered_confirmation_snapshot` (no seed opens it).
  - render_tests 1 pending (`halfwidth_sound_marks_*`: no seed of a 5x2 styled line; write an entry or n/a with reason).
  - user verification 7: 1 pass, 2 na, 4 pending (clipped header, waiting = controller, long URLs G72, full-screen pager).
- chat_approvals_1
  - `approval_requests.rs` 8: 2 na, 6 pending (decision cells 5, write_stdin 1 = stale candidate).
  - `exec_flow.rs` approvals 11: 2 pass (`approval_modal_exec_*`), 2 na, 7 pending (multiline prefix, patch snapshot,
    untrusted, omits diff summary, manual flow, manual header, apply events → patch cell after approval).
  - `exec_flow.rs` NOT approvals, 35 (34 pending + 1 na) → other chunks:
    - cells-markdown 23: replayed_command_completion, failed_exploration, exploration_nonzero, exploration_reasoning,
      adjacent_exploration, replayed_commands_preserve, exec_history_cell_shows_working_then_completed/failed,
      exec_end_without_begin x3, overlapping_exploring, unified_exec_startup, unified_exec_unknown_end, wait_after_final,
      wait_before_streamed, empty_then_non_empty, view_image x2, image_generation, exec_history_extends_previous,
      user_shell_command_renders_output, interrupt_preserves_unified_exec_wait_streak;
    - chat-status 4: unified_exec_begin_restores_working_status, unified_exec_waiting_multiple_empty,
      wait_status_renders_command_in_single_details_row, non_empty_then_empty;
    - chat-commands 4 (+ na bang_shell): replayed_background_terminals, interrupt_preserves_unified_exec_processes,
      turn_complete_keeps_unified_exec_processes (/ps), disabled_slash_command_while_task_running;
    - chat-live 3: external_writer_snapshot (G10), final_worked_for, user_message_during_user_shell (queue).
- chat_approvals_2
  - guardian 11 pending; question notifications 4 na; question_turn_end 5 pending; questions_tests 28: 14 na, 14 pending.
- pane_footer_2 (chat-status's file, "belongs to approvals" per chat-status.md:42): lines 454-584, 6 tests:
  `overlay_not_shown_above_approval_modal`, `approval_request_is_delayed_after_recent_typing`,
  `typed_approval_shortcuts_during_delay_stay_in_composer`, `delayed_approval_shortcut_works_after_idle_deadline`,
  `composer_shown_after_denied_while_task_running` (5 pending) + `turn_start_keeps_an_outstanding_approval_paused` (na).
**In-chunk: 198 tests = 48 pass + 54 n/a + 96 pending.** Of the 96: 15 wait for the keymap seed, 6 sit on open
gaps (G72 2, G49+G72 2, G49+G39b 1, G39b 1), 1 on the kit's shift+enter, 1 on the kit's quote row; ~73 are build work.
State tests left by the shard tool go to `core/tests/approvals.rs`: approval_overlay 9, elicitation 13, RUI 21,
async questions 1, user verification 1 (queue order, ctrl+c, interrupt, resolved-removes-one).

## 3. What exists in ours (codex-rs/tui-rtwk)
- Request queue, app-owned (ADR-003): `core/src/app.rs` 105-142 (`PendingRequest`, `RequestQueue`), arrival 930-980
  (`BackendEv::Request` → desktop notice → queue push with prompt and thread label AT PUSH TIME), `serverRequest/resolved`
  855-870, reconnect reset 645-690, seeded prompt 1022-1032, answers 1425-1456, `resolve_request` 1540-1555 →
  `Command::ResolveRequest`; `DismissRequest` → `Command::RejectRequest`. Answer mapping `core/src/protocol/requests.rs`
  1-173 (exec incl. network rule and execpolicy amendment, patch, permissions incl. strict, elicitation, RUI).
- Presenter `src/sheets/presenter.rs` 1-159: `show_dialog_from` the Navigator home's handle, one dialog at a time,
  head of the queue (FIFO). No typing delay, no stream deferral, no stacking of another kind.
- Core prompt model `core/src/screens/approvals.rs` 1-748 (Prompt kinds 116-200, `thread_label` 433, `pending_threads`
  436-460, `prompt_of` 462-560 reads the request as untyped JSON keys; `PatchPrompt` has no changes: 474-477);
  `approvals/form.rs` 1-490, `approvals/input.rs` 1-102. Per-thread slot parts are EMPTY: `ThreadPart`/`ThreadEv`/
  `Command`/`Reply`/`Ctx` at 312-330 — the place for guardian and async-question state.
- Sheets `src/sheets/approvals/`: dialog.rs 299 (Esc cancel, `o` → `hs.open_thread` 102-104, 256-259), present.rs 427,
  form.rs 648, user_input.rs 808 (its "interrupt" 244-248 only dismisses = RejectRequest; oracle sends turn interrupt),
  banner.rs 72 (`PendingThreadApprovals`, not mounted), dismiss.rs 89, tips.rs 50, strings.rs 347, mod.rs 315 (entries).
- Tests: `core/tests/approvals.rs` 33, `tests/sheets/approvals.rs` 40. Pty `check.py --only exec-approval /
  request-user-input`: PASS on both binaries (parity pack "Run in the binary").
- Missing data paths (app-server-protocol `src/protocol/common.rs`):
  - guardian: notifications `item/autoApprovalReview/started` / `completed` (2090-2091), `guardianWarning` (2142) are
    DROPPED by `core/src/protocol/targets.rs:16-40` (`_ => return None`); request `thread/approveGuardianDeniedAction`
    (786) not mapped (thread RPC → `ThreadFx::request` with an `approvals::Command`, mapped in `core/src/protocol/mod.rs`).
  - user verification: client request `userVerification/verify` (558, not thread-scoped) → an app-level `Command`
    sent by `core/src/app.rs`, mapped in `core/src/protocol/mod.rs`, run by `src/services/backend/rpc.rs`.
  - async questions: `ThreadItem::AgentMessage.questions` (`v2/item.rs:347`) is not read (`core/src/threads.rs` Item::Agent
    has no questions); the answer is an ordinary user turn (`turn/start` / steer via `ThreadFx::start_turn`) whose text is
    the reply envelope of `async_question_reply.rs`.
  - no `Item` for a decision (`core/src/threads.rs:78-125`; `threads/cells.rs:209` `Notice` has 3 kinds).

## 4. Flutter -> rtwk
| piece | Flutter | rtwk (found) |
|---|---|---|
| open a request | BlocListener + showModalBottomSheet | `AppCx::show_dialog_from` + `sheets::sheet_options()` (`src/sheets/presenter.rs:131-140`) |
| a second kind over the first | a second showModalBottomSheet on the Navigator | the same call again: rtwk dialogs are routes, they stack (`references/widgets/navigation.md`) |
| typing-idle delay | `Timer` debounce | `AppBinding::start_timer(after, handler)` / `binding.now()` (rtwk-widgets/src/binding/app_binding.rs:542, 664); our use: `src/pages/session/activity.rs:94` |
| numbered options | ListView of tiles | `src/widgets/options.rs` `Options` (exists; needs an empty-cursor state) |
| clipped header + `[… N lines]` | LayoutBuilder + TextPainter line metrics + `Text(maxLines)` | `LayoutBuilder` + `TextPainter::intrinsic_height` (rtwk-render/src/objects/text/painter.rs:710) + `Text::max_lines` (rtwk-widgets/src/text/widgets.rs:199); picks the spelling, cuts nothing by hand |
| full-screen preview | `Navigator.push(fullscreenDialog)` | the static pager route of the transcript chunk (hook below); body `rtwk-rich` `Highlighter` (bash), `DiffView` (patch) |
| decision cell | ListView item, `Text.rich` with hanging indent | `Row[Text("✔ "), Expanded(Text::rich(..).wrap)]` in `src/pages/session/cells/decision.rs` |
| countdown | `Timer.periodic` | `AppBinding::start_periodic` (app_binding.rs:672) in the sheet's State |
| pending-thread banner | Column child above the input | `PendingThreadApprovals` (`src/sheets/approvals/banner.rs`) |
| `/auto-review` list | showModalBottomSheet + searchable ListView, two columns | the select sheet `src/sheets/select` (search + description column) |
| question editor | Column of questions, a TextField + TextEditingController each, expand/collapse | `TextArea` over a per-question `TextEditingController` held in `ThreadUi` (`src/widgets/thread_ui.rs`, R14), `Options` for choices, `ExpansionController` (rtwk-widgets/src/layout/expansion.rs:33) |
| yield the mouse for links | n/a in Flutter | none: rtwk tracking = max(mounted widgets, open mode) (rtwk/src/app.rs:2182-2220) → G-new row |

Gap rows: G72, G49, G39b, G55 (secret field), existing. G8 (typeahead discard) is NOT needed: the oracle delays the
request instead of discarding input (bottom_pane/mod.rs:2098-2119); recommend closing G8 as "not needed".
**G-new**: Flutter none / web `pointer-events`; rtwk tried: Mouse::sync (rtwk/src/app.rs:2195-2220) sets tracking from
mounted widgets with the open mode as floor; a widget cannot turn reporting off while it shows (expanded question with links).

## 5. Area and conflicts
Porter's tm area (exclusive): `src/sheets/approvals/**`, `src/sheets/presenter.rs`, `core/src/screens/approvals.rs`,
`core/src/screens/approvals/**` (new `guardian.rs`, `questions.rs`), `core/src/protocol/requests.rs`,
`core/tests/approvals.rs`, `tests/sheets/approvals.rs`, `tests/oracle/{approvals_1,approvals_2,chat_approvals_1,chat_approvals_2}.rs`,
`tests/oracle/fixtures/{chat_approvals_1,chat_approvals_2}/**`, new `src/pages/session/cells/decision.rs`,
new `src/pages/session/questions.rs` (+ its strings), new `src/sheets/approvals/auto_review.rs`, `docs/rtwk-port/parity/approvals.md`.
Shared files, minimal change each (orchestrator sequences or hands to one owner):
- `core/src/app.rs` (keymap holds it now): label derived from the active thread instead of at push (930-980); patch
  changes from the thread's `Item::Patch` at push; inactive-thread patch preview cell; `ThreadEv::Decided` in
  `resolve_request` when the request's thread is active; ctrl+c `Intent::CancelRequests`; RUI Esc → `Intent::Interrupt`
  + drop queued RUI; user-verification controller (+ `Command::VerifyUser`); stream deferral flag read for the presenter.
- `core/src/threads.rs` + `threads/cells.rs`: `Item::Decision(Decision{subject, outcome, actor})`, `ThreadEv::Decided`,
  `Item::Agent` keeps `questions`; apply arms route guardian events to `approvals::ThreadPart`.
- `core/src/protocol/targets.rs` 16-40: three guardian arms. `core/src/protocol/mod.rs`: `client_request` arms for
  `thread/approveGuardianDeniedAction`, `userVerification/verify`. `src/services/backend/fake.rs` 440-510: answer them.
- `src/pages/session/transcript_pane.rs` (chat-transcript porter): one arm `Item::Decision(d) => cells::decision::view(d)`.
- `src/pages/session/session_page.rs` (chat-live): mount `PendingThreadApprovals` and the question editor above the composer.
- `src/pages/session/activity.rs` (chat-live / chat-status): status header `Reviewing approval request` from
  `approvals::ThreadPart` guardian reviews (oracle tool_requests.rs 37-95); pause the status timer under a modal.
- `src/pages/session/composer/*` (composer chunk): a "last edited" signal (notifier on the controller) the presenter reads;
  its vim and history-search editing reused by the question editor.
- `src/pages/session/open.rs` + transcript chunk: **hook** `Target::StaticPager { title, body }` (the static pager page of
  the transcript chunk, pager.md C1); approvals push it with `E X E C` / `P A T C H` / `U S E R  V E R I F I C A T I O N`.
- `core/src/commands/dispatch.rs:344` (chat-commands): `AutoReview` opens the auto-review sheet instead of the shared arm.
- `src/widgets/options.rs`: an empty-cursor state. `src/widgets/thread_ui.rs` (transcript porter): question draft controllers.
- `tests/oracle/pane_footer_2.rs` 454-584 (chat-status): bodies of the 6 approval tests (add `s.feed(request)`).
- `tests/oracle_snap/` (kit): parse a row holding `\"`; `tests/kit/seed.rs` (keymap): `.keymap(..)`.
Needs (app/host, not the porter's own call): host runs `userVerification/verify` (device proof stays in host/app, never a
view); remote-workspace answer `Unavailable(ProviderUnavailable)` + warning (app/user_verification.rs:34-60).

## 6. The plan (one job, tests first per step)
1. Probe and re-tag (section 0). Port the left state tests into `core/tests/approvals.rs` as each step needs them.
2. Decision cells. Core `Decision` (subject Command / NetworkAccess / Permissions; outcome approved once / session /
   prefix / denied / canceled / timed out; actor User / Guardian) worded by `history_cell/approvals.rs` 26-280; recorded
   by the app through `apply_thread(.., ThreadEv::Decided)` only for the active thread (approval_overlay.rs 392, 446);
   Esc/ctrl+c records Cancel. Cell widget `cells/decision.rs` (two-space hanging indent, `...` for multi-line, 80-col
   truncation). Turns on: approvals_1 3 cells; chat_approvals_1 `exec_approval_emits_*`, network x3, `*_truncates_*`
   (fixture `exec_long.jsonl` via `s.feed`); `apply_patch_manual_*` x2, `apply_patch_events_emit_history_cells`
   (fixtures: approval then `item/started` fileChange).
3. Live arrival fidelity. (a) label from (request thread, active thread) at present time, so a switched thread shows
   its own requests unlabelled (thread_routing.rs 339-520, pending_interactive_replay.rs); (b) patch changes from the
   thread's item (file_change_approvals.rs); inactive patch preview cell (thread_routing.rs 507-519); (c) defer while the
   active thread's answer streams (streaming.rs 702-725); (d) order: same kind joins the shown view, approvals next =
   newest (`Vec::pop`, approval_overlay.rs 507), elicitation and RUI FIFO (mcp_server_elicitation.rs 1161/2497,
   request_user_input/mod.rs 251/1770); another kind stacks over the shown one (bottom_pane/mod.rs 2085-2130) — a core
   fn `RequestQueue::to_show(shown)` tested from those oracle tests, the presenter only follows it; (e) ctrl+c cancels
   the shown approval and drops the queued ones (515-551); (f) RUI Esc interrupts the turn (`Intent::Interrupt`) and
   drops queued RUI (request_user_input tests 1796, 2705-2796). Fixtures for untrusted / omits-diff patch requests.
   Turns on: write_stdin, multiline prefix, patch snapshot, untrusted, omits diff summary.
4. Typing-idle delay in the presenter (bottom_pane/mod.rs 250, 808-884, 2098-2119: approvals only; RUI and forms are
   not delayed, 2123): a `Timer` for the remainder; typed `y`/`a` stay in the composer. Turns on pane_footer_2 4 + the
   two modal tests (no Working overlay above the modal; composer back after `n` while running).
5. Other threads: mount the banner (`pending_threads`) above the composer; `o` → `OpenThread` (exists) — check it on a
   live script. Turns on approvals_2 `render_single/multiple_threads_snapshot`.
6. Clipped header + full-screen preview: header cut to the room with `[… N lines] ⌃a view all` (hint only when
   `open_fullscreen` is bound); ctrl+a / ctrl+shift+a push the static pager hook with the oracle's bodies
   (event_dispatch.rs 3673-3730). Entry `exec_clipped`. Turns on clipping test, `clipped_prompt_exposes_fullscreen_hint`,
   `full_screen_pager_reaches_the_end_of_a_long_request` (after the pager lands).
7. User verification controller (app + host, oracle app/user_verification*.rs; its ~700 test lines → core tests):
   `Decision::Verify` sets `waiting`, sends `userVerification/verify {title, description, challenge}` with an attempt id;
   a late result after cancel/resolve/reconnect is dropped; accept with the proof or `failed`. Turns on `waiting_snapshot_*`.
8. RUI rest: auto-resolution (grace 60 s hidden, 60 s visible `auto-resolves in 1m 00s`, red, snooze on key/paste,
   empty answer at the end; not for `is_blocking`); Options empty cursor (backspace); confirmation seed; the halfwidth
   render entry. Turns on 2 countdown + backspace + confirmation (+ keymap 6 after the seed).
9. Guardian / auto-review: `approvals::ThreadPart { reviews, recent_denials }` fed by the three notifications; status
   header with the aggregate of parallel reviews (via the status indicator need); denied / timed-out cells and the
   guardian warning (Item::Warning exists); approved = no cell; cleanup restores the MCP status; `/auto-review` sheet
   (`Auto-review Denials`, `Action` / `Rationale` header, search) → `ThreadFx::request(approveGuardianDeniedAction)` +
   the two info cells (permission_popups.rs 184-270). Turns on chat_approvals_2 guardian 11.
10. Async questions: model in `approvals::ThreadPart` from `AgentMessage.questions` (dedupe by message id, replay does
    not reopen handled ones); editor above the composer (numbered choices, Other, freeform, collapsed summary, alt+up/down,
    ctrl+5 skip, blank refused, `Answer too long`), answer = `Intent::Submit` with the reply envelope (source
    QuestionAnswer, queued when a turn runs), turn-end recovery of open drafts into the composer (editor State's
    did_update_widget on the model's turn end), desktop answer dismisses only its question. Entries `approvals/user_input_*`
    (13 named in approvals_1). Turns on approvals_1 15, approvals_2 1, chat_approvals_2 19 (vim 2 + history 3 after the
    composer chunk lands vim / ctrl+r).
11. Pack `parity/approvals.md` (counts, gap rows, handover), full checks, `check.py --only exec-approval /
    request-user-input` on both binaries + a guardian / question scenario if the mock model has one.

## 7. Gaps, risks, decisions
- Risk: `core/src/app.rs` is held by keymap; steps 2-4, 7, 10 need it → start with 5, 6 (view side), 8, 9 core parts.
- Risk: label computed at push (app.rs 950-960) goes stale when the active thread changes: derive it in the view.
- Risk: `prompt_of` reads untyped JSON (approvals.rs 462-560); the typed `ServerRequest` params exist; switch when touched.
- Risk: async questions reuse composer editing; sequencing after the composer chunk's vim / history search.
- Risk: `user_input.rs:244` dismisses instead of interrupting: a user-visible protocol difference today (step 3f).
- Gap: G-new mouse yield (section 4); G72, G49, G39b, G55 stay; G8 recommend close (not needed).
- Decision (person): the 35 non-approval `exec_flow` tests go to cells-markdown 23 / chat-status 4 / chat-commands 4 /
  chat-live 3. Recommend: yes, re-tag before the chunk starts.
- Decision (person): approval queue order = oracle's newest-next (`Vec::pop`), not FIFO. Recommend: match the oracle.
- Decision (person): ctrl+c drops queued approvals without answering them (oracle). Recommend: match the oracle.
- Decision (person): the async-questions editor is in this chunk (its tests are here), not the composer's. Recommend: yes,
  as the last step, on the composer chunk's editing widgets.
