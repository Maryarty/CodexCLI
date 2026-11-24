# core-review-a: the core hubs (app, threads, commands, rpc, protocol, ids)

Reviewer: codex-reviewer (Opus), 2026-11-24, read-only on `claude/tui-rtwk` at 8cf058010.
Scope: `codex-rs/tui-rtwk/core/src/` app.rs, app/, threads.rs, threads/, commands.rs, commands/,
rpc.rs, protocol/, ids.rs, lib.rs. I also read the rules (`tests/rules/{layers,adr013,registration}.rs`,
`hub_budget.txt`), the host's reconnect path and the W2 branches' diffs against these files.
I ran no tests (code review, no diff). Line numbers are at 8cf058010.

## (a) Verdict

1. ADR-003 ownership holds by type. The only writer of a `ThreadModel` is `threads::apply` (via `ThreadSet::get_mut`, app.rs:3228), and the model is sealed. ADR-012's doors hold as well.
2. One defect: a lost remote connection never fails the app-scoped asks in flight and never tells the features (F1). Reading the code, a `/plugins` listing stays `Loading` for the rest of the process.
3. One perf risk: the VS1 copy-on-write class (P1, `ScreenStates`) is back on the shown `ThreadModel`. ADR-003 requires a budget test for this, and there is none (F2, F3).
4. Three seams still make packages edit hub files: host channels written as `Command` variants (F5), dispatch gates plus hand-written test tables (F6), and thread asks decoded as an agents screen's replies (F4).
5. app.rs and threads.rs each mix 5-6 concerns. They are the week's top two hotspots (124 and 99 commits), so splitting them is about merge conflicts, not size (F8).

## (b) Findings

Kinds: D defect, R risk, N note. Size = lines touched. Clash = what running work it meets
(w2-commands: `core/src/features/{daemon,feedback,recap,security_setup,side,small_cmds,windows_sandbox}`;
rtwk-repin-5: Cargo + selection; w2-status / land/w2-b still edit `commands/dispatch.rs`).

