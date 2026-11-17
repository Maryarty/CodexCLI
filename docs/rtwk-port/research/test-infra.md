# Test infrastructure for the Codex TUI port to rtwk: what exists, what we reuse, the parity harness

Research for tm `port-design` (2026-11-17). Oracle `codex-rs/tui` at ORACLE_REF `c6a41e32`, rtwk at `675a98d7`.
Both repos were only read. Codex was not built: `codex-rs/target` does not exist yet because a cold build is
running. rtwk-test and rtwk-dev were built and run through acargo. Counts come from scripts in
`$TMPDIR/codexcli-research/scripts/` (`classify.py`, `snaps.py`, `snapsrc.py`), which classify heuristically.
Out of scope here: perf metrics and how to show them (that is `perf-compare.md`). Section D4 lists only the hooks.

## Summary

**What we reuse as it is.**
- The oracle's 1,549 `.snap` files are the expected values, read where they are.
- Expected text for another 444 inline snapshots lives in Rust source and needs an extractor.
- `VT100Backend` (`tui/src/test_backend.rs:21`) is included by `#[path]`, as `tui/tests/test_backend.rs` already does.
- `core_test_support::responses` provides a mocked model: wiremock Responses-API SSE builders, plus gated streaming in `streaming_sse.rs`.
- `app_test_support` provides `MockResponsesConfig` and fake rollouts for resume.
- The in-process embedded app-server (`start_test_embedded_app_server`, `lib_tests.rs:408`).
- From rtwk:
  - `rtwk-test` (`Tester`, `FakeClock`, the strict emulator, `cell()`, `written()`, `scrollback()`, `styled_screen()`, `screen_frames`);
  - rtwk-term's `TerminalSession`, a pty plus an emulator that answers the startup probes and keeps scrollback;
  - `rtwk record/play/tape`;
  - rtwk's oracle method (a pinned ref, generators, JSONL goldens, `regen.sh`).
- The skeleton `tui-rtwk/tests/parity.rs` already does one snapshot replay and one cell diff by hand.

**The harness in 10 lines.**
1. One helper crate, `codex-tui-parity` (dev-only; rtwk-term only behind feature `pty`), plus rtwk adapters in `tui-rtwk/tests/support`.
2. Expected text is the oracle `.snap` (or the inline literal), read in place, normalized as insta does (`trim_end`, CRLF to LF). It is never written back.
3. rtwk renders headless in a `Tester` of the oracle's size. Its screen becomes a ratatui-shaped `Buffer`: tail cells become `" "`, links become OSC 8 inside the symbol.
4. That Buffer is flattened by the same one of 7 flatteners the oracle test used (TestBackend `Display`, Buffer `Debug`, first-char, popup-trim, rows-trim, vt100 contents, Lines). The registry records which flattener each snapshot uses.
5. Cell diff: the oracle Buffer vs rtwk's cells, field by field (symbol, link, fg, bg, underline colour, attrs), compared in rtwk's colour space. Tail cells are skipped.
6. Fixtures are **event scripts**: Seed, Notify(ServerNotification), Request, Key, Paste, Resize, Advance, Check, in JSONL.
7. The oracle's side of a script runs inside codex-tui:
   - live in tests, through a `frontend-testing` feature (ADR-001 shared logic);
   - or from a generator injected into a worktree at ORACLE_REF that writes goldens. This is mandatory when codex-tui stays untouched.
8. A generator check proves each script reproduces its `.snap` before the script becomes a parity test.
9. Scenarios run both binaries (`codex-tui`, `codex-rtwk`) with the same wiremock model, `CODEX_HOME` and key script. Both are read by one emulator (rtwk-term), comparing rows, scrollback and cursor at named checkpoints, with timers and paths normalized.
10. Registry status per snapshot (`flatten`, `script`, `pass/fail`) goes into `codex-registry.csv` (ADR-008).

**Biggest risks.**
- (1) **Fixtures reach into private state.**
  - chatwidget tests write private fields 864 times, app tests 858 times; there are 600 `chat.bottom_pane.` reach-ins.
  - Event scripts therefore need a Seed vocabulary, and turning about 1,450 assertions into scripts is the L-sized part of the work.
- (2) **Too many ways to flatten text, and cell semantics differ.**
  - The oracle uses at least 7 flatteners, and 444 inline snapshots keep their expected text in source.
  - Wide-char tails, OSC 8 inside ratatui symbols, and vt100 joining soft-wrapped rows differ from rtwk; so do `Red` and `Indexed(1)`, which rtwk maps to the same colour.
- (3) **Time and emulator determinism.**
  - The oracle uses paused tokio time. rtwk-test uses `FakeClock`, with no bridge between the two (needed if ADR-003 keeps an async controller).
  - pty runs follow the wall clock.
  - vt100 does not rewrap on resize; rtwk-term rewraps and is strict.
- (4) **Build and CI.**
  - `tui-rtwk` is already a workspace member (`codex-rs/Cargo.toml:121`) with path dependencies on `../../../rtwk`.
  - Bazel (the PR gate) and `rust-ci-full` will break unless `tui-rtwk` is excluded or rtwk is checked out at RTWK_REF.

## A. Codex side (`codex-rs`)

### A.1 The 6,329 tests, classified (heuristic, per test fn body)
- **Attributes:** 3,738 `#[test]` and 2,591 `#[tokio::test]`. Of the tokio tests, 43 are multi-thread and 60 use `start_paused`. There are 51 `tokio::time::advance` calls.
- **Files:** 634 files hold tests.
- **Serialization and ignores:** 26 tests are `#[serial]` (pets image protocol, env, terminal stderr, external editor); 7 tests are `#[ignore]`.

