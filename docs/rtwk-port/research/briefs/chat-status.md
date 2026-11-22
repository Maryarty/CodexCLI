# Brief: chat-status (SessionPage bottom pane: status line, footer, hints, Working row, previews, title; screen map C05)

Recon 2026-11-22 (recon-chat-status), read-only, tip b7e2a0aef; test state from the shard files and
`codex-rs/tui-rtwk/target/nextest/oracle/junit.xml`. One codex-porter carries the whole chunk, steps 1-8 in order.

## 0. First step for the porter
1. Wait for, or rebase onto, the landings this chunk builds on (section 5): chat-live C1 (it carves `session_page.rs`
   into `activity.rs`, `pending_input.rs`, `banners.rs`) and transcript tier 1 (it holds `composer/footer.rs`). Then
   widen the area with `task_update` (section 5 list) before the first edit.
2. Probe the stale markers, nothing else built yet:
   `KIT_PROBE_PENDING=1 python3 -I tests/oracle/tools/summary.py --run pane_footer_1 pane_footer_2 chat_status_1 chat_status_2 chat_status_3 --list stale`.
   Candidates by reading the reasons against the code: `pane_footer_2::ctrl_c_cancels_history_search_without_clearing_draft_or_showing_quit_hint`
   (Ctrl+R history search exists now: session_page.rs `search_key`); `pane_footer_1::input_history_search_uses_the_final_row`
   (its body has no assertion: add `assert_cursor(x, 5)` before un-pending, oracle status_surface_tests.rs:1098);
   any test whose snapshot is the pending input preview if C1 already landed the oracle layout of `pending_input.rs`
   (chat-live brief line 81 folds it into C1). Everything else is truly missing: most bodies are `.live().open()` with a
   `// seed:` comment and cannot pass until rewritten.
3. Stale reasons to rewrite while you go (not stale tests): "the kit compares one frame" is wrong: `Screen::snap(name).frame(header)`
   and `assert_snap_section` compare one frame of a multi-frame snapshot (tests/kit/mod.rs:341-373, 553). The FORCE_COLOR
   reason needs only a kit option over `rtwk_test::Options::env` (rtwk-test/src/tester.rs:181), not an rtwk change.
4. Seeds: footer-alone snapshots (`footer_*`, 80x1) become entries that open the real chat page at the snapshot width
   and a height that holds the composer; the test compares the footer row with `s.snap(name).at(row, 0).assert()`.
   UI-only modes (quit reminder, Esc hint, `?` overlay) come from `{"step":"key"}` lines in the fixture
   (core/src/fixtures/script.rs:36-57), model facts from `{"step":"screen","dir":"chat","ev":{"status":{..}}}`
   (core/src/screens/chat.rs `seed`, StatusFacts deserialises as is) and thread notifications. Never hand props to a widget.

## 1. Scope (oracle `codex-rs/tui/src`)
- bottom_pane/footer.rs:55-235 FooterProps, CollaborationModeIndicator (`Plan mode (⇧tab to cycle)`), GoalStatusIndicator,
  FooterMode {HistorySearch, QuitShortcutReminder, ShortcutOverlay, EscHint, ComposerEmpty, ComposerHasDraft},
  toggle/esc/reset transitions, footer_height; :286-543 left side and single-line collapse (`? for shortcuts`,
  `tab to queue message`, drop order, context right); :544-720 right indicators (mode, goal `Pursuing goal (40K / 50K)`,
  status-line-right, side conversation, agent label); :736-930 quit reminder `ctrl + c again to quit`, Esc hint
  `esc again to edit previous message`, context `N% context left`; QUIT_SHORTCUT_TIMEOUT 1 s (bottom_pane/mod.rs:248).
- bottom_pane/chat_composer/passive_hints.rs:1-353 whole-hint fitting: label forms, collapse order, a second row only when an
  input mode (Shell, Vim, Plan) and an action (queue, side) fit apart but not together; footer_state.rs:43-430 (flash, hint
  override); status_surface.rs:1-138 (status rows above hints, budget, `?` help grows above input); composer_layout.rs:31-70
  (FooterRows budget); warning_notice.rs (109) `⚠ N warnings · /warnings to view`; sparkle.rs (426) + sparkle_field.rs (70).
