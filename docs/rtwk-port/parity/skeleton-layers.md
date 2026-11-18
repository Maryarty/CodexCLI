# skeleton-layers — parity

The skeleton (`codex-rs/tui-rtwk`) split into layers so that no slice copies a
mixed structure: a UI-free core crate, view-only screen directories, a shell
that names no widget, and JSONL fixtures that seed a real core model. The
behaviour of the skeleton is unchanged (every `port-skeleton` test still
passes, the menu test finds its rows by name now). Every number below was run:
`acargo nextest run --workspace` 137 tests (host 2 + 4 helper re-runs, core 24
`model` + 9 `script`, UI package 6 `fixtures`, 9 `layers`, 4 `pin`, 16
`rules`, 60 `screens`, 3 `theme`), `clippy --workspace --all-targets -D
warnings`, `fmt --check`, `verify_cargo_workspace_manifests.py` clean, no diff
in `codex-rs/tui`, `codex-rs/Cargo.toml`, `codex-rs/Cargo.lock`.

## Layer map

| layer | where | may name | may not name | held by |
|---|---|---|---|---|
| core | `core/` (`codex-tui-rtwk-core`): ids, rpc, app and thread models, `apply`, `threads::project`, `wire`, `screens/<dir>.rs` x22, `fixtures/` | the protocol crate, serde, std | rtwk, tokio, futures, UI types, IO, clock, prose | compiler (`core/Cargo.toml`), `core/clippy.toml`, `layers.rs` |
| view | `src/screens/<dir>/` x22, `src/ui/`, `src/l10n/`, `src/theme.rs` | core, rtwk widgets, own strings | the protocol, `core::wire`, shell, tokio, IO, `apply` | `layers.rs`, `rules.rs` (R14, ADR-010) |
| shell | `src/shell.rs`, `src/shell/` | core, the protocol, tokio, rtwk `Cx`/`DialogResult` | widgets, views, theme | `layers.rs`, `rules.rs` (R1, R11, R12) |
| run, host | `src/run.rs`, `host/` | everything | - | `rules.rs` (R1 host) |

The UI model is `UiApp { core: CodexApp, ui: UiState { nav_key, thread_ui,
lang, debug } }`. A screen directory is two halves of one name,
`core/src/screens/<dir>.rs` and `src/screens/<dir>/`, made by one macro list
(`screen_list!`, in core, calling a macro of the UI crate too).

## Rules and their bites

Each rule below was shown biting: a violation injected, the test run, the file
restored from a copy (the working tree was clean after each).

| rule | test | injected violation | result |
|---|---|---|---|
| core has no rtwk or runtime dependency | `layers::core_manifest_has_no_ui_or_runtime_dependency` | `rtwk = { workspace = true }` in `core/Cargo.toml` | FAIL |
| core names no UI, runtime, IO or clock | `layers::core_sources_name_no_ui_runtime_or_io` | `use std::fs as _;` in `core/src/ids.rs` | FAIL |
| core holds no prose | `layers::core_sources_hold_no_user_visible_prose` | `format!("Cannot open the file {n}")` in core | FAIL |
| core lints (R1/R11/R12) | `core/clippy.toml` | `std::time::Instant::now()` in core | clippy error `disallowed_methods`, "use of a disallowed method `std::time::Instant::now`" |
| views name no protocol, shell, tokio, IO | `layers::views_never_name_protocol_shell_tokio_or_io` | `use codex_app_server_protocol::ServerNotification;` in `ui/seeded.rs` | FAIL |
| views never apply the model | `layers::views_never_apply_the_model` | the text `apply_app(` in `ui/seeded.rs` (first try a call: the crate did not compile, so no bite; redone as a string literal) | FAIL |
| view dirs hold view files only | `layers::screen_dirs_hold_view_files_only` | `pub struct Model {}` in `screens/chat/mod.rs` | FAIL |
| both halves of every directory | `layers::every_screen_dir_has_a_core_half_and_a_view_half` | a directory `src/screens/zzz/` | FAIL |
| the shell names no widget | `layers::shell_names_no_widget` | `use rtwk::widgets::Text;` in `src/shell.rs` | FAIL |
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
| `seed` | `{"step":"seed","seed":"resume_list","rows":[{id,preview,model_provider,timestamp}]}` | `resume::Ev::Listed` through `ingest` (the fields `create_fake_rollout` takes) |
| `notify` | `{"step":"notify","message":{"method":"turn/started","params":{..}}}` | the protocol's own JSON of a `ServerNotification` |
| `request` | `{"step":"request","message":{"method":"item/commandExecution/requestApproval","id":1,..}}` | the protocol's own JSON of a `ServerRequest` |
| `key`, `resize` | `{"step":"key","keys":"down"}`, `{"step":"resize","width":60,"height":20}` | handed back in order (`Played.inputs`) for a tester (`Harness::play_inputs`) |