| kind | tests | how recognised |
|---|---|---|
| pure logic | 2,085 | none of the below |
| async controller (`#[tokio::test]`, no render) | 1,499 | ChatWidget/App driven through handlers |
| render/snapshot with a Buffer | 1,048 | `Buffer::empty`, `TestBackend`, `.render(`, `render_bottom_popup`, … |
| snapshot of text, no Buffer in the body | 721 | `assert_snapshot!` on strings from helpers |
| Lines / content model (no Buffer) | 522 | `display_lines`, `render_lines`, `lines_to_single_string`, markdown |
| sync controller (ChatWidget/App, no render) | 330 | `ChatWidget`/`AppEvent` without tokio |
| vt100 scenario | 124 | `VT100Backend`, `custom_terminal::Terminal`, `insert_history_lines` |

Per module (total | logic | lines | render | vt100 | sync ctl | async ctl):

| module | total | logic | lines | render | vt100 | sync ctl | async ctl |
|---|---|---|---|---|---|---|---|
| chatwidget | 1,322 | 53 | 201 | 427 | 21 | 3 | 617 |
| bottom_pane | 1,170 | 485 | 18 | 384 | 0 | 279 | 4 |
| app | 1,050 | 145 | 50 | 336 | 4 | 32 | 483 |
| transcript_view | 291 | 131 | 15 | 143 | 0 | 2 | 0 |
| markdown_render | 223 | 45 | 122 | 56 | 0 | 0 | 0 |
| history_cell | 192 | 24 | 46 | 118 | 1 | 0 | 3 |
| analytics | 158 | 69 | 0 | 40 | 0 | 0 | 49 |
| keymap | 133 | 132 | 0 | 0 | 0 | 0 | 1 |
| resume_picker | 115 | 45 | 2 | 11 | 10 | 0 | 47 |
| tui | 108 | 45 | 0 | 6 | 22 | 0 | 35 |
| streaming | 108 | 82 | 15 | 11 | 0 | 0 | 0 |
| app_server_session | 76 | 22 | 0 | 2 | 0 | 1 | 51 |
| tests/ (integration) | 32 | 8 | 0 | 2 | 10 | 0 | 12 |
| other 125 modules | 1,451 | — | — | — | — | — | — |

**Snapshots.** 1,450 assertions in 340 files: 1,413 `assert_snapshot!` and 37 `assert_debug_snapshot!`. 444 of them are inline (`@r#"…"#`, the expected text in source). There are 1,549 `.snap` files in 36 `snapshots/` dirs: chatwidget 441, bottom_pane 363, app 242, history_cell 94, transcript_view 57, markdown_render 50, onboarding 35, analytics 33, status 24, diff_render 23.
- **By origin** (from the `expression:` header and the `let` it names): about 908 Buffer or vt100 render; **65 styled `Buffer {…}` Debug** (text and styles, ready-made style goldens: 26 bottom_pane, 16 transcript_view); about 282 Lines→text without a Buffer; about 13 state Debug; about 280 unresolved (variables such as `rendered`).
- **Content:** 267 TestBackend-quoted bodies (`"…"` rows at full width, trailing spaces kept), 33 vt100; 54 contain wide glyphs, 124 have trailing spaces, 4 have raw ESC (OSC 8).
- **Platform:** 3 snapshots are `@windows` (`snapshot_suffix`: `chatwidget/tests/status_and_layout.rs:2592`, `permissions.rs:450,1505`).

### A.2 Snapshot helpers: each test flattens text in its own way

| flattener | where | semantics |
|---|---|---|
| TestBackend `Display` | `assert_snapshot!(terminal.backend())`, 193 snaps (e.g. `diff_render.rs:1477` `snapshot_lines`) | quoted rows, full width; ratatui-core 0.1.2 skips cells hidden by a wide glyph |
| Buffer `Debug` | `format!("{buf:?}")`, e.g. `history_cell/messages_tests.rs:241` | rows plus `styles: [x,y,fg,bg,underline,modifier]` runs |
| first char per cell | `snapshot_buffer` `bottom_pane/mod.rs:2771`, `request_user_input/mod.rs:1747`, `mcp_server_elicitation.rs:1822`; `render_snapshot` `:2783`, `:1759`, `:1839`, `app_link_view.rs:1064` | full grid; a wide glyph's tail shows as `' '` |
| popup trim | `render_bottom_popup` `chatwidget/tests/helpers.rs:1372` (133 snaps) | `desired_height`, strips OSC 8, trims rows' ends, drops blank first and last rows |
| rows to text | `buffer_to_text` `status/tests.rs:203`, `pager_overlay/transcript_tests.rs:401`, `inline_visualization_tests.rs:72`; `buffer_text` ×5 (`app/owned_transcript_tests.rs:65`, `app/tests/transcript_composer.rs:14`, …); `side_frame` `app/test_support.rs:29` (whole App frame via `make_test_tui`) | variants: OSC 8 strip, trim |
| vt100 contents | `terminal.backend().vt100().screen().contents()`, e.g. `chatwidget/tests/exec_flow.rs:1438` | vt100 0.16.2: soft-wrapped rows **joined**, trailing newlines dropped |
| Lines→text | `lines_to_single_string` `helpers.rs:479`, `render_lines` (13 local copies), `snapshot_lines_text` `diff_render.rs:1511` | no Buffer: shared content code (ADR-004) |

Normalizers live in `chatwidget/tests/helpers.rs`: `normalize_working_snapshot` (`:51`, "Working (Ns)" becomes 0s), `normalize_snapshot_paths` (`:61`), `normalized_backend_snapshot` (`:129`).

**insta.**
- Version 1.46.3; there is no `insta.yaml`.
- `assert_chatwidget_snapshot!` (`chatwidget/tests.rs:190`) uses `set_prepend_module_to_snapshot(false)` plus `set_snapshot_path(chatwidget_snapshot_dir())` (`:179`, found through `codex_utils_cargo_bin::find_resource!`, which also works in Bazel runfiles).
- `app/*` uses `with_settings!({snapshot_path => "../snapshots"})`.
- A ctor sets `INSTA_WORKSPACE_ROOT` to `codex-rs` (`core/tests/common/lib.rs:71`).
- insta compares `normalize()`, which is `trim_end()` plus CRLF→LF, and dedent for inline snapshots (`insta-1.46.3/src/snapshot.rs:753`). Header metadata is ignored.
- Outside CI, a failing assert writes `.snap.new` **next to the expected file**. So insta must never point at the oracle's directories.

