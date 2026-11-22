# Recon brief: chat-status (SessionPage status line, footer row, hints; screen map C05)

Recon cut short (orchestrator archived). Sections 1-3 and 5 are verified; 4, 6, 7 are partial. Read-only recon, no cargo run.

## 0. First step for the porter (most important finding)
All five shards were written 2026-11-18 (commits a360fa222, 11792c0a9), BEFORE the main chat screen landed
(001b778b9 / 9f7b6b7e3, 2026-11-20). Many pending reasons are STALE: they say "the chat draws no footer rows /
no status line / no config seed", but ours now has a status line, warning notice, activity row, shortcut panel
and terminal title, and a config seed exists (`{"step":"screen","dir":"chat","ev":{"status":{..StatusFacts..}}}`,
core/src/screens/chat.rs:569-580). Run once, before any code:
`KIT_PROBE_PENDING=1 acargo nextest run --profile oracle -E 'binary(oracle) & test(/^(pane_footer_[12]|chat_status_[123])::/)'`
then `python3 -I tests/oracle/tools/summary.py --list stale` (tests/oracle/tools/summary.py:1-30). Remove the
`kit::pending` of every stale test; rewrite `// seed:` comments of `.live()` tests into `.script(..)` with a chat
`status` screen step (many tests in chat_status_2 are `.live().open()` + a `// seed:` comment only).

## 1. Scope (oracle, codex-rs/tui/src)
- bottom_pane/footer.rs:55-160 FooterProps, CollaborationModeIndicator, GoalStatusIndicator, FooterKeyHints, FooterMode.
- bottom_pane/footer.rs:181-235 mode transitions (toggle_shortcut_mode, esc_hint_mode, reset_mode_after_activity, footer_height).
- bottom_pane/footer.rs:286-543 left side + single-line collapse (`? for shortcuts` / `tab to queue message` / `Plan mode (⇧tab to cycle)`, drop order, context right).
- bottom_pane/footer.rs:544-720 right indicators: mode, goal (`Pursuing goal (40K / 50K)`), status-line-right, side-conversation, right_aligned_x.
- bottom_pane/footer.rs:736-930 passive footer status line, hint items, quit reminder, esc hint, context_window_line (`100% context left`).
- bottom_pane/chat_composer/passive_hints.rs:1-353 whole-hint fitting, second row when an input mode and an action do not fit together.
- bottom_pane/chat_composer/footer_state.rs:43-430 composer footer props/mode/flash (footer flash, hint override).
- bottom_pane/chat_composer/status_surface.rs:1-138 status line rows above hints, status budget.
- bottom_pane/chat_composer/composer_layout.rs:31-70 status budget / FooterRows allocation.
- bottom_pane/chat_composer/warning_notice.rs (109 lines) `⚠ N warnings · /warnings to view`.
- bottom_pane/status_line.rs (187) wrap/measure status line; status_line_style.rs (519) accents; status_line_setup.rs (846) /statusline + /title popups.
- bottom_pane/shortcut_overlay.rs (219) `?` overlay; effort_status_line.rs (279), effort_ignition.rs (254) effort transitions; chat_composer/sparkle.rs (426) sparkle.
- status_indicator_widget.rs (589) Working row (header, timer, interrupt hint, details).
- bottom_pane/pending_input_preview.rs (486) queued messages / pending steers; unified_exec_footer.rs (117); actionable_banner.rs (291).
- chatwidget/status_surfaces.rs:62-1140 item values (723 status_line_value_for_item, 906 terminal_title_value_for_item, 1023 run_state, 1046 title spinner, 317/392 action-required title).
User sees: below the composer a 1-2 row footer (hints left, context/mode/goal right), above it the configured status
line (items joined ` · `), above the composer the Working row and queued previews; the terminal title mirrors items.

