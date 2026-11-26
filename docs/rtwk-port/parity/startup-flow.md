# startup-flow: parity

Slice: the StartupPage (S01-S04). It has a step controller in widget state, the
welcome header on rtwk-animation (the blossom stage), the update prompt (S01) and
the directory trust (S04). Sign-in (S02) and provider (S03) are slots that task
`onboarding` fills. The host reads the launch facts and runs the page's asks.
Oracle: codex-tui `onboarding/{onboarding_screen,trust_directory,welcome}.rs`,
`update_prompt.rs`, and for the host side `lib.rs` (`get_login_status`, the
trust check at launch), `config_update.rs`, `updates.rs`, `updates_cache.rs`,
`update_action.rs`, and the CLI's `run_update_action`.

## Oracle tests

Shard `onboarding`: `tests/oracle/onboarding.rs`, run with `--profile oracle -E
'binary(oracle) and test(/^onboarding::/)'`. Core state tests are in
`core/src/screens/onboarding.rs` (mod tests). Protocol readers are in
`core/src/protocol/onboarding.rs`.

| oracle (file:line name) | ours | ported | n/a: reason (owner) | delta |
|---|---|---|---|---|
| onboarding_screen.rs:1064 buffered_enter_is_discarded_when_trust_step_becomes_active | oracle shard, same name; core test, same name | yes | | |
| onboarding_screen.rs:1130 restricted_acceptance_and_cancellation_do_not_persist_trust | core test, same name (no write asked); the shard closes the page | yes | screen part na: persistence is not drawn | |
| onboarding_screen.rs persist failure (`persist_selected_trust`) | core trust_persistence_failure_keeps_trust_step_in_progress; snap renders_snapshot_for_trust_error | yes | | |
| onboarding_screen.rs suppress_quit_* (4) | | no | key routing of the auth steps (owner: `onboarding`) | |
| trust_directory.rs:296 release_event_does_not_change_selection | shard, same name | yes (press path) | release kinds: rtwk input delivers presses only | |
| trust_directory.rs:328 fragmented_terminal_response_cannot_grant_directory_trust | shard, same name | yes | | |
| trust_directory.rs:346 renders_snapshot_for_git_repo | snap, same name | yes | | |
| trust_directory.rs:360 directory_change_consent_can_keep_current_directory | snap, same name | yes | | |
| trust_directory.rs:374 renders_snapshot_for_remote_git_subdirectory | snap, same name | yes | | |
| trust_directory.rs:402 renders_restricted_folder | snaps renders_restricted_folder, existing_untrusted_task | yes | | |
| trust_directory.rs:424 renders_snapshot_for_trust_error | snap, same name | yes | | |
| trust_directory.rs:440 picker_wraps_paths_and_restricted_actions | snap folder_picker_restricted_40x24 | yes | | |
| trust_directory.rs:456 long_paths_keep_disclosure_and_choices_visible | shard, pending(G51): snaps long_checkout_40x13, long_repository_root_40x17, only_repository_root_fits_40x16 | pending | rtwk `RenderFlex` hands the remainder to the first flexible children, the oracle's `FlexRenderable` shortens the paths first (gap G51, `rtwk-gaps.md`) | |
| welcome.rs:166 welcome_renders_animation_on_first_draw | shard, same name | yes | | |
| welcome.rs:184 welcome_skips_animation_below_height_breakpoint | shard, same name | yes | | |
| welcome.rs:199 welcome_logo_layout (160x48) | shard, `Open::first_frame()` (the Enter that opens the entry runs one frame) | yes | | |
| update_prompt.rs:294 update_prompt_snapshot | snap update_prompt_modal | yes | | |
| update_prompt.rs:305 update_prompt_confirm_selects_update | core test, same name; the shard keeps the prompt up until the host stores the update | yes | screen part na: the choice is an outcome; the page leaves only after `UpdateQueued` (delta 5) | |
| update_prompt.rs:313 update_prompt_dismiss_option_leaves_prompt_in_normal_state | core test, same name | yes | same | |
| update_prompt.rs:322 update_prompt_dont_remind_selects_dismissal | core test, same name | yes | same | |
| update_prompt.rs:332 update_prompt_ctrl_c_skips_update | core test, same name | yes | same | |
| update_prompt.rs:340 update_prompt_navigation_wraps_between_entries | shard, same name | yes | | |
| update_prompt.rs:349 long_update_command_keeps_selected_skip_visible_in_a_short_viewport | snap update_picker_selected_28x12 | yes | | |

Snapshots: 10 green, 1 pending (long_paths_keep_disclosure_and_choices_visible: G51).

