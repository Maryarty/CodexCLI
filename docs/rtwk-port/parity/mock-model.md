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
tools/rtwk-port/mock-model/try.sh router rtwk            # one session: the word you type picks the response
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
| `configs.py` | config.toml pieces scenarios share (the MCP server table, the compaction limit and prompt) |
| `router.py`, `scenarios/router.py` | router mode (below) |
| `mcp_server.py` | a stdlib stdio MCP server, the twin of core's `test_stdio_server` (which needs cargo) |
| `coverage.py` | prints the coverage tables of this page from the scenarios and `check.py --json` results |
| `scenarios/*.py` | one file per scenario |
| `server.py` | `ScenarioServer`: the harness server plus a title-request hook, response status/headers, body-matched responses, the router hook (below) |
| `serve.py`, `home.py`, `bins.py`, `try.sh`, `check.py` | the tools |
| `selftest.py` | `python3 -I selftest.py`: queued() constant over reloads; `harness.verify()` fails naming each missing private name |

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
| `reasoning-raw` | 1 | 1 | summary deltas plus raw `reasoning_text` deltas | `show_raw_agent_reasoning` |
| `exec-command` | 1 | 3 | two `exec_command` calls with no approval (the second fails, exit 3) | never |
| `patch-heredoc` | 1 | 2 | `apply_patch` typed into `exec_command` with a heredoc | on-request, read-only; one approval prompt |
| `plan-tool` | 1 | 4 | `update_plan` three times, with a reasoning item | `[tools.update_plan] enabled` |
| `proposed-plan` | 1 | 1 | a `<proposed_plan>` block (a plan cell in Plan mode) | never |
| `view-image` | 1 | 2 | `view_image` of a seeded PNG | never |
| `image-generation` | 1 | 1 | `image_generation_call` with a 64x64 PNG | never |
| `mcp-tool` | 1 | 3 | MCP `echo` (structured result) and `image_scenario` (text, image) through `mcp_server.py` | `[mcp_servers.rmcp]` |
| `unified-exec` | 1 | 3 | `exec_command /bin/cat` on a tty, then `write_stdin` to the process | never |
| `background-terminal` | 1 | 2 | `exec_command sleep 3000`: a background terminal outlives the turn | never |
| `request-user-input` | 1 | 2 | `request_user_input` with a multiple-choice question (pty: a person answers) | `default_mode_request_user_input` |
| `multi-agent` | 1 | 3-4 | `spawn_agent` (+ `wait_agent` for v2); the child's request is matched by body | `MOCK_AGENT_API=v1` (tree) or `v2` (installed 0.167) |
| `code-mode` | 1 | 2 | the `exec` custom tool with a JS snippet | `code_mode` |
| `tool-search` | 1 | 2 | `tool_search_call` (client execution) | never |
| `custom-tool-unknown` | 1 | 2 | `custom_tool_call` in namespace `functions`, refused by the client | never |
| `model-verification` | 1 | 1 | `response.metadata` with `openai_verification_recommendation` | never |
| `token-usage` | 3 | 3 | `response.completed` usage of 12k, 48k, 96k tokens (cached and reasoning parts) | never |
| `rate-limits` | 2 | 2 | `x-codex-primary/secondary-*` response headers on an answer | never |
| `usage-limit` | 2 | 2 | HTTP 429 `usage_limit_reached` with rate-limit headers; the turn fails | never |
| `error-context-window` | 2 | 3 | `context_length_exceeded`: Codex compacts, the turn fails | never |
| `error-stream-closed` | 1 | 2 | the SSE stream ends without `response.completed`; the retry answers | `stream_max_retries = 2` |
| `interrupted` | 2 | 2 | a 30 s paced stream to cut with Esc, then a normal turn (pty) | never |
| `compact-manual` | 3 | 3 | a turn, `/compact` (reasoning item + summary), a turn after it (pty) | `compact_prompt` marker |
| `compact-auto` | 3 | 4 | usage 70k then 330k; the third prompt compacts first, then answers | limit 200k, `compact_prompt` marker |
| `router` | 29 | many | router mode, below | on-request, read-only |

"Many turns" is built as many rounds inside one turn (`long-transcript`), which
a single prompt reaches in both binaries; `multi-turn` is the several-prompt
conversation. Every scenario ends its last answer with a sentinel token
(`END-<name>`).

### Router mode

