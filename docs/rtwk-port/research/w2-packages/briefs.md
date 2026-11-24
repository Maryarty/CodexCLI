# W2 UI packages: 21 → 8 briefs, superseded tasks, script candidates


Date 2026-11-24, HEAD 9f638a656. The person's decision: the 21 W2 UI packages of `reports/port-map.md` are merged into 8, with areas drawn by owned folders. kit-seeds stays separate. This file is research. The orchestrator writes the tm tasks from it.

Paths: port paths are relative to `codex-rs/tui-rtwk/`; oracle paths are relative to `codex-rs/tui/src/`.

**How the areas were checked.** I simulated the merge with `tools/rtwk-port/map-full/check.py`'s logic: the union of the merged own-lists, plus the landed W1 slice files, each given to the group whose UI uses them. Result: **0 files are owned by two packages.** The only pattern overlaps fall inside Settings, where `settings/**` covers `config_panel/**`.

**Shared door files, owned by nobody.** Each package appends its own line and never rewrites other entries (port-map rule 3):
- `core/src/features/mod.rs` (`feature_list!`) and `src/features/mod.rs` (`sheet_list!`).
- `src/pages/session/key_actions/mod.rs` (`action_list!`).
- `src/pages/session/cells/mod.rs` (`cell_list!`).
- `src/pages/session/transcript/mod.rs` (`mode_list!`).

**Test files.** These are edited one function at a time, through `map-full/unpend/<pkg>.txt`.

**Rules every brief inherits:**
- A new behaviour is a slice (ADR-012), with one `feature_list!` line plus `RUNS`/`OPENS`. Its UI half is `FeatureView` (shown, note_words, cell, slot, actions) and, for slices that have `OPENS`, `FeatureSheet`.
- Drawn state (cursors, search, tabs, selection) is UI State (ADR-013).
- Rules tested on every package: `hub_enums_only_shrink`, `hubs_only_shrink`, `features_are_named_only_through_the_list`, `app_ctx_names_no_feature`, `feature_sheet_only_with_opens`, `feature_view_has_the_cell_slot_and_actions_doors`, `test_roots_have_no_hand_mods`, and the `hub_budget.txt` numbers.

**fw-hubs-c waits**, marked **[hubs-c]** below:
- `host_capability_list!` (P6)
- `host_feed_list!` (P7)
- `AppCx::verb(PublicVerb)` (P9)
- `rpc_list!` + `core::data`

**rtwk waits** are marked **[rtwk Gnn]**.

---

## A. Briefs

### 1. w2-transcript — Transcript (ui-pager-find + ui-select-copy + ui-transcript-chrome)

**Goal:** the Ctrl+T pager with find, select mode, copy mode and turn details, the /diff pager, and the transcript chrome all work as in the oracle.

**Items**
1. Pager overlay and static pager. Oracle: `pager_overlay.rs`. Port: `src/pages/session/open/pager.rs` (new), `transcript/pager.rs`. Replace the `not_yet` at `transcript/pager.rs:356`. The pager is a `mode_list!` mode.
2. Find: search, the find footer and the strings. Oracle: `transcript_view/search.rs`, `footer.rs`. Port: `transcript/{find,find_footer,strings_find}.rs` (new), `key_actions/find.rs`. The F3 action already exists in `action_list!`.
3. Select mode: the pager motion and the selection text. Oracle: `select_mode.rs`, `selection.rs`. Port: `transcript/{select_mode,strings_select}.rs` (new), `transcript/selection.rs`. Text comes through `ItemTexts` (`cells/text.rs`). Add a `mode_list!` line.
4. Copy mode, literal blocks and markdown copy. Oracle: `copy_mode.rs`, `markdown_copy.rs`, `clipboard_copy.rs`. Port: `transcript/copy_mode.rs` (new), `transcript/{copy,copy_feedback}.rs`, `key_actions/copy.rs` (new, one `action_list!` line). The core is the `copy` slice, which is done.
5. Turn details (turn rows and tool groups). Oracle: `turn_details.rs`. Port: `transcript/turn_details.rs`, `key_actions/focus_activity.rs` (new).
6. The /diff pager: a presenter for the diff slice's `Wish::Pager`, rendered with the rtwk-rich diff view. Oracle: `diff_render.rs`. Port: `src/features/diff/`, `transcript/diff_view.rs` (new). The note "UI LEFT" at `src/features/diff/mod.rs:1-25` is closed by this item.
7. Chrome: the prompt header, the activity line, follow and history pages. Oracle: `prompt_header.rs`, `pager_overlay.rs:169-359`. Port: `src/pages/session/{transcript_header,transcript_activity,activity,transcript_pane}.rs`, `transcript/{follow,history}.rs`. **[rtwk ListController visible_range]** is missing. Use `Scrollable::ensure_visible` and record the difference as a gap row.

