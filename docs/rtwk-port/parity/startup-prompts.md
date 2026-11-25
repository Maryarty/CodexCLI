# startup-prompts parity

Slice S05: the startup prompts of codex-tui, one `Panel` over the shared `Options` list
(`src/widgets/options.rs`). Code: `core/src/screens/startup.rs` (pure: `Prompt`, `Model`,
`Ev::Asked`/`HooksTrusted`, `Intent::{Ask, Answer, TrustHooks}`, `apply`, and what a step, a
number or a taken row does: `Prompt::{rows, step, number, take}`), `src/pages/startup/prompts/`
(`panel.rs` the widget, `keys.rs` the key bindings, `strings.rs` words, `mod.rs` the entry
points, `fixtures/*.jsonl` 21 mock scripts through core `apply`).

## Prompts

| oracle | ours | state |
|---|---|---|
| cwd_prompt.rs (resume / fork, remembered current, remote exec) | `Prompt::Cwd` | ported |
| model_migration.rs (copy, link, markdown template, opt-out) | `Prompt::Migration` | ported (markdown through `MarkdownView`) |
| startup_hooks_review.rs (review / trust all / continue, trusting, error) | `Prompt::Hooks` | ported |
| unarchive_prompt.rs (resume / fork) | `Prompt::Unarchive` | ported |
| oss_selection.rs (LM Studio / Ollama and their status) | `Prompt::Oss` | ported |
| daemon_recovery.rs | `Prompt::Daemon` | ported |
| chatwidget/windows_sandbox_prompts.rs | not built here (see "Scope of the rest") | dialogs-pane |
| security_setup.rs (`Notice::valid`, `prefetch`) | not built here | chat banner (dialogs-pane); the pure check needs `url` |
| startup_error.rs (`LocalStateDbStartupError`) | not built here | reuse-carry (a REUSE row: framework-free, no screen) |
| startup_orchestration.rs, lib.rs `run_ratatui_app` (order and conditions) | startup-finish: core `screens::startup::launch` (`review_is_needed`, `hooks_prompt`, `should_show_model_migration_prompt`, `migration_prompt`, `due`: hooks review, then the model migration); the host reads hooks/list and model/list from the config reloaded after the trust or the sign-in (`host/src/launch.rs`, `Ask::Launch`) and writes the answers (`Ask::TrustHooks`: `hooks.state` upsert; `Ask::SaveMigration`: `record_model_migration_seen` + `set_model`); the StartupPage shows them embedded (`Step::Prompts`) | ported: `bypass_hook_trust_suppresses_startup_review`, `untrusted_hooks_need_review_without_bypass` and the migration decision tests of `app/tests/model_catalog.rs` (`model_migration_prompt_only_shows_for_deprecated_models`, `retired_model_migration_respects_catalog_metadata`, `model_migration_prompt_respects_hide_flag_and_self_target`, `model_migration_prompt_skips_when_target_missing_or_hidden`, `model_migration_prompt_shows_for_hidden_model`) are unit tests of `launch.rs` on a synthetic catalog (the oracle's read the builtin presets). `--oss` / `--local-provider` (startup-oss): `Cli::{oss, oss_provider}`, `host/src/oss.rs` (`choose`: flag, config `oss_provider`, the one running server probed on the two default ports; `apply`: the `model_provider` kv override kept by every reload, the provider's default model, raw reasoning; `ensure_ready`: models pulled, service answering; a failure ends the launch through `fail`); `explicit_model_provider` (`--oss` is the override, a managed pin beats it); the provider panel is not asked yet: both or no server running ends the launch naming `--local-provider` until task f-oss-picker asks `Prompt::Oss` before the config is built. Not launch prompts of the rtwk binary: cwd (no resume/fork by id), unarchive, daemon recovery; `localhost_probe_*` and `security_setup_*` stay with dialogs-pane |

## Keys