- bottom_pane/status_line.rs (187) wrap/measure; status_line_style.rs (519) accents, light-theme correction, colour depths;
  footer/footer_content.rs (135) linked footer content; shortcut_overlay.rs (219) `?`; effort_status_line.rs (279),
  effort_ignition.rs (254) (`U L T R A`, prompt accent); pending_input_preview.rs (486); unified_exec_footer.rs (117);
  hook_status.rs (32); voice_strip.rs (203) and footer.rs voice item (:890).
- status_indicator_widget.rs (589) Working row: header, `fmt_elapsed_compact`, interrupt hint, wrapped details (max lines),
  inline message, no-spinner, shimmer/progress switches.
- chatwidget/status_surfaces.rs:62-1297 item selection, invalid-item warning (:140-160), git/headline/thread-usage requests
  (:163-201, :605-700), values (:723-848), terminal title (:906+, run state :1023, spinner :1046, action required :317/:392);
  chatwidget/goal_status.rs (228); terminal_title.rs (284); branch_summary.rs (755, git + gh probes).
User sees: under the composer a 1-2 row footer (hints and modes left, context/goal/mode right, transient quit/Esc/flash lines),
above it the configured status line (items joined ` · `, links, up to the budget); above the composer the Working row, the
background-terminal summary and the queued/steer preview; the window title mirrors the title items with a spinner.

## 2. Tests (278 in the five shards: 0 pass, 19 n/a, 259 pending)
In chunk: 138 (6 n/a, 132 to turn on, 0 green today).
- pane_footer_1 (55, all pending): in 51 = STATUS 19, HELP 5, sparkle 19 (2 of them HELP_REASON), WARN 4, EFFORT 3,
  `input_history_search_uses_the_final_row` 1. Other: Find footer 4 (footer_state_tests 62/110/134,
  `transcript_search_keeps_status_and_query_cursor`) -> transcript.
- pane_footer_2 (74: 11 n/a, 63 pending): in 44 (5 n/a) = effort ignition/status-line 9 (3 n/a), footer.rs 3
  (`footer_snapshots` = 23 snaps, `footer_status_line_truncates_to_keep_mode_indicator`, voice footer), footer links 3,
  pending input preview 13 (1 n/a), status/queue stack 7 (mod.rs 3574-3806), unified exec 3 (1 n/a), status_line_style 2,
  voice strip 2, `escape_closes_shortcut_help_before_inline_banner`, `ctrl_c_cancels_history_search_...`.
  Other: approvals 6 (`flush_buffered_typing...`, `overlay_not_shown_above_approval_modal`, 3 delayed-approval,
  `composer_shown_after_denied...`) -> chat-approvals-live; banners 8 (`inline_banner_snapshot`, `backend_banner_snapshots...`,
  actionable_banner_tests 6) + `remapped_interrupt_turn...` -> chat-live (C3 / interrupt); `apply_text_suggestion...`,
  `selection_view_esc_respects_remapped_list_cancel`, `centered_dialog...` 3 -> popups; `remote_images...`, Esc-with-popup 4,
  `paste_completion...` 6 -> composer; mod.rs n/a 6 stay n/a.
- chat_status_1 (58: 6 n/a): in 7 = `resumed_session_hides_unknown_token_usage...`, git items 2, `raw_output_mode...`,
  `status_widget_active_snapshot`, small heights 2. Other: chat-live 34 (rate-limit/workspace prompts 19 incl. 1 n/a,
  reasoning status 2, cyber/daybreak/model-verification notices 3, Working lifecycle 3 `streaming_final_answer...`,
  `answer_phase...`, `final_answer_completion_restores...`, goal interrupt 4 incl. 3 n/a, `warning_event...`, program_status n/a,
  flush_answer n/a); cells-markdown 9 (computer/dynamic activity 7, voice transcript, plan table); pets 6 (S24, not in batch);
  `completed_reasoning_stays_in_expanded_transcript...` -> transcript; `status_widget_and_approval_modal_snapshot` -> approvals.