**read:**
- `codex-rs/tui/src/transcript_view/search.rs:66-110` — Search state; `110-853` is the impl; `find_literal` is at 938.
- `codex-rs/tui/src/pager_overlay.rs:44-113` — Overlay; `169-359` is PagerView; `420-605` is StaticOverlay.
- `codex-rs/tui/src/select_mode.rs:57-120` — pager_motion and the start of the impl.
- `codex-rs/tui/src/copy_mode.rs:21-120` — the copy-mode impl head; `literal_block_ranges` is at 353.
- `codex-rs/tui/src/turn_details.rs:15-200` — TurnRow, ToolGroup, TurnDetails.
- `codex-rs/tui-rtwk/src/pages/session/transcript/mod.rs:38-50` — `mode_list!` and the `TranscriptMode` trait.
- `codex-rs/tui-rtwk/src/pages/session/cells/text.rs:1-30` — the ItemTexts contract (selection text).
- `codex-rs/tui-rtwk/src/pages/session/open.rs:49-67` — the open registry; Target::Pager.

**Sample:** 233e77bfb (`mode_list!`), a8977e924 (`action_list!`) and `src/features/export/sheet.rs`, which shows a slice's presenter.

**Done-when:**
- Rules: `mode_list_names_a_mode_file_each`, `transcript_mod_keeps_no_mode`, `action_list_names_a_file_and_a_key_action_each`, `every_target_has_one_line_in_open`, `open_hubs_hold_no_stub`.
- Tests: the `transcript_view_*`, `pager_*` and `diff_render_*` shards for find, select, copy and turn details, and the `chat_transcript_*` tests for chrome.
- The 14 real-body stale tests of this group are un-pended.

**Area:**
- Slices: `core/src/features/{copy,diff}/`, `src/features/{copy,diff}/`.
- Core screens: `core/src/screens/transcript.rs` and `core/src/screens/transcript/`.
- `src/pages/session/{activity,transcript_activity,transcript_header,transcript_pane}.rs`.
- `src/pages/session/key_actions/{find,copy,focus_activity}.rs`, `src/pages/session/open/pager.rs`.
- `src/pages/session/transcript/*`, except `mod.rs`.

**Runs:** 4 (5 before). ItemTexts and the rtwk diff view exist, and the copy and diff cores are done. The 218 pending tests need about 4 test-and-fix rounds.

### 2. w2-composer — Composer (ui-composer-states + composer-sweep)

**Goal:** every composer state, burst, vim, mouse, queue and external-editor behaviour matches the oracle.

**Items**
1. Land or redo the 3 unmerged commits of `fleet/core-composer`, up to 5410ee3b5: the /vim fact, remote_images, the history replay seed.
2. Input states: disabled or "Shutting down..." placeholders, from `composer_state`. Oracle: `chat_composer.rs:4440-4451`. Port: `src/pages/session/composer/states.rs` (new), `core/src/threads/composer.rs:10-36`.
3. Tab-queue of slash and shell lines, and the drain. Oracle: `input_queue.rs`. Port: `composer/drain.rs` (new), `core/src/threads/queue.rs`.
4. The Ctrl+G external editor: the verb and its key action. Oracle: `external_editor.rs`. Port: `key_actions/external_editor.rs` (new, one `action_list!` line), `src/features/external_editor/`. **[hubs-c P6 Editor/Suspend]**.
5. Mouse in the composer. Oracle: `chat_composer/mouse.rs`. Port: `composer/mouse.rs` (new).
6. Vim and search. Oracle: `textarea.rs`, `chat_composer.rs`. Port: `composer/{vim,search}.rs`. This is 59 tests. **[rtwk G65 G66 G68 G78]** are open for parts of it.
7. Paste burst and sparkle. Oracle: `paste_burst.rs`, `sparkle.rs`. Port: `composer/burst.rs`, `footer/sparkle.rs`, `widgets/composer_sparkle.rs`. **[rtwk G67]** is open.
8. Mentions, suggest, popovers and the drafts widget. Port: `composer/{mentions,suggest}.rs`, `src/pages/session/popovers/`, `widgets/composer_draft*`. Also fold in the rework-dialogs leftover in popovers (`query.pop`).

**read:**
- `codex-rs/tui/src/chat_composer.rs:300-330` — the input_enabled contract.
- `codex-rs/tui/src/chat_composer.rs:4430-4460` — set_input_enabled and its placeholder.
- `codex-rs/tui/src/chat_composer/mouse.rs:1-168` — mouse.
- `codex-rs/tui/src/input_queue.rs:1-195` — the queue.
- `codex-rs/tui/src/external_editor.rs:1-120` — the editor flow.
- `codex-rs/tui-rtwk/core/src/threads/composer.rs:10-36` — ComposerState, takes_text, takes_commands.
- `codex-rs/tui-rtwk/core/src/features/external_editor/mod.rs:25-150` — the slice core.
- `codex-rs/tui-rtwk/src/pages/session/key_actions/mod.rs:50-62` — `action_list!`.

**Sample:** a8977e924 (`action_list!`) and `src/features/rename/{mod,prompt}.rs`.

**Done-when:**
- Rules: `action_list_names_a_file_and_a_key_action_each`, `every_key_action_file_is_in_the_list`, `key_hubs_match_on_no_action`.
- Tests: the `composer_*` shards (states, vim, burst, mouse, queue) and `chat_composer` external-editor tests.
- The 20 real-body stale tests are un-pended; 12 of them are in `composer_4`.