### A.3 Fixtures and how tests feed events

**ChatWidget.**
- `make_chatwidget_manual(model)` (`chatwidget/tests/helpers.rs:213`, used 1,338 times in 95 files) and `…_with_auth` (`:229`) return the widget plus two receivers, `UnboundedReceiver<AppEvent>` and `<Op>`.
- They build a real `ChatWidget` from `test_config()` (a tempdir `CODEX_HOME`) and `FrameRequester::test_dummy()`, then set private fields directly (`clock_format`, `transcript.active_cell`, …).
- Notification feeders `handle_*` (`:524-1338`) call `chat.handle_server_notification(ServerNotification::…, replay_kind)`: 302 calls in 60 files.
- `drain_insert_history` (`:407`, 474 uses) renders the cells taken from `AppEvent::InsertHistoryCell` at width 80.
- Keys: `handle_key_event`, 2,315 calls; `handle_paste`, 289.

**App.** `make_test_app()` (`app/test_support.rs:36`, 782 uses), `make_test_app_with_channels()` (`app/tests.rs:9310`). `make_test_tui()` (`tui/test_support.rs:20`, 490 uses) creates an 80×24 `custom_terminal::Terminal` (`custom_terminal.rs:250`). App tests feed `handle_event` (486 calls, `app/event_dispatch.rs:35`) and `handle_tui_event` (343, `app.rs:886`).

**Event source and frames.** `FakeEventSource` (`tui/event_stream.rs:369`) is local to that file's tests; the trait is `EventSource` (`:45`). `FrameRequester::test_dummy` (`frame_requester.rs:62`, 198 uses); `test_channel` (`:138`) records frame requests (`record_frame_requests`, `tui/test_support.rs:12`). BottomPane: `test_pane` (31 uses).

**Private state.** Tests write `chat.<field> = …` 864 times and `app.<field> = …` 858 times, and reach `chat.bottom_pane.` 600 times. All helpers are `pub(super)` under `cfg(test)`, so no other crate can call them.

### A.4 vt100 and pty

**`VT100Backend`** (`src/test_backend.rs:21-135`) wraps `CrosstermBackend<vt100::Parser>` with scrollback and `force_color_output(true)`, and never queries the real terminal; `Display` gives `screen().contents()`. It is used through ratatui `Terminal` or `codex_tui::Terminal` (re-exported for integration tests: `lib.rs:149` `Terminal`, `:179` `insert_history_lines`, `:188` `RowBuilder`): 105 uses in 35 files, most in `insert_history.rs` (19), `resume_picker.rs` (12), `onboarding/trust_directory.rs` (7).

**`tests/all.rs`** builds one binary from `tests/suite/mod.rs`: 14 modules, 10 unix-only (`resize_reflow`, `status_indicator`, `vt100_history`, `vt100_live_commit` run everywhere); `focus_palette.rs:25-29` pulls in 3 more files through `#[path]`.

| file | tests | what |
|---|---|---|
| vt100_history | 7 | `insert_history_lines` into 20×6 with a 1-row viewport: wrapping, emoji, colours in scrollback |
| vt100_live_commit | 1 | `RowBuilder::drain_commit_ready` plus insert |
| focus_palette | 6 (+3 via `#[path]`) | **`PtyCodex`** (`:296`): `libc::openpty` 120×32, spawns `codex-tui`/`codex`, `vt100::Parser`; `answer_startup_queries` (`:466`) answers CPR `ESC[6n`, DA1 `ESC[c`, kitty `ESC[?u`, OSC 10/11; `wait_for_screen`, `write_input` |
| resize_reflow | 4 (`#[ignore]`) | tmux plus wiremock plus a built `codex`, `--no-alt-screen`, `capture-pane`; `write_config` (`:590`) points a `model_provider` at the mock |
| reconnect, directory_trust, worktree_stack, thread_title, provider_defaults, statusline_provider, daemon_*, screen_reader | 1–2 each | the binary in a pty or through `cargo_bin`, with `core_test_support` mocks |
| status_indicator | 1 | pure ANSI strip |

### A.5 Backend fakes for scenarios

| level | asset | gives |
|---|---|---|
| L1 notification script (in-process) | `chat.handle_server_notification` / `app.handle_event` | deterministic, no I/O; only works inside codex-tui. `ServerNotification` is serde `{"method","params"}` (`app-server-protocol/src/protocol/common.rs:1833`), so a script can be JSONL |
| L2 mocked model | `core_test_support::responses` (`core/tests/common/responses.rs`): `start_mock_server` `:1264`, `sse` `:798`, `ev_*` `:818-1120` (text deltas, reasoning, exec/apply_patch calls, web search, image), `mount_sse_once` `:1205`, `mount_sse_sequence` `:1556`, websocket server `:1281`. `streaming_sse.rs:21,60`: per-chunk gates (timing under control). `app_test_support::MockResponsesConfig` (`app-server/tests/common/config.rs:7`) writes `config.toml` for the mock | a real app-server and real notifications. In process: `start_embedded_app_server_for_picker` + `make_test_tui` (e.g. `app/tests/session_lifecycle_requests.rs:1971`); or a binary in a pty |
| L3 rollout replay | `app_test_support::create_fake_rollout*` (`rollout.rs:42-331`, 7 variants), tui's `create_history_rollout`/`write_session_rollout` | resume renders history with no model |

**Not TUI fakes.** `app-server-test-client` drives `codex app-server` over websocket (manual probing, `just app-server-test-client`). `session_log.rs` (`CODEX_TUI_RECORD_SESSION=1`, `:85`) logs AppEvents in and ops out, not server notifications. `tests/fixtures/oss-story.jsonl` (8,041 lines, old event shape) is referenced by nothing. No recorder of app-server notifications exists.

