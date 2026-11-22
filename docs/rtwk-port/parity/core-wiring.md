# core-wiring — parity pack

Task `core-wiring` (the person, 2026-11-22: logic and data first, UI chunks only compose widgets). Area: `core/src/app.rs`,
`core/src/threads.rs`, `core/src/protocol/{mod,import,onboarding,requests,targets}.rs`, `core/src/fixtures/script.rs`,
`src/app/view.rs`, `tests/app/skeleton.rs`, `core/tests/app_wiring.rs`, new files `core/src/app/effects.rs`,
`core/src/threads/{phase,decision,history_state}.rs`. Base: `claude/tui-rtwk` 1759cf2bc.

How to read: **Need** = what the pack of the asking chunk says is missing on the shared files; **Who** = the asking chunk; **Where** = the
file (pack / brief line); **Oracle** = the codex-tui source that is the spec; **State** = `done-by-owner` (landed or in the owner's branch: not
redone here), `built` (this task, group letter), `open` (not built; the reason), `host` (host-services executes it), `view` (a view file's
owner wires it). A row's last column says which chunk can use it now.

Sources read: parity packs chat-live, cells, composer, popups, transcript, usage, chat-status, status-card, chat-commands, approvals, settings;
briefs `research/briefs/*.md` §5 (shared-file tables), and the owners' `task_checkpoint` notes of 2026-11-22. Packs and checkpoints of branches
not yet on the tip were read from the owners' branches (read-only).

## 1. App commands and events (core/src/app.rs, protocol/mod.rs)