The oracle's `handle_key` of each prompt is `src/pages/startup/prompts/keys.rs`: `bindings(&Prompt)`
returns `(KeyPress, Cmd)` pairs (the page's `Shortcuts`), the panel binds them on its `Focus`
and answers a `Cmd` with `Cmd::react`, which asks core (`Prompt::step` / `number` / `take`)
what it comes to. Core has no key type; `Intent::Answer` is validated there (`takes`: an
answer another prompt owns, or any answer while the trust write runs, is dropped).
The tests of the oracle's key rules are in `keys.rs` (the 16 ported names plus
`a_migration_without_opt_out_reads_only_accept_and_quit`), the cursor rules in core
(`step_wraps_round_the_ends...`, `a_number_takes_a_plain_row...`,
`the_cwd_prompt_has_no_fourth_row...`).

Ctrl+C in the hooks review does nothing in the oracle (`startup_hooks_review.rs` handles Esc,
Enter, Up/Down, digits only, and nothing at all while trusting); `bindings` has no Ctrl+C
for it either, and none at all while the trust write runs (`picker_progress_blocks_selection`).

## Scope of the rest (Windows sandbox, security_setup, startup_error)

Decided by what each is, not built in this slice:

* Windows sandbox prompts (enable / fallback / status): bottom-pane selection views of the
  chat, behind `cfg(target_os = "windows")` in the oracle; their oracle tests are
  `chat_popups_2.rs` (`startup_prompts_for_windows_sandbox_when_agent_requested`, pending
  `NO_WINDOWS`, owner dialogs-pane). They belong to the chat popups, not to the full-screen
  startup prompts.
* `security_setup.rs`: a chat banner fed by an RPC (`prefetch`); its screen tests are in
  `chat_popups_4.rs` (owner dialogs-pane). `Notice::valid` is pure but needs the `url` crate and
  core's dependency list is fixed (`tests/layers.rs`): a change of that list is the person's
  or the orchestrator's, with the banner's task.
* `startup_error.rs`: framework-free (a `thiserror` type over `codex_state`), no screen: a
  REUSE row for the reuse-carry slice.

Owners (batch review fix round 1, startup-fix-2): item (1) of the brief, the three of them,
is owned by tm task `startup-orchestration` (its description lists "windows sandbox /
security_setup / startup_error" among what it wires into the launch order, and the logic tests of
`startup_orchestration.rs`). The split above stays: the screens of the Windows sandbox prompts
and of the security banner are the chat popups' (dialogs-pane, `chat_popups_2` / `chat_popups_4`);
`startup_error.rs` is carried by reuse-carry as a REUSE row; `startup-orchestration` decides
when each is shown, and builds a screen only where none of those owners has one.

## How startup-flow calls the prompts

* `prompts::ask(cx, Prompt::..)` emits `Intent::Ask` (core stores the prompt in
  `Model.open`) and pushes `prompts::route()`; or the same two steps by hand.
* The answer lands in `ScreenStates.startup.answers` as an `Answer`: `Cwd(CwdSelection)`,
  `CwdExit`, `Migration(MigrationOutcome)`, `Hooks(HooksChoice)`, `Unarchive(UnarchiveChoice)`,
  `Oss(Provider)`, `OssCancelled`, `Daemon(DaemonChoice)`, `DaemonQuit`; the panel pops itself.
* Trust all: the panel emits `Intent::TrustHooks` (state `Trusting(request)`); the host
  does the write and sends `Ev::HooksTrusted { request, failure }`; a stale request is
  dropped, a failure shows `Failed(text)` under the list.
* The host also persists the cwd preference (`tui.resume_cwd`) and inserts the history line
  `Failed to save working directory preference.` on failure (not built here).
* Debug entries: `prompts::debug_entries()` (21), registered by the existing
  `entry!(startup, startup::prompts)` row.

## Oracle tests (`tests/oracle/startup_prompts.rs`, profile `oracle`)

34 tests: 26 green, 6 pending, 2 n/a (the shard file's red = pending count
in the oracle profile: pending panics as `PENDING[..]`). Expected text is the oracle's
snapshots through the replay helper.