- chat_status_2 (59: 1 n/a): in 23 = items 11 (invalid warn once, hostname, context x3, cost, credits, workspace headline,
  processing tier, reasoning-only, title thread usage), footer snapshots 9 (cost, fast, plan, renamed, speed, context, goal x2,
  `goal_status_indicator_line_formats_goal_text`), `terminal_title_model_updates...`, vt100 exec/status layout 2
  (`chatwidget_exec_and_status_layout_vt100_snapshot`, `chatwidget_tall`). Other: /status card 12 (1 n/a) -> usage-status;
  hooks 14 -> cells-markdown; streaming headers + runtime metrics 7, 2 warning cells -> chat-live;
  `chatwidget_markdown_code_blocks_vt100_snapshot` -> cells-markdown.
- chat_status_3 (32: 1 n/a): in 13 (1 n/a) = status indicator 7, terminal title 4, `session_status_follows_existing_lifecycle_state`
  (background terminal line), `session_status_detail...` n/a. Other: /statusline + /title setup previews 16 -> settings
  (sheets in src/sheets/settings/setup.rs); subagent 2 + image result 1 -> cells-markdown.
- Keymap-bound in chunk (wait on the keymap seed, tests/kit/seed.rs:111-123): `activity_shortcut_help_follows_runtime_binding`,
  `shortcut_help_close_row_uses_runtime_binding...`, `render_one_pending_steer_with_remapped_interrupt_binding`,
  `renders_remapped_interrupt_hint`, `voice_controls_follow_configured_mute_binding...` (5).

## 3. What exists in ours (codex-rs/tui-rtwk)
- src/pages/session/session_page.rs:429-802 `ChatState::build`: status slot (:468-516, `Activity` keyed by thread+turn),
  `shortcut_panel` (:517, :640), history search footer (:520-547), composer + `FooterLine` choice (:552-571:
  Close / backtrack hint / queue hint / Keys), shell mode label, `with_warning_notice` (:575-594, key `f2`, oracle `/warnings`),
  queue preview 3 + `more queued` (:603-615, not the oracle layout), notice, backend control, banner `Callout` (:678-704),
  status line row with a 2-cell inset (:715-721), title (:723-748). Ctrl+C (:300-307) interrupts or quits at once: no
  quit reminder; Esc primes backtrack (:374-389). Missing: FooterMode, flash, right side, two rows, collapse.
- src/pages/session/composer/footer.rs:1-61 `FooterLine {Keys, Hint, Close}` over `page_keys::footer_with` (= rtwk
  `KeyHintsBar`, src/widgets/page_keys.rs:39); no context %, mode, goal, side/agent label, no quit/Esc line, no flash.
- src/widgets/status_line.rs:1-203 `status_line()` (Text::rich, ` · `, accents, max 3 lines, ellipsis), `item_style`,
  `Notice` + `with_warning_notice` (LayoutBuilder full/compact/count, :160-203): the pattern every right-side indicator follows.
- src/pages/session/status_surfaces.rs:1-186 `StatusContext::{items_for, title_parts, value_for}`: model(-with-reasoning),
  fast-mode, daybreak, reasoning, current-dir, thread-name/title/id, hostname, profile, provider, codex-version, run-state,
  permissions, context-remaining/used/window, used/in/out tokens, 5h/weekly limits, task-progress. Missing ids (oracle
  status_line_setup.rs:59-160): project-root, git-branch (fact exists: StatusFacts.branch), pull-request-number (with link),
  branch-changes, approval-mode, thread-credits, estimated-thread-cost, raw-output, workspace-headline; no invalid-item warning.
- src/pages/session/window_title.rs:1-74 `WindowTitle` (TerminalTitle + AnimatedBuilder, SpinnerFrames::DOTS).
- src/pages/session/activity.rs:1-183 `Activity` (header, shimmer AnimationController, Timer, offset, step); missing
  details wrap/max lines, inline message, hook status line, interrupt-binding label, hidden-row timer.
