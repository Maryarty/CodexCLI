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
| agent_center_tests::backspace_edits_search_and_rename_without_deleting_tasks | same | yes (core part: `SetSearch`/`Commit`; the text edit is the widget's) | | |
| agent_center_tests::live_center_show_more_navigation_search_and_loading | same | yes (core part: show more, search, loading; keys are UI) | | |
| agent_center_tests::live_center_columns, _pages_visible_rows_without_wrapping, _metadata_clips_at_grapheme_boundaries, _navigation_and_complete_hints, _fixed_shortcuts_yield_to_configured_actions, _hints_follow_configured_bindings, _search_row_appears_only_while_editing | | no | rendering and keys: UI side (`agents-live-ui`) | |
| agent_center_tests::escape_returns_to_a_chat_that_is_still_starting, overview_clears_voice_badge_after_async_close | | no | App-level (chat session start, voice): chat/session slices, not the center's core (owner: chat-nav follow-up) | |
| agents_overview_tests::shared_overview_shows_only_root_sessions | same | yes | | |
| agents_overview_tests::overview_model_grouping_shows_details_and_preserves_selection | same | yes | | |
| agents_overview_tests::agents_overview_details_show_available_attention_without_expanding_rows | same | yes (core part: details data) | render: UI | |
| agents_overview_tests::server_version_overview_notice_updates_and_clears | same | yes | | |
| agents_overview_tests::overview_right_opens_current_or_highlighted_task | same | yes (was red: `Intent::Right` distinct from `Accept`) | | |
| agents_overview_tests::overview_right_preserves_editors_and_offline_state | same | yes (was red) | | |
| agents_overview_tests::overview_grouping_persists_across_config_reloads | same | yes (core part) | | |
| agents_overview_tests::command_center_refresh_failure_is_inline_and_clears_on_success | same | yes | | |
| agents_overview_tests::command_center_action_failures_remain_visible | same | yes | | |
| agents_overview_tests::command_center_new_actions_use_selection_and_leave_metadata_text_alone | same | yes | | |
| agents_overview_tests::worktrees_overview_groups_by_project_or_directory | same | yes (was red: project and directory group keys; keys arrive as `Ev::Directories`, core stays pure) | the shell does not read them yet, see Gaps | |
| agents_overview_tests::overview_directory_group_keys_normalize_aliases | same | yes (was red; same remark) | the host function that canonicalises and finds the repo root is not written | |
| agents_overview_tests::overview_fork_preserves_an_open_side_conversation, overview_fork_keeps_idle_source_input_queued | `fork_opens_the_copy_of_the_selected_task` (new; core part: `Intent::Fork`, `Command::Fork`, the copy becomes current) | partly | the side conversation and queued input are chat state (owner: chat-nav follow-up) | |
| agents_overview_tests::overview_worktree_creation_busy_state | `command_center_new_worktree_uses_the_selected_directory` (core part: `w`, `Notice::CreatingWorktree`, `Command::Worktree`) | partly | rendering of the busy notice: UI | |
| agents_overview_tests::shared_overview_keeps_rows_and_replays_changes_over_stale_reads, shared_overview_seeds_once_and_retains_locally_resumed_history, hidden_system_thread_does_not_refresh_shared_overview, hidden_title_thread_does_not_refresh_shared_overview, refreshing_one_backends_pins_preserves_cross_backend_rank, overview_keeps_rows_and_routes_actions_to_the_owner | | no | need the oracle's shared overview over two sources (interactive + exec/app server); the core has one source plus a buffer, see Gaps (owner: follow-up core task, orchestrator to plan) | |
| agents_overview_tests::agents_overview_reasoning_uses_existing_events_and_expires_with_attachment | | no | reasoning summaries from live events: not modelled (owner: follow-up core task) | |
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

- Intents: `Intent::Right` (open, unless editing), `Fork` (oracle key
  `fork`), `NewWorktree` (`w`), `NeedUsage` (view asks for the selected
  task's usage; the core answers with `Out`), `Select(Sel)`, `SetSearch`,
  `Commit(text)`.
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
`oracle_snap`) and the unit tests of `usage.rs` and `layout.rs`. Mock state is
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
| agents_overview_tests::empty_command_center_can_open_resume_picker | same | green (screen part: snapshot `agents_overview_empty_narrow`; the `o` key opens the picker in core) | |
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
| agents_overview_tests::overview_thread_colors_match_footer_and_respect_color_suppression | same | pending (agents-live-shell) | no per-thread colour on `RowData`, no `status_line_use_colors` setting in the model; snapshot is a ratatui `Style` dump |
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
- Tab strip: hand-made from spans (`layout::strip`, the oracle's `occupied`
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