### A.6 Tooling

- **nextest** (`codex-rs/.config/nextest.toml`): `retries = 1`, slow-timeout 30 s × 2, test-groups for app-server and sandbox suites, JUnit; no TUI overrides.
- **justfile:** `just test` runs `RUST_MIN_STACK=8 MiB NEXTEST_PROFILE=local cargo nextest run --no-fail-fast` (`justfile:87`); also `bench`, `bench-e2e` (Bazel), `app-server-test-client` (`:45`).
- **CI on PRs:** `blocking-ci.yml` runs **Bazel** (`bazel test //…`) plus `rust-ci` (lint, shear). TUI tests are gated by Bazel's `tui-unit-tests`: 8 shards, `RUST_MIN_STACK` 24 MiB, snapshots as `test_data` (`tui/BUILD.bazel`).
- **CI after merge** (or a `**full-ci**` branch): `rust-ci-full.yml:495-554` runs nextest on 5 platforms (macOS aarch64, Linux x64/arm64, Windows x64/arm64) from an archive, `--partition hash:N/4` (`rust-ci-full-nextest-platform.yml:399`).
- **cargo-insta is not installed** (`which cargo-insta` finds nothing); neither repo documents a review workflow.

## B. rtwk side

### B.1 rtwk-test (202 tests in 3 binaries; one was run)

**`Tester`** (`crates/rtwk-test/src/tester.rs`).
- Options `Options::{size, mode(ScreenMode), history, viewport, kitty_keyboard, max_fps, input_at_once, start_time, env}` (`:98-246`). Running: `pump_widget`, `pump_app(AppView)` (`:498`, `:525`); `pump`/`pump_for`/`pump_until_idle` move `FakeClock` (`rtwk-core/src/headless.rs:77`) on a `HeadlessTerminal` (`:261`). App access: `act`, `read_app`, `update_app`, `state`.
- Input `input`, `type_text`, `press("ctrl+s")`, `paste`, `click`, `mouse`, `resize` (`:761-845`), decoded by rtwk-input.
- Screen: `lines` (wide glyph once, trailing blanks trimmed), `screen` (empty end rows dropped), `marked_screen`, `styled_screen` (twk screen-file format: size, cursor, style layer, legend), `lines_in`, `cell(x,y)` (`Cell{text, fg, bg, ul, attrs, link, tail}`, `screen.rs:27`), `cursor`, `scrollback`, `viewport_lines`, `written()` (every byte the app wrote), `frames()`, `write_trace`.
- Finders and asserts: `find::{by_type, by_key, text}`, `assert_line`, `assert_style_at`, `assert_screen`; `assert_screen_snapshot!` wraps insta.

**Also in rtwk-test.**
- A strict emulator: `Screen` wraps rtwk-term's `TerminalEmulator`, keeps unknown sequences in `unknown()`, keeps scrollback, rewraps on resize.
- `screen_frames(w, h, [TerminalWrite])` turns a recording into a screen per write at 2026 synchronized-update boundaries.
- `CliScreen`/`VtTerminal`, `start_process`/`run_process` (killed on drop, bounded).
- **No tokio-time bridge:** rtwk-tokio's tests use a real runtime and wall-clock `wait_until` (`rtwk-tokio/tests/common/mod.rs`).

### B.2 rtwk-e2e and rtwk-term pty

- **`rtwk-e2e`** holds headless and pty scenarios over the public APIs, with rtwk-term's emulator as the strict oracle (`docs/design/e2e.md`).
- **`pty::ChildApp`** (`src/pty.rs:29-190`) re-runs its own test binary as the child app in a scenario. Its methods are `spawn_with`, `pump`, `rows`, `wait_for`, `send`, `press`, `type_text`, `resize`, `wait_exit` and `report`.
- **The child's terminal** is `rtwk_term::TerminalSession::start_with(PtyCommand, Some((w,h)), 1000, TextMode)`. It runs **any program**.
- **The emulator answers queries:** DSR/CPR, DA, DECRQM, DECRQSS, XTVERSION, OSC 10/11 and kitty `CSI ? u` (`emulator/mod.rs:201`, `dispatch.rs:575`). This covers both Codex's probes and rtwk's.
- **Other pieces:**
  - `oracle::ScreenOracle` (`:11`) provides `feed`, `rows`, `look(x,y)` and `restored()`;
  - `driver::Harness` (`:135`) steps a headless frame loop on a fake clock;
  - `feed::TokenFeed` with `agent.rs` (648 lines) form a Codex-like agent app driven by a token script;
  - `measure.rs` provides a median, a bootstrap CI, p90 and machine load.

### B.3 rtwk-dev `rtwk record | play | tape` (ran)

- **`record`.** `rtwk record [-o file] [--idle-limit s] [--keys] -- cmd…` writes asciicast v2. `--keys` also records input as `"i"` events.
- **`play`.** `rtwk play [-s speed] file` replays a recording.
- **`tape`.** `rtwk tape file.tape` runs a VHS-like script (`Type`, `Enter`, `Ctrl+…`, `Sleep`, `Wait+Screen`, `Wait+Line`, `Resize` via `Set`, `Screenshot`, `Output`) and writes `.cast` or `.svg`. The repo has no `.tape` samples.
- **Use for parity.** A cast's `"o"` events are a `TerminalWrite` list, so `screen_frames` turns casts of both binaries into comparable screens.

### B.4 The oracle and golden method, and the registry