`try.sh router rtwk|oracle` serves no queue: `router.py` picks every response from the
request body. It reads the user's last real message (the last `user` message that does not
start with `<` (injected context) or `# AGENTS.md`) and the step (the number of tool outputs
after it: a tool call needs the next request to be answered, so step k is the k-th response
of that type's scenario). The first word of the message that is a keyword picks the type:

| keyword | response | keyword | response |
|---|---|---|---|
| `text` | 600 streamed words | `exec` | two commands, one fails, no approval |
| `markdown` | markdown answer | `approve` | a command that asks for approval |
| `reasoning` | summary, then answer | `patch` | `apply_patch` (update + add), asks |
| `heredoc` | patch typed into a shell heredoc | `plan` | `update_plan` three times |
| `proposal` | a `<proposed_plan>` block | `image` | `view_image` of a PNG |
| `imagegen` | `image_generation_call` | `web` | web search item |
| `mcp` | MCP echo + image tool | `unified` | a process kept alive, `write_stdin` |
| `background` | a command left running | `input` | `request_user_input` |
| `agent` | `spawn_agent` | `code` | code-mode `exec` |
| `toolsearch` | `tool_search_call` | `custom` | a custom tool call Codex refuses |
| `verify` | model verification metadata | `tokens` | 12k tokens used |
| `limits` | rate-limit headers | `quota` | HTTP 429 usage limit |
| `error` | `insufficient_quota` | `interrupt` | a 30 s stream to cut with Esc |
| `compact` | usage over the limit: the next message compacts first | `all` | the showcase |

`all` plays reasoning, web search, plan, exec, approval, patch, view_image, image generation,
MCP, unified exec, a question and an agent in one turn (each response ends in a tool call,
the last answers); the approvals and the question are answered by the person. Any other
text gets a short answer listing the keywords. Title requests keep their own handling
(`server.py`). The router also answers the compaction request (the config sets
`compact_prompt` to a marker) and a spawned agent's request (its message carries
`[mock-child]`). Not routable: `error-retry`, `error-stream-closed` (the response depends on
how often it was asked), `error-context-window` (whether Codex compacts afterwards depends on
the history), the multi-prompt scenarios.

The router's CODEX_HOME is the union of what the types need: on-request + read-only, the
`rmcp` MCP server, `default_mode_request_user_input`, `code_mode`, `update_plan`, a 200k
auto-compaction limit. `check.py` runs the `router` scenario like any other: one turn per
keyword in a pty (Esc for `interrupt`, `y` at approvals, Enter at the question, `2` at the
rate-limit nudge), then `compact`, a message with no keyword, and `all`.

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
| `ev_completed_with_tokens` (`input_tokens`, cached, `output_tokens`, reasoning, `total_tokens`) | `responses.rs` (`ev_completed_with_tokens`); used by `core/tests/suite/compact.rs`, `token_count`-style tests |
| `ev_model_verification_metadata` (`response.metadata`) | `responses.rs`, `core/tests/suite/model_verification.rs` |
| `ev_image_generation_call` | `responses.rs`; `core/tests/suite/image_generation.rs` |
| `ev_function_call_with_namespace`, `ev_tool_search_call`, `ev_custom_tool_call`, `ev_custom_tool_call_with_namespace` | `responses.rs`; `core/tests/suite/tool_search.rs`, `code_mode.rs`, `unified_exec.rs` |
| `ev_apply_patch_exec_command_call_via_heredoc` | `responses.rs`; `core/tests/suite/apply_patch_cli.rs` |
| `ev_rate_limits` (`codex.rate_limits`) | `responses.rs`; a WebSocket-only event (see the limits below). HTTP carries the same data in `x-codex-*` headers, which `events.rate_limit_headers` writes |
| the 429 usage-limit body | `core/tests/suite/usage_limit.rs`-style `usage_limit_reached` error JSON with `resets_at` |
| MCP tool result shapes | `rmcp-client/src/bin/test_stdio_server.rs` (`echo`, `image_scenario`), twinned by `mcp_server.py` |
| `request_user_input` arguments | `core/tests/suite/request_user_input.rs` |
| `spawn_agent` / `wait_agent` arguments | `core/tests/suite/subagent_notifications.rs` (v1 `multi_agent_v1`), and what installed 0.167 accepts (v2: namespace `collaboration`, `task_name` + `message`) |

