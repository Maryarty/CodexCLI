# Reuse plan: what tui-rtwk carries from codex-tui, and how

Research task `reuse-plan` (2026-11-18), input for `reuse-carry` and
`agents-live`. Oracle: `rust-v0.170.0` (the integration branch moves there in
`oracle-0170`; between `c6a41e32` and the tag, `agents_overview*` lost 1,243
lines and gained 284). Scope changed mid-task by the person: **VS1 is now
only the Agent command center, live** (`agents_overview*.rs`, the RPCs it
needs, the `codex agents` entry); chat and session moved to VS2, which §3
covers more briefly.

## 0. Synthesis

- **Including files by path works, and it is proved.** A crate under `/tmp`,
  added as a member of a `rust-v0.170.0` copy of codex-rs, compiled 11 oracle
  files from `tui/src` with `#[path]` and **ran 71 of their own tests
  unchanged, all green**: chunking, app_command, app_server_event_targets,
  text_formatting, width, line_truncation, plus thread_events,
  thread_event_buffer, pending_interactive_replay and replay_filter, for
  which 156 lines of shims stand in for the `crate::…` names they import.
  Clippy with codex-rs's lints reported only dead code.
- **The agent command center cannot be carried unchanged.** Every
  `agents_overview*` file except a 19-line loading stub is ADAPT in
  reuse-map: it is `impl App` (async, `AppServerSession`, `ChatWidget`) or a
  ratatui `BottomPaneView`. Including it by path would need a stand-in for
  `App` and `ChatWidget`, which is the seam architecture.md §0 rules out.
  What can be reused:
  (1) three leaf REUSE files included by path (631 prod lines, 35 oracle
  tests);
  (2) about 570 lines of pure functions in the agent files, copied verbatim
  and held to the oracle by a drift test on each item. A prototype of that
  test, run between `c6a41e32` and the tag, caught 3 real changes (§2.3).
  The rest of the 5.7k ADAPT lines is behaviour to retype in rtwk widgets
  and in core `apply`: about 20% reused, 80% retyped.
- **The by-path route has four requirements**, each found by the proof:
  1. `#[rustfmt::skip]` on every include. Without it, `cargo fmt` reformats
     the oracle file in place (shown in §1).
  2. Feature parity with codex-tui (`serde_json/preserve_order`; without it
     3 text_formatting tests fail).
  3. The layers scan must follow `#[path]`. Today it reads only `core/src`.
  4. `#[allow(dead_code)]` on the include.
- **Decisions for the person** (§4): may `codex-tui-core` take
  `codex-protocol` (oracle `ThreadId`) and `codex-config` as direct
  dependencies? Does core keep the oracle's `ThreadEventStore` as each
  thread's log? Is crossterm allowed as key data in the UI crate (keymap)?

## 1. The methods, and the proof

| method | how | follows the oracle | cost |
|---|---|---|---|
| (a) by path | `#[rustfmt::skip] #[allow(dead_code)] #[path = "../../../tui/src/x.rs"] mod x;`; a shim module for each `crate::…` path the file imports | every edit, at compile time; a new import or a changed protocol type breaks the build at the repin, which is the signal we want | shims; feature parity with codex-tui; the file must compile with only the shim around it |
| (b) verbatim copy + drift test | copy one **item** (fn, enum, struct) with a marker `// oracle: <file> <kind> <name>`; a rules test compares its normalized tokens with the item in `codex-rs/tui/src` (the oracle at `ORACLE_REF` in the same checkout) | at the next test run, which fails | an update by hand at each repin |
| (c) retype | behaviour re-expressed in core `apply` or rtwk widgets, with the oracle's tests ported as parity tests | the parity tests only | the full cost |

Whole-file copies (b) are never better than (a): a file that compiles copied
also compiles by path. (b) is for *fragments* of files that cannot be
included.

**Proof (a).** The proof crate `codex-reuse-proof` was a member of the
`rust-v0.170.0` copy of codex-rs, so it used that copy's lockfile and patches
(`CARGO_INCREMENTAL=0 acargo test`; the target dir, 7.6 GB, is deleted).