| id | kind | file:line | problem | why it matters | proposed refactor | size | risk | rules test | clash |
|---|---|---|---|---|---|---|---|---|---|
| F1 | D | app.rs:1651-1690, :1207-1223, :1029-1053; host/src/in_process.rs:584 | Three code paths handle a lost connection, and each does a different subset. On a remote reconnect, `Feature`, `Structured`, `Select` and `Opening` asks are never answered (`_ => true`, :1687). Features never get `on_connection(Lost/Up)`, and interests are not asked again | stuck `Loading`, stale `Remote` (R6 class) | `app/connection.rs`: one `lost(app, backend)` and one `up(app, backend)`. `AppCtx::on_lost() -> Fail\|Keep` as an exhaustive match. Connection facts become `AppEv`, not `agents::Ev` | ~150 | M | yes: no `_` arm in an `AppCtx` match; a test over every `AppCtx` variant on Reconnecting | none |
| F2 | R | threads.rs:1268-1270, :1099-1176, :586; app.rs:977-983; src/app/view.rs:140 | The UI provider holds the old `Rc<ThreadModel>`, so the first apply after each frame deep-copies the model. That includes `submissions` (every prompt of the session), the popovers' skills and plugins, the queue, `streaming`, all `ThreadSlots` and parts, and the streaming item's text | per-frame cost grows with session length; perf must beat the oracle | `Shared<>` per slot in `ThreadSlots` / `FeatureThreadParts` (generated, as `ScreenStates` does, screens/mod.rs:115-122). `Rc` around `submissions` and `streaming` | ~60 | L | yes: an allocation-budget test (missing; ADR-003 requires one) | none |
| F3 | R | app.rs:1111-1124; app/mentions.rs:31-38; app/features.rs:91-107; threads.rs:2352-2361, :3199 | Global passes run after every event, every delta included. `mentions::sync` compares three `Vec`s deep (it also keeps a derived catalog in sync by hand, an R7 departure). `features::wake` costs O(threads × features). Each delta clones three id `String`s and the delta, plus `key.turn.clone()` for `flush` even when there is no streak | hot path | Mention catalog as a view memo, or keyed by revision. Recompute `wake` only when a step touched the features or threads. `project` takes the boxed notification by value. `flush(Option<&TurnId>)` | ~80 | L | the F2 budget test | none |
| F4 | R | threads.rs:1609-1988; protocol/mod.rs:50-247; protocol/agents.rs:170-260 | The thread lifecycle RPCs are split. `ThreadAsk` (hand-written `wire`) sits beside app `Command::{StartThread,StartTurn,InterruptTurn}`, with their own hand-written wire and decode. There are three `thread/start` builders (threads.rs:1813, :2009, protocol/mod.rs:71) and two `turn/start` builders. Replies are decoded as `agents::Reply`, and the app names `agents::{Reply,Ctx,Command::Thread}` 40 times. `ThreadAsk` also carries `config/read` (`McpServers`). Ids are `String` in 48 fields (R13) | each new lifecycle RPC touches 4-5 files (fix-recap-title added 3); a screen type sits in the app layer | `threads/asks.rs` as one `rpc_list!` with its own `ThreadReply`, sent as `Command::Thread { ticket, thread: ThreadId, ask }`. Drop the protocol/mod.rs exemption from `thread_rpcs_only_in_threads` | ~500 (mostly moves) | M | yes: extend `wire_is_generated` to threads.rs and protocol/mod.rs; ratchet `agents::Reply::` in app/; R13 ratchet on `thread: String` | none |
| F5 | R (seam) | app.rs:363-495; protocol/mod.rs:110-125, :231-246; src/services/backend/fake.rs:516-551; host/src/effects.rs:259 | Per-screen host channels as hub variants: `Usage`, `Popovers`, `Import`, `Startup`, `Directories`, `SaveGrouping`, `CreateWorktree`, `BindWorktree`, `VerifyUser`, `OpenUrl`. Each one needs two `None` arms, a host arm and a fake arm. w2-status adds a hand-written `Usage` Redeem responder to fake.rs. `Command::OpenUrl` duplicates `Ask::OpenUrl` (app/effects.rs:141) | porting speed: shared host/fake edits in every package | Make them lines of `host_capability_list!`. The fake implements `HostDoors` once; RPC-shaped ones (Redeem) go through the generic RPC path | ~300 | M | yes: a `Command` budget that only goes down, plus "`client_request` answers `None` only for Effect and request answers" | w2-status (fake.rs) |
| F6 | R (seam) | commands/dispatch.rs:416-451; core/tests/commands.rs:1139-1150 | When a slice's command needs a precondition, the package edits `dispatch_bare` (w2-status: `/usage` login gate). Every command moved into a slice edits the hand-written target table (2ed379c64 `/feedback`, land/w2-b `/usage`) | recurring hub edits | `SlashCommand::needs() -> Needs{Thread, BackendAuth, Session}`, a table beside `available_during_task` (commands.rs:402-478), checked once in `dispatch_command`. The test derives the `Target::Feature` rows from `features::target_of`; only non-feature rows stay hand-written | ~60 | L | the generated table is itself the test | w2-status, land/w2-b (land first) |
| F7 | R | app.rs:1395-1405, :3041-3080; app/session.rs (11); screens/agents/mod.rs:462 | Domain held in screen states. Routing (who owns a thread, which epoch, what gets detached) reads `app.screens.agents.owners` (29 reads). Config facts are read from `app.screens.chat.status` (22 reads) | a screen seed or reset changes routing; per-thread data outside `ThreadSet` (R2) | Move ownership into `ThreadSet` (catalog) and config facts into `AppFacts`/`tui` | ~120 | M | yes: ratchet `screens.agents.owner` and `screens.chat.status` in app/ | none |
| F8 | N (structure) | app.rs:1143-1760, :2128-2353; threads.rs:292-745, :1609-2030, :2228-2430, :2659-3270 | `apply_event` is 620 lines: connection, rate limits (:1228-1267 plus the reply at :1910-1957), the account boundary, a hand-ordered fan-out to about 12 observers (:1328-1460), 16 `ScreenEv::` peeks and fixture seeds. `apply_intent` has 14 `ScreenIntent::` peeks. threads.rs holds Transcript, asks, projection and apply | merge conflicts, not size | Moves: `app/connection.rs` (F1), `app/rates.rs`, `app/ingress.rs` (the fan-out written once, as a list), `threads/transcript.rs`, `threads/asks.rs` (F4), `threads/project.rs` | ~1500 moved | L logic, H conflicts | yes: `arms core/src/app.rs apply_intent ScreenIntent:: 14`, `apply_event ScreenEv:: 16` | every package that edits app.rs: do between waves |
| F9 | N | app.rs:1613-1647; threads.rs:3083-3098; protocol/agents.rs:348 | Live `apply` has fixture-only events: `agents::Ev::Current` opens a model and sets `active` with no resume; `chat::Ev::Pending` overwrites the queue and steers; `approvals::Presented` and `status::Card` do the same kind of thing. Only script decoders produce them | breaks the fixture rule "a script cannot build a state the app cannot reach" | `AppEv::Seed(Seed)` in `app/seed.rs`, the debug menu's one door | ~80 | L | yes: `Seed` is built only in fixtures/ and the script decoders | none |
| F10 | N | threads.rs:2941; tests/rules/layers.rs:135-163 | User-visible prose in core ("Couldn’t continue this chat. …"). The prose rule scans only `format!("…")` | ADR-010 / layers | Use a `Failure` variant; the rule also scans `"…".to_owned()/into()` literals with ≥3 words | ~30 | L | yes (the rule extension) | none |
| F11 | N | app.rs:103-114 | `switches`, `removed` and `fork_failed` are one-shot signals written as counters the UI diffs | ADR-013 Ж says `Directive<T>` with `seq` | Move to `Directive` the next time the session page changes | ~40 | L | `adr013_directives_read_in_one_place` could count them | none |
| F12 | N | threads.rs:1559; app/features.rs:37-60; features/side/mod.rs:81 | `can_retry_safety` has no caller. `run` does `make_mut` of `FeatureStates` on every hook, and `on_connection`/`on_shown` reach every slice's `Shared` mutably. Side projects every `ItemStarted` of every thread | dead code; copies on hooks | Delete it; the generated dispatch skips slices with the default hook (overlaps core-review-b) | ~40 | L | no | w2-commands (side): touch only app/features.rs |