Steps change the model only through `CodexApp::ingest`, the live app's path
(epoch check included), so a script cannot make a state the app cannot reach.
A `DebugEntry` carries `script: include_str!(..)`; the debug menu plays it into
a fresh model, wraps the screen in `Seeded::scope` (providers of the seeded
session, thread, request queue and screen states, shadowing the app's) and
shows the real widget. `EntryOpen::Route` pushes a route, `EntryOpen::Dialog`
shows a dialog (the builder returns the whole dialog).

| example | script | oracle | shows |
|---|---|---|---|
| chat transcript with a streamed answer | `src/screens/chat/fixtures/streamed_answer.jsonl` | `live_app_server_turn_completion_repairs_dropped_message_deltas` (`chatwidget/tests/app_server.rs:1842`) | `hello`, then `The transport kept this.` / `And dropped this.` |
| exec approval request | `src/screens/approvals/fixtures/exec_approval.jsonl` | `exec_approval_request_from_params` (`chatwidget/tests/helpers.rs`): a command approval for thread-1 | the skeleton's dialog, `1 needs an answer.`, open on the seeded queue |
| resume list from a fake rollout | `src/screens/resume/fixtures/fake_rollouts.jsonl` | the three `create_fake_rollout` calls of `app-server/tests/suite/v2/thread_list.rs:713-736` | three rows `2025-01-02T12:00:00Z  Hello  mock_provider` and two older |

## Oracle tests

| oracle test | ours | ported | n/a: reason | delta |
|---|---|---|---|---|
| `chatwidget/tests/app_server.rs::live_app_server_turn_completion_repairs_dropped_message_deltas` | `script::live_app_server_turn_completion_repairs_dropped_message_deltas` (core, the whole transcript after the script) and `fixtures::the_chat_entry_shows_the_streamed_answer` (the screen) | yes (the scenario; the text, not the snapshot) | | D1 |
| `app-server/tests/suite/v2/thread_list.rs` (the three-rollout list) | `script::a_resume_list_seed_fills_the_pickers_model`, `fixtures::the_resume_entry_lists_the_fake_rollouts` | data only | n/a: the picker's behaviour is S18's | D2 |
| `chatwidget/tests/helpers.rs::exec_approval_request_from_params` | `script::a_request_step_queues_a_request`, `fixtures::the_approvals_entry_shows_the_request` | data only | n/a: the approval dialog is S13's | |

Tests of this task, no oracle: `script::*` (9: blank and comment lines, the
error line, key and resize order, notify equals the protocol's JSON, the
session seed, the request step, the resume seed, the streamed answer, a
second session seed fails), `fixtures::*` (6, with `every_debug_entry_script_parses_and_applies`
and `a_seeded_entry_leaves_the_real_model_alone`), `layers::*` (9). The loader
and fixtures tests were red in the first commit (`todo!()` loader, no entries)
and green after.

## Snapshots
`debug/menu.snap` (ours, no oracle): three entry rows under their directories;
the test runs at 80x30 now: 22 directories and 3 entries no longer fit the 24
lines of an oracle screen, which a debug menu need not fit.

## Visual deltas
### D1 — streamed answer: the repair arrives as `item/completed`
kind: K01 (behaviour of the skeleton's projection, not a look)
cause: other: skeleton. The oracle repairs the dropped deltas from the summary
items of `turn/completed` (and consolidates the message); `threads::project`
maps `item/completed` and ignores a turn's items, so the script carries the
whole text as `item/completed`. The resulting transcript is the same
(`The transport kept this.\nAnd dropped this.`, once). The oracle's directive
line (`::code-comment`) and question block are the chat slice's.
oracle (`live_app_server_turn_completion_repairs_dropped_message_deltas`, the consolidated source):
```
The transport kept this.
And dropped this.
...
```
ours (`fixtures::the_chat_entry_shows_the_streamed_answer`):
```
› hello
• The transport kept this.
And dropped this.
```
proposal: accept until S10/S11 (cells and the transcript repair from
`turn/completed`).

### D2 — resume list: rows are our own, not `Thread` JSON
kind: K01
cause: other: skeleton. `thread/list` answers `Thread` values (30 fields, no
`Deserialize` for the whole struct); the seed takes the fields
`create_fake_rollout` takes (id, preview, provider, start time) and the model's
`Row` keeps those. The resume slice maps the real `thread/list` reply into
`Row`s in `resume::decode`.
oracle: `thread/list` data, three rollouts, previews `Hello`.
ours: three rows `2025-01-02T12:00:00Z  Hello  mock_provider`, the two older
ones below.
proposal: accept; the resume slice replaces the list and may widen `Row`.

## What did not split cleanly
- `shell::dialog_closed` stays in the shell: it builds no widget but names
  `rtwk::Cx` and `DialogResult`, because R1 (spawn only in the shell) is what
  lets it await a dialog's future. The view reaches it through the injected
  `CloseWatcher` (`run.rs` passes it). rtwk gap: `show_dialog` has no
  on-closed message, so every presenter needs a spawned watcher.
- A seeded screen's handlers (`Hs`) are the app's own: they cannot be made
  without an `AppCx`, so a submit from a seeded chat goes to the real model.
  Documented in `ui/seeded.rs`; the debug menu only looks.
- `FAKE_THREAD` and `shell/fake.rs` (the scripted backend) stay in the shell:
  they are an effect runner's test double, not fixtures of a screen.
- `Key` and `Resize` steps do nothing in the debug menu (no tester there);
  tests play them with `Harness::play_inputs`.

## Docs
`architecture.md`: ADR-001 (the layout of the workspace), ADR-002 (per-thread
widget state lives in `UiApp.ui`, not in `CodexApp`; `Lang` likewise), the
Skeleton file list and a "Layers" section. `design.md` (and `design.html`,
regenerated; all 11 Mermaid diagrams parse with `mmcheck`): §3 shows the two
crates with only the allowed arrows, §3b the layers table, §6 the class
diagram `UiApp { core, ui: UiState { thread_ui } }`, and the ThreadUi wording
at lines 10, 47, 168, 175-176, 203, 213, 248 and 391.