| included by path | shims needed | oracle tests run |
|---|---|---|
| `streaming/chunking.rs` | none | 9 |
| `app_command.rs` (+ `app_command_tests.rs`: nested `#[path]` resolves from the oracle dir) | none (dependencies: codex-protocol, codex-config) | 2 |
| `app/app_server_event_targets.rs` | `test_support` (2 re-exports) | 7 |
| `text_formatting.rs`, `width.rs` (+ `line_truncation.rs`, tests only) | none; dev-dependency `ratatui` for width's comparison test | 25 + 3 + 1 |
| `app/thread_events.rs`, `app/thread_event_buffer.rs` (+ tests), `app/pending_interactive_replay.rs`, `app/replay_filter.rs`, `session_state.rs`, `backend_id.rs` | a shim `app` module (the names `use super::*` gives: protocol re-exports, `mpsc`/`Mutex`, `THREAD_EVENT_CHANNEL_CAPACITY`, `WindowsSandboxHost`, `SideParentStatus` copied, `recap::RecapProgress` copied from `app/recap.rs:425-448`); `app_event` (`FeedbackCategory`, `FeedbackReceipt` copied, `HistoryLookupResponse` stub); `bottom_pane::FeedbackAudience`; `chatwidget` (`realtime_delegation_input` and `is_private_realtime_agent_item` copied from `chatwidget/realtime.rs:206-238`; `ThreadInputState` stub) | 10 + 1 + 13 |

The four requirements, as the proof found them:

- **rustfmt.** A badly formatted file included by path showed
  `Diff in …/tui/src/zz_fmt.rs` under `cargo fmt --check`. A plain
  `cargo fmt` would have rewritten the oracle. With `#[rustfmt::skip]` on
  the `mod` item the file is not touched. (Today both workspaces have the
  same `rustfmt.toml` and toolchain, but that does not protect a later
  bump.)
- **Features.** `text_formatting::format_json_compact` depends on
  `serde_json/preserve_order`, which codex-tui turns on. Without it 3 oracle
  tests fail. A crate that includes oracle files must enable codex-tui's
  features for the dependencies those files use.
- **Layers.** `tests/layers.rs` reads only `core/src/**`. An oracle file
  included into core is not scanned, so `thread_events.rs` would pass even
  though it uses `tokio::sync::mpsc` through the shim. The scan must follow
  `#[path]`, and the shims count as core sources.
- **Dead code.** Oracle items the port does not use raise warnings, and
  clippy runs with `-D warnings`. `#[allow(dead_code)]` on the include is
  enough; it is not `clippy::disallowed_*`, so R1 does not cover it.

**Proof (b).** `/tmp/rp/frag.py` (prototype, not committed) extracts an item
from the oracle by kind and name, normalizes `pub(…)` to `pub` and
whitespace, and compares. Between `c6a41e32` and `rust-v0.170.0` it reported
`list_pinned_threads`, `matches_query` and `SourcePage::fill` as DRIFT, and
`is_overview_thread` as new. The 26 other fragments are equal. The real
test should use `syn` (dev-dependency) and compare token streams.

## 2. VS1: the Agent command center (decided)

### 2.1 Oracle files and their routes

Prod lines at the tag. "map" is the reuse-map category (at c6a41e32).

> **SUPERSEDED 2026-11-18 (rtwk first, process.md §5).** The row for `text_formatting.rs` + `width.rs` (carried by path for display widths and truncation) is dropped: widths and
> end ellipsis come from rtwk-unicode and `TextOverflow::Ellipsis`; a missing helper is our own small function held to the oracle's tests
> of it (not the carried file). Only the domain formatters of the other rows stay.