| oracle (file::name) | ours (`tests/oracle/startup_prompts.rs`) | status | why |
|---|---|---|---|
| cwd_prompt.rs::cwd_prompt_snapshot | `cwd_prompt_snapshot` | green |  |
| cwd_prompt.rs::cwd_prompt_fork_snapshot | `cwd_prompt_fork_snapshot` | green |  |
| cwd_prompt.rs::cwd_prompt_remote_exec_snapshot | `cwd_prompt_remote_exec_snapshot` | green |  |
| cwd_prompt.rs::cwd_prompt_remembered_current_snapshot | `cwd_prompt_remembered_current_snapshot` | green |  |
| cwd_prompt.rs::cwd_prompt_selects_session_by_default | `cwd_prompt_selects_session_by_default` | green |  |
| cwd_prompt.rs::cwd_prompt_can_select_current | `cwd_prompt_can_select_current` | green |  |
| cwd_prompt.rs::narrow_picker_keeps_selected_choice_and_escape_uses_session | `narrow_picker_keeps_selected_choice_and_escape_uses_session` | pending | G49: the window (`↑`, rows 3 and 4) and the hanging indent are the oracle's; `TextWrap::Word` does not break after `/` (`(/` / `Users/example/current)`) |
| cwd_prompt.rs::cwd_prompt_omits_unusable_remembered_current_choice | `cwd_prompt_omits_unusable_remembered_current_choice` | green |  |
| cwd_prompt.rs::cwd_prompt_ctrl_c_exits_instead_of_selecting | `cwd_prompt_ctrl_c_exits_instead_of_selecting` | green |  |
| cwd_prompt.rs::cwd_prompt_remembered_choices_select_and_persist_matching_directory | `cwd_prompt_remembered_choices_select_and_persist_matching_directory` | green |  |
| cwd_prompt.rs::cwd_prompt_persistence_failure_snapshot | `cwd_prompt_persistence_failure_snapshot` | pending | the cwd prompt has no caller in the rtwk binary yet (no resume/fork by id), so its write and failure line have none; earlier reason: the history line is inserted by the host after the prompt closes; its config write is not built here |
| model_migration.rs::prompt_snapshot | `prompt_snapshot` | pending | G48: hyphen wrap of the heading |
| model_migration.rs::prompt_snapshot_gpt5_family | `prompt_snapshot_gpt5_family` | green |  |
| model_migration.rs::escape_key_accepts_prompt | `escape_key_accepts_prompt` | green |  |
| model_migration.rs::selecting_use_existing_model_rejects_upgrade | `selecting_use_existing_model_rejects_upgrade` | green |  |
| model_migration.rs::markdown_prompt_keeps_long_url_tail_visible_when_narrow | `markdown_prompt_keeps_long_url_tail_visible_when_narrow` | green |  |
| model_migration.rs::migration_choice_and_hints_stay_visible_in_a_short_viewport | `migration_choice_and_hints_stay_visible_in_a_short_viewport` | green |  |
| startup_hooks_review.rs::fragmented_terminal_response_cannot_grant_startup_hook_trust | `fragmented_terminal_response_cannot_grant_startup_hook_trust` | green |  |
| startup_hooks_review.rs::non_trust_startup_hook_shortcuts_remain_immediate | `non_trust_startup_hook_shortcuts_remain_immediate` | green |  |
| startup_hooks_review.rs::renders_prompt | `renders_prompt` | green |  |
| startup_hooks_review.rs::renders_prompt_with_trust_error | `renders_prompt_with_trust_error` | green |  |
| startup_hooks_review.rs::picker_progress_blocks_selection | `picker_progress_blocks_selection` | green |  |
| unarchive_prompt_tests.rs::resume_prompt_snapshot | `resume_prompt_snapshot` | green |  |
| unarchive_prompt_tests.rs::fork_prompt_cancel_snapshot | `fork_prompt_cancel_snapshot` | green |  |
| unarchive_prompt_tests.rs::confirmation_requires_an_accept_key | `confirmation_requires_an_accept_key` | green |  |
| unarchive_prompt_tests.rs::inline_prompt_preserves_shell_context_snapshot | `inline_prompt_preserves_shell_context_snapshot` | pending | inline presentation: the shell row above the panel is terminal scrollback |
| unarchive_prompt_tests.rs::narrow_inline_prompt_wraps_snapshot | `narrow_inline_prompt_wraps_snapshot` | pending | inline presentation (as above) |
| unarchive_prompt_tests.rs::inline_prompt_only_claims_content_height | `inline_prompt_only_claims_content_height` | n/a | asserts the Tui viewport rect: no counterpart (rtwk owns the viewport) |
| unarchive_prompt_tests.rs::alternate_prompt_restores_original_viewport | `alternate_prompt_restores_original_viewport` | n/a | asserts the Tui viewport rect: no counterpart |
| oss_selection.rs::ctrl_h_l_move_provider_selection | `ctrl_h_l_move_provider_selection` | green |  |
| oss_selection_tests.rs::provider_picker_snapshots | `provider_picker_snapshots` | pending | case 4 (28x12): G50, the oracle gives the slack of a short viewport to its last flex child; cases 1-3 are green, with the oracle's label styles (underlined hotkey, green / red / yellow status) asserted |
| oss_selection_tests.rs::navigation_wraps_without_confirming | `navigation_wraps_without_confirming` | green |  |
| oss_selection_tests.rs::shortcuts_select_the_provider_even_when_unavailable | `shortcuts_select_the_provider_even_when_unavailable` | green |  |
| daemon_recovery_tests.rs::daemon_recovery_requires_explicit_restart_and_defaults_to_cancel | `daemon_recovery_requires_explicit_restart_and_defaults_to_cancel` | green |  |