**Area:**
- Core: `core/src/features/external_editor/`, `core/src/kit/`, `core/src/screens/composer.rs` and `composer/`, `core/src/screens/popovers.rs`, `core/src/threads/{composer,queue}.rs`.
- UI slice: `src/features/external_editor/`.
- `src/pages/session/composer/*`, except `mod.rs`.
- `src/pages/session/footer/{flash,sparkle}.rs`, `src/pages/session/key_actions/external_editor.rs`.
- `src/pages/session/popovers/`, `src/widgets/composer_draft.rs` and `composer_draft/`, `src/widgets/composer_sparkle.rs`.

**Runs:** 3 (3 before). There are 98 pending tests. The rtwk gaps and hubs-c pend some of them as gap rows rather than as work.

### 3. w2-settings — Settings (ui-settings-feeds + ui-config-panel + ui-keymap-editor)

**Goal:** the /config panel, the /keymap editor and the settings sheets work as in the oracle, with the settings slice's UI split by concern.

**Items**
1. Split `src/features/settings/mod.rs` by concern into `which.rs`, `open.rs` and `shown.rs`. Today Which is at lines 70-77, open at 293, open_empty at 314 and Shown at 338-343. No behaviour change.
2. /config panel: input, render and toggle. Oracle: `bottom_pane/config_panel/{mod,input,render,toggle}.rs`. Port: `src/features/settings/config_panel/` (new). Shown::Config moves from `Shown::Other` to a real slot.
3. /keymap editor: actions, capture, debug and picker. Oracle: `keymap_setup/{actions,capture,debug,picker}.rs`. Port: `src/features/settings/keymap_editor/` (new). Uses the generated `core/src/features/settings/keymap_actions.rs`.
4. Typed `KeymapError` (`core/src/keymap/resolve.rs:32`): the core half of keymap-debt.
5. The OkOverridden warning, and the experimental features write. The write goes through the host. **[hubs-c P7 host_feed_list! settings facts]**.
6. Settings `Wish::{Status, Model}` become public verbs. **[hubs-c P9 PublicVerb]**.

**read:**
- `codex-rs/tui/src/bottom_pane/config_panel/mod.rs:1-120` — panel state.
- `codex-rs/tui/src/bottom_pane/config_panel/render.rs:1-120` — the rows; 3 of its 22 literals are in the port.
- `codex-rs/tui/src/keymap_setup/picker.rs:1-120` — the picker; 0 of its 7 literals are in the port.
- `codex-rs/tui/src/keymap_setup/capture.rs:1-120` — key capture.
- `codex-rs/tui-rtwk/src/features/settings/mod.rs:60-120` — Which mapping.
- `codex-rs/tui-rtwk/src/features/settings/mod.rs:285-345` — open, open_empty, Shown.
- `codex-rs/tui-rtwk/core/src/features/settings/mod.rs:85-130` — Wish.
- `codex-rs/tui-rtwk/core/src/keymap/resolve.rs:20-60` — KeymapError(String).

**Sample:** bb231d3f2 (the FeatureView/FeatureSheet split) and `src/features/goal/sheet.rs`.

**Done-when:**
- Rules: `feature_sheet_only_with_opens`, `feature_view_has_the_cell_slot_and_actions_doors`.
- Tests: the config_panel and keymap_setup oracle snapshots, plus the 9 pending settings tests.
- Items 5 and 6 are either landed after hubs-c or pended under `kit::pending(Owner::FwHubsC, ..)`.

**Area:** `core/src/features/settings/`, `core/src/keymap.rs` and `core/src/keymap/`, `src/features/settings/`.

**Runs:** 2 (3 before). The core is 9970 lines and done. Two sheets plus a split remain.

### 4. w2-cells — Cells (ui-exec-mcp + ui-safety-compaction + cells-sweep)

**Goal:** the exec, MCP, compaction, safety, exploration and /ps cells, and the rest of the cells sweep, draw as in the oracle.

**Items**
1. Exec cell: active command, output lines, terminal-interaction cells and the footer wait row. Oracle: `exec_cell/render.rs`, `history_cell/exec.rs`. Port: `src/pages/session/cells/` exec files.
2. MCP cells. Covers the McpToolCallCell, the /mcp inventory cell and the loading spinner from `State::loading`. Oracle: `history_cell/mcp.rs`. Port: `cells/mcp_inventory.rs` (new), `src/features/mcp_inventory/`. /mcp login is part of this item.
3. Exploration cell. Oracle: `history_cell` exploration. Port: `cells/exploration.rs` (new), from `core/src/threads/exploration.rs:58` and the `exploration_groups` at 121.
4. /ps cell (16 rows, 80-grapheme truncation). Port: `cells/processes.rs` (new), `src/features/processes/`.
5. Compaction header, timer and took cell. Oracle: `chatwidget/compaction.rs`. Port: `cells/compaction.rs` (new), `core/src/threads/compaction.rs:41,51`.
6. Safety and misalignment. The work: prompt and confirm sheets, recourse rows (Enable Daybreak, Give feedback), the precaution and review sheets, and Retry/Rewrite through `ThreadCx::start_turn`. Oracle: `safety_buffering.rs`. Port: `cells/misalignment.rs` (new), `core/src/threads/safety.rs:176-200`.
7. Guardian. The work: the Working row and denial dedup. Port: `src/sheets/approvals/guardian.rs`.
8. Provider-error cell (the red ■ error cell from chat-nav-pane). Port: `src/pages/session/live/provider_error.rs`.

