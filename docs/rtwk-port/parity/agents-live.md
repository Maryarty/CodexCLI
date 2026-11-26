# agents-live — parity

Slice: S21, the Agent command center against the real backend. Oracle:
codex-tui rust-v0.170.0, `codex-rs/tui/src/app/agents_overview*.rs`,
`agent_center_tests.rs`. Two sides, one pack: the core side (task
`agents-live`, below) and the UI side (task `agents-live-ui`, the section
"UI side", added by that porter after the core's commits).

## Core side (task `agents-live`)

Core tests: `codex-rs/tui-rtwk/core/tests/agents.rs` (32 pass). The oracle's
tests drive widgets and `App`; ours drive `update` / `apply_answer` /
`observe` and read `Model::center()`, with the oracle's expected values.

### Oracle tests

| oracle (file::name) | ours (core/tests/agents.rs::name) | ported | n/a: reason (owner) | delta |
|---|---|---|---|---|
| agent_center_tests::live_center_keeps_status_order_across_progress_refreshes | same | yes | | |
| agent_center_tests::failed_background_worker_only_needs_attention_for_a_pending_request | same | yes | | |
| agent_center_tests::live_center_sorts_arrivals_and_status_transitions_by_update_time | same | yes (incl. the parent that waits on a descendant, was red) | | |
| agent_center_tests::live_center_status_order_tracks_updates_while_filtered_and_pinned | same | yes | | |
| agent_center_tests::live_center_rename_retains_target_when_status_leaves_filter | same | yes | | |
| agent_center_tests::backspace_edits_search_and_rename_without_deleting_tasks | same | yes (core part: `SetSearch`/`Rename`; the text edit is the widget's) | | |
| agent_center_tests::live_center_show_more_navigation_search_and_loading | same | yes (core part: show more, search, loading; keys are UI) | | |
| agent_center_tests::live_center_columns, _pages_visible_rows_without_wrapping, _metadata_clips_at_grapheme_boundaries, _navigation_and_complete_hints, _fixed_shortcuts_yield_to_configured_actions, _hints_follow_configured_bindings, _search_row_appears_only_while_editing | | no | rendering and keys: UI side (`agents-live-ui`) | |
| agent_center_tests::escape_returns_to_a_chat_that_is_still_starting, overview_clears_voice_badge_after_async_close | | no | App-level (chat session start, voice): chat/session slices, not the center's core (owner: chat-nav follow-up) | |
| agents_overview_tests::shared_overview_shows_only_root_sessions | same | yes | | |
| agents_overview_tests::overview_model_grouping_shows_details_and_preserves_selection | same | yes | | |
| agents_overview_tests::agents_overview_details_show_available_attention_without_expanding_rows | same | yes (core part: details data) | render: UI | |
| agents_overview_tests::server_version_overview_notice_updates_and_clears | same | yes | | |
| agents_overview_tests::overview_right_opens_current_or_highlighted_task | same | yes (the view sends `Accept` for Right unless a line is open) | | |
| agents_overview_tests::overview_right_preserves_editors_and_offline_state | same | yes (was red) | | |
| agents_overview_tests::overview_grouping_persists_across_config_reloads | same | yes | core: `SaveGrouping` intent and `Ev::Grouping` at start (`core/tests/agents_details.rs`, `tests/shell_agents.rs::a_grouping_change_is_saved_to_the_config_but_the_directory_one`, `the_configured_grouping_is_the_one_the_center_opens_with`, `the_shell_saves_the_grouping_and_reports_a_failure`); the host writes `tui.agents_overview_grouping` through `ConfigEditsBuilder` | |
| agents_overview_tests::command_center_refresh_failure_is_inline_and_clears_on_success | same | yes | | |
| agents_overview_tests::command_center_action_failures_remain_visible | same | yes | | |
| agents_overview_tests::command_center_new_actions_use_selection_and_leave_metadata_text_alone | same | yes | | |
| agents_overview_tests::worktrees_overview_groups_by_project_or_directory | same | yes (was red: project and directory group keys; keys arrive as `Ev::Directories`, core stays pure) | the shell does not read them yet, see Gaps | |
| agents_overview_tests::overview_directory_group_keys_normalize_aliases | same | yes (was red; same remark) | the host function that canonicalises and finds the repo root is not written | |
| agents_overview_tests::overview_fork_preserves_an_open_side_conversation, overview_fork_keeps_idle_source_input_queued | `fork_opens_the_copy_of_the_selected_task` (new; core part: `Intent::Fork`, `Command::Fork`, the copy becomes current) | partly | the side conversation and queued input are chat state (owner: chat-nav follow-up) | |
| agents_overview_tests::overview_worktree_creation_busy_state | `command_center_new_worktree_uses_the_selected_directory` (core part: `w`, `Notice::CreatingWorktree`, `Command::Worktree`) | partly | rendering of the busy notice: UI | |
| agents_overview_tests::shared_overview_keeps_rows_and_replays_changes_over_stale_reads, shared_overview_seeds_once_and_retains_locally_resumed_history, hidden_system_thread_does_not_refresh_shared_overview, hidden_title_thread_does_not_refresh_shared_overview, refreshing_one_backends_pins_preserves_cross_backend_rank, overview_keeps_rows_and_routes_actions_to_the_owner | | no | need the oracle's shared overview over two sources (interactive + exec/app server); the core has one source plus a buffer, see Gaps (owner: follow-up core task, orchestrator to plan) | |
| agents_overview_tests::agents_overview_reasoning_uses_existing_events_and_expires_with_attachment | same | yes (agents-parity-fix) | reasoning deltas accumulate keyed by (turn, item, summary index) in a 512-char buffer, the first `**..**` run is the Latest activity; TurnStarted/TurnCompleted/ItemStarted/PartAdded/non-Active status end it; `core/tests/agents_details.rs::latest_activity_is_the_first_bold_run_across_deltas`. The expiry on detach is not modelled (the center has no attachment) | |
| agents_overview_tests::root_switch_*, failed_root_switch_*, overview_selection_applies_user_permissions_only_to_unloaded_threads, overview_cold_resume_honors_working_directory_selection, restored_server_permission_profile_survives_cd_without_turn_override, cancelling_resume_picker_*, resuming_active_session_closes_command_center, resume_failure_keeps_command_center_available, resume_picker_round_trip_*, command_center_handles_resume_failure_and_success, command_center_attach_conflict_opens_read_only_and_retries, empty_command_center_can_open_resume_picker | | no | App-level session switching and the resume picker (`App`, config, permissions): chat/session and resume-list slices (owner: `routes-lists`, chat-nav follow-up) | |
| agents_overview_tests::agents_overview_details_render_markdown, _preview_links_survive_wrapping_and_clipping, _markdown_preview_preserves_layout_and_bounds, filtered_dashboard_actions_use_configured_shortcuts, command_center_cursor_tracks_fixed_footer, older_server_notice_*, overview_thread_colors_* | | no | rendering and keys: UI side | |
| agents_overview_actions_tests::pin_action_updates_shared_section_state | same | yes | | |
| agents_overview_actions_tests::cloud_archive_preserves_attached_child_and_other_tasks | same | yes (was red: a cloud archive removes only the selected task) | | |
| agents_overview_actions_tests::lifecycle_shortcuts_target_filtered_task_in_any_state | same | yes | | |
| agents_overview_actions_tests::external_removals_preserve_adjacent_selection | same | yes | | |
| agents_overview_actions_tests::hiding_rename_target_does_not_transfer_draft_to_neighbor | same | yes | | |
| agents_overview_actions_tests::hidden_task_stays_hidden_through_activity_and_seed_until_explicit_resume | same | yes | | |
| agents_overview_actions_tests::rejected_delete_preserves_a_live_attachment_and_draft | `rejected_delete_preserves_a_live_attachment` | yes (core part; the draft is chat state) | | |
| agents_overview_actions_tests::lifecycle_removes_background_and_current_tasks_without_losing_the_dashboard | `lifecycle_removes_background_and_current_tasks` | yes | | |
| agents_overview_actions_tests::archiving_selects_the_next_displayed_task, hiding_tasks_keeps_selection_adjacent_in_display_order | | no | the next selection is picked from the displayed order by the widget's cursor; the core keeps `selected` by id only. Needs the UI row order with `SelectionController` (owner: `agents-live-ui`; blocked in part on gap G35) | |
| agents_overview_actions_tests::fork_shortcut_respects_metadata_editing | | no | key gate while editing: core part is `blocked()`/`Right` tests; the `f` key is the UI's (owner: `agents-live-ui`) | |
| agents_overview_actions_tests::archive_confirmation_number_keys_act_immediately | | no | UI key | |
| agents_overview_discovery_tests::overview_cloud_discovery_uses_only_interactive_pages | same | yes (was red: backend mode) | | |
| agents_overview_discovery_tests::overview_show_more_refills_archived_rows_and_retries_without_losing_rows | `overview_show_more_refills_archived_rows` | yes (was red: refill count and the cursor retry) | | |
| agents_overview_usage_tests::selected_usage_is_cached_and_account_changes_discard_old_results | `selected_usage_is_cached` | yes (was red: 60 s cache, settled zero estimates, `usage_disabled`, `reset_for_backend_switch`) | | |
| agents_overview_usage_tests::incomplete_billing_groups_do_not_display_partial_token_totals | | no | the grouping of totals is shown by the view from `Usage` and `Details`; the formatting helpers live in codex-tui `status` (owner: `agents-live-ui`) | |
| agents_overview_action_progress_tests::modal_consumes_input_and_finishes_after_input_closes | | no | core part is the key gate (`blocked()`, covered by the Right/offline tests); the modal is the UI's | |

New core tests with no oracle name: `fork_opens_the_copy_of_the_selected_task`
(the oracle's `ForkAgentsOverviewThread` event, its test is app-level).

Bites: the 10 tests were red at the start of this task (the predecessor's
checkpoint named them) and green after `646f838d2`; one commit holds tests and
code together (the red state was the predecessor's `0d4a8ca43`, `e5f072add`,
`fbb0139a5`, `25ea0592b`), so there is no separate red commit of this task.

### Gaps and n/a items (core side)

| item | state | owner |
|---|---|---|
| Discovery is one source (interactive) plus a buffer and 10-row pages; the oracle merges a second source (exec / app server threads) by recency | recorded fidelity gap: exec and app-server threads are not listed | follow-up core task (orchestrator to plan) |
| Directory and project group keys: the shell reads canonical paths and the repo root and sends `Ev::Directories`; the host function (`dunce::canonicalize`, `codex_git_utils::repository_identity`) is not written, and `Ev::Backend{worktrees}` is never sent, so the worktree grouping and `w` are inert in the binary (the keys are only used when worktrees are on) | core done and tested; shell n/a | follow-up shell task (host `dirs.rs`) |
| `Command::Worktree{cwd}`: the shell creates the worktree and starts the thread; no host code answers it (with `worktrees` false the intent never fires) | n/a until the shell side above exists | same follow-up |
| Usage fetch timeout (65 s) and the 180 ms debounce of the first search page are timing the core cannot hold | n/a: shell or UI timer on `Out` (the core reports `usage_wait` and `Search{first}`) | `agents-live-ui` (debounce), shell follow-up (timeout) |
| Pinned-thread listing (`section_position`, `thread/loaded/list`) | not modelled | follow-up core task |
| Account and backend switch hooks | `Ev::Account`, `reset_for_backend_switch` modelled; the shell does not send them | shell follow-up |
| `Model` holds no UI-widget state (process.md §5): scroll, cursor row, wrap, j/k, ctrl-n/ctrl-p are the widget's | by design; the four oracle rules the widget hides from us are the gap G35 | `docs/rtwk-port/rtwk-gaps.md` G35 |
| Review of `src/lib.rs` and `src/ui/view.rs` (predecessor): `Cli.agents` and `start_agents` in `view` are what the entry (`codex-rtwk agents`) needs; nothing else was added, nothing dropped | kept | |
| Clippy of `src/shell.rs`: `ambient` is an `async fn` now (`manual_async_fn`) | fixed in `agents-live` | |
| Clippy of `src/screens/agents/view.rs:184`: `grouping_name` needless lifetimes | not mine; the UI porter's file, named in the handover | `agents-live-ui` |

### Handover to the UI side

- Intents (agents-core-rest, VS1 review L1/L2): the core model holds no UI-only
  state. The help, the confirmation, the status tab, the search line being
  open and the rename draft are the view's `ScreenState`; core's `Lens`
  (tab, search open, rename target) is handed to `center_at` /
  `selection_after_removal`, and to `SetSearch` / `MoreMatches`, which need the
  visible count. `Archive`/`Delete` run the removal the view confirmed;
  `Rename { thread, name }`, `ClearSearch`, `MoreMatches(Lens)`; `Right`,
  `Confirmed`, `BeginSearch`, `BeginRename`, `CancelEditing`, `ToggleHelp`,
  `CycleFilter`, `SetFilter`, `Commit` are gone. `Model::held()` is the gate
  the view asks before it opens a confirmation. The rule
  `core_screen_models_hold_no_ui_state_fields` (tests/layers.rs) holds it.
  `RowData` projects `name`, `preview`, `cwd`, `updated_at`,
  `reasoning_effort`, `branch`; `Usage` exposes `credits_micros`,
  `usd_micros`, `native_usd_micros`, `has_estimate`; views never read `.thread`
  or `.estimate`. Still in the list: `Fork` (oracle key `fork`), `NewWorktree`
  (`w`), `NeedUsage` (view asks for the selected task's usage; the core answers
  with `Out`), `SetSearch { text, lens }`.
- `Notice::CreatingWorktree` (busy line), `Notice::LoadFailed`.
- `Details.usage: Usage` (`tokens`, `estimate`, `fetched_at`,
  `token_counts()`); the cost formatters `thread_cost_micros`,
  `format_credit_micros`, `format_estimated_usd_micros` live in codex-tui
  `status` and must be rebuilt in the view.
- `usage_wait(&Model)` is the hint while a refresh is pending; `History`
  (`loading`, `failed`, `has_more()`) gives "Searching history..." and "Show
  more matches"; `search_matches` rows are in `center().rows` while the
  search text is set.
- Debug rows needed (named by the UI porter's red stubs): worktree busy,
  older-server notice, markdown details, delete confirmation, usage with
  incomplete billing.

## UI side (task `agents-live-ui`)

Files: `codex-rs/tui-rtwk/src/screens/agents/{mod,view,layout,strings,usage}.rs`,
fixtures `src/screens/agents/fixtures/*.jsonl` (debug rows `overview`,
`attention`, `navigation`, `search`, `clip`, `paging5`, `paging30`, `confirm`,
`progress`, `columns`, `empty_narrow`), tests `codex-rs/tui-rtwk/tests/agents.rs`
(screen tests at the oracle's size against its `.snap` text through
`oracle_snap`) and the unit tests of `usage.rs` and `path.rs`. Mock state is
a fixture script through core `apply`; no widget gets data by hand.

Checks at the last commit: `clippy --workspace --all-targets -D warnings`
clean; `nextest --workspace`: 535 run, 521 pass, 14 fail, and the 14 are the
`pending(owner, reason)` markers below (a marker panics `PENDING[owner]:
reason`; `KIT_PROBE_PENDING=1` runs the body behind it, which is how each
reason was checked to be the only difference).

### Oracle tests (UI side)

| oracle (file::name) | ours (tests/agents.rs::name) | state | reason / owner |
|---|---|---|---|
| agents_overview_tests::shared_overview_shows_only_root_sessions | same | green | snapshot `agents_overview` |
| agents_overview_tests::agents_overview_details_show_available_attention_without_expanding_rows | same | green | snapshot `agents_overview_attention` |
| agents_overview_tests::empty_command_center_can_open_resume_picker | same | green (screen part: snapshot `agents_overview_empty_narrow`; `o` opens the resume picker, `tests/agents.rs::o_opens_the_resume_picker`, held back while busy: `o_is_held_back_while_busy`) | |
| cli daemon_startup::agents_overview_before_folder_consent | `agents_overview_before_folder_consent_empty_center_rows` | green | first 4 rows; the consent dialog is the daemon startup flow |
| agent_center_tests::live_center_metadata_clips_at_grapheme_boundaries | same | green | |
| agent_center_tests::live_center_navigation_and_complete_hints | same | green | snapshots status_filter, help 1/2/3 columns, hints |
| agent_center_tests::live_center_search_row_appears_only_while_editing | same | green | snapshot `live_center_search_active` |
| agent_center_tests::live_center_pages_visible_rows_without_wrapping | same | green for the online flow (5 tasks, 30 tasks) with PageDown/PageUp | the oracle's loop key `ctrl+f` and Down wrapping to the first row: see next row |
| (same test, the rest) | `live_center_page_keys_and_wrap_follow_the_list_keymap`, `live_center_pages_visible_rows_while_reconnecting` | pending (rtwk, agents-live-shell) | G35: `SelectionList` has no page/move call, no wrap and no key table; page keys do not reach the list while the search line has the focus; the reconnecting notice cannot be seeded (`seed_ev` has no `notice`) |
| agent_center_tests::live_center_columns | same | pending (agents-live-shell) | the `voice` badge needs a flag on core `RowData` and a seed for it; the rest of the 160-cell snapshot and the 50-cell `Error:` text are the oracle's (`KIT_PROBE_PENDING=1` shows `voice` as the only differing cell). The third part, `live_center_selected_status_styles`, dumps ratatui `Style` values: no rtwk counterpart, not ported |
| agent_center_tests::live_center_fixed_shortcuts_yield_to_configured_actions, live_center_hints_follow_configured_bindings | same | pending (keymap) | the configured keymap (`agents.*`, `list.*`) does not exist in tui-rtwk; the screen's keys are the oracle's defaults |
| agents_overview_tests::filtered_dashboard_actions_use_configured_shortcuts | same | pending (keymap) | same |
| agents_overview_tests::command_center_cursor_tracks_fixed_footer | same | green | terminal cursor on the search and rename rows at 48, 96, 120 cells |
| agents_overview_tests::agents_overview_markdown_preview_preserves_layout_and_bounds | same | green | `preview_markdown` of core |
| agents_overview_tests::agents_overview_details_render_markdown (snapshots markdown, markdown_long_lines, markdown_table) | same | pending (reuse-carry) | the oracle's `markdown_render` is not carried; the details print the text plain; no seed for the last message |
| agents_overview_tests::agents_overview_preview_links_survive_wrapping_and_clipping | same | pending (reuse-carry) | same, plus OSC 8 links |
| agents_overview_tests::overview_worktree_creation_busy_state | same | pending (agents-live-shell) | `seed_ev` has no `notice` key (busy and server-version notices); the footer renders both (`footer`) |
| agents_overview_tests::older_server_notice_is_visible_in_agents_overview, older_server_notice_truncates_in_narrow_overview | same | pending (agents-live-shell) | same |
| agents_overview_tests::overview_thread_colors_match_footer_and_respect_color_suppression | same | green (`vs1-followups`) | see the follow-ups section: colour from the theme, checked on cells (fg, bold) rather than the ratatui `Style` dump |
| agents_overview_actions_tests::archive_confirmation_number_keys_act_immediately | same | green | snapshot `archive_task_confirmation`; `1` cancels, `2` starts the archive (the removing notice appears) |
| agents_overview_actions_tests (delete) `lifecycle_removes_background_and_current_tasks_without_losing_the_dashboard`, confirmation half | `delete_task_confirmation_and_failure_snapshots` | green | snapshot `delete_task_confirmation`; `2` only moves the cursor, Enter confirms |
| (same, failure half) archive_task_failure, delete_task_failure | `lifecycle_failure_dialogs` | pending (agents-live-shell) | core keeps a failed removal as footer text (`Notice::Error`, refresh.rs:211, `let _ = action`); it has no failure (action, error, attempted) to show as the dialog |
| agents_overview_action_progress_tests::modal_consumes_input_and_finishes_after_input_closes | same | pending (agents-live-shell) | `Intent::Confirmed(true)` does not hide the removed task while the request runs (the oracle's `prepare_agents_overview_removal` + `hidden_threads`): 3 rows and `All 3` where `archive_progress` has 2; every other row and the footer match. Esc leaving and the modal's input queue are shell mechanics (below) |
| agents_overview_usage_tests::incomplete_billing_groups_do_not_display_partial_token_totals | `usage::tests::incomplete_billing_groups_do_not_display_partial_token_totals` (unit) | green | with `credits_and_dollars_read_as_the_oracle_writes_them` |
| agents_overview_actions_tests::archiving_selects_the_next_displayed_task, hiding_tasks_keeps_selection_adjacent_in_display_order, lifecycle_shortcuts_target_filtered_task_in_any_state (keys), fork_shortcut_respects_metadata_editing | | not ported | matrices over 3-4 groupings, filtered or not, with configured F5/F7/F8 keys: they need the configured keymap (owner keymap); core already tests the selection after a removal (`external_removals_preserve_adjacent_selection`) |

### What the view does and where rtwk owns it

- List: `SelectionList` + `SelectionController` (cursor, arrows, page, home/end,
  Enter, scroll); headings and gaps are disabled rows. The keys `j`/`k`/ctrl-n/
  ctrl-p/ctrl-j/ctrl-k step through core's `Select` (one row).
- Search and rename lines: `TextField` + `TextEditingController`; the list gets
  `ExcludeFocus` while one is open; focus returns after the frame through
  `AppBinding::add_post_frame_callback` (the node that a mode drops releases
  focus with the frame).
- Confirmation (archive, delete): composed of primitives (a `Stack` with a
  `Positioned` panel of two `SelectionList` rows) to the oracle's geometry:
  72 cells at most, centered, header wrapped at the inner width, a blank row
  above, `› 1.` rows, the hint; keys 1, 2, Enter, Esc, arrows; a delete waits for
  Enter (`require_explicit_confirmation`). Cloud archive uses its own text.
- Usage lines: `usage.rs` formats credits and the estimated dollars as the
  oracle's `status/thread_usage.rs`; the view asks core for the usage of the
  highlighted task when its key (task, fetched-at, pending, clock) changes.
- Tab strip: hand-made from spans (`tabs.rs`, the oracle's `occupied`
  window) because rtwk's `TabBar` cannot draw the filled bar: G36.

### Deltas and needs (no visual delta is accepted)

- None accepted. What differs is listed as pending with its reason above; each
  has a path (core seed or field, the markdown carry, the keymap, rtwk G35/G36).
- Handover to `agents-live-shell` / core (not my area, named here): (1) a
  `notice` key in `seed_ev` and a failure model for archive/delete; (2) hide the
  removed task at `Confirmed(true)` and unhide on failure; (3) a voice flag on
  `RowData`; (4) `Model::plan_bills_in_dollars` (the view's `bills_in_dollars` is
  `false` until then); (5) the 180 ms debounce of the first history search page:
  core emits `Command::Search{first}`; the shell must delay running it by 180 ms
  and drop it if a newer search arrives (an rtwk timer, `AppBinding::start_timer`,
  or a `tokio::time` race in the effect), the view cannot, the text of the model
  must change at once; (6) the oracle's modal queue during archive/delete (input
  is consumed until the request ends) is the shell's: in tui-rtwk the center stays
  live, `Esc` leaves, `Enter` and `n` are held by core's gate (`blocked`).
- rtwk: G35 (SelectionList: page/move call, wrap, key table) and G36 (TabBar).

## Shell side (task `agents-live-shell`)

Done (tests: `tests/shell_agents.rs` on the scripted backend, `core/tests/agents_live.rs`):
directory and repository keys, backend facts, the worktree command, discovery
from both sources (`shell::rpc`), the 65 s usage timeout, the failure dialogs,
seeded notices and fixtures, the voice badge, `plan_bills_in_dollars`, and:

- The history search's debounce (`shell::search::Latest`, oracle
  `search_agents_overview`): the first page of a search waits 180 ms, a newer
  search drops the pending one, other commands neither wait nor drop; used by
  the host pump and the scripted backend. Tests: `the_search_waits_before_it_asks_the_server`,
  `a_newer_search_drops_the_pending_one`, `other_commands_neither_wait_nor_drop_a_search`,
  `typing_a_word_asks_the_server_once` (bite shown by removing the abort and the delay).
- The pinned section is read with the first page of the list (core
  `sources.rs`: two more `thread/list` requests sorted by `section_position`,
  `Page.pins`): ranks, pinned tasks outside the recent page are rows, a server
  that refuses has no pins, the help then offers `p Pin/unpin`. Gap: one page
  of 100 per source (the oracle follows `next_cursor`); the client's failure text
  is read for `(code -32602..-32600)` because the shell's client closure gives text.
- `codex agents` starts no thread on connect (`CodexApp::start_thread_on_connect`,
  set by `with_start_agents`), so no `Untitled task` row exists.
- Ctrl+C: closes a search or rename line, otherwise quits (oracle `on_ctrl_c`
  NotHandled -> the app quits); an empty list keeps the keys (focus moves to
  the screen root), so Shift+Tab leaves a filter without tasks.

Pending (owner, reason): `overview_thread_colors_*` (reuse-carry: the syntect
theme palette behind `thread_color` is not carried, nothing is invented);
`live_center_pages_visible_rows_while_reconnecting` (rtwk G35).

Pty scenario `agents` (codex-rtwk against the pinned oracle), checkpoints
`overview second-row help help-closed filter-needs-you filter-all after-exit`:
all rows equal except (1) `help`: the Delete key reads `backspace` where the
oracle's binary reads `delete` (oracle `key_hint.rs` prints `backspace` under
`cfg(test)` only: the snapshots and our tests say `backspace`, the binary says
`delete`; the columns of the help widen with it), and (2) the cursor position
of a hidden cursor, which `tests/support/compare.rs` compares (oracle leaves it
where it last drew; unobservable).

## Follow-ups (task `vs1-followups`)

Tests first (commit `b95c3b2c1`, red), code in the next. Tests: `core/tests/agents_followups.rs`,
`core/tests/agents_live.rs`, `tests/shell_agents.rs`, `tests/agents.rs`, `tests/theme.rs`,
`tests/cli.rs`, `tests/rules.rs`, `tests/scenarios/unit.rs`.

| Item | What | Oracle | Tests |
|---|---|---|---|
| 0 Thread colours | core `thread_color::slot(id, n)` = the oracle's FNV-1a over the id text; the colours are `theme::ThreadPalette` (mocha on a dark theme, latte on a light one: the 14 accents the oracle's `theme_accents` finds in its default syntax theme, computed once with its own function, written as values, no syntect); the view reads `ThreadPalette::of(Theme::of(cx))`; `Ev::ThreadColors(bool)` carries `tui.status_line_use_colors` (the host reads it from the config; off: every title plain). Unselected title in the thread's colour, the details title too (bold); the selected row keeps the selection style | `thread_color.rs`, `agents_overview_view.rs:348`, `rows.rs:246` | `assignments_reach_all_fourteen_default_accents`, `the_slot_is_the_oracles_fnv_1a_of_the_full_id`, `thread_colours_are_on_until_the_setting_turns_them_off`, `theme::the_thread_palette_is_the_oracles_default_accents`, `a_thread_gets_the_colour_the_oracle_gives_it` (id 42 is `Rgb(242, 205, 205)`), `agents::overview_thread_colors_match_footer_and_respect_color_suppression` (cells: on and off) |
| 1 Delete label | the help entry reads `delete` as the shipped binary does; the screen tests normalize the oracle's rows (`as_shipped` in `tests/agents.rs`: the label and the three cells it takes out of the Tasks column, help screen only) in the comparison only | `key_hint.rs:108` (`cfg(test)` arms) | the four help snapshots, strict scenario `agents_parity` with no help-column difference |
| 2 Account | the shell reads `account/read` at connect (`shell::workspace::account`, host and scripted backend), core `account_request`/`account_ev`; `AccountUpdated` is observed (usage dropped, plan, Codex backend auth) | `read_account`, `SessionAccount::from`, `app_server_events.rs:403` | `the_account_*`, `an_account_update_changes_the_plan_and_drops_the_usage`, `the_live_center_fetches_the_usage_of_the_highlighted_task` |
| 3 Pins | `PinReader` follows each source's cursor to its end; the refusal is told by `ServerError.code` (the host maps `TypedRequestError::Server`), never by the message; the pins ride with the first page, read next to the list's own two requests | `list_pinned_threads` | `pins_follow_the_cursor_beyond_the_first_page`, `a_server_without_pins_and_a_failed_read` (a message that reads like a refusal is not one) |
| 4 Hidden cursor | the harness compares the cursor position only when it is visible on both sides | | `a_cursor_hidden_on_both_sides_is_not_compared`, `a_cursor_visible_on_one_side_or_at_another_place_differs` |
| 5 Unsubscribe | a task started in a worktree that cannot be bound is unsubscribed before the error shows | `agents_overview_new.rs` | `a_task_that_cannot_be_bound_is_unsubscribed`, `a_bound_task_stays_subscribed` |
| 6 Help, version | `Cli::early_answer`: the oracle's own text for `--help`, `-h`, `help`, `help agents`, `agents --help|-h`, `-V`, `--version` (captured from the shipped 0.170.0 into `src/shell/help/`, pinned by the copies in `tests/cli_help/`); `agents --version` is no answer, as in the oracle | `codex --help` | `tests/cli.rs` (6 tests) |
| 7 Pending out of the gate | the seven `pending(..)` screen tests run in `--profile pending` (each must fail with its owner), the default profile leaves them out; `tests/rules.rs` keeps the list in `.config/nextest.toml` equal to the markers | | `the_pending_profile_lists_exactly_the_pending_markers`, `..._leaves_out_the_pending_group_and_the_pending_profile_runs_it` |
| 8 Flaky debounce | the search's first page waits at a `Gate` in the tests (`Latest::paused_by`, `FakeHandle::hold_searches`), not 180 ms of wall clock; the live tests also wait for the search line and for a confirmation before their next key (a key that came first went to the list) | | `typing_a_word_asks_the_server_once` and the three `Latest` tests; 3 x 20 stress runs of the binary, and 20 with the screens binary running beside it, all green |

Deltas: none visual. Known differences that are the person's to see: `codex-rtwk --help` and `-h`
print the oracle's whole command list (the commands we do not have too), as the shipped binary
does; the wrap of the long help is the one clap uses without a terminal (width 100).

## View on rtwk's own means (task `agents-view-rtwk`)

- `layout.rs` is gone. The view is `Text` (no wrap, `TextOverflow::Ellipsis`; word
  wrap for the notes), `Row`/`Column`/`Expanded`/`SizedBox`/`Padding`/`Divider`/
  `VerticalDivider`, `LayoutBuilder` for the width breakpoints (two panes from 90,
  metadata from 57, keys hint from 80), `Grid` for the help (`help.rs`; the key
  track is as wide as the longest key of all entries, as the oracle's), and
  `Dialog::bare(..).max_width(72)` for the confirmation and the refusal panels.
  Width of text for column tracks is `rtwk_unicode::string_width`.
- Rule R15 (`tests/rules.rs`): no `.chars().count()`, `UnicodeWidthStr`,
  `repeat(" ")` padding, no `fn wrap/ellipsize/truncate/pad` in `src/screens/**`.
  Allowlisted with a gap and a size limit (170 code lines): `path.rs` (G37),
  `tabs.rs` (G36), `soft.rs` (G38). Files of other screens that still break it
  are in `R15_NOT_YET`, each to go with its own rework.
- Gaps found: N1 (`TextOverflow::Ellipsis` drops the blanks before the `…`: the
  clipped tab ` All…  ›` against the oracle's ` All … ›`; the test
  `older_server_notice_truncates_in_narrow_overview` is `pending(Rtwk)`), G37, G38.
- D4: the old view's `width - panel` / `inner - 4` underflow is gone (constraints
  replace the arithmetic); `the_center_draws_at_widths_one_to_ten` (widths 1..10, heights
  3, 6, 24, both the overview and the confirmation) is green. It uses
  `common::start_clipping` (Tester without `fail_on_overflow`): at heights 1..3 the
  debug menu below the screen overflows, which is not this screen's.
- S5 done in `agents-view-rest` (see below).
- Size (code lines, tests excluded): layout.rs 390 -> 0, view.rs 1238 -> 974,
  mod.rs 892 -> 881, new help.rs 107, path.rs 162, tabs.rs 96, soft.rs 15;
  all `.rs` of the screen 2,921 -> 2,636 (the target 1,600 is not met: mod.rs (state,
  handlers) was out of this slice's reach and path/tabs/soft are the gaps' pieces).

## Rest of the view (task `agents-view-rest`)

- S5: the list is `view::RowsList`, a stateless `impl_widget!(stateless eq)` widget
  over `ListInputs` (look, strings, model, the center's `rows` and `entries`, never its
  cursor fields), the list's controller/focus/scroll notifier and handlers. The row
  handlers (`cx.map` per row) are kept in `ScreenState.row_taps` while the row keys are
  the same, `open` is `map_static!`: a handler made anew is never equal. Proof:
  unit test `the_list_inputs_ignore_the_cursor` (view.rs); and a counter probe (an
  `eprintln!` in `RowsList::build`, removed) over `down_moves_the_highlight_and_the_details`
  printed one build for the whole flow, the Down press not among them. The test does
  not count builds in the tree; it proves the inputs' equality.
- Confirm and failure panels are `show_dialog` routes (`dialogs.rs`): the modal focus
  scope, Esc and the focus given back are rtwk's (`confirmation_dialog_closes_on_escape_and_gives_the_list_its_keys_back`
  is green and was written against the old view's behaviour). Options: `barrier_alpha: Some(0)`,
  `fade_in: ZERO`; the failure at `Alignment::TOP_LEFT` with the screen's content not drawn
  while it shows (the oracle's refusal view replaces the center). Removed from the screen:
  the confirm/failure key routing, `confirm_selection`, `confirm_focus`, `confirm`, the
  Stack overlay. The confirmation carries the question's text (`view::Ask`: name, cloud
  flag) instead of reading the model: a seeded debug screen's providers sit below the
  `Navigator`, so a dialog route cannot see them (it showed `Untitled task`); the live failure
  dialog does read the model (its test is `shell_agents::lifecycle_failure_dialogs`, green).
  The answer reaches the screen through a handler of the screen (`cx.map`), since R1 keeps
  `spawn` in the shell.
- Keys: `act_of`/`step_of`/`on_key` are gone; per mode tables (`HELP_KEYS`, `SEARCH_KEYS`,
  `RENAME_KEYS`, `BROWSE_KEYS`) feed rtwk's `KeyBindings`; the focused widgets keep the
  keys they take. Still in the screen: `refocus_after_frame`/`Msg::Refocus` (help and the
  empty list put the focus on the root node; the list and the editing line after a mode
  change), because the list is `ExcludeFocus`ed while a line is open and a widget a mode
  drops lets go of the focus with the frame; no rtwk facility replacing it was found in
  the time of this task.
- G36: `agents_overview_tab_strip_draws_oracle_filled_tabs_on_tabbar` is `pending(Rtwk)`
  (probe run: `TabBar` rows differ from the oracle's, see the gap row).

## agents-canonical-fix (review `reviews/agents-canonical.md`)

- Keys sit on each mode's widget (browse on the list or the empty line's focus stop, search/rename on the editing line, help on the help pane); `Mode { Browse, Help, Search, Rename }` replaces `help` + `editing`; no `Msg::Refocus`, `root_focus`, `listed`. List, editing line and help stay mounted under `Offstage` + `ExcludeFocus`, so the focus recovers with no request (tests `modes_hand_the_focus_back_without_a_request`, `the_empty_list_and_the_list_each_hold_the_keys`). An unmounted editor or help, or `SelectionList::enabled(false)`, loses the recovery in the app (the focus lands on the page scope): measured, so the mounted-offstage shape is used.
- A rename refuses the mouse with `IgnorePointer` at the list (not `enabled(false)`, which loses the focus recovery). The screen's `Msg::Row` guard already refused the click; the test `a_click_while_renaming_leaves_the_highlight_where_it_was` guards it and does not bite on its own.
- Usage is asked on `Msg::Cursor` (test `an_arrow_move_asks_for_the_new_tasks_usage`, bites).
- `path`, `soft`, `tabs`, `help` measure with `segmenter_of(cx)` (unit test `rows_follow_the_segmenter_given`).
- Build writes no state (row/tab taps made in `did_change_dependencies` and after `update`); the filter is a `TabController`.
- One `ui::session_list` (Agents + Resume) and one `ui::age`; G38 facade export only; N2 not a gap.
- Left: one shared confirmation picker for `agents/dialogs.rs` and `select/dialog.rs`.

## agents-parity-fix (review `reviews/agents-feature-parity.md`)

| Item | State |
|---|---|
| 1 Details Prompt / Last message / title via `thread/read` and `thread/turns/list {limit:1}`, `thread/loaded/list`, re-read after ThreadStarted/Closed/StatusChanged/NameUpdated | done: `core/src/screens/agents/reads.rs`, tests `core/tests/agents_details.rs`. Not ported: `decode_task_links` in the prompt (only Text inputs are joined) |
| 2 `o` opens the resume picker | DONE (agents-parity-fix-2): `Act::Resume` pushes the picker over the center (`Navigator.push`, not a replacement); opening it (`resume::Intent::Open`) lists the sessions again and its rows follow the agents list (`resume::listing`, hidden tasks included); a pick resumes the task (`agents::Intent::Resume`, unhides it) and closes the picker; Esc pops back to the center as it was |
| 3 Latest activity accumulates `**..**` across deltas | done, oracle test ported |
| 4 F3 closes the search again | done (`f3_toggles_the_search_line`, shown biting) |
| 5 Grouping persisted and read at start | done, Directory is not saved (as the oracle) |
| 6 ThreadReverted resets Prompt, Last message, tokens | done |
| 7 CLI `--cd/-C`, `--no-alt-screen`, `--remote`/`--no-daemon` | `--cd` and `--no-alt-screen` done (inline rendering not verified by a test); `--remote` and `--no-daemon` now live (see agents-daemon below) |
| 8 `/subagents` | opens `Target::AgentPicker`; the picker is not built, the sheet shows "Not implemented yet: subagent picker"; `/agents` is the command center |

## agents-daemon (daemon, remote, reconnect, older-server notice)

| Oracle | Port |
|---|---|
| `codex agents` starts or reuses the local daemon (`start_with_features_classified`) and connects over its UDS | `host/src/in_process.rs` `connect_first`; default for `agents`; verified by hand in a pty against the real daemon (real tasks shown) |
| `--no-daemon` embeds the app server | `Mode::resolve` in `host/src/target.rs`; see daemon-fallback below |
| `--remote ADDR` (`ws://`, `wss://` with `--remote-auth-token-env`, `unix://`) and its refusals (`--no-daemon` with `--remote`, token over plain ws to non-loopback) | `host/src/target.rs` `resolve_remote_addr`, tests ported there; `Cli.remote*` in `src/lib.rs`, `tests/cli.rs` |
| reconnect policy 0, 1, 2, 4, then 8 s, budget 120 s; "Reconnecting - agent list is stale", "Reconnect failed - agent list is stale; relaunch to retry" | `src/shell/reconnect.rs`; tests `tests/shell_connection.rs` on tokio paused time (shown biting: delays mutated, red); core `core/tests/agents_reconnect.rs` (`reconnecting_drops_the_stale_reads_and_says_so`, `a_new_connection_ends_the_notice_and_reads_the_list`, `a_failed_reconnect_asks_for_nothing`) |
| older-server notice `Service v{server} {<|!=} Codex CLI v{client}` (+ ` / daemon` locally), lowest footer priority | `src/shell/version.rs`, `Model.server_version`, tests `tests/shell_connection.rs` (stable/prerelease/other lines/words) |
| `refreshing_one_backends_pins_preserves_cross_backend_rank`, `overview_keeps_rows_and_routes_actions_to_the_owner`, `shared_overview_*` | DONE (see section agents-cloud and agents-backend-switch below) |

Notes: the epoch of the connection is kept across a reconnect and in-flight work is dropped; after `Reconnected` core reads the list again.

## daemon-fallback (a fresh CODEX_HOME without the daemon package)

The oracle does not fall back to the embedded server here. `codex agents` calls `start_with_features_classified`; with an unpackaged CLI and no installed daemon it fails at once with `this CLI has no complete local package; install a packaged Codex CLI or use the standalone installer` plus the hint `To work without the background server, rerun the same command with --no-daemon (including resume or fork and its arguments).`, `run_main_inner` returns it and the process ends `Error: <text>` (exit 1). Ours did the same call but showed the error nowhere on the command center: it stayed on "Loading tasks...".

| Oracle | Port |
|---|---|
| any start error ends the process with `Error: <message>` after the terminal is restored | core `AppFx::ends_app` (set by `AppEv::StartFailed`), `UiApp::ingest` quits; the host hands the message to `main` (`InProcess.failed`) which returns it; tests `a_start_failure_ends_the_app`, `a_connection_that_does_not_start_ends_the_app`; checked by hand in a pty: the oracle's two lines, byte for byte |
| `-c key=value` over the config; overrides other than `features.*` bool, `suppress_unstable_features_warning`, `tui.fullscreen_transcript` exclude the daemon (`daemon_startup::config_exclusion`) and a notice `Running without the shared background server: command-line configuration overrides (-c, --enable, --disable, or --search) requires embedded mode.` is shown | `Cli.config_overrides`, `host/src/target.rs` `config_exclusion`/`startup_warning` (test `feature_overrides_allow_daemon_without_allowing_arbitrary_config` ported, `overrides_the_daemon_cannot_take_keep_the_command_center_embedded`), applied with `ConfigBuilder::cli_overrides`; the notice is kept among the startup warnings (`/warnings`), the oracle puts it into the chat history as a `StartupWarningsCell` (the chat history is not built from it here) |
| scenario `agents` | oracle `-c analytics.enabled=false agents --no-daemon`, ours `-c analytics.enabled=false agents` (default mode; `-c` makes the oracle's default mode embedded as well): 6/6 scenarios green |

Not ported: `--enable/--disable/--search` flags (so the exclusion by them is unreachable), the other exclusions the command line cannot express (`--profile`, `--strict-config`, `--dangerously-bypass-hook-trust`, workload identity, a custom loader). The chat decides its server by the same rules once the config is read (`f-chat-daemon-autostart`: `target::auto_start`, `daemon::wizard_exclusion`, WSL DrvFS, `--oss`, `CODEX_EXEC_SERVER_URL`), so the `-c` exclusion and its notice cover a plain chat too.

## agents-cloud (multi-backend overview)

Oracle (rust-v0.170.0): `Feature::TuiMultiBackend` + `cloud_preview_available` + macOS/Linux + no workload identity/config server -> `cloud_backend::connect(&config)` (`app-server-client/src/cloud_backend`, used as it is) is registered next to the local session (`app/backend_sessions.rs`); the overview merges the sessions' rows.

| Oracle | Port |
|---|---|
| `BackendRegistry::observe/owner` (first backend that lists an id owns it; later rows of that id are dropped) | DONE in core: `Model::observe`, `owners`, `owner_of` (`core/src/screens/agents/mod.rs`), `merge_page(owner, ..)`; `Ev::ListedBy(BackendId, Page)` |
| `pins_by_backend`, ranks over its values; `refreshing_one_backends_pins_preserves_cross_backend_rank` | DONE: `Model::set_pins`, `pins_by_backend`, `pinned` derived; test `core/tests/agents_backends.rs::refreshing_one_backends_pins_preserves_cross_backend_rank` |
| `overview_keeps_rows_and_routes_actions_to_the_owner` | ported in parts: rows/owners/pins as `overview_keeps_rows_and_owners_per_backend` (a list page is the unit here, not the oracle's refresh result: no `None` reads, so 3 rows not 2); `select_agents_overview_thread` -> `ResumeBackendSession(owner)` as `opening_a_foreign_row_resumes_it_on_its_owner` (the resume RPC goes to the owner, its answer makes the owner the displayed backend: `Model::display`); `view.pin_action_supported` as `pin_is_supported_by_the_displayed_backends_pins`; routing of every task command to its owner: `agents::backend_of`, `CodexApp::backend_of`. DONE by agents-backend-switch: notifications carry their origin backend (`Ingress.backend`), a `ThreadNameUpdated` of a foreign id from a non-owner is dropped (test `a_notification_of_a_foreign_thread_is_dropped_by_origin`) |
| per-backend discovery/paging (`discovery`, `initialized`, `has_more(backend)`) | DONE in core: the displayed backend's paging is `Model.paging`, the others' are `Model.parked` (`paging_of`, `has_more`, `display`, `Ev::Displayed`); another backend's page sets its own cursor and initialized flag; test `discovery_is_kept_per_backend`. `active_refresh: (Uuid, BackendId)` is n/a: our refresh is a ticket per command. `shared_overview_*` (526, 697): DONE by agents-backend-switch, see below |
| cloud archive: the owner's mode, not the displayed one | DONE: `Model::clouds`, `is_cloud(owner_of(thread))`; test `a_cloud_owned_row_archives_only_the_task_from_a_local_session` (the older `cloud_archive_preserves_attached_child_and_other_tasks` stays green) |
| the cloud connector in host/shell, the feature gate, commands carrying the owner | DONE: `host/src/cloud.rs` (`cloud_backend::connect` as it is, behind `gate`: macOS/Linux, `Feature::TuiMultiBackend`, `preview_available` = embedded or implicit daemon, no workload identity, no config server; its own pump); the shell runs `Backend::Several` and sends each command to `CodexApp::backend_of`; test `tests/shell_backends.rs` on two scripted servers (bite: routing off fails it). Failure: shown as a startup warning (source `Cloud`, text `Could not add the Cloud connection. {error}`) where the oracle inserts a history line (a delta: the chat has no free history line yet). Epochs, `CycleNewConversationBackend` / `change_host` and the origin of notifications: DONE by agents-backend-switch (below) |

Carried vs written (reuse rule): `is_overview_thread`, `supports_shared_pinning`, `matches_query`, the groups are carried verbatim in `carried.rs` (marker + `tests/carried.rs` drift test, as before). Nothing new was carriable: `BackendRegistry::observe/owner` and `BackendId` are tied to `ThreadId`/uuid types of the oracle (our core keys threads by `String` and backends by `u32`), `AgentsOverviewDiscovery::next_batch/fill` mix the merge rule with `AppServerRequestHandle` IO (the merge half already lives in `sources.rs`), the pin-rank is an inline expression of the view builder. Written by hand: `Model::observe/owner_of/set_pins/display/paging_of/has_more/pin_action_supported/is_cloud`, `backend_of` (glue: Command -> backend), the shell's `Backend::Several` and `host/src/cloud.rs` (glue and IO).

## agents-backend-switch (backend switch, epochs per backend, origin, shared overview)

| Oracle | Port |
|---|---|
| `change_host` / `CycleNewConversationBackend` (an empty conversation moves to the next listening backend; label and key in the footer) | DONE. Core: `Intent::CycleBackend`, `Command::StartThread{ticket, backend}` to the target backend, display and ownership switch only when the reply succeeds (`switch_to`: `observe`, `Model::display`, current backend, `reset_for_backend_switch`); a failed start keeps the displayed backend; with one backend or a conversation that has a turn the intent does nothing (empty = no transcript, no running turn, empty queue, no `last_submit`). The label comes from `SessionSlice` (`BackendLabel`, `Model::label/next_backend`). UI (`src/screens/chat/home.rs`, `strings.rs`): chord Ctrl+X then b on the chat's `FocusNode` key handler, a control row with the current backend name and key text (`\u{2303}x b to cycle`), the draft is carried across (`follow_core`, composer reset with the text) until the first submission. Cloud rows appear after a switch: the displayed backend's list is refreshed (`refresh_cycle`) |
| one connection epoch per backend | DONE: `Ingress{backend, epoch, ev}`; the first backend's epoch is `SessionSlice.epoch`, the others are in `epochs`; `epoch_of(backend)`, `attach_backend`; a reconnect of one backend leaves the other's events alive (test `each_backend_has_its_own_connection_epoch`, bite: one shared epoch fails it). The shell (`attach_backend`, `emit_from`), the fake server and `host/src/cloud.rs` stamp with their backend's epoch |
| notifications carry their origin; `ThreadNameUpdated` for a non-displayed backend | DONE: `RequestKey` epoch from `epoch_of(origin)`; thread notifications are checked against `owners` (drop when another backend owns it, claim when none); `notification_thread` is adapted from the oracle's `server_notification_thread_target` (the oracle's takes `ServerNotification` over `ThreadId`; ours returns a `String` id) |
| `shared_overview_keeps_rows_and_replays_changes_over_stale_reads` | ported as `core/tests/agents_shared.rs::shared_overview_keeps_rows_and_replays_changes_over_stale_reads`: facts translated to the per-task read chain. Mechanism written: notifications that arrive while a task's read is in flight are recorded in `Model.overlay` and replayed over the read's answer (`reads.rs::on_read`); `turning`/`stale` drop an old Turns answer after a revert (`on_turns`); `Ev::Lagged` invalidates and re-lists (`refresh.rs::lagged`). `observe` asks the read first, applies, then asks for rows the notification added. Bite: overlay replay off fails it |
| `shared_overview_seeds_once_and_retains_locally_resumed_history` | ported as `core/tests/agents_shared.rs::shared_overview_seeds_once_and_retains_locally_resumed_history` (a Resumed answer upserts the row; history seeded once, retained over a later read). Same mechanism |
| chat-level switch through the shell | test `tests/shell_cycle.rs::empty_thread_backend_chord_preserves_draft_until_first_submission` on two fake backends: chord, `Command::StartThread` goes to the second backend, label changes, draft kept |

Deltas and limits: the oracle's refresh-result and `DynamicToolThreadStarted` mechanism do not exist here (our refresh is a ticket per command), so the two shared_overview tests assert the same facts through the read chain; the switch is a chat key (as in the oracle), not a command-center key; the oracle's per-backend model/reasoning settings and the thread-model swap beyond an empty conversation are not carried (an empty conversation has nothing to swap). No rtwk gap.

Checks: nextest --workspace 745 passed / 17 skipped (735 before, 10 new tests); clippy --workspace --all-targets -D warnings clean; fmt clean; pty scenarios 6/6 green.

## agents-parity-fix-2 (second feature-parity round)

| Review item | Port |
|---|---|
| 44 / 69 `o` live resume picker, Esc back to the center | DONE. Tests `tests/shell_agents.rs`: `the_picker_of_the_center_lists_the_sessions`, `cancelling_resume_picker_preserves_command_center_state` (oracle test of the same name; the center's screen is equal after Esc), `a_pick_in_the_picker_resumes_the_task`, `a_hidden_task_comes_back_through_the_picker`. The picker lists the sessions the command center holds (one `thread/list` source, one model), not an own paged list: the oracle's picker filters, sorts and pages by itself (not built). |
| 70 `ThreadSettingsUpdated` | DONE: row cwd, model, provider, the task read again, replayed over a stale read (`Overlay.settings`); test `core/tests/agents.rs::thread_settings_updated_changes_the_row_and_reads_it_again`. The oracle's permission-profile bookkeeping is not built (no per-task permission selection yet). |
| 71 `ThreadUnarchived` / `ThreadStarted` clear `removed` | DONE; test `unarchived_and_started_threads_are_no_longer_removed`. |
| 8 CLI refusals | DONE: prompt/images, conflicting remote endpoints (`Cli::parse`, `tests/app/cli.rs::agents_refuses_a_prompt_images_and_conflicting_endpoints`), remote + `sandbox_workspace_write.writable_roots` (`target.rs`, `a_remote_center_refuses_local_writable_roots`), stdin/stdout not a terminal on the embedded center (`terminal_refusal`, `the_embedded_center_needs_a_terminal`). `--oss`, `--oss-provider`, `--add-dir` are no flags of this binary (unknown argument), so their remote refusal is not reachable. |
| 5 flaky `a_connection_that_does_not_start_ends_the_app` | Cause: the test's wait, not the core: `StartFailed` crosses the fake's tokio task and the ingress subscription, and ten fixed 20 ms frames of the fake clock did not wait for it under load (1 of 8 runs failed with 12 busy loops; the screen was still the center). Fix: `wait_until_ended` (bounded real-time poll); 30 of 30 green under the same load. The other two backend-crossing waits of the new tests use `wait_for_request`. |