- **Goldens.** `oracle/TWK_REF` pins twk. 71 generators `crates/*/tests/oracle/gen_*.dart` write 120 JSONL goldens in `crates/*/tests/data/` (up to 18 MB per crate). Rust tests replay them, e.g. `rtwk-test/tests/twk_test/oracle.rs`: 836 key specs, 250 byte streams to rows, styled rows and cells, 160 tester scripts.
- **`oracle/regen.sh`** refuses a checkout at another commit or with local changes, reruns every generator, fails when a golden changed or appeared; goldens are never edited by hand (`docs/TESTING.md:112`).
- **Registry.** `docs/parity/registry.csv`, 6,469 rows (3,294 verified-on-oracle, 1,843 tested, 748 present-untested, 509 replaced, 66 excluded, 7 missing), with `api_map.csv`, `tests.csv`, `evidence.csv`, `registry.md`; generated by `tools/parity/{rust_api,rust_tests,registry,dart_api}.py`.
- **Precedent for comparing renderers.** `bench/` (its own workspace with ratatui): `rtwk-bench --verify` checks rtwk and ratatui hash the same screens after 300 frames.

### B.5 Other rtwk test facilities

- **Budgets:** counting allocators give exact budgets (e.g. `rtwk-test/tests/twk_test/budgets.rs`). `assert_perf_baseline_eq!` asserts only under `RTWK_PERF_REPORT` (`xtask/test_support/perf_baseline.rs`); `cargo xtask perf-report <filter>` runs them.
- **xtask:** `check` (fmt, clippy debug and release, tests, doctests), `gate` (responsiveness through PTYs, `dist` profile, exits 75 under load), `bench-check`, `layers`, `sweep`.
- **nextest** (`.config/nextest.toml`): `retries = 1`, `flaky-result = "pass"` (shown as FLAKY), slow 60 s × 5.
- **Mutants and fuzzing:** cargo-mutants 27.x is installed and used per research task (e.g. `docs/research/style-table.md:235`); `crates/rtwk-input/fuzz` target `feed.rs` (`cargo-fuzz` not installed).
- **CI:** `ci.yml` runs only when started by hand (Linux, macOS, Windows).

## C. Mapping

Verdicts: reuse = as is; adapt = changed for the port; replace = an rtwk equivalent; new = built for the port.

| asset | side | verdict | how | effort |
|---|---|---|---|---|
| 1,549 `.snap` (≈973 render incl. 65 styled Debug; ≈282 Lines; ≈13 state) | Codex | reuse | expected values read in place at ORACLE_REF (`oracle_snap`), insta-normalized | S reader + L twins |
| 444 inline snapshots | Codex | adapt | extract the `@r#"…"#` literal at file:line, insta-dedent | S |
| 65 styled `Buffer{…}` Debug snaps | Codex | reuse | parse into styled goldens: cell diff with no generator | S |
| flatteners (A.2 table) | Codex | adapt | one `Flatten` enum; TestBackend `Display`, Buffer `Debug` and vt100 used through the real crates; per-snapshot choice in the registry | M |
| normalizers (`helpers.rs:51-129`) | Codex | adapt | copy into `Normalize` (timers, paths, spinner glyphs) | S |
| `VT100Backend` | Codex | reuse | `#[path = "../../tui/src/test_backend.rs"]` | S |
| vt100_history / live_commit (8 tests; 105 VT100 uses in src) | Codex | replace | these test the ratatui inline mechanism. rtwk's `Static`/`Screen::commit` tests cover the mechanism; port the invariants (wrap, colours in scrollback, no lost rows) as scenarios | M |
| `PtyCodex` (`focus_palette.rs:296-560`) | Codex | replace | rtwk-term `TerminalSession` answers both binaries' probes and keeps scrollback; keep PtyCodex's env hygiene (`TERM`, `TMUX`, `OPENAI_*`) | S |
| tmux resize smoke (4 ignored) | Codex | adapt | parameterise by binary; or `TerminalSession::resize` | S |
| `make_chatwidget_manual`, `handle_*`, `drain_insert_history` | Codex | adapt | the oracle side of event scripts (in-crate generator or `frontend-testing` child-module fixtures); `handle_*` becomes `Step::Notify` | M/L |
| `make_test_app*`, `make_test_tui`, `side_frame` | Codex | adapt | same; `side_frame` gives whole-frame oracle Buffers | M |
| `FrameRequester::test_dummy`/`test_channel` | Codex | reuse (A) / n/a (B) | publish counting in the A-host | S |
| `FakeEventSource` | Codex | replace | rtwk-test `press`/`type_text`/`paste`/`resize` | S |
| paused tokio time | Codex | new bridge (A) / replace (B) | A: `pump_for(d)` also `tokio::time::advance(d)` on a current-thread runtime; B: FakeClock only | M |
| `core_test_support::responses` + `streaming_sse` | Codex | reuse | scripted model for pty and in-process scenarios, and rate control for perf | S |
| `app_test_support` (`MockResponsesConfig`, rollouts, `TestAppServer`) | Codex | reuse | `CODEX_HOME` setup, resume scenarios | S |
| embedded app-server (`lib_tests.rs:408`) | Codex | reuse (A) | in-process scenario backend; B: the shell's effect runner, or scripted effects | S/M |
| app-server-test-client | Codex | — | manual only | — |
| session_log, `oss-story.jsonl` | Codex | — / new | not a replay source; a notification recorder would be new (optional) | M |
| controller and logic tests (2,085 + 1,829) | Codex | reuse (A) / adapt (B) | A: stay in codex-tui and cover shared logic; B: ported to the core; vendored leaves keep their oracle tests | A S / B L |
| insta settings (`chatwidget/tests.rs:190`, `INSTA_WORKSPACE_ROOT`) | Codex | adapt | naming knowledge only; never `set_snapshot_path` into oracle dirs | S |
| nextest config, `just test`, `RUST_MIN_STACK` | Codex | adapt | test-group `parity_pty` max-threads 4; `RUST_MIN_STACK` ≥ 8 MiB for ChatWidget/App fixtures; `just rtwk-parity` | S |
| CI (Bazel PR gate, rust-ci-full 5 OS) and Bazel | Codex | adapt | exclude `tui-rtwk` from upstream jobs or check out rtwk at RTWK_REF; a separate parity job (unix) | M |
| `Tester`, `FakeClock`, `HeadlessTerminal`, Options (Inline, history, viewport) | rtwk | reuse | renders every native region; `written()` feeds vt100 for vt100-made snaps | S |
| `cell()`, `styled_screen`, `ScreenFile` | rtwk | reuse | cell diff and its report format | S |
| `screen_frames`, `TerminalWrite` | rtwk | reuse | casts and pty bytes to frames | S |
| rtwk-term `TerminalSession` + emulator | rtwk | reuse | the pty driver and the one emulator for both binaries | S |
| rtwk-e2e (`ChildApp`, `ScreenOracle`, `Harness`, `TokenFeed`, `measure`) | rtwk | adapt | patterns; ChildApp spawns itself, we spawn `codex-tui`/`codex-rtwk` | M |
| `rtwk record/play/tape` | rtwk | reuse | manual review, side-by-side casts, scripted visual checks | S |
| oracle method (`TWK_REF`, `regen.sh`, `gen_*.dart`) | rtwk | adapt | `ORACLE_REF` + `gen_*.rs` injected into a worktree of codex-tui + `oracle-regen.sh` | M |
| parity registry + `tools/parity/*.py` | rtwk | adapt | `codex-registry.csv` columns `snap, kind, flatten, script, status` (seeded by `snapsrc.py`) | M |
| budgets, `RTWK_PERF_REPORT`, gate, bench | rtwk | reuse | owned by perf-compare | — |
| cargo-mutants | rtwk | reuse | prove parity tests fail (mutate `convert.rs`, `LinesView`) | S |
| fuzz | rtwk | — | n/a | — |