## 2. Tests (counts from the shard files; all pending/na/todo now, 0 pass)
pane_footer_1.rs (55): FIND 4 (transcript Find footer); SPARKLE 18; HELP 7 (`?` overlay; ours has a shortcut panel -> probe);
STATUS 19 (status rows, plan indicator, budget, slash popup vs status); WARN 4 (ours has with_warning_notice -> probe);
EFFORT 3. Reason constants at pane_footer_1.rs:27-32.
pane_footer_2.rs (77; 12 n/a, 64 pending, 1 todo): effort prompt/ignition/transition 7 (entries chat/effort_*);
footer states 1 test with 22 entries chat/footer_* + footer_status_line_plan_mode (core of this block);
footer links 3 (katakana/arabic/prepared tree); voice footer 1 + voice strip 2; inline/backend/promoted/information
banners 9; delayed exec approval 4 + approval modal 2 (belongs to approvals); status indicator/status+composer 3;
pending input preview 13 (render_* , steers); unified exec footer 2; esc-with-popup-while-running 5 (2 need keymap
seed -> stays pending: kit seed.rs:117 `missing(..)` for any `.keymap()`); light/limited-colour status line 2;
centered dialog 1; history search ctrl-c 1; rename prompt 1; paste under views 1.
chat_status_1.rs (62; 7 n/a): NOT this block mostly: computer/dynamic activity 6, reasoning status 3, rate-limit
prompts/nudges ~20, pets 6, goal 2, warnings/notices 4. In block: status_line_git_summary_items_render_values,
status_line_branch_changes_render_no_changes, resumed_session_hides_unknown_token_usage..., status_widget_active_snapshot,
streaming_final_answer_keeps_task_running_state, answer_phase_controls_working_status_snapshot, ui_snapshots_small_heights_*.
chat_status_2.rs (60; 2 n/a): status line items 9 (hostname, context used/remaining/legacy, workspace headline,
invalid items warn once, processing tier, reasoning-only); footer snapshots 7 (fast, plan, renamed, speed,
context remaining, goal x2); terminal title 3; thread usage (cost/credits fetch) 4 + /status card 11 (S19, other
block); hooks 15 (other block: cells/hook); streaming headers 6 (transcript block); exec/status layout vt100 3.
chat_status_3.rs (33; 2 n/a): /statusline + /title setup popups 15 (settings porter built the setup sheets in
src/sheets/settings: probe; may be green or need only seeds); status indicator row 7 (renders_with_working_header,
truncated, wrapped details, no spinner, remapped hint [keymap -> pending], hook status, shimmer); terminal title
4 (action required, thread identity, daybreak, goal clock); subagent 2 + image result 1 (other blocks).
Estimate in-block: ~150 of 287; the rest re-tag to their owner block in the gap/pack, do not build here.

## 3. What exists in ours (codex-rs/tui-rtwk)
- src/pages/session/status_surfaces.rs:1-186 StatusContext: items_for (status line), title_parts, value_for (model,
  model-with-reasoning, fast, daybreak, reasoning, current-dir, thread-name/title/id, hostname, profile, provider,
  version, run-state, permissions, context-*, tokens, 5h/weekly limits, task-progress); falls back to StatusFacts::value.
  Missing ids: git-branch summary (pull-request-number, branch-changes), thread-credits, estimated-thread-cost,
  workspace-headline, raw-output, goal; no invalid-item warning.
- src/pages/session/window_title.rs:1-74 WindowTitle (TerminalTitle + AnimatedBuilder spinner SpinnerFrames::DOTS).
- src/widgets/status_line.rs:1-203 status_line() (Text::rich, ` · `, accents, max 3 lines, ellipsis), item_style, Notice +
  with_warning_notice (LayoutBuilder full/compact/count).
- core/src/screens/status.rs:1-271 /status card model (S19) + ThreadPart token usage (project ThreadTokenUsageUpdated)
  + RateLimits; Command/Reply empty (no thread-usage RPC).
- core/src/screens/chat.rs:33-120 Model.status: Rc<StatusFacts> (items, colors, terminal_title, animations, ...); Ev::Status; seed.
- src/pages/session/composer/footer.rs:1-61 FooterLine {Keys, Hint, Close}: Keys = `← for agents · ? for shortcuts`;
  no right side (context %, mode, goal), no collapse, no quit reminder line, no flash.
- src/pages/session/session_page.rs:429-800 build: activity row (activity.rs Activity), queue preview (3 + more), notice,
  shortcut panel, backend control, backend banner (Callout), composer, status line row (2-cell inset), footer, title.