## Deviations from the brief

* The brief wanted the choice lists on the select sheet (`Menu::List`, `Picked::Item`).
  Not used: the sheet has no open-by-intent entry for a startup prompt and no styled
  header, and the oracle's picker rows (numbers over enabled rows only, explicit-confirm
  rows reached by digits, disabled rows as `   label` with the reason, the 2-line arrow rows)
  differ from the sheet's. One `Panel` over `Options` serves all six prompts.

## Wraps and shelves

* The migration heading (`gpt-5.1-codex-max.`) breaks only at spaces in the oracle;
  rtwk's `TextWrap::Word` also breaks after a hyphen: gap G48 (`rtwk-gaps.md`), the heading is
  a plain `Text` and `prompt_snapshot` is pending(G48). (An earlier one-`Text`-per-word
  `Wrap` workaround was removed: process.md §5.)
* Short viewports: the migration body gives way as one clipped block (`Column`
  with `overflow_intended`, leading blank inside it); OSS notes and blanks give way one by
  one. The oracle's `FlexRenderable` first satisfies every flex child that needs less than
  its share and gives the slack to the last one; rtwk's `Flexible` loose gives each child
  its share. They agree unless the free space is under about three rows.

## Shared `Options` (src/widgets/options.rs)

* `RichRow { prefix, label: Vec<Span>, description }` and `Options::view_rich`: the mark and
  the prefix (`1. `, or blanks) in a fixed grid track, the label (spans: the OSS rows' underlined
  hotkey and green / red / yellow status) in the next, so a wrapped label hangs under its first
  cell as the oracle's `wrap_indent` does. `OptionRow` and `Options::view` are unchanged.
* One `Options::scroll()` and `Options::arrow(above, mark, style)`: the `↑` / `↓` rows follow
  the scroll (the panel's top and bottom rows of the list, blank while nothing is hidden).
* The window goes by whole rows as the oracle's (`adjust_start_for_wrapped_selection_visibility`):
  the first row shown goes on one by one until the cursor row fits whole; the content has room
  under it for the last row at the top (the oracle leaves blank lines there).

## Needed elsewhere (not rtwk gaps)

* Nothing outstanding: the `startup_prompts` module line is in `tests/oracle.rs`, the kit has
  `Owner::StartupPrompts` (slice `startup-prompts`), and `shards.py` maps the seven oracle files to
  the shard (`shards.py --check`: 0 stale).
* The launch wiring of the hooks review and the model migration is done (startup-finish); the cwd, unarchive, OSS and daemon prompts wait for the CLI paths that ask them (resume/fork by id, the OSS panel before the config (f-oss-picker), the daemon start). `--oss` itself works (startup-oss): the device-code sign-in of `resume <id>` is for the resumed thread's provider (`Ask::GatewayDevice { thread }`, the read first).

## Checks that were red on arrival

* `chat::transitions::slash_import_opens_the_import` and
  `navigation::import_is_a_step_of_the_startup_page` (tests/pages) fail on the base commit
  `f51e15937` too (a blank frame, `RenderList ... given 0 rows`); not this slice's.
