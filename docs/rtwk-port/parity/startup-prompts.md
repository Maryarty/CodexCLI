# startup-prompts parity

Slice S05: the startup prompts of codex-tui, one `Panel` over the shared `Options` list
(`src/widgets/options.rs`). Code: `core/src/screens/startup.rs` (pure: `Prompt`, `Model`,
`Ev::Asked`/`HooksTrusted`, `Intent::{Ask, Answer, TrustHooks}`, `apply`, the key table
`Prompt::on_key`), `src/pages/startup/prompts/` (`panel.rs` the widget, `strings.rs` words,
`mod.rs` the entry points, `fixtures/*.jsonl` 21 mock scripts through core `apply`).

## Prompts

| oracle | ours | state |
|---|---|---|
| cwd_prompt.rs (resume / fork, remembered current, remote exec) | `Prompt::Cwd` | ported |
| model_migration.rs (copy, link, markdown template, opt-out) | `Prompt::Migration` | ported (markdown through `MarkdownView`) |
| startup_hooks_review.rs (review / trust all / continue, trusting, error) | `Prompt::Hooks` | ported |
| unarchive_prompt.rs (resume / fork) | `Prompt::Unarchive` | ported |
| oss_selection.rs (LM Studio / Ollama and their status) | `Prompt::Oss` | ported |
| daemon_recovery.rs | `Prompt::Daemon` | ported |
| chatwidget/windows_sandbox_prompts.rs | not done | next slice |
| security_setup.rs (`Notice::valid`), startup_error.rs | not done / n-a (plain stderr text) | |
| startup_orchestration.rs (order and conditions) | not done: the launcher (startup-flow) decides | its logic tests (`bypass_hook_trust_suppresses_startup_review`, `untrusted_hooks_need_review_without_bypass`, `localhost_probe_succeeds_with_invalid_inherited_ca_bundle`, `security_setup_rejects_unsafe_actions_and_terminal_controls`) are not in this shard |

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

34 tests: 24 green, 8 pending, 2 n/a (the shard file's red = pending count
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
| cwd_prompt.rs::narrow_picker_keeps_selected_choice_and_escape_uses_session | `narrow_picker_keeps_selected_choice_and_escape_uses_session` | pending | scrolled window with `↑` needs Options::scroll() and the hanging indent (shared options.rs) |
| cwd_prompt.rs::cwd_prompt_omits_unusable_remembered_current_choice | `cwd_prompt_omits_unusable_remembered_current_choice` | green |  |
| cwd_prompt.rs::cwd_prompt_ctrl_c_exits_instead_of_selecting | `cwd_prompt_ctrl_c_exits_instead_of_selecting` | green |  |
| cwd_prompt.rs::cwd_prompt_remembered_choices_select_and_persist_matching_directory | `cwd_prompt_remembered_choices_select_and_persist_matching_directory` | green |  |
| cwd_prompt.rs::cwd_prompt_persistence_failure_snapshot | `cwd_prompt_persistence_failure_snapshot` | pending | the history line is inserted by the host after the prompt closes; its config write is not built here |
| model_migration.rs::prompt_snapshot | `prompt_snapshot` | pending | G48: hyphen wrap of the heading |
| model_migration.rs::prompt_snapshot_gpt5_family | `prompt_snapshot_gpt5_family` | green |  |
| model_migration.rs::escape_key_accepts_prompt | `escape_key_accepts_prompt` | green |  |
| model_migration.rs::selecting_use_existing_model_rejects_upgrade | `selecting_use_existing_model_rejects_upgrade` | green |  |
| model_migration.rs::markdown_prompt_keeps_long_url_tail_visible_when_narrow | `markdown_prompt_keeps_long_url_tail_visible_when_narrow` | green |  |
| model_migration.rs::migration_choice_and_hints_stay_visible_in_a_short_viewport | `migration_choice_and_hints_stay_visible_in_a_short_viewport` | green |  |
| startup_hooks_review.rs::fragmented_terminal_response_cannot_grant_startup_hook_trust | `fragmented_terminal_response_cannot_grant_startup_hook_trust` | green |  |
| startup_hooks_review.rs::non_trust_startup_hook_shortcuts_remain_immediate | `non_trust_startup_hook_shortcuts_remain_immediate` | green |  |
| startup_hooks_review.rs::renders_prompt | `renders_prompt` | pending | hanging indent of a wrapped label (shared options.rs) |
| startup_hooks_review.rs::renders_prompt_with_trust_error | `renders_prompt_with_trust_error` | green |  |
| startup_hooks_review.rs::picker_progress_blocks_selection | `picker_progress_blocks_selection` | pending | hanging indent of a wrapped label (shared options.rs) |
| unarchive_prompt_tests.rs::resume_prompt_snapshot | `resume_prompt_snapshot` | green |  |
| unarchive_prompt_tests.rs::fork_prompt_cancel_snapshot | `fork_prompt_cancel_snapshot` | green |  |
| unarchive_prompt_tests.rs::confirmation_requires_an_accept_key | `confirmation_requires_an_accept_key` | green |  |
| unarchive_prompt_tests.rs::inline_prompt_preserves_shell_context_snapshot | `inline_prompt_preserves_shell_context_snapshot` | pending | inline presentation: the shell row above the panel is terminal scrollback |
| unarchive_prompt_tests.rs::narrow_inline_prompt_wraps_snapshot | `narrow_inline_prompt_wraps_snapshot` | pending | inline presentation (as above) |
| unarchive_prompt_tests.rs::inline_prompt_only_claims_content_height | `inline_prompt_only_claims_content_height` | n/a | asserts the Tui viewport rect: no counterpart (rtwk owns the viewport) |
| unarchive_prompt_tests.rs::alternate_prompt_restores_original_viewport | `alternate_prompt_restores_original_viewport` | n/a | asserts the Tui viewport rect: no counterpart |
| oss_selection.rs::ctrl_h_l_move_provider_selection | `ctrl_h_l_move_provider_selection` | green |  |
| oss_selection_tests.rs::provider_picker_snapshots | `provider_picker_snapshots` | pending | case 4 (28x12): hanging indent plus the oracle gives the slack of a short viewport to its last flex child; cases 1-3 are green |
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

## Needed elsewhere (not rtwk gaps)

* `src/widgets/options.rs` (shared, not this slice's): (a) a hanging indent for a wrapped
  label (`OptionRow` field; Row of the mark and an `Expanded` label) - 3 pending tests;
  (b) `scroll()` (startup-flow's branch) so the arrow rows above and below the window can be
  drawn (`ListenableBuilder` on it, as `update_prompt.rs`): the arrow rows are blank here;
  (c) a rich label (the oracle's OSS rows carry an underlined hotkey letter and
  colour-coded status; `OptionRow.label` is a `String`): a delta today.
* `tests/oracle.rs`: the `startup_prompts` module line; `shards.py` / `test-shards.md` rule
  and a kit `Owner::StartupPrompts` (the shard runs as `Owner::DialogsPane` for now).
* Not covered: the hooks Ctrl+C mapping is unverified against the oracle; the Windows
  sandbox prompts.