Not used: `scripts/mock_responses_websocket_server.py` and `streaming_sse.rs`
(WebSocket and Rust-only servers); the scenarios serve HTTP SSE
(`supports_websockets = false`).

## Coverage

Regenerate with `python3 -I tools/rtwk-port/mock-model/coverage.py --results oracle.json --results rtwk.json`
(the JSON files are `check.py --json` output). "installed codex" is
`codex-cli 0.167.0` standing in for the oracle build; "codex-rtwk served it" means
the prebuilt skeleton drove the scenario to the end against the mock; "renders the
type" is n.a. until the slice that draws it (codex-rtwk is a skeleton).

### The 23 `ev_*` builders of `core/tests/common/responses.rs`

| Rust builder (responses.rs) | Python | scenarios |
|---|---|---|
| `ev_response_created` | `ev_response_created` | every scenario (through `text_response`) |
| `ev_completed` | `ev_completed` | every scenario (through `text_response`) |
| `ev_completed_with_tokens` | `ev_completed_with_tokens` | compact-auto, rate-limits, token-usage |
| `ev_model_verification_metadata` | `ev_model_verification_metadata` | model-verification |
| `ev_assistant_message` | `ev_assistant_message` | every scenario (through `text_response`) |
| `ev_message_item_added` | `ev_message_item_added` | every scenario (through `text_response`) |
| `ev_output_text_delta` | `ev_output_text_delta` | every scenario (through `text_response`) |
| `ev_reasoning_item` | `ev_reasoning_item` | compact-manual, plan-tool, reasoning, reasoning-raw |
| `ev_reasoning_item_added` | `ev_reasoning_item_added` | reasoning, reasoning-raw |
| `ev_reasoning_summary_text_delta` | `ev_reasoning_summary_text_delta` | reasoning, reasoning-raw |
| `ev_reasoning_text_delta` | `ev_reasoning_text_delta` | reasoning-raw |
| `ev_web_search_call_added_partial` | `ev_web_search_call_added_partial` | web-search |
| `ev_web_search_call_done` | `ev_web_search_call_done` | web-search |
| `ev_image_generation_call` | `ev_image_generation_call` | image-generation |
| `ev_function_call` | `ev_function_call` | plan-tool, request-user-input, unified-exec, view-image |
| `ev_function_call_with_namespace` | `ev_function_call_with_namespace` | mcp-tool, multi-agent |
| `ev_tool_search_call` | `ev_tool_search_call` | tool-search |
| `ev_custom_tool_call` | `ev_custom_tool_call` | code-mode |
| `ev_custom_tool_call_with_namespace` | `ev_custom_tool_call_with_namespace` | custom-tool-unknown |
| `ev_apply_patch_custom_tool_call` | `ev_apply_patch_custom_tool_call` | patch-approval |
| `ev_exec_command_call` | `exec_command_call` | background-terminal, exec-approval, exec-command, long-transcript, unified-exec |
| `ev_exec_command_call_with_args` | `yield_time_ms=` | background-terminal, exec-approval, long-transcript, unified-exec |
| `ev_apply_patch_exec_command_call_via_heredoc` | `ev_apply_patch_exec_command_call_via_heredoc` | patch-heredoc |
| `sse_failed` (not an `ev_*`) | `ev_failed_with_code` | error-context-window, error-fatal |

Every `pub fn ev_*` of responses.rs is mapped (`coverage.py` lists any that is not).
`ev_rate_limits` (`codex.rate_limits`) is the one builder with no scenario use on HTTP:
Codex reads that event on WebSocket only; the same data goes as `x-codex-*`
headers (`rate-limits`, `usage-limit`).

### Response types, scenarios, oracle tests