Launch order (startup-finish): once the steps are over the page asks the host for the launch's
prompts (`Ask::Launch`, after `Ask::ReloadConfig` when the trust was written or a sign-in ran;
the host reads them under the reloaded config); the gate keeps the home empty until they are
known, the page shows them (`Step::Prompts`), and a command-line start page (`--agents`,
`resume`) mounts only after the page popped (`StartupGate::holds_child`, `route_then`). A
provider change on the reload restarts the embedded server (host `in_process.rs`). Tests:
core `a_launch_with_no_step_due_asks_for_its_startup_prompts`,
`launched_prompts_keep_the_page_due_until_asked`, `a_launch_ask_lost_with_the_connection_is_asked_again`,
`the_launch_waits_for_the_steps_and_skips_a_launch_that_ends`,
`a_confirmed_trust_write_asks_for_the_config_reload`; screens `startup::gate::*`
(`a_launch_prompt_is_drawn_without_a_chat_frame_before_it`, `a_start_page_waits_for_the_startup_page`,
`ctrl_c_on_the_sign_in_quits_without_a_chat_frame`: a quit leaves from the page, no frame of the chat under it).
E2E (startup-finish, pty, empty CODEX_HOME, 100x30): sign-in menu by keys and double clicks down to the
API key entry and back; Ctrl-C quits; with `auth.json` the trust step, a double click on "Trust and
continue" writes `trust_level = "trusted"` and the chat follows; Down+Enter opens the agents overview.

