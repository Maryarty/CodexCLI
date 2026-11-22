# Recon brief: chat-live (SessionPage live turn, screen map C06, tm `chat-live-shards`)

Status: recon cut short by the coordinator (2026-11-22). Sections 1-3 and 5-6 are from the code; the parts marked
`TODO (recon cut short)` were not checked. NO cargo was run: no probe of stale pendings was made (the last junit,
target/nextest/oracle/junit.xml of 2026-11-21 22:22, is a normal run: 3 pass / 33 n/a / 185 pending / 30 todo).

## 1. Scope
On screen: the rows between the transcript and the composer while a turn runs, and the turn's notices.
- Status indicator (spinner, header Working/Thinking/Compacting/Starting MCP servers, elapsed, `esc to interrupt`, step line): oracle `tui/src/status_indicator_widget.rs` (1-589), `chatwidget/status_state.rs` (1-196), `chatwidget/turn_lifecycle.rs` (1-114), `chatwidget/turn_runtime.rs` (1-625).
- Streaming answer and its completion (`Worked for`, final separator): `chatwidget/streaming.rs` (1-851), `chatwidget/completion.rs` (1-21).
- Interrupt and its notices (`Conversation interrupted`, `Model interrupted to submit steer instructions`, budget message): `chatwidget/interrupts.rs` (1-299), `chatwidget/input_restore.rs` (1-870).
- Queued input and pending steers (preview above the composer, restore on interrupt): `chatwidget/input_queue.rs` (1-195), `chatwidget/input_submission.rs` (1-652), `chatwidget/input_flow.rs` (1-455), `bottom_pane/pending_input_preview.rs` (1-486).
- Backend banners, Luna Reserve, rate-limit notices: `chatwidget/backend_banners.rs` (1-527), `bottom_pane/actionable_banner.rs` (1-291), `chatwidget/luna_reserve_model.rs` (1-67), `chatwidget/rate_limits.rs` (1-710), `chatwidget/usage_notice.rs` (1-194).
- Safety buffering / misalignment: `chatwidget/safety_buffering.rs` (1-296), `chatwidget/misalignment_policy.rs` (1-248).
- MCP startup header and warnings: `chatwidget/mcp_startup.rs` (1-226).
- Warnings, errors, provider error: `chatwidget/warnings.rs` (1-157), `chatwidget/provider_error.rs` (1-177), `chatwidget/protocol.rs` (1-773).
- Also in the shards (wider than the block title): plan mode (`chatwidget/plan_implementation.rs` 1-106), review mode (`review.rs`, `review_popups.rs`), side conversations (`side.rs`), /usage and reset credits (`usage.rs` 1-601, `reset_credits.rs` 1-79), thread usage (`thread_usage.rs` 1-432), history replay (`replay.rs` 1-484), compaction (`compaction.rs` 1-57), goal (`goal_status.rs` 1-228).
User: sends while a turn runs (Enter = steer, Tab = queue), presses Esc / Ctrl+C to interrupt, sees the working row, the queued/steer preview, banners and their CTAs (click / dismiss), the retry offer of a safety buffer.

## 2. Tests (251: 3 pass, 33 n/a, 185 pending, 30 todo)
Per oracle file (shard): pending P / n/a NA / todo T / pass.
- chat_live_1 (56): input_queue 1 P; pets 1 NA; rendering_tests 14 (11 P: external writer x5, active cell x2, session header, clipped hyperlinks, provider error, owned live history; 3 NA cache counters); app_server 40 (2 pass, 1 NA feedback event, 37 P).
  app_server pendings by reason const in the file: SAFETY_PENDING 6 (+1 side), WARN_PENDING 6, ERR_PENDING 4, COLLAB_PENDING 3, STATUS_PENDING 1, others: session header/settings x3, thread name x3, user-message cells x2, command/file-change/sub-agent cells x4, working/Worked-for 1, strict review 1, startup warnings panel 1, tool-suggestion install URL 1.
