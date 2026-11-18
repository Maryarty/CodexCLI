# skeleton-layers — parity

The skeleton (`codex-rs/tui-rtwk`) split into layers so that no slice copies a
mixed structure: a UI-free core crate, view-only screen directories, a shell
that names no widget, and JSONL fixtures that seed a real core model. The
behaviour of the skeleton is unchanged (every `port-skeleton` test still
passes, the menu test finds its rows by name now). Every number below was run:
`acargo nextest run --workspace` 143 tests and 1 ignored (host 2 + 4 helper re-runs, core 24
`model` + 13 `script` (+1 ignored: delta D1), UI package 8 `fixtures`, 9 `layers`, 4 `pin`, 16
`rules`, 60 `screens`, 3 `theme`), `clippy --workspace --all-targets -D
warnings`, `fmt --check`, `verify_cargo_workspace_manifests.py` clean, no diff
in `codex-rs/tui`, `codex-rs/Cargo.toml`, `codex-rs/Cargo.lock`.

## Layer map

| layer | where | may name | may not name | held by |
|---|---|---|---|---|
| core | `core/` (`codex-tui-rtwk-core`): ids, rpc, app and thread models, `apply`, `threads::project`, `wire`, `screens/<dir>.rs` x22, `fixtures/` | the protocol crate, serde, std | rtwk, tokio, futures, UI types, IO, clock, prose; no direct runtime or IO dependency (the protocol crate pulls tokio in transitively, core's sources name none) | compiler (`core/Cargo.toml`), `core/clippy.toml` (`disallowed_methods`, `disallowed_types`), `layers.rs` |
| view | `src/screens/<dir>/` x22, `src/ui/`, `src/l10n/`, `src/theme.rs` | core, rtwk widgets, own strings | the protocol, `core::wire`, shell, tokio, IO (`std::fs`, `std::io::stdout`, grouped `use std::{..}`), `apply`, `CodexApp::ingest`, `fixtures::script` | `layers.rs` (a view is every file of `src/` except the shell, `app.rs`, `run.rs`, `lib.rs`: an allowlist), `rules.rs` (R14, ADR-010) |
| shell | `src/shell.rs`, `src/shell/` | core, the protocol, tokio, rtwk `Cx`/`DialogResult` | widgets, views, theme, `rtwk::prelude::*` | `layers.rs`, `rules.rs` (R1, R11, R12) |
| run, host | `src/run.rs`, `host/` | everything | - | `rules.rs` (R1 host) |

The UI model is `UiApp { core: CodexApp, ui: UiState { nav_key, thread_ui,
lang, debug }, link: shell::Link }`. A screen directory is two halves of one name,
`core/src/screens/<dir>.rs` and `src/screens/<dir>/`, made by one macro list
(`screen_list!`, in core, calling a macro of the UI crate too).

## Rules and their bites

Each rule below was shown biting: a violation injected, the test run, the file
restored from a copy (the working tree was clean after each).

| rule | test | injected violation | result |
|---|---|---|---|
| core's direct dependencies are an allowlist (`CORE_DEPS`) | `layers::core_manifest_has_no_ui_or_runtime_dependency` | `rtwk = { workspace = true }`; round 2: `anyhow = "1"` in `core/Cargo.toml` | FAIL, FAIL |
| core names no UI, runtime, IO or clock | `layers::core_sources_name_no_ui_runtime_or_io` | `use std::fs as _;`; round 2: grouped `use std::{fs, io};`, `std::io::stdout()` in `core/src/ids.rs` | FAIL, FAIL, FAIL |
| core holds no prose | `layers::core_sources_hold_no_user_visible_prose` | `format!("Cannot open the file {n}")` in core | FAIL |
| core lints (R1/R11/R12) | `core/clippy.toml` | `std::time::Instant::now()` in core; round 2: a `std::fs::File` type and `codex_app_server_protocol::generate_ts` in core | clippy errors `disallowed_methods`, `disallowed_types` (a `disallowed_types` list: `File`, `TcpStream`, `process::Command`, `Stdout`; the protocol's four generators in `disallowed-methods`) |
| views name no protocol, shell, tokio, IO | `layers::views_never_name_protocol_shell_tokio_or_io` | `use codex_app_server_protocol::ServerNotification;` in `ui/seeded.rs`; round 2: `use std::{fs, io};` in `theme.rs`, `std::io::stdout()` in `ui/seeded.rs`, `use codex_tui_rtwk_core::fixtures::script;` in `ui/seeded.rs`, a new dir `src/newdir/mod.rs` no list names (`is_view` is an allowlist: all of `src/` except `shell*`, `app.rs`, `run.rs`, `lib.rs`) | FAIL x5 |
| views never apply the model | `layers::views_never_apply_the_model` | the text `apply_app(` in `ui/seeded.rs` (first try a call: the crate did not compile, so no bite; redone as a string literal); round 2: `ingest(` in `screens/chat/mod.rs` (`CodexApp::ingest`; `debug_menu.rs` called it through `script::build` and now goes through `Sandbox`) | FAIL, FAIL |
| view dirs hold view files only | `layers::screen_dirs_hold_view_files_only` | `pub struct Model {}` in `screens/chat/mod.rs` | FAIL |
| both halves of every directory | `layers::every_screen_dir_has_a_core_half_and_a_view_half` | a directory `src/screens/zzz/` | FAIL |
| the shell names no widget | `layers::shell_names_no_widget` | `use rtwk::widgets::Text;`; round 2: `use rtwk::prelude::*;` in `src/shell.rs` | FAIL, FAIL |
| seeded writes stay in the seed | `fixtures::a_seeded_submit_sends_no_command_to_the_backend` | red in the fix-round red commit (`43c067ad7`: the seeded chat's submit went to the live `Hs`, `StartTurn` reached the fake backend); green with `SeededScope` and its own `Hs` over a core `Sandbox` | FAIL, then PASS |
| the UI model is core plus UI state | `layers::the_ui_model_is_the_core_model_plus_ui_state` | the word `nav_key` in `core/src/app.rs` | FAIL |

The compiler holds the strongest one: core's manifest has no rtwk, so a widget
in core does not build. The R1-R14, ADR-002 and ADR-010 tests of
`port-skeleton` (`rules::*`, 16) pass unchanged except that their scans cover
both crates' sources (`tests/srcscan`).

## Fixtures

A script is JSONL, one step a line, `#` lines and blank lines skipped, a bad
line is an error with its line number (`fixtures::script::parse`):

| step | line | what it does |
|---|---|---|
| `seed` | `{"step":"seed","seed":"session","thread":"thread-1"}` | `Connected` and the reply to its `StartThread`, through `ingest`: a ready session, `thread-1` active |
| `screen` | `{"step":"screen","dir":"resume","ev":{"kind":"listed","rows":[..]}}` | the dir's own seed fn (`core/src/screens/<dir>.rs`: `seed(value) -> Result<Ev, SeedError>`), dispatched by `screens::seed_ev` from `screen_list!`; the `Ev` goes through `ingest`. A dir without a seed answers `NoSeeds`, an unknown dir `UnknownDir`, a bad event `Bad`, each with the line number. A slice adds its seeds in its own file and never edits `script.rs` |
| `notify` | `{"step":"notify","message":{"method":"turn/started","params":{..}}}` | the protocol's own JSON of a `ServerNotification` |
| `request` | `{"step":"request","message":{"method":"item/commandExecution/requestApproval","id":1,..}}` | the protocol's own JSON of a `ServerRequest` |
| `key`, `resize` | `{"step":"key","keys":"down"}`, `{"step":"resize","width":60,"height":20}` | handed back in order (`Played.inputs`) for a tester (`Harness::play_inputs`) |

Steps change the model only through `CodexApp::ingest`, the live app's path
(epoch check included), so a script cannot make a state the app cannot reach.
Views never name `ingest` or `fixtures::script`: they get `fixtures::Sandbox`
(`play(text)`, `app()`, `send(Intent)`: applies and drops the effects).
A `DebugEntry` carries `script: include_str!(..)`; the debug menu plays it (through `Sandbox`) into
a fresh model, wraps the screen in `Seeded::scope` (providers of the seeded
session, thread, request queue and screen states, shadowing the app's) and
shows the real widget inside a `SeededScope`, which owns its `Hs` (handlers over the sandbox) so a submit or dismiss never reaches the live model or the backend. `EntryOpen::Route` pushes a route, `EntryOpen::Dialog`
shows a dialog (the builder returns the whole dialog).

| example | script | oracle | shows |
|---|---|---|---|
| chat transcript with a streamed answer | `src/screens/chat/fixtures/streamed_answer.jsonl` | `live_app_server_turn_completion_repairs_dropped_message_deltas` (`chatwidget/tests/app_server.rs:1842`) | `hello`, then `The transport kept this.` (the rest is delta D1) |
| exec approval request | `src/screens/approvals/fixtures/exec_approval.jsonl` | `app_server_exec_approval_request_splits_shell_wrapped_command` (`chatwidget/tests/approval_requests.rs:55`; its helper `exec_approval_request_from_params` is in `chatwidget.rs:906`, not `helpers.rs`): a command approval for thread-1 | the skeleton's dialog, `1 needs an answer.`, open on the seeded queue |
| resume list from a fake rollout | `src/screens/resume/fixtures/fake_rollouts.jsonl` | the three `create_fake_rollout` calls of `app-server/tests/suite/v2/thread_list.rs:713-736` | three rows, each `<timestamp>  Hello  mock_provider`, from `2025-01-02T12:00:00Z` down |

## Oracle tests

| oracle test | ours | ported | n/a: reason | delta |
|---|---|---|---|---|
| `chatwidget/tests/app_server.rs::live_app_server_turn_completion_repairs_dropped_message_deltas` | `script::live_app_server_turn_completion_repairs_dropped_message_deltas` (core, the oracle's whole transcript; `#[ignore]`d red until session-core), `script::d1_the_turn_completion_does_not_repair_the_streamed_text_yet` (the current behaviour) and `fixtures::the_chat_entry_shows_the_streamed_answer` (the screen) | the scenario with the oracle's own events (`turn/completed` and its summary items kept); the repair is missing | | D1 |
| `app-server/tests/suite/v2/thread_list.rs` (the three-rollout list) | `script::a_screen_step_goes_to_the_dirs_own_seed`, `fixtures::the_resume_entry_lists_the_fake_rollouts` | data only | n/a: the picker's behaviour is S18's | D2 |
| `chatwidget/tests/approval_requests.rs::app_server_exec_approval_request_splits_shell_wrapped_command` (data from `chatwidget.rs:906`) | `script::a_request_step_queues_a_request`, `fixtures::the_approvals_entry_shows_the_request` | data only | n/a: the approval dialog is S13's | |

Tests of this task, no oracle: `script::*` (13 and 1 ignored: blank and comment lines, the
error line, key and resize order, notify equals the protocol's JSON, the
session seed, the request step, the screen step for a dir (resume), an unknown dir, a dir without seeds, a bad event, the
sandbox, the D1 current behaviour, a script error in `play`; ignored: the D1 oracle test), `fixtures::*` (8, with `every_debug_entry_script_parses_and_applies`,
`a_seeded_entry_leaves_the_real_model_alone`, `a_seeded_submit_sends_no_command_to_the_backend` and `a_seeded_dismiss_rejects_nothing_on_the_backend`), `layers::*` (9).
Note: `a_seeded_dismiss_rejects_nothing_on_the_backend` does not bite: it also
passes with the seeded Dismiss made a no-op (checked: `SeededMsg::Dismiss`
mapped to `Rebuild::No`; the screen assertions and the backend assertion all
still held). It guards the path against a future change, no more; the submit test is
the one that bites. The loader
and fixtures tests were red in the first commit (`todo!()` loader, no entries)
and green after.

## Snapshots
`debug/menu.snap` (ours, no oracle): the heading rows only (the directories'
names), at 80x120: 22 directories do not fit the 24 lines of an oracle screen,
which a debug menu need not fit. The entry rows are not in the golden, so a
slice that adds an entry never edits it; `fixtures::*` find the entries by
name (`Harness::open_debug_row`) and `debug_menu_lists_the_directories` builds the expected rows
from `screens::dirs()`.

## Visual deltas
### D1 — streamed answer: the repair of dropped deltas from `turn/completed` is missing
kind: K01 (behaviour of the skeleton's projection, not a look)
cause: other: skeleton. The oracle repairs the dropped deltas from the summary
items of `turn/completed` (and consolidates the message); `threads::project`
ignores a turn's items, so the skeleton shows the delta it received only. The
fixture keeps the oracle's events as they are (`turn/started`, the user item,
the first delta, `turn/completed` with its summary items), nothing rewritten
to hide the gap. session-core owns the repair: the oracle test is ported in
core as `script::live_app_server_turn_completion_repairs_dropped_message_deltas`,
red and `#[ignore]`d (`delta D1`), to lose its `#[ignore]` when the repair
lands; `script::d1_the_turn_completion_does_not_repair_the_streamed_text_yet`
and `fixtures::the_chat_entry_shows_the_streamed_answer` pin what shows today.
The oracle's directive line (`::code-comment`) and question block are the
chat slice's.
oracle (the consolidated source):
```
The transport kept this.
And dropped this.
```
ours (`fixtures::the_chat_entry_shows_the_streamed_answer`):
```
› hello
• The transport kept this.
```
proposal: accept until session-core ports the repair; then un-ignore the core
test and flip the two current-behaviour assertions.

### D2 — resume list: rows are our own, not `Thread` JSON
kind: K01
cause: other: skeleton. `thread/list` answers `Thread` values (a wide struct, most of it
not a picker's); the seed takes the fields
`create_fake_rollout` takes (id, preview, provider, start time) and the model's
`Row` keeps those. The resume slice maps the real `thread/list` reply into
`Row`s in `resume::decode`.
oracle: `thread/list` data, three rollouts, previews `Hello`.
ours: three rows, each `<timestamp>  Hello  mock_provider`, newest first.
proposal: accept; the resume slice replaces the list and may widen `Row`.

## What did not split cleanly
- `shell::dialog_closed` stays in the shell: it builds no widget but names
  `rtwk::Cx` and `DialogResult`, because R1 (spawn only in the shell) is what
  lets it await a dialog's future. The view reaches it through the injected
  `CloseWatcher` (`run.rs` passes it). rtwk gap: `show_dialog` has no
  on-closed message, so every presenter needs a spawned watcher.
- A seeded screen's handlers (`Hs`) now are its own: `SeededScope` is a
  stateful widget that holds a core `Sandbox`, makes its own `Hs` from its
  `Cx` (`cx.map`) and provides it (with the session, thread, request queue and
  screen states) through a `MultiProvider`; a submit or dismiss changes the
  sandbox, never the live model or the backend
  (`fixtures::a_seeded_submit_sends_no_command_to_the_backend`). The first
  round claimed this could not be done; it was wrong.
- `FAKE_THREAD` and `shell/fake.rs` (the scripted backend) stay in the shell:
  they are an effect runner's test double, not fixtures of a screen.
- `Key` and `Resize` steps do nothing in the debug menu (no tester there);
  tests play them with `Harness::play_inputs`.

## Docs
`architecture.md`: ADR-001 (the layout of the workspace; core "has no direct
runtime or IO dependency": the protocol crate pulls tokio in transitively),
ADR-002 (per-thread widget state lives in `UiApp.ui`, not in `CodexApp`; `Lang`
likewise), the Skeleton file list, a "Layers" section (with `UiApp.link`) and
gap G16 (rtwk `show_dialog` has no on-closed message: why `shell::dialog_closed`
exists). `design.md` (and `design.html`, regenerated; the Mermaid diagrams
parse with `mmcheck`): §3 shows the two crates with only the allowed arrows,
§3b the layers table, §6 the class diagram `UiApp { core, ui: UiState {
thread_ui }, link }`, and the ThreadUi wording at lines 10, 47, 168, 175-176,
203, 213, 248 and 391.

## Fix round (the Opus review, 8 items)
1. Seeded screens wrote through the live `Hs`: `SeededScope` with its own `Hs`
   over `Sandbox`; test `a_seeded_submit_sends_no_command_to_the_backend` (red,
   then green); the "did not split" paragraph corrected.
2. `debug/menu.snap` heading-only (80x120); entries found by name.
3. `{"step":"screen","dir":..,"ev":..}` dispatched by `screen_list!` to each
   dir's `seed`; ResumeList moved to `resume`'s seed; `script.rs` is closed to
   slices.
4. Layer-rule gaps: grouped imports, `std::io::stdout()`, `rtwk::prelude::*` in
   the shell, `ingest`/`fixtures::script` in views, `is_view` an allowlist;
   `core/clippy.toml` `disallowed_types` and the protocol generators (bites
   above).
5. Core's direct dependencies are an allowlist (`CORE_DEPS`); docs and the
   `core/Cargo.toml` and `lib.rs` comments say "no direct runtime or IO
   dependency".
6. G16 added to the architecture gaps.
7. D1 rewritten: the oracle's `turn/completed` kept in the fixture; the
   repair is an ignored red core test for session-core; the exec data source
   is `approval_requests.rs:55` (helper `chatwidget.rs:906`).
8. `UiApp.link: shell::Link` shown in the Layers docs, design.md §6.
