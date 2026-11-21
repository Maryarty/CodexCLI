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
| trust_directory.rs:456 long_paths_keep_disclosure_and_choices_visible | snaps long_checkout_40x13, long_repository_root_40x17, only_repository_root_fits_40x16 | yes | | |
| welcome.rs:166 welcome_renders_animation_on_first_draw | shard, same name | yes | | |
| welcome.rs:184 welcome_skips_animation_below_height_breakpoint | shard, same name | yes | | |
| welcome.rs:199 welcome_logo_layout (160x48) | shard, pending | pending | the test kit reads entries 100 ms in, but the oracle reads the logo's first frame; a kit seed is needed (kit owner) | |
| update_prompt.rs:294 update_prompt_snapshot | snap update_prompt_modal | yes | | |
| update_prompt.rs:305 update_prompt_confirm_selects_update | core test, same name; shard closes the prompt | yes | screen part na: the choice is an outcome | |
| update_prompt.rs:313 update_prompt_dismiss_option_leaves_prompt_in_normal_state | core test, same name | yes | same | |
| update_prompt.rs:322 update_prompt_dont_remind_selects_dismissal | core test, same name | yes | same | |
| update_prompt.rs:332 update_prompt_ctrl_c_skips_update | core test, same name | yes | same | |
| update_prompt.rs:340 update_prompt_navigation_wraps_between_entries | shard, same name | yes | | |
| update_prompt.rs:349 long_update_command_keeps_selected_skip_visible_in_a_short_viewport | snap update_picker_selected_28x12 | yes | | |

Snapshots: 12 green, 1 pending (welcome_logo_layout).

Own tests:
- core: steps_run_in_the_oracle_order, launch_facts_are_asked_once_and_stale_answers_dropped, update_choices_ask_the_host, exit_ends_every_step and seeds_read_the_fixture_forms;
- protocol: 8 reader tests (sign-in due, the trust lookup over local roots, the update offer and versions);
- services: 2 tests (`services/startup.rs`, the version cache and the dismissal);
- screen: tests/pages/onboarding.rs the_startup_page_opens_on_the_launch_facts_and_writes_the_trust (launch, the stale answer dropped, Enter, the trust write, the page left).

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
  - RunUpdate: stores the install context's action, which `main` runs after the terminal is restored, from `/`.
  - The file and process IO lives in `src/services/startup.rs`: layers rule R12.

## Departures (no visual delta)

- Trust lookup: local host only. A remote workspace is not checked, as in the oracle's agents and remote paths.
- An error on account/read or config/read at launch counts as "nothing due". The oracle fails the launch.
- The network refresh of the version cache (`check_for_update`) is not ported. The offer reads the cache as it is.
- Provider facts are always false (task `onboarding`).
- The session connects in parallel with the page. A saved trust does not reload the session config.
- Flex: a loose `Flexible` lays out as in Flutter. This differs from the oracle's FlexRenderable only when part of the space is left over, which no snapshot covers.

## rtwk gaps

None.

## Checks (2026-11-21)

- `clippy --workspace --all-targets -D warnings`: clean.
- `nextest --workspace`: 825 of 830 passed. The 5 failures are timing flakes in `screens::status_data` and `shell_agents`. They pass on rerun and still fail with this slice's launch edits disabled (StartupGate unwrapped, the Connected ask removed: 6 runs each).
- Oracle shard: 20 green, 1 pending.