Every new cell is one `cell_list!` line (Item pattern => file).

**read:**
- `codex-rs/tui/src/exec_cell/render.rs:59-212` — new_active_exec_command and output_lines; `impl HistoryCell` starts at 212.
- `codex-rs/tui/src/history_cell/mcp.rs:51-140` — McpToolCallCell.
- `codex-rs/tui/src/history_cell/mcp.rs:472-520` — new_mcp_tools_output; the loading cell is at 798-843.
- `codex-rs/tui/src/chatwidget/compaction.rs:1-57` — compaction.
- `codex-rs/tui/src/safety_buffering.rs:1-120` — safety buffering.
- `codex-rs/tui-rtwk/core/src/threads/safety.rs:176-200` — violation, blocks_input, retry, rewrite.
- `codex-rs/tui-rtwk/src/pages/session/cells/mod.rs:61-86` — `cell_list!`.
- `codex-rs/tui-rtwk/core/src/threads/exploration.rs:58-130` — ExplorationGroup and exploration_groups.

**Sample:** 2a29a1a42 (the `cell_list!` registry with `decision.rs`).

**Done-when:**
- Rules: `cell_list_covers_every_item_with_a_view_file`, `dispatch_holds_no_item_arm`.
- Tests: the `cells_*` and `history_cell` shards (exec, mcp, exploration), the `chat_approvals_2` safety tests, and the compaction tests.
- The 5 real-body stale tests are un-pended.

**Area:**
- Slices: `core/src/features/{mcp_inventory,processes}/`, `src/features/{mcp_inventory,processes}/`.
- Core screens: `core/src/screens/cells.rs` and `cells/`.
- `core/src/threads/`: activity, calls, citations, collab, compaction, daybreak, decision, directives, exploration, followups, goal, local_links, message, note, previews, processes, replay, `safety.rs` and `safety/`, status, stream_preview, user_prompt.
- `src/pages/session/cells/*`, except: mod, dispatch and notice (fw-ui-registry); decision and request_input (package 5); debug_config, forked and recap (package 6); `status/` (package 7).
- `src/pages/session/live/provider_error.rs`, `src/pages/session/status/terminals.rs`, `src/sheets/approvals/guardian.rs`.

**Runs:** 3 (5 before). The c5 and c6 cores are done. 110 tests are pending.

### 5. w2-approvals — Approvals and dialogs (ui-approvals-rest + dialogs-sweep)

**Goal:** the approval overlay, request-user-input and every select or dialog sheet match the oracle, and bare /review and /permissions open.

**Items**
1. Decision cell, review status line and the multi-line command snippet. Oracle: `history_cell/approvals.rs:46,347`. Port: `src/sheets/approvals/decision_cell.rs:31,75,341`, `cells/decision.rs`.
2. Approval overlay. Oracle: `bottom_pane/approval_overlay*`. Port: `src/sheets/approvals/`. Includes the write_stdin title "send input to terminal {id}" from `ExecPrompt.process` (`core/src/screens/approvals.rs:81`).
3. Request user input: drafts, Vim search and mouse in the answer field. Oracle: `bottom_pane/request_user_input/`. Port: `src/sheets/approvals/user_input.rs:44`, `cells/request_input.rs`.
4. Bare /review and /permissions as slices. Each gets a `feature_list!` line plus `RUNS` or `OPENS`. This removes the NotAvailable arm at `core/src/app/session.rs:328-337`.
5. Select-sheet sweep. Oracle: `list_selection_view.rs`, `multi_select_picker.rs`, `custom_prompt_view.rs`. Port: `src/sheets/dialogs/`, `src/widgets/picker/`, and models, reasoning, collaboration and apps under `core/src/screens/select/`.
6. Shortcut overlay and warnings view. Oracle: `shortcut_overlay.rs`, `warnings_view*.rs`. Port: `src/sheets/dialogs/`.
7. User verification sheet. Oracle: `bottom_pane/user_verification.rs`. Port: `src/sheets/dialogs/user_verification.rs` (new).
8. Move the rework-dialogs leftover `ScrollState`/`ensure_visible` (`core/src/commands.rs:691-861`) out of core into UI state (ADR-013). Also un-pend the stale reasons in `chat_approvals_1`. **[rtwk G8 G56]** are open for parts of item 5.

**read:**
- `codex-rs/tui/src/history_cell/approvals.rs:46-120` — new_approval_decision_cell; new_review_status_line is at 347.
- `codex-rs/tui/src/bottom_pane/list_selection_view.rs:1-120` — the base select view.
- `codex-rs/tui/src/bottom_pane/shortcut_overlay.rs:1-120` — the shortcut overlay; 22 of its 23 literals are in the port.
- `codex-rs/tui-rtwk/core/src/screens/approvals.rs:60-81` — ExecPrompt and its process field.
- `codex-rs/tui-rtwk/src/sheets/approvals/decision_cell.rs:31-110` — DecisionCell and ReviewLineCell; snippet is at 341.
- `codex-rs/tui-rtwk/src/sheets/approvals/user_input.rs:44-120` — UserInputView.
- `codex-rs/tui-rtwk/core/src/app/session.rs:320-358` — the NotAvailable arms and feature_command.
- `codex-rs/tui-rtwk/core/src/commands.rs:691-760` — ScrollState, which is to leave core.

