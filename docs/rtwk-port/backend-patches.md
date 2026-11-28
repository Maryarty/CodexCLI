# Backend patches

The backend (`codex-core`, `codex-app-server-*`, `codex-protocol` and the rest of `codex-rs`
outside `tui*`) is upstream. Where the protocol lacks a fact the oracle has, the port carries a
patch, never an ad-hoc edit (person, 2026-11-28: "patch the backend, keep a catalog so the backend
can be updated calmly"). A patch is one row here, one series of `git format-patch` files in
`tools/rtwk-port/backend-patches/series/`, and one branch of this repository. `BACKEND_REF` is
the tip of the series, so the pin (docs/rtwk-port/README.md "Backend pin") is upstream plus
exactly the rows below. `tests/rules/backend_patches.rs` fails when `BACKEND_REF` is not
`series/TIP`, or when the catalog lacks the base, the tip or a series file.

| id | what the protocol lacks | base | tip (`BACKEND_REF`) | upstream proposal |
|---|---|---|---|---|
| [BP-1](#bp-1-turnruntimemetrics) | per-turn Responses WebSocket timing (`turn/runtimeMetrics`) | `97400b8a7da207a59f2fce7bcb320a6a2b4d4dc1` (rust-v0.170.0-780-g97400b8a7d) | `e8ddca3f2163186149ab1e79e132ce12f7e9e6b0` | none filed |

## BP-1 turnruntimemetrics

**Why.** The oracle draws the WebSocket timing rows ("WebSocket timing: ..." and the totals on the
final separator) from the telemetry of its own process: `chatwidget/turn_runtime.rs:60`
`collect_runtime_metrics_delta` reads `session_telemetry.runtime_metrics_summary()`, `:102`
resets it at the turn start, `:175` collects it at the turn end. That delta is process-wide, so
two parallel turns mix, and an external client (the port talks to the backend through the
app-server protocol, in process or remote) has no such signal: the v2 turn notifications carry
no timing (`app-server-protocol/src/protocol/v2/turn.rs`, `TurnCompletedNotification`), the
pinned core logs TTFT/TBT as TRACE events without a thread or turn id, and a scoped tracing
subscriber is lost when the embedded server spawns (loopback experiment of 2026-11-28: four exact
samples with a process-wide layer, none with a scoped one). Owner and turn must come from the
transport, so the backend publishes the sample addressed.

**What.** With the existing `RuntimeMetrics` feature and the client's experimental-API opt-in,
every Responses WebSocket sample the client recognises inside the current request's
`response.created`..`response.completed` window becomes the notification `turn/runtimeMetrics`
(`TurnRuntimeMetricsNotification`: `threadId`, `turnId` and any valid of `overheadMs`,
`inferenceTimeMs`, `iapiTtftMs`, `serviceTtftMs`, `iapiTbtMs`, `serviceTbtMs`; finite
non-negative milliseconds, zero and fractions kept, a frame with no valid field publishes
nothing). Warmup and pre-creation frames are excluded; HTTP SSE and providers without these
events give nothing. The embedded bounded bridge keeps this low-rate event (delivery class of the
other turn notifications). Legacy clients, a client that opts out of the method and
`RuntimeMetrics` off get zero samples (wire matrix 6/0/0/0).

**Commits** (branch `origin/codex/backend-runtime-metrics-0.170`, series file in
`tools/rtwk-port/backend-patches/series/`; `generated` patches are not applied to a new base,
the release's generators write those files again):

| # | commit | kind | series file | what |
|---|---|---|---|---|
| 1 | `63ddeb2e1e4ff621852f6cc60797ca08646274f3` | source | `0001-s10-websocket-timing-backend-publish-addressed-runti.patch` | the event and notification, the WebSocket client parser and response window, the core hook |
| 2 | `c55e62a4960b6514e8ecb389a96afe90ee57fd3b` | source + test | `0002-s10-websocket-timing-backend-preserve-timing-across-.patch` | the embedded bridge keeps it under backpressure; the app-server integration test |
| 3 | `4f79b980f0311f84d3f1b18b6ef56e46ec38b288` | generated | `0003-s10-websocket-timing-backend-regenerate-experimental.patch` | `app-server-exports-experimental.json.zst` |
| 4 | `6a7dfdb595c5d0a37b0c5a80d3608f82b2821adb` | generated | `0004-s10-websocket-timing-backend-refresh-stable-protocol.patch` | stable JSON/TS schema, `app-server-exports-stable.json.zst`, Python SDK types |
| 5 | `e8ddca3f2163186149ab1e79e132ce12f7e9e6b0` | test | `0005-s10-websocket-timing-backend-exercise-legacy-thread-.patch` | the legacy-client case starts the thread without experimental params |

**Crates touched** (all under `codex-rs/`): `app-server-protocol` (`protocol/common.rs`,
`protocol/v2/turn.rs`), `app-server` (`bespoke_event_handling.rs`, `in_process.rs`,
`notification_media.rs`), `codex-api` (`common.rs`, `endpoint/responses_websocket.rs` and its
`connector.rs`, `lib.rs`), `core` (`client.rs`, `session/turn.rs`, `turn_timing.rs`,
`guardian/response_resampling.rs`), `otel` (`events/session_telemetry.rs`), `protocol`
(`protocol.rs`: `EventMsg::TurnRuntimeMetrics`), `rollout` (`policy.rs`), `rollout-trace`
(`protocol_event.rs`); the new variant is also a new arm in every exhaustive match over
`ServerNotification` / `EventMsg` (the ratatui `tui/` has two, which the patch leaves alone: the
oracle binary is a separate build of the unpatched 0.170).

**Generated artifacts** (patches 3 and 4; on a new base `apply.sh` regenerates them with the
release's `app-server-protocol/scripts/write_schema_fixtures.py`, both modes):
`app-server-protocol/schema/json/{ServerNotification.json, codex_app_server_protocol.schemas.json,
codex_app_server_protocol.v2.schemas.json, v2/TurnRuntimeMetricsNotification.json}`,
`schema/typescript/{ServerNotification.ts, ServerNotificationEnvelope.ts,
v2/TurnRuntimeMetricsNotification.ts, v2/index.ts}`,
`schema/precomputed/app-server-exports-{stable,experimental}.json.zst`,
`sdk/python/src/openai_codex/generated/{notification_registry.py, v2_all.py}`. On the 0.170 base
the Python generator drifts from the checked-in SDK for unrelated fields (older reviewed MCP
scope and model-provider fields), so patch 4 carries only the two new typed models and their
registry/union entries (three-way merge of original, baseline-generated and new-generated).

**Proof.** Backend: `app-server` integration test
`runtime_metrics::websocket_runtime_metrics_follow_the_owning_turn` (4 cases: on, feature off,
legacy client, method opt-out; 6.36 s), the unit test
`websocket_timing_keeps_valid_siblings_and_rejects_empty_samples` (`codex-api`), the TS/JSON schema
fixture tests `schema_fixtures_tests::{typescript,json}_schema_fixtures_match_generated` 2/2, and
a replay over the real app-server binary and a local Responses WebSocket: two overlapping turns
gave six samples, each on its own turn (TTFT 120 to 80 ms and 220 to 180 ms), with the red control
(`63ddeb2e1` alone emitted the two pre-creation values). Port: `core/tests/runtime_metrics.rs`
(`addressed_websocket_timing_keeps_live_rows_and_scopes_the_latest_fields`,
`failed_without_final_answer_and_old_backend_keep_their_own_completion_behavior`), oracle
`chat_status_2::runtime_metrics_websocket_timing_logs_and_final_separator_sums_totals`, and a real
PTY run against a Responses WebSocket (two rows before the end, 120/50 then 80, one final
separator 80/50). Reports: tm task `s10-websocket-timing-backend` and `s10-runtime-metrics-source`.

**Port side.** `core/src/protocol/targets.rs` (`notification_thread`, the exhaustive match),
`core/src/app.rs` (`Ev::RuntimeMetrics` to the owning thread; a sample of an unknown thread is
dropped), `core/src/threads.rs` (`Item::WebSocketTiming`), `core/src/screens/cells/metrics.rs`,
`src/pages/session/cells/websocket_timing.rs`. Against a backend without the patch (an external
app-server of upstream) no notification arrives and no timing row or total is drawn.

**Upstream proposal.** Not filed; no issue or PR is recorded as of 2026-11-28. The series stays
until upstream publishes a per-turn timing event; then this row is dropped (delete the series,
`BACKEND_REF` moves to the upstream tag, the rules test follows).

## Updating the backend

1. Pick the release (`rust-v0.172.0`, or a commit). `tools/rtwk-port/backend-patches/apply.sh
   <tag>` opens a new worktree and branch on it, `git am --3way`s the source patches, runs the
   release's schema and SDK generators and commits what changed. It prints what did not land
   (the patch, the conflicted files, `git apply --check` of the rest) and exits 1 then; fix the
   patch on a backend branch and rerun with another `--worktree`/`--branch`.
2. Build the port against the new tip: move the backend pin
   (`git -C ~/.cache/codexcli/backend-pin checkout --detach <tip>`; a flag day, every worktree
   moves with `BACKEND_REF`), write `<tip>` into `docs/rtwk-port/BACKEND_REF`, and fix the
   exhaustive matches the release added (`notification_thread` and friends).
3. `tools/rtwk-port/backend-patches/export.sh <new base> <tip>` rewrites `series/` (files,
   `BASE`, `TIP`); then edit the BP rows above (base, tip, commits, series files).
   `--test rules` checks `BACKEND_REF == series/TIP` and that the catalog names every file.
4. `apply.sh <series BASE> --verbatim --worktree <scratch> --branch <scratch>` applies all five
   patches including the generated ones to the series' own base and compares the tree with
   `TIP`: the proof that `series/` is complete.
5. The oracle (`ORACLE_REF`, `codex-rs/tui`) moves on its own; the patch does not touch it.

## Transfer trials

- 2026-11-28, `rust-v0.170.0-780` to `rust-v0.172.0` (`3d86e43b075adeba2193ec5c22ac92f55afd82e6`),
  `apply.sh 3d86e43b075adeba2193ec5c22ac92f55afd82e6`: patches 1, 2 and 5 applied by `git am
  --3way` without a conflict, 3 and 4 skipped as generated, the release's own
  `write_schema_fixtures.py` (both modes) and the Python SDK generator wrote the same 12 files as
  the 0.170 series (+348/-2, the Python SDK regenerates cleanly on the release) and `apply.sh`
  committed them. Tip `5ac295bf0499a87e90bbef079332b0e73d16469a`, local branch
  `codex/backend-runtime-metrics-0.172` (not pushed). `cargo check -p codex-app-server -p
  codex-api --tests` is clean (the three `unused import` warnings are upstream's `sleep.rs`), and
  `cargo test -p codex-app-server --test all runtime_metrics::` passes 4/4 (on, feature off,
  legacy client, method opt-out). Not done: the port against this tip (the pin move, the
  exhaustive matches 0.172 may add, `ORACLE_REF` 0.172 is its own wave).
- The failure path was run on `rust-v0.17.0` (`--no-generate`): the script printed the patch that
  does not apply, the conflicted files, the `git apply --check` of every later patch, and left
  the worktree on the last good tree.
- `apply.sh 97400b8a7da207a59f2fce7bcb320a6a2b4d4dc1 --verbatim` reproduces the tree of
  `e8ddca3f` (the series is complete).

## Known gaps

- `BACKEND_REF` moves the shared pin: until `git -C ~/.cache/codexcli/backend-pin checkout --detach
  <BACKEND_REF>` is run, `pin.rs` is red and the port does not build (it names
  `TurnRuntimeMetrics`). A worktree that does not carry the port's arm in
  `core/src/protocol/targets.rs` does not build once the pin moved: land the move with the merge.
- The ratatui `codex-rs/tui` (the oracle's source) does not compile against a patched backend
  (two exhaustive matches, `chatwidget/protocol.rs` and `app/app_server_event_targets.rs`); the
  oracle binary (`~/.cache/codexcli/oracle-bin`) is built from the unpatched `ORACLE_REF`.