Own tests:
- core: steps_run_in_the_oracle_order, launch_facts_are_asked_once_and_stale_answers_dropped, update_choices_ask_the_host, exit_ends_every_step and seeds_read_the_fixture_forms;
- protocol: 8 reader tests (sign-in due, the trust lookup over local roots, the update offer and versions);
- services: 2 tests (`services/startup.rs`, the version cache and the dismissal);
- screen: tests/pages/onboarding.rs the_startup_page_opens_on_the_launch_facts_and_writes_the_trust (launch, the stale answer dropped, Enter, the trust write, the page left).
- fix round and deltas (each red first, in its own commit):
  - F1/F2: the_trust_cursor_stays_at_the_ends (tests/pages/onboarding.rs);
  - N-O2/N-O1, deltas 1-4 (read error, version refresh, config reload, remote trust): about 46 host tests in `host/src/startup.rs` (fake seams for the browser, the network, the reload and the remote server) and the core tests of `core/src/screens/onboarding.rs`;
  - delta 5: update_now_quits_the_app_only_after_the_host_stored_the_update (tests/pages/onboarding.rs) and the core tests;
  - delta 6: 5 tests in `host/src/update.rs` (Windows PATH lookup, powershell.exe, relative entries ignored, the oracle's error words, `/` off Windows);
  - delta 7: 3 tests in `core/tests/onboarding.rs` (connection lost while saving, late answer dropped, a made choice untouched);
  - delta 8: 3 tests in `tests/pages/startup.rs` (`gate`: nothing drawn while the facts are awaited, the chat when nothing is due, a due step with no chat frame before it).

## Launch and host side

- Launch: `StartupGate` (`src/app/start_page.rs`) wraps the home. When the startup model is `due()`, it pushes `onboarding::route(Step::Update)` once. The page shows the first step that is due.
- Core asks the host: on the first backend's `Connected`, the root `Command::Startup { ask: Ask::Facts { request } }` asks for the facts once. The page's choices are asks too: `WriteTrust`, `DismissUpdate` and `RunUpdate`. Answers carry the request number, and stale ones are dropped in `apply`.
- Host (`host/src/startup.rs`, dispatched by `work()` in `host/src/in_process.rs`):
  - Facts:
    - sign-in from `account/read`;
    - the trust from `config/read` with layers, plus local project and git roots (`discover_project_root`, `resolve_root_git_project_for_trust`, `normalized_project_trust_keys`);
    - the update offer from `version.json`, never in a debug build.
  - Trust write: `ConfigBatchWrite projects."<key>".trust_level="trusted"`. The key is `project_trust_key` when embedded, or the folder as read for a daemon. The oracle's error context goes into the failure message.
  - Dismissal: writes `dismissed_version` to the cache.
  - RunUpdate: stores the install context's action, then answers `UpdateQueued` (the page leaves only then); `main` runs it after the terminal is restored (`host/src/update.rs`, the CLI's `run_update_action`: from `/` off Windows; on Windows the command is found on the absolute PATH entries, the standalone installer goes through `powershell.exe`, in a fresh directory).
  - The launch facts fail the launch when `account/read` or `config/read` cannot be read (`StartFailed`: the process ends with `Error: <message>` after the terminal is restored).
  - Remote workspaces: the trust of the `--cd` folder is read over the remote config (`read_remote_project_trust`).
  - Version cache: when missing or older than 20 h the facts carry `refresh_update`, the page asks `RefreshUpdateCache` and the shell runs it (the oracle's `check_for_update`; rule R1: the host spawns nothing).
  - A launch with a prompt (`codex "text"`) offers no update (`skip_update_prompt`, `Startup.launch_prompt`); the cache refresh and the first conversation's banner still run.
  - `/daemon` "Update and exit": `DaemonAsk::Update(source)` is kept as `UpdateAction::Daemon(source)` in the update slot, then answered `UpdateQueued`, on which the feature ends the app (`AppCx::end_app`); `main` runs `run_daemon_update` after the terminal is restored: the daemon crate is called in-process (`update` / `update_from_cli`, the sheet's confirmation is the `--yes`) where the oracle execs `codex app-server daemon update [--from-cli --yes]`; prints `Updating the local background server...`, the output line and `Relaunch Codex to reconnect.`, or `Daemon update failed: {error}`.
  - `/daemon` facts feed (`host/src/feeds/daemon.rs`): at start the CLI (`current_exe`, canonical; `has_package` from its install context; the package's version else the build's), at each connection the server's version and whether it is the local daemon; `managed` is left to the probe `/daemon` runs when it opens, `offline` is not wired (no feed for a lost connection: the Restart row stays enabled while offline).
  - The chat's server (`in_process::run`, after the config is read; oracle `startup_orchestration.rs`): `daemon_auto_start` (Stable, on) starts the shared background server when it is not running and attaches to it (required: a failure ends the launch with the hint), then the compatibility check and the recovery menu run on the first step. Excluded and so embedded: `--no-daemon`, `--oss`, `CODEX_EXEC_SERVER_URL`, a `-c` the daemon cannot take, a GCP or gateway sign-in wizard due without a login, a DrvFS `CODEX_HOME`; the startup warning `Running without the shared background server: <reason> requires embedded mode.` names the reason when the feature was on. With the feature off and nothing excluded, a daemon already listening on the default socket (50 ms probe) is attached, the embedded server standing in when it does not answer. Not carried: telemetry (`daemon_telemetry::record_start`), the Windows elevated warning and detached-launch fallback; the launch's `--profile`, `--strict-config`, `--dangerously-bypass-hook-trust` and workload-identity exclusions (flags this command line does not take).
  - After the trust is saved the page asks `ReloadConfig`: the host rebuilds the config with the folder trusted.
  - The browser opens (`OpenBrowser`) only for the login attempt still awaited, on `spawn_blocking`, through a seam; the host offers no provider wizards (`WIZARDS_BUILT`).
  - The file IO lives in `src/services/startup.rs`: layers rule R12.
- Launch order (R-F5): the real host answers the facts, so `UiApp::with_awaited_facts(true)` makes `StartupGate` draw nothing until they are read; then the chat, or with a step due the page alone: no chat frame before the prompt. The homes of `agents` and `resume` mount only after the page popped (`StartupGate::holds_child`).

## Departures (no visual delta)

- Provider facts are always false (task `onboarding`).
- A source build (`CARGO_PKG_VERSION` 0.0.0) never offers or refreshes an update: `is_source_build_version` (core `protocol::onboarding`, the one version rule with `is_newer` and `parse_version`: three numbers, a fourth component is not read) is that version, and the oracle's own source build has the same 0.0.0, so the update prompt does not show in either (the oracle's `updates.rs` is also compiled out of debug builds). Not a defect; the prompt is reached through the debug entry `update_prompt`.
- Flex: a loose `Flexible` lays out as in Flutter (G51 below).

## rtwk gaps

- G51 (`rtwk-gaps.md`): a column's flex children share the space unlike the oracle's `FlexRenderable`, so a long path pushes the disclosure and choices off a short screen; the three long-path snapshots are the pending test (they were green before F2; see Checks). G52: the path cut to one line by segments (`trust.rs` `path()` still uses the carried `center_truncate_path` and `measure::grapheme_rows`, G37's closed exception).

## Checks (2026-11-21, startup-fix-2 after fix round 1)

Run in the worktree of startup-fix-2 (branch from `6b1fb3955`):

- `fmt --check`: clean. `clippy --workspace --all-targets -D warnings`: clean (rc 0; the warnings printed are the pin's and the backend's, not the workspace's).
- `nextest --workspace`: 1018 run, 1011 passed, 7 failed, all timing flakes of the known kind, none touching this task: `screens::status_data` (3 to 4 of them, a different one each run, also with `--test-threads 1`), `shell_agents` (3), `navigation::import_is_a_step_of_the_startup_page` (task flaky-apply-wait). The core crate (452 tests, delta 5 included) and the rules (R1 with `spawn_blocking(`) pass; `rtwk_g49_g51` 11 of 11 (probes).
- `shards.py --check`: 55 shards, 0 files stale.
- Red then green: R1 red at `ce48b6c8a` (host/src/startup.rs:278), green at `d35c68b24`; delta 5 red at `ffbc70d91` (2 of the 3 tests), green at `9ef1bfa9b`.

- Delta 5 is extended: `disconnected` also resets `running` and reopens the update prompt (core test `a_connection_lost_while_the_update_is_stored_opens_the_prompt_again`, `a_late_ack_of_the_update_of_a_lost_connection_is_dropped`, `a_lost_connection_leaves_a_stored_update_alone`; red first).
- The browser open runs from `src/services/startup.rs` `open_in_browser`; the host calls it and spawns nothing; rule R1 now also greps `spawn_blocking(` in the host.
- G51: the three long-path snapshots (`long_checkout_40x13`, `long_repository_root_40x17`, `only_repository_root_fits_40x16`) were GREEN before startup-deltas F2, when the trust step computed the reserved rows from `measure::rows` (the oracle does the same arithmetic, `trust_directory.rs:190-200`); F2 removed it as a workaround and they are pending G51. The row now carries the three routes (Column alone, a height-keyed variant picker, a Grid) and probes; the cut-to-one-line path is G52.
