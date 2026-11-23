# Brief: popups (chat pickers in the bottom pane: the Picker brick, /model + reasoning, /permissions, review, worktree, presets, /plan, goal, feedback, plugins, apps; screen map S14 + S23 feedback)

Recon 2026-11-22 (recon-popups), read-only, tip b7e2a0aef; test state from the shard files and
`codex-rs/tui-rtwk/target/nextest/oracle/junit.xml` (full oracle run of 04ff6c716). One codex-porter carries the whole
chunk, steps 1-7 in order. Paths below are under `codex-rs/tui-rtwk/` (ours) and `codex-rs/tui/src/` (oracle).

## 0. First step for the porter
1. Area: claim section 5's list. `tests/oracle/picker.rs` and `chat_popups_4.rs` are in the settings porter's claim; take
   `picker.rs` now if the orchestrator hands it over (settings' only test there is already live), `chat_popups_4.rs` after
   settings lands (section 2 lists whose tests are whose). Until then steps 1-2 write their kit seeds and widgets and run
   the tests by name without editing those files.
2. Probe: `KIT_PROBE_PENDING=1 python3 -I tests/oracle/tools/summary.py --run picker chat_popups_1 chat_popups_2 chat_popups_3 chat_popups_4 --list stale`.
   Stale candidates by reading reasons against the code: `chat_popups_4::feedback_selection_popup_snapshot` (reason: the
   first frame is the composer's slash popup; the composer popup exists now); the 9 picker keymap tests once the keymap
   porter lands the kit `.keymap(..)` seed (tests/kit/seed.rs, keymap's claim). Everything else is truly missing.
3. Stale reasons to rewrite (not stale tests): picker `snapshot_auto_*_col_width_mode_scroll_behavior` say "the kit
   cannot address `before scroll:`": the kit addresses frames (`assert_snap_section`, tests/kit/mod.rs:360-366) but
   `tests/oracle_snap/mod.rs:213-244 header()` has no `label:` kind; one header kind there (a column-0 line ending in `:`
   followed by rows) turns them on (G39b still holds the auto-visible one).
4. Windows: 3 oracle tests are whole-fn `#[cfg(target_os = "windows")]` (oracle chatwidget/tests/permissions.rs:789, :905,
   :989): mark them `kit::na` with that reason. The other Windows-named tests run on every OS under `cfg(test)`
   (windows_sandbox_prompts.rs `#[cfg(any(target_os = "windows", test))]`): port them as Picker data (step 3).
5. `/personality` and `/collab` do not exist at ORACLE_REF: n/a, nothing to build. Collaboration = `/plan`, Shift+Tab and
   the `collaborationMode/list` catalog.

## 1. Scope (oracle `codex-rs/tui/src`)
The user sees a bottom sheet over the composer: title, subtitle, optional search line and tabs, numbered rows with a `›`
cursor, descriptions, a side preview, a hint footer; Enter picks, Esc goes back one level, a pick changes the thread
(model, effort, permissions) or opens the next sheet (reasoning, full-access confirmation, plugin detail).
- Picker brick: bottom_pane/list_selection_view.rs 1-3515 (SelectionViewParams, SelectionItem, tabs, search, side content,
  ColumnWidthMode, accept alternative, footer hints; tests :1800-3515); selection_popup_common.rs 1-1110 (row rendering,
  wrapping, URL rows, fuzzy highlight, semantic spans, selected accent; tests from :700); multi_select_picker.rs 1-1149
  (checkboxes, orderable rows, preview, section breaks); selection_picker_layout.rs 1-54 (`picker_areas` chrome priority);
  selection_row_layout.rs 1-181, selection_row_links.rs 1-183, selection_tabs.rs 1-112, popup_consts.rs 1-60,
  picker_option.rs 1-95, picker_presets.rs 1-32, picker_rows.rs 1-108, picker_style.rs 1-68 (active tab Reset :27,
  palette surfaces :34-48).
- /model: chatwidget/model_popups.rs 1-818 (open_model_popup :18, with_presets :108, all_models :241,
  plan_reasoning_scope_prompt :390, reasoning :516, advanced reasoning :691); model_popup_state.rs 1-144,
  model_command.rs 1-184, session_model_selection.rs 1-105, service_tiers.rs 1-133, reasoning_shortcuts.rs 1-333,
  luna_reserve_model.rs 1-67 (ultra concurrency warning); slash_dispatch.rs:375 (`defer_input_until_settings_applied`).
- /permissions: chatwidget/permission_popups.rs 1-517 (approvals :14, permissions :19, legacy presets :36-183,
  full_access_confirmation :425; auto_review_denials :184-424 is chat-approvals-live's, see section 2);
  permissions_menu.rs 1-342 (profiles popup :148), chatwidget/permission_discovery.rs 1-240 + permission_discovery.rs
  1-183, permission_shortcuts.rs 1-153, windows_sandbox_prompts.rs 1-328; slash_dispatch.rs:415 (suppress queue autosend)
  and :828 (`/permissions <arg>`).
- Other pickers: review_popups.rs 1-159 (slash :364), worktree_picker.rs 1-405 (open_session_checkout_picker :48, slash :318),
  quick_dials.rs 1-637 (open_quick_dials_picker :218, save :225, slash :379 Presets), collaboration_modes.rs 1-103
  (/plan :382), goal_menu.rs 1-143 (/goal :385, Feature::Goals), tui_mode_picker.rs 1-42 (/tui routes to settings: n/a here).
- Feedback: bottom_pane/feedback_view.rs 1-568 (category, consent, note; slash :241), app/feedback_upload.rs 1-135.
- Plugins and apps: chatwidget/plugins.rs 1-1040 (popups :233-407, auth :840), plugin_catalog.rs 1-2059,
  connectors.rs 1-512 (:199, :209).
- Persistence and thread settings: app/thread_settings.rs 1-314, app/config_persistence.rs 1-2202 (the model/effort/
  approval/sandbox/quick-dial writes only).

## 2. Tests (in-chunk 217: 42 pass (24 live + 18 n/a), 175 pending, of which 3 become n/a (Windows))
| shard (owner) | group | n | state | chunk |
|---|---|---|---|---|
| chat_popups_1 (DialogsPane) | bedrock (astra/govcloud/ultrafast) | 3 | P | popups (model) |
| | collaboration_catalog | 1 | P | popups (/plan) |
| | goal_menu | 10 | 8P 2NA | popups (goal) |
| | model_display_name: pickers test | 1 | P | popups (model) |
| | model_display_name: `custom_model_display_name_in_status_line_and_fallback` | 1 | P | chat-status |
| | model_picker (1 keymap, 1 discovery `Loading models…`) | 2 | P | popups (model) |
| | permission_picker | 2 | P | popups (permissions) |
| | permission_shortcuts | 3 | 2P 1NA | popups (permissions) |
| | copy_export | 3 | 1P 2NA | transcript |
| chat_popups_2 (DialogsPane) | permissions.rs (NO_DISCOVERY, NO_PROFILE_POPUP, NO_WINDOWS, NO_FULL_ACCESS, NO_HISTORY, NO_AUTO_REVIEW, NO_CONSTRAINED, NO_QUEUE) | 42 | 1 live (`approvals_selection_popup_snapshot`), 35P, 6NA | popups (permissions) |
| | plugin_catalog (NO_PLUGINS) | 4 | P | popups (plugins) |
| chat_popups_3 (DialogsPane) | plugins (PLUGINS) | 36 | 35P 1NA | popups (plugins) |
| | apps (APPS) | 6 | P | popups (apps) |
| | `hooks_popup_shows_list_diagnostics` | 1 | P | settings (hooks browser) |
| chat_popups_4 (settings holds) | model (snapshot, refresh x6, queued_all_models, hides show_in_picker=false) | 9 | P | popups |
| | reasoning (snapshots, ultra warning, shortcuts, extra-high, single option skip, advanced only, Esc back to model) | 15 | 10P 5NA | popups |
| | session_model_selection 3 + provider_switch 1 | 4 | P | popups |
| | feedback (selection, no destination, upload consent, connectivity diagnostics) | 4 | P | popups |
| | quick_dials 2, review_picker 1, worktree_picker 5 | 8 | P | popups |
| | apps 4 + `account_change_dismisses_the_previous_app_directory_snapshot` | 5 | P | popups |
| | experimental 4 (popup, toggle saves on exit, loading, available), feature_enable_prompts 1, memories 4 (2 live, 2 NA), skills 2 | 11 | 2 live 7P 2NA | settings |
| | security_setup | 5 | P | chat-live (actionable banner, G4) |
| picker (UiPane, settings holds) | live today (list view, search, tabs, side, columns) | 23 | pass | popups |
| | selection_tabs (2 NA pass, 1 P G54) | 3 | 2NA 1P | popups |
| | keymap (printable accept alt, list binding vs search toggle, cancel vs space, C0 Ctrl+P/N x3, page/jump x2, hints follow bindings) | 9 | P (kit `.keymap`) | popups |
| | multi_select_picker | 6 | P | popups |
| | selection_picker_tests (Panel "browser" layout; overflow arrows need a palette seed) | 6 | P | popups |
| | selection_popup_common (semantic spans, selected row, wrapped URLs, halfwidth, hard breaks, fuzzy emoji, exact fit) | 7 | P | popups |
| | selection_row_links 1, side bg 1, side clearing 2, col width scroll 2, narrow halfwidth 1 (G44), description hide 1 | 8 | P | popups |
| | `theme_picker_subtitle_uses_fallback_text_in_94x35_terminal` | 1 | live | settings |
Per shard in-chunk: cp1 22, cp2 46, cp3 42, cp4 45, picker 62 = 217. By step: Picker 36, model 29, permissions 39
(36 after the Windows n/a), generic pickers 9 + feedback 4, goal 8, plugins 39 + apps 11.
Other shards whose widgets this chunk builds (their tests stay there): chat_live_5 review branch/commit pickers (4, chat-live);
`/approve` auto-review denials (chat_approvals_2, chat-approvals-live, built on this chunk's Picker); small_dialogs_1/2
feedback_view 6, feedback_note 12, app_link 14 (settings holds the files; the feedback ones turn green with step 4).
Missing select entries the picker tests name: browser_keymap, shared_menu, multi_twelve, multi_orderable(_two),
multi_fields_twelve, multi_appearance, multi_alpha_jupiter, section_break, semantic_spans, selected_row, wrapped_urls,
wrap_halfwidth, hard_breaks, fuzzy_emoji, exact_fit, description_hide_when_narrow, side_marker(_low), side_content_blue,
toggle_then_next, animations_checkbox, accept_alternative.

## 3. What exists in ours (codex-rs/tui-rtwk)
- core/src/screens/select.rs 1-288: `Menu {Models(Vec<ModelRow>), Permissions{current}, List(Box<List>)}` :43,
  `Access {Ask, Full}` :33 (placeholder), `Picked` :69 + `Model.picked` :83 (a dead end: nobody reads it), intents
  Open/Pick/Choose/Secondary/Toggle/Confirm/Close :100, one menu slot, empty thread hooks ThreadPart/ThreadEv/Command/
  Reply/Ctx/wire/decode :190-235 (the ThreadFx path is there, unused), seed kinds models/list/permissions :267.
- core/src/screens/select/list.rs 1-640: `List`/`Item`/`Tab`/`Columns`/`Side`/`SideWidth`, MAX_ROWS 8, search, settle,
  paging, number shortcuts, paste, flip. Works: the 23 live picker tests.
- src/sheets/select/: mod.rs 1-262 (41 debug entries over fixtures/*.jsonl, `open(cx)` = show_dialog, root navigator false),
  dialog.rs 1-631 (`MenuDialog` StatefulWidget: FocusNode, TextEditingController query, TabController,
  SelectionController<usize>, ListController; reads `states.select.menu` through the provider, emits
  `ScreenIntent::Select`), rows.rs 1-273, side.rs 1-57, lists.rs 1-48, strings.rs 1-84.
- Live path is a stub: core/src/commands/dispatch.rs:333-345 sends Model, Presets, AutoReview, Review, Permissions to
  `Target::Select` and src/pages/session/open.rs opens the sheet, but nobody fills `select.menu` from live data;
  dispatch.rs:365-405 sends Worktree/Plan/Goal/Apps/Plugins to `session(..)` → NotAvailable; :408-470 `/model <arg>`,
  `/permissions <arg>` → SessionIntent; Feedback → `Target::Dialogs` (:346).
- core/src/screens/chat.rs ThreadPart :310-324 has model, effort, provider, service_tier; no approval_policy,
  approvals_reviewer, sandbox_policy, active_permission_profile, collaboration_mode, personality (all in protocol
  `ThreadSettings`, app-server-protocol v2/thread.rs:315-337, projected at chat.rs `project()` :422).
  `screens::project_thread` (core/src/screens/mod.rs:36-44) gives a notification to ONE dir (or_else), so these fields
  belong in chat's ThreadPart, not select's.
- core/src/screens/startup/launch.rs:120-133: a `model/list` request + decoder used for startup migration only; no
  app-scoped catalog. core/src/protocol/onboarding.rs:449: a ConfigBatchWrite builder to reuse.
- Feedback: core/src/screens/dialogs.rs 1-407 (Feedback, FeedbackCategory, Attachments), src/sheets/dialogs/feedback.rs
  1-154 (hand list; the only file in R15_NOT_YET, tests/rules/rules.rs:624-720).
- Settings sheets (theme.rs, setup.rs, features.rs, memories.rs) hand-build their own lists beside MenuDialog: not this
  chunk; they move onto the Picker after settings lands (note for that owner).
- src/sheets/presenter.rs 1-159: ADR-002 request presenter (approvals): `show_dialog_from` the Navigator home's kept
  `ViewRoot`, next request decided on the model; called from src/app/mod.rs:158. The pattern for the menu stack.
Missing protocol (app-server-protocol/src/protocol/common.rs) and where it enters:
- thread (ThreadFx, the select dir's Command/wire/decode): `thread/settings/update` :734 (params v2/thread.rs:239-283);
  `thread/goal/get|set|clear` :651-661; `review/start` :1179 (if chat-live has not added it).
- app (AppFx in core/src/app.rs `AppCtx` :339 + host): `model/list` :1185; `permissionProfile/list` :1225;
  `collaborationMode/list` :1279; `configRequirements/read` :1571; `config/batchWrite` :1564; `feedback/upload` :1467,
  `feedback/save` :1473; `account/read` (GetAccount) :1577; `plugin/list|search|installed|read|install|uninstall`
  :980-1108, `marketplace/add|remove|upgrade` :965-975; `app/list|read|installed` :1036-1046.
- notifications: `thread/settings/updated` :2081 (chat projects it), `app/list/updated` :2124 (new app-scoped Ev for select).
- host facts (host/src/facts.rs): auto-review feature + reviewer availability, Windows sandbox requirement level,
  managed config requirements, feature flags (Goals, plugins, apps), feedback upload destination / cloud thread.

## 4. Flutter -> rtwk
| piece | Flutter | rtwk (found) |
|---|---|---|
| sheet over composer | showModalBottomSheet | `show_dialog` rtwk-overlay/src/dialog.rs:368, `Dialog::bare` :822, `AppCx::show_dialog` rtwk/src/app/dialog.rs:174; ours `sheet_options()` |
| sheet the app opens (stack head) | BlocListener + showModalBottomSheet | `AppCx::show_dialog_from` + kept `ViewRoot` (src/sheets/presenter.rs pattern); `close_dialog` rtwk-overlay/src/dialog.rs:443 |
| list with cursor | ListView.builder + selection | `ListController` rtwk-scroll/src/list_view.rs:89, `SelectionController` rtwk-data/src/selection_controller.rs:72; ours widgets/session_list.rs (fleet "one list") |
| numbered options | RadioListTile list | ours src/widgets/options.rs (Column(min)[Flexible(SingleChildScrollView(Grid))], G39 closed) |
| search line | TextField + TextEditingController | `TextEditingController` rtwk-editing/src/text_editing.rs:418 |
| fuzzy highlight | custom TextSpan | `fuzzy_match` rtwk-widgets/src/text/fuzzy.rs:152 |
| tabs | TabBar + TabController | `TabController` rtwk-controls/src/tabs.rs:63; ours widgets/tab_strip.rs (G45 composition) |
| checkboxes (multi select) | CheckboxListTile | `Checkbox` rtwk-controls/src/toggle.rs:64 (`MultiSelect` rtwk-controls/src/select.rs:393 for reference) |
| reorder rows | ReorderableListView | none in rtwk; key reorder is a data op on core `List` (not a gap: the oracle reorders by keys only) |
| layout slots by priority (Panel browser) | LayoutBuilder | `LayoutBuilder` rtwk-framework/src/layout_builder.rs:56 |
| narrow: hide description | LayoutBuilder | same |
| wrapped URL rows | Text.rich + link | `Span::link` rtwk-widgets/src/style/style.rs:413 (`Style::link` :147) |
| width of halfwidth marks | TextPainter | `segmenter_of` rtwk-widgets/src/text/segmenter.rs:34 (G44 open) |
| key bindings | Shortcuts/Actions | `KeyBindings`/`Shortcuts` rtwk-framework/src/focus/shortcuts.rs:1257/:1448 + ours src/keymap (keymap porter) |
| loading row | CircularProgressIndicator | `Spinner` rtwk-animation/src/spinner.rs:47 |
| terminal default colours (palette surfaces) | Theme from platform brightness | rtwk-core terminal/probe.rs `background_rgb` (:415); kit has no seed for it (kit gap, not rtwk) |
Gap rows (existing, none new for rtwk): G39b auto-visible column over the window; G44 halfwidth sound mark; G54 Reset
style under the active tab; G49 no break after `/` (picker rows with paths); G50/G51/G53 Column flex priority and ListView
shrink-wrap (risk for the Panel layout). Kit rows (tests/kit, tests/oracle_snap): palette seed; `label:` frame header;
pre-painted background for side-clearing.

## 5. Area and conflicts
Claim (tm area): `src/sheets/select/**`, `src/widgets/picker/**` (new), `src/sheets/dialogs/feedback.rs`,
`core/src/screens/select.rs`, `core/src/screens/select/**`, `core/src/screens/dialogs.rs`, `core/tests/select.rs`,
`core/tests/models.rs` (new; `core/tests/model.rs` is the app model), `core/tests/permissions.rs` (new),
`tests/sheets/select.rs`, `tests/sheets/pickers.rs` (new), `tests/oracle/chat_popups_1.rs`, `chat_popups_2.rs`,
`chat_popups_3.rs`, `docs/rtwk-port/parity/popups.md` (new), `docs/rtwk-port/parity/select-dialog.md`; by handover
`tests/oracle/picker.rs` (now) and `tests/oracle/chat_popups_4.rs` (after settings).
Layout in the dir: grow `select` with submodules, no new screen dirs (no `screen_list!` change): core
`select/{stack,models,permissions,review,worktree,quick_dials,collaboration,goal,plugins,apps}.rs`, UI the same names
under src/sheets/select/. The Picker widget itself: `src/widgets/picker/` (props-driven, used by select, dialogs/feedback,
later settings and approvals).
Shared files, minimal change each (orchestrator sequences or gives to one owner):
- core/src/screens/mod.rs (unclaimed): `ScreenThreadIntent::Select(select::ThreadIntent)` + its arm in
  `apply_thread_intent` :22-34.
- core/src/screens/chat.rs (chat-live holds): ThreadPart fields approval_policy, approvals_reviewer, sandbox_policy,
  active_permission_profile, collaboration_mode, personality from ThreadStarted/ThreadSettingsUpdated in `project()`
  :422. If chat-status adds collaboration_mode for the footer, one field, one writer.
- core/src/app.rs (keymap holds): AppCtx variants + routing for the app RPCs of section 3 and the `app/list/updated`
  notification into `select::Ev`; `config/batchWrite` for persisted picks (skipped for session-only picks).
- host/src/facts.rs (keymap holds): the host facts of section 3.
- core/src/commands/dispatch.rs (unclaimed) :333-470: Model/Permissions with and without args, Plan, Goal, Presets,
  Worktree, Review, Plugins, Apps → `Target::Select` openers carrying the thread id (R14) instead of SessionIntent.
- src/pages/session/open.rs (unclaimed), src/pages/session/actions.rs:135-160 (settings holds): open by menu kind.
- src/app/mod.rs:158 (unclaimed): call the select presenter next to `presenter::present`.
- core/src/threads.rs (unclaimed): a local note item for "Permissions updated to …" / "Model changed to …" history
  cells, shared with chat-commands (`Item::Local`); whoever lands first adds it.
- src/app/debug/mod.rs, src/sheets/mod.rs: registration of the new entries/modules.
- tests/kit (keymap holds seed.rs): `.keymap(..)` (keymap porter), a palette seed (`Screen::palette(fg, bg)` over the
  test terminal's default colours); tests/oracle_snap/mod.rs:213 `label:` header kind.
- docs/rtwk-port/rtwk-gaps.md (transcript holds): no new row needed.
Needs the porter must not do itself: the app RPC routing and host facts above (app.rs/facts.rs owner), the queue
behaviours `/model` defers queued input (slash_dispatch.rs:375) and `/permissions` suppresses autosend (:415)
(chat-live queue owner; this chunk only emits the intent), the chat ThreadPart fields (chat-live).

## 6. The plan (one job, tests first in each step)
1. Picker brick (picker.rs 36). Move MenuDialog's view into `src/widgets/picker/` as `Picker` taking a `List` as props
   + callbacks (R14 allows props; controllers stay in its State); MenuDialog becomes the seeded adapter. Add to core
   `List`: multi select (checkbox rows, orderable by keys, preview, `section_break_after`), Panel surface "browser"
   layout (LayoutBuilder slots per selection_picker_layout.rs:1-54), typed span roles → theme tokens (semantic spans,
   selected accent), fuzzy highlight, hard breaks, link spans, narrow description hide, side background. Write the 21
   missing select entries first (section 2 list). Oracle: list_selection_view.rs tests :1800-3515,
   selection_popup_common.rs :700-1110, multi_select_picker.rs, picker_style.rs. Keymap 9 wait for `.keymap`; 2 col
   width + overflow arrows wait for the kit rows; side clearing 2 per decision 4.
2. Menu stack + /model family (29). Core `select::Model { stack: Vec<Menu>, catalog, .. }`; `Menu` variants per picker
   with typed data (no words, ADR-010); stack changed only by apply (pick model → reasoning, single option skip,
   advanced only, Esc pops, refresh keeps highlight / dismisses obsolete or empty, `show_in_picker=false` hidden,
   service tier controls, plan reasoning scope prompt, ultra warning). Select presenter (sheets/presenter.rs pattern)
   shows one Picker route per stack head; results return as intents. Thread pick → `select::ThreadIntent` →
   Command `thread/settings/update` (select wire/decode); persist via app `config/batchWrite`. Catalog from app
   `model/list` (seed step `{"step":"screen","dir":"select",..}` in the kit). Bedrock 3 need a provider seed in the
   catalog Ev. Oracle: model_popups.rs :18-818, model_popup_state.rs, session_model_selection.rs, service_tiers.rs,
   reasoning_shortcuts.rs, luna_reserve_model.rs.
3. /permissions (39, 36 after n/a). Legacy presets, profiles popup (`permissionProfile/list`), discovery loading/stale
   results, full-access confirmation (a stack push), constrained/managed requirements, shortcuts, history cells (local
   note), Windows sandbox prompts as Picker data. Reads the thread's policy from chat ThreadPart by id. Oracle:
   permission_popups.rs :14-183 :425-517, permissions_menu.rs, permission_discovery*.rs, permission_shortcuts.rs,
   windows_sandbox_prompts.rs.
4. Generic pickers + feedback (13). Review presets (`review/start`), worktree checkout picker, quick dials (multi
   select + save), `/plan` collaboration catalog (`collaborationMode/list`); feedback category → consent → note on the
   Picker in the dialogs dir (`feedback/upload|save`, no-destination stop, connectivity diagnostics filename), which
   takes feedback.rs out of R15_NOT_YET. Oracle: review_popups.rs, worktree_picker.rs, quick_dials.rs,
   collaboration_modes.rs, feedback_view.rs, app/feedback_upload.rs.
5. Goal menu (8). `thread/goal/get|set|clear` through ThreadFx; reads goal state from chat-live's projection if it
   exists, else the get reply in select's ThreadPart. Oracle: goal_menu.rs.
6. Plugins (39) and apps (11). App-scoped catalog in select::Model (plugin/marketplace/app RPCs, `app/list/updated`),
   detail and auth sheets as stack pushes, account change dismisses the app directory. Oracle: plugins.rs :233-407
   :840, plugin_catalog.rs, connectors.rs.
7. Handover sweep: once chat_popups_4.rs is in the area, un-pend its 45; parity/popups.md lists every test, n/a reason
   and remaining gap row; the settings sheets' move onto Picker is left as a note for the settings owner.

## 7. Gaps and risks
- G39b, G44, G54 keep 3 picker tests pending; G50/G51/G53 may bite the Panel slot layout (fallback: LayoutBuilder
  computes heights, no hand width math, R15).
- Kit rows (palette seed, `label:` header) sit in keymap-held / unclaimed test infra: 3 tests wait on them.
- The 9 keymap tests wait on the keymap landing.
- chat.rs (chat-live) and app.rs/facts.rs (keymap) changes gate steps 2-6; step 1 needs none of them.
- Risk: two writers of collaboration_mode (chat-status footer vs this chunk): one field in chat ThreadPart.
- Risk: `select::Model.picked` and `Menu::Models/Permissions` are replaced, not kept beside the stack (one source).
Decisions for the person:
1. ui-pane ("Picker twin of ListSelectionView") is stale: S14 built it in substance. Recommend: this chunk builds the
   props-driven Picker as step 1 and ui-pane is closed as done by S14 + this step (its other bricks are other chunks').
2. Plugins + apps (50 tests): recommend the same porter as steps 6 (chat_popups_2/3 are shared with permissions).
3. Windows-only prompts: recommend port as Picker data; the 3 `cfg(windows)` tests n/a.
4. Side-content clearing (2 picker tests prove ratatui's stale-cell clearing at a buffer offset): recommend n/a, rtwk
   paints the dialog's whole region; else a kit pre-paint option.
5. Settings sheets onto the Picker: recommend after settings lands, by the settings owner, not this chunk.