- src/pages/session/activity.rs:1-183 Working row (spinner, shimmer, timer, interrupt hint).
- Debug entries for chat: one only, `streamed_answer` (src/pages/session/mod.rs:43-55).
- Host data missing: git summary (PR number, branch changes), workspace headline, thread usage (cost/credits backend
  call), collaboration mode (Plan), goal status, side-conversation label. Name the backend call per item in the pack.

## 4. Flutter -> rtwk (partial)
- Footer row = Row(Expanded(left hints), right indicator) chosen by LayoutBuilder over the oracle's drop order:
  rtwk LayoutBuilder + Row/Expanded (already used in widgets/status_line.rs:176). Hints from KeyBindings `.hints`
  / widgets/page_keys.rs footer_with. No hand width math beyond choosing the variant (R15).
- Status line wrap = Text::rich max_lines (exists). Links = rtwk Hyperlink spans.
- Working row = existing Activity (AnimationController, Timer).
- Shortcut overlay = existing shortcut_panel (session_page.rs:519).
- Terminal title = rtwk::binding::TerminalTitle (exists).
- Footer flash / quit reminder = State Timer (rtwk::binding::Timer) in the page state.
- TODO (recon cut short): exact rtwk types for two-row passive hints and pending-input preview.

## 5. Dependencies and conflicts
- tm area of chat-status: the 4 files + 5 shards + tests/pages/chat_status.rs + docs/rtwk-port/parity/chat-status.md.
  The work needs, beyond it: src/pages/session/session_page.rs (footer composition), src/pages/session/composer/footer.rs,
  src/pages/session/mod.rs (debug entries) + src/pages/session/fixtures/, core/src/screens/chat.rs (StatusFacts fields:
  collaboration mode, goal, git summary). None of these is in an active claim (checked 2026-11-22 00:45), but
  mod.rs/session_page.rs are shared with the transcript porter's page: widen the area with task_update first.
- Held: settings (src/pages/session/actions.rs, src/sheets/settings: /statusline,/title setup sheets live there);
  chat-transcript (src/pages/session/transcript*); startup-rest (core/src/app.rs, host/src, src/lib.rs).
- App-level / host needs (list in handover, do not edit): host facts for git summary, workspace headline, thread
  usage RPC (a status Command in ThreadFx, ADR-003), Plan collaboration mode and goal into StatusFacts or ThreadPart.
- Keymap seed (kit seed.rs:117) does not exist: every `.keymap()` test stays pending.

## 6. Chunks (partial; TODO refine after the probe run)
1. Probe + footer row (parallel-safe alone): stale probe; FooterMode/right side (context %, Plan mode, goal) and the
   collapse in composer/footer.rs + session_page.rs; chat/footer_* entries (or scripts); turns on footer_snapshots,
   footer_status_line_truncates_to_keep_mode_indicator, chat_status_2 footer snapshots (7), STATUS group part.
2. Status line items + terminal title: status_surfaces.rs ids, invalid-item warning, title action-required/identity;
   chat_status_2 items (9) + title (3), chat_status_3 title (4), chat_status_1 git items (2). Can run parallel to 1
   only if 1 does not touch status_surfaces.rs.
3. Status indicator row + queued/pending preview + unified exec footer: activity.rs, session_page.rs queue part;
   chat_status_3 indicator (6), pane_footer_2 status/preview (~18). After 1 (session_page.rs).
4. Warning notice / `?` overlay / quit + esc hints / flash: pane_footer_1 WARN 4 + HELP 7 (many may be stale-green).
5. Later / other owners: sparkle 18, effort ignition/transition 10, Find footer 4, banners 9, voice 3, setup popups 15 (settings).

## 7. Gaps and risks
- Kit compares one frame: multi-frame snapshots (effort gallery, voice 12 states, centered dialog) stay pending (gap rows exist in reasons).
- FORCE_COLOR colour-depth seed missing in the kit (status_line_theme_colors_at_limited_color_depths).
- Keymap seed missing (kit seed.rs:117).
- TODO (recon cut short): rtwk gap candidates not researched.