**Sample:** 913860d68 (the /goal slice) and `src/features/plugins/` for a slice with a sheet.

**Done-when:**
- Rules: `adr013.rs` (no drawn state in core), `every_target_has_one_line_in_open`, `feature_sheet_only_with_opens`, `only_open_matches_on_target`.
- Tests: the `chat_approvals_*`, `approval_overlay`, `request_user_input`, `list_selection` and `shortcut_overlay` shards.

**Area:**
- `core/src/data/{files,history,links,mention,task}.rs`.
- Slices: `core/src/features/{goal,plugins}/`, `src/features/{goal,plugins}/`.
- Core screens: `core/src/screens/approvals.rs` and `approvals/`, `core/src/screens/{dialogs,select}.rs`.
- `core/src/screens/select/{apps,collaboration,models,permission_args,permission_shortcuts,permissions,persist,reasoning,review}.rs`.
- `src/pages/session/cells/{decision,request_input}.rs`.
- `src/sheets/approvals/` (except `guardian.rs`) and `src/sheets/dialogs/` (except `feedback.rs`).
- `src/widgets/`: age, directives, dots, interest, key_guard, measure, `mod.rs`, options, page_keys, `picker/`, sheet, tab_strip, thread_ui.

**Runs:** 3 (4 before). 112 tests are pending.

### 6. w2-commands — Commands and pickers (ui-openers-side + ui-worktree-sandbox + ui-feedback-cmds)

**Goal:** /side, /subagents, the daemon menu, presets, the worktree picker, the Windows sandbox, security setup, /feedback and the small commands behave as in the oracle.

**Items**
1. Daemon sheet from `Wish::Menu` and `State::page()`, through the sheet and `sheet_list!`. Oracle: `app/daemon_menu.rs`. Port: `src/features/daemon/`, `src/sheets/select/daemon` (new), `src/pages/session/open/daemon.rs` (new). Replaces the `not_yet` at `src/features/daemon/mod.rs:29-34`.
   - Host daemon facts: **[hubs-c P7]**.
2. /subagents picker from `agent_picker::rows`, then `AppCx::open_thread`. Port: `src/sheets/select/agent_picker` (new), `open/agent_picker.rs` (new). Replaces the `not_yet` at `src/pages/agents/mod.rs:56`.
3. Side: Ctrl+/ toggle and Ctrl+C discard verbs (`AppCx::discard`), and the footer wording. Oracle: `app/side.rs`. Port: `src/features/side/`, `key_actions/side.rs`.
4. Presets and quick dials: the composer HUD sends `Intent::Quick{gate, Open::Menu}`. Port: `src/sheets/select/quick*`. The `not_yet` at `src/sheets/select/mod.rs:38` must go.
5. Worktree: checkout choice and the creation-progress view. Oracle: `chatwidget/worktree_picker.rs`. Port: `src/sheets/select/worktree*`.
6. Windows sandbox and security setup. Covers: enable and fallback sheets, the "Setting up sandbox..." status, the "(non-admin sandbox)" label and elevate hint, and the inline security banner. Oracle: `windows_sandbox_prompts.rs` (1 of its 14 literals is in the port), `security_setup.rs`.
   - Port: `src/features/{windows_sandbox,security_setup}/`.
   - `Fact::Trusted` from the host: **[hubs-c P7]**.
7. /feedback. Covers the note, consent and category steps in `src/features/feedback`, the switch from dialogs to the slice's `Intent::Open` plus a wish, and the success cell as a multi-line hyperlink cell. Oracle: `bottom_pane/feedback_view.rs`, `feedback_note_view.rs` (1 of its 21 literals is in the port).
8. App link view, plus the debug-config, forked and recap cells. Oracle: `bottom_pane/app_link_view.rs`. Port: `src/sheets/select/app_link*`, `cells/{debug_config,forked,recap}.rs` (new, `cell_list!` lines).

**read:**
- `codex-rs/tui/src/app/daemon_menu.rs:1-120` — the daemon menu.
- `codex-rs/tui/src/chatwidget/worktree_picker.rs:1-120` — the worktree picker.
- `codex-rs/tui/src/bottom_pane/feedback_note_view.rs:1-120` — the note step.
- `codex-rs/tui/src/app/side.rs:1-120` — side verbs.
- `codex-rs/tui-rtwk/core/src/features/daemon/mod.rs:40-130` — Wish::Menu; cx.wish is at 123.
- `codex-rs/tui-rtwk/core/src/screens/select/agent_picker.rs:110-170` — rows, opened, has_subagents.
- `codex-rs/tui-rtwk/src/features/mod.rs:169-240` — FeatureSheet and `sheet_list!`.
- `codex-rs/tui-rtwk/core/src/features/cx.rs:425-470` — open_thread, spawn_thread, discard, submit, give_back.

**Sample:** d4064af06 (the diff and daemon slices), 64b4002cd (the hubs-b doors) and `src/features/export/sheet.rs`.

**Done-when:**
- Rules: `feature_sheet_only_with_opens`, `open_hubs_hold_no_stub`, `every_target_has_one_line_in_open`.
- Tests: the `chat_commands_*` shards (side, subagents, feedback), plus the worktree, sandbox and daemon snapshots.
- The `not_yet` stubs listed above are gone.