- chat_live_2 (56): backend_banners 12 P (all "no seed for a backend banner / rate-limit snapshot"); compaction 4 P; completion_styling 2 (1 P, 1 NA); config_errors 1 P; goal_validation 2 P; list_spacing 1 P; luna_reserve 4 P; history_projection 6 T; history_replay 24 T.
- chat_live_3 (23): mcp_startup 20 P ("no MCP startup state"; 8 also need an app-server lag step); misalignment 3 P.
- chat_live_4 (41): plan_mode 40 (15 NA app events, 25 P: no collaboration mode / Implement-this-plan / reasoning-scope prompt / seeds); replay_render 1 pass.
- chat_live_5 (43): review_mode 33 (4 NA, 29 P: steer x12, interrupt notices x5, review banners x4, review popups x4, ctrl+c x2, budget x2); side 10 (1 NA, 9 P).
- chat_live_6 (32): usage 26 (5 NA, 21 P: rate-limit reset flow); thread_usage 5 (2 NA frame scheduling, 3 P billing); usage_notice 1 P.
Feature groups (for chunks):
- G1 live-turn core (working row, Worked for, error/warning cells, interrupt notices, status header retry, thread name/settings, session header): ~45 tests in app_server, review_mode interrupt_*, compaction, completion. MANY LIKELY STALE: ours already has Activity (Working/Thinking/Compacting), Item::Warning/Error, ThreadEv::Settings/Name, TurnError (see §3). Probe first.
- G2 queue / steer / interrupt restore: input_queue 1, review_mode steer_* / manual_interrupt_* / esc_* (~14), plan_mode queued 2.
- G3 safety buffering + misalignment: 10.
- G4 backend banners + Luna Reserve + usage notice: 17.
- G5 MCP startup: 20.
- G6 review mode + side: ~38 P.
- G7 plan mode: 25 P.
- G8 /usage reset credits + thread billing: 24 P.
- G9 history replay/projection: 30 T (ports never written; each becomes pass or pending).
- G10 external writer / rendering: 11 P (read-only view exists; needs a kit/feed step that makes the thread a foreign writer).

