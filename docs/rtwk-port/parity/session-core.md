# session-core — parity

## Oracle tests

| Oracle reference | Port coverage | Result |
|---|---|---|
| `app/thread_routing.rs`, `app/thread_events.rs`, `app/thread_event_buffer.rs` | `core/tests/adr003.rs`: held writer round trip, switch during streaming, early notifications before resume, reconnect, ordered interleavings | The active page reads one thread model. Replay is limited to the attachment window. |
| `app/thread_routing.rs`, `app/tests.rs::active_turn_interrupt_race_extracts_actual_turn_id_from_mismatch` | `core/tests/model.rs`: both orders of start reply and notification, end before start reply, scoped retry on mismatch, late interrupt reply | The initial interrupt waits for an ID. The typed mismatch permits one retry for the server's current turn. |
| `chatwidget/tests/app_server.rs::live_app_server_turn_completion_repairs_dropped_message_deltas` | `core/tests/script.rs` and `tests/rules/fixtures.rs` | D1 fixed. Summary items correct the final answer and preserve the user message. Full history replaces a turn as a unit, including synthesized item IDs. |
| `streaming/controller.rs` | `core/tests/model.rs::streaming_commits_complete_utf8_lines_and_keeps_a_preview`, `tests/app/skeleton.rs` streaming and resize tests | Core finds line boundaries by scanning incoming bytes. The rtwk frame loop coalesces view updates. |
| `app/app_server_requests.rs` | `core/tests/approvals.rs`, `tests/sheets/presenter.rs`, `host/tests/session_sse.rs` | The retained request supplies the wire parameters. A real embedded server receives the confirmed exec decision and the next model request carries the command output. |
| `status/card.rs` and rate-limit updates | `core/tests/model.rs` status projection and stale-rate-reply tests | Tokens and context belong to the target thread. Account rate limits belong to the backend. A newer notification supersedes a pending read. |

## Snapshots

The streamed-answer debug entry now shows the repaired line from D1. The session row uses the pinned rtwk-rich `MarkdownView` for a retained Markdown document. Full oracle transcript snapshots belong to the chat cells and transcript slices.

## Visual deltas

### D2 — streaming and the transcript

The oracle animates committed screen lines and keeps mutable tables and fences at the tail. In the full-screen port, the raw tail is available to the retained Markdown widget while core tracks the last complete source line. rtwk-rich maintains the unsettled blocks and recomputes table layout at the widget's width. The inline terminal-history commit animation remains part of the inline-mode slice.

### D3 — requests and status

The existing sheets send command, file-change and permissions decisions, empty MCP form responses, and one-question option answers. Core also accepts explicit MCP content and responses keyed by question ID. Form editing and device verification have no corresponding widgets in this slice. Status retains rate-limit reset timestamps in core. The card does not yet format their local clock times.

## Scenarios (pty)

| Scenario | Result |
|---|---|
| `exec-approval` | Pass. The PTY pressed `y`, the command ran and the second request included its output. |
| `interrupted` | Pass on the pinned rtwk binary and the cached oracle with a lone `0x1b` byte. The PTY driver starts the Escape delay when it observes the model request, then requires the mock's SSE connection to disconnect before it sends the second prompt. The follow-up answer must appear on screen. `host/tests/session_sse.rs::standalone_escape_disconnects_the_live_stream` runs this check against the embedded server. |
| `error-context-window` | The cached oracle passed the original exec fixture. The embedded PTY reached the fatal error after one request and waited for the next submit. The original harness expects the compaction request before that submit. Moving the existing compaction response to the second turn's response sequence verified recovery on the same thread with three total requests and the final sentinel on screen. |
| `multi-agent` | The v1 fixture failed on both binaries. The embedded server answered `unsupported call: multi_agent_v1spawn_agent` and made two requests. `MOCK_AGENT_API=v2` passed on both binaries with four requests, including child and wait. |

## Rules

- ADR-002 and ADR-004. Existing routes, sheets, focus handling and the virtualized transcript use rtwk widgets. The session transcript delegates Markdown, table widths and code fences to the pinned framework.
- ADR-003 R2–R5. The app owns attachment and display. `ThreadModel` owns the transcript, running turn and thread tickets, and each screen reads an immutable projection. `ThreadFx` supplies the target and ticket for turn RPCs and per-thread screen commands. The rules test covers rename and compact wire construction.
- R1 and R7–R10. App and thread intents take the ordered ingress. Tests cover pending replies, switch and restore, snapshot replacement, interruption and late completions. Backend request keys include backend, epoch, thread and typed JSON-RPC ID. A reconnect expires the previous connection's answering keys.
- R12 and R14. Core depends on protocol and data libraries. Host pumps send and receive protocol messages. The UI uses rtwk provider projections and keeps its controllers under the UI model.
- `Rc` remains on the rtwk UI thread. `Ingress` and `Command` are `Send` and cross the Tokio boundary by value. Streaming deltas share the settled map and order index with the provider and mutate the small live set. An upstream consumer that moves an entire app model across threads would need a separate ownership decision.
- The ratatui oracle and its workspace manifests stay at the base revision.

## rtwk needs

The initial PTY report measured 34.9 s between requests with a lone Escape and 7.8 s with CSI-u. The old driver accepted either run once the expected request count arrived. The slow run did not recur on the committed binary. Three single-byte runs sent the follow-up request about 7.8 s after the first request. The HTTP handler observed a broken pipe during the first response. Suppressing the Escape write in the updated driver produces a failure when the complete response is sent, before the second prompt.