**Area:**
- Core slices: `core/src/features/{daemon,feedback,recap,security_setup,side,small_cmds,windows_sandbox}/`.
- Core screens: `core/src/screens/select/{agent_picker,quick_dials,worktree}.rs` and `worktree/`.
- UI slices: `src/features/{daemon,feedback,recap,security_setup,side,small_cmds,windows_sandbox}/`.
- `src/pages/session/cells/{debug_config,forked,recap}.rs`, `src/pages/session/key_actions/side.rs`, `src/pages/session/open/{agent_picker,daemon}.rs`.
- `src/sheets/dialogs/feedback.rs`.
- `src/sheets/select/`: agent_picker, `app_link.rs` and `app_link/`, confirm, daemon, permissions, `quick.rs` and `quick/`, `worktree.rs` and `worktree/`.

Note: the recap slice dirs overlap the active fix-recap-title claim. Hold Commands until that task lands.

**Runs:** 3 (5 before). The cores are done. 64 tests are pending, but 233 oracle literals are missing.

### 7. w2-status — Status (ui-usage-limits + footer-status-sweep)

**Goal:** /usage with reset credits, thread usage, rate-limit prompts, the /status card, the footer and the status line all match the oracle.

**Items**
1. A thread-usage slice (new) with the `thread_usage` state machine. Moves out of `core/src/screens/usage/thread.rs:22-102` into `core/src/features/thread_usage/`, which is added to this area. Gets a `feature_list!` line. The timer goes through `wake_at` and `AppFacts.now`.
2. The usage page, with real bodies for the 14 bodiless stubs. Oracle: `chatwidget/usage.rs`, `status/thread_usage.rs`. Port: `src/pages/usage/`.
3. Redeem reset: the entry, picker, confirm and consuming popups, and `Intent::RefreshRateLimits`. Oracle: `reset_credits.rs`. Port: `core/src/screens/usage/reset.rs:19-136`, `live/usage_menu.rs` (new). The RPC: **[hubs-c rpc_list! ConsumeAccountRateLimitResetCredit]**. Today it is in `host/src/usage.rs:10-25`.
4. Rate limits. Covers the switch prompt at >=90 and the composer usage-notice line from `limits().notice()`. Oracle: `rate_limits.rs` (8 of its 22 literals are in the port), `usage_notice.rs`. Port: `core/src/threads/limits.rs` (new), `live/limits.rs` (new).
5. /status card. Covers account, remote connection, the limit warnings and the thread-usage row. Oracle: `status/{card,rate_limits,account,remote_connection,format}.rs`. Port: `src/pages/session/cells/status/`, `core/src/screens/status/`.
6. Footer and status line. Covers hints, indicators, effort, and the status-line setup and style, including the billing items. Oracle: `footer.rs`, `bottom_pane/status_line*.rs`, `status_indicator_widget.rs`. Port: `src/pages/session/footer/`, `src/widgets/status_line.rs`.
7. Window title and status surfaces. Port: `src/pages/session/{window_title,status_surfaces}.rs`.

**read:**
- `codex-rs/tui/src/chatwidget/usage.rs:1-120` — the /usage view.
- `codex-rs/tui/src/chatwidget/reset_credits.rs:1-79` — reset credits.
- `codex-rs/tui/src/chatwidget/rate_limits.rs:1-120` — the switch prompt and warnings.
- `codex-rs/tui/src/status/card.rs:1-120` — the /status card.
- `codex-rs/tui-rtwk/core/src/screens/usage/thread.rs:22-102` — the thread-usage machine, which moves to the slice.
- `codex-rs/tui-rtwk/core/src/screens/usage/reset.rs:19-136` — Outcome, CreditOption, options, MenuEntry, Listing.
- `codex-rs/tui-rtwk/core/src/screens/status/warnings.rs:150-270` — blocked and notice.
- `codex-rs/tui-rtwk/core/src/features/mod.rs:350-372` — wake_at and thread_wake_at.

**Sample:** fw-hubs-a's wake_at clock commit, plus 913860d68 (the /goal slice shape).

**Done-when:**
- Rules: `features_are_named_only_through_the_list`, `app_ctx_names_no_feature`.
- Tests: the `chat_status_*`, `pane_footer_*`, `status_card_*` and usage shards.
- The 18 real-body stale tests are un-pended; 9 of them are in `chat_status_1`.
- The usage stubs get bodies.

**Area:**
- `core/src/data/usage.rs`, `core/src/features/thread_usage/` (new).
- Core screens: `core/src/screens/status.rs` and `status/`, `core/src/screens/usage.rs` and `usage/`.
- `core/src/threads/limits.rs`, `host/src/analytics.rs` and `analytics/`.
- `src/pages/session/cells/status/`.
- `src/pages/session/footer/*`, except flash and sparkle.
- `src/pages/session/live/{limits,usage_menu}.rs`.
- `src/pages/session/status/{entries,fixtures,mod,pending,strings}`.
- `src/pages/session/{status_surfaces,window_title}.rs`, `src/pages/usage/`.
- `src/services/history/usage.rs`, `src/widgets/status_line.rs`.

**Runs:** 4 (4 before). 208 tests are pending, the thread-usage slice is new work, and the reset item waits on hubs-c.

### 8. w2-live — Live turn (live-turn-sweep + markdown-gaps)