## 3. What exists in ours
- src/pages/session/activity.rs:16-183 `Activity` StatefulWidget: spinner (rtwk Spinner), shimmer header (AnimationController + AnimatedBuilder), 1 s periodic timer, `esc to interrupt` label, 2-line step; `compact_elapsed` = oracle fmt_elapsed_compact.
- src/pages/session/session_page.rs:52-802 `Chat`/`ChatState`: status slot (:470-516: read-only none, Reconnecting, failure text, Starting, Activity keyed by (thread, turn)); queue preview of 3 + `more queued` (:603-615, plain Text, not the oracle's pending_input_preview layout, no steers); chat notice (:616-630); backend control; backend banner as rtwk `Callout` with links and dismiss (:678-704); backend notices (:705-707); Esc interrupts when `can_interrupt` (:374-379), Ctrl+C interrupt-or-quit (:298-305); Tab = QueueDraft (:779-781); footer queue hint while running (:563-564); read-only external-writer notice/footer (:541-544, external_writer.rs) with r/f/q keys.
- core/src/screens/chat.rs: `Model` (flags, tiers, signed_in, notice, status facts); `ThreadPart` (:310-325: error, name, cwd, model, effort, provider, service_tier, daybreak, activity phase, compaction, plan progress/step); `ThreadEv` Metadata/Settings/Phase/Compaction/Name/Plan; `project()` (:422-451) maps ThreadStarted, ThreadNameUpdated, ThreadSettingsUpdated, ReasoningSummaryPartAdded, ContextCompaction start/complete; thread intents Compact/Rename/Shell; `seed()` (:465) reads `screen` steps of dir `chat` (Flags, ServiceTiers, SignedIn, Status). core/src/screens/chat/banner.rs (207): `Banner`, `BannerOccurrence` (visible per model, dismissible, reserve_pending, links).
- core/src/threads.rs (held by startup-rest): `ThreadModel` queue (:692, Submit queues while busy :1394-1409, one per turn end `send_next` :1621), Interrupt (:1411-1423, retry on turn changed), TurnError -> error row (:1499-1504), Item::Warning/Error/Deprecation/Notice/Collab/Tool/Patch/PlanUpdate (:78-126), project() (:1158-1250: TurnStarted/Completed, Error(!will_retry), Warning, ItemStarted/Completed, TurnPlanUpdated, HookCompleted, AgentMessageDelta, CommandExecutionOutputDelta). No steer: `ThreadAsk` (:908-960) has no `turn/steer`; queue has no steer/rejected categories.
- core/src/app.rs (held by startup-rest): AccountRateLimitsUpdated -> rates + backend_banners refresh (:763-777), Warning/ConfigWarning/DeprecationNotice -> backend_notices when no thread (:781-797), desktop notice on TurnCompleted (:854).
- Missing protocol feeds: `model/safetyBuffering/updated` (ServerNotification::ModelSafetyBufferingUpdated, app-server-protocol common.rs:2140), `turn/steer` (ClientRequest TurnSteer, common.rs:1126), `thread/goal/updated` (common.rs:2066), MCP startup status notification (TODO (recon cut short): exact variant name; grep `McpServerStatus` in app-server-protocol/src/protocol/common.rs), rate-limit reset credits RPCs (TODO).
- Strings: `Worked for` only in src/pages/session/strings.rs (transcript side); no `Conversation interrupted`, `Model interrupted to submit`, `Starting MCP servers`, `Goal budget`, `Approaching rate limits` anywhere in src/core.

## 4. Flutter -> rtwk
- Working row: StatefulWidget + AnimationController + Timer.periodic -> exists (activity.rs; rtwk `Spinner`, `AnimationController`, `AnimatedBuilder`, `AppBinding::start_periodic`). MCP startup / retry header: same widget, header text from state.
- Pending input preview (queued, pending steers, rejected): a Column of `Text` with maxLines/ellipsis inside its own StatelessWidget -> new `src/pages/session/pending_input.rs` (rtwk `Column`, `Text::max_lines`, `TextOverflow::Ellipsis`); no hand wrapping (R15).
- Backend banner / Luna Reserve / safety notice with actions: MaterialBanner -> rtwk `Callout` (rtwk::controls, already used :690) with `Hyperlink` actions and `on_dismissed`; selector rows (Luna Reserve arrows/enter/esc) -> `Focus` + `KeyBindings` around the Callout actions, or a sheet. TODO (recon cut short): check `.claude/skills/rtwk/references/` for a Callout with selectable actions.
- Retry confirmation / Implement-this-plan / reasoning scope / reset-credit picker: showDialog / showModalBottomSheet -> `overlay::show_dialog` or the app's sheets (src/sheets), as the approvals sheet does.
- Interrupt keys: Shortcuts/Actions -> `Focus::bind` / `KeyBindings` (exists).
- State: bloc per thread -> `ThreadPart` (core chat.rs) for per-thread live state (safety buffering, steer preview echo, goal), `chat::Model` for app-scoped (MCP startup, banners via app.session). Views read via `watch::<ScreenStates>` / `read::<ThreadModel>` (ADR-003).

## 5. Dependencies and conflicts
- startup-rest holds core/src/app.rs, core/src/threads.rs, core/src/protocol/mod.rs, host/src, src/app/*: G2 (turn/steer in ThreadAsk + ThreadFx, steer/rejected queue categories, interrupt restore of pending steers to the composer), G5 (routing an app-scoped MCP status notification to chat::apply), G8 (reset-credit RPCs) need edits there. List for the handover:
  1. threads.rs: `ThreadAsk::Steer { thread, turn, input }` + `ThreadFx::steer`, `ThreadEv::Steer`, pending-steer list on ThreadModel, `ThreadEv::Submit` -> steer when running and not in a review turn; rejected steer -> queue.
  2. threads.rs project(): `ModelSafetyBufferingUpdated` and `ThreadGoalUpdated` (or let them fall to `screens::project_thread` -> chat.rs; preferred, no threads.rs edit).
  3. app.rs: MCP startup status (no thread id) -> `chat::apply(Ev::McpStartup ..)`; lag step.
- chat-transcript holds src/pages/session/transcript*: cells drawn in the transcript (warning/error/interrupt notice cells, Worked for, command cells) are drawn there; src/pages/session/cells is unclaimed (TODO (recon cut short): check who owns cells/).
- settings holds src/pages/session/actions.rs; chat-status (not claimed now) status_surfaces.rs.
- session_page.rs is touched by every group: run chunks touching it one at a time, or chunk 1 first carves the build into child widgets (status slot, pending input, banners) so later chunks edit their own files.

## 6. Chunks
1. C1 probe + live-turn core (G1, G10 cheap part). Area: src/pages/session/session_page.rs, activity.rs, new pending_input.rs, banners.rs, tests/oracle/chat_live_1.rs, chat_live_5.rs (interrupt tests), chat_live_2.rs (compaction/completion), tests/pages/chat_live.rs. Steps: (a) `KIT_PROBE_PENDING=1 python3 -I tests/oracle/tools/summary.py --run chat_live_1 chat_live_2 chat_live_3 chat_live_4 chat_live_5 chat_live_6 --list stale`, unpend the stale; (b) carve session_page build: status slot -> activity.rs, queue preview -> pending_input.rs, banner+notices -> banners.rs; (c) fix remaining G1 tests whose data exists in core (warnings, errors, thread name/settings, working/Worked for, status header retry via `Error(will_retry)` -> chat ThreadPart). Done: G1 tests green or re-pended with a true reason; page tests (Esc, Ctrl+C, wheel/click on banner) in tests/pages/chat_live.rs.
2. C2 safety buffering + misalignment (G3, 10). Area: core/src/screens/chat/safety.rs (new), chat.rs project()/ThreadPart, src/pages/session/safety.rs (new), shard tests. Parallel with C4 after C1.
3. C3 backend banners + Luna Reserve + usage notice (G4, 17). Area: core/src/screens/chat/banner.rs, chat.rs seed (new `screen` ev kinds for banner/rate-limit snapshot), src/pages/session/banners.rs, chat_live_2.rs, chat_live_6.rs usage_notice. After C1 (banners.rs).
4. C4 history replay/projection ports (G9, 30 todo). Area: chat_live_2.rs only (+ fixtures tests/oracle/fixtures/chat_live_*). Parallel with anything: write each test, pass or pend.
5. C5 queue/steer/interrupt restore (G2, ~17). WAITS on startup-rest releasing threads.rs (or a task_update widening agreed by the orchestrator).
6. C6 MCP startup (G5, 20). WAITS on app.rs routing.
7. C7 review mode + side (G6, ~38): TODO (recon cut short): review popups probably belong to chat-popups / approvals; side needs side-thread state (ADR-003: a second ThreadModel through the app).
8. C8 plan mode (G7, 25): collaboration mode in composer footer + sheets; large, own task.
9. C9 /usage reset credits + thread billing (G8, 24): sheets + account RPCs (app.rs); own task.
Parallel: C4 with all; C2 with C3 only if C1 landed the carve; C5/C6 blocked.

## 7. Gaps and risks
- Our Enter while running queues (threads.rs:1399); the oracle steers (Enter) and queues (Tab): a user-visible difference until C5; no gap row yet.
- Queue preview layout differs from oracle pending_input_preview (prefix, wrap, edit hint): fold into C1.
- Pending reasons in the shards are partly stale (written before Activity / warnings / settings existed): the probe decides.
- Unknown: whether rtwk Callout can host a selectable action row (Luna Reserve selector): TODO (recon cut short).
- Unknown: kit step for a foreign-writer thread and for app-server lag (rendering_tests external writer x5, mcp_startup x8).