## D. The parity harness

**Layout.** Works under both outcomes of ADR-001 and ADR-003.
- `codex-rs/tui-rtwk/parity/` is the crate `codex-tui-parity`: dev-only, `publish = false`. Its default build depends on ratatui, vt100 and serde only, so the injected generator can use it. It holds:
  - the snapshot reader, the flatteners, the script types, Buffer-vs-Buffer diff;
  - the pty runner, behind a feature `pty` (the only part that uses rtwk-term).
- rtwk adapters (`to_buffer`, `cell_diff` against a `Tester`) live in `tui-rtwk/tests/support/`.
- Tests:
  - `tui-rtwk/tests/parity/main.rs`, one module per area (history_cell, list_selection, footer, chatwidget, …), in one binary, as rtwk's `all.rs` does;
  - `tests/scenarios/*.rs`.
- Data:
  - `tests/scripts/*.jsonl`, the event scripts;
  - `tests/data/*.jsonl`, the oracle goldens.
- Generators `tui-rtwk/oracle/gen_*.rs` plus `tools/rtwk-port/oracle-regen.sh`.

**Two cases.**
- **Case A, ADR-001 as written** (logic shared in codex-tui behind `frontend-api`):
  - The oracle side can run **live** in the same test.
  - A feature `frontend-testing` adds child-module `fixtures.rs` files (one `mod` line per component). Like `view_model.rs`, they see private fields. Their constructors copy `make_chatwidget_manual`, `test_pane` and `make_test_app`.
  - Test-only dependencies become optional normal dependencies under the feature.
- **Case B, state-v2's recommendation** (`codex-tui-core` with behaviour ported; codex-tui untouched):
  - The oracle side runs **only** in the generator. `tui-rtwk` does not link codex-tui.
  - It includes only `test_backend.rs` by path and depends on `core_test_support`/`app_test_support` (dev).

### D1. Snapshot replay
- **Size.** The `Tester` gets the oracle's area, `(width, desired_height(width))` or the test's `Rect`. The script's `Check` records it; `render_bottom_popup` uses `chat.desired_height(w)`.
- **Text.** `to_buffer(&tester, area)` builds a `ratatui::buffer::Buffer`:
  - a lead cell gets the grapheme; a `tail` cell gets reset `" "` (ratatui-core 0.1.2 `set_stringn` resets hidden cells);
  - `link = Some(u)` becomes the symbol `\x1b]8;;u\x07X\x1b]8;;\x07` (as `terminal_hyperlinks.rs:1010` writes it);
  - colours are mapped back (indexed 0–15 to the named colours).
- **Flattening.** `flatten(&buf, how)` gives the text.
  - For **vt100-made** snapshots: feed `tester.written()` into `vt100::Parser::new(h, w, sb)` and take `contents()`. That is the oracle's emulator on rtwk's bytes, so wrapped-row joining and scrollback match.
  - Trailing spaces stay where the flattener keeps them (TestBackend, first-char). Cursor is not in text snapshots; checkpoints assert it separately.
- **Expected value.** `oracle_snap(name)`:
  - Reads `codex-rs/tui/src/**/snapshots/{name}.snap`, splits off the header, applies `trim_end` and CRLF→LF.
  - `pins.py` checks `git diff --quiet ORACLE_REF -- 'codex-rs/tui/**/snapshots'`, so the working tree is the ORACLE_REF copy and nothing is copied.
  - Inline snapshots: `inline_snap(file, line)`.
- **Insta.** Our own `assert_oracle` (pretty diff) writes `target/parity/{name}.snap.new` for review and never writes under `tui/`. Using insta itself is possible only with `INSTA_UPDATE=no`, and it would still write next to the oracle.
- **Fidelity guard.** In case A the live oracle Buffer must first equal the `.snap` (the skeleton already checks this); in case B the generator checks it.

### D2. Cell and style diff
- **Inputs.** The oracle Buffer comes live (A), from a golden (B: `{name, w, h, rows:[[sym,fg,bg,ul,mods],…]}`), or parsed from one of the 65 Buffer-Debug snaps. The rtwk side is `Tester::cell(x, y)`.
- **Per cell:**
  - split OSC 8 out of the oracle symbol into (link, text);
  - map colours with `convert::color` / `attrs` into rtwk's space, so `Red` equals `Indexed(1)`;
  - skip `tail` cells; `" "` equals empty.
- **Fields:** symbol, link, fg, bg, ul, attrs.
- **Output.** `Vec<CellDelta>`. For a human, both sides go into rtwk's `ScreenFile` text, so `styled_screen` diffs read the same.