- src/sheets/dialogs/mod.rs:25 `shortcut_panel` + shortcuts.rs (197): the `?` overlay body and close hint; entry `shortcut_overlay`.
- core/src/screens/chat.rs:64-95 `StatusFacts` (no git summary, headline, raw output, collaboration mode); :309-323 `ThreadPart`
  (no goal); `seed` reads Flags/ServiceTiers/SignedIn/Status. core/src/screens/status.rs ThreadPart = token usage.
- Debug entries `chat` dir: only `streamed_answer` (src/pages/session/mod.rs:43-55); fixtures dir src/pages/session/fixtures/.
- Host / protocol data missing, per item (grep `codex-rs/app-server-protocol/src/protocol/common.rs`):
  - pull-request-number, branch-changes: no protocol; oracle runs `git` + `gh pr view` (branch_summary.rs:142-520) in the
    workspace -> host fact next to `branch` in host/src/facts.rs:77 (`codex_git_utils`), refreshed on cwd change.
  - thread-credits, estimated-thread-cost (also /status card): `account/usage/read` = `ClientRequest::GetAccountTokenUsage`
    {thread_id} (common.rs:1449; oracle app/background_requests.rs:876, polled by chatwidget/thread_usage.rs). Ours has the
    wire only for the agents page (core/src/protocol/agents.rs:90 `Command::Usage`).
  - workspace-headline: `account/workspaceMessages/read` = `GetWorkspaceMessages` (common.rs:1455; oracle :914, due-time throttle
    status_surfaces.rs:640-700). Not wired in ours.
  - goal indicator + title goal clock: `thread/goal/get` (common.rs:656) on open, `thread/goal/updated` / `thread/goal/cleared`
    notifications (common.rs:2066-2069). Not wired.
  - Plan mode indicator: `collaborationMode/list` (common.rs:1279) + local Shift+Tab cycling (chat-live C8). Not wired.
  - context %, tokens: `thread/tokenUsage/updated` (exists: status.rs ThreadPart). Rate limits: `account/rateLimits/updated` (exists).
  - unified exec footer: `item/started` CommandExecution with `source = unifiedExecStartup` and `process_id`
    (v2/item.rs:397-399; oracle command_lifecycle.rs:25-45, 170-262); ours drops both in core/src/threads/activity.rs:91-117.
  - hostname, profile, provider, version, cwd, branch, animations: host/src/facts.rs:38-81 (exist).

## 4. Flutter -> rtwk
- Footer as a whole: a StatelessWidget `ComposerFooter(props)` under the composer, its mode in the page `State` (Flutter: an
  enum field + `Timer` for transient modes). rtwk: `StatefulWidget`/`State` (rtwk::), `rtwk::binding::Timer` (activity.rs uses it),
  props compared for `Rebuild::No`. Mode is UI state (R14), never core.
- One hint row with drop order: `LayoutBuilder` choosing spellings (G41: not a gap) + `Row`/`Expanded`/`SizedBox`
  (`rtwk::layout`), labels as `Text::rich_line(spans)` (`rtwk::widgets::Text`, `rtwk::style::Span`); widths by
  `rtwk::text::segmenter_of(cx)` / `rtwk_core::unicode::string_width` (as status_line.rs:155). Key labels from the page's
  `KeyBindings` hints through `rtwk::controls::KeyHintsBar::entries` (rtwk-controls/src/key_hints.rs:347) so every key shown is bound.
- Two-row passive hints (exact types): `LayoutBuilder` -> `Column` (`rtwk::layout::Column`, `MainAxisSize::Min`) of one or two
  `Row`s, each `Row[ Text::rich_line(left labels), Expanded(SizedBox), Text::rich_line(right labels), SizedBox(1) ]`; the
  row split and chosen forms come from a pure fit fn over measured label widths (the oracle's Kind/forms/collapse order,
  passive_hints.rs:15-353, carried with a comment naming the file). Not `rtwk::layout::Wrap` (rtwk-widgets/src/layout/wrap.rs:25):
  Wrap flows by width, the oracle assigns rows by kind (modes above, actions below).
