# mock-model: a mock model for both binaries

Codex's own `MockResponsesServer` (`sdk/python/tests/app_server_harness.py`)
serves named scenarios to `codex-rtwk` and to the oracle `codex`, so a screen
can be tried, compared and measured without a model. Nothing here is a server
of our own: the harness is imported by path and never copied or edited.
Tools live in `tools/rtwk-port/mock-model/`; Python is run as `python3 -I`,
stdlib only.

scenario-harness (pty scenarios) and codex-perf-baseline use this server; perf
drops its planned `perf/fake_model.py` (`text-100` and `text-1000` are its
streams).

## Run

```
tools/rtwk-port/mock-model/try.sh --list                 # scenarios
tools/rtwk-port/mock-model/try.sh markdown rtwk          # codex-rtwk, this terminal
tools/rtwk-port/mock-model/try.sh markdown oracle        # oracle codex, this terminal
python3 -I tools/rtwk-port/mock-model/check.py           # every scenario, every built binary
python3 -I tools/rtwk-port/mock-model/serve.py text-1000 # only the server: prints MOCK_URL=...
```

`try.sh` starts `serve.py`, writes a temporary CODEX_HOME and workdir
(`home.py`; removed on exit, `KEEP=1` keeps them) and prints the prompts to
type, in order. Binaries come from `bins.py` (never built here): `RTWK_BIN`,
else the newest built `codex-rs/tui-rtwk/target/{debug,release}/codex-rtwk` in
this tree, the main checkout or a worktree; `ORACLE_BIN`, else
`~/.cache/codexcli/oracle-bin/codex`, else `codex-rs/target/{debug,release}/codex`.
`~/.codex` is never read or written: CODEX_HOME is the temp dir.
The oracle is started with `-c analytics.enabled=false -C <workdir>`;
`codex-rtwk` takes no such arguments (its command line is `--debug-screens`,
`--model`, `--locale`), it runs in the workdir.

## Files

| file | what |
|---|---|
| `harness.py` | loads `app_server_harness.py` by path (a stub for its one foreign import, `openai_codex.CodexConfig`, a type used only by `AppServerHarness`) and re-exports its `MockResponsesServer`, `sse`, `ev_*` |
| `events.py` | the event shapes the Python harness lacks, each naming the Rust helper it mirrors |
| `scenario.py` | `Scenario` / `Turn` / `Resp`, builders `text_response`, `tool_response`, the loader |
| `scenarios/*.py` | one file per scenario |
| `server.py` | `ScenarioServer`: the harness server plus a title-request hook (below) |
| `serve.py`, `home.py`, `bins.py`, `try.sh`, `check.py` | the tools |

### Why `ScenarioServer` adds a hook

An interactive Codex sends a title request (a `thread_source` other than `user` in the
body: `system` in the tree of `resize_reflow.rs`, `thread_title` in the installed 0.167,
found by running it) next to the user's turn. The harness queue is FIFO, so the title request
would take the turn's response. codex-tui's own `resize_reflow.rs`
(`mount_resize_response`) solves it by matching the body; `ScenarioServer`
overrides the harness's private `_record_request` / `_next_response` to answer
a title request with a title and leave the queue alone. This is the only
behaviour added to the harness server. `codex exec` sends no title request
(check.py reports 0).

## Scenarios

Each is a list of turns (a prompt, and the responses the model gives while the
turn runs: a tool call needs a following response). `pace` is the pause after
each SSE event (`delay_between_events_s` of the harness; sleep-paced, so the
rate is nominal; text-1000 measures about 600-900 events/s in practice).

| scenario | prompts | responses | what it shows | config |
|---|---|---|---|---|
| `text-100` | 1 | 1 | 600 one-word deltas at 100 events/s | never |
| `text-1000` | 1 | 1 | 6000 one-word deltas at 1000 events/s (perf) | never |
| `markdown` | 1 | 1 | heading, lists, rust code block, table, quote, inline code; 16 chars per delta at 200/s | never |
| `reasoning` | 1 | 1 | reasoning summary deltas and item, then a streamed answer | never |
| `exec-approval` | 1 | 2 | `exec_command` with `sandbox_permissions = require_escalated` + justification | on-request, read-only; one approval prompt |
| `patch-approval` | 1 | 2 | freeform `apply_patch`: update `notes.txt` (seeded), add `added-by-mock.txt` | on-request, read-only; one approval prompt |
| `web-search` | 1 | 1 | `web_search_call` in progress, then completed with a query, then an answer | never |
| `error-retry` | 1 | 2 | `response.failed` (`server_error`, retryable) after a partial delta; the client reconnects, the retry answers | `stream_max_retries = 2` |
| `error-fatal` | 2 | 2 | `insufficient_quota` (not retried): the turn ends in an error; the second prompt answers | never |
| `long-transcript` | 1 | 41 | one turn of 40 rounds (a message, an `echo step N` command), then a long list; `MOCK_LONG_ROUNDS` sets the count | never |
| `multi-turn` | 3 | 3 | greeting, code answer, summary | never |

"Many turns" is built as many rounds inside one turn (`long-transcript`), which
a single prompt reaches in both binaries; `multi-turn` is the several-prompt
conversation. Every scenario ends its last answer with a sentinel token
(`END-<name>`).

### CODEX_HOME (`home.py`)