| # | Need | Who | Where | Oracle | State |
|---|---|---|---|---|---|
| 1.1 | Drain `popovers::Model::take_asks()` after every apply: `Ask::Lookup`/`Append` (history), `Ask::FileSearch`, `Ask::RecordMention` become one command | composer | composer.md "Needs" (8a), brief composer §5 | `chat_composer_history.rs`, `file_search.rs`, `mention_usage` | built A: `Command::Popovers { ask }`, drained from `popovers::Model::take_asks()` after every apply (the composer's history/file-search/mention asks, in order, once); the host answers with `AppEv::Screen(ScreenEv::Popovers(Ev::..))`; the fake records it. Unblocks chunk-composer 8a, host-services (history, file search) |
| 1.2 | `Command::Usage { ask: usage::Ask }`, answered with `usage::Ev` under the ask's session+stamp | usage | usage.md "Needs of others"; brief usage §5 | `status/usage` analytics `usage_report` | built A: `Command::Usage { ask: usage::Ask }` (variant, `backend_of`, fake records, protocol none-lists); the page's `emit` pushes it (one line in `screens/usage/mod.rs`, usage's file); the host answers with `usage::Ev` under the ask's stamp = usage step 3 (`host/src/analytics.rs`) |
| 1.3 | `Command::GitDiff` + its answer, `/diff` into `transcript::Ev::Pager` | transcript(-2), commands | transcript.md Needs 2/3 | `get_git_diff.rs` (933 l), `app/pager.rs` | built A (Effect seam): `Intent::Effect { request, ask: Ask::GitDiff { cwd } }` -> `Command::Effect` -> host runs `diff::Diff` through its workspace executor -> `AppEv::Effect { request, done: Done::GitDiff(Result<diff::Report, diff::Failure>) }`; read with `CodexApp::effect(request)`; the words are the consumer's. Unblocks transcript-2 (`/diff`), commands |
| 1.4 | Clipboard service: `Command::Copy`, `Ev::Copied` with a typed status (native / WSL / tmux / PRIMARY) | transcript-2 (4) | transcript-2 checkpoint "UNBLOCKS (4)" | `clipboard_copy.rs` | built A (Effect seam): `Ask::Copy { text, format: CopyFormat }` -> `Done::Copy(CopyStatus::{Confirmed, Unconfirmed, Pending(n), Busy})` (oracle `clipboard_copy.rs`; `Pending` is replaced by the later answer of the same request). Unblocks transcript-2 (4); host-services implements |
| 1.5 | `Command::SuggestThreadTitle { ticket, thread }` answering the title | commands | brief chat-commands §5 (title suggestion row) | `temporary_structured_request.rs:129-145`, `/rename` empty arg | built A (Effect seam): `Ask::SuggestTitle { thread }` -> `Done::Title(Option<String>)`; the host runs the ephemeral thread/start + structured turn + unsubscribe (`backend_of` routes by the thread's owner). Unblocks commands (`/rename` without an argument) |
| 1.6 | `Command::ExportTranscript { thread, dest }` + "Saved conversation to {path}" note | commands | brief chat-commands §5 | `/export` (`slash_dispatch.rs`) | built A (Effect seam): `Ask::Export { thread, dest }` -> `Done::Export(Result<path, String>)`; the "Saved conversation to {path}" note is the consumer's (`Item::Note`, chunk-commands). Unblocks commands (`/export`) |
| 1.7 | `Command::VerifyUser { key, attempt }`, `verify::Verifications` owned by `CodexApp` (ADR-003), result events with the attempt, cancel on discard, clear on reconnect, remote workspace warns and fails `ProviderUnavailable` | approvals (6) | approvals.md Needs 6/7 | `app/user_verification*.rs`, `user_verification_tests.rs` | built A: `Intent::Screen(Approvals(Answer { Decision::Verify }))` begins one attempt (`Verifications` owned by `Effects` in `CodexApp`, ADR-003), the dialog's `waiting` flips, one `Command::VerifyUser { key, attempt, params }`; the host runs `userVerification/verify` and answers `AppEv::Verified { key, attempt, result: Verified::{Proof, Failed(data), Unreachable} }`; a result of a void attempt (cancelled, resolved by the server, reconnect) is dropped; success answers `accept` + proof, failure answers `cancel` + closed reason and keeps the cause (`CodexApp::verification_failure(&key)`: the approvals chunk words it with `Strings::verification_failure`). Remote workspace: the host answers `Failed(Some(unavailable/providerUnavailable))` and warns itself (no `workspace.remote` fact in core yet: 1.16). `cancel_thread`: the model has no thread discard to hook. Unblocks approvals 6/7; host-services runs the device |
| 1.8 | Guardian denial approve: `thread/approveGuardianDeniedAction` as a thread RPC from the slot's `Ask` | approvals (4) | approvals.md Needs 4/7 | `app_server_session.rs` approve_guardian | built C2: no `ThreadAsk` (one route, ADR-003): the slot's own `Command::Screen(ApproveDenied)` through `ThreadFx` is the wire; the app only takes `Intent::AutoReview { thread, what: AutoReview::Approve { id } }` from the picker |
| 1.9 | `resolve_request` records `Decided` on the active thread; `Intent::CancelRequests` (ctrl+c drops queued approvals unanswered); RUI Esc interrupts and drops queued RUI; label at present time, not at push | approvals (5) | approvals.md Needs 5, brief chat-approvals §5 | `chatwidget/approvals.rs`, `app/pending_interactive_replay.rs` | built B1 + C2 (`resolve_request` applies `approvals::ThreadEv::Decided` through `apply_thread` once the request is removed; `Intent::CancelRequests` records `Aborted`; test: `guardian::an_answered_request_of_the_shown_thread_is_recorded`, `a_cancelled_request_is_recorded_as_aborted`); label derivation stays `view` (presenter reads (request thread, active thread)) |
| 1.10 | `Intent::EditPrompt { thread, nth }` registration (`ScreenThreadIntent::EditPrompt` in `screens/mod.rs`, popups' file) and `backtrack::apply_thread_intent` | transcript, chunk-live | transcript.md Needs 2/6 | `app/backtrack.rs` | open: `core/src/screens/mod.rs` is chunk-popups' file; the `ThreadScreen` arm in `app.rs` routes any registered intent |
| 1.11 | `RateLimits{origin}` with `RateLimitOrigin`, `refresh_rates`, clock facts `status::Ev::Clock`, `account/updated` clears the store | status-card, usage, chat-live | status-card checkpoint bd241cb48 (branch), usage Needs, chat-live C5/C6 | `status/rate_limits.rs`, `app/background_requests.rs` | done-by-owner (status-card branch `worktree-agent-af31519f2b1f93b55`; `account/updated` done in chat-live C6): not redone, to avoid a double write of `app.rs` |
| 1.12 | `model/list`, `permissionProfile/list`, `configRequirements/read`, `collaborationMode/list`, `plugin/*`, `app/list` requests and the `app/list/updated` notification into `select::Ev` | popups | popups.md Needs, brief popups §3/§5 | `bottom_pane/list_selection_view.rs`, `model_popup.rs`, `permissions_popup.rs` | open (E): the `select` types of step 2/3 (`Ev::Fetched/Discovered/Listed`, `permissions::*_request`) are in chunk-popups' branch only; one `Command::Select` over the owner's wire/decode lands after it is on the tip |
| 1.13 | Persistence of settings/config edits: one root command for `Edit::ThemeSelected`, `SetupSaved`, `SkillToggled`, `Toggled`, `FeaturesSave`, hook edits, with a confirmation `Ev`; also `ReserveReturnModel`, model picks (`config/batchWrite`) | settings, popups, chat-live | settings.md Needs outside the area; popups; chat-live C6 | `config_panel`, `config/batchWrite` | built A (Effect seam): `Ask::ConfigWrite { edits: Vec<ConfigEdit { key_path, value }> }` -> `Done::Config(Result<(), String>)` (`config/batchWrite`, value replaces); one command for theme/setup/skills/toggles/features/hook edits/model picks. Unblocks settings, popups (`Model::picked`), chat-live C6 |
| 1.14 | Host sends `cells::Ev::{LinkDisplay, Clock, RuntimeMetrics}`; `Notice::UpdateAvailable` / `ServerVersion` appended by the app | cells | cells.md "Needs for other owners" | `update_available`, `version_notice` | built D2 (`Appended` of both notices); the Ev senders are host |
| 1.15 | `Command::OpenUrl` host arm and the fake recording it; `SendAddCreditsNudge` host arm | chat-live | chat-live C5/C6 Needs | `AppEvent::OpenUrlInBrowser` | host (the fake logs every command already) |
| 1.16 | Workspace headline (`account/workspaceMessages/read`), thread usage (`account/usage/read`), git summary probe, `workspace.remote` fact | chat-status, chat-commands | brief chat-status §5 | `status_surfaces.rs:640`, `thread_usage.rs` | open: status-card step 3 and host facts (`host/src/facts.rs`, host-services) |
| 1.17 | `Command::HooksList`, skills config, experimental feature list/save | settings | settings.md Needs | `hooks/list`, `skills/config/write` | open (E): typed as `ConfigWrite` edits where it is a write; the reads need the owner's `Open::Hooks` types after the settings pack lands |

## 2. ThreadAsk and ThreadEv (core/src/threads.rs; ADR-003: RPCs only in `ThreadFx`)

| # | Need | Who | Where | Oracle | State |
|---|---|---|---|---|---|
| 2.1 | `ThreadAsk::Review { thread, target }` (`review/start`) | commands | brief chat-commands §5 | `review/start`, `ReviewStartParams` | built B2 |
| 2.2 | `ThreadAsk::GoalSet`, `GoalClear`, `GoalGet` and `ThreadEv::Goal` into the thread's goal | commands, popups | brief chat-commands §5, popups §3 | `thread/goal/*`, `thread/goal/updated|cleared` | built B2 (the goal fact is a field of `ThreadModel`; `chat::ThreadPart` reads it) |
| 2.3 | `ThreadAsk::CleanTerminals { thread }` (`thread/backgroundTerminals/clean`) | commands | brief chat-commands §5 | `/stop` | built B2 |
| 2.4 | `ThreadAsk::SettingsUpdate` (`thread/settings/update`) for model, effort, permission, collaboration-mode picks | popups | popups.md Needs (effects of `Model::picked`) | `thread/settings/update` (`v2/thread.rs:239-283`) | built B2 (typed fields, no prose) |
| 2.5 | `ThreadAsk::{ReadTurns, Revert}` and the `ThreadReverted` arm | transcript | transcript.md Needs 3 | `app/backtrack.rs` | done-by-owner (chat-live C5, on the tip) |
| 2.6 | `ThreadAsk::Steer` and pending steers / queue categories / restore | chat-live | chat-live C2 | `input_queue`, `turn/steer` | done-by-owner (on the tip) |
| 2.7 | Guardian history drain: after `ScreenThreadEv::Approvals`, `slots.approvals.take_history()` becomes items (decision, visible warning, error, info); `TurnEnded` applied to the slot | approvals (2) | approvals.md Needs 2 | `chatwidget/approvals.rs`, `guardian review` cells | built C2 (`Item::Decision(Decided)`, `Item::Review(ReviewLine)` as interim typed values until chunk-commands' `Item::Note` lands; `append_history` after `ThreadEv::Screen`; `TurnEnded` applied in `end_turn`; tests `guardian::*` in `core/tests/app_wiring.rs`); the reviewer's warning is an `Item::Warning` (oracle `on_warning`); the "Automatic approval review approved (" notice is dropped by the slot |
| 2.8 | Routing of the three guardian notifications (`item/autoApprovalReview/started|completed`, `guardianWarning`) in `protocol/targets.rs` | approvals (1) | approvals.md Needs 1 | `protocol/common.rs` | built C2 (`targets.rs` `notification_thread`; `threads.rs` `project` chains `screens::approvals::project` after `screens::project_thread`, which is chunk-popups' file: when it adds the arm the chain entry can go) |
| 2.9 | `Item::Agent` carries the message phase (`MessagePhase`: commentary / final answer), explored-vs-ran class of a call, per-call labels and ids, the turn-completion marker, `Item::Status`, history states `Partial/Loading/Failed/Complete` | transcript-2 (2), cells, status-card | transcript-2 checkpoint "UNBLOCKS (2)", transcript.md Needs 5 | `history_cell/agent.rs`, `transcript_view/turn_details.rs`, `thread/read` history | built C1 (phase, turn marker, history state, call labels); `Item::Status` built C4 over the status-card's `Model.cell` |
| 2.10 | `tui.collapse_turn_details` reader | transcript-2 | checkpoint (2) | `turn_details` setting | open: a config fact (`StatusFacts` in chat.rs, chunk-live) |
| 2.11 | `Item::Note` / `NoteKey` and `chat::Intent::Note` | commands | chat-commands.md | `history_cell/notices.rs:334-357` | done-by-owner (chunk-commands branch a8d68e986): not redone |
| 2.12 | `Fault` arm: cyber-policy error stamps Daybreak kind and appends `Notice::CyberPolicy(kind)`; biology-policy appends `Notice::SafetyAccessBlock`; the oracle cause lines from `codex_error_info` | cells, chat-live | cells.md "Needs for other owners (chat-live)" | `chatwidget/protocol.rs`, `provider_error.rs:84-124` | built D1 |
| 2.13 | `item/plan/delta` into a streaming `Item::Plan` | cells | cells.md | `plan_stream` | built D3 |
| 2.14 | `UserPrompt` keeps local/remote images and element placeholders; composer restore | transcript, composer | transcript.md Need 3/4 | `backtrack_revert` | done-by-owner (cells `threads/user_prompt.rs`, composer 7/8) |

## 3. Fixture script (core/src/fixtures/script.rs)

| # | Need | Who | Oracle | State |
|---|---|---|---|---|
| 3.1 | `reply` step: answers the next request of a method (cells brief §5; transcript `history_pages`) | cells, transcript | n/a (test seam) | built F1 |
| 3.2 | `lagged` step | chat-live | n/a | done-by-owner |
| 3.3 | `respond` step (answering a server request) | chat-live | n/a | open: `Screen::respond` is in `tests/kit/mod.rs` (not mine) |

## 4. Not here (and why)

- chunk-live's remaining steps 4 (safety buffering), 8 (external writer), 9 (plan mode), 10 (replay `ReplayKind`) have their hunks in
  `threads.rs`/`app.rs`; chunk-live's area no longer holds them. They are not in the task's list; the next core task takes them.
- Host-side execution of every new command: host-services.
- `core/src/screens/mod.rs` registrations (`ScreenThreadIntent::EditPrompt`, the guardian `project_thread` arm): chunk-popups.

## Done / Checks

(filled per group below)