| scenario | response type shown | Rust helper | core suite test it mirrors | installed codex 0.167 (check.py) | codex-rtwk served it | codex-rtwk renders the type |
|---|---|---|---|---|---|---|
| `background-terminal` | exec_command `sleep 3000` yielding at 250 ms: the turn ends with a background terminal still running | ev_exec_command_call_with_args | unified_exec.rs:2910 unified_exec_keeps_long_running_session_after_turn_end | yes (exec) | yes | n.a. until its slice |
| `code-mode` | custom_tool_call `exec` with a JavaScript snippet (code mode), then an answer | ev_custom_tool_call (responses.rs:1058) | code_mode.rs:294 (custom_tool_call exec), app_tool_exposure.rs:155 | yes (exec) | yes | n.a. until its slice |
| `compact-auto` | usage 70k, then 330k (over the 200k limit), then the next prompt first compacts (summary request), then answers | ev_assistant_message, ev_completed_with_tokens (responses.rs:849) | compact.rs:2543 auto_compact_runs_after_token_limit_hit | yes (exec) | yes | n.a. until its slice |
| `compact-manual` | a normal turn, then /compact (the model answers the compaction request with a reasoning item and a summary), then a turn after it | ev_reasoning_item (responses.rs:911), ev_assistant_message | compact.rs:1894 manual_compact_emits_context_compaction_items | yes (pty) | no: turn 2: mock served 1 of 2 model request(s) in 61s; screen ends: t limit y▎/compact  summa | n.a. until its slice |
| `custom-tool-unknown` | custom_tool_call functions.custom_echo (unknown): the client returns an error output, the model answers | ev_custom_tool_call_with_namespace (responses.rs:1070) | search_tool.rs:1016 (custom_tool_call in namespace functions) | yes (exec) | yes | n.a. until its slice |
| `error-context-window` | response.failed context_length_exceeded: Codex compacts (the 2nd response is the summary), the turn ends with the error; the next prompt answers | sse_failed (responses.rs:1120) | codex-api/src/sse/responses.rs:1216 (context_length_exceeded as a fatal error), compact.rs:4782 manual_compact_retries_after_context_window_error | yes (exec) | no: turn 1: mock served 1 of 2 model request(s) in 61s; screen ends: _Gi=31,s=1,v=1,a=q,t=d,f | n.a. until its slice |
| `error-fatal` | insufficient_quota (not retried): the turn ends with an error; the second prompt answers | sse_failed with code insufficient_quota (responses.rs:1120) | quota_exceeded.rs | yes (exec) | yes | n.a. until its slice |
| `error-retry` | response.failed (server_error, retryable) once, the client reconnects (stream_max_retries=2), the retry answers | ev_failed / sse_failed (responses.rs:1120) | codex-api/src/sse/responses.rs:1151 (retryable server_error), client.rs retry tests | yes (exec) | yes | n.a. until its slice |
| `error-stream-closed` | SSE ends with no response.completed (stream closed before completion); stream_max_retries=2; the retry answers | sse (events without ev_completed) | stream_no_completed.rs:27 retries_on_early_close | yes (exec) | yes | n.a. until its slice |
| `exec-approval` | one exec_command with sandbox_permissions=require_escalated; approval policy on-request | ev_exec_command_call_with_args (responses.rs:1103) + approvals.rs:361 exec_command_event | approvals.rs (exec_command_event with require_escalated) | yes (exec) | yes | n.a. until its slice |
| `exec-command` | two exec_command calls without approval (an ls-like listing, a failing command), then an answer | ev_exec_command_call (responses.rs:1103) | unified_exec.rs:830 unified_exec_emits_exec_command_begin_event, :1014 ..._end_event, :1941 exec_command_reports_chunk_and_exit_metadata | yes (exec) | yes | n.a. until its slice |
| `image-generation` | image_generation_call completed (revised prompt + a 64x64 PNG), then a streamed answer | ev_image_generation_call (responses.rs:998) | model_switching.rs:1075 (the item is only replayed there; no suite test renders it) | yes (exec) | yes | n.a. until its slice |
| `interrupted` | a 30 s paced answer to interrupt with Esc, then a normal turn | ev_output_text_delta (stream cut by the client) | abort_tasks.rs (interrupt of a running turn), abort_lifecycle.rs | yes (pty) | yes | n.a. until its slice |
| `long-transcript` | one turn, 40 rounds of (message + exec_command echo), then a long list as the answer (MOCK_LONG_ROUNDS changes the count) | ev_assistant_message, ev_exec_command_call x40 | unified_exec.rs:1822 unified_exec_emits_one_begin_and_one_end_event (many commands); no suite twin for 40 rounds | yes (exec) | yes | n.a. until its slice |
| `markdown` | markdown answer (heading, lists, code block, table, quote) streamed 16 chars per delta at 200/s | ev_output_text_delta, ev_assistant_message | items.rs (agent message items); markdown content has no suite twin | yes (exec) | yes | n.a. until its slice |
| `mcp-tool` | MCP echo (structured result), MCP image_scenario text_then_image, then an answer; server = mcp_server.py | ev_function_call_with_namespace (responses.rs:1028) | rmcp_client.rs:711 text_only_mcp_content_uses_content_items (namespace mcp__rmcp, tool image_scenario); echo: rmcp_client.rs:1222 | yes (exec) | yes | n.a. until its slice |
| `model-verification` | response.metadata carrying openai_verification_recommendation = trusted_access_for_cyber, then a streamed answer | ev_model_verification_metadata (responses.rs:838) | safety_check_downgrade.rs:332 (model verification metadata on a normal turn), :405 (with a tool call) | yes (exec) | yes | n.a. until its slice |
| `multi-agent` | spawn_agent (namespace multi_agent_v1): the child request is matched by its prompt, the parent resumes after it | ev_function_call_with_namespace (responses.rs:1028), mount_sse_once_match (the child is matched by body) | subagent_notifications.rs:440-540 setup_turn_one_with_custom_spawned_child | yes (exec) | no: turn 1: mock served 2 of 3 model request(s) in 60s; screen ends: _Gi=31,s=1,v=1,a=q,t=d,f | n.a. until its slice |
| `multi-turn` | three turns, one response each: a greeting, a code answer, a closing summary | ev_assistant_message | no suite twin (several turns of one thread) | yes (exec) | yes | n.a. until its slice |
| `patch-approval` | one freeform apply_patch call (update notes.txt, add added-by-mock.txt); on-request, read-only | ev_apply_patch_custom_tool_call (responses.rs:1091) | approvals.rs:310 build_add_file_patch, apply_patch_cli.rs | yes (exec) | yes | n.a. until its slice |
| `patch-heredoc` | apply_patch via exec_command + heredoc (update todo.txt); on-request, read-only | ev_apply_patch_exec_command_call_via_heredoc (responses.rs:1113) | unified_exec.rs:630 unified_exec_intercepts_apply_patch_exec_command | yes (exec) | yes | n.a. until its slice |
| `plan-tool` | update_plan three times (steps move pending -> in progress -> completed), then an answer | ev_function_call (name update_plan), ev_reasoning_item | tool_lifecycle.rs:134 (update_plan x2 with a reasoning item), items.rs:570 | yes (exec) | yes | n.a. until its slice |
| `proposed-plan` | a streamed answer with a <proposed_plan> block (a plan cell in Plan mode, plain text in Default mode) | ev_message_item_added, ev_output_text_delta, ev_assistant_message | items.rs:557 (a <proposed_plan> streamed in Plan mode) | yes (exec) | yes | n.a. until its slice |
| `rate-limits` | two answers with x-codex-primary/secondary-used-percent headers (12.5/40.0, then 81.0/92.5) | ev_completed_with_tokens; headers as client.rs sets them (no ev_* builder: the codex.rate_limits event, request_user_input.rs:218, is read on WebSocket only) | client.rs:3750 token_count_includes_rate_limits_snapshot | yes (exec) | yes | n.a. until its slice |
| `reasoning` | reasoning summary deltas, the reasoning item, then a streamed answer | ev_reasoning_item_added, ev_reasoning_summary_text_delta, ev_reasoning_item (responses.rs:911,943,959) | compact.rs:1897 (a reasoning item), items.rs:1172 (summary deltas) | yes (exec) | yes | n.a. until its slice |
| `reasoning-raw` | reasoning summary deltas plus raw reasoning_text deltas and the item with raw content, then the answer | ev_reasoning_text_delta (responses.rs:967), ev_reasoning_item with raw_content (:911), ev_reasoning_item_added, ev_reasoning_summary_text_delta | items.rs:1172-1300 (summary and raw reasoning text deltas, show_raw_agent_reasoning = true) | yes (exec) | yes | n.a. until its slice |
| `request-user-input` | request_user_input with one multiple-choice question, then an answer that ends the turn | ev_function_call (name request_user_input) | request_user_input.rs:72 request_user_input_round_trip_resolves_pending (Plan mode), :428 ..._in_default_mode_with_feature | yes (pty) | yes | n.a. until its slice |
| `router` | one session: the keyword in the user's message picks the response type; `all` plays the showcase | every scenario; routing by the request body (resize_reflow.rs `mount_resize_response` matches bodies the same way) | n/a: Codex tests each type separately; the router is a convenience of this tool | yes (pty) | yes | n.a. until its slice |
| `text-100` | 600 one-word text deltas at 100 events/s (nominal, sleep-paced) | ev_response_created, ev_message_item_added, ev_output_text_delta, ev_assistant_message, ev_completed | client.rs (streamed message deltas); perf stream, no suite twin | yes (exec) | yes | n.a. until its slice |
| `text-1000` | 6000 one-word text deltas at 1000 events/s (nominal; sleep(0.001) gives about 600-900/s) | same as text-100, 6000 deltas | perf stream, no suite twin | yes (exec) | yes | n.a. until its slice |
| `token-usage` | three turns whose response.completed carries 12k, 48k and 96k tokens (cached and reasoning parts included) | ev_completed_with_tokens (responses.rs:849); the harness's ev_completed_with_usage for the cached/reasoning split | token_usage_rollout.rs, client.rs:3750 token_count_includes_rate_limits_snapshot | yes (exec) | yes | n.a. until its slice |
| `tool-search` | tool_search_call (execution client, query + limit), then an answer | ev_tool_search_call (responses.rs:1046) | search_tool.rs:256 (tool_search_call with query and limit 8) | yes (exec) | yes | n.a. until its slice |
| `unified-exec` | exec_command /bin/cat (tty, yields at 200 ms, session stays), write_stdin sends a line to it, then an answer | ev_exec_command_call_with_args, ev_function_call (name write_stdin) | unified_exec.rs:3100 unified_exec_reuses_session_via_stdin | yes (exec) | yes | n.a. until its slice |
| `usage-limit` | 429 usage_limit_reached (plan pro, primary 100%): the turn fails; the next prompt answers | no ev_* builder (a ResponseTemplate::new(429) with x-codex-* headers and a JSON error body) | client.rs:3887 usage_limit_error_emits_rate_limit_event, quota_exceeded.rs | yes (exec) | yes | n.a. until its slice |
| `view-image` | view_image of assets/example.png (a 320x120 red PNG, seeded), then an answer | ev_function_call (name view_image) | view_image.rs:420 view_image_tool_attaches_local_image | yes (exec) | yes | n.a. until its slice |
| `web-search` | web_search_call in progress, completed with a query, then a streamed answer | ev_web_search_call_added_partial, ev_web_search_call_done (responses.rs:975,986) | scenarios_web_search.rs:481 | yes (exec) | yes | n.a. until its slice |

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
- `--pty` drives the oracle in the same pty driver. Scenarios marked `pty_only`
  (interrupted, compact-manual, request-user-input, router) need a person's
  key (Esc, `/compact`, an answer) and run in the pty for the oracle too.