| oracle file | map | prod | route | layer | oracle tests |
|---|---|---|---|---|---|
| `app/agents_overview.rs` | ADAPT | 1345 | (c): `AgentsOverviewState`, open, refresh apply, attach, stop → core `screens/agents` model + intents, shell effects | core / shell | via `agents_overview_tests.rs` |
| `app/agents_overview_view.rs` | ADAPT | 901 | (c) view in rtwk widgets; (b) fragments §2.2 | UI / core | — |
| `app/agent_center/{mod,navigation,input}.rs` | REUSE* | 238 | (b) fragments (they are `impl AgentsOverviewView`, which owns a `Mutex` state and an `AppEventSender`); `input.rs` → rtwk key handlers | core / UI | `agent_center_tests.rs` 16 (13 snap) |
| `app/agent_center/{render,rows,hints}.rs` | ADAPT/REUSE* | 757 | (c) rtwk widgets (ratatui `Line` and `Buffer` code) | UI | snapshots |
| `app/agents_overview_discovery.rs` | ADAPT | 357 | (b) RPC paging fragments → shell; filters → core; `impl App` search → (c) | shell / core | `…_discovery_tests.rs` 2 |
| `app/agents_overview_threads.rs` | ADAPT | 524 | (c) notification tracking → core `apply`; (b) `detail_thread_ids` | core | in overview tests |
| `app/agents_overview_details.rs` | ADAPT | 303 | (b) `preview_text`, `request_preview`; (c) activity tracking | core | — |
| `app/agents_overview_grouping.rs` | ADAPT | 163 | (b) `model_name`, `is_pinned`, `same_group`; grouping persistence → shell | core | — |
| `app/agents_overview_actions.rs` | ADAPT | 478 | (c) pin, archive, delete, stop via modal → intents + dialogs | core / UI | `…_actions_tests.rs` 12 (5 snap), progress 1 |
| `app/agents_overview_usage.rs` | ADAPT | 225 | (c) refresh every 60 s → shell timer; `usage_lines` → rtwk widgets | shell / UI | `…_usage_tests.rs` 2 (2 snap) |
| `app/agents_overview_{new,selection,errors,loading}.rs` | ADAPT/REUSE | 438 | (c) new-task flow and selection → intents; errors and loading → widgets | core / UI | in overview tests |
| `app/app_server_event_targets.rs` | REUSE | 239 | **(a)** maps a notification or request to its thread (`server_notification_thread_target`) | core (needs codex-protocol, §4) | 7, by path |
| `text_formatting.rs` + `width.rs` | REUSE | 392 | **(a)** `center_truncate_path` and display widths for the rows | UI | 25 + 3, by path |
| `status/thread_usage.rs`, `status/helpers.rs`, `resume_picker.rs` | REUSE/ADAPT | — | (b) fragments: `thread_cost_micros`, `format_credit_micros`, `format_estimated_usd_micros`, `format_tokens_compact`, `format_directory_display`, `format_relative_time` (they sit in ratatui- or tokio-coupled files) | core (cost) / UI (text) | — |
| `app/background_requests.rs` | ADAPT | 1377 | (b) `fetch_thread_usage` only → shell | shell | 1 |

\* reuse-map marks `agent_center/{mod,navigation,input,hints}.rs` REUSE.
Its scanner saw no ratatui import, but the files are `impl` blocks on the
ratatui view, or (hints) depend on renderers. They are not carriable whole.

The oracle tests of the screen (`agents_overview_tests.rs` 44 tests with 26
snapshots, `agent_center_tests.rs`, actions, usage, discovery) all build
`make_test_app_with_channels`. They are the parity spec for `agents-live`,
ported as core state tests and screen tests (process.md §3). None of them
runs unchanged.

### 2.2 Fragments to copy (b), with their layer