### D3. Scenarios: both frontends, one emulator
- **Backend:**
  - L2: `responses::start_mock_server` plus `mount_sse_sequence(turns)` (or `StreamingSseServer` with gates for staged streaming);
  - `MockResponsesConfig::write(codex_home)` and a trusted `[projects]` entry, as in `resize_reflow.rs:590`;
  - for history scenarios, L3: `create_fake_rollout` + `resume <id>`.
- **Frontends.** `codex-tui` (`cargo_bin`) and `codex-rtwk`, same args (`-c analytics.enabled=false`, `--no-alt-screen` for inline, `-C`, prompt), the same env as PtyCodex.
- **Driver.** `TerminalSession::start_with` per binary at the same size answers the probes; input comes from the key script (`send_key`, `send_paste`, `resize`).
- **Checkpoints.**
  - Wait for a screen text, or for quiet: no output for 150 ms and outside a 2026 synchronized update.
  - Then capture rows, `history()` (scrollback), cursor, `styled`.
  - `compare` after `Normalize` (timers to 0s, `/tmp` paths, spinner frames, model names).
- **Modes.** Owned alt-screen mode compares rows plus what the normal screen holds after exit. Inline mode also compares scrollback.
- **Strictness.** Strict-mode `unknown()` is a failure for rtwk and a logged note for the oracle.
- **Headless variant** (fast, deterministic):
  - The same script runs against the oracle side (A live / B goldens) and the rtwk `Tester`, with `Advance` driving FakeClock.
  - A: plus tokio `advance`; B: effects answered by a scripted effect handler.
- **Unix only at first.** The suite and rtwk ConPTY are partial on Windows.

### D4. Hooks for performance (metrics live in perf-compare.md)
- The same `Backend` with `StreamingSseServer` gates gives a controlled token rate.
- The pty runner keeps a timestamped `Vec<TerminalWrite>` per binary (bytes, frames by synchronized update, time to first and last write).
- Headless runs on FakeClock give exact frame and byte counts (`Tester::frames()`, `written()`).
- `rtwk record --keys` makes side-by-side casts.

### D5. Sketches (`codex-tui-parity` unless marked)
```rust
pub struct OracleSnap { pub name: String, pub source: String, pub expression: String, pub body: String }
pub fn oracle_snap(name: &str) -> OracleSnap;            // tui/**/snapshots/{name}.snap, insta-normalized
pub fn inline_snap(file: &str, line: u32) -> String;     // @r#"…"# at file:line, insta-dedented
pub fn assert_oracle(name: &str, actual: &str);          // diff + target/parity/{name}.snap.new, never into tui/
pub enum Flatten { Backend, BufferDebug, FirstChar, PopupTrim, RowsTrim, Vt100, Lines }
pub fn flatten(buf: &Buffer, how: Flatten) -> String;     // TestBackend Display / {buf:?} via ratatui itself
pub fn vt100_contents(bytes: &[u8], w: u16, h: u16, sb: usize) -> String; // oracle's emulator on any bytes
pub struct CellDelta { pub x: u16, pub y: u16, pub field: &'static str, pub oracle: String, pub other: String }
pub fn buffer_diff(oracle: &Buffer, other: &Buffer) -> Vec<CellDelta>;   // OSC 8 split, colours in rtwk space

#[derive(Serialize, Deserialize)] pub enum Step {        // tests/scripts/<name>.jsonl, one Step a line
    Seed(Seed),                    // config TOML, features, auth, model presets, cwd, clock format
    Notify(ServerNotification),    // app-server wire JSON {"method","params"}
    Request(ServerRequest),        // approvals, elicitations, user input
    Key(String), Paste(String), Resize(u16, u16),
    Advance(Duration),             // oracle: tokio paused time; rtwk: FakeClock (+ tokio in case A)
    Check(Check),                  // checkpoint
}
pub struct Check { pub snap: Option<String>, pub width: u16, pub height: Option<u16>, pub flatten: Flatten }
pub trait Side { fn apply(&mut self, step: &Step); fn frame(&mut self, at: &Check) -> Frame; }
pub struct Frame { pub buffer: Buffer, pub bytes: Vec<u8> }   // cells + what reached the terminal

// tui-rtwk/tests/support/rtwk_side.rs
pub fn to_buffer(t: &rtwk_test::Tester, area: Rect) -> Buffer;  // tail → reset " ", link → OSC 8 symbol
pub struct RtwkSide { tester: rtwk_test::Tester /* + host or core */ }  // impl Side
// oracle side: case A tui-rtwk/tests/support/oracle_side.rs over codex_tui::frontend::testing;
//              case B tui-rtwk/oracle/gen_chatwidget.rs, injected, writes tests/data/*.jsonl

// codex-tui-parity/src/pty.rs
pub struct Frontend { pub exe: PathBuf, pub args: Vec<String> }      // codex-tui | codex-rtwk
pub struct Backend { pub model: MockServer, pub home: TempDir }       // responses + MockResponsesConfig
pub fn spawn(fe: &Frontend, be: &Backend, w: u16, h: u16) -> TerminalSession;
pub enum Until { Text(String), Quiet(Duration) }
pub struct Shot { pub rows: Vec<String>, pub scrollback: Vec<String>, pub cursor: (usize, usize), pub styled: String }
pub fn shot(s: &mut TerminalSession, until: Until) -> Shot;
pub fn compare(oracle: &[Shot], rtwk: &[Shot], n: &Normalize) -> Vec<String>;
```
**`oracle-regen.sh`** mirrors `rtwk/oracle/regen.sh`:
- `git worktree add $T $(cat docs/rtwk-port/ORACLE_REF)`;
- copy `gen_*.rs` to `$T/codex-rs/tui/src/parity_gen/`, add `#[cfg(test)] mod parity_gen;` and a dev-dependency on `codex-tui-parity`, in the worktree only;
- run `cargo nextest run -p codex-tui -E 'test(/^parity_gen::/)' --run-ignored only`;
- copy to `tests/data/`, then `git diff --exit-code`.