**Goal:** backtrack, plan mode, interrupts, banners, the empty-state animation and markdown rendering match the oracle.

**Items**
1. Backtrack: Esc primes, Esc browses prompts, Left/Right choose, Enter forks. Oracle: `app_backtrack.rs`. Port: `src/pages/session/backtrack/`, `core/src/screens/backtrack.rs`.
2. Plan implementation prompt. Oracle: `chatwidget/plan_implementation.rs`. Port: `live/plan_mode.rs:23,52`.
3. Interrupts and restore. Oracle: `interrupts.rs`. Port: `live/{restore,mod}.rs`.
4. MCP startup banner and fallback banners. Oracle: `mcp_startup.rs`. Port: `src/pages/session/banners.rs`.
5. Empty-state animation. Oracle: `empty_state_animation.rs`. Port: `src/pages/session/blossom/`.
6. Markdown rendering gaps. Oracle: `markdown_render.rs`. Port: `src/widgets/markdown.rs:27-71`, `markdown/`. **[rtwk G61 G62 G63 G64 G73]** are open. What they block stays pended against the gap row. Link destinations wait on the rtwk MarkdownView suffix option.

**read:**
- `codex-rs/tui/src/app_backtrack.rs:1-120` — the backtrack states.
- `codex-rs/tui/src/chatwidget/plan_implementation.rs:1-106` — the plan prompt.
- `codex-rs/tui/src/interrupts.rs:1-120` — interrupts.
- `codex-rs/tui/src/mcp_startup.rs:1-120` — the banner.
- `codex-rs/tui/src/markdown_render.rs:1-120` — the renderer head.
- `codex-rs/tui-rtwk/src/widgets/markdown.rs:27-71` — view, view_in, prepared, themed.
- `codex-rs/tui-rtwk/src/pages/session/live/plan_mode.rs:20-60` — menu and ask.
- `codex-rs/tui-rtwk/core/src/screens/backtrack.rs:1-120` — the backtrack core.

**Sample:** 64b4002cd (the hubs-b doors; the backtrack fork uses `spawn_thread`/`submit`).

**Done-when:**
- Tests: the `chat_live_*`, `markdown_*`, `markdown_streaming_*` and `app_backtrack` shards.
- The 9 real-body stale tests are un-pended.
- Markdown tests blocked by rtwk stay pended under the G-row.

**Area:** `core/src/screens/backtrack.rs`, `src/pages/session/backtrack/`, `src/pages/session/banners.rs`, `src/pages/session/blossom/`, `src/pages/session/live/{browse,mod,plan_mode,restore}.rs`, `src/widgets/markdown.rs` and `markdown/`.

**Runs:** 2 (3 before). There are 158 pending tests, 112 of them markdown, and many of those markdown tests are gap-pended. The other 4 items are small.

**Run totals:** 4 + 3 + 2 + 3 + 3 + 3 + 4 + 2 = **24**, against about 33 for the 21 old packages.

---

## B. Superseded tm tasks

| Old alias | New package / verdict |
|---|---|
| pager, transcript-slash-copy | Transcript |
| chunk-transcript, chunk-transcript-2-find, -select-mode, -copy-mode, -turn-details, -history-pages | Transcript |
| core-transcript2 | close: by ADR-013 selection, search and copy are UI State, so no core slices are needed |
| chat-popovers, chunk-composer and every chunk-composer-* (states, vim, images, history, live-mentions, embedded) | Composer |
| chunk-composer-mention-seeds | kit-seeds (seeds) plus Composer (view) |
| chunk-status-sparkle, chunk-commands-queue, chunk-commands-submission | Composer |
| core-composer | Composer item 1 (land or redo the 3 commits up to 5410ee3b5) |
| chat-nav-pane | Composer; the red ■ error cell goes to Cells item 8 |
| settings, settings-gaps | Settings; the host catalogues and features write wait on hubs-c P7 |
| keymap-debt | Settings takes the `core/src/keymap` half. The `src/keymap` reload half is not covered: keep it (fw-ui-registry area) |
| chunk-cells and every chunk-cells-* (background-terminals, mcp-inventory, notices, question-result, reasoning, session-info, tool-cells), core-cells | Cells |
| chunk-live-safety, chunk-live-provider-error, core-live-debts (safety part) | Cells |
| chunk-cells-rendering-prefs | Live turn (markdown prefs) |
| chunk-approvals, dialogs-pane (approvals and select part) | Approvals+dialogs |
| chunk-popups, chunk-popups-goal, -plugins, -review, -sweep, -permission-args, -picker-rest, -apps-rest | Approvals+dialogs |
| small-dialogs | Approvals+dialogs, except daemon, app link and feedback |
| rework-dialogs | Approvals+dialogs item 8 (ScrollState) and Composer item 8 (`query.pop`); the pets part goes to pets-ambient (W3) |
| chunk-popups-feedback, -quick-dials, -worktree, -windows-sandbox, core-popups | Commands+pickers |
| small-dialogs (daemon, app link, feedback) | Commands+pickers |
| chunk-commands | Commands+pickers; /rename is done |
| usage, chunk-usage, chunk-usage-reset | Status |
| chunk-status, chunk-status-card and -card-*, -indicators | Status |
| core-status, dialogs-pane (the /status cell) | Status |
| backtrack, chunk-live, chunk-live-banner-fallback, chunk-live-cap, chunk-popups-plan, core-live-debts (rest) | Live turn |
| transcript-link-destinations | Live turn item 6; still deferred on the rtwk suffix option |
| chunk-status-voice | not covered — keep (voice, W3) |
| chunk-commands-host-effects | not covered — keep (fw-hubs-c / terminal-features) |
| chunk-commands-export | done (1a654bebc); close |