- **core** (`screens/agents`, about 260 lines):
  - `AgentsOverviewGroup` + `for_thread` + `is_subagent`: status grouping
    from the protocol `Thread`, derived and not stored;
  - `display_title`, `model_name`, `is_pinned`, `same_group`;
  - `TASK_FILTERS` (its group mapping; the labels go to the UI's strings);
  - `center_rows`, `selectable_indices`, `reconcile_command_center_selection`,
    `page_selection`, `move_selection`, `selection_after_removal`;
  - `matches_query`, `is_overview_thread`, `supports_shared_pinning`;
  - `detail_thread_ids`, `preview_text`, `request_preview`, `thread_cost_micros`;
  - the `AgentsOverviewViewState` fields minus `TextArea` and `completion`,
    split into core state and `UiApp.ui`.

  Each item is rewritten from `&self` on the view to the core model's
  types, so its drift test compares the body with a declared rename map.
  An item that cannot keep its body is (c), not (b).
- **shell** (about 190 lines): `listed_threads`, `list_pinned_threads`,
  `SourcePage`, `AgentsOverviewDiscovery::{has_more, next_batch}`,
  `SourcePage::fill` (`thread/list` paging across the two source groups,
  merged by recency), `fetch_thread_usage`. They take an
  `AppServerRequestHandle`, which the shell already has.
- **UI** (about 120 lines): the number, path and relative-time formatters
  (`format_relative_time` takes `reference` as a parameter, so it needs no
  clock).
- `directory_group_key` and `AgentsOverviewProjectGroup::for_thread` call
  `dunce::canonicalize` and `codex_git_utils::repository_identity`, which
  are file-system IO. They belong in the shell, which computes a group key
  per cwd and hands it to core. They do not go into core.

### 2.3 RPCs it needs (all in the shell, typed by core tickets)

`thread/list` (discovery, pinned, search; `ThreadSortKey::{RecencyAt,
SectionPosition}`, `PINNED_THREAD_SECTION_ID` from `codex-state`),
`thread/loaded/list`, `thread/read` and `thread/turns/list` (details),
`thread/section/move` (pin), `thread/archive`, `thread/delete`,
`thread/unarchive`, `thread/unsubscribe`, `turn/interrupt` (stop),
`account/rateLimits/read` and thread usage, and `thread/start` / `thread/resume`
(open, new task; these come with VS2). In the oracle these go through
`AppServerSession` methods. VS1 needs only about ten of its 93 methods; the
shell calls `request_typed` on the handle directly, the way the discovery
fragment does. Carrying `app_server_session` whole is VS2 work (§3).

### 2.4 What it replaces in tui-rtwk

- `core/src/screens/agents.rs` (238 lines): its retyped `Task`/`Status`/
  `Child` become rows of the protocol `Thread`, plus the carried
  `AgentsOverviewGroup::for_thread`, `TASK_FILTERS` and the navigation
  fragments.
- `src/screens/agents/mod.rs` (492) is rebuilt on those rows;
  `strings.rs` (152) takes the oracle's labels.
- `tests/agents.rs` (73) grows into the ported oracle tests.

### 2.5 Numbers for VS1

Oracle prod lines: 5,729 (`agents_overview*` 4,734 + `agent_center` 995).
Reused: 631 by path (+35 oracle tests that run unchanged) plus about 570 as
fragments under drift tests, about 1.2k in all (≈20%). Retyped: about 4.5k:
around 1.6k of ratatui rendering becomes rtwk widgets (view, render, rows,
hints, errors, loading); around 2.9k of `impl App` orchestration becomes core
`apply` and shell effects.

## 3. VS2: session, streaming, keymap (brief)

| oracle file(s) | map | prod | route | layer | notes |
|---|---|---|---|---|---|
| `app_command.rs` | REUSE | 390 | **(a)** proved | core | needs codex-protocol + codex-config (§4); replaces the hand-built approval replies in `core/src/screens/approvals.rs` |
| `app/thread_events.rs`, `thread_event_buffer.rs`, `pending_interactive_replay.rs`, `replay_filter.rs` | REUSE/ADAPT | 1104 | **(a)** proved with shims | core if §4 says yes (shim `mpsc`/`Mutex` → `std::sync`; only `ThreadEventChannel::new` touches them) | 24 tests come with them; they back `core/src/threads.rs` pending requests and replay |
| `session_state.rs`, `backend_id.rs` | REUSE | 90 | (a) proved | core (`BackendId::new` uses `uuid::new_v4`: the shell mints it) | — |
| `app/thread_routing.rs` | ADAPT | 2261 | (c) the interrupt race (`:806-855`) and switch rules → core `apply`; the pure helpers as fragments | core | — |
| `streaming/chunking.rs` | REUSE | 295 | **(a)** proved | core (the queue age is passed in; no clock) | adaptive commit pace, 9 tests |
| `streaming/table_holdback.rs`, `table_detect.rs` | REUSE | 512 | (a), unproved; `markdown_render::preferences::current()` is a global, so its shim must read the setting from the model | core | newline and table holdback on source text |
| `streaming/{controller,commit_tick,mod,render,code_fence,prose_preview}.rs`, `markdown_stream.rs` | ADAPT/REUSE | 2.1k | (c): ratatui lines and `HistoryCell` | — | rtwk-rich renders; commit semantics come from chunking + holdback |
| `app_server_session.rs` + 18 submodules | REUSE | ~5.9k | (a), unproved | shell (host) | closure: config_update, local_settings, resume_permissions, service_tier_resolution, managed_worktree_tools, dynamic_tools_mcp, collaboration_modes, windows_sandbox (all REUSE, by path) + shims `AppEvent`/`AppEventSender` (the variants the files send, forwarded into the rtwk channel), `status::{StatusAccountDisplay, plan_type_display_name}`. Not clean: `history.rs` calls `thread_transcript::thread_items_to_transcript_cells` and `side_chat_history.rs` names `ChatWidget`, both renderers, so they would need stub shims, which an uncalled-function rules test must guard. Recommended: prove it in `reuse-carry`'s first step before committing to it |
| `keymap.rs` + `keymap/{bindings,leader,vim_search,chords}.rs` | REUSE | 4.8k | (a), unproved | UI | crossterm `KeyEvent` throughout (386 references), and `chords.rs` pulls `bottom_pane::ShortcutHud`, `keymap_setup` and tokio. It needs crossterm as a key-data dependency, one rtwk→crossterm key conversion at input, and a `key_hint` shim |
| `markdown_copy.rs`, `clipboard_*` | REUSE/REPLACE | 1.9k | markdown_copy (c) (it is built on `terminal_hyperlinks::HyperlinkLine`, a ratatui line; REPLACE); clipboard OS side (a) in the shell (`worker.rs` needs a `FrameRequester` shim) | shell | G11 |
| `slash_command.rs` | REUSE | 333 | (a) possible, no dependencies; carries prose (`description()`), which collides with core's no-prose rule | UI or core with an exemption | already retyped as `core/src/commands.rs` (902) |
| `analytics/{models,normalize,report_data,tokens}.rs` | REUSE | 879 | (a), no `crate::` UI dependencies | core | `/usage` dashboard; not VS1 or VS2 |

## 4. Layer questions (to decide before `reuse-carry`)

1. **Core direct dependencies.** Allowed today (`CORE_DEPS`): protocol,
   serde, serde_json. Carried core files also need:
   - `codex-protocol` (`ThreadId`, approvals types; already in the build
     through the protocol crate, and upstream-friendly);
   - `codex-config` (`app_command`'s `ApprovalsReviewer`; table preferences).

   Recommended: allow both. The alternative is (c) for `app_command`,
   `pending_interactive_replay`, `thread_events` and `event_targets`, about
   2k lines retyped.
2. **Thread log.** Today core keeps one live projection per thread. The
   oracle keeps a bounded `ThreadEventStore` and replays it on switch, with
   rules: private items hidden after a voice handoff, resolved misalignment
   errors omitted, pending-request replay, delta coalescing.
   Recommended: keep the projection and carry the store by path as each
   thread's request and replay policy. Its 24 tests then guard the rules
   instead of a retype. `session-core` decides.
3. **Key data.** Allowing crossterm `KeyEvent` as plain data in the UI crate
   lets keymap be carried (4.8k lines, ~1.5k test lines). Without it,
   keymap is (c) over rtwk keys (architecture.md §0's current text).
   ratatui-core in the UI crate is needed only for (c) renderers, which are
   not carried, so the answer there is no. One exception: a ratatui
   dev-dependency for width's comparison test.

## 5. Carry plan (`reuse-carry`) and the rules test

Order, each step its own commit after `oracle-0170` lands:

1. `tests/carried.rs` and the carried list. Every `#[path]` into
   `codex-rs/tui/src`:
   - resolves to a file;
   - carries `#[rustfmt::skip]` and `#[allow(dead_code)]`;
   - is listed in the test's table with its layer.

   Every `// oracle: <file> <kind> <name>` marker is followed by an item
   whose normalized `syn` token stream equals the oracle's (with the
   declared rename map). `layers.rs` scans the include targets with the
   rules of the crate that includes them. Features match codex-tui's for
   the dependencies the included files use.
2. **For `agents-live` (VS1):**
> **SUPERSEDED 2026-11-18 (rtwk first, process.md §5).** In this list drop `text_formatting.rs` + `width.rs` (see the note above the table in §2.1); rows use rtwk-unicode widths and `TextOverflow`.

   - `app_server_event_targets.rs` (core, after §4.1) and
     `text_formatting.rs` + `width.rs` (UI) by path;
   - the core fragments of §2.2 into `screens/agents`, with their drift
     markers;
   - the shell fragments (discovery, pinned, usage fetch);
   - the UI formatters.

   `agents-live` then ports the oracle tests of §2.1 against them.
3. **For `session-core` (VS2):**
   - `app_command.rs`, `session_state.rs`;
   - `thread_events` + `thread_event_buffer` + `pending_interactive_replay`
     + `replay_filter` with the `app` shim (after §4.1 and §4.2);
   - `chunking.rs`, `table_detect.rs`, `table_holdback.rs`;
   - prove `app_server_session` with its shims, or fall back to fragments
     of its param builders (`thread_start_params_from_config` and the
     like).
4. **For `chat-live` (VS2):** keymap after §4.3; clipboard OS side in the
   shell.

Lines for VS1 by this plan: about 1.2k oracle lines reused (631 by path +
about 570 fragments), about 4.5k retyped, and 35 oracle tests running
unchanged. The 77 oracle tests of the screen (47 snapshots) are still ported by hand.
For VS2, the proved files alone are 1,879 prod lines by path with 35
oracle tests;
with app_server_session and keymap, if proved, about 12k.

## 6. Live VS1 implementation (2026-11-19)

Choose the reuse boundary at the call site. Keep the pinned oracle as the
behaviour reference and use the port's ADRs, data ownership and measured
cost to choose the implementation.

| Area | Implementation |
|---|---|
| Command-center reads and request previews | Carry `listed_threads` and `request_preview` verbatim under the existing drift test. Keep receipts with their listed rows. Retain the already carried grouping, filtering and formatting fragments. |
| Discovery | Share the oracle's recency ordering across merging and detail selection. Move owned wire rows through the merge and retain the existing per-source continuation contract. |
| Protocol targets | Match the oracle's thread-bearing request variants on the typed protocol enum. Borrow the exact backend ID and keep numeric and string JSON-RPC IDs distinct. Notification routing covers the events consumed on this live path. |
| Command-center actions | Look up the visible root in the existing catalog. The rtwk view uses the full row projection when drawing the list. |
| Session ownership and transport | `CodexApp` and `ThreadModel` own session state under ADR-003. The existing host transports protocol commands and executes effects. Reuse of individual session helpers goes through these boundaries. |
| VS2 consumers | `session-core`, `keymap`, `chat-live` and `usage` pick up the reusable logic when their live call sites are in place. rtwk owns input routing, frame pacing and the rendering and selection state. |

The component benchmark compared the integration tree `674b13c8c` with
this slice using the same Debug build profile and Rust toolchain. A
counting allocator measured allocations and requested bytes, including
reallocations. Fixture construction was outside each timed loop. The
median is from eight runs per side, alternating run order. Lookup used the
last root in catalogs built from the 30-row fixture. Merge used two
10-row pages with 880-byte previews and took the first 10. Each lookup and
merge sample ran 2,000 operations. Request ingress queued one typed
tool-input request per app across 10,000 apps.

| Operation | Allocations before → after | Bytes before → after | Median time before → after |
|---|---:|---:|---:|
| Lookup, 30 rows | 310 → 0 | 79,122 → 0 | 31.1 → 0.3 µs |
| Lookup, 300 rows | 3,019 → 0 | 1,268,922 → 0 | 282.7 → 2.0 µs |
| Decode and merge two pages | 381 → 91 | 143,589 → 58,565 | 203.6 → 182.8 µs |
| Request ingress | 20 → 3 | 1,838 → 411 | 5.6 → 2.0 µs |

The measurements cover the named operations inside the application.
S11 and S12 measure the end-to-end terminal latency with release builds.