The recipe of `codex-rs/tui/tests/suite/resize_reflow.rs` (`write_config`,
`write_auth`: model, `model_provider`, `[model_providers.X]` with
`wire_api = "responses"`, `supports_websockets = false`, the workdir trusted,
`auth.json` with a dummy key) and of the harness's `_write_config`
(`request_max_retries = 0`, `stream_max_retries`), plus the scenario's
`approval_policy` and `sandbox_mode`. The model is `gpt-6-sol`, a slug in the
bundled `models.json` with `apply_patch_tool_type = freeform`; an unknown slug
(the old `gpt-5.4` of the resize test) gets fallback metadata and the
`apply_patch` custom tool is then answered "unsupported custom tool call".

## Where each event shape came from

| shape | from |
|---|---|
| `sse`, `ev_response_created`, `ev_completed`, `ev_assistant_message`, `ev_message_item_added`, `ev_output_text_delta`, `ev_function_call`, `ev_failed` | `sdk/python/tests/app_server_harness.py`, imported by path |
| `ev_reasoning_item`, `_added`, `ev_reasoning_summary_text_delta`, `ev_reasoning_text_delta` | `codex-rs/core/tests/common/responses.rs:911,943,959,967` |
| `ev_web_search_call_added_partial`, `ev_web_search_call_done` | `responses.rs:975,986` |
| `ev_apply_patch_custom_tool_call` | `responses.rs:1091` |
| `exec_command_call` (`cmd`, `yield_time_ms`, `sandbox_permissions`, `justification`) | `responses.rs:1103` (`ev_exec_command_call_with_args`) and `core/tests/suite/approvals.rs:361` (`exec_command_event`) |
| `ev_failed_with_code` | `responses.rs:1120` (`sse_failed`); the fatal/retry code classes from `codex-api/src/sse/responses.rs` (tests at 1151) |
| `add_file_patch` (kept for patches with an Add hunk) | `approvals.rs:310` (`build_add_file_patch`) |
| approval policy for an approval prompt | `app-server-test-client` `trigger-cmd-approval` / `trigger-patch-approval` (OnRequest + read-only); it has no event shapes of its own (it talks to a real model) |
| title-request match | `resize_reflow.rs` `mount_resize_response` |

Not used: `scripts/mock_responses_websocket_server.py` and `streaming_sse.rs`
(WebSocket and Rust-only servers); the scenarios serve HTTP SSE
(`supports_websockets = false`).

## check.py

For each built binary and each scenario, a fresh server, home and workdir.
- Oracle: `codex exec --json` per turn (`exec resume --last` for the next
  turns). Asserts `turn.completed` / `turn.failed` as the turn's `outcome`, the
  sentinel in the last `agent_message`, every queued response served, and every
  tool call answered by an output item that is not "unsupported".
  `exec` forces the approval policy to Never, so approval prompts do not
  appear there: the tool call is rejected by policy and the turn ends; the
  prompts are for `try.sh` / the pty driver.
- `codex-rtwk` has no exec mode, so check.py drives it in a pty (120x40,
  answers the cursor and device queries), types each prompt and Enter, sends
  `y` when an approval prompt shows, then Ctrl+C. Pass: every queued response
  served, every tool call answered, process up until the quit. "sentinel on
  screen" is reported next to it (a VT-less text match with whitespace removed).
- `--pty` drives the oracle in the same pty driver.
- A binary that is not built is reported and skipped; nothing is built.

## Result of the one run (2026-11-18)

Binaries: the oracle build (`~/.cache/codexcli/oracle-bin/`, `codex-rs/target`)
does not exist yet, so no oracle was run. Stand-in, named by `ORACLE_BIN`:
the installed `codex-cli 0.167.0` (`~/.local/bin/codex`; not the pinned oracle,
only a real Codex binary; it talks to the local mock only). `codex-rtwk`:
`.claude/worktrees/screens-dialogs/codex-rs/tui-rtwk/target/debug/codex-rtwk`
(prebuilt, not rebuilt here; the first run, with the port-skeleton build, gave
the same results).

| binary | mode | result |
|---|---|---|
| installed codex 0.167.0 | `exec --json` | 11/11 pass |
| installed codex 0.167.0 | pty (`--pty`) | 11/11 pass (the approval scenarios got `y` at the prompt) |
| `codex-rtwk` | pty | 11/11 pass (the approval scenarios show the skeleton's "This build cannot answer it yet" dialog; Enter dismisses it; the turn then continues) |

What the runs found, fixed in the tools: the old `gpt-5.4` slug of the resize
test gives fallback metadata and refuses `apply_patch` (hence `gpt-6-sol`);
`codex exec` forces approval policy Never; the TUI's title request says
`thread_title`, not `system`; `-I` drops the script directory from `sys.path`.
`text-1000` takes about 9 s for 6000 events here (about 660 events/s).
`codex-rtwk` sends no title request (0), the oracle sends one.

## Notes for the others

- scenario-harness: `ScenarioServer` + `home.write_home` are importable
  (`sys.path.insert(0, "tools/rtwk-port/mock-model")`).
- Titles: only an interactive run sends them; a scenario needs no response for them.
- The scenarios are defined by Python files; a new one is a file in
  `scenarios/` whose `SCENARIO.name` equals the stem.