- The pty driver answers what a person would: `y` at an approval, Enter at a
  `request_user_input` question, `2` at the model-switch nudge a 429 raises,
  Enter at the skeleton's dismiss dialog, Esc `esc_after` seconds after a turn's
  Enter (the `interrupted` turn).
- `--json FILE` writes one record per run (scenario, binary, mode, status,
  detail, notes) that `coverage.py --results` reads; `--dump` (or
  `MOCK_DUMP_TAIL`, `MOCK_DUMP_META`) prints the tail items, tool outputs and
  metadata of each request, to see what Codex sent; `MOCK_TURNS=a:b` runs a
  slice of the turns of a scenario named by `--only`; `MOCK_DUMP_SCREEN=1`
  prints the end of the screen when the sentinel is missing.
- The tool-call check also covers `tool_search_call`; a scenario with
  `allow_unsupported` (custom-tool-unknown, router) expects the refusal.
- A binary that is not built is reported and skipped; nothing is built.
- The server subclasses private names of Codex's harness (`_ResponsesHandler`,
  `MockResponsesServer._record_request` / `_next_response` / `_server` /
  `_responses`, the `ev_*` helpers). `harness.verify()` runs when `harness.py` is
  imported (so before `check.py` or `serve.py` build a server): a missing name
  stops them with exit 2 and a message naming it, instead of an error inside a
  request thread. In `--loop` mode `load()` resets the matched responses on every
  reload, so `queued()` no longer grows by the unused matched ones (batch review 1, G11).

