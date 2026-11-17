# Does anything off the UI thread read app state in codex-tui? (2026-11-17)

Question for ADR-003: does code running off the foreground `App::run` task
(`codex-rs/tui/src/app/startup.rs:248`) read application/UI state through
shared memory, so the rtwk app would need a published snapshot?
Method: grep of `codex-rs/tui/src` (tests skipped) for `Arc<Mutex|RwLock>`,
atomics, `watch`, `OnceLock`/`LazyLock`/statics, `thread::spawn`, and a look
at what every `tokio::spawn` / `spawn_blocking` captures. Paths relative to
`codex-rs/tui/src/`.

**Verdict: no snapshot needed.** Channels suffice. Four places touch UI state
off-thread; each becomes a message handled on the UI thread.

## (a) UI state touched off-thread — and how the rtwk app serves it

| place | today | rtwk app |
|---|---|---|
| `onboarding/auth.rs:313-314` `sign_in_state: Arc<RwLock<..>>`, written by bg login tasks (`auth.rs:1049`, `onboarding/bedrock.rs:677`, `provider_setup.rs:139`, `gateway_login.rs:110`, `auth/headless_chatgpt_login.rs:179-215`) with a compare-and-set on `request_id` | bg reads + writes, fg renders | bg sends `AuthResult{request_id, ..}`; the UI thread drops stale ids and writes |
| `app_event_sender.rs:28,44` `voice_only: Arc<AtomicBool>`, read inside every `send()` (bg clones too) to drop events outside an allowlist | flag read on any thread | the filter moves to UI-thread dispatch (also filters events already queued — accepted) |
| `app/thread_routing.rs:806-855` interrupt retry task writes `pending_interrupt_turn_id`, pushes a notification and reads `store.active` of `Arc<Mutex<ThreadEventStore>>` | the only bg access to a thread store | bg sends `InterruptRace{thread, turn_id}` / `InterruptFailed{thread, error}`; the UI thread writes |
| `dynamic_tools_mcp.rs:100` `Arc<RwLock<Option<(AppServerRequestHandle, AppEventSender)>>>`, read per call by the in-process MCP server | connection handle, not UI state | a `watch` of the handle owned by the session layer |

Request/reply already over channels (port as is):
`chatwidget/statusline_reader.rs:252-300` (bg asks `ReadStep::Snapshot(.., oneshot)` every second, fg answers at `:195`);
`dynamic_tools.rs:1230` (`DynamicToolThreadStarted{registered: oneshot}`).

## (b) Channels or result caches in disguise (bg produces, fg consumes)

`app/thread_events.rs:655` + `app/thread_event_buffer.rs` (replay store paired
with an mpsc; fg-only apart from the interrupt task); `analytics/client.rs:18-45`
(bg-written caches read through `Load::start`, `analytics/data.rs:62` — return
them in the load result); `file_search.rs:17,103` (bg reads `latest_query` only
to drop stale results — a generation counter checked on receipt);
`managed_worktree_tools.rs:38` (tool-server registry, fg never reads);
`inline_visualization.rs:40` (dedup cache); `session_log.rs:16-19` (write-only
sink); clipboard workers (`clipboard_copy/worker.rs`, `clipboard_paste/text.rs:22`,
`clipboard_copy/wsl.rs`); `chatwidget/image_submission.rs:12` (serialization
lock); `ide_context/ipc.rs` (no shared state).

## (c) Immutable, cloned at spawn

`Arc<Config>`, `ModelCatalog` (`app/provider.rs:307`), the request handle;
init-once statics (`keymap.rs:674`, `system_motion.rs`, `screen_reader.rs`,
`clock_format.rs`, `shimmer.rs`, `tooltips.rs:177`, `highlight.rs` syntax set).
Runtime-written render globals: `markdown_render/preferences.rs:9,46`,
`render/highlight.rs:66,68` (theme) — written by fg, no bg reader found. In the
rtwk app the theme and rendering preferences are app state (Theme provider);
work spawned off-thread gets them cloned.

## (d) Cancellation, generations, one-shot guards

`task_mentions.rs:52`, `app_server_session.rs:413` (generation counters),
`chatwidget/usage.rs:167`, `worktree_startup.rs:59`, `model_catalog.rs:13`,
`app.rs:681`, `app.rs:566`, `statusline_reader.rs` (CancellationToken),
`provider.rs` dismiss watch, id counters (`realtime.rs:66`, `pets.rs:10`,
`connectors.rs:8`). Kept as they are: not app state.

## (e) Terminal plumbing rtwk replaces

`tui.rs:749,751` (`alt_screen_active`, `terminal_focused`), `tui/size_monitor.rs:30`,
`tui/tmux.rs`, `tui/keyboard_modes.rs`, `tui/job_control.rs`,
`terminal_program_status.rs`, `terminal_probe.rs`, `terminal_palette.rs`,
`frame_requester`.

## Specific answers

- Desktop notifications: decided on the UI thread (`chatwidget/notifications.rs`,
  `app.rs:1381` → `Tui::notify`); nothing off-thread reads UI state.
- Status line and title: UI thread; git branch/summary lookups
  (`status_surfaces.rs:627/643`) and the MCP status line provider reply by event.
- History lookup replies by event; otel and the daemon share no mutable state.