- Status line rows: `Text::rich` + `max_lines` + `TextOverflow::Ellipsis` (exists), links `Span::link` / `Style::link`
  (rtwk-widgets/src/style/style.rs:147, :413); row budget by `LayoutBuilder` + `crate::widgets::measure::rows` (G38: not a gap).
- Right indicators (context, mode, goal, side, agent, warning): same `LayoutBuilder` spelling choice as `with_warning_notice`.
- Quit reminder / Esc hint / footer flash: `State` field + `rtwk::binding::Timer` (1 s quit, flash duration from the caller).
- `?` overlay: existing `shortcut_panel` (src/sheets/dialogs/mod.rs:25) placed above the composer in the page `Column`.
- Working row: existing `Activity` (`rtwk::animation::{AnimationController, AnimatedBuilder, Spinner}`, `Timer`); details as
  `Text` max_lines + ellipsis; hook status as one more `Text` row.
- Pending input preview (exact types): `Column` of: header `Row[Text("• "), Expanded(Text::rich(header spans))]` (2-cell hang),
  per message `Row[Text("  ↳ ").style(dim), Expanded(Column[Text(message).max_lines(Some(3)).overflow(TextOverflow::Clip),
  Text("…") when measure::rows(seg, message, width-4) > 3])]`, the edit hint `Text::rich_line` (4-cell indent, the edit binding's spans, ` edit last queued message`), blank `SizedBox(None, Some(1))` between sections; URLs as `Span::link`. All `rtwk::layout` / `rtwk::widgets::Text`.
  Flutter: `Column` + `Row(leading, Expanded(Text(maxLines)))` + `TextPainter.didExceedMaxLines`.
- Unified exec footer: one `Text::rich_line` with ellipsis, two cells in (`N background terminals running · /ps to view · /stop to close`).
- Effort status-line transition and ignition: `AnimationController` + `rtwk::animation::Interval` per phase
  (scroll-out 620 ms, assemble 700, hold 360, fade 340, fade-in 480; effort_status_line.rs:34-45), `rtwk::animation::ColorTween` for colours,
  `AnimatedBuilder` repainting spans. Flutter: staggered animation with Interval.
- Sparkle: `Stack` over the composer + `rtwk::layout::{CustomPaint, CellPainter}` painting braille dots into blank cells (the
  welcome blossom does the same: src/pages/session/blossom/mod.rs) + `AnimationController` (150 ms tick, 15 s idle, 1 s fade);
  eligibility in a small controller fed by the composer's change events.
- Terminal title: existing `rtwk::binding::TerminalTitle` + `WindowTitle`.
- Limited colour depth: rtwk downgrades by `ColorMode` (rtwk-core/src/style/table.rs:564); the kit needs `Options::env`.
- No new rtwk gap found. Existing rows that apply: G44 (half-width katakana grapheme), G48/G49 (URL-like preview wraps).

## 5. Area and conflicts
Porter's area (claim with task_update, all unclaimed today unless noted):
- new `src/pages/session/footer/` (mod.rs `ComposerFooter`, hints.rs fit + widget, indicators.rs, status_rows.rs, effort.rs,
  sparkle.rs, unified_exec.rs, entries.rs, strings.rs, fixtures/); `src/pages/session/status_surfaces.rs`,
  `src/pages/session/window_title.rs`, `src/widgets/status_line.rs`, `src/sheets/dialogs/shortcuts.rs`;
- tests: `tests/oracle/{pane_footer_1,pane_footer_2,chat_status_1,chat_status_2,chat_status_3}.rs`,
  `tests/oracle/fixtures/{pane_footer_1,pane_footer_2,chat_status_1,chat_status_2,chat_status_3}/`, new
  `tests/pages/chat_status.rs`, `docs/rtwk-port/parity/chat-status.md`;
- after chat-live C1 lands (handover of its area): `src/pages/session/activity.rs`, `src/pages/session/pending_input.rs`.
Shared files, minimal change (orchestrator sequences or gives to one owner):
- `src/pages/session/session_page.rs` (chat-live): replace :552-594 + :715-721 by one `footer::ComposerFooter` + status rows;
  Ctrl+C (:300-307) idle -> quit-reminder mode, second press within 1 s quits; Esc idle -> EscHint mode; mount the unified
  exec line and the preview slot order of the oracle (bottom_pane/mod.rs:2483-2540).
- `src/pages/session/mod.rs` (chat-live): `mod footer;` and `debug_entries()` chains `footer::entries::debug_entries()` (2 lines).
- `src/pages/session/composer/footer.rs` (transcript): retire into `footer/`; the transcript's `enter/esc latest` hints become an
  Action label of `footer::hints`. After transcript tier 1.
- `tests/screens.rs` (transcript): one `#[path = "pages/chat_status.rs"] mod chat_status;`.
- `src/pages/session/composer/mod.rs` (composer chunk): prompt glyph/accent input for Max/Ultra effort; the sparkle's change
  events (typed, paste, history accept/cancel, attachment) and its placeholder rect.
- `core/src/screens/chat.rs` (chat-live): StatusFacts += `git: Option<GitSummary{pr_number, pr_url, changes}>`,
  `workspace_headline`, `raw_output`; ThreadPart += `goal` projected from `thread/goal/updated|cleared`.
Needs (app / host / kit, the porter does not do them):
- host/src/facts.rs + core/src/app.rs (keymap porter now): git summary probe (carry branch_summary.rs, REUSE), workspace
  headline request + throttle (`account/workspaceMessages/read`), thread-usage polling.
- core/src/screens/status.rs (status-card porter): a per-thread `Usage` command (`account/usage/read`, issued like chat's
  `Command::Compact` through ThreadFx wiring) whose reply lands in `status::ThreadPart`; /status card and status line read it.
- core/src/threads/activity.rs (unclaimed; cells-markdown may touch): keep `process_id` + unified-exec source on the tool item
  so the footer list is a selector over the thread, no second copy (ADR-003).
- chat-live C8: collaboration mode (Plan) state + Shift+Tab; the footer only renders the indicator.
- tests/kit/seed.rs (keymap porter): keymap seed (5 tests above) and a colour-depth option (`Options::env("FORCE_COLOR", ..)`).

## 6. The plan (one job, steps in order, tests first in each step)
1. Footer row and modes (15 tests). Files: footer/{mod,hints,indicators,entries,strings}.rs, session_page.rs swap, mod.rs chain.
   Read oracle footer.rs:160-930, passive_hints.rs, footer_state.rs, warning_notice.rs. Build FooterMode in page State, quit
   reminder + Esc hint timers, flash, left labels (agents, shortcuts, queue, side, agent name), right (shell/vim/plan mode,
   goal, context, warning), pure fit fn with unit tests, one/two rows. Turns on: `footer_snapshots` (23 entries `chat/footer_*`),
   `footer_status_line_truncates_to_keep_mode_indicator`, `ctrl_c_cancels_history_search...`, pane_footer_1 174/651/683/771/830/
   852/1031 and WARN 4, chat_status_1 `resumed_session_hides_unknown_token_usage...`. Page tests: double Ctrl+C quits, single
   Ctrl+C reminder expires after 1 s, Esc hint, click on a key label presses it.
2. Status line rows (16). Files: footer/status_rows.rs, widgets/status_line.rs. Read status_surface.rs, composer_layout.rs:31-70,
   status_line.rs, status_line_style.rs, footer/footer_content.rs. Row budget vs draft, links on wrapped rows, slash popup
   priority, light-theme correction, colour depths (after the kit option). Turns on pane_footer_1 27/131/205/269/316/416/489/
   542/1109/1130 + `input_history_search_uses_the_final_row`, footer links 3 (katakana -> `pending(G44)` if it differs),
   status_line_style 2.
3. Status items, warnings, title (28). Files: status_surfaces.rs, window_title.rs, footer/indicators.rs. Read
   status_surfaces.rs:62-1297, goal_status.rs, terminal_title.rs. Add missing ids, invalid-item warning once (a notice cell via
   the existing notice path), goal indicator/clock, action-required, thread identity, daybreak. Turns on chat_status_2 items 11 +
   footer snapshots 9 + title 1, chat_status_1 git 2 + raw output 1, chat_status_3 title 4. Items whose data is a "need" stay
   `pending` with the need named (thread usage, headline, git summary, goal) until it lands; seed the fact through the
   status screen event where the fact is StatusFacts.
4. `?` overlay (6). Files: sheets/dialogs/shortcuts.rs, footer/mod.rs. Read shortcut_overlay.rs, status_surface_tests.rs:883-1002,
   1198. Turns on HELP 5 and `escape_closes_shortcut_help_before_inline_banner` (after chat-live C3's banner).
5. Bottom-pane stack: Working row, preview, background terminals (34). Files: activity.rs, pending_input.rs, footer/unified_exec.rs,
   session_page.rs slot order. Read status_indicator_widget.rs, pending_input_preview.rs, unified_exec_footer.rs, hook_status.rs,
   bottom_pane/mod.rs:2483-2540. Turns on chat_status_3 indicator 7 + `session_status_follows...`, pane_footer_2 status/queue 7 +
   preview 12 + unified exec 2, chat_status_1 `status_widget_active_snapshot` + small heights 2, chat_status_2 vt100 2 (after the
   exec cells of cells-markdown). URL previews: `pending(G49)` / `pending(G48)` only if the run shows the difference.
6. Effort transition and ignition (11). Files: footer/effort.rs, composer prompt accent (shared). Read effort_status_line.rs,
   effort_ignition.rs, chat_composer_effort_tests.rs. Turns on EFFORT 3, pane_footer_1 460/571, pane_footer_2 effort 6
   (frames via `snap().frame(..)`).
7. Sparkle (19). Files: footer/sparkle.rs + composer change events (shared). Read sparkle.rs, sparkle_field.rs, sparkle_tests.rs.
   Turns on the 19 sparkle tests (fake clock: `s.wait(ms)`).
8. Voice footer and strip (3). Files: footer/indicators.rs, src/sheets/voice (read only). Read footer.rs:890, voice_strip.rs.
   Turns on `voice_footer_rendering...`, voice strip 2 (mute binding test waits on the keymap seed).
End: parity pack docs/rtwk-port/parity/chat-status.md, pty run of `codex-rtwk` beside the oracle binary (idle footer, `?`,
Ctrl+C twice, a running turn with a queued message, a narrow 40-column terminal), mouse tests for clickable hints.

## 7. Gaps and risks
- ADR-003: every status value is read from ThreadModel slots / ScreenStates; no screen-local copy of usage, goal or processes.
- R15: only spelling choice from measured widths; no string cutting. A difference that needs cutting: gap row + pending.
- G44 katakana grapheme width (footer link test); G48/G49 wrap rules (URL-like preview messages).
- Kit options owed by the keymap porter (keymap seed, colour depth): 5 + 1 tests wait on them.
- Voice footer/strip need a voice listening state that the port does not have yet (src/sheets/voice holds only the sheet).
- Sparkle eligibility needs the model catalog fact of the "Astra" model (oracle select_sparkle_model, config_panel.rs:106).
- vt100 exec/status layout snapshots need cells-markdown's exec cells; do them last.
- Decision (person): the 5 "extension" status tests and `prepared_tree_uses_its_natural_width...` seed a ui_tree status line that
  no oracle code path produces (only tests and previews call `FooterContent::from(UiTree)`). Recommendation: re-express the
  behaviour assertions (links, width, dim, theme) with native items that yield a link (`pull-request-number`), and `kit::na`
  only the snapshots whose rows are tree-only text (`plain`).
- Decision (orchestrator): one owner for `activity.rs` + `pending_input.rs` after chat-live C1. Recommendation: C1 only carves,
  chat-status takes both files at C1's landing; if C1 already ported the oracle preview layout, step 5 only turns the tests on.
- Decision (orchestrator): owner of the goal fact (ThreadPart.goal) and of the status-line facts in core/src/screens/chat.rs.
  Recommendation: chat-live adds them (it holds chat.rs), chat-status renders; StatusFacts fields can go with step 3 if chat.rs is free.