## Result of the full run (2026-11-18)

Binaries: the oracle build (`~/.cache/codexcli/oracle-bin/`) does not exist yet.
Stand-in, named by `ORACLE_BIN`: the installed `codex-cli 0.167.0`
(`~/.local/bin/codex`; not the pinned oracle, only a real Codex binary; it talks
to the local mock only). `codex-rtwk`: `~/.cache/codexcli/bin/codex-rtwk` (the
port skeleton, prebuilt, not rebuilt here). 36 scenarios (the 11 of the first
version, 24 new, and the router).

| binary | mode | result |
|---|---|---|
| installed codex 0.167.0 | `exec --json` (32) and pty for the `pty_only` ones (4: interrupted, compact-manual, request-user-input, router) | 36/36 pass (`MOCK_AGENT_API=v2` for multi-agent) |
| `codex-rtwk` skeleton | pty | 33/36: it fails `compact-manual` (it does not run `/compact`; the screen stays on the command popup), `error-context-window` and `multi-agent` (it served fewer requests than queued within the 60 s budget; cause not investigated: the skeleton is not a finished client, and the v1 agent namespace the pinned tree should accept was not verified on it) |

The skeleton's failures are the expected "no" of the coverage column, not mock
faults: the same requests pass on the installed codex.

What the installed codex accepted (all of them): every builder listed in the
first table that has a scenario; every scenario above runs to its outcome with
every queued response served and every tool call answered by a non-"unsupported"
output, except `custom-tool-unknown`, where the refusal is the scenario's point.