### F1 in detail (the defect)

- **How the remote host reports a loss.** `host/src/in_process.rs:576-590` drops the in-flight futures and calls `reconnect::restore`. That sends only `ScreenEv::Agents(Reconnecting)` and later `Reconnected` (`src/services/backend/reconnect.rs:40,49`). `AppEv::Disconnected` is sent only by the embedded server and the cloud backend.
- **What the Reconnecting arm does** (app.rs:1653-1722). It drops some `AppCtx` and fails resume pages. Everything else is kept by `_ => true`.
- **Why nothing else catches it.** `fail_in_flight` (:1029) would fail those asks, but it runs only from `attach_backend`, which the shell calls once per backend at start (src/services/shell.rs:180,203). The `Reconnected` path (:1724-1757) calls neither `features::reconnected` (:1198) nor `features::connection(Up)`, and nothing calls `structured::lost`.
- **Failing input** (from reading the code, not reproduced):
  1. Start `codex-rtwk` against a remote app server.
  2. Open `/plugins`, which raises `Interest::Listing` and calls `prefetch`.
  3. Drop the server before `plugin/list` answers. It reconnects.
  4. Close and reopen `/plugins`: `prefetch` returns early because `catalog.is_loading()` (features/plugins/state.rs:557-565), so the list loads forever.

  `/recap` in the same window ends as `TimedOut` after 30 s, not `Lost`. `/model`'s `LoadModels` (`AppCtx::Select`) stays loading.
- **Oracle.** It shows no stuck list: a request on a lost connection errors out, and the overview refreshes after `finish_reconnect`.
- **Fix.** One `connection::lost`. It runs `fail_in_flight(backend)`, `structured::lost`, `features::connection(Lost)` and the cleanup shared with Disconnected (onboarding, `reset_requests`, banners, reserve, `detach_threads`). One `connection::up` runs `features::reconnected` and `features::connection(Up)`. `AppCtx` gets an exhaustive `on_lost` so that a new variant has to choose.

### F2 in detail (the copy-on-write class, again)

`app.core.active_thread()` clones the `Rc<ThreadModel>` into `Provider::value_shared` on every build. The next `threads.get_mut` therefore does `Rc::make_mut` on a shared `Rc`. That clones `ThreadCore` with all its non-`Rc` fields:
- `queue`, `pending_steers`, `rejected_steers`, `hooks`, `completed_items`;
- `submissions` (`replay::Submissions`, every replayed prompt with its images and elements);
- `processes`;
- the `streaming` map, and then the streaming item's whole text through `Rc::make_mut(&mut live.item)`.