The generator asserts `flatten(oracle Buffer) == oracle_snap(..)` before writing.

### D6. What needs ADR-003 (and ADR-001) decided
- **Independent:**
  - reader, normalizer, flatteners;
  - `to_buffer`, cell diff;
  - pty runner, wiremock and rollout backends;
  - script format;
  - registry columns;
  - generator skeleton.
  The skeleton's two tests already exercise reader and diff.
- **Depends on ADR-003:**
  - (a) how a fixture reaches rtwk: A, `component.view_model()` live; B, script to `core::update` to VMs;
  - (b) time: A needs the FakeClock↔tokio-paused bridge for an async controller task; B, time is an event and FakeClock suffices;
  - (c) the headless effect runner: A, the embedded app-server plus wiremock; B, scripted effects;
  - (d) whether the 3,914 logic and controller tests cover rtwk for free (A) or must be ported (B);
  - (e) whether the generator is optional (A) or the only oracle path (B).

## E. How to look at it (commands; [ran] = run for this report, [not run] = written, not run)

```sh
# Codex: the tests without building them
cd ~/Documents/dev_projects/CodexCLI/codex-rs
rg -c '#\[(tokio::)?test' tui/src tui/tests | awk -F: '{s+=$2} END{print s}'      # [ran] 6329
python3 -I $TMPDIR/codexcli-research/scripts/classify.py tui                       # [ran] kinds × modules
python3 -I $TMPDIR/codexcli-research/scripts/snapsrc.py .                          # [ran] .snap by origin
find tui -name '*.snap' | wc -l; ls tui/src/chatwidget/snapshots | head            # [ran] 1549
# Codex: list and run (needs the workspace built; RUST_MIN_STACK as `just test` sets it)
cargo nextest list -p codex-tui | wc -l                                            # [not run] cold build
RUST_MIN_STACK=8388608 cargo nextest run -p codex-tui \
  -E 'test(=history_cell::messages::tests::spoken_user_messages_have_a_red_chevron_without_changing_raw_text)'  # [not run]
just test -p codex-tui -E 'test(/bottom_pane::list_selection_view/)'               # [not run]
cargo nextest run -p codex-tui --test all -E 'test(/vt100_/)'                      # [not run] the vt100 suite
cargo nextest run -p codex-tui -E 'test(/insert_history|scrollback|history_tail/)' # [not run] in-crate vt100
cargo nextest run -p codex-tui --test all --run-ignored only -E 'test(/tmux/)'     # [not run] needs tmux + built codex
cargo nextest run -p codex-tui-rtwk --test parity                                  # [not run] the skeleton's parity
# snapshot review
which cargo-insta                                                                  # [ran] not found
cargo install --locked cargo-insta                                                 # [not run]
cargo insta test -p codex-tui --review   # or: cargo insta pending-snapshots / cargo insta review  # [not run]
find tui -name '*.snap.new' -o -name '*.pending-snap'                              # [ran] none
# rtwk: tests, examples, record/play, registry
cd ~/Documents/dev_projects/rtwk
~/.cache/rtwk/acargo nextest run -p rtwk-test \
  -E 'test(=screen_file::matches_the_screen_the_file_holds_whatever_characters_its_legend_gives_the_styles)'  # [ran] 1 passed, 24 s build
~/.cache/rtwk/acargo nextest list -p rtwk-test                                    # [ran] 202 tests, 3 binaries
~/.cache/rtwk/acargo test -p rtwk-test --doc                                      # [not run] the API examples
~/.cache/rtwk/acargo nextest run -p rtwk-e2e --test agent_headless                # [not run]
cargo run -p rtwk-rich --example chat ; cargo run -p rtwk-gallery                 # [not run] interactive
~/.cache/rtwk/acargo build -p rtwk-dev --bin rtwk                                 # [ran] 4 s
target/debug/rtwk record -o hello.cast -- sh -c 'printf "\033[1mhello\033[0m\n"'  # [ran] asciicast v2 written
target/debug/rtwk play --speed 10 hello.cast                                      # [ran] bytes replayed
target/debug/rtwk record --keys -o oracle.cast -- ../CodexCLI/codex-rs/target/debug/codex-tui --no-alt-screen  # [not run]
cut -d, -f14 docs/parity/registry.csv | sort | uniq -c | sort -rn                 # [ran] status counts
column -s, -t < docs/parity/registry.csv | less -S                                # [not run] interactive
# Codex parity registry: docs/rtwk-port/refs/codex-registry.csv does not exist yet (ADR-008);
# for now use research/codex-tui-features.csv and reuse-map.csv
```

## Commands run for this report
- Read with `rg`, `grep -n`, `sed -n a,bp` and `wc -l` over:
  - Codex: `codex-rs/tui`, `core/tests/common`, `app-server/tests/common`, `app-server-test-client`, `.github/workflows`, `justfile`, `.config/nextest.toml`;
  - rtwk: `crates/rtwk-test`, `rtwk-e2e`, `rtwk-dev`, `rtwk-term`, `rtwk-tokio`, `docs/TESTING.md`, `oracle/`, `docs/parity`, `xtask`, `bench`;
  - the insta 1.46.3, ratatui-core 0.1.2 and vt100 0.16.2 sources in `~/.cargo/registry`.
- Scripts in `$TMPDIR/codexcli-research/scripts/`: `classify.py` (6,329 tests by kind and module), `snaps.py` and `snapsrc.py` (`.snap` by format and origin).
- acargo: one rtwk-test test (passed), `nextest list -p rtwk-test`, `build -p rtwk-dev`; then `rtwk record` and `rtwk play` in `$TMPDIR/codexcli-research/cast/`.
- `which cargo-insta` found nothing; nextest is 0.9.151; cargo-mutants is installed. Load average was about 37 during the run (the parallel cold build).
- Codex was not built or run: `codex-rs/target` was missing.