What the runs found, fixed in the tools:
- The first version: the old `gpt-5.4` slug of the resize test gives fallback
  metadata and refuses `apply_patch` (hence `gpt-6-sol`); `codex exec` forces
  approval policy Never; the TUI's title request says `thread_title`, not `system`;
  `-I` drops the script directory from `sys.path`.
- `update_plan` needs `[tools.update_plan] enabled = true`; `request_user_input`
  needs `default_mode_request_user_input`; the code-mode `exec` tool needs
  `code_mode`.
- Installed 0.167 requests carry no `tools` array in the body, so a tool's
  availability is only visible in the answer ("unsupported call").
- 0.167's multi-agent tools are in namespace `collaboration` (`spawn_agent` with
  `task_name` + `message`, plus `wait_agent`); the pinned tree's `multi_agent_v1`
  gives "unsupported call: multi_agent_v1spawn_agent" there. `MOCK_AGENT_API=v1|v2`
  picks; the default is v1 (the tree).
- Process ids of `exec_command` are random: `write_stdin` takes the id from
  the output (`@@SESSION_ID@@` is replaced in the next response from the
  previous `session ID N` text).
- The spawned agent's request is a subagent: its `thread_source` is `subagent`, not a
  title request, and is matched by body (`Resp.pick`), not by queue order.
- A 429 makes the TUI offer a model switch (nudge dialog): the driver answers `2`
  (keep the model); `try.sh` users press the same.
- `context_length_exceeded` makes Codex compact and then fail the turn, so the
  summary is a second response of turn 1; the router leaves it out (the number of
  requests depends on the history).
- `text-1000` takes about 9 s for 6000 events (about 660 events/s).
- The router run (65 requests, 144 s) ends with `sentinel on screen: no` on the full
  oracle run, `yes` when its last three turns run alone (`MOCK_TURNS=26:29`): the
  full transcript scrolls the final line out of the rendered text the driver
  keeps; every request was served.

## Limits: what the helpers cannot express

- Remote compaction (`/responses/compact`, an OpenAI-provider feature): the mock
  provider is not OpenAI's, so Codex compacts locally (the `compact_prompt` marker).
- `codex.rate_limits` as an event: WebSocket only; over HTTP the data travels in
  headers, and a limit that has no quota header cannot be drawn.
- The real model's tool list and wording: the mock never sees `tools` (0.167
  does not send it); a tool the config does not enable is answered "unsupported"
  by Codex, not by the model.
- Anything the client does with a real account (ChatGPT plan, credits, the
  `Jan 1st, 2024` reset time of the 429 body is a fixed value of ours).
- `web_search` result pages, `ev_image_generation_call` revised prompt, and any text
  that a real model would vary: fixed strings.

## Notes for the others

- scenario-harness: `ScenarioServer` + `home.write_home` are importable
  (`sys.path.insert(0, "tools/rtwk-port/mock-model")`).
- Titles: only an interactive run sends them; a scenario needs no response for them.
- The scenarios are defined by Python files; a new one is a file in
  `scenarios/` whose `SCENARIO.name` equals the stem.