It also clones `ThreadSlots` (popovers' `Option<Vec<Mention>>` ×2, the chat part) and `FeatureThreadParts`. VS1 fixed exactly this for `ScreenStates` with `Shared<T>`; the per-thread parts never got it.

ADR-003 says deps must be "(rev, Rc<T>) over structurally shared items" and asks for "a per-release budget test on the scale scenario". The repository has no allocation-counting test (searched `tests/`, `core/tests/`).

Proposed test: a counting `#[global_allocator]` in one core test binary. Hold the previous `Rc` of a thread with 500 prompts and 50 skills, apply 100 `AgentDelta`s across 10 "frames", and assert fewer than K allocations and that no `Submission` is copied.

### F4 / F5 / F6: the seams, and the door for each

| seam (who had to edit it in W1/W2) | door |
|---|---|
| a lifecycle RPC: threads.rs `ThreadAsk` + `wire`, protocol/agents.rs decode, `agents::Reply`, app.rs match (fix-recap-title 771377e0e) | `threads/asks.rs` `rpc_list!` + `ThreadReply`; the app matches its own reply type |
| a host channel: `Command` variant, two `None` arms, host arm, fake arm (w2-status fake.rs, fix-stream-lost 34ec1e886 `ReconnectIdleThread`) | a line of `host_capability_list!`; the fake is one `impl HostDoors` with a responder fallback |
| a slice's slash precondition: `dispatch_bare` (w2-status) | `SlashCommand::needs()` data table |
| a slice's command target: `core/tests/commands.rs` table (2ed379c64, land/w2-b) | rows derived from `features::target_of`; the slice's own `core/tests/<f>.rs` asserts its route |
| a UI page's helper module: `src/pages/session/live/mod.rs` `mod limits;` (w2-status) | one sorted line, harmless; extend the registry only if it starts to conflict |

`features/mod.rs` (core and UI) is generated (fw-hubs-c5) and was not a seam in W2 except for w2-composer's queue hooks, which are core-review-b's.

## Q1: what breaks a decision and no rule catches (each one could be a test)

| decision | escape | test that would catch it |
|---|---|---|
| R6 / ADR-003 crit. 3: every async result is answered or dropped by id | F1: `_ => true` keeps asks forever | lint: no wildcard arm in a match over `AppCtx`; behaviour test: on Reconnecting, every variant in flight is answered |
| ADR-003 "structurally shared", budget test | F2/F3 | the allocation-budget test (ADR-003 lists it; not written) |
| R2 per-thread data in `ThreadModel` / `ThreadSet` | F7: thread ownership in `screens.agents` | ratchet on `screens.agents.owner` in app/ |
| R3 thread RPCs only in `ThreadAsk` | F4: the protocol/mod.rs exemption, replies typed by a screen | `wire_is_generated` over threads.rs and protocol/mod.rs; `app_names_no_screen_reply` |
| R7 derived values not stored | F3: the mention catalog kept in sync after every apply | review item (hard to lint); a memo removes it |
| R13 a newtype per id | 48 `String` thread/turn fields in hubs and agents | ratchet on `(thread\|turn\|parent): String` in core |
| ADR-012 hub shrinks | app `apply_intent` / `apply_event` peek into screens (14 + 16) with no budget | two `arms` lines in hub_budget.txt |
| fixtures through the live path | F9 seed events bypass the lifecycle | `Seed` built only by the script |
| no prose in core | F10 | prose rule over string literals |

## (c) Top 5 by payoff / cost

1. **F1 one connection path** (~150 lines, fixes a defect, creates `app/connection.rs` for F8). Do now; it meets no running package.
2. **F2 + its budget test** (~140 lines). This is the perf guarantee ADR-003 already promised; `Shared<>` is a proven pattern here. The same test covers F3.
3. **F6 `needs()` table + derived command test** (~60 lines). It removes the two hub edits every slash-command slice made in W2. Land after w2-status / land/w2-b.
4. **F4 thread asks as `rpc_list!` + `ThreadReply`** (~500 lines, mostly moves). It ends the agents-screen coupling, the triple `thread/start` builders and the R3 exemption. Do it between waves, with F8's threads.rs moves.
5. **F5 host channels into `host_capability_list!`** (~300 lines). It ends fake.rs / in_process.rs / protocol edits per screen and the duplicate `OpenUrl`. Coordinate with whoever owns host/ next wave.

Next after these: F8's app.rs moves (between waves only), then F7 (ownership into `ThreadSet`).

## Decisions for the person / orchestrator

- F1: a debt task now (defect, no gap row). It does not belong to a running package.
- F2's budget test: who writes it, and the number K (measure first, then fix it as a ratchet).
- F4/F5/F8 change the shape of hub enums (`Command`, `ThreadAsk`, `AppEv`). Each needs a budget paragraph in ADR-012, and they should be scheduled between waves, not alongside W2 packages.