Completed chunks listed at `chat-feature-map.md:19` need nothing:
- chunk-popups-picker-rest, chunk-popups-apps-rest
- chunk-cells-tool-cells, -question-result, -session-info
- chunk-transcript-2 (mouse), -turn-details

---

## C. What scripts can generate instead of porter work (ranked by saving)

**1. Stale un-pend sweep.** Saves about 1 porter run per landing, about 3 to 4 runs across W2.
- Input: `summary.py --list stale` from a `KIT_PROBE_PENDING=1` run, with `map-full/pending-tests.tsv`.
- Output: the `kit::pending` lines removed for tests that now pass, grouped by owner, as one commit per package.
- Stays manual: a reviewer checks for vacuity. Some stale bodies only assert that something is absent, and those must be strengthened, not just un-pended.
- Also teach `summary.py` to mark bodiless stubs, so that the 43 usage false positives drop out.
- Evidence: today 95 real-body tests are stale (Composer 20, Status 18, Transcript 14, Live 9). The 2026-11-21 probe had 258.
- Status: new. The probe exists; the editor does not.

**2. map-full regeneration for 21 → 8.** Saves the orchestrator about 1 run, and prevents area conflicts.
- Input: the 8 own-lists of part A, written into `tools/rtwk-port/map-full/packages.py`.
- Output: `build.py` rewrites the port-map tables and `unpend/<pkg>.txt`; `check.py` proves 0 overlaps.
- Stays manual: the package prose in `port-map.md`.
- Status: **exists**; only the data needs editing. The simulation in this file used its logic.

**3. Strings tables from oracle literals.** Saves about 300 to 400 lines per package with many literals, about 2 runs over W2.
- Input: oracle string literals per module (`port-map-modules.tsv` maps module → package), filtered of tracing and log strings.
- Output: per-package `strings.rs` constant tables, each with an oracle `file:line` comment.
- Stays manual: wiring the constants into views, and format arguments.
- Evidence: about 1300 literals are missing in the 8 groups:
  - Live 397, Commands 233, Cells 209, Status 124, Settings 103, Approvals 96, Transcript 71, Composer 62.
  - Port-wide, 1823 of 3633 are missing.
- Status: new.

**4. Seeds and fixtures from oracle test setups.** Saves part of kit-seeds and about 1 run in Cells, Transcript and Status. Medium confidence.
- Input: oracle test bodies (the `history_cell/tests.rs` and `chatwidget/tests` builders).
- Output: `.jsonl` step fixtures in the existing format (`seed` / `notify` with app-server messages, plus a `# oracle file:line name` header).
- Stays manual: semantic seeds the builder cannot infer, and the review.
- Evidence: 361 pending reasons name an entry, fixture or seed, and 978 fixtures are hand-written today. `tools/rtwk-port/mock-model/` (events.py) can emit server messages.
- Status: new; it can build on mock-model.

**5. Generated UI registries.** Saves conflict resolution when 8 packages run in parallel, roughly 0.5 run each on merge days.
- Input: the files in `key_actions/`, `transcript/*_mode.rs` and `cells/`, and core `OPENS`.
- Output: `action_list!`, `mode_list!`, `cell_list!` and `sheet_list!` lines from a `build.rs`, following `test-mods/build.rs`.
- Stays manual: the Item pattern of `cell_list!`, which could come from a file-header attribute.
- Note: the core half (`feature!` and `feature_list!` registration) is fw-hubs-c item 5. Proposing the UI half there too keeps one design.
- Status: the pattern exists (`test-mods/build.rs`, `scan.rs`); the generator is new.

**6. `slice-codemod.py new` refresh.** Saves about 150 lines of boilerplate per new slice.
- Input: a slice name plus its components.
- Output: core and UI slice skeletons on the current trait: Event, Wish, Interest, FeatureView, FeatureSheet plus a `sheet_list!` line, and a core test.
- Stays manual: behaviour.
- Status: **exists but stale**. It still writes Menu/Prompt and menu_list/prompt_words (last edited b7908b078). It overlaps hubs-c's `feature!` macro, so refresh it only if that macro slips.
- Users: the new slices thread_usage, review and permissions.

**7. Hint and keymap tables.** Small saving, about 100 to 200 lines in Status, Transcript and Settings.
- Input: oracle keymap defaults and footer hint pairs.
- Output: hint tables, following `keymap-actions/gen.py`, which generated the 1204-line `keymap_actions.rs`.
- Status: the pattern exists.

**8. Port-map counts refresh from junit.** Saves orchestrator time only.
- Input: `summary.py` junit output plus `map-full`.
- Output: refreshed test counts per package in the port-map.
- Status: partly exists in `map-full/build.py`.

**Not worth a script:** snapshot generation. `tests/oracle_snap/mod.rs` reads the oracle `.snap` files directly.
